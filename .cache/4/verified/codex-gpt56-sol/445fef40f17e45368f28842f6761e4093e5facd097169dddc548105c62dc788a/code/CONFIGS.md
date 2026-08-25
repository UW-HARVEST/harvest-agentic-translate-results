# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional definitions. There is exactly one valid combination:

| # | Rust features | CMake options | [ ] |
|---|---------------|---------------|-----|
| 1 | none (`--no-default-features`) | none (default configuration) | [x] |

## Runtime configurations

The public header declares only `void driver(double)`. The implementation has
no runtime option, mode, flag, conditional, switch, size, count, format, byte
order, or alternate entry point. It always emits the same three views of the
input (`uint64_t` hex bits, `%a`, and `%.4f`) followed by a newline. Thus the
source distinguishes one runtime configuration, spanning every IEEE-754 bit
pattern.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | no options; arbitrary `double` bit pattern, with randomized coverage plus finite extrema, normal/subnormal boundaries, signed zero, infinities, and NaN payload/sign variants | [x] |
