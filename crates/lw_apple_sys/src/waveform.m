#import <AVFoundation/AVFoundation.h>
#import <CommonCrypto/CommonDigest.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "platform.h"

static int fail(char **error, NSString *message) {
    if (error) *error = strdup((message ?: @"Could not decode audio waveform").UTF8String);
    return 0;
}
static NSDictionary *attributes(NSURL *url, char **error) {
    NSError *failure = nil;
    NSDictionary *values = [NSFileManager.defaultManager attributesOfItemAtPath:url.path error:&failure];
    if (!values) { fail(error, failure.localizedDescription); return nil; }
    if (![values[NSFileType] isEqual:NSFileTypeRegular]) { fail(error, @"Waveform source must be a regular file"); return nil; }
    return values;
}
int lw_waveform_version(const char *path, int knownLocal, LWFileInfo *info, char **error) {
    @autoreleasepool {
        if (!lw_file_probe(path, knownLocal, info, error)) return 0;
        if (info->availability == 2 || (info->provider && info->availability == 0)) return fail(error, @"Waveform source is not confirmed local");
        NSDictionary *values = attributes([NSURL fileURLWithPath:[NSString stringWithUTF8String:path]], error);
        if (!values) return 0;
        info->size = [values[NSFileSize] longLongValue];
        info->modified = values[NSFileModificationDate] ? [values[NSFileModificationDate] timeIntervalSince1970] : NAN;
        return 1;
    }
}
void lw_sha256(const unsigned char *bytes, uint32_t length, unsigned char digest[32]) {
    CC_SHA256(bytes, length, digest);
}
static BOOL same_version(const LWFileInfo *a, const LWFileInfo *b) {
    return a->size == b->size && ((isnan(a->modified) && isnan(b->modified)) || a->modified == b->modified);
}

static int accumulate(CMSampleBufferRef sample, double duration, float peaks[512], int (*cancelled)(void *), void *user, char **error) {
    CMFormatDescriptionRef format = CMSampleBufferGetFormatDescription(sample);
    const AudioStreamBasicDescription *description = format ? CMAudioFormatDescriptionGetStreamBasicDescription(format) : NULL;
    CMBlockBufferRef block = CMSampleBufferGetDataBuffer(sample);
    CMItemCount frames = CMSampleBufferGetNumSamples(sample);
    if (!description || !block || frames < 0 || !isfinite(description->mSampleRate) || description->mSampleRate <= 0) return fail(error, @"Invalid waveform sample format");
    uint32_t channels = description->mChannelsPerFrame;
    float samples[16384];
    if (!channels || channels > sizeof(samples) / sizeof(float) || description->mBitsPerChannel != 32 || description->mBytesPerFrame != channels * sizeof(float) || !(description->mFormatFlags & kAudioFormatFlagIsFloat) || (description->mFormatFlags & (kAudioFormatFlagIsNonInterleaved | kAudioFormatFlagIsBigEndian))) return fail(error, @"Unsupported waveform PCM format");
    size_t frameBytes = channels * sizeof(float), bytes = CMBlockBufferGetDataLength(block);
    if ((uint64_t)frames > bytes / frameBytes) return fail(error, @"Truncated waveform samples");
    double start = CMTimeGetSeconds(CMSampleBufferGetPresentationTimeStamp(sample));
    if (!isfinite(start)) return 1;
    for (size_t offset = 0; offset < (uint64_t)frames;) {
        if (cancelled(user)) return fail(error, @"Waveform decoding was cancelled");
        size_t count = MIN(sizeof(samples) / frameBytes, (size_t)frames - offset);
        if (CMBlockBufferCopyDataBytes(block, offset * frameBytes, count * frameBytes, samples) != kCMBlockBufferNoErr) return fail(error, @"Could not read waveform PCM");
        for (size_t frame = 0; frame < count; frame++) {
            double position = (start + (double)(offset + frame) / description->mSampleRate) / duration;
            size_t bucket = position <= 0 ? 0 : position >= 1 ? 511 : (size_t)(position * 512);
            for (uint32_t channel = 0; channel < channels; channel++) {
                float value = fabsf(samples[frame * channels + channel]);
                if (isfinite(value)) peaks[bucket] = fmaxf(peaks[bucket], value);
            }
        }
        offset += count;
    }
    return 1;
}
static int decode(NSURL *url, int (*cancelled)(void *), void *user, float peaks[512], char **error) {
    AVURLAsset *asset = [AVURLAsset URLAssetWithURL:url options:nil];
    __block NSArray<AVAssetTrack *> *tracks = nil;
    __block NSError *trackError = nil;
    dispatch_group_t ready = dispatch_group_create();
    dispatch_group_enter(ready);
    [asset loadTracksWithMediaType:AVMediaTypeAudio completionHandler:^(NSArray<AVAssetTrack *> *value, NSError *failure) { tracks = value; trackError = failure; dispatch_group_leave(ready); }];
    dispatch_group_enter(ready);
    [asset loadValuesAsynchronouslyForKeys:@[@"duration"] completionHandler:^{ dispatch_group_leave(ready); }];
    uint64_t deadline = clock_gettime_nsec_np(CLOCK_UPTIME_RAW) + 3 * NSEC_PER_SEC;
    while (dispatch_group_wait(ready, dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_MSEC))) {
        if (cancelled(user) || clock_gettime_nsec_np(CLOCK_UPTIME_RAW) >= deadline) { [asset cancelLoading]; return fail(error, cancelled(user) ? @"Waveform decoding was cancelled" : @"Waveform audio loading timed out"); }
    }
    if (cancelled(user)) return fail(error, @"Waveform decoding was cancelled");
    NSError *failure = nil;
    if (trackError || [asset statusOfValueForKey:@"duration" error:&failure] != AVKeyValueStatusLoaded) return fail(error, trackError.localizedDescription ?: failure.localizedDescription);
    double duration = CMTimeGetSeconds(asset.duration);
    if (!tracks.count || !isfinite(duration) || duration <= 0) return 2;
    AVAssetReader *reader = [[AVAssetReader alloc] initWithAsset:asset error:&failure];
    if (!reader) return fail(error, failure.localizedDescription);
    AVAssetReaderTrackOutput *output = [[AVAssetReaderTrackOutput alloc] initWithTrack:tracks.firstObject outputSettings:@{AVFormatIDKey: @(kAudioFormatLinearPCM), AVLinearPCMIsFloatKey: @YES, AVLinearPCMBitDepthKey: @32, AVLinearPCMIsNonInterleaved: @NO, AVLinearPCMIsBigEndianKey: @NO}];
    output.alwaysCopiesSampleData = NO;
    if (![reader canAddOutput:output]) return fail(error, @"Could not configure waveform reader");
    [reader addOutput:output];
    if (![reader startReading]) return fail(error, reader.error.localizedDescription);
    for (;;) {
        @autoreleasepool {
            if (cancelled(user)) { [reader cancelReading]; return fail(error, @"Waveform decoding was cancelled"); }
            CMSampleBufferRef sample = [output copyNextSampleBuffer];
            if (!sample) break;
            int success = accumulate(sample, duration, peaks, cancelled, user, error);
            CFRelease(sample);
            if (!success) { [reader cancelReading]; return 0; }
        }
    }
    if (reader.status != AVAssetReaderStatusCompleted) return fail(error, reader.error.localizedDescription ?: @"Waveform audio read did not complete");
    float maximum = 0;
    for (size_t i = 0; i < 512; i++) maximum = fmaxf(maximum, peaks[i]);
    if (maximum > 0) for (size_t i = 0; i < 512; i++) peaks[i] = powf(peaks[i] / maximum, 0.7f);
    return 1;
}
int lw_waveform_decode(const char *path, int knownLocal, const LWFileInfo *expected, int (*cancelled)(void *), void *user, float peaks[512], char **error) {
    @autoreleasepool {
        if (NSThread.isMainThread) return fail(error, @"Waveform decoding must run on a worker thread");
        if (cancelled(user)) return fail(error, @"Waveform decoding was cancelled");
        LWFileInfo before;
        if (!lw_waveform_version(path, knownLocal, &before, error)) return 0;
        if (!same_version(expected, &before)) return fail(error, @"Waveform source changed before decoding; retry it");
        NSURL *url = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path]];
        __block int result = 0;
        __block char *readError = NULL;
        NSError *coordinationError = nil;
        [[NSFileCoordinator new] coordinateReadingItemAtURL:url options:0 error:&coordinationError byAccessor:^(NSURL *readable) {
            NSDictionary *initial = attributes(readable, &readError);
            if (!initial) return;
            LWFileInfo opened = before;
            opened.size = [initial[NSFileSize] longLongValue];
            opened.modified = initial[NSFileModificationDate] ? [initial[NSFileModificationDate] timeIntervalSince1970] : NAN;
            if (!same_version(&before, &opened)) { fail(&readError, @"Waveform source changed before decoding; retry it"); return; }
            memset(peaks, 0, 512 * sizeof(float));
            result = decode(readable, cancelled, user, peaks, &readError);
            if (!result) return;
            NSDictionary *final = attributes(readable, &readError);
            if (!final) { result = 0; return; }
            for (NSString *key in @[NSFileSize, NSFileModificationDate, NSFileSystemFileNumber, NSFileSystemNumber]) {
                if (initial[key] != final[key] && ![initial[key] isEqual:final[key]]) { result = fail(&readError, @"Waveform source changed during decoding; retry it"); break; }
            }
        }];
        if (!result || coordinationError) {
            if (readError) { if (error) *error = readError; else free(readError); }
            else fail(error, coordinationError.localizedDescription);
            return 0;
        }
        LWFileInfo after;
        if (!lw_waveform_version(path, knownLocal, &after, error)) return 0;
        if (!same_version(&before, &after)) return fail(error, @"Waveform source changed during decoding; retry it");
        if (cancelled(user)) return fail(error, @"Waveform decoding was cancelled");
        return result;
    }
}
