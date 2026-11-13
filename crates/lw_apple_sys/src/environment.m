#import <AppKit/AppKit.h>
#include "platform.h"

static char LWAppearanceContext;
@interface LWEnvironment : NSObject {
@public
    LWEnvironmentState state;
    void (*wake)(void);
    NSApplication *application;
    NSWorkspace *workspace;
    id displayObserver;
}
- (instancetype)initWithApplication:(NSApplication *)app workspace:(NSWorkspace *)space wake:(void (*)(void))callback;
- (void)refresh;
@end

@implementation LWEnvironment
- (instancetype)initWithApplication:(NSApplication *)app workspace:(NSWorkspace *)space wake:(void (*)(void))callback {
    self = [super init];
    if (self) {
        application = app;
        workspace = space;
        [self refresh];
        wake = callback;
        [application addObserver:self forKeyPath:@"effectiveAppearance" options:0 context:&LWAppearanceContext];
        __weak LWEnvironment *owner = self;
        displayObserver = [workspace.notificationCenter addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:workspace queue:nil usingBlock:^(NSNotification *note) {
            (void)note;
            [owner refresh];
        }];
    }
    return self;
}
- (void)refresh {
    if (!NSThread.isMainThread) {
        __weak LWEnvironment *owner = self;
        dispatch_async(dispatch_get_main_queue(), ^{ [owner refresh]; });
        return;
    }
    NSString *match = [application.effectiveAppearance bestMatchFromAppearancesWithNames:@[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]];
    LWEnvironmentState next = { [match isEqual:NSAppearanceNameDarkAqua] ? 2u : [match isEqual:NSAppearanceNameAqua] ? 1u : 0u, workspace.accessibilityDisplayShouldReduceMotion ? 1u : 0u };
    if (next.appearance != state.appearance || next.reduced_motion != state.reduced_motion) {
        state = next;
        if (wake) wake();
    }
}
- (void)observeValueForKeyPath:(NSString *)keyPath ofObject:(id)object change:(NSDictionary *)change context:(void *)context {
    if (context == &LWAppearanceContext) [self refresh];
    else [super observeValueForKeyPath:keyPath ofObject:object change:change context:context];
}
- (void)dealloc {
    [application removeObserver:self forKeyPath:@"effectiveAppearance" context:&LWAppearanceContext];
    [workspace.notificationCenter removeObserver:displayObserver];
}
@end

void *lw_environment_create(void (*wake)(void)) {
    if (!NSThread.isMainThread || !NSApp) return NULL;
    @autoreleasepool { return (__bridge_retained void *)[[LWEnvironment alloc] initWithApplication:NSApp workspace:NSWorkspace.sharedWorkspace wake:wake]; }
}
void lw_environment_read(void *raw, LWEnvironmentState *state) {
    *state = ((__bridge LWEnvironment *)raw)->state;
}
void lw_environment_release(void *raw) {
    @autoreleasepool { (void)CFBridgingRelease(raw); }
}

#ifdef LW_INTEROP_TEST
// These objects affect only this test process, never global preferences or another LocalWave instance.
@interface LWTestWorkspace : NSObject
@property BOOL accessibilityDisplayShouldReduceMotion;
@property(strong) NSNotificationCenter *notificationCenter;
@end
@implementation LWTestWorkspace
@end
static unsigned testWakes;
static void testWake(void) { ++testWakes; }
int lw_environment_test_notifications(void) {
    if (!NSThread.isMainThread || !NSApp) return 0;
    @autoreleasepool {
        NSAppearance *original = NSApp.appearance;
        LWTestWorkspace *space = [LWTestWorkspace new];
        space.notificationCenter = [NSNotificationCenter new];
        LWEnvironment *monitor = [[LWEnvironment alloc] initWithApplication:NSApp workspace:(NSWorkspace *)space wake:testWake];
        NSApp.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
        unsigned baseline = testWakes;
        BOOL passed = monitor->state.appearance == 1 && monitor->state.reduced_motion == 0;
        space.accessibilityDisplayShouldReduceMotion = YES;
        [space.notificationCenter postNotificationName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:space];
        passed &= monitor->state.reduced_motion == 1 && testWakes == baseline + 1;
        [space.notificationCenter postNotificationName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:space];
        passed &= testWakes == baseline + 1;
        space.accessibilityDisplayShouldReduceMotion = NO;
        [space.notificationCenter postNotificationName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:space];
        passed &= monitor->state.reduced_motion == 0 && testWakes == baseline + 2;
        NSApp.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
        passed &= monitor->state.appearance == 2 && testWakes == baseline + 3;
        monitor = nil;
        NSApp.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
        [space.notificationCenter postNotificationName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification object:space];
        passed &= testWakes == baseline + 3;
        NSApp.appearance = original;
        return passed;
    }
}
#endif
