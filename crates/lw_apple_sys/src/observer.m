#import <Foundation/Foundation.h>
#include <TargetConditionals.h>
#include "platform.h"

// One source folder's change stream. Callbacks only forward to `notify` on a private serial queue.
// Release never blocks its caller: the queue stops the stream, then calls `done(context)` last, so
// no callback follows `done`.
// macOS uses FSEvents: directory NSFilePresenters there miss most subitem changes and make the
// app's own coordinated reads wait on presenter registration.
#if TARGET_OS_OSX
#include <CoreServices/CoreServices.h>
#include <stdlib.h>

typedef struct {
    FSEventStreamRef stream;
    dispatch_queue_t queue;
    void (*notify)(void *);
    void (*done)(void *);
    void *context;
} LWObserver;

static void lw_observer_event(ConstFSEventStreamRef stream, void *info, size_t count, void *paths,
                              const FSEventStreamEventFlags flags[], const FSEventStreamEventId ids[]) {
    (void)stream; (void)count; (void)paths; (void)flags; (void)ids;
    LWObserver *observer = info;
    observer->notify(observer->context);
}

void *lw_observer_create(const char *path, void (*notify)(void *), void (*done)(void *), void *context) {
    @autoreleasepool {
        LWObserver *observer = calloc(1, sizeof(LWObserver));
        if (!observer) return NULL;
        observer->notify = notify;
        observer->done = done;
        observer->context = context;
        NSArray *paths = @[[NSString stringWithUTF8String:path]];
        FSEventStreamContext stream_context = {0, observer, NULL, NULL, NULL};
        observer->stream = FSEventStreamCreate(NULL, lw_observer_event, &stream_context,
                                               (__bridge CFArrayRef)paths, kFSEventStreamEventIdSinceNow,
                                               0.2, kFSEventStreamCreateFlagNoDefer);
        if (!observer->stream) { free(observer); return NULL; }
        observer->queue = dispatch_queue_create("localwave.source-observer", DISPATCH_QUEUE_SERIAL);
        FSEventStreamSetDispatchQueue(observer->stream, observer->queue);
        // Registration with fseventsd can block for seconds; the caller's worker never waits on it.
        FSEventStreamRef stream = observer->stream;
        dispatch_async(observer->queue, ^{ FSEventStreamStart(stream); });
        return observer;
    }
}

void lw_observer_release(void *raw) {
    LWObserver *observer = raw;
    // Ordered after any pending start and callback on the stream's queue.
    dispatch_async(observer->queue, ^{
        FSEventStreamStop(observer->stream);
        FSEventStreamInvalidate(observer->stream);
        FSEventStreamRelease(observer->stream);
        observer->done(observer->context);
        free(observer);
    });
}
#else
// shortcut: iOS presents the folder with NSFilePresenter; directory delivery is unqualified until I2.
@interface LWSourcePresenter : NSObject <NSFilePresenter>
@property(nonatomic, strong) NSURL *url;
@property(nonatomic, strong) NSOperationQueue *queue;
@end

@implementation LWSourcePresenter {
    NSLock *_lock;
    BOOL _active;
    void (*_notify)(void *);
    void (*_done)(void *);
    void *_context;
}
- (instancetype)initWithURL:(NSURL *)url notify:(void (*)(void *))notify done:(void (*)(void *))done context:(void *)context {
    if ((self = [super init])) {
        _url = url;
        _notify = notify;
        _done = done;
        _context = context;
        _lock = [NSLock new];
        _active = YES;
        _queue = [NSOperationQueue new];
        _queue.name = @"localwave.source-presenter";
        _queue.maxConcurrentOperationCount = 1;
    }
    return self;
}
- (NSURL *)presentedItemURL { return _url; }
- (NSOperationQueue *)presentedItemOperationQueue { return _queue; }
- (void)changed {
    [_lock lock];
    if (_active) _notify(_context);
    [_lock unlock];
}
- (void)invalidate {
    [_lock lock];
    BOOL was = _active;
    _active = NO;
    [_lock unlock];
    if (was) [NSFileCoordinator removeFilePresenter:self];
    _done(_context);
}
- (void)presentedItemDidChange { [self changed]; }
- (void)presentedSubitemDidAppearAtURL:(NSURL *)url { [self changed]; }
- (void)presentedSubitemAtURL:(NSURL *)url didMoveToURL:(NSURL *)newURL { [self changed]; }
- (void)presentedSubitemDidChangeAtURL:(NSURL *)url { [self changed]; }
- (void)presentedItemDidMoveToURL:(NSURL *)newURL { [self changed]; }
- (void)accommodatePresentedSubitemDeletionAtURL:(NSURL *)url completionHandler:(void (^)(NSError *))done {
    [self changed];
    done(nil);
}
@end

void *lw_observer_create(const char *path, void (*notify)(void *), void (*done)(void *), void *context) {
    NSURL *url = [NSURL fileURLWithFileSystemRepresentation:path isDirectory:YES relativeToURL:nil];
    LWSourcePresenter *presenter = [[LWSourcePresenter alloc] initWithURL:url notify:notify done:done context:context];
    [NSFileCoordinator addFilePresenter:presenter];
    return (__bridge_retained void *)presenter;
}

void lw_observer_release(void *raw) {
    LWSourcePresenter *presenter = (__bridge_transfer LWSourcePresenter *)raw;
    [presenter invalidate];
}
#endif
