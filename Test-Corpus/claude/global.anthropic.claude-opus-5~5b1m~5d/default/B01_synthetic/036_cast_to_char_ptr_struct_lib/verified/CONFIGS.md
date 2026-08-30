# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/include/driver.h` (the full public API) and
`c_src/src/driver.c` (the branches the code actually takes).

## Axes the C code actually distinguishes

**Public entry points (complete set, from the header):**

| entry point | signature | exported? |
|---|---|---|
| `driver` | `void driver(int x)` | YES — the only public symbol |
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | NO (`static`, file-local; absent from `nm -D` on both `.so`s) |

There are no other entry points — no init/teardown, no "convenience wrapper vs
low-level" split. `driver` *is* the lowest-level exported entry point, and it is
tested directly below.

**Runtime options / modes / flags:** NONE. The library has no configuration
struct, no setters, no globals, no environment lookups, and no `#ifdef` in either
file. `grep -c 'ifdef\|ifndef\|#if' src/driver.c` finds only the header's include
guard. So there is no option cross-product to enumerate.

**State:** NONE. `driver` is a pure function of its `int` argument plus two
hard-coded constants (`bedrooms = 3`, `bathrooms = 2.`); `house` is a fresh stack
local each call. Therefore call ORDER and call COUNT are additional axes worth
exercising (to prove there is no hidden static state and that the shared `stdout`
FILE stream is used identically).

**Input shapes the code is sensitive to:** the only input is one `int`. It lands
in `house.floors` at offset 0 and is emitted as 4 little-endian bytes by the
`%02x` loop. The shapes that matter are therefore *bit patterns* of that `int`:

- sign (bit 31) — exercises signed→unsigned reinterpretation
- per-byte high bit (`0x80` in any byte) — exercises the `unsigned char` vs
  `signed char` promotion in `printf("%02x", p[i])`; a `signed char` bug prints
  `ffffff80` instead of `80`
- bytes `< 0x10` — exercises the `%02x` zero-padding
- byte value `0x00` in interior positions
- extremes: `INT_MIN`, `INT_MAX`, `0`, `-1`
- value colliding with the `bedrooms` constant (`3`)

**Fixed output shape (constant across all inputs, still asserted):** bytes 4..7 =
`bedrooms` = 3 → `03000000`; bytes 8..15 = `bathrooms` = 2.0 as IEEE-754
little-endian double → `0000000000000040`; total length `sizeof(house_t)` = 16 →
32 hex chars + `\n` = 33 bytes. `house_t house = {0}` zero-initialises the whole
object *including padding*; on the LP64 target the struct has NO interior or
trailing padding (4 + 4 + 8 = 16 == sizeof), so there are no
indeterminate padding bytes — but the tests verify the length and every byte
anyway, which would catch a layout/padding divergence.

## Configuration table

One row per meaningful combination of (entry point × input shape × call
pattern) that the C treats distinctly. Every row is driven through BOTH `.so`s
via `libloading` and compared byte-for-byte on captured `stdout`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | no options (none exist); `x = 0` — all-zero bit pattern | [x] |
| C2 | `driver` | `x = 1` — minimal positive, single low bit | [x] |
| C3 | `driver` | `x = 3` — collides with the hard-coded `bedrooms` constant | [x] |
| C4 | `driver` | `x = 2` — collides with the integral value of the `bathrooms` constant | [x] |
| C5 | `driver` | `x` in `1..=255` — exhaustive single-byte range (byte 0 varies, bytes 1-3 zero); covers `%02x` zero-padding for every low nibble/high nibble | [x] |
| C6 | `driver` | `x` in `256..=65535`, randomized — two significant bytes | [x] |
| C7 | `driver` | `x` in `0x10000..=0xFFFFFF`, randomized — three significant bytes | [x] |
| C8 | `driver` | `x = INT_MAX` (`0x7FFFFFFF`) — max positive, four significant bytes | [x] |
| C9 | `driver` | `x = INT_MIN` (`0x80000000`) — sign bit only; high-bit byte in position 3 | [x] |
| C10 | `driver` | `x = -1` (`0xFFFFFFFF`) — all bits set; high bit in every byte | [x] |
| C11 | `driver` | `x` = each single-bit value `1 << k` for `k` in `0..=31` — walking-ones, isolates every bit position incl. the sign bit | [x] |
| C12 | `driver` | `x` = each `!(1 << k)` for `k` in `0..=31` — walking-zeros complement | [x] |
| C13 | `driver` | `x` = each byte-aligned high-bit pattern: `0x80`, `0x8000`, `0x800000`, `0x80000000`, `0x80808080`, `0xFF00FF00`, `0x00FF00FF` (as `i32`) — the `unsigned char` promotion axis (`signed char` bug ⇒ `ffffff80`) | [x] |
| C14 | `driver` | `x` = byte patterns with every byte `< 0x10`: `0x01020304`, `0x0F0F0F0F`, `0x00010203`, `0x0A0B0C0D` — `%02x` padding axis | [x] |
| C15 | `driver` | `x` = interior-zero patterns: `0xFF0000FF`, `0x00FFFF00`, `0xFF00FF00`, `0x000000FF`, `0xFF000000` | [x] |
| C16 | `driver` | `x` = negative non-extreme values, randomized in `INT_MIN..0` | [x] |
| C17 | `driver` | `x` = uniform random `i32` over the full 32-bit range, 4096 samples, fixed seed | [x] |
| C18 | `driver` | `x` = full exhaustive sweep of the 256 possible values of each individual byte position (`b`, `b<<8`, `b<<16`, `b<<24` for all `b` in `0..=255`) — proves per-byte hex formatting at every offset | [x] |
| C19 | `driver` | call COUNT axis: single call in isolation (statelessness baseline) | [x] |
| C20 | `driver` | call COUNT axis: many sequential calls in one capture window, same value repeated 64× — proves no hidden static state accumulates and the `stdout` FILE stream is reused identically | [x] |
| C21 | `driver` | call COUNT/ORDER axis: many sequential calls in one capture window with *distinct* values — proves output ordering and per-call independence | [x] |
| C22 | `driver` | interleaving axis: C and Rust calls alternated within a SINGLE capture window on the SHARED libc `stdout` FILE — proves both `.so`s use the same stream with the same buffering, so line interleaving is byte-identical | [x] |
| C23 | `driver` (structural) | output-shape invariants held across ALL of the above: exactly 33 bytes per call, 32 lowercase hex digits + `\n`, bytes 4..7 == `03000000`, bytes 8..15 == `0000000000000040` (IEEE-754 2.0 LE), and `sizeof(house_t) == 16` with no padding divergence | [x] |
| C24 | `print_hex` | not reachable: `static`, absent from `nm -D` on both `.so`s — asserted structurally (symbol lookup must FAIL on both) | [x] |

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so the complete feature space is the
single default (empty) configuration. `--no-default-features` builds identical
code. All rows above therefore hold under every feature combination that exists;
verified mechanically by `check_features.sh`.
