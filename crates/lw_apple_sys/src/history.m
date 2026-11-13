#import <Foundation/Foundation.h>
#include <fcntl.h>
#include <math.h>
#include <sys/stat.h>
#include <unistd.h>
#include "platform.h"

@interface LWPlaybackHistory : NSObject
@property(nonatomic, strong) NSDictionary *counts, *dates;
@end
@implementation LWPlaybackHistory
@end

static NSLock *historyLock(void) {
    static NSLock *lock;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ lock = [NSLock new]; });
    return lock;
}

void *lw_preferences_history(void *raw) {
    @autoreleasepool {
        [historyLock() lock];
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        LWPlaybackHistory *history = [LWPlaybackHistory new];
        history.counts = [defaults dictionaryForKey:@"lwPlayCounts"] ?: @{};
        history.dates = [defaults dictionaryForKey:@"lwLastPlayed"] ?: @{};
        [historyLock() unlock];
        return (__bridge_retained void *)history;
    }
}
void lw_history_get(void *raw, int64_t song, int64_t *plays, double *lastPlayed) {
    @autoreleasepool {
        LWPlaybackHistory *history = (__bridge LWPlaybackHistory *)raw;
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)song];
        id count = history.counts[key], date = history.dates[key];
        *plays = [count isKindOfClass:NSNumber.class] ? MAX(0, [count longLongValue]) : 0;
        *lastPlayed = [date isKindOfClass:NSNumber.class] ? [date doubleValue] : NAN;
    }
}
void lw_preferences_record_playback(void *raw, int64_t song, double now) {
    @autoreleasepool {
        [historyLock() lock];
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        NSMutableDictionary *counts = [[defaults dictionaryForKey:@"lwPlayCounts"] mutableCopy] ?: [NSMutableDictionary new];
        NSMutableDictionary *dates = [[defaults dictionaryForKey:@"lwLastPlayed"] mutableCopy] ?: [NSMutableDictionary new];
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)song];
        id value = counts[key];
        int64_t count = [value isKindOfClass:NSNumber.class] ? MAX(0, [value longLongValue]) : 0;
        counts[key] = @(count == INT64_MAX ? count : count + 1);
        dates[key] = @(now);
        [defaults setObject:counts forKey:@"lwPlayCounts"];
        [defaults setObject:dates forKey:@"lwLastPlayed"];
        [historyLock() unlock];
    }
}

// Open relative to the real cache directory so replacing a leaf with a symlink cannot touch an original.
int lw_touch_cache_file(const char *path, const char *root, double now) {
    @autoreleasepool {
        NSString *file = [NSString stringWithUTF8String:path], *directory = [NSString stringWithUTF8String:root];
        if (![file.stringByDeletingLastPathComponent isEqual:directory] || !isfinite(now) || now < 0 || now >= (double)INT64_MAX) return 0;
        int parent = open(root, O_RDONLY | O_DIRECTORY | O_CLOEXEC | O_NOFOLLOW);
        if (parent < 0) return 0;
        int fd = openat(parent, file.lastPathComponent.fileSystemRepresentation, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK);
        close(parent);
        if (fd < 0) return 0;
        struct stat status;
        struct timespec times[2] = {{ .tv_nsec = UTIME_OMIT }, { .tv_sec = (time_t)now, .tv_nsec = (long)((now - floor(now)) * 1e9) }};
        int ok = fstat(fd, &status) == 0 && S_ISREG(status.st_mode) && futimens(fd, times) == 0;
        close(fd);
        return ok;
    }
}
