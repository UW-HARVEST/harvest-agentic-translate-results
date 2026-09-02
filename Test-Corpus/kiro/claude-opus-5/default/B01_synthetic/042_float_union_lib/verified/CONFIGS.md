# CONFIGS.md — configuration-surface table

## How this table was derived

### Public entry points (complete set)

`c_src/include/driver.h` exports exactly one entry point, and it is
simultaneously the highest- and lowest-level one — there is no convenience
wrapper layered over a lower-level API:

```c
void driver(double f);
```

There is therefore no "call the low-level API directly" variant to add; every
row below drives `driver` itself.

### Runtime options / modes / flags

Grep of the C source for option state: **none exist**. There is no
`#ifdef` in `driver.c` or `driver.h`, no flag parameter, no global/`static`
configuration variable, no setter, no environment lookup, no locale call.
`translation/Cargo.toml` declares no `[features]`, so there is no
compile-time axis either. The single configuration axis is therefore the
**shape of the one input value**.

### Input shapes the code actually distinguishes

`driver` funnels its argument into three conversions, and it is those
conversions' branches that the input shape selects:

```c
raw_double_t u = {.f = f};              /* type-punned reinterpretation  */
printf("%llx %a %.4f\n", u.x, f, f);
```

* `%llx` — branches on the raw 64-bit pattern: leading-zero suppression means
  the field width varies from 1 to 16 hex digits, and the all-zero pattern is
  special (must still emit one `0`).
* `%a` (glibc `__printf_fphex`) — branches on: the sign bit; whether the biased
  exponent field is `0` (leading digit `0`, no renormalisation), `0x7ff`
  (`inf`/`nan` names, numeric path skipped entirely), `>= 1023` (`p+`) or
  `< 1023` (`p-`); and whether the 52-bit mantissa is zero (radix point and all
  13 digits suppressed) or has trailing zero nibbles (trailing-zero trimming).
* `%.4f` (glibc `__printf_fp`) — branches on: sign; non-finite names; whether
  the magnitude rounds to `0.0000`; whether the integer part is `0` (leading
  `0.`) or many digits (up to 309 for `DBL_MAX`); and the round-half-to-even
  decision at the 4th fractional digit, including carry propagation across the
  radix point.

The rows below are the cross-product of {sign} × {exponent-field class} ×
{mantissa shape} × {`%.4f` magnitude class}, pruned to the combinations the C
actually treats differently. Each row is exercised in `tests/differential.rs`
with **many randomized inputs drawn from that row's class using a fixed seed**
(SplitMix64, seed `0x2545F4914F6CDD1D`), never a single hand-picked value, and
compared byte-for-byte between the C `.so` and the Rust `.so`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | sign `+`, exponent field `0`, mantissa `0` → `+0.0`; degenerate `%llx` (all-zero pattern), `%a` mantissa suppressed, `%.4f` integer part `0` | [x] |
| 2 | `driver` | sign `-`, exponent field `0`, mantissa `0` → `-0.0`; sign must survive all three conversions | [x] |
| 3 | `driver` | sign `+`, exponent field `0`, mantissa random non-zero → positive subnormals; `%a` leading digit `0`, `p-1022`, `%.4f` underflows to `0.0000`; `%llx` has ≤ 13 digits (leading nibbles zero) | [x] |
| 4 | `driver` | sign `-`, exponent field `0`, mantissa random non-zero → negative subnormals; `-0x0.…p-1022`, `%.4f` → `-0.0000` | [x] |
| 5 | `driver` | exponent field `0`, mantissa = single random set bit → subnormal with exactly one significant hex nibble; maximal trailing-zero trimming in `%a` | [x] |
| 6 | `driver` | exponent field `0`, mantissa `0x…fffff` patterns (all low bits set) → subnormal with the full 13 mantissa hex digits, no trimming | [x] |
| 7 | `driver` | sign `±`, exponent field `1` (min normal exponent), mantissa random → `%a` leading digit `1` with `p-1022`; `%.4f` still `0.0000` | [x] |
| 8 | `driver` | sign `±`, exponent field random in `2..=0x3fe` (negative binary exponent, `p-`), mantissa random → magnitudes below 1; `%.4f` integer part `0`, fraction may round to `0.0000` or not | [x] |
| 9 | `driver` | sign `±`, exponent field exactly `0x3fe` (`p-1`), mantissa random → magnitudes in `[0.5, 1)`; `%.4f` boundary between `0.xxxx` and carry to `1.0000` | [x] |
| 10 | `driver` | sign `±`, exponent field exactly `0x3ff` (`p+0`), mantissa random → magnitudes in `[1, 2)`; the `exponent == IEEE754_DOUBLE_BIAS` branch, `p+0` | [x] |
| 11 | `driver` | sign `±`, exponent field random in `0x400..=0x433` (positive exponent, value `< 2^52`), mantissa random → ordinary mixed integer+fraction `%.4f` | [x] |
| 12 | `driver` | sign `±`, exponent field random in `0x434..=0x7fe` (value `≥ 2^53`, integral), mantissa random → `%.4f` fraction is exactly `.0000`, integer part 16…309 digits | [x] |
| 13 | `driver` | sign `±`, exponent field `0x7fe` (max finite), mantissa random → near-`DBL_MAX`; longest possible `%.4f` expansion (309 integer digits) | [x] |
| 14 | `driver` | sign `±`, exponent field random `1..=0x7fe`, mantissa `0` → `%a` prints no radix point at all (`0x1p±N`); `%llx` has trailing zero nibbles | [x] |
| 15 | `driver` | sign `±`, exponent field random `1..=0x7fe`, mantissa = one random set bit → single significant `%a` nibble after full 13-digit zero padding + trimming | [x] |
| 16 | `driver` | sign `±`, exponent field random `1..=0x7fe`, mantissa `0xfffffffffffff` (all 52 bits set) → `%a` prints all 13 digits, no trimming | [x] |
| 17 | `driver` | sign `±`, exponent field random `1..=0x7fe`, mantissa random with the low 4·k bits forced to zero (k = 1..12) → every distinct trailing-zero trim length in `%a` | [x] |
| 18 | `driver` | exponent field `0x7ff`, mantissa `0`, sign `+` → `+inf`: `%a`/`%.4f` take the special-name path, `%llx` still prints the pattern | [x] |
| 19 | `driver` | exponent field `0x7ff`, mantissa `0`, sign `-` → `-inf` | [x] |
| 20 | `driver` | exponent field `0x7ff`, mantissa random non-zero, sign `±` → NaNs (quiet and signalling, arbitrary payload): `nan`/`-nan` regardless of payload, payload still visible via `%llx` | [x] |
| 21 | `driver` | fully uniform random 64-bit patterns reinterpreted as `double` (no class restriction) — the unbiased cross-product sweep over all of the above | [x] |
| 22 | `driver` | random small integers `n` as `(double) n`, `n ∈ [-2^20, 2^20]` → exact `%a` powers, `%.4f` with `.0000` fraction, short `%llx` free of trailing-zero-only patterns | [x] |
| 23 | `driver` | random exact dyadic rationals `m / 2^k`, `k ∈ 1..=20` → `%.4f` values that are *exact* decimal ties or exact terminating decimals; drives round-half-to-even | [x] |
| 24 | `driver` | random values of the form `n ± 0.00005` and `n + 5·10^-5·(2j+1)` → decimal tie candidates at the 4th fractional digit (nearest representable double to a tie) | [x] |
| 25 | `driver` | random values just below an integer (`n - 10^-k`, `k ∈ 4..=12`) → `%.4f` carry propagation across the radix point and through `9…9` runs | [x] |
| 26 | `driver` | random values in `(0, 1e-4)` and `(-1e-4, 0)` → `%.4f` rounds to `0.0000`/`-0.0000` while `%a` shows a full non-degenerate mantissa | [x] |
| 27 | `driver` | random values `± 10^e` for `e ∈ -320..=308` (decimal-scale sweep, incl. subnormal decades) → exercises `__printf_fp`'s big-number/small-number paths across the whole dynamic range | [x] |
| 28 | `driver` | exhaustive sweep of all 2^11 raw exponent-field values with a fixed random mantissa and both signs → every `%a` exponent branch, incl. reserved fields `0`/`0x7ff` | [x] |
| 29 | `driver` | exhaustive sweep of all 16 leading-nibble values of the raw pattern and of `%llx` field widths 1..16 (patterns `0x1`, `0xf`, `0x10`, … `0xffffffffffffffff`) → `%llx` leading-zero-suppression widths | [x] |
| 30 | `driver` | repeated / interleaved invocation of C then Rust in the same process on the same `stdout` `FILE` → verifies the Rust export writes through glibc `stdout` with identical buffering/newline behaviour and no extra output | [x] |

### Ambient-state axes

`printf` is not a pure function of its arguments: it reads two pieces of
caller-controlled process state on every call, and both change its output.
These are runtime options of the library just as much as a flag parameter would
be, so they get their own rows and are crossed with the input-shape sweep above.

* `LC_NUMERIC`'s decimal point — `__printf_fp` and `__printf_fphex` both take
  the radix character from `_NL_CURRENT (LC_NUMERIC, DECIMAL_POINT)`, so in a
  comma-radix locale the C library prints `0x1,8p+0` and `1,5000`. It may be a
  multi-byte string (`ps_AF` uses U+066B).
* the FP rounding direction — `__printf_fp` calls `get_rounding_mode ()` and
  feeds it to `round_away ()`, so `%.4f` of `0.99999` is `1.0000` under
  `FE_TONEAREST` but `0.9999` under `FE_TOWARDZERO` and `FE_DOWNWARD`.

`%llx` is insensitive to both, and `%a` carries no precision so it prints the
value exactly and never rounds.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 31 | `driver` | each of the four rounding directions (`FE_TONEAREST`, `FE_DOWNWARD`, `FE_UPWARD`, `FE_TOWARDZERO`) × the broad randomized input sweep (specials, exact ties, subnormals, `DBL_MAX`, non-finites, random patterns) | [x] |
| 32 | `driver` | each locale installed on the host (`C`, `POSIX`, `en_US.utf8`, comma-radix `de_DE.utf8` / `fr_FR.UTF-8` / `de_DE.iso88591` / `ru_RU.utf8`, multi-byte-radix `ps_AF.utf8`) × the broad input sweep | [x] |
| 33 | `driver` | full cross-product locale × rounding direction × broad input sweep | [x] |
| 34 | `driver` | non-dyadic decimals (infinite binary tail, so `more_bits` is always set) at every magnitude and both signs × all four rounding directions — the inputs where truncate vs. away-from-zero is observable | [x] |
