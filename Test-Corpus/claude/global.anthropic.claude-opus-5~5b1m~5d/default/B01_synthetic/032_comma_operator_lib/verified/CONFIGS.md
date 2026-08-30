# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

Mechanically derived from the C source, not from assumptions.

## Axes the C code actually branches on

The whole library is one function with one loop, so the branch inventory is
short and complete:

1. **Public entry points** — `nm -D` + `c_src/include/driver.h` give exactly one:
   `void driver(int x)`. There is no convenience/one-shot wrapper layer over a
   lower-level API; `driver` *is* the lowest level exported. Nothing is hidden
   behind a simplified facade.
2. **Runtime options / modes / flags** — none. There is no init function, no
   context/handle struct, no setter, no global/static configuration variable, no
   environment lookup, and no `#ifdef` in the compiled source (the only `#if` in
   the tree is the header's include guard). `Cargo.toml` likewise declares **no
   `[features]`**, so there is exactly one feature combination: the default.
3. **Control-flow branch** — `i < x`, the loop guard. Taken/not-taken splits the
   input domain into `x <= 0` (zero iterations, in `ERRORS.md`) and `x >= 1`
   (`x` iterations).
4. **Loop-carried state shapes** — `i` steps by 1 from 0; `j` steps by 2 from 0,
   so `j == 2*i` for every iteration. The two counters cross `%d` decimal-width
   boundaries at *different* values of `i` (`i` at 10, 100, 1000…; `j` at 5, 50,
   500…). Each such crossing is a distinct output shape produced by `printf`'s
   `%d` conversion, so it is a real input-shape axis.
5. **Output volume / stdio buffering** — `printf` to `stdout`. Once total output
   exceeds the stdio buffer (4096 bytes when stdout is a file, as under test)
   the library performs real `write(2)` flushes mid-loop. Small (< 1 buffer),
   buffer-boundary-straddling, and multi-flush volumes are distinct shapes.
6. **Call multiplicity / residual state** — one call vs. many calls vs. C and
   Rust calls interleaved, and calls made from a non-main thread. The C function
   has no `static` storage, so every call must be independent; that invariant is
   itself something to verify differentially.

Everything is exercised through the exported `driver` symbol of BOTH `.so`s,
loaded with `libloading`. Each measurement runs in a `fork()`ed child whose
private fd 1 points at a scratch file, so the captured bytes are exactly what
the library wrote (the test harness's own progress output cannot leak in). Both
the stdout bytes **and** the child's termination status are compared, so a panic
or abort in the Rust build is detected rather than silently matching.

Note: the Rust `.so` is tested in **both** `release` and `debug` builds where
both are present, because `panic = "abort"` and overflow-check settings differ
between profiles and could in principle change behaviour on the arithmetic.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | `x = 1` — minimum accepted count, exactly one iteration; only `i == j == 0` printed | [x] |
| C2 | `driver` | `x = 2` — two iterations, first `j != i` (`j = 2`) | [x] |
| C3 | `driver` | `x = 3, 4` — both counters still single-digit for every line | [x] |
| C4 | `driver` | `x = 5, 6, 7` — `j` crosses 1→2 decimal digits (`j = 10` at `i = 5`) while `i` is still 1 digit: asymmetric field widths | [x] |
| C5 | `driver` | `x = 9, 10, 11` — `i` crosses 1→2 decimal digits | [x] |
| C6 | `driver` | `x = 49, 50, 51` — `j` crosses 2→3 digits (`j = 100` at `i = 50`) | [x] |
| C7 | `driver` | `x = 99, 100, 101` — `i` crosses 2→3 digits | [x] |
| C8 | `driver` | `x = 499, 500, 501` — `j` crosses 3→4 digits | [x] |
| C9 | `driver` | `x = 999, 1000, 1001` — `i` crosses 3→4 digits; output first exceeds the 4096-byte stdio buffer, forcing mid-loop `write(2)` | [x] |
| C10 | `driver` | `x = 4999, 5000, 5001` — `j` crosses 4→5 digits | [x] |
| C11 | `driver` | `x = 9999, 10000, 10001` — `i` crosses 4→5 digits; multi-flush output (~100 KB) | [x] |
| C12 | `driver` | randomised `x` in `1..=9` (many seeded draws) — dense coverage of the all-single-digit regime | [x] |
| C13 | `driver` | randomised `x` in `1..=100` (many seeded draws) | [x] |
| C14 | `driver` | randomised `x` in `1..=2000` (many seeded draws) — straddles the stdio buffer boundary at random offsets | [x] |
| C15 | `driver` | randomised `x` in `2000..=20000` (many seeded draws) — multi-flush volumes at random offsets | [x] |
| C16 | `driver` | `x = 65536` and `x = 100000` — large single call, ~1.2 MB of output, hundreds of buffer flushes | [x] |
| C17 | `driver` | randomised `x` in `100000..=200000` (seeded) — largest practically comparable volume | [x] |
| C18 | `driver` | same `x` invoked repeatedly (10x, `x = 37`) — verifies no residual/static state and byte-identical repeats in both libs | [x] |
| C19 | `driver` | interleaved call sequence C→Rust→C→Rust with *varying* `x` (seeded), each step compared — verifies neither library leaks state across calls or across the other library's calls | [x] |
| C20 | `driver` | `x` = valid count called from a **non-main thread** (no TLS/thread-affine state; both libs) | [x] |
| C21 | `driver` | mixed valid/invalid sequence: `x <= 0` interleaved with `x >= 1` (seeded) — the accepting and rejecting halves of the guard in one session | [x] |
| C22 | `driver` | powers-of-two and off-by-one shapes `x = 2^k` and `2^k ± 1` for `k = 1..=14` — exhaustive width/carry boundary sweep | [x] |
| C23 | `driver` | every `x` in `1..=300` exhaustively (not sampled) — full small-domain coverage, guarantees no single value in the dense regime diverges | [x] |

All 23 rows are exercised against the release Rust `.so` and, when present, the
debug Rust `.so`, in the single (default) feature configuration — the crate
defines no Cargo features.
