/* Limit / edge-case differential tests for PCRE2. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <locale.h>

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"

/* ---- custom allocator, to exercise the memctl paths ---- */
static unsigned long long alloc_count = 0, free_count = 0, alloc_bytes = 0;

static void *my_malloc(size_t size, void *data)
{
  (void)data;
  alloc_count++; alloc_bytes += size;
  return malloc(size);
}
static void my_free(void *block, void *data)
{
  (void)data;
  if (block != NULL) free_count++;
  free(block);
}

static void print_str(const unsigned char *s, size_t len)
{
  for (size_t i = 0; i < len; i++)
    {
    unsigned c = s[i];
    if (c >= 32 && c < 127 && c != '\\') putchar(c);
    else printf("\\x%02x", c);
    }
}

static PCRE2_SIZE case_callout(PCRE2_SPTR in, PCRE2_SIZE inlen, PCRE2_UCHAR *out,
  PCRE2_SIZE outlen, int to_case, void *data)
{
  (void)data;
  printf("      casecallout inlen=%lu outlen=%lu to_case=%d in='",
    (unsigned long)inlen, (unsigned long)outlen, to_case);
  print_str(in, inlen);
  printf("'\n");
  if (outlen < inlen) return PCRE2_UNSET;
  for (PCRE2_SIZE i = 0; i < inlen; i++)
    {
    unsigned c = in[i];
    out[i] = (to_case == PCRE2_SUBSTITUTE_CASE_UPPER)?
      ((c >= 'a' && c <= 'z')? c - 32 : c) :
      ((c >= 'A' && c <= 'Z')? c + 32 : c);
    }
  return inlen;
}

static int guard_fn(uint32_t depth, void *data)
{
  (void)data;
  return (depth > 10)? 1 : 0;
}

int main(void)
{
  int ec; PCRE2_SIZE eo;

  /* ---- limits: match limit, depth limit, heap limit ---- */
  {
    static const char *pats[] = { "(a+)+b", "(a|aa)+$", "(?:a?){20}b", "(a*)*b",
      "((((((((((a))))))))))+b",
      "(*NO_AUTO_POSSESS)(a+)+b", "(*NO_AUTO_POSSESS)(a|a?)+b",
      "(*NO_AUTO_POSSESS)(?:a|aa)+c", "(*NO_START_OPT)(*NO_AUTO_POSSESS)(a|ab)*d",
      NULL };
    char subj[200];
    memset(subj, 'a', sizeof(subj)); subj[sizeof(subj)-1] = 0;
    /* a second subject that ends in 'b' so that the required-code-unit
    optimization does not short-circuit the catastrophic backtracking */
    char subjb[200];
    memset(subjb, 'a', sizeof(subjb)); subjb[sizeof(subjb)-1] = 0;
    for (int p = 0; pats[p] != NULL; p++)
      {
      pcre2_code *re = pcre2_compile((PCRE2_SPTR)pats[p], PCRE2_ZERO_TERMINATED, 0,
        &ec, &eo, NULL);
      if (re == NULL) { printf("limits '%s' compile err=%d\n", pats[p], ec); continue; }
      for (int len = 10; len <= 40; len += 10)
        for (int which = 0; which < 3; which++)
          {
          static const uint32_t lim[] = { 100, 1000, 20 };
          pcre2_match_context *mc = pcre2_match_context_create(NULL);
          if (which == 0) pcre2_set_match_limit(mc, lim[0]);
          else if (which == 1) pcre2_set_depth_limit(mc, lim[2]);
          else pcre2_set_heap_limit(mc, 1);
          pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
          int rc = pcre2_match(re, (PCRE2_SPTR)subj, len, 0, 0, md, mc);
          int rc2;
          {
            int ws[100];
            rc2 = pcre2_dfa_match(re, (PCRE2_SPTR)subj, len, 0, 0, md, mc, ws, 100);
          }
          subjb[len - 1] = 'b';
          int rcb = pcre2_match(re, (PCRE2_SPTR)subjb, len, 0, 0, md, mc);
          int rcb2;
          {
            int ws[100];
            rcb2 = pcre2_dfa_match(re, (PCRE2_SPTR)subjb, len, 0, 0, md, mc, ws, 100);
          }
          subjb[len - 1] = 'a';
          printf("limits '%s' len=%d which=%d rc=%d dfarc=%d rcb=%d dfarcb=%d\n",
            pats[p], len, which, rc, rc2, rcb, rcb2);
          pcre2_match_data_free(md);
          pcre2_match_context_free(mc);
          }
      pcre2_code_free(re);
      }
  }

  /* ---- recursion loop detection ---- */
  {
    static const char *pats[] = { "(?R)", "(a?(?R))", "(?:(?1))((?2))((?1))",
      "^(?:(?=a)|(?1))", NULL };
    for (int p = 0; pats[p] != NULL; p++)
      {
      pcre2_code *re = pcre2_compile((PCRE2_SPTR)pats[p], PCRE2_ZERO_TERMINATED, 0,
        &ec, &eo, NULL);
      printf("recurse '%s' -> ", pats[p]);
      if (re == NULL) { printf("err=%d off=%lu\n", ec, (unsigned long)eo); continue; }
      pcre2_match_data *md = pcre2_match_data_create(10, NULL);
      for (int o = 0; o < 2; o++)
        {
        int rc = pcre2_match(re, (PCRE2_SPTR)"aaab", 4, 0,
          o? PCRE2_DISABLE_RECURSELOOP_CHECK : 0, md, NULL);
        printf("rc[%d]=%d ", o, rc);
        }
      printf("\n");
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
  }

  /* ---- long subjects: exercises the memchr/req-unit start optimizations ---- */
  {
    char *big = malloc(200000);
    for (int i = 0; i < 200000; i++) big[i] = "abcde"[i % 5];
    big[199999] = 'z';
    static const char *pats[] = { "z", "az", "e+z", "\\bz", "cde", "(?i)Z",
      "a[bc]+z", "^a", "z$", "(?s).{199998}z", NULL };
    for (int p = 0; pats[p] != NULL; p++)
      {
      pcre2_code *re = pcre2_compile((PCRE2_SPTR)pats[p], PCRE2_ZERO_TERMINATED, 0,
        &ec, &eo, NULL);
      if (re == NULL) { printf("big '%s' err=%d\n", pats[p], ec); continue; }
      pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
      int rc = pcre2_match(re, (PCRE2_SPTR)big, 200000, 0, 0, md, NULL);
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
      printf("big '%s' rc=%d ov=%ld,%ld\n", pats[p], rc, rc > 0? (long)ov[0] : -1,
        rc > 0? (long)ov[1] : -1);
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    free(big);
  }

  /* ---- partial matching and DFA restart ---- */
  {
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"abcd", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    pcre2_match_data *md = pcre2_match_data_create(10, NULL);
    int ws[100];
    int rc = pcre2_dfa_match(re, (PCRE2_SPTR)"ab", 2, 0, PCRE2_PARTIAL_HARD, md, NULL,
      ws, 100);
    printf("dfa partial rc=%d\n", rc);
    rc = pcre2_dfa_match(re, (PCRE2_SPTR)"cd", 2, 0, PCRE2_DFA_RESTART, md, NULL,
      ws, 100);
    printf("dfa restart rc=%d\n", rc);
    /* workspace too small */
    rc = pcre2_dfa_match(re, (PCRE2_SPTR)"abcd", 4, 0, 0, md, NULL, ws, 10);
    printf("dfa small ws rc=%d\n", rc);
    /* DFA shortest */
    pcre2_code *re2 = pcre2_compile((PCRE2_SPTR)"a+", PCRE2_ZERO_TERMINATED, 0, &ec, &eo, NULL);
    rc = pcre2_dfa_match(re2, (PCRE2_SPTR)"aaaa", 4, 0, PCRE2_DFA_SHORTEST, md, NULL, ws, 100);
    PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
    printf("dfa shortest rc=%d ov=%ld,%ld\n", rc, (long)ov[0], (long)ov[1]);
    pcre2_code_free(re2);
    pcre2_match_data_free(md);
    pcre2_code_free(re);
  }

  /* ---- custom allocators everywhere ---- */
  {
    pcre2_general_context *gc = pcre2_general_context_create(my_malloc, my_free, NULL);
    pcre2_compile_context *cc = pcre2_compile_context_create(gc);
    pcre2_match_context *mc = pcre2_match_context_create(gc);
    pcre2_convert_context *vc = pcre2_convert_context_create(gc);
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"(?<x>a+)(b)", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, cc);
    printf("alloc compile ok=%d\n", re != NULL);
    if (re != NULL)
      {
      pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, gc);
      int rc = pcre2_match(re, (PCRE2_SPTR)"aaab", 4, 0, 0, md, mc);
      printf("alloc match rc=%d\n", rc);
      PCRE2_UCHAR *s = NULL; PCRE2_SIZE sl = 0;
      printf("alloc getsub rc=%d\n", pcre2_substring_get_byname(md, (PCRE2_SPTR)"x", &s, &sl));
      if (s != NULL) { printf("  sub='"); print_str(s, sl); printf("'\n"); pcre2_substring_free(s); }
      PCRE2_UCHAR **list = NULL; PCRE2_SIZE *lens = NULL;
      if (pcre2_substring_list_get(md, &list, &lens) == 0) pcre2_substring_list_free(list);
      uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
      const pcre2_code *codes[2]; codes[0] = re; codes[1] = re;
      int32_t src = pcre2_serialize_encode(codes, 2, &bytes, &blen, gc);
      printf("alloc serialize rc=%d len=%lu\n", src, (unsigned long)blen);
      if (src > 0)
        {
        pcre2_code *out[2] = { NULL, NULL };
        printf("alloc deserialize rc=%d\n", pcre2_serialize_decode(out, 2, bytes, gc));
        pcre2_code_free(out[0]); pcre2_code_free(out[1]);
        pcre2_serialize_free(bytes);
        }
      PCRE2_UCHAR *conv = NULL; PCRE2_SIZE convlen = 0;
      printf("alloc convert rc=%d\n", pcre2_pattern_convert((PCRE2_SPTR)"*.c",
        PCRE2_ZERO_TERMINATED, PCRE2_CONVERT_GLOB, &conv, &convlen, vc));
      if (conv != NULL) { printf("  conv='"); print_str(conv, convlen); printf("'\n");
        pcre2_converted_pattern_free(conv); }
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    /* the counts must be identical between the two libraries */
    printf("alloc counts: allocs=%llu frees=%llu bytes=%llu\n", alloc_count, free_count,
      alloc_bytes);
    pcre2_convert_context_free(vc);
    pcre2_match_context_free(mc);
    pcre2_compile_context_free(cc);
    pcre2_general_context_free(gc);
  }

  /* ---- locale tables ---- */
  {
    setlocale(LC_ALL, "C");
    const uint8_t *tables = pcre2_maketables(NULL);
    pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
    pcre2_set_character_tables(cc, tables);
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"[[:alpha:]]+\\w+", PCRE2_ZERO_TERMINATED,
      PCRE2_CASELESS, &ec, &eo, cc);
    printf("locale compile ok=%d\n", re != NULL);
    if (re != NULL)
      {
      pcre2_match_data *md = pcre2_match_data_create(10, NULL);
      int rc = pcre2_match(re, (PCRE2_SPTR)"Hello_World", 11, 0, 0, md, NULL);
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
      printf("locale match rc=%d ov=%ld,%ld\n", rc, (long)ov[0], (long)ov[1]);
      pcre2_code *cp = pcre2_code_copy_with_tables(re);
      printf("copy_with_tables ok=%d\n", cp != NULL);
      if (cp != NULL)
        {
        rc = pcre2_match(cp, (PCRE2_SPTR)"Hello_World", 11, 0, 0, md, NULL);
        printf("copy match rc=%d\n", rc);
        pcre2_code_free(cp);
        }
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    pcre2_compile_context_free(cc);
    pcre2_maketables_free(NULL, tables);
  }

  /* ---- compile recursion guard ---- */
  {
    pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
    pcre2_set_compile_recursion_guard(cc, guard_fn, NULL);
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"((((((((((((((((a))))))))))))))))",
      PCRE2_ZERO_TERMINATED, 0, &ec, &eo, cc);
    printf("guard compile ok=%d err=%d off=%lu\n", re != NULL, ec, (unsigned long)eo);
    pcre2_code_free(re);
    pcre2_compile_context_free(cc);
  }

  /* ---- substitute with case callout and overflow ---- */
  {
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"(\\w+)", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    pcre2_match_context *mc = pcre2_match_context_create(NULL);
    pcre2_set_substitute_case_callout(mc, case_callout, NULL);
    for (int sz = 0; sz < 12; sz++)
      {
      PCRE2_UCHAR out[64]; PCRE2_SIZE olen = sz;
      memset(out, '#', sizeof(out));
      int rc = pcre2_substitute(re, (PCRE2_SPTR)"hello world", 11, 0,
        PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_EXTENDED, NULL, mc,
        (PCRE2_SPTR)"\\U$1\\E-", PCRE2_ZERO_TERMINATED, out, &olen);
      printf("subst sz=%d rc=%d len=%lu out='", sz, rc, (unsigned long)olen);
      if (rc >= 0) print_str(out, olen);
      printf("'\n");
      }
    /* replacement errors */
    static const char *bad[] = { "$", "${", "${1", "$x", "\\", "\\q", "${99}",
      "$1$2$3", "\\L$0\\E", "${1:+a:b}", "${1:-}", NULL };
    for (int i = 0; bad[i] != NULL; i++)
      {
      PCRE2_UCHAR out[64]; PCRE2_SIZE olen = sizeof(out);
      int rc = pcre2_substitute(re, (PCRE2_SPTR)"hi", 2, 0, PCRE2_SUBSTITUTE_EXTENDED,
        NULL, NULL, (PCRE2_SPTR)bad[i], PCRE2_ZERO_TERMINATED, out, &olen);
      printf("subst bad '%s' rc=%d len=%lu\n", bad[i], rc, (unsigned long)olen);
      }
    pcre2_match_context_free(mc);
    pcre2_code_free(re);
  }

  /* ---- invalid UTF handling ---- */
  {
    static const char *bad[] = { "\xff", "\xc3", "\xe4\xb8", "\x80", "a\xffb",
      "\xed\xa0\x80", "\xf5\x80\x80\x80", "\xc0\x80", NULL };
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"a", PCRE2_ZERO_TERMINATED, PCRE2_UTF,
      &ec, &eo, NULL);
    pcre2_code *re2 = pcre2_compile((PCRE2_SPTR)"a", PCRE2_ZERO_TERMINATED,
      PCRE2_UTF|PCRE2_MATCH_INVALID_UTF, &ec, &eo, NULL);
    pcre2_match_data *md = pcre2_match_data_create(10, NULL);
    for (int i = 0; bad[i] != NULL; i++)
      {
      size_t len = strlen(bad[i]);
      int rc = pcre2_match(re, (PCRE2_SPTR)bad[i], len, 0, 0, md, NULL);
      int rcn = pcre2_match(re, (PCRE2_SPTR)bad[i], len, 0, PCRE2_NO_UTF_CHECK, md, NULL);
      int rci = pcre2_match(re2, (PCRE2_SPTR)bad[i], len, 0, 0, md, NULL);
      int ws[100];
      int rcd = pcre2_dfa_match(re, (PCRE2_SPTR)bad[i], len, 0, 0, md, NULL, ws, 100);
      printf("badutf %d rc=%d nocheck=%d invalidutf=%d dfa=%d startchar=%lu\n", i, rc,
        rcn, rci, rcd, (unsigned long)pcre2_get_startchar(md));
      }
    /* compile-time invalid UTF pattern */
    pcre2_code *re3 = pcre2_compile((PCRE2_SPTR)"\xff", 1, PCRE2_UTF, &ec, &eo, NULL);
    printf("badutf pattern ok=%d err=%d off=%lu\n", re3 != NULL, ec, (unsigned long)eo);
    pcre2_code_free(re3);
    pcre2_match_data_free(md);
    pcre2_code_free(re2);
    pcre2_code_free(re);
  }

  /* ---- huge / deep patterns ---- */
  {
    char *pat = malloc(70000);
    /* deeply nested groups (just below the parens nest limit) */
    int n = 200;
    pat[0] = 0;
    for (int i = 0; i < n; i++) strcat(pat, "(");
    strcat(pat, "a");
    for (int i = 0; i < n; i++) strcat(pat, ")");
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, 0, &ec, &eo, NULL);
    printf("deep nest ok=%d err=%d\n", re != NULL, ec);
    if (re != NULL)
      {
      uint32_t cc = 0; pcre2_pattern_info(re, PCRE2_INFO_CAPTURECOUNT, &cc);
      pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
      printf("deep nest cc=%u rc=%d\n", cc, pcre2_match(re, (PCRE2_SPTR)"a", 1, 0, 0, md, NULL));
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    /* long alternation */
    pat[0] = 0;
    for (int i = 0; i < 2000; i++) { char buf[16]; sprintf(buf, "%s%d", i? "|" : "", i); strcat(pat, buf); }
    re = pcre2_compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, 0, &ec, &eo, NULL);
    printf("long alt ok=%d err=%d\n", re != NULL, ec);
    if (re != NULL)
      {
      PCRE2_SIZE sz = 0; pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz);
      pcre2_match_data *md = pcre2_match_data_create(10, NULL);
      printf("long alt size=%lu rc=%d\n", (unsigned long)sz,
        pcre2_match(re, (PCRE2_SPTR)"1999", 4, 0, 0, md, NULL));
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    /* many named groups */
    pat[0] = 0;
    for (int i = 0; i < 500; i++) { char buf[32]; sprintf(buf, "(?<n%d>a)", i); strcat(pat, buf); }
    re = pcre2_compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, 0, &ec, &eo, NULL);
    printf("many names ok=%d err=%d\n", re != NULL, ec);
    if (re != NULL)
      {
      uint32_t nc = 0, nes = 0;
      pcre2_pattern_info(re, PCRE2_INFO_NAMECOUNT, &nc);
      pcre2_pattern_info(re, PCRE2_INFO_NAMEENTRYSIZE, &nes);
      printf("many names nc=%u nes=%u num=%d\n", nc, nes,
        pcre2_substring_number_from_name(re, (PCRE2_SPTR)"n499"));
      pcre2_code_free(re);
      }
    /* a big literal */
    memset(pat, 'a', 60000); pat[60000] = 0;
    re = pcre2_compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, 0, &ec, &eo, NULL);
    printf("big literal ok=%d err=%d\n", re != NULL, ec);
    pcre2_code_free(re);
    free(pat);
  }


  /* ---- serialization error paths ---- */
  {
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"(a)(b)", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    const pcre2_code *codes[1]; codes[0] = re;
    uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
    printf("ser encode rc=%d\n", pcre2_serialize_encode(codes, 1, &bytes, &blen, NULL));
    printf("ser encode bad n rc=%d\n", pcre2_serialize_encode(codes, 0, &bytes, &blen, NULL));
    printf("ser encode null rc=%d\n", pcre2_serialize_encode(NULL, 1, &bytes, &blen, NULL));
    printf("ser ncodes null rc=%d\n", pcre2_serialize_get_number_of_codes(NULL));
    if (bytes != NULL)
      {
      pcre2_code *out[2] = { NULL, NULL };
      /* good */
      printf("ser decode rc=%d\n", pcre2_serialize_decode(out, 1, bytes, NULL));
      pcre2_code_free(out[0]); out[0] = NULL;
      /* too many */
      printf("ser decode toomany rc=%d\n", pcre2_serialize_decode(out, 2, bytes, NULL));
      /* negative count = decode all */
      printf("ser decode all rc=%d\n", pcre2_serialize_decode(out, -1, bytes, NULL));
      pcre2_code_free(out[0]); out[0] = NULL;
      /* corrupt each of the first 16 bytes in turn */
      for (int i = 0; i < 16; i++)
        {
        uint8_t save = bytes[i];
        bytes[i] ^= 0xff;
        int rc = pcre2_serialize_decode(out, 1, bytes, NULL);
        printf("ser decode corrupt[%d] rc=%d\n", i, rc);
        if (rc > 0) { pcre2_code_free(out[0]); out[0] = NULL; }
        bytes[i] = save;
        }
      printf("ser decode nullbytes rc=%d\n", pcre2_serialize_decode(out, 1, NULL, NULL));
      printf("ser decode nullcodes rc=%d\n", pcre2_serialize_decode(NULL, 1, bytes, NULL));
      pcre2_serialize_free(bytes);
      }
    pcre2_serialize_free(NULL);
    pcre2_code_free(re);
  }

  /* ---- API misuse / boundary values ---- */
  {
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"(a)(?<nm>b)?", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    pcre2_match_data *md = pcre2_match_data_create(1, NULL);
    printf("oveccount1 rc=%d\n", pcre2_match(re, (PCRE2_SPTR)"ab", 2, 0, 0, md, NULL));
    printf("  count=%u\n", pcre2_get_ovector_count(md));
    /* start offset beyond the subject */
    printf("badoffset rc=%d\n", pcre2_match(re, (PCRE2_SPTR)"ab", 2, 3, 0, md, NULL));
    /* substring functions on an unset / missing group */
    PCRE2_SIZE len = 0;
    printf("len[5] rc=%d\n", pcre2_substring_length_bynumber(md, 5, &len));
    printf("len[nm] rc=%d\n", pcre2_substring_length_byname(md, (PCRE2_SPTR)"nm", &len));
    printf("len[zz] rc=%d\n", pcre2_substring_length_byname(md, (PCRE2_SPTR)"zz", &len));
    printf("num[zz] rc=%d\n", pcre2_substring_number_from_name(re, (PCRE2_SPTR)"zz"));
    printf("num[nm] rc=%d\n", pcre2_substring_number_from_name(re, (PCRE2_SPTR)"nm"));
    PCRE2_SPTR first = NULL, last = NULL;
    printf("nametable rc=%d\n", pcre2_substring_nametable_scan(re, (PCRE2_SPTR)"nm",
      &first, &last));
    /* pattern_info error paths */
    uint32_t u = 0;
    printf("info bad rc=%d\n", pcre2_pattern_info(re, 999, &u));
    printf("info null rc=%d\n", pcre2_pattern_info(NULL, PCRE2_INFO_SIZE, &u));
    printf("info nulldst rc=%d\n", pcre2_pattern_info(re, PCRE2_INFO_SIZE, NULL));
    /* zero-length and NULL subjects */
    printf("empty subj rc=%d\n", pcre2_match(re, (PCRE2_SPTR)"", 0, 0, 0, md, NULL));
    printf("null subj rc=%d\n", pcre2_match(re, NULL, 0, 0, 0, md, NULL));
    pcre2_match_data_free(md);
    pcre2_code_free(re);
  }

  printf("STRESSDONE\n");
  return 0;
}
