/* LD_PRELOAD helper that makes exactly one malloc() of a chosen size fail.
 *
 * This is what lets the Phase C tests reach the two allocation-failure branches
 * of create_state() (lib.c:60 and lib.c:78) -- and therefore confusion()'s
 * `return -1` at lib.c:188 -- which are otherwise unreachable through the
 * public ABI. It calls glibc's public __libc_malloc directly, so no dlsym (and
 * hence no re-entrant allocation) is needed.
 */
#include <stddef.h>

extern void *__libc_malloc(size_t);
extern void __libc_free(void *);

static size_t armed_size;
static int armed;
static int fired;

/* Allocation trace, so the two implementations can be compared on *allocator*
 * behaviour too: same number of malloc/free calls, same total bytes. A leak, a
 * double free or a differently-sized allocation shows up as a mismatch. */
static unsigned long n_malloc, n_free, n_bytes;

void oom_reset(void) { n_malloc = 0; n_free = 0; n_bytes = 0; }
unsigned long oom_mallocs(void) { return n_malloc; }
unsigned long oom_frees(void) { return n_free; }
unsigned long oom_bytes(void) { return n_bytes; }

void oom_arm(size_t size) {
    armed_size = size;
    armed = 1;
    fired = 0;
}

void oom_disarm(void) { armed = 0; }

int oom_fired(void) { return fired; }

void *malloc(size_t size) {
    if (armed && size == armed_size) {
        armed = 0;
        fired = 1;
        return NULL;
    }
    n_malloc++;
    n_bytes += size;
    return __libc_malloc(size);
}

void free(void *p) {
    if (p) n_free++;
    __libc_free(p);
}
