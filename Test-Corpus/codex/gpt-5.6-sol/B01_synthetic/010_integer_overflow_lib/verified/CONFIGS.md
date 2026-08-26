# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` defines no
options or conditional sources. There is exactly one valid feature
combination:

| # | default features | explicit features |
|---|------------------|-------------------|
| 1 | Disabled | None (empty feature set) |

## Runtime Configurations

The public dynamic-symbol surface has two entry points. The C source contains
no runtime options, modes, flags, branches, switches, pointer/length shapes,
or element formats. A C `char` is the sole input shape, so each row covers its
complete 256-value object-representation domain, including zero, signed
boundaries, and the conversion after adding one.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printHexCharLine` | No options; direct low-level call with every `char` byte value | [x] |
| 2 | `driver` | No options; composed add-one then print call with every `char` byte value | [x] |
