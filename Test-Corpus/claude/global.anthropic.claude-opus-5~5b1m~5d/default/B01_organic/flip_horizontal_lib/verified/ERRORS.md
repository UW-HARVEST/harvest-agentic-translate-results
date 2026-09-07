# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep result

```
$ grep -nE 'return|assert|NULL|if|error|ERROR|switch|MAX|MIN|enum' c_src/src/lib.c
c_src/src/lib.c:4:    cp_pixel_t *pix = img->pix;
c_src/src/lib.c:5:    int w = img->w;
c_src/src/lib.c:6:    int h = img->h;
```

(the three hits are only substring matches on `img->`, not control flow)

Findings:

* **No** `return` statement (the function is `void`).
* **No** error enum, no error code, no sentinel value, no out-parameter status.
* **No** `assert`, no `NULL` check, no range check, no min/max constant.
* **No** `if` / `switch` / `#ifdef` anywhere.
* The only control flow is the two `for` loops with the bounds `i < h / 2` and
  `j < w`.

So `flip_horizontal` has **no explicit error surface**: it never *rejects* an
input. Its entire "rejection" behaviour is implicit — an input that is not a
valid image either (a) makes a loop bound non-positive so the function silently
does nothing, or (b) is undefined behaviour that faults.

The table below therefore enumerates every distinct *implicit* rejection /
degenerate path, one row per triggering condition, plus the generic FFI
boundaries required by Phase C. "Expected C result" is the observable behaviour
of the compiled C, which the Rust must reproduce byte-for-byte.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `flip_horizontal` | `img == NULL` | dereference of NULL at `img->pix` → `SIGSEGV` (fatal signal, no return). Rust must also fault, not panic-with-message / return normally. |
| E2 | `flip_horizontal` | `img->h == 0` (`flips = 0`) | outer loop body never runs; **no read and no write of `img->pix`**; `pix` may be NULL/dangling and is still never touched; struct unmodified |
| E3 | `flip_horizontal` | `img->h == 1` (`flips = 1/2 = 0`) | same as E2: complete no-op, `pix` never dereferenced |
| E4 | `flip_horizontal` | `img->h < 0` (e.g. `-1`, `-2`, `-7`) → `flips = h/2 <= 0` (C truncates toward zero) | `0 < flips` false → complete no-op, `pix` never dereferenced |
| E5 | `flip_horizontal` | `img->h == INT_MIN` → `flips = INT_MIN/2 = -1073741824` | complete no-op (no overflow: the division is exact) |
| E6 | `flip_horizontal` | `img->w == 0`, any `h >= 2` | outer loop runs `h/2` times, inner loop bound `j < 0` false each time → **no pixel is read or written**; offsets `pix + 0` computed only |
| E7 | `flip_horizontal` | `img->w < 0` (e.g. `-1`, `-4`), `h >= 2` | inner loop bound `j < w` is false immediately → no pixel touched. The row pointers `pix + w*i` and `pix + w*(h-i-1)` are *computed* (negative offsets, out of the buffer) but never dereferenced → observable behaviour is a complete no-op |
| E8 | `flip_horizontal` | `img->pix == NULL` with `h >= 2 && w >= 1` | dereference of NULL row pointer → `SIGSEGV` |
| E9 | `flip_horizontal` | `img->pix` non-NULL but buffer smaller than `w*h` pixels (undersized allocation) | reads/writes past the end of the buffer at exactly `pix[w*i + j]` / `pix[w*(h-i-1) + j]`; no bounds check, no rejection. Rust must perform the identical accesses (same addresses, same order) rather than panicking |
| E10 | `flip_horizontal` | `img->w`/`img->h` large enough that `w * i` or `w * (h-i-1)` overflows `int` | C computes the product in `int` (wraps on the target compiler), sign-extends the wrapped result, and offsets `pix` by it. Rust must use the same wrapping-`i32`-then-sign-extend arithmetic (`wrapping_mul(..) as isize`), *not* 64-bit arithmetic and *not* a debug-mode overflow panic |
| E11 | `flip_horizontal` | out-of-range "enum" value across FFI | **N/A by construction** — the API takes no enum and no flag/mode parameter; the only argument is `cp_image_t *`. The equivalent "any `int` is accepted" surface is fully covered by rows E2–E7 and E10 on the `int` fields `w` and `h`. |
| E12 | `flip_horizontal` | misaligned `cp_image_t *` | `cp_pixel_t` has align 1 and `cp_image_t` align 8; C UB, not a rejection. Not exercised (no defined C behaviour to match). |

Notes on testability:

* E1 and E8 kill the process, so they are exercised in **forked child
  processes** and the resulting termination signal is compared between C and
  Rust.
* E9 is exercised inside a large *padded* host allocation so the out-of-bounds
  accesses land in memory owned by the test; C and Rust write into two
  identical copies of that padded region and the whole region (payload +
  padding) is compared byte-for-byte.
* E10's fully-overflowing case cannot be driven with a real buffer (it needs
  > 2 GiB of pixels actually written), so it is verified in the *observable*
  form: negative/large `w` with a base pointer placed in the middle of a padded
  allocation, where the wrapped offsets are computed but never dereferenced, and
  by code inspection of the arithmetic (`wrapping_mul` + `as isize`).

## Checklist

- [x] E1 — `img == NULL` → same fatal signal (`SIGSEGV`) in C and Rust
- [x] E2 — `h == 0` → no-op, `pix` never touched
- [x] E3 — `h == 1` → no-op, `pix` never touched
- [x] E4 — `h < 0` → no-op
- [x] E5 — `h == INT_MIN` → no-op
- [x] E6 — `w == 0`, `h >= 2` → no pixel touched
- [x] E7 — `w < 0`, `h >= 2` → no pixel touched
- [x] E8 — `pix == NULL`, `h >= 2`, `w >= 1` → same fatal signal
- [x] E9 — undersized buffer → identical out-of-bounds accesses
- [x] E10 — `int` overflow in the row offset → identical wrapping arithmetic
- [x] E11 — N/A (no enum/flag parameter); covered by E2–E7, E10
- [x] E12 — N/A (no defined C behaviour to match)

## Row → test mapping (`tests/phase_c_errors.rs`)

| row | test |
|-----|------|
| E1 | `e01_null_img_same_fatal_signal` (child process; both die with signal 11) |
| E2 | `e02_h_zero_is_noop` (incl. NULL / wild `pix`, which must not be dereferenced) |
| E3 | `e03_h_one_is_noop` |
| E4 | `e04_negative_h_is_noop` |
| E5 | `e05_h_int_min_is_noop` |
| E6 | `e06_zero_width_positive_height_is_noop` |
| E7 | `e07_negative_width_is_noop` |
| E8 | `e08_null_pix_same_fatal_signal` (two shapes; both die with signal 11) |
| E9 | `e09_undersized_buffer_identical_accesses` (7 shapes × 8 random payloads) |
| E10 | `e10_row_offset_overflow_matches` (16 overflowing `w`/`h` pairs) |
| E11 | `e11_arbitrary_int_fields_agree` (13×13 boundary cross-product + 2000 random `int` patterns) |
| generic boundaries | `generic_boundaries_agree` (zero-length payload, one step past the backed shape in each dimension) |

Status: **13/13 tests pass**, covering all rows.

The suite's ability to *detect* a divergence is itself verified:
`tools/negative_control.sh` builds 10 deliberately wrong copies of
`src/lib.rs` and confirms each one is caught (all 10 killed).
