/* Minimal external consumer: dlopen a libdriver.so, call driver() over a list
 * of ints read from argv, and exit WITHOUT any explicit fflush -- so the
 * at-exit flushing behaviour of the library's chosen stdout is part of the
 * comparison. */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s <so> [ints...]\n", argv[0]); return 2; }
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 3; }
    void (*driver)(int) = (void (*)(int))dlsym(h, "driver");
    if (!driver) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 4; }
    for (int i = 2; i < argc; i++) driver((int)strtol(argv[i], NULL, 0));
    return 0;  /* no fflush, no dlclose: exit-time flush is under test */
}
