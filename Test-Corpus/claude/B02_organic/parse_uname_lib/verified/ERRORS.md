# ERRORS.md — Error-surface table (Phase A, tested in Phase C)

Mechanically derived from every rejection / early-return / failure path in
`c_src/src/lib.c`. The library has no error enum and no `assert`; it signals
failure only by (a) `return 0` from `w_regexec`, (b) `return NULL` from
`get_os_arch`, (c) a bare `return` from `parse_uname_string`, and (d) *leaving
an `os_data` member untouched* when a sub-parse does not fire. Case (d) is a
real, observable rejection path, so it gets rows too.

Grep basis:

```
$ grep -n 'return\|if (!' c_src/src/lib.c
19:    char * os_arch = NULL;          <- sentinel initialiser
29:    return os_arch;                 <- NULL when no arch matched
36:    if (!(pattern && string)) {
37:        return 0;
40:    if (regcomp(&regex, pattern, REG_EXTENDED)) {
41:        fprintf(stderr, "Couldn't compile regular expression '%s'\n", pattern);
42:        return 0;
47:    return !result;                 <- 0 when regexec != 0 (REG_NOMATCH)
64:    if (!osd)
65:        return;
```

All rows are covered by `tests/phase_c_errors.rs` unless noted otherwise.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `w_regexec` | `pattern == NULL`, `string` valid (`lib.c:36`) | returns `0`; `pmatch` untouched; nothing printed | [x] |
| 2  | `w_regexec` | `string == NULL`, `pattern` valid (`lib.c:36`) | returns `0`; `pmatch` untouched; nothing printed | [x] |
| 3  | `w_regexec` | `pattern == NULL && string == NULL` (`lib.c:36`) | returns `0`; `pmatch` untouched | [x] |
| 4  | `w_regexec` | `regcomp` fails — unmatched `(` e.g. `"("` (`lib.c:40`) | prints `Couldn't compile regular expression '('` to `stderr`, returns `0`, `pmatch` untouched | [x] |
| 5  | `w_regexec` | `regcomp` fails — unmatched `[` e.g. `"[a-"` | as row 4, returns `0` | [x] |
| 6  | `w_regexec` | `regcomp` fails — trailing backslash `"a\\"` | as row 4, returns `0` | [x] |
| 7  | `w_regexec` | `regcomp` fails — bad repetition `"*"` / `"a{2,1}"` / `"+"` / `"?"` | as row 4, returns `0` | [x] |
| 8  | `w_regexec` | `regcomp` fails — invalid back-reference / bad class `"[[:bogus:]]"` | as row 4, returns `0` | [x] |
| 9  | `w_regexec` | valid pattern, `regexec` returns `REG_NOMATCH` (`lib.c:45,47`) | returns `0` (`!result`), `pmatch` **is** clobbered by glibc-defined amount — compared byte-for-byte | [x] |
| 10 | `w_regexec` | `nmatch == 0` with `pmatch == NULL` (degenerate but legal) | return value only (`1`/`0`); no write through `pmatch` | [x] |
| 11 | `w_regexec` | `nmatch == 0` with non-NULL `pmatch` | buffer left entirely untouched | [x] |
| 12 | `w_regexec` | `nmatch` **smaller** than the number of groups (`nmatch == 1`, pattern has 1+ groups) | only `pmatch[0]` written | [x] |
| 13 | `w_regexec` | `nmatch` **larger** than `re_nsub+1` (`nmatch == 8`, 1 group) | glibc fills the surplus entries with `-1`; all 8 entries compared | [x] |
| 14 | `w_regexec` | non-participating group, e.g. pattern `"^(a)?b"` vs `"b"` — match succeeds but group 1 unset | returns `1`, `pmatch[1] == {-1,-1}` | [x] |
| 15 | `w_regexec` | empty pattern `""` (compiles, matches everything) | returns `1`, `pmatch[0] == {0,0}` | [x] |
| 16 | `w_regexec` | empty subject `""` with pattern that cannot match it | returns `0` | [x] |
| 17 | `get_os_arch` | no architecture substring anywhere in `os_header` (`lib.c:19,29`) | returns `NULL` | [x] |
| 18 | `get_os_arch` | empty string `""` | returns `NULL` | [x] |
| 19 | `get_os_arch` | near-miss / case-mismatched arch (`"X86_64"`, `"aix"`, `"ARM64"`, `"x86-64"`, `"armv"`, `"i38"`) | returns `NULL` (`strstr` is case-sensitive, no partial credit) | [x] |
| 20 | `parse_uname_string` | `osd == NULL` (`lib.c:64-65`) | returns immediately; `uname` buffer **not** modified at all | [x] |
| 21 | `parse_uname_string` | `osd == NULL` **and** `uname == NULL` | returns immediately, no dereference of `uname` — no crash | [x] |
| 22 | `parse_uname_string` | neither `" [Ver: "` nor `" ["` present (`lib.c:68,98`) | `os_name`/`os_version`/`os_major`/`os_minor`/`os_codename`/`os_platform`/`os_build` stay `NULL`; only `os_arch` may be set | [x] |
| 23 | `parse_uname_string` | `" ["` present but no `": "` in the bracket body (`lib.c:102` false → `lib.c:131`) | `os_version`, `os_major`, `os_minor`, `os_codename` stay `NULL`; `os_name` loses its last byte | [x] |
| 24 | `parse_uname_string` | `" ["` + `": "`, but no `" ("` (`lib.c:109` false) | `os_codename` stays `NULL` | [x] |
| 25 | `parse_uname_string` | `" ["` + `": "`, version does not start with digits, e.g. `"rolling"` (`lib.c:117` false) | `os_major` **and** `os_minor` stay `NULL` | [x] |
| 26 | `parse_uname_string` | `" ["` + `": "`, version has a major but no `.minor`, e.g. `"9]"` (`lib.c:124` false) | `os_major` set, `os_minor` stays `NULL` | [x] |
| 27 | `parse_uname_string` | no `"\|"` in `os_name` (`lib.c:135` false) | `os_platform` stays `NULL` in the non-Windows branch | [x] |
| 28 | `parse_uname_string` | `" [Ver: "` branch, version not starting with a digit (`lib.c:75` false) | `os_major`, `os_minor`, `os_build` all stay `NULL`; `os_version`/`os_platform`/`os_name` still set | [x] |
| 29 | `parse_uname_string` | `" [Ver: "` branch, only a major (`"10]"`) → `lib.c:82`/`89` false | `os_minor` and `os_build` stay `NULL` | [x] |
| 30 | `parse_uname_string` | `" [Ver: "` branch, major.minor only (`"6.1]"`) → `lib.c:89` false | `os_build` stays `NULL` | [x] |
| 31 | `parse_uname_string` | `" [Ver: "` branch — `os_arch` is **never** consulted (arch text in input is ignored) | `os_arch` stays `NULL` even for `"x86_64 ... [Ver: 10.0]"` | [x] |
| 32 | `parse_uname_string` | boundary: `uname` is exactly `" [Ver: "` → `str_tmp` points at `""`, so `*(str_tmp+strlen-1)` writes **one byte before** the substring (`lib.c:72`) | reproduced identically; whole padded `uname` buffer compared byte-for-byte | [x] |
| 33 | `parse_uname_string` | boundary: `uname` is exactly `" ["` → `os_name = strdup("")`, then `*(os_name+strlen-1)` writes one byte **before** the heap block (`lib.c:131`) | reproduced identically (on x86-64 glibc that byte is the high byte of the chunk-size field, i.e. already `0`); outputs compared | [x] |
| 34 | `parse_uname_string` | boundary: `": "` at the very end → `os_version = strdup("")`, `*(os_version-1) = 0` (`lib.c:106`) | reproduced identically; outputs compared | [x] |
| 35 | `parse_uname_string` | boundary: `" ("` at the very end → `os_codename = strdup("")`, `*(os_codename-1) = 0` (`lib.c:113`) | reproduced identically; outputs compared | [x] |
| 36 | `parse_uname_string` | boundary: `"\|"` at the very end of `os_name` → `os_platform = strdup("")` | `os_platform` is `""`, not `NULL` | [x] |
| 37 | `parse_uname_string` | empty `uname` `""` | no substring found → every field stays `NULL` | [x] |
| 38 | `parse_uname_string` | `uname` = `NULL` with non-NULL `osd` — `strstr(NULL, …)` (`lib.c:68`) | **crashes (SIGSEGV) in C**; verified in a forked child that C and Rust die with the *same* signal | [x] |
| 39 | `get_os_arch` | `os_header == NULL` — `strstr(NULL, …)` (`lib.c:23`) | **crashes (SIGSEGV) in C**; same-signal parity verified in a forked child | [x] |
| 40 | `w_regexec` | compilable pattern that **matches**, `nmatch > 0`, `pmatch == NULL` (`lib.c:45`) — glibc `regexec` writes through the NULL pointer | **crashes (SIGSEGV) in C**; same-signal parity verified in a forked child. Negative controls: `nmatch == 0`, a non-matching pattern, or a pattern that fails to compile all return `0`/`1` cleanly on both sides | [x] |

## Generic FFI boundary cases also covered

| item | how it is covered |
|------|-------------------|
| null pointers | rows 1–3, 10, 20, 21, 38, 39, 40 (`pattern`, `string`, `pmatch`, `osd`, `uname`, `os_header`) |
| zero lengths | rows 10, 11, 15, 16, 18, 37 (`nmatch == 0`, empty pattern, empty subject, empty `uname`) |
| oversized lengths | row 13 (`nmatch` ≫ group count); plus a 4 KiB `uname` and a 4 KiB pattern subject in Phase B |
| one step past a valid range | rows 12 (`nmatch = nsub`), 13 (`nmatch = nsub + 7`), 19 (arch names one character off), 26/29/30 (version one component short) |
| out-of-range **enum** values across FFI | **N/A by inspection** — the public surface has no `enum` parameter. `w_regexec`'s `cflags`/`eflags` are hard-coded (`REG_EXTENDED`, `0`) inside the C function and are not reachable from a caller; the only integral parameter is `size_t nmatch`, whose out-of-domain values are rows 10–13. A dedicated test asserts this by driving `nmatch` over `{0,1,2,3,8,64}` — i.e. every value with and without a "valid variant". |
| garbage / non-zeroed `os_data` on entry | Phase B passes a `0xAA`-filled `os_data` to both, proving the C leaves untouched members alone and the Rust does too |

## Row → test mapping

| rows | test file :: test fn |
|------|----------------------|
| 1 | `tests/phase_c_errors.rs::row01_null_pattern` |
| 2 | `…::row02_null_string` |
| 3 | `…::row03_both_null` |
| 4-8 | `…::rows04to08_regcomp_failures`, `…::rows04to08_are_really_compile_failures`, `tests/phase_c_stderr.rs::regcomp_diagnostic_is_byte_identical` |
| 9 | `…::row09_no_match` |
| 10 | `…::row10_nmatch_zero_null_pmatch` |
| 11 | `…::row11_nmatch_zero_with_buffer` |
| 12 | `…::row12_nmatch_smaller_than_group_count`, `…::pmatch_write_extent_is_identical` |
| 13 | `…::row13_nmatch_larger_than_group_count`, `…::row13b_nmatch_dense_sweep` |
| 14 | `…::row14_non_participating_group` |
| 15 | `…::row15_empty_pattern` |
| 16 | `…::row16_empty_subject` |
| 17 | `…::row17_no_arch_found` |
| 18 | `…::row18_empty_header` |
| 19 | `…::row19_case_and_near_miss` |
| 20 | `…::row20_null_osd`, `tests/phase_c_crash_parity.rs::row40_negative_controls` |
| 21 | `…::row21_null_osd_and_null_uname` |
| 22 | `…::row22_neither_marker` |
| 23 | `…::row23_bracket_without_colon_space` |
| 24 | `…::row24_no_codename_marker` |
| 25 | `…::row25_non_numeric_version_unix` |
| 26 | `…::row26_major_without_minor_unix` |
| 27 | `…::row27_no_pipe_in_os_name` |
| 28 | `…::row28_windows_non_numeric_version` |
| 29 | `…::row29_windows_major_only` |
| 30 | `…::row30_windows_major_minor_only` |
| 31 | `…::row31_windows_never_sets_arch` |
| 32 | `…::row32_ver_marker_at_end` |
| 33 | `…::row33_bracket_marker_at_end` |
| 34 | `…::row34_colon_space_at_end` |
| 35 | `…::row35_paren_marker_at_end` |
| 36 | `…::row36_pipe_at_end_of_os_name` |
| 37 | `…::row37_empty_uname` |
| 38 | `tests/phase_c_crash_parity.rs::row38_null_uname_with_valid_osd` |
| 39 | `…::row39_null_os_header` |
| 40 | `…::row40_null_pmatch_with_nonzero_nmatch` (+ `row40_negative_controls`) |
| — | `tests/phase_c_errors.rs::os_uname_member_is_never_written`, `…::valid_odd_patterns_take_the_match_path`, `tests/phase_c_stderr.rs::null_short_circuit_prints_nothing`, `…::parse_uname_string_prints_nothing` |

Rows are asserted, not merely "both failed": every rejection row checks the same
sentinel (`0` / `NULL` / poison-value-untouched member), and the two crashing
rows compare the terminating **signal number** of a forked child, with
negative controls proving the harness distinguishes a crash from a clean return.
