#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "platform.h"

@interface LWContextStrings : NSObject
@property(nonatomic, strong) NSData *name;
@property(nonatomic, strong) NSData *artist;
@end
@implementation LWContextStrings
@end

static BOOL context_integer(id value, int64_t *result) {
    if (![value isKindOfClass:NSNumber.class] || CFGetTypeID((__bridge CFTypeRef)value) == CFBooleanGetTypeID()) return NO;
    if (CFNumberIsFloatType((__bridge CFNumberRef)value)) {
        double number = [value doubleValue];
        if (!isfinite(number) || number < -0x1p63 || number >= 0x1p63 || trunc(number) != number) return NO;
    }
    int64_t integer = [value longLongValue];
    NSDecimal actual = [value decimalValue], expected = [@(integer) decimalValue];
    if (NSDecimalCompare(&actual, &expected) != NSOrderedSame) return NO;
    *result = integer;
    return YES;
}

void *lw_context_decode(const unsigned char *data, size_t len, LWContext *fields, char **error) {
    @autoreleasepool {
        if (len > 64 * 1024 * 1024) { if (error) *error = strdup("Playback context exceeds its memory limit"); return NULL; }
        id object = [NSJSONSerialization JSONObjectWithData:[NSData dataWithBytes:data length:len] options:0 error:nil];
        NSArray<NSString *> *names = @[@"library", @"album", @"artist", @"playlist", @"folder", @"automatic", @"selection"];
        NSUInteger kind = NSNotFound;
        if ([object isKindOfClass:NSDictionary.class]) {
            for (NSUInteger i = 0; i < names.count; i++) {
                if (object[names[i]]) {
                    if (kind != NSNotFound) { kind = NSNotFound; break; }
                    kind = i;
                }
            }
        }
        NSDictionary *payload = kind != NSNotFound ? object[names[kind]] : nil;
        BOOL valid = [payload isKindOfClass:NSDictionary.class];
        NSString *name = @"", *artist = @"";
        *fields = (LWContext){0};
        fields->kind = (int)kind;
        if (valid) {
            if (kind == 1 || kind == 3 || kind == 4) name = payload[@"name"];
            if (kind == 2 || kind == 5 || kind == 6) name = payload[@"_0"];
            if (kind == 1) artist = payload[@"artist"];
            valid = [name isKindOfClass:NSString.class] && [artist isKindOfClass:NSString.class];
            if (kind == 3) valid = valid && context_integer(payload[@"id"], &fields->id);
            if (kind == 4) {
                valid = valid && context_integer(payload[@"sourceID"], &fields->id);
                id path = payload[@"pathID"];
                fields->has_path = path && path != NSNull.null;
                if (fields->has_path) valid = valid && context_integer(path, &fields->path);
            }
        }
        if (!valid) { if (error) *error = strdup("Invalid Swift playback context"); return NULL; }
        LWContextStrings *strings = [LWContextStrings new];
        strings.name = [name dataUsingEncoding:NSUTF8StringEncoding];
        strings.artist = [artist dataUsingEncoding:NSUTF8StringEncoding];
        if (!strings.name || !strings.artist) { if (error) *error = strdup("Playback context is not valid UTF-8"); return NULL; }
        fields->name = (LWBytes){(unsigned char *)strings.name.bytes, strings.name.length};
        fields->artist = (LWBytes){(unsigned char *)strings.artist.bytes, strings.artist.length};
        return (__bridge_retained void *)strings;
    }
}

int lw_context_encode(const LWContext *fields, LWBytes *data, char **error) {
    @autoreleasepool {
        *data = (LWBytes){0};
        // Bound the worst-case JSON escaping before Foundation allocates the encoded strings.
        if (fields->kind < 0 || fields->kind > 6 || fields->name.len > (64 * 1024 * 1024 - 1024) / 6 || fields->artist.len > (64 * 1024 * 1024 - 1024) / 6 - fields->name.len) {
            if (error) *error = strdup("Playback context exceeds its memory limit"); return 0;
        }
        NSString *name = [[NSString alloc] initWithBytes:fields->name.data length:fields->name.len encoding:NSUTF8StringEncoding];
        NSString *artist = [[NSString alloc] initWithBytes:fields->artist.data length:fields->artist.len encoding:NSUTF8StringEncoding];
        if (!name || !artist) { if (error) *error = strdup("Playback context is not valid UTF-8"); return 0; }
        NSArray<NSString *> *names = @[@"library", @"album", @"artist", @"playlist", @"folder", @"automatic", @"selection"];
        NSDictionary *payload;
        switch (fields->kind) {
            case 0: payload = @{}; break;
            case 1: payload = @{@"name": name, @"artist": artist}; break;
            case 3: payload = @{@"id": @(fields->id), @"name": name}; break;
            case 4: payload = fields->has_path ? @{@"sourceID": @(fields->id), @"pathID": @(fields->path), @"name": name} : @{@"sourceID": @(fields->id), @"name": name}; break;
            default: payload = @{@"_0": name}; break;
        }
        NSData *json = [NSJSONSerialization dataWithJSONObject:@{names[fields->kind]: payload} options:0 error:nil];
        if (!json) { if (error) *error = strdup("Could not encode playback context"); return 0; }
        data->data = malloc(json.length);
        if (!data->data) { if (error) *error = strdup("Could not allocate playback context"); return 0; }
        data->len = json.length;
        memcpy(data->data, json.bytes, json.length);
        return 1;
    }
}
