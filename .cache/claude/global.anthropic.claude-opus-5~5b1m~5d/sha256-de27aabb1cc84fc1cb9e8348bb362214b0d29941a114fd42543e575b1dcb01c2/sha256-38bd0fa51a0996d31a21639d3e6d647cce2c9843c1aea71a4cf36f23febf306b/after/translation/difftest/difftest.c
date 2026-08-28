// Differential harness: loads the C reference .so and the Rust .so and
// compares cp_inflate / unfilter behaviour plus all exported data symbols.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

typedef int (*inflate_fn)(void *, int, void *, int);
typedef int (*unfilter_fn)(int, int, int, uint8_t *);

#define OUTCAP (1 << 20)
#define INCAP (1 << 20)

typedef struct {
  int ret;
  int have_err;
  char err[512];
  unsigned char out[OUTCAP];
} shared_t;

static shared_t *shm_new(void) {
  shared_t *p = mmap(NULL, sizeof(shared_t), PROT_READ | PROT_WRITE,
                     MAP_SHARED | MAP_ANONYMOUS, -1, 0);
  if (p == MAP_FAILED) { perror("mmap"); exit(1); }
  return p;
}

static unsigned char *in_shared;

static const char *libs[2];

// run kind: 0 = inflate, 1 = unfilter
static int run_child(int which, shared_t *sh, int kind, int a, int b, int c,
                     int in_off, int in_len, const unsigned char *in_data,
                     int out_bytes, const unsigned char *out_init,
                     int out_init_len) {
  pid_t pid = fork();
  if (pid == 0) {
    alarm(5);
    memset(sh->out, 0xAA, OUTCAP);
    if (out_init) memcpy(sh->out, out_init, out_init_len);
    memset(in_shared, 0, INCAP);
    if (in_data && in_len > 0) memcpy(in_shared + in_off, in_data, in_len);
    void *h = dlopen(libs[which], RTLD_NOW | RTLD_LOCAL);
    if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); _exit(2); }
    const char **errp = (const char **)dlsym(h, "cp_error_reason");
    if (kind == 0 && a != 0) {
      // Mutate exported tables before the call to verify the Rust code reads
      // the same public globals the C code reads.
      uint8_t *ft = (uint8_t *)dlsym(h, "cp_fixed_table");
      uint8_t *po = (uint8_t *)dlsym(h, "cp_permutation_order");
      uint8_t *lex = (uint8_t *)dlsym(h, "cp_len_extra_bits");
      uint32_t *lb = (uint32_t *)dlsym(h, "cp_len_base");
      uint8_t *dex = (uint8_t *)dlsym(h, "cp_dist_extra_bits");
      uint32_t *db = (uint32_t *)dlsym(h, "cp_dist_base");
      switch (a) {
      case 1: lb[0] = 5; lb[1] = 7; break;
      case 2: db[0] = 2; db[3] = 9; break;
      case 3: ft[0] = 7; ft[143] = 9; break;
      case 4: po[0] = 3; po[3] = 16; break;
      case 5: lex[0] = 1; break;
      case 6: dex[0] = 2; break;
      case 7: for (int i = 0; i < 288; i++) ft[i] = (i % 9) + 1; break;
      }
    }
    sh->ret = -12345;
    sh->have_err = 0;
    sh->err[0] = 0;
    if (kind == 0) {
      inflate_fn f = (inflate_fn)dlsym(h, "cp_inflate");
      sh->ret = f(in_shared + in_off, in_len, sh->out, out_bytes);
    } else {
      unfilter_fn f = (unfilter_fn)dlsym(h, "unfilter");
      sh->ret = f(a, b, c, sh->out);
    }
    if (errp && *errp) {
      sh->have_err = 1;
      snprintf(sh->err, sizeof(sh->err), "%s", *errp);
    }
    _exit(0);
  }
  int st = 0;
  waitpid(pid, &st, 0);
  return st;
}

static int failures = 0, cases = 0;

static void cmp_case(const char *name, int kind, int a, int b, int c,
                     int in_off, int in_len, const unsigned char *in_data,
                     int out_bytes, const unsigned char *out_init,
                     int out_init_len) {
  static shared_t *s0, *s1;
  if (!s0) { s0 = shm_new(); s1 = shm_new(); }
  int st0 = run_child(0, s0, kind, a, b, c, in_off, in_len, in_data, out_bytes, out_init, out_init_len);
  int st1 = run_child(1, s1, kind, a, b, c, in_off, in_len, in_data, out_bytes, out_init, out_init_len);
  cases++;
  int bad = 0;
  if (st0 != st1) { printf("MISMATCH[%s]: status C=%d Rust=%d\n", name, st0, st1); bad = 1; }
  else if (!WIFEXITED(st0) || WEXITSTATUS(st0) != 0) {
    // both crashed identically; treat as informational
    printf("both-crashed[%s] status=%d\n", name, st0);
    return;
  } else {
    if (s0->ret != s1->ret) { printf("MISMATCH[%s]: ret C=%d Rust=%d\n", name, s0->ret, s1->ret); bad = 1; }
    if (memcmp(s0->out, s1->out, OUTCAP) != 0) {
      size_t i = 0; while (i < OUTCAP && s0->out[i] == s1->out[i]) i++;
      printf("MISMATCH[%s]: out buffer differs at %zu (C=%02x Rust=%02x)\n", name, i, s0->out[i], s1->out[i]);
      bad = 1;
    }
    if (s0->have_err != s1->have_err || strcmp(s0->err, s1->err) != 0) {
      printf("MISMATCH[%s]: err C='%s'(%d) Rust='%s'(%d)\n", name, s0->err, s0->have_err, s1->err, s1->have_err);
      bad = 1;
    }
  }
  if (bad) failures++;
}

static void check_data_symbols(void) {
  void *h0 = dlopen(libs[0], RTLD_NOW | RTLD_LOCAL);
  void *h1 = dlopen(libs[1], RTLD_NOW | RTLD_LOCAL);
  if (!h0 || !h1) { printf("dlopen failed: %s\n", dlerror()); failures++; return; }
  struct { const char *n; size_t sz; } tabs[] = {
      {"cp_fixed_table", 320}, {"cp_permutation_order", 19},
      {"cp_len_extra_bits", 31}, {"cp_len_base", 124},
      {"cp_dist_extra_bits", 32}, {"cp_dist_base", 128},
  };
  for (size_t i = 0; i < sizeof(tabs) / sizeof(tabs[0]); i++) {
    void *p0 = dlsym(h0, tabs[i].n), *p1 = dlsym(h1, tabs[i].n);
    if (!p0 || !p1) { printf("MISSING symbol %s (C=%p Rust=%p)\n", tabs[i].n, p0, p1); failures++; continue; }
    if (memcmp(p0, p1, tabs[i].sz) != 0) { printf("MISMATCH table %s\n", tabs[i].n); failures++; }
    else printf("ok table %s (%zu bytes)\n", tabs[i].n, tabs[i].sz);
  }
  void *e0 = dlsym(h0, "cp_error_reason"), *e1 = dlsym(h1, "cp_error_reason");
  if (!e0 || !e1) { printf("MISSING cp_error_reason\n"); failures++; }
  else if (*(void **)e0 != NULL || *(void **)e1 != NULL) { printf("MISMATCH cp_error_reason not NULL initially\n"); failures++; }
  else printf("ok cp_error_reason initial NULL\n");
}

int main(int argc, char **argv) {
  if (argc < 3) { fprintf(stderr, "usage: %s libC libRust casefile\n", argv[0]); return 1; }
  libs[0] = argv[1];
  libs[1] = argv[2];
  in_shared = mmap(NULL, INCAP, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0);
  if (in_shared == MAP_FAILED) { perror("mmap"); return 1; }

  check_data_symbols();

  // Case file format (binary): repeated records
  //   u8 kind (0=inflate,1=unfilter)
  //   i32 a,b,c        (unfilter w,h,bpp ; unused for inflate)
  //   i32 in_off, in_len, out_bytes, out_init_len, name_len
  //   name bytes, in bytes, out_init bytes
  FILE *f = fopen(argv[3], "rb");
  if (!f) { perror("casefile"); return 1; }
  static unsigned char in_buf[INCAP];
  static unsigned char out_init[OUTCAP];
  char name[256];
  while (1) {
    unsigned char kind;
    if (fread(&kind, 1, 1, f) != 1) break;
    int hdr[8];
    if (fread(hdr, sizeof(int), 8, f) != 8) break;
    int a = hdr[0], b = hdr[1], c = hdr[2], in_off = hdr[3], in_len = hdr[4],
        out_bytes = hdr[5], out_init_len = hdr[6], name_len = hdr[7];
    if (name_len >= (int)sizeof(name)) { printf("bad name len\n"); return 1; }
    if (fread(name, 1, name_len, f) != (size_t)name_len) break;
    name[name_len] = 0;
    if (in_len > 0 && fread(in_buf, 1, in_len, f) != (size_t)in_len) break;
    if (out_init_len > 0 && fread(out_init, 1, out_init_len, f) != (size_t)out_init_len) break;
    cmp_case(name, kind, a, b, c, in_off, in_len,
             in_len > 0 ? in_buf : NULL, out_bytes,
             out_init_len > 0 ? out_init : NULL, out_init_len);
  }
  fclose(f);
  printf("\n=== %d cases, %d failures ===\n", cases, failures);
  return failures != 0;
}
