# CONFIGS.md — Configuration surface table (Phase A → Phase B)

## How this table was derived

From the C source only. The axes below are the ones the C code *actually*
branches on or is shaped by; nothing here is guessed.

### Axis 1 — public entry points (the FULL set, lowest level first)

`nm -D --defined-only c_src/build/libdriver.so` yields exactly five. They form
a call hierarchy, and the table drives the **low-level ones directly**, not
just the `driver()` one-shot wrapper:

| level | entry point | callees |
|-------|-------------|---------|
| L0 (leaf) | `printLine(const char*)`      | `printf`/`puts` |
| L0 (leaf) | `printIntLine(int)`           | `printf` |
| L1        | `bad(float)`                 | `printIntLine` |
| L1        | `good(float)`                | `goodG2B` (static) -> `printIntLine`; `goodB2G` (static) -> `printIntLine` / `printLine` |
| L2 (top)  | `driver(float, float)`       | `printLine` x4, `good`, `bad` |

`goodG2B` / `goodB2G` are `static`, so they are only reachable through `good()`
and `driver()` — rows 20+ exist specifically to exercise that composed pipeline.

### Axis 2 — runtime options / modes / flags

**There are none.** Exhaustive check:

```sh
grep -nE "#if|#ifdef|#ifndef|switch|extern .*=|static .*=|getenv" c_src/src/driver.c
```

finds no preprocessor configuration, no globals, no setters, no mode enums, no
environment lookups. The library is stateless: output depends only on the
arguments of the current call. `driver.h`'s single `#ifndef DRIVER_H_` is an
include guard, not a feature toggle. `Cargo.toml` likewise has no `[features]`,
so `default` == `--no-default-features` == the whole cross-product.

The one piece of *implicit* shared state is libc's `stdout` buffer, which both
`.so`s write into — so **call ordering / interleaving** is treated as an axis
(rows 24-26).

### Axis 3 — input shapes the code special-cases

* `float data`, branched on by `driver.c:61` (`fabs(data) > 0.000001`) and
  consumed by `100.0 / data` at `driver.c:45/54/63`. Distinct shapes:
  sign (+/-), zero, subnormal, below-threshold, exactly-threshold,
  just-above-threshold, small, ~1, large, `FLT_MAX`, inf, NaN — plus the
  int-range cliff of the quotient at `|100.0/data| >= 2^31`
  (i.e. `|data| <= ~4.66e-8`) which changes what `cvttsd2si` yields.
* `int intNumber`, consumed by `printf("%d\n", ...)`: negative / zero /
  positive / `INT_MIN` / `INT_MAX` (digit count and sign change the bytes).
* `const char *line`, consumed by `printf("%s\n", ...)` (GCC rewrote this to
  `puts`): NULL / empty / 1 byte / many bytes / bytes past the 4 KiB stdio
  buffer / non-UTF-8 bytes / format-specifier bytes.
* `float` **bit patterns** rather than values, since the argument crosses the
  FFI boundary in `xmm0` and any 32 bits are a legal input.

## The table

One row per combination the C treats differently. Every row is driven with
**many randomized inputs** (`SEED = 0x5EED_1234_ABCD_0001`, splitmix64 —
reproducible) unless the row is a single exact bit pattern.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `printIntLine` | uniform random `i32` over the full range, 4000 draws | [x] |
| 2  | `printIntLine` | boundary set: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` | [x] |
| 3  | `printIntLine` | small magnitudes `-1000..=1000` (every digit count + sign) | [x] |
| 4  | `printIntLine` | powers of two and `±10^k` (digit-count carries) | [x] |
| 5  | `printLine`    | random ASCII printable strings, lengths 0..64, 2000 draws | [x] |
| 6  | `printLine`    | random byte strings over `0x01..=0xFF` (non-UTF-8), lengths 0..64, 2000 draws | [x] |
| 7  | `printLine`    | length sweep 0,1,2,…,80 plus 4095/4096/4097 and 65536 (stdio buffer crossings) | [x] |
| 8  | `printLine`    | strings that are themselves format specifiers (`%s`, `%d`, `%n`, `%%`) | [x] |
| 9  | `printLine`    | strings containing `\t`, `\r`, `\x0b`, and an interior `\n` | [x] |
| 10 | `bad`          | uniform random `f32` **bit patterns** (all 2^32 reachable, incl. NaN/inf/subnormal), 20000 draws | [x] |
| 11 | `bad`          | random *finite normal* `f32` in `±[1e-3, 1e3]`, 8000 draws (in-range quotient) | [x] |
| 12 | `bad`          | random `f32` in `±[1e-45, 1e-6]` (subnormal + tiny → quotient overflows `int`), 4000 draws | [x] |
| 13 | `bad`          | random large `f32` in `±[1e6, FLT_MAX]` (quotient truncates to 0), 4000 draws | [x] |
| 14 | `bad`          | exact `f32` values straddling the `cvttsd2si` cliff: `100/2^31`, `nextafter` on both sides, `±`variants | [x] |
| 15 | `bad`          | `±0.0`, `±inf`, quiet/signalling/negative NaN, `±FLT_MIN`, `±FLT_MAX`, `±1.0` | [x] |
| 16 | `good`         | uniform random `f32` bit patterns, 20000 draws (drives `goodG2B` **and** `goodB2G`) | [x] |
| 17 | `good`         | random `f32` with `|data|` in `(1e-6, 1e-3)` — just inside the guard, 6000 draws | [x] |
| 18 | `good`         | random `f32` with `|data|` in `(0, 1e-6]` — the `else` branch, 6000 draws | [x] |
| 19 | `good`         | exact guard boundary: `1e-6f`, `nextafter(1e-6f, ±inf)`, `-1e-6f`, and the `double` literal `0.000001` as `f32` | [x] |
| 20 | `driver`       | random `(goodData, badData)` bit-pattern pairs, 12000 draws (full composed pipeline) | [x] |
| 21 | `driver`       | cross-product of the 10-element degenerate set `{±0, ±inf, NaN, ±1e-7, ±2.0}` x itself = 100 rows | [x] |
| 22 | `driver`       | `goodData` in the guard-passing band x `badData` in the overflow band, 4000 draws | [x] |
| 23 | `driver`       | `goodData` in the guard-failing band x `badData` normal, 4000 draws | [x] |
| 24 | mixed sequence | `printLine`,`printIntLine`,`bad`,`good`,`driver` called back-to-back in one capture, random args, 500 rounds (shared `stdout` buffer / ordering) | [x] |
| 25 | mixed sequence | `driver` called repeatedly (statelessness: N calls == N concatenated single calls) | [x] |
| 26 | mixed sequence | `printLine(NULL)` interleaved between printing calls (skipped branch must not disturb ordering) | [x] |

## How the rows are executed

`translation/tests/differential.rs` contains one case per row, named
`phase_b_rowNN_*`. Each case:

1. resolves the five exported symbols in **both** `.so`s with `libloading`
   (`dlopen`/`dlsym`) — the Rust functions are never called directly, so the
   `#[no_mangle] extern "C"` wrappers are part of what is under test;
2. redirects file descriptor 1 to a temp file, replays the row's whole input
   batch against the C library, restores fd 1, then does the same for the Rust
   library;
3. compares the two byte strings with `assert_eq`-style exactness.

Replaying inputs in **batches inside one capture** is deliberate: it keeps the
run fast *and* means every row also exercises repeated/sequenced calls through
the shared libc `stdout` buffer, which a one-call-per-capture design would not.

Two harness details are load-bearing:

* the suite runs under `harness = false`. libtest writes its own progress text
  to fd 1 from a different thread, and that text lands inside the captured bytes
  and produces spurious divergences. The custom runner in `main()` prints only
  while fd 1 is un-redirected.
* each case runs in a `fork()`ed child. A translation bug that dereferences a
  pointer the C guards against shows up as `SIGSEGV`; without isolation that
  would kill the runner and hide every other result. Signal deaths are reported
  as failures.

`cargo test` alone does **not** re-link the `cdylib`, so it is possible to run
the entire suite against a stale `libdriver.so`. The harness therefore refuses
to run when `src/lib.rs` or `Cargo.toml` is newer than the `.so`. Use
`./run_verification.sh` (or `cargo build --release && cargo test --release`).

## Results

All 26 rows pass, under every configuration:

| configuration | result |
|---------------|--------|
| default features, Rust release cdylib vs C (CMake default, no `-O`) | 58/58 cases pass |
| `--no-default-features` (identical: the crate declares no features) | 58/58 cases pass |
| Rust **debug** cdylib vs the same C reference | 58/58 cases pass |
| Rust release cdylib vs C rebuilt with `-O2` | 58/58 cases pass |
| Rust release cdylib vs C rebuilt with `-O3` | 58/58 cases pass |

The last three matter because `bad()` relies on C undefined behaviour
(`(int)` of an out-of-range `double`). Matching at `-O0`, `-O2` and `-O3` shows
the Rust reproduces the platform's actual `cvttsd2si` semantics rather than an
artefact of one optimisation level. The alternative C builds are produced with
`gcc` into a temp directory; `c_src/` is never modified.

58 cases cover 26 `CONFIGS.md` rows + 32 `ERRORS.md` rows (several error rows
share a case where the C treats them identically) + 2 reachability proofs + the
symbol-parity check.
