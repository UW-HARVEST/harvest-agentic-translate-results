# CONFIGS.md — Phase A: configuration / valid-input surface table

## Public entry points

`c_src/include/lib.h` declares exactly **one** entry point, and it is also the
lowest-level one (there are no helper/convenience layers, no `static` helpers, no
init/teardown state, no context struct, no global configuration):

```c
char *searchAndReplace(const char *orig, const char *search, const char *value);
```

So every row below drives `searchAndReplace` directly through the `.so` export.

## Axes the C code actually branches on

Enumerated from every `if` / `while` in `c_src/src/lib.c` plus the documented
behaviour of the three libc primitives it calls (`strstr`, `strncpy`, `realloc`):

| axis | source | states |
|------|--------|--------|
| A. match present | `if (p == NULL)` (l.23) | no match → `strdup(orig)` / ≥1 match → rewrite loop |
| B. prefix before first match | `if (inx_start > 0)` (l.32) | `inx_start == 0` (no `malloc`, `tmp` stays `NULL`, first `realloc(NULL,…)`) / `inx_start > 0` (`malloc` + `strncpy` prefix) |
| C. loop trip count | `while (p != NULL)` (l.42) | 1 match / 2 matches / many (≥8) matches |
| D. further match found | `if (p != NULL)` (l.55) | last iteration (`inx_start` frozen) / non-last |
| E. gap between matches | `if (inx_start2 > from)` (l.59) | adjacent matches (`gap == 0`, no `realloc`) / separated (`gap > 0`, gap `strncpy`) |
| F. tail after last match | `if ((from < orig_len) && from > 0)` (l.78) | tail present / match ends exactly at `orig_len` (no tail copy) |
| G. `value_len` | `total += value_len`, `strncpy(…, value, total-tmp_offset)` (ll.44‑50) | `0` (pure deletion; `strncpy` writes only the NUL pad) / `< search_len` (shrink) / `== search_len` (in‑place size) / `> search_len` (grow) |
| H. `search_len` | `strstr` + `orig + inx_start + search_len` (l.54) | `1` / `> 1` / `== orig_len` (whole string) / `> orig_len` (`strstr` early-out → no match) |
| I. occurrence overlap | non-overlapping restart at `orig + inx_start + search_len` (l.54) | needle whose occurrences overlap (`"aa"` in `"aaaa…"`) → only every other one replaced |
| J. `value` ⊇ `search` | output is never re-scanned | `value` contains `search` (must NOT be re-replaced) / `value == search` (identity) |
| K. byte domain | pure `char`/byte comparisons, no locale | ASCII / high-bit bytes `0x80…0xFF` (non-UTF‑8) / all bytes `1..=255` |
| L. size | `realloc` growth path | tiny (`orig_len` 0–3) / small (≤64) / large (~64 KiB, hundreds of matches) |

Rows below are the pruned cross-product: one row per combination the code treats
differently. **Every row is exercised with many randomized inputs (fixed seed,
`SplitMix64`), not a single hand-picked value**, and both `.so`s are called
through their exported `searchAndReplace` symbol and compared byte-for-byte
(NUL-ness of the pointer, `strlen`, and the full byte content).

| # | entry point | configuration (axes set + input shape) | test fn | ✔ |
|---|-------------|-----------------------------------------|---------|---|
| C1 | `searchAndReplace` | A=no match; random ASCII `orig` (1..64), random absent `search` (1..8), random `value` | `c1_no_match_random` | [x] |
| C2 | `searchAndReplace` | A=no match, H=`search_len > orig_len` (needle longer than haystack) | `c2_no_match_needle_longer` | [x] |
| C3 | `searchAndReplace` | A=no match, L=`orig_len == 0` (empty haystack, non-empty needle) → `strdup("")` | `c3_no_match_empty_orig` | [x] |
| C4 | `searchAndReplace` | A=match, B=no prefix, C=1, F=tail, G=`value_len == search_len` | `c4_single_at_start_tail_same_len` | [x] |
| C5 | `searchAndReplace` | A=match, B=no prefix, C=1, F=tail, G=grow (`value_len` up to 64 ≫ `search_len`) | `c5_single_at_start_tail_grow` | [x] |
| C6 | `searchAndReplace` | A=match, B=no prefix, C=1, F=tail, G=shrink (`value_len < search_len`, `search_len` 4..8) | `c6_single_at_start_tail_shrink` | [x] |
| C7 | `searchAndReplace` | A=match, B=no prefix, C=1, F=tail, G=`value_len == 0` (deletion) | `c7_single_at_start_tail_delete` | [x] |
| C8 | `searchAndReplace` | A=match, B=no prefix, C=1, F=**no tail** (`search == orig`, whole string), G>0 | `c8_whole_string_match` | [x] |
| C9 | `searchAndReplace` | A=match, B=no prefix, C=1, F=no tail, G=0 → result is the empty string | `c9_whole_string_match_delete` | [x] |
| C10 | `searchAndReplace` | A=match, B=**prefix > 0**, C=1, F=tail; random prefix/tail, all `value_len` classes | `c10_prefix_single_tail` | [x] |
| C11 | `searchAndReplace` | A=match, B=prefix > 0, C=1, F=no tail (match ends at `orig_len`) | `c11_prefix_single_no_tail` | [x] |
| C12 | `searchAndReplace` | A=match, C=2, E=**gap > 0**, B/F random | `c12_two_matches_with_gap` | [x] |
| C13 | `searchAndReplace` | A=match, C=2, E=**gap == 0** (adjacent matches, gap `realloc` skipped) | `c13_two_matches_adjacent` | [x] |
| C14 | `searchAndReplace` | A=match, C=2, B=no prefix, F=no tail (match at index 0 and at the very end) | `c14_two_matches_start_and_end` | [x] |
| C15 | `searchAndReplace` | A=match, C=many (8..24) **all adjacent** (`"ababab…"`), G random | `c15_many_adjacent_matches` | [x] |
| C16 | `searchAndReplace` | A=match, C=many with **random gaps**, random prefix and tail (general case) | `c16_many_random_gaps` | [x] |
| C17 | `searchAndReplace` | I=overlapping occurrences, even run lengths (`"aa"` in `"a"*2k`) | `c17_overlapping_even_runs` | [x] |
| C18 | `searchAndReplace` | I=overlapping occurrences, odd run lengths (leftover byte becomes tail) | `c18_overlapping_odd_runs` | [x] |
| C19 | `searchAndReplace` | H=`search_len == 1`, C=many, G ∈ {0,1,many} over a 2-letter alphabet | `c19_single_byte_needle` | [x] |
| C20 | `searchAndReplace` | H=long needle (`search_len` 8..24, `orig_len` up to 96), C=1..3 | `c20_long_needle` | [x] |
| C21 | `searchAndReplace` | J=`value` **contains** `search` (output must not be re-scanned) | `c21_value_contains_search` | [x] |
| C22 | `searchAndReplace` | J=`value == search` (identity rewrite, output must equal input) | `c22_value_equals_search` | [x] |
| C23 | `searchAndReplace` | K=high-bit bytes `0x80..0xFF` in `orig`/`search`/`value` (non-UTF‑8) | `c23_high_bit_bytes` | [x] |
| C24 | `searchAndReplace` | L=tiny shapes: `orig_len` ∈ {1,2,3} × `search_len` ∈ {1,2,3} × `value_len` ∈ {0,1,2}, exhaustive over a 2-byte alphabet | `c24_exhaustive_tiny_shapes` | [x] |
| C25 | `searchAndReplace` | L=large: `orig_len` ≈ 64 KiB, hundreds of matches, `value_len` up to 32 (hammers the `realloc` growth path) | `c25_large_input_many_matches` | [x] |
| C26 | `searchAndReplace` | B=prefix > 0 **and** C=many **and** E=mixed adjacent/gapped **and** F=no tail (all guards in one shape) | `c26_prefix_mixed_gaps_no_tail` | [x] |
| C27 | `searchAndReplace` | needles with self-overlapping prefixes (`"aab"`, `"aba"`) inside runs that contain many partial matches | `c27_partial_match_prefixes` | [x] |
| C28 | `searchAndReplace` | F boundary: tail of exactly 1 byte (`from == orig_len - 1`) | `c28_one_byte_tail` | [x] |
| C29 | `searchAndReplace` | full random property sweep over the 2-letter alphabet: `orig` 0..40, `search` 1..3, `value` 0..4 — 20 000 cases (dense cross-product of A–J) | `c29_property_sweep_small_alphabet` | [x] |
| C30 | `searchAndReplace` | full random property sweep over **all** bytes `1..=255`: `orig` 0..48, `search` 1..4, `value` 0..6, needle sometimes copied out of `orig` to force matches — 20 000 cases | `c30_property_sweep_full_byte_range` | [x] |
