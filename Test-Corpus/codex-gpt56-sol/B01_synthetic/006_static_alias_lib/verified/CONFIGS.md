# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, compile definitions, conditional sources, or conditional branches.
The complete feature matrix therefore contains one valid combination:

| # | Rust feature set | CMake configuration | Compile check |
|---|------------------|---------------------|---------------|
| 1 | `{}` (no default or explicit features) | default | [x] |

The Rust command for the empty combination is:

```sh
cargo check --no-default-features --features ''
```

## Runtime Configurations

The public header declares two entry points. The low-level `static_alias`
branches on `*outer >= inner`, mutates one of two possible storage locations,
returns an alias to that location, and retains `inner` across calls. `driver`
branches through its loop condition and feeds each returned alias into the next
call. The table separates branch boundaries, persistent alias shapes, and loop
cardinalities.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `static_alias` | fresh caller-owned `int`, `*outer < inner` | [x] |
| 2 | `static_alias` | fresh caller-owned `int`, `*outer == inner` boundary | [x] |
| 3 | `static_alias` | fresh caller-owned `int`, `*outer > inner` | [x] |
| 4 | `static_alias` | returned caller-owned pointer reused while its value remains below `inner` | [x] |
| 5 | `static_alias` | returned caller-owned pointer reused across the below-to-equal/above transition | [x] |
| 6 | `static_alias` | returned static `inner` pointer reused, so source and destination alias | [x] |
| 7 | `driver` | `iterations < 0`; loop body is skipped | [x] |
| 8 | `driver` | `iterations == 0`; loop body is skipped | [x] |
| 9 | `driver` | one iteration, `initial_value < inner` | [x] |
| 10 | `driver` | one iteration, `initial_value == inner` boundary | [x] |
| 11 | `driver` | one iteration, `initial_value > inner` | [x] |
| 12 | `driver` | many iterations; returned pointer remains caller-owned throughout | [x] |
| 13 | `driver` | many iterations; returned pointer transitions from caller-owned to static | [x] |
| 14 | `driver` | many iterations; first call returns static and later calls self-alias it | [x] |

All integer inputs use the C `int` width and native byte order. There are no
element-type, format, byte-order, flag, option, or enum axes in the public API.
