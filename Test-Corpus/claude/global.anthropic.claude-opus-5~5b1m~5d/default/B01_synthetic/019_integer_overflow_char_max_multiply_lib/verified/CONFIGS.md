# CONFIGS.md — Configuration surface table (valid inputs)

Mechanically derived from `c_src/include/driver.h` (the public header) and every
`if` / branch in `c_src/src/driver.c`. There are **no** `#ifdef`s, no global
state, no init/teardown, and no runtime option struct in this library.

## Axes the C code actually branches on

| axis | where | distinct values the C distinguishes |
|------|-------|--------------------------------------|
| A. entry point | `nm -D` / header | `driver` (header-declared wrapper), `good`, `bad`, `printHexCharLine`, `printLine` (all 5 are public `.so` exports; `driver` is the only one in `driver.h`, so the 4 lower-level ones must be driven **directly** too) |
| B. `useGood` truthiness | driver.c:91 `if (useGood)` | `0` vs. any non-zero `int` (full `int` range, incl. negative, `INT_MIN`, `INT_MAX`, low-byte-zero values) |
| C. `data > 0` positivity guard | driver.c:47, :58, :70 | taken (all three call sites: `data` is `127`, `2`, `127` — always > 0) |
| D. `data < CHAR_MAX/2` range check | driver.c:72 | not taken (`127 < 63` is false) → diagnostic branch |
| E. `line != NULL` | driver.c:32 | NULL vs. non-NULL |
| F. `char` sign of `printHexCharLine` arg | driver.c:40 `%02x` after integer promotion | `0`, `1..15` (needs zero-pad), `16..127` (2 digits), `-128..-1` (sign-extends to 8 digits) — i.e. all 256 bit patterns |
| G. `printLine` payload shape | driver.c:34 `%s` | empty, 1 byte, many bytes, long (>4 KiB), ASCII, high bytes `0x80..0xFF` (non-UTF-8), `printf` conversion specifiers as data, interior NUL |
| H. call sequencing / stdout interleaving | shared `stdout` FILE | single call vs. many calls in sequence vs. mixed entry points (ordering & buffering must match) |

## Rows (pruned cross-product of the axes)

Every row is exercised against **both** `.so`s via `libloading`, with stdout
captured on fd 1 and compared byte-for-byte. Rows marked *randomized* use many
inputs from a fixed-seed PRNG (seed `0x5EED_1234_ABCD_0001`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printHexCharLine` | axis F exhaustive: **all 256** `char` bit patterns `-128..=127`, one call each | [x] |
| 2 | `printHexCharLine` | axis F, `0` only (zero boundary) | [x] |
| 3 | `printHexCharLine` | axis F, `1..=15` (zero-pad band) | [x] |
| 4 | `printHexCharLine` | axis F, `16..=127` (plain 2-digit band) | [x] |
| 5 | `printHexCharLine` | axis F, `-128..=-1` (sign-extension / 8-digit band) | [x] |
| 6 | `printHexCharLine` | axis F + H: *randomized* long sequences (256 random values per batch, 64 batches) in one capture — checks buffering/ordering, not just single values | [x] |
| 7 | `printLine` | axis E non-NULL + axis G empty string `""` | [x] |
| 8 | `printLine` | axis G single byte, *randomized* over all non-NUL bytes `0x01..=0xFF` | [x] |
| 9 | `printLine` | axis G *randomized* printable-ASCII strings, lengths 0..=64 | [x] |
| 10 | `printLine` | axis G *randomized* arbitrary non-NUL byte strings (incl. `0x80..=0xFF`, non-UTF-8), lengths 1..=128 | [x] |
| 11 | `printLine` | axis G long payload: 4 KiB and 64 KiB strings (past stdio buffer size) | [x] |
| 12 | `printLine` | axis G payload containing `%s %d %n %% %p` as **data** | [x] |
| 13 | `printLine` | axis G payload with interior NUL — printing must stop at first NUL | [x] |
| 14 | `printLine` | axis G + H: *randomized* sequence of 100 mixed-shape strings in one capture | [x] |
| 15 | `bad` | no inputs; the whole `data=CHAR_MAX; data>0; result=data*2` truncation path (axis C taken) | [x] |
| 16 | `bad` | axis H: called 50 times in one capture (idempotence + buffering) | [x] |
| 17 | `good` | no inputs; composes `goodG2B` (axis C taken) then `goodB2G` (axis C taken, axis D **not** taken → diagnostic) — verifies the two-line ordered output | [x] |
| 18 | `good` | axis H: called 50 times in one capture | [x] |
| 19 | `driver` | axis B `useGood == 0` → `bad()` | [x] |
| 20 | `driver` | axis B `useGood == 1` → `good()` | [x] |
| 21 | `driver` | axis B non-zero edge values: `-1`, `2`, `-2`, `INT_MAX`, `INT_MIN`, `0x100`, `0x10000`, `0x7FFF_FF00` (low byte zero → narrowing-bug probes) | [x] |
| 22 | `driver` | axis B *randomized*: 4096 random `i32` values, mixed zero/non-zero, each compared | [x] |
| 23 | `driver` | axis B + H: *randomized* interleaved sequence of `driver(rand)` calls in one capture | [x] |
| 24 | all 5 exports | axis A + H: *randomized* interleaving of `driver`/`good`/`bad`/`printHexCharLine`/`printLine` in one capture — the composed-pipeline check that per-function tests cannot see | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set. Enumerated and confirmed by
`scripts/check_features.sh`:

```
feature combinations found: 1  ->  <default>  (== --no-default-features)
```

Both `cargo test` and `cargo test --no-default-features` are run over the full
suite; they are the same configuration and both must pass.
