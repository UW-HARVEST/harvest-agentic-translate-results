/* Randomised differential driver. A deterministic PRNG builds patterns from a
token pool and subjects from a byte pool, so both libraries see exactly the same
inputs; every observable result is printed for byte-comparison. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

static uint64_t rs;
static uint32_t rnd(uint32_t n)
{
rs = rs * 6364136223846793005ULL + 1442695040888963407ULL;
return (uint32_t)((rs >> 33) % n);
}

static const char *toks[] = {
  "a", "b", "c", "x", "1", "2", ".", "^", "$", "|", "-", "_", ":", "=", "!",
  "*", "+", "?", "*?", "+?", "??", "*+", "++", "?+",
  "{2}", "{1,3}", "{2,}", "{,3}", "{0,0}", "{1,2}?", "{1,2}+",
  "(", ")", "(?:", "(?i)", "(?-i)", "(?x)", "(?s)", "(?m)", "(?J)", "(?U)",
  "(?=", "(?!", "(?<=", "(?<!", "(?>", "(?|", "(?P<n>", "(?<m>", "(?'q'",
  "(?(1)", "(?(?=a)", "(?(<n>)", "(?(DEFINE)", "(?(R)", "(?(VERSION>=10)",
  "(?C1)", "(?C)", "(*sr:", "(*asr:", "(*atomic:", "(*script_run:",
  "(*napla:", "(*naplb:", "(*plb:", "(*pla:", "(*nlb:", "(*nla:",
  "(*MARK:m)", "(*ACCEPT)", "(*FAIL)", "(*COMMIT)", "(*PRUNE)", "(*SKIP)",
  "(*THEN)", "(*PRUNE:p)", "(*SKIP:s)", "(*THEN:t)", "(*NOTEMPTY)",
  "(*UTF)", "(*UCP)", "(*CR)", "(*LF)", "(*CRLF)", "(*ANY)", "(*ANYCRLF)",
  "(*BSR_UNICODE)", "(*BSR_ANYCRLF)", "(*LIMIT_MATCH=50)", "(*LIMIT_DEPTH=50)",
  "(*NO_AUTO_POSSESS)", "(*NO_START_OPT)", "(*NO_DOTSTAR_ANCHOR)",
  "[", "]", "[^", "[a-z]", "[^a-z]", "[abc]", "[]]", "[a-]", "[-a]",
  "[[:alpha:]]", "[[:^digit:]]", "[[:word:]]", "[[:space:]]", "[[:punct:]]",
  "[[:xdigit:]]", "[[:graph:]]", "[[:print:]]", "[[:cntrl:]]", "[[:upper:]]",
  "[\\d]", "[\\D]", "[\\s\\w]", "[\\x{100}]", "[\\x{100}-\\x{200}]",
  "[\\p{L}]", "[\\P{L}]", "[\\p{Greek}]", "[\\p{Han}]", "[a\\p{L}]",
  "[[:word:]a]", "[a[:digit:]z]", "[\\p{L}\\d_]", "[^\\p{L}a]",
  "(?[", "]) ", "&&", "||", "--", "~~", "!",
  "\\d", "\\D", "\\s", "\\S", "\\w", "\\W", "\\b", "\\B", "\\A", "\\Z",
  "\\z", "\\G", "\\K", "\\C", "\\R", "\\X", "\\N", "\\h", "\\H", "\\v", "\\V",
  "\\p{L}", "\\P{N}", "\\p{Lu}", "\\p{Nd}", "\\p{Any}", "\\p{Xan}", "\\p{Xsp}",
  "\\p{Xwd}", "\\p{Xps}", "\\p{Xuc}", "\\p{sc=Latin}", "\\p{scx:Greek}",
  "\\p{bidiclass=L}", "\\p{Alphabetic}", "\\p{ASCII}",
  "\\1", "\\2", "\\g1", "\\g{1}", "\\g{-1}", "\\g<1>", "\\k<n>", "\\k'q'",
  "\\k{n}", "(?1)", "(?2)", "(?-1)", "(?+1)", "(?R)", "(?&n)", "(?P>n)",
  "\\Q", "\\E", "\\x41", "\\x{41}", "\\101", "\\o{101}", "\\cA", "\\e",
  "\\a", "\\t", "\\n", "\\r", "\\f", "\\0", "\\8", "\\9",
  "\xc3\xa9", "\xe2\x82\xac", "\xf0\x9f\x98\x80", "\x80", "\xff",
  "#c\n", " ", "\t", "\n",
};
#define NTOKS (sizeof(toks)/sizeof(toks[0]))

static const char *subj_atoms[] = {
  "a", "b", "c", "x", "1", "2", "-", "_", " ", "\t", "\n", "\r", "\r\n",
  "A", "Z", "z", ":", ".", "[", "]", "(", ")", "\xc3\xa9", "\xe2\x82\xac",
  "\xf0\x9f\x98\x80", "\x80", "\xff", "\xc3", "abc", "aaa", "\0",
};
#define NSUBJ (sizeof(subj_atoms)/sizeof(subj_atoms[0]))

static const uint32_t copts[] = {
  0, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_EXTENDED,
  PCRE2_EXTENDED_MORE, PCRE2_UTF, PCRE2_UTF|PCRE2_UCP, PCRE2_UCP,
  PCRE2_ANCHORED, PCRE2_ENDANCHORED, PCRE2_UNGREEDY, PCRE2_NO_AUTO_CAPTURE,
  PCRE2_DUPNAMES, PCRE2_ALLOW_EMPTY_CLASS, PCRE2_AUTO_CALLOUT,
  PCRE2_FIRSTLINE, PCRE2_ALT_BSUX, PCRE2_ALT_CIRCUMFLEX, PCRE2_ALT_VERBNAMES,
  PCRE2_LITERAL, PCRE2_NO_START_OPTIMIZE, PCRE2_NO_AUTO_POSSESS,
  PCRE2_NO_DOTSTAR_ANCHOR, PCRE2_MATCH_UNSET_BACKREF, PCRE2_DOLLAR_ENDONLY,
  PCRE2_ALT_EXTENDED_CLASS, PCRE2_NEVER_BACKSLASH_C, PCRE2_MATCH_INVALID_UTF,
  PCRE2_UTF|PCRE2_MATCH_INVALID_UTF, PCRE2_CASELESS|PCRE2_UTF|PCRE2_UCP,
};
#define NCOPT (sizeof(copts)/sizeof(copts[0]))

static const uint32_t xopts[] = {
  0, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
  PCRE2_EXTRA_MATCH_WORD, PCRE2_EXTRA_MATCH_LINE, PCRE2_EXTRA_ESCAPED_CR_IS_LF,
  PCRE2_EXTRA_ALT_BSUX, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
  PCRE2_EXTRA_CASELESS_RESTRICT, PCRE2_EXTRA_ASCII_BSD, PCRE2_EXTRA_ASCII_BSS,
  PCRE2_EXTRA_ASCII_BSW, PCRE2_EXTRA_ASCII_POSIX, PCRE2_EXTRA_ASCII_DIGIT,
  PCRE2_EXTRA_PYTHON_OCTAL, PCRE2_EXTRA_NO_BS0, PCRE2_EXTRA_TURKISH_CASING,
};
#define NXOPT (sizeof(xopts)/sizeof(xopts[0]))

static const uint32_t mopts[] = {
  0, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART,
  PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED, PCRE2_ENDANCHORED,
  PCRE2_COPY_MATCHED_SUBJECT, PCRE2_DISABLE_RECURSELOOP_CHECK,
};
/* PCRE2_NO_UTF_CHECK is deliberately absent: combined with the invalid UTF-8
subjects generated below it is documented undefined behaviour, and both
libraries then read out of bounds. */
#define NMOPT (sizeof(mopts)/sizeof(mopts[0]))

static int callout_fn(pcre2_callout_block *cb, void *data)
{
(void)data;
printf("   cb n=%u top=%u last=%u pp=%zu nil=%zu cp=%zu sm=%u fl=%u",
  cb->callout_number, cb->capture_top, cb->capture_last,
  cb->pattern_position, cb->next_item_length, cb->current_position,
  (unsigned)cb->start_match, cb->callout_flags);
if (cb->callout_string != NULL)
  printf(" s=<%.*s>", (int)cb->callout_string_length,
    (const char *)cb->callout_string);
printf("\n");
return (int)(cb->callout_number % 3) - 1;   /* 0, 1, or -1 */
}

static int subcall_fn(pcre2_substitute_callout_block *scb, void *data)
{
(void)data;
printf("   scb count=%u ovec=%u out=[%zu,%zu]\n", scb->subscount,
  scb->oveccount, scb->output_offsets[0], scb->output_offsets[1]);
return (int)(scb->subscount % 3) - 1;
}

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

int main(int argc, char **argv)
{
uint64_t seed = (argc > 1) ? strtoull(argv[1], NULL, 0) : 1;
long iters = (argc > 2) ? strtol(argv[2], NULL, 0) : 2000;
rs = seed;

char pat[512];
unsigned char subj[256];

for (long it = 0; it < iters; it++)
  {
  /* Build a pattern */
  size_t pl = 0;
  uint32_t ntok = 1 + rnd(14);
  for (uint32_t i = 0; i < ntok; i++)
    {
    const char *t = toks[rnd(NTOKS)];
    size_t tl = strlen(t);
    if (pl + tl + 1 >= sizeof(pat)) break;
    memcpy(pat + pl, t, tl);
    pl += tl;
    }
  pat[pl] = 0;

  /* Build a subject (may contain NULs, so keep an explicit length) */
  size_t sl = 0;
  uint32_t nat = rnd(8);
  for (uint32_t i = 0; i < nat; i++)
    {
    const char *a = subj_atoms[rnd(NSUBJ)];
    size_t al = (a[0] == 0) ? 1 : strlen(a);
    if (sl + al >= sizeof(subj)) break;
    memcpy(subj + sl, a, al);
    sl += al;
    }

  uint32_t copt = copts[rnd(NCOPT)] | ((rnd(4) == 0) ? copts[rnd(NCOPT)] : 0);
  uint32_t xopt = xopts[rnd(NXOPT)];
  uint32_t mopt = mopts[rnd(NMOPT)] | ((rnd(4) == 0) ? mopts[rnd(NMOPT)] : 0);

  printf("--- it=%ld\n", it);
  emit("pat", (const unsigned char *)pat, pl);
  emit("subj", subj, sl);
  printf("copt=%u xopt=%u mopt=%u\n", copt, xopt, mopt);

  pcre2_compile_context *cc = pcre2_compile_context_create(NULL);
  pcre2_set_compile_extra_options(cc, xopt);
  if (rnd(8) == 0) pcre2_set_newline(cc, 1 + rnd(6));
  if (rnd(8) == 0) pcre2_set_bsr(cc, 1 + rnd(2));
  if (rnd(8) == 0) pcre2_set_max_varlookbehind(cc, rnd(300));
  if (rnd(8) == 0) pcre2_set_optimize(cc, rnd(2));

  int errcode = 0; PCRE2_SIZE erroffset = 0;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)pat, pl, copt, &errcode,
    &erroffset, cc);
  if (re == NULL)
    {
    PCRE2_UCHAR eb[256];
    int r = pcre2_get_error_message(errcode, eb, sizeof(eb));
    printf("compile FAIL code=%d off=%zu msgrc=%d <%s>\n", errcode, erroffset,
      r, (char *)eb);
    pcre2_compile_context_free(cc);
    continue;
    }
  printf("compile OK\n");

  /* Compiled byte code, via serialization */
  {
  uint8_t *bytes = NULL; PCRE2_SIZE blen = 0;
  int rc = pcre2_serialize_encode((const pcre2_code **)&re, 1, &bytes, &blen, NULL);
  printf("ser rc=%d len=%zu\n", rc, blen);
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
  }

  /* Pattern info */
  {
  static const uint32_t what[] = {
    PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_EXTRAOPTIONS,
    PCRE2_INFO_BACKREFMAX, PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT,
    PCRE2_INFO_FIRSTCODEUNIT, PCRE2_INFO_FIRSTCODETYPE, PCRE2_INFO_HASCRORLF,
    PCRE2_INFO_JCHANGED, PCRE2_INFO_LASTCODEUNIT, PCRE2_INFO_LASTCODETYPE,
    PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MAXLOOKBEHIND, PCRE2_INFO_MINLENGTH,
    PCRE2_INFO_NAMECOUNT, PCRE2_INFO_NAMEENTRYSIZE, PCRE2_INFO_NEWLINE,
    PCRE2_INFO_HASBACKSLASHC };
  for (size_t i = 0; i < sizeof(what)/sizeof(what[0]); i++)
    {
    uint32_t v = 0xdeadbeef;
    printf("i%u=%d/%u ", what[i], pcre2_pattern_info(re, what[i], &v), v);
    }
  putchar('\n');
  PCRE2_SIZE sz = 0;
  printf("isize=%d/%zu ", pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz), sz);
  sz = 0;
  printf("iframe=%d/%zu\n", pcre2_pattern_info(re, PCRE2_INFO_FRAMESIZE, &sz), sz);
  const uint8_t *bm = NULL;
  if (pcre2_pattern_info(re, PCRE2_INFO_FIRSTBITMAP, &bm) == 0 && bm != NULL)
    emit("bitmap", bm, 32);
  }

  pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
  pcre2_match_context *mc = pcre2_match_context_create(NULL);
  if (rnd(3) == 0) pcre2_set_callout(mc, callout_fn, NULL);
  if (rnd(6) == 0) pcre2_set_match_limit(mc, 1 + rnd(500));
  if (rnd(6) == 0) pcre2_set_depth_limit(mc, 1 + rnd(200));
  if (rnd(6) == 0) pcre2_set_heap_limit(mc, rnd(64));
  pcre2_set_substitute_callout(mc, subcall_fn, NULL);

  for (PCRE2_SIZE so = 0; so <= sl; so++)
    {
    int rc = pcre2_match(re, subj, sl, so, mopt, md, mc);
    printf("m so=%zu rc=%d", so, rc);
    if (rc > 0 || rc == PCRE2_ERROR_PARTIAL)
      {
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
      uint32_t n = (rc > 0) ? (uint32_t)rc : 1;
      for (uint32_t i = 0; i < n; i++)
        printf(" [%zd,%zd]", (ssize_t)ov[2*i], (ssize_t)ov[2*i+1]);
      printf(" sc=%zu", pcre2_get_startchar(md));
      PCRE2_SPTR mk = pcre2_get_mark(md);
      if (mk != NULL) printf(" mk=<%s>", (const char *)mk);
      }
    putchar('\n');

    if (rc > 0)
      {
      for (int i = 0; i < rc && i < 4; i++)
        {
        PCRE2_UCHAR sb[128]; PCRE2_SIZE sbl = sizeof(sb);
        int r2 = pcre2_substring_copy_bynumber(md, i, sb, &sbl);
        printf("  s%d=%d/%zu", i, r2, sbl);
        if (r2 == 0) emit("", sb, sbl);
        else putchar('\n');
        }
      PCRE2_SIZE ns = 0; uint32_t nopt = 0;
      printf("  nextm=%d", pcre2_next_match(md, &ns, &nopt));
      printf(" ns=%zu nopt=%u\n", ns, nopt);
      }

    /* DFA matching, with its own match data and workspace */
    {
    int wsp[64];
    pcre2_match_data *dmd = pcre2_match_data_create(8, NULL);
    int rc2 = pcre2_dfa_match(re, subj, sl, so, mopt, dmd, mc, wsp, 64);
    printf("d so=%zu rc=%d", so, rc2);
    if (rc2 > 0 || rc2 == PCRE2_ERROR_PARTIAL)
      {
      PCRE2_SIZE *ov = pcre2_get_ovector_pointer(dmd);
      uint32_t n = (rc2 > 0) ? (uint32_t)rc2 : 1;
      for (uint32_t i = 0; i < n; i++)
        printf(" [%zd,%zd]", (ssize_t)ov[2*i], (ssize_t)ov[2*i+1]);
      }
    putchar('\n');
    pcre2_match_data_free(dmd);
    }
    }

  /* Substitution */
  {
  static const char *reps[] = { "X", "[$0]", "<$1>", "${1}y", "\\U$0\\E",
    "\\l$1", "$*", "a$", "${name}", "\\Q$1\\E", "" };
  static const uint32_t sopts[] = { 0, PCRE2_SUBSTITUTE_GLOBAL,
    PCRE2_SUBSTITUTE_EXTENDED,
    PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_EXTENDED,
    PCRE2_SUBSTITUTE_LITERAL, PCRE2_SUBSTITUTE_UNSET_EMPTY,
    PCRE2_SUBSTITUTE_UNKNOWN_UNSET, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    PCRE2_SUBSTITUTE_REPLACEMENT_ONLY };
  for (int k = 0; k < 3; k++)
    {
    const char *rep = reps[rnd(sizeof(reps)/sizeof(reps[0]))];
    uint32_t so2 = sopts[rnd(sizeof(sopts)/sizeof(sopts[0]))];
    PCRE2_UCHAR out[600]; PCRE2_SIZE ol = sizeof(out);
    memset(out, 0, sizeof(out));
    int rc = pcre2_substitute(re, subj, sl, 0, so2 | mopt, md, mc,
      (PCRE2_SPTR)rep, strlen(rep), out, &ol);
    printf("sub <%s> opt=%u rc=%d len=%zu ", rep, so2, rc, ol);
    if (rc >= 0) emit("out", out, ol); else putchar('\n');
    }
  }

  pcre2_match_context_free(mc);
  pcre2_match_data_free(md);
  pcre2_code_free(re);
  pcre2_compile_context_free(cc);
  }

printf("== fuzz done ==\n");
return 0;
}
