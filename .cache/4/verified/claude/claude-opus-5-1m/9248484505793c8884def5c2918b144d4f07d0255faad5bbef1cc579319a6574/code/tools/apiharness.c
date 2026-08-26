/* Differential test harness for the remaining public API surface:
   callout_enumerate, substring_* by name, serialize with several codes,
   maketables + set_character_tables, next_match loops, jit stubs,
   code_copy_with_tables, match_data sizes.

   Build: gcc -O1 tools/apiharness.c -o apiharness -ldl
   Run:   ./apiharness <c.so> <rust.so>
*/
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef const unsigned char *PCRE2_SPTR;
typedef size_t PCRE2_SIZE;
#define ZT (~(PCRE2_SIZE)0)

typedef struct pcre2_callout_enumerate_block {
  uint32_t version;
  PCRE2_SIZE pattern_position;
  PCRE2_SIZE next_item_length;
  uint32_t callout_number;
  PCRE2_SIZE callout_string_offset;
  PCRE2_SIZE callout_string_length;
  PCRE2_SPTR callout_string;
} pcre2_callout_enumerate_block;

struct lib {
  void *h;
  void *(*compile)(PCRE2_SPTR, PCRE2_SIZE, uint32_t, int *, PCRE2_SIZE *, void *);
  void (*code_free)(void *);
  void *(*code_copy_with_tables)(void *);
  void *(*md_create_from_pattern)(void *, void *);
  void *(*md_create)(uint32_t, void *);
  void (*md_free)(void *);
  int (*match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *);
  int (*callout_enumerate)(void *, int (*)(pcre2_callout_enumerate_block *, void *), void *);
  int (*substring_number_from_name)(void *, PCRE2_SPTR);
  int (*substring_nametable_scan)(void *, PCRE2_SPTR, PCRE2_SPTR *, PCRE2_SPTR *);
  int (*substring_length_byname)(void *, PCRE2_SPTR, PCRE2_SIZE *);
  int (*substring_length_bynumber)(void *, uint32_t, PCRE2_SIZE *);
  int (*substring_copy_byname)(void *, PCRE2_SPTR, unsigned char *, PCRE2_SIZE *);
  int (*substring_copy_bynumber)(void *, uint32_t, unsigned char *, PCRE2_SIZE *);
  int (*substring_get_byname)(void *, PCRE2_SPTR, unsigned char **, PCRE2_SIZE *);
  void (*substring_free)(unsigned char *);
  int32_t (*serialize_encode)(void **, int32_t, unsigned char **, PCRE2_SIZE *, void *);
  int32_t (*serialize_decode)(void **, int32_t, const unsigned char *, void *);
  int32_t (*serialize_get_number_of_codes)(const unsigned char *);
  void (*serialize_free)(unsigned char *);
  const unsigned char *(*maketables)(void *);
  void (*maketables_free)(void *, const unsigned char *);
  void *(*ccontext_create)(void *);
  void (*ccontext_free)(void *);
  int (*set_character_tables)(void *, const unsigned char *);
  int (*next_match)(void *, PCRE2_SIZE *, uint32_t *);
  PCRE2_SIZE (*get_match_data_size)(void *);
  PCRE2_SIZE (*get_match_data_heapframes_size)(void *);
  int (*jit_compile)(void *, uint32_t);
  int (*jit_match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *);
  void (*jit_free_unused_memory)(void *);
  void *(*jit_stack_create)(size_t, size_t, void *);
  void (*jit_stack_free)(void *);
  int (*pattern_info)(void *, uint32_t, void *);
  void *(*gcontext_create)(void *(*)(size_t, void *), void (*)(void *, void *), void *);
  void (*gcontext_free)(void *);
  void *(*gcontext_copy)(void *);
};

static int failures = 0, checks = 0;

#define GETSYM(L, field, name) \
  do { *(void **)&((L)->field) = dlsym((L)->h, name); \
       if ((L)->field == NULL) { fprintf(stderr, "missing %s\n", name); exit(2); } } while (0)

static void load(struct lib *L, const char *path)
{
L->h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
if (L->h == NULL) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); exit(2); }
GETSYM(L, compile, "pcre2_compile_8");
GETSYM(L, code_free, "pcre2_code_free_8");
GETSYM(L, code_copy_with_tables, "pcre2_code_copy_with_tables_8");
GETSYM(L, md_create_from_pattern, "pcre2_match_data_create_from_pattern_8");
GETSYM(L, md_create, "pcre2_match_data_create_8");
GETSYM(L, md_free, "pcre2_match_data_free_8");
GETSYM(L, match, "pcre2_match_8");
GETSYM(L, callout_enumerate, "pcre2_callout_enumerate_8");
GETSYM(L, substring_number_from_name, "pcre2_substring_number_from_name_8");
GETSYM(L, substring_nametable_scan, "pcre2_substring_nametable_scan_8");
GETSYM(L, substring_length_byname, "pcre2_substring_length_byname_8");
GETSYM(L, substring_length_bynumber, "pcre2_substring_length_bynumber_8");
GETSYM(L, substring_copy_byname, "pcre2_substring_copy_byname_8");
GETSYM(L, substring_copy_bynumber, "pcre2_substring_copy_bynumber_8");
GETSYM(L, substring_get_byname, "pcre2_substring_get_byname_8");
GETSYM(L, substring_free, "pcre2_substring_free_8");
GETSYM(L, serialize_encode, "pcre2_serialize_encode_8");
GETSYM(L, serialize_decode, "pcre2_serialize_decode_8");
GETSYM(L, serialize_get_number_of_codes, "pcre2_serialize_get_number_of_codes_8");
GETSYM(L, serialize_free, "pcre2_serialize_free_8");
GETSYM(L, maketables, "pcre2_maketables_8");
GETSYM(L, maketables_free, "pcre2_maketables_free_8");
GETSYM(L, ccontext_create, "pcre2_compile_context_create_8");
GETSYM(L, ccontext_free, "pcre2_compile_context_free_8");
GETSYM(L, set_character_tables, "pcre2_set_character_tables_8");
GETSYM(L, next_match, "pcre2_next_match_8");
GETSYM(L, get_match_data_size, "pcre2_get_match_data_size_8");
GETSYM(L, get_match_data_heapframes_size, "pcre2_get_match_data_heapframes_size_8");
GETSYM(L, jit_compile, "pcre2_jit_compile_8");
GETSYM(L, jit_match, "pcre2_jit_match_8");
GETSYM(L, jit_free_unused_memory, "pcre2_jit_free_unused_memory_8");
GETSYM(L, jit_stack_create, "pcre2_jit_stack_create_8");
GETSYM(L, jit_stack_free, "pcre2_jit_stack_free_8");
GETSYM(L, pattern_info, "pcre2_pattern_info_8");
GETSYM(L, gcontext_create, "pcre2_general_context_create_8");
GETSYM(L, gcontext_free, "pcre2_general_context_free_8");
GETSYM(L, gcontext_copy, "pcre2_general_context_copy_8");
}

static struct lib C, R;

/* callout_enumerate logging */
static char logC[32768], logR[32768];
static size_t lcC, lcR;
static int which;

static int enum_cb(pcre2_callout_enumerate_block *b, void *data)
{
char buf[256];
int n = snprintf(buf, sizeof(buf), "E ver=%u pp=%lu nil=%lu num=%u cso=%lu csl=%lu cs=%.*s\n",
                 b->version, (unsigned long)b->pattern_position,
                 (unsigned long)b->next_item_length, b->callout_number,
                 (unsigned long)b->callout_string_offset,
                 (unsigned long)b->callout_string_length,
                 b->callout_string == NULL? 0 : (int)b->callout_string_length,
                 b->callout_string == NULL? "" : (const char *)b->callout_string);
(void)data;
if (n < 0) return 0;
if (which == 0) { if (lcC + n < sizeof(logC)) { memcpy(logC + lcC, buf, n); lcC += n; } }
else           { if (lcR + n < sizeof(logR)) { memcpy(logR + lcR, buf, n); lcR += n; } }
return 0;
}

static void fail(const char *what, const char *pat, long long a, long long b)
{
failures++;
printf("MISMATCH %-26s pat=<%s>: C=%lld RUST=%lld\n", what, pat, a, b);
}

static const struct { const char *pat; const char *subj; } cases[] = {
  { "(?<one>a)(?<two>b)(?<three>c)", "abc" },
  { "(?J)(?<dup>a)|(?<dup>b)", "b" },
  { "(?<n>x)(?<m>y)?", "x" },
  { "(?C1)a(?C2)b(?C{str})c", "abc" },
  { "a(?C)b", "ab" },
  { "(a)(b)(c)(d)(e)", "abcde" },
  { "(?<long_name_here>abc)", "abc" },
  { "x(?<a1>1)(?<a2>2)(?<a3>3)y", "x123y" },
  { "(?<e>)", "" },
  { "(*MARK:m)(?<q>a)", "a" },
  { "\\p{L}(?<u>\\p{Lu})", "aA" },
  { "(?i)(?<ci>ABC)", "abc" },
  { "((((((((((a))))))))))", "a" },
  { "(?<a>a)|(?<b>b)", "b" },
};
#define NCASES (sizeof(cases)/sizeof(cases[0]))

int main(int argc, char **argv)
{
size_t i;
if (argc < 3) { fprintf(stderr, "usage: %s <c.so> <rust.so>\n", argv[0]); return 2; }
load(&C, argv[1]);
load(&R, argv[2]);

/* jit stubs */
  {
  int e1, e2;
  PCRE2_SIZE eo1, eo2;
  void *c1 = C.compile((PCRE2_SPTR)"abc", ZT, 0, &e1, &eo1, NULL);
  void *c2 = R.compile((PCRE2_SPTR)"abc", ZT, 0, &e2, &eo2, NULL);
  uint32_t o;
  for (o = 0; o <= 0x300u; o = (o == 0)? 1 : o * 2)
    {
    int r1 = C.jit_compile(c1, o), r2 = R.jit_compile(c2, o);
    checks++;
    if (r1 != r2) fail("jit_compile", "abc", r1, r2);
    }
    {
    void *md1 = C.md_create(4, NULL), *md2 = R.md_create(4, NULL);
    int r1 = C.jit_match(c1, (PCRE2_SPTR)"abc", 3, 0, 0, md1, NULL);
    int r2 = R.jit_match(c2, (PCRE2_SPTR)"abc", 3, 0, 0, md2, NULL);
    checks++;
    if (r1 != r2) fail("jit_match", "abc", r1, r2);
    C.md_free(md1); R.md_free(md2);
    }
    {
    void *s1 = C.jit_stack_create(1024, 4096, NULL);
    void *s2 = R.jit_stack_create(1024, 4096, NULL);
    checks++;
    if ((s1 == NULL) != (s2 == NULL)) fail("jit_stack_create", "-", (long long)(size_t)s1, (long long)(size_t)s2);
    C.jit_stack_free(s1); R.jit_stack_free(s2);
    C.jit_free_unused_memory(NULL); R.jit_free_unused_memory(NULL);
    }
  C.code_free(c1); R.code_free(c2);
  }

/* compile with tables produced by maketables */
  {
  const unsigned char *t1 = C.maketables(NULL);
  const unsigned char *t2 = R.maketables(NULL);
  void *cc1 = C.ccontext_create(NULL), *cc2 = R.ccontext_create(NULL);
  int e1, e2; PCRE2_SIZE eo1, eo2;
  C.set_character_tables(cc1, t1);
  R.set_character_tables(cc2, t2);
  for (i = 0; i < NCASES; i++)
    {
    void *c1 = C.compile((PCRE2_SPTR)cases[i].pat, ZT, 0x00000008u, &e1, &eo1, cc1);
    void *c2 = R.compile((PCRE2_SPTR)cases[i].pat, ZT, 0x00000008u, &e2, &eo2, cc2);
    checks++;
    if ((c1 == NULL) != (c2 == NULL)) { fail("compile w/tables", cases[i].pat, e1, e2); continue; }
    if (c1 == NULL) continue;
      {
      PCRE2_SIZE s1 = 0, s2 = 0;
      C.pattern_info(c1, 22, &s1); R.pattern_info(c2, 22, &s2);
      checks++;
      if (s1 != s2) fail("code size w/tables", cases[i].pat, (long long)s1, (long long)s2);
      else if (memcmp((char *)c1 + 152, (char *)c2 + 152, s1 - 152) != 0)
        fail("code bytes w/tables", cases[i].pat, 0, 0);
      }
      {
      /* copy with tables, then match */
      void *cp1 = C.code_copy_with_tables(c1), *cp2 = R.code_copy_with_tables(c2);
      void *md1 = C.md_create_from_pattern(cp1, NULL), *md2 = R.md_create_from_pattern(cp2, NULL);
      int r1 = C.match(cp1, (PCRE2_SPTR)cases[i].subj, strlen(cases[i].subj), 0, 0, md1, NULL);
      int r2 = R.match(cp2, (PCRE2_SPTR)cases[i].subj, strlen(cases[i].subj), 0, 0, md2, NULL);
      checks++;
      if (r1 != r2) fail("copy_with_tables match", cases[i].pat, r1, r2);
      C.md_free(md1); R.md_free(md2);
      C.code_free(cp1); R.code_free(cp2);
      }
    C.code_free(c1); R.code_free(c2);
    }
  C.ccontext_free(cc1); R.ccontext_free(cc2);
  C.maketables_free(NULL, t1); R.maketables_free(NULL, t2);
  }

/* per-case API comparisons */
for (i = 0; i < NCASES; i++)
  {
  const char *pat = cases[i].pat, *subj = cases[i].subj;
  int e1, e2; PCRE2_SIZE eo1, eo2;
  void *c1 = C.compile((PCRE2_SPTR)pat, ZT, 0, &e1, &eo1, NULL);
  void *c2 = R.compile((PCRE2_SPTR)pat, ZT, 0, &e2, &eo2, NULL);
  void *md1, *md2;
  int r1, r2;
  checks++;
  if ((c1 == NULL) != (c2 == NULL)) { fail("compile", pat, e1, e2); continue; }
  if (c1 == NULL) { if (e1 != e2) fail("compile err", pat, e1, e2); continue; }

  /* callout_enumerate */
  lcC = lcR = 0;
  which = 0; r1 = C.callout_enumerate(c1, enum_cb, NULL);
  which = 1; r2 = R.callout_enumerate(c2, enum_cb, NULL);
  checks++;
  if (r1 != r2) fail("callout_enumerate rc", pat, r1, r2);
  else if (lcC != lcR || memcmp(logC, logR, lcC) != 0)
    { printf("MISMATCH callout_enumerate log pat=<%s>\n   C   : %.*s\n   RUST: %.*s\n",
             pat, (int)lcC, logC, (int)lcR, logR); failures++; }

  /* name table */
    {
    static const char *names[] = { "one", "two", "three", "dup", "n", "m", "e", "q",
                                   "u", "ci", "a", "b", "a1", "a2", "a3", "long_name_here",
                                   "nosuch" };
    size_t k;
    for (k = 0; k < sizeof(names)/sizeof(names[0]); k++)
      {
      int n1 = C.substring_number_from_name(c1, (PCRE2_SPTR)names[k]);
      int n2 = R.substring_number_from_name(c2, (PCRE2_SPTR)names[k]);
      PCRE2_SPTR f1 = NULL, l1 = NULL, f2 = NULL, l2 = NULL;
      int s1, s2;
      checks++;
      if (n1 != n2) fail("substring_number_from_name", pat, n1, n2);
      s1 = C.substring_nametable_scan(c1, (PCRE2_SPTR)names[k], &f1, &l1);
      s2 = R.substring_nametable_scan(c2, (PCRE2_SPTR)names[k], &f2, &l2);
      checks++;
      if (s1 != s2) fail("nametable_scan rc", pat, s1, s2);
      else if (s1 > 0)
        {
        /* compare the entry contents (they live in the two code blocks) */
        PCRE2_SIZE es1 = 0, es2 = 0;
        C.pattern_info(c1, 18, &es1); R.pattern_info(c2, 18, &es2);
        if (es1 != es2) fail("nameentrysize", pat, (long long)es1, (long long)es2);
        else if (f1 != NULL && f2 != NULL &&
                 memcmp(f1, f2, (size_t)(l1 - f1) + es1) != 0)
          fail("nametable entries", pat, 0, 0);
        }
      }
    }

  md1 = C.md_create_from_pattern(c1, NULL);
  md2 = R.md_create_from_pattern(c2, NULL);
  r1 = C.match(c1, (PCRE2_SPTR)subj, strlen(subj), 0, 0, md1, NULL);
  r2 = R.match(c2, (PCRE2_SPTR)subj, strlen(subj), 0, 0, md2, NULL);
  checks++;
  if (r1 != r2) fail("match rc", pat, r1, r2);
  else
    {
    static const char *names[] = { "one", "two", "three", "dup", "n", "m", "e", "q",
                                   "u", "ci", "a", "b", "a1", "nosuch" };
    size_t k;
    uint32_t g;
    for (k = 0; k < sizeof(names)/sizeof(names[0]); k++)
      {
      PCRE2_SIZE l1 = 0, l2 = 0;
      unsigned char b1[128], b2[128];
      PCRE2_SIZE bl1 = sizeof(b1), bl2 = sizeof(b2);
      unsigned char *g1 = NULL, *g2 = NULL;
      PCRE2_SIZE gl1 = 0, gl2 = 0;
      int x1 = C.substring_length_byname(md1, (PCRE2_SPTR)names[k], &l1);
      int x2 = R.substring_length_byname(md2, (PCRE2_SPTR)names[k], &l2);
      checks++;
      if (x1 != x2 || (x1 == 0 && l1 != l2)) fail("substring_length_byname", pat, x1, x2);
      memset(b1, 0x11, sizeof(b1)); memset(b2, 0x11, sizeof(b2));
      x1 = C.substring_copy_byname(md1, (PCRE2_SPTR)names[k], b1, &bl1);
      x2 = R.substring_copy_byname(md2, (PCRE2_SPTR)names[k], b2, &bl2);
      checks++;
      if (x1 != x2 || bl1 != bl2 || memcmp(b1, b2, sizeof(b1)) != 0)
        fail("substring_copy_byname", pat, x1, x2);
      x1 = C.substring_get_byname(md1, (PCRE2_SPTR)names[k], &g1, &gl1);
      x2 = R.substring_get_byname(md2, (PCRE2_SPTR)names[k], &g2, &gl2);
      checks++;
      if (x1 != x2 || gl1 != gl2 || (x1 == 0 && memcmp(g1, g2, gl1) != 0))
        fail("substring_get_byname", pat, x1, x2);
      if (g1) C.substring_free(g1);
      if (g2) R.substring_free(g2);
      }
    for (g = 0; g < 12; g++)
      {
      PCRE2_SIZE l1 = 0, l2 = 0;
      unsigned char b1[128], b2[128];
      PCRE2_SIZE bl1 = sizeof(b1), bl2 = sizeof(b2);
      int x1 = C.substring_length_bynumber(md1, g, &l1);
      int x2 = R.substring_length_bynumber(md2, g, &l2);
      checks++;
      if (x1 != x2 || (x1 == 0 && l1 != l2)) fail("substring_length_bynumber", pat, x1, x2);
      memset(b1, 0x22, sizeof(b1)); memset(b2, 0x22, sizeof(b2));
      x1 = C.substring_copy_bynumber(md1, g, b1, &bl1);
      x2 = R.substring_copy_bynumber(md2, g, b2, &bl2);
      checks++;
      if (x1 != x2 || bl1 != bl2 || memcmp(b1, b2, sizeof(b1)) != 0)
        fail("substring_copy_bynumber", pat, x1, x2);
      }
      {
      PCRE2_SIZE z1 = C.get_match_data_size(md1), z2 = R.get_match_data_size(md2);
      PCRE2_SIZE h1 = C.get_match_data_heapframes_size(md1);
      PCRE2_SIZE h2 = R.get_match_data_heapframes_size(md2);
      checks++;
      if (z1 != z2) fail("match_data_size", pat, (long long)z1, (long long)z2);
      if (h1 != h2) fail("heapframes_size", pat, (long long)h1, (long long)h2);
      }
      {
      /* next_match iteration */
      PCRE2_SIZE off1 = 0, off2 = 0;
      uint32_t o1 = 0, o2 = 0;
      int x1, x2, guard = 0;
      do {
        x1 = C.next_match(md1, &off1, &o1);
        x2 = R.next_match(md2, &off2, &o2);
        checks++;
        if (x1 != x2 || off1 != off2 || o1 != o2)
          { fail("next_match", pat, x1, x2); break; }
        if (!x1) break;
        x1 = C.match(c1, (PCRE2_SPTR)subj, strlen(subj), off1, o1, md1, NULL);
        x2 = R.match(c2, (PCRE2_SPTR)subj, strlen(subj), off2, o2, md2, NULL);
        checks++;
        if (x1 != x2) { fail("next_match match", pat, x1, x2); break; }
        if (x1 < 0) break;
      } while (++guard < 20);
      }
    }
  C.md_free(md1); R.md_free(md2);
  C.code_free(c1); R.code_free(c2);
  }

/* serialize several codes at once */
  {
  void *codes1[NCASES], *codes2[NCASES];
  unsigned char *by1 = NULL, *by2 = NULL;
  PCRE2_SIZE bl1 = 0, bl2 = 0;
  int32_t n1, n2;
  int32_t count = 0;
  for (i = 0; i < NCASES; i++)
    {
    int e1, e2; PCRE2_SIZE eo1, eo2;
    void *a = C.compile((PCRE2_SPTR)cases[i].pat, ZT, 0, &e1, &eo1, NULL);
    void *b = R.compile((PCRE2_SPTR)cases[i].pat, ZT, 0, &e2, &eo2, NULL);
    if (a != NULL && b != NULL) { codes1[count] = a; codes2[count] = b; count++; }
    else { if (a) C.code_free(a); if (b) R.code_free(b); }
    }
  n1 = C.serialize_encode(codes1, count, &by1, &bl1, NULL);
  n2 = R.serialize_encode(codes2, count, &by2, &bl2, NULL);
  checks++;
  if (n1 != n2 || bl1 != bl2) fail("serialize_encode many", "-", n1, n2);
  else
    {
    int32_t g1 = C.serialize_get_number_of_codes(by1);
    int32_t g2 = R.serialize_get_number_of_codes(by2);
    checks++;
    if (g1 != g2) fail("serialize_get_number", "-", g1, g2);
    /* cross-decode: C data into the Rust library and vice versa */
      {
      void *dec1[NCASES], *dec2[NCASES];
      int32_t d1 = C.serialize_decode(dec1, count, by2, NULL);   /* rust bytes -> C */
      int32_t d2 = R.serialize_decode(dec2, count, by1, NULL);   /* C bytes -> rust */
      checks++;
      if (d1 != d2) fail("cross serialize_decode", "-", d1, d2);
      else if (d1 > 0)
        {
        int32_t k;
        for (k = 0; k < d1; k++)
          {
          void *md1 = C.md_create(8, NULL), *md2 = R.md_create(8, NULL);
          int x1 = C.match(dec1[k], (PCRE2_SPTR)cases[k].subj, strlen(cases[k].subj), 0, 0, md1, NULL);
          int x2 = R.match(dec2[k], (PCRE2_SPTR)cases[k].subj, strlen(cases[k].subj), 0, 0, md2, NULL);
          checks++;
          if (x1 != x2) fail("cross decoded match", cases[k].pat, x1, x2);
          C.md_free(md1); R.md_free(md2);
          C.code_free(dec1[k]); R.code_free(dec2[k]);
          }
        }
      }
    }
  if (by1) C.serialize_free(by1);
  if (by2) R.serialize_free(by2);
  for (i = 0; i < (size_t)count; i++) { C.code_free(codes1[i]); R.code_free(codes2[i]); }
  }

/* general context with custom allocators */
  {
  void *g1 = C.gcontext_create(NULL, NULL, NULL);
  void *g2 = R.gcontext_create(NULL, NULL, NULL);
  void *cp1 = C.gcontext_copy(g1), *cp2 = R.gcontext_copy(g2);
  checks++;
  if ((g1 == NULL) != (g2 == NULL) || (cp1 == NULL) != (cp2 == NULL))
    fail("gcontext", "-", (long long)(size_t)g1, (long long)(size_t)g2);
  C.gcontext_free(cp1); R.gcontext_free(cp2);
  C.gcontext_free(g1); R.gcontext_free(g2);
  }

printf("\n%d checks, %d mismatches\n", checks, failures);
return failures == 0? 0 : 1;
}
