# Configuration Surface

`Cargo.toml` defines no features, and `c_src/CMakeLists.txt` defines no options
or conditional source selection. The sole build-time configuration is the
empty feature set (`--no-default-features`).

The rows below cover every globally defined entry point reported by `nm -D`,
including symbols not declared in the public header. The C source has no
runtime modes, flags, formats, element types, byte-order branches, or
size/count parameters.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | No options; non-null, NUL-terminated byte string, including empty and non-empty randomized strings | [x] |
| 2 | `bad` | No options and no input; fixed output path | [x] |
| 3 | `good` | No options and no input; fixed output path including `helperGood` | [x] |
| 4 | `driver` | No options and no input; full composed call path through `good` and `bad` | [x] |
