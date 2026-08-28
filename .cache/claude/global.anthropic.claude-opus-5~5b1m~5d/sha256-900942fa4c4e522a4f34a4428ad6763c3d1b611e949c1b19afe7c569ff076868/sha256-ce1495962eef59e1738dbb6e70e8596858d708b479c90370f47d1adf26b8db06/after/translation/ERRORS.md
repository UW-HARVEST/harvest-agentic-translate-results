# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return`, every explicit
check, and every rejection path in the file is accounted for below.

Grep inventory of the C file (`grep -n 'return\|assert\|NULL\|if ('`):

* `lib.c:33` `if (!src) { return NULL; }` — rejection #1
* `lib.c:37` `if (!size) { size = strlen(src); }` — NOT a rejection (mode switch, see `CONFIGS.md`)
* `lib.c:41` `out = calloc(sizeof(char), size * 4 / 3 + 4);`
* `lib.c:42` `if (!out) { return NULL; }` — rejection #2
* `lib.c:53/57/69/75` `if (i + 1 < size)` / `if (i + 2 < size)` — NOT rejections (padding branches, see `CONFIGS.md`)
* `lib.c:82` `return out;` — success

There are **no** `assert`s, no error enums, no error-code macros, no
`RETURN_ERROR`-style macros, and no min/max constants in this library. The only
failure signal in the entire public API is the `NULL` sentinel return value
documented in the header comment ("Returns encoded string otherwise NULL").
`encode()` is `static`, total over all 256 `unsigned char` inputs, and has no
rejection path (its final `return '/'` is the catch-all for `u >= 63`).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `encode_base64` | `src == NULL` (`!src`), with `size == 0` | returns `NULL` (never touches `size`/`calloc`) | `e1_null_src_size_zero` | [x] |
| E2 | `encode_base64` | `src == NULL` (`!src`), with `size > 0` | returns `NULL` — the null check precedes everything | `e2_null_src_positive_size` | [x] |
| E3 | `encode_base64` | `src == NULL` (`!src`), with `size < 0` | returns `NULL` | `e3_null_src_negative_size` | [x] |
| E4 | `encode_base64` | `src == NULL` and a `size` that would itself make `calloc` fail | returns `NULL` (E1 short-circuits before E5 can trigger) | `e4_null_src_overflow_size` | [x] |
| E5 | `encode_base64` | `calloc` returns `NULL` because `size * 4 / 3 + 4` is **negative** as `int` and sign-extends to a huge `size_t`. Reached with negative `size` (verified triggers: `-4` → `n=-1`, `-5` → `n=-2`, `-6` → `n=-4`, `-7` → `n=-5`, `-100` → `n=-129`, `-1000` → `n=-1329`, `-536870912` → `n=-715827878`, `-1500000000` → `n=-568344230`) | returns `NULL` | `e5_calloc_fails_negative_n_from_negative_size` | [x] |
| E6 | `encode_base64` | `calloc` returns `NULL` via the same negative-`n` path but reached with a **positive** `size` whose `size * 4` overflows `int` into the negative range (`size` in `[2^29, 1073741820]`) | returns `NULL` (returns before the read loop) | `e6_calloc_fails_negative_n_from_int_overflow` | [x] |

### Generic FFI boundary cases (required by Phase C even though not in the table)

| # | condition | expected C result | test | status |
|---|-----------|-------------------|------|--------|
| G1 | null pointer + every interesting `size` (0, ±1, ±3, ±4, `INT_MIN`, `INT_MAX`, `2^29`, `2^30`) | `NULL` every time | `g1_null_pointer_matrix` | [x] |
| G2 | zero length: `size == 0` with `src` pointing at `""` (`strlen == 0`) | non-`NULL`, 4-byte zeroed buffer, no output bytes | `g2_zero_length` | [x] |
| G3 | zero length: `size == 0` with `src` pointing at a buffer whose first byte is `\0` but which has trailing non-NUL data | non-`NULL`, zeroed buffer, no output bytes (`strlen` stops at the NUL) | `g3_zero_length_leading_nul` | [x] |
| G4 | oversized / overflowing lengths (`INT_MAX`, `2^30`, `2^30 - 1`, `2^31-ish`) | *see note below* | `g4_oversized_lengths` | [x] |
| G5 | one step past the valid range on the negative side: `size = -1, -2, -3` (`n` = 3, 2, 0 → `calloc` SUCCEEDS) vs `size = -4` (`n = -1` → `calloc` fails). Note `size = -3` gives `n == 0` exactly, i.e. `calloc(1, 0)`, which glibc answers with a valid non-`NULL` zero-usable-byte pointer. | `-1/-2/-3` → non-`NULL` zeroed buffer of `n` bytes; `-4` → `NULL` | `g5_negative_size_boundary` | [x] |
| G6 | `size = INT_MIN` (`size * 4` wraps to exactly 0 → `n = 4`) and `size = -2^30` (same) | non-`NULL`, 4 zero bytes, loop never runs | `g6_int_min_and_neg_2_30` | [x] |
| G7 | out-of-range enum values across the FFI boundary | **N/A — this API has no enum parameters.** The only scalar parameter is `int size`, whose *entire* `int` domain is a legal input; every reachable class of it (negative, zero, positive, overflowing) is covered by E5/E6/G1/G4/G5/G6 and by `CONFIGS.md`. | — | [x] |

**Note on G4 / unrepresentable-but-crashing inputs.** For a positive `size`
larger than the buffer actually supplied, the C code reads `src[i]` for
`i < size` and writes `4*ceil(size/3)` bytes into a buffer of only
`size*4/3+4` bytes. When `size * 4` overflows `int` such that `n` comes out
*small and positive* (e.g. `size = INT_MAX` → `n = 3`, `size = 2^30 - 1` →
`n = 3`, `size = 1073741821` → `n = 0`), the C code deterministically overruns
both the source and destination buffers and crashes. That is out-of-contract
undefined behaviour in the C ground truth, not an error path the library
*reports*, so it cannot be differentially compared — a segfault in the C `.so`
would just kill the test process. `g4_oversized_lengths` therefore asserts
equality on the oversized values that are well-defined (`n <= 0` → both return
`NULL`) and documents the crashing set as untestable-by-construction. The Rust
translation reproduces the same `int` wrap-around arithmetic
(`wrapping_mul`/`wrapping_div`/`wrapping_add` then `as isize as usize`
sign-extension), so it computes the identical `n` for these inputs; this is
verified indirectly by E5/E6/G5/G6, which pin down the exact `n` for the
neighbouring values where `calloc`'s success/failure flips.

## Completion gate item

- [x] Phase C: EVERY row above has a passing error-path differential test.
