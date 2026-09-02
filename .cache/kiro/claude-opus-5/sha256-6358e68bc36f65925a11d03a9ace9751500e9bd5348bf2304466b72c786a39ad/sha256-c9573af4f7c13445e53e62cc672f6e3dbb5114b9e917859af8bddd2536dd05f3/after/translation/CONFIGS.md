# CONFIGS.md — configuration-surface table (Phase B gate)

Derived mechanically from `c_src/include/pow.h` + `c_src/src/pow.c`.

## Axes the C actually has

**Public entry points** — the header declares exactly one, and it is also the
lowest level one; there is no convenience wrapper / one-shot wrapper layering:

```
$ grep -n '(' c_src/include/pow.h
28:double my_pow(double base, double exponent);
```

**Runtime options / modes / flags** — none. `my_pow` takes no flags, no
context struct, no mode enum; there is no global state and no init/teardown.
`grep -n '#if\|#ifdef\|switch' c_src/src/pow.c` → no matches. The only
conditional state the C reads is `errno`, which is not caller-settable in a
way that survives line 33 (`errno = 0`) — covered as `ERRORS.md` rows 10–11.

**Input shapes** — the entire input space is two IEEE-754 binary64 values.
The shapes the code (via glibc `pow`) distinguishes are: sign of base;
integrality/parity of exponent; the five FP classes (normal, subnormal, zero,
infinity, NaN) for each argument independently; magnitudes that drive the
result into overflow / underflow; and — because the error paths print with
`%.2f` — the decimal-formatting shape of each argument.

So the cross-product below is *base FP class × sign* × *exponent FP class ×
sign × integrality/parity* × *result magnitude regime*, pruned to the
combinations glibc/the C treat differently. Rows 1–29 are the enumerated
combinations; rows 30–34 are bulk randomized sweeps over the whole space.

Every row is exercised through **both** `.so`s via `libloading`, comparing
(a) the returned `double` **bit pattern** (not `==`, so `-0.0` vs `0.0` and
NaN payloads are caught), and (b) the exact bytes written to `stderr`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `my_pow` | base normal `> 1`, exponent positive small integer (`2^10`) | [x] |
| 2  | `my_pow` | base normal `> 1`, exponent negative integer (`2^-10`) | [x] |
| 3  | `my_pow` | base normal `> 1`, exponent positive non-integral (`2^0.5` — irrational result, rounding-sensitive) | [x] |
| 4  | `my_pow` | base normal in `(0,1)`, exponent positive non-integral | [x] |
| 5  | `my_pow` | base normal `< 0`, exponent **odd** integer → negative result | [x] |
| 6  | `my_pow` | base normal `< 0`, exponent **even** integer → positive result | [x] |
| 7  | `my_pow` | base normal `< 0`, exponent large integral valued but `> 2^53` (parity undetectable) | [x] |
| 8  | `my_pow` | base `+1.0`, arbitrary exponent incl. NaN/inf → always `1.0` | [x] |
| 9  | `my_pow` | base `-1.0`, exponent `±inf` → `1.0`; `-1.0` with odd/even int exponent | [x] |
| 10 | `my_pow` | exponent `+0.0` with every base class (normal, NaN, inf, zero) → `1.0` | [x] |
| 11 | `my_pow` | exponent `-0.0` with every base class → `1.0` | [x] |
| 12 | `my_pow` | exponent `1.0` → base returned, incl. `-0.0` sign preservation | [x] |
| 13 | `my_pow` | exponent `-1.0` on normal base (reciprocal) | [x] |
| 14 | `my_pow` | exponent `0.5` on positive base (`sqrt` path) | [x] |
| 15 | `my_pow` | base `+0.0`, exponent positive (odd int / even int / fractional) → `+0.0` | [x] |
| 16 | `my_pow` | base `-0.0`, exponent positive **odd** int → `-0.0` (sign of zero) | [x] |
| 17 | `my_pow` | base `-0.0`, exponent positive **even** int / fractional → `+0.0` | [x] |
| 18 | `my_pow` | base `±0.0`, exponent `±0.0` → `1.0` | [x] |
| 19 | `my_pow` | base `+inf`, exponent positive / negative / zero | [x] |
| 20 | `my_pow` | base `-inf`, exponent positive odd int / even int / fractional / negative | [x] |
| 21 | `my_pow` | exponent `+inf` with base `|b|>1`, `|b|<1`, `|b|==1` | [x] |
| 22 | `my_pow` | exponent `-inf` with base `|b|>1`, `|b|<1`, `|b|==1` | [x] |
| 23 | `my_pow` | subnormal base, exponent small (`1.0`, `2.0`, `0.5`) | [x] |
| 24 | `my_pow` | subnormal exponent (≈`5e-324`) with assorted bases | [x] |
| 25 | `my_pow` | boundary magnitudes: `DBL_MAX`, `DBL_MIN`, `DBL_EPSILON`, `2^-1074` as base and as exponent | [x] |
| 26 | `my_pow` | result exactly at the overflow boundary (`DBL_MAX^1`, `2^1024` vs `2^1023`) | [x] |
| 27 | `my_pow` | result at the underflow / subnormal boundary (`2^-1022`, `2^-1074`, `2^-1075`) | [x] |
| 28 | `my_pow` | integral exponents swept `-64..=64` against a set of bases (sign/parity cross-product) | [x] |
| 29 | `my_pow` | repeated / interleaved calls: error call followed by valid call, in both orders, same thread (state carry-over) | [x] |
| 30 | `my_pow` | **randomized**: uniform random bit patterns in both arguments (any FP class, any NaN payload), fixed seed | [x] |
| 31 | `my_pow` | **randomized**: random "reasonable" magnitudes, base in `[-1e3,1e3]`, exponent in `[-300,300]` (mixes success/overflow/underflow/domain) | [x] |
| 32 | `my_pow` | **randomized**: random negative base × random non-integral exponent (mass domain-error sweep, exercises `%.2f` of many values) | [x] |
| 33 | `my_pow` | **randomized**: random base × random *integral* exponent (no domain error; overflow/underflow mix) | [x] |
| 34 | `my_pow` | **randomized**: values drawn from a pool of special constants, cross-producted pairwise (all class×class combinations) | [x] |

## Feature combinations

```
$ grep -n '^\[features\]' translation/Cargo.toml   # no matches
```

The crate declares **no** `[features]` and no `default` feature, so the
feature power-set is the single empty configuration. `cargo check` /
`cargo test` with `--no-default-features` is therefore identical to the
default build; both are run by `run_all.sh` for completeness, together with
`--release` (which additionally enables `panic = "abort"` and full
optimisation — the configuration where an `errno`/`pow` reordering bug would
show up if the barriers in `src/ffi.rs` were insufficient).
