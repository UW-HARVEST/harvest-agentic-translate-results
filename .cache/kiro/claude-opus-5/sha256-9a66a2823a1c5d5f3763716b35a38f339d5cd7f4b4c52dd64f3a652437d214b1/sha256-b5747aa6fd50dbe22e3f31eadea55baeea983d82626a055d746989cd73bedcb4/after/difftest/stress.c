/* Targeted differential tests for areas the random fuzzer under-covers:
resource limits, deep backtracking, many capture groups, long subjects, DFA
restart, substitution case callouts, name lookups, locale tables, and
code copying. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <locale.h>

static void emit(const char *tag, const unsigned char *s, size_t n)
{
printf("%s[%zu]=", tag, n);
for (size_t i = 0; i < n; i++)
  {
  unsigned c = s[i];
  if (c >= 32 && c < 127 && c != '\\') putchar(c); else printf("\\x%02x", c);
  }
putchar('\n');
}

/* ---------------- resource limits and deep backtracking ---------------- */

static void test_limits(void)
{
static const char *pats[] = {
  "(a+)+b", "(a|aa)+c", "(?:a?){20}b", "(a*)*b", "((a)*)*c",
  "a{1,1000}b", "(?:(?:(?:(?:a)*)*)*)*b", "(\\w+\\s?)+$", "(?R)?a",
  "a(?1)?b(c)", "(?:ab|a)*c", "\\b(\\w+)\\b\\s+\\1", NULL };
static const char *subs[] = {
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "abababababababababababab",
  "the the quick brown fox", "aaab", "", NULL };
static const uint32_t mlim[] = { 0, 1, 10, 100, 1000, 100000 };
static const uint32_t dlim[] = { 0, 1, 5, 50, 1000 };
static const uint32_t hlim[] = { 0, 1, 8, 64, 1000000 };

printf("== limits ==\n");
for (const char **p = pats; *p != NULL; p++)
  {
  int ec; PCRE2_SIZE eo;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, 0,
    &ec, &eo, NULL);
  if (re == NULL) { printf("L <%s> FAIL %d\n", *p, ec); continue; }
  pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
  PCRE2_SIZE fs = 0;
  pcre2_pattern_info(re, PCRE2_INFO_FRAMESIZE, &fs);
  printf("L <%s> framesize=%zu mdsize=%zu\n", *p, fs,
    pcre2_get_match_data_size(md));
  for (const char **s = subs; *s != NULL; s++)
    for (size_t i = 0; i < sizeof(mlim)/sizeof(mlim[0]); i++)
      {
      pcre2_match_context *mc = pcre2_match_context_create(NULL);
      pcre2_set_match_limit(mc, mlim[i]);
      pcre2_set_depth_limit(mc, dlim[i % (sizeof(dlim)/sizeof(dlim[0]))]);
      pcre2_set_heap_limit(mc, hlim[i % (sizeof(hlim)/sizeof(hlim[0]))]);
      int rc = pcre2_match(re, (PCRE2_SPTR)*s, strlen(*s), 0, 0, md, mc);
      printf("  m ml=%u rc=%d hf=%zu\n", mlim[i], rc,
        pcre2_get_match_data_heapframes_size(md));
      int wsp[32];
      pcre2_match_data *dmd = pcre2_match_data_create(4, NULL);
      rc = pcre2_dfa_match(re, (PCRE2_SPTR)*s, strlen(*s), 0, 0, dmd, mc,
        wsp, 32);
      printf("  d ml=%u rc=%d\n", mlim[i], rc);
      pcre2_match_data_free(dmd);
      pcre2_match_context_free(mc);
      }
  pcre2_match_data_free(md);
  pcre2_code_free(re);
  }
}

/* ---------------- many capture groups ---------------- */

static void test_many_groups(void)
{
printf("== many groups ==\n");
for (int n = 1; n <= 300; n += 37)
  {
  char *pat = malloc((size_t)n * 8 + 32);
  char *subj = malloc((size_t)n + 2);
  size_t pl = 0;
  for (int i = 0; i < n; i++) pl += (size_t)sprintf(pat + pl, "(a)");
  pat[pl] = 0;
  for (int i = 0; i < n; i++) subj[i] = 'a';
  subj[n] = 0;

  int ec; PCRE2_SIZE eo;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)pat, pl, 0, &ec, &eo, NULL);
  if (re == NULL) { printf("G n=%d FAIL %d\n", n, ec); free(pat); free(subj); continue; }
  uint32_t cc = 0; PCRE2_SIZE fs = 0, sz = 0;
  pcre2_pattern_info(re, PCRE2_INFO_CAPTURECOUNT, &cc);
  pcre2_pattern_info(re, PCRE2_INFO_FRAMESIZE, &fs);
  pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz);
  pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
  int rc = pcre2_match(re, (PCRE2_SPTR)subj, (size_t)n, 0, 0, md, NULL);
  printf("G n=%d cc=%u frame=%zu size=%zu rc=%d ovec=%u\n", n, cc, fs, sz, rc,
    pcre2_get_ovector_count(md));
  if (rc > 0)
    {
    PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
    for (int i = 0; i < rc && i < 8; i++)
      printf("  [%zd,%zd]", (ssize_t)ov[2*i], (ssize_t)ov[2*i+1]);
    printf("\n");
    }
  pcre2_match_data_free(md);
  pcre2_code_free(re);
  free(pat); free(subj);
  }
}

/* ---------------- long subjects (bumpalong / memchr paths) ---------------- */

static void test_long_subject(void)
{
printf("== long subject ==\n");
static const char *pats[] = { "needle", "n[e]edle", "^needle", "needle$",
  "\\bneedle\\b", "(?i)NEEDLE", "z+needle", "(?:.*)needle", "x{2,}needle",
  "\\Aneedle", "needle|haystack", NULL };
size_t len = 60000;
char *subj = malloc(len + 32);
memset(subj, 'x', len);
memcpy(subj + 40000, "needle", 6);
memcpy(subj + 6000, "haystack", 8);
subj[len] = 0;

for (const char **p = pats; *p != NULL; p++)
  for (int nso = 0; nso < 2; nso++)
    {
    int ec; PCRE2_SIZE eo;
    uint32_t opt = nso ? PCRE2_NO_START_OPTIMIZE : 0;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, opt,
      &ec, &eo, NULL);
    if (re == NULL) { printf("X <%s> FAIL %d\n", *p, ec); continue; }
    pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
    for (PCRE2_SIZE so = 0; so <= len; so += 9973)
      {
      int rc = pcre2_match(re, (PCRE2_SPTR)subj, len, so, 0, md, NULL);
      printf("X <%s> nso=%d so=%zu rc=%d", *p, nso, so, rc);
      if (rc > 0)
        {
        PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
        printf(" [%zd,%zd] sc=%zu", (ssize_t)ov[0], (ssize_t)ov[1],
          pcre2_get_startchar(md));
        }
      printf("\n");
      }
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }
free(subj);
}

/* ---------------- DFA restart and workspace sizes ---------------- */

static void test_dfa_restart(void)
{
printf("== dfa restart ==\n");
static const char *pats[] = { "abcd", "a.*d", "(a|b)+cd", "\\d{3}-\\d{4}",
  "^ab", "ab$", "(?:ab)+", NULL };
const char *part1 = "abc";
const char *part2 = "d123-4567";
for (const char **p = pats; *p != NULL; p++)
  for (int ws = 8; ws <= 128; ws *= 2)
    {
    int ec; PCRE2_SIZE eo;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    if (re == NULL) continue;
    pcre2_match_data *md = pcre2_match_data_create(8, NULL);
    int *wsp = calloc((size_t)ws, sizeof(int));
    int rc = pcre2_dfa_match(re, (PCRE2_SPTR)part1, strlen(part1), 0,
      PCRE2_PARTIAL_HARD, md, NULL, wsp, ws);
    printf("R <%s> ws=%d first rc=%d\n", *p, ws, rc);
    if (rc == PCRE2_ERROR_PARTIAL)
      {
      rc = pcre2_dfa_match(re, (PCRE2_SPTR)part2, strlen(part2), 0,
        PCRE2_DFA_RESTART, md, NULL, wsp, ws);
      printf("R <%s> ws=%d restart rc=%d\n", *p, ws, rc);
      }
    /* Deliberate bad restart data */
    rc = pcre2_dfa_match(re, (PCRE2_SPTR)part2, strlen(part2), 0,
      PCRE2_DFA_RESTART, md, NULL, wsp, ws);
    printf("R <%s> ws=%d badrestart rc=%d\n", *p, ws, rc);
    free(wsp);
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }
}

/* ---------------- substitution case handling ---------------- */

static PCRE2_SIZE case_callout(PCRE2_SPTR input, PCRE2_SIZE inlen,
  PCRE2_UCHAR *output, PCRE2_SIZE outlen, int type, void *data)
{
(void)data;
printf("   casecb type=%d inlen=%zu outlen=%zu\n", type, inlen, outlen);
if (outlen < inlen) return ~(PCRE2_SIZE)0;
for (PCRE2_SIZE i = 0; i < inlen; i++)
  {
  unsigned c = input[i];
  if (type == PCRE2_SUBSTITUTE_CASE_UPPER && c >= 'a' && c <= 'z') c -= 32;
  else if (type == PCRE2_SUBSTITUTE_CASE_LOWER && c >= 'A' && c <= 'Z') c += 32;
  else if (type == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST && i == 0 &&
           c >= 'a' && c <= 'z') c -= 32;
  output[i] = (PCRE2_UCHAR)c;
  }
return inlen;
}

static void test_subst_case(void)
{
printf("== substitute case ==\n");
static const char *reps[] = { "\\U$0", "\\L$0", "\\u$0", "\\l$0",
  "\\U$1\\E-\\L$2", "\\u\\L$0", "a\\Ub\\Ec", "\\U", "\\E", "\\u", NULL };
static const char *pats[] = { "(\\w)(\\w+)", "\\w+", "(a)(b)", NULL };
static const char *subs[] = { "hello World", "ab", "AB", "", NULL };
for (int usecb = 0; usecb < 2; usecb++)
  for (const char **p = pats; *p != NULL; p++)
    {
    int ec; PCRE2_SIZE eo;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED, 0,
      &ec, &eo, NULL);
    if (re == NULL) continue;
    pcre2_match_context *mc = pcre2_match_context_create(NULL);
    if (usecb) pcre2_set_substitute_case_callout(mc, case_callout, NULL);
    for (const char **r = reps; *r != NULL; r++)
      for (const char **s = subs; *s != NULL; s++)
        {
        PCRE2_UCHAR out[256]; PCRE2_SIZE ol = sizeof(out);
        memset(out, 0, sizeof(out));
        int rc = pcre2_substitute(re, (PCRE2_SPTR)*s, strlen(*s), 0,
          PCRE2_SUBSTITUTE_EXTENDED|PCRE2_SUBSTITUTE_GLOBAL, NULL, mc,
          (PCRE2_SPTR)*r, strlen(*r), out, &ol);
        printf("C cb=%d <%s> <%s> <%s> rc=%d ", usecb, *p, *r, *s, rc);
        if (rc >= 0) emit("out", out, ol); else printf("\n");
        }
    pcre2_match_context_free(mc);
    pcre2_code_free(re);
    }
}

/* ---------------- names, tables, copies ---------------- */

static void test_names_and_copies(void)
{
printf("== names/copies ==\n");
static const char *pats[] = {
  "(?<a>x)(?<b>y)(?<c>z)", "(?<dup>a)|(?<dup>b)", "(?<n1>.)(?<n2>.)",
  "(?<verylongnamehere>q)", "(a)(?<mid>b)(c)", NULL };
for (const char **p = pats; *p != NULL; p++)
  for (int dup = 0; dup < 2; dup++)
    {
    int ec; PCRE2_SIZE eo;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED,
      dup ? PCRE2_DUPNAMES : 0, &ec, &eo, NULL);
    if (re == NULL) { printf("N <%s> dup=%d FAIL %d\n", *p, dup, ec); continue; }
    printf("N <%s> dup=%d OK\n", *p, dup);
    static const char *names[] = { "a", "b", "c", "dup", "n1", "n2", "mid",
      "verylongnamehere", "nope", "", NULL };
    pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
    int rc = pcre2_match(re, (PCRE2_SPTR)"xyzabcq", 7, 0, 0, md, NULL);
    printf("  match rc=%d\n", rc);
    for (const char **nm = names; *nm != NULL; nm++)
      {
      printf("  num <%s> = %d\n", *nm,
        pcre2_substring_number_from_name(re, (PCRE2_SPTR)*nm));
      PCRE2_UCHAR *first, *last;
      int r2 = pcre2_substring_nametable_scan(re, (PCRE2_SPTR)*nm,
        &first, &last);
      printf("  scan <%s> = %d\n", *nm, r2);
      PCRE2_UCHAR buf[64]; PCRE2_SIZE bl = sizeof(buf);
      printf("  copyname <%s> = %d", *nm,
        pcre2_substring_copy_byname(md, (PCRE2_SPTR)*nm, buf, &bl));
      printf(" len=%zu\n", bl);
      PCRE2_SIZE ll = 0;
      printf("  lenname <%s> = %d len=%zu\n", *nm,
        pcre2_substring_length_byname(md, (PCRE2_SPTR)*nm, &ll), ll);
      PCRE2_UCHAR *gp = NULL; PCRE2_SIZE gl = 0;
      int r3 = pcre2_substring_get_byname(md, (PCRE2_SPTR)*nm, &gp, &gl);
      printf("  getname <%s> = %d len=%zu\n", *nm, r3, gl);
      if (r3 == 0) pcre2_substring_free(gp);
      }
    /* Copies */
    pcre2_code *c1 = pcre2_code_copy(re);
    pcre2_code *c2 = pcre2_code_copy_with_tables(re);
    for (int k = 0; k < 2; k++)
      {
      pcre2_code *cc = k ? c2 : c1;
      if (cc == NULL) { printf("  copy%d NULL\n", k); continue; }
      PCRE2_SIZE sz = 0;
      pcre2_pattern_info(cc, PCRE2_INFO_SIZE, &sz);
      pcre2_match_data *m2 = pcre2_match_data_create_from_pattern(cc, NULL);
      printf("  copy%d size=%zu rc=%d\n", k, sz,
        pcre2_match(cc, (PCRE2_SPTR)"xyzabcq", 7, 0, 0, m2, NULL));
      pcre2_match_data_free(m2);
      pcre2_code_free(cc);
      }
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }
}

static void test_locale_tables(void)
{
printf("== locale tables ==\n");
static const char *locs[] = { "C", "POSIX", "en_US.UTF-8", "de_DE", NULL };
for (const char **l = locs; *l != NULL; l++)
  {
  const char *got = setlocale(LC_CTYPE, *l);
  printf("locale <%s> -> <%s>\n", *l, got ? got : "(null)");
  const uint8_t *t = pcre2_maketables(NULL);
  if (t == NULL) { printf("  tables NULL\n"); continue; }
  unsigned sum = 0;
  for (int i = 0; i < 1088; i++) sum = sum * 31 + t[i];
  printf("  hash=%u\n", sum);
  pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
  pcre2_set_character_tables(cc, t);
  int ec; PCRE2_SIZE eo;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)"[[:alpha:]]+", PCRE2_ZERO_TERMINATED,
    PCRE2_CASELESS, &ec, &eo, cc);
  if (re != NULL)
    {
    pcre2_match_data *md = pcre2_match_data_create(4, NULL);
    printf("  match rc=%d\n",
      pcre2_match(re, (PCRE2_SPTR)"Hello", 5, 0, 0, md, NULL));
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }
  else printf("  compile FAIL %d\n", ec);
  pcre2_compile_context_free(cc);
  pcre2_maketables_free(NULL, (uint8_t *)t);
  }
setlocale(LC_CTYPE, "C");
}

/* ---------------- serialize round trips ---------------- */

static void test_serialize_many(void)
{
printf("== serialize many ==\n");
static const char *pats[] = { "abc", "(a)(b)(c)", "\\p{L}+", "(?<n>x)",
  "[a-z]{2,4}", "(?i)WORD", "a(?R)?b", NULL };
int n = 0;
pcre2_code *list[8];
for (const char **p = pats; *p != NULL; p++)
  {
  int ec; PCRE2_SIZE eo;
  list[n] = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED,
    PCRE2_UTF, &ec, &eo, NULL);
  if (list[n] != NULL) n++;
  }
printf("compiled %d\n", n);
uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
int rc = pcre2_serialize_encode((const pcre2_code **)list, n, &bytes, &blen, NULL);
printf("encode rc=%d len=%zu numcodes=%d\n", rc, blen,
  rc > 0 ? pcre2_serialize_get_number_of_codes(bytes) : -999);
if (rc > 0)
  {
  for (PCRE2_SIZE k = 0; k < blen; k++)
    {
    printf("%02x", bytes[k]);
    if ((k % 32) == 31) putchar('\n');
    }
  putchar('\n');
  /* Decode with too-small count, then correctly */
  pcre2_code *out[8];
  printf("decode small rc=%d\n", pcre2_serialize_decode(out, 1, bytes, NULL));
  pcre2_code_free(out[0]);
  int rc2 = pcre2_serialize_decode(out, n, bytes, NULL);
  printf("decode rc=%d\n", rc2);
  for (int i = 0; i < rc2; i++)
    {
    pcre2_match_data *md = pcre2_match_data_create_from_pattern(out[i], NULL);
    printf("  decoded %d rc=%d\n", i,
      pcre2_match(out[i], (PCRE2_SPTR)"abcxword", 8, 0, 0, md, NULL));
    pcre2_match_data_free(md);
    pcre2_code_free(out[i]);
    }
  /* Corrupt the data in a few ways */
  for (size_t off = 0; off < 20 && off < blen; off += 4)
    {
    uint8_t saved = bytes[off];
    bytes[off] ^= 0xff;
    printf("corrupt off=%zu rc=%d\n", off,
      pcre2_serialize_decode(out, n, bytes, NULL));
    bytes[off] = saved;
    }
  pcre2_serialize_free(bytes);
  }
for (int i = 0; i < n; i++) pcre2_code_free(list[i]);
}

int main(int argc, char **argv)
{
const char *sec = (argc > 1) ? argv[1] : "all";
#define SEC(name, fn) if (strcmp(sec, "all") == 0 || strcmp(sec, name) == 0) fn();
SEC("limits", test_limits)
SEC("groups", test_many_groups)
SEC("long", test_long_subject)
SEC("dfa", test_dfa_restart)
SEC("case", test_subst_case)
SEC("names", test_names_and_copies)
SEC("locale", test_locale_tables)
SEC("sermany", test_serialize_many)
printf("== stress done ==\n");
return 0;
}
