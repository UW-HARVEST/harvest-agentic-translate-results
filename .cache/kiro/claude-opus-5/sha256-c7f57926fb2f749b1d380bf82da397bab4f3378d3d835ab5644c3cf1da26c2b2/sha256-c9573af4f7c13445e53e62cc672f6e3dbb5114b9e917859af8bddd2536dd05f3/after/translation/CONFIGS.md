# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from what the C code branches on.

## Axes the C code actually distinguishes

Enumerated from `c_src/src/driver.c` + `c_src/include/driver.h`:

| axis | values the C code distinguishes | evidence |
|------|--------------------------------|----------|
| runtime option / mode / flag | **none** | 0 `if`/`switch`/`#ifdef` on any flag; no setter, no context struct, no global |
| public entry points | **one**: `driver` | `driver.h` declares exactly one function; `nm -D` confirms one `T` symbol |
| lowest-level entry point | `print_hex` is `static` → NOT public | absent from `nm -D` on the C `.so`; only reachable through `driver` |
| parameter count / width | one `int` (32-bit on this ABI) | `void driver(int x)` |
| length / count | fixed `sizeof(x)` == 4, not caller-controllable | `print_hex(..., sizeof(x))` |
| byte order | host order — the raw object representation is walked byte 0..3 | `(unsigned char *)&x` then `p[i]` ascending |
| per-byte formatting | `%02x`: lowercase, zero-padded to 2 | the only format string |
| value-dependent shape | which bytes are `0x00`, which are `< 0x10` (need the zero pad), which have high bit set, sign of `x` | `%02x` on each byte independently |

So the cross-product collapses to **one entry point × the value-shape axis**.
Rows below are the value shapes the byte-wise formatting actually distinguishes,
pruned to distinct cases. Every row is driven with **many randomized inputs**
(fixed seed, deterministic SplitMix64) plus the hand-picked boundary, not a
single scalar.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | no options (none exist); `x = 0` — all four bytes `0x00`, exercises the zero-pad path on every byte | [x] |
| C2 | `driver` | `x` = each single-byte-set value `1 << k` for all k in 0..32 — walks a set bit through every byte position and every bit within a byte | [x] |
| C3 | `driver` | `x` = each single-byte-clear value `!(1 << k)` for all k in 0..32 — complement of C2 | [x] |
| C4 | `driver` | `x` in `[0x00, 0xFF]` placed in byte 0 only (`x = b`) — every possible byte value through the `%02x` formatter, incl. all 16 values `< 0x10` that require the leading zero | [x] |
| C5 | `driver` | `x = b << 8`, `b << 16`, `b << 24` for all b in `[0, 0xFF]` — every byte value in every byte *position*, so a wrong index/stride shows up | [x] |
| C6 | `driver` | positive `x`: 4096 randomized values in `[0, INT_MAX]`, seeded | [x] |
| C7 | `driver` | negative `x`: 4096 randomized values in `[INT_MIN, -1]`, seeded — sign bit set, two's-complement object repr | [x] |
| C8 | `driver` | full-range `x`: 8192 randomized values over all 2^32 bit patterns, seeded — the unconstrained property test | [x] |
| C9 | `driver` | boundary values: `0, 1, -1, 2, -2, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1, 0x7fffffff, 0x80000000, 0xffffffff, 0x0000ffff, 0xffff0000, 0x00ff00ff, 0xff00ff00, 0x0f0f0f0f, 0xf0f0f0f0, 0x01020304, 0x04030201, 0x10000000, 0x0000000f` | [x] |
| C10 | `driver` | byte-order-sensitive pairs: `x` and `bswap32(x)` for randomized `x` — asserts C and Rust agree on the *same* traversal direction, so an accidental big-endian/reversed loop in Rust is caught | [x] |
| C11 | `driver` | "empty / one / many" call-count shape: 0 calls, 1 call, then 256 consecutive calls without re-loading the `.so` — checks the accumulated stdout stream and that no hidden state leaks between calls | [x] |
| C12 | `driver` | interleaved order: alternate C-call / Rust-call within one captured stream, and Rust-first vs C-first, verifying no dependence on which `.so`'s stdout buffer was touched first | [x] |
| C13 | `driver` | both `.so`s loaded simultaneously in one process and driven through the same captured `fd 1` (the real consumer scenario for the `#[no_mangle]` wrapper) — every other row runs under this condition | [x] |

## Completion gate item

- [x] EVERY row passes across randomized inputs.

## Suite sensitivity (negative controls)

To confirm these rows are not vacuously passing, five semantic mutations were
applied to `src/lib.rs`, each built and run through the full suite, then
reverted. Every mutant was caught:

| mutation | cases that detect it |
|----------|----------------------|
| `%02x` → `%2x` (drop the zero pad) | 20 / 25 |
| reverse the byte traversal (`p[i]` → `p[len-1-i]`) | 19 / 25 |
| `%02x` → `%02X` (uppercase hex) | 17 / 25 |
| length `sizeof(int)` → `3` (truncate) | 22 / 25 |
| remove the trailing `printf("\n")` | 22 / 25 |
| sign-extend the byte (`as i8 as c_int`) | 20 / 25 |

## How to run

```sh
cd translation
cargo build --release
cargo test                # 25 cases, default configuration
./verify_all.sh           # every feature combo x debug/release + symbol parity
```
