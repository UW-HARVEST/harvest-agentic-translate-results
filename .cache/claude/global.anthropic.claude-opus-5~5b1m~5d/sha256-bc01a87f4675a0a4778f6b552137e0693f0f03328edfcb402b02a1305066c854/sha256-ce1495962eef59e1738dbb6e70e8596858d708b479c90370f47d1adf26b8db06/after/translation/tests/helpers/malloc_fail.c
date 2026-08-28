/* LD_PRELOAD malloc interposer used to reach checkshift's allocation-failure
 * guard (ERRORS.md row E16), which is otherwise unreachable because the
 * allocation is only sizeof(ComputeState) == 12 bytes.
 *
 * Failure is OFF until the driver calls mf_set_fail_size(), so that dlopen()
 * and the loaded library's initialisers are unaffected.
 *
 * Uses glibc's __libc_malloc directly instead of dlsym(RTLD_NEXT), avoiding any
 * risk of recursing into malloc during symbol resolution.
 */
#include <stddef.h>

extern void *__libc_malloc(size_t);

static size_t fail_size = 0;
static int    fail_armed = 0;

void mf_set_fail_size(size_t size) {
    fail_size = size;
    fail_armed = (size != 0);
}

void *malloc(size_t size) {
    if (fail_armed && size == fail_size) {
        return NULL;
    }
    return __libc_malloc(size);
}
