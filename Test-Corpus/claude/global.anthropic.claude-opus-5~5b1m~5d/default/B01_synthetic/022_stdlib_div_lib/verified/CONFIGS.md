# CONFIGS.md — Phase B configuration-surface table

## Axes derived from the C source

`c_src/src/driver.c` contains no options, no modes, no flags, no global state and
no `#ifdef`s, and `c_src/include/driver.h` exposes exactly one entry point:

```c
void driver(int x, int y);
```

So there is exactly **one** public entry point, and it is simultaneously the
lowest-level one — there is no convenience wrapper to hide behind, and no
per-call configuration to set. The behavioural axes are therefore entirely
**input shape**, and they come from what the two callees actually distinguish:

**Axis A — `div(3)` / `idivl` truncation semantics.** C99 division truncates
toward zero and the remainder takes the sign of the *numerator*. The sign
quadrant of `(x, y)` is a real branch in the observable result, so all four
quadrants must be covered separately.

**Axis B — magnitude relationship.** `|x| < |y|` (quotient `0`, remainder `x`),
`|x| == |y|` (quotient `±1`, remainder `0`), `|x| > |y|`.

**Axis C — exactness.** `x % y == 0` versus `x % y != 0`; these produce different
remainder text.

**Axis D — degenerate/identity divisors.** `y == 1` (quotient `x`), `y == -1`
(quotient `-x`, the trap case when `x == INT_MIN`), `x == 0` (both outputs `0`).

**Axis E — extreme values.** `INT_MIN`, `INT_MIN + 1`, `INT_MAX`, `INT_MAX - 1`
in either position. These matter because `INT_MIN` has no positive counterpart,
and because they are the widest `%d` conversions.

**Axis F — `printf("%d")` formatting width/sign.** The output text is the only
observable, so the number of digits and the presence of `-` are part of the
surface: 1-digit, multi-digit, 10-digit, and negative renderings of *both*
`quot` and `rem`, independently.

**Axis G — ABI shape of the call.** `int` parameters occupy the low 32 bits of
`rdi`/`rsi`; the upper 32 bits are undefined per the SysV ABI. Passing dirty high
bits is a valid caller behaviour that both libraries must ignore identically.

**Axis H — repeated / interleaved invocation and stdout buffering.** `printf` is
buffered and the buffer is process-global; call sequencing and flush behaviour
are observable in the byte stream, so single-call and many-call-in-sequence
shapes are distinct configurations.

Rows below are the cross-product of these axes, pruned to combinations the code
actually distinguishes. Every row is driven with **many randomized inputs**
(seeded, reproducible LCG — seed `0x5EED_1234_ABCD_0001`) rather than one
hand-picked value, except where the row names a specific boundary constant.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `driver` | quadrant `x>0, y>0`, `|x|>|y|`, inexact; randomized | `cfg_row01_pos_pos_inexact` | [x] |
| 2 | `driver` | quadrant `x<0, y>0`, `|x|>|y|`, inexact; randomized (remainder must be negative) | `cfg_row02_neg_pos_inexact` | [x] |
| 3 | `driver` | quadrant `x>0, y<0`, `|x|>|y|`, inexact; randomized (quotient negative, remainder positive) | `cfg_row03_pos_neg_inexact` | [x] |
| 4 | `driver` | quadrant `x<0, y<0`, `|x|>|y|`, inexact; randomized (quotient positive, remainder negative) | `cfg_row04_neg_neg_inexact` | [x] |
| 5 | `driver` | all four quadrants, exact division (`x = k*y`); randomized `k`, `y` | `cfg_row05_exact_all_quadrants` | [x] |
| 6 | `driver` | `|x| < |y|` in all four quadrants → quotient `0`, remainder `x`; randomized | `cfg_row06_smaller_magnitude` | [x] |
| 7 | `driver` | `|x| == |y|` in all four quadrants → quotient `±1`, remainder `0`; randomized | `cfg_row07_equal_magnitude` | [x] |
| 8 | `driver` | `x == 0`, random non-zero `y` (both signs) | `cfg_row08_zero_numerator` | [x] |
| 9 | `driver` | `y == 1`, random `x` incl. `INT_MIN`/`INT_MAX` (identity divisor) | `cfg_row09_divisor_one` | [x] |
| 10 | `driver` | `y == -1`, random `x` **excluding** `INT_MIN` (negation divisor; `INT_MIN` is `ERRORS.md` row 3) | `cfg_row10_divisor_minus_one` | [x] |
| 11 | `driver` | `x` ∈ {`INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`} × random `y` (both signs) | `cfg_row11_extreme_numerator` | [x] |
| 12 | `driver` | `y` ∈ {`INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`} × random `x` (both signs) | `cfg_row12_extreme_divisor` | [x] |
| 13 | `driver` | full boundary cross-product: `x`,`y` ∈ {`INT_MIN`, `INT_MIN+1`, `-2`, `-1`, `0`, `1`, `2`, `INT_MAX-1`, `INT_MAX`}, minus the trapping combinations | `boundary_extremes_cross_product` | [x] |
| 14 | `driver` | `%d` width sweep: operands chosen so `quot` and `rem` independently render as 1, 2, 5, 9 and 10 digits, and as negative | `cfg_row14_printf_width_sweep` | [x] |
| 15 | `driver` | powers of two and powers-of-two ± 1 for `y` (the shapes a compiler would strength-reduce differently from a true `idiv`) | `cfg_row15_power_of_two_divisors` | [x] |
| 16 | `driver` | unrestricted random `(x, y)`, `y != 0`, large-volume fuzz sweep (20 000 pairs) over the whole `int` range | `cfg_row16_unrestricted_fuzz` | [x] |
| 17 | `driver` | ABI: dirty high 32 bits in the 64-bit argument registers, randomized garbage | `abi_high_garbage_bits_ignored` | [x] |
| 18 | `driver` | many sequential calls without an intervening flush (stdout buffering / output ordering over a long run) | `cfg_row18_buffering_long_run` | [x] |
| 19 | `driver` | C and Rust calls interleaved within one captured stream, sharing the one process-global `stdout` buffer | `cfg_row19_interleaved_shared_stdout` | [x] |

## Feature combinations

`Cargo.toml` has no `[features]`, so the default set is the only set; the runner
script nonetheless executes the whole suite under both the default build and
`--no-default-features` to satisfy the Phase D requirement.
