#import <TargetConditionals.h>
#if TARGET_OS_OSX
#import <AppKit/AppKit.h>
#else
#import <UIKit/UIKit.h>
#endif
#import <MediaPlayer/MediaPlayer.h>
#include <math.h>
#include <string.h>
#include "platform.h"

@interface LWMedia : NSObject {
@public
    NSLock *lock;
    LWMediaCommand pending[32];
    size_t head, count;
    unsigned enabled;
    BOOL closed;
    void (*wake)(void);
    MPNowPlayingInfoCenter *center;
    MPRemoteCommandCenter *commands;
    NSArray<MPRemoteCommand *> *registered;
    NSMutableArray *targets;
    NSMutableDictionary *info;
    LWMediaState state;
}
- (instancetype)initWithCenter:(MPNowPlayingInfoCenter *)sink commands:(MPRemoteCommandCenter *)remote wake:(void (*)(void))callback;
- (MPRemoteCommandHandlerStatus)enqueue:(int)kind seconds:(double)seconds;
- (void)close;
@end

@implementation LWMedia
- (instancetype)initWithCenter:(MPNowPlayingInfoCenter *)sink commands:(MPRemoteCommandCenter *)remote wake:(void (*)(void))callback {
    self = [super init];
    if (self) {
        lock = [NSLock new];
        center = sink;
        commands = remote;
        wake = callback;
        if (commands) {
            registered = @[commands.playCommand, commands.pauseCommand, commands.togglePlayPauseCommand,
                           commands.stopCommand, commands.nextTrackCommand, commands.previousTrackCommand,
                           commands.changePlaybackPositionCommand];
            targets = [NSMutableArray arrayWithCapacity:registered.count];
            __weak LWMedia *owner = self;
            for (NSUInteger index = 0; index < registered.count; ++index) {
                int kind = (int)index + 1;
                id token = [registered[index] addTargetWithHandler:^MPRemoteCommandHandlerStatus(MPRemoteCommandEvent *event) {
                    LWMedia *live = owner;
                    if (!live) return MPRemoteCommandHandlerStatusCommandFailed;
                    double seconds = 0;
                    if (kind == 7) {
                        if (![event isKindOfClass:MPChangePlaybackPositionCommandEvent.class]) return MPRemoteCommandHandlerStatusCommandFailed;
                        seconds = ((MPChangePlaybackPositionCommandEvent *)event).positionTime;
                    }
                    return [live enqueue:kind seconds:seconds];
                }];
                [targets addObject:token];
                registered[index].enabled = NO;
            }
        }
    }
    return self;
}
- (MPRemoteCommandHandlerStatus)enqueue:(int)kind seconds:(double)seconds {
    if (kind < 1 || kind > 7 || !isfinite(seconds) || seconds < 0) return MPRemoteCommandHandlerStatusCommandFailed;
    [lock lock];
    MPRemoteCommandHandlerStatus result;
    if (closed) result = MPRemoteCommandHandlerStatusCommandFailed;
    else if (!(enabled & (1u << kind))) result = MPRemoteCommandHandlerStatusNoActionableNowPlayingItem;
    else if (count == 32) result = MPRemoteCommandHandlerStatusCommandFailed;
    else {
        pending[(head + count++) % 32] = (LWMediaCommand){kind, seconds};
        result = MPRemoteCommandHandlerStatusSuccess;
        if (wake) wake();
    }
    [lock unlock];
    return result;
}
- (void)close {
    [lock lock];
    closed = YES;
    wake = NULL;
    [lock unlock];
    for (NSUInteger index = 0; index < registered.count; ++index) {
        [registered[index] removeTarget:targets[index]];
        registered[index].enabled = NO;
    }
    [targets removeAllObjects];
    registered = nil;
    center.nowPlayingInfo = nil;
    center.playbackState = MPNowPlayingPlaybackStateStopped;
}
@end

void *lw_media_create(int publish, void (*wake)(void)) {
    if (!NSThread.isMainThread) return NULL;
#if TARGET_OS_OSX
    if (publish && !NSApp) return NULL;
#endif
    @autoreleasepool {
        return (__bridge_retained void *)[[LWMedia alloc] initWithCenter:publish ? MPNowPlayingInfoCenter.defaultCenter : nil
            commands:publish ? MPRemoteCommandCenter.sharedCommandCenter : nil wake:wake];
    }
}
int lw_media_peek(void *raw, LWMediaCommand *command) {
    LWMedia *media = (__bridge LWMedia *)raw;
    [media->lock lock];
    BOOL present = media->count != 0;
    if (present) *command = media->pending[media->head];
    [media->lock unlock];
    return present;
}
void lw_media_ack(void *raw) {
    LWMedia *media = (__bridge LWMedia *)raw;
    [media->lock lock];
    if (media->count) { media->head = (media->head + 1) % 32; --media->count; }
    [media->lock unlock];
}
static NSString *text(const unsigned char *bytes, size_t len) {
    return [[NSString alloc] initWithBytes:bytes length:len encoding:NSUTF8StringEncoding];
}
int lw_media_update(void *raw, const LWMediaInfo *value) {
    if (!NSThread.isMainThread || !isfinite(value->position) || value->position < 0 ||
        !isfinite(value->duration) || value->duration < 0 || value->playback_state < 1 || value->playback_state > 3) return 0;
    @autoreleasepool {
        LWMedia *media = (__bridge LWMedia *)raw;
        NSString *title = text(value->title, value->title_len), *artist = text(value->artist, value->artist_len), *album = text(value->album, value->album_len);
        if (!title || !artist || !album) return 0;
        BOOL identity = value->song != media->state.song || value->generation != media->state.generation;
        NSMutableDictionary *next = nil;
        if (value->song) {
            next = [@{MPMediaItemPropertyTitle:title, MPMediaItemPropertyArtist:artist, MPMediaItemPropertyAlbumTitle:album,
                MPNowPlayingInfoPropertyElapsedPlaybackTime:@(value->position), MPMediaItemPropertyPlaybackDuration:@(value->duration),
                MPNowPlayingInfoPropertyPlaybackRate:@(value->playback_state == 1 ? 1.0 : 0.0),
                MPNowPlayingInfoPropertyDefaultPlaybackRate:@1.0, MPNowPlayingInfoPropertyMediaType:@(MPNowPlayingInfoMediaTypeAudio),
                MPNowPlayingInfoPropertyIsLiveStream:@NO} mutableCopy];
            if (!identity && media->info[MPMediaItemPropertyArtwork]) next[MPMediaItemPropertyArtwork] = media->info[MPMediaItemPropertyArtwork];
        }
        media->info = next;
        unsigned enabled = value->has_queue ? (1u << 1) | (1u << 3) | (1u << 5) | (1u << 6) : 0;
        if (value->song) enabled |= (1u << 2) | (1u << 4);
        if (value->song && value->duration > 0) enabled |= (1u << 7);
        [media->lock lock]; media->enabled = enabled; [media->lock unlock];
        for (NSUInteger index = 0; index < media->registered.count; ++index) media->registered[index].enabled = (enabled & (1u << (index + 1))) != 0;
        media->state = (LWMediaState){value->song, value->generation, value->position, value->duration,
            value->playback_state, identity ? 0 : media->state.artwork_width, identity ? 0 : media->state.artwork_height, media->state.updates + 1};
        media->center.nowPlayingInfo = media->info;
        media->center.playbackState = value->playback_state == 1 ? MPNowPlayingPlaybackStatePlaying :
            value->playback_state == 2 ? MPNowPlayingPlaybackStatePaused : MPNowPlayingPlaybackStateStopped;
        return 1;
    }
}
int lw_media_artwork(void *raw, const unsigned char *pixels, size_t len, uint32_t width, uint32_t height) {
    if (!NSThread.isMainThread || width > 1024 || height > 1024 || len != (size_t)width * height * 4 || (len && !pixels) || ((!width || !height) && (width || height))) return 0;
    @autoreleasepool {
        LWMedia *media = (__bridge LWMedia *)raw;
        if (!media->info) return 1;
        if (width && height) {
#if TARGET_OS_OSX
            NSBitmapImageRep *bitmap = [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:NULL pixelsWide:width pixelsHigh:height bitsPerSample:8 samplesPerPixel:4 hasAlpha:YES isPlanar:NO colorSpaceName:NSDeviceRGBColorSpace bitmapFormat:NSBitmapFormatAlphaNonpremultiplied bytesPerRow:width * 4 bitsPerPixel:32];
            if (!bitmap) return 0;
            memcpy(bitmap.bitmapData, pixels, len);
            bitmap = [bitmap bitmapImageRepByRetaggingWithColorSpace:NSColorSpace.sRGBColorSpace];
            if (!bitmap) return 0;
            NSImage *image = [[NSImage alloc] initWithSize:NSMakeSize(width, height)];
            [image addRepresentation:bitmap];
            media->info[MPMediaItemPropertyArtwork] = [[MPMediaItemArtwork alloc] initWithBoundsSize:image.size requestHandler:^NSImage *(CGSize size) { (void)size; return image; }];
#else
            NSData *data = [NSData dataWithBytes:pixels length:len];
            if (!data) return 0;
            CGDataProviderRef provider = CGDataProviderCreateWithCFData((__bridge CFDataRef)data);
            CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            CGImageRef bitmap = provider && space ? CGImageCreate(width, height, 8, 32, width * 4, space,
                kCGBitmapByteOrder32Big | kCGImageAlphaLast, provider, NULL, false, kCGRenderingIntentDefault) : NULL;
            if (provider) CGDataProviderRelease(provider);
            if (space) CGColorSpaceRelease(space);
            if (!bitmap) return 0;
            UIImage *image = [UIImage imageWithCGImage:bitmap scale:1 orientation:UIImageOrientationUp];
            CGImageRelease(bitmap);
            if (!image) return 0;
            media->info[MPMediaItemPropertyArtwork] = [[MPMediaItemArtwork alloc] initWithBoundsSize:image.size requestHandler:^UIImage *(CGSize size) { (void)size; return image; }];
#endif
        } else [media->info removeObjectForKey:MPMediaItemPropertyArtwork];
        media->state.artwork_width = width;
        media->state.artwork_height = height;
        media->center.nowPlayingInfo = media->info;
        return 1;
    }
}
void lw_media_release(void *raw) {
    @autoreleasepool { LWMedia *media = CFBridgingRelease(raw); [media close]; }
}

#ifdef LW_INTEROP_TEST
#include <stdatomic.h>
@interface LWTestMediaCommand : NSObject
@property BOOL enabled;
@property(copy) MPRemoteCommandHandlerStatus (^handler)(MPRemoteCommandEvent *);
@property(strong) id token;
@property unsigned removed;
- (id)addTargetWithHandler:(MPRemoteCommandHandlerStatus (^)(MPRemoteCommandEvent *))handler;
- (void)removeTarget:(id)target;
@end
@implementation LWTestMediaCommand
- (id)addTargetWithHandler:(MPRemoteCommandHandlerStatus (^)(MPRemoteCommandEvent *))handler { self.handler = handler; self.token = [NSObject new]; return self.token; }
- (void)removeTarget:(id)target { if (target == self.token) { ++self.removed; self.handler = nil; self.token = nil; } }
@end
@interface LWTestMediaCenter : NSObject
@property(copy) NSDictionary *nowPlayingInfo;
@property MPNowPlayingPlaybackState playbackState;
@end
@implementation LWTestMediaCenter
@end
@interface LWTestRemoteCenter : NSObject
@property(strong) NSArray<LWTestMediaCommand *> *items;
@end
@implementation LWTestRemoteCenter
- (id)playCommand { return self.items[0]; }
- (id)pauseCommand { return self.items[1]; }
- (id)togglePlayPauseCommand { return self.items[2]; }
- (id)stopCommand { return self.items[3]; }
- (id)nextTrackCommand { return self.items[4]; }
- (id)previousTrackCommand { return self.items[5]; }
- (id)changePlaybackPositionCommand { return self.items[6]; }
@end
@interface LWTestSeekEvent : NSObject
@property double positionTime;
@end
@implementation LWTestSeekEvent
- (BOOL)isKindOfClass:(Class)type { return type == MPChangePlaybackPositionCommandEvent.class || [super isKindOfClass:type]; }
@end
static atomic_uint mediaTestWakes;
static void mediaTestWake(void) { atomic_fetch_add(&mediaTestWakes, 1); }
int lw_media_test_callbacks(void) {
    if (!NSThread.isMainThread) return 1;
    @autoreleasepool {
        LWTestMediaCenter *center = [LWTestMediaCenter new];
        LWTestRemoteCenter *remote = [LWTestRemoteCenter new];
        NSMutableArray *items = [NSMutableArray new];
        for (unsigned i = 0; i < 7; ++i) [items addObject:[LWTestMediaCommand new]];
        remote.items = items;
        LWMedia *media = [[LWMedia alloc] initWithCenter:(MPNowPlayingInfoCenter *)center commands:(MPRemoteCommandCenter *)remote wake:mediaTestWake];
        for (LWTestMediaCommand *command in remote.items) if (command.enabled) return 2;
        if (remote.items[0].handler(nil) != MPRemoteCommandHandlerStatusNoActionableNowPlayingItem) return 3;
        const unsigned char title[] = {'A', 0, 'B', 0xf0, 0x9f, 0x8e, 0xb5};
        LWMediaInfo value = {7, 8, title, sizeof(title), (const unsigned char *)"Artist", 6, (const unsigned char *)"Album", 5, 2.5, 20, 1, 1};
        void *raw = (__bridge void *)media;
        if (!lw_media_update(raw, &value) || center.playbackState != MPNowPlayingPlaybackStatePlaying ||
            ![center.nowPlayingInfo[MPMediaItemPropertyTitle] isEqual:text(title, sizeof(title))] ||
            [center.nowPlayingInfo[MPNowPlayingInfoPropertyElapsedPlaybackTime] doubleValue] != 2.5 ||
            [center.nowPlayingInfo[MPMediaItemPropertyPlaybackDuration] doubleValue] != 20 ||
            [center.nowPlayingInfo[MPNowPlayingInfoPropertyPlaybackRate] doubleValue] != 1) return 4;
        unsigned before = atomic_load(&mediaTestWakes);
        for (unsigned i = 0; i < 6; ++i) if (!remote.items[i].enabled || remote.items[i].handler(nil) != MPRemoteCommandHandlerStatusSuccess) return 5;
        LWTestSeekEvent *event = [LWTestSeekEvent new]; event.positionTime = 6.25;
        if (remote.items[6].handler((MPRemoteCommandEvent *)event) != MPRemoteCommandHandlerStatusSuccess ||
            remote.items[6].handler(nil) != MPRemoteCommandHandlerStatusCommandFailed) return 6;
        event.positionTime = NAN;
        if (remote.items[6].handler((MPRemoteCommandEvent *)event) != MPRemoteCommandHandlerStatusCommandFailed) return 7;
        LWMediaCommand command;
        for (unsigned i = 1; i <= 7; ++i) {
            if (!lw_media_peek(raw, &command) || command.kind != (int)i || (i == 7 && command.seconds != 6.25)) return 8;
            LWMediaCommand repeated; lw_media_peek(raw, &repeated);
            if (repeated.kind != command.kind || repeated.seconds != command.seconds) return 9;
            lw_media_ack(raw);
        }
        if (lw_media_peek(raw, &command) || atomic_load(&mediaTestWakes) != before + 7) return 10;
        // Real callbacks may arrive off the main thread; their queue must not touch the player.
        dispatch_apply(32, dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^(size_t i) {
            (void)i; remote.items[0].handler(nil);
        });
        if (media->count != 32 || remote.items[1].handler(nil) != MPRemoteCommandHandlerStatusCommandFailed) return 11;
        for (unsigned i = 0; i < 32; ++i) { if (!lw_media_peek(raw, &command) || command.kind != 1) return 12; lw_media_ack(raw); }
        const unsigned char pixels[] = {255, 0, 0, 255, 0, 128, 255, 128};
        if (!lw_media_artwork(raw, pixels, sizeof(pixels), 2, 1)) return 13;
        MPMediaItemArtwork *artwork = center.nowPlayingInfo[MPMediaItemPropertyArtwork];
#if TARGET_OS_OSX
        NSImage *image = [artwork imageWithSize:NSMakeSize(2, 1)];
        NSBitmapImageRep *bitmap = (NSBitmapImageRep *)image.representations.firstObject;
        if (!bitmap || memcmp(bitmap.bitmapData, pixels, sizeof(pixels)) || media->state.artwork_width != 2) return 14;
#else
        UIImage *image = [artwork imageWithSize:CGSizeMake(2, 1)];
        CGImageRef bitmap = image.CGImage;
        CFDataRef data = bitmap ? CGDataProviderCopyData(CGImageGetDataProvider(bitmap)) : NULL;
        BOOL matches = bitmap && CGImageGetWidth(bitmap) == 2 && CGImageGetHeight(bitmap) == 1 &&
            CGImageGetBytesPerRow(bitmap) == 8 && data && CFDataGetLength(data) == sizeof(pixels) &&
            memcmp(CFDataGetBytePtr(data), pixels, sizeof(pixels)) == 0 && media->state.artwork_width == 2;
        if (data) CFRelease(data);
        if (!matches) return 14;
#endif
        value.playback_state = 2;
        if (!lw_media_update(raw, &value) || center.playbackState != MPNowPlayingPlaybackStatePaused ||
            center.nowPlayingInfo[MPMediaItemPropertyArtwork] != artwork || [center.nowPlayingInfo[MPNowPlayingInfoPropertyPlaybackRate] doubleValue] != 0) return 15;
        value.duration = NAN;
        if (lw_media_update(raw, &value) || [center.nowPlayingInfo[MPMediaItemPropertyPlaybackDuration] doubleValue] != 20) return 21;
        value.duration = 20;
        value.generation++;
        if (!lw_media_update(raw, &value) || center.nowPlayingInfo[MPMediaItemPropertyArtwork]) return 16;
        value.duration = 0;
        if (!lw_media_update(raw, &value) || remote.items[6].enabled || remote.items[6].handler((MPRemoteCommandEvent *)event) == MPRemoteCommandHandlerStatusSuccess) return 17;
        value.song = 0; value.has_queue = 0; value.playback_state = 3;
        if (!lw_media_update(raw, &value) || center.nowPlayingInfo || center.playbackState != MPNowPlayingPlaybackStateStopped) return 18;
        MPRemoteCommandHandlerStatus (^late)(MPRemoteCommandEvent *) = remote.items[0].handler;
        [media close];
        for (LWTestMediaCommand *registered in remote.items) if (registered.enabled || registered.removed != 1 || registered.handler) return 19;
        media = nil;
        if (late(nil) != MPRemoteCommandHandlerStatusCommandFailed) return 20;
        return 0;
    }
}
int lw_media_test_enqueue(void *raw, int kind, double seconds) {
    return (int)[(__bridge LWMedia *)raw enqueue:kind seconds:seconds];
}
void lw_media_test_state(void *raw, LWMediaState *state) { *state = ((__bridge LWMedia *)raw)->state; }
#endif
