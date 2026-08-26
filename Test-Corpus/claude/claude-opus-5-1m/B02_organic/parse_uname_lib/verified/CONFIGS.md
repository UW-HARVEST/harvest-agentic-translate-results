# CONFIGS.md — Configuration-surface table (Phase A, tested in Phase B)

The library has **no build-time configuration** (no `[features]` in
`Cargo.toml`, no `option()`/`#ifdef` in `c_src/`). Its entire configuration
surface is therefore *runtime*: the shape of the input strings, and the
parameters of the low-level entry points. Those axes are enumerated below
straight from the `if`/`for`/`strstr` branches in `c_src/src/lib.c`.

## Axes the C code actually branches on

**A. Entry point** (all three exported symbols, including the low-level ones —
`get_os_arch` and `w_regexec` are *not* in `lib.h` but are exported and are the
building blocks `parse_uname_string` composes):

* `get_os_arch(char*)`
* `w_regexec(const char*, const char*, size_t, regmatch_t*)`
* `parse_uname_string(char*, os_data*)`

**B. `parse_uname_string` top-level branch** (`lib.c:68` vs `lib.c:98`):

* B1 `" [Ver: "` present → Windows branch (sets `os_platform="windows"`, never `os_arch`)
* B2 no `" [Ver: "`, `" ["` present → Unix branch
* B3 neither present → arch-only

**C. Sub-branches inside B2** (independent, cross-producted):

* C1 `": "` present in bracket body (`lib.c:102`) — yes/no
* C2 `" ("` present in version (`lib.c:109`) — yes/no (only reachable when C1)
* C3 `"|"` present in `os_name` (`lib.c:135`) — yes/no
* C4 arch substring present in the truncated prefix (`lib.c:142`) — yes/no

**D. Version-string shape** (drives the three regexes at `lib.c:75/82/89` and
`lib.c:117/124`):

* D1 non-numeric (no match at all)
* D2 major only
* D3 major.minor
* D4 major.minor.build
* D5 major.minor.build.rev… (multi-component build — only the B1 regex has the `(\.[0-9]+)*` group)
* D6 multi-digit / leading-zero / very long numeric components

**E. `get_os_arch` table position & precedence** (`lib.c:18`, `strstr`, first
hit in table order wins): each of the 12 entries, plus multi-arch strings where
table order — not string position — decides, plus overlapping names
(`i386`/`i686`, `ia64`/`amd64`, `aarch64`/`arm64`, `armv6`/`armv7`).

**F. `w_regexec` parameters**: `nmatch` ∈ {0,1,2,3,8,64}, `pmatch` NULL/non-NULL,
group count 0/1/2/nested, anchored/unanchored, matching/non-matching.

**G. Buffer/`os_data` state**: zeroed vs `0xAA`-poisoned `os_data`; short vs
4 KiB `uname`; guard-padded `uname` buffer so the in-place mutations *and* the
one-byte-before-the-string write at `lib.c:72` are compared byte-for-byte.

## Rows (pruned cross-product — one row per combination the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed seed, see
`tests/common/mod.rs::Rng`), not one hand-picked value, and asserts
byte-identical results from the C `.so` and the Rust `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `get_os_arch` | each of the 12 table entries alone, embedded at a random position in random filler | [x] |
| 2  | `get_os_arch` | arch at the very start of the string / at the very end / as the whole string | [x] |
| 3  | `get_os_arch` | two or more archs present — table order must win over string position (all 132 ordered pairs) | [x] |
| 4  | `get_os_arch` | overlapping/confusable pairs: `i386`+`i686`, `amd64`+`ia64`, `aarch64`+`arm64`, `armv6`+`armv7`, `x86_64`+`amd64` | [x] |
| 5  | `get_os_arch` | random binary-ish filler with no arch, lengths 0…4096 | [x] |
| 6  | `get_os_arch` | arch name split across a random NUL-free boundary so only a prefix appears (`x86_6`, `aarch6`, …) | [x] |
| 7  | `w_regexec` | the 5 patterns actually used by `parse_uname_string`, over randomized version strings, `nmatch=2` | [x] |
| 8  | `w_regexec` | `nmatch` swept over {0,1,2,3,8,64} with a 64-entry buffer; full buffer compared | [x] |
| 9  | `w_regexec` | 0-group / 1-group / 2-group / nested-group patterns, matching and non-matching | [x] |
| 10 | `w_regexec` | unanchored patterns matching in the middle of a long (4 KiB) subject | [x] |
| 11 | `w_regexec` | non-participating group (`^(a)?b`) — `{-1,-1}` propagation | [x] |
| 12 | `w_regexec` | alternation where which group participates depends on the value (`^(a+)\|(b+)$`) | [x] |
| 13 | `parse_uname_string` | **B1** `" [Ver: "` × **D1** non-numeric version | [x] |
| 14 | `parse_uname_string` | **B1** × **D2** major only | [x] |
| 15 | `parse_uname_string` | **B1** × **D3** major.minor | [x] |
| 16 | `parse_uname_string` | **B1** × **D4** major.minor.build | [x] |
| 17 | `parse_uname_string` | **B1** × **D5** major.minor.build.rev (2–5 components) | [x] |
| 18 | `parse_uname_string` | **B1** × **D6** multi-digit / leading-zero / 40-digit components | [x] |
| 19 | `parse_uname_string` | **B1** with arch text also present — `os_arch` must stay `NULL` | [x] |
| 20 | `parse_uname_string` | **B1** with a `"\|"` in the name part — Windows branch ignores it, `os_platform` is `"windows"` | [x] |
| 21 | `parse_uname_string` | **B1** where `" [Ver: "` occurs twice (first hit wins) | [x] |
| 22 | `parse_uname_string` | **B1** where both `" [Ver: "` and `" ["` occur, in both orders (Ver always wins) | [x] |
| 23 | `parse_uname_string` | **B1** with no trailing `"]"` (last real byte gets eaten instead) | [x] |
| 24 | `parse_uname_string` | **B2** × C1=yes, C2=yes, C3=no × D3 — `name [os: 1.2 (code)]` | [x] |
| 25 | `parse_uname_string` | **B2** × C1=yes, C2=no,  C3=no × D3 — `name [os: 1.2]` | [x] |
| 26 | `parse_uname_string` | **B2** × C1=yes, C2=yes, C3=yes × D3 — `name [os\|plat: 1.2 (code)]` | [x] |
| 27 | `parse_uname_string` | **B2** × C1=yes, C2=no,  C3=yes × D3 — `name [os\|plat: 1.2]` | [x] |
| 28 | `parse_uname_string` | **B2** × C1=no,  C3=no — `name [os]` (last byte of `os_name` eaten) | [x] |
| 29 | `parse_uname_string` | **B2** × C1=no,  C3=yes — `name [os\|plat]` (truncation happens *before* the `\|` split) | [x] |
| 30 | `parse_uname_string` | **B2** × C1=yes × D1 non-numeric version (`rolling`, `unstable`) | [x] |
| 31 | `parse_uname_string` | **B2** × C1=yes × D2 major only | [x] |
| 32 | `parse_uname_string` | **B2** × C1=yes × D4/D5 — extra components ignored by the 2 Unix regexes | [x] |
| 33 | `parse_uname_string` | **B2** × C4=yes — arch in the prefix, all 12 archs × C1/C2/C3 variants | [x] |
| 34 | `parse_uname_string` | **B2** × C4=yes but arch only *after* the `" ["` — must **not** be found (prefix was truncated) | [x] |
| 35 | `parse_uname_string` | **B2** with multiple `" ["` (first wins) and multiple `": "`/`" ("`/`"\|"` (first wins each) | [x] |
| 36 | `parse_uname_string` | **B2** where `": "` appears *inside* the codename part | [x] |
| 37 | `parse_uname_string` | **B3** neither marker — arch present (all 12) and absent | [x] |
| 38 | `parse_uname_string` | fully randomized `uname` built from an alphabet that makes every marker (`" ["`, `" [Ver: "`, `": "`, `" ("`, `"\|"`, `"]"`, arch names, digits, dots) appear with high probability — 20 000 cases | [x] |
| 39 | `parse_uname_string` | **G**: `os_data` pre-filled with `0xAA` instead of zeros (untouched-member fidelity) | [x] |
| 40 | `parse_uname_string` | **G**: 4 KiB `uname` with markers at random offsets; guard-padded buffer, all bytes compared | [x] |
| 41 | `parse_uname_string` | real-world corpus: Windows/Linux/macOS/AIX/Solaris/BSD uname strings from the upstream project's shape | [x] |
| 42 | composed | `parse_uname_string` immediately followed by `get_os_arch` on the *already mutated* buffer, and `w_regexec` on the produced `os_version` — pipeline state carried across entry points | [x] |
| 43 | `parse_uname_string` | deep byte-fuzz over the marker alphabet, 0x00 and 0xAA `os_data` (1 000 000 cases at `HARVEST_FUZZ_ITERS=1000000`) | [x] |
| 44 | `parse_uname_string` | deep token-fuzz: random concatenations of the exact markers, arch names and version literals | [x] |
| 45 | composed | deep pipeline fuzz (`parse_uname_string` → `get_os_arch` → `w_regexec`) over the token soup | [x] |
| 46 | `get_os_arch` | deep fuzz incl. arbitrary non-NUL bytes 0x01-0xFF | [x] |
| 47 | `w_regexec` | deep fuzz over production + generated patterns × random subjects × `nmatch` ∈ {0,1,2,3,5,9,17} | [x] |

## Row → test mapping

| rows | test file :: test fn |
|------|----------------------|
| 1 | `tests/phase_b_get_os_arch.rs::row01_each_arch_at_random_position` |
| 2 | `…::row02_arch_at_boundaries` |
| 3 | `…::row03_table_order_beats_string_position`, `…::row03b_many_archs_at_once` |
| 4 | `…::row04_overlapping_confusable_pairs` |
| 5 | `…::row05_no_arch_random_lengths`, `…::row05b_arch_in_long_string` |
| 6 | `…::row06_arch_prefixes_only`, `…::row06b_biased_byte_fuzz` |
| 7 | `tests/phase_b_w_regexec.rs::row07_production_patterns_over_random_versions` |
| 8 | `…::row08_nmatch_sweep` |
| 9 | `…::row09_group_arities` |
| 10 | `…::row10_long_subjects` |
| 11 | `…::row11_non_participating_groups` |
| 12 | `…::row12_value_dependent_group_choice`, `…::row12b_pattern_and_subject_fuzz` |
| 13-18 | `tests/phase_b_parse_uname.rs::row13…row18_windows_*` |
| 19 | `…::row19_windows_ignores_arch` |
| 20 | `…::row20_windows_pipe_in_name` |
| 21 | `…::row21_windows_marker_twice` |
| 22 | `…::row22_both_markers_ver_wins` |
| 23 | `…::row23_windows_unterminated` |
| 24-29 | `…::row24…row29_unix_*` |
| 30-32 | `…::row30to32_unix_version_shapes` |
| 33 | `…::row33_unix_arch_in_prefix` |
| 34 | `…::row34_arch_only_after_bracket` |
| 35 | `…::row35_repeated_markers` |
| 36 | `…::row36_colon_inside_codename` |
| 37 | `…::row37_no_markers` |
| 38 | `…::row38_marker_rich_fuzz`, `…::row38b_token_fuzz` |
| 39 | `…::row39_poisoned_os_data` (also folded into every other row via `both_poisons`) |
| 40 | `…::row40_large_uname` |
| 41 | `…::row41_real_world_corpus` |
| 42 | `…::row42_composed_pipeline` |
| 43-47 | `tests/phase_b_deep_fuzz.rs::deep_fuzz_*` |

## What "byte-identical" means here

For every case, `tests/common/mod.rs` compares:

1. the return value / `os_data` member set (`NULL`-vs-set is distinguished from
   `""`-vs-`NULL` by poisoning the struct with `0xAA` as well as `0x00`);
2. the **contents** of every produced heap string;
3. every byte of the guard-padded `uname` buffer, so the in-place mutations and
   the one-byte-before-the-string write at `lib.c:72` are covered;
4. the full `pmatch` array including the slots `regexec` may leave alone;
5. for the `regcomp` failure path, the exact `stderr` text
   (`tests/phase_c_stderr.rs`).

## Verified configurations

| axis | values verified |
|------|-----------------|
| feature combination | the single empty/default one (`Cargo.toml` has no `[features]`) |
| cargo profile | `dev` (overflow-checks + debug-assertions on) and `release` (`panic = "abort"`, optimised) |
| C build | `cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON` (the only configuration `c_src/CMakeLists.txt` offers) |
