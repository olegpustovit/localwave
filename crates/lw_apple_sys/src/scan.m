#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

@interface LWDirectoryScan : NSObject
@property(nonatomic, strong) NSURL *root;
@property(nonatomic, strong) NSURL *registered;
@property(nonatomic, strong) NSArray<NSString *> *excluded;
@property(nonatomic, strong) NSDirectoryEnumerator<NSURL *> *enumerator;
@property(nonatomic, strong) NSError *failure;
@end
@implementation LWDirectoryScan
@end

static int fail(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Folder enumeration failed").UTF8String);
    return -1;
}

static BOOL under(NSString *path, NSString *root) {
    return [path isEqualToString:root] || [path hasPrefix:[root hasSuffix:@"/"] ? root : [root stringByAppendingString:@"/"]];
}

static BOOL excluded(NSURL *url, NSArray<NSString *> *roots) {
    NSString *path = url.URLByResolvingSymlinksInPath.URLByStandardizingPath.path;
    for (NSString *root in roots) if (under(path, root)) return YES;
    return NO;
}

void *lw_scan_open(void *rootURL, const char *registeredRoot, const char *const *excludedPaths, size_t excludedCount, char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) { fail(error, @"Folder scans must run on a worker thread"); return NULL; }
        NSURL *root = (__bridge NSURL *)rootURL;
        NSString *registered = registeredRoot ? [NSString stringWithUTF8String:registeredRoot] : nil;
        if (!root.isFileURL || !registered.isAbsolutePath) { fail(error, @"Folder scan requires absolute file locations"); return NULL; }
        NSMutableArray *roots = [NSMutableArray arrayWithCapacity:excludedCount];
        for (size_t i = 0; i < excludedCount; i++) {
            NSString *path = [NSString stringWithUTF8String:excludedPaths[i]];
            if (!path.isAbsolutePath) { fail(error, @"Generated-path exclusions must be absolute"); return NULL; }
            [roots addObject:[NSURL fileURLWithPath:path].URLByResolvingSymlinksInPath.URLByStandardizingPath.path];
        }
        if (excluded(root, roots)) { fail(error, @"Choose your original music folder. LocalWave's generated cache cannot be used as a source."); return NULL; }
        NSNumber *directory = nil;
        NSError *failure = nil;
        if (![root getResourceValue:&directory forKey:NSURLIsDirectoryKey error:&failure] || !directory.boolValue) { fail(error, failure.localizedDescription ?: @"Music source is not a folder"); return NULL; }
        LWDirectoryScan *scan = [LWDirectoryScan new];
        scan.root = root.URLByStandardizingPath;
        scan.registered = [NSURL fileURLWithPath:registered isDirectory:YES];
        scan.excluded = roots;
        __weak LWDirectoryScan *weak = scan;
        scan.enumerator = [NSFileManager.defaultManager enumeratorAtURL:scan.root includingPropertiesForKeys:@[NSURLIsDirectoryKey, NSURLIsSymbolicLinkKey, NSURLFileSizeKey] options:NSDirectoryEnumerationSkipsHiddenFiles | NSDirectoryEnumerationSkipsPackageDescendants errorHandler:^BOOL(NSURL *url, NSError *issue) {
            (void)url;
            weak.failure = issue;
            return NO;
        }];
        if (!scan.enumerator) { fail(error, @"Could not enumerate the source folder"); return NULL; }
        return (__bridge_retained void *)scan;
    }
}

int lw_scan_next(void *raw, LWScanEntry *entry, char **error) {
    @autoreleasepool {
        LWDirectoryScan *scan = (__bridge LWDirectoryScan *)raw;
        for (;;) {
            NSURL *url = scan.enumerator.nextObject;
            if (scan.failure) return fail(error, scan.failure.localizedDescription);
            if (!url) return 0;
            NSError *failure = nil;
            NSDictionary *values = [url resourceValuesForKeys:@[NSURLIsDirectoryKey, NSURLIsSymbolicLinkKey, NSURLFileSizeKey] error:&failure];
            if (!values) return fail(error, failure.localizedDescription);
            BOOL directory = [values[NSURLIsDirectoryKey] boolValue];
            if (excluded(url, scan.excluded)) {
                if (directory) [scan.enumerator skipDescendants];
                continue;
            }
            if ([values[NSURLIsSymbolicLinkKey] boolValue]) {
                // On a leaf, skipDescendants can suppress the next sibling folder.
                if (directory) [scan.enumerator skipDescendants];
                NSString *target = url.URLByResolvingSymlinksInPath.URLByStandardizingPath.path;
                NSString *root = scan.root.URLByResolvingSymlinksInPath.URLByStandardizingPath.path;
                if (!under(target, root) || ![NSFileManager.defaultManager fileExistsAtPath:target]) continue;
            }
            NSString *path = url.URLByStandardizingPath.path;
            NSString *base = scan.root.path;
            if (!under(path, base) || [path isEqual:base]) return fail(error, @"Enumerated file is outside its source folder");
            NSString *relative = [path substringFromIndex:base.length + ([base hasSuffix:@"/"] ? 0 : 1)];
            NSURL *identity = [scan.registered URLByAppendingPathComponent:relative isDirectory:directory];
            entry->name = strdup(url.lastPathComponent.UTF8String);
            entry->relative = strdup(relative.UTF8String);
            entry->url = lw_normalized_file_url(identity.path.UTF8String);
            entry->parent = lw_normalized_file_url(identity.URLByDeletingLastPathComponent.path.UTF8String);
            entry->directory = directory;
            entry->size = !directory && values[NSURLFileSizeKey] ? [values[NSURLFileSizeKey] longLongValue] : -1;
            if (!entry->name || !entry->relative || !entry->url || !entry->parent) return fail(error, @"Could not encode an indexed file path");
            return 1;
        }
    }
}

void lw_scan_entry_clear(LWScanEntry *entry) {
    free(entry->name); free(entry->relative); free(entry->url); free(entry->parent);
    memset(entry, 0, sizeof(*entry));
}
void lw_scan_release(void *raw) {
    @autoreleasepool { id scan = (__bridge_transfer id)raw; (void)scan; }
}
