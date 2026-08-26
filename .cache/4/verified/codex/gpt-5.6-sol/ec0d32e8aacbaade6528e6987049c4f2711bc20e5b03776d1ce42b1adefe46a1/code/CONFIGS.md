# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table or optional dependencies, and the C
source/CMake files have no configurable preprocessor branches. The full valid
feature matrix is therefore:

| # | default features | named features | command |
|---|------------------|----------------|---------|
| 1 | disabled | none (empty set) | `cargo check --no-default-features` |

## Runtime and Input Configurations

The public header declares `helloworld()`. The shared-library symbol scan also
exposes the composed program entry point `main()`. Neither accepts input, and
the C source contains no option, mode, flag, shape, size, type, count, format,
byte-order, boundary, `if`, or `switch` branch. Consequently, each entry point
has exactly one meaningful configuration and no randomizable input axis.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `helloworld` | no options; zero arguments; emits `Hello World!\n`; returns `0`; repeated calls | [x] |
| 2 | `main` | no options; zero arguments; delegates to `helloworld`; emits `Hello World!\n`; returns `0`; repeated calls | [x] |
