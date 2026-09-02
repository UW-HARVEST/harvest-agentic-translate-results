# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## How this table was derived

From the complete public surface, mechanically:

* `c_src/include/driver.h` declares exactly **one** public entry point:
  `void driver(int x);` — and `nm -D` on the C `.so` confirms `driver` is the
  only exported symbol. There is no convenience wrapper vs. low-level split:
  `driver` *is* the lowest-level public entry point, so exercising it directly
  satisfies the "not only the convenience wrappers" requirement.
* Runtime options / modes / flags: **none.** `grep -nE 'if *\(|switch|#if'`
  over `src/driver.c` returns no matches, so the C takes no data-dependent or
  configuration-dependent branch. There is no setter, no context/handle, no
  global state, no environment lookup.
* Compile-time configuration: `CMakeLists.txt` defines no options and no
  `-D` macros (only `-fno-strict-aliasing`). `translation/Cargo.toml` declares
  **no `[features]` section**, so the only feature combination that exists is
  the default one (verified in Phase D).
* Input shapes the code distinguishes: the only input is a by-value `int`. The
  code has no size, count, width, element-type, format, or byte-order
  parameter — the struct shape (`int floors; int bedrooms; double bathrooms;`,
  16 bytes, offsets 0/4/8, no padding on LP64) and the dump length
  (`sizeof(house_t)`) are fixed at compile time.

Therefore the configuration axes reduce to a single axis: **the bit pattern of
`floors`**, plus the process-observable axis **repeated / interleaved
invocation** (the C uses buffered `stdio`, so call sequencing is a real shape
the implementations must agree on). The rows below are the pruned cross-product
of the value classes the byte-dump makes observable — sign, zero, all-ones,
byte-boundary values, values whose little-endian encoding contains `0x00`
bytes, and the extremes — plus sequencing.

Every row is checked with **many randomized inputs from that class** (fixed
seed `0x5EED_1234`, SplitMix64), not a single hand-picked value, and asserts the
captured `stdout` bytes from the C `.so` and the Rust `.so` are identical.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | no options (none exist); `floors == 0` — the all-zero bit pattern, the "empty" shape | [x] |
| 2 | `driver` | `floors == 1` — the "one" shape / minimal positive | [x] |
| 3 | `driver` | small positives, randomized in `1..=255` (encoding fits in the low byte; upper three bytes are `00`) | [x] |
| 4 | `driver` | small negatives, randomized in `-255..=-1` (sign-extension → `ff` bytes in the upper three bytes) | [x] |
| 5 | `driver` | randomized full-range positives `0..=INT_MAX` — the "many"/arbitrary-value shape | [x] |
| 6 | `driver` | randomized full-range negatives `INT_MIN..=-1` (two's-complement encoding) | [x] |
| 7 | `driver` | randomized arbitrary 32-bit bit patterns reinterpreted as `int` (uniform over all 2^32 encodings, including ones with embedded `0x00` and `0xff` bytes) | [x] |
| 8 | `driver` | byte/word boundary values: `0xff`, `0x100`, `0x7f`, `0x80`, `0xffff`, `0x10000`, `0x7fff`, `0x8000`, `0xffffff`, `0x1000000`, `0x7fffffff`, `-0x80000000`, `-1`, `-256`, `-257`, `-65536`, `-65537` (each ± 1 neighbour) | [x] |
| 9 | `driver` | powers of two `1 << k` for `k = 0..=30`, and their negations `-(1 << k)` — exercises every single-bit position in the dumped `floors` field | [x] |
| 10 | `driver` | repeated invocation: the same value called N times in a row (buffered-`stdio` shape — output must be N identical lines, one `\n` each, no dropped or coalesced flush) | [x] |
| 11 | `driver` | sequenced invocation: a randomized *sequence* of many different values through a single `.so` handle, captured as one byte stream (catches any residual/leaked state between calls — the C has none, so the Rust must have none either) | [x] |
| 12 | `driver` | interleaved invocation: alternating C-call / Rust-call into the *same* redirected `stdout`, asserting each pair of lines matches (catches stdio-buffer interaction differences between the two `.so`s) | [x] |
| 13 | `driver` | value passed as a widened 64-bit register (`extern "C" fn(i64)` view of the same symbol) — the ABI shape a caller with a mismatched prototype produces; both `.so`s must observe only the low 32 bits | [x] |
