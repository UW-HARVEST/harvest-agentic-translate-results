# CONFIGS.md — Configuration-surface table (Phase B)

## Axis derivation

There is exactly **one** public entry point (`c_src/include/pow.h` declares only
`double my_pow(double base, double exponent)`), and it *is* the lowest-level
entry point — there is no convenience wrapper layered over an internal API, and
`pow.c` contains no `static` helpers. So the "full set of public entry points"
is `{ my_pow }`.

**Runtime options / modes / flags:** none. There is no init function, no context
object, no global setting, no `#ifdef`, and no environment lookup. `CMakeLists.txt`
defines no compile-time options; `Cargo.toml` has no `[features]` section. So
the configuration surface is **not** option-driven — it is entirely driven by
**input shape**.

Accordingly the axes below are the input shapes that the code actually branches
on. The C body branches on `errno` (`EDOM` / `ERANGE` / neither) and formats both
arguments with `%.2f`; the value of `errno` and of `result` is decided by
`pow@GLIBC_2.29`, whose own special-case ladder is therefore part of this
library's observable branch structure.

| axis | values the code distinguishes |
|---|---|
| A. base sign | `-`, `-0.0`, `+0.0`, `+` |
| B. base magnitude class | `0`, subnormal, `<1`, `==1`, `>1`, `DBL_MAX`, `INF`, `NaN` |
| C. exponent integrality | `0`, odd integer, even integer, non-integer, non-finite |
| D. exponent sign | `-`, `+` |
| E. exponent magnitude | subnormal, small, large, `1e18`, `INF` |
| F. resulting errno branch | `EDOM`, `ERANGE` (pole / overflow / underflow), none |
| G. `%.2f` rendering shape of the two args | ordinary, `-0.00`, rounds-to-`0.00`, 309-digit, `inf`, `-inf`, `nan` |
| H. NaN payload / signaling bit | quiet, signaling, negative-signed, random payload |

## Rows

Every row is exercised through **both** `.so`s via `libloading` and compared on
all three observables — **return bit pattern** (`f64::to_bits`, so `-0.0` vs
`+0.0` and NaN payloads are distinguished), **stderr bytes**, and **residual
`errno`**. Rows marked *randomized* draw many inputs from a fixed-seed
SplitMix64 PRNG (see `tests/common/mod.rs`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `my_pow` | positive normal base `>1`, small positive **integer** exponent — *randomized* over base ∈ (1,1000), exp ∈ 0..64 | [x] |
| C2 | `my_pow` | positive normal base `>1`, negative integer exponent — *randomized* | [x] |
| C3 | `my_pow` | positive normal base `>1`, **non-integer** exponent — *randomized* | [x] |
| C4 | `my_pow` | positive normal base `<1`, positive non-integer exponent — *randomized* | [x] |
| C5 | `my_pow` | positive normal base `<1`, negative non-integer exponent — *randomized* | [x] |
| C6 | `my_pow` | exponent `== 0.0` with every base class (normal, `±0`, `±INF`, NaN, subnormal, `±DBL_MAX`) → C99 requires `1.0` for all, incl. NaN | [x] |
| C7 | `my_pow` | exponent `== -0.0` with every base class → `1.0` | [x] |
| C8 | `my_pow` | base `== 1.0` with every exponent class (incl. `NaN`, `±INF`, huge) → `1.0` | [x] |
| C9 | `my_pow` | negative normal base, **odd** integer exponent (sign-preserving path) — *randomized* | [x] |
| C10 | `my_pow` | negative normal base, **even** integer exponent (sign-cancelling path) — *randomized* | [x] |
| C11 | `my_pow` | negative normal base, non-integer exponent → `EDOM` branch + domain message — *randomized* | [x] |
| C12 | `my_pow` | base `== +0.0`, exponent `> 0` (odd int / even int / non-int) → `+0.0`, no errno | [x] |
| C13 | `my_pow` | base `== -0.0`, exponent positive **odd** integer → `-0.0` (sign of zero must survive; `to_bits` check) | [x] |
| C14 | `my_pow` | base `== -0.0`, exponent positive even integer / non-integer → `+0.0` | [x] |
| C15 | `my_pow` | base `== ±0.0`, exponent `< 0` → `ERANGE` pole branch (covers E4–E7) | [x] |
| C16 | `my_pow` | base `== +INF`, exponent `> 0` → `+INF`; exponent `< 0` → `+0.0` — *randomized exponent* | [x] |
| C17 | `my_pow` | base `== -INF`, exponent positive **odd** int → `-INF`; positive even/non-int → `+INF` | [x] |
| C18 | `my_pow` | base `== -INF`, exponent negative odd int → `-0.0`; negative even/non-int → `+0.0` | [x] |
| C19 | `my_pow` | exponent `== +INF`, `\|base\| > 1` → `+INF`; `\|base\| < 1` → `+0.0` — *randomized base* | [x] |
| C20 | `my_pow` | exponent `== -INF`, `\|base\| > 1` → `+0.0`; `\|base\| < 1` → `+INF` — *randomized base* | [x] |
| C21 | `my_pow` | base `== -1.0`, exponent `== ±INF` → `1.0` (the C99 special case that is *not* `\|base\|<1`/`>1`) | [x] |
| C22 | `my_pow` | base `== -1.0`, integer exponents of both parities and signs → `±1.0`, incl. the `-1.0`/sentinel collision | [x] |
| C23 | `my_pow` | **overflow** shapes: `base>1` with large positive exp, `base<1` with large negative exp → `ERANGE` — *randomized* | [x] |
| C24 | `my_pow` | **underflow** shapes: `base>1` with large negative exp, `base<1` with large positive exp → `ERANGE` — *randomized* | [x] |
| C25 | `my_pow` | **boundary straddle**: for random bases, `nextafter`-bisected largest non-overflowing and smallest overflowing exponent (and the underflow mirror) — both sides must classify identically | [x] |
| C26 | `my_pow` | base is **quiet NaN** (`0x7FF8…`), exponent ≠ 0 → NaN propagation, payload compared bit-exactly | [x] |
| C27 | `my_pow` | base is **signaling NaN** (`0x7FF0000000000001`) → must be quieted to `0x7FF8000000000001` with payload preserved | [x] |
| C28 | `my_pow` | base is **negative NaN** (`0xFFF8…`) / exponent is negative NaN → sign-of-NaN handling | [x] |
| C29 | `my_pow` | exponent is NaN (quiet + signaling), base ≠ 1 → NaN propagation, payload bit-exact | [x] |
| C30 | `my_pow` | **both** arguments NaN, with independent random payloads → which payload propagates must match | [x] |
| C31 | `my_pow` | NaN with *random* mantissa payloads, both signs, both quiet/signaling — *randomized* | [x] |
| C32 | `my_pow` | **subnormal base** (`5e-324`, `nextafter(DBL_MIN,0)`, random subnormals), both signs × positive/negative exponents | [x] |
| C33 | `my_pow` | **subnormal exponent** (`5e-324`, random subnormals) with assorted bases → result ≈ `1.0`, no errno | [x] |
| C34 | `my_pow` | `±DBL_MAX` base × exponents `{0, ±1, 2, 0.5, ±INF, NaN}` — includes the 309-digit `%.2f` rendering path | [x] |
| C35 | `my_pow` | `±DBL_MIN` base × the same exponent set — includes the rounds-to-`0.00` rendering path | [x] |
| C36 | `my_pow` | base near `1.0` (`nextafter(1.0, ±INF)`, `1±2^-52`) × huge exponents (`1e15`…`1e18`) — extreme cancellation, no errno | [x] |
| C37 | `my_pow` | exponent `== ±1.0` and `== ±2.0` (identity / squaring shortcuts a compiler may strength-reduce) across all base classes | [x] |
| C38 | `my_pow` | exponent `== 0.5` (sqrt shortcut) and `== ±(1/3)`, `1.5`, `2.5` across all base classes | [x] |
| C39 | `my_pow` | half-integer and exactly-representable-integer exponents up to `2^53` and just past it (`2^53+1`, where integrality/parity detection changes) | [x] |
| C40 | `my_pow` | **`%.2f` rendering matrix**: argument pairs chosen so each renders as ordinary / `-0.00` / `0.00` / `inf` / `-inf` / `nan` / 309-digit, in an error-producing configuration | [x] |
| C41 | `my_pow` | **repeated / stateful calls**: the same pair called many times, and an error-producing call immediately followed by a valid call, to prove no residual state (`errno`, buffering) leaks between calls | [x] |
| C42 | `my_pow` | **interleaving** of C and Rust calls in one process sharing one `errno` TLS slot and one `stderr` `FILE*` | [x] |
| C43 | `my_pow` | *randomized* structured sweep: base drawn log-uniformly over the full exponent range (`1e-308`…`1e308`) × exponent over `-1000..1000`, both signs — hits normal/overflow/underflow/EDOM mix | [x] |
| C44 | `my_pow` | *randomized* **full-entropy fuzz**: both arguments are uniformly random `u64` bit patterns reinterpreted as `f64` (covers all IEEE classes incl. non-canonical NaNs) | [x] |
| C45 | `my_pow` | *randomized* small-integer grid: base ∈ `-20..20` (incl. `±0`) × exponent ∈ `-20..20` exhaustive cross-product | [x] |
| C46 | `my_pow` | **concurrent** calls from 8 threads × 4000 iterations, mixing all three branches, so threads continually overwrite each other's `errno`. `errno` is thread-local, so this is the only row that can catch a translation which cached `__errno_location()` in a global or used a process-wide variable — a bug invisible to every single-threaded row. Implemented in `tests/threads.rs` (its own binary, so the fd-2 redirection cannot race). | [x] |

## Notes on axes that turned out to be unreachable

Derived from the C, then confirmed by test rather than assumed:

- The `inf` / `-inf` / `nan` spellings of `%.2f` (axis G) are **unreachable in the
  error messages**: glibc's `pow` never sets `errno` when either argument is
  non-finite, so a non-finite argument can never reach an `fprintf` call. Row C40
  proves this by asserting an empty stderr for every base/exponent class
  combination in which either argument is non-finite.
- A **signaling** NaN is not covered by the C99 `pow(x, 0) == 1` /
  `pow(1, y) == 1` carve-outs; glibc returns the quieted sNaN with its payload
  preserved. Rows C6/C7/C8 and C27 encode the real behaviour.

## Checklist

- [x] All 46 rows implemented in `tests/configs.rs` / `tests/threads.rs` and passing (see `RESULTS.md`).
