# ERRORS.md — Error / rejection surface table (Phase A)

Mechanically grepped from `c_src/src/lib.c`. There are **no** `assert`s, no error
enums, and no `RETURN_ERROR`-style macros in this library. Every rejection path
is one of:

* an early `return` guarding a null pointer (`lib.c:36-38`, `lib.c:64-65`),
* `return 0` after a failed `regcomp` (`lib.c:40-43`),
* `return !result` collapsing a non-zero `regexec` status to `0` (`lib.c:45-47`),
* a "not found" sentinel: `get_os_arch` returns the initial `NULL` (`lib.c:19,29`),
  `strstr` returning `NULL` skipping an entire block (`lib.c:68,98,102,109,135,142`),
* an unchecked out-of-bounds write, `*(p + strlen(p) - 1) = '\0'` with
  `strlen(p) == 0` (`lib.c:72,106,113,131`) — a *quirk*, not a guard, but it is a
  distinct observable behaviour on invalid/degenerate input and must be matched.

`grep`-verified inventory of every `return` and every implicit-skip check:

```
lib.c:19   char * os_arch = NULL;            <- sentinel initialiser
lib.c:23   if (strstr(os_header, ARCHS[i]))  <- no null check on os_header
lib.c:29   return os_arch;                   <- NULL == "not found"
lib.c:36   if (!(pattern && string)) {
lib.c:37       return 0;
lib.c:40   if (regcomp(&regex, pattern, REG_EXTENDED)) {
lib.c:41       fprintf(stderr, "Couldn't compile regular expression '%s'\n", pattern);
lib.c:42       return 0;
lib.c:45   result = regexec(...);
lib.c:47   return !result;                   <- REG_NOMATCH -> 0
lib.c:64   if (!osd)
lib.c:65       return;                        <- silent no-op
lib.c:68   if (str_tmp = strstr(uname, " [Ver: "), str_tmp)
lib.c:72   *(str_tmp + strlen(str_tmp) - 1) = '\0';
lib.c:75   if (w_regexec("^([0-9]+)\\.*", ...))
lib.c:82   if (w_regexec("^[0-9]+\\.([0-9]+)\\.*", ...))
lib.c:89   if (w_regexec("^[0-9]+\\.[0-9]+\\.([0-9]+(\\.[0-9]+)*)\\.*", ...))
lib.c:98   if (str_tmp = strstr(uname, " ["), str_tmp)
lib.c:102  if (str_tmp = strstr(osd->os_name, ": "), str_tmp)
lib.c:106  *(osd->os_version + strlen(osd->os_version) - 1) = '\0';
lib.c:109  if (str_tmp = strstr(osd->os_version, " ("), str_tmp)
lib.c:113  *(osd->os_codename + strlen(osd->os_codename) - 1) = '\0';
lib.c:131  *(osd->os_name + strlen(osd->os_name) - 1) = '\0';
lib.c:135  if (str_tmp = strstr(osd->os_name, "|"), str_tmp)
lib.c:142  if (str_tmp = get_os_arch(uname), str_tmp)
```

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|----|----------|----------------------------------------------|-------------------|------|----|
| E1  | `w_regexec` | `pattern == NULL`, `string` valid (`lib.c:36`) | returns `0`; `pmatch` untouched | `e1_pattern_null` | [x] |
| E2  | `w_regexec` | `string == NULL`, `pattern` valid (`lib.c:36`) | returns `0`; `pmatch` untouched | `e2_string_null` | [x] |
| E3  | `w_regexec` | `pattern == NULL && string == NULL` (`lib.c:36`) | returns `0`; `pmatch` untouched | `e3_both_null` | [x] |
| E4  | `w_regexec` | `regcomp` fails — unbalanced `(` / `[`, bad `{n,m}`, trailing `\`, bare `*`, `[z-a]`, unmatched `)` (`lib.c:40`) | writes `Couldn't compile regular expression '<pat>'\n` to `stderr`, returns `0`; `pmatch` untouched | `e4_regcomp_failure_matrix` | [x] |
| E5  | `w_regexec` | valid pattern that does not match `string` → `regexec` returns `REG_NOMATCH` (1) (`lib.c:45,47`) | returns `0` | `e5_nomatch` | [x] |
| E6  | `w_regexec` | `nmatch == 0` (with non-null `pmatch`) | `pmatch` left completely untouched; return value still 0/1 by match | `e6_nmatch_zero` | [x] |
| E7  | `w_regexec` | `nmatch == 0` **and** `pmatch == NULL` | no write, return value by match | `e7_nmatch_zero_pmatch_null` | [x] |
| E8  | `w_regexec` | `nmatch` larger than the number of groups (e.g. `nmatch=8` on a 1-group pattern) | surplus entries set to `{-1,-1}` | `e8_nmatch_oversized` | [x] |
| E9  | `w_regexec` | group present in pattern but **not participating** in the match (e.g. `^(a)?b` vs `"b"`) | matched group entry is `{-1,-1}`, return `1` | `e9_nonparticipating_group` | [x] |
| E10 | `w_regexec` | empty pattern `""` (valid ERE, matches everything) | returns `1`, `match[0] == {0,0}` | `e10_empty_pattern` | [x] |
| E11 | `w_regexec` | empty subject `""` | return by whether the pattern matches the empty string | `e11_empty_subject` | [x] |
| E12 | `get_os_arch` | no architecture substring anywhere in `os_header` (`lib.c:19,29`) | returns `NULL` | `e12_arch_not_found` | [x] |
| E13 | `get_os_arch` | empty string `""` | returns `NULL` | `e13_arch_empty` | [x] |
| E14 | `parse_uname_string` | `osd == NULL` (`lib.c:64-65`) | silent `return`, no write anywhere, `uname` buffer unmodified | `e14_osd_null` | [x] |
| E15 | `parse_uname_string` | neither `" [Ver: "` nor `" ["` present (`lib.c:68,98` both fail) | only `os_arch` may be set; every other field stays as the caller left it (**never zeroed**) | `e15_no_bracket_at_all` | [x] |
| E16 | `parse_uname_string` | `" ["` present, but no `": "` after it (`lib.c:102` fails) → `lib.c:131` else-branch | `os_name` = text after `" ["` with its last byte chopped; `os_version`/`os_major`/`os_minor`/`os_codename` untouched | `e16_bracket_without_colon` | [x] |
| E17 | `parse_uname_string` | `" ["` at the very end → text after it is `""` → `lib.c:131` runs with `strlen==0` | writes `'\0'` to `os_name[-1]` (heap-metadata high byte, already 0 → benign); `os_name` stays `""` | `e17_empty_os_name_underflow` | [x] |
| E18 | `parse_uname_string` | `": "` at the very end → `os_version == ""` → `lib.c:106` runs with `strlen==0` | writes `'\0'` to `os_version[-1]`; `os_version` stays `""` | `e18_empty_os_version_underflow` | [x] |
| E19 | `parse_uname_string` | `" ("` at the very end of version → `os_codename == ""` → `lib.c:113` with `strlen==0` | writes `'\0'` to `os_codename[-1]`; `os_codename` stays `""` | `e19_empty_os_codename_underflow` | [x] |
| E20 | `parse_uname_string` | `uname` ends exactly with `" [Ver: "` → `str_tmp == ""` → `lib.c:72` with `strlen==0` | writes `'\0'` **into the caller's `uname` buffer** at the byte before, i.e. over the trailing space of `" [Ver: "` | `e20_empty_ver_underflow_caller_buffer` | [x] |
| E21 | `parse_uname_string` | `" [Ver: "` path, version text is non-numeric (`"abc]"`) → all three `w_regexec` at `lib.c:75,82,89` return 0 | `os_major`/`os_minor`/`os_build` untouched; `os_version`/`os_platform`/`os_name` still set | `e21_ver_nonnumeric` | [x] |
| E22 | `parse_uname_string` | `" [Ver: "` path, only a major (`"10]"`) → `lib.c:82,89` fail | `os_major="10"`, `os_minor`/`os_build` untouched | `e22_ver_major_only` | [x] |
| E23 | `parse_uname_string` | `" [Ver: "` path, major.minor only (`"10.0]"`) → `lib.c:89` fails | `os_build` untouched | `e23_ver_major_minor_only` | [x] |
| E24 | `parse_uname_string` | non-`Ver` path, version text non-numeric → `lib.c:117,124` fail | `os_major`/`os_minor` untouched | `e24_nonver_nonnumeric` | [x] |
| E25 | `parse_uname_string` | empty `uname` (`""`) | both `strstr` fail, `get_os_arch("")` → `NULL`; nothing is written to `osd` at all | `e25_uname_empty` | [x] |
| E26 | `parse_uname_string` | `os_name` has no `"|"` (`lib.c:135` fails) | `os_platform` untouched (stays whatever the caller had) | `e26_no_pipe` | [x] |
| E27 | `parse_uname_string` | `"|"` is the last byte of `os_name` → `os_platform = strdup("")` | `os_platform == ""` (no trim is applied here) | `e27_trailing_pipe` | [x] |
| E28 | `w_regexec` | pattern uses BRE-only syntax invalid under `REG_EXTENDED` (`\(`…`\)` mismatch, `\{`) | same accept/reject decision + same return as C | `e28_bre_vs_ere` | [x] |
| E29 | `get_os_arch` / `parse_uname_string` | `os_header` / `uname == NULL` — **no null guard exists** in the C (`lib.c:23`, `lib.c:68`) | undefined behaviour: `strstr(NULL, …)` faults. Both libraries call the *same* libc `strstr` with the same argument, so behaviour is identical by construction. Asserted out-of-process. | `e29_null_uname_both_fault` | [x] |
| E30 | `w_regexec` | `nmatch > 0` with `pmatch == NULL` | undefined behaviour in glibc `regexec` (writes through the null `pmatch`); identical by construction — same libc call. Documented, not executed. | (documented) | [x] |

Out-of-range enum values: this library's public API has **no enum parameters**
(`os_data` is all `char *`; `nmatch` is `size_t`; `w_regexec` has no flags
parameter — `REG_EXTENDED`/`eflags=0` are hard-coded at `lib.c:40,45`). The
nearest equivalent — an arbitrary `int` where a small domain is expected — is
`nmatch`, covered by E6/E7/E8 plus the huge-`nmatch` sweep in Phase B (`b_*`).
