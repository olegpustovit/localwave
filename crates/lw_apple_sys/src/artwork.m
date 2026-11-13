#import <Foundation/Foundation.h>
#import <CommonCrypto/CommonDigest.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

int lw_artwork_store(const char *documents, const unsigned char *bytes, size_t length, char **relative, char **error) {
    @autoreleasepool {
        if (!length) return 0;
        NSString *root = documents ? [NSString stringWithUTF8String:documents] : nil;
        if (!root.isAbsolutePath || length > 64 * 1024 * 1024) {
            if (error) *error = strdup("Invalid artwork location or size");
            return -1;
        }
        // Match the shipping Mac importer's PNG/JPEG signature check without decoding on import.
        BOOL png = length >= 4 && !memcmp(bytes, "\x89PNG", 4);
        BOOL jpeg = length >= 4 && !memcmp(bytes, "\xff\xd8\xff", 3);
        if (!png && !jpeg) return 0;
        unsigned char digest[CC_SHA256_DIGEST_LENGTH];
        CC_SHA256(bytes, (CC_LONG)length, digest);
        char hex[65];
        for (size_t i = 0; i < sizeof(digest); i++) { hex[i * 2] = "0123456789abcdef"[digest[i] >> 4]; hex[i * 2 + 1] = "0123456789abcdef"[digest[i] & 15]; }
        hex[64] = 0;
        NSString *path = [NSString stringWithFormat:@"CoverArt/cover-%s.%@", hex, png ? @"png" : @"jpg"];
        NSURL *destination = [[NSURL fileURLWithPath:root isDirectory:YES] URLByAppendingPathComponent:path];
        NSError *failure = nil;
        NSFileManager *manager = NSFileManager.defaultManager;
        if (![manager createDirectoryAtURL:destination.URLByDeletingLastPathComponent withIntermediateDirectories:YES attributes:nil error:&failure] ||
            (![manager fileExistsAtPath:destination.path] && ![[NSData dataWithBytesNoCopy:(void *)bytes length:length freeWhenDone:NO] writeToURL:destination options:NSDataWritingAtomic error:&failure])) {
            if (error) *error = strdup((failure.localizedDescription ?: @"Could not save cover art").UTF8String);
            return -1;
        }
        *relative = strdup(path.UTF8String);
        if (!*relative) { if (error) *error = strdup("Could not encode cover art path"); return -1; }
        return 1;
    }
}
