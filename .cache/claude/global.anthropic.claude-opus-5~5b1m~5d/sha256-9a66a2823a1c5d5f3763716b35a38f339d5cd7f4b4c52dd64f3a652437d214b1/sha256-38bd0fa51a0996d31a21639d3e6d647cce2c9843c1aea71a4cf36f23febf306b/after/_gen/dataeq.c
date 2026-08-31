/* Compare the exported data tables and simple internal functions of the C and
   Rust libpcre2.so, byte for byte. Usage: dataeq <c.so> <rust.so> */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdint.h>

static void *hc, *hr;
static int failures = 0;

static void cmp_bytes(const char *name, size_t len)
{
  void *a = dlsym(hc, name), *b = dlsym(hr, name);
  if (a == NULL || b == NULL)
    {
    printf("MISSING %s (c=%p rust=%p)\n", name, a, b);
    failures++;
    return;
    }
  if (memcmp(a, b, len) != 0)
    {
    size_t i;
    for (i = 0; i < len; i++)
      if (((unsigned char *)a)[i] != ((unsigned char *)b)[i]) break;
    printf("DIFFER %s at byte %zu (c=0x%02x rust=0x%02x) of %zu\n", name, i,
           ((unsigned char *)a)[i], ((unsigned char *)b)[i], len);
    failures++;
    }
  else printf("ok %s (%zu bytes)\n", name, len);
}

static void cmp_str(const char *name)
{
  char **a = (char **)dlsym(hc, name), **b = (char **)dlsym(hr, name);
  if (a == NULL || b == NULL) { printf("MISSING %s\n", name); failures++; return; }
  if (strcmp(*a, *b) != 0)
    { printf("DIFFER %s ('%s' vs '%s')\n", name, *a, *b); failures++; }
  else printf("ok %s ('%s')\n", name, *a);
}

int main(int argc, char **argv)
{
  if (argc != 3) { fprintf(stderr, "usage: dataeq c.so rust.so\n"); return 2; }
  hc = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  hr = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
  if (hc == NULL || hr == NULL)
    { fprintf(stderr, "dlopen failed: %s\n", dlerror()); return 2; }

  /* Sizes taken from the C build (8-bit, LINK_SIZE 2, SUPPORT_UNICODE). */
  cmp_bytes("_pcre2_default_tables_8", 1088);
  cmp_bytes("_pcre2_OP_lengths_8", 173);
  cmp_bytes("_pcre2_hspace_list_8", 4 * 20);
  cmp_bytes("_pcre2_vspace_list_8", 4 * 8);
  cmp_bytes("_pcre2_callout_start_delims_8", 4 * 9);
  cmp_bytes("_pcre2_callout_end_delims_8", 4 * 9);
  cmp_bytes("_pcre2_utf8_table1", 4 * 6);
  cmp_bytes("_pcre2_utf8_table1_size", 4);
  cmp_bytes("_pcre2_utf8_table2", 4 * 6);
  cmp_bytes("_pcre2_utf8_table3", 4 * 6);
  cmp_bytes("_pcre2_utf8_table4", 64);
  cmp_bytes("_pcre2_ucp_gentype_8", 4 * 30);
  cmp_bytes("_pcre2_ucp_gbtable_8", 4 * 15);
  cmp_bytes("_pcre2_utt_8", 6 * 518);
  cmp_bytes("_pcre2_utt_names_8", 3834);
  cmp_bytes("_pcre2_utt_size_8", 8);
  cmp_bytes("_pcre2_posix_class_maps8", 4 * 42);
  cmp_bytes("_pcre2_ucd_records_8", 12 * 1563);
  cmp_bytes("_pcre2_ucd_stage1_8", 2 * 8704);
  cmp_bytes("_pcre2_ucd_stage2_8", 2 * 40192);
  cmp_bytes("_pcre2_ucd_caseless_sets_8", 4 * 118);
  cmp_bytes("_pcre2_ucd_digit_sets_8", 4 * 78);
  cmp_bytes("_pcre2_ucd_script_sets_8", 4 * 476);
  cmp_bytes("_pcre2_ucd_boolprop_sets_8", 4 * 382);
  cmp_bytes("_pcre2_ucd_nocase_ranges_8", 4 * 84);
  cmp_bytes("_pcre2_ucd_nocase_ranges_size_8", 4);
  cmp_bytes("_pcre2_ucd_turkish_dotted_i_caseset_8", 4);
  cmp_str("_pcre2_unicode_version_8");

  /* Simple internal functions. */
  {
    unsigned (*f1)(uint32_t, uint8_t *) = dlsym(hc, "_pcre2_ord2utf_8");
    unsigned (*f2)(uint32_t, uint8_t *) = dlsym(hr, "_pcre2_ord2utf_8");
    if (f1 && f2)
      {
      int bad = 0;
      for (uint32_t c = 0; c <= 0x110010; c += 7)
        {
        uint8_t b1[8], b2[8];
        memset(b1, 0, 8); memset(b2, 0, 8);
        unsigned r1 = f1(c, b1), r2 = f2(c, b2);
        if (r1 != r2 || memcmp(b1, b2, 8) != 0)
          { printf("DIFFER ord2utf(%u): %u vs %u\n", c, r1, r2); bad = 1; failures++; break; }
        }
      if (!bad) printf("ok _pcre2_ord2utf_8\n");
      }
    else { printf("MISSING _pcre2_ord2utf_8\n"); failures++; }
  }
  {
    int (*f1)(const uint8_t *, size_t, size_t *) = dlsym(hc, "_pcre2_valid_utf_8");
    int (*f2)(const uint8_t *, size_t, size_t *) = dlsym(hr, "_pcre2_valid_utf_8");
    if (f1 && f2)
      {
      int bad = 0;
      unsigned long long st = 12345;
      for (int n = 0; n < 200000; n++)
        {
        uint8_t buf[12];
        size_t len = 1 + (n % 8);
        for (size_t i = 0; i < len; i++)
          { st = st * 6364136223846793005ULL + 1442695040888963407ULL;
            buf[i] = (uint8_t)(st >> 33); }
        size_t o1 = 12345, o2 = 54321;
        int r1 = f1(buf, len, &o1), r2 = f2(buf, len, &o2);
        if (r1 != r2 || (r1 != 0 && o1 != o2))
          { printf("DIFFER valid_utf n=%d: %d/%zu vs %d/%zu\n", n, r1, o1, r2, o2);
            bad = 1; failures++; break; }
        }
      if (!bad) printf("ok _pcre2_valid_utf_8\n");
      }
    else { printf("MISSING _pcre2_valid_utf_8\n"); failures++; }
  }
  {
    int (*f1)(const uint8_t *, const uint8_t *, int) = dlsym(hc, "_pcre2_script_run_8");
    int (*f2)(const uint8_t *, const uint8_t *, int) = dlsym(hr, "_pcre2_script_run_8");
    if (f1 && f2)
      {
      static const char *strs[] = { "abc", "ab1", "\xcf\x80\xce\xb1", "a\xcf\x80",
        "\xd0\xb0\xd0\xb1", "12\xd9\xa1", "\xe4\xb8\xad\xe6\x96\x87",
        "\xe3\x81\x82\xe4\xb8\xad", "\xc3\xa9\x61", "a\xc2\xb7\x62" };
      int bad = 0;
      for (unsigned i = 0; i < sizeof(strs)/sizeof(strs[0]); i++)
        for (int utf = 0; utf < 2; utf++)
          {
          const uint8_t *s = (const uint8_t *)strs[i];
          int r1 = f1(s, s + strlen(strs[i]), utf), r2 = f2(s, s + strlen(strs[i]), utf);
          if (r1 != r2)
            { printf("DIFFER script_run('%s',%d): %d vs %d\n", strs[i], utf, r1, r2);
              bad = 1; failures++; }
          }
      if (!bad) printf("ok _pcre2_script_run_8\n");
      }
    else { printf("MISSING _pcre2_script_run_8\n"); failures++; }
  }
  {
    int (*f1)(const uint8_t *, uint32_t, const uint8_t *, uint32_t *, int) =
      dlsym(hc, "_pcre2_is_newline_8");
    int (*f2)(const uint8_t *, uint32_t, const uint8_t *, uint32_t *, int) =
      dlsym(hr, "_pcre2_is_newline_8");
    if (f1 && f2)
      {
      int bad = 0;
      static const char *strs[] = { "\n", "\r", "\r\n", "\v", "\f", "\xc2\x85",
        "\xe2\x80\xa8", "\xe2\x80\xa9", "a", "" };
      for (unsigned i = 0; i < sizeof(strs)/sizeof(strs[0]); i++)
        for (uint32_t type = 1; type <= 2; type++)
          for (int utf = 0; utf < 2; utf++)
            {
            const uint8_t *s = (const uint8_t *)strs[i];
            uint32_t l1 = 99, l2 = 99;
            int r1 = f1(s, type, s + strlen(strs[i]), &l1, utf);
            int r2 = f2(s, type, s + strlen(strs[i]), &l2, utf);
            if (r1 != r2 || l1 != l2)
              { printf("DIFFER is_newline i=%u type=%u utf=%d: %d/%u vs %d/%u\n",
                       i, type, utf, r1, l1, r2, l2); bad = 1; failures++; }
            }
      if (!bad) printf("ok _pcre2_is_newline_8\n");
      }
    else { printf("MISSING _pcre2_is_newline_8\n"); failures++; }
  }
  {
    int (*f1)(size_t *, int, int) = dlsym(hc, "_pcre2_ckd_smul_8");
    int (*f2)(size_t *, int, int) = dlsym(hr, "_pcre2_ckd_smul_8");
    if (f1 && f2)
      {
      int bad = 0;
      int vals[] = { 0, 1, -1, 2, 100, 65535, 100000, 2000000000, -2000000000 };
      for (unsigned i = 0; i < sizeof(vals)/sizeof(vals[0]); i++)
        for (unsigned j = 0; j < sizeof(vals)/sizeof(vals[0]); j++)
          {
          size_t r1v = 0, r2v = 0;
          int r1 = f1(&r1v, vals[i], vals[j]), r2 = f2(&r2v, vals[i], vals[j]);
          if (r1 != r2 || r1v != r2v)
            { printf("DIFFER ckd_smul(%d,%d): %d/%zu vs %d/%zu\n", vals[i], vals[j],
                     r1, r1v, r2, r2v); bad = 1; failures++; }
          }
      if (!bad) printf("ok _pcre2_ckd_smul_8\n");
      }
    else { printf("MISSING _pcre2_ckd_smul_8\n"); failures++; }
  }
  {
    const uint8_t *(*f1)(uint32_t, const uint8_t *, const uint8_t *, const uint8_t *, int, int *)
      = dlsym(hc, "_pcre2_extuni_8");
    const uint8_t *(*f2)(uint32_t, const uint8_t *, const uint8_t *, const uint8_t *, int, int *)
      = dlsym(hr, "_pcre2_extuni_8");
    if (f1 && f2)
      {
      int bad = 0;
      static const char *strs[] = { "a\xcc\x81\x62", "\xe0\xa4\x95\xe0\xa4\xbe",
        "\xf0\x9f\x91\xa8\xe2\x80\x8d\xf0\x9f\x91\xa9", "\xea\xb0\x80\xe1\x86\xa8",
        "\xf0\x9f\x87\xa6\xf0\x9f\x87\xa7\xf0\x9f\x87\xa8", "abc" };
      for (unsigned i = 0; i < sizeof(strs)/sizeof(strs[0]); i++)
        {
        const uint8_t *s = (const uint8_t *)strs[i];
        size_t len = strlen(strs[i]);
        uint32_t c = s[0];
        if (c >= 0xc0)
          { /* decode the first character crudely for the call */
            if ((c & 0x20) == 0) c = ((c & 0x1f) << 6) | (s[1] & 0x3f);
            else if ((c & 0x10) == 0) c = ((c & 0x0f) << 12) | ((s[1] & 0x3f) << 6) | (s[2] & 0x3f);
            else c = ((c & 0x07) << 18) | ((s[1] & 0x3f) << 12) | ((s[2] & 0x3f) << 6) | (s[3] & 0x3f);
          }
        const uint8_t *start = s;
        const uint8_t *p = s + ((s[0] >= 0xf0)? 4 : (s[0] >= 0xe0)? 3 : (s[0] >= 0xc0)? 2 : 1);
        int x1 = 0, x2 = 0;
        const uint8_t *r1 = f1(c, p, start, s + len, 1, &x1);
        const uint8_t *r2 = f2(c, p, start, s + len, 1, &x2);
        if ((r1 - s) != (r2 - s) || x1 != x2)
          { printf("DIFFER extuni i=%u: %ld/%d vs %ld/%d\n", i, (long)(r1 - s), x1,
                   (long)(r2 - s), x2); bad = 1; failures++; }
        }
      if (!bad) printf("ok _pcre2_extuni_8\n");
      }
    else { printf("MISSING _pcre2_extuni_8\n"); failures++; }
  }

  printf(failures == 0 ? "ALL DATA/FUNCTION CHECKS PASS\n" : "%d FAILURES\n", failures);
  return failures != 0;
}
