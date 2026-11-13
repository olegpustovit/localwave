#import <AVFoundation/AVFoundation.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "platform.h"

static int fail(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Could not prepare audio playback").UTF8String);
    return 0;
}
static double seconds(CMTime time) { double value = CMTimeGetSeconds(time); return isfinite(value) && value >= 0 ? value : 0; }
@interface LWPreparedPlayerItem : NSObject
@property(nonatomic, strong) AVPlayerItem *item;
@property(nonatomic) uint64_t token, occurrence, generation;
@property(nonatomic) BOOL consumed, reportedFailure, reportedCompletion, reportedStart, discarded;
@end
@implementation LWPreparedPlayerItem
@end

void *lw_player_prepare(const char *path, uint64_t token, uint64_t occurrence, int knownLocal, int (*cancelled)(void *), void *user, char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) { fail(error, @"Audio preparation must run on a worker thread"); return NULL; }
        if (!token || !occurrence) { fail(error, @"Playback identities must be nonzero"); return NULL; }
        if (cancelled(user)) { fail(error, @"Audio preparation was cancelled"); return NULL; }
        LWFileInfo info;
        if (!lw_file_probe(path, knownLocal, &info, error)) return NULL;
        if (info.availability == 2 || (info.provider && info.availability == 0)) { fail(error, @"Playback source is not confirmed local"); return NULL; }
        NSURL *url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path]];
        NSError *failure = nil;
        NSDictionary *attributes = [NSFileManager.defaultManager attributesOfItemAtPath:url.path error:&failure];
        if (![attributes[NSFileType] isEqual:NSFileTypeRegular]) { fail(error, failure.localizedDescription ?: @"Playback source must be a regular file"); return NULL; }
        AVURLAsset *asset = [AVURLAsset URLAssetWithURL:url options:nil];
        dispatch_semaphore_t ready = dispatch_semaphore_create(0);
        [asset loadValuesAsynchronouslyForKeys:@[@"playable"] completionHandler:^{ dispatch_semaphore_signal(ready); }];
        uint64_t deadline = clock_gettime_nsec_np(CLOCK_UPTIME_RAW) + 3 * NSEC_PER_SEC;
        while (dispatch_semaphore_wait(ready, dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_MSEC))) {
            if (cancelled(user) || clock_gettime_nsec_np(CLOCK_UPTIME_RAW) >= deadline) { [asset cancelLoading]; fail(error, cancelled(user) ? @"Audio preparation was cancelled" : @"Audio preparation timed out"); return NULL; }
        }
        if (cancelled(user)) { fail(error, @"Audio preparation was cancelled"); return NULL; }
        if ([asset statusOfValueForKey:@"playable" error:&failure] != AVKeyValueStatusLoaded || !asset.playable) { fail(error, failure.localizedDescription ?: @"The audio file could not be decoded"); return NULL; }
        LWPreparedPlayerItem *prepared = [LWPreparedPlayerItem new];
        prepared.item = [AVPlayerItem playerItemWithAsset:asset];
        prepared.token = token;
        prepared.occurrence = occurrence;
        return (__bridge_retained void *)prepared;
    }
}
void lw_player_prepared_release(void *raw) { @autoreleasepool { id object = CFBridgingRelease(raw); (void)object; } }

@interface LWNativePlayer : NSObject {
@public
    LWPlayerEvent events[64];
    size_t head, count;
    BOOL overflow, mutating, active;
    uint64_t generation, queued[3];
    void (*wake)(void);
}
@property(nonatomic, strong) AVQueuePlayer *engine;
@property(nonatomic, strong) NSMutableArray<LWPreparedPlayerItem *> *items;
- (void)push:(int)kind item:(LWPreparedPlayerItem *)item error:(NSError *)error;
- (void)refresh;
- (void)reportStarted;
- (void)attach:(LWPreparedPlayerItem *)item;
- (void)detach:(LWPreparedPlayerItem *)item;
- (void)finished:(NSNotification *)notification;
- (void)failed:(NSNotification *)notification;
@end
@implementation LWNativePlayer
- (void)reportStarted {
    if (!active || mutating || self.engine.timeControlStatus != AVPlayerTimeControlStatusPlaying) return;
    AVPlayerItem *current = self.engine.currentItem;
    for (LWPreparedPlayerItem *item in self.items) if (item.item == current) {
        if (!item.reportedStart && item.item.status == AVPlayerItemStatusReadyToPlay) {
            item.reportedStart = YES;
            [self push:7 item:item error:nil];
        }
        break;
    }
}
- (void)push:(int)kind item:(LWPreparedPlayerItem *)item error:(NSError *)error {
    if (!active || mutating) return;
    if (count == 64) { if (!overflow) { overflow = YES; [self.engine pause]; } return; }
    LWPlayerEvent *event = &events[(head + count++) % 64];
    memset(event, 0, sizeof(*event));
    event->kind = kind;
    event->generation = item ? item.generation : generation;
    event->token = item.token;
    event->occurrence = item.occurrence;
    event->status = item ? (int)item.item.status : (int)self.engine.timeControlStatus;
    event->seconds = item ? seconds(item.item.duration) : 0;
    const char *message = error.localizedDescription.UTF8String;
    if (message) {
        size_t length = MIN(strlen(message), sizeof(event->message) - 1);
        while (length && ((unsigned char)message[length] & 0xc0) == 0x80) length--;
        memcpy(event->message, message, length);
    }
    if (count == 1 && wake) wake();
}
- (void)refresh {
    NSArray<AVPlayerItem *> *enqueued = self.engine.items;
    memset(queued, 0, sizeof(queued));
    for (LWPreparedPlayerItem *item in self.items) {
        if (item.item.status == AVPlayerItemStatusFailed && !item.reportedFailure) { item.reportedFailure = YES; [self push:3 item:item error:item.item.error]; }
        NSUInteger index = [enqueued indexOfObjectIdenticalTo:item.item];
        if (index < 3) queued[index] = item.token;
    }
    if (self.items.count > 6) { overflow = YES; [self.engine pause]; }
}
- (void)attach:(LWPreparedPlayerItem *)item {
    item.consumed = YES;
    item.generation = generation;
    [self.items addObject:item];
    [item.item addObserver:self forKeyPath:@"status" options:NSKeyValueObservingOptionNew context:(__bridge void *)item];
    [NSNotificationCenter.defaultCenter addObserver:self selector:@selector(finished:) name:AVPlayerItemDidPlayToEndTimeNotification object:item.item];
    [NSNotificationCenter.defaultCenter addObserver:self selector:@selector(failed:) name:AVPlayerItemFailedToPlayToEndTimeNotification object:item.item];
}
- (void)detach:(LWPreparedPlayerItem *)item {
    [item.item removeObserver:self forKeyPath:@"status" context:(__bridge void *)item];
    [NSNotificationCenter.defaultCenter removeObserver:self name:AVPlayerItemDidPlayToEndTimeNotification object:item.item];
    [NSNotificationCenter.defaultCenter removeObserver:self name:AVPlayerItemFailedToPlayToEndTimeNotification object:item.item];
}
- (void)finished:(NSNotification *)notification {
    if (!NSThread.isMainThread) {
        __weak LWNativePlayer *owner = self;
        dispatch_async(dispatch_get_main_queue(), ^{ [owner finished:notification]; });
        return;
    }
    for (LWPreparedPlayerItem *item in self.items) if (item.item == notification.object) {
        if (!item.reportedCompletion && !item.discarded) { item.reportedCompletion = YES; [self push:2 item:item error:nil]; }
        break;
    }
}
- (void)failed:(NSNotification *)notification {
    if (!NSThread.isMainThread) {
        __weak LWNativePlayer *owner = self;
        dispatch_async(dispatch_get_main_queue(), ^{ [owner failed:notification]; });
        return;
    }
    for (LWPreparedPlayerItem *item in self.items) if (item.item == notification.object) {
        if (!item.reportedFailure) { item.reportedFailure = YES; [self push:3 item:item error:notification.userInfo[AVPlayerItemFailedToPlayToEndTimeErrorKey] ?: item.item.error]; }
        break;
    }
}
- (void)observeValueForKeyPath:(NSString *)path ofObject:(id)object change:(NSDictionary *)change context:(void *)context {
    if (!NSThread.isMainThread) {
        __weak LWNativePlayer *owner = self;
        LWPreparedPlayerItem *item = context ? (__bridge LWPreparedPlayerItem *)context : nil;
        dispatch_async(dispatch_get_main_queue(), ^{ [owner observeValueForKeyPath:path ofObject:object change:change context:(__bridge void *)item]; });
        return;
    }
    if (!active || mutating) return;
    if (object == self.engine) {
        if ([path isEqual:@"currentItem"]) {
            [self refresh];
            id value = change[NSKeyValueChangeNewKey];
            AVPlayerItem *changed = value == NSNull.null ? nil : [value isKindOfClass:AVPlayerItem.class] ? value : self.engine.currentItem;
            if (!changed && self.engine.currentItem) return;
            LWPreparedPlayerItem *current = nil;
            for (LWPreparedPlayerItem *item in self.items) if (item.item == changed) { current = item; break; }
            if (changed && !current) return;
            [self push:1 item:current error:nil];
            [self reportStarted];
        } else if ([path isEqual:@"status"] && self.engine.status == AVPlayerStatusFailed) [self push:6 item:nil error:self.engine.error];
        else { [self push:5 item:nil error:nil]; [self reportStarted]; }
    } else {
        LWPreparedPlayerItem *item = (__bridge LWPreparedPlayerItem *)context;
        if (![self.items containsObject:item]) return;
        if (item.item.status == AVPlayerItemStatusFailed && !item.reportedFailure) { item.reportedFailure = YES; [self push:3 item:item error:item.item.error]; }
        else if (item.item.status == AVPlayerItemStatusReadyToPlay) { [self push:4 item:item error:nil]; [self reportStarted]; }
    }
}
- (void)dealloc {
    active = NO;
    [self.engine removeObserver:self forKeyPath:@"currentItem"];
    [self.engine removeObserver:self forKeyPath:@"timeControlStatus"];
    [self.engine removeObserver:self forKeyPath:@"status"];
    for (LWPreparedPlayerItem *item in self.items) [self detach:item];
    [self.engine pause];
    [self.engine removeAllItems];
}
@end

void *lw_player_create(char **error) {
    @autoreleasepool {
        if (!NSThread.isMainThread) { fail(error, @"The native queue player belongs to the main thread"); return NULL; }
        LWNativePlayer *player = [LWNativePlayer new];
        player.engine = [AVQueuePlayer new];
        player.engine.actionAtItemEnd = AVPlayerActionAtItemEndAdvance;
        player.items = [NSMutableArray arrayWithCapacity:6];
        [player.engine addObserver:player forKeyPath:@"currentItem" options:NSKeyValueObservingOptionNew context:NULL];
        [player.engine addObserver:player forKeyPath:@"timeControlStatus" options:NSKeyValueObservingOptionNew context:NULL];
        [player.engine addObserver:player forKeyPath:@"status" options:NSKeyValueObservingOptionNew context:NULL];
        player->active = YES;
        return (__bridge_retained void *)player;
    }
}
int lw_player_replace(void *raw, uint64_t version, void *preparedRaw, char **error) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        LWPreparedPlayerItem *item = (__bridge LWPreparedPlayerItem *)preparedRaw;
        if (!version || !item || item.consumed) return fail(error, @"A playback item can be installed only once");
        if (player.engine.status == AVPlayerStatusFailed) return fail(error, @"The audio engine failed; create a new queue player before retrying");
        player->mutating = YES;
        [player.engine pause];
        for (LWPreparedPlayerItem *old in player.items) [player detach:old];
        [player.engine removeAllItems];
        [player.items removeAllObjects];
        player->generation = version;
        player->head = player->count = 0;
        player->overflow = NO;
        [player attach:item];
        [player.engine insertItem:item.item afterItem:nil];
        player->mutating = NO;
        [player refresh];
        [player push:1 item:item error:nil];
        return 1;
    }
}
int lw_player_upcoming(void *raw, uint64_t version, uint64_t current, void *const *prepared, size_t count, char **error) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        [player refresh];
        if (player->generation != version || player->queued[0] != current || !current) return 2;
        if (count > 2) return fail(error, @"Only two upcoming audio items may be prepared");
        for (size_t i = 0; i < count; i++) {
            LWPreparedPlayerItem *item = (__bridge LWPreparedPlayerItem *)prepared[i];
            if (!item || item.consumed || item.token == current) return fail(error, @"Upcoming playback identities must be distinct");
            for (LWPreparedPlayerItem *old in player.items) if (old.token == item.token) return fail(error, @"Playback tokens must be distinct");
            for (size_t j = 0; j < i; j++) if (item.token == ((__bridge LWPreparedPlayerItem *)prepared[j]).token) return fail(error, @"Upcoming playback identities must be distinct");
        }
        player->mutating = YES;
        AVPlayerItem *playing = player.engine.currentItem;
        NSArray *enqueued = player.engine.items;
        for (LWPreparedPlayerItem *old in [player.items copy]) if (old.item != playing && ([enqueued containsObject:old.item] || old.reportedCompletion || old.reportedFailure || old.discarded)) { [player detach:old]; [player.engine removeItem:old.item]; [player.items removeObjectIdenticalTo:old]; }
        AVPlayerItem *previous = playing;
        for (size_t i = 0; i < count; i++) {
            LWPreparedPlayerItem *item = (__bridge LWPreparedPlayerItem *)prepared[i];
            [player attach:item];
            [player.engine insertItem:item.item afterItem:previous];
            previous = item.item;
        }
        player->mutating = NO;
        [player refresh];
        return 1;
    }
}
int lw_player_append(void *raw, uint64_t version, uint64_t current, void *preparedRaw, char **error) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        LWPreparedPlayerItem *item = (__bridge LWPreparedPlayerItem *)preparedRaw;
        [player refresh];
        if (player->generation != version || player->queued[0] != current || !current) return 2;
        NSArray<AVPlayerItem *> *enqueued = player.engine.items;
        if (enqueued.count >= 3) return fail(error, @"The native prequeue already contains two upcoming items");
        if (!item || item.consumed) return fail(error, @"A playback item can be installed only once");
        for (LWPreparedPlayerItem *old in player.items) if (old.token == item.token) return fail(error, @"Playback tokens must be distinct");
        player->mutating = YES;
        for (LWPreparedPlayerItem *old in [player.items copy]) if (old.item != player.engine.currentItem && ![enqueued containsObject:old.item] && (old.reportedCompletion || old.reportedFailure || old.discarded)) { [player detach:old]; [player.items removeObjectIdenticalTo:old]; }
        [player attach:item];
        [player.engine insertItem:item.item afterItem:enqueued.lastObject];
        player->mutating = NO;
        [player refresh];
        return 1;
    }
}
void lw_player_play(void *raw) { [((__bridge LWNativePlayer *)raw).engine play]; }
void lw_player_set_wake(void *raw, void (*wake)(void)) { ((__bridge LWNativePlayer *)raw)->wake = wake; }
int lw_player_trim(void *raw, uint64_t version, uint64_t current, size_t keep, char **error) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        [player refresh];
        if (player->generation != version || player->queued[0] != current || !current) return 2;
        NSArray<AVPlayerItem *> *enqueued = player.engine.items;
        if (keep > 2 || keep + 1 > enqueued.count) return fail(error, @"Invalid native prequeue prefix");
        player->mutating = YES;
        for (NSUInteger i = keep + 1; i < enqueued.count; i++) {
            AVPlayerItem *removed = enqueued[i];
            for (LWPreparedPlayerItem *item in [player.items copy]) if (item.item == removed) {
                item.discarded = YES;
                [player detach:item];
                [player.items removeObjectIdenticalTo:item];
                break;
            }
            [player.engine removeItem:removed];
        }
        player->mutating = NO;
        [player refresh];
        return 1;
    }
}
void lw_player_pause(void *raw) { [((__bridge LWNativePlayer *)raw).engine pause]; }
void lw_player_advance(void *raw) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        for (LWPreparedPlayerItem *item in player.items) if (item.item == player.engine.currentItem) item.discarded = YES;
        [player.engine advanceToNextItem];
        [player refresh];
    }
}
void lw_player_stop(void *raw) {
    @autoreleasepool {
        LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
        player->mutating = YES;
        [player.engine pause];
        for (LWPreparedPlayerItem *item in player.items) [player detach:item];
        [player.engine removeAllItems];
        [player.items removeAllObjects];
        memset(player->queued, 0, sizeof(player->queued));
        player->head = player->count = 0;
        player->overflow = NO;
        player->mutating = NO;
    }
}
void lw_player_seek(void *raw, double value) {
    LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
    double duration = player.engine.currentItem ? seconds(player.engine.currentItem.duration) : 0;
    __weak LWNativePlayer *owner = player;
    [player.engine seekToTime:CMTimeMakeWithSeconds(fmin(fmax(0, value), duration), 600) toleranceBefore:kCMTimeZero toleranceAfter:kCMTimeZero completionHandler:^(BOOL finished) {
        dispatch_async(dispatch_get_main_queue(), ^{ if (finished) [owner push:5 item:nil error:nil]; });
    }];
}
void lw_player_volume(void *raw, float volume) { ((__bridge LWNativePlayer *)raw).engine.volume = volume; }
void lw_player_snapshot(void *raw, LWPlayerSnapshot *snapshot) {
    @autoreleasepool {
    LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
    [player refresh];
    memset(snapshot, 0, sizeof(*snapshot));
    snapshot->generation = player->generation;
    memcpy(snapshot->queued, player->queued, sizeof(snapshot->queued));
    AVPlayerItem *current = player.engine.currentItem;
    for (LWPreparedPlayerItem *item in player.items) if (item.item == current) { snapshot->token = item.token; snapshot->occurrence = item.occurrence; break; }
    snapshot->position = seconds(player.engine.currentTime);
    snapshot->duration = current ? seconds(current.duration) : 0;
    snapshot->rate = player.engine.rate;
    snapshot->volume = player.engine.volume;
    snapshot->status = current ? (int)current.status : 0;
    snapshot->time_control = (int)player.engine.timeControlStatus;
    }
}
int lw_player_event(void *raw, LWPlayerEvent *event) {
    LWNativePlayer *player = (__bridge LWNativePlayer *)raw;
    if (player->overflow) return -1;
    if (!player->count) return 0;
    *event = player->events[player->head];
    player->head = (player->head + 1) % 64;
    player->count--;
    return 1;
}
void lw_player_release(void *raw) { @autoreleasepool { id player = CFBridgingRelease(raw); (void)player; } }
void lw_player_run_loop(double value) { @autoreleasepool { CFRunLoopRunInMode(kCFRunLoopDefaultMode, value, true); } }
