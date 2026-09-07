# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source, the same way `ERRORS.md` is derived.

## Axes the C code actually branches on

### 1. Runtime options / modes / flags

**None.** Greps over all C sources and headers for `#if`/`#ifdef`
(only the `DRIVER_H_` include guard), `if`, `switch`, ternary, global/`static`
mutable state, setter functions, and environment reads all come back empty.
`driver` has no configuration knob: it is a pure function of its single `int`
argument (plus the process-wide `stdout` stream it writes to).

### 2. Full set of public entry points

`c_src/include/driver.h` declares exactly one:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(int x)` | the only one — simultaneously the lowest-level and the top-level API |

`print_hex` is `static` (file-local), is not declared in the header and is not
in the dynamic symbol table (see `SYMBOLS.md`), so it is **not** reachable
across the FFI boundary. It is exercised transitively, on every row below, as
the composed pipeline `driver → memcpy → print_hex → printf("%02x") ×4 →
printf("\n")`.

### 3. Distinct input shapes the code special-cases

`sizeof(int) == 4` on the target, so the byte count is fixed at 4 and the
`i < len` loop always runs exactly 4 iterations. The remaining
*value-dependent* distinctions in the pipeline are:

- **`%02x` zero-padding**: a byte `< 0x10` prints one significant hex digit and
  must be left-padded with `'0'`; a byte `>= 0x10` prints two.
- **`unsigned char` → `int` integer promotion**: a byte `>= 0x80` has its high
  bit set. If the translation used a signed byte type it would sign-extend and
  `%02x` would emit `ffffffXX` instead of `XX`. This is the classic divergence
  and must be probed at every one of the 4 byte offsets.
- **Byte order**: `memcpy(raw, &x, sizeof x)` copies the *native*
  representation, so the printed byte order is the platform's (little-endian
  here). A translation using `to_be_bytes` would pass on palindromic inputs and
  fail on asymmetric ones.
- **Sign of `x` / extreme magnitudes**: `INT_MIN`/`INT_MAX` and negative values
  exercise the top byte's high bit.
- **Call sequencing**: `print_hex` terminates each record with `"\n"`, so a
  sequence of calls must produce exactly one line per call, in order, with no
  leaked state between calls.

## Configuration table

One row per meaningful combination of the axes above (options × input shape).
Every row is driven through **both** `.so` exports via `libloading`, with
stdout captured at the file-descriptor level and compared byte-for-byte.
Randomized rows use a fixed-seed SplitMix64 PRNG for reproducibility.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | no options (none exist) + `x == 0` — all four bytes `0x00`, exercises `%02x` zero-padding at all 4 offsets simultaneously | [x] |
| 2 | `driver` | `x == -1` (`0xffffffff`) — all four bytes `>= 0x80`, exercises the integer-promotion / sign-extension path at all 4 offsets simultaneously | [x] |
| 3 | `driver` | `x == INT_MAX` (`0x7fffffff`) — top byte `0x7f`, low bytes `0xff`; byte-order-asymmetric | [x] |
| 4 | `driver` | `x == INT_MIN` (`0x80000000`) — top byte exactly `0x80`, low bytes `0x00`; the high-bit boundary | [x] |
| 5 | `driver` | boundary neighbours one step past / before the extremes: `INT_MAX-1`, `INT_MIN+1`, `-2`, `1`, and the `0x7fffffff↔0x80000000` wrap pair | [x] |
| 6 | `driver` | single low-nibble byte at each offset: `0x0000000N`, `0x00000N00`, `0x000N0000`, `0x0N000000` for every `N` in `1..=0xf` — padding × byte-position cross-product (60 inputs) | [x] |
| 7 | `driver` | single high-bit byte at each offset: `0x000000HH`, `0x0000HH00`, `0x00HH0000`, `0xHH000000` for every `HH` in `0x80..=0xff` — sign-extension × byte-position cross-product, each against an all-`0x00` and an all-`0xff` background (1024 inputs) | [x] |
| 8 | `driver` | exhaustive sweep of one byte position at a time: each of the 4 offsets × all 256 byte values, other bytes held at `0x00` and again at `0xff` (2048 inputs), **plus** an exhaustive sweep of every value of the low 16-bit half-word and of the high 16-bit half-word (2 x 65 536 inputs) | [x] |
| 9 | `driver` | byte-order-sensitive asymmetric patterns: `0x000000ff`, `0xff000000`, `0x0000ff00`, `0x00ff0000`, `0x12345678`, `0x78563412`, `0xdeadbeef`, `0xefbeadde` | [x] |
| 10 | `driver` | mixed nibble patterns where every byte differs in both nibbles and in padding class: `0x0f1e2d3c`, `0xa0b1c2d3`, `0x01f0e0d0`, … | [x] |
| 11 | `driver` | randomized, uniform over the **full** `i32` domain — 200 000 inputs, fixed seed | [x] |
| 12 | `driver` | randomized, every byte constrained to `0x00..=0x0f` — stresses padding-only outputs, 2 000 inputs, fixed seed | [x] |
| 13 | `driver` | randomized, every byte constrained to `0x80..=0xff` — stresses promotion-only outputs, 2 000 inputs, fixed seed | [x] |
| 14 | `driver` | randomized, bytes drawn from the padding/promotion boundary set `{0x00,0x0f,0x10,0x7f,0x80,0xff}` — 2 000 inputs, fixed seed | [x] |
| 15 | `driver` | **sequenced pipeline**: one long run of 5 000 randomized calls into a single captured stream, comparing the whole multi-line transcript — verifies one `"\n"`-terminated line per call, correct ordering, and no state leaking between calls | [x] |
| 16 | `driver` | interleaved C/Rust calls into the **same** stdout stream (C, Rust, C, Rust, …) — verifies the Rust `.so` shares libc's `stdout` buffering and does not reorder or duplicate output relative to C | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`.
**The project builds no driver binary**, so the "compare C and Rust stdout of
the binaries" obligation is vacuous. (Stdout *is* nevertheless compared
byte-for-byte on every row above, since stdout is this library's only output.)

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, hence exactly one
configuration: the default (`--no-default-features` is equivalent). Verified by
the loop in `run_all.sh`.
