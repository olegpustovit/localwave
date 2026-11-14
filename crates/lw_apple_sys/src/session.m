#import <Foundation/Foundation.h>
#import <TargetConditionals.h>
#if TARGET_OS_IOS
#import <AVFAudio/AVFAudio.h>
#endif
#include <stdlib.h>
#include <string.h>
#include "platform.h"
_Static_assert(sizeof(LWSessionEvent) == 16, "audio-session event layout");

@interface LWAudioSession : NSObject {
@public
    NSLock *lock, *activation;
    BOOL configured, active, overflow;
    uint64_t epoch, sequence, pauseSequence, processedSequence;
    LWSessionEvent events[32];
    size_t head, count;
    void (*wake)(void);
    BOOL (^configure)(NSError **), (^activate)(NSError **);
    NSNotificationCenter *center;
    id interruption, route, reset;
}
- (void)push:(uint32_t)kind resume:(BOOL)resume;
@end
@implementation LWAudioSession
- (instancetype)init {
    if ((self = [super init])) { lock = [NSLock new]; activation = [NSLock new]; if (!lock || !activation) return nil; }
    return self;
}
- (void)push:(uint32_t)kind resume:(BOOL)resume {
    [lock lock];
    if (kind == 1) active = NO;
    if (kind == 5) configured = active = NO;
    if (kind != 2) ++epoch;
    if (count == 32) { head = count = 0; overflow = YES; active = NO; ++epoch; }
    if (!overflow || count == 0) {
        uint32_t queuedKind = overflow ? 4 : kind;
        events[(head + count++) % 32] = (LWSessionEvent){++sequence, queuedKind, resume && !overflow};
        if (queuedKind != 2) pauseSequence = sequence;
    }
    void (*callback)(void) = wake;
    [lock unlock];
    if (callback) callback();
}
- (void)dealloc {
    [center removeObserver:interruption];
    [center removeObserver:route];
    [center removeObserver:reset];
}
@end

void *lw_session_create(void (*wake)(void)) {
    @autoreleasepool {
        LWAudioSession *owner = [LWAudioSession new];
        if (!owner) return NULL;
        owner->wake = wake;
#if TARGET_OS_IOS
        owner->configure = ^BOOL(NSError **error) { return [AVAudioSession.sharedInstance setCategory:AVAudioSessionCategoryPlayback mode:AVAudioSessionModeDefault options:0 error:error]; };
        owner->activate = ^BOOL(NSError **error) { return [AVAudioSession.sharedInstance setActive:YES error:error]; };
        owner->center = NSNotificationCenter.defaultCenter;
        __weak LWAudioSession *weak = owner;
        owner->interruption = [owner->center addObserverForName:AVAudioSessionInterruptionNotification object:nil queue:nil usingBlock:^(NSNotification *note) {
            NSNumber *type = note.userInfo[AVAudioSessionInterruptionTypeKey];
            if (![type isKindOfClass:NSNumber.class]) return;
            if (type.unsignedIntegerValue == AVAudioSessionInterruptionTypeBegan) [weak push:1 resume:NO];
            else if (type.unsignedIntegerValue == AVAudioSessionInterruptionTypeEnded) {
                NSNumber *options = note.userInfo[AVAudioSessionInterruptionOptionKey];
                [weak push:2 resume:[options isKindOfClass:NSNumber.class] && (options.unsignedIntegerValue & AVAudioSessionInterruptionOptionShouldResume) != 0];
            }
        }];
        owner->route = [owner->center addObserverForName:AVAudioSessionRouteChangeNotification object:nil queue:nil usingBlock:^(NSNotification *note) {
            NSNumber *reason = note.userInfo[AVAudioSessionRouteChangeReasonKey];
            if ([reason isKindOfClass:NSNumber.class] && reason.unsignedIntegerValue == AVAudioSessionRouteChangeReasonOldDeviceUnavailable) [weak push:3 resume:NO];
        }];
        owner->reset = [owner->center addObserverForName:AVAudioSessionMediaServicesWereResetNotification object:nil queue:nil usingBlock:^(NSNotification *note) { (void)note; [weak push:5 resume:NO]; }];
#else
        owner->configure = ^BOOL(NSError **error) { (void)error; return YES; };
        owner->activate = ^BOOL(NSError **error) { (void)error; return YES; };
#endif
        return (__bridge_retained void *)owner;
    }
}
int lw_session_activate(void *raw, char **error) {
    if (NSThread.isMainThread) { if (error) *error = strdup("Audio session activation must run on a worker"); return -1; }
    @autoreleasepool {
        LWAudioSession *owner = (__bridge LWAudioSession *)raw;
        [owner->activation lock];
        [owner->lock lock];
        BOOL ready = owner->active, blocked = owner->pauseSequence > owner->processedSequence || owner->overflow, configured = owner->configured;
        uint64_t epoch = owner->epoch;
        [owner->lock unlock];
        int result = blocked ? 0 : ready ? 1 : -1;
        NSError *failure = nil;
        // A notification can arrive inside either blocking call; stale results must not restore caches.
        if (!blocked && !ready) {
            BOOL configuredNow = configured || owner->configure(&failure);
            [owner->lock lock];
            BOOL stale = owner->epoch != epoch;
            if (!stale) owner->configured = configuredNow;
            [owner->lock unlock];
            if (configuredNow && !stale) {
                BOOL activated = owner->activate(&failure);
                [owner->lock lock];
                stale = owner->epoch != epoch;
                if (activated && !stale) owner->active = YES;
                [owner->lock unlock];
                result = stale ? 0 : activated ? 1 : -1;
            } else if (stale) result = 0;
        }
        [owner->activation unlock];
        if (result < 0 && error) *error = strdup((failure.localizedDescription ?: @"Could not activate the audio session").UTF8String);
        return result;
    }
}
int lw_session_peek(void *raw, LWSessionEvent *event) {
    LWAudioSession *owner = (__bridge LWAudioSession *)raw;
    [owner->lock lock];
    BOOL present = owner->count != 0;
    if (present) *event = owner->events[owner->head];
    [owner->lock unlock];
    return present;
}
void lw_session_ack(void *raw, uint64_t sequence) {
    LWAudioSession *owner = (__bridge LWAudioSession *)raw;
    [owner->lock lock];
    if (owner->count && owner->events[owner->head].sequence == sequence) {
        if (owner->events[owner->head].kind == 4) owner->overflow = NO;
        owner->head = (owner->head + 1) % 32; --owner->count;
    }
    [owner->lock unlock];
}
void lw_session_processed(void *raw, uint64_t sequence) {
    LWAudioSession *owner = (__bridge LWAudioSession *)raw;
    [owner->lock lock];
    owner->processedSequence = MAX(owner->processedSequence, sequence);
    [owner->lock unlock];
}
void lw_session_release(void *raw) { @autoreleasepool { (void)CFBridgingRelease(raw); } }

#ifdef LW_INTEROP_TEST
void lw_session_test_push(void *raw, uint32_t kind, int resume) { [(__bridge LWAudioSession *)raw push:kind resume:resume != 0]; }
int lw_session_test_notifications(void) {
#if TARGET_OS_IOS
    @autoreleasepool {
        LWAudioSession *owner = CFBridgingRelease(lw_session_create(NULL));
        void *raw = (__bridge void *)owner;
        NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
        [center postNotificationName:AVAudioSessionInterruptionNotification object:nil userInfo:@{AVAudioSessionInterruptionTypeKey:@"invalid"}];
        LWSessionEvent event;
        if (lw_session_peek(raw,&event)) return 1;
        [center postNotificationName:AVAudioSessionRouteChangeNotification object:nil userInfo:@{AVAudioSessionRouteChangeReasonKey:@(AVAudioSessionRouteChangeReasonNewDeviceAvailable)}];
        if (lw_session_peek(raw,&event)) return 2;
        [center postNotificationName:AVAudioSessionInterruptionNotification object:nil userInfo:@{AVAudioSessionInterruptionTypeKey:@(AVAudioSessionInterruptionTypeBegan)}];
        if (!lw_session_peek(raw,&event) || event.kind!=1) return 3;
        lw_session_ack(raw,event.sequence);
        [center postNotificationName:AVAudioSessionRouteChangeNotification object:nil userInfo:@{AVAudioSessionRouteChangeReasonKey:@(AVAudioSessionRouteChangeReasonOldDeviceUnavailable)}];
        if (!lw_session_peek(raw,&event) || event.kind!=3) return 4;
        lw_session_ack(raw,event.sequence);
        [center postNotificationName:AVAudioSessionInterruptionNotification object:nil userInfo:@{AVAudioSessionInterruptionTypeKey:@(AVAudioSessionInterruptionTypeEnded),AVAudioSessionInterruptionOptionKey:@(AVAudioSessionInterruptionOptionShouldResume)}];
        if (!lw_session_peek(raw,&event) || event.kind!=2 || event.resume!=1) return 5;
        lw_session_ack(raw,event.sequence);
        if (lw_session_peek(raw,&event)) return 6;
        [center postNotificationName:AVAudioSessionMediaServicesWereResetNotification object:nil];
        if (!lw_session_peek(raw,&event) || event.kind!=5 || owner->active || owner->configured) return 7;
        lw_session_ack(raw,event.sequence);
    }
#endif
    return 0;
}
int lw_session_test_activation(void) {
    if (NSThread.isMainThread) return 1;
    @autoreleasepool {
        LWAudioSession *owner = [LWAudioSession new];
        __block unsigned configurations = 0, activations = 0;
        owner->configure = ^BOOL(NSError **error) { (void)error; ++configurations; return YES; };
        owner->activate = ^BOOL(NSError **error) { (void)error; ++activations; return YES; };
        void *raw = (__bridge void *)owner;
        for (unsigned i = 0; i < 100; ++i) if (lw_session_activate(raw, NULL) != 1) return 2;
        if (configurations != 1 || activations != 1) return 3;
        [owner push:1 resume:NO];
        if (lw_session_activate(raw, NULL) != 0) return 4;
        LWSessionEvent notice;
        if (!lw_session_peek(raw,&notice)) return 12;
        lw_session_ack(raw,notice.sequence);
        if (lw_session_activate(raw,NULL)!=0) return 13;
        [owner push:1 resume:NO];
        lw_session_processed(raw,notice.sequence);
        if (lw_session_activate(raw,NULL)!=0) return 14;
        lw_session_processed(raw,owner->pauseSequence);
        if (lw_session_activate(raw,NULL)!=1 || activations!=2) return 5;
        [owner push:2 resume:YES];
        if (lw_session_activate(raw, NULL) != 1 || configurations != 1 || activations != 2) return 5;
        [owner push:5 resume:NO];
        if (owner->active || owner->configured || lw_session_activate(raw,NULL)!=0) return 9;
        lw_session_processed(raw,owner->pauseSequence);
        if (lw_session_activate(raw,NULL)!=1 || configurations!=2 || activations!=3) return 9;
        owner->active = NO;
        __weak LWAudioSession *weak = owner;
        owner->activate = ^BOOL(NSError **error) { (void)error; [weak push:1 resume:NO]; return YES; };
        if (lw_session_activate(raw, NULL) != 0 || owner->active) return 6;
        [owner push:2 resume:NO];
        lw_session_processed(raw,owner->pauseSequence);
        owner->activate = ^BOOL(NSError **error) { if (error) *error = [NSError errorWithDomain:@"fixture" code:1 userInfo:nil]; return NO; };
        char *failure = NULL;
        int result = lw_session_activate(raw, &failure);
        BOOL failed = result == -1 && failure;
        free(failure);
        if (!failed) return 7;
        owner->configured = NO;
        owner->configure = ^BOOL(NSError **error) { if (error) *error = [NSError errorWithDomain:@"configuration" code:2 userInfo:nil]; return NO; };
        failure = NULL;
        result = lw_session_activate(raw, &failure);
        failed = result == -1 && failure && !owner->configured;
        free(failure);
        if (!failed) return 8;
        owner->configure = ^BOOL(NSError **error) { (void)error; [weak push:5 resume:NO]; return YES; };
        if (lw_session_activate(raw,NULL)!=0 || owner->configured || owner->active) return 10;
        lw_session_processed(raw,owner->pauseSequence);
        owner->configure = ^BOOL(NSError **error) { (void)error; return YES; };
        owner->activate = ^BOOL(NSError **error) { (void)error; for (unsigned i=0;i<33;++i) [weak push:2 resume:NO]; return YES; };
        if (lw_session_activate(raw,NULL)!=0 || !owner->overflow || owner->active) return 11;
        return 0;
    }
}
#endif
