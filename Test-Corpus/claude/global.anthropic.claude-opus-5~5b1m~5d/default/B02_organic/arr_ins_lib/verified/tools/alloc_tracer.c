/* LD_PRELOAD allocator interposer: logs every malloc/calloc/realloc/free that
 * happens while tracing is enabled, so the C and Rust libraries' *allocation
 * call sequences* can be compared, not just their results.
 *
 * Tracing is gated on the ARRINS_TRACE env var being set by the driver right
 * before it calls into the library under test.                              */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

static void *(*real_malloc)(size_t);
static void *(*real_realloc)(void *, size_t);
static void *(*real_calloc)(size_t, size_t);
static void  (*real_free)(void *);
static FILE *log_fp;
static int   in_hook;

__attribute__((constructor)) static void init(void) {
  real_malloc  = dlsym(RTLD_NEXT, "malloc");
  real_realloc = dlsym(RTLD_NEXT, "realloc");
  real_calloc  = dlsym(RTLD_NEXT, "calloc");
  real_free    = dlsym(RTLD_NEXT, "free");
}

static FILE *lg(void) {
  const char *p = getenv("ARRINS_TRACE");
  if (!p || !*p) return NULL;
  if (!log_fp) log_fp = fopen(p, "w");
  return log_fp;
}

void *malloc(size_t n) {
  if (!real_malloc) init();
  void *r = real_malloc(n);
  if (!in_hook) { in_hook = 1; FILE *f = lg(); if (f) fprintf(f, "malloc(%zu)\n", n); in_hook = 0; }
  return r;
}
void *calloc(size_t a, size_t b) {
  if (!real_calloc) init();
  void *r = real_calloc(a, b);
  if (!in_hook) { in_hook = 1; FILE *f = lg(); if (f) fprintf(f, "calloc(%zu,%zu)\n", a, b); in_hook = 0; }
  return r;
}
void *realloc(void *p, size_t n) {
  if (!real_realloc) init();
  void *r = real_realloc(p, n);
  if (!in_hook) { in_hook = 1; FILE *f = lg(); if (f) fprintf(f, "realloc(%s,%zu)\n", p ? "PTR" : "NULL", n); in_hook = 0; }
  return r;
}
void free(void *p) {
  if (!real_free) init();
  if (!in_hook) { in_hook = 1; FILE *f = lg(); if (f) fprintf(f, "free(%s)\n", p ? "PTR" : "NULL"); in_hook = 0; }
  real_free(p);
}
