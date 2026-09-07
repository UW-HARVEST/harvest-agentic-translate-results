# CONFIGS.md — Phase A: Configuration-surface table

Mechanically derived from `c_src/include/pow.h` + `c_src/src/pow.c` +
`c_src/CMakeLists.txt`.

## Axes the C actually branches on

**Runtime options / modes / flags:** NONE. `c_src` has no init function, no
context struct, no setter, no global configuration variable, and no `#ifdef`
outside the `ECHO_H_` include guard. `grep -n '#if\|#ifdef\|#ifndef' c_src/src/pow.c
c_src/include/pow.h` yields only the header guard. So the runtime-option axis is
a single point.

**Build/feature configurations:** `translation/Cargo.toml` declares **no
`[features]` table**, so the only feature combination is the default (empty) one.
`crate-type = ["cdylib"]`. This is confirmed and re-verified in Phase D.

**Public entry points:** exactly one — `my_pow(double, double)`. There is no
convenience-vs-low-level split; `my_pow` *is* the lowest-level entry point. The
CMake project builds **only** a shared library (`add_library(pow SHARED ...)`),
**no binary/driver executable**, so the "compare binary stdout" gate is N/A.

**Observable outputs per call (all three compared byte-for-byte):**
1. the returned `double` — compared by **raw bit pattern** (`to_bits()`), not by
   `==`, so that `+0.0` vs `-0.0` and distinct NaN payloads are distinguished;
2. the bytes written to `stderr` (captured by redirecting fd 2 to a temp file);
3. the value of `errno` as observed by the caller after the call returns.

**Input SHAPES the code special-cases.** The C body has no shape branches of its
own, but it branches on `errno` from glibc `pow`, whose behaviour is fully
shape-dependent (C99 Annex F.9.4.4). The shape axes are therefore:

- `base` sign class: `+normal`, `-normal`, `+0.0`, `-0.0`, `+inf`, `-inf`, `NaN`,
  `+subnormal`, `-subnormal`, exactly `1.0`, exactly `-1.0`
- `base` magnitude class: `<1`, `==1`, `>1`, near `DBL_MAX`, near `DBL_MIN`
- `exponent` integrality: integral-even, integral-odd, non-integral, huge (beyond
  integral-precision), `±0.0`, `±inf`, `NaN`, `±0.5` (root), `±1.0`
- result class: normal, subnormal, flush-to-zero, overflow-to-inf, NaN

## Configuration table (cross-product, pruned to combinations glibc `pow` distinguishes)

Each row is exercised with **many randomized inputs** drawn from that row's shape
class using a fixed-seed xorshift PRNG (seed `0x2545F4914F6CDD1D`), not a single
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `my_pow` | positive normal base > 1, small positive integral exponent (even) — randomized `base∈(1,100)`, `exp∈{2,4,…,40}` | [x] |
| C2 | `my_pow` | positive normal base > 1, small positive integral exponent (odd) | [x] |
| C3 | `my_pow` | positive normal base > 1, positive **non-integral** exponent — randomized `exp∈(0,40)` | [x] |
| C4 | `my_pow` | positive normal base > 1, **negative** integral exponent (even and odd) | [x] |
| C5 | `my_pow` | positive normal base > 1, negative non-integral exponent | [x] |
| C6 | `my_pow` | positive normal base **< 1** (`(0,1)`), positive integral / non-integral exponent | [x] |
| C7 | `my_pow` | positive normal base < 1, negative exponent (result grows) | [x] |
| C8 | `my_pow` | **negative** base, integral **even** exponent (positive result, no EDOM) | [x] |
| C9 | `my_pow` | negative base, integral **odd** exponent (negative result, no EDOM) | [x] |
| C10 | `my_pow` | negative base, **negative** integral exponent (even / odd) | [x] |
| C11 | `my_pow` | base `== 1.0` with arbitrary exponent incl. `NaN`, `±inf`, huge | [x] |
| C12 | `my_pow` | base `== -1.0` with integral / `±inf` / `NaN` exponent | [x] |
| C13 | `my_pow` | exponent `== +0.0` and `== -0.0` with arbitrary base incl. `NaN`, `±inf`, `±0.0` | [x] |
| C14 | `my_pow` | exponent `== 1.0` / `== -1.0` with arbitrary base (identity / reciprocal) | [x] |
| C15 | `my_pow` | exponent `== 0.5` / `== -0.5` (square root) with positive base | [x] |
| C16 | `my_pow` | base `== +0.0`, positive exponent (integral, non-integral, `+inf`) | [x] |
| C17 | `my_pow` | base `== -0.0`, positive exponent (odd integral → `-0.0`, even integral → `+0.0`, non-integral → `+0.0`) | [x] |
| C18 | `my_pow` | base `== +inf`, exponent positive / negative / `±0.0` / `NaN` | [x] |
| C19 | `my_pow` | base `== -inf`, exponent positive-odd / positive-even / negative-odd / negative-even / non-integral / `±0.0` | [x] |
| C20 | `my_pow` | exponent `== +inf`, base with `|base|<1` / `==1` / `>1` / `±0.0` / `±inf` / `NaN` | [x] |
| C21 | `my_pow` | exponent `== -inf`, base with `|base|<1` / `==1` / `>1` / `±0.0` / `±inf` / `NaN` | [x] |
| C22 | `my_pow` | `NaN` base with non-zero exponent; `NaN` exponent with base ≠ 1; both `NaN` — incl. distinct quiet-NaN **payloads** and signalling-NaN bit patterns | [x] |
| C23 | `my_pow` | **subnormal** base (positive and negative), assorted exponents | [x] |
| C24 | `my_pow` | exponent producing a **subnormal, nonzero** result (no ERANGE) — e.g. `(2.0, -1030±)` sweep across the whole subnormal band | [x] |
| C25 | `my_pow` | boundary at the **smallest normal**: `(2.0, -1022.0)` and neighbours, `DBL_MIN`-scaled bases | [x] |
| C26 | `my_pow` | boundary at the **largest finite**: bases near `DBL_MAX`, exponents just under the overflow threshold (result finite, no ERANGE) | [x] |
| C27 | `my_pow` | **huge integral** exponents beyond exact-integer precision (`1e17`, `2^53`, `2^53+1`) with base `1.0`, `-1.0`, `>1`, `<1` | [x] |
| C28 | `my_pow` | fully **unstructured random bit patterns** for both operands (uniform over all 2^64 encodings, incl. NaNs/infs/zeros/subnormals) — 200 000 pairs, the broadest sweep | [x] |
| C29 | `my_pow` | randomized pairs drawn from a **curated special-value pool** cross-product (all 40+ special doubles × all 40+ special doubles, exhaustive) | [x] |
| C30 | `my_pow` | **stale `errno`** configuration: caller pre-sets `errno` to `EDOM`/`ERANGE`/other before a *valid* call (verifies the `errno = 0` reset in the C is replicated) | [x] |
| C31 | `my_pow` | **repeated / interleaved** calls: an error-triggering call followed by a valid call, and vice versa, verifying no state leaks between calls in either library | [x] |
| C32 | `my_pow` | happy-path inputs whose genuine result is exactly `-1.0` (aliases the error sentinel) — `(-1.0, odd int)` | [x] |

## Checklist

- [x] All 32 rows exercised against BOTH `.so`s via `libloading`, comparing
      return bits + stderr bytes + caller-visible `errno`.
- [x] No binary/driver in `c_src/CMakeLists.txt` → stdout-comparison gate N/A.
- [x] Only one feature combination exists (no `[features]` in `Cargo.toml`).
