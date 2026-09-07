# CONFIGS.md — configuration-surface table (Phase B)

The mirror of `ERRORS.md`: every **valid** input configuration the C actually
branches on. Axes derived mechanically from the `if` / `switch` / guard
structure of `c_src/src/lib.c` plus the public surface in
`c_src/include/lib.h` and `nm -D`.

## Axes the C code actually distinguishes

There are no runtime option structs, no global mode flags, no `#ifdef`s and no
Cargo features in this library — every "option" is a scalar argument. The axes
are therefore:

| axis | values the C treats differently | where it branches |
|------|--------------------------------|-------------------|
| **entry point** | 9 exported symbols; 5 low-level (`shift_array`, `process_string`, `apply_bitmask`, `init_matrix`, `compare_allocations`) + 4 composed (`arity4`, `arity3`, `arity2`, `arity`) | `nm -D` |
| `apply_bitmask.operation` | `0`, `1`, `2`, `3`, other | `switch` |
| `shift_array` guard | `positions <= 0` \| `0 < positions < size` \| `positions >= size` | `if (positions > 0 && positions < size)` |
| `shift_array.positions` shape | `1`, `2`, `3` (mid), `size-1` (max shifting) | loop bound + `memmove` length |
| `process_string` guard | first byte `0` vs non-zero | `if (*str)` |
| `process_string` length shape | 0, 1, many, high-bit bytes, embedded NUL | `strlen` |
| `compare_allocations.val1` sign | `val1 > 0` vs `val1 <= 0` | `(*uninit_ptr > 0) ? 10 : 0` |
| **heap phase** | tcache parity: `ptr1 < ptr2` vs `ptr1 > ptr2` | address compare; alternates per call |
| `arity4.param1 mod 4` | `0`,`1`,`2`,`3` and `-1`,`-2`,`-3` (C `%` keeps the sign, so negative `param1` reaches `default:`) | `apply_bitmask(result, param1 % 4)` |
| `arity4.param3` | `== 0` (skip rescale) vs `!= 0` (`*param3 / 100`), positive vs negative | `if (param3 != 0)` |
| `arity4.param4` | `== 0` vs `!= 0` | `if (param4 != 0)` |
| `arity.len` dispatch | `<2` → `-1`; `2` → `arity2`; `3` → `arity3`; `>=4` → `arity4`; all after `& 0xFF` | `if/else if/else` chain |
| magnitude | small, `INT_MAX`/`INT_MIN` boundaries, wrapping-overflow values | signed arithmetic |

Every row below is checked with **many randomized inputs from a fixed seed**
(a deterministic SplitMix64 PRNG in the test file), not one hand-picked value.
Rows touching `compare_allocations` transitively are compared as phase-neutral
call *pairs* (see `SYMBOLS.md`) so that both heap phases are observed per input.

## Table

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| 1 | `apply_bitmask` | `operation = 0` (`value & 0xF0`), `value` random over full `i32` incl. negatives | [x] |
| 2 | `apply_bitmask` | `operation = 1` (`value & 0x0F`), full-range random `value` | [x] |
| 3 | `apply_bitmask` | `operation = 2` (`value \| 0xAA`), full-range random `value` | [x] |
| 4 | `apply_bitmask` | `operation = 3` (`value ^ 0x55`), full-range random `value` | [x] |
| 5 | `apply_bitmask` | `operation` random over full `i32` (mostly `default:` identity), full-range `value` — cross-product of both args | [x] |
| 6 | `init_matrix` | fresh 3×4 buffer pre-filled with a random poison pattern; assert all 12 cells overwritten with `1..12` and no out-of-bounds write (guard cells around the buffer) | [x] |
| 7 | `init_matrix` | called twice on the same buffer (idempotence) | [x] |
| 8 | `process_string` | empty string (guard-false path) | [x] |
| 9 | `process_string` | length 1 | [x] |
| 10 | `process_string` | random length 2..64 of random **non-zero** bytes over the full `1..=255` range (exercises signed-`char` high-bit values) | [x] |
| 11 | `process_string` | first byte non-zero, embedded NUL in the middle → `strlen` stops early | [x] |
| 12 | `process_string` | long string (512 bytes) | [x] |
| 13 | `shift_array` | `size` random 1..16, `positions = 1` (the value `arity4` actually uses), random contents | [x] |
| 14 | `shift_array` | `size` random 2..16, `positions` random in `1..size-1` (guard-true, mid range) | [x] |
| 15 | `shift_array` | `positions = size - 1` (maximal in-range shift: 1 element moved, `size-1` zeroed) | [x] |
| 16 | `shift_array` | `positions = 0` and `positions < 0` (guard-false no-op) with random contents | [x] |
| 17 | `shift_array` | `positions == size` and `positions > size` (guard-false no-op) | [x] |
| 18 | `shift_array` | `size = 1`, `size = 0`, and negative `size`, random `positions` | [x] |
| 19 | `shift_array` | overlapping-region correctness: `size = 16`, `positions = 8` (dst/src overlap exactly half — the case a naive `memcpy` would corrupt) | [x] |
| 20 | `compare_allocations` | `val1 > 0` (adds 10) — phase-neutral pair, random `val1` in `1..=INT_MAX`, random `val2` | [x] |
| 21 | `compare_allocations` | `val1 == 0` and `val1 < 0` (no bonus) — phase-neutral pair, random `val2` | [x] |
| 22 | `compare_allocations` | both `val1`,`val2` random full-range `i32` — phase-neutral pair, many iterations | [x] |
| 23 | `arity4` | `param1 % 4 == 0`, `param3 == 0`, `param4 == 0` (both guards skipped) | [x] |
| 24 | `arity4` | `param1 % 4 == 1`, `param3 == 0`, `param4 == 0` | [x] |
| 25 | `arity4` | `param1 % 4 == 2`, `param3 == 0`, `param4 == 0` | [x] |
| 26 | `arity4` | `param1 % 4 == 3`, `param3 == 0`, `param4 == 0` | [x] |
| 27 | `arity4` | `param1 < 0` so `param1 % 4 ∈ {-1,-2,-3}` → `apply_bitmask` `default:` identity | [x] |
| 28 | `arity4` | `param3 != 0` positive (rescale `*param3/100`), `param4 == 0` | [x] |
| 29 | `arity4` | `param3 != 0` negative (negative product → truncate-toward-zero division), `param4 == 0` | [x] |
| 30 | `arity4` | `param3 != 0`, `param4 != 0` (both branches taken) | [x] |
| 31 | `arity4` | `param3 == 0`, `param4 != 0` (only the second branch) | [x] |
| 32 | `arity4` | all four params random full-range `i32` — the full cross-product, many iterations, exercises wrapping mul/add and every `%4` residue | [x] |
| 33 | `arity4` | boundary magnitudes: each param drawn from `{0, ±1, ±2, ±3, ±4, ±99, ±100, ±101, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1}` — exhaustive cross-product over a curated set | [x] |
| 34 | `arity2` | random `p1`,`p2` full range; also asserted equal to `arity4(p1,p2,0,0)` on the C side | [x] |
| 35 | `arity3` | random `p1..p3` full range incl. `p3 == 0` and `p3 != 0` | [x] |
| 36 | `arity` | `len = 2` → `arity2` dispatch, random `params` (buffer sized exactly 2 to catch over-read) | [x] |
| 37 | `arity` | `len = 3` → `arity3` dispatch, random `params` (buffer sized exactly 3) | [x] |
| 38 | `arity` | `len = 4` → `arity4` dispatch, random `params` | [x] |
| 39 | `arity` | `len = 5..255` → still `arity4`, reads only first 4; random `params` of matching length | [x] |
| 40 | `arity` | `len = 258`/`259`/`260` → truncate to 2/3/4, same dispatch as `len = 2/3/4` | [x] |
| 41 | `arity` | `len` negative → low byte unsigned; `-1`→255→`arity4`, `-254`→2→`arity2` | [x] |
| 42 | *composed pipeline* | `arity` driven end-to-end as a real consumer would: random `len` and random `params` together, many iterations — the low-level helpers reached only through the composed call chain | [x] |
| 43 | *cross-check* | low-level vs composed consistency: independently recompute `arity4`'s expected value from the C's own `shift_array` / `apply_bitmask` / `process_string` / `init_matrix` exports and confirm C and Rust agree at every stage, not just at the end | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only combination is
the default/empty one. Phase D runs the suite under both `cargo test` and
`cargo test --no-default-features` to confirm they are equivalent.
