/*
 * Malloc interposer used by the Phase C allocation-failure rows (E5, E6, E7).
 *
 * `gotomach` clamps `iterations` to [0, 65535], so its largest request is
 * 262 140 bytes: the malloc-failure branches cannot be reached by choosing
 * argument values. This shim makes the Nth malloc issued *inside* a single
 * `gotomach` call return NULL, so each branch can be driven deterministically
 * and the C and Rust libraries can be compared on it.
 *
 * It is only ever built into translation/target/phase_c/ at test time; it is
 * not part of the library under test, and nothing in c_src/ is touched.
 *
 * `__libc_malloc` is used instead of dlsym(RTLD_NEXT, "malloc") so that the
 * shim never re-enters the allocator during its own initialisation.
 */
#include <stddef.h>

extern void *__libc_malloc(size_t);

static long fm_count = 0;    /* mallocs seen while armed */
static long fm_fail_at = 0;  /* 1-based index to fail; 0 => never fail */
static int fm_armed = 0;

void fm_arm(long fail_at) {
    fm_count = 0;
    fm_fail_at = fail_at;
    fm_armed = 1;
}

long fm_disarm(void) {
    fm_armed = 0;
    fm_fail_at = 0;
    return fm_count;
}

void *malloc(size_t n) {
    if (fm_armed) {
        fm_count++;
        if (fm_fail_at > 0 && fm_count == fm_fail_at) {
            return NULL;
        }
    }
    return __libc_malloc(n);
}
