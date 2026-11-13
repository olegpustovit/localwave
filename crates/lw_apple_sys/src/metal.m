#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#include "platform.h"

@interface LWMetalFrames : NSObject {
@public
    CAMetalLayer *layer;
    id<MTLDevice> device;
    id<MTLCommandQueue> queue;
    MTLRenderPassDescriptor *pass;
    id<MTLCommandBuffer> pending[3];
    uint64_t serials[3];
    uint64_t serial;
    uint64_t completed;
    id<CAMetalDrawable> drawable;
    id<MTLCommandBuffer> commands;
    id<MTLRenderCommandEncoder> encoder;
}
@end
@implementation LWMetalFrames
@end

void *lw_metal_create(void *layerPointer, void *devicePointer, void *queuePointer) {
    @autoreleasepool {
        if (![NSThread isMainThread] || !layerPointer || !devicePointer || !queuePointer) return NULL;
        LWMetalFrames *frames = [LWMetalFrames new];
        frames->layer = (__bridge CAMetalLayer *)layerPointer;
        frames->device = (__bridge id<MTLDevice>)devicePointer;
        frames->queue = (__bridge id<MTLCommandQueue>)queuePointer;
        frames->layer.device = frames->device;
        frames->layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
        frames->layer.framebufferOnly = YES;
        frames->layer.maximumDrawableCount = 3;
        frames->layer.displaySyncEnabled = YES;
        frames->pass = [MTLRenderPassDescriptor new];
        return (__bridge_retained void *)frames;
    }
}

int lw_metal_begin(void *raw, uint32_t width, uint32_t height) {
    @autoreleasepool {
        LWMetalFrames *frames = (__bridge LWMetalFrames *)raw;
        if (![NSThread isMainThread] || frames->commands || !width || !height) return -1;
        const NSUInteger slot = frames->serial % 3;
        id<MTLCommandBuffer> previous = frames->pending[slot];
        if (previous) {
            [previous waitUntilCompleted];
            if (previous.status == MTLCommandBufferStatusError) return -1;
            frames->completed = MAX(frames->completed, frames->serials[slot]);
            frames->pending[slot] = nil;
        }
        const CGSize size = CGSizeMake(width, height);
        if (!CGSizeEqualToSize(frames->layer.drawableSize, size)) frames->layer.drawableSize = size;
        frames->drawable = [frames->layer nextDrawable];
        if (!frames->drawable) return 0;
        frames->commands = [frames->queue commandBuffer];
        if (!frames->commands) { frames->drawable = nil; return -1; }
        frames->serial++;
        return 1;
    }
}

uint64_t lw_metal_serial(void *raw) { return ((__bridge LWMetalFrames *)raw)->serial; }
uint64_t lw_metal_completed(void *raw) { return ((__bridge LWMetalFrames *)raw)->completed; }
void *lw_metal_commands(void *raw) { return (__bridge void *)((__bridge LWMetalFrames *)raw)->commands; }

void *lw_metal_pass(void *raw, double red, double green, double blue) {
    @autoreleasepool {
        LWMetalFrames *frames = (__bridge LWMetalFrames *)raw;
        if (![NSThread isMainThread] || !frames->commands || frames->encoder) return NULL;
        MTLRenderPassColorAttachmentDescriptor *color = frames->pass.colorAttachments[0];
        color.texture = frames->drawable.texture;
        color.loadAction = MTLLoadActionClear;
        color.storeAction = MTLStoreActionStore;
        color.clearColor = MTLClearColorMake(red, green, blue, 1);
        frames->encoder = [frames->commands renderCommandEncoderWithDescriptor:frames->pass];
        return (__bridge void *)frames->encoder;
    }
}

int lw_metal_submit(void *raw) {
    @autoreleasepool {
        LWMetalFrames *frames = (__bridge LWMetalFrames *)raw;
        if (![NSThread isMainThread] || !frames->commands || !frames->encoder) return 0;
        [frames->encoder endEncoding];
        frames->encoder = nil;
        [frames->commands presentDrawable:frames->drawable];
        [frames->commands commit];
        const NSUInteger slot = (frames->serial - 1) % 3;
        frames->pending[slot] = frames->commands;
        frames->serials[slot] = frames->serial;
        frames->commands = nil;
        frames->drawable = nil;
        frames->pass.colorAttachments[0].texture = nil;
        return 1;
    }
}

int lw_metal_wait(void *raw) {
    @autoreleasepool {
        LWMetalFrames *frames = (__bridge LWMetalFrames *)raw;
        BOOL success = YES;
        for (NSUInteger i = 0; i < 3; i++) {
            [frames->pending[i] waitUntilCompleted];
            if (frames->pending[i].status == MTLCommandBufferStatusError) success = NO;
            frames->completed = MAX(frames->completed, frames->serials[i]);
            frames->pending[i] = nil;
        }
        return success ? 1 : 0;
    }
}

void lw_metal_destroy(void *raw) {
    @autoreleasepool {
        LWMetalFrames *frames = (__bridge_transfer LWMetalFrames *)raw;
        [frames->encoder endEncoding];
        frames->encoder = nil;
        lw_metal_wait((__bridge void *)frames);
    }
}
