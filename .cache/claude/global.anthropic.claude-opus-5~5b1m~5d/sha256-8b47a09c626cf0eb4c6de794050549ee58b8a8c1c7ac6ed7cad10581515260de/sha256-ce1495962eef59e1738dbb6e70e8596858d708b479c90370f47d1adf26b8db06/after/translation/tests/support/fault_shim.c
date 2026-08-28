/*
 * Fault-injection shim for the Phase C error-path differential tests.
 *
 * Both the C `.so` and the Rust `.so` reach their two error branches only
 * through libc: `malloc(50)` returning NULL, and `strncmp("VALID","VALID",5)`
 * returning non-zero. Neither is reachable from the public arguments, so this
 * library is LD_PRELOADed to interpose those two calls. Because it sits ahead of
 * libc in the symbol search order it affects *both* implementations equally,
 * which is exactly what makes the comparison a fair differential test.
 *
 * Fault injection is off until harvest_shim_arm() is called, so process
 * start-up, dlopen() and the test harness itself are untouched. The scope is
 * deliberately as narrow as possible:
 *   - malloc: fails only for a request of exactly 50 bytes;
 *   - strncmp: reports a mismatch only for n == 5 with both operands "VALID".
 *
 * Built at test time with:
 *   cc -shared -fPIC -O2 -o <profile>/harvest_fault_shim.so fault_shim.c -ldl
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stddef.h>

static void *(*real_malloc)(size_t);
static void (*real_free)(void *);
static int (*real_strncmp)(const char *, const char *, size_t);

static int fail_malloc50;
static int fail_strncmp;

/* Counters so the tests can prove the interposed call really happened. */
static unsigned long malloc50_hits;
static unsigned long strncmp_hits;
static unsigned long free_hits;
static unsigned long free50_hits;

/* Outstanding 50-byte allocations, so free() can tell whether the block the
 * library asked for is the block it releases. This is what proves both
 * implementations really call free() and neither leaks nor double-frees. */
#define LIVE50_MAX 32
static void *live50[LIVE50_MAX];

static void live50_add(void *p) {
    for (int i = 0; i < LIVE50_MAX; i++) {
        if (live50[i] == NULL) {
            live50[i] = p;
            return;
        }
    }
}

/* Returns 1 and forgets `p` if it was a tracked 50-byte block. */
static int live50_take(const void *p) {
    for (int i = 0; i < LIVE50_MAX; i++) {
        if (live50[i] == p) {
            live50[i] = NULL;
            return 1;
        }
    }
    return 0;
}

/* Emergency arena, used only if malloc is called before the constructor has
 * resolved the real one (should never happen, but recursing through dlsym would
 * be fatal). free() range-checks against it. */
static char arena[1 << 16];
static size_t arena_off;
static int resolving;

static void *arena_alloc(size_t n) {
    size_t a = (n + 15u) & ~(size_t)15u;
    if (a == 0) a = 16;
    if (arena_off + a > sizeof arena) return NULL;
    void *p = arena + arena_off;
    arena_off += a;
    return p;
}

static int from_arena(const void *p) {
    return (const char *)p >= arena && (const char *)p < arena + sizeof arena;
}

__attribute__((constructor)) static void shim_init(void) {
    if (!real_malloc) real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    if (!real_free) real_free = (void (*)(void *))dlsym(RTLD_NEXT, "free");
    if (!real_strncmp)
        real_strncmp = (int (*)(const char *, const char *, size_t))dlsym(RTLD_NEXT, "strncmp");
}

/* --- control surface, called by the test through dlsym ------------------- */

void harvest_shim_arm(int malloc50, int strncmp_mismatch) {
    fail_malloc50 = malloc50;
    fail_strncmp = strncmp_mismatch;
}

unsigned long harvest_shim_malloc50_hits(void) { return malloc50_hits; }
unsigned long harvest_shim_strncmp_hits(void) { return strncmp_hits; }
unsigned long harvest_shim_free_hits(void) { return free_hits; }
unsigned long harvest_shim_free50_hits(void) { return free50_hits; }

void harvest_shim_reset_counters(void) {
    malloc50_hits = 0;
    strncmp_hits = 0;
    free_hits = 0;
    free50_hits = 0;
}

int harvest_shim_present(void) { return 1; }

/* --- interposed libc ----------------------------------------------------- */

void *malloc(size_t n) {
    if (!real_malloc) {
        if (resolving) return arena_alloc(n);
        resolving = 1;
        real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
        resolving = 0;
        if (!real_malloc) return arena_alloc(n);
    }
    if (n == 50) {
        malloc50_hits++;
        if (fail_malloc50) return NULL;
        void *p = real_malloc(n);
        if (p) live50_add(p);
        return p;
    }
    return real_malloc(n);
}

void free(void *p) {
    if (p == NULL) return;
    if (from_arena(p)) return;
    free_hits++;
    if (live50_take(p)) free50_hits++;
    if (!real_free) {
        real_free = (void (*)(void *))dlsym(RTLD_NEXT, "free");
        if (!real_free) return;
    }
    real_free(p);
}

int strncmp(const char *a, const char *b, size_t n) {
    if (!real_strncmp) {
        real_strncmp = (int (*)(const char *, const char *, size_t))dlsym(RTLD_NEXT, "strncmp");
        if (!real_strncmp) return 0;
    }
    if (n == 5 && a != NULL && b != NULL && real_strncmp(a, "VALID", 6) == 0 &&
        real_strncmp(b, "VALID", 6) == 0) {
        strncmp_hits++;
        if (fail_strncmp) return 1;
    }
    return real_strncmp(a, b, n);
}
