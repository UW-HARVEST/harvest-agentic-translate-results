# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional compilation. The complete valid feature combination is
therefore the empty set:

| # | Cargo feature combination | C configuration | check |
|---|---------------------------|-----------------|-------|
| 1 | `--no-default-features` (no `--features` argument) | default/unconditional | [x] `cargo check --no-default-features` |

## Runtime Configurations

The public API has no options, modes, flags, pointers, lengths, enums, byte
order controls, or variable element counts. `sizeof(float)` fixes the
`driver` output shape at four bytes in native byte order. The rows below cover
the full exported entry-point set and the input outcomes distinguished by the
fixed `%f` conversion used by `main`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; arbitrary `float` bit pattern, including signed zero, subnormal, normal, infinity, and NaN classes; four native-order bytes. | [x] |
| 2 | `main` | No options; stdin starts with a valid `%f` token (random finite decimal, infinity, or NaN spelling). | [x] |
| 3 | `main` | No options; stdin starts with a valid `%f` token followed by unconsumed trailing bytes. | [x] |
| 4 | `main` | No options; stdin starts with text that cannot be converted by `%f`; initialized positive zero is printed. | [x] |
| 5 | `main` | No options; stdin is at EOF before conversion; initialized positive zero is printed. | [x] |
