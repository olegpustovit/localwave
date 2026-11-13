#import <AVFoundation/AVFoundation.h>
#import <CommonCrypto/CommonDigest.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

@interface LWAudioMetadata : NSObject
@property(nonatomic, strong) NSArray *text;
@property(nonatomic, strong) NSArray<NSNumber *> *numbers;
@property(nonatomic, strong) NSData *artwork;
@property(nonatomic, strong) NSData *digest;
@end
@implementation LWAudioMetadata
@end

static int fail(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Could not read audio file").UTF8String);
    return 0;
}

static NSURL *file_url(const char *path) {
    NSString *text = path ? [NSString stringWithUTF8String:path] : nil;
    return text.isAbsolutePath ? [NSURL fileURLWithPath:text] : nil;
}

static BOOL under(NSURL *file, NSURL *root) {
    NSString *path = file.URLByResolvingSymlinksInPath.URLByStandardizingPath.path;
    NSString *base = root.URLByResolvingSymlinksInPath.URLByStandardizingPath.path;
    return [path isEqualToString:base] || [path hasPrefix:[base hasSuffix:@"/"] ? base : [base stringByAppendingString:@"/"]];
}

static BOOL provider(NSURL *url) {
    NSArray<NSString *> *parts = url.URLByResolvingSymlinksInPath.URLByStandardizingPath.pathComponents;
    NSArray *names = @[@"CloudStorage", @"Mobile Documents", @"CloudDocs", @"FileProviderStorage", @"File Provider Storage", @"FileProvider"];
    for (NSUInteger i = 0; i + 1 < parts.count; i++) {
        if ([parts[i] isEqual:@"Library"] && [names containsObject:parts[i + 1]]) return YES;
    }
    return ([parts containsObject:@"AppGroup"] || [parts containsObject:@"PluginKitPlugin"]) && ([parts containsObject:@"FileProviderStorage"] || [parts containsObject:@"File Provider Storage"]);
}

static int probe(NSURL *url, BOOL knownLocal, LWFileInfo *info, char **error) {
    if (!url) return fail(error, @"Expected an absolute audio path");
    NSError *failure = nil;
    NSDictionary *values = [url resourceValuesForKeys:@[NSURLIsDirectoryKey, NSURLFileSizeKey, NSURLContentModificationDateKey, NSURLIsUbiquitousItemKey, NSURLUbiquitousItemDownloadingStatusKey, NSURLVolumeIsLocalKey, NSURLVolumeIsRemovableKey, NSURLVolumeIsInternalKey] error:&failure];
    if (!values) return fail(error, failure.localizedDescription);
    if ([values[NSURLIsDirectoryKey] boolValue]) return fail(error, @"Audio path is a folder");
    info->size = values[NSURLFileSizeKey] ? [values[NSURLFileSizeKey] longLongValue] : -1;
    info->modified = values[NSURLContentModificationDateKey] ? [values[NSURLContentModificationDateKey] timeIntervalSince1970] : NAN;
    info->provider = provider(url);
    info->availability = 0;
    if ([values[NSURLIsUbiquitousItemKey] boolValue]) {
        NSString *status = values[NSURLUbiquitousItemDownloadingStatusKey];
        if ([status isEqual:NSURLUbiquitousItemDownloadingStatusNotDownloaded]) info->availability = 2;
        else if ([status isEqual:NSURLUbiquitousItemDownloadingStatusDownloaded] || [status isEqual:NSURLUbiquitousItemDownloadingStatusCurrent]) info->availability = 1;
        return 1;
    }
    NSFileManager *manager = NSFileManager.defaultManager;
    if (![manager isReadableFileAtPath:url.path]) return 1;
    if (knownLocal) { info->availability = 1; return 1; }
    if (info->provider || ![values[NSURLVolumeIsLocalKey] boolValue]) return 1;
    NSArray<NSString *> *parts = url.URLByResolvingSymlinksInPath.URLByStandardizingPath.pathComponents;
    if ([values[NSURLVolumeIsRemovableKey] boolValue] || (values[NSURLVolumeIsInternalKey] && ![values[NSURLVolumeIsInternalKey] boolValue]) || (parts.count > 1 && [parts[1] isEqual:@"Volumes"])) { info->availability = 1; return 1; }
    for (NSNumber *directory in @[@(NSDocumentDirectory), @(NSDownloadsDirectory), @(NSDesktopDirectory), @(NSMusicDirectory)]) {
        for (NSURL *root in [manager URLsForDirectory:directory.unsignedIntegerValue inDomains:NSUserDomainMask]) {
            if (under(url, root)) { info->availability = 1; return 1; }
        }
    }
    if (under(url, manager.temporaryDirectory)) { info->availability = 1; return 1; }
    NSUInteger users = [parts indexOfObject:@"Users"];
    if (users != NSNotFound && parts.count > users + 2 && [@[@"Music", @"Documents", @"Desktop", @"Downloads", @"Movies", @"Public"] containsObject:parts[users + 2]]) info->availability = 1;
    NSUInteger app = [parts indexOfObject:@"Application"];
    if (app != NSNotFound && parts.count > app + 2 && [[NSUUID alloc] initWithUUIDString:parts[app + 1]] && [parts[app + 2] isEqual:@"Documents"]) info->availability = 1;
    return 1;
}

int lw_file_probe(const char *path, int knownLocal, LWFileInfo *info, char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) return fail(error, @"File probes must run on a worker thread");
        return probe(file_url(path), knownLocal, info, error);
    }
}

static NSNumber *index_number(NSData *data) {
    if (data.length < 4) return nil;
    const unsigned char *bytes = data.bytes;
    unsigned int number = ((unsigned int)bytes[2] << 8) | bytes[3];
    return number ? @(number) : nil;
}

static NSNumber *tag_number(NSString *value) {
    if (!value) return nil;
    NSString *part = [value componentsSeparatedByString:@"/"].firstObject;
    NSScanner *scanner = [NSScanner scannerWithString:part];
    scanner.charactersToBeSkipped = nil;
    long long result;
    return [scanner scanLongLong:&result] && scanner.isAtEnd ? @(result) : nil;
}

static LWAudioMetadata *read_audio(NSURL *url, int (*cancelled)(void *), void *user, char **error) {
    NSError *failure = nil;
    AVAudioFile *audio = [[AVAudioFile alloc] initForReading:url error:&failure];
    NSString *warning = audio ? nil : failure.localizedDescription;
    double duration = NAN;
    double sampleRate = audio.fileFormat.sampleRate;
    unsigned int bits = audio ? audio.fileFormat.streamDescription->mBitsPerChannel : 0;
    AVURLAsset *asset = [AVURLAsset URLAssetWithURL:url options:nil];
    dispatch_time_t deadline = dispatch_time(DISPATCH_TIME_NOW, 3 * NSEC_PER_SEC);
    dispatch_semaphore_t loaded = dispatch_semaphore_create(0);
    [asset loadValuesAsynchronouslyForKeys:@[@"availableMetadataFormats", @"duration"] completionHandler:^{ dispatch_semaphore_signal(loaded); }];
    NSString *tagIssue = nil;
    if (dispatch_semaphore_wait(loaded, deadline) || [asset statusOfValueForKey:@"availableMetadataFormats" error:&failure] != AVKeyValueStatusLoaded) {
        [asset cancelLoading]; tagIssue = failure.localizedDescription ?: @"Audio metadata timed out";
    }
    if ([asset statusOfValueForKey:@"duration" error:nil] == AVKeyValueStatusLoaded) duration = CMTimeGetSeconds(asset.duration);
    NSMutableArray *text = [NSMutableArray arrayWithArray:@[NSNull.null, NSNull.null, NSNull.null, NSNull.null]];
    NSNumber *year = nil, *track = nil, *disc = nil;
    NSData *artwork = nil;
    for (AVMetadataFormat format in tagIssue ? @[] : asset.availableMetadataFormats) {
        __block NSArray<AVMetadataItem *> *items = nil;
        __block NSError *metadataError = nil;
        dispatch_semaphore_t ready = dispatch_semaphore_create(0);
        [asset loadMetadataForFormat:format completionHandler:^(NSArray<AVMetadataItem *> *value, NSError *issue) { items = value; metadataError = issue; dispatch_semaphore_signal(ready); }];
        if (dispatch_semaphore_wait(ready, deadline)) { [asset cancelLoading]; tagIssue = @"Audio metadata timed out"; break; }
        if (!items || metadataError) { tagIssue = metadataError.localizedDescription ?: @"Audio tags could not be read"; break; }
        dispatch_group_t values = dispatch_group_create();
        for (AVMetadataItem *item in items) {
            dispatch_group_enter(values);
            [item loadValuesAsynchronouslyForKeys:@[@"value"] completionHandler:^{ dispatch_group_leave(values); }];
        }
        if (dispatch_group_wait(values, deadline)) { [asset cancelLoading]; tagIssue = @"Audio tag values timed out"; break; }
        for (AVMetadataItem *item in items) {
            if ([item statusOfValueForKey:@"value" error:nil] != AVKeyValueStatusLoaded) continue;
            NSString *common = item.commonKey, *identifier = item.identifier;
            NSString *key = [item.key isKindOfClass:NSString.class] ? (NSString *)item.key : nil;
            NSString *value = item.stringValue;
            if ([common isEqual:AVMetadataCommonKeyArtist] && text[0] == NSNull.null && value.length) text[0] = value;
            if ([common isEqual:AVMetadataCommonKeyTitle] && text[1] == NSNull.null && value.length) text[1] = value;
            if ([common isEqual:AVMetadataCommonKeyAlbumName] && text[2] == NSNull.null && value.length) text[2] = value;
            if (([identifier isEqual:AVMetadataIdentifieriTunesMetadataAlbumArtist] || [key isEqual:@"TPE2"] || [key isEqual:@"aART"]) && text[3] == NSNull.null && value.length) text[3] = value;
            if (!year && ([identifier isEqual:AVMetadataIdentifieriTunesMetadataReleaseDate] || [key isEqual:@"TDRC"])) year = tag_number([value substringToIndex:MIN((NSUInteger)4, value.length)]);
            if (!track && [identifier isEqual:AVMetadataIdentifieriTunesMetadataTrackNumber]) track = index_number(item.dataValue);
            if (!disc && [identifier isEqual:AVMetadataIdentifieriTunesMetadataDiscNumber]) disc = index_number(item.dataValue);
            if (!track && [key isEqual:@"TRCK"]) track = tag_number(value);
            if (!disc && [key isEqual:@"TPOS"]) disc = tag_number(value);
            if (!artwork) {
                NSData *data = item.dataValue;
                if (data.length >= 4 && data.length <= 64 * 1024 * 1024) {
                    const unsigned char *bytes = data.bytes;
                    if (!memcmp(bytes, "\x89PNG", 4) || !memcmp(bytes, "\xff\xd8\xff", 3)) artwork = data;
                }
            }
        }
    }
    if (tagIssue) {
        text = [NSMutableArray arrayWithArray:@[NSNull.null, NSNull.null, NSNull.null, NSNull.null]];
        year = nil; track = nil; disc = nil;
    }
    [text addObject:tagIssue ?: warning ?: (id)NSNull.null];
    if (cancelled(user)) { fail(error, @"Audio import was cancelled"); return nil; }
    NSInputStream *stream = [NSInputStream inputStreamWithURL:url];
    [stream open];
    CC_SHA256_CTX context;
    CC_SHA256_Init(&context);
    unsigned char buffer[256 * 1024];
    NSInteger count;
    while ((count = [stream read:buffer maxLength:sizeof(buffer)]) > 0) {
        if (cancelled(user)) { [stream close]; fail(error, @"Audio import was cancelled"); return nil; }
        CC_SHA256_Update(&context, buffer, (CC_LONG)count);
    }
    failure = stream.streamError;
    [stream close];
    if (count < 0 || failure) { fail(error, failure.localizedDescription); return nil; }
    unsigned char digest[CC_SHA256_DIGEST_LENGTH];
    CC_SHA256_Final(digest, &context);
    LWAudioMetadata *result = [LWAudioMetadata new];
    NSMutableArray *encoded = [NSMutableArray arrayWithCapacity:text.count];
    for (id value in text) {
        if (value == NSNull.null) { [encoded addObject:value]; continue; }
        NSData *bytes = [value dataUsingEncoding:NSUTF8StringEncoding];
        if (!bytes) { fail(error, @"Audio tag is not valid UTF-8"); return nil; }
        [encoded addObject:bytes];
    }
    result.text = encoded;
    result.numbers = @[@(duration), @(sampleRate), @(bits), year ?: @(NAN), track ?: @(NAN), disc ?: @(NAN)];
    result.artwork = artwork;
    result.digest = [NSData dataWithBytes:digest length:sizeof(digest)];
    return result;
}

void *lw_audio_read(const char *path, int knownLocal, int (*cancelled)(void *), void *user, int (*extra)(void *, const char *), void *extraUser, char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) { fail(error, @"Audio import must run on a worker thread"); return NULL; }
        if (cancelled(user)) { fail(error, @"Audio import was cancelled"); return NULL; }
        NSURL *url = file_url(path);
        LWFileInfo before;
        if (!probe(url, knownLocal, &before, error)) return NULL;
        if (before.availability == 2 || (before.provider && before.availability == 0)) { fail(error, @"Audio file is not confirmed local"); return NULL; }
        __block LWAudioMetadata *result = nil;
        __block char *readError = NULL;
        NSError *coordinationError = nil;
        NSFileCoordinator *coordinator = [NSFileCoordinator new];
        [coordinator coordinateReadingItemAtURL:url options:0 error:&coordinationError byAccessor:^(NSURL *readable) {
            result = read_audio(readable, cancelled, user, &readError);
            if (result && !extra(extraUser, readable.path.UTF8String)) {
                result = nil;
                fail(&readError, @"Additional audio metadata read failed");
            }
        }];
        if (!result || coordinationError) {
            if (readError) { if (error) *error = readError; else free(readError); }
            else fail(error, coordinationError.localizedDescription);
            return NULL;
        }
        LWFileInfo after;
        [url removeAllCachedResourceValues];
        if (!probe(url, knownLocal, &after, error)) return NULL;
        if (before.size != after.size || (isfinite(before.modified) && isfinite(after.modified) && before.modified != after.modified)) { fail(error, @"Audio file changed during import; retry it"); return NULL; }
        return (__bridge_retained void *)result;
    }
}

const unsigned char *lw_audio_text(void *raw, unsigned int slot, size_t *len) {
    LWAudioMetadata *audio = (__bridge LWAudioMetadata *)raw;
    id value = slot < audio.text.count ? audio.text[slot] : nil;
    NSData *data = [value isKindOfClass:NSData.class] ? value : nil;
    *len = data.length;
    return data.bytes;
}

double lw_audio_number(void *raw, unsigned int slot) {
    LWAudioMetadata *audio = (__bridge LWAudioMetadata *)raw;
    return slot < audio.numbers.count ? audio.numbers[slot].doubleValue : NAN;
}

const unsigned char *lw_audio_data(void *raw, unsigned int slot, size_t *len) {
    LWAudioMetadata *audio = (__bridge LWAudioMetadata *)raw;
    NSData *data = slot == 0 ? audio.artwork : audio.digest;
    *len = data.length;
    return data.bytes;
}

void lw_audio_release(void *raw) {
    @autoreleasepool { id audio = (__bridge_transfer id)raw; (void)audio; }
}
