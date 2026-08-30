/* Loads the shared object named by argv[1] with RTLD_LOCAL (exactly how the
 * differential harness loads it), then calls both exported entry points. */
#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv) {
    void *h;
    void (*drv)(char);
    void (*phcl)(char);
    if (argc < 2) { fprintf(stderr, "usage: %s <lib.so>\n", argv[0]); return 2; }
    h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
    drv  = (void (*)(char))dlsym(h, "driver");
    phcl = (void (*)(char))dlsym(h, "printHexCharLine");
    if (!drv || !phcl) { fprintf(stderr, "dlsym: %s\n", dlerror()); return 1; }
    drv(0x41);
    phcl(0x42);
    fflush(stdout);
    return 0;
}
