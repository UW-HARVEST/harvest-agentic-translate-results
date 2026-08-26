# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

## Axes derived from the C source

`c_src/src/lib.c` is the entire library and has **one** public entry point.
There is no init/teardown, no handle, no option struct, no global state and no
`#ifdef`, so the "runtime options" axis is empty. What the code *does* branch
on (mechanically, every `if` / loop condition in the file):

| axis | source of the branch | distinct values the C treats differently |
|------|----------------------|------------------------------------------|
| A. entry point | `lib.h` | `wcscat` (the only public symbol; it is simultaneously the lowest-level and the only API) |
| B. `dst` pointer | `if (!dst ...)` | non-NULL / NULL (NULL → `ERRORS.md`) |
| C. `src` pointer | `if (!src)` | non-NULL / NULL (NULL → `ERRORS.md`) |
| D. `numElem` | `numElem == 0`, `ptr < dst + numElem` | `0`; `1`; `2`; small (3..8); large (≥64); values whose `*4` wraps 2^64 |
| E. position `k` of the first NUL in `dst[0..numElem]` (result of loop 1) | `while (ptr < dst+numElem && *ptr != 0)` | `k == 0` (empty dst string); `0 < k < numElem-1`; `k == numElem-1` (only the terminator slot is free); **no NUL** (`k == numElem`) |
| F. `wcslen(src)` relative to the free room `numElem - k` | `while (ptr < dst+numElem) { if ((*ptr++ = *src++) == 0) ... }` | `0` (empty `src`); fits with slack; fits **exactly** (`len+1 == numElem-k`); overflows by exactly 1; overflows by many |
| G. `wchar_t` values used | `*ptr != 0` / `== 0` comparisons only | ASCII; non-BMP (> 0xFFFF); **negative** (`wchar_t` is signed 32-bit here); `INT_MIN` / `INT_MAX`; lone surrogates; `0x80000000` |
| H. bytes in `dst` **after** its NUL and after the `numElem` window | the copy loop overwrites `dst[k..]` unconditionally | garbage-filled (proves the exact write extent, incl. that `dst[numElem..]` is never touched) |
| I. aliasing of `src` and `dst` | none — the C has no aliasing check | disjoint; `src == dst`; `src` inside `dst` before `ptr`; `src` inside `dst` after `ptr` |
| J. `numElem` vs. the real allocation | none — the C trusts `numElem` | `numElem == alloc`; `numElem < alloc` (window shorter than buffer) |

Rows below are the pruned cross-product: one row per combination the C code
actually distinguishes. Every row is driven with **many randomized inputs**
(fixed seed, SplitMix64 PRNG in `tests/common/mod.rs`) — random `numElem`
inside the row's class, random `k`, random `src` length inside the row's class,
random `wchar_t` payload from the row's value class, and random pre-fill
garbage for the untouched tail.

## Table

| #   | entry point(s) | configuration (options set + input shape)                                                                                                   | test | [x] |
|-----|----------------|----------------------------------------------------------------------------------------------------------------------------------------------|------|-----|
| C1  | `wcscat` | `k = 0` (`dst[0]==0`), `src` empty, `numElem` random 1..64, garbage tail                                                                          | `c1_empty_dst_empty_src` | [x] |
| C2  | `wcscat` | `k = 0`, `src` non-empty, fits with slack, ASCII payload, `numElem` random 2..64                                                                   | `c2_empty_dst_src_fits` | [x] |
| C3  | `wcscat` | `k = 0`, `src` fits **exactly** (`len+1 == numElem`)                                                                                              | `c3_empty_dst_src_exact_fit` | [x] |
| C4  | `wcscat` | `k = 0`, `src` overflows by exactly 1                                                                                                             | `c4_empty_dst_src_over_by_one` | [x] |
| C5  | `wcscat` | `k = 0`, `src` overflows by many (`len ≫ numElem`)                                                                                                | `c5_empty_dst_src_over_by_many` | [x] |
| C6  | `wcscat` | `0 < k < numElem-1` (real append), `src` empty                                                                                                    | `c6_midstring_empty_src` | [x] |
| C7  | `wcscat` | `0 < k < numElem-1`, `src` fits with slack (the ordinary append case)                                                                              | `c7_midstring_src_fits` | [x] |
| C8  | `wcscat` | `0 < k < numElem-1`, `src` fits **exactly** (`k+len+1 == numElem`)                                                                                 | `c8_midstring_src_exact_fit` | [x] |
| C9  | `wcscat` | `0 < k < numElem-1`, `src` overflows by exactly 1 → partial copy + `dst[0]=0` + `34`                                                              | `c9_midstring_over_by_one` | [x] |
| C10 | `wcscat` | `0 < k < numElem-1`, `src` overflows by many → whole tail filled from `src` prefix, then `dst[0]=0`                                               | `c10_midstring_over_by_many` | [x] |
| C11 | `wcscat` | `k == numElem-1` (only the terminator slot free), `src` empty → success                                                                            | `c11_k_last_slot_empty_src` | [x] |
| C12 | `wcscat` | `k == numElem-1`, `src` non-empty → one element written then `34`                                                                                  | `c12_k_last_slot_nonempty_src` | [x] |
| C13 | `wcscat` | **no NUL** in `dst[0..numElem]` (all elements non-zero) — `src` never read                                                                         | `c13_unterminated_window` | [x] |
| C14 | `wcscat` | `numElem == 1`, all three sub-shapes (`dst[0]==0` + empty `src`; `dst[0]==0` + non-empty `src`; `dst[0]!=0`)                                       | `c14_numelem_one_all_shapes` | [x] |
| C15 | `wcscat` | `numElem == 2`, all `k ∈ {0,1,none}` × `src ∈ {empty, 1 char, 2+ chars}`                                                                           | `c15_numelem_two_full_cross` | [x] |
| C16 | `wcscat` | large `numElem` (256..4096), random `k`, random `src` length across all fit classes                                                                | `c16_large_buffers` | [x] |
| C17 | `wcscat` | `numElem < alloc` (window shorter than the physical buffer); NUL of `dst` inside the window; asserts `dst[numElem..]` is untouched                 | `c17_window_shorter_than_alloc` | [x] |
| C18 | `wcscat` | `numElem < alloc`; NUL of `dst` **outside** the window → `34` path, tail beyond window untouched                                                   | `c18_window_excludes_terminator` | [x] |
| C19 | `wcscat` | payload class: non-BMP / > 0xFFFF `wchar_t` values, both in `dst` prefix and `src`, all fit classes                                               | `c19_wide_codepoints` | [x] |
| C20 | `wcscat` | payload class: **negative** `wchar_t` values (`-1`, `INT_MIN`, random negatives) — must be treated as non-zero, not as terminators                | `c20_negative_wchars` | [x] |
| C21 | `wcscat` | payload class: extremes `INT_MAX`, `INT_MIN`, `0x80000000`, `0xD800`, `0xDFFF`, `0x110000` mixed randomly                                         | `c21_extreme_wchars` | [x] |
| C22 | `wcscat` | garbage after `dst`'s NUL **and** after `src`'s NUL (proves read/write extents exactly)                                                            | `c22_garbage_after_terminators` | [x] |
| C23 | `wcscat` | aliasing: `src == dst` (append a string to itself, `k` random)                                                                                     | `c23_alias_src_eq_dst` | [x] |
| C24 | `wcscat` | aliasing: `src == dst + j` with `j < k` (source region overlaps the destination region being written)                                             | `c24_alias_src_before_k` | [x] |
| C25 | `wcscat` | aliasing: `src == dst + j` with `k < j < numElem` (source lies in the region about to be overwritten)                                             | `c25_alias_src_after_k` | [x] |
| C26 | `wcscat` | repeated invocation: call `wcscat` several times in a row on the same buffer (accumulating appends until it overflows), comparing after every call | `c26_repeated_appends` | [x] |
| C27 | `wcscat` | fully unconstrained fuzz: random `numElem` 1..64, random buffer contents from the full `wchar_t` range (so `k` and fit class are random too)       | `c27_unconstrained_fuzz` | [x] |
| C28 | `wcscat` | `numElem` values whose `*sizeof(wchar_t)` wraps 2^64 → `end <= dst + 1` (`2^62`, `2^62+1`, `SIZE_MAX`, `SIZE_MAX-1`, `2^63`) with random buffers   | `c28_wrapping_numelem` | [x] |

Every row is checked off only after passing all of its randomized iterations
against **both** `.so` files, under **every** feature combination listed in
`SYMBOLS.md`.

## How to run

```
./run_all_features.sh          # every feature combo x {debug, release} Rust .so
cargo test --offline --test valid_paths     # Phase B only
```

Status: **28/28 rows pass** for both feature combinations
(`--no-default-features` and `--features default`) and for both the debug and
the release (`panic = "abort"`, optimized) Rust `.so`.
