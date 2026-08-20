/*
 * A surgical `malloc` interposer used by the differential tests to reach the
 * allocation-failure branches of the library (ERRORS.md rows 1, 10 and 41):
 *
 *   shape.c:177   if (!shapes[i]) { fprintf(stderr, "Error: Failed to allocate
 *                                   shape\n"); exit(1); }
 *   scene.c:33    if (!scene) return NULL;
 *   main.c:83     else printf("Error creating scene\n");
 *
 * Only allocations of one exact size fail, so the surrounding harness (and the
 * Rust standard library inside the harness) keeps working:
 *
 *   FAILMALLOC_SIZE  = the byte size that must fail (2444 = sizeof(shape_t),
 *                      472 = sizeof(scene_t))
 *   FAILMALLOC_AFTER = how many allocations of that size still succeed first
 *
 * This file is test scaffolding; it is not part of the translated program and
 * lives outside c_src/.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>

static void *(*real_malloc)(size_t) = NULL;
static size_t fail_size = 0;
static long fail_after = 0;
static long seen = 0;
static int initialised = 0;

static void failmalloc_init(void)
{
    initialised = 1;
    real_malloc = (void *(*)(size_t))dlsym(RTLD_NEXT, "malloc");
    const char *s = getenv("FAILMALLOC_SIZE");
    if (s) {
        fail_size = (size_t)strtoul(s, NULL, 10);
    }
    const char *a = getenv("FAILMALLOC_AFTER");
    if (a) {
        fail_after = strtol(a, NULL, 10);
    }
}

void *malloc(size_t size)
{
    if (!initialised) {
        failmalloc_init();
    }
    if (fail_size != 0 && size == fail_size) {
        if (seen++ >= fail_after) {
            return NULL;
        }
    }
    return real_malloc(size);
}
