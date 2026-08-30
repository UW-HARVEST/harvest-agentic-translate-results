# CONFIGS.md — Configuration-surface table (Phase A)

Mirror of `ERRORS.md` for **valid** inputs.

## Axes the C actually branches on

Derived from `c_src/include/pow.h` + `c_src/src/pow.c`:

* **Runtime options / modes / flags: NONE.** The public header declares a single
  function with no flags parameter, no context/handle struct, no setter, and no
  global configuration variable. `grep -n '#ifdef\|#if\|switch' c_src/src/pow.c`
  returns nothing — there is no conditional compilation either.
* **Public entry points: exactly one — `my_pow`.** It *is* the lowest-level
  entry point; there is no convenience wrapper layered over a lower-level API,
  so "exercise the low-level entry points, not just the wrappers" collapses to
  calling `my_pow` directly, which is what all tests do.
* **Explicit branches in the C body: 2**, both on `errno` (`== EDOM`,
  `== ERANGE`) — these are the `ERRORS.md` rows.
* **Input shapes.** Both parameters are `double`. The C code itself does not
  inspect their shape, but it forwards them to glibc `pow`, whose result *and*
  `errno` side effect drive the two branches. The shape axes that therefore
  change observable behaviour are the ones libm special-cases:
  * sign of `base`: negative / `-0.0` / `+0.0` / positive
  * magnitude of `base`: `< 1`, `== 1`, `> 1`, `DBL_MAX`, `DBL_MIN`, subnormal
  * `exponent` integrality: integral vs non-integral
  * `exponent` parity when integral: even vs odd (controls result sign)
  * sign of `exponent`: negative / zero / positive
  * non-finite classes: quiet NaN, signalling NaN, `+Inf`, `-Inf`
  * result classification: normal / overflow to `±Inf` / underflow to `±0`
* **Fall-through path**: `errno == 0` → `return result` (line 49), the only
  success path.

## Table

One row per combination the code treats differently. Every row is driven with
many randomized inputs (fixed seed `0x5EED_1234_ABCD_0001`, ChaCha-free
xorshift64* PRNG implemented in the test file so there is no dependency churn)
and asserted **bit-for-bit** on the returned `double`, plus an assertion that
the returned bits are identical between the C and Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `my_pow` | positive base > 1, positive integral exponent, result in range → normal success path | [x] |
| C2 | `my_pow` | positive base in `(0,1)`, positive integral exponent → normal, result shrinks | [x] |
| C3 | `my_pow` | positive base, negative integral exponent, result in range | [x] |
| C4 | `my_pow` | positive base, positive **non-integral** exponent (fractional powers / roots) | [x] |
| C5 | `my_pow` | positive base, negative non-integral exponent | [x] |
| C6 | `my_pow` | **negative** base, **even** integral exponent → positive result, `errno == 0` | [x] |
| C7 | `my_pow` | **negative** base, **odd** integral exponent → negative result, `errno == 0` | [x] |
| C8 | `my_pow` | exponent `== 0.0` with arbitrary base (incl. NaN, `±Inf`, `±0`) → always `1.0`, `errno == 0` | [x] |
| C9 | `my_pow` | exponent `== 1.0` with arbitrary base → identity | [x] |
| C10 | `my_pow` | base `== 1.0` with arbitrary exponent (incl. NaN, `±Inf`) → always `1.0` | [x] |
| C11 | `my_pow` | base `== -1.0` with `±Inf` exponent → `1.0`; with integral exponents → `±1.0` | [x] |
| C12 | `my_pow` | base `== ±0.0`, **positive** exponent (even / odd / fractional) → `±0.0`, `errno == 0`; sign-of-zero path | [x] |
| C13 | `my_pow` | `base == +Inf` combined with every exponent class (neg / zero / pos / NaN / `±Inf`) | [x] |
| C14 | `my_pow` | `base == -Inf` combined with every exponent class — odd/even integral exponent controls result sign | [x] |
| C15 | `my_pow` | `exponent == +Inf` / `-Inf` with base magnitude `<1`, `==1`, `>1` → `0`/`Inf`/`1` cross-product | [x] |
| C16 | `my_pow` | quiet NaN base with non-zero exponent; quiet NaN exponent with base `!= 1`; NaN in both → NaN payload propagation, `errno == 0` | [x] |
| C17 | `my_pow` | **signalling** NaN bit patterns in either argument → exact returned bit pattern must match | [x] |
| C18 | `my_pow` | subnormal base with small positive exponent → still in range, `errno == 0` | [x] |
| C19 | `my_pow` | `±DBL_MAX` / `±DBL_MIN` base with exponent `1.0`, `0.0`, `-1.0` → boundary magnitudes without overflow | [x] |
| C20 | `my_pow` | exponent straddling the overflow threshold via `nextafter` — the in-range side returns the real value, the out-of-range side takes the `ERANGE` branch and returns `-1.0` | [x] |
| C21 | `my_pow` | exponent straddling the underflow threshold via `nextafter` — same straddle on the other side | [x] |
| C22 | `my_pow` | fully random `u64` bit patterns reinterpreted as `f64` for **both** arguments (unbiased fuzz over the whole `2^128` input space, incl. all the non-finite and error classes mixed together) | [x] |
| C23 | `my_pow` | randomized *integral* exponents in `[-1074, 1024]` against randomized bases — walks the whole exponent range where overflow/underflow flip | [x] |
| C24 | `my_pow` | errno hygiene / statefulness: repeated calls in sequence where an earlier call left `errno == EDOM` or `ERANGE`, interleaved with valid calls — verifies the `errno = 0` reset at line 34 and that the function is stateless across calls | [x] |
| C25 | `my_pow` | argument-order asymmetry sweep: for each pair `(a,b)` from the special-value set, call both `my_pow(a,b)` and `my_pow(b,a)` — catches a swapped-parameter translation bug | [x] |

## Feature combinations

`translation/Cargo.toml` has **no** `[features]` table and no optional
dependencies, so `--all-features`, `--no-default-features` and the default build
are the same build. There is exactly one feature combination:

| combo | command | status |
|-------|---------|--------|
| default (= only combo) | `cargo test --release` | [x] |
| `--no-default-features` (identical to default: no features declared) | `cargo test --release --no-default-features` | [x] |
| `--all-features` (identical to default) | `cargo test --release --all-features` | [x] |
