# CONFIGS.md — Phase B configuration-surface table

## Axes, derived from what the C actually branches on

Enumerated from `c_src/src/driver.c` (14 non-comment lines) and
`c_src/include/driver.h`, not from guesses:

* **Runtime options / modes / flags:** NONE. `driver(int)` takes no flag, no
  mode, no context/handle, no format selector; there is no setter, no global,
  no `#ifdef` (`grep -nE '#if|#ifdef|#ifndef' c_src/src/` → no matches), and no
  `switch`. The library is stateless apart from libc's `stdout` buffer.
* **Public entry points (FULL set, lowest level included):** exactly one —
  `driver`. `print_hex` is `static` (internal linkage, see `SYMBOLS.md`) so it is
  not a public entry point in either `.so`; it is reached only through `driver`,
  always with `p = raw` (non-null) and `len = sizeof(int) = 4`. There is no
  convenience-vs-low-level split to under-test here.
* **Input shape:** one by-value `int`. `sizeof(int) == 4` and byte order is the
  target's native (little-endian x86-64), fixed by `memcpy(raw, &x, sizeof(x))`.
  What the code genuinely *distinguishes* is the **per-byte value class** driving
  the `%02x` conversion, and the **positional arrangement** of those bytes:
  * `0x00` — zero, both hex digits from padding;
  * `0x01..0x0f` — one significant digit, so `%02x` must zero-pad;
  * `0x10..0x7f` — two digits, no padding;
  * `0x80..0xff` — high bit set; exercises the `(unsigned char *)` cast and the
    default-argument promotion to `int` (a signed `char` here would print
    `ffffff__`, 8 chars, instead of 2);
  * position 0..3 — distinguishes byte ordering / endianness.
* **Stream state (the only cross-call state):** number of calls in one captured
  stdout window, and whether C and Rust write into the *same* `FILE *stdout`
  buffer in one window.
* **Loaded artifact:** the Rust `.so` in `debug` vs `release` (release sets
  `panic = "abort"`), since that is a real difference in the shipped object.

Rows below are the pruned cross-product of those axes — the combinations the C
treats differently. Every row is asserted byte-for-byte against the C `.so`
through `libloading`, with many randomized inputs per row (fixed seed
`0x2545F4914F6CDD1D`, SplitMix64) wherever the row is not a single exact value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|------------------------------------------|------|-----|
| 1 | `driver` | all bytes `0x00` — `x = 0`; pure zero-pad path | `cfg_row01_all_zero_bytes` | [x] |
| 2 | `driver` | all bytes `0xff` — `x = -1`; high bit set in every position | `cfg_row02_all_high_bytes` | [x] |
| 3 | `driver` | `x = INT_MAX` (`0x7fffffff`) — positive extreme | `cfg_row03_int_max` | [x] |
| 4 | `driver` | `x = INT_MIN` (`0x80000000`) — negative extreme, sign byte only | `cfg_row04_int_min` | [x] |
| 5 | `driver` | exactly one byte non-zero, that byte in `0x01..0x0f` (zero-pad class), swept across all 4 positions × all 15 values | `cfg_row05_single_low_nibble_byte_each_position` | [x] |
| 6 | `driver` | exactly one byte non-zero, that byte in `0x10..0x7f` (two-digit class), swept across all 4 positions, randomized values | `cfg_row06_single_mid_byte_each_position` | [x] |
| 7 | `driver` | exactly one byte non-zero, that byte in `0x80..0xff` (high-bit class), swept across all 4 positions, randomized values | `cfg_row07_single_high_byte_each_position` | [x] |
| 8 | `driver` | all four byte classes present at once, in every one of the 24 position permutations (`0x00`, low-nibble, mid, high) — catches endianness / ordering | `cfg_row08_all_four_classes_every_permutation` | [x] |
| 9 | `driver` | every byte drawn from `0x01..0x0f` (all four positions need padding simultaneously), randomized | `cfg_row09_all_bytes_low_nibble` | [x] |
| 10 | `driver` | every byte drawn from `0x80..0xff` (high bit in all four positions), randomized | `cfg_row10_all_bytes_high` | [x] |
| 11 | `driver` | byte-boundary sweep: every value in `0x00..0xff` placed in byte 0 (covers the `0x0f/0x10` and `0x7f/0x80` class edges exhaustively) | `cfg_row11_exhaustive_byte0` | [x] |
| 12 | `driver` | single-bit values: `1 << k` for all `k` in `0..32` (`k = 31` is `INT_MIN`) | `cfg_row12_single_bit_set` | [x] |
| 13 | `driver` | `(1 << k) - 1` and `!((1 << k) - 1)` masks for all `k` in `0..32` — run-length boundaries | `cfg_row13_bit_masks` | [x] |
| 14 | `driver` | strictly negative `x`, uniformly randomized (2048 inputs) | `cfg_row14_random_negative` | [x] |
| 15 | `driver` | strictly positive `x`, uniformly randomized (2048 inputs) | `cfg_row15_random_positive` | [x] |
| 16 | `driver` | full 32-bit range, uniformly randomized over all bit patterns (8192 inputs) | `cfg_row16_random_full_range` | [x] |
| 17 | `driver` | small-magnitude neighbourhood: every `x` in `-1024..=1024` (sign-transition dense sweep) | `cfg_row17_dense_small_magnitude` | [x] |
| 18 | `driver` (→ static `print_hex`, `len = 4`) | output-shape invariant: for randomized `x`, output is exactly 9 bytes — 8 lowercase hex digits + one `\n` — and contains only `[0-9a-f\n]` | `cfg_row18_output_shape_invariant` | [x] |
| 19 | `driver` | stream state: 256 randomized calls into ONE captured stdout window (accumulation and ordering across calls, libc buffer parity) | `cfg_row19_many_calls_one_stream` | [x] |
| 20 | `driver` | stream state: C and Rust calls INTERLEAVED in one captured window (both must share the same `FILE *stdout` and flush identically) | `cfg_row20_interleaved_same_stream` | [x] |
| 21 | `driver` | loaded artifact = Rust `target/debug/libdriver.so` vs C, over the row-16 randomized corpus | `cfg_row21_debug_profile_artifact` | [x] |
| 22 | `driver` | loaded artifact = Rust `target/release/libdriver.so` (`panic = "abort"`) vs C, over the row-16 randomized corpus | `cfg_row22_release_profile_artifact` | [x] |

All 22 rows pass in `translation/tests/differential.rs` (module `phase_b`).
