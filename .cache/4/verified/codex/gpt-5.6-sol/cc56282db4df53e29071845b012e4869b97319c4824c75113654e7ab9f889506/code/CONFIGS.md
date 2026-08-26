# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so Cargo defines no optional or default
features. `c_src/CMakeLists.txt` has no options or conditional sources.
Therefore the full valid build-time configuration set contains one member:

| # | Cargo invocation | CMake configuration |
|---|------------------|---------------------|
| 1 | `cargo ... --no-default-features` (no `--features` argument) | default configuration |

## Runtime and Input Configurations

The public header declares one entry point, `custom_strdup`. It has no runtime
options, modes, flags, element types, byte-order choices, formats, or count
parameters. The rows below cover the C-string storage shapes that exercise its
`strlen + 1`, allocation, and byte-copy behavior. Invalid shapes are in
`ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `custom_strdup` | no options; empty visible string (`str[0] == 0`) with varied trailing storage | [x] |
| 2 | `custom_strdup` | no options; one non-NUL byte followed by the terminator | [x] |
| 3 | `custom_strdup` | no options; many non-NUL bytes (random lengths and byte values) followed by the terminator | [x] |
| 4 | `custom_strdup` | no options; an early NUL followed by randomized trailing bytes, proving copying stops at the first terminator | [x] |

Call hierarchy: `custom_strdup` is both the lowest-level and only public API.
