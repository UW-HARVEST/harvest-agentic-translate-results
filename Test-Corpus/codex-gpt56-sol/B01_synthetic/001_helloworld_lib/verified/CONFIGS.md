# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so there are no named or implicit
optional-dependency features. The complete valid feature combination set is:

| # | default features | named features | check command |
|---|------------------|----------------|---------------|
| 1 | disabled | none (empty set) | `cargo check --no-default-features` |

`c_src/CMakeLists.txt` defines one shared-library target from `src/hello.c` and
contains no options or conditional source selection. The requested default C
configuration is therefore the only C configuration.

## Runtime Configurations

Mechanical search of the public header and C implementation found one public
entry point, no arguments, and no runtime options, modes, flags, state,
conditionals, switches, or input shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `helloworld` | no options; no input; prints `Hello World!\n` and returns `0` | [x] |
