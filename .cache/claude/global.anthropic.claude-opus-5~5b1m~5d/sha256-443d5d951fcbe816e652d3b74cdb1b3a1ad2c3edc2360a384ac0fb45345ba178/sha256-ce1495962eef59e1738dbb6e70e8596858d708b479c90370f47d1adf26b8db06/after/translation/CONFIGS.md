# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

Derived mechanically from the C source and header, not from assumptions.

## Axis enumeration

### Runtime options / modes / flags

Grepping `c_src/include/driver.h` and `c_src/src/driver.c` for anything a caller
can toggle: there are **no** setters, globals, environment reads, `#ifdef`s,
`switch`es, or `if`s in the library. `driver`'s control flow is straight-line;
the only branch in the whole file is the `for (i < len)` loop header, whose bound
is the compile-time constant `sizeof(house) == 16`. So the option axis has
exactly one value: *none*.

### Public entry points (full set, including lowest level)

| entry point | linkage | signature |
|---|---|---|
| `driver` | exported (`driver.h`, confirmed by `nm -D`) | `void driver(int floors)` |
| `print_hex` | `static` — NOT an entry point, not in `nm -D` of either `.so` | `void print_hex(unsigned char *, int)` |

`driver` is simultaneously the highest- and lowest-level public entry point;
there is no convenience wrapper to prefer over a primitive. `print_hex` cannot be
reached from outside the object, so it is exercised transitively through `driver`
(and its own boundary conditions are recorded as E8/E9 in `ERRORS.md`).

### Input shapes the code actually distinguishes

The single `int` parameter is stored into `house.floors` and re-read as 4
little-endian bytes by `print_hex`, so what the code is really sensitive to is
the **byte pattern** of the value, plus the struct layout constants:

- sign (positive / zero / negative — affects the top bit of byte 3)
- magnitude class (fits in 1 / 2 / 3 / 4 bytes — affects how many bytes are `00`)
- presence of `0x00` bytes inside the value (probes for any accidental
  C-string / `strlen`-style truncation in the hex printer)
- byte values that need zero-padding in `%02x` (nibble `< 0x10`) vs. not — the
  `%02x` format is the one place a formatting divergence can hide
- byte values `>= 0x80` (probes `char`-vs-`unsigned char` sign-extension, the
  classic bug given the C casts `char raw[]` to `unsigned char *`)
- the two hard-coded fields: `bedrooms = 3` (int) and `bathrooms = 2.0` (double,
  IEEE-754 `0x4000000000000000`) — fix the tail 12 bytes and the struct
  offsets 4 and 8
- padding bytes: `house_t` is 16 bytes with offsets 0/4/8 and **no** padding
  holes on this ABI, but `house = {0}` zero-fills anyway; a Rust layout that
  introduced padding or reordered fields would show up as differing bytes
- call multiplicity: one / many / interleaved (probes retained state)

## Configuration table

One row per meaningful combination the C treats differently. Every row is driven
through **both** `.so` exports via `libloading` with **many randomized inputs**
(fixed seed `0x5EED_1234_5678_9ABC`, SplitMix64) except where a row names exact
constants, and stdout is captured and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | no options (the only mode); `floors = 0` — the all-zero byte pattern | [x] |
| C2 | `driver` | `floors = 1` — smallest positive, three `00` bytes, exercises `%02x` zero-padding | [x] |
| C3 | `driver` | positive values fitting in 1 byte: all 256 of `0x00..0xFF` exhaustively | [x] |
| C4 | `driver` | positive values fitting in 2 bytes: randomized `0x0100..0xFFFF` | [x] |
| C5 | `driver` | positive values fitting in 3 bytes: randomized `0x010000..0xFFFFFF` | [x] |
| C6 | `driver` | full-width positive values: randomized `0x01000000..0x7FFFFFFF` | [x] |
| C7 | `driver` | negative values, small magnitude: randomized `-1..-255` (high bytes `ff`) | [x] |
| C8 | `driver` | negative values, full range: randomized `INT_MIN..-1` | [x] |
| C9 | `driver` | byte patterns containing embedded `0x00` bytes (`0x00FF00FF`, `0xFF00FF00`, `0x00010000`, `0x0000FF00`, …) — truncation probe | [x] |
| C10 | `driver` | byte patterns where every byte is `>= 0x80` (`0x80808080`, `0xFFFFFFFF`, `0x8090A0B0`) — sign-extension probe | [x] |
| C11 | `driver` | byte patterns where every byte is `< 0x10` (`0x01020304`, `0x0F0E0D0C`) — `%02x` padding probe on all four bytes | [x] |
| C12 | `driver` | bytes that are printable ASCII (`0x41424344` = "ABCD", `0x2F2E2D2C`) — probes any accidental `%s`/char handling | [x] |
| C13 | `driver` | boundary values: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`, `0x7FFFFFFF`, `i32::MIN as u32` | [x] |
| C14 | `driver` | powers of two and powers-of-two-minus-one for all 32 bit positions (`1<<k`, `(1<<k)-1`) — walks a single 1-bit through every byte lane | [x] |
| C15 | `driver` | uniformly random full-width `i32` (bulk property test, 4096 values) | [x] |
| C16 | `driver` | struct-shape invariants held constant by the C: output length is always 33 bytes; bytes 4..8 always `03000000`; bytes 8..16 always `0000000000000040` (IEEE-754 `2.0` LE) — asserted on every randomized input, for both libraries | [x] |
| C17 | `driver` | call multiplicity: one call; then many sequential calls in one captured region (concatenated output); then C/Rust calls interleaved in alternating order — probes retained state and stdout buffering order | [x] |
| C18 | `driver` | repeated identical input (same `floors` 64 times) — idempotence / no accumulation | [x] |
| C19 | `driver` | symbol-level parity of the entry point itself: `nm -D` C set ⊆ Rust set | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table and no optional
dependencies, so the feature power-set is a single element: the default (empty)
configuration. `--no-default-features` is therefore identical to the default
build, and is still run explicitly by `run_all_feature_combos.sh` for the record.
