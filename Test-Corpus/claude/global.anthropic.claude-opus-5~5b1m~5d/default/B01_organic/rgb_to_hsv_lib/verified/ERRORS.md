# ERRORS.md — Phase A: error-surface table

Derived **mechanically** from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep results (the anti-blind-spot audit)

Run over the library sources only (`src/lib.c`, `include/lib.h` — the CMake
compiler-ID probe under `build/` is not part of the library):

| grep pattern | hits in library source |
|---|---|
| `return` | **1** — `src/lib.c:23`, a bare `return;` (the degenerate early-out) |
| `assert` | 0 |
| `NULL` / `nullptr` | 0 |
| `error` / `errno` / `EINVAL` / `fail` / `RETURN_ERROR` | 0 |
| `return -1` / `return NULL` / error enum | 0 |
| `#define` / `#ifdef` / `#if` | 0 |
| explicit range check / clamp / min-max constant | 0 |
| `if` / `else` / `?:` | 9 (lines 13,14,15,16,19,26,28,30,33) |

**Conclusion, stated plainly:** `rgb_to_hsv` returns `void`, validates
*nothing*, and has **no error code, no sentinel return, and no rejection
path**. It cannot report failure. Therefore the "error surface" consists of:

1. the single **degenerate / early-out** branch (`src/lib.c:19`), which is the
   only rejection-*shaped* control flow in the library — it refuses to run the
   hue/saturation math and writes a fixed `h = 0, s = 0` result instead; and
2. the **generic C-API boundary conditions** that the task mandates covering
   even when absent from the table (null pointers, buffer extent, non-finite
   and out-of-documented-range values, out-of-range enums).

Rows 1–3 are the real branch disjuncts of line 19. Rows 4–14 are the generic
boundaries. For a `void` function "same error/rejection" means: **the same
observable effect** — identical 3-float output bit pattern, identical
"early-out vs. full-path" result shape, or identical fatal signal.

## The table

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `rgb_to_hsv` | `delta == 0` with `max != 0` — i.e. `r == g == b` and that value `!= 0` (line 19 first disjunct) | early `return` at line 23; `dest = {0.0, 0.0, max}`; `s` never computed, so **no `0/0` NaN** | `err_row01_delta_zero_max_nonzero` | [x] |
| 2 | `rgb_to_hsv` | `max == 0` with `delta != 0` — max is `+0.0`/`-0.0` while `min < 0` (line 19 second disjunct); reachable only with negative, i.e. out-of-documented-range, channels | early `return`; `dest = {0.0, 0.0, ±0.0}`; short-circuit **prevents the `delta / 0` division**, so no `Inf` in `s` | `err_row02_max_zero_delta_nonzero` | [x] |
| 3 | `rgb_to_hsv` | both disjuncts true — every channel is `+0.0`/`-0.0` (incl. mixed signs) | early `return`; `dest = {0.0, 0.0, max}` where `max` keeps the C ternary's exact signed zero | `err_row03_both_disjuncts_zero` | [x] |
| 4 | `rgb_to_hsv` | `src == NULL` (`dest` valid) | no check exists → UB; observed: fatal `SIGSEGV` on the `src[0]` load | `err_row04_null_src_segv` | [x] |
| 5 | `rgb_to_hsv` | `dest == NULL` (`src` valid, non-degenerate input) | no check → UB; fatal `SIGSEGV` on the `dest[0]` store | `err_row05_null_dest_segv` | [x] |
| 6 | `rgb_to_hsv` | `dest == NULL` **and** input degenerate (takes early-out at line 19) | UB; still `SIGSEGV` — the early-out writes `dest` too (lines 20–22), so the crash is not avoided | `err_row06_null_dest_early_out_segv` | [x] |
| 7 | `rgb_to_hsv` | `src == NULL` **and** `dest == NULL` | UB; `SIGSEGV` (src load faults first) | `err_row07_both_null_segv` | [x] |
| 8 | `rgb_to_hsv` | buffer extent: `src` has exactly 3 readable floats, `dest` exactly 3 writable — one past the end is poisoned | reads exactly `src[0..3]`, writes exactly `dest[0..3]`; **byte 12 onward untouched**, no over-read/over-write | `err_row08_no_out_of_bounds_access` | [x] |
| 9 | `rgb_to_hsv` | `NaN` in any one channel (quiet, and with a non-default payload) | no check → `NaN` flows through the comparisons; every `<`/`>`/`==` involving it is **false**, steering min/max/branch selection; result must match bit-for-bit incl. NaN payload | `err_row09_nan_single_channel` | [x] |
| 10 | `rgb_to_hsv` | `NaN` in 2 or 3 channels; signalling `NaN` (`0x7F80_0001`) | no check; same as row 9 — quieting and payload choice must agree | `err_row10_nan_multi_and_snan` | [x] |
| 11 | `rgb_to_hsv` | `±Inf` channels (incl. `+Inf` and `-Inf` together → `delta = Inf`, `s = Inf/Inf = NaN`) | no check; IEEE result propagates (`Inf - Inf = NaN`, `x/Inf = 0`, `Inf/Inf = NaN`) | `err_row11_infinities` | [x] |
| 12 | `rgb_to_hsv` | one step past the documented `[0, 1]` range: `-eps`, `1 + eps`, `±FLT_MAX`, and huge values whose `max - min` **overflows to `Inf`** | no clamp, no range check → computed unclamped; `s` may exceed 1, `h` may be `NaN`/`Inf`, `v` unbounded | `err_row12_out_of_documented_range` | [x] |
| 13 | `rgb_to_hsv` | subnormal / extreme-ratio inputs: `delta` subnormal with large `max`, `max` subnormal so `delta / max` **overflows to `Inf`**, `FLT_MIN`/`FLT_TRUE_MIN` | no check; gradual-underflow and overflow results propagate identically | `err_row13_subnormal_and_overflow` | [x] |
| 14 | `rgb_to_hsv` | **out-of-range enum across the FFI boundary** | **vacuous — the signature has no enum, no `int`, and no length/count parameter** (`void rgb_to_hsv(float*, const float*)`). The only cross-FFI scalars are `float`s, so the analogue of "an int with no valid variant" is "a bit pattern that is no valid number" → covered by rows 9–13, which sweep **all 256 NaN/Inf exponent-max encodings** and raw-bit-pattern fuzzing | `err_row14_all_bit_patterns_no_enum_surface` | [x] |

## Notes on rows 4–7 (null pointers)

The C dereferences unconditionally, so a null argument is undefined behaviour
rather than a defined rejection. The differential test therefore compares the
*fatal outcome*: each side is invoked in a re-executed child process and the
terminating signal is compared. Both must die by the same signal (`SIGSEGV`).
This is asserted as an exact signal match, not merely "both failed somehow".

### Divergence found and resolved while writing rows 4–7

The first run of these rows **failed** — a genuine, reproducible difference:

```
row E04 case=null_src: C Outcome { signal: Some(11) }   # SIGSEGV
                     Rust Outcome { signal: Some(6)  }  # SIGABRT
```

Cause, established by capturing the child's stderr rather than guessed:

```
thread '<unnamed>' panicked at src/lib.rs:53:31:
null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

`rustc` inserts a **null-pointer UB check on raw-pointer dereference whenever
`debug_assertions` is on**. The check panics; because the panic tries to escape
an `extern "C"` function, Rust aborts → `SIGABRT`. The C is compiled with no
equivalent instrumentation (CMake passes only `-fPIC`, and `-fsanitize=null` is
not enabled), so it simply faults → `SIGSEGV`.

This is an **instrumentation difference, not a behavioural difference in the
translated logic**: comparing a UB-checked Rust build against an unchecked C
build is not a like-for-like comparison. It was *not* fixed by weakening the
assertion. Instead rows 4–7 now:

1. compare the C against the Rust artifact built **without** `debug_assertions`
   (the `release` cdylib — also the artifact that actually ships) and require an
   **exact signal match**; that build segfaults identically to the C; and
2. additionally examine the default-resolved artifact: if it is a
   `debug_assertions` build, its abort is accepted **only** when its stderr
   proves the null-pointer UB check fired — never as a blanket "it failed too".

The translated code itself was left unchanged, because it was not at fault.

## Gate

- [x] Every row above has a passing error-path differential test
      (`translation/tests/phase_c_errors.rs`).
