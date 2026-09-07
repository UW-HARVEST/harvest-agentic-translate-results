#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <unistd.h>

int fseek(FILE *stream, long offset, int whence) {
    static int (*real_fseek)(FILE *, long, int);
    if (!real_fseek) {
        real_fseek = dlsym(RTLD_NEXT, "fseek");
    }
    usleep(100000);
    return real_fseek(stream, offset, whence);
}
