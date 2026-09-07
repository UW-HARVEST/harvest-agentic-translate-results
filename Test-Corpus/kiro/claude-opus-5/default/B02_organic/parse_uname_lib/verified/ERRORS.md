# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c` (147 lines) and
`c_src/include/lib.h`. Every `return` that yields a "nothing produced" result,
every `NULL` guard, every regex-failure branch (the `if (w_regexec(...))` that
leaves an OUT field untouched when it fails), every implicit out-of-bounds /
undefined-behaviour write, and every generic FFI boundary.

Notes on how to read this:

* There is **no error enum, no `RETURN_ERROR` macro, and no `assert`** in this
  translation unit (`grep -n 'assert\|RETURN_ERROR\|errno\|enum' c_src/src/lib.c`
  → no hits). The library signals failure exclusively by returning `NULL` /
  `0`, or by *leaving an OUT field untouched*.
* `parse_uname_string` returns `void`. Its "error result" is therefore observed
  as *which `os_data` fields stay at their pre-call value*. All tests
  zero-initialise `os_data`, so "untouched" == `NULL`.
* Rows marked **UB** are undefined behaviour in the C. They are still real
  inputs the C accepts, so the Rust must reproduce them bit-for-bit. They are
  run in a `fork()`ed child so a crash is observed (and compared) rather than
  killing the test process.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `get_os_arch` | `os_header` contains none of the 12 `ARCHS` strings (loop falls through, `os_arch` stays `NULL`) | returns `NULL` |
| 2 | `get_os_arch` | `os_header == NULL` (no null check; `strstr(NULL, "x86_64")`) — **UB** | `SIGSEGV` |
| 3 | `get_os_arch` | `os_header == ""` (empty string, no arch) | returns `NULL` |
| 4 | `w_regexec` | `pattern == NULL`, `string != NULL` → `!(pattern && string)` | returns `0`, `pmatch` untouched |
| 5 | `w_regexec` | `pattern != NULL`, `string == NULL` → `!(pattern && string)` | returns `0`, `pmatch` untouched |
| 6 | `w_regexec` | `pattern == NULL && string == NULL` | returns `0`, `pmatch` untouched |
| 7 | `w_regexec` | `regcomp()` fails — unbalanced bracket `"["` (REG_EBRACK) | writes `Couldn't compile regular expression '['\n` to `stderr`, returns `0`; **no `regfree`** |
| 8 | `w_regexec` | `regcomp()` fails — trailing backslash `"\\"` (REG_EESCAPE) | stderr message, returns `0` |
| 9 | `w_regexec` | `regcomp()` fails — unmatched `"("` (REG_EPAREN) | stderr message, returns `0` |
| 10 | `w_regexec` | `regcomp()` fails — bad repetition `"*"` / `"{1"` / `"a{2,1}"` | stderr message, returns `0` |
| 11 | `w_regexec` | valid pattern, `regexec` returns `REG_NOMATCH` (1) → `!1` | returns `0`; `pmatch` **is** written by glibc (all `-1`) for `nmatch>0` |
| 12 | `w_regexec` | `nmatch == 0`, `pmatch == NULL` | no write through `pmatch`; return is match/no-match |
| 13 | `w_regexec` | `nmatch == 0`, `pmatch != NULL` | `pmatch` untouched, return is match/no-match |
| 14 | `w_regexec` | `nmatch > 0`, `pmatch == NULL` (no null check) — **UB** | `SIGSEGV` inside `regexec` |
| 15 | `w_regexec` | `nmatch` larger than `re_nsub+1` (e.g. `nmatch=8` on a 1-group pattern) | surplus entries set to `{-1,-1}`, return `1` |
| 16 | `w_regexec` | `nmatch` smaller than the group count used by the caller (`nmatch=1`, pattern has 1 group) | only `pmatch[0]` written; `pmatch[1]` keeps its previous value (the caller's stale data) |
| 17 | `w_regexec` | `string == ""` with a pattern that cannot match empty | returns `0` |
| 18 | `w_regexec` | `pattern == ""` (empty regex is valid POSIX-extended in glibc) | returns `1`, `pmatch[0] = {0,0}` |
| 19 | `parse_uname_string` | `osd == NULL` | returns immediately; `uname` buffer **not** modified |
| 20 | `parse_uname_string` | `uname == NULL` (checked *after* `osd`; `strstr(NULL, " [Ver: ")`) — **UB** | `SIGSEGV` |
| 21 | `parse_uname_string` | `uname == NULL` **and** `osd == NULL` | returns before dereferencing `uname` → **no** crash |
| 22 | `parse_uname_string` | `uname` matches neither `" [Ver: "` nor `" ["` | all 9 fields stay `NULL` except `os_arch`, set only if `get_os_arch(uname)` hits |
| 23 | `parse_uname_string` | `uname` matches neither marker **and** has no arch | all 9 fields stay `NULL` |
| 24 | `parse_uname_string` | Ver branch, `str_tmp` does not start with digits → `^([0-9]+)\.*` fails | `os_major` stays `NULL` |
| 25 | `parse_uname_string` | Ver branch, `^[0-9]+\.([0-9]+)\.*` fails (e.g. `"10"`, `"10.x"`) | `os_minor` stays `NULL` |
| 26 | `parse_uname_string` | Ver branch, `^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*` fails (e.g. `"10.0"`) | `os_build` stays `NULL` |
| 27 | `parse_uname_string` | Ver branch always taken → `get_os_arch` is **never** called | `os_arch` stays `NULL` even when the uname contains `x86_64` |
| 28 | `parse_uname_string` | Ver branch, text after `" [Ver: "` is empty (`uname == " [Ver: "`) → `*(str_tmp + strlen-1)` writes at `str_tmp[-1]`, i.e. `uname[6]` — **UB but in-buffer** | `uname` becomes `"\0[Ver:\0\0"`; `os_name=""`, `os_version=""`, `os_platform="windows"`, major/minor/build `NULL` |
| 29 | `parse_uname_string` | Unix branch, no `": "` in `os_name` → `else` arm strips the last char of `os_name` | `os_version`, `os_codename`, `os_major`, `os_minor` stay `NULL` |
| 30 | `parse_uname_string` | Unix branch, `os_name` is empty (`uname == " ["`) → `strip_last_char("")` writes at `os_name[-1]` (heap byte *before* the `malloc` block) — **UB** | no crash on glibc; `os_name=""`, everything else `NULL` |
| 31 | `parse_uname_string` | Unix branch, text after `": "` is empty (`uname == " [a: "`) → `strip_last_char(os_version)` on `""` writes at `os_version[-1]` — **UB** | `os_version=""` |
| 32 | `parse_uname_string` | Unix branch, text after `" ("` is empty (`uname == " [a: 1 ("`) → `strip_last_char(os_codename)` on `""` — **UB** | `os_codename=""` |
| 33 | `parse_uname_string` | Unix branch, `os_version` has no `" ("` | `os_codename` stays `NULL` |
| 34 | `parse_uname_string` | Unix branch, `os_name` has no `"\|"` | `os_platform` stays `NULL` (contrast row 27's `"windows"`) |
| 35 | `parse_uname_string` | Unix branch, `os_version` does not start with digits | `os_major`, `os_minor` stay `NULL` |
| 36 | `parse_uname_string` | Unix branch, `os_version` has a major but no `.minor` | `os_minor` stays `NULL`, `os_major` set |
| 37 | `parse_uname_string` | Unix branch: `os_arch` comes from `get_os_arch(uname)` **after** `uname` was truncated at `" ["` — an arch name that appears only *after* the `" ["` is invisible | `os_arch` stays `NULL` |
| 38 | `parse_uname_string` | `os_uname` is never assigned by any code path | `os_uname` always stays at its pre-call value |
| 39 | `parse_uname_string` | `" [Ver: "` present but *later* in the string than a `" ["` (e.g. `"a [b] c [Ver: 1.2.3]"`) | Ver branch wins (`strstr` for `" [Ver: "` is tested first, not leftmost-marker order) |
| 40 | `parse_uname_string` | `uname` is `""` (empty) | no marker, no arch → all fields `NULL`, buffer unchanged |
| 41 | `parse_uname_string` | Ver branch with a *negative-looking* version (`"-1.2.3"`) → all three regexes are anchored `^`, fail | major/minor/build all `NULL`, `os_version="-1.2.3"` |
| 42 | `parse_uname_string` | Ver branch, digits overflow `int` (`"99999999999999999999.1.2"`) | regexes still match textually; `match_size` is the *byte* length, so no overflow — `os_major` is the full digit run |
| 43 | FFI generic | out-of-range enum value | **N/A** — the public ABI (`include/lib.h` + the two non-`static` helpers) declares **no enum parameter**. `nmatch` is `size_t`, `pmatch` is a pointer, everything else is `char *`. Documented here so the row is not silently skipped. |
| 44 | FFI generic | oversized length: `w_regexec` with `nmatch = 4096` against a 4096-entry `pmatch` array | returns match result, entries `>re_nsub` all `{-1,-1}` |
| 45 | FFI generic | oversized input: 64 KiB `uname` / `os_header` | no length limit in the C; behaves as normal |
| 46 | FFI generic | `parse_uname_string` called twice on the same `os_data` (already-populated fields) | fields are *overwritten* (leak); previous pointers lost — same in both |

## Status — row → test mapping

Every row is covered by a test in `translation/tests/errors.rs`. Each test's
doc comment names the rows it covers.

| ERRORS row(s) | test in `tests/errors.rs` |
|---------------|---------------------------|
| 1, 3 | `err01_arch_returns_null_when_absent` |
| 2 | `err02_arch_null_pointer_faults_identically` (asserts SIGSEGV in both) |
| 4, 5, 6 | `err04_regexec_null_argument_guard` |
| 7, 8, 9, 10 | `err07_regexec_regcomp_failures`, `err07b_regexec_regcomp_stderr_text` |
| 11, 17 | `err11_regexec_no_match` |
| 12, 13 | `err12_regexec_nmatch_zero_is_safe` |
| 14 | `err14_regexec_null_pmatch_faults_identically` (asserts SIGSEGV in both) |
| 15, 16 | `err15_regexec_nmatch_vs_group_count` |
| 18 | `err18_regexec_empty_pattern_is_valid` |
| 19 | `err19_parse_null_osd_is_a_noop` |
| 20, 21 | `err20_parse_null_uname` (asserts row 20 faults, row 21 does not) |
| 22, 23, 40 | `err22_parse_no_marker` |
| 24, 25, 26 | `err24_parse_ver_regex_failures` |
| 27 | `err27_parse_ver_leaves_arch_null` |
| 28 | `err28_parse_ver_empty_tail_writes_into_buffer` |
| 29, 33, 34 | `err29_parse_missing_submarkers` |
| 30, 31, 32 | `err30_parse_empty_strdup_underflow` |
| 35, 36 | `err35_parse_unix_regex_failures` |
| 37 | `err37_parse_arch_hidden_after_truncation` |
| 38 | `err38_parse_never_touches_os_uname` |
| 39 | `err39_parse_ver_marker_wins` |
| 41, 42 | `err41_parse_numeric_edges` |
| 43, 44 | `err43_regexec_oversized_nmatch` |
| 45 | `err45_oversized_inputs` |
| 46 | `err46_parse_repeated_overwrites_fields` |
| generic boundaries | `err_generic_boundary_matrix` (every single byte 0x01–0xFF as input to all three entry points; every NULL-argument combination; every 2- and 3-byte string over the marker alphabet) |

Result: 27 tests, 27 passing, 0 failing — in the `debug` and `release`
profiles, under both feature combinations.

- [x] Every row has a passing error-path differential test.
