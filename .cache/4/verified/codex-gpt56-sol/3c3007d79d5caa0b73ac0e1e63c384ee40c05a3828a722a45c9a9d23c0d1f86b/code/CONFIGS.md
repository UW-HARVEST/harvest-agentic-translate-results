# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section, and `c_src/CMakeLists.txt` defines no
options or conditional sources. There is one valid feature combination:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features` (empty set) | default |

## Runtime Configurations

The public header declares only `driver(int x, int y)`. The implementation has
no runtime option, mode, flag, conditional branch, input-shape distinction, or
lower-level entry point. Both arguments may hold any value in the C `int`
domain, including `INT_MIN`, `INT_MAX`, zero, and negative values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; scalar `(x, y)` spans the full two's-complement C `int` domain; output is the decimal representation of `x bitor compl y` followed by `\n` | [x] |
