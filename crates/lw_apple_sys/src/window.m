#import <AppKit/AppKit.h>
#include "platform.h"

int lw_window_open_link(int kind) {
    if (!NSThread.isMainThread || !NSApp || (kind != 0 && kind != 1)) return 0;
    @autoreleasepool {
        NSString *link = kind == 0 ? @"https://github.com/olegpustovit/localwave/blob/dev/docs/privacy.md" : @"https://github.com/olegpustovit/localwave/issues";
        return [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:link]];
    }
}
