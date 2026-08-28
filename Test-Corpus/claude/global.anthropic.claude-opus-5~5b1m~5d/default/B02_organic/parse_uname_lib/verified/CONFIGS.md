# CONFIGS.md — Configuration surface table (Phase A)

Mechanically derived from the branches the C actually takes.

## Public entry points (all three, lowest-level first)

| entry point | signature (`c_src`) | level |
|-------------|---------------------|-------|
| `get_os_arch`        | `char *get_os_arch(char *os_header)` (`lib.c:17`) | lowest — pure lookup, no dependencies |
| `w_regexec`          | `int w_regexec(const char *pattern, const char *string, size_t nmatch, regmatch_t *pmatch)` (`lib.c:32`) | lowest — thin POSIX-regex wrapper |
| `parse_uname_string` | `void parse_uname_string(char *uname, os_data *osd)` (`lib.h:13`, `lib.c:57`) | composed pipeline: calls both of the above 0–4 times |

`parse_uname_string` is the only header-declared function, but `get_os_arch` and
`w_regexec` are exported and are therefore part of the tested surface and are
driven **directly**, not only through the wrapper.

## Branch axes the C distinguishes

| axis | values the C code distinguishes | source |
|------|---------------------------------|--------|
| `X` — top-level branch selector | `" [Ver: "` found (Windows path) / not found (POSIX path). Note `" [Ver: "` *contains* `" ["`, so the Windows path shadows the POSIX path. | `lib.c:68` vs `lib.c:97-98` |
| `V` — `[Ver: …]` payload shape | empty / non-numeric / `M` / `M.m` / `M.m.b` / `M.m.b.r` / `M.m.b.r.s` / leading zeros / huge ints / digits+junk / junk+digits / trailing dots | `lib.c:75,82,89` (3 regexes) |
| `B` — `" ["` present (POSIX path) | yes / no | `lib.c:98` |
| `C` — `": "` present in the post-`" ["` text | yes (version path) / no (trim-last-byte path) | `lib.c:102` vs `lib.c:130-132` |
| `K` — `" ("` present in the version text | yes (codename extracted) / no | `lib.c:109` |
| `P` — `"\|"` present in `os_name` | yes (`os_platform` from suffix) / no | `lib.c:135` |
| `A` — architecture token in the (truncated) `uname` | none / each of the 12 / several (ARCHS-order precedence, **not** string order) / near-miss substrings | `lib.c:18-27`, called only at `lib.c:142` |
| `N` — `nmatch` | 0 / 1 / 2 / 3 / 8 / 64 | `lib.c:45` |
| `G` — capture-group count/nesting in the pattern | 0 / 1 / 2 / nested / non-participating | `lib.c:45` |
| `O` — match offset | at 0 (anchored) / > 0 (unanchored) | `lib.c:45` |
| `S` — input shape | empty / 1 byte / separator-only / typical / long (≥ 512 B) / non-ASCII high bytes / embedded separators repeated | `strstr`/`strlen` throughout |
| `D` — caller's pre-state of `os_data` | all-`NULL` / pre-filled with non-null garbage (the C **never** zeroes `osd`, so untouched fields must survive verbatim) | absence of any init in `lib.c:57-66` |

Every row is checked with **many randomized inputs** (fixed seed `0x5EED_1234`,
xorshift64* PRNG), comparing, byte-for-byte: the return value, every one of the
9 `os_data` pointer fields (NULL-ness *and* full string bytes), the `regmatch_t`
array contents, **and the caller's mutated `uname` buffer**.

## Table

| #  | entry point(s) | configuration (options set + input shape) | test | [ ] |
|----|----------------|--------------------------------------------|------|-----|
| C1  | `get_os_arch` | `A` = each of the 12 ARCHS tokens, alone (`S`=exact) | `c1_arch_each_alone` | [x] |
| C2  | `get_os_arch` | `A` = one token embedded at start / middle / end of a random filler string | `c2_arch_embedded_positions` | [x] |
| C3  | `get_os_arch` | `A` = 2–4 tokens present at once → ARCHS-array order decides, not string order (e.g. `"aarch64 x86_64"` → `x86_64`) | `c3_arch_precedence` | [x] |
| C4  | `get_os_arch` | `A` = near-miss substrings only (`x86`, `86_64`, `I386`, `i38`, `sparc64`⊃`sparc`, `amd6`, `arm`, `armv8`, `aarch6`, `arm6`, `ia6`, `aix`, `i86p`) | `c4_arch_near_misses` | [x] |
| C5  | `get_os_arch` | `S` = fully random byte strings (printable + high bytes), 0–128 B, 3000 iterations | `c5_arch_random_fuzz` | [x] |
| C6  | `w_regexec` | the 3 hard-coded parser patterns × random version-like subjects × `N`=2 | `c6_regexec_parser_patterns` | [x] |
| C7  | `w_regexec` | `G`=0 groups, `N` ∈ {0,1,2,3,8,64}, `O`=0 and `O`>0 | `c7_regexec_no_groups_nmatch_sweep` | [x] |
| C8  | `w_regexec` | `G`=1..3 groups + nested groups, `N` ∈ {1,2,3,8,64} — surplus slots must be `{-1,-1}` | `c8_regexec_groups_nmatch_sweep` | [x] |
| C9  | `w_regexec` | `O`>0 unanchored patterns, random subjects — `rm_so`/`rm_eo` offsets must agree | `c9_regexec_unanchored_offsets` | [x] |
| C10 | `w_regexec` | ERE feature matrix: alternation, `{n,m}`, `[[:digit:]]`, `[^…]`, `.`, `$`, `\|`, escapes, `+`/`?`/`*`, long 512 B subjects | `c10_regexec_ere_features` | [x] |
| C11 | `w_regexec` | random *pattern* fuzz (valid and invalid mixed) × random subject, `N`=4 | `c11_regexec_random_pattern_fuzz` | [x] |
| C12 | `parse_uname_string` | `X`=Ver, `V`=`M` only, random majors incl. 0/leading zeros/huge | `c12_ver_major_only` | [x] |
| C13 | `parse_uname_string` | `X`=Ver, `V`=`M.m`, random | `c13_ver_major_minor` | [x] |
| C14 | `parse_uname_string` | `X`=Ver, `V`=`M.m.b`, random | `c14_ver_major_minor_build` | [x] |
| C15 | `parse_uname_string` | `X`=Ver, `V`=`M.m.b.r[.s]` — multi-dot build via the `(\.[0-9]+)*` group, 2–5 components | `c15_ver_multidot_build` | [x] |
| C16 | `parse_uname_string` | `X`=Ver, `V`= digits followed by junk / junk followed by digits / trailing dots | `c16_ver_mixed_junk` | [x] |
| C17 | `parse_uname_string` | `X`=Ver, prefix (→`os_name`) empty / random / contains `"\|"` / contains an arch token (arch must **not** be extracted on this path) | `c17_ver_prefix_shapes` | [x] |
| C18 | `parse_uname_string` | `X`=Ver, `" [Ver: "` occurs 2–3 times → first occurrence wins | `c18_ver_repeated_marker` | [x] |
| C19 | `parse_uname_string` | `X`=Ver where the payload itself contains `" ["`, `": "`, `" ("`, `"\|"` (must stay on the Windows path and ignore them) | `c19_ver_payload_contains_posix_separators` | [x] |
| C20 | `parse_uname_string` | POSIX path `B`=y, `C`=y, `K`=n, `P`=n, `A`=n — plain `"host [Distro: 1.2]"` | `c20_posix_plain` | [x] |
| C21 | `parse_uname_string` | POSIX `B`=y, `C`=y, `K`=**y**, `P`=n — codename extracted | `c21_posix_codename` | [x] |
| C22 | `parse_uname_string` | POSIX `B`=y, `C`=y, `K`=n, `P`=**y** — `os_name\|platform` split | `c22_posix_pipe` | [x] |
| C23 | `parse_uname_string` | POSIX `B`=y, `C`=y, `K`=y, `P`=y — full cross-product of codename + pipe | `c23_posix_codename_and_pipe` | [x] |
| C24 | `parse_uname_string` | POSIX `B`=y, `C`=**n** (no `": "`) × `P` ∈ {y,n} — `lib.c:131` trim path | `c24_posix_no_colon_pipe_cross` | [x] |
| C25 | `parse_uname_string` | POSIX `B`=y, `A`=**y**: each of the 12 arch tokens in the prefix, plus arch appearing only *after* `" ["` (→ must NOT be found, prefix is truncated) | `c25_posix_arch_each_and_after_bracket` | [x] |
| C26 | `parse_uname_string` | POSIX `B`=**n** (no `" ["` at all) × `A` ∈ {y,n} — only `os_arch` may be written | `c26_posix_no_bracket_arch_cross` | [x] |
| C27 | `parse_uname_string` | POSIX, `"\|"` located *after* the `": "` split point → must NOT be found (os_name already truncated) | `c27_posix_pipe_after_colon` | [x] |
| C28 | `parse_uname_string` | POSIX, multiple `" ["` / multiple `": "` / multiple `" ("` → first occurrence of each wins | `c28_posix_repeated_separators` | [x] |
| C29 | `parse_uname_string` | POSIX, version `M`-only and `M.m.p.q` shapes → `os_major`/`os_minor` regex behaviour (**no** `os_build` on this path) | `c29_posix_version_shapes` | [x] |
| C30 | `parse_uname_string` | `D` = `os_data` pre-filled with non-null sentinel pointers, over `X` ∈ {Ver, POSIX-full, POSIX-no-bracket} → untouched fields must keep the caller's values | `c30_prefilled_osdata` | [x] |
| C31 | `parse_uname_string` | `S` = long inputs (≥ 512 B) on both paths; and non-ASCII high bytes in name/version/codename/arch regions | `c31_long_and_non_ascii` | [x] |
| C32 | `parse_uname_string` | `S` = fully random byte strings (0–160 B) drawn from an alphabet rich in `' '`, `'['`, `']'`, `':'`, `'('`, `')'`, `'\|'`, digits, `'.'`, `'V'`, `'e'`, `'r'` — 6000 iterations, structure-aware fuzz that hits every branch combination by construction | `c32_full_random_fuzz` | [x] |
| C33 | `parse_uname_string` | end-to-end realistic corpus: real Wazuh-style uname strings (Windows, Ubuntu, CentOS, macOS, AIX, Solaris, Alpine, arm64 …) | `c33_realistic_corpus` | [x] |
| C35 | `parse_uname_string`, `get_os_arch` | allocation-size sweep: `match_size`/`strlen` = 1..120 across every 16-byte malloc bin boundary, for `os_major`/`os_minor`/`os_build` (`malloc(match_size + 1)`) and every `strdup`ed field | `c35_allocation_size_boundaries`, `alloc_sizes_match_in_lockstep_subprocesses` | [x] |
| C34 | `parse_uname_string` | `S` = 1-byte and separator-only inputs: `""`, `" "`, `"["`, `" ["`, `" [Ver: "`, `" [: "`, `" [ ("`, `"\|"`, `" [\|"` … | `c34_degenerate_separator_only` | [x] |
