#include <TargetConditionals.h>
#if TARGET_OS_IOS
#import <UIKit/UIKit.h>
#else
#import <AppKit/AppKit.h>
#endif
#include "platform.h"

int lw_window_open_link(int kind) {
    if (!NSThread.isMainThread || (kind != 0 && kind != 1)) return 0;
    @autoreleasepool {
        NSString *link = kind == 0 ? @"https://github.com/olegpustovit/localwave/blob/dev/docs/privacy.md" : @"https://github.com/olegpustovit/localwave/issues";
#if TARGET_OS_IOS
        NSURL *url = [NSURL URLWithString:link];
        if (![UIApplication.sharedApplication canOpenURL:url]) return 0;
        [UIApplication.sharedApplication openURL:url options:@{} completionHandler:nil];
        return 1;
#else
        if (!NSApp) return 0;
        return [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:link]];
#endif
    }
}

// Queued on the main run loop so worker threads may request it; a headless process never services it.
int lw_window_reveal_file(const char *path) {
#if TARGET_OS_IOS
    (void)path;
    return 0;
#else
    NSString *text = path ? [NSString stringWithUTF8String:path] : nil;
    if (!text || !text.isAbsolutePath) return 0;
    NSURL *url = [NSURL fileURLWithPath:text];
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSWorkspace.sharedWorkspace activateFileViewerSelectingURLs:@[url]];
    });
    return 1;
#endif
}
