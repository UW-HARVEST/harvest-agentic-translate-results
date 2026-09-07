# CONFIGS.md — configuration / valid-input surface table (Phase B gate)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

**Compile-time options.** None. `c_src/CMakeLists.txt` defines no
`target_compile_definitions`; `grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c
c_src/include/lib.h` → no hits. Rust side: `Cargo.toml` has no `[features]`
table, so `default` is the only feature combination.

**Runtime options.** None in the usual sense — there is no options struct and
no flag argument. The "configuration" is carried entirely by *input shape*:

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| A1 entry point | `get_os_arch`, `w_regexec`, `parse_uname_string` | 3 non-`static` functions |
| A2 top-level marker | `" [Ver: "` present (Windows branch) / absent but `" ["` present (Unix branch) / neither | `parse_uname_string` outer `if/else` |
| A3 `": "` in `os_name` | present / absent | Unix branch inner `if/else` |
| A4 `" ("` in `os_version` | present / absent | codename sub-branch |
| A5 `"\|"` in `os_name` | present / absent | platform sub-branch |
| A6 major regex `^([0-9]+)\.*` | match / no match | 4 call sites (2 Ver, 2 Unix) |
| A7 minor regex `^[0-9]+\.([0-9]+)\.*` | match / no match | 2 branches |
| A8 build regex `^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*` | match / no match, 1 / 2 / N dotted components | Ver branch only |
| A9 arch token | each of the 12 `ARCHS` entries, in priority order; none; several at once; overlapping (`i386` vs `i686`, `amd64` vs `ia64`, `x86_64` vs `i86pc`, `aarch64` vs `arm64`) | `get_os_arch` loop |
| A10 arch position | before `" ["`, after `" ["`, absent | `get_os_arch(uname)` is called on the already-truncated buffer |
| A11 `nmatch` | 0, 1, 2, 3, 8, 4096 | passed straight to `regexec` |
| A12 `pmatch` | valid array / `NULL` | no null check |
| A13 group count of the pattern | 0, 1, 2, nested | interacts with A11 |
| A14 string length | empty, 1 char, typical, 64 KiB | no length limits |
| A15 `os_data` pre-state | zeroed / already populated | fields are blindly overwritten |
| A16 in-place mutation of the caller's buffer | truncation at each marker, `strip_last_char` writes | all branches |

## Row table (cross-product, pruned to combinations the C treats differently)

Each row is exercised with **many randomized inputs** (fixed seed `0x5EED_1234`,
xorshift64* PRNG) through **both** `.so` exports, comparing the return value,
all 9 `os_data` fields, the full `pmatch` array, the mutated input buffer
(including 8 guard bytes on each side), and the child's `stderr`.

### `get_os_arch` (low-level entry point, not in the header)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_os_arch` | exactly one arch token, randomized surrounding text/position — one row-instance per each of the 12 `ARCHS` entries | [x] |
| 2 | `get_os_arch` | two or more distinct arch tokens present → must return the *first in `ARCHS` order*, not the leftmost in the string | [x] |
| 3 | `get_os_arch` | overlapping tokens: `i386`/`i686`, `amd64`/`ia64`, `x86_64`/`i86pc`, `aarch64`/`arm64`, `armv6`/`armv7` | [x] |
| 4 | `get_os_arch` | arch token as a *substring* of a longer word (`xxx86_64yyy`, `Xarm64Y`) — still matches, `strstr` is not word-anchored | [x] |
| 5 | `get_os_arch` | arch token split across the string so it does *not* occur (`x86-64`, `arm_64`) → `NULL` | [x] |
| 6 | `get_os_arch` | random ASCII/high-byte noise, length 0..64, no arch → `NULL` | [x] |
| 7 | `get_os_arch` | realistic full uname lines (Linux/Darwin/AIX/SunOS/Windows) | [x] |
| 8 | `get_os_arch` | 64 KiB input with the arch at the very end | [x] |

### `w_regexec` (low-level entry point, not in the header)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 9 | `w_regexec` | the 3 *actual* patterns used by `parse_uname_string` × randomized version-like strings, `nmatch=2`, 2-entry `pmatch` | [x] |
| 10 | `w_regexec` | 0-group pattern (`^[0-9]+`), `nmatch=2` → `pmatch[1] = {-1,-1}` | [x] |
| 11 | `w_regexec` | 1-group pattern, `nmatch=1` → only `pmatch[0]` written, `pmatch[1]` keeps caller's sentinel | [x] |
| 12 | `w_regexec` | 2-group / nested-group pattern (`([0-9]+(\.[0-9]+)*)`), `nmatch=3` | [x] |
| 13 | `w_regexec` | `nmatch=0` with a non-`NULL` `pmatch` → array untouched, return still correct | [x] |
| 14 | `w_regexec` | `nmatch=8` with an 8-entry array on a 1-group pattern → surplus `{-1,-1}` | [x] |
| 15 | `w_regexec` | `nmatch=4096` with a 4096-entry array | [x] |
| 16 | `w_regexec` | anchored (`^…`) vs unanchored patterns, match at offset 0 vs deeper | [x] |
| 17 | `w_regexec` | alternation / character classes / `+` / `*` / `{m,n}` REG_EXTENDED constructs | [x] |
| 18 | `w_regexec` | empty pattern `""` against random strings | [x] |
| 19 | `w_regexec` | empty subject `""` against random patterns | [x] |
| 20 | `w_regexec` | subject containing NUL-free high bytes (0x80..0xFF) and `\n` | [x] |
| 21 | `w_regexec` | 64 KiB subject, match near the end (unanchored) | [x] |

### `parse_uname_string` (public header entry point)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 22 | `parse_uname_string` | **Ver branch**, `major.minor.build` (3 components), randomized digits | [x] |
| 23 | `parse_uname_string` | Ver branch, `major.minor.build.rev` (4+ dotted components → build regex's inner `(\.[0-9]+)*` repeats) | [x] |
| 24 | `parse_uname_string` | Ver branch, `major.minor` only → `os_build` `NULL` | [x] |
| 25 | `parse_uname_string` | Ver branch, `major` only → `os_minor`, `os_build` `NULL` | [x] |
| 26 | `parse_uname_string` | Ver branch, non-numeric version → all three `NULL`, `os_version` still set | [x] |
| 27 | `parse_uname_string` | Ver branch, leading zeros / very long digit runs (40 digits) | [x] |
| 28 | `parse_uname_string` | Ver branch, trailing extra text after the version (`"10.0.19041 SP1]"`) | [x] |
| 29 | `parse_uname_string` | Ver branch, arch token present in the prefix → `os_arch` must stay `NULL` (Ver branch never calls `get_os_arch`) | [x] |
| 30 | `parse_uname_string` | Ver branch, `" [Ver: "` occurring *after* a plain `" ["` → Ver branch still wins | [x] |
| 31 | `parse_uname_string` | Ver branch, two `" [Ver: "` occurrences → leftmost used | [x] |
| 32 | `parse_uname_string` | Ver branch, empty prefix (`" [Ver: 6.1.7601]"`) → `os_name = ""` | [x] |
| 33 | `parse_uname_string` | **Unix branch**, `name [os: major.minor (codename)]` — full shape, randomized | [x] |
| 34 | `parse_uname_string` | Unix branch, `name [os: major.minor]` — no codename → `os_codename` `NULL` | [x] |
| 35 | `parse_uname_string` | Unix branch, `name [os]` — no `": "` → `else` arm, only `os_name` (last char stripped) | [x] |
| 36 | `parse_uname_string` | Unix branch, `os_name` contains `"\|"` → `os_platform` from the tail; both with and without `": "` | [x] |
| 37 | `parse_uname_string` | Unix branch, `"\|"` present *and* codename present (all sub-branches at once) | [x] |
| 38 | `parse_uname_string` | Unix branch, version has major but no minor → `os_minor` `NULL` | [x] |
| 39 | `parse_uname_string` | Unix branch, non-numeric version → `os_major`/`os_minor` `NULL` | [x] |
| 40 | `parse_uname_string` | Unix branch, arch token *before* `" ["` → `os_arch` set | [x] |
| 41 | `parse_uname_string` | Unix branch, arch token *after* `" ["` only → `os_arch` `NULL` (buffer already truncated) | [x] |
| 42 | `parse_uname_string` | Unix branch, arch tokens on both sides → the pre-`" ["` one wins | [x] |
| 43 | `parse_uname_string` | Unix branch, multiple `": "` occurrences → leftmost splits | [x] |
| 44 | `parse_uname_string` | Unix branch, multiple `" ("` occurrences → leftmost splits | [x] |
| 45 | `parse_uname_string` | Unix branch, multiple `"\|"` occurrences → leftmost splits | [x] |
| 46 | `parse_uname_string` | Unix branch, multiple `" ["` occurrences → leftmost used | [x] |
| 47 | `parse_uname_string` | **Neither branch**, arch present → only `os_arch` set | [x] |
| 48 | `parse_uname_string` | Neither branch, no arch → all fields `NULL` | [x] |
| 49 | `parse_uname_string` | `"["` without the leading space (`"a[b: 1]"`) → neither branch | [x] |
| 50 | `parse_uname_string` | `" [Ver:"` without the trailing space → falls through to `" ["` branch | [x] |
| 51 | `parse_uname_string` | real-world corpus: Wazuh-style agent unames for Ubuntu/CentOS/Debian/Amazon Linux/macOS/Windows/AIX/Solaris/Alpine | [x] |
| 52 | `parse_uname_string` | 64 KiB uname, markers at random positions | [x] |
| 53 | `parse_uname_string` | high bytes (0x80..0xFF) and embedded `\n`/`\t` in every field | [x] |
| 54 | `parse_uname_string` | `os_data` pre-populated with sentinel pointers → confirms which fields are overwritten vs preserved (`os_uname` never touched) | [x] |
| 55 | `parse_uname_string` | called twice in a row on the same buffer + same `os_data` (second call sees the already-truncated buffer) | [x] |
| 56 | `parse_uname_string` | fully random byte soup (length 0..96) with the four marker substrings injected at random offsets — 4000 iterations | [x] |

### Driver binary

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 57 | binary executable | **N/A** — `c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/lib.c)`; there is no `add_executable`, and `translation/Cargo.toml` has no `[[bin]]` and no `src/main.rs`. No stdout comparison is possible or required. | [x] |

## Status — row → test mapping

Every row is covered by a test in `translation/tests/configs.rs` (the test name
carries the row number). Each test builds its cases with the seeded xorshift64\*
PRNG from `tests/common/mod.rs` and calls `assert_same`, which compares the C
and Rust `.so` byte-for-byte.

| CONFIGS row(s) | test in `tests/configs.rs` |
|----------------|-----------------------------|
| 1 | `cfg01_arch_single_token_each_arch` (12 archs × 200 randomized) |
| 2 | `cfg02_arch_multiple_tokens_priority_order` (800 randomized + all 144 ordered pairs) |
| 3 | `cfg03_arch_overlapping_tokens` |
| 4 | `cfg04_arch_token_inside_word` |
| 5 | `cfg05_arch_near_misses` |
| 6 | `cfg06_arch_random_noise` (2000 randomized) |
| 7 | `cfg07_arch_realistic_unames` |
| 8 | `cfg08_arch_huge_input` (64 KiB) |
| 9 | `cfg09_regexec_real_patterns` (800 randomized × 3 patterns) |
| 10 | `cfg10_regexec_zero_group_pattern` |
| 11 | `cfg11_regexec_nmatch_one` |
| 12 | `cfg12_regexec_nested_groups` |
| 13 | `cfg13_regexec_nmatch_zero` |
| 14 | `cfg14_regexec_nmatch_surplus` |
| 15 | `cfg15_regexec_nmatch_4096` |
| 16 | `cfg16_regexec_anchoring` |
| 17 | `cfg17_regexec_extended_constructs` |
| 18 | `cfg18_regexec_empty_pattern` |
| 19 | `cfg19_regexec_empty_subject` |
| 20 | `cfg20_regexec_high_bytes` |
| 21 | `cfg21_regexec_huge_subject` (64 KiB) |
| 22 | `cfg22_parse_ver_three_components` (600 randomized) |
| 23 | `cfg23_parse_ver_many_components` (4–9 components × 200) |
| 24 | `cfg24_parse_ver_two_components` |
| 25 | `cfg25_parse_ver_one_component` |
| 26 | `cfg26_parse_ver_non_numeric` |
| 27 | `cfg27_parse_ver_long_digit_runs` (up to 400 digits) |
| 28 | `cfg28_parse_ver_trailing_text` |
| 29 | `cfg29_parse_ver_never_sets_arch` |
| 30, 31 | `cfg30_parse_marker_precedence` |
| 32 | `cfg32_parse_ver_empty_prefix` |
| 33 | `cfg33_parse_unix_full` (800 randomized) |
| 34 | `cfg34_parse_unix_no_codename` (800 randomized) |
| 35 | `cfg35_parse_unix_no_colon_space` |
| 36, 37 | `cfg36_parse_unix_pipe_platform` |
| 38, 39 | `cfg38_parse_unix_version_shapes` |
| 40, 41, 42 | `cfg40_parse_unix_arch_position` (12 × 12 arch cross-product) |
| 43, 44, 45, 46 | `cfg43_parse_unix_duplicate_markers` |
| 47, 48, 49, 50 | `cfg47_parse_neither_branch` |
| 51 | `cfg51_parse_realistic_corpus` |
| 52 | `cfg52_parse_huge_input` (64 KiB, marker at 4 different positions) |
| 53 | `cfg53_parse_high_bytes` |
| 54 | `cfg54_parse_prepopulated_osdata` |
| 55 | `cfg55_parse_repeated_calls` |
| 56 | `cfg56_parse_random_soup` (4000), `cfg56b_parse_marker_alphabet_soup` (4000) |
| 57 | N/A — no binary target exists (see the row) |

Result: 46 tests, 46 passing, 0 failing — in the `debug` and `release`
profiles, under both feature combinations.

### What is compared per case

* `get_os_arch`: returned string (or NULL) **and** the input buffer plus its
  8-byte guard padding on each side (`get_os_arch` must not mutate it).
* `w_regexec`: the `int` return value **and** every `regmatch_t` entry of the
  caller's array (pre-filled with `{-7777, -8888}` so "untouched" is
  distinguishable from "written"), **and** the child's `stderr`.
* `parse_uname_string`: all 9 `os_data` fields (NULL vs contents) **and** the
  full mutated input buffer including both guard regions, so in-place
  truncations and the one-byte-back `strip_last_char` writes are observed.
* Always: the child's exit status / fatal signal.

- [x] Every row passes across randomized inputs (see `tests/configs.rs`).
