# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/include/pow.h` (the complete public header) and
`c_src/src/pow.c`.

**Runtime options / modes / flags:** none. The public header exposes a single
function with no context struct, no init call, no setters, no `#ifdef`s and no
global configuration. `grep -c 'ifdef\|ifndef\|define' c_src/src/pow.c` finds
only the include guard in the header. So the option axis is empty and the
whole configuration surface is the **input-shape** axis.

**Full set of public entry points:** exactly one, and it *is* the lowest level
one — `double my_pow(double base, double exponent)`. There are no convenience
wrappers layered over anything.

**Input shapes the code / its callee `pow` special-cases** (the branch taken at
lines 35/41 is a function of `errno`, which is a function of these shapes; libm
`pow` itself is specified case-by-case on them, and the C code returns its
result verbatim, so each case is a distinct path through the composed
operation):

* sign of `base`: negative / `-0.0` / `+0.0` / positive
* class of `base`: NaN / infinite / normal / subnormal / zero / exactly `±1.0`
* class of `exponent`: NaN / infinite / zero / integer (odd vs even) /
  non-integer / half-integer / huge-magnitude
* result magnitude: normal / overflow / underflow-to-zero / underflow-to-subnormal / pole
* `errno` state carried in from the caller (cleared at line 33)
* call sequencing (state left behind between successive calls)

The table below is the pruned cross-product: one row per combination glibc's
`pow` and the two `errno` branches actually treat differently.

| #  | entry point(s) | configuration (options set + input shape) | done |
|----|----------------|-------------------------------------------|-----|
| C1  | `my_pow` | `base` positive normal, `exponent` positive small even integer (2,4,6) — plain path, no errno | [x] |
| C2  | `my_pow` | `base` positive normal, `exponent` positive small odd integer (1,3,5) | [x] |
| C3  | `my_pow` | `base` positive normal, `exponent` negative integer | [x] |
| C4  | `my_pow` | `base` positive normal, `exponent` non-integer positive (incl. `0.5`, which libm may route to `sqrt`) | [x] |
| C5  | `my_pow` | `base` positive normal, `exponent` non-integer negative | [x] |
| C6  | `my_pow` | `base` negative normal, `exponent` even integer → positive finite result | [x] |
| C7  | `my_pow` | `base` negative normal, `exponent` odd integer → negative finite result | [x] |
| C8  | `my_pow` | `base` negative normal, `exponent` negative even integer | [x] |
| C9  | `my_pow` | `base` negative normal, `exponent` negative odd integer | [x] |
| C10 | `my_pow` | `exponent == 0.0` with every class of `base` (normal, negative, `±0`, `±inf`, NaN) → `1.0` even for NaN | [x] |
| C11 | `my_pow` | `exponent == -0.0` with every class of `base` → `1.0` | [x] |
| C12 | `my_pow` | `base == 1.0`, `exponent` = anything incl. NaN and `±inf` → `1.0` | [x] |
| C13 | `my_pow` | `base == -1.0`, `exponent` = `±inf` → `1.0`; odd/even integers → `∓1.0` | [x] |
| C14 | `my_pow` | `base == +0.0`, `exponent > 0` (odd int / even int / non-integer) → `+0.0` | [x] |
| C15 | `my_pow` | `base == -0.0`, `exponent > 0` odd integer → `-0.0`; even/non-integer → `+0.0` | [x] |
| C16 | `my_pow` | `base == +inf`, `exponent > 0` → `+inf`; `exponent < 0` → `+0.0` | [x] |
| C17 | `my_pow` | `base == -inf`, `exponent` positive odd int → `-inf`; positive even/non-int → `+inf` | [x] |
| C18 | `my_pow` | `base == -inf`, `exponent` negative odd int → `-0.0`; negative even/non-int → `+0.0` | [x] |
| C19 | `my_pow` | `exponent == +inf`, `\|base\| > 1` → `+inf`; `\|base\| < 1` → `+0.0` | [x] |
| C20 | `my_pow` | `exponent == -inf`, `\|base\| > 1` → `+0.0`; `\|base\| < 1` → `+inf` | [x] |
| C21 | `my_pow` | `base` NaN, `exponent` non-zero → NaN (payload/bit pattern compared) | [x] |
| C22 | `my_pow` | `exponent` NaN, `base != 1.0` → NaN | [x] |
| C23 | `my_pow` | both NaN | [x] |
| C24 | `my_pow` | `base` subnormal positive, `exponent` small positive/negative | [x] |
| C25 | `my_pow` | `base` `±DBL_MAX` / `±DBL_MIN`, `exponent` near `±1.0` (boundary magnitudes, no overflow) | [x] |
| C26 | `my_pow` | `exponent` huge integer (`1e18`, `2^53`, `2^53+1`) with `\|base\|` slightly above/below 1 | [x] |
| C27 | `my_pow` | randomized pairs, both operands uniform in `[-10, 10]` — mixed signs, mostly non-integer exponents → heavy EDOM mix | [x] |
| C28 | `my_pow` | randomized pairs, `base` uniform in `(0, 100]`, `exponent` uniform in `[-50, 50]` — mixed normal/overflow/underflow | [x] |
| C29 | `my_pow` | randomized pairs, `base` positive, `exponent` a random *integer* in `[-64, 64]` (odd/even split) | [x] |
| C30 | `my_pow` | randomized pairs, `base` negative, `exponent` a random *integer* in `[-64, 64]` | [x] |
| C31 | `my_pow` | randomized *raw 64-bit patterns* for both operands (covers every float class incl. NaN payloads, subnormals, infinities) | [x] |
| C32 | `my_pow` | randomized pairs drawn from an explicit special-value pool (`±0, ±1, ±inf, NaN, ±DBL_MAX, ±DBL_MIN, ±5e-324, ±0.5, ±2, ±3`) — full 24×24 cross product | [x] |
| C33 | `my_pow` | call *sequencing*: a long interleaved sequence of erroring and valid calls, asserting each result and each `stderr` line in order (line 33's `errno = 0` and no cross-call state) | [x] |
| C34 | `my_pow` | caller pre-sets `errno` to `EDOM` / `ERANGE` / arbitrary value before a valid call | [x] |
| C35 | `my_pow` | `stderr` byte-for-byte comparison for every configuration above (`%.2f` rendering of `-0.00`, `nan`, `-nan`, `inf`, `-inf`, rounding at 2 dp) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(pow SHARED src/pow.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. There is therefore no driver
binary whose stdout could be compared; that completion-gate item is
inapplicable (`grep -c add_executable c_src/CMakeLists.txt` → 0).

## Feature combinations

No `[features]` in `Cargo.toml` → one combination (the default). See
`SYMBOLS.md`.

## Phase B status — all 35 rows pass

Each row `Cn` is implemented by the test `cn_*` in
`tests/phase_b_valid_paths.rs`; each test loads **both** `.so` files with
`libloading`, calls the exported `my_pow` symbol (never the Rust function
directly), and compares three observables per input: the returned `double`
**bit-for-bit** (`f64::to_bits`, so `±0.0` and NaN payloads are distinguished),
the exact bytes written to `stderr`, and the value of `errno` left behind.
Every row is run twice per input — once with `errno == 0` on entry and once with
a neutral non-zero `errno` — so the `errno = 0` at the top of the C function is
itself under test.

Randomized rows use a fixed-seed xorshift64\* PRNG (`common::Rng`), so runs are
reproducible: C27–C31 contribute 6,300 random pairs, and the structured Phase D
grids below extend this to every possible biased-exponent field of both
parameters.

```
$ cargo test --release --test phase_b_valid_paths
test result: ok. 35 passed; 0 failed
```

## Mechanical backstop (Phase D sweeps)

Because this table is hand-derived from the C source, it can only be as complete
as that reading. `tests/phase_d_sweeps.rs` adds ~1.2 M further input pairs that
do not depend on it:

| test | what it covers |
|------|----------------|
| `d1` | 150 k uniformly random 64-bit patterns for both operands |
| `d2` | 150 k `±m × 10^k` pairs spanning every decade from `10^-320` to `10^308` |
| `d3` | 150 k integer exponents in `[-2100, 2100]`, both parities, both base signs |
| `d4` | 120 k pairs placed within a few ULPs of the overflow / underflow thresholds |
| `d5` | 120 k pairs with `\|base\|` a few ULPs from `1.0` or from `0.0`, huge exponents |
| `d6` | 120 k special-value / random mixtures |
| `d7` | 1 500 randomized inputs with full `stderr` byte comparison, biased so both diagnostics fire ≥100 times |
| `d13` | **every** one of the 2048 biased-exponent fields of `base`, × 4 mantissas × 2 signs × 16 exponents |
| `d14` | the same systematic walk over the exponent parameter |

`d2`, `d3` and `d4` additionally assert their own coverage — the sweep fails if
it did not actually reach the no-error, `EDOM` *and* `ERANGE` outcomes, so a
sweep cannot pass by accidentally testing only the happy path.

## Configuration axes outside the C source but observable through it

`tests/phase_d_threads_and_fenv.rs` and `tests/phase_d_buffered_stderr.rs` cover
three axes that the C source never mentions yet which its behaviour depends on:

| test | axis |
|------|------|
| `d9` | `stderr` switched to **fully buffered** via `setvbuf`; both libraries must still have their diagnostic sitting in the `FILE` buffer before `fflush`, proving the Rust port writes through glibc's `stderr` stream rather than `write(2)` or Rust's own `io::stderr()` |
| `d10` | 8 threads × 20 000 pairs concurrently, deliberately driving different error branches at once — catches an `errno` that is not per-thread |
| `d11` | a parked thread holds `errno == EDOM` while this thread makes successful calls; both libraries must still return the true result |
| `d12` | all four IEEE rounding modes (`FE_TONEAREST`, `FE_DOWNWARD`, `FE_UPWARD`, `FE_TOWARDZERO`) × 40 000 pairs each — rounding mode shifts the thresholds at which `pow` reports `ERANGE` |
