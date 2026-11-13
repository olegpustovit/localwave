#import <AppKit/AppKit.h>
#import <QuartzCore/CALayer.h>
#include "platform.h"

static NSView *windowView(void *raw) {
    if (!NSThread.isMainThread || !raw) return nil;
    id delegate = ((__bridge CALayer *)raw).delegate;
    return [delegate isKindOfClass:NSView.class] ? delegate : nil;
}

int lw_window_configure(void *raw) {
    @autoreleasepool {
        NSWindow *window = windowView(raw).window;
        if (!window) return 0;
        window.styleMask |= NSWindowStyleMaskFullSizeContentView;
        window.titlebarAppearsTransparent = YES;
        window.titleVisibility = NSWindowTitleHidden;
        window.titlebarSeparatorStyle = NSTitlebarSeparatorStyleNone;
        window.collectionBehavior = (window.collectionBehavior & ~NSWindowCollectionBehaviorFullScreenAuxiliary) | NSWindowCollectionBehaviorFullScreenPrimary;
        window.contentMinSize = NSMakeSize(640, 480);
        return 1;
    }
}

int lw_window_read(void *raw, LWWindowState *state) {
    @autoreleasepool {
        NSView *view = windowView(raw);
        NSWindow *window = view.window;
        if (!window || !state) return 0;
        NSRect layout = [view convertRect:window.contentLayoutRect fromView:nil];
        double top = view.isFlipped ? NSMinY(layout) - NSMinY(view.bounds) : NSMaxY(view.bounds) - NSMaxY(layout);
        *state = (LWWindowState){ MAX(0, top), NSWidth(view.bounds), NSHeight(view.bounds), (window.styleMask & NSWindowStyleMaskResizable) != 0, [window standardWindowButton:NSWindowZoomButton].enabled, (window.styleMask & NSWindowStyleMaskFullSizeContentView) != 0, (window.collectionBehavior & NSWindowCollectionBehaviorFullScreenPrimary) != 0 };
        return 1;
    }
}
