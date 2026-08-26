/* Allocator fault-injector for ERRORS.md row E17.
 *
 * LD_PRELOAD'ed into a child process so that `checkshift`'s
 * `malloc(sizeof(ComputeState))` can be made to fail on demand, letting the
 * differential test actually execute the `state == NULL` branch in BOTH
 * shared objects instead of merely reasoning about it.
 *
 * The failure window is opened explicitly by the child (arm/disarm) so that
 * unrelated allocations - dlopen, the Rust std runtime, stdio buffers - are
 * never disturbed.
 *
 * This file is test scaffolding; it is NOT part of the library under test and
 * lives outside c_src/.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

static void *(*real_malloc)(size_t) = NULL;
static int armed = 0;
static size_t target_size = 0;

/* Resolve the real malloc up front, at load time, so we never have to call
 * dlsym() from inside malloc() (dlsym may itself allocate). */
__attribute__((constructor)) static void failmalloc_init(void) {
    real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
}

/* Start failing allocations of exactly `sz` bytes. */
void arm_fail_malloc(size_t sz) {
    target_size = sz;
    armed = 1;
}

/* Stop failing allocations. */
void disarm_fail_malloc(void) { armed = 0; }

void *malloc(size_t sz) {
    if (real_malloc == NULL) {
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    }
    if (armed && sz == target_size) {
        return NULL;
    }
    return real_malloc(sz);
}
