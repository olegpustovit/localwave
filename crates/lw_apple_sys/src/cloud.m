#import <Foundation/Foundation.h>
#include <stdlib.h>
#include <string.h>
#include "platform.h"

static int archiveIdentity(id identity, char **encoded, char **error) {
    if (!encoded) return 0;
    *encoded = NULL;
    if (!identity) return 1;
    @try {
        NSError *failure = nil;
        NSData *data = [NSKeyedArchiver archivedDataWithRootObject:identity requiringSecureCoding:YES error:&failure];
        if (!data) {
            if (error) *error = strdup("Could not securely archive the iCloud identity");
            return 0;
        }
        *encoded = strdup([data base64EncodedStringWithOptions:0].UTF8String);
        if (!*encoded && error) *error = strdup("Could not allocate the iCloud identity");
        return *encoded != NULL;
    } @catch (NSException *exception) {
        if (error) *error = strdup("Could not securely archive the iCloud identity");
        return 0;
    }
}

int lw_cloud_identity(char **encoded, char **error) {
    @autoreleasepool {
        @try { return archiveIdentity(NSFileManager.defaultManager.ubiquityIdentityToken, encoded, error); }
        @catch (NSException *exception) {
            if (encoded) *encoded = NULL;
            if (error) *error = strdup("Could not read the iCloud identity");
            return 0;
        }
    }
}

#ifdef LW_INTEROP_TEST
int lw_cloud_identity_fixture(int kind, char **encoded, char **error) {
    @autoreleasepool {
        id identity = kind == 0 ? nil : kind == 1 ? @"test-fixture-token-名前🦊" : [NSObject new];
        return archiveIdentity(identity, encoded, error);
    }
}
#endif
