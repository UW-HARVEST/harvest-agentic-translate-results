# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no feature declarations, and `c_src/CMakeLists.txt` has no
options, compile definitions, or conditional sources. There is one valid
feature combination:

| # | Cargo invocation | C configuration | [ ] |
|---|------------------|-----------------|-----|
| 1 | `--no-default-features --features ""` | default (only) | [x] |

## Runtime Configurations

The source has no option, mode, flag, type, format, byte-order, width, or
length branches. `printLine` has one valid branch (`line != NULL`) and one
invalid branch recorded in `ERRORS.md`. `main` does not inspect either
argument; its boundary-shaped inputs are retained below to verify that
behavior through the ABI.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-null NUL-terminated byte string; empty, one-byte, and many-byte randomized values | [x] |
| 2 | `bad` | no arguments; direct low-level call | [x] |
| 3 | `good` | no arguments; direct call including `helperGood` composition | [x] |
| 4 | `main` | `argc == 0`, `argv == NULL` | [x] |
| 5 | `main` | ignored argument boundaries: negative, positive, and `INT_MAX` `argc`, with null and non-null `argv` | [x] |
