/* dlopen-based harness: no copy relocations, so the library reads its OWN
 * .data. Complements harness.c (which declares the tables extern and therefore
 * makes the linker create copy relocations). */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <unistd.h>
#include <sys/wait.h>

typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;
typedef struct cp_image_t { int w, h; cp_pixel_t *pix; } cp_image_t;

static cp_image_t (*p_load)(const uint8_t *, int);
static int (*p_inflate)(void *, int, void *, int);
static const char **p_reason;
static uint8_t *p_fixed, *p_perm, *p_lenx, *p_distx;
static uint32_t *p_lenb, *p_distb;

#define OUTCAP (1 << 16)

static uint8_t *slurp(const char *path, int *len) {
  FILE *f = fopen(path, "rb");
  if (!f) { *len = 0; return NULL; }
  fseek(f, 0, SEEK_END); long n = ftell(f); fseek(f, 0, SEEK_SET);
  uint8_t *b = calloc(1, (size_t)n + 64);
  if (n > 0 && fread(b, 1, (size_t)n, f) != (size_t)n) {}
  fclose(f); *len = (int)n; return b;
}

static void do_png(const char *path) {
  int n; uint8_t *b = slurp(path, &n);
  const char *base = strrchr(path, '/'); base = base ? base + 1 : path;
  printf("=== PNG %s len=%d\n", base, n);
  *p_reason = NULL;
  cp_image_t img = p_load(b, n);
  printf("  w=%d h=%d pix_null=%d reason=%s\n", img.w, img.h, img.pix == NULL,
         *p_reason ? *p_reason : "(null)");
  if (img.pix) {
    long npix = (long)img.w * (long)img.h;
    unsigned long long h = 1469598103934665603ull;
    const unsigned char *pb = (const unsigned char *)img.pix;
    for (long i = 0; i < npix * 4; ++i) h = (h ^ pb[i]) * 1099511628211ull;
    printf("  npix=%ld hash=%llu\n", npix, h);
    free(img.pix);
  }
  free(b);
}

static void do_inf(const char *path) {
  int n; uint8_t *b = slurp(path, &n);
  const char *base = strrchr(path, '/'); base = base ? base + 1 : path;
  printf("=== INF %s len=%d\n", base, n);
  static const int outs[] = {0, 1, 7, 64, 1024, OUTCAP, -1};
  for (int align = 0; align < 4; ++align) {
    uint8_t *raw = calloc(1, (size_t)n + 64);
    memcpy(raw + align, b, (size_t)n);
    for (unsigned oi = 0; oi < sizeof(outs)/sizeof(outs[0]); ++oi) {
      uint8_t *out = calloc(1, OUTCAP + 4096);
      *p_reason = NULL;
      int r = p_inflate(raw + align, n, out, outs[oi]);
      unsigned long long h = 1469598103934665603ull; size_t nz = 0;
      for (size_t i = 0; i < OUTCAP; ++i) { h = (h ^ out[i]) * 1099511628211ull; if (out[i]) nz = i + 1; }
      printf("  align=%d out_bytes=%d ret=%d nz=%zu hash=%llu reason=%s\n",
             align, outs[oi], r, nz, h, *p_reason ? *p_reason : "(null)");
      free(out);
    }
    free(raw);
  }
  free(b);
}

static int cmpstr(const void *a, const void *b) { return strcmp(*(const char **)a, *(const char **)b); }

static void walk(const char *dir, void (*fn)(const char *)) {
  DIR *d = opendir(dir); if (!d) return;
  char **names = NULL; size_t n = 0, cap = 0; struct dirent *e;
  while ((e = readdir(d))) {
    if (e->d_name[0] == '.') continue;
    if (n == cap) { cap = cap ? cap * 2 : 64; names = realloc(names, cap * sizeof(char *)); }
    size_t l = strlen(dir) + strlen(e->d_name) + 2; char *p = malloc(l);
    snprintf(p, l, "%s/%s", dir, e->d_name); names[n++] = p;
  }
  closedir(d); qsort(names, n, sizeof(char *), cmpstr);
  for (size_t i = 0; i < n; ++i) {
    fflush(stdout);
    pid_t pid = fork();
    if (pid == 0) { fn(names[i]); fflush(stdout); _exit(0); }
    int st = 0; waitpid(pid, &st, 0);
    const char *base = strrchr(names[i], '/'); base = base ? base + 1 : names[i];
    if (WIFEXITED(st)) printf("  [%s exit=%d]\n", base, WEXITSTATUS(st));
    else printf("  [%s signal=%d]\n", base, WTERMSIG(st));
    free(names[i]);
  }
  free(names);
}

int main(int argc, char **argv) {
  setvbuf(stdout, NULL, _IONBF, 0);
  void *h = dlopen(argv[1], RTLD_NOW);
  if (!h) { printf("dlopen failed: %s\n", dlerror()); return 1; }
  p_load = dlsym(h, "load_png_mem");
  p_inflate = dlsym(h, "cp_inflate");
  p_reason = dlsym(h, "cp_error_reason");
  p_fixed = dlsym(h, "cp_fixed_table");
  p_perm = dlsym(h, "cp_permutation_order");
  p_lenx = dlsym(h, "cp_len_extra_bits");
  p_distx = dlsym(h, "cp_dist_extra_bits");
  p_lenb = dlsym(h, "cp_len_base");
  p_distb = dlsym(h, "cp_dist_base");
  if (!p_load || !p_inflate || !p_reason || !p_fixed || !p_perm || !p_lenx ||
      !p_distx || !p_lenb || !p_distb) { printf("dlsym failed\n"); return 1; }
  printf("all 9 symbols resolved via dlsym\n");
  /* relative layout of the tables inside the library's own .data */
  printf("rel dist_extra-dist_base=%ld len_base-dist_base=%ld "
         "len_extra-dist_base=%ld perm-dist_base=%ld fixed-dist_base=%ld\n",
         (long)((char *)p_distx - (char *)p_distb),
         (long)((char *)p_lenb - (char *)p_distb),
         (long)((char *)p_lenx - (char *)p_distb),
         (long)((char *)p_perm - (char *)p_distb),
         (long)((char *)p_fixed - (char *)p_distb));
  if (argc > 2) walk(argv[2], do_png);
  if (argc > 3) walk(argv[3], do_inf);
  return 0;
}
