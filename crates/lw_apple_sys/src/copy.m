#import <Foundation/Foundation.h>
#import <CommonCrypto/CommonDigest.h>
#include <errno.h>
#include <fcntl.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include "platform.h"

static int fail(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Offline copy failed").UTF8String);
    return 0;
}
static int system_fail(char **error) { return fail(error, [NSString stringWithUTF8String:strerror(errno)]); }
static int interrupted(char **error) { fail(error, @"Offline copy was cancelled"); return -1; }
static int require_local(const char *source, char **error) {
    LWFileInfo info;
    if (!lw_file_probe(source, 0, &info, error)) return 0;
    if (info.availability == 1) return 1;
    fail(error, @"The audio file is not confirmed local while network simulation is enabled");
    return -2;
}
static int network_check(const char *source, void *preferences, char **error) {
    return lw_preferences_no_network(preferences) ? require_local(source, error) : 1;
}
static ssize_t chunk(int fd, unsigned char *buffer, size_t capacity) {
    size_t total = 0;
    while (total < capacity) {
        ssize_t count = read(fd, buffer + total, capacity - total);
        if (count < 0) { if (errno == EINTR) continue; return -1; }
        if (!count) break;
        total += (size_t)count;
    }
    return (ssize_t)total;
}
static BOOL identical(const char *left, const char *right, int (*cancelled)(void *), void *user) {
    int a = open(left, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK);
    int b = open(right, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK);
    struct stat sa, sb;
    BOOL same = a >= 0 && b >= 0 && !fstat(a, &sa) && !fstat(b, &sb) && S_ISREG(sa.st_mode) && S_ISREG(sb.st_mode) && sa.st_size == sb.st_size;
    unsigned char first[64 * 1024], second[64 * 1024];
    while (same) {
        if (cancelled(user)) { same = NO; break; }
        ssize_t size = chunk(a, first, sizeof(first)), other = chunk(b, second, sizeof(second));
        if (size < 0 || size != other || memcmp(first, second, (size_t)size)) { same = NO; break; }
        if (!size) break;
    }
    if (a >= 0) close(a);
    if (b >= 0) close(b);
    return same;
}

static int copy_stream(const char *source, NSURL *readable, NSURL *directory, BOOL preserveName, void *preferences, int (*cancelled)(void *), void *user, int (*progress)(void *, uint64_t), void *progressUser, LWCopyResult *result, char **error) {
    LWFileInfo before;
    if (!lw_file_probe(source, 0, &before, error)) return 0;
    int status = network_check(source, preferences, error);
    if (status != 1) return status;
    if (cancelled(user)) return interrupted(error);
    NSError *issue = nil;
    if (![NSFileManager.defaultManager createDirectoryAtURL:directory withIntermediateDirectories:YES attributes:nil error:&issue]) return fail(error, issue.localizedDescription);
    NSURL *temporary = [directory URLByAppendingPathComponent:[NSUUID.UUID.UUIDString stringByAppendingString:@".partial"]];
    int input = -1, output = -1;
    @try {
        input = open(readable.fileSystemRepresentation, O_RDONLY | O_CLOEXEC | O_NONBLOCK);
        if (input < 0) return system_fail(error);
        struct stat original;
        if (fstat(input, &original)) return system_fail(error);
        if (!S_ISREG(original.st_mode)) return fail(error, @"Offline audio source is not a regular file");
        output = open(temporary.fileSystemRepresentation, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
        if (output < 0) return system_fail(error);
        BOOL checkedSimulation = NO;
        CC_SHA256_CTX hash;
        if (!preserveName) CC_SHA256_Init(&hash);
        uint64_t total = 0;
        unsigned char buffer[256 * 1024];
        for (;;) {
            if (cancelled(user)) return interrupted(error);
            if (!checkedSimulation && lw_preferences_no_network(preferences)) {
                status = require_local(source, error);
                if (status != 1) return status;
                checkedSimulation = YES;
            }
            ssize_t count = read(input, buffer, sizeof(buffer));
            if (count < 0) { if (errno == EINTR) continue; return system_fail(error); }
            if (!count) break;
            for (ssize_t done = 0; done < count;) {
                ssize_t written = write(output, buffer + done, (size_t)(count - done));
                if (written < 0 && errno == EINTR) continue;
                if (written <= 0) { if (!written) errno = EIO; return system_fail(error); }
                done += written;
            }
            if (!preserveName) CC_SHA256_Update(&hash, buffer, (CC_LONG)count);
            total += (uint64_t)count;
            if (!progress(progressUser, total)) return fail(error, @"Offline copy progress callback failed");
        }
        struct stat after;
        LWFileInfo version;
        if (fstat(input, &after)) return system_fail(error);
        if (!lw_file_probe(source, 0, &version, error)) return 0;
        if (original.st_size != after.st_size || original.st_mtimespec.tv_sec != after.st_mtimespec.tv_sec || original.st_mtimespec.tv_nsec != after.st_mtimespec.tv_nsec || before.size != version.size || (isfinite(before.modified) && isfinite(version.modified) && before.modified != version.modified)) return fail(error, @"Audio file changed during offline copy; retry it");
        if (fsync(output)) return system_fail(error);
        int closing = output; output = -1;
        if (close(closing)) return system_fail(error);
        if (cancelled(user)) return interrupted(error);
        NSString *filename = [NSString stringWithUTF8String:source].lastPathComponent;
        if (!preserveName) {
            unsigned char digest[32]; char hex[65];
            CC_SHA256_Final(digest, &hash);
            for (size_t i = 0; i < sizeof(digest); i++) { hex[i * 2] = "0123456789abcdef"[digest[i] >> 4]; hex[i * 2 + 1] = "0123456789abcdef"[digest[i] & 15]; }
            hex[64] = 0;
            NSString *extension = filename.pathExtension.lowercaseString;
            filename = extension.length ? [NSString stringWithFormat:@"%s.%@", hex, extension] : [NSString stringWithUTF8String:hex];
        }
        NSURL *destination = [directory URLByAppendingPathComponent:filename];
        if (renamex_np(temporary.fileSystemRepresentation, destination.fileSystemRepresentation, RENAME_EXCL)) {
            if (errno != EEXIST) return system_fail(error);
            if (!identical(temporary.fileSystemRepresentation, destination.fileSystemRepresentation, cancelled, user)) {
                if (cancelled(user)) return interrupted(error);
                if (!preserveName) return fail(error, @"The existing offline file does not match its content digest");
                filename = [NSString stringWithFormat:@"%@-%@", NSUUID.UUID.UUIDString, filename];
                destination = [directory URLByAppendingPathComponent:filename];
                if (cancelled(user)) return interrupted(error);
                if (renamex_np(temporary.fileSystemRepresentation, destination.fileSystemRepresentation, RENAME_EXCL)) return system_fail(error);
            }
        }
        if (cancelled(user)) return interrupted(error);
        result->path = strdup(destination.URLByResolvingSymlinksInPath.path.UTF8String);
        if (!result->path) return fail(error, @"Could not encode offline file location");
        result->modified = version.modified;
        result->size = version.size;
        return 1;
    } @finally {
        if (input >= 0) close(input);
        if (output >= 0) close(output);
        unlink(temporary.fileSystemRepresentation);
    }
}

int lw_file_copy(const char *source, const char *destination, int preserveName, void *preferences, int (*cancelled)(void *), void *user, int (*progress)(void *, uint64_t), void *progressUser, LWCopyResult *result, char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) return fail(error, @"Offline copies must run on a worker thread");
        if (cancelled(user)) return interrupted(error);
        NSString *path = source ? [NSString stringWithUTF8String:source] : nil;
        NSString *target = destination ? [NSString stringWithUTF8String:destination] : nil;
        if (!path.isAbsolutePath || !target.isAbsolutePath) return fail(error, @"Offline copy requires absolute file locations");
        int allowed = network_check(source, preferences, error);
        if (allowed != 1) return allowed;
        NSURL *url = [NSURL fileURLWithPath:path], *directory = [NSURL fileURLWithPath:target isDirectory:YES];
        __block int status = 0;
        NSError *coordinationError = nil;
        [NSFileCoordinator.new coordinateReadingItemAtURL:url options:0 error:&coordinationError byAccessor:^(NSURL *readable) {
            status = copy_stream(source, readable, directory, preserveName != 0, preferences, cancelled, user, progress, progressUser, result, error);
        }];
        if (coordinationError) { free(result->path); result->path = NULL; if (error && *error) { free(*error); *error = NULL; } return fail(error, coordinationError.localizedDescription); }
        return status;
    }
}
