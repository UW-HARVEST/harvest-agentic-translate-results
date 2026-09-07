# CONFIGS.md — configuration surface table (valid inputs)

## Axes, derived from the branches the C actually takes

`c_src/src/lib.c` has no options, flags, modes, globals, `#ifdef`s or setup
calls — the public surface is one pure function of three C strings
(`c_src/include/lib.h`), and it *is* the lowest-level entry point (there is no
convenience wrapper above it and no internal helper exported below it).
`grep -n 'if\|while\|switch\|#if' c_src/src/lib.c` yields exactly these
decision points, which define the axes:

| axis | source | values the code distinguishes |
|------|--------|-------------------------------|
| M — match presence / count | `p = strstr(...)` (L22), `while (p != NULL)` (L42), re-search (L54) | 0 matches, 1 match, 2 matches, many matches |
| P — prefix before first match | `if (inx_start > 0)` (L32) — chooses `malloc` + `strncpy` vs. leaving `tmp = NULL` and going straight to `realloc` | match at offset 0 (no prefix) vs. offset > 0 (prefix) |
| G — spacing of consecutive matches | `if (inx_start2 > from)` (L59) | adjacent (`inx_start2 == from`, gap skipped) vs. separated (gap copied) |
| O — overlapping candidates | re-search starts at `orig + inx_start + search_len` (L54), so overlapping occurrences are skipped | needle that can overlap itself (`"aa"` in `"aaaa"`, `"aba"` in `"ababa"`) vs. not |
| T — tail after last match | `if ((from < orig_len) && from > 0)` (L78) | last match ends at `orig_len` (no tail) vs. tail present |
| V — replacement length vs. needle length | governs `total_bytes_allocated` growth and the `strncpy` count `total - tmp_offset` (L50) | `value == ""` (deletion), `value_len < search_len`, `== search_len`, `> search_len` |
| S — needle shape | `strlen(search)` (L12) and `strstr` semantics | 1 byte, multi-byte, equal to the whole of `orig`, longer than `orig` |
| L — subject size | number of `realloc` rounds | empty, 1 byte, short (< 32 B), long (multi-KB with hundreds of matches) |
| B — byte values | none (byte-transparent), but `strstr`/`strncpy` are `char`-signedness sensitive | pure ASCII vs. high bytes `0x80..0xFF` |

`search == ""` is a valid *input* but non-terminating; it is tracked as
`ERRORS.md` rows 10-11, not here.

## Rows (pruned cross-product — one row per combination the C treats differently)

Every row is exercised through the `.so` exports of **both** libraries with
many randomized inputs (fixed seed `0x5EED_1234`, see
`tests/differential.rs`), except where marked *fixed* (a shape that only has
one interesting instance).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `searchAndReplace` | M=0 matches, L=short, ASCII → `strdup` path | `cfg_01_no_match_short` | [x] |
| 2 | `searchAndReplace` | M=0, L=empty `orig`, S=1 byte | `cfg_02_no_match_empty_orig` | [x] |
| 3 | `searchAndReplace` | M=0, S=longer than `orig` (`search_len > orig_len`) | `cfg_03_needle_longer_than_orig` | [x] |
| 4 | `searchAndReplace` | M=0, L=long (4 KiB), high bytes | `cfg_04_no_match_long_highbytes` | [x] |
| 5 | `searchAndReplace` | M=1, P=no prefix, T=no tail, S=whole string (`search == orig`), V=`> search_len` | `cfg_05_whole_string_match` | [x] |
| 6 | `searchAndReplace` | M=1, P=no prefix, T=no tail, V=`""` (delete whole string) | `cfg_06_whole_string_delete` | [x] |
| 7 | `searchAndReplace` | M=1, P=no prefix, T=tail present, V=random length | `cfg_07_match_at_start_with_tail` | [x] |
| 8 | `searchAndReplace` | M=1, P=prefix present, T=no tail (match at end) | `cfg_08_match_at_end_with_prefix` | [x] |
| 9 | `searchAndReplace` | M=1, P=prefix present, T=tail present (match in the middle) | `cfg_09_match_in_middle` | [x] |
| 10 | `searchAndReplace` | M=2, G=adjacent, P=no prefix, T=no tail | `cfg_10_two_adjacent_no_prefix_no_tail` | [x] |
| 11 | `searchAndReplace` | M=2, G=adjacent, P=prefix, T=tail | `cfg_11_two_adjacent_prefix_tail` | [x] |
| 12 | `searchAndReplace` | M=2, G=separated (gap copied), P=no prefix, T=no tail | `cfg_12_two_separated_no_prefix_no_tail` | [x] |
| 13 | `searchAndReplace` | M=2, G=separated, P=prefix, T=tail | `cfg_13_two_separated_prefix_tail` | [x] |
| 14 | `searchAndReplace` | M=many, G=all adjacent (`orig` is the needle repeated), V=`> search_len` | `cfg_14_many_adjacent_growing` | [x] |
| 15 | `searchAndReplace` | M=many, G=all adjacent, V=`""` (pure deletion) | `cfg_15_many_adjacent_delete` | [x] |
| 16 | `searchAndReplace` | M=many, G=mixed adjacent/separated, P/T random, S=1 byte, V=1 byte (`V == search_len`) | `cfg_16_many_mixed_len_equal` | [x] |
| 17 | `searchAndReplace` | M=many, G=mixed, S=multi-byte, V=`< search_len` (shrinking) | `cfg_17_many_mixed_shrinking` | [x] |
| 18 | `searchAndReplace` | M=many, G=mixed, S=multi-byte, V=`> search_len` (growing) | `cfg_18_many_mixed_growing` | [x] |
| 19 | `searchAndReplace` | O=self-overlapping needle `"aa"`/`"aaa"` over runs of `a` (odd and even runs) | `cfg_19_overlapping_runs` | [x] |
| 20 | `searchAndReplace` | O=partially-overlapping needle `"aba"` in `"ababababa"` | `cfg_20_overlapping_aba` | [x] |
| 21 | `searchAndReplace` | L=long (4-16 KiB), M=hundreds of matches, V=`> search_len` → many `realloc` rounds | `cfg_21_long_many_matches` | [x] |
| 22 | `searchAndReplace` | L=long, M=hundreds, V=`""` → many `realloc`s with zero growth | `cfg_22_long_many_matches_delete` | [x] |
| 23 | `searchAndReplace` | B=high bytes (0x80..0xFF) in all three arguments, M=many | `cfg_23_high_bytes_many_matches` | [x] |
| 24 | `searchAndReplace` | L=1-byte `orig`, S=1-byte needle, M=1, V=empty and non-empty | `cfg_24_single_byte` | [x] |
| 25 | `searchAndReplace` | `value` contains the needle (no re-scan of output — must not recurse) | `cfg_25_value_contains_needle` | [x] |
| 26 | `searchAndReplace` | `value == search` (identity replacement) | `cfg_26_identity_replacement` | [x] |
| 27 | `searchAndReplace` | fully random alphabet-2 fuzz: `orig` 0-40 B, `search` 1-4 B, `value` 0-6 B, 200 000 cases — hits every M/P/G/O/T/V combination by construction | `cfg_27_fuzz_alphabet2` | [x] |
| 28 | `searchAndReplace` | fully random alphabet-256 fuzz (all byte values except `NUL`): `orig` 0-64 B, `search` 1-8 B, `value` 0-16 B, 200 000 cases, each also re-run with a needle cut out of the subject so it is guaranteed to match | `cfg_28_fuzz_bytes` | [x] |
| 29 | `searchAndReplace` | **exhaustive** over `{a,b}`: every `orig` of length 0-12 (8 191) x every `search` of length 1-3 (14) x every `value` of length 0-2 (7) = 802 718 cases — subsumes rows 5-26 at small sizes with no sampling luck | `exhaustive_alphabet2` | [x] |
| 30 | `searchAndReplace` | **exhaustive** over `{a,b,c}`: every `orig` of length 0-7 x `search` 1-2 x `value` 0-1 = 157 440 cases — adds a third byte so "gap content differs from needle and replacement content" is covered exhaustively | `exhaustive_alphabet3` | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED src/lib.c)` — no
`add_executable`, and `translation/Cargo.toml` declares only `[lib] crate-type
= ["cdylib"]` with no `[[bin]]` and no `src/main.rs`. There is therefore **no
executable whose stdout could be compared**; that completion-gate item is
vacuously satisfied.

## Feature combinations

No `[features]` in `Cargo.toml` → the default configuration is the only one.
`tests/run_all_features.sh` enumerates feature sets from `Cargo.toml` and runs
the whole suite for each; it finds exactly one (default).

## How to reproduce

```
cd translation && tests/run_all_features.sh
```

Runs, for each feature set (one: the default) and for each of the debug and
release cdylibs: the `nm -D` symbol diff, an `ldd -r` unresolved-import check,
the 31 Phase B/C in-process tests (~1.15 M differential calls) and the 16
Phase C child-process probes.

## Result

All 30 rows pass, in both the debug and the release cdylib.

The test harness was mutation-checked: injecting `inx_start + 1` in place of
`inx_start + search_len` at the re-search, and `inx_start + 2` in place of
`inx_start + 1` in the prefix allocation, each made 16 of the 31 tests fail.
(Two further mutations — `inx_start2 >= from` and `src.len() <= n` — were not
caught because they are provably semantics-preserving: the first makes a
zero-length gap copy, and the second differs only where both branches yield
the same value.)
