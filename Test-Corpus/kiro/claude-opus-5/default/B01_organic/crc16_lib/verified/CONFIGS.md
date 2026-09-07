# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

## Mechanical derivation of the axes

### Public entry points (the FULL set, from `c_src/include/lib.h`)

There is exactly one — there are no convenience wrappers and no lower-level
functions hidden behind them, because the library is a single function:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

### Runtime options / modes / flags

**None.** `crc16` takes no option, mode, flag or context struct. Grepping the C
for `if (` / `switch` / `#if` / `#ifdef` yields **zero** matches, so there is no
compile-time or runtime configuration branch to enumerate.

### Cargo features

`translation/Cargo.toml` has **no `[features]` section**, so the complete set of
feature combinations is `{ default }` = `{ }`. `cargo test`,
`cargo test --no-default-features` and `cargo test --all-features` are the same
build. There is no second configuration to repeat Phases B–C under.

### Binary executable

`CMakeLists.txt` builds `add_library(... SHARED src/lib.c)` only — no
`add_executable`. `Cargo.toml` declares `crate-type = ["cdylib"]` with no
`[[bin]]`. **No driver binary exists on either side**, so the stdout-comparison
sub-task of Phase B is not applicable.

### Input shapes the C actually special-cases

Derived from the two loop conditions, which are the only branches:

- `while (len >= 8)` — bulk 8-bytes-at-a-time path, consuming `d[0..8]` through
  8 *different* tables (`tables[7]`…`tables[0]`, plus the `crc16 >> 8` /
  `crc16 & 0xFF` split feeding lanes 7 and 6).
- `while (len--)` — byte-at-a-time tail path, `tables[0]` only.

Axes:

| axis | distinct values the C treats differently |
|---|---|
| `len` vs. bulk threshold | `0`; `1..7` (tail only); `8` (exactly one bulk); `>8` (many bulk) |
| `len % 8` | `0` (no tail) vs. `1..7` (tail of that length) |
| seed `crc` | `0x0000`, `0xFFFF`, arbitrary — selects the lane-7/lane-6 table indices |
| data byte values | `0x00`, `0xFF`, arbitrary, full 0..255 coverage per lane |
| `d` pointer | non-null; null (only reachable with `len == 0`); unaligned offset |
| chaining | one-shot call vs. streaming: repeated calls feeding the previous return in as the next seed, split at every offset (this is how a real consumer drives a CRC, and it is the composed pipeline that per-call tests cannot see) |

## Configuration table

One row per meaningful combination of the axes above. Every row is exercised
with **many randomized inputs** (seeded `SplitMix64`, fixed seed → reproducible),
calling **both** `.so`s through `libloading` and comparing the returned `u16`
byte-for-byte. Rows are checked only after they pass across all their random
inputs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `crc16` | `len == 0`, non-null `d`, random seed — both loops skipped | [x] |
| 2 | `crc16` | `len == 0`, `d == NULL`, random seed — pointer must not be dereferenced | [x] |
| 3 | `crc16` | `len` in `1..=7` (tail-only path), random bytes, random seed — every sub-8 length | [x] |
| 4 | `crc16` | `len == 8` exactly (single bulk iteration, no tail), random bytes, random seed | [x] |
| 5 | `crc16` | `len` a multiple of 8 in `16..=512` (bulk only, no tail), random bytes, random seed | [x] |
| 6 | `crc16` | `len > 8` with `len % 8` in `1..=7` (bulk **then** tail), random bytes, random seed | [x] |
| 7 | `crc16` | `len` swept over **every** value `0..=64` (covers both thresholds and all 8 residues), random bytes, random seed | [x] |
| 8 | `crc16` | seed `crc == 0x0000` with random data at random lengths | [x] |
| 9 | `crc16` | seed `crc == 0xFFFF` (max; drives `crc>>8` and `crc&0xFF` to table index 255) with random data at random lengths | [x] |
| 10 | `crc16` | seed swept over all 65536 `u16` values, fixed short data — full seed-domain coverage | [x] |
| 11 | `crc16` | all data bytes `0x00` (table index 0 in all 8 lanes), lengths swept `0..=64`, random seed | [x] |
| 12 | `crc16` | all data bytes `0xFF` (table index 255 in all 8 lanes), lengths swept `0..=64`, random seed | [x] |
| 13 | `crc16` | data = `0x00..0xFF` counting pattern, length 256 and 2048 — every byte value in every bulk lane, i.e. every one of the 8×256 table slots read | [x] |
| 14 | `crc16` | unaligned `d` (buffer offset `1..=7`) with random bytes/length/seed — byte-wise reads must not assume alignment | [x] |
| 15 | `crc16` | large buffer, `len == 1 MiB`, random bytes, random seed — many bulk iterations | [x] |
| 16 | `crc16` | large buffer with `len == 1 MiB + r` for `r` in `1..=7` — many bulk iterations **plus** tail | [x] |
| 17 | `crc16` | **streaming/composed**: buffer split into random chunks, each call seeded with the previous return, compared against a single one-shot call over the whole buffer — on both libs, and C-vs-Rust | [x] |
| 18 | `crc16` | **streaming at every split point**: for a fixed buffer, split at each offset `0..=len`, chain the two calls, compare C vs Rust (catches divergence only visible when a bulk boundary is crossed mid-stream) | [x] |
| 19 | `crc16` | pure property fuzz: 20 000 random `(len ∈ 0..=1024, random bytes, random seed)` triples | [x] |
| 20 | `crc16` | feature combination coverage: the single existing configuration (no `[features]`), verified under `cargo test`, `--no-default-features`, and `--all-features` | [x] |
