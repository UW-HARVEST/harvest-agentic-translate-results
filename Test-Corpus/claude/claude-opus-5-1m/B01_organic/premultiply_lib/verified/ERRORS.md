# ERRORS.md — Error / rejection surface of the C library

## Mechanical derivation

```sh
grep -nE "return|assert|NULL|if *\(|else|switch|case|errno|error|ERROR|enum|#if|MAX|MIN" \
     c_src/src/lib.c c_src/include/lib.h
```

The grep matches **no** `return` statement, **no** `assert`, **no** `if`,
**no** `switch`, **no** `NULL` comparison, **no** error enum, **no** `errno`
use and **no** `#if`/`#ifdef` in the entire C library. Full inventory of the C
source's control flow and constants:

| construct | location | note |
|-----------|----------|------|
| `for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))` | `lib.c:8` | the **only** branch in the library |
| `sizeof(cp_pixel_t)` == `4` | `lib.c:6`, `lib.c:8` | stride/step constant |
| `255.0f` | `lib.c:9-12`, `lib.c:16-18` | scale constant (not a bound check) |

Consequences that define the whole error surface:

* `premultiply` returns `void`. **There is no error-return channel at all** —
  no error code, no sentinel, no out-parameter. Therefore every "rejection"
  is either
  (a) **implicit**: the loop guard `i < (int)stride * h` is false on entry, so
      the function returns having done nothing, or
  (b) **fatal**: an unchecked dereference faults (`SIGSEGV`).
* `img` is dereferenced unconditionally (`lib.c:4`) — no null check.
* `img->pix` is dereferenced only inside the loop — no null check.
* `w` and `h` are never validated: no non-negativity check, no upper bound, no
  overflow check. `stride = w * sizeof(cp_pixel_t)` is computed in `size_t`
  and truncated back to `int`; `(int)stride * h` is a 32-bit `int`
  multiplication. GCC at `-O0` emits `shl $0x2,%eax` / `imul -0xc(%rbp),%eax`
  (both 32-bit), i.e. **wrapping** 32-bit arithmetic — this is the observable
  ground truth the Rust must reproduce.
* There are **no enum parameters** anywhere in the public header, so the
  "out-of-range enum value across FFI" class collapses to *out-of-range `int`
  values of `w`/`h`*, which rows 6–17 cover exhaustively at the boundaries.

## The error-surface table

Every row is a distinct way the C library rejects, ignores or dies on its
input. `end` denotes `(int)stride * h` == `(int)((int)(w*4) * h)` with 32-bit
wrapping. "no-op" = returns normally with the caller's pixel buffer bitwise
unchanged.

| #  | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|----|----------|------------------------------------------|-------------------|------|-----|
| 1  | `premultiply` | `img == NULL` | no null check at `lib.c:4` → load from address `0x0` → process killed by **`SIGSEGV` (signal 11)**; never returns | `err01_null_img_segv_parity` | [x] |
| 2  | `premultiply` | `img->pix == NULL`, `end > 0` (`w=1,h=1`) | no null check → first loop iteration loads `data[3]` from `0x3` → **`SIGSEGV` (signal 11)** | `err02_null_pix_with_work_segv_parity` | [x] |
| 3  | `premultiply` | `img->pix == NULL`, `end <= 0` (`w=0,h=0`) | loop body never entered → **returns normally**, no fault | `err03_null_pix_no_work_ok` | [x] |
| 4  | `premultiply` | `img->pix` is a wild/non-null dangling pointer, `end <= 0` | never dereferenced → **returns normally** | `err04_wild_pix_no_work_ok` | [x] |
| 5  | `premultiply` | `w == 0` (zero length), `h > 0` | `stride=0` → `end=0` → **no-op** | `err05_zero_w_noop` | [x] |
| 6  | `premultiply` | `h == 0` (zero length), `w > 0` | `end=0` → **no-op** | `err06_zero_h_noop` | [x] |
| 7  | `premultiply` | `w < 0`, `h > 0` (negative dimension) with `\|4·w·h\| < 2³¹` | `stride=4w<0` → `end<0` → loop guard false → **no-op** (silently accepted, *not* an error) | `err07_neg_w_pos_h_noop` | [x] |
| 8  | `premultiply` | `w > 0`, `h < 0` (negative dimension) with `\|4·w·h\| < 2³¹` | `end<0` → **no-op** | `err08_pos_w_neg_h_noop` | [x] |
| 9  | `premultiply` | `w < 0` **and** `h < 0` (both negative) | `end = 4*w*h > 0` → the library **happily processes `w*h` pixels**; no rejection | `err09_neg_w_neg_h_processes` | [x] |
| 10 | `premultiply` | `w == INT_MAX` (`0x7FFFFFFF`), `h == 1` — one step past any sane range | `stride = 4*INT_MAX mod 2^32 = -4` → `end=-4` → **no-op** | `err10_w_int_max_noop` | [x] |
| 11 | `premultiply` | `w == INT_MIN` (`-0x80000000`), `h == 1` | `stride = 0` → `end=0` → **no-op** | `err11_w_int_min_noop` | [x] |
| 12 | `premultiply` | `w == 0x40000000`, any `h` — `w*4` overflows to exactly `0` | `stride=0` → `end=0` → **no-op** | `err12_w_stride_overflow_to_zero_noop` | [x] |
| 13 | `premultiply` | `w == 0x40000001`, `h == 1` — `w*4` wraps to `4` | `stride=4`, `end=4` → **exactly 1 pixel processed** (an "oversized length" that is *not* rejected) | `err13_w_stride_wrap_to_four_processes_one` | [x] |
| 14 | `premultiply` | `w == 0x20000000`, `h == 2` — `end` overflows to `0` | `stride=INT_MIN`, `end=0` → **no-op** | `err14_end_overflow_to_zero_noop` | [x] |
| 15 | `premultiply` | `w == 0x20000000`, `h == 3` — `end` overflows to `INT_MIN` | `end=INT_MIN<0` → **no-op** | `err15_end_overflow_to_int_min_noop` | [x] |
| 16 | `premultiply` | `w == 1`, `h == INT_MAX` (oversized height) | `stride=4`, `end = 4*INT_MAX mod 2^32 = -4` → **no-op** | `err16_h_int_max_noop` | [x] |
| 17 | `premultiply` | `w == 2`, `h == INT_MIN` (oversized/negative height) | `end = 8*INT_MIN mod 2^32 = 0` → **no-op** | `err17_h_int_min_noop` | [x] |
| 18 | `premultiply` | buffer larger than `w*h` pixels (out-of-range index probe) | no bounds check, but the touched extent is **exactly** bytes `[0, 4*w*h)`; every byte at or past `4*w*h` is untouched, and the alpha byte of each touched pixel is untouched | `err18_touched_extent_is_exact` | [x] |
| 19 | `premultiply` | out-of-range enum value across FFI | **N/A** — the public header declares no enum and `premultiply` takes a single pointer parameter. Documented for completeness; the analogous "no valid variant" inputs for the only scalar fields (`w`, `h`) are rows 5–17, and `err19_extreme_scalar_field_sweep` sweeps a 25×25 matrix of extreme `int` values across the FFI boundary. | `err19_extreme_scalar_field_sweep` | [x] |
| 20 | `premultiply` | mixed-sign dimensions whose 32-bit wrap makes `end` **positive** (e.g. `w=-0x3FFFFFFF, h=1` → `end=4`; `w=3, h=-357913941` → `end=4`) | **NOT rejected** — a negative dimension is not always a no-op: the library processes `end/4` pixels. Found by the differential harness, which initially proved a wrong assumption in rows 7/8. | `err20_mixed_sign_wraps_to_positive` | [x] |

## Notes on rows 1–2 (fatal rows)

`SIGSEGV` is an observable, comparable outcome: the differential test re-execs
the test binary as a child process, makes the child call the null-pointer case
through the C `.so` or through the Rust `.so`, and asserts **both children die
with the same signal**. This is a same-error assertion, not merely
"both failed somehow".

## Divergence found and fixed in the Rust translation

Row 1 initially **failed**: the C `.so` died with `SIGSEGV` (11) but the Rust
`.so` died with `SIGABRT` (6), reporting

```
thread '<unnamed>' panicked at src/lib.rs:77:20: null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

Cause: `rustc` inserts a null-pointer check for a raw-pointer place
dereference (`(*img).w`) whenever `-C debug-assertions` is on. The C has no
such check, so the two libraries reported the same invalid input with
*different* signals.

Fix (`src/lib.rs`): the three `cp_image_t` field loads now go through a
byte-wise `c_load` helper built from `core::ptr::read::<u8>` +
`wrapping_add`, and the pixel bytes through `core::ptr::read`/`write` +
`wrapping_offset`. None of those carry a null-ness or alignment precondition,
so the faulting behaviour is now identical to the C **in every build profile**
(verified with `-C debug-assertions=on`, the `dev` profile, and the `release`
profile). The observable arithmetic is unchanged.

## Note on rows 7/8 and row 20

The first draft of rows 7/8 claimed "a negative dimension is always a no-op".
The differential harness disproved it: `w=-1000000, h=1000` gives
`4·w·h ≡ +294967296 (mod 2³²)`, so `end > 0` and pixels *are* processed. Rows
7/8 are now scoped to the non-wrapping magnitudes and row 20 covers the
wrapping ones. The C is the ground truth in both.
