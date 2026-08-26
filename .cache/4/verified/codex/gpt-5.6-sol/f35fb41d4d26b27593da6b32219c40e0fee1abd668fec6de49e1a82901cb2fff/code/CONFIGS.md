# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table, optional dependencies, or default
features. `c_src/CMakeLists.txt` has no CMake options or conditional sources.
There is exactly one valid feature combination:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features --features ''` | default, unconditional build |

## Runtime configurations

The public header declares only `driver(int x)`. The implementation has no
public options, modes, flags, formats, element types, counts, lengths, pointer
arguments, or conditional branches. `sizeof(int)` fixes the output shape to
the native object representation, and the loop always emits every byte in
increasing address order.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; every native `c_int` value, including `INT_MIN`, `-1`, `0`, `1`, `INT_MAX`, leading-zero byte patterns, and many fixed-seed randomized bit patterns | [x] |
