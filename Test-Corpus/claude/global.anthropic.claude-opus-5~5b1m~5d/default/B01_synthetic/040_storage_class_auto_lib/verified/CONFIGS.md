# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Derived mechanically from the C source and public header.

## Axes the C code actually distinguishes

Public entry points (`c_src/include/driver.h`) — the FULL set:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(int x)` | lowest level == only level; there is no convenience wrapper and no other exported function |

Runtime options / modes / flags: **none.** `grep -nE 'if|switch|#ifdef|#if'
c_src/src/driver.c` matches nothing but the include guard, and `CMakeLists.txt`
defines no compile-time options (`add_library(driver SHARED src/driver.c)` only).
There is no global state, no init/config function, no environment variable read,
and no `#ifdef`-gated behaviour.

Input shapes the code is sensitive to: the single `int` argument. Although the
source is branch-free, the *value* of `x` selects materially different behaviour
in the parts that are not written by hand — the two's-complement wraparound of
`2*x` and `y += 300`, and `printf("%d\n", …)`'s sign/digit-count formatting. Those
are the axes below.

Observable output: bytes written to `stdout` by `printf`. Every row is verified by
capturing `stdout` (via `dup2` fd-redirection) around the call into **both** the C
`.so` and the Rust `.so`, loaded with `libloading`, and comparing byte-for-byte.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | no options (none exist); `x = 0` — identity/zero shape | [x] |
| 2 | `driver` | `x` small positive, randomized in `1..=1_000` (output 3–4 digits, no overflow) | [x] |
| 3 | `driver` | `x` small negative, randomized in `-1_000..=-1` (output crosses zero, `%d` minus-sign path) | [x] |
| 4 | `driver` | `x = -150` exactly → output is the single byte-pair `0\n` (result exactly zero) | [x] |
| 5 | `driver` | `x` in `-160..=-140` (exhaustive sweep across the zero crossing: negative → 0 → positive output) | [x] |
| 6 | `driver` | `x` randomized in `0..=INT_MAX/2` (positive, no overflow in either step) | [x] |
| 7 | `driver` | `x` randomized in `INT_MIN/2..=-1` (negative, no overflow in either step) | [x] |
| 8 | `driver` | `x` randomized in `INT_MAX/2+1..=INT_MAX` (the `2*x` multiply overflows → wraps negative) | [x] |
| 9 | `driver` | `x` randomized in `INT_MIN..=INT_MIN/2-1` (the `2*x` multiply overflows negatively → wraps positive) | [x] |
| 10 | `driver` | `x` in the narrow band `1_073_741_670..=1_073_741_680` where only the `y += 300` step overflows (exhaustive sweep of the exact `(INT_MAX-300)/2` boundary) | [x] |
| 11 | `driver` | extreme boundary constants: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `INT_MAX/2`, `INT_MAX/2+1`, `INT_MIN/2`, `INT_MIN/2-1`, `±1`, `±2`, `-149`, `-151` | [x] |
| 12 | `driver` | output-width shape: `x` chosen so the printed value has 1, 2, 3, …, 10 digits, both signs (exercises `%d` field widths) | [x] |
| 13 | `driver` | `x` randomized uniformly over the **full** `i32` range, 20 000 samples, fixed seed (value-dependent + wraparound coverage in one property-style sweep) | [x] |
| 14 | `driver` | repeated-call / stream-state shape: 256 randomized calls into the *same* loaded library without an intervening flush, comparing the whole accumulated stdout stream (catches any per-call buffering or state divergence) | [x] |
| 15 | `driver` | library-load shape: the Rust `.so` built in **debug** (overflow checks ON) and in **release** (`panic = "abort"`, checks off) each compared against the C `.so` — the same rows must pass for both artifacts | [x] |

15 rows. Rows are the pruned cross-product of {no options} × {the value shapes the
arithmetic and formatting actually distinguish} × {load-time artifact shapes}.
