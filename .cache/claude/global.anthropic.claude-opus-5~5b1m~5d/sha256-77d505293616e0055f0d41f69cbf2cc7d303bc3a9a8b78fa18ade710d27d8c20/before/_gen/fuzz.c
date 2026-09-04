/* Deterministic random differential test for PCRE2. Link against the C and the
   Rust libpcre2.so and diff the outputs. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"

static unsigned long long rngstate = 88172645463325252ULL;

static unsigned rnd(unsigned n)
{
  rngstate ^= rngstate << 13;
  rngstate ^= rngstate >> 7;
  rngstate ^= rngstate << 17;
  return (unsigned)((rngstate >> 11) % n);
}

/* Pattern fragments, chosen to exercise lots of the compiler and matcher. */
static const char *frags[] = {
  "a", "b", "c", "z", "0", "9", "_", " ", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S",
  "\\h", "\\v", "\\R", "\\X", "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G", "\\K",
  ".", "[a-c]", "[^a-c]", "[[:alpha:]]", "[\\d\\s]", "[a-\\x{200}]", "[^\\W]",
  "(a)", "(?:b)", "(?<n1>c)", "(?'n2'd)", "(?P<n3>e)", "(?|(x)|(y))",
  "(?=a)", "(?!a)", "(?<=a)", "(?<!a)", "(?>a+)", "(?(1)a|b)", "(?(?=a)b|c)",
  "*", "+", "?", "{2}", "{1,3}", "{2,}", "*?", "+?", "??", "{1,3}?", "*+", "++",
  "?+", "{1,3}+", "|", "^", "$", "\\1", "\\2", "\\k<n1>", "(?1)", "(?&n1)", "(?R)",
  "\\p{L}", "\\P{L}", "\\p{Nd}", "\\p{Greek}", "\\p{Han}", "\\p{ASCII}",
  "\\x41", "\\x{1234}", "\\101", "\\o{101}", "\\cA", "\\e", "\\n", "\\r", "\\t",
  "(*MARK:m)", "(*FAIL)", "(*ACCEPT)", "(*PRUNE)", "(*SKIP)", "(*THEN)", "(*COMMIT)",
  "(?C1)", "(?i)", "(?-i)", "(?s)", "(?m)", "(?x)", "(?J)", "(?U)",
  "(*script_run:a+)", "(*atomic:a+)", "(*pla:a)", "(*nlb:a)",
  "[\\p{L}&&\\p{Ll}]", "(?[\\p{L}-[a-z]])", "\\Qa+b\\E", "(*NO_START_OPT)",
  "(?<n4>a)(?<n4>b)", "\\g{1}", "\\g1", "\\g{-1}", "(?-1)", "(?+1)",
};
#define NFRAGS (sizeof(frags)/sizeof(frags[0]))

static const char *subjfrags[] = {
  "a", "b", "c", "ab", "abc", "aaa", "A", "Z", "0", "12", " ", "\t", "\n", "\r\n",
  "_", "x", "\xc3\xa9", "\xe4\xb8\xad", "\xf0\x9f\x98\x80", "\xc2\x85", "ss",
  "\xcf\x80", "\xd0\xb0", "!", "]", "[", "-", "\x7f", "\xc4\xb0",
};
#define NSUBJ (sizeof(subjfrags)/sizeof(subjfrags[0]))

static const uint32_t copts[] = {
  0, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_EXTENDED, PCRE2_UNGREEDY,
  PCRE2_UTF, PCRE2_UTF|PCRE2_UCP, PCRE2_DUPNAMES, PCRE2_ANCHORED, PCRE2_AUTO_CALLOUT,
  PCRE2_NO_AUTO_CAPTURE, PCRE2_NO_START_OPTIMIZE, PCRE2_ENDANCHORED, PCRE2_FIRSTLINE,
  PCRE2_ALLOW_EMPTY_CLASS, PCRE2_ALT_EXTENDED_CLASS, PCRE2_MATCH_UNSET_BACKREF,
  PCRE2_NO_AUTO_POSSESS, PCRE2_NO_DOTSTAR_ANCHOR, PCRE2_EXTENDED_MORE, PCRE2_LITERAL,
};
#define NCOPTS (sizeof(copts)/sizeof(copts[0]))

static const uint32_t mopts[] = {
  0, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART,
  PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_ANCHORED, PCRE2_ENDANCHORED,
  PCRE2_DISABLE_RECURSELOOP_CHECK, PCRE2_COPY_MATCHED_SUBJECT,
};
#define NMOPTS (sizeof(mopts)/sizeof(mopts[0]))

static void print_str(const unsigned char *s, size_t len)
{
  for (size_t i = 0; i < len; i++)
    {
    unsigned c = s[i];
    if (c >= 32 && c < 127 && c != '\\') putchar(c);
    else printf("\\x%02x", c);
    }
}

static int callout_fn(pcre2_callout_block *cb, void *data)
{
  (void)data;
  printf("      co %u %u %lu %lu %lu %u\n", cb->callout_number, cb->capture_top,
    (unsigned long)cb->start_match, (unsigned long)cb->current_position,
    (unsigned long)cb->pattern_position, cb->callout_flags);
  return (cb->callout_number == 1)? 0 : 0;
}

int main(int argc, char **argv)
{
  int iters = (argc > 1)? atoi(argv[1]) : 4000;
  if (argc > 2) rngstate = strtoull(argv[2], NULL, 10) | 1;
  char pat[512], subj[256];
  int wspace[300];

  for (int n = 0; n < iters; n++)
    {
    /* Build a random pattern. */
    int nf = 1 + rnd(6);
    pat[0] = 0;
    for (int i = 0; i < nf; i++)
      {
      const char *f = frags[rnd(NFRAGS)];
      if (strlen(pat) + strlen(f) + 1 >= sizeof(pat)) break;
      strcat(pat, f);
      }
    uint32_t co = copts[rnd(NCOPTS)];
    if (rnd(4) == 0) co |= copts[rnd(NCOPTS)];

    int errcode; PCRE2_SIZE erroffset;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, co,
      &errcode, &erroffset, NULL);
    printf("[%d] pat='", n); print_str((const unsigned char *)pat, strlen(pat));
    printf("' co=0x%x -> ", co);
    if (re == NULL)
      {
      PCRE2_UCHAR buf[200];
      pcre2_get_error_message(errcode, buf, sizeof(buf));
      printf("err=%d off=%lu msg='%s'\n", errcode, (unsigned long)erroffset, (char *)buf);
      continue;
      }

    /* Info */
    {
      uint32_t cc = 0, fcu = 0, fct = 0, lcu = 0, lct = 0, ml = 0, mlb = 0, opts = 0;
      PCRE2_SIZE sz = 0;
      pcre2_pattern_info(re, PCRE2_INFO_CAPTURECOUNT, &cc);
      pcre2_pattern_info(re, PCRE2_INFO_FIRSTCODEUNIT, &fcu);
      pcre2_pattern_info(re, PCRE2_INFO_FIRSTCODETYPE, &fct);
      pcre2_pattern_info(re, PCRE2_INFO_LASTCODEUNIT, &lcu);
      pcre2_pattern_info(re, PCRE2_INFO_LASTCODETYPE, &lct);
      pcre2_pattern_info(re, PCRE2_INFO_MINLENGTH, &ml);
      pcre2_pattern_info(re, PCRE2_INFO_MAXLOOKBEHIND, &mlb);
      pcre2_pattern_info(re, PCRE2_INFO_ALLOPTIONS, &opts);
      pcre2_pattern_info(re, PCRE2_INFO_SIZE, &sz);
      printf("ok cc=%u fcu=%u fct=%u lcu=%u lct=%u ml=%u mlb=%u opts=0x%x size=%lu\n",
        cc, fcu, fct, lcu, lct, ml, mlb, opts, (unsigned long)sz);
      const uint8_t *bm = NULL;
      if (pcre2_pattern_info(re, PCRE2_INFO_FIRSTBITMAP, &bm) == 0 && bm != NULL)
        { printf("    bm="); for (int i = 0; i < 32; i++) printf("%02x", bm[i]); printf("\n"); }
    }

    /* Match against a few random subjects. */
    pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
    pcre2_match_context *mc = pcre2_match_context_create(NULL);
    pcre2_set_callout(mc, callout_fn, NULL);
    for (int s = 0; s < 4; s++)
      {
      int ns = rnd(5);
      subj[0] = 0;
      for (int i = 0; i < ns; i++)
        {
        const char *f = subjfrags[rnd(NSUBJ)];
        if (strlen(subj) + strlen(f) + 1 >= sizeof(subj)) break;
        strcat(subj, f);
        }
      size_t slen = strlen(subj);
      uint32_t mo = mopts[rnd(NMOPTS)];

      int rc = pcre2_match(re, (PCRE2_SPTR)subj, slen, 0, mo, md, mc);
      printf("    m '"); print_str((const unsigned char *)subj, slen);
      printf("' mo=0x%x rc=%d", mo, rc);
      if (rc > 0)
        {
        PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
        for (int i = 0; i < rc*2; i++) printf(" %ld", (long)ov[i]);
        printf(" sc=%lu", (unsigned long)pcre2_get_startchar(md));
        PCRE2_SPTR mk = pcre2_get_mark(md);
        if (mk) { printf(" mark="); print_str(mk, strlen((const char *)mk)); }
        }
      printf("\n");

      int rc2 = pcre2_dfa_match(re, (PCRE2_SPTR)subj, slen, 0, mo, md, mc, wspace, 300);
      printf("    d rc=%d", rc2);
      if (rc2 > 0)
        {
        PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
        for (int i = 0; i < rc2*2; i++) printf(" %ld", (long)ov[i]);
        }
      printf("\n");

      /* Substitution */
      {
        PCRE2_UCHAR out[512]; PCRE2_SIZE olen = sizeof(out);
        static const char *repls[] = { "[$0]", "<${1:-x}>", "\\U$0", "$*", "x" };
        static const uint32_t sopts[] = { 0, PCRE2_SUBSTITUTE_GLOBAL,
          PCRE2_SUBSTITUTE_EXTENDED, PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_EXTENDED,
          PCRE2_SUBSTITUTE_LITERAL, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
          PCRE2_SUBSTITUTE_UNSET_EMPTY|PCRE2_SUBSTITUTE_UNKNOWN_UNSET };
        const char *rp = repls[rnd(5)];
        uint32_t so = sopts[rnd(7)];
        int rc3 = pcre2_substitute(re, (PCRE2_SPTR)subj, slen, 0, so, NULL, NULL,
          (PCRE2_SPTR)rp, PCRE2_ZERO_TERMINATED, out, &olen);
        printf("    s repl='%s' so=0x%x rc=%d len=%lu out='", rp, so, rc3,
          (unsigned long)olen);
        if (rc3 >= 0) print_str(out, olen);
        printf("'\n");
      }
      }
    pcre2_match_context_free(mc);
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }
  printf("FUZZDONE\n");
  return 0;
}
