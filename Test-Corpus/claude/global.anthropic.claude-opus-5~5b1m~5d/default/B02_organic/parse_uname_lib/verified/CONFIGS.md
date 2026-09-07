# CONFIGS.md — Phase A: configuration / valid-input surface table

Mechanically derived from the branches the C actually takes in
`c_src/src/lib.c`. There are **no runtime option flags, no modes, no
`#ifdef`s, and no global state** in this library — the only "configuration"
axes are (a) which public entry point is called, and (b) the *shape* of the
input string. So the table below enumerates the cross-product of the input
shapes the C distinguishes.

## Axes the C branches on

**Entry points (all three, including the low-level ones — `get_os_arch` and
`w_regexec` are NOT in the public header but ARE exported and are called
directly here, not only via the `parse_uname_string` wrapper):**

| entry point | branch points in C |
|---|---|
| `get_os_arch` | 12-iteration first-match-wins loop over `ARCHS[]` (line 22) |
| `w_regexec` | NULL guard (36), `regcomp` result (40), `regexec` result (45), `nmatch` passed through (45) |
| `parse_uname_string` | `!osd` (64), `" [Ver: "` (68), 3 regexes (75/82/89), `" ["` (98), `": "` (102), `" ("` (109), `"|"` (135), 2 regexes (117/124), `get_os_arch` (142) |

**Input-shape axes**

* A1 `ARCHS` index hit: 0..11 (`x86_64 i386 i686 sparc amd64 i86pc ia64 AIX armv6 armv7 aarch64 arm64`) / none
* A2 arch position: prefix / infix / suffix / multiple archs present (order priority matters)
* B1 regex pattern: the 3 literal patterns used internally, plus arbitrary valid EREs with 0/1/2 capture groups, anchored/unanchored, alternation, nested groups, non-participating groups
* B2 `nmatch`: 0 / 1 / 2 / > `re_nsub+1`
* B3 subject string: empty / matching / non-matching / matching at offset > 0 / very long
* C1 separator present: `" [Ver: "` / `" ["` / neither / both
* C2 within Windows branch: version has 0 / 1 / 2 / 3 / 4+ dotted numeric components; leading non-digits; trailing junk; `]` present or absent
* C3 within Unix branch: `": "` present / absent
* C4 within Unix branch: `" ("` present / absent (codename)
* C5 within Unix branch: `"|"` present / absent (platform) — and `"|"` before vs. after where `": "` was
* C6 arch present in the *truncated* `uname` prefix (only the part before `" ["` is searched, because C NUL-terminates there first)
* C7 multiple occurrences of a separator (`strstr` finds the FIRST)
* C8 `osd` pre-populated with non-NULL garbage (C never clears fields ⇒ untouched fields must survive)

## Table (one row per combination the C treats differently)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `get_os_arch` | each of the 12 ARCHS alone, exactly (`"x86_64"`, …, `"arm64"`) — 12 sub-cases | [x] |
| 2  | `get_os_arch` | arch as prefix / infix / suffix of a longer random string (randomized position + padding) | [x] |
| 3  | `get_os_arch` | **two or more** archs present ⇒ first-in-`ARCHS`-order wins, not left-most-in-string (e.g. `"sparc x86_64"` → `x86_64`) — randomized pairs | [x] |
| 4  | `get_os_arch` | no arch present (randomized alphabet excluding arch substrings) | [x] |
| 5  | `get_os_arch` | empty string; 1-char string; string shorter than the shortest arch | [x] |
| 6  | `get_os_arch` | overlapping/near-miss tokens: `"i86"`, `"arm"`, `"aarch"`, `"x86"`, `"64"`, `"aix"` (lower-case ⇒ no match, C is case-sensitive) | [x] |
| 7  | `get_os_arch` | very long input (4 KiB) with the arch at the very end | [x] |
| 8  | `w_regexec` | the 3 internal patterns × matching subjects, `nmatch=2`, `pmatch[2]` — randomized version strings | [x] |
| 9  | `w_regexec` | pattern with 0 capture groups, `nmatch=2` ⇒ `pmatch[1]` untouched by regexec (must agree incl. leftovers) | [x] |
| 10 | `w_regexec` | pattern with 1 group, `nmatch=1` ⇒ only `pmatch[0]` filled | [x] |
| 11 | `w_regexec` | pattern with 1 group, `nmatch=0` ⇒ nothing filled, return still reflects match | [x] |
| 12 | `w_regexec` | pattern with 2 groups, `nmatch=8` (> `re_nsub+1`) with an 8-slot buffer ⇒ tail slots set to `{-1,-1}` | [x] |
| 13 | `w_regexec` | non-participating group (`"^(a)|(b)$"` style / `"([0-9]+(\\.[0-9]+)*)"` with no second component) ⇒ `{-1,-1}` for that group | [x] |
| 14 | `w_regexec` | unanchored pattern matching at a nonzero offset (`rm_so > 0`) | [x] |
| 15 | `w_regexec` | empty subject `""` with a pattern that matches empty (`"^.*"`) and one that doesn't (`"^a"`) | [x] |
| 16 | `w_regexec` | ERE metacharacters exercised: `+ * ? | () [] {} . ^ $ \\.` — randomized valid patterns | [x] |
| 17 | `w_regexec` | long subject (4 KiB) with match near the end (unanchored) | [x] |
| 18 | `parse_uname_string` | **Windows branch**, full `"<name> [Ver: MAJ.MIN.BUILD]"` — randomized MAJ/MIN/BUILD digits | [x] |
| 19 | `parse_uname_string` | Windows branch, 4+ dotted components `"[Ver: 10.0.19041.1234]"` ⇒ `os_build` captures the whole multi-dot tail via the optional group | [x] |
| 20 | `parse_uname_string` | Windows branch, 2 components `"[Ver: 6.1]"` ⇒ build NULL | [x] |
| 21 | `parse_uname_string` | Windows branch, 1 component `"[Ver: 10]"` ⇒ minor+build NULL | [x] |
| 22 | `parse_uname_string` | Windows branch with an arch token also present ⇒ `os_arch` stays NULL (branch skips `get_os_arch`) | [x] |
| 23 | `parse_uname_string` | Windows branch, name part contains `"|"` and/or `" ("` ⇒ those are NOT parsed in this branch (`os_platform` is forced to `"windows"`) | [x] |
| 24 | `parse_uname_string` | Windows branch, `uname` also contains `" ["` later ⇒ `" [Ver: "` wins | [x] |
| 25 | `parse_uname_string` | Windows branch, multiple `" [Ver: "` ⇒ first occurrence used | [x] |
| 26 | `parse_uname_string` | Windows branch, version with leading zeros / very large numbers (`"[Ver: 0007.0000.99999999999]"`) | [x] |
| 27 | `parse_uname_string` | Windows branch, trailing char that is not `]` (C blindly deletes the last char whatever it is) | [x] |
| 28 | `parse_uname_string` | **Unix branch**, `"<pfx> [<name>: <maj>.<min> (<codename>)]"` — full form, randomized | [x] |
| 29 | `parse_uname_string` | Unix branch, `"<pfx> [<name>: <ver>]"` — no codename | [x] |
| 30 | `parse_uname_string` | Unix branch, `"<pfx> [<name>]"` — no `": "` ⇒ trim-last-char path on `os_name` | [x] |
| 31 | `parse_uname_string` | Unix branch, `"<pfx> [<name>|<platform>: <ver>]"` ⇒ `os_platform` from after `"|"`, `os_name` truncated at `"|"` | [x] |
| 32 | `parse_uname_string` | Unix branch, `"<pfx> [<name>|<platform>]"` (no `": "`) ⇒ trim-last then split on `"|"` | [x] |
| 33 | `parse_uname_string` | Unix branch, `"|"` located in the *version* part ⇒ NOT found in `os_name` (already truncated at `": "`) ⇒ `os_platform` NULL | [x] |
| 34 | `parse_uname_string` | Unix branch, arch token before `" ["` ⇒ `os_arch` set | [x] |
| 35 | `parse_uname_string` | Unix branch, arch token only *after* `" ["` ⇒ `os_arch` NULL (uname was NUL-terminated at `" ["` before `get_os_arch` runs) | [x] |
| 36 | `parse_uname_string` | Unix branch, multiple `" ["` ⇒ first used; multiple `": "` ⇒ first used; multiple `" ("` ⇒ first used | [x] |
| 37 | `parse_uname_string` | Unix branch, codename containing `"("`/`")"`/spaces | [x] |
| 38 | `parse_uname_string` | Unix branch, version = `"maj.min.patch"` (3 components) ⇒ `os_major`,`os_minor` only (no build regex in this branch) | [x] |
| 39 | `parse_uname_string` | Unix branch, version starting with digits then letters (`"20.04LTS"`) | [x] |
| 40 | `parse_uname_string` | **Neither separator**: `"Linux x86_64 5.4.0"` ⇒ only `os_arch` set | [x] |
| 41 | `parse_uname_string` | Neither separator and no arch ⇒ every field left untouched | [x] |
| 42 | `parse_uname_string` | `osd` pre-filled with non-NULL sentinel pointers ⇒ untouched fields must keep the sentinel identically in C and Rust (proves which fields each branch writes) | [x] |
| 43 | `parse_uname_string` | `os_uname` never written in any branch (checked on every row via the sentinel) | [x] |
| 44 | `parse_uname_string` | in-place mutation of the caller's `uname` buffer compared byte-for-byte (all branches) | [x] |
| 45 | `parse_uname_string` | fully randomized fuzz: random separators/tokens/archs/digits assembled from the grammar, seeded PRNG, 20 000 cases | [x] |
| 46 | `parse_uname_string` | empty `uname` `""` ⇒ no separator, no arch, all fields untouched | [x] |
| 47 | `parse_uname_string` | long `uname` (4 KiB) with separators near the end | [x] |
| 48 | composed pipeline | `w_regexec` used with the exact 5 internal call-sites' arguments, then the resulting `pmatch` fed through the `malloc`+`snprintf("%.*s")` duplication, compared against `parse_uname_string`'s field output | [x] |
