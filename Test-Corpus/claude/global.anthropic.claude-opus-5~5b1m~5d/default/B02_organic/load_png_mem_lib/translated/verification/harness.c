/* Differential harness: linked separately against the C and the Rust .so and
 * the two stdout streams are compared byte-for-byte. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <unistd.h>
#include <sys/wait.h>

typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;
typedef struct cp_image_t { int w, h; cp_pixel_t *pix; } cp_image_t;

extern cp_image_t load_png_mem(const uint8_t *png_data, int png_length);
extern int cp_inflate(void *in, int in_bytes, void *out, int out_bytes);
extern const char *cp_error_reason;
extern uint8_t cp_fixed_table[288 + 32];
extern uint8_t cp_permutation_order[19];
extern uint8_t cp_len_extra_bits[29 + 2];
extern uint32_t cp_len_base[29 + 2];
extern uint8_t cp_dist_extra_bits[30 + 2];
extern uint32_t cp_dist_base[30 + 2];

static void dump(const char *label, const void *p, size_t n) {
  const unsigned char *b = (const unsigned char *)p;
  printf("%s[%zu]=", label, n);
  for (size_t i = 0; i < n; ++i) printf("%02x", b[i]);
  printf("\n");
}

static void tables(void) {
  dump("cp_fixed_table", cp_fixed_table, sizeof(cp_fixed_table));
  dump("cp_permutation_order", cp_permutation_order, sizeof(cp_permutation_order));
  dump("cp_len_extra_bits", cp_len_extra_bits, sizeof(cp_len_extra_bits));
  dump("cp_len_base", cp_len_base, sizeof(cp_len_base));
  dump("cp_dist_extra_bits", cp_dist_extra_bits, sizeof(cp_dist_extra_bits));
  dump("cp_dist_base", cp_dist_base, sizeof(cp_dist_base));
  printf("cp_error_reason_initially_null=%d\n", cp_error_reason == NULL);
}

/* read whole file into a padded, zeroed buffer so that the library's
 * out-of-bounds reads on malformed input stay deterministic */
static uint8_t *slurp(const char *path, int *len) {
  FILE *f = fopen(path, "rb");
  if (!f) { *len = 0; return NULL; }
  fseek(f, 0, SEEK_END);
  long n = ftell(f);
  fseek(f, 0, SEEK_SET);
  uint8_t *buf = (uint8_t *)calloc(1, (size_t)n + 64);
  if (n > 0 && fread(buf, 1, (size_t)n, f) != (size_t)n) { /* ignore */ }
  fclose(f);
  *len = (int)n;
  return buf;
}

static void do_png(const char *path) {
  int len = 0;
  uint8_t *buf = slurp(path, &len);
  const char *base = strrchr(path, '/');
  base = base ? base + 1 : path;
  printf("=== PNG %s len=%d\n", base, len);
  cp_error_reason = NULL;
  cp_image_t img = load_png_mem(buf, len);
  printf("  w=%d h=%d pix_null=%d reason=%s\n", img.w, img.h, img.pix == NULL,
         cp_error_reason ? cp_error_reason : "(null)");
  if (img.pix) {
    long npix = (long)img.w * (long)img.h;
    unsigned long long sum = 0;
    for (long i = 0; i < npix; ++i)
      sum = sum * 1315423911u + img.pix[i].r + 7u * img.pix[i].g +
            13u * img.pix[i].b + 17u * img.pix[i].a;
    /* hash the WHOLE malloc'd pixel buffer, i.e. (w+1)*h*4 bytes, not just the
     * visible pixels -- catches differences in the inflate scratch area */
    long full = ((long)img.w + 1) * (long)img.h * 4;
    unsigned long long fh = 1469598103934665603ull;
    const unsigned char *pb = (const unsigned char *)img.pix;
    for (long i = 0; i < full; ++i) fh = (fh ^ pb[i]) * 1099511628211ull;
    printf("  npix=%ld hash=%llu fullbytes=%ld fullhash=%llu\n", npix, sum, full, fh);
    long show = npix < 64 ? npix : 64;
    for (long i = 0; i < show; ++i)
      printf("  px%ld=%02x%02x%02x%02x\n", i, img.pix[i].r, img.pix[i].g,
             img.pix[i].b, img.pix[i].a);
    if (npix > 0) {
      long i = npix - 1;
      printf("  last=%02x%02x%02x%02x\n", img.pix[i].r, img.pix[i].g,
             img.pix[i].b, img.pix[i].a);
    }
    free(img.pix); /* must be a libc-malloc'd pointer */
  }
  free(buf);
}

#define OUTCAP (1 << 16)

static void do_inflate(const char *path) {
  int len = 0;
  uint8_t *buf = slurp(path, &len);
  const char *base = strrchr(path, '/');
  base = base ? base + 1 : path;
  printf("=== INF %s len=%d\n", base, len);
  static const int outs[] = {0, 1, 7, 64, 1024, OUTCAP, -1};
  for (int align = 0; align < 4; ++align) {
    /* copy into a fresh 16-byte-aligned block, then offset it */
    uint8_t *raw = (uint8_t *)calloc(1, (size_t)len + 64);
    memcpy(raw + align, buf, (size_t)len);
    for (unsigned oi = 0; oi < sizeof(outs) / sizeof(outs[0]); ++oi) {
      uint8_t *out = (uint8_t *)calloc(1, OUTCAP + 4096);
      cp_error_reason = NULL;
      int r = cp_inflate(raw + align, len, out, outs[oi]);
      unsigned long long h = 1469598103934665603ull;
      size_t nz = 0;
      for (size_t i = 0; i < OUTCAP; ++i) {
        h = (h ^ out[i]) * 1099511628211ull;
        if (out[i]) nz = i + 1;
      }
      printf("  align=%d out_bytes=%d ret=%d nz=%zu hash=%llu reason=%s\n",
             align, outs[oi], r, nz, h,
             cp_error_reason ? cp_error_reason : "(null)");
      size_t show = nz < 48 ? nz : 48;
      printf("   head=");
      for (size_t i = 0; i < show; ++i) printf("%02x", out[i]);
      printf("\n");
      free(out);
    }
    free(raw);
  }
  free(buf);
}

static int cmpstr(const void *a, const void *b) {
  return strcmp(*(const char **)a, *(const char **)b);
}

static void walk(const char *dir, void (*fn)(const char *)) {
  DIR *d = opendir(dir);
  if (!d) { printf("cannot open %s\n", dir); return; }
  char **names = NULL;
  size_t n = 0, cap = 0;
  struct dirent *e;
  while ((e = readdir(d))) {
    if (e->d_name[0] == '.') continue;
    if (n == cap) { cap = cap ? cap * 2 : 64; names = realloc(names, cap * sizeof(char *)); }
    size_t l = strlen(dir) + strlen(e->d_name) + 2;
    char *p = malloc(l);
    snprintf(p, l, "%s/%s", dir, e->d_name);
    names[n++] = p;
  }
  closedir(d);
  qsort(names, n, sizeof(char *), cmpstr);
  for (size_t i = 0; i < n; ++i) {
    /* run each case in a child so that a (faithfully reproduced) crash does
     * not abort the whole comparison run */
    fflush(stdout);
    pid_t pid = fork();
    if (pid == 0) { fn(names[i]); fflush(stdout); _exit(0); }
    int st = 0;
    waitpid(pid, &st, 0);
    const char *base = strrchr(names[i], '/');
    base = base ? base + 1 : names[i];
    if (WIFEXITED(st)) printf("  [%s exit=%d]\n", base, WEXITSTATUS(st));
    else printf("  [%s signal=%d]\n", base, WTERMSIG(st));
    free(names[i]);
  }
  free(names);
}

/* ------------------------------------------------------------- edge cases */
static const char *g_png_path;

static void report(cp_image_t img) {
  printf("  w=%d h=%d pix_null=%d reason=%s\n", img.w, img.h, img.pix == NULL,
         cp_error_reason ? cp_error_reason : "(null)");
  if (img.pix) {
    long npix = (long)img.w * (long)img.h;
    unsigned long long sum = 0;
    for (long i = 0; i < npix; ++i)
      sum = sum * 1315423911u + img.pix[i].r + 7u * img.pix[i].g +
            13u * img.pix[i].b + 17u * img.pix[i].a;
    printf("  npix=%ld hash=%llu\n", npix, sum);
    free(img.pix);
  }
}

static void e_null_png(void) { report(load_png_mem(NULL, 0)); }
static void e_null_png2(void) { report(load_png_mem(NULL, 100)); }
static void e_neg_len(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  report(load_png_mem(b, -1));
}
static void e_len8(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  report(load_png_mem(b, 8));
}
static void e_len_partial(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  report(load_png_mem(b, n / 2));
}
static void e_twice(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  report(load_png_mem(b, n));
  report(load_png_mem(b, n));
}
static void e_inflate_null_in(void) {
  uint8_t *out = (uint8_t *)calloc(1, 256);
  int r = cp_inflate(NULL, 0, out, 256);
  printf("  ret=%d reason=%s\n", r, cp_error_reason ? cp_error_reason : "(null)");
}
static void e_inflate_null_out(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  int r = cp_inflate(b, n, NULL, 0);
  printf("  ret=%d reason=%s\n", r, cp_error_reason ? cp_error_reason : "(null)");
}
static void e_png(void) {
  int n; uint8_t *b = slurp(g_png_path, &n);
  report(load_png_mem(b, n));
}

/* run a raw deflate stream and report everything about the result */
static const char *g_inf_path;
static void run_stream(void) {
  int n; uint8_t *b = slurp(g_inf_path, &n);
  uint8_t *out = (uint8_t *)calloc(1, OUTCAP + 4096);
  cp_error_reason = NULL;
  int r = cp_inflate(b, n, out, 4096);
  unsigned long long h = 1469598103934665603ull;
  size_t nz = 0;
  for (size_t i = 0; i < OUTCAP; ++i) {
    h = (h ^ out[i]) * 1099511628211ull;
    if (out[i]) nz = i + 1;
  }
  printf("  ret=%d nz=%zu hash=%llu reason=%s\n", r, nz, h,
         cp_error_reason ? cp_error_reason : "(null)");
  printf("   head=");
  for (size_t i = 0; i < (nz < 48 ? nz : 48); ++i) printf("%02x", out[i]);
  printf("\n");
}
static void e_inf_base(void) { run_stream(); }
static void e_inf_len_base(void) { cp_len_base[0] = 100; run_stream(); }
static void e_inf_dist_base(void) { cp_dist_base[0] = 3; run_stream(); }
static void e_inf_len_extra(void) { cp_len_extra_bits[0] = 1; run_stream(); }
static void e_inf_dist_extra(void) { cp_dist_extra_bits[0] = 2; run_stream(); }
static void e_inf_fixed_table(void) {
  /* swap the two halves of the 8-bit literal range: still a valid, complete
   * canonical tree, but literals get decoded to different symbols */
  for (int i = 0; i < 144; ++i) cp_fixed_table[i] = 9;
  for (int i = 144; i < 256; ++i) cp_fixed_table[i] = 8;
  for (int i = 256; i < 272; ++i) cp_fixed_table[i] = 8;
  for (int i = 272; i < 288; ++i) cp_fixed_table[i] = 7;
  run_stream();
}
static void e_inf_perm(void) {
  for (int i = 0; i < 19; ++i) cp_permutation_order[i] = (uint8_t)(18 - i);
  run_stream();
}

struct edge { const char *name; void (*fn)(void); };

int main(int argc, char **argv) {
  setvbuf(stdout, NULL, _IONBF, 0);
  tables();
  if (argc > 1) walk(argv[1], do_png);
  if (argc > 2) walk(argv[2], do_inflate);
  if (argc > 3) {
    g_png_path = argv[3];
    g_inf_path = argc > 4 ? argv[4] : argv[3];
    static const struct edge edges[] = {
        {"png", e_png},
        {"null_png", e_null_png},
        {"null_png2", e_null_png2},
        {"neg_len", e_neg_len},
        {"len8", e_len8},
        {"len_partial", e_len_partial},
        {"twice", e_twice},
        {"inflate_null_in", e_inflate_null_in},
        {"inflate_null_out", e_inflate_null_out},
        {"inf_base", e_inf_base},
        {"inf_mut_len_base", e_inf_len_base},
        {"inf_mut_dist_base", e_inf_dist_base},
        {"inf_mut_len_extra", e_inf_len_extra},
        {"inf_mut_dist_extra", e_inf_dist_extra},
        {"inf_mut_fixed_table", e_inf_fixed_table},
        {"inf_mut_perm", e_inf_perm},
    };
    for (unsigned i = 0; i < sizeof(edges) / sizeof(edges[0]); ++i) {
      printf("=== EDGE %s\n", edges[i].name);
      fflush(stdout);
      pid_t pid = fork();
      if (pid == 0) { edges[i].fn(); fflush(stdout); _exit(0); }
      int st = 0;
      waitpid(pid, &st, 0);
      if (WIFEXITED(st)) printf("  [exit=%d]\n", WEXITSTATUS(st));
      else printf("  [signal=%d]\n", WTERMSIG(st));
    }
  }
  /* error reason persistence across calls (the library never clears it) */
  printf("final_reason=%s\n", cp_error_reason ? cp_error_reason : "(null)");
  return 0;
}
