# Configuration surface

## Build-time configurations

`Cargo.toml` contains no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional sources. Therefore the full valid feature matrix is:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features` (no features enabled) | default/unconditional |

## Runtime configurations

The public header declares only `void driver(int x)`. The implementation has no
runtime options or data-shape branches. Its sole loop always emits every byte
of the fixed-width C `int` object representation in native memory order.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; all C `int` values, including zero, extrema, mixed byte patterns, and many fixed-seed randomized values; native byte order and `sizeof(int)` output width | [x] |
