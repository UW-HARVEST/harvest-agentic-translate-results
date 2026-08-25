# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo feature combination | CMake configuration | Verified |
|---|---------------------------|---------------------|----------|
| 1 | Empty set (`--no-default-features`) | Default | [x] |

## Runtime Configurations

Mechanical scan scope: the complete public C surface and every conditional,
switch, preprocessor branch, parameter, option, mode, flag, and input-shape
check in the non-generated C source.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `main` | No options and no input; write `Hello World!\n`, then return `0` | [x] |

There are no lower-level entry points, public headers, runtime options, input
shapes, or value-dependent branches.
