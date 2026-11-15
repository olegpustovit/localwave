#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

void *lw_collation_create(const unsigned char *bytes, size_t len) {
    if (!bytes && len) return NULL;
    @autoreleasepool {
        NSString *key = [[NSString alloc] initWithBytes:bytes ?: (const unsigned char *)"" length:len encoding:NSUTF8StringEncoding];
        return key ? (void *)CFBridgingRetain(key) : NULL;
    }
}
int32_t lw_collation_compare(void *left, void *right) {
    @autoreleasepool { return (int32_t)[(__bridge NSString *)left localizedStandardCompare:(__bridge NSString *)right]; }
}
void lw_collation_release(void *key) { if (key) CFRelease(key); }
int32_t lw_collation_contains(void *needle, const unsigned char *bytes, size_t len) {
    if (!needle || (!bytes && len)) return 0;
    @autoreleasepool {
        NSString *value = [[NSString alloc] initWithBytes:bytes ?: (const unsigned char *)"" length:len encoding:NSUTF8StringEncoding];
        return value && [value localizedStandardContainsString:(__bridge NSString *)needle];
    }
}
int32_t lw_collation_contains_case_insensitive(void *needle, const unsigned char *bytes, size_t len) {
    if (!needle || (!bytes && len)) return 0;
    @autoreleasepool {
        NSString *value = [[NSString alloc] initWithBytes:bytes ?: (const unsigned char *)"" length:len encoding:NSUTF8StringEncoding];
        return value && [value localizedCaseInsensitiveContainsString:(__bridge NSString *)needle];
    }
}

char *lw_normalized_file_url(const char *path) {
    @autoreleasepool {
        NSString *text = path ? [NSString stringWithUTF8String:path] : nil;
        if (!text || !text.isAbsolutePath) return NULL;
        NSURL *url = [NSURL fileURLWithPath:text isDirectory:NO].standardizedURL;
        NSURLComponents *components = [NSURLComponents componentsWithURL:url resolvingAgainstBaseURL:NO];
        if (!components) return NULL;
        NSString *normalized = components.path;
        while (normalized.length > 1 && [normalized hasSuffix:@"/"]) {
            normalized = [normalized substringToIndex:normalized.length - 1];
        }
        components.path = normalized;
        const char *result = components.URL.absoluteString.UTF8String;
        return result ? strdup(result) : NULL;
    }
}

void lw_free_string(char *text) { free(text); }

int lw_is_main_thread(void) { return NSThread.isMainThread; }
