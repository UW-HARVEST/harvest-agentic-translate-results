# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional sources. There is exactly one valid feature
combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (empty feature set) | Default configuration | [x] |

## Runtime configurations

The only public entry point is `bin2hex`. It has no modes, flags, enums, byte
order options, or element types beyond bytes. The rows below cover the loop's
empty/one/many shapes, the capacity guard's valid boundary, and all four
combinations selected by the high- and low-nibble numeric (`0..=9`) versus
alphabetic (`10..=15`) encoding arithmetic.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `bin2hex` | Empty input (`bin_len = 0`); valid `hex_maxlen >= 1`; loop skipped | [x] |
| 2 | `bin2hex` | One byte; high nibble numeric and low nibble numeric; exact minimum output capacity | [x] |
| 3 | `bin2hex` | One byte; high nibble numeric and low nibble alphabetic; exact minimum output capacity | [x] |
| 4 | `bin2hex` | One byte; high nibble alphabetic and low nibble numeric; exact minimum output capacity | [x] |
| 5 | `bin2hex` | One byte; high nibble alphabetic and low nibble alphabetic; exact minimum output capacity | [x] |
| 6 | `bin2hex` | Many bytes with mixed nibble classes; exact minimum output capacity `2 * bin_len + 1` | [x] |
| 7 | `bin2hex` | Many bytes with mixed nibble classes; excess output capacity greater than `2 * bin_len + 1` | [x] |
