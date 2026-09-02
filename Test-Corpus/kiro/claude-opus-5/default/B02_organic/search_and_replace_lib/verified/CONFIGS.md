# CONFIGS.md — configuration / valid-input surface table (Phase A)

Mechanically derived from `c_src/src/lib.c`. There are no runtime options, no
flags, no modes, no `#ifdef`s and no compile-time configuration (`CMakeLists.txt`
sets no `target_compile_definitions`; `Cargo.toml` declares no features), so the
configuration axes are exactly the **input shapes the C code branches on**:

Axes taken straight from the source:

* **A1** `strstr(orig, search) == NULL` (line 23) → `strdup` early-out, vs. a match.
* **A2** `inx_start > 0` (line 32) → prefix `malloc` branch, vs. match at offset 0
  (`tmp` stays `NULL`, first `realloc` acts as `malloc`).
* **A3** number of `while (p != NULL)` iterations (line 42): 1 vs. 2 vs. many.
* **A4** `inx_start2 > from` (line 59) → gap-copy branch, vs. adjacent matches
  (gap `== 0`, branch skipped).
* **A5** `(from < orig_len) && from > 0` (line 78) → tail-copy branch, vs. last
  match ending exactly at the end of `orig`.
* **A6** `value_len` relative to `search_len`: `0` (deletion, `total_bytes_allocated`
  does not grow), `<`, `==`, `>` (buffer shrinks / stays / grows).
* **A7** `search_len`: `1`, `>1`, `== orig_len`, `> orig_len`.
* **A8** rescan start `orig + inx_start + search_len` (line 53) → overlapping
  occurrences are skipped; also lands exactly on the NUL terminator when a match
  ends the string.
* **A9** `orig_len`: `0`, small, large enough to force many `realloc`s.
* **A10** byte range: ASCII, bytes `>= 0x80` (signed-`char` sensitivity of
  `strstr`/`strncpy`), and the full `0x01..0xFF` alphabet.
* **A11** the replacement `value` itself containing `search` → the algorithm scans
  `orig` only, never its own output.

Entry points: the library has exactly one — `searchAndReplace` — and it *is* the
lowest level; there are no convenience wrappers, no internal `static` helpers with
external visibility, and no state to set up. Every row below therefore drives
`searchAndReplace` through both `.so`s (C and Rust release + Rust debug) and
compares the returned C string byte-for-byte, plus `NULL`-ness.

Every row is exercised with **many randomized inputs** generated from a fixed
seed (SplitMix64, seed per row) rather than a single hand-picked value; the
hand-picked shape is included as the first case of each row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `searchAndReplace` | A1 no match, `orig == ""`, `search` non-empty, `value` random | [x] |
| 2 | `searchAndReplace` | A1 no match, `search_len > orig_len` (A7), random `orig`/`value` | [x] |
| 3 | `searchAndReplace` | A1 no match, `search_len == orig_len` but different bytes (A7) | [x] |
| 4 | `searchAndReplace` | A1 no match, random `orig` over a 2-letter alphabet with `search` guaranteed absent | [x] |
| 5 | `searchAndReplace` | A2 match at offset 0, A3 one match, A5 tail present, `value == ""` (A6 delete) | [x] |
| 6 | `searchAndReplace` | A2 match at offset 0, one match, tail present, `value_len < search_len` | [x] |
| 7 | `searchAndReplace` | A2 match at offset 0, one match, tail present, `value_len == search_len` | [x] |
| 8 | `searchAndReplace` | A2 match at offset 0, one match, tail present, `value_len > search_len` | [x] |
| 9 | `searchAndReplace` | A2 match at offset 0, one match, **no tail** (`search == orig`, A5/A7), `value` non-empty | [x] |
| 10 | `searchAndReplace` | A2 match at offset 0, one match, no tail, `value == ""` → result is `""` | [x] |
| 11 | `searchAndReplace` | prefix present (A2 `inx_start > 0`), one match in the middle, tail present, `value == ""` | [x] |
| 12 | `searchAndReplace` | prefix present, one match in the middle, tail present, `value_len > search_len` | [x] |
| 13 | `searchAndReplace` | prefix present, single match ending exactly at end of `orig` (A5 tail skipped) | [x] |
| 14 | `searchAndReplace` | two adjacent matches (A4 gap `== 0`), random prefix/tail/value | [x] |
| 15 | `searchAndReplace` | two matches with a non-empty gap (A4 gap branch), random prefix/tail/value | [x] |
| 16 | `searchAndReplace` | matches at both ends: one at offset 0 and one ending at `orig_len` (A2+A5 both skipped) | [x] |
| 17 | `searchAndReplace` | overlapping occurrences (A8), e.g. `search = "aa"` in runs of `a`, `value` random | [x] |
| 18 | `searchAndReplace` | many matches (A3, 8..200 occurrences) → many `realloc`s, random gaps incl. zero | [x] |
| 19 | `searchAndReplace` | `search_len == 1` (A7) over a tiny alphabet so matches are dense | [x] |
| 20 | `searchAndReplace` | `value_len == 1`, many matches | [x] |
| 21 | `searchAndReplace` | `value` contains `search` (A11), several matches | [x] |
| 22 | `searchAndReplace` | high bytes only: all three arguments drawn from `0x80..0xFF` (A10) | [x] |
| 23 | `searchAndReplace` | full byte alphabet `0x01..0xFF` for `orig`/`search`/`value`, random lengths (A10) | [x] |
| 24 | `searchAndReplace` | large `orig` (4 KiB..64 KiB, A9) with a short `search` and random `value`, many matches | [x] |
| 25 | `searchAndReplace` | `orig == ""` … `orig_len == 1` boundary sweep with `search_len` in `1..=3` (A9/A7 cross-product, exhaustive over a 2-letter alphabet) | [x] |
| 26 | `searchAndReplace` | `value == ""` with many matches and no prefix → `total_bytes_allocated` stays `1` until the first gap (A6 boundary) | [x] |
| 27 | `searchAndReplace` | exhaustive cross-product: **all** `orig` of length 0..8 over `{a,b}` (511 strings), **all** `search` of length 1..3 over `{a,b}` (14), `value` in `{"", "a", "XY", "ab", "aaa"}` — 35 770 cases, brute-forces every A1-A8 branch combination that fits in 8 bytes | [x] |
| 28 | `searchAndReplace` | large `value` (2-8 KiB) against a 200-2000 byte `orig` with a 1-2 byte `search`: buffer grows by `value_len` per match (opposite growth pattern from row 24) | [x] |
| 29 | `searchAndReplace` | unbounded random soak across all four alphabets, lengths 0..80, `search` half the time lifted out of `orig` so matches are dense — 2.2M cases run across 5 seeds | [x] |

Rows 1-29 are implemented in `tests/differential.rs`, one `#[test]` per row,
named `row01_…` .. `row27_…` plus `row24_large_input_many_reallocs` (row 28) and
`soak_random_fuzz` (row 29); `row00_harness_loads_both_shared_objects` guards
against a vacuously-passing harness. A row is checked off only after it passes
for all of its randomized inputs against **both** Rust `.so` profiles
(`target/release/libdriver.so` and `target/debug/libdriver.so`), compared against
`c_src/build/libdriver.so`.

Row 22 additionally has an exhaustive variant
(`row22_high_bytes_exhaustive`): every shape up to 6 bytes over the high-byte
alphabet `{0x80, 0xFF}`, which is what rules out a signed-`char` divergence
between C `strstr`/`strncpy` and the Rust `u8` comparisons.

Status: **29/29 rows pass**, under both feature combinations
(`scripts/check_features.sh`).
