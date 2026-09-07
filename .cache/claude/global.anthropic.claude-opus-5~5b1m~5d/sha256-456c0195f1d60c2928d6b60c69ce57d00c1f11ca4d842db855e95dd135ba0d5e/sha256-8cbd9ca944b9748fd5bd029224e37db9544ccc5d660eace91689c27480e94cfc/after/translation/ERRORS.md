# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Grep audit of every rejection mechanism the C source could use

```
$ grep -nE 'return -1|return NULL|RETURN_ERROR|assert|errno|abort|exit\(|if \(|\?|<|>|==|!=' c_src/src/lib.c
```

Findings:

* `return -1` / `return NULL` / `RETURN_ERROR` / error enums / status codes: **none**.
* `assert` / `abort` / `exit` / `errno`: **none**.
* pointer parameters (⇒ possible null checks): **none** — `cb_rgb_255` is passed
  **by value**, so there is no null-pointer input to reject.
* length / count / size parameters: **none**.
* enum parameters: **none** (no `enum` in the header, so there is no
  out-of-range-enum input class for this API).
* explicit range checks: **none**. `A.R/G/B` are `unsigned char`, so every one
  of the 256 values per channel is in range by construction; the C code performs
  no validation.
* min/max constants: **none**.

The only conditionals in the source are *value-dependent branches*, not
rejections:

| location | conditional | nature |
|----------|-------------|--------|
| `cbLuminance` (×3) | `X > 0.04045 ? pow(...) : X / 12.92` | sRGB transfer-curve branch (valid path, tracked in `CONFIGS.md`) |
| `cbContrastRatio` | `if (High < Low) { swap }` | ordering branch (valid path, tracked in `CONFIGS.md`) |

`contrast_ratio` therefore has a **total** domain: it cannot fail and returns no
error sentinel. Its only "degenerate" outputs are IEEE-754 special values
produced by the unguarded division `High / Low`. Those are the rows below: they
are the C library's *de facto* rejection/degenerate surface and the Rust must
reproduce them bit-for-bit (including the sign of infinity and the exact NaN
payload class).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `contrast_ratio` | `B == {0,0,0}` (black), `A` non-black ⇒ `Low == 0.0f`, `High > 0` — division by zero, **no guard** in C | `+inf` (`0x7F800000`) |
| 2 | `contrast_ratio` | `A == {0,0,0}` (black), `B` non-black ⇒ swap taken (`High < Low` true), then `Low == 0` | `+inf` (`0x7F800000`) |
| 3 | `contrast_ratio` | `A == B == {0,0,0}` ⇒ `0.0f / 0.0f` | `NaN` (quiet, `0x7FC00000`) |
| 4 | `contrast_ratio` | `A == B` (any equal non-black pair) ⇒ `High == Low`, swap not taken, exact self-division | exactly `1.0f` (`0x3F800000`) |
| 5 | `contrast_ratio` | boundary of the sRGB transfer branch: channel byte `10` ⇒ `10/255 = 0.039215688f ≤ 0.04045` (linear arm) vs channel byte `11` ⇒ `0.043137256f > 0.04045` (`pow` arm). The comparison is done in `double` after promotion, so the *exact* branch point must match | branch-identical result; no error |
| 6 | `contrast_ratio` | every channel at its maximum `255` for both args ⇒ `High = Low = 1.0f` (largest representable luminance) | exactly `1.0f` |
| 7 | `contrast_ratio` | `A` = black, `B` = the *smallest* non-black colour `{0,0,1}` ⇒ `Low` is the smallest non-zero luminance (`0.0722f * (1/255)/12.92`), producing the largest finite ratio | large finite `float`, no overflow to `inf` |
| 8 | `contrast_ratio` | argument-order asymmetry: `contrast_ratio(A,B)` vs `contrast_ratio(B,A)` — the `if (High < Low)` swap must make the function exactly symmetric, **including** for the `0`/`NaN` cases (NaN compares false ⇒ no swap) | identical bits for both orders (except row 3, where both are NaN) |
| 9 | `contrast_ratio` | struct padding / trailing garbage: `cb_rgb_255` is 3 bytes with align 1 but is passed in a register; a caller that leaves the 4th…8th register bytes non-zero must not change the result (C reads only 3 bytes) | result independent of the padding bytes |
| 10 | `contrast_ratio` | out-of-"range" byte values: there are none — `unsigned char` inputs are exhaustively valid. Passing e.g. `256` wraps to `0` at the C ABI level | wrap-around to `0`; no rejection |

Rows 1–10 are all covered by `tests/differential.rs` (Phase C section); every
row is asserted on **raw `u32` bit patterns**, so `+inf` vs `-inf` vs `NaN` vs a
finite value are distinguished, not merely "both failed".

## Status

All 10 rows have a passing differential test (`tests/differential.rs`,
`phase_c_row_1_*` … `phase_c_row_10_*`, plus `phase_c_generic_boundaries` for
the generic FFI boundaries: extremes, one-step-past-branch-point values, wide
integers truncated at the ABI, and the structural proof that there is no
pointer/length/enum parameter and therefore no null / oversized-length /
invalid-enum input class). Verified against the C library built at `-O0`,
`-O2`, `-O3`, and via CMake defaults, and against both the debug and release
Rust `cdylib`. [x] ×10
