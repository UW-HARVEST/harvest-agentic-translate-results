# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every line of `c_src/src/lib.c` was scanned for rejection constructs:

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|enum|#if|<|>|==|!=' c_src/src/lib.c
```

Findings, exhaustively:

- **Error-return macros** (`RETURN_ERROR`, `CHECK`, `TRY`, …): **none**. The file
  includes only `<math.h>` and `"lib.h"`; no macros are defined or used.
- **`return` statements**: exactly one, line 14 — a bare `return;` from a `void`
  function. It is a **fast path, not a rejection**: it is taken when `s == 0`
  and it *writes a full, valid result* (`dest[0..2] = l`) before returning.
- **`assert`**: none. `<assert.h>` is not included.
- **Error enums / sentinel values / `errno` / `NULL` returns**: none. The
  function returns `void`, so there is **no channel through which it can report
  an error**.
- **Null-pointer checks**: none. `src` is dereferenced unconditionally at lines
  6–8 and `dest` is written unconditionally on every path.
- **Explicit range checks on values**: the six hue comparisons on lines 19–39
  and the `s == 0` test on line 10. None of these reject; each one *selects an
  output formula*, and the terminal `else` (lines 43–46) is a total fallback.
  They are therefore configuration axes, and live in `CONFIGS.md`.
- **Min/max constants**: `0.0f`, `60.0f`, `120.0f`, `180.0f`, `240.0f`,
  `300.0f`, `360.0f`, `0.5f`, `1.0f`, `2.0f`. Confirmed against the compiled
  `.rodata` (`3f800000`=1.0, `3f000000`=0.5, `42700000`=60, `40000000`=2,
  `42f00000`=120, `43340000`=180, `43700000`=240, `43960000`=300,
  `43b40000`=360, plus the `7fffffff` `andps` mask for `fabsf`). These are
  branch thresholds, not validity limits.
- **Length / count / size parameters**: none. The arity is fixed at 3 `float`s
  in and 3 `float`s out, implied by the code, not passed in. So there is no
  "zero length" or "oversized length" input to construct.
- **Enum parameters crossing the FFI boundary**: none. Both parameters are
  `float *`. There is no `int`-backed enum, so there is no "out-of-range enum
  variant" input to construct.

**Conclusion: the C function has ZERO explicit rejection paths.** It is a total
function over `float[3]` — every one of the 2^96 possible inputs produces a
defined, non-erroring write of 3 floats. Consequently the error surface consists
of (a) the *domain boundaries* that a caller would reasonably call "invalid
input" but which the C silently accepts and must therefore be matched
bit-for-bit, and (b) the *memory-safety preconditions* the C leaves unchecked,
whose violation must fault identically. Both classes are enumerated and tested
below.

## Error-surface table

Rows 1–14: inputs a caller could consider invalid/out-of-range. The C performs
no check, so the "expected C result" is the exact value it silently produces;
the differential test asserts the Rust produces the *same bit pattern*, not
merely "also didn't error".

Rows 15–18: unchecked memory-safety preconditions. Expected result is the same
fatal signal from both libraries, asserted from a forked child process.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|----------------------------------------------|-------------------|------|-----|
| 1  | `hsl_to_rgb` | `h` below the documented range: `h < 0` (e.g. `-1.0`, `-0.0` is *not* below) with `s != 0` | No rejection. Falls past arms 1–2, and arm 3's predicate `h < 120 && h < 180` is TRUE, so it returns `(m, c+m, x+m)` — the arm the source author meant for `[120,180)`. Bit-exact match required. | `err_row01_negative_hue` | [x] |
| 2  | `hsl_to_rgb` | `h` one step past the top of the documented range: `h >= 360` (`360.0`, `nextafter(360,inf)`, `1e30`) with `s != 0` | No rejection. All six arms false → terminal `else` → `(m, m, m)`. | `err_row02_hue_at_or_above_360` | [x] |
| 3  | `hsl_to_rgb` | `h` in `[120, 180)` — the range the buggy arm-3 predicate fails to claim | No rejection. Arms 1,2 false; arm 3 false (`h < 120` is false); arms 4,5,6 false → terminal `else` → flat grey `(m, m, m)`. Must NOT be "fixed" to `(m, c+m, x+m)`. | `err_row03_hue_120_to_180_dead_range` | [x] |
| 4  | `hsl_to_rgb` | `h = NaN` (quiet, both signs, several payloads) with `s != 0` | No rejection. `comiss` sets PF on unordered, so every one of the six predicates is false → terminal `else` → `(m, m, m)`. `x` is NaN but is discarded. | `err_row04_hue_nan` | [x] |
| 5  | `hsl_to_rgb` | `h = +inf` / `h = -inf` with `s != 0` | No rejection. `h/60 = ±inf`; glibc `fmodf(±inf, 2)` returns NaN (and sets `EDOM`, which is never read). Predicates: `+inf` fails all six → `(m,m,m)`; `-inf` satisfies arm 3 → `(m, c+m, x+m)` with `x = NaN`. | `err_row05_hue_infinite` | [x] |
| 6  | `hsl_to_rgb` | `s` out of `[0,1]`: `s < 0` or `s > 1`, and `s = ±inf` | No rejection, no clamping. `c = (1-\|2l-1\|)*s` is computed and propagated as-is, producing out-of-gamut or infinite/NaN channels. | `err_row06_saturation_out_of_range` | [x] |
| 7  | `hsl_to_rgb` | `s = NaN` (both signs, several payloads) | No rejection. `s == 0` is false for NaN (unordered), so the fast path is skipped. `c = mulss(1-\|2l-1\|, s)`: dest is non-NaN so the *source* NaN `s` is returned quieted. Exact sign+payload of every output channel must match. | `err_row07_saturation_nan` | [x] |
| 8  | `hsl_to_rgb` | `s = -0.0f` (a negative zero passed where "no saturation" is meant) | No rejection, and **the fast path IS taken**: IEEE `-0.0 == 0` is true, so `ucomiss` reports equal → `dest[0..2] = l` (returning `l` unchanged even if `l` is itself out of range or NaN). | `err_row08_saturation_negative_zero` | [x] |
| 9  | `hsl_to_rgb` | `l` out of `[0,1]`: `l < 0` or `l > 1` | No rejection, no clamping. `\|2l-1\| > 1` → `c < 0` → negative/out-of-gamut channels emitted verbatim. | `err_row09_lightness_out_of_range` | [x] |
| 10 | `hsl_to_rgb` | `l = ±inf` with `s != 0` | No rejection. `2l-1 = ±inf`, `\|·\| = inf`, `1-inf = -inf`, `c = -inf*s` (`±inf`), `m = l - 0.5*c` → `inf - inf = NaN` for some sign combinations. Whatever the C emits (including NaN sign/payload) must match. | `err_row10_lightness_infinite` | [x] |
| 11 | `hsl_to_rgb` | `l = NaN` (both signs, several payloads) with `s != 0` | No rejection. `c`, `m` and `x` all become NaN but with **different signs**: `fabsf` is an `andps`, so `c`/`x` carry sign 0 while `m` re-propagates the original `l`, keeping its sign bit. The output therefore depends on which operand is the SSE *destination* of each `addss`. Bit-exact match required. | `err_row11_lightness_nan` | [x] |
| 12 | `hsl_to_rgb` | Signalling NaN (`0x7fa0_0000` / `0xffa0_0000`) in any of `h`, `s`, `l` | No rejection, no trap (SSE exceptions are masked). The sNaN is quieted by the first arithmetic op that consumes it (`\|0x0040_0000`), preserving sign and payload. | `err_row12_signalling_nan` | [x] |
| 13 | `hsl_to_rgb` | Subnormal / minimum-magnitude inputs: `h`, `s`, `l` in `{±MIN_POSITIVE, ±1e-45, ±0.0}` | No rejection and no FTZ (default MXCSR). `s = ±MIN_POSITIVE` is *not* `== 0`, so the slow path runs with a subnormal `c`. | `err_row13_subnormal_inputs` | [x] |
| 14 | `hsl_to_rgb` | Maximum-magnitude inputs: `h`/`s`/`l` `= ±f32::MAX`, forcing overflow to `±inf` mid-computation | No rejection. Overflow to infinity is silent; `inf - inf` NaNs are emitted. | `err_row14_extremal_magnitudes` | [x] |
| 15 | `hsl_to_rgb` | `src == NULL` (with valid `dest`) | No null check — line 6 dereferences it. Fatal `SIGSEGV` (signal 11), no return. | `err_row15_null_src_faults` | [x] |
| 16 | `hsl_to_rgb` | `dest == NULL` (with valid `src`, `s != 0` so the slow path writes) | No null check. Fatal `SIGSEGV`. | `err_row16_null_dest_faults` | [x] |
| 17 | `hsl_to_rgb` | `dest == NULL` **and** `s == 0` (the early-return path still writes 3 floats) | No null check; the fast path is not a "no write" path. Fatal `SIGSEGV`. | `err_row17_null_dest_fast_path_faults` | [x] |
| 18 | `hsl_to_rgb` | Both pointers `NULL` | Fatal `SIGSEGV` on the `src` read (which happens first). | `err_row18_both_null_faults` | [x] |

## Aliasing note (not a rejection, but an unchecked precondition)

`src` is `const float *` and `dest` is `float *`; the C declares no `restrict`
and performs no overlap check. At `-O0` gcc reads all three `src` elements into
stack slots *before* any store to `dest`, so full or partial aliasing
(`dest == src`, `dest == src+1`, `dest == src-1`, …) is well-defined in practice.
The Rust reads `h`, `s`, `l` into locals up front for the same reason. Covered by
`CONFIGS.md` rows 27–29 rather than here, since no error is produced.

## Defect found by rows 15-18

Rows 15-18 initially FAILED in the **debug profile** (they passed in release,
which is why profile coverage matters):

```
[ERRORS row 15] null_src : C gave Signal(11) but Rust gave Signal(6)
```

Cause: the translation loaded its inputs with raw place projections
(`*src.add(0)`). With `debug-assertions` enabled, rustc emits a null/alignment
UB check around such a projection, so a null pointer made the function *abort*
(`SIGABRT`, signal 6). The C has no check and dies with `SIGSEGV` (signal 11).

Fix: load and store through `core::ptr::read` / `core::ptr::write`. Those live in
the precompiled standard library, whose UB checks are off, so they fault exactly
like the C in every profile. Confirmed by isolating the four load/store forms:

| form | signal on null |
|------|----------------|
| `*p` | 6 (`SIGABRT`) |
| `core::ptr::read_unaligned(p)` | 6 (`SIGABRT`) |
| `core::ptr::read(p)` | **11 (`SIGSEGV`)** |
| `core::ptr::read_volatile(p)` | 11 (`SIGSEGV`) |
| `*p = v` | 6 (`SIGABRT`) |
| `core::ptr::write(p, v)` | **11 (`SIGSEGV`)** |

Regression-guarded by `mutation_check.sh` mutations 14 and 15, which revert the
loads/stores and confirm the suite still catches it.
