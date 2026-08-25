# Configuration Surface

## Build-Time Configuration

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional sources. There is exactly one valid build
configuration:

| # | Rust feature invocation | CMake configuration | Status |
|---|-------------------------|---------------------|--------|
| B1 | `--no-default-features` (empty feature set) | default, with position-independent code enabled | [x] compile checked |

## Runtime Configuration

The public header exposes only `next_double(cn_rnd_t *rnd)`. The C
implementation contains no `if`, `switch`, conditional compilation, runtime
option, mode, flag, type, format, count, width, or byte-order branch. Both
`uint64_t` state words accept their full value range. The state transition is
straight-line unsigned arithmetic and bit manipulation.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| V1 | `next_double` | no options; arbitrary two-word `uint64_t` state, including zero/maximum boundaries; one and repeated stateful calls | [x] |
