# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/driver.c`

## Axes actually present in the C source

Derived from the source, not from assumptions.

**Runtime options / modes.** Exactly one: the `int useGood` parameter of
`driver`, which selects between the `good()` and `bad()` code paths
(`if (useGood)` at line 59). There are no global flags, no init/config struct, no
environment variables, no `#ifdef`s (`grep -c '#if' c_src/src/driver.c` → 1, the
header's `#ifndef DRIVER_H_` include guard only), and no compile-time features
in `translation/Cargo.toml` (no `[features]` section → the sole build
configuration is `--no-default-features`, which is identical to the default).

**Public entry points.** Four, all exported (`nm -D`), listed low-level first:

1. `printLine(const char *)` — lowest level; the only one taking data
2. `bad(void)` — wraps `printLine(helperBad())`
3. `good(void)` — wraps `printLine(helperGood1())`
4. `driver(int)` — top-level convenience wrapper over `good`/`bad`

Phase B drives **all four** directly, not just `driver`.

**Input shapes the code distinguishes.** `printLine` branches on null vs
non-null and then hands the pointer to C stdio, so the shapes that matter are
the ones stdio and the terminator scan distinguish: length (0 / 1 / small /
around the 1 KiB, 4 KiB, 8 KiB, 64 KiB stdio buffer boundaries), byte values
(ASCII / high-bit / every non-NUL byte 0x01–0xFF), embedded terminators,
embedded newlines, format-specifier bytes, and pointer offset into a larger
allocation. `driver` distinguishes only zero vs non-zero, but the *value* space
is the full `int` range including negatives and both extremes.

**State / sequencing.** `helperGood1`'s array has `static` storage duration, so
its address and contents persist across calls; `helperBad`'s does not. Call
*sequences* are therefore an axis: repeated and interleaved invocations must
produce the same accumulated byte stream in the same order.

## Configuration table

Every row is exercised against **both** `.so`s through `libloading`, comparing
the captured stdout byte-for-byte. Rows marked "randomised" use a fixed-seed
xorshift PRNG (seed `0x2545F4914F6CDD1D`) with the stated iteration count.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | `line = ""` — empty, zero-length string (passes null guard, empty payload) | [x] |
| 2 | `printLine` | single-byte string; **every** value `0x01..=0xFF` exhaustively | [x] |
| 3 | `printLine` | randomised printable-ASCII string, length 1..=64, 256 iterations | [x] |
| 4 | `printLine` | randomised arbitrary non-NUL bytes `0x01..=0xFF` (invalid UTF-8 included), length 1..=256, 256 iterations | [x] |
| 5 | `printLine` | string consisting of printf format specifiers: `%s`, `%n`, `%d`, `%%`, `%1000000d`, mixed | [x] |
| 6 | `printLine` | string containing embedded `\n`, `\r`, `\t`, `\x0b`, `\x0c` (multi-line payload) | [x] |
| 7 | `printLine` | buffer with an embedded `\0` at offset `k` for randomised `k` (truncation at terminator), 128 iterations | [x] |
| 8 | `printLine` | length exactly at/around stdio buffer boundaries: 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193 | [x] |
| 9 | `printLine` | large payloads: 65535, 65536, 65537, 1 MiB | [x] |
| 10 | `printLine` | interior pointer `&buf[k]` into a larger allocation, randomised `k`, 128 iterations | [x] |
| 11 | `printLine` | back-to-back sequence of 64 randomised strings in one capture (ordering + buffer accumulation), 64 iterations | [x] |
| 12 | `printLine` | interleaved with `NULL` arguments inside one capture (valid/invalid mix) | [x] |
| 13 | `bad` | single call, no arguments | [x] |
| 14 | `bad` | 1000 repeated calls in one capture | [x] |
| 15 | `good` | single call, no arguments | [x] |
| 16 | `good` | 1000 repeated calls in one capture (static-storage reuse: same bytes every time) | [x] |
| 17 | `driver` | `useGood = 1` (canonical true → `good`) | [x] |
| 18 | `driver` | `useGood = 0` (canonical false → `bad`) | [x] |
| 19 | `driver` | `useGood` ∈ {`-1`, `2`, `-2`, `i32::MIN`, `i32::MIN+1`, `i32::MAX`, `i32::MAX-1`, `0x0001_0000`, `0xFFFF_0000 as i32`} — non-canonical truthy values | [x] |
| 20 | `driver` | randomised `i32` over the full range, 1024 iterations | [x] |
| 21 | `driver` | randomised values biased to produce many zeros (`v & 1`, `v % 3`), 512 iterations — exercises both branches densely | [x] |
| 22 | `driver`+`good`+`bad`+`printLine` | randomised interleaved call sequence across all four entry points, 64 sequences × 32 calls, one capture per sequence (composed pipeline) | [x] |
| 23 | all four | called after each other in a single process with the *same* loaded handles (no reload) — confirms no per-call init and no cross-call state divergence | [x] |
| 24 | all four | C `.so` loaded first vs Rust `.so` loaded first — confirms `bad`/`good` resolve their `printLine` through their own object (both are `RTLD_LOCAL`, and `bad` calls `printLine@plt` in C) and no cross-object interposition changes the result | [x] |

## Feature-combination matrix

`translation/Cargo.toml` declares no `[features]`, so the complete matrix is:

| combination | command |
|-------------|---------|
| default (empty) | `cargo test --release` |
| `--no-default-features` (identical, no features exist) | `cargo test --release --no-default-features` |

Both are run by `run_all.sh`.
