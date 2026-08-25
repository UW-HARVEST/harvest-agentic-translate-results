# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` defines no
options or conditional sources. There is exactly one valid feature
combination:

| # | Cargo invocation | C configuration | checked |
|---|------------------|-----------------|---------|
| 1 | `--no-default-features` (no named features) | default CMake configuration | [x] |

## Runtime Matrix

The public dynamic surface consists of `run(int)` and `driver(int)`.
`driver(int)` invokes `run(int)` twice. Both operate on the same persistent
process-global house state. There are no runtime modes, flags, formats,
element types, byte-order choices, conditional branches, or input shape
variants. Random inputs must keep all signed C additions in range.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `run` | Fresh library state; one direct call; randomized negative, zero, and positive scalar `int` values | [x] |
| 2 | `run` | Fresh library state; many direct calls with randomized scalar `int` values; verify persistent state after every call | [x] |
| 3 | `driver` | Fresh library state; one composed call (two internal `run` calls); randomized negative, zero, and positive scalar `int` values | [x] |
| 4 | `run`, `driver` | Fresh library state; randomized mixed sequence of both entry points; verify their shared persistent state after every call | [x] |
