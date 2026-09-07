# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Axes actually present in the C code

Derived from `c_src/include/staticloop.h` (the complete public API) and every
branch in `c_src/src/staticloop.c`:

**Public entry points (2, both covered — there is no convenience-vs-low-level
split, but `driver` *is* the composed wrapper over `static_sum`, so both the
wrapper and the low-level function are driven directly):**

* `int static_sum(int update)` — lowest-level entry point; mutates the hidden
  accumulator and returns it.
* `void driver(int stride)` — composed pipeline: 10 iterations of
  `printf("%d\n", static_sum(i * stride))`.

**Runtime options / modes / flags:** *none.* There is no setter, no context
struct, no global config, no environment lookup, no `#ifdef` other than the
header include guard, and no Cargo feature in `translation/Cargo.toml`
(`[features]` section absent ⇒ the only feature combination is the default,
which is empty).

**Branches the C actually takes:** the single `for (int i = 0; i < 10; i++)`
loop in `driver`. Its bounds are compile-time constants, so trip count is always
exactly 10 — independent of input. There is no `if` and no `switch`.

**Hidden state axis (the axis that really matters here):** the function-local
`static int sum`, zero-initialized once per loaded `.so`, never reset. It makes
both entry points *order-dependent*: the result of a call is a function of the
whole preceding call history. Every configuration below is therefore
parameterized by the accumulator's incoming state as well as by the argument.

**Input-shape axis:** `update` / `stride` is a bare `int` — no size, count,
width, element type, format, or byte-order parameter exists. The distinguishable
shapes are the value classes: zero, ±1, small ±, large ± , `INT_MAX`, `INT_MIN`,
and the arithmetic-edge values where `i * stride` or `sum + update` crosses the
32-bit signed boundary.

**Observable channels:** `static_sum`'s `int` return value; `driver`'s stdout
bytes (compared byte-for-byte after `fflush`); and the post-call accumulator
state, probed with `static_sum(0)`.

**Count/multiplicity axis:** zero / one / many calls, and interleavings of the
two entry points.

> All rows are exercised through both `.so` files via `libloading` only. Every
> call is mirrored to C and to Rust under one lock, so the two independent
> accumulators stay in lock-step and remain comparable.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `static_sum` | fresh accumulator (0), single call, `update = 0` | [x] |
| C2 | `static_sum` | fresh accumulator (0), single call, `update = +1` | [x] |
| C3 | `static_sum` | fresh accumulator (0), single call, `update = -1` | [x] |
| C4 | `static_sum` | accumulator positive, many calls, randomized small positive `update` (seeded) | [x] |
| C5 | `static_sum` | accumulator positive, many calls, randomized small negative `update` (seeded) | [x] |
| C6 | `static_sum` | accumulator arbitrary, many calls, randomized full-range `int` `update` (seeded, crosses overflow repeatedly) | [x] |
| C7 | `static_sum` | accumulator driven near `INT_MAX`, then `update > 0` ⇒ positive wrap | [x] |
| C8 | `static_sum` | accumulator driven near `INT_MIN`, then `update < 0` ⇒ negative wrap | [x] |
| C9 | `static_sum` | `update = INT_MAX` from a fresh-equivalent state, twice in a row | [x] |
| C10 | `static_sum` | `update = INT_MIN` twice in a row (`INT_MIN + INT_MIN` wrap) | [x] |
| C11 | `static_sum` | long run of many calls (1000+) checking the accumulator identity holds cumulatively | [x] |
| C12 | `driver` | `stride = 0` — degenerate: all 10 addends are 0, accumulator unchanged, 10 identical lines | [x] |
| C13 | `driver` | `stride = 1` — canonical: addends `0..9`, sum grows by 45 | [x] |
| C14 | `driver` | `stride = -1` — negative addends, sum drops by 45 | [x] |
| C15 | `driver` | small positive `stride` (randomized, seeded), accumulator fresh/positive | [x] |
| C16 | `driver` | small negative `stride` (randomized, seeded) | [x] |
| C17 | `driver` | full-range randomized `stride` (seeded) — `i * stride` wraps for large `i` | [x] |
| C18 | `driver` | `stride = INT_MAX/9` — largest stride for which no `i * stride` overflows | [x] |
| C19 | `driver` | `stride = INT_MAX/9 + 1` — first stride where `9 * stride` overflows | [x] |
| C20 | `driver` | `stride = INT_MAX` / `INT_MIN` — `i * stride` overflows from `i = 2` on | [x] |
| C21 | `driver` | repeated `driver` calls back-to-back with the same stride (order-dependence of the shared accumulator across whole pipeline runs) | [x] |
| C22 | `driver` | `driver` invoked on a *non-fresh* accumulator (preceded by `static_sum` calls) — stdout depends on carried-in state | [x] |
| C23 | `static_sum` + `driver` | interleaved: `static_sum`, `driver`, `static_sum`, `driver`, ... with randomized args (seeded) | [x] |
| C24 | `static_sum` after `driver` | accumulator value observed via `static_sum(0)` immediately after each `driver` — verifies `driver` left identical hidden state in both libs | [x] |
| C25 | `driver` | stdout byte-for-byte: exact `"%d\n"` formatting incl. negative sign and no trailing/leading padding, captured via fd-1 redirection and `fflush` | [x] |
| C26 | both | zero calls / library-load only: fresh accumulator reads as 0 in both (`static_sum(0) == 0` as the very first call) | [x] |
| C27 | both | fixed-seed randomized *program*: 2000 randomly chosen operations (`static_sum` with a random `int`, or `driver` with a random `int`) applied to both libs, comparing return values and stdout at every step | [x] |

Feature combinations: the crate declares no `[features]`, so the only
combination is the default (empty) one; `--no-default-features` is equivalent
and is also run.

Binary/driver executable: `c_src/CMakeLists.txt` declares only
`add_library(StaticLoop SHARED ...)` — there is **no** `add_executable`, and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]`. Hence there is no binary-stdout comparison to perform; the
`driver`-stdout comparison in rows C12–C25 covers the printing path.

## Traceability

Row `Cn` ↔ test `cn_*`:

* C1–C25, C27 → `tests/phase_b_valid.rs`
* C26 → `tests/c26_fresh_state.rs` (its own test binary: only the first call in a
  fresh process can observe the zero-initialized accumulator, and cargo runs each
  test target in a separate process)

Run with `cargo test -- --test-threads=1` (single-threaded is required: the
`driver` rows compare stdout by temporarily redirecting fd 1, and libtest's own
progress output also goes to fd 1). `./verify.sh` does the whole matrix.

Beyond comparing C against Rust, every randomized row is also checked against an
independent reference `Model` in `tests/common/mod.rs`, so a shared
misunderstanding between the two shared objects cannot pass silently.
