# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no build
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo invocation suffix | C configuration | [ ] |
|---|-------------------------|-----------------|-----|
| 1 | `--no-default-features` | CMake defaults | [x] |

The compile check for this combination passes.

## Runtime Configurations

The source scan covered the public header and implementation for conditionals,
switches, options, flags, formats, counts, element types, and data-dependent
branches. The library has one public entry point, no runtime options, and no
conditional branches. Its input is a scalar `uint32_t`, so one row covers the
full input domain. Boundary values and values with nonzero upper 16 bits are
included because the C masks intentionally discard those upper bits.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `rev16` | No options; every `uint32_t` value, including `0`, `UINT16_MAX`, values above `UINT16_MAX`, and `UINT32_MAX` | [x] |
