#import <Foundation/Foundation.h>
#include "platform.h"

// Callbacks only forward to `notify`; `active` is cleared under `lock` before unregistering, so
// no callback can touch `context` once invalidate returns.
@interface LWSourcePresenter : NSObject <NSFilePresenter>
@property(nonatomic, strong) NSURL *url;
@property(nonatomic, strong) NSOperationQueue *queue;
@end

@implementation LWSourcePresenter {
    NSLock *_lock;
    BOOL _active;
    void (*_notify)(void *);
    void *_context;
}
- (instancetype)initWithURL:(NSURL *)url notify:(void (*)(void *))notify context:(void *)context {
    if ((self = [super init])) {
        _url = url;
        _notify = notify;
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

void *lw_presenter_create(const char *path, void (*notify)(void *), void *context) {
    NSURL *url = [NSURL fileURLWithFileSystemRepresentation:path isDirectory:YES relativeToURL:nil];
    LWSourcePresenter *presenter = [[LWSourcePresenter alloc] initWithURL:url notify:notify context:context];
    [NSFileCoordinator addFilePresenter:presenter];
    return (__bridge_retained void *)presenter;
}

void lw_presenter_release(void *raw) {
    LWSourcePresenter *presenter = (__bridge_transfer LWSourcePresenter *)raw;
    [presenter invalidate];
}

int lw_presenter_write(const char *path, const unsigned char *bytes, size_t len) {
    NSURL *url = [NSURL fileURLWithFileSystemRepresentation:path isDirectory:NO relativeToURL:nil];
    __block int ok = 0;
    NSError *error = nil;
    [[[NSFileCoordinator alloc] initWithFilePresenter:nil] coordinateWritingItemAtURL:url options:NSFileCoordinatorWritingForReplacing error:&error byAccessor:^(NSURL *writable) {
        ok = [[NSData dataWithBytes:bytes length:len] writeToURL:writable atomically:NO];
    }];
    return error == nil && ok;
}

// Test hook: runs every presenter callback directly, as the coordinator would on the queue.
void lw_presenter_test_fire(void *raw) {
    LWSourcePresenter *p = (__bridge LWSourcePresenter *)raw;
    NSURL *url = p.url;
    [p presentedItemDidChange];
    [p presentedSubitemDidAppearAtURL:url];
    [p presentedSubitemDidChangeAtURL:url];
    [p presentedSubitemAtURL:url didMoveToURL:url];
    [p presentedItemDidMoveToURL:url];
    [p accommodatePresentedSubitemDeletionAtURL:url completionHandler:^(NSError *error) { (void)error; }];
}

size_t lw_presenter_test_count(void) { return NSFileCoordinator.filePresenters.count; }
