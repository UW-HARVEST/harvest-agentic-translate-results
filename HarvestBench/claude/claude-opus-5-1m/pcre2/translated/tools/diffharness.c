/* Differential test harness: loads the C libpcre2.so and the Rust libpcre2.so
   and compares their behaviour for a corpus of patterns and subjects.

   Build: gcc -O1 tools/diffharness.c -o diffharness -ldl
   Run:   ./diffharness <c.so> <rust.so>
*/
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef const unsigned char *PCRE2_SPTR;
typedef size_t PCRE2_SIZE;

#define PCRE2_ZERO_TERMINATED (~(PCRE2_SIZE)0)
#define PCRE2_UNSET           (~(PCRE2_SIZE)0)

/* Offsets inside pcre2_real_code (verified against the C build) */
#define CODE_HEADER_SIZE 152
#define OFF_BLOCKSIZE     72
#define OFF_CODE_START    80

#define PCRE2_INFO_ALLOPTIONS  0
#define PCRE2_INFO_ARGOPTIONS  1
#define PCRE2_INFO_BACKREFMAX  2
#define PCRE2_INFO_BSR         3
#define PCRE2_INFO_CAPTURECOUNT 4
#define PCRE2_INFO_FIRSTCODEUNIT 5
#define PCRE2_INFO_FIRSTCODETYPE 6
#define PCRE2_INFO_FIRSTBITMAP 7
#define PCRE2_INFO_HASCRORLF   8
#define PCRE2_INFO_JCHANGED    9
#define PCRE2_INFO_LASTCODEUNIT 11
#define PCRE2_INFO_LASTCODETYPE 12
#define PCRE2_INFO_MATCHEMPTY  13
#define PCRE2_INFO_MATCHLIMIT  14
#define PCRE2_INFO_MAXLOOKBEHIND 15
#define PCRE2_INFO_MINLENGTH   16
#define PCRE2_INFO_NAMECOUNT   17
#define PCRE2_INFO_NAMEENTRYSIZE 18
#define PCRE2_INFO_NEWLINE     20
#define PCRE2_INFO_DEPTHLIMIT  21
#define PCRE2_INFO_SIZE        22
#define PCRE2_INFO_HASBACKSLASHC 23
#define PCRE2_INFO_FRAMESIZE   24
#define PCRE2_INFO_HEAPLIMIT   25
#define PCRE2_INFO_EXTRAOPTIONS 26

struct lib {
  void *h;
  void *(*compile)(PCRE2_SPTR, PCRE2_SIZE, uint32_t, int *, PCRE2_SIZE *, void *);
  void (*code_free)(void *);
  void *(*md_create)(uint32_t, void *);
  void *(*md_create_from_pattern)(void *, void *);
  void (*md_free)(void *);
  int (*match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *);
  int (*dfa_match)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *, int *, PCRE2_SIZE);
  PCRE2_SIZE *(*get_ovector_pointer)(void *);
  uint32_t (*get_ovector_count)(void *);
  PCRE2_SPTR (*get_mark)(void *);
  PCRE2_SIZE (*get_startchar)(void *);
  int (*pattern_info)(void *, uint32_t, void *);
  int (*get_error_message)(int, unsigned char *, PCRE2_SIZE);
  int (*substitute)(void *, PCRE2_SPTR, PCRE2_SIZE, PCRE2_SIZE, uint32_t, void *, void *,
                    PCRE2_SPTR, PCRE2_SIZE, unsigned char *, PCRE2_SIZE *);
  int (*pattern_convert)(PCRE2_SPTR, PCRE2_SIZE, uint32_t, unsigned char **, PCRE2_SIZE *, void *);
  void (*converted_pattern_free)(unsigned char *);
  int (*config)(uint32_t, void *);
  const unsigned char *(*maketables)(void *);
  void (*maketables_free)(void *, const unsigned char *);
  void *(*ccontext_create)(void *);
  void (*ccontext_free)(void *);
  int (*set_character_tables)(void *, const unsigned char *);
  int32_t (*serialize_encode)(void **, int32_t, unsigned char **, PCRE2_SIZE *, void *);
  int32_t (*serialize_decode)(void **, int32_t, const unsigned char *, void *);
  void (*serialize_free)(unsigned char *);
  int (*substring_number_from_name)(void *, PCRE2_SPTR);
  int (*substring_get_bynumber)(void *, uint32_t, unsigned char **, PCRE2_SIZE *);
  void (*substring_free)(unsigned char *);
  int (*substring_list_get)(void *, unsigned char ***, PCRE2_SIZE **);
  void (*substring_list_free)(unsigned char **);
  int (*next_match)(void *, PCRE2_SIZE *, uint32_t *);
  PCRE2_SIZE (*get_match_data_size)(void *);
  PCRE2_SIZE (*get_match_data_heapframes_size)(void *);
};

static int failures = 0;
static int checks = 0;
static int verbose = 0;

#define GETSYM(L, field, name) \
  do { *(void **)&((L)->field) = dlsym((L)->h, name); \
       if ((L)->field == NULL) { fprintf(stderr, "missing %s\n", name); exit(2); } } while (0)

static void load(struct lib *L, const char *path)
{
L->h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
if (L->h == NULL) { fprintf(stderr, "dlopen %s: %s\n", path, dlerror()); exit(2); }
GETSYM(L, compile, "pcre2_compile_8");
GETSYM(L, code_free, "pcre2_code_free_8");
GETSYM(L, md_create, "pcre2_match_data_create_8");
GETSYM(L, md_create_from_pattern, "pcre2_match_data_create_from_pattern_8");
GETSYM(L, md_free, "pcre2_match_data_free_8");
GETSYM(L, match, "pcre2_match_8");
GETSYM(L, dfa_match, "pcre2_dfa_match_8");
GETSYM(L, get_ovector_pointer, "pcre2_get_ovector_pointer_8");
GETSYM(L, get_ovector_count, "pcre2_get_ovector_count_8");
GETSYM(L, get_mark, "pcre2_get_mark_8");
GETSYM(L, get_startchar, "pcre2_get_startchar_8");
GETSYM(L, pattern_info, "pcre2_pattern_info_8");
GETSYM(L, get_error_message, "pcre2_get_error_message_8");
GETSYM(L, substitute, "pcre2_substitute_8");
GETSYM(L, pattern_convert, "pcre2_pattern_convert_8");
GETSYM(L, converted_pattern_free, "pcre2_converted_pattern_free_8");
GETSYM(L, config, "pcre2_config_8");
GETSYM(L, maketables, "pcre2_maketables_8");
GETSYM(L, maketables_free, "pcre2_maketables_free_8");
GETSYM(L, ccontext_create, "pcre2_compile_context_create_8");
GETSYM(L, ccontext_free, "pcre2_compile_context_free_8");
GETSYM(L, set_character_tables, "pcre2_set_character_tables_8");
GETSYM(L, serialize_encode, "pcre2_serialize_encode_8");
GETSYM(L, serialize_decode, "pcre2_serialize_decode_8");
GETSYM(L, serialize_free, "pcre2_serialize_free_8");
GETSYM(L, substring_number_from_name, "pcre2_substring_number_from_name_8");
GETSYM(L, substring_get_bynumber, "pcre2_substring_get_bynumber_8");
GETSYM(L, substring_free, "pcre2_substring_free_8");
GETSYM(L, substring_list_get, "pcre2_substring_list_get_8");
GETSYM(L, substring_list_free, "pcre2_substring_list_free_8");
GETSYM(L, next_match, "pcre2_next_match_8");
GETSYM(L, get_match_data_size, "pcre2_get_match_data_size_8");
GETSYM(L, get_match_data_heapframes_size, "pcre2_get_match_data_heapframes_size_8");
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


static void diff(const char *what, const char *pat, const char *subj, long long a, long long b)
{
failures++;
printf("MISMATCH %-22s pat=<%s> subj=<%s>: C=%lld RUST=%lld\n", what, pat,
       subj == NULL? "(none)" : subj, a, b);
}

/* ---------------------------------------------------------------- corpus */

static const struct { const char *pat; uint32_t opts; } patterns[] = {
  { "abc", 0 },
  { "a*b+c?", 0 },
  { "^abc$", 0 },
  { "(a)(b)(c)", 0 },
  { "(?<name>a)(?<other>b)", 0 },
  { "a|b|cd|ef", 0 },
  { "[a-z]+", 0 },
  { "[^a-z]+", 0 },
  { "[[:alpha:]]+", 0 },
  { "[[:^digit:]]", 0 },
  { "\\d+\\s*\\w+", 0 },
  { "\\D\\S\\W", 0 },
  { "a{2,5}", 0 },
  { "a{2,}", 0 },
  { "a{0,3}?", 0 },
  { "a{3}", 0 },
  { "(?i)AbC", 0 },
  { "abc", 0x00000008u /* CASELESS */ },
  { "^a.c$", 0x00000020u /* DOTALL */ },
  { "^a.c$", 0x00000400u /* MULTILINE */ },
  { "(?m)^b", 0 },
  { "(?s).+", 0 },
  { "(?x) a  b # comment\n c", 0 },
  { "(a+)+b", 0 },
  { "(?:abc){2,3}", 0 },
  { "(?>a+)b", 0 },
  { "a(?=b)", 0 },
  { "a(?!b)", 0 },
  { "(?<=a)b", 0 },
  { "(?<!a)b", 0 },
  { "(?<=ab|cde)f", 0 },
  { "\\bword\\b", 0 },
  { "\\Bword", 0 },
  { "\\Aabc\\z", 0 },
  { "abc\\Z", 0 },
  { "\\Gabc", 0 },
  { "a\\Kb", 0 },
  { "(a)\\1", 0 },
  { "(?<x>a)\\k<x>", 0 },
  { "(?:(a)|(b))\\2", 0 },
  { "(?(1)a|b)", 0 },
  { "(a)(?(1)b|c)", 0 },
  { "(?(?=a)ab|cd)", 0 },
  { "(?(DEFINE)(?<x>a))(?&x)b", 0 },
  { "a(?R)?b", 0 },
  { "(a|b(?1))", 0 },
  { "(?1)(abc)", 0 },
  { "x(*COMMIT)y", 0 },
  { "a(*PRUNE)b", 0 },
  { "a(*SKIP)b", 0 },
  { "a(*THEN)b|ac", 0 },
  { "a(*MARK:m1)b", 0 },
  { "(*FAIL)", 0 },
  { "a(*ACCEPT)b", 0 },
  { "(*UTF)\\x{100}", 0 },
  { "\\x{263a}", 0x00080000u /* UTF */ },
  { "[\\x{100}-\\x{200}]", 0x00080000u },
  { "\\p{L}+", 0x00080000u },
  { "\\p{Greek}", 0x00080000u },
  { "\\P{Nd}", 0x00080000u },
  { "\\X", 0x00080000u },
  { "\\R", 0 },
  { "\\H\\h\\V\\v", 0 },
  { "\\C", 0 },
  { "(?i)\\x{1e9e}", 0x00080000u },
  { "[\\p{Han}\\p{Hiragana}]", 0x00080000u },
  { "(*sr:\\w+)", 0x00080000u },
  { "(?[\\p{L} && \\p{Ll}])", 0x08000000u /* ALT_EXTENDED_CLASS */ },
  { "[a-z--[aeiou]]", 0x08000000u },
  { "a++b", 0 },
  { "a*+b", 0 },
  { "[abc]*+d", 0 },
  { "(a)*+b", 0 },
  { "\\Qa+b\\Ec", 0 },
  { "(?# comment)abc", 0 },
  { "(?i:a)b", 0 },
  { "(?-i:a)b", 0x00000008u },
  { "(?J)(?<n>a)|(?<n>b)", 0 },
  { "(?|(a)|(b))", 0 },
  { "\\N{U+0041}", 0x00080000u },
  { "[\\d\\s]", 0 },
  { "[^\\D]", 0 },
  { "(?C1)abc", 0 },
  { "a(?C{cb})b", 0 },
  { "\\o{101}", 0 },
  { "\\101", 0 },
  { "\\x41", 0 },
  { "\\cA", 0 },
  { "\\e\\a\\f\\n\\r\\t", 0 },
  { "[\\b]", 0 },
  { "a\\z", 0x80000000u /* ANCHORED */ },
  { "abc", 0x20000000u /* ENDANCHORED */ },
  { "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)", 0 },
  { ".*", 0 },
  { "^", 0 },
  { "$", 0 },
  { "", 0 },
  { "(?<=\\d{3})x", 0 },
  { "(?<=a{2,4})b", 0 },
  { "(*LIMIT_MATCH=100)a+", 0 },
  { "(*CRLF)a$", 0 },
  { "(*ANY)a$", 0 },
  { "(*BSR_ANYCRLF)\\R", 0 },
  { "(*NO_START_OPT)abc", 0 },
  { "(*NOTEMPTY)a*", 0 },
  { "[[:word:]]+", 0 },
  { "\\w{2,}?", 0 },
  { "(?:a|ab)(?:c|bc)", 0 },
  { "(?=(a))\\1b", 0 },
  { "(?<!\\d)abc", 0 },
  { "(?*napla:a)b", 0 },
  { "a\\b", 0 },
  { "[]]", 0x00000001u /* ALLOW_EMPTY_CLASS */ },
  { "[^]]", 0x00000001u },
  { "\\p{Xan}", 0x00080000u },
  { "\\p{Xps}\\p{Xsp}\\p{Xwd}", 0x00080000u },
  { "(?i)[k]", 0x00080000u | 0x00020000u /* UTF|UCP */ },
  { "\\w+", 0x00020000u /* UCP */ },
  { "(?i)straße", 0x00080000u },
  /* patterns with errors */
  { "(", 0 },
  { ")", 0 },
  { "a{3,1}", 0 },
  { "[z-a]", 0 },
  { "\\", 0 },
  { "(?<>a)", 0 },
  { "(?P<a>x)(?P<a>y)", 0 },
  { "\\p{Nonsense}", 0 },
  { "a{100000}", 0 },
  { "(?(999)a)", 0 },
  { "[[:nonsense:]]", 0 },
  { "(?i", 0 },
  { "a**", 0 },
  { "\\x{110000}", 0x00080000u },
};
#define NPATTERNS (sizeof(patterns)/sizeof(patterns[0]))

static const char *subjects[] = {
  "",
  "a",
  "b",
  "ab",
  "abc",
  "aaa",
  "aaab",
  "xabcx",
  "abcabc",
  "ABC",
  "AbC",
  "a\nb",
  "a\r\nb",
  "a\rb",
  "line1\nline2",
  "  spaces  ",
  "123",
  "a1b2c3",
  "word boundary",
  "\xc3\xa9\xc3\xa8",           /* UTF-8 e-acute e-grave */
  "\xe2\x98\xba",               /* smiley */
  "\xf0\x9f\x98\x80",           /* emoji */
  "\xe1\xbe\xbf\xce\xb1",       /* greek */
  "\xc4\xb0i",                  /* dotted I */
  "abcdefghijklmnopqrstuvwxyz",
  "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
  "x",
  "\x00\x01\x02",
  "kK\xe2\x84\xaa",             /* k K kelvin */
  "stra\xc3\x9f" "e",
  "\xe4\xb8\xad\xe6\x96\x87",   /* han */
  "\xe3\x81\x82\xe3\x81\x84",   /* hiragana */
};
#define NSUBJECTS (sizeof(subjects)/sizeof(subjects[0]))

static const uint32_t match_options[] = {
  0,
  0x00000001u,  /* NOTBOL */
  0x00000002u,  /* NOTEOL */
  0x00000004u,  /* NOTEMPTY */
  0x00000008u,  /* NOTEMPTY_ATSTART */
  0x00000010u,  /* PARTIAL_SOFT */
  0x00000020u,  /* PARTIAL_HARD */
  0x80000000u,  /* ANCHORED */
  0x20000000u,  /* ENDANCHORED */
};
#define NMATCHOPTS (sizeof(match_options)/sizeof(match_options[0]))

static const uint32_t info_items[] = {
  PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_BACKREFMAX,
  PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT, PCRE2_INFO_FIRSTCODEUNIT,
  PCRE2_INFO_FIRSTCODETYPE, PCRE2_INFO_HASCRORLF, PCRE2_INFO_JCHANGED,
  PCRE2_INFO_LASTCODEUNIT, PCRE2_INFO_LASTCODETYPE, PCRE2_INFO_MATCHEMPTY,
  PCRE2_INFO_MATCHLIMIT, PCRE2_INFO_MAXLOOKBEHIND, PCRE2_INFO_MINLENGTH,
  PCRE2_INFO_NAMECOUNT, PCRE2_INFO_NAMEENTRYSIZE, PCRE2_INFO_NEWLINE,
  PCRE2_INFO_DEPTHLIMIT, PCRE2_INFO_HASBACKSLASHC, PCRE2_INFO_HEAPLIMIT,
  PCRE2_INFO_EXTRAOPTIONS,
};
#define NINFO (sizeof(info_items)/sizeof(info_items[0]))

/* ---------------------------------------------------------------- tests */

static void test_config(void)
{
uint32_t what;
for (what = 0; what <= 16; what++)
  {
  char bufc[128], bufr[128];
  int rc1, rc2;
  memset(bufc, 0xaa, sizeof(bufc));
  memset(bufr, 0xaa, sizeof(bufr));
  rc1 = C.config(what, bufc);
  rc2 = R.config(what, bufr);
  checks++;
  if (rc1 != rc2) diff("config rc", "-", NULL, rc1, rc2);
  else if (rc1 > 0 && memcmp(bufc, bufr, (size_t)rc1 > sizeof(bufc)? sizeof(bufc) : (size_t)rc1) != 0)
    diff("config data", "-", NULL, what, what);
  rc1 = C.config(what, NULL);
  rc2 = R.config(what, NULL);
  checks++;
  if (rc1 != rc2) diff("config len", "-", NULL, rc1, rc2);
  }
}

static void test_error_messages(void)
{
int e;
for (e = -80; e <= 230; e++)
  {
  unsigned char b1[256], b2[256];
  int r1, r2;
  memset(b1, 0x55, sizeof(b1));
  memset(b2, 0x55, sizeof(b2));
  r1 = C.get_error_message(e, b1, sizeof(b1));
  r2 = R.get_error_message(e, b2, sizeof(b2));
  checks++;
  if (r1 != r2) diff("errmsg rc", "-", NULL, r1, r2);
  else if (memcmp(b1, b2, sizeof(b1)) != 0) diff("errmsg text", "-", NULL, e, e);
  }
}

static void test_maketables(void)
{
const unsigned char *t1 = C.maketables(NULL);
const unsigned char *t2 = R.maketables(NULL);
checks++;
if (t1 == NULL || t2 == NULL) diff("maketables null", "-", NULL, (long long)(size_t)t1, (long long)(size_t)t2);
else if (memcmp(t1, t2, 1088) != 0) diff("maketables data", "-", NULL, 0, 0);
if (t1 != NULL) C.maketables_free(NULL, t1);
if (t2 != NULL) R.maketables_free(NULL, t2);
}

static void test_convert(void)
{
static const struct { const char *pat; uint32_t opt; } cases[] = {
  { "a*b", 0x00000010u },        /* GLOB */
  { "*.txt", 0x00000010u },
  { "**/x", 0x00000010u },
  { "a?b[c-e]", 0x00000010u },
  { "/usr/**", 0x00000010u },
  { "a**b", 0x00000030u },
  { "x/**/y", 0x00000050u },
  { "abc", 0x00000004u },        /* POSIX_BASIC */
  { "a\\(b\\)c", 0x00000004u },
  { "a+b", 0x00000008u },        /* POSIX_EXTENDED */
  { "[[:alpha:]]x", 0x00000008u },
  { "a{2,3}", 0x00000008u },
  { "^a$", 0x00000008u },
  { "[]a]", 0x00000008u },
  { "\\1", 0x00000004u },
};
size_t i;
for (i = 0; i < sizeof(cases)/sizeof(cases[0]); i++)
  {
  unsigned char *b1 = NULL, *b2 = NULL;
  PCRE2_SIZE l1 = 0, l2 = 0;
  int r1, r2;
  r1 = C.pattern_convert((PCRE2_SPTR)cases[i].pat, PCRE2_ZERO_TERMINATED, cases[i].opt, &b1, &l1, NULL);
  r2 = R.pattern_convert((PCRE2_SPTR)cases[i].pat, PCRE2_ZERO_TERMINATED, cases[i].opt, &b2, &l2, NULL);
  checks++;
  if (r1 != r2) diff("convert rc", cases[i].pat, NULL, r1, r2);
  else if (r1 == 0)
    {
    if (l1 != l2) diff("convert len", cases[i].pat, NULL, (long long)l1, (long long)l2);
    else if (memcmp(b1, b2, l1) != 0) diff("convert text", cases[i].pat, (const char *)b1, 0, 0);
    }
  if (b1 != NULL) C.converted_pattern_free(b1);
  if (b2 != NULL) R.converted_pattern_free(b2);
  }
}

static void test_substitute(void *code1, void *code2, const char *pat, const char *subj)
{
static const struct { const char *rep; uint32_t opt; } reps[] = {
  { "X", 0 },
  { "X", 0x00000100u },                       /* GLOBAL */
  { "[$0]", 0 },
  { "[${1}]", 0 },
  { "a$1b", 0x00000100u },
  { "$*", 0 },
  { "\\U$0\\E", 0x00000200u },                /* EXTENDED */
  { "${1:-empty}", 0x00000200u },
  { "$1", 0x00000400u|0x00000100u },          /* UNSET_EMPTY|GLOBAL */
  { "x", 0x00008000u },                       /* LITERAL */
  { "$0$0", 0x00020000u },                    /* REPLACEMENT_ONLY */
};
size_t i;
for (i = 0; i < sizeof(reps)/sizeof(reps[0]); i++)
  {
  unsigned char b1[512], b2[512];
  PCRE2_SIZE l1 = sizeof(b1), l2 = sizeof(b2);
  int r1, r2;
  memset(b1, 0x33, sizeof(b1));
  memset(b2, 0x33, sizeof(b2));
  r1 = C.substitute(code1, (PCRE2_SPTR)subj, PCRE2_ZERO_TERMINATED, 0, reps[i].opt,
                    NULL, NULL, (PCRE2_SPTR)reps[i].rep, PCRE2_ZERO_TERMINATED, b1, &l1);
  r2 = R.substitute(code2, (PCRE2_SPTR)subj, PCRE2_ZERO_TERMINATED, 0, reps[i].opt,
                    NULL, NULL, (PCRE2_SPTR)reps[i].rep, PCRE2_ZERO_TERMINATED, b2, &l2);
  checks++;
  if (r1 != r2) { diff("subst rc", pat, subj, r1, r2); continue; }
  if (l1 != l2) { diff("subst len", pat, subj, (long long)l1, (long long)l2); continue; }
  if (r1 >= 0 && memcmp(b1, b2, sizeof(b1)) != 0) diff("subst text", pat, subj, 0, 0);
  }
}

static void test_pattern(const char *pat, uint32_t opts)
{
int err1 = 0, err2 = 0;
PCRE2_SIZE eo1 = 0, eo2 = 0;
void *c1, *c2;
size_t si, oi, ii;

c1 = C.compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, opts, &err1, &eo1, NULL);
c2 = R.compile((PCRE2_SPTR)pat, PCRE2_ZERO_TERMINATED, opts, &err2, &eo2, NULL);
checks++;

if ((c1 == NULL) != (c2 == NULL))
  {
  diff("compile null", pat, NULL, (long long)(size_t)c1, (long long)(size_t)c2);
  if (c1 != NULL) C.code_free(c1);
  if (c2 != NULL) R.code_free(c2);
  return;
  }

if (c1 == NULL)
  {
  if (err1 != err2) diff("compile errcode", pat, NULL, err1, err2);
  if (eo1 != eo2) diff("compile erroffset", pat, NULL, (long long)eo1, (long long)eo2);
  return;
  }

/* Compare the pattern_info values */
for (ii = 0; ii < NINFO; ii++)
  {
  uint32_t v1 = 0xdeadbeef, v2 = 0xdeadbeef;
  int r1 = C.pattern_info(c1, info_items[ii], &v1);
  int r2 = R.pattern_info(c2, info_items[ii], &v2);
  checks++;
  if (r1 != r2) diff("info rc", pat, NULL, r1, r2);
  else if (v1 != v2)
    { printf("MISMATCH info[%u]             pat=<%s>: C=%u RUST=%u\n", info_items[ii], pat, v1, v2); failures++; }
  }
  {
  PCRE2_SIZE s1 = 0, s2 = 0, f1 = 0, f2 = 0;
  C.pattern_info(c1, PCRE2_INFO_SIZE, &s1);
  R.pattern_info(c2, PCRE2_INFO_SIZE, &s2);
  checks++;
  if (s1 != s2) diff("info size", pat, NULL, (long long)s1, (long long)s2);
  C.pattern_info(c1, PCRE2_INFO_FRAMESIZE, &f1);
  R.pattern_info(c2, PCRE2_INFO_FRAMESIZE, &f2);
  checks++;
  if (f1 != f2) diff("info framesize", pat, NULL, (long long)f1, (long long)f2);

  /* Compare the compiled byte code, byte for byte (skipping the uninitialised
  alignment padding between the name table and the character lists) */
  if (s1 == s2 && s1 > CODE_HEADER_SIZE)
    {
    uint32_t nc = 0, nes = 0;
    size_t where = 0, cs1 = *(size_t *)((char *)c1 + OFF_CODE_START);
    checks++;
    C.pattern_info(c1, PCRE2_INFO_NAMECOUNT, &nc);
    C.pattern_info(c1, PCRE2_INFO_NAMEENTRYSIZE, &nes);
    if (code_blocks_differ((const unsigned char *)c1, (const unsigned char *)c2,
                           s1, (size_t)nc * nes, cs1, &where))
      {
      printf("MISMATCH compiled bytes       pat=<%s> (size %lu) first difference at %lu: C=%02x RUST=%02x\n",
             pat, (unsigned long)s1, (unsigned long)where,
             ((unsigned char *)c1)[where], ((unsigned char *)c2)[where]);
      failures++;
      }
    }
  }
  {
  const unsigned char *bm1 = NULL, *bm2 = NULL;
  int r1 = C.pattern_info(c1, PCRE2_INFO_FIRSTBITMAP, &bm1);
  int r2 = R.pattern_info(c2, PCRE2_INFO_FIRSTBITMAP, &bm2);
  checks++;
  if (r1 != r2) diff("info bitmap rc", pat, NULL, r1, r2);
  else if ((bm1 == NULL) != (bm2 == NULL)) diff("info bitmap null", pat, NULL, (long long)(size_t)bm1, (long long)(size_t)bm2);
  else if (bm1 != NULL && memcmp(bm1, bm2, 32) != 0) diff("info bitmap data", pat, NULL, 0, 0);
  }

/* Matching */
for (si = 0; si < NSUBJECTS; si++)
  {
  const char *subj = subjects[si];
  size_t slen = strlen(subj);
  for (oi = 0; oi < NMATCHOPTS; oi++)
    {
    void *md1 = C.md_create(16, NULL);
    void *md2 = R.md_create(16, NULL);
    int r1 = C.match(c1, (PCRE2_SPTR)subj, slen, 0, match_options[oi], md1, NULL);
    int r2 = R.match(c2, (PCRE2_SPTR)subj, slen, 0, match_options[oi], md2, NULL);
    checks++;
    if (r1 != r2) diff("match rc", pat, subj, r1, r2);
    else
      {
      PCRE2_SIZE *o1 = C.get_ovector_pointer(md1);
      PCRE2_SIZE *o2 = R.get_ovector_pointer(md2);
      uint32_t n = r1 > 0? (uint32_t)r1 : 1;
      uint32_t k;
      if (r1 < 0 && r1 != -2) n = 0;   /* ovector is not set when there is no match */
      for (k = 0; k < 2 * n && k < 32; k++)
        if (o1[k] != o2[k])
          { printf("MISMATCH match ovector[%u]  pat=<%s> subj=<%s>: C=%ld RUST=%ld\n",
                   k, pat, subj, (long)o1[k], (long)o2[k]); failures++; break; }
      if (r1 >= 0)
        {
        PCRE2_SIZE sc1 = C.get_startchar(md1), sc2 = R.get_startchar(md2);
        if (sc1 != sc2) diff("match startchar", pat, subj, (long long)sc1, (long long)sc2);
        }
        {
        PCRE2_SPTR m1 = C.get_mark(md1), m2 = R.get_mark(md2);
        if ((m1 == NULL) != (m2 == NULL)) diff("match mark null", pat, subj, (long long)(size_t)m1, (long long)(size_t)m2);
        else if (r1 >= 0 && m1 != NULL && strcmp((const char *)m1, (const char *)m2) != 0)
          diff("match mark text", pat, subj, 0, 0);
        }
      }
    C.md_free(md1);
    R.md_free(md2);
    }

  /* DFA matching */
  for (oi = 0; oi < NMATCHOPTS; oi++)
    {
    void *md1 = C.md_create(16, NULL);
    void *md2 = R.md_create(16, NULL);
    int ws1[100], ws2[100];
    int r1 = C.dfa_match(c1, (PCRE2_SPTR)subj, slen, 0, match_options[oi], md1, NULL, ws1, 100);
    int r2 = R.dfa_match(c2, (PCRE2_SPTR)subj, slen, 0, match_options[oi], md2, NULL, ws2, 100);
    checks++;
    if (r1 != r2) diff("dfa rc", pat, subj, r1, r2);
    else
      {
      PCRE2_SIZE *o1 = C.get_ovector_pointer(md1);
      PCRE2_SIZE *o2 = R.get_ovector_pointer(md2);
      uint32_t n = r1 > 0? (uint32_t)r1 : 1;
      uint32_t k;
      if (r1 < 0 && r1 != -2) n = 0;   /* ovector is not set when there is no match */
      for (k = 0; k < 2 * n && k < 32; k++)
        if (o1[k] != o2[k])
          { printf("MISMATCH dfa ovector[%u]    pat=<%s> subj=<%s>: C=%ld RUST=%ld\n",
                   k, pat, subj, (long)o1[k], (long)o2[k]); failures++; break; }
      }
    C.md_free(md1);
    R.md_free(md2);
    }

  test_substitute(c1, c2, pat, subj);
  }

/* substring extraction on the first subject that matches */
for (si = 0; si < NSUBJECTS; si++)
  {
  void *md1 = C.md_create_from_pattern(c1, NULL);
  void *md2 = R.md_create_from_pattern(c2, NULL);
  int r1 = C.match(c1, (PCRE2_SPTR)subjects[si], strlen(subjects[si]), 0, 0, md1, NULL);
  int r2 = R.match(c2, (PCRE2_SPTR)subjects[si], strlen(subjects[si]), 0, 0, md2, NULL);
  if (r1 == r2 && r1 > 0)
    {
    uint32_t g;
    for (g = 0; g < (uint32_t)r1; g++)
      {
      unsigned char *s1 = NULL, *s2 = NULL;
      PCRE2_SIZE l1 = 0, l2 = 0;
      int e1 = C.substring_get_bynumber(md1, g, &s1, &l1);
      int e2 = R.substring_get_bynumber(md2, g, &s2, &l2);
      checks++;
      if (e1 != e2) diff("substring rc", pat, subjects[si], e1, e2);
      else if (e1 == 0)
        {
        if (l1 != l2) diff("substring len", pat, subjects[si], (long long)l1, (long long)l2);
        else if (memcmp(s1, s2, l1) != 0) diff("substring text", pat, subjects[si], 0, 0);
        }
      if (s1) C.substring_free(s1);
      if (s2) R.substring_free(s2);
      }
      {
      unsigned char **list1 = NULL, **list2 = NULL;
      PCRE2_SIZE *lens1 = NULL, *lens2 = NULL;
      int e1 = C.substring_list_get(md1, &list1, &lens1);
      int e2 = R.substring_list_get(md2, &list2, &lens2);
      checks++;
      if (e1 != e2) diff("substring_list rc", pat, subjects[si], e1, e2);
      else if (e1 == 0)
        {
        uint32_t g2;
        for (g2 = 0; g2 < (uint32_t)r1; g2++)
          {
          if (lens1[g2] != lens2[g2]) { diff("substring_list len", pat, subjects[si], (long long)lens1[g2], (long long)lens2[g2]); break; }
          if (memcmp(list1[g2], list2[g2], lens1[g2]) != 0) { diff("substring_list text", pat, subjects[si], 0, 0); break; }
          }
        }
      if (list1) C.substring_list_free(list1);
      if (list2) R.substring_list_free(list2);
      }
    /* next_match iteration */
      {
      PCRE2_SIZE off1 = 0, off2 = 0;
      uint32_t mo1 = 0, mo2 = 0;
      int e1 = C.next_match(md1, &off1, &mo1);
      int e2 = R.next_match(md2, &off2, &mo2);
      checks++;
      if (e1 != e2) diff("next_match rc", pat, subjects[si], e1, e2);
      else if (e1) { if (off1 != off2 || mo1 != mo2) diff("next_match values", pat, subjects[si], (long long)off1, (long long)off2); }
      }
      {
      PCRE2_SIZE z1 = C.get_match_data_size(md1), z2 = R.get_match_data_size(md2);
      PCRE2_SIZE h1 = C.get_match_data_heapframes_size(md1), h2 = R.get_match_data_heapframes_size(md2);
      checks++;
      if (z1 != z2) diff("md size", pat, subjects[si], (long long)z1, (long long)z2);
      if (h1 != h2) diff("md heapframes size", pat, subjects[si], (long long)h1, (long long)h2);
      }
    }
  C.md_free(md1);
  R.md_free(md2);
  }

/* serialization round trip */
  {
  unsigned char *bytes1 = NULL, *bytes2 = NULL;
  PCRE2_SIZE bl1 = 0, bl2 = 0;
  void *codes1[1], *codes2[1];
  int32_t s1, s2;
  codes1[0] = c1; codes2[0] = c2;
  s1 = C.serialize_encode(codes1, 1, &bytes1, &bl1, NULL);
  s2 = R.serialize_encode(codes2, 1, &bytes2, &bl2, NULL);
  checks++;
  if (s1 != s2) diff("serialize rc", pat, NULL, s1, s2);
  else if (s1 >= 0)
    {
    if (bl1 != bl2) diff("serialize len", pat, NULL, (long long)bl1, (long long)bl2);
    else
      {
      /* The serialized data contains the memctl pointers of the encoding
         library at the start, so compare only after that (32 bytes). */
      if (bl1 > 40 && memcmp(bytes1 + 40, bytes2 + 40, bl1 - 40) != 0)
        diff("serialize data", pat, NULL, 0, 0);
      }
    /* decode in the other library and match with the decoded code */
      {
      void *dec1[1] = { NULL }, *dec2[1] = { NULL };
      int32_t d1 = C.serialize_decode(dec1, 1, bytes1, NULL);
      int32_t d2 = R.serialize_decode(dec2, 1, bytes2, NULL);
      checks++;
      if (d1 != d2) diff("deserialize rc", pat, NULL, d1, d2);
      else if (d1 > 0)
        {
        void *md1 = C.md_create(16, NULL), *md2 = R.md_create(16, NULL);
        int r1 = C.match(dec1[0], (PCRE2_SPTR)"abcabc", 6, 0, 0, md1, NULL);
        int r2 = R.match(dec2[0], (PCRE2_SPTR)"abcabc", 6, 0, 0, md2, NULL);
        checks++;
        if (r1 != r2) diff("deserialized match", pat, "abcabc", r1, r2);
        C.md_free(md1); R.md_free(md2);
        }
      if (dec1[0]) C.code_free(dec1[0]);
      if (dec2[0]) R.code_free(dec2[0]);
      }
    }
  if (bytes1) C.serialize_free(bytes1);
  if (bytes2) R.serialize_free(bytes2);
  }

C.code_free(c1);
R.code_free(c2);
}

int main(int argc, char **argv)
{
size_t i;
if (argc < 3) { fprintf(stderr, "usage: %s <c.so> <rust.so> [-v]\n", argv[0]); return 2; }
if (argc > 3 && strcmp(argv[3], "-v") == 0) verbose = 1;
load(&C, argv[1]);
load(&R, argv[2]);

test_config();
test_error_messages();
test_maketables();
test_convert();

for (i = 0; i < NPATTERNS; i++)
  {
  if (verbose) printf("... pattern %2lu: <%s>\n", (unsigned long)i, patterns[i].pat);
  fflush(stdout);
  test_pattern(patterns[i].pat, patterns[i].opts);
  }

printf("\n%d checks, %d mismatches\n", checks, failures);
return failures == 0? 0 : 1;
}
