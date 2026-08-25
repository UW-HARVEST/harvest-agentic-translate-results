# Configuration Surface

## Build-time configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` declares a feature, option, or
conditional source. The complete feature matrix therefore has one member:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `cargo ... --no-default-features` (empty feature set) | default | [x] |

## Runtime configurations

The complete public API is `void driver(int x, int y)`. It has no options,
mode flags, state, pointers, lengths, element types, formats, or byte-order
settings. The rows below partition the defined `div` domain by the observable
quotient/remainder cases used by the implementation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | `x == 0`, `y > 0` | [x] |
| 2 | `driver` | `x == 0`, `y < 0` | [x] |
| 3 | `driver` | `x > 0`, `y > 0`, exact division | [x] |
| 4 | `driver` | `x > 0`, `y > 0`, nonzero remainder | [x] |
| 5 | `driver` | `x > 0`, `y < 0`, exact division | [x] |
| 6 | `driver` | `x > 0`, `y < 0`, nonzero remainder | [x] |
| 7 | `driver` | `x < 0`, `y > 0`, exact division | [x] |
| 8 | `driver` | `x < 0`, `y > 0`, nonzero remainder | [x] |
| 9 | `driver` | `x < 0`, `y < 0`, exact and non-exact division | [x] |
| 10 | `driver` | defined-domain `INT_MIN`/`INT_MAX` boundary operands | [x] |
