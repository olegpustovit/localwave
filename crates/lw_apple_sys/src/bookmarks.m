#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

@interface LWBookmarkAccess : NSObject
@property(nonatomic, strong) NSURL *url;
@property(nonatomic) BOOL scoped;
@end
@implementation LWBookmarkAccess
- (void)dealloc { if (_scoped) [_url stopAccessingSecurityScopedResource]; }
@end

static int fail(char **error, NSString *text) {
    if (error) *error = strdup((text ?: @"Folder access failed").UTF8String);
    return 0;
}

static int copy_data(NSData *data, LWBytes *out, char **error) {
    out->data = malloc(MAX((size_t)1, data.length));
    if (!out->data) return fail(error, @"Could not allocate bookmark data");
    out->len = data.length;
    if (out->len) memcpy(out->data, data.bytes, out->len);
    return 1;
}

int lw_bookmark_create(const char *path, LWBytes *bookmark, char **error) {
    @autoreleasepool {
        NSString *text = path ? [NSString stringWithUTF8String:path] : nil;
        if (!text || !text.isAbsolutePath) return fail(error, @"Expected an absolute file path");
        NSURL *url = [NSURL fileURLWithPath:text];
        NSError *failure = nil;
        NSData *data = [url bookmarkDataWithOptions:NSURLBookmarkCreationWithSecurityScope includingResourceValuesForKeys:nil relativeToURL:nil error:&failure];
        if (!data) return fail(error, failure.localizedDescription);
        return copy_data(data, bookmark, error);
    }
}

void *lw_bookmark_resolve(const unsigned char *bookmark, size_t len, LWBytes *renewed, char **error) {
    @autoreleasepool {
        if (!bookmark || !len) { fail(error, @"Bookmark is empty"); return NULL; }
        NSData *data = [NSData dataWithBytes:bookmark length:len];
        BOOL stale = NO;
        NSError *failure = nil;
        NSURL *url = [NSURL URLByResolvingBookmarkData:data options:NSURLBookmarkResolutionWithSecurityScope | NSURLBookmarkResolutionWithoutUI relativeToURL:nil bookmarkDataIsStale:&stale error:&failure];
        if (!url) { fail(error, failure.localizedDescription); return NULL; }
        if (!url.isFileURL || !url.path.isAbsolutePath) { fail(error, @"Bookmark does not resolve to an absolute file URL"); return NULL; }
        LWBookmarkAccess *access = [LWBookmarkAccess new];
        access.url = url;
        access.scoped = [url startAccessingSecurityScopedResource];
        if (!access.scoped && ![[NSFileManager defaultManager] isReadableFileAtPath:url.path]) {
            fail(error, @"Folder access has expired. Reconnect its source folder.");
            return NULL;
        }
        if (stale) {
            NSData *replacement = [url bookmarkDataWithOptions:NSURLBookmarkCreationWithSecurityScope includingResourceValuesForKeys:nil relativeToURL:nil error:&failure];
            if (!replacement || !copy_data(replacement, renewed, error)) {
                if (!replacement) fail(error, failure.localizedDescription);
                return NULL;
            }
        }
        return (__bridge_retained void *)access;
    }
}

char *lw_bookmark_path(void *raw) {
    @autoreleasepool {
        LWBookmarkAccess *access = (__bridge LWBookmarkAccess *)raw;
        return strdup(access.url.path.UTF8String);
    }
}

void *lw_bookmark_url(void *raw) {
    LWBookmarkAccess *access = (__bridge LWBookmarkAccess *)raw;
    return (__bridge void *)access.url;
}

void lw_bookmark_release(void *raw) {
    @autoreleasepool {
        id access = (__bridge_transfer id)raw;
        (void)access;
    }
}
