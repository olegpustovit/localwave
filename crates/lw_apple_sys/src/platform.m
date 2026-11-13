#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <errno.h>
#include <pthread/qos.h>
#include <os/signpost.h>
#include "platform.h"

char *lw_normalized_file_url(const char *path) {
    @autoreleasepool {
        NSString *text = path ? [NSString stringWithUTF8String:path] : nil;
        if (!text || !text.isAbsolutePath) return NULL;
        NSURL *url = [NSURL fileURLWithPath:text isDirectory:NO].standardizedURL;
        NSURLComponents *components = [NSURLComponents componentsWithURL:url resolvingAgainstBaseURL:NO];
        if (!components) return NULL;
        NSString *normalized = components.path;
        while (normalized.length > 1 && [normalized hasSuffix:@"/"]) {
            normalized = [normalized substringToIndex:normalized.length - 1];
        }
        components.path = normalized;
        const char *result = components.URL.absoluteString.UTF8String;
        return result ? strdup(result) : NULL;
    }
}

void lw_free_string(char *text) { free(text); }

int lw_is_main_thread(void) { return NSThread.isMainThread; }

uint64_t lw_thread_cpu_ns(void) {
    struct timespec time;
    if (clock_gettime(CLOCK_THREAD_CPUTIME_ID, &time) != 0) return UINT64_MAX;
    return (uint64_t)time.tv_sec * 1000000000 + (uint64_t)time.tv_nsec;
}

int lw_ui_thread_priority(void) {
    if (!NSThread.isMainThread) return EINVAL;
    return pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
}

static os_log_t lwFrameLog;
uint64_t lw_frame_interval_begin(uint64_t frame) {
    static dispatch_once_t once;
    dispatch_once(&once, ^{ lwFrameLog = os_log_create("com.olegpustovit.localwave", OS_LOG_CATEGORY_POINTS_OF_INTEREST); });
    if (!os_signpost_enabled(lwFrameLog)) return OS_SIGNPOST_ID_NULL;
    os_signpost_id_t interval = os_signpost_id_generate(lwFrameLog);
    os_signpost_interval_begin(lwFrameLog, interval, "UI build", "frame=%llu", (unsigned long long)frame);
    return interval;
}
void lw_frame_interval_end(uint64_t interval, uint64_t wall, uint64_t cpu) {
    if (interval != OS_SIGNPOST_ID_NULL) os_signpost_interval_end(lwFrameLog, interval, "UI build", "wall_us=%llu cpu_us=%llu qos=%u", (unsigned long long)wall, (unsigned long long)cpu, (unsigned int)qos_class_self());
}
