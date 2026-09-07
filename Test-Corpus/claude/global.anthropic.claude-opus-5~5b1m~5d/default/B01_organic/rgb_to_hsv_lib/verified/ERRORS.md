# ERRORS.md — Error-surface table

Mechanically derived from `c_src/src/lib.c` (38 lines) and
`c_src/include/lib.h` (1 line).

## Grep audit of every rejection mechanism in the C source

```
$ grep -nE 'return|assert|NULL|errno|-1|if *\(' c_src/src/lib.c
19:    if (delta == 0 || max == 0) {
23:        return;                <- bare early return, no value
26:    if (r == max)
28:    else if (g == max)
33:    if (h < 0)
```

Findings:

- `rgb_to_hsv` returns `void` — there is **no error code, no sentinel, no
  errno use, and no output flag**. It cannot signal failure.
- There are **zero** `assert` statements.
- There are **zero** null-pointer checks on `dest` or `src`.
- There are **zero** range checks on the input floats (no clamping to
  `[0, 1]`, no rejection of negatives, NaN, or infinities).
- There are **zero** min/max constants, enums, or length/count parameters,
  so there is no enum-value or size validation surface.

Therefore the library's *only* "rejection-like" control flow is the early-out
branch at line 19, plus the value-dependent branch selection. Those rows are
listed below and each has a differential test. The remaining rows are the
generic C-API boundaries mandated by the task (out-of-range values, extreme
values, aliasing) expressed in terms of what this API actually accepts: a
raw pointer pair and three `float`s.

Null pointers are explicitly **undefined behaviour in the C** (line 4
dereferences `src` unconditionally, lines 20/35 dereference `dest`); the C
segfaults rather than returning an error. Row 12 is therefore tested three ways
in `tests/errors.rs`:

- `err12_null_pointer_is_ub_documented` — guard-word test proving both
  implementations touch exactly `dest[0..3]` and never modify `src`.
- `err12b/err12c/err12d_*` — pass NULL as `dest`, as `src`, and as both, each in
  a re-exec'd child process, and compare how the two libraries die.

**FINDING (real divergence, correctly scoped):** the C dies with `SIGSEGV` (11)
in both profiles. The **dev-profile** Rust cdylib dies with `SIGABRT` (6),
because `-C debug-assertions=on` makes rustc emit a library-level null/alignment
UB check in front of the raw store, which panics before the hardware fault can
occur. The **release** cdylib faults with `SIGSEGV` (11), identical to the C.
This is a Rust sanitiser artefact on a path the C itself declares undefined, not
a divergence in translated logic, so `err12b` asserts:

- unconditionally: both must die from a fatal signal, never exit cleanly;
- under `not(debug_assertions)`: the signal must be *exactly* the same (11).

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `rgb_to_hsv` | `delta == 0` because `r == g == b` (achromatic grey, incl. all-equal non-zero) | early return at line 23; `dest = {0, 0, max}`; `h` and `s` left at their `0` initialisers |
| 2  | `rgb_to_hsv` | `max == 0` with all components `0` (`{0,0,0}` black) | early return; `dest = {0, 0, 0}` |
| 3  | `rgb_to_hsv` | `max == 0` while `delta != 0`, i.e. all components `<= 0` with at least one negative (e.g. `{-1, 0, 0}` → `max == 0`, `delta == 1`) | early return (second disjunct); `dest = {0, 0, 0}` — `s = delta/max` division is **skipped** |
| 4  | `rgb_to_hsv` | `max < 0` (all components negative, e.g. `{-1,-2,-3}`) — neither early-out disjunct fires; `s = delta / max` yields a **negative** saturation | no early return; `s` negative, `v` negative, `h` computed normally; no rejection |
| 5  | `rgb_to_hsv` | any component is `NaN`: `min`/`max` are built with raw `<` / `>` ternaries, which propagate the *second* operand when the comparison is false, so a `NaN` may be silently discarded or retained depending on position | no rejection; result is whatever the raw-ternary min/max ordering produces (`h`/`s`/`v` may be `NaN` or a finite value) |
| 6  | `rgb_to_hsv` | all three components `NaN` | `min = max = NaN`, `delta = NaN`; `delta == 0` is **false** and `max == 0` is **false**, so no early return; `r == max` is false, `g == max` is false → `else` branch; `h = (4 + NaN) * 60 = NaN`; `h < 0` false; `dest = {NaN, NaN, NaN}` |
| 7  | `rgb_to_hsv` | `+INFINITY` component | `max = inf`, `delta = inf`, `s = inf/inf = NaN`; branch on `r == max` etc. still works; `h` may be `NaN` |
| 8  | `rgb_to_hsv` | `-INFINITY` component together with a finite one | `min = -inf`, `delta = +inf`, `s = inf/max`; `h` numerator `/inf` → `±0` or `NaN` |
| 9  | `rgb_to_hsv` | both `+INFINITY` and `-INFINITY` present | `delta = inf - (-inf) = inf`; no early out; result contains `NaN`/`inf` per the arithmetic |
| 10 | `rgb_to_hsv` | `h < 0` after `h *= 60` (i.e. `r == max` and `g < b`) — the wrap-around correction at line 33 | `h += 360`, so `dest[0]` in `[0, 360)`; **not** an error, but the only corrective branch |
| 11 | `rgb_to_hsv` | subnormal / `FLT_MIN`-scale components where `delta` underflows to exactly `0.0` although the components differ | early return via `delta == 0`; `dest = {0, 0, max}` even though the input was chromatic |
| 12 | `rgb_to_hsv` | `src == NULL` or `dest == NULL` (or both) | **undefined behaviour** — unconditional dereference at line 4 (`src`) / lines 20 & 35 (`dest`); the C does not check and dies with `SIGSEGV` (11). No error code exists to compare. Release-profile Rust matches exactly; dev-profile Rust aborts (6) via rustc's debug-only UB check — see the FINDING note above. |
| 13 | `rgb_to_hsv` | `dest == src` (full aliasing) | all three loads (lines 4-6) precede all stores, so the C reads consistent inputs; result identical to the non-aliased call |
| 14 | `rgb_to_hsv` | `dest == src + 1` / `dest == src - 1` (partial overlap) | C is compiled without `restrict`; loads still all precede stores, so behaviour is well-defined and identical to the non-aliased call |
| 15 | `rgb_to_hsv` | `-0.0` components (signed zero); `-0.0 == 0` is **true** in C | `max == 0` disjunct fires when `max` is `-0.0`; early return with `dest[2] = -0.0` (sign preserved in the store) |
| 16 | `rgb_to_hsv` | ties between components (`r == max` **and** `g == max`) — the `if / else if` chain picks the **first** match | `r == max` wins; `h = (g - b)/delta`, never the `g`-branch. Branch-order fidelity, not an error. |

## Row -> test mapping

| row | test in `translation/tests/errors.rs` |
|-----|----------------------------------------|
| 1  | `err01_delta_zero_grey` |
| 2  | `err02_black_both_disjuncts` |
| 3  | `err03_max_zero_delta_nonzero` |
| 4  | `err04_all_negative_negative_saturation` |
| 5  | `err05_single_nan_ternary_ordering` |
| 6  | `err06_all_nan_falls_through_to_else` |
| 7  | `err07_plus_infinity` |
| 8  | `err08_minus_infinity` |
| 9  | `err09_mixed_infinities` |
| 10 | `err10_hue_wrap_correction` |
| 11 | `err11_subnormal_delta_underflow` |
| 12 | `err12_null_pointer_is_ub_documented`, `err12b_null_pointer_same_fatal_signal`, `err12c_null_src_same_fatal_signal`, `err12d_both_null_same_fatal_signal` |
| 13 | `err13_full_aliasing` |
| 14 | `err14_partial_overlap` |
| 15 | `err15_signed_zero_early_out_sign_preserved` |
| 16 | `err16_tie_first_match_wins` |

Beyond the table (generic C-API boundaries required regardless):
`generic_all_ieee_classes_cross_product` (4096-point cross-product over all 16
distinguished IEEE-754 classes, standing in for the out-of-range-enum case,
since every 32-bit pattern is a legal `float` argument across this FFI
boundary), `generic_one_step_past_documented_range` (ULP neighbours of 0 and 1),
and `generic_exact_and_oversized_buffers`.

**All 16 rows pass** in both the dev and release profiles.
