/* Randomised differential test harness: generates pseudo-random patterns and
   subjects, and compares the C and Rust PCRE2 libraries in detail.

   Build: gcc -O1 tools/fuzzharness.c -o fuzzharness -ldl
   Run:   ./fuzzharness <c.so> <rust.so> [iterations] [seed]
*/
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdarg.h>

typedef const unsigned char *PCRE2_SPTR;
typedef size_t PCRE2_SIZE;
#define PCRE2_ZERO_TERMINATED (~(PCRE2_SIZE)0)

#define CODE_HEADER_SIZE 152

/* callout block layout (must match pcre2.h for the 8-bit library) */
typedef struct pcre2_callout_block {
  uint32_t version;
  uint32_t callout_number;
  uint32_t capture_top;
  uint32_t capture_last;
  PCRE2_SIZE *offset_vector;
  PCRE2_SPTR mark;
  PCRE2_SPTR subject;
  PCRE2_SIZE subject_length;
  PCRE2_SIZE start_match;
  PCRE2_SIZE current_position;
  PCRE2_SIZE pattern_position;
  PCRE2_SIZE next_item_length;
  PCRE2_SIZE callout_string_offset;
  PCRE2_SIZE callout_string_length;
  PCRE2_SPTR callout_string;
  uint32_t callout_flags;
} pcre2_callout_block;

typedef struct pcre2_substitute_callout_block {
  uint32_t version;
  PCRE2_SPTR input;
  PCRE2_SPTR output;
  PCRE2_SIZE output_offsets[2];
  PCRE2_SIZE *ovector;
  uint32_t oveccount;
  uint32_t subscount;
} pcre2_substitute_callout_block;

struct lib {
  void *h;
  void *(*compile)(PCRE2_SPTR, PCRE2_SIZE, uint32_t, int *, PCRE2_SIZE *, void *);
  void (*code_free)(void *);
  void *(*code_copy)(void *);
  void *(*code_copy_with_tables)(void *);
  void *(*md_create)(uint32_t, void *);
  void (*md_free)(void *);
  int (*match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *);
  int (*dfa_match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *, int *, PCRE2_SIZE);
  PCRE2_SIZE *(*get_ovector_pointer)(void *);
  PCRE2_SPTR (*get_mark)(void *);
  PCRE2_SIZE (*get_startchar)(void *);
  int (*pattern_info)(void *, uint32_t, void *);
  int (*substitute)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *,
                    PCRE2_SPTR, PCRE2_SIZE, unsigned char *, PCRE2_SIZE *);
  void *(*mcontext_create)(void *);
  void (*mcontext_free)(void *);
  int (*set_match_limit)(void *, uint32_t);
  int (*set_depth_limit)(void *, uint32_t);
  int (*set_heap_limit)(void *, uint32_t);
  int (*set_offset_limit)(void *, PCRE2_SIZE);
  int (*set_callout)(void *, int (*)(pcre2_callout_block *, void *), void *);
  int (*set_substitute_callout)(void *, int (*)(pcre2_substitute_callout_block *, void *), void *);
  void *(*ccontext_create)(void *);
  void (*ccontext_free)(void *);
  int (*set_newline)(void *, uint32_t);
  int (*set_bsr)(void *, uint32_t);
  int (*set_optimize)(void *, uint32_t);
  int (*set_max_varlookbehind)(void *, uint32_t);
  int (*set_compile_extra_options)(void *, uint32_t);
  int (*set_character_tables)(void *, const unsigned char *);
  const unsigned char *(*maketables)(void *);
  void (*maketables_free)(void *, const unsigned char *);
  int (*callout_enumerate)(void *, int (*)(void *, void *), void *);
  int (*next_match)(void *, PCRE2_SIZE *, uint32_t *);
  PCRE2_SIZE (*get_match_data_heapframes_size)(void *);
  int (*pattern_convert)(PCRE2_SPTR, PCRE2_SIZE, uint32_t, unsigned char **, PCRE2_SIZE *, void *);
  void (*converted_pattern_free)(unsigned char *);
};

static int failures = 0;
static long checks = 0;

static int tracing = 0;
#define TRACE(msg) do { if (tracing) fprintf(stderr, "   phase: %s\n", msg); } while (0)

#define GETSYM(L, field, name) \
  do { *(void **)&((L)->field) = dlsym((L)->h, name); \
       if ((L)->field == NULL) { fprintf(stderr, "missing %s\n", name); exit(2); } } while (0)

static void load(struct lib *L, const char *path)
{
L->h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
if (L->h == NULL) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); exit(2); }
GETSYM(L, compile, "pcre2_compile_8");
GETSYM(L, code_free, "pcre2_code_free_8");
GETSYM(L, code_copy, "pcre2_code_copy_8");
GETSYM(L, code_copy_with_tables, "pcre2_code_copy_with_tables_8");
GETSYM(L, md_create, "pcre2_match_data_create_8");
GETSYM(L, md_free, "pcre2_match_data_free_8");
GETSYM(L, match, "pcre2_match_8");
GETSYM(L, dfa_match, "pcre2_dfa_match_8");
GETSYM(L, get_ovector_pointer, "pcre2_get_ovector_pointer_8");
GETSYM(L, get_mark, "pcre2_get_mark_8");
GETSYM(L, get_startchar, "pcre2_get_startchar_8");
GETSYM(L, pattern_info, "pcre2_pattern_info_8");
GETSYM(L, substitute, "pcre2_substitute_8");
GETSYM(L, mcontext_create, "pcre2_match_context_create_8");
GETSYM(L, mcontext_free, "pcre2_match_context_free_8");
GETSYM(L, set_match_limit, "pcre2_set_match_limit_8");
GETSYM(L, set_depth_limit, "pcre2_set_depth_limit_8");
GETSYM(L, set_heap_limit, "pcre2_set_heap_limit_8");
GETSYM(L, set_offset_limit, "pcre2_set_offset_limit_8");
GETSYM(L, set_callout, "pcre2_set_callout_8");
GETSYM(L, set_substitute_callout, "pcre2_set_substitute_callout_8");
GETSYM(L, ccontext_create, "pcre2_compile_context_create_8");
GETSYM(L, ccontext_free, "pcre2_compile_context_free_8");
GETSYM(L, set_newline, "pcre2_set_newline_8");
GETSYM(L, set_bsr, "pcre2_set_bsr_8");
GETSYM(L, set_optimize, "pcre2_set_optimize_8");
GETSYM(L, set_max_varlookbehind, "pcre2_set_max_varlookbehind_8");
GETSYM(L, set_compile_extra_options, "pcre2_set_compile_extra_options_8");
GETSYM(L, set_character_tables, "pcre2_set_character_tables_8");
GETSYM(L, maketables, "pcre2_maketables_8");
GETSYM(L, maketables_free, "pcre2_maketables_free_8");
GETSYM(L, callout_enumerate, "pcre2_callout_enumerate_8");
GETSYM(L, next_match, "pcre2_next_match_8");
GETSYM(L, get_match_data_heapframes_size, "pcre2_get_match_data_heapframes_size_8");
GETSYM(L, pattern_convert, "pcre2_pattern_convert_8");
GETSYM(L, converted_pattern_free, "pcre2_converted_pattern_free_8");
}

static struct lib C, R;
/* Compare two compiled code blocks, skipping the 0-3 uninitialised alignment
   padding bytes that PCRE2 leaves between the name table and the character
   lists (see the re_blocksize computation in pcre2_compile.c). */
static int code_blocks_differ(const unsigned char *a, const unsigned char *b,
                              size_t size, size_t nts, size_t code_start,
                              size_t *where)
{
size_t i, gap_start = 152 + nts, gap_end = gap_start;
if (code_start > 152 + nts)                 /* character lists are present */
  gap_end = 152 + ((nts + 3) & ~(size_t)3);
for (i = 152; i < size; i++)
  {
  if (i >= gap_start && i < gap_end) continue;
  if (a[i] != b[i]) { *where = i; return 1; }
  }
return 0;
}


/* ------------------------------------------------------- callout recording */

#define CALLOUT_LOG_SIZE 65536
static char clogC[CALLOUT_LOG_SIZE], clogR[CALLOUT_LOG_SIZE];
static size_t clogClen, clogRlen;
static int which_lib;   /* 0 = C, 1 = Rust */
static int in_dfa;      /* the DFA callout block has no meaningful ovector */

static void logf_(const char *fmt, ...)
{
va_list ap;
char buf[4096];
int n;
va_start(ap, fmt);
n = vsnprintf(buf, sizeof(buf), fmt, ap);
va_end(ap);
if (n < 0) return;
if ((size_t)n >= sizeof(buf)) n = (int)sizeof(buf) - 1;   /* vsnprintf returns the untruncated length */
if (which_lib == 0)
  { if (clogClen + (size_t)n < CALLOUT_LOG_SIZE) { memcpy(clogC + clogClen, buf, (size_t)n); clogClen += (size_t)n; } }
else
  { if (clogRlen + (size_t)n < CALLOUT_LOG_SIZE) { memcpy(clogR + clogRlen, buf, (size_t)n); clogRlen += (size_t)n; } }
}

static int callout_fn(pcre2_callout_block *cb, void *data)
{
uint32_t i;
(void)data;
logf_("CO ver=%u num=%u ctop=%u clast=%u sl=%lu sm=%lu cp=%lu pp=%lu nil=%lu cso=%lu csl=%lu fl=%u mark=%s ov=",
      cb->version, cb->callout_number, cb->capture_top, cb->capture_last,
      (unsigned long)cb->subject_length, (unsigned long)cb->start_match,
      (unsigned long)cb->current_position, (unsigned long)cb->pattern_position,
      (unsigned long)cb->next_item_length, (unsigned long)cb->callout_string_offset,
      (unsigned long)cb->callout_string_length, cb->callout_flags,
      cb->mark == NULL? "(null)" : (const char *)cb->mark);
if (!in_dfa)
  for (i = 0; i < 2 * cb->capture_top && i < 20; i++)
    logf_("%ld,", (long)cb->offset_vector[i]);
if (cb->callout_string != NULL)
  logf_(" cs=%.*s", (int)cb->callout_string_length, (const char *)cb->callout_string);
logf_("\n");
return 0;
}

static int subst_callout_fn(pcre2_substitute_callout_block *scb, void *data)
{
uint32_t i;
(void)data;
logf_("SCO ver=%u oo=%lu,%lu ovc=%u subs=%u out=%.*s ov=",
      scb->version, (unsigned long)scb->output_offsets[0],
      (unsigned long)scb->output_offsets[1], scb->oveccount, scb->subscount,
      (int)scb->output_offsets[1], (const char *)scb->output);
for (i = 0; i < 2 * scb->oveccount && i < 20; i++) logf_("%ld,", (long)scb->ovector[i]);
logf_("\n");
return 0;
}

/* --------------------------------------------------------------- generator */

static uint64_t rngstate = 88172645463325252ULL;
static uint32_t rnd(uint32_t n)
{
rngstate ^= rngstate << 13;
rngstate ^= rngstate >> 7;
rngstate ^= rngstate << 17;
return (uint32_t)(rngstate % n);
}

static const char *frag[] = {
  "a", "b", "c", "x", "y", "z", "0", "9", " ", "-", "_", "\\n", "\\t",
  ".", "^", "$", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S", "\\h", "\\v",
  "\\R", "\\X", "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G", "\\K", "\\C",
  "\\Q+*\\E", "\\x41", "\\x{1e9e}", "\\101", "\\o{101}", "\\cA", "\\e",
  "[abc]", "[^abc]", "[a-z]", "[[:alpha:]]", "[[:^digit:]]", "[\\d\\s]",
  "[\\x{100}-\\x{200}]", "[^\\W]", "[]a]", "[a\\-z]",
  "\\p{L}", "\\p{Lu}", "\\P{Nd}", "\\p{Greek}", "\\p{Han}", "\\p{Xan}",
  "\\p{Xsp}", "\\p{Xwd}", "\\p{ASCII}", "\\p{Alphabetic}", "\\p{scx:Latin}",
  "(a)", "(?:b)", "(?<n1>c)", "(?'n2'd)", "(?P<n3>e)", "(?>f)", "(?|(g)|(h))",
  "(?=a)", "(?!b)", "(?<=c)", "(?<!d)", "(*napla:x)", "(*naplb:y)",
  "(?(1)a|b)", "(?(<n1>)a|b)", "(?(R)x|y)", "(?(R1)x|y)", "(?(DEFINE)(?<dd>z))",
  "(?&dd)", "(?1)", "(?-1)", "(?+1)", "(?R)", "\\g{1}", "\\g<1>", "\\k<n1>",
  "\\1", "\\2", "(?C1)", "(?C{x})", "(*MARK:m)", "(*ACCEPT)", "(*FAIL)",
  "(*COMMIT)", "(*PRUNE)", "(*SKIP)", "(*THEN)", "(*PRUNE:p)", "(*SKIP:s)",
  "(*sr:\\w+)", "(*asr:\\w+)", "(*atomic:a)", "(*script_run:x)",
  "(?i)", "(?-i)", "(?s)", "(?m)", "(?x)", "(?J)", "(?U)", "(?n)", "(?i:q)",
  "(?[\\p{L} && \\p{Ll}])", "[a-z--[aeiou]]", "[\\p{L}&&[^x]]",
  "*", "+", "?", "*?", "+?", "??", "*+", "++", "?+",
  "{2}", "{2,}", "{2,4}", "{0,3}?", "{1,2}+", "|", "()", "(?#c)",
  "\\N{U+0041}", "\\N", "(?<=\\d{2,4})", "(?<n4>)", "(?J)(?<dn>a)|(?<dn>b)",
  "(*LIMIT_MATCH=1000)", "(*LIMIT_DEPTH=100)", "(*CRLF)", "(*ANY)", "(*ANYCRLF)",
  "(*BSR_UNICODE)", "(*BSR_ANYCRLF)", "(*NO_START_OPT)", "(*NO_AUTO_POSSESS)",
  "(*UTF)", "(*UCP)", "(*NOTEMPTY)", "(*NOTEMPTY_ATSTART)", "(*NO_DOTSTAR_ANCHOR)",
  "(*CASELESS_RESTRICT)", "\\u0041", "\\U", "\\Y",
};
#define NFRAG (sizeof(frag)/sizeof(frag[0]))

static const char *subjfrag[] = {
  "a", "b", "c", "x", "y", "z", "0", "9", " ", "\n", "\r\n", "\t", "-", "_",
  "abc", "aaa", "xyz", "AbC", "123", "\xc3\xa9", "\xe2\x98\xba", "\xf0\x9f\x98\x80",
  "\xce\xb1\xce\xb2", "\xe4\xb8\xad", "\xc4\xb0", "\xc3\x9f", "k", "K",
  "\x85", "\xc2\xa0", "\xe2\x80\xa8", "", "aaaaaaaaaa", "\x00", "\x7f",
};
#define NSUBJFRAG (sizeof(subjfrag)/sizeof(subjfrag[0]))

static void gen_pattern(char *buf, size_t bufsize)
{
size_t len = 0;
uint32_t n = 1 + rnd(7);
uint32_t i;
buf[0] = 0;
for (i = 0; i < n; i++)
  {
  const char *f = frag[rnd(NFRAG)];
  size_t fl = strlen(f);
  if (len + fl + 1 >= bufsize) break;
  memcpy(buf + len, f, fl);
  len += fl;
  }
buf[len] = 0;
}

static size_t gen_subject(char *buf, size_t bufsize)
{
size_t len = 0;
uint32_t n = rnd(6);
uint32_t i;
for (i = 0; i < n; i++)
  {
  const char *f = subjfrag[rnd(NSUBJFRAG)];
  size_t fl = strlen(f);
  if (fl == 0) fl = 1;   /* the "\x00" entry */
  if (len + fl + 1 >= bufsize) break;
  memcpy(buf + len, f, fl);
  len += fl;
  }
buf[len] = 0;
return len;
}

static const uint32_t compile_opts[] = {
  0, 0x00000008u /*CASELESS*/, 0x00000020u /*DOTALL*/, 0x00000400u /*MULTILINE*/,
  0x00080000u /*UTF*/, 0x00020000u /*UCP*/, 0x00080000u|0x00020000u,
  0x00000080u /*EXTENDED*/, 0x00002000u /*NO_AUTO_CAPTURE*/, 0x00000040u /*DUPNAMES*/,
  0x00040000u /*UNGREEDY*/, 0x00000004u /*AUTO_CALLOUT*/, 0x08000000u /*ALT_EXTENDED_CLASS*/,
  0x00000002u /*ALT_BSUX*/, 0x02000000u /*LITERAL*/, 0x00010000u /*NO_START_OPTIMIZE*/,
  0x00004000u /*NO_AUTO_POSSESS*/, 0x00000100u /*FIRSTLINE*/, 0x00000010u /*DOLLAR_ENDONLY*/,
  0x00200000u /*ALT_CIRCUMFLEX*/, 0x01000000u /*EXTENDED_MORE*/, 0x04000000u /*MATCH_INVALID_UTF*/,
  0x00000200u /*MATCH_UNSET_BACKREF*/, 0x00008000u /*NO_DOTSTAR_ANCHOR*/,
};
#define NCOPTS (sizeof(compile_opts)/sizeof(compile_opts[0]))

static const uint32_t extra_opts[] = {
  0, 0x00000001u, 0x00000002u, 0x00000010u, 0x00000020u, 0x00000080u,
  0x00000100u, 0x00000200u, 0x00000400u, 0x00000800u, 0x00001000u,
  0x00002000u, 0x00004000u, 0x00010000u,
};
#define NEOPTS (sizeof(extra_opts)/sizeof(extra_opts[0]))

static const uint32_t match_opts[] = {
  0, 0x00000001u, 0x00000002u, 0x00000004u, 0x00000008u, 0x00000010u,
  0x00000020u, 0x80000000u, 0x20000000u, 0x00040000u, 0x00004000u,
};
#define NMOPTS (sizeof(match_opts)/sizeof(match_opts[0]))

static void cmp_log(const char *pat, const char *subj, const char *what)
{
checks++;
if (clogClen != clogRlen || memcmp(clogC, clogR, clogClen) != 0)
  {
  size_t i;
  failures++;
  printf("MISMATCH %s log pat=<%s> subj=<%s>\n", what, pat, subj);
  for (i = 0; i < clogClen && i < clogRlen; i++) if (clogC[i] != clogR[i]) break;
  printf("   C   : %.200s\n", clogC + (i > 100? i - 100 : 0));
  printf("   RUST: %.200s\n", clogR + (i > 100? i - 100 : 0));
  }
}

int main(int argc, char **argv)
{
long iters = 20000;
long it;
if (argc < 3) { fprintf(stderr, "usage: %s <c.so> <rust.so> [iters] [seed]\n", argv[0]); return 2; }
load(&C, argv[1]);
load(&R, argv[2]);
if (argc > 3) iters = atol(argv[3]);
tracing = getenv("FUZZ_TRACE") != NULL;
if (argc > 4) rngstate = (uint64_t)atol(argv[4]) * 2654435761u + 88172645463325252ULL;

for (it = 0; it < iters; it++)
  {
  char pat[512], subj[512];
  uint32_t copt = compile_opts[rnd(NCOPTS)];
  uint32_t eopt = extra_opts[rnd(NEOPTS)];
  uint32_t nl = rnd(7);         /* 0 = don't set */
  uint32_t bsr = rnd(3);
  uint32_t optimize = rnd(4);
  int err1 = 0, err2 = 0;
  PCRE2_SIZE eo1 = 0, eo2 = 0;
  void *cc1, *cc2, *c1, *c2;
  size_t slen;

  gen_pattern(pat, sizeof(pat));
  slen = gen_subject(subj, sizeof(subj));
  if (tracing) fprintf(stderr, "it=%ld pat=<%s> subj=<%s> copt=%08x eopt=%08x nl=%u bsr=%u opt=%u\n", it, pat, subj, copt, eopt, nl, bsr, optimize);

  cc1 = C.ccontext_create(NULL);
  cc2 = R.ccontext_create(NULL);
  if (nl > 0) { C.set_newline(cc1, nl); R.set_newline(cc2, nl); }
  if (bsr > 0) { C.set_bsr(cc1, bsr); R.set_bsr(cc2, bsr); }
  if (optimize == 1) { C.set_optimize(cc1, 0); R.set_optimize(cc2, 0); }
  else if (optimize == 2) { C.set_optimize(cc1, 65); R.set_optimize(cc2, 65); }
  else if (optimize == 3) { C.set_optimize(cc1, 69); R.set_optimize(cc2, 69); }
  if (eopt != 0) { C.set_compile_extra_options(cc1, eopt); R.set_compile_extra_options(cc2, eopt); }
  if (rnd(8) == 0) { uint32_t v = rnd(300); C.set_max_varlookbehind(cc1, v); R.set_max_varlookbehind(cc2, v); }

  TRACE("compile");
  c1 = C.compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, copt, &err1, &eo1, cc1);
  c2 = R.compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, copt, &err2, &eo2, cc2);
  checks++;

  if ((c1 == NULL) != (c2 == NULL))
    {
    printf("MISMATCH compile null pat=<%s> opts=%08x eopt=%08x: C=%p RUST=%p (errC=%d errR=%d)\n",
           pat, copt, eopt, c1, c2, err1, err2);
    failures++;
    }
  else if (c1 == NULL)
    {
    if (err1 != err2 || eo1 != eo2)
      {
      printf("MISMATCH compile error pat=<%s> opts=%08x eopt=%08x: C=(%d,%lu) RUST=(%d,%lu)\n",
             pat, copt, eopt, err1, (unsigned long)eo1, err2, (unsigned long)eo2);
      failures++;
      }
    }
  else
    {
    /* compare the compiled code byte for byte */
    PCRE2_SIZE s1 = 0, s2 = 0;
    uint32_t iv;
    C.pattern_info(c1, 22 /*SIZE*/, &s1);
    R.pattern_info(c2, 22 /*SIZE*/, &s2);
    checks++;
    if (s1 != s2)
      { printf("MISMATCH code size pat=<%s> opts=%08x: C=%lu RUST=%lu\n", pat, copt,
               (unsigned long)s1, (unsigned long)s2); failures++; }
    else
      {
      uint32_t nc = 0, nes = 0;
      size_t where = 0, cs1 = *(size_t *)((char *)c1 + 80);
      C.pattern_info(c1, 17 /*NAMECOUNT*/, &nc);
      C.pattern_info(c1, 18 /*NAMEENTRYSIZE*/, &nes);
      if (code_blocks_differ((const unsigned char *)c1, (const unsigned char *)c2,
                             s1, (size_t)nc * nes, cs1, &where))
        {
        printf("MISMATCH code bytes pat=<%s> opts=%08x eopt=%08x size=%lu first diff at %lu: C=%02x RUST=%02x\n",
               pat, copt, eopt, (unsigned long)s1, (unsigned long)where,
               ((unsigned char *)c1)[where], ((unsigned char *)c2)[where]);
        failures++;
        }
      }
    for (iv = 0; iv <= 26; iv++)
      {
      char b1[64], b2[64];
      int r1, r2;
      if (iv == 7 || iv == 19) continue;   /* pointer results */
      memset(b1, 0xbb, sizeof(b1)); memset(b2, 0xbb, sizeof(b2));
      r1 = C.pattern_info(c1, iv, b1);
      r2 = R.pattern_info(c2, iv, b2);
      checks++;
      if (r1 != r2 || memcmp(b1, b2, sizeof(b1)) != 0)
        { printf("MISMATCH info %u pat=<%s> opts=%08x: rc C=%d R=%d\n", iv, pat, copt, r1, r2);
          failures++; }
      }

      {
      /* matching, with a random context and options */
      void *mc1 = C.mcontext_create(NULL), *mc2 = R.mcontext_create(NULL);
      uint32_t mopt = match_opts[rnd(NMOPTS)] | (rnd(4) == 0? match_opts[rnd(NMOPTS)] : 0);
      PCRE2_SIZE start = (slen == 0)? 0 : (PCRE2_SIZE)rnd((uint32_t)slen + 1);
      int use_callout = (rnd(3) == 0);
      void *md1, *md2;
      int r1, r2;
      uint32_t oveccount = 1 + rnd(8);

      if (rnd(4) == 0) { uint32_t v = 1 + rnd(2000); C.set_match_limit(mc1, v); R.set_match_limit(mc2, v); if (tracing) fprintf(stderr, "   match_limit=%u\n", v); }
      if (rnd(6) == 0) { uint32_t v = 1 + rnd(200); C.set_depth_limit(mc1, v); R.set_depth_limit(mc2, v); if (tracing) fprintf(stderr, "   depth_limit=%u\n", v); }
      { uint32_t hv = (rnd(8) == 0)? rnd(100) : 8192; C.set_heap_limit(mc1, hv); R.set_heap_limit(mc2, hv); }
      if (0) { uint32_t v = rnd(100); C.set_heap_limit(mc1, v); R.set_heap_limit(mc2, v); if (tracing) fprintf(stderr, "   heap_limit=%u\n", v); }
      if (rnd(8) == 0) { uint32_t v = rnd(20); C.set_offset_limit(mc1, (PCRE2_SIZE)v); R.set_offset_limit(mc2, (PCRE2_SIZE)v); if (tracing) fprintf(stderr, "   offset_limit=%u\n", v); }
      md1 = C.md_create(oveccount, NULL);
      md2 = R.md_create(oveccount, NULL);

      if (use_callout)
        {
        C.set_callout(mc1, callout_fn, NULL);
        R.set_callout(mc2, callout_fn, NULL);
        }
      clogClen = clogRlen = 0;
      which_lib = 0;
      if (tracing) fprintf(stderr, "   match args: mopt=%08x start=%lu ovec=%u callout=%d\n", mopt, (unsigned long)start, oveccount, use_callout);
  TRACE("match C");
      r1 = C.match(c1, (PCRE2_SPTR)subj, slen, start, mopt, md1, mc1);
      which_lib = 1;
  TRACE("match R");
      r2 = R.match(c2, (PCRE2_SPTR)subj, slen, start, mopt, md2, mc2);
      checks++;
      if (r1 != r2)
        { printf("MISMATCH match rc pat=<%s> copt=%08x mopt=%08x start=%lu subj=<%s>: C=%d RUST=%d\n",
                 pat, copt, mopt, (unsigned long)start, subj, r1, r2); failures++; }
      else
        {
        PCRE2_SIZE *o1;
        PCRE2_SIZE *o2;
        uint32_t k, n;
        TRACE("get ovector");
        o1 = C.get_ovector_pointer(md1);
        o2 = R.get_ovector_pointer(md2);
        n = (r1 > 0)? (uint32_t)r1 : 1;
        if (n > oveccount) n = oveccount;
        /* PCRE2 leaves the ovector untouched (i.e. uninitialised) when there is
           no match, so only compare it for a match or a partial match. */
        if (r1 < 0 && r1 != -2) n = 0;
        for (k = 0; k < 2 * n; k++)
          if (o1[k] != o2[k])
            { printf("MISMATCH match ov[%u] pat=<%s> subj=<%s>: C=%ld RUST=%ld\n", k, pat, subj,
                     (long)o1[k], (long)o2[k]); failures++; break; }
          {
          PCRE2_SPTR m1, m2;
          TRACE("get mark");
          m1 = C.get_mark(md1); m2 = R.get_mark(md2);
          if (tracing) fprintf(stderr, "   marks: C=%p R=%p\n", (void*)m1, (void*)m2);
          /* On a failed match PCRE2 may leave a stale/garbage mark pointer; both
             libraries return the same value, so only dereference on success. */
          if (r1 >= 0 && ((m1 == NULL) != (m2 == NULL) ||
              (m1 != NULL && strcmp((const char *)m1, (const char *)m2) != 0)))
            { printf("MISMATCH match mark pat=<%s> subj=<%s>\n", pat, subj); failures++; }
          }
          {
          PCRE2_SIZE sc1, sc2;
          TRACE("get startchar");
          sc1 = C.get_startchar(md1); sc2 = R.get_startchar(md2);
          if (r1 >= 0 && sc1 != sc2)
            { printf("MISMATCH startchar pat=<%s> subj=<%s>: C=%lu R=%lu\n", pat, subj,
                     (unsigned long)sc1, (unsigned long)sc2); failures++; }
          }
        }
      TRACE("cmp callout log");
      if (use_callout) cmp_log(pat, subj, "callout");

      /* DFA match */
        {
        int ws1[200], ws2[200];
        void *dmd1, *dmd2;
        TRACE("dfa md_create");
        dmd1 = C.md_create(oveccount, NULL); dmd2 = R.md_create(oveccount, NULL);
        int d1, d2;
        clogClen = clogRlen = 0;
        in_dfa = 1;
        which_lib = 0;
  TRACE("dfa C");
        d1 = C.dfa_match(c1, (PCRE2_SPTR)subj, slen, start, mopt & ~0x00040000u, dmd1, mc1, ws1, 200);
        which_lib = 1;
  TRACE("dfa R");
        d2 = R.dfa_match(c2, (PCRE2_SPTR)subj, slen, start, mopt & ~0x00040000u, dmd2, mc2, ws2, 200);
        checks++;
        if (d1 != d2)
          { printf("MISMATCH dfa rc pat=<%s> copt=%08x mopt=%08x start=%lu subj=<%s>: C=%d RUST=%d\n",
                   pat, copt, mopt, (unsigned long)start, subj, d1, d2); failures++; }
        else
          {
          PCRE2_SIZE *o1 = C.get_ovector_pointer(dmd1);
          PCRE2_SIZE *o2 = R.get_ovector_pointer(dmd2);
          uint32_t k, n = (d1 > 0)? (uint32_t)d1 : 1;
          if (n > oveccount) n = oveccount;
          if (d1 < 0 && d1 != -2) n = 0;
          for (k = 0; k < 2 * n; k++)
            if (o1[k] != o2[k])
              { printf("MISMATCH dfa ov[%u] pat=<%s> subj=<%s>: C=%ld RUST=%ld\n", k, pat, subj,
                       (long)o1[k], (long)o2[k]); failures++; break; }
          }
        in_dfa = 0;
        if (use_callout) cmp_log(pat, subj, "dfa callout");
        C.md_free(dmd1); R.md_free(dmd2);
        }

      /* substitute */
        {
        static const char *reps[] = { "X", "[$0]", "${1}", "a$1b", "\\U$0\\E", "$*" };
        const char *rep = reps[rnd(6)];
        uint32_t sopt = 0;
        unsigned char b1[1024], b2[1024];
        PCRE2_SIZE l1, l2;
        int use_scallout = (rnd(4) == 0);
        int s1_, s2_;
        if (rnd(2)) sopt |= 0x00000100u;   /* GLOBAL */
        if (rnd(3) == 0) sopt |= 0x00000200u;   /* EXTENDED */
        if (rnd(4) == 0) sopt |= 0x00000400u;   /* UNSET_EMPTY */
        if (rnd(5) == 0) sopt |= 0x00001000u;   /* OVERFLOW_LENGTH */
        if (rnd(6) == 0) sopt |= 0x00020000u;   /* REPLACEMENT_ONLY */
        if (rnd(7) == 0) sopt |= 0x00008000u;   /* LITERAL */
        l1 = (rnd(4) == 0)? (PCRE2_SIZE)rnd(20) : sizeof(b1);
        l2 = l1;
        memset(b1, 0x77, sizeof(b1)); memset(b2, 0x77, sizeof(b2));
        if (use_scallout)
          {
          C.set_substitute_callout(mc1, subst_callout_fn, NULL);
          R.set_substitute_callout(mc2, subst_callout_fn, NULL);
          }
        clogClen = clogRlen = 0;
        which_lib = 0;
  TRACE("subst C");
        s1_ = C.substitute(c1, (PCRE2_SPTR)subj, slen, start, sopt, md1, mc1,
                           (PCRE2_SPTR)rep, PCRE2_ZERO_TERMINATED, b1, &l1);
        which_lib = 1;
  TRACE("subst R");
        s2_ = R.substitute(c2, (PCRE2_SPTR)subj, slen, start, sopt, md2, mc2,
                           (PCRE2_SPTR)rep, PCRE2_ZERO_TERMINATED, b2, &l2);
        checks++;
        if (s1_ != s2_ || l1 != l2 || memcmp(b1, b2, sizeof(b1)) != 0)
          {
          printf("MISMATCH subst pat=<%s> subj=<%s> rep=<%s> sopt=%08x: rc C=%d R=%d len C=%lu R=%lu\n",
                 pat, subj, rep, sopt, s1_, s2_, (unsigned long)l1, (unsigned long)l2);
          if (s1_ >= 0) printf("   C=<%s>\n   R=<%s>\n", (char *)b1, (char *)b2);
          failures++;
          }
        if (use_scallout) cmp_log(pat, subj, "subst callout");
        if (use_scallout)
          { C.set_substitute_callout(mc1, NULL, NULL); R.set_substitute_callout(mc2, NULL, NULL); }
        }

      C.md_free(md1); R.md_free(md2);
      C.mcontext_free(mc1); R.mcontext_free(mc2);
      }

    /* code_copy round trip */
    if (rnd(10) == 0)
      {
  TRACE("code_copy");
      void *cp1 = C.code_copy(c1), *cp2 = R.code_copy(c2);
      void *md1 = C.md_create(4, NULL), *md2 = R.md_create(4, NULL);
      int r1 = C.match(cp1, (PCRE2_SPTR)subj, slen, 0, 0, md1, NULL);
      int r2 = R.match(cp2, (PCRE2_SPTR)subj, slen, 0, 0, md2, NULL);
      checks++;
      if (r1 != r2)
        { printf("MISMATCH code_copy match pat=<%s>: C=%d RUST=%d\n", pat, r1, r2); failures++; }
      C.md_free(md1); R.md_free(md2);
      C.code_free(cp1); R.code_free(cp2);
      }

    C.code_free(c1);
    R.code_free(c2);
    }
  C.ccontext_free(cc1);
  R.ccontext_free(cc2);

  if ((it % 2000) == 0) { printf("... %ld iterations, %ld checks, %d mismatches\n", it, checks, failures); fflush(stdout); }
  }

printf("\n%ld checks, %d mismatches\n", checks, failures);
return failures == 0? 0 : 1;
}
