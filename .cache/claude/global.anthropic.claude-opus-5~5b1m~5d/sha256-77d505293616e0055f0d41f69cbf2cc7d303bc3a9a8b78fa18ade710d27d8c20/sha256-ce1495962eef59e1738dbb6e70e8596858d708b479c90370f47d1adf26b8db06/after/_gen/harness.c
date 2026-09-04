/* Differential test harness: link this against the C libpcre2.so and against the
   Rust libpcre2.so and diff the outputs. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"

static void print_str(const unsigned char *s, size_t len)
{
  for (size_t i = 0; i < len; i++)
    {
    unsigned c = s[i];
    if (c >= 32 && c < 127 && c != '\\') putchar(c);
    else printf("\\x%02x", c);
    }
}

/* ---------------- callout functions ---------------- */

static int callout_fn(pcre2_callout_block *cb, void *data)
{
  (void)data;
  printf("    callout: num=%u ctop=%u clast=%u start=%lu cur=%lu patpos=%lu nextlen=%lu"
         " strofs=%lu strlen=%lu flags=%u mark=%s\n",
    cb->callout_number, cb->capture_top, cb->capture_last,
    (unsigned long)cb->start_match, (unsigned long)cb->current_position,
    (unsigned long)cb->pattern_position, (unsigned long)cb->next_item_length,
    (unsigned long)cb->callout_string_offset, (unsigned long)cb->callout_string_length,
    cb->callout_flags, cb->mark == NULL ? "(null)" : (const char *)cb->mark);
  return 0;
}

static int enum_fn(pcre2_callout_enumerate_block *cb, void *data)
{
  (void)data;
  printf("    enum: patpos=%lu nextlen=%lu num=%u strofs=%lu strlen=%lu\n",
    (unsigned long)cb->pattern_position, (unsigned long)cb->next_item_length,
    cb->callout_number, (unsigned long)cb->callout_string_offset,
    (unsigned long)cb->callout_string_length);
  return 0;
}

static int subst_callout_fn(pcre2_substitute_callout_block *cb, void *data)
{
  (void)data;
  printf("    subcallout: oveccount=%u subscount=%u out=[%lu,%lu] ovec0=%lu ovec1=%lu\n",
    cb->oveccount, cb->subscount, (unsigned long)cb->output_offsets[0],
    (unsigned long)cb->output_offsets[1], (unsigned long)cb->ovector[0],
    (unsigned long)cb->ovector[1]);
  return 0;
}

/* ---------------- info dump ---------------- */

static void dump_info(pcre2_code *re)
{
  uint32_t u; PCRE2_SIZE z; const uint8_t *bm; int rc;
  static const int reqs[] = {
    PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_BACKREFMAX,
    PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT, PCRE2_INFO_FIRSTCODEUNIT,
    PCRE2_INFO_FIRSTCODETYPE, PCRE2_INFO_HASCRORLF, PCRE2_INFO_JCHANGED,
    PCRE2_INFO_JITSIZE, PCRE2_INFO_LASTCODEUNIT, PCRE2_INFO_LASTCODETYPE,
    PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MATCHLIMIT, PCRE2_INFO_MAXLOOKBEHIND,
    PCRE2_INFO_MINLENGTH, PCRE2_INFO_NAMECOUNT, PCRE2_INFO_NAMEENTRYSIZE,
    PCRE2_INFO_NEWLINE, PCRE2_INFO_DEPTHLIMIT, PCRE2_INFO_HASBACKSLASHC,
    PCRE2_INFO_FRAMESIZE, PCRE2_INFO_HEAPLIMIT, PCRE2_INFO_EXTRAOPTIONS };
  for (unsigned i = 0; i < sizeof(reqs)/sizeof(reqs[0]); i++)
    {
    u = 0xdeadbeef;
    rc = pcre2_pattern_info(re, reqs[i], &u);
    printf("  info[%d] rc=%d val=%u\n", reqs[i], rc, u);
    }
  rc = pcre2_pattern_info(re, PCRE2_INFO_SIZE, &z);
  printf("  info[SIZE] rc=%d val=%lu\n", rc, (unsigned long)z);
  rc = pcre2_pattern_info(re, PCRE2_INFO_FIRSTBITMAP, &bm);
  printf("  info[FIRSTBITMAP] rc=%d", rc);
  if (rc == 0 && bm != NULL)
    { printf(" map="); for (int i = 0; i < 32; i++) printf("%02x", bm[i]); }
  printf("\n");
  rc = pcre2_pattern_info(re, PCRE2_INFO_NAMECOUNT, &u);
  if (rc == 0 && u > 0)
    {
    uint32_t es; PCRE2_SPTR tab;
    pcre2_pattern_info(re, PCRE2_INFO_NAMEENTRYSIZE, &es);
    pcre2_pattern_info(re, PCRE2_INFO_NAMETABLE, &tab);
    for (uint32_t i = 0; i < u; i++)
      {
      const unsigned char *e = tab + i*es;
      printf("  name[%u] num=%d name=", i, (e[0] << 8) | e[1]);
      print_str(e + 2, strlen((const char *)e + 2));
      printf("\n");
      }
    }
  /* Serialize / deserialize round trip. */
  {
    uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
    const pcre2_code *codes[1]; codes[0] = re;
    int32_t src = pcre2_serialize_encode(codes, 1, &bytes, &blen, NULL);
    printf("  serialize rc=%d len=%lu ncodes=%d\n", src, (unsigned long)blen,
      src > 0 ? pcre2_serialize_get_number_of_codes(bytes) : -999);
    if (src > 0)
      {
      pcre2_code *out[1] = { NULL };
      int32_t drc = pcre2_serialize_decode(out, 1, bytes, NULL);
      printf("  deserialize rc=%d\n", drc);
      if (drc > 0 && out[0] != NULL)
        {
        PCRE2_SIZE sz1 = 0, sz2 = 0;
        pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz1);
        pcre2_pattern_info(out[0], PCRE2_INFO_SIZE, &sz2);
        printf("  deserialized size %lu (orig %lu)\n", (unsigned long)sz2, (unsigned long)sz1);
        pcre2_code_free(out[0]);
        }
      pcre2_serialize_free(bytes);
      }
  }
  printf("  enumerate rc=%d\n", pcre2_callout_enumerate(re, enum_fn, NULL));
}

/* ---------------- one match run ---------------- */

static void run_match(pcre2_code *re, const char *subject, size_t slen,
  uint32_t mopts, int use_dfa)
{
  pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
  pcre2_match_context *mc = pcre2_match_context_create(NULL);
  int rc;
  int wspace[200];
  pcre2_set_callout(mc, callout_fn, NULL);

  if (use_dfa)
    rc = pcre2_dfa_match(re, (PCRE2_SPTR)subject, slen, 0, mopts, md, mc, wspace, 200);
  else
    rc = pcre2_match(re, (PCRE2_SPTR)subject, slen, 0, mopts, md, mc);

  printf("  %s subject='", use_dfa ? "dfa" : "match");
  print_str((const unsigned char *)subject, slen);
  printf("' opts=0x%x rc=%d", mopts, rc);
  if (rc > 0)
    {
    PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
    printf(" ovec=");
    for (int i = 0; i < rc*2; i++)
      printf("%s%ld", i ? "," : "", (long)ov[i]);
    printf(" startchar=%lu", (unsigned long)pcre2_get_startchar(md));
    PCRE2_SPTR mark = pcre2_get_mark(md);
    if (mark != NULL) { printf(" mark="); print_str(mark, strlen((const char *)mark)); }
    }
  else if (rc == 0)
    printf(" (ovector too small)");
  printf("\n");

  /* substring extraction */
  if (rc > 0 && !use_dfa)
    {
    for (int i = 0; i < rc; i++)
      {
      PCRE2_UCHAR buf[256]; PCRE2_SIZE bl = sizeof(buf); PCRE2_SIZE len = 0;
      int r1 = pcre2_substring_length_bynumber(md, i, &len);
      int r2 = pcre2_substring_copy_bynumber(md, i, buf, &bl);
      printf("    sub[%d] lenrc=%d len=%lu copyrc=%d val='", i, r1,
        (unsigned long)len, r2);
      if (r2 == 0) print_str(buf, bl);
      printf("'\n");
      }
    {
      PCRE2_UCHAR **list = NULL; PCRE2_SIZE *lens = NULL;
      int r = pcre2_substring_list_get(md, &list, &lens);
      printf("    list rc=%d\n", r);
      if (r == 0)
        {
        for (int i = 0; list[i] != NULL; i++)
          { printf("    list[%d]='", i); print_str(list[i], lens[i]); printf("'\n"); }
        pcre2_substring_list_free(list);
        }
    }
    /* next_match iteration */
    {
      PCRE2_SIZE off = 0; uint32_t opts = 0;
      int n = 0;
      while (pcre2_next_match(md, &off, &opts) && n < 5)
        {
        printf("    next_match off=%lu opts=0x%x\n", (unsigned long)off, opts);
        int rc2 = pcre2_match(re, (PCRE2_SPTR)subject, slen, off, opts, md, mc);
        if (rc2 < 0) { printf("    next rc=%d\n", rc2); break; }
        PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
        printf("    next match ovec=%ld,%ld\n", (long)ov[0], (long)ov[1]);
        n++;
        }
    }
    }
  pcre2_match_context_free(mc);
  pcre2_match_data_free(md);
}

/* ---------------- substitution ---------------- */

static void run_subst(pcre2_code *re, const char *subject, const char *repl,
  uint32_t opts)
{
  PCRE2_UCHAR out[512];
  PCRE2_SIZE outlen = sizeof(out);
  pcre2_match_context *mc = pcre2_match_context_create(NULL);
  pcre2_set_substitute_callout(mc, subst_callout_fn, NULL);
  int rc = pcre2_substitute(re, (PCRE2_SPTR)subject, PCRE2_ZERO_TERMINATED, 0,
    opts, NULL, mc, (PCRE2_SPTR)repl, PCRE2_ZERO_TERMINATED, out, &outlen);
  printf("  subst subj='%s' repl='%s' opts=0x%x rc=%d len=%lu out='", subject, repl,
    opts, rc, (unsigned long)outlen);
  if (rc >= 0) print_str(out, outlen);
  printf("'\n");
  /* Length-only form */
  {
    PCRE2_SIZE need = 0;
    int rc2 = pcre2_substitute(re, (PCRE2_SPTR)subject, PCRE2_ZERO_TERMINATED, 0,
      opts | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, NULL, NULL, (PCRE2_SPTR)repl,
      PCRE2_ZERO_TERMINATED, NULL, &need);
    printf("  subst(len) rc=%d need=%lu\n", rc2, (unsigned long)need);
  }
  pcre2_match_context_free(mc);
}

/* ---------------- the pattern/subject corpus ---------------- */

static const char *patterns[] = {
  "abc", "a.c", "a.*c", "^abc$", "(a)(b)(c)", "a|b|c", "[a-z]+", "[^a-z]+",
  "\\d+", "\\D+", "\\w+", "\\W+", "\\s+", "\\S+", "\\bword\\b", "\\Bword",
  "a{2,4}", "a{2,}", "a{0,3}?", "a*+", "a++b", "(?i)ABC", "(?s)a.c", "(?m)^b",
  "(?x) a b c", "(a)(?1)", "(?<name>a)(?&name)", "(?P<x>a)\\k<x>",
  "(?:abc)", "(?=abc)abc", "(?!abc)...", "(?<=a)bc", "(?<!a)bc",
  "(?>a*)b", "(?|(a)|(b))", "a(?#comment)b", "(?(1)a|b)", "(?(?=a)a|b)",
  "\\Qa.c\\E", "\\p{L}+", "\\P{L}+", "\\X", "\\R", "\\h", "\\v", "\\H", "\\V",
  "[[:alpha:]]+", "[[:^digit:]]+", "[\\d\\s]+", "[a-c\\x{100}-\\x{200}]",
  "\\x{1000}", "\\x41", "\\101", "\\o{101}", "\\cA", "\\e", "\\a", "\\t\\n\\r\\f",
  "(*CR)a.b", "(*ANYCRLF)a$", "(*UTF)\\x{1234}", "(*UCP)\\w+",
  "a(*FAIL)", "a(*ACCEPT)b", "a(*MARK:m1)b", "a(*PRUNE)b", "a(*SKIP)b", "a(*THEN)b",
  "(?C1)abc", "(?C{cs})abc", "(?J)(?<n>a)|(?<n>b)",
  "\\A\\d\\z", "\\Z", "\\G", "(a)\\1", "(?1)(a)", "(?R)?a",
  "[]a]", "[^]a]", "[a-]", "[-a]", "((((((((((a))))))))))",
  "(?<=\\d{2})x", "(?<=a|bb|ccc)x", "\\K", "a\\Kb",
  "(?i)[\\x{100}-\\x{200}]", "\\p{Greek}\\p{Han}", "\\p{Script=Latin}",
  "\\p{scx:Cyrillic}", "\\p{Bidi_Class=AL}", "\\p{ASCII}", "\\p{Any}",
  "[\\p{L}--[a-z]]", "[\\p{L}&&\\p{Ll}]", "(?[\\p{L} - [a-z]])",
  "a(?=b)(?<=a)b", "(*script_run:\\w+)", "(*atomic_script_run:\\w+)",
  "(*scan_substring:(1))b", "(?i:strasse)", "(?-i)a", "(?^i)a",
  "x{1000000}", "a**", "(", ")", "[", "\\", "(?", "(?<", "(?P>",
  "\\p{Unknown}", "\\x{110000}", "a{4,2}", "[z-a]", "(?<n>a)(?<n>b)",
  "(*LIMIT_MATCH=100)a+", "(*LIMIT_DEPTH=5)(a+)+b", "(*NO_START_OPT)abc",
  "(*NOTEMPTY)a*", "(*CASELESS_RESTRICT)a", "(*TURKISH_CASING)i",
  NULL };

static const char *subjects[] = {
  "", "a", "abc", "ABC", "aaa", "aaabbbccc", "xyz", "a.c", "a\nb", "a\rb",
  "a\r\nb", "word boundary", "123456", " \t\n ", "wordword", "aXbXc",
  "\xc3\xa9\xc3\xa8", "\xe4\xb8\xad\xe6\x96\x87", "\xf0\x9f\x98\x80",
  "straße", "Ii\xc4\xb0\xc4\xb1", "\xcf\x80\xce\xb1", "ab]c", "-a-",
  "abcabcabc", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab", NULL };

static const uint32_t compile_opts[] = {
  0, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_EXTENDED,
  PCRE2_UTF, PCRE2_UTF|PCRE2_UCP, PCRE2_UNGREEDY, PCRE2_ANCHORED,
  PCRE2_NO_AUTO_CAPTURE, PCRE2_DUPNAMES, PCRE2_AUTO_CALLOUT,
  PCRE2_ALT_BSUX, PCRE2_LITERAL, PCRE2_ENDANCHORED, PCRE2_ALLOW_EMPTY_CLASS,
  PCRE2_FIRSTLINE, PCRE2_NO_START_OPTIMIZE, PCRE2_MATCH_UNSET_BACKREF,
  PCRE2_ALT_EXTENDED_CLASS, PCRE2_ALT_CIRCUMFLEX, PCRE2_EXTENDED_MORE };

static const uint32_t match_opts[] = {
  0, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART,
  PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED, PCRE2_ENDANCHORED };

int main(void)
{
  /* ---- pcre2_config ---- */
  {
    int rc; uint32_t u; char buf[128];
    for (int what = 0; what <= 17; what++)
      {
      if (what == PCRE2_CONFIG_JITTARGET || what == PCRE2_CONFIG_UNICODE_VERSION ||
          what == PCRE2_CONFIG_VERSION)
        {
        memset(buf, 0, sizeof(buf));
        rc = pcre2_config(what, buf);
        printf("config[%d] rc=%d str='%s'\n", what, rc, buf);
        rc = pcre2_config(what, NULL);
        printf("config[%d] len rc=%d\n", what, rc);
        }
      else
        {
        u = 0xdeadbeef;
        rc = pcre2_config(what, &u);
        printf("config[%d] rc=%d val=%u\n", what, rc, u);
        }
      }
  }

  /* ---- error messages ---- */
  for (int e = -80; e <= 225; e++)
    {
    PCRE2_UCHAR buf[256];
    int rc = pcre2_get_error_message(e, buf, sizeof(buf));
    printf("errmsg[%d] rc=%d '", e, rc);
    if (rc > 0) print_str(buf, rc);
    printf("'\n");
    }
  /* short buffer behaviour */
  {
    PCRE2_UCHAR buf[8];
    for (int size = 0; size <= 8; size++)
      {
      memset(buf, '#', sizeof(buf));
      int rc = pcre2_get_error_message(PCRE2_ERROR_NOMATCH, buf, size);
      printf("errmsg short size=%d rc=%d buf='", size, rc);
      print_str(buf, sizeof(buf));
      printf("'\n");
      }
  }

  /* ---- maketables ---- */
  {
    const uint8_t *t = pcre2_maketables(NULL);
    unsigned long sum = 0;
    if (t != NULL)
      {
      for (int i = 0; i < 1088; i++) sum = sum*31 + t[i];
      printf("maketables hash=%lu\n", sum);
      pcre2_maketables_free(NULL, t);
      }
    else printf("maketables NULL\n");
  }

  /* ---- pattern conversion ---- */
  {
    static const char *globs[] = { "*.c", "a?b", "[a-z]*", "**/x", "a/b\\*c",
      "!x", "[!a-z]", "a{b,c}", NULL };
    static const uint32_t copts[] = { PCRE2_CONVERT_GLOB,
      PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, PCRE2_CONVERT_GLOB_NO_STARSTAR,
      PCRE2_CONVERT_POSIX_BASIC, PCRE2_CONVERT_POSIX_EXTENDED };
    for (int i = 0; globs[i] != NULL; i++)
      for (unsigned j = 0; j < sizeof(copts)/sizeof(copts[0]); j++)
        {
        PCRE2_UCHAR *out = NULL; PCRE2_SIZE outlen = 0;
        int rc = pcre2_pattern_convert((PCRE2_SPTR)globs[i], PCRE2_ZERO_TERMINATED,
          copts[j], &out, &outlen, NULL);
        printf("convert '%s' opts=0x%x rc=%d out='", globs[i], copts[j], rc);
        if (rc == 0) print_str(out, outlen);
        printf("'\n");
        if (rc == 0) pcre2_converted_pattern_free(out);
        }
  }

  /* ---- compile + match everything ---- */
  for (int p = 0; patterns[p] != NULL; p++)
    {
    for (unsigned o = 0; o < sizeof(compile_opts)/sizeof(compile_opts[0]); o++)
      {
      int errcode; PCRE2_SIZE erroffset;
      pcre2_code *re = pcre2_compile((PCRE2_SPTR)patterns[p], PCRE2_ZERO_TERMINATED,
        compile_opts[o], &errcode, &erroffset, NULL);
      printf("PATTERN '%s' copts=0x%x -> %s", patterns[p], compile_opts[o],
        re == NULL ? "FAIL" : "OK");
      if (re == NULL)
        {
        PCRE2_UCHAR buf[256];
        pcre2_get_error_message(errcode, buf, sizeof(buf));
        printf(" err=%d offset=%lu msg='", errcode, (unsigned long)erroffset);
        print_str(buf, strlen((const char *)buf));
        printf("'\n");
        continue;
        }
      printf("\n");
      dump_info(re);
      /* a copy, to exercise code_copy */
      {
        pcre2_code *c1 = pcre2_code_copy(re);
        pcre2_code *c2 = pcre2_code_copy_with_tables(re);
        PCRE2_SIZE s1 = 0, s2 = 0;
        if (c1) pcre2_pattern_info(c1, PCRE2_INFO_SIZE, &s1);
        if (c2) pcre2_pattern_info(c2, PCRE2_INFO_SIZE, &s2);
        printf("  copy sizes %lu %lu\n", (unsigned long)s1, (unsigned long)s2);
        pcre2_code_free(c1); pcre2_code_free(c2);
      }
      for (int s = 0; subjects[s] != NULL; s++)
        {
        size_t slen = strlen(subjects[s]);
        for (unsigned m = 0; m < sizeof(match_opts)/sizeof(match_opts[0]); m++)
          {
          run_match(re, subjects[s], slen, match_opts[m], 0);
          run_match(re, subjects[s], slen, match_opts[m], 1);
          }
        }
      run_subst(re, "abcabc", "[$0]", 0);
      run_subst(re, "abcabc", "[$0]", PCRE2_SUBSTITUTE_GLOBAL);
      run_subst(re, "abcabc", "<${1:-none}>", PCRE2_SUBSTITUTE_EXTENDED);
      run_subst(re, "abcabc", "\\U$0\\E-\\L$0", PCRE2_SUBSTITUTE_EXTENDED);
      run_subst(re, "abcabc", "x", PCRE2_SUBSTITUTE_LITERAL|PCRE2_SUBSTITUTE_GLOBAL);
      run_subst(re, "abcabc", "$1", PCRE2_SUBSTITUTE_UNKNOWN_UNSET|PCRE2_SUBSTITUTE_UNSET_EMPTY);
      pcre2_code_free(re);
      }
    }

  /* ---- context setters (error codes) ---- */
  {
    pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
    pcre2_match_context *mc = pcre2_match_context_create(NULL);
    pcre2_convert_context *vc = pcre2_convert_context_create(NULL);
    printf("set_bsr: %d %d %d\n", pcre2_set_bsr(cc, 1), pcre2_set_bsr(cc, 2),
      pcre2_set_bsr(cc, 3));
    for (uint32_t n = 0; n <= 7; n++) printf("set_newline(%u)=%d\n", n, pcre2_set_newline(cc, n));
    for (uint32_t d = 0; d <= 70; d++)
      { int r = pcre2_set_optimize(cc, d); if (r != PCRE2_ERROR_BADOPTION)
          printf("set_optimize(%u)=%d\n", d, r); }
    printf("set_glob_sep: %d %d %d\n", pcre2_set_glob_separator(vc, '/'),
      pcre2_set_glob_separator(vc, 'x'), pcre2_set_glob_separator(vc, '.'));
    printf("set_glob_esc: %d %d %d %d\n", pcre2_set_glob_escape(vc, '\\'),
      pcre2_set_glob_escape(vc, 0), pcre2_set_glob_escape(vc, 'a'),
      pcre2_set_glob_escape(vc, 300));
    printf("set_limits: %d %d %d %d\n", pcre2_set_match_limit(mc, 100),
      pcre2_set_depth_limit(mc, 100), pcre2_set_heap_limit(mc, 100),
      pcre2_set_offset_limit(mc, 3));
    /* compile with a non-default context */
    pcre2_set_newline(cc, PCRE2_NEWLINE_ANYCRLF);
    pcre2_set_bsr(cc, PCRE2_BSR_ANYCRLF);
    pcre2_set_max_pattern_length(cc, 4);
    int ec; PCRE2_SIZE eo;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)"abcdefg", PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, cc);
    printf("maxlen compile: re=%s err=%d offset=%lu\n", re?"nonnull":"null", ec, (unsigned long)eo);
    pcre2_set_max_pattern_length(cc, PCRE2_UNSET);
    pcre2_set_parens_nest_limit(cc, 3);
    re = pcre2_compile((PCRE2_SPTR)"((((a))))", PCRE2_ZERO_TERMINATED, 0, &ec, &eo, cc);
    printf("nest compile: re=%s err=%d offset=%lu\n", re?"nonnull":"null", ec, (unsigned long)eo);
    pcre2_set_parens_nest_limit(cc, 250);
    pcre2_set_max_varlookbehind(cc, 1);
    re = pcre2_compile((PCRE2_SPTR)"(?<=ab|c)x", PCRE2_ZERO_TERMINATED, 0, &ec, &eo, cc);
    printf("varlb compile: re=%s err=%d offset=%lu\n", re?"nonnull":"null", ec, (unsigned long)eo);
    /* offset limit matching */
    pcre2_match_context *mc2 = pcre2_match_context_create(NULL);
    pcre2_set_offset_limit(mc2, 2);
    re = pcre2_compile((PCRE2_SPTR)"b", PCRE2_ZERO_TERMINATED, PCRE2_USE_OFFSET_LIMIT,
      &ec, &eo, NULL);
    if (re != NULL)
      {
      pcre2_match_data *md = pcre2_match_data_create(4, NULL);
      printf("offlimit rc=%d\n", pcre2_match(re, (PCRE2_SPTR)"aaaab", 5, 0, 0, md, mc2));
      pcre2_match_data_free(md);
      pcre2_code_free(re);
      }
    pcre2_match_context_free(mc2);
    pcre2_compile_context_free(cc);
    pcre2_match_context_free(mc);
    pcre2_convert_context_free(vc);
  }

  /* ---- error paths of the API ---- */
  {
    int ec; PCRE2_SIZE eo;
    printf("null pattern: %s\n", pcre2_compile(NULL, 0, 0, &ec, &eo, NULL)?"nonnull":"null");
    printf("  err=%d\n", ec);
    printf("bad options: %s", pcre2_compile((PCRE2_SPTR)"a", 1, 0x40000000u,
      &ec, &eo, NULL)?"nonnull":"null");
    printf(" err=%d\n", ec);
    pcre2_match_data *md = pcre2_match_data_create(0, NULL);
    printf("md oveccount=%u size=%lu hfsize=%lu\n", pcre2_get_ovector_count(md),
      (unsigned long)pcre2_get_match_data_size(md),
      (unsigned long)pcre2_get_match_data_heapframes_size(md));
    pcre2_match_data_free(md);
    pcre2_code_free(NULL);
    pcre2_match_data_free(NULL);
    printf("frees ok\n");
  }

  printf("DONE\n");
  return 0;
}
