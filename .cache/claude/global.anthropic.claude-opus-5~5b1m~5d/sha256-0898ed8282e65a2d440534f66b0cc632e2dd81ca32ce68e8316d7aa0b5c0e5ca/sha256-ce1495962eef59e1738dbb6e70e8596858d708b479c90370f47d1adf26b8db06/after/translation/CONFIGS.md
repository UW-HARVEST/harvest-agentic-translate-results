# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/include/driver.h`, `c_src/src/driver.c` and the
gcc codegen of the two exported functions.

## Axes the C code actually distinguishes

The library has **no runtime options, modes, flags, globals or `#ifdef`
branches** (see `ERRORS.md` grep evidence: the source has zero `if`/`switch`/
`#if` other than the header guard). The behaviour is therefore driven purely by
**input shape**, along these axes:

* **Entry point** (2, both exported; the low-level one is `print_foo`, which
  takes the raw `foo_t*` — it is *not* only reachable through the `driver`
  convenience wrapper and is tested directly):
  * `driver(unsigned int x, unsigned int y, bool b, int z)` — builds a `foo_t`
    on the stack from four scalars, then calls `print_foo`.
  * `print_foo(const foo_t *foo)` — reads the *bit-field storage byte* at
    offset 0 and the `int` at offset 4 and `printf`s them.
* **`foo_t` memory layout** (SysV x86-64, verified against gcc's codegen):
  `x` = byte0 bits 0..1, `y` = byte0 bits 2..4, `b` = byte0 bit 5,
  byte0 bits 6..7 + bytes 1..3 = padding, `z` = bytes 4..7 (LE),
  `sizeof == 8`, `alignof == 4`.
* **`x` shape**: in-range (0..3) / out-of-range (4, 5, 255, 0x1_0000, UINT_MAX)
  — the 2-bit field truncates.
* **`y` shape**: in-range (0..7) / out-of-range (8, 9, 255, UINT_MAX)
  — the 3-bit field truncates.
* **`b` shape**: canonical `_Bool` bytes 0 / 1 / non-canonical 2..255
  (only bit 0 survives).
* **`z` shape**: 0, +1, −1, small +, small −, `INT_MAX`, `INT_MIN`, values with
  all byte lanes distinct, random 32-bit patterns (sign handling of `%d`).
* **`print_foo` buffer shape**: all 256 possible bit-field bytes × padding bytes
  zero / 0xFF / random, pointer aligned / misaligned by 1,2,3, buffer read from
  the heap / stack / static memory.
* **Output/stdout state**: the printed line is compared byte-for-byte after
  `fflush`; repeated calls in one process are also compared as a *sequence* so
  that stdio buffering and any hidden per-call state would show up.

## Table — one row per combination the C treats differently

| #  | entry point(s)         | configuration (options set + input shape)                                                                        | [x] |
|----|------------------------|-------------------------------------------------------------------------------------------------------------------|-----|
| 1  | `driver`               | exhaustive small grid: every `x` ∈ 0..=7 × every `y` ∈ 0..=15 × `b` ∈ {0,1} × `z` ∈ {0,1,−1} (in-range *and* first out-of-range values) | [x] |
| 2  | `driver`               | random `x`,`y` over the full `u32` range, `b` ∈ {0,1}, `z` = 0 (isolates field truncation, seeded RNG, 4096 cases) | [x] |
| 3  | `driver`               | `x`,`y` in range, `b` ∈ {0,1}, random full-range `i32` `z` (seeded RNG, 4096 cases)                                | [x] |
| 4  | `driver`               | fully random 4-tuple `(u32, u32, u8 0..=255, i32)` (seeded RNG, 20000 cases) — cross-product smoke                 | [x] |
| 5  | `driver`               | boundary `z` values {`INT_MIN`, `INT_MIN+1`, −1, 0, 1, `INT_MAX-1`, `INT_MAX`} × `x`,`y` ∈ {0, max in-range, max+1, `UINT_MAX`} × `b` ∈ {0,1} | [x] |
| 6  | `driver`               | `b` byte = every value 0..=255 (non-canonical `_Bool`s) with fixed `x=1,y=5,z=-7`                                  | [x] |
| 7  | `driver`               | `b` argument register carrying a wide value (`0x100`, `0xFF00`, `0xFFFFFF01`) — only low byte is ABI-significant   | [x] |
| 8  | `print_foo`            | exhaustive: bit-field byte = every value 0..=255, padding bytes 1..3 = 0, `z` = 0                                  | [x] |
| 9  | `print_foo`            | exhaustive bit-field byte 0..=255 with padding bytes 1..3 = 0xFF and `z` = `INT_MIN` (padding must be ignored)     | [x] |
| 10 | `print_foo`            | fully random 8-byte buffers (seeded RNG, 20000 cases), heap-allocated, 4-byte aligned                              | [x] |
| 11 | `print_foo`            | random buffers at pointers misaligned by 1, 2 and 3 bytes                                                         | [x] |
| 12 | `print_foo`            | buffer in static/`'static` memory and on the caller's stack (same content ⇒ same output)                           | [x] |
| 13 | `print_foo`            | `z` bytes 4..7 = each of {0x00.., 0xFF.., 0x80000000, 0x7FFFFFFF, distinct-lane 0x01020304}                        | [x] |
| 14 | `driver` + `print_foo` | cross-check: `driver(x,y,b,z)` output == `print_foo` output on the byte image `[(x&3)|((y&7)<<2)|((b&1)<<5), pad, pad, pad, z_le…]` for random inputs — verifies the composed pipeline, not just each wrapper | [x] |
| 15 | both, interleaved      | long randomized *sequence* of mixed `driver`/`print_foo` calls in one process; whole multi-line stdout image compared (catches hidden state / buffering differences) | [x] |
| 16 | both                   | many repeated calls with identical arguments (idempotence: no hidden accumulating state)                           | [x] |

All 16 rows are exercised in `tests/valid_paths.rs` with a fixed-seed
(deterministic) xorshift/SplitMix RNG, comparing the *exact bytes* written to
stdout by the C `.so` and the Rust `.so`.
