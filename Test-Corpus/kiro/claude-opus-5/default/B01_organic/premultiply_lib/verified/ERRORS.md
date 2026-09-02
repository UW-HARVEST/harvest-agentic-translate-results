# ERRORS.md — error-surface table (Phase A, gate for Phase C)

Mechanically derived by grepping the entire C source for every rejection
mechanism. Result of that grep:

```
$ grep -nE 'return|assert|NULL|errno|-1|error|ERROR|if *\(|switch' c_src/src/lib.c c_src/include/lib.h
(no matches other than the `for` loop condition)
```

There is **no** `return` statement, **no** `assert`, **no** error enum, **no**
`RETURN_ERROR`-style macro, **no** null check, **no** explicit range check and
**no** min/max constant anywhere in `c_src/`. `premultiply` returns `void`.

That does *not* mean there is no error surface. The rejection behaviour is
entirely *implicit*, expressed through the single loop bound

```c
int stride = w * sizeof(cp_pixel_t);      /* size_t multiply, truncated to int */
for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
```

Whenever `(int)((int)(w*4) * h) <= 0` the loop body never executes and the call
is a silent no-op — that is how this API "rejects" bad geometry. Each distinct
way of reaching a non-positive bound, plus each pointer/aliasing precondition,
is one row below.

`expected C result` is stated as the observable effect, since there is no
return value: either **no-op** (buffer bit-identical after the call) or
**faults** (SIGSEGV, undefined behaviour).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `premultiply` | `w == 0`, any `h` → `stride == 0` → bound `0` | no-op; buffer unchanged |
| 2 | `premultiply` | `h == 0`, any `w` → bound `0` | no-op; buffer unchanged |
| 3 | `premultiply` | `w == 0 && h == 0` | no-op; buffer unchanged |
| 4 | `premultiply` | `w < 0`, `h > 0` → `stride < 0` → bound `< 0` | no-op; buffer unchanged |
| 5 | `premultiply` | `w > 0`, `h < 0` → bound `< 0` | no-op; buffer unchanged |
| 6 | `premultiply` | `w < 0 && h < 0` → bound `> 0`; loop runs and walks **forward** over `|w*4*h|` bytes despite both dimensions being negative | writes `w*h` pixels' worth of bytes (NOT a no-op) |
| 7 | `premultiply` | `w == INT_MIN` (`h > 0`) → `size_t` multiply truncates to `int` `0` → bound `0` | no-op |
| 8 | `premultiply` | `w == 0x2000_0000` (2^29) → `w*4` truncates to `0x8000_0000` = `INT_MIN` < 0 → bound `<= 0` | no-op |
| 9 | `premultiply` | `w == 0x4000_0000` (2^30) → `w*4` truncates to `0` → bound `0` | no-op |
| 10 | `premultiply` | `w == 0x2000_0001` → `w*4` truncates to `0x8000_0004` < 0 | no-op |
| 11 | `premultiply` | `w == 0x1000_0000` (2^28), `h == 2` → `stride = 0x4000_0000` > 0 but `stride*h` overflows `int` to `INT_MIN` < 0 | no-op (signed-overflow wrap, matches gcc `imul`) |
| 12 | `premultiply` | `h == INT_MIN`, **any** `w` → `stride` is always `w*4` hence always even, and any even multiple of `INT_MIN` wraps to exactly `0` in 32-bit, so the bound is `0` unconditionally | no-op for every `w` |
| 13 | `premultiply` | `w == 3`, `h == 0x4000_0001` (2^30+1) → `stride = 12`, `12 * (2^30+1) = 3*2^32 + 12` wraps to `int` `12` | loop runs over the wrapped byte count only: exactly 3 pixels touched, not `3 * (2^30+1)` |
| 14 | `premultiply` | `img->pix == NULL` **with** a non-positive bound (e.g. `w==0`/`h==0`) | no-op; `pix` never dereferenced, no fault |
| 15 | `premultiply` | `img->pix == NULL` with a positive bound | dereferences NULL → SIGSEGV (UB) |
| 16 | `premultiply` | `img == NULL` | dereferences NULL at `img->w` → SIGSEGV (UB) |
| 17 | `premultiply` | `img->pix` buffer shorter than `w*h` pixels (bound exceeds allocation) | reads/writes past the end of the caller's array; UB, but on x86-64 it is a plain unchecked byte walk with no bounds test |
| 18 | `premultiply` | `img->pix` misaligned (not 4-byte aligned) | accepted; access is byte-wise (`uint8_t *`), no alignment requirement, no fault |
| 19 | `premultiply` | alpha byte `data[i+3]` is never written | alpha channel left untouched for every pixel — not "fixed" |

Notes on how rows 15/16 are tested: a differential test cannot simply call and
compare, since both implementations fault. They are exercised in a forked child
process and the *signal disposition* of C and Rust is compared, so "both
implementations fault identically" is asserted rather than assumed. Row 17 is
tested with a deliberately over-allocated backing buffer so that the
out-of-bounds walk stays inside a live mapping and the byte-for-byte comparison
is meaningful and reproducible.

## Boundary cases required by Phase C regardless of the table

| # | case | note |
|---|------|------|
| B1 | `img == NULL` | row 16 |
| B2 | `img->pix == NULL` | rows 14, 15 |
| B3 | zero length (`w==0`, `h==0`) | rows 1–3 |
| B4 | oversized length (`w`/`h` near `INT_MAX`) | rows 7–13 |
| B5 | one step past a valid range (`w = 2^29`, `2^29+1`, `2^30`) | rows 8–10 |
| B6 | out-of-range enum across FFI | **not applicable** — `lib.h` declares no `enum` and no flag/mode parameter; the only parameter is a struct pointer. The equivalent "any bit pattern is a legal C value" case for this API is an arbitrary `int` in `w`/`h`, which is covered exhaustively-by-sampling in row set 7–13 and by the randomized `i32` fuzz test. |

## Check-off — every row has a passing differential test

Tests live in `tests/phase_c_error_paths.rs` and drive both `.so` files through
their exported `premultiply` symbol via `libloading`.

| row | test | [x] |
|---|---|---|
| 1 | `err01_width_zero` | [x] |
| 2 | `err02_height_zero` | [x] |
| 3 | `err03_both_zero` | [x] |
| 4 | `err04_negative_width_positive_height` | [x] |
| 5 | `err05_positive_width_negative_height` | [x] |
| 6 | `err06_both_negative_is_not_a_noop` | [x] |
| 7 | `err07_width_int_min` | [x] |
| 8 | `err08_width_two_pow_29` | [x] |
| 9 | `err09_width_two_pow_30` | [x] |
| 10 | `err10_width_two_pow_29_plus_one` | [x] |
| 11 | `err11_stride_times_height_overflows` | [x] |
| 12 | `err12_height_int_min` | [x] |
| 13 | `err13_bound_wraps_to_small_positive` | [x] |
| 14 | `err14_null_pix_with_nonpositive_bound` | [x] |
| 15 | `err15_null_pix_with_positive_bound_faults_identically` (child process, signal compared) | [x] |
| 16 | `err16_null_img_faults_identically` (child process, signal compared) | [x] |
| 17 | `err17_bound_exceeds_logical_array` | [x] |
| 18 | `err18_misaligned_pix_accepted` | [x] |
| 19 | `err19_alpha_never_written` | [x] |
| B1–B5 | `boundary_full_i32_domain_sampling`, `boundary_zero_length_buffer`, `boundary_dangling_pix_with_zero_bound` | [x] |

## Divergence found and fixed

One real divergence was found by row 16. It is invisible to any happy-path test
and only appears when UB checks are enabled:

* **`img == NULL` aborted instead of segfaulting.** The C dereferences `img->w`
  immediately and dies with `SIGSEGV` (signal 11). The Rust used a plain place
  deref `(*img).w`, which rustc instruments under `-C debug-assertions=on` (the
  default for the `dev` profile) with a null check that raises a non-unwinding
  panic — `SIGABRT` (signal 6). Same "it crashes", different observable
  termination.

  Fix: read the three header fields with `read_volatile` applied to the field
  *address* (`addr_of!((*img).w)`), which is not instrumented. Same values, same
  order (`w`, then `h`, then `pix`, exactly as the C), and the null case now
  faults with `SIGSEGV` in every profile. Verified with
  `RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"` as well as with the
  stock `dev` and `release` profiles.
