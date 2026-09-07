# ERRORS.md — Phase A: error / rejection surface table

## Mechanical derivation

Greps run over the whole C tree (`c_src/include/lib.h`, `c_src/src/lib.c`):

| pattern | hits |
|---|---|
| `return` | 0 |
| `return -1` / `return NULL` / `return 0` | 0 |
| `RETURN_ERROR` / `GOTO` / `goto` | 0 |
| `assert` / `NDEBUG` | 0 |
| `errno` | 0 |
| `if` / `switch` / `?:` | 0 |
| `#ifdef` / `#if` | 0 |
| `enum` | 0 |
| `MIN` / `MAX` / `_MAX` / `_MIN` constants | 0 |
| `NULL` checks | 0 |
| explicit range checks | 0 |

`premultiply` is declared `void` and contains **no** explicit rejection path: no
error code, no sentinel return, no assertion, no null check, no bounds check.
Consequently the "error surface" of this library consists entirely of *implicit*
rejections — inputs for which the single loop bound `(int)stride * h` evaluates
to `<= 0` and the function therefore performs **zero work and zero memory
accesses** — plus the undefined-behaviour cases that both languages must handle
the same way at the ABI level.

The rows below are derived from the only two expressions in the C that can
"reject" work:

```c
int stride = w * sizeof(cp_pixel_t);            /* line 6  */
for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))   /* line 8 */
```

`sizeof` has type `size_t` (unsigned 64-bit on this target), so `w` is converted
to `size_t` (sign-extended), multiplied by 4, and the `size_t` result is
truncated back to `int` on assignment to `stride` — observationally identical to
a wrapping 32-bit `w * 4`. `(int)stride * h` is a signed 32-bit multiply whose
overflow wraps in practice on the build used here. `i += sizeof(...)` likewise
promotes `i` to `size_t`, adds 4, and truncates back to `int`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | differential test (`tests/phase_c_errors.rs`) | [x] |
|---|----------|----------------------------------------------|-------------------|---|---|
| 1 | `premultiply` | `img->w == 0` (any `h`, any `pix`, incl. `pix == NULL`) | `stride = 0` → bound `0` → loop body never runs; **no pixel written, no dereference of `pix`**; returns normally | `row01_w_zero` | [x] |
| 2 | `premultiply` | `img->h == 0` (any `w`, any `pix`, incl. `pix == NULL`) | bound `stride * 0 == 0` → loop body never runs; **no write, no deref of `pix`**; returns normally | `row02_h_zero` | [x] |
| 3 | `premultiply` | `img->w < 0`, `img->h > 0` (e.g. `w = -4, h = 3`) | `stride = 4*w < 0`, bound `stride*h < 0` → `0 < negative` false → **no iterations**, no deref; returns normally | `row03_negative_w_positive_h` | [x] |
| 4 | `premultiply` | `img->h < 0`, `img->w > 0` (e.g. `w = 4, h = -3`) | bound `stride*h < 0` → **no iterations**, no deref; returns normally | `row04_positive_w_negative_h` | [x] |
| 5 | `premultiply` | **both** `img->w < 0` **and** `img->h < 0` (e.g. `w = -1, h = -1`) | double negation makes the bound **positive**: bound `= (4*w)*h > 0` → the loop **does** run for `w*h` pixels starting at `pix[0]`, premultiplying them. Must be replicated, not "fixed" | `row05_both_negative_performs_work` | [x] |
| 6 | `premultiply` | `img->w` such that `w * 4` overflows `int` (e.g. `w = 0x2000_0000` → `stride = 0`) | `stride` wraps to `0` → bound `0` → **no iterations** | `row06_stride_overflow_to_zero` | [x] |
| 7 | `premultiply` | `img->w` such that `w * 4` wraps to a **negative** `stride` (e.g. `w = 0x1000_0000` → `stride = 0x4000_0000`; `w = 0x30000000` → `stride = 0xC0000000` = negative), with `h > 0` | bound negative → **no iterations** | `row07_stride_wraps_negative` | [x] |
| 8 | `premultiply` | `stride * h` overflows to exactly `0` (e.g. `w = 0x10000`, `h = 0x4000` → `stride = 0x40000`, `stride*h = 2^32 → 0`) | **no iterations**, no deref of `pix` | `row08_bound_overflow_to_zero` | [x] |
| 9 | `premultiply` | `stride * h` overflows to a **negative** value (e.g. `w = 1000, h = 1_000_000` → `4_000_000 * 1_000_000` wraps negative) | **no iterations**, no deref of `pix` | `row09_bound_overflow_negative` | [x] |
| 10 | `premultiply` | `stride * h` overflows to a **small positive** value (e.g. `w = 0x10000`, `h = 0x4001` → wraps to `0x40000`) | loop runs for the **wrapped** (much smaller) count only | `row10_bound_overflow_small_positive` | [x] |
| 11 | `premultiply` | `img->pix == NULL` **with** a zero bound (rows 1–4, 6–9) | safe: `pix` is never dereferenced; both implementations must return normally without faulting | `row11_null_pix_with_zero_bound` | [x] |
| 12 | `premultiply` | `img->pix == NULL` **with** a positive bound (`w > 0, h > 0`) | C dereferences a null pointer → **SIGSEGV**. UB in both languages; parity asserted at the *process-signal* level (both must die with the same signal) | `row12_null_pix_with_positive_bound_same_signal` | [x] |
| 13 | `premultiply` | `img == NULL` | C dereferences `img->w` immediately → **SIGSEGV**. UB in both languages; parity asserted at the *process-signal* level | `row13_null_img_same_signal` | [x] |
| 14 | `premultiply` | `img->w`/`img->h` describe **more** pixels than the buffer holds (out-of-range index, e.g. `w = 8, h = 8` over a 4-pixel buffer) | C reads/writes past the end of the allocation. UB; behaviour is "walk `w*h` pixels regardless". Verified with a deliberately over-sized *padded* allocation so both implementations touch and must agree on the same in-bounds bytes | `row14_out_of_range_index` | [x] |
| 15 | `premultiply` | `img->w == INT_MIN` (`stride = INT_MIN*4 = 0`) with any `h` | `stride` wraps to `0` → bound `0` → **no iterations** | `row15_w_int_min` | [x] |
| 16 | `premultiply` | `img->h == INT_MIN` with `w > 0` | bound `stride * INT_MIN` wraps; for `stride` a multiple of 4 the product is `0` → **no iterations** | `row16_h_int_min` | [x] |
| 17 | `premultiply` | out-of-range "enum"-like values across the FFI boundary | **N/A — the C API declares no enum type.** `cp_image_t` has only `int w`, `int h`, `cp_pixel_t *pix`; the entire `int` range for `w`/`h` is a legal FFI input and is covered by rows 1–10 and 15–16, plus randomized full-`i32`-range fuzzing in Phase C | `row17_no_enum_in_api_full_int_range_instead` | [x] |

### Non-error notes (documented so the table is provably exhaustive)

* There is no allocation in `premultiply`, hence no allocation-failure path.
* There is no I/O, hence no `errno` path.
* Pixel *values* can never make the function fail: every channel is a `uint8_t`,
  so `v/255.0f ∈ [0,1]`, `r*a*255.0f ∈ [0,255]`, and the `(uint8_t)` conversion
  is always in range (never the UB out-of-range float→int conversion).
  `a == 0` and `a == 255` are ordinary valid inputs, covered in `CONFIGS.md`.

## Phase C status

All 17 rows have a passing differential test. Additionally `generic_boundaries`
sweeps the 32x32 cross-product of the interesting `w`/`h` values (`INT_MIN`,
`INT_MIN+1`, +/-2^30, +/-2^16, 0, +/-1, 254..257, 2^29, 2^30, `INT_MAX-1`,
`INT_MAX`, and one step past each) against both libraries -- with `pix = NULL`
for every non-positive bound, and with a real padded buffer for every small
positive (wrapped) bound.

Run:

```
cargo test --test phase_c_errors      # 18 tests, all pass
bash verify.sh                        # every profile x C optimization level
```
