# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
`option()` or configurable `set()` declarations. The complete valid feature
combination set is:

1. Empty feature set: `--no-default-features`

## Runtime and Input Configurations

The public header declares only `void driver(int x)`. The implementation has
no runtime options, modes, flags, format choices, byte-order choices, element
types, variable counts, or variable sizes. Its only loop serializes the fixed
`sizeof(house_t)` byte sequence. `floors` changes output data but does not
select a distinct branch, so the full C `int` domain forms one configuration
row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | No options; one by-value C `int`, covering `INT_MIN`, negative, zero, positive, `INT_MAX`, and randomized values; fixed 16-byte `house_t` output as 32 lowercase hexadecimal bytes plus newline | [x] |
