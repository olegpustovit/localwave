#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdatomic.h>
#include "platform.h"

@interface LWImportPreferences : NSObject
@property(nonatomic, strong) NSSet<NSString *> *pinned;
@property(nonatomic, strong) NSDictionary *versions;
@end
@implementation LWImportPreferences
@end

@interface LWCacheProtection : NSObject
@property(nonatomic, strong) NSSet<NSString *> *pinned;
@property(nonatomic, strong) NSString *active;
@property(nonatomic, strong) NSString *activePath;
@property(nonatomic, strong) NSSet<NSString *> *queued;
@property(nonatomic) uint64_t limit;
@end
@implementation LWCacheProtection
@end

static NSRecursiveLock *cacheLock(void) {
    static NSRecursiveLock *lock;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ lock = [NSRecursiveLock new]; });
    return lock;
}
static _Atomic uint64_t cacheGeneration;
uint64_t lw_cache_generation(void) { return atomic_load(&cacheGeneration); }
void lw_cache_lock(void) { [cacheLock() lock]; }
void lw_cache_unlock(void) { [cacheLock() unlock]; }

void *lw_preferences_cache_snapshot(void *raw) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        LWCacheProtection *protection = [LWCacheProtection new];
        protection.pinned = [NSSet setWithArray:[defaults stringArrayForKey:@"lwPinnedSongs"] ?: @[]];
        protection.active = [[defaults stringForKey:@"lwActiveSongID"] copy];
        protection.activePath = [[defaults stringForKey:@"lwActiveCachePath"] copy];
        protection.queued = [NSSet setWithArray:[defaults stringArrayForKey:@"lwQueuedCachePaths"] ?: @[]];
        NSInteger gb = [defaults integerForKey:@"lwCacheGB"];
        if (![@[@2, @4, @8, @16, @32] containsObject:@(gb)]) gb = 8;
        protection.limit = (uint64_t)gb * 1073741824;
        return (__bridge_retained void *)protection;
    }
}
int lw_preferences_cache_owner_protected(void *raw, int64_t song) {
    @autoreleasepool {
        LWCacheProtection *protection = (__bridge LWCacheProtection *)raw;
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)song];
        return [protection.pinned containsObject:key] || [protection.active isEqual:key];
    }
}
int lw_preferences_cache_path_protected(void *raw, const char *path) {
    @autoreleasepool {
        LWCacheProtection *protection = (__bridge LWCacheProtection *)raw;
        NSString *key = [NSString stringWithUTF8String:path];
        return [protection.activePath isEqual:key] || [protection.queued containsObject:key];
    }
}
uint64_t lw_preferences_cache_limit(void *raw) { return ((__bridge LWCacheProtection *)raw).limit; }
void lw_preferences_set_cache_limit(void *raw, int64_t gigabytes) {
    @autoreleasepool { [(__bridge NSUserDefaults *)raw setInteger:(NSInteger)gigabytes forKey:@"lwCacheGB"]; atomic_fetch_add(&cacheGeneration, 1); }
}
void lw_preferences_set_cache_protection(void *raw, int64_t song, const char *active, const char *const *queued, size_t count) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        if (song >= 0) [defaults setObject:[NSString stringWithFormat:@"%lld", (long long)song] forKey:@"lwActiveSongID"]; else [defaults removeObjectForKey:@"lwActiveSongID"];
        if (active) [defaults setObject:[NSString stringWithUTF8String:active] forKey:@"lwActiveCachePath"]; else [defaults removeObjectForKey:@"lwActiveCachePath"];
        NSMutableArray *paths = [NSMutableArray arrayWithCapacity:count];
        for (size_t i = 0; i < count; i++) [paths addObject:[NSString stringWithUTF8String:queued[i]]];
        [defaults setObject:paths forKey:@"lwQueuedCachePaths"];
        atomic_fetch_add(&cacheGeneration, 1);
    }
}

void *lw_preferences_open(const char *suite) {
    @autoreleasepool {
        NSUserDefaults *defaults = suite ? [[NSUserDefaults alloc] initWithSuiteName:[NSString stringWithUTF8String:suite]] : NSUserDefaults.standardUserDefaults;
        return (__bridge_retained void *)defaults;
    }
}

int lw_preferences_no_network(void *raw) {
    @autoreleasepool { return [(__bridge NSUserDefaults *)raw boolForKey:@"lwSimulateNoNetwork"]; }
}
void lw_preferences_set_no_network(void *raw, int disabled) {
    @autoreleasepool { [(__bridge NSUserDefaults *)raw setBool:disabled forKey:@"lwSimulateNoNetwork"]; }
}

int lw_preferences_bookmark(void *raw, int64_t hash, LWBytes *bytes) {
    @autoreleasepool {
        NSData *data = [(__bridge NSUserDefaults *)raw dataForKey:[NSString stringWithFormat:@"%lld", (long long)hash]];
        if (!data) return 0;
        if (data.length > 64 * 1024 * 1024) return -1;
        bytes->data = malloc(MAX(data.length, 1));
        if (!bytes->data) return -1;
        bytes->len = data.length;
        if (data.length) memcpy(bytes->data, data.bytes, data.length);
        return 1;
    }
}

void lw_preferences_set_bookmark(void *raw, int64_t hash, const unsigned char *data, size_t len) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)hash];
        if (data) [defaults setObject:[NSData dataWithBytes:data length:len] forKey:key];
        else [defaults removeObjectForKey:key];
    }
}

void lw_preferences_release(void *raw) {
    @autoreleasepool { id defaults = (__bridge_transfer id)raw; (void)defaults; }
}

void *lw_preferences_import_snapshot(void *raw) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        LWImportPreferences *snapshot = [LWImportPreferences new];
        snapshot.pinned = [NSSet setWithArray:[defaults stringArrayForKey:@"lwPinnedSongs"] ?: @[]];
        snapshot.versions = [[defaults dictionaryForKey:@"lwCacheSourceVersions"] copy] ?: @{};
        return (__bridge_retained void *)snapshot;
    }
}

int lw_preferences_pinned(void *raw, int64_t song) {
    @autoreleasepool {
        return [((__bridge LWImportPreferences *)raw).pinned containsObject:[NSString stringWithFormat:@"%lld", (long long)song]];
    }
}

int lw_preferences_cache_matches(void *raw, int64_t song, const char *cachePath, double modified, int64_t size) {
    @autoreleasepool {
        id record = ((__bridge LWImportPreferences *)raw).versions[[NSString stringWithFormat:@"%lld", (long long)song]];
        if (![record isKindOfClass:NSDictionary.class] || ![record[@"cachePath"] isEqual:[NSString stringWithUTF8String:cachePath]]) return -1;
        id recordedDate = record[@"modified"], recordedSize = record[@"size"];
        BOOL hasDate = [recordedDate isKindOfClass:NSNumber.class], hasSize = [recordedSize isKindOfClass:NSNumber.class];
        return (isfinite(modified) ? hasDate && [recordedDate doubleValue] == modified : !hasDate) && (size >= 0 ? hasSize && [recordedSize longLongValue] == size : !hasSize);
    }
}

void lw_preferences_set_pinned(void *raw, int64_t song, int pinned) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        NSMutableSet *songs = [NSMutableSet setWithArray:[defaults stringArrayForKey:@"lwPinnedSongs"] ?: @[]];
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)song];
        if (pinned) [songs addObject:key]; else [songs removeObject:key];
        [defaults setObject:songs.allObjects forKey:@"lwPinnedSongs"];
        atomic_fetch_add(&cacheGeneration, 1);
    }
}

void lw_preferences_set_cache_version(void *raw, int64_t song, const char *cachePath, double modified, int64_t size) {
    @autoreleasepool {
        NSUserDefaults *defaults = (__bridge NSUserDefaults *)raw;
        NSMutableDictionary *versions = [[defaults dictionaryForKey:@"lwCacheSourceVersions"] mutableCopy] ?: [NSMutableDictionary new];
        NSString *key = [NSString stringWithFormat:@"%lld", (long long)song];
        if (!cachePath) [versions removeObjectForKey:key];
        else {
            NSMutableDictionary *record = [NSMutableDictionary dictionaryWithObject:[NSString stringWithUTF8String:cachePath] forKey:@"cachePath"];
            if (isfinite(modified)) record[@"modified"] = @(modified);
            if (size >= 0) record[@"size"] = @(size);
            versions[key] = record;
        }
        [defaults setObject:versions forKey:@"lwCacheSourceVersions"];
    }
}

char *lw_app_path(unsigned int slot) {
    @autoreleasepool {
        NSFileManager *manager = NSFileManager.defaultManager;
        NSURL *url = nil;
        if (slot == 0) url = [manager URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask].firstObject;
        else if (slot == 1) url = [manager URLsForDirectory:NSCachesDirectory inDomains:NSUserDomainMask].firstObject;
        else if (slot == 2) url = manager.temporaryDirectory;
        else if (slot == 3) {
            NSString *testID = NSProcessInfo.processInfo.environment[@"LOCALWAVE_UI_TEST_ID"];
            if (![[NSUUID alloc] initWithUUIDString:testID]) return NULL;
            url = [[manager URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask].firstObject URLByAppendingPathComponent:[NSString stringWithFormat:@"uitest-%@.sqlite", testID]];
        }
        return url ? strdup(url.path.UTF8String) : NULL;
    }
}

char *lw_file_path(const char *value) {
    @autoreleasepool {
        NSString *text = value ? [[NSString stringWithUTF8String:value] stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet] : nil;
        if (!text.length) return NULL;
        NSURL *url = nil;
        if ([text hasPrefix:@"/"] || [text hasPrefix:@"~"]) {
            text = text.stringByExpandingTildeInPath;
            if (!text.isAbsolutePath) return NULL;
            url = [NSURL fileURLWithPath:text isDirectory:NO];
        } else {
            url = [NSURL URLWithString:text];
            if (!url.isFileURL || (url.host.length && ![url.host isEqualToString:@"localhost"]) || url.query || url.fragment) return NULL;
        }
        if ([url.absoluteString rangeOfString:@"%00" options:NSCaseInsensitiveSearch].location != NSNotFound) return NULL;
        NSString *path = url.URLByStandardizingPath.path;
        if (!path.isAbsolutePath || [path rangeOfString:[NSString stringWithCharacters:(unichar[]){0} length:1]].location != NSNotFound) return NULL;
        return strdup(path.UTF8String);
    }
}
