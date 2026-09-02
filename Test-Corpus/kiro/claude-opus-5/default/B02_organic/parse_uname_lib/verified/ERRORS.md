# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no** error enum,
**no** `assert`, and **no** `RETURN_ERROR`-style macro. Every rejection is
either an early `return`, a NULL sentinel, or a "field left untouched" outcome.
Each distinct branch below is one row.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `w_regexec` (L36-38) | `pattern == NULL`, `string` valid | returns `0`; `pmatch` untouched; no regex compiled | `e1_pattern_null` | [x] |
| E2 | `w_regexec` (L36-38) | `string == NULL`, `pattern` valid | returns `0`; `pmatch` untouched | `e2_string_null` | [x] |
| E3 | `w_regexec` (L36-38) | both `pattern` and `string` `NULL` | returns `0` | `e3_both_null` | [x] |
| E4 | `w_regexec` (L40-43) | `regcomp` fails (malformed ERE: `"["`, `"("`, `"a{2,1}"`, `"*"`, `"\\"`, `"[z-a]"`, `"a{"`, `"(|"` …) | prints `Couldn't compile regular expression '<pat>'\n` to `stderr`, returns `0`; **no `regfree`** (C leaks; Rust must not diverge observably) | `e4_regcomp_failure` | [x] |
| E5 | `w_regexec` (L45-47) | valid pattern that does **not** match `string` | `regexec` returns `REG_NOMATCH`, so `!result == 0` → returns `0` | `e5_no_match` | [x] |
| E6 | `w_regexec` (L45) | `nmatch == 0` together with `pmatch == NULL` | legal; returns `1` on match, `0` on no-match; nothing written | `e6_nmatch_zero_null_pmatch` | [x] |
| E7 | `w_regexec` (L45) | `nmatch` **larger** than `1 + re_nsub` (oversized length) | surplus `regmatch_t` slots are set to `{-1,-1}` by glibc; return value unaffected | `e7_oversized_nmatch` | [x] |
| E8 | `w_regexec` (L45) | `nmatch` smaller than the group count (e.g. `1`, so group 1 is not reported) | return value still `1`; `pmatch[1]` **not** written (stale value preserved) | `e8_undersized_nmatch` | [x] |
| E9 | `w_regexec` (L45) | pattern matches but a capture group does **not participate** (`"^(a)|(b)$"` on `"b"`) | non-participating group reported as `rm_so == rm_eo == -1`; caller would compute `match_size == 0` | `e9_nonparticipating_group` | [x] |
| E10 | `get_os_arch` (L19-29) | `os_header` contains none of the 12 architecture literals | returns `NULL` (sentinel) | `e10_arch_not_found` | [x] |
| E11 | `get_os_arch` (L23) | `os_header` is the empty string `""` | `strstr("", arch)` never matches → returns `NULL` | `e11_arch_empty_string` | [x] |
| E12 | `parse_uname_string` (L64-65) | `osd == NULL` (null pointer for the out-struct) | returns immediately, **before** touching `uname`; `uname` is left completely unmodified even if it contains `" [Ver: "` / `" ["` | `e12_osd_null` | [x] |
| E13 | `parse_uname_string` (L68 / L98) | `uname` contains neither `" [Ver: "` nor `" ["` | both `strstr` fail: `os_name`, `os_version`, `os_major`, `os_minor`, `os_codename`, `os_platform`, `os_build`, `os_uname` are **all left untouched**; only `os_arch` may be set | `e13_no_bracket_marker` | [x] |
| E14 | `parse_uname_string` (L142-145) | no `" ["` **and** no architecture literal | nothing at all is written to `*osd`; every field stays as the caller left it (verified with a pre-poisoned struct) | `e14_nothing_written` | [x] |
| E15 | `parse_uname_string` (L75-93) | Windows branch, version text is non-numeric (`"abc"`) so all three regexes fail | `os_major`, `os_minor`, `os_build` left untouched (NULL if caller zeroed); `os_version`/`os_platform`/`os_name` still set | `e15_windows_non_numeric` | [x] |
| E16 | `parse_uname_string` (L82-86) | Windows branch, only a major present (`"10"`) so the minor and build regexes fail | `os_minor`, `os_build` untouched | `e16_windows_major_only` | [x] |
| E17 | `parse_uname_string` (L89-93) | Windows branch, `major.minor` only (`"6.1"`) so the build regex fails | `os_build` untouched | `e17_windows_no_build` | [x] |
| E18 | `parse_uname_string` (L117-128) | Unix branch, `os_version` non-numeric so the major/minor regexes fail | `os_major`, `os_minor` untouched | `e18_unix_non_numeric` | [x] |
| E19 | `parse_uname_string` (L102-133) | Unix branch, `os_name` contains no `": "` | `else` at L131: `*(os_name + strlen(os_name) - 1) = '\0'` — the last byte of `os_name` is dropped; `os_version`, `os_codename`, `os_major`, `os_minor` all left untouched | `e19_unix_no_colon` | [x] |
| E20 | `parse_uname_string` (L109-113) | Unix branch with `": "` but no `" ("` | `os_codename` left untouched | `e20_unix_no_codename` | [x] |
| E21 | `parse_uname_string` (L135-139) | Unix branch, no `"\|"` in the (already truncated) `os_name` | `os_platform` left untouched (stays NULL — note the Windows branch always sets it to `"windows"`) | `e21_unix_no_platform` | [x] |
| E22 | `parse_uname_string` (L135) | `"\|"` present in `uname` but **after** the `": "` separator, i.e. removed by the L103 truncation | `strstr(os_name, "\|")` fails → `os_platform` left untouched | `e22_pipe_after_colon` | [x] |
| E23 | `parse_uname_string` (L142) | architecture literal present in `uname` but **after** `" ["`, i.e. cut off by the L99 `*str_tmp = '\0'` | `get_os_arch` sees only the prefix → returns `NULL` → `os_arch` untouched | `e23_arch_after_bracket` | [x] |
| E24 | `parse_uname_string` (L68-96) | Windows branch taken **and** `uname` contains an architecture literal | `get_os_arch` is **never called** in the `if` branch → `os_arch` stays NULL even though the arch is present | `e24_windows_never_sets_arch` | [x] |
| E25 | `parse_uname_string` (L72) | Windows branch, zero-length version remainder (`uname` ends exactly with `" [Ver: "`) | `strlen == 0` → `*(str_tmp - 1) = '\0'` writes one byte **before** `str_tmp`, i.e. into the trailing space of the `" [Ver: "` literal inside the caller's buffer; then `os_version = strdup("")` | `e25_windows_empty_version` | [x] |
| E26 | `parse_uname_string` (L105) | Unix branch, zero-length version (`"…: ]"` → remainder after `": "` is `"]"`, strip yields `""`; and the `": "`-at-end case) | `strdup` of a 1-byte buffer followed by `*(p-1) = '\0'` — an out-of-bounds write into the malloc chunk header (a no-op for the high byte of the size field on x86-64 LE, but must be reproduced) | `e26_unix_empty_version` | [x] |
| E27 | `parse_uname_string` (L131) | Unix branch, zero-length `os_name` (`uname` ends exactly with `" ["`) | `os_name = strdup("")`, then `*(os_name - 1) = '\0'` — same out-of-bounds write | `e27_unix_empty_name` | [x] |
| E28 | `parse_uname_string` (L112) | Unix branch, zero-length codename (`"… (…)"` where the codename text is a single `")"`) | `os_codename = strdup("")` after the strip, or the `*(p-1)` OOB write when the codename part is empty | `e28_unix_empty_codename` | [x] |
| E29 | `parse_uname_string` (L61) | `uname` is the empty string `""` | no marker found, `get_os_arch("")` → NULL → nothing written | `e29_uname_empty` | [x] |
| E30 | `w_regexec` (L45) | `string` is the empty string with a pattern that cannot match it | returns `0` | `e30_empty_subject` | [x] |
| E31 | `w_regexec` (L40) | `pattern` is the empty string `""` | glibc `regcomp` **accepts** it (ERE empty pattern) and it matches at offset 0 → returns `1` with `pmatch[0] = {0,0}` | `e31_empty_pattern` | [x] |
| E32 | `w_regexec` / `parse_uname_string` | `regmatch_t` values one step past a valid range: `pmatch` slot reused across calls so `rm_so`/`rm_eo` hold a stale offset from a *longer* previous subject, then a failing regex leaves them stale | `parse_uname_string` only reads `match[1]` when `w_regexec` returned non-zero, so staleness is unobservable — asserted to be identical anyway | `e32_stale_pmatch_reuse` | [x] |

## Notes on "out-of-range enum values across the FFI boundary"

The public ABI of this library has **no enum parameter**: the three exported
functions take only `char *`, `size_t` and `regmatch_t *`. The nearest
equivalent to "an integer with no valid variant" is:

* `nmatch: size_t` — an unbounded count that has no valid upper limit; covered
  by rows **E6/E7/E8** (0, undersized, oversized) and additionally fuzzed with
  `nmatch` values up to the size of the caller's buffer.
* `cflags`/`eflags` are hard-coded constants (`REG_EXTENDED`, `0`) inside
  `w_regexec`, so a caller cannot supply an invalid value.

Both are covered above; there is no reachable invalid-enum input to test.

## Deliberately untested

* `malloc` returning `NULL` (L77, L84, L91, L119, L126) is never checked by the
  C code; forcing an allocation failure would make both implementations
  dereference NULL identically and abort the test process. Not reachable in a
  differential test.
* `uname == NULL` with a non-NULL `osd`: the C dereferences it inside `strstr`
  at L68 and segfaults. The Rust does the same (`strstr` is the identical libc
  call reached in the identical position, *after* the `!osd` check). Asserting
  "both crash" is not something a differential test can do in-process, and it
  is not a *rejection* the C implements — so it is recorded here rather than as
  a table row.
