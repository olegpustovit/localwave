#import <Foundation/Foundation.h>
#import <CommonCrypto/CommonDigest.h>
#import <ImageIO/ImageIO.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

static void artwork_error(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Could not decode cover art").UTF8String);
}

void *lw_artwork_thumbnail(const char *path, uint32_t maxPixels, uint32_t *width, uint32_t *height, char **error) {
    @autoreleasepool {
        NSString *name = path ? [NSString stringWithUTF8String:path] : nil;
        if (NSThread.isMainThread || !name.isAbsolutePath || !width || !height || !maxPixels || maxPixels > 1024) {
            artwork_error(error, @"Cover thumbnails require an absolute file path, a worker and 1...1024 pixels"); return NULL;
        }
        LWFileInfo before;
        if (!lw_file_probe(path, 0, &before, error)) return NULL;
        if (before.availability == 2 || (before.provider && before.availability != 1)) {
            artwork_error(error, @"Cover art is not confirmed local"); return NULL;
        }
        if (before.size > 64 * 1024 * 1024) { artwork_error(error, @"Cover art exceeds 64 MiB"); return NULL; }
        NSURL *url = [NSURL fileURLWithPath:name];
        __block CGImageRef image = NULL;
        __block NSError *readError = nil;
        NSError *coordinationError = nil;
        NSFileCoordinator *coordinator = [NSFileCoordinator new];
        [coordinator coordinateReadingItemAtURL:url options:0 error:&coordinationError byAccessor:^(NSURL *readable) {
            LWFileInfo coordinated;
            char *probeError = NULL;
            if (!lw_file_probe(readable.path.UTF8String, 0, &coordinated, &probeError) || coordinated.availability == 2 || (coordinated.provider && coordinated.availability != 1)) {
                free(probeError); return;
            }
            NSNumber *regular = nil;
            if (![readable getResourceValue:&regular forKey:NSURLIsRegularFileKey error:&readError] || !regular.boolValue) return;
            NSFileHandle *file = [NSFileHandle fileHandleForReadingFromURL:readable error:&readError];
            if (!file) return;
            NSUInteger limit = (NSUInteger)(coordinated.size >= 0 && coordinated.size < 64 * 1024 * 1024 ? coordinated.size + 1 : 64 * 1024 * 1024 + 1);
            NSData *data = [file readDataUpToLength:limit error:&readError];
            NSError *closeError = nil;
            [file closeAndReturnError:&closeError];
            if (!readError) readError = closeError;
            if (readError || !data.length || data.length > 64 * 1024 * 1024 || (coordinated.size >= 0 && data.length != (uint64_t)coordinated.size)) return;
            CGImageSourceRef source = CGImageSourceCreateWithData((__bridge CFDataRef)data, (__bridge CFDictionaryRef)@{ (__bridge NSString *)kCGImageSourceShouldCache: @NO });
            if (!source) return;
            NSDictionary *properties = CFBridgingRelease(CGImageSourceCopyPropertiesAtIndex(source, 0, NULL));
            NSNumber *sourceWidth = properties[(__bridge NSString *)kCGImagePropertyPixelWidth];
            NSNumber *sourceHeight = properties[(__bridge NSString *)kCGImagePropertyPixelHeight];
            uint64_t w = [sourceWidth isKindOfClass:NSNumber.class] ? sourceWidth.unsignedLongLongValue : 0;
            uint64_t h = [sourceHeight isKindOfClass:NSNumber.class] ? sourceHeight.unsignedLongLongValue : 0;
            if (!w || !h || w > 32768 || h > 32768 || w * h > 64 * 1024 * 1024) {
                CFRelease(source);
                readError = [NSError errorWithDomain:@"LocalWave.Artwork" code:1 userInfo:@{NSLocalizedDescriptionKey: @"Cover art exceeds its source pixel bound"}];
                return;
            }
            image = CGImageSourceCreateThumbnailAtIndex(source, 0, (__bridge CFDictionaryRef)@{
                (__bridge NSString *)kCGImageSourceCreateThumbnailFromImageAlways: @YES,
                (__bridge NSString *)kCGImageSourceCreateThumbnailWithTransform: @YES,
                (__bridge NSString *)kCGImageSourceThumbnailMaxPixelSize: @(maxPixels),
                (__bridge NSString *)kCGImageSourceShouldCacheImmediately: @YES
            });
            CFRelease(source);
        }];
        if (!image || readError || coordinationError) {
            if (image) CGImageRelease(image);
            artwork_error(error, (readError ?: coordinationError).localizedDescription); return NULL;
        }
        LWFileInfo after;
        if (!lw_file_probe(path, 0, &after, error)) { CGImageRelease(image); return NULL; }
        if (before.size != after.size || (isfinite(before.modified) && isfinite(after.modified) && before.modified != after.modified)) {
            CGImageRelease(image); artwork_error(error, @"Cover art changed during decoding; retry it"); return NULL;
        }
        size_t w = CGImageGetWidth(image), h = CGImageGetHeight(image);
        if (!w || !h || w > maxPixels || h > maxPixels) { CGImageRelease(image); artwork_error(error, @"Invalid cover thumbnail dimensions"); return NULL; }
        *width = (uint32_t)w; *height = (uint32_t)h;
        return (void *)image;
    }
}

int lw_artwork_pixels(void *raw, unsigned char *pixels, size_t length) {
    @autoreleasepool {
        if (NSThread.isMainThread || !raw || !pixels) return 0;
        CGImageRef image = (CGImageRef)raw;
        size_t w = CGImageGetWidth(image), h = CGImageGetHeight(image);
        if (!w || !h || w > 1024 || h > 1024 || length != w * h * 4) return 0;
        CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
        CGContextRef context = space ? CGBitmapContextCreate(pixels, w, h, 8, w * 4, space, kCGBitmapByteOrder32Big | kCGImageAlphaPremultipliedLast) : NULL;
        if (space) CGColorSpaceRelease(space);
        if (!context) return 0;
        CGContextSetBlendMode(context, kCGBlendModeCopy);
        CGContextDrawImage(context, CGRectMake(0, 0, w, h), image);
        CGContextRelease(context);
        // PXUI samples straight-alpha images; Core Graphics writes premultiplied RGBA.
        for (size_t i = 0; i < length; i += 4) {
            unsigned alpha = pixels[i + 3];
            if (alpha == 255) continue;
            for (size_t c = 0; c < 3; ++c) pixels[i + c] = alpha ? (unsigned char)MIN(255u, ((unsigned)pixels[i + c] * 255u + alpha / 2u) / alpha) : 0;
        }
        return 1;
    }
}
void lw_artwork_thumbnail_release(void *raw) { if (raw) CGImageRelease((CGImageRef)raw); }

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
