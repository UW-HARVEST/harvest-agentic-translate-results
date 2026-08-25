# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options, compile definitions, conditional sources, or other selectable
backends. The full valid feature-combination set is:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features --features ""` (empty set) | Default and only configuration |

## Runtime Configurations

The sole public entry point is `crc16(const uint8_t *, uint32_t, uint16_t)`.
The source branches on `len >= 8` and then on each remaining byte. Data bytes
and all 65,536 initial CRC values affect table indices but do not create
additional control-flow modes, so each row randomizes both.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `crc16` | Empty input (`len == 0`), null and non-null pointers, arbitrary initial CRC | [x] |
| 2 | `crc16` | Remainder-only input (`len == 1..=7`), arbitrary bytes and initial CRC | [x] |
| 3 | `crc16` | Exactly one slicing-by-8 iteration (`len == 8`), no remainder | [x] |
| 4 | `crc16` | One slicing-by-8 iteration plus remainder (`len == 9..=15`) | [x] |
| 5 | `crc16` | Multiple slicing-by-8 iterations, no remainder (`len >= 16`, multiple of 8) | [x] |
| 6 | `crc16` | Multiple slicing-by-8 iterations plus remainder (`len >= 17`, not a multiple of 8) | [x] |

Every row must compare calls loaded from both shared objects through
`libloading`, using deterministic randomized inputs and byte-identical `u16`
results.
