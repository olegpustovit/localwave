#include <TargetConditionals.h>
#if TARGET_OS_IOS
#import <UIKit/UIKit.h>
#import <AVKit/AVKit.h>
#else
#import <AppKit/AppKit.h>
#import <AVKit/AVKit.h>
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

// Swift's WaveAudioRoute: the system audio route picker from a 44-point key at (x, y) in window points (top-left origin). iOS only:
// a transient AVRoutePickerView presents its sheet from its own button and is removed once it is shown. macOS has no route key.
int lw_window_route_picker(double x, double y) {
#if TARGET_OS_IOS
    if (!NSThread.isMainThread) return 0;
    @autoreleasepool {
        UIWindow *window = nil;
        for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
            if (![scene isKindOfClass:UIWindowScene.class]) continue;
            for (UIWindow *candidate in ((UIWindowScene *)scene).windows) if (candidate.isKeyWindow) window = candidate;
        }
        if (!window) return 0;
        AVRoutePickerView *picker = [[AVRoutePickerView alloc] initWithFrame:CGRectMake(x, y, 44, 44)];
        picker.alpha = 0.011;
        [window addSubview:picker];
        UIButton *button = nil;
        for (UIView *view in picker.subviews) if ([view isKindOfClass:UIButton.class]) button = (UIButton *)view;
        if (!button) { [picker removeFromSuperview]; return 0; }
        [button sendActionsForControlEvents:UIControlEventTouchUpInside];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(NSEC_PER_SEC)), dispatch_get_main_queue(), ^{ [picker removeFromSuperview]; });
        return 1;
    }
#else
    if (!NSThread.isMainThread) return 0;
    @autoreleasepool {
        NSWindow *window = NSApp.keyWindow ?: NSApp.mainWindow;
        NSView *content = window.contentView;
        if (!content) return 0;
        // one picker per window, moved under the key on each press: its menu anchors to it, so it stays
        static AVRoutePickerView *picker = nil;
        if (!picker) { picker = [[AVRoutePickerView alloc] initWithFrame:NSMakeRect(0, 0, 44, 44)]; picker.alphaValue = 0.011; }
        if (picker.superview != content) { [picker removeFromSuperview]; [content addSubview:picker]; }
        CGFloat top = content.isFlipped ? y : NSHeight(content.bounds) - y - 44;
        picker.frame = NSMakeRect(x, top, 44, 44);
        NSButton *button = nil;
        NSMutableArray<NSView *> *views = [NSMutableArray arrayWithObject:picker];
        while (views.count && !button) {
            NSView *view = views.lastObject;
            [views removeLastObject];
            if ([view isKindOfClass:NSButton.class]) button = (NSButton *)view; else [views addObjectsFromArray:view.subviews];
        }
        if (!button) return 0;
        [button performClick:nil];
        return 1;
    }
#endif
}
