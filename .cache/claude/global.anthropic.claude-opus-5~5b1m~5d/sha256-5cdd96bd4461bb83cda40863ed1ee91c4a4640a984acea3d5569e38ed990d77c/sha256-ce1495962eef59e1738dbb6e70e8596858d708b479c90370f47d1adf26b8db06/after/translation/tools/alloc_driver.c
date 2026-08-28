/* Drives one library through a fixed script while the tracer records every
 * allocator call. argv[1] = library path, argv[2] = scenario name.           */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>

typedef void  *(*arrgrowf_t)(void *, size_t, size_t, size_t);
typedef void   (*arrfreef_t)(void *);
typedef void   (*rand_seed_t)(size_t);
typedef void  *(*hmput_key_t)(void *, size_t, void *, size_t, int);
typedef void  *(*hmget_key_t)(void *, size_t, void *, size_t, int);
typedef void  *(*hmdel_key_t)(void *, size_t, void *, size_t, size_t, int);
typedef void  *(*hmput_default_t)(void *, size_t);
typedef void   (*hmfree_t)(void *, size_t);
typedef void  *(*shmode_t)(size_t, int);
typedef char  *(*stralloc_t)(void *, char *);
typedef void   (*strreset_t)(void *);
typedef void   (*arr_ins_t)(int);

struct arena { void *storage; size_t remaining; unsigned char block, mode; };

int main(int argc, char **argv) {
  void *h = dlopen(argv[1], RTLD_NOW);
  if (!h) { fprintf(stderr, "%s\n", dlerror()); return 1; }
  const char *sc = argv[2];

  arrgrowf_t  arrgrowf  = (arrgrowf_t)  dlsym(h, "stbds_arrgrowf");
  arrfreef_t  arrfreef  = (arrfreef_t)  dlsym(h, "stbds_arrfreef");
  rand_seed_t rand_seed = (rand_seed_t) dlsym(h, "stbds_rand_seed");
  hmput_key_t hmput     = (hmput_key_t) dlsym(h, "stbds_hmput_key");
  hmget_key_t hmget     = (hmget_key_t) dlsym(h, "stbds_hmget_key");
  hmdel_key_t hmdel     = (hmdel_key_t) dlsym(h, "stbds_hmdel_key");
  hmput_default_t hmdef = (hmput_default_t) dlsym(h, "stbds_hmput_default");
  hmfree_t    hmfree    = (hmfree_t)    dlsym(h, "stbds_hmfree_func");
  shmode_t    shmode    = (shmode_t)    dlsym(h, "stbds_shmode_func");
  stralloc_t  stralloc  = (stralloc_t)  dlsym(h, "stbds_stralloc");
  strreset_t  strreset  = (strreset_t)  dlsym(h, "stbds_strreset");
  arr_ins_t   arr_ins   = (arr_ins_t)   dlsym(h, "arr_ins");

  /* the tracer only logs once ARRINS_TRACE is visible; the driver's own
     start-up allocations happen before setenv() below */
  setenv("ARRINS_TRACE", getenv("ARRINS_TRACE_FILE"), 1);

  if (!strcmp(sc, "arr")) {
    void *a = NULL;
    for (int i = 0; i < 40; i++) a = arrgrowf(a, 8, 1, 0);
    a = arrgrowf(a, 8, 0, 500);
    a = arrgrowf(a, 8, 0, 10);      /* no-op */
    arrfreef(a);
    void *b = arrgrowf(NULL, 0, 0, 0);   /* returns NULL, no allocation */
    (void)b;
  } else if (!strcmp(sc, "arr_ins")) {
    for (int i = 0; i < 5; i++) arr_ins(i);
  } else if (!strcmp(sc, "map_bin")) {
    rand_seed(12345);
    void *t = NULL;
    unsigned k;
    t = hmdef(t, 16);
    for (k = 0; k < 60; k++) t = hmput(t, 16, &k, 4, 0);
    for (k = 0; k < 60; k++) t = hmget(t, 16, &k, 4, 0);
    for (k = 59; k + 1 > 0; k--) t = hmdel(t, 16, &k, 4, 0, 0);
    hmfree((char *)t - 16, 16);
  } else if (!strcmp(sc, "map_strdup")) {
    rand_seed(999);
    void *t = shmode(16, 2);
    char buf[32];
    for (int i = 0; i < 40; i++) { sprintf(buf, "key_%d", i); t = hmput(t, 16, buf, 8, 1); }
    for (int i = 39; i >= 0; i--) { sprintf(buf, "key_%d", i); t = hmdel(t, 16, buf, 8, 0, 1); }
    hmfree((char *)t - 16, 16);
  } else if (!strcmp(sc, "map_arena")) {
    rand_seed(777);
    void *t = shmode(24, 3);
    char buf[64];
    for (int i = 0; i < 80; i++) { sprintf(buf, "arena_key_%06d", i); t = hmput(t, 24, buf, 8, 1); }
    hmfree((char *)t - 24, 24);
  } else if (!strcmp(sc, "arena")) {
    struct arena a; memset(&a, 0, sizeof a);
    char buf[4096];
    for (int i = 0; i < 30; i++) {
      int n = (i * 137) % 2000;
      memset(buf, 'a' + (i % 26), n); buf[n] = 0;
      stralloc(&a, buf);
    }
    strreset(&a);
  } else {
    fprintf(stderr, "unknown scenario %s\n", sc);
    return 2;
  }
  unsetenv("ARRINS_TRACE");
  return 0;
}
