#import <Foundation/Foundation.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

@interface LWPlayerPreferences : NSObject
@property(nonatomic, strong) NSArray<NSNumber *> *queue;
@property(nonatomic) int64_t index;
@property(nonatomic) double volume;
@property(nonatomic, strong) NSData *context;
@end
@implementation LWPlayerPreferences
@end

void *lw_preferences_for_ui_test(const char *testID) {
    @autoreleasepool {
        NSUUID *uuid = testID ? [[NSUUID alloc] initWithUUIDString:[NSString stringWithUTF8String:testID]] : nil;
        NSUserDefaults *defaults = uuid ? [[NSUserDefaults alloc] initWithSuiteName:[@"com.olegpustovit.localwave.ui-test." stringByAppendingString:uuid.UUIDString]] : NSUserDefaults.standardUserDefaults;
        return (__bridge_retained void *)defaults;
    }
}
int lw_preferences_is_standard(void *raw) { return (__bridge NSUserDefaults *)raw == NSUserDefaults.standardUserDefaults; }
void lw_preferences_settings(void *raw, LWSettings *settings) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        settings->download_ahead = ![defaults objectForKey:@"lwDownloadAhead"] || [defaults boolForKey:@"lwDownloadAhead"];
        settings->show_non_audio = [defaults boolForKey:@"lwShowNonAudio"];
        settings->no_network = [defaults boolForKey:@"lwSimulateNoNetwork"];
        settings->reduce_motion = [defaults boolForKey:@"lwReduceMotion"];
        NSString *appearance = [defaults stringForKey:@"lwAppearance"], *signal = [defaults stringForKey:@"lwSignal"];
        settings->appearance = [appearance isEqual:@"light"] ? 1 : [appearance isEqual:@"dark"] ? 2 : 0;
        settings->signal = [signal isEqual:@"red"] ? 1 : [signal isEqual:@"green"] ? 2 : [signal isEqual:@"mono"] ? 3 : 0;
        int64_t gb = [defaults integerForKey:@"lwCacheGB"];
        settings->cache_gb = [@[@2, @4, @8, @16, @32] containsObject:@(gb)] ? gb : 8;
    }
}
void lw_preferences_save_settings(void *raw, const LWSettings *settings) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        [defaults setBool:settings->download_ahead forKey:@"lwDownloadAhead"];
        [defaults setBool:settings->show_non_audio forKey:@"lwShowNonAudio"];
        lw_preferences_set_no_network(raw, settings->no_network);
        [defaults setBool:settings->reduce_motion forKey:@"lwReduceMotion"];
        [defaults setObject:@[@"system", @"light", @"dark"][settings->appearance] forKey:@"lwAppearance"];
        [defaults setObject:@[@"blue", @"red", @"green", @"mono"][settings->signal] forKey:@"lwSignal"];
        lw_preferences_set_cache_limit(raw, settings->cache_gb);
    }
}

void *lw_preferences_player(void *raw, char **error) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        LWPlayerPreferences *state = [LWPlayerPreferences new];
        id index = [defaults objectForKey:@"currentQueueIndex"];
        NSArray *queue = [defaults arrayForKey:@"currentQueue"];
        BOOL valid = [index isKindOfClass:NSNumber.class] && CFGetTypeID((__bridge CFTypeRef)index) != CFBooleanGetTypeID() && !CFNumberIsFloatType((__bridge CFNumberRef)index);
        if (queue.count > 1000000) { if (error) *error = strdup("Persisted queue exceeds one million entries"); return NULL; }
        for (id song in queue) {
            if (![song isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)song) == CFBooleanGetTypeID() || CFNumberIsFloatType((__bridge CFNumberRef)song)) { valid = NO; break; }
        }
        state.queue = valid ? [queue copy] ?: @[] : @[];
        state.index = valid ? [index longLongValue] : 0;
        state.volume = [defaults objectForKey:@"playerVolume"] ? [defaults doubleForKey:@"playerVolume"] : NAN;
        state.context = [defaults dataForKey:@"lwPlaybackContext"];
        if (state.context.length > 64 * 1024 * 1024) { if (error) *error = strdup("Playback context exceeds its memory limit"); return NULL; }
        return (__bridge_retained void *)state;
    }
}
size_t lw_player_preferences_count(void *raw) { return ((__bridge LWPlayerPreferences *)raw).queue.count; }
int64_t lw_player_preferences_song(void *raw, size_t index) { return [((__bridge LWPlayerPreferences *)raw).queue[index] longLongValue]; }
int64_t lw_player_preferences_index(void *raw) { return ((__bridge LWPlayerPreferences *)raw).index; }
double lw_player_preferences_volume(void *raw) { return ((__bridge LWPlayerPreferences *)raw).volume; }
const unsigned char *lw_player_preferences_context(void *raw, size_t *len, int *present) {
    NSData *data = ((__bridge LWPlayerPreferences *)raw).context;
    *len = data.length; *present = data != nil;
    return data.bytes;
}
void lw_preferences_save_player(void *raw, const int64_t *queue, size_t count, int64_t index, double volume, const unsigned char *context, size_t contextLen) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        NSMutableArray *songs = [NSMutableArray arrayWithCapacity:count];
        for (size_t i = 0; i < count; i++) [songs addObject:@(queue[i])];
        [defaults setObject:songs forKey:@"currentQueue"];
        [defaults setInteger:(NSInteger)index forKey:@"currentQueueIndex"];
        if (isfinite(volume)) [defaults setFloat:(float)volume forKey:@"playerVolume"]; else [defaults removeObjectForKey:@"playerVolume"];
        if (context) [defaults setObject:[NSData dataWithBytes:context length:contextLen] forKey:@"lwPlaybackContext"]; else [defaults removeObjectForKey:@"lwPlaybackContext"];
    }
}
void lw_preferences_save_player_index(void *raw, int64_t index) {
    @autoreleasepool { [(__bridge NSUserDefaults *)raw setInteger:(NSInteger)index forKey:@"currentQueueIndex"]; }
}
void lw_preferences_save_player_volume(void *raw, float volume) {
    @autoreleasepool { [(__bridge NSUserDefaults *)raw setFloat:volume forKey:@"playerVolume"]; }
}
