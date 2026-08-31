/* Differential tests for UTF validity handling: the UTF check itself, the
PCRE2_MATCH_INVALID_UTF fragment loop, and offsets that fall inside characters. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>

static const struct { const char *bytes; size_t len; } subs[] = {
  { "abc", 3 },
  { "\xc3\xa9", 2 },
  { "\xc3", 1 },                       /* truncated 2-byte */
  { "\xe2\x82", 2 },                   /* truncated 3-byte */
  { "\xe2\x82\xac", 3 },
  { "\xf0\x9f\x98\x80", 4 },
  { "\xf0\x9f\x98", 3 },               /* truncated 4-byte */
  { "\x80", 1 },                       /* isolated continuation */
  { "\xfe", 1 },                       /* illegal byte */
  { "\xff", 1 },
  { "\xc0\x80", 2 },                   /* overlong */
  { "\xe0\x80\x80", 3 },               /* overlong */
  { "\xed\xa0\x80", 3 },               /* surrogate */
  { "\xf4\x90\x80\x80", 4 },           /* > 0x10ffff */
  { "a\xffb", 3 },
  { "a\xc3\xa9\xff""b\xe2\x82\xac", 9 },
  { "\xf0\x9f\x98\x80\xff\xf0\x9f\x98\x80", 9 },
  { "a\x00\x62", 3 },
  { "", 0 },
};
#define NSUB (sizeof(subs)/sizeof(subs[0]))

static const char *pats[] = {
  "a", ".", ".+", "\\X", "\\C", "[^x]", "\\w+", "\\p{L}", "b",
  "\\x{e9}", "\\x{20ac}", "\\x{1f600}", "^.", ".$", "(?s).*", "\\R", "\\b",
  NULL
};

int main(void)
{
static const uint32_t copts[] = {
  PCRE2_UTF,
  PCRE2_UTF|PCRE2_UCP,
  PCRE2_UTF|PCRE2_MATCH_INVALID_UTF,
  PCRE2_UTF|PCRE2_UCP|PCRE2_MATCH_INVALID_UTF,
  0,
};
static const uint32_t mopts[] = { 0, PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT,
  PCRE2_NOTBOL, PCRE2_ANCHORED };

printf("== utf validity ==\n");
for (const char **p = pats; *p != NULL; p++)
  for (size_t ci = 0; ci < sizeof(copts)/sizeof(copts[0]); ci++)
    {
    int ec; PCRE2_SIZE eo;
    pcre2_code *re = pcre2_compile((PCRE2_SPTR)*p, PCRE2_ZERO_TERMINATED,
      copts[ci], &ec, &eo, NULL);
    if (re == NULL)
      { printf("U <%s> copt=%u FAIL %d off=%zu\n", *p, copts[ci], ec, eo); continue; }
    pcre2_match_data *md = pcre2_match_data_create_from_pattern(re, NULL);
    for (size_t si = 0; si < NSUB; si++)
      for (size_t mi = 0; mi < sizeof(mopts)/sizeof(mopts[0]); mi++)
        for (PCRE2_SIZE so = 0; so <= subs[si].len; so++)
          {
          int rc = pcre2_match(re, (PCRE2_SPTR)subs[si].bytes, subs[si].len,
            so, mopts[mi], md, NULL);
          printf("U <%s> copt=%u si=%zu mo=%u so=%zu rc=%d", *p, copts[ci],
            si, mopts[mi], so, rc);
          if (rc > 0 || rc == PCRE2_ERROR_PARTIAL)
            {
            PCRE2_SIZE *ov = pcre2_get_ovector_pointer(md);
            printf(" [%zd,%zd] sc=%zu", (ssize_t)ov[0], (ssize_t)ov[1],
              pcre2_get_startchar(md));
            }
          printf("\n");

          int wsp[64];
          pcre2_match_data *dmd = pcre2_match_data_create(4, NULL);
          int rc2 = pcre2_dfa_match(re, (PCRE2_SPTR)subs[si].bytes,
            subs[si].len, so, mopts[mi], dmd, NULL, wsp, 64);
          printf("V rc=%d", rc2);
          if (rc2 > 0 || rc2 == PCRE2_ERROR_PARTIAL)
            {
            PCRE2_SIZE *ov = pcre2_get_ovector_pointer(dmd);
            printf(" [%zd,%zd]", (ssize_t)ov[0], (ssize_t)ov[1]);
            }
          printf("\n");
          pcre2_match_data_free(dmd);

          /* Substitution over the same input */
          PCRE2_UCHAR out[256]; PCRE2_SIZE ol = sizeof(out);
          int rc3 = pcre2_substitute(re, (PCRE2_SPTR)subs[si].bytes,
            subs[si].len, so, PCRE2_SUBSTITUTE_GLOBAL|mopts[mi], NULL, NULL,
            (PCRE2_SPTR)"<$0>", 4, out, &ol);
          printf("W rc=%d len=%zu ", rc3, ol);
          if (rc3 >= 0)
            {
            for (PCRE2_SIZE k = 0; k < ol; k++) printf("%02x", out[k]);
            }
          printf("\n");
          }
    pcre2_match_data_free(md);
    pcre2_code_free(re);
    }

/* The UTF checker itself, via compiling invalid UTF patterns */
printf("== utf pattern check ==\n");
for (size_t si = 0; si < NSUB; si++)
  {
  int ec; PCRE2_SIZE eo;
  pcre2_code *re = pcre2_compile((PCRE2_SPTR)subs[si].bytes, subs[si].len,
    PCRE2_UTF, &ec, &eo, NULL);
  printf("P si=%zu ", si);
  if (re == NULL)
    {
    PCRE2_UCHAR eb[128];
    pcre2_get_error_message(ec, eb, sizeof(eb));
    printf("FAIL %d off=%zu <%s>\n", ec, eo, (char *)eb);
    }
  else { printf("OK\n"); pcre2_code_free(re); }
  }
printf("== utf done ==\n");
return 0;
}
