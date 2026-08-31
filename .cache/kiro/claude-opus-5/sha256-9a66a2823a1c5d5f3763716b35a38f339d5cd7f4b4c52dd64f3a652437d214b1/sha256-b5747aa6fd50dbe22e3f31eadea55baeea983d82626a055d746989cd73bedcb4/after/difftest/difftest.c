/* Differential test driver: exercises the PCRE2 8-bit API and prints everything
it observes, so that the output of the C library and the Rust translation can be
compared byte for byte. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static void dump(const char *tag, const unsigned char *s, size_t n)
{
printf("%s[%zu]=", tag, n);
for (size_t i = 0; i < n; i++)
  {
  unsigned c = s[i];
  if (c >= 32 && c < 127 && c != '\\') putchar(c); else printf("\\x%02x", c);
  }
putchar('\n');
}

/* ---------------- config and version ---------------- */

static void test_config(void)
{
uint32_t u; char buf[128]; int rc;
printf("== config ==\n");
for (uint32_t what = 0; what <= 17; what++)
  {
  rc = pcre2_config(what, NULL);
  printf("config(%u) len=%d\n", what, rc);
  if (rc < 0) continue;
  if (what == PCRE2_CONFIG_JITTARGET || what == PCRE2_CONFIG_UNICODE_VERSION ||
      what == PCRE2_CONFIG_VERSION)
    {
    memset(buf, 0, sizeof(buf));
    rc = pcre2_config(what, buf);
    printf("config(%u) rc=%d str=<%s>\n", what, rc, buf);
    }
  else
    {
    u = 0xdeadbeef;
    rc = pcre2_config(what, &u);
    printf("config(%u) rc=%d val=%u\n", what, rc, u);
    }
  }
}

/* ---------------- error messages ---------------- */

static void test_errors(void)
{
PCRE2_UCHAR buf[256];
printf("== errors ==\n");
for (int e = -80; e <= 230; e++)
  {
  memset(buf, 0, sizeof(buf));
  int rc = pcre2_get_error_message(e, buf, sizeof(buf));
  printf("err %d rc=%d <%s>\n", e, rc, (char *)buf);
  }
/* Short buffer behaviour */
for (size_t sz = 0; sz < 12; sz++)
  {
  memset(buf, '#', sizeof(buf));
  int rc = pcre2_get_error_message(101, buf, sz);
  printf("err101 sz=%zu rc=%d ", sz, rc);
  dump("buf", buf, 12);
  }
}

/* ---------------- patterns ---------------- */

static const char *patterns[] = {
  "abc", "a.c", "a.*c", "^abc$", "(a)(b)(c)", "a{2,4}", "a{2,}", "a{0}",
  "[a-z]+", "[^a-z]+", "[[:alpha:]]+", "[[:^digit:]]", "\\d+", "\\D+",
  "\\s+", "\\S+", "\\w+", "\\W+", "\\bfoo\\b", "\\Bfoo", "a|b|c",
  "(?i)ABC", "(?x) a b c ", "(?s)a.c", "(?m)^b", "(?:abc)+", "(?>a+)b",
  "(?=abc)a", "(?!abc)a", "(?<=abc)d", "(?<!abc)d", "(?<name>a)(?P<n2>b)",
  "(?P<x>a)\\k<x>", "(a)\\1", "(a)(?1)", "(?R)?a", "a(?1)?b", "(?(1)a|b)(c)",
  "(?(?=a)a|b)", "(?(DEFINE)(?<w>a))(?&w)", "\\p{L}+", "\\p{Greek}",
  "\\P{Nd}", "\\pL", "\\X", "\\R", "\\H", "\\V", "\\h", "\\v", "\\N",
  "\\Qa.b\\Ec", "\\A\\z", "\\Z", "\\G", "\\K", "\\Cx",
  "a*+", "a++", "a?+", "a{1,3}+", "(a)*+", "[abc]*+",
  "(*UTF)\\x{100}", "(*UCP)\\w", "(*CRLF)a$", "(*ANY)a$", "(*ANYCRLF)a$",
  "(*BSR_ANYCRLF)\\R", "(*LIMIT_MATCH=100)a", "(*NO_AUTO_POSSESS)a+b",
  "(*MARK:x)a", "a(*FAIL)", "a(*ACCEPT)b", "a(*COMMIT)b", "a(*PRUNE)b",
  "a(*SKIP)b", "a(*THEN)b", "(*sr:abc)", "(*asr:abc)", "(*atomic:abc)",
  "(*napla:a)", "(*naplb:a)", "(*plb:a)", "(*positive_lookahead:a)",
  "(?C1)abc", "(?C`x`)abc", "\\x41", "\\x{41}", "\\101", "\\o{101}",
  "\\cA", "\\e", "\\a", "\\t", "\\n", "\\r", "\\f", "\\0", "\\8",
  "[\\x{100}-\\x{200}]", "[\\d\\s]", "[a-\\d]", "[]a]", "[^]a]",
  "[[:word:]-]", "[a[:digit:]z]", "[\\p{L}\\p{N}]",
  "(?[ \\p{L} & \\p{Ll} ])", "(?[ [a-z] - [aeiou] ])", "[[a-z]&&[^aeiou]]",
  "a(?i:b)c", "(?-i)a", "(?i-x:a)", "(?^i:a)", "(?n)(a)(b)",
  "(?|(a)|(b))", "(?<a>x)(?<a>y)", "((((((((((a))))))))))",
  "(?1)(a)", "\\g{1}(a)", "\\g1(a)", "\\g{-1}(a)", "(a)\\g{-1}",
  "x{2,4}?", "x{2,4}+", "(?U)a+", "a\\b", "\\Qab", "[\\Qa]\\E]",
  /* Deliberately invalid patterns, to compare error codes and offsets */
  "(", ")", "[", "a{4,2}", "a**", "(?<", "\\", "(?P", "[z-a]",
  "\\p{Xyz}", "(?(1)a)", "\\g{999}", "(?&nope)", "a{100000}",
  "(?i", "(*VERB)", "(*MARK)", "\\x{110000}", "[[:foo:]]", "(?#comment",
  "((?", "a\\", "(?'", "(?<>a)", "\\k<>", "(?1", "(?()a)", "(?[)",
  NULL
};

static const char *subjects[] = {
  "", "a", "abc", "ABC", "aaa", "aaaa", "xabcx", "a.c", "a\nb", "a\r\nb",
  "b", "d", "abcd", "foo", "xfoo", "foox", "123", "a1b2", " \t ",
  "\xc3\xa9", "\xe2\x82\xac", "hello world", "aaaaaaaaaaaaaaaaaaaab",
  "\xf0\x9f\x98\x80", "AbC", "ab", "ba", "cab", "xyz", "_", "-",
  NULL
};

static const uint32_t compile_opts[] = {
  0, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_EXTENDED,
  PCRE2_UTF, PCRE2_UTF|PCRE2_UCP, PCRE2_ANCHORED, PCRE2_UNGREEDY,
  PCRE2_NO_AUTO_CAPTURE, PCRE2_DUPNAMES, PCRE2_ALLOW_EMPTY_CLASS,
  PCRE2_AUTO_CALLOUT, PCRE2_ENDANCHORED, PCRE2_FIRSTLINE,
  PCRE2_ALT_BSUX, PCRE2_LITERAL, PCRE2_NO_START_OPTIMIZE,
  PCRE2_ALT_EXTENDED_CLASS, PCRE2_NO_AUTO_POSSESS, PCRE2_NO_DOTSTAR_ANCHOR
};

static const uint32_t match_opts[] = {
  0, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART,
  PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED, PCRE2_ENDANCHORED
};

static int callout_fn(pcre2_callout_block *cb, void *data)
{
(void)data;
printf("   callout n=%u top=%u last=%u pos=%zu nil=%zu flags=%u",
  cb->callout_number, cb->capture_top, cb->capture_last,
  cb->pattern_position, cb->next_item_length, cb->callout_flags);
if (cb->callout_string != NULL)
  printf(" str=<%.*s>", (int)cb->callout_string_length,
    (const char *)cb->callout_string);
printf("\n");
return 0;
}

static void show_info(pcre2_code *re)
{
uint32_t u32; PCRE2_SIZE sz; const uint8_t *bm;
static const uint32_t what[] = {
  PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_EXTRAOPTIONS,
  PCRE2_INFO_BACKREFMAX, PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT,
  PCRE2_INFO_FIRSTCODEUNIT, PCRE2_INFO_FIRSTCODETYPE, PCRE2_INFO_HASCRORLF,
  PCRE2_INFO_JCHANGED, PCRE2_INFO_JITSIZE, PCRE2_INFO_LASTCODEUNIT,
  PCRE2_INFO_LASTCODETYPE, PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MATCHLIMIT,
  PCRE2_INFO_MAXLOOKBEHIND, PCRE2_INFO_MINLENGTH, PCRE2_INFO_NAMECOUNT,
  PCRE2_INFO_NAMEENTRYSIZE, PCRE2_INFO_NEWLINE, PCRE2_INFO_DEPTHLIMIT,
  PCRE2_INFO_HASBACKSLASHC, PCRE2_INFO_HEAPLIMIT };
for (size_t i = 0; i < sizeof(what)/sizeof(what[0]); i++)
  {
  u32 = 0xdeadbeef;
  int rc = pcre2_pattern_info(re, what[i], &u32);
  printf("  info %u rc=%d val=%u\n", what[i], rc, u32);
  }
sz = 0; printf("  info SIZE rc=%d val=%zu\n",
  pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz), sz);
sz = 0; printf("  info FRAMESIZE rc=%d val=%zu\n",
  pcre2_pattern_info(re, PCRE2_INFO_FRAMESIZE, &sz), sz);
bm = NULL;
if (pcre2_pattern_info(re, PCRE2_INFO_FIRSTBITMAP, &bm) == 0 && bm != NULL)
  dump("  firstbitmap", bm, 32);
{
PCRE2_SPTR nt = NULL;
uint32_t nc = 0, nes = 0;
pcre2_pattern_info(re, PCRE2_INFO_NAMECOUNT, &nc);
pcre2_pattern_info(re, PCRE2_INFO_NAMEENTRYSIZE, &nes);
if (nc > 0 && pcre2_pattern_info(re, PCRE2_INFO_NAMETABLE, &nt) == 0)
  for (uint32_t i = 0; i < nc; i++)
    {
    const uint8_t *e = nt + i * nes;
    printf("  name %u: num=%u <%s>\n", i, (e[0] << 8) | e[1], (const char *)(e + 2));
    }
}
}

static void run_matches(pcre2_code *re, const char *pat, uint32_t copt)
{
pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
pcre2_match_context *mc = pcre2_match_context_create(NULL);
pcre2_set_callout(mc, callout_fn, NULL);
printf("  mdsize=%zu hfsize=%zu ovecount=%u\n",
  pcre2_get_match_data_size(md), pcre2_get_match_data_heapframes_size(md),
  pcre2_get_ovector_count(md));

for (const char **s = subjects; *s != NULL; s++)
  {
  size_t slen = strlen(*s);
  for (size_t oi = 0; oi < sizeof(match_opts)/sizeof(match_opts[0]); oi++)
    {
    uint32_t mopt = match_opts[oi];
    int rc = pcre2_match(re, (PCRE2_SPTR)*s, slen, 0, mopt, md, mc);
    printf("  M pat=<%s> copt=%u subj=<%s> mopt=%u rc=%d", pat, copt, *s, mopt, rc);
    if (rc > 0)
      {
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
      for (int i = 0; i < rc; i++)
        printf(" [%zd,%zd]", (ssize_t)ov[2*i], (ssize_t)ov[2*i+1]);
      printf(" startchar=%zu", pcre2_get_startchar(md));
      PCRE2_SPTR mk = pcre2_get_mark(md);
      if (mk != NULL) printf(" mark=<%s>", (const char *)mk);
      }
    else if (rc == PCRE2_ERROR_PARTIAL)
      {
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
      printf(" partial [%zd,%zd]", (ssize_t)ov[0], (ssize_t)ov[1]);
      }
    printf("\n");

    /* Substring extraction for successful matches */
    if (rc > 0)
      {
      for (int i = 0; i < rc; i++)
        {
        PCRE2_UCHAR sbuf[128]; PCRE2_SIZE sl = sizeof(sbuf);
        int r2 = pcre2_substring_copy_bynumber(md, i, sbuf, &sl);
        printf("   sub %d rc=%d len=%zu <%s>\n", i, r2, sl,
          r2 == 0 ? (char *)sbuf : "");
        PCRE2_SIZE ll = 0;
        printf("   sublen %d rc=%d len=%zu\n", i,
          pcre2_substring_length_bynumber(md, i, &ll), ll);
        PCRE2_UCHAR *gp = NULL; PCRE2_SIZE gl = 0;
        int r3 = pcre2_substring_get_bynumber(md, i, &gp, &gl);
        printf("   subget %d rc=%d len=%zu <%s>\n", i, r3, gl,
          r3 == 0 ? (char *)gp : "");
        if (r3 == 0) pcre2_substring_free(gp);
        }
      PCRE2_UCHAR **lst = NULL;
      int r4 = pcre2_substring_list_get(md, &lst, NULL);
      printf("   list rc=%d\n", r4);
      if (r4 == 0) pcre2_substring_list_free(lst);
      }

    /* DFA matching */
    {
    int wsp[128];
    pcre2_match_data *dmd = pcre2_match_data_create(16, NULL);
    int rc2 = pcre2_dfa_match(re, (PCRE2_SPTR)*s, slen, 0,
      mopt & ~(uint32_t)0, dmd, mc, wsp, 128);
    printf("  D pat=<%s> subj=<%s> mopt=%u rc=%d", pat, *s, mopt, rc2);
    if (rc2 > 0)
      {
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(dmd);
      for (int i = 0; i < rc2; i++)
        printf(" [%zd,%zd]", (ssize_t)ov[2*i], (ssize_t)ov[2*i+1]);
      }
    printf("\n");
    pcre2_match_data_free(dmd);
    }
    }
  }
pcre2_match_context_free(mc);
pcre2_match_data_free(md);
}

static void test_substitute(void)
{
static const char *reps[] = { "X", "[$0]", "<$1>", "${1}x", "$*", "\\U$0",
  "\\l$0", "a\\nb", "$2", "${name}", "$", "\\", "\\Q$1\\E", NULL };
static const char *pats[] = { "a", "(a)(b)?", "(?<name>a)", "", "a*", NULL };
uint32_t opts[] = { 0, PCRE2_SUBSTITUTE_GLOBAL, PCRE2_SUBSTITUTE_EXTENDED,
  PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_EXTENDED,
  PCRE2_SUBSTITUTE_LITERAL, PCRE2_SUBSTITUTE_UNSET_EMPTY,
  PCRE2_SUBSTITUTE_UNKNOWN_UNSET|PCRE2_SUBSTITUTE_UNSET_EMPTY,
  PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY };
printf("== substitute ==\n");
for (const char **p = pats; *p != NULL; p++)
  {
  int errcode; PCRE2_SIZE erroffset;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, 0,
    &errcode, &erroffset, NULL);
  if (re == NULL) { printf("sub compile <%s> failed %d\n", *p, errcode); continue; }
  for (const char **r = reps; *r != NULL; r++)
    for (size_t oi = 0; oi < sizeof(opts)/sizeof(opts[0]); oi++)
      for (const char **s = subjects; *s != NULL; s++)
        {
        PCRE2_UCHAR out[512]; PCRE2_SIZE outlen = sizeof(out);
        memset(out, 0, sizeof(out));
        int rc = pcre2_substitute(re, (PCRE2_SPTR)*s, strlen(*s), 0, opts[oi],
          NULL, NULL, (PCRE2_SPTR)*r, strlen(*r), out, &outlen);
        printf("S <%s> <%s> opt=%u subj=<%s> rc=%d len=%zu out=<%s>\n",
          *p, *r, opts[oi], *s, rc, outlen, rc >= 0 ? (char *)out : "");
        /* Also probe the length-only / too-small buffer path */
        PCRE2_SIZE small = 2;
        PCRE2_UCHAR sout[2];
        rc = pcre2_substitute(re, (PCRE2_SPTR)*s, strlen(*s), 0, opts[oi],
          NULL, NULL, (PCRE2_SPTR)*r, strlen(*r), sout, &small);
        printf("S small rc=%d len=%zu\n", rc, small);
        }
  pcre2_code_free(re);
  }
}

static void test_convert(void)
{
static const char *globs[] = { "*.c", "a?b", "**/x", "[a-z]*", "a/b",
  "\\*", "{a,b}", "*", "", "a**b", NULL };
uint32_t opts[] = { PCRE2_CONVERT_GLOB, PCRE2_CONVERT_POSIX_BASIC,
  PCRE2_CONVERT_POSIX_EXTENDED, PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
  PCRE2_CONVERT_GLOB_NO_STARSTAR, PCRE2_CONVERT_GLOB|PCRE2_CONVERT_UTF };
printf("== convert ==\n");
for (const char **g = globs; *g != NULL; g++)
  for (size_t oi = 0; oi < sizeof(opts)/sizeof(opts[0]); oi++)
    {
    PCRE2_UCHAR *out = NULL; PCRE2_SIZE outlen = 0;
    int rc = pcre2_pattern_convert((PCRE2_SPTR)*g, PCRE2_ZERO_TERMINATED,
      opts[oi], &out, &outlen, NULL);
    printf("C <%s> opt=%u rc=%d len=%zu out=<%s>\n", *g, opts[oi], rc, outlen,
      rc == 0 ? (char *)out : "");
    if (rc == 0) pcre2_converted_pattern_free(out);
    }
}

static void test_serialize(void)
{
printf("== serialize ==\n");
int errcode; PCRE2_SIZE erroffset;
pcre2_code *list[3];
const char *pats[3] = { "abc", "(a)(b)", "\\p{L}+" };
for (int i = 0; i < 3; i++)
  {
  list[i] = pcre2_compile((PCRE2_SPTR)pats[i], PCRE2_ZERO_TERMINATED,
    PCRE2_UTF, &errcode, &erroffset, NULL);
  if (list[i] == NULL) { printf("ser compile fail %d\n", errcode); return; }
  }
uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
int rc = pcre2_serialize_encode((const pcre2_code **)list, 3, &bytes, &blen, NULL);
printf("encode rc=%d len=%zu\n", rc, blen);
if (rc > 0)
  {
  printf("numcodes=%d\n", pcre2_serialize_get_number_of_codes(bytes));
  pcre2_code *out[3] = { NULL, NULL, NULL };
  int rc2 = pcre2_serialize_decode(out, 3, bytes, NULL);
  printf("decode rc=%d\n", rc2);
  if (rc2 > 0)
    {
    pcre2_match_data *md = pcre2_match_data_create(8, NULL);
    for (int i = 0; i < 3; i++)
      {
      int r = pcre2_match(out[i], (PCRE2_SPTR)"abc", 3, 0, 0, md, NULL);
      printf("decoded %d match rc=%d\n", i, r);
      pcre2_code_free(out[i]);
      }
    pcre2_match_data_free(md);
    }
  pcre2_serialize_free(bytes);
  }
for (int i = 0; i < 3; i++) pcre2_code_free(list[i]);
}

static void test_maketables(void)
{
printf("== maketables ==\n");
const uint8_t *t = pcre2_maketables(NULL);
if (t == NULL) { printf("maketables NULL\n"); return; }
dump("tables", t, 1088);
pcre2_maketables_free(NULL, (uint8_t *)t);
}

static void test_jit_stubs(void)
{
printf("== jit stubs ==\n");
int errcode; PCRE2_SIZE erroffset;
pcre2_code *re = pcre2_compile((PCRE2_SPTR)"abc", PCRE2_ZERO_TERMINATED, 0,
  &errcode, &erroffset, NULL);
printf("jit_compile rc=%d\n", pcre2_jit_compile(re, PCRE2_JIT_COMPLETE));
printf("jit_compile bad rc=%d\n", pcre2_jit_compile(re, 0x80000000u));
printf("jit_compile null rc=%d\n", pcre2_jit_compile(NULL, PCRE2_JIT_COMPLETE));
pcre2_match_data *md = pcre2_match_data_create(4, NULL);
printf("jit_match rc=%d\n",
  pcre2_jit_match(re, (PCRE2_SPTR)"abc", 3, 0, 0, md, NULL));
printf("jit_stack_create=%p\n", (void *)pcre2_jit_stack_create(1, 2, NULL));
pcre2_jit_free_unused_memory(NULL);
pcre2_match_data_free(md);
pcre2_code_free(re);
}

static void test_next_match(void)
{
printf("== next_match ==\n");
int errcode; PCRE2_SIZE erroffset;
static const char *pats[] = { "a", "a*", "(?<x>b)", "", NULL };
for (const char **p = pats; *p != NULL; p++)
  {
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, 0,
    &errcode, &erroffset, NULL);
  if (re == NULL) continue;
  pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
  const char *subj = "abcabca";
  int rc = pcre2_match(re, (PCRE2_SPTR)subj, strlen(subj), 0, 0, md, NULL);
  printf("N <%s> first rc=%d\n", *p, rc);
  for (int i = 0; i < 6 && rc > 0; i++)
    {
    PCRE2_SIZE nstart = 0; uint32_t nopts = 0;
    int more = pcre2_next_match(md, &nstart, &nopts);
    printf("N  more=%d start=%zu opts=%u\n", more, nstart, nopts);
    if (!more) break;
    rc = pcre2_match(re, (PCRE2_SPTR)subj, strlen(subj), nstart, nopts, md, NULL);
    PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
    printf("N  next rc=%d [%zd,%zd]\n", rc, (ssize_t)ov[0], (ssize_t)ov[1]);
    }
  pcre2_match_data_free(md);
  pcre2_code_free(re);
  }
}

static void test_contexts(void)
{
printf("== contexts ==\n");
pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
printf("set_bsr ok=%d bad=%d\n", pcre2_set_bsr(cc, 1), pcre2_set_bsr(cc, 99));
printf("set_newline ok=%d bad=%d\n", pcre2_set_newline(cc, 3),
  pcre2_set_newline(cc, 0));
printf("set_optimize %d %d %d %d\n", pcre2_set_optimize(cc, 0),
  pcre2_set_optimize(cc, 1), pcre2_set_optimize(cc, 64),
  pcre2_set_optimize(cc, 200));
printf("set_max_varlookbehind=%d\n", pcre2_set_max_varlookbehind(cc, 10));
printf("set_parens_nest=%d\n", pcre2_set_parens_nest_limit(cc, 10));
pcre2_compile_context *cc2 = pcre2_compile_context_copy(cc);
printf("copy=%d\n", cc2 != NULL);
pcre2_compile_context_free(cc2);
pcre2_compile_context_free(cc);

pcre2_convert_context *vc = pcre2_convert_context_create(NULL);
printf("glob_sep %d %d %d\n", pcre2_set_glob_separator(vc, '/'),
  pcre2_set_glob_separator(vc, 'x'), pcre2_set_glob_separator(vc, '.'));
printf("glob_esc %d %d %d\n", pcre2_set_glob_escape(vc, '\\'),
  pcre2_set_glob_escape(vc, 'x'), pcre2_set_glob_escape(vc, 0));
pcre2_convert_context_free(vc);

pcre2_match_context *mc = pcre2_match_context_create(NULL);
printf("limits %d %d %d %d\n", pcre2_set_match_limit(mc, 100),
  pcre2_set_depth_limit(mc, 100), pcre2_set_heap_limit(mc, 100),
  pcre2_set_offset_limit(mc, 100));
pcre2_match_context_free(mc);
}

static void test_compile_all(void)
{
printf("== compile/match ==\n");
for (const char **p = patterns; *p != NULL; p++)
  for (size_t oi = 0; oi < sizeof(compile_opts)/sizeof(compile_opts[0]); oi++)
    {
    int errcode = 0; PCRE2_SIZE erroffset = 0;
    pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED,
      compile_opts[oi], &errcode, &erroffset, cc);
    printf("P <%s> opt=%u ", *p, compile_opts[oi]);
    if (re == NULL)
      {
      PCRE2_UCHAR ebuf[256];
      pcre2_get_error_message(errcode, ebuf, sizeof(ebuf));
      printf("FAIL code=%d off=%zu <%s>\n", errcode, erroffset, (char *)ebuf);
      }
    else
      {
      printf("OK\n");
      show_info(re);
      run_matches(re, *p, compile_opts[oi]);
      pcre2_code *cp = pcre2_code_copy(re);
      pcre2_code *cpt = pcre2_code_copy_with_tables(re);
      if (cp != NULL)
        {
        pcre2_match_data *md = pcre2_match_data_create_from_pattern(cp, NULL);
        printf("  copy match rc=%d\n",
          pcre2_match(cp, (PCRE2_SPTR)"abc", 3, 0, 0, md, NULL));
        pcre2_match_data_free(md);
        pcre2_code_free(cp);
        }
      if (cpt != NULL) pcre2_code_free(cpt);
      pcre2_code_free(re);
      }
    pcre2_compile_context_free(cc);
    }
}

/* Dump the serialized (i.e. compiled) form of every pattern, so that the
compiled byte code of the two libraries can be compared directly. */

static void test_serialize_dump(void)
{
printf("== serialize dump ==\n");
for (const char **p = patterns; *p != NULL; p++)
  for (size_t oi = 0; oi < sizeof(compile_opts)/sizeof(compile_opts[0]); oi++)
    {
    int errcode = 0; PCRE2_SIZE erroffset = 0;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED,
      compile_opts[oi], &errcode, &erroffset, NULL);
    printf("Z <%s> opt=%u ", *p, compile_opts[oi]);
    if (re == NULL) { printf("FAIL %d %zu\n", errcode, erroffset); continue; }
    uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
    int rc = pcre2_serialize_encode((const pcre2_code **)&re, 1, &bytes, &blen, NULL);
    printf("rc=%d len=%zu\n", rc, blen);
    if (rc > 0)
      {
      for (PCRE2_SIZE k = 0; k < blen; k++)
        {
        printf("%02x", bytes[k]);
        if ((k % 32) == 31) putchar('\n');
        }
      putchar('\n');
      pcre2_serialize_free(bytes);
      }
    pcre2_code_free(re);
    }
}

int main(int argc, char **argv)
{
const char *sec = (argc > 1) ? argv[1] : "all";
#define SEC(name, fn) if (strcmp(sec, "all") == 0 || strcmp(sec, name) == 0) fn();
SEC("config", test_config)
SEC("errors", test_errors)
SEC("contexts", test_contexts)
SEC("maketables", test_maketables)
SEC("jit", test_jit_stubs)
SEC("serdump", test_serialize_dump)
SEC("compile", test_compile_all)
SEC("substitute", test_substitute)
SEC("convert", test_convert)
SEC("serialize", test_serialize)
SEC("next", test_next_match)
printf("== done ==\n");
return 0;
}
