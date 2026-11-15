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

// Prepared copies are shown in the system picker anchored at a window point (top-left origin).
// Queued on the main run loop so worker threads may request it; a headless process never services it.
int lw_window_share_files(const char *const *paths, size_t count, double x, double y) {
#if TARGET_OS_IOS
    (void)paths; (void)count; (void)x; (void)y;
    return 0;
#else
    if (!paths || count == 0 || !isfinite(x) || !isfinite(y)) return 0;
    NSMutableArray<NSURL *> *urls = [NSMutableArray arrayWithCapacity:count];
    for (size_t index = 0; index < count; index++) {
        NSString *text = paths[index] ? [NSString stringWithUTF8String:paths[index]] : nil;
        if (!text || !text.isAbsolutePath) return 0;
        [urls addObject:[NSURL fileURLWithPath:text]];
    }
    dispatch_async(dispatch_get_main_queue(), ^{
        static NSSharingServicePicker *picker; // retained while its popover is open
        NSWindow *window = NSApp.keyWindow ?: NSApp.mainWindow ?: NSApp.windows.firstObject;
        NSView *view = window.contentView;
        if (!view) return;
        double top = view.isFlipped ? y : view.bounds.size.height - y;
        picker = [[NSSharingServicePicker alloc] initWithItems:urls];
        [picker showRelativeToRect:NSMakeRect(x, top, 1, 1) ofView:view preferredEdge:NSRectEdgeMinY];
    });
    return 1;
#endif
}
