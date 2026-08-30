# CONFIGS.md — Configuration / valid-input surface table

Derived mechanically from `c_src/include/driver.h` + `c_src/src/driver.c`.

## Axes the C code actually branches on

There is **no runtime option, mode, flag, global, or `#ifdef`** in this library
(`grep -c '#if\|#ifdef\|static .*=\|extern .*;' src/driver.c` finds no
configuration state; the only `#ifndef` is the header include guard). The library
is pure and stateless. Therefore the configuration axes are entirely
*input-shape* axes:

| axis | values the C distinguishes | evidence |
|------|---------------------------|----------|
| A. entry point | `printLine`, `printIntLine`, `bad`, `good`, `driver` (all 5 exported symbols — **including the low-level ones**, not only the `driver` one-shot wrapper) | `nm -D` |
| B. pointer validity (`printLine`) | NULL / non-NULL | `if (line != NULL)` :31 |
| C. byte-string shape (`printLine`) | empty / 1 byte / ASCII / embedded `%` / embedded `\n` / high (non-UTF-8) bytes / very long | `printf("%s\n", line)` :33 |
| D. `int` value shape (`printIntLine`) | 0 / positive / negative / `INT_MIN` / `INT_MAX` | `printf("%d\n", n)` :39 |
| E. index sign (`bad`, `goodB2G`) | `data >= 0` vs `data < 0` | :46, :85 |
| F. index magnitude (`goodB2G` only) | `data < 10` vs `data >= 10` | :85 (`bad` has **no** upper guard) |
| G. index position within buffer | first (`0`), interior (`1..8`), last (`9`) — each selects a *different* element of the 10-line dump, so each is a distinct output shape | `buffer[data] = 1` + `for(i=0;i<10;i++)` |
| H. index one past / far past the end (`bad` only) | `10`, `11..`, i.e. the unchecked overflow | missing upper guard at :46 |
| I. composition (`good`) | `goodG2B()` (fixed `data = 7`) always runs, **then** `goodB2G(data)` | :102–103 |
| J. composition (`driver`) | cross product of `goodData` × `badData` through the full 6-print pipeline | :106–114 |

## Table — one row per combination the C treats differently

| # | entry point(s) | configuration (options set + input shape) | randomized? | [x] |
|---|----------------|------------------------------------------|-------------|-----|
| 1 | `printLine` | non-NULL, 1-byte ASCII string | 256 random single bytes (non-zero) | [x] |
| 2 | `printLine` | non-NULL, empty string `""` | fixed (degenerate) | [x] |
| 3 | `printLine` | non-NULL, random printable ASCII, len 1..200 | 200 seeded random strings | [x] |
| 4 | `printLine` | non-NULL, contains `printf` specifiers (`%s`, `%d`, `%n`, `%%`) | 100 seeded random specifier soups | [x] |
| 5 | `printLine` | non-NULL, contains embedded `\n`, `\r`, `\t` | 100 seeded random | [x] |
| 6 | `printLine` | non-NULL, arbitrary non-NUL bytes 0x01..0xFF (non-UTF-8) | 200 seeded random byte strings | [x] |
| 7 | `printLine` | non-NULL, very long (4 KiB .. 64 KiB) | 8 seeded random lengths | [x] |
| 8 | `printLine` | NULL | fixed (covered again in ERRORS row 1) | [x] |
| 9 | `printIntLine` | `data == 0` | fixed | [x] |
| 10 | `printIntLine` | `data > 0`, full 31-bit range | 500 seeded random positives | [x] |
| 11 | `printIntLine` | `data < 0`, full range | 500 seeded random negatives | [x] |
| 12 | `printIntLine` | `INT_MIN`, `INT_MAX`, `-1`, `1` | fixed boundaries | [x] |
| 13 | `bad` | `data == 0` — write to first element | fixed | [x] |
| 14 | `bad` | `data` interior in-bounds `1..=8` | all 8 values | [x] |
| 15 | `bad` | `data == 9` — write to last element | fixed | [x] |
| 16 | `bad` | `data` in `0..=9`, randomized | 300 seeded random in-range | [x] |
| 17 | `bad` | `data < 0` (negative branch) | 300 seeded random negatives + `INT_MIN` | [x] |
| 18 | `bad` | `data == 10` — one past end, unchecked write (CWE-121); lands in frame padding | fixed | [x] |
| 19 | `bad` | `data == 11` — last overflow slot still inside `bad`'s own frame; aliases the loop counter `i` at `-0x4(%rbp)` | fixed, batched + isolated | [x] |
| 19b | `bad` | `data >= 12` — the write leaves `bad`'s frame and hits the caller's saved `rbp` / return address. **Not status-comparable**: measured crash indices differ in both directions and vary with call depth. Compared for identical *printed output* instead, over `12..=400`, plus `{1e5, 1e6, 1e8, INT_MAX}` where both must die. See `UB.md`. | `tests/phase_b_ub.rs`, 389 indices | [x] |
| 20 | `good` | `data == 0` (goodG2B dump + goodB2G writes elem 0) | fixed | [x] |
| 21 | `good` | `data == 7` (goodB2G index coincides with goodG2B's fixed index) | fixed | [x] |
| 22 | `good` | `data == 9` (last in-bounds) | fixed | [x] |
| 23 | `good` | `data` in `0..=9`, randomized (both dumps emitted, 20 lines) | 300 seeded random in-range | [x] |
| 24 | `good` | `data == 10` (rejected by upper guard) | fixed | [x] |
| 25 | `good` | `data >= 10` randomized incl. `INT_MAX` (rejected) | 300 seeded random | [x] |
| 26 | `good` | `data < 0` randomized incl. `INT_MIN` (rejected) | 300 seeded random | [x] |
| 27 | `driver` | both in range: `goodData ∈ 0..=9`, `badData ∈ 0..=9` | 200 seeded random pairs (+ full 10×10 grid) | [x] |
| 28 | `driver` | `goodData` in range, `badData` negative | 100 seeded random pairs | [x] |
| 29 | `driver` | `goodData` in range, `badData` in `10..=11` (overflow that stays inside `bad`'s own frame, reached through the composed pipeline) | 103 pairs, isolated | [x] |
| 29b | `driver` | `goodData` in range, `badData >= 12` | `tests/phase_b_ub.rs` `ub_03`, prefix-compared, `12..=200` | [x] |
| 30 | `driver` | `goodData` out of range (`<0`), `badData` in range | 100 seeded random pairs | [x] |
| 31 | `driver` | `goodData` out of range (`>=10`), `badData` in range | 100 seeded random pairs | [x] |
| 32 | `driver` | both out of range (all 4 sign/magnitude quadrants; `badData` restricted to its comparable domain `INT_MIN..=11`) | 200 seeded random pairs, isolated | [x] |
| 33 | `driver` | boundary pairs: `{-1,0,9,10}` × `{-1,0,9,10}` | full 4×4 grid | [x] |
| 34 | all 5 entry points | **sequenced in one process** — repeated interleaved calls, to prove statelessness / no hidden global is mutated between calls | 400 seeded random ops | [x] |

## Result

All rows pass. Test files:

| rows | file | tests |
|------|------|-------|
| 1–34 | `tests/phase_b_configs.rs` | 35 |
| 19b, 29b | `tests/phase_b_ub.rs` | 4 |

Every row is compared for **byte-identical stdout and identical exit status**,
except rows 19b / 29b, where the C's behaviour is caller-frame corruption; those
compare printed output only. See `UB.md` for the derivation and measurements.

## Feature combinations

`Cargo.toml` has no `[features]` table → the only combination is the default
build (asserted by `tests/phase_d_symbols.rs::parity_05_manifest_declares_no_features`,
so the claim cannot silently rot). Both cargo profiles are nevertheless run,
because `release` sets `panic = "abort"` and full optimisation, which changes the
Rust `.so`'s own frame layout:

```
$ ./verify.sh          # dev + release x every feature combination
```

See `SYMBOLS.md`.
