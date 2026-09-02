/* LD_PRELOAD allocation-failure interposer for ERRORS.md rows 3-5.
 *
 * Fails ONLY allocations whose requested size matches an exact value given in
 * the environment, so the injected failure lands precisely on the
 * malloc/realloc/strdup call site under test and on nothing else in the
 * process. Every other allocation is forwarded to the real allocator.
 *
 *   FAILMALLOC_SIZE=<n>   -> malloc(n)      returns NULL (ENOMEM)
 *   FAILREALLOC_SIZE=<n>  -> realloc(p, n)  returns NULL (ENOMEM)
 *
 * `strdup` is covered implicitly: glibc's strdup calls the (interposed) malloc
 * with strlen(s)+1.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stddef.h>
#include <stdlib.h>

static void *(*real_malloc)(size_t);
static void *(*real_realloc)(void *, size_t);
static size_t fail_malloc_size;
static size_t fail_realloc_size;
static int initialized;

static void init(void)
{
    if (initialized) {
        return;
    }
    real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    real_realloc = (void *(*)(void *, size_t))dlsym(RTLD_NEXT, "realloc");
    const char *e = getenv("FAILMALLOC_SIZE");
    if (e != NULL) {
        fail_malloc_size = (size_t)strtoull(e, NULL, 10);
    }
    e = getenv("FAILREALLOC_SIZE");
    if (e != NULL) {
        fail_realloc_size = (size_t)strtoull(e, NULL, 10);
    }
    if (real_malloc != NULL && real_realloc != NULL) {
        initialized = 1;
    }
}

__attribute__((constructor)) static void shim_ctor(void)
{
    init();
}

void *malloc(size_t n)
{
    if (real_malloc == NULL) {
        init();
    }
    if (fail_malloc_size != 0 && n == fail_malloc_size) {
        errno = ENOMEM;
        return NULL;
    }
    return real_malloc(n);
}

void *realloc(void *p, size_t n)
{
    if (real_realloc == NULL) {
        init();
    }
    if (fail_realloc_size != 0 && n == fail_realloc_size) {
        errno = ENOMEM;
        return NULL;
    }
    return real_realloc(p, n);
}
