# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived by grepping `c_src/src/lib.c` for every `return`,
sentinel, guard, and range check. There are no `assert`s, no error enums, and
no `RETURN_ERROR`-style macros in this library; rejection is expressed
entirely as **NULL returns**, **`0` returns**, and **early `return;`**.

Grep evidence:

```
$ grep -n 'return\|if (!' c_src/src/lib.c
29:    return os_arch;                 # NULL when no arch matched
37:        return 0;                   # !(pattern && string)
42:        return 0;                   # regcomp failed  (+ fprintf to stderr)
47:    return !result;                 # 0 when regexec did not match
65:        return;                     # !osd
```

Every `if (str_tmp = strstr(...), str_tmp)` is additionally a *silent*
rejection path: when the separator is absent the corresponding output field is
left **untouched** (i.e. whatever the caller pre-set, NULL for a zeroed
`os_data`). Those are rows 8–15.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `get_os_arch` | `os_header` contains none of the 12 ARCHS (e.g. `"Linux ppc64le 5.4"`) | returns `NULL` (`os_arch` stays `NULL`, loop falls through `ARCHS[12]==NULL`) |
| 2 | `get_os_arch` | `os_header` is the empty string `""` | returns `NULL` |
| 3 | `w_regexec` | `pattern == NULL` (string non-NULL) | returns `0`, `pmatch` untouched, nothing printed |
| 4 | `w_regexec` | `string == NULL` (pattern non-NULL) | returns `0`, `pmatch` untouched, nothing printed |
| 5 | `w_regexec` | `pattern == NULL && string == NULL` | returns `0` |
| 6 | `w_regexec` | `pattern` is not a valid POSIX ERE, so `regcomp` != 0 (e.g. `"["`, `"("`, `"a{2,1}"`, `"*"`, `"a\\"`, `"(|"`, `"[z-a]"`) | returns `0`; writes `Couldn't compile regular expression '<pattern>'\n` to **stderr**; `regfree` NOT called |
| 7 | `w_regexec` | valid pattern that does not match `string` (`regexec` returns `REG_NOMATCH`) | returns `0` (`!result` where `result != 0`); `pmatch` contents are unspecified/left by regexec |
| 8 | `w_regexec` | `nmatch == 0` (with any `pmatch`, incl. NULL) | `regexec` writes nothing; return value is still `1` on match / `0` on no-match |
| 9 | `parse_uname_string` | `osd == NULL` | returns immediately, no writes, `uname` **not** mutated (not even the `strstr` truncation) |
| 10 | `parse_uname_string` | `uname` contains neither `" [Ver: "` nor `" ["` (e.g. `"Linux x86_64"`) | takes the `else` branch, inner `if` fails: `os_name/os_version/os_major/os_minor/os_codename/os_platform/os_build` all left untouched; ONLY `os_arch` is set (or left NULL per row 1) |
| 11 | `parse_uname_string` | Windows branch, version text has no leading digits (e.g. `"W [Ver: abc]"`) | all three `w_regexec` calls return 0 ⇒ `os_major`, `os_minor`, `os_build` left `NULL`; `os_name`, `os_version`, `os_platform="windows"` still set; **`os_arch` never set** (Windows branch skips `get_os_arch`) |
| 12 | `parse_uname_string` | Windows branch, only a major present (e.g. `"W [Ver: 10]"`) | `os_major="10"`; minor & build regexes fail ⇒ `os_minor`/`os_build` stay `NULL` |
| 13 | `parse_uname_string` | Windows branch, major+minor only (e.g. `"W [Ver: 10.0]"`) | `os_major`,`os_minor` set; `os_build` stays `NULL` |
| 14 | `parse_uname_string` | Unix branch, no `": "` after `" ["` (e.g. `"Linux [ubuntu]"`) | `else` at line 130-132: `*(os_name+strlen(os_name)-1)='\0'`; `os_version`/`os_major`/`os_minor`/`os_codename` stay `NULL` |
| 15 | `parse_uname_string` | Unix branch, `os_version` has no `" ("` (e.g. `"Linux [ubuntu: 20.04]"`) | `os_codename` stays `NULL` |
| 16 | `parse_uname_string` | Unix branch, `os_name` has no `"|"` (e.g. `"Linux [ubuntu: 20.04]"`) | `os_platform` stays `NULL` |
| 17 | `parse_uname_string` | Unix branch, `os_version` has no leading digits (e.g. `"L [name: notaversion]"`) | `os_major`/`os_minor` stay `NULL` |
| 18 | `parse_uname_string` | Unix branch, `os_version` major only (e.g. `"L [n: 7]"`) | `os_major="7"`, `os_minor` `NULL` |
| 19 | `parse_uname_string` | `os_uname` field | **never** written by any code path — always left as the caller set it, in every branch |
| 20 | `parse_uname_string` | Windows branch never sets `os_arch`/`os_codename` even when `uname` contains an arch string (e.g. `"W x86_64 [Ver: 10.0.1]"`) | `os_arch == NULL`, `os_codename == NULL` |
| 21 | `parse_uname_string` | Underflow write: Windows branch with empty version, `"W [Ver: "` ⇒ `strlen(str_tmp)==0` ⇒ `*(str_tmp-1)='\0'` | writes the NUL one byte *before* `str_tmp`, i.e. **inside** the caller's `uname` buffer (the `' '` of `" [Ver: "`); then all regexes fail, `os_version=""`, `os_platform="windows"` |
| 22 | `parse_uname_string` | Underflow write: Unix branch `"Linux ["` ⇒ `os_name=strdup("")` ⇒ `*(os_name-1)='\0'` | out-of-bounds write 1 byte before a heap block (heap-metadata corruption). Same UB must be reproduced by Rust byte-for-byte |
| 23 | `parse_uname_string` | Underflow write: Unix branch `"L [n: "` ⇒ `os_version=strdup("")` ⇒ `*(os_version-1)='\0'` | same 1-byte heap underflow |
| 24 | `parse_uname_string` | Underflow write: Unix branch `"L [n: 1.2 ()"` ⇒ codename becomes `""` ⇒ `*(os_codename-1)='\0'` | same 1-byte heap underflow |
| 25 | `parse_uname_string`/`dup_match` | regex matched overall but group 1 did not participate ⇒ `match[1] = {-1,-1}` ⇒ `match_size = 0`, `malloc(1)`, `snprintf(dst,1,"%.*s",0, base-1)` | allocates a 1-byte `""` string (not NULL). Reachable pattern-wise only for the build regex's optional group; must match C exactly |
| 26 | `parse_uname_string` | `uname == NULL` with non-NULL `osd` | **not checked by C** ⇒ `strstr(NULL, ...)` segfaults. Rust must be identically unchecked (verified by symmetric crash in a forked child) |
| 27 | `get_os_arch` | `os_header == NULL` | **not checked** ⇒ `strstr(NULL,...)` segfaults; Rust identical |
| 28 | `w_regexec` | `pmatch == NULL` with `nmatch > 0` and a matching pattern | **not checked** ⇒ `regexec` writes to NULL / segfaults; Rust identical |
| 29 | `w_regexec` | oversized `nmatch` (e.g. `SIZE_MAX`, or `64` with a 2-element `pmatch`) | **not checked** ⇒ `regexec` writes `min(nmatch, re_nsub+1)` entries; buffer overrun beyond `re_nsub+1` does NOT occur, so a large `nmatch` with a big enough buffer is well-defined and must agree |
| 30 | `parse_uname_string` | `" [Ver: "` present **and** `" ["` present (`" [Ver: "` wins, since it is tested first) | Windows branch taken; the `" ["` / arch logic is never reached |

---

## Verification status — every row has a passing differential test

Run with `cargo test --offline --release --test phase_c_errors` (34/34 pass;
also re-run for the debug profile and every feature combination by
`./check_features.sh`).

| # | covering test in `tests/phase_c_errors.rs` | [x] |
|---|--------------------------------------------|-----|
| 1 | `err01_get_os_arch_no_match_returns_null` | [x] |
| 2 | `err02_get_os_arch_empty_returns_null` | [x] |
| 3 | `err03_w_regexec_null_pattern` | [x] |
| 4 | `err04_w_regexec_null_string` | [x] |
| 5 | `err05_w_regexec_both_null` | [x] |
| 6 | `err06_w_regexec_regcomp_failure_return_value` + `err06b_..._stderr_message` (stderr compared verbatim in child processes) | [x] |
| 7 | `err07_w_regexec_no_match_returns_zero` | [x] |
| 8 | `err08_w_regexec_nmatch_zero` | [x] |
| 9 | `err09_parse_null_osd` | [x] |
| 10 | `err10_no_separator_only_arch_set` | [x] |
| 11 | `err11_windows_non_numeric_version` | [x] |
| 12 | `err12_windows_major_only` | [x] |
| 13 | `err13_windows_major_minor_only` | [x] |
| 14 | `err14_unix_no_colon` | [x] |
| 15 | `err15_unix_no_codename` | [x] |
| 16 | `err16_unix_no_pipe_platform` | [x] |
| 17 | `err17_unix_non_numeric_version` | [x] |
| 18 | `err18_unix_major_only` | [x] |
| 19 | `err19_os_uname_never_written` | [x] |
| 20 | `err20_windows_never_sets_arch_or_codename` | [x] |
| 21 | `err21_windows_empty_version_underflow_in_buffer` | [x] |
| 22 | `err22_heap_underflow_os_name` (isolated child + in-process field compare) | [x] |
| 23 | `err23_heap_underflow_os_version` | [x] |
| 24 | `err24_heap_underflow_os_codename` | [x] |
| 25 | `err25_non_participating_group_yields_empty_string` | [x] |
| 26 | `err26_parse_null_uname_crash_parity` (both fault with the same signal) | [x] |
| 27 | `err27_get_os_arch_null_crash_parity` | [x] |
| 28 | `err28_regexec_null_pmatch_crash_parity` | [x] |
| 29 | `err29_oversized_nmatch` + `err29b_oversized_nmatch_stdout_parity` (`SIZE_MAX`) | [x] |
| 30 | `err30_ver_marker_takes_precedence` | [x] |
| — | generic boundaries: NULL pointers, zero/oversized lengths, one-past-range `nmatch`, every single byte value 1..=255 as a uname, embedded NULs, empty patterns → `generic_boundaries_sweep` | [x] |

### Note on out-of-range enum values

The public surface (`include/lib.h` + the two extra exported functions)
contains **no enum parameters** — the only non-pointer arguments are
`size_t nmatch` (covered by rows 8, 28, 29 and the boundary sweep). The
`regmatch_t` values crossing the boundary are plain `int`s and are seeded with
the out-of-band pattern `{0x5A5A5A5A, 0x3C3C3C3C}` before every call so that
"slot left untouched" is observable and compared.
