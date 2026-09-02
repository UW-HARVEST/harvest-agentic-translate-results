# CONFIGS.md — configuration / valid-input surface table

## Axis inventory (mechanically derived from `c_src/src/lib.c`)

**Runtime options / modes / flags:** none. The library has no init function, no
global state, no setters, and no flag parameters. `regcomp` is always called
with `REG_EXTENDED` and `regexec` always with `eflags == 0` (both hard-coded at
L40/L45). There are no `#ifdef`s. The *only* configuration axes are input
shapes, so they are enumerated exhaustively below.

**Public entry points (all three, including the low-level ones):**

| entry point | declared in header? | level |
|---|---|---|
| `parse_uname_string(char *uname, os_data *osd)` | yes | top-level composed pipeline |
| `get_os_arch(char *os_header)` | no (exported anyway) | **low-level**, called by `parse_uname_string` L142 |
| `w_regexec(const char*, const char*, size_t, regmatch_t*)` | no (exported anyway) | **low-level**, called by `parse_uname_string` L75/L82/L89/L117/L124 |

**Branch axes in `parse_uname_string`:**

| axis | source | values |
|---|---|---|
| A1 marker | L68 `strstr(uname, " [Ver: ")` | present → Windows branch / absent |
| A2 marker | L98 `strstr(uname, " [")` | present → Unix branch / absent → arch-only |
| A3 separator | L102 `strstr(os_name, ": ")` | present / absent |
| A4 codename | L109 `strstr(os_version, " (")` | present / absent |
| A5 platform | L135 `strstr(os_name, "\|")` | present-before-`": "` / present-after / absent |
| A6 version shape | L75/L82/L89/L117/L124 regexes | none · `M` · `M.m` · `M.m.b` · `M.m.b.r…` · leading zeros · huge digit runs · digits-then-text |
| A7 architecture | L18 `ARCHS[]` | each of the 12 literals · none · several (ARCHS-order precedence, **not** string-order) · before vs. after `" ["` |
| A8 length/emptiness | L72/L105/L112/L131 `*(p+strlen(p)-1)` | non-empty / 1 char / empty (the OOB-strip shapes) |
| A9 multiplicity | `strstr` returns the **first** occurrence | 1 vs. many occurrences of each marker |

**Branch axes in `get_os_arch`:** position of the literal in the haystack; which
of the 12 literals; overlapping literals (`i386` ⊂ `x86_64`? no, but `arm64` ⊂
`aarch64`? no — however `i386`/`i686`/`i86pc` and `armv6`/`armv7`/`arm64`/
`aarch64` and `x86_64`/`amd64`/`ia64` all share substrings, so precedence
matters); embedded in a longer token; count of literals present.

**Branch axes in `w_regexec`:** `nmatch` (0/1/2/3/oversized); `pmatch` NULL when
`nmatch == 0`; match vs. no-match; group participation; anchored vs. floating;
ERE metacharacters; subject length 0/1/many; the five literal patterns the
library itself uses.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed, see
`tests/common/mod.rs::Rng`), both `.so`s loaded via `libloading`, all 9
`os_data` fields plus the mutated `uname` buffer compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `get_os_arch` | haystack containing exactly one of the 12 ARCHS literals, at a random position, with random surrounding noise (all 12 literals covered) | `c1_arch_single` | [x] |
| C2 | `get_os_arch` | haystack containing 2–5 ARCHS literals in random string-order → must return the ARCHS-**array**-order winner, not the leftmost | `c2_arch_precedence` | [x] |
| C3 | `get_os_arch` | literal embedded inside a longer token (`"xx86_64yy"`, `"aarch64le"`, `"arm64e"`) — `strstr` is a plain substring search, so it still matches | `c3_arch_embedded` | [x] |
| C4 | `get_os_arch` | random ASCII/hi-byte noise with no literal, plus near-misses (`"x86-64"`, `"i38"`, `"ARM64"`, `"aix"`) | `c4_arch_noise` | [x] |
| C5 | `get_os_arch` | boundary lengths: `""`, 1 char, literal at offset 0, literal at the very end, 4 KiB haystack | `c5_arch_lengths` | [x] |
| C6 | `w_regexec` | the 5 patterns `parse_uname_string` actually uses, against randomized version-like subjects, `nmatch=2`, comparing return value **and** both `regmatch_t` slots | `c6_regexec_library_patterns` | [x] |
| C7 | `w_regexec` | `nmatch ∈ {0,1,2,3,8}` × (match, no-match), `pmatch` buffer pre-poisoned with a sentinel so untouched slots are detectable | `c7_regexec_nmatch_matrix` | [x] |
| C8 | `w_regexec` | ERE feature matrix: alternation, `+ * ? {n,m}`, character classes, `[[:digit:]]`, anchors `^ $`, nested groups, back-to-back groups — random subjects | `c8_regexec_ere_features` | [x] |
| C9 | `w_regexec` | subject length 0 / 1 / long (4 KiB); pattern length 0 / 1 / long; NUL-terminated exactly | `c9_regexec_lengths` | [x] |
| C10 | `parse_uname_string` | **Windows branch**, version = `M` only (random 1–6 digit major) | `c10_win_major_only` | [x] |
| C11 | `parse_uname_string` | **Windows branch**, version = `M.m` | `c11_win_major_minor` | [x] |
| C12 | `parse_uname_string` | **Windows branch**, version = `M.m.b` (the canonical real-world shape) | `c12_win_major_minor_build` | [x] |
| C13 | `parse_uname_string` | **Windows branch**, version = `M.m.b.r` and `M.m.b.r.s` → build regex captures the multi-dot tail | `c13_win_multidot_build` | [x] |
| C14 | `parse_uname_string` | **Windows branch**, version with leading zeros / very long digit runs / trailing dots (`"0006.0001."`) | `c14_win_odd_numbers` | [x] |
| C15 | `parse_uname_string` | **Windows branch**, version starting with non-digits (`"abc 1.2"`) → anchored regexes all fail | `c15_win_leading_text` | [x] |
| C16 | `parse_uname_string` | **Windows branch**, `os_name` part empty / random / containing `"\|"` and `": "` (which are *not* looked for in this branch) | `c16_win_name_shapes` | [x] |
| C17 | `parse_uname_string` | **Windows branch**, `" [Ver: "` occurring 2–3 times → first occurrence wins | `c17_win_multiple_markers` | [x] |
| C18 | `parse_uname_string` | **Windows branch** where the string *also* contains `" ["` and an ARCHS literal → `" [Ver: "` wins and arch is never probed | `c18_win_shadows_unix` | [x] |
| C19 | `parse_uname_string` | **Windows branch**, version remainder of length 1 (strip yields `""`) and length 0 (strip writes at `str_tmp-1`) | `c19_win_short_version` | [x] |
| C20 | `parse_uname_string` | **Unix branch**, full shape `"<name> [<dist>: <ver> (<codename>)]"` with randomized parts | `c20_unix_full` | [x] |
| C21 | `parse_uname_string` | **Unix branch**, `": "` present, no `" ("` → no codename | `c21_unix_no_codename` | [x] |
| C22 | `parse_uname_string` | **Unix branch**, no `": "` → `os_name` loses its last byte, nothing else set | `c22_unix_no_colon` | [x] |
| C23 | `parse_uname_string` | **Unix branch**, `"\|"` before `": "` → `os_platform` = text after `"\|"`, `os_name` = text before | `c23_unix_pipe_platform` | [x] |
| C24 | `parse_uname_string` | **Unix branch**, `"\|"` present *and* no `": "` → pipe search runs on the last-byte-stripped name | `c24_unix_pipe_no_colon` | [x] |
| C25 | `parse_uname_string` | **Unix branch**, multiple `"\|"` → first wins; `"\|"` at position 0 → empty `os_name` | `c25_unix_pipe_shapes` | [x] |
| C26 | `parse_uname_string` | **Unix branch**, version shapes `M`, `M.m`, `M.m.p`, `M-suffix`, non-numeric (note: **no build regex** in this branch) | `c26_unix_version_shapes` | [x] |
| C27 | `parse_uname_string` | **Unix branch**, multiple `" ("` in the version → first wins; codename containing `": "`, `"\|"`, `"("`, `")"` | `c27_unix_codename_shapes` | [x] |
| C28 | `parse_uname_string` | **Unix branch**, multiple `": "` → first wins (later ones stay inside `os_version`) | `c28_unix_multiple_colons` | [x] |
| C29 | `parse_uname_string` | **Unix branch**, multiple `" ["` → first wins | `c29_unix_multiple_brackets` | [x] |
| C30 | `parse_uname_string` | **Unix branch** × each of the 12 ARCHS literals placed in the prefix (before `" ["`) → `os_arch` set | `c30_unix_arch_each` | [x] |
| C31 | `parse_uname_string` | **Unix branch**, several ARCHS literals in the prefix → ARCHS-order precedence through the composed pipeline | `c31_unix_arch_precedence` | [x] |
| C32 | `parse_uname_string` | **Unix branch**, ARCHS literal straddling the `" ["` boundary (partially truncated) | `c32_unix_arch_straddle` | [x] |
| C33 | `parse_uname_string` | **arch-only branch** (no `" ["` at all) × each ARCHS literal → only `os_arch` written | `c33_archonly_each` | [x] |
| C34 | `parse_uname_string` | **arch-only branch**, `" ["` -like near-misses (`"["`, `" [ "` with no closing, `"[Ver: "` without the leading space, `" [Ver:"` without the trailing space) | `c34_archonly_near_misses` | [x] |
| C35 | `parse_uname_string` | pre-poisoned (non-NULL, non-zero) `os_data` for every branch → proves exactly which fields the C leaves untouched | `c35_poisoned_struct` | [x] |
| C36 | `parse_uname_string` | fully random byte soup (printable + `[`, `]`, `(`, `)`, `:`, `\|`, space, digits, arch fragments) — 20 000 seeded cases, any branch | `c36_fuzz_mixed` | [x] |
| C37 | `parse_uname_string` | random *real-world corpus* shapes (Windows/Ubuntu/CentOS/Debian/macOS/AIX/Solaris uname strings) with randomized numbers | `c37_realworld_corpus` | [x] |
| C38 | `parse_uname_string` | composed low-level interaction: the same `regmatch_t` array is reused across the 3 (Windows) / 2 (Unix) `w_regexec` calls — inputs where an earlier regex matches and a later one fails, so stale offsets are live | `c38_pmatch_reuse_pipeline` | [x] |
| C39 | `parse_uname_string` | long inputs: 4 KiB and 64 KiB uname strings in each branch (offset arithmetic > `i16`, and `regoff_t` sanity) | `c39_long_inputs` | [x] |
| C40 | `parse_uname_string` | high-bit / non-UTF-8 bytes (`0x80`–`0xFF`) in name, version, codename and platform parts | `c40_non_utf8_bytes` | [x] |

## Feature combinations

There is no `[features]` table in `Cargo.toml` and no `cfg` in the source, so
the default configuration is the complete matrix. `scripts/feature_matrix.sh`
verifies this mechanically and runs the whole suite for every combination it
finds (which is exactly one: the default).
