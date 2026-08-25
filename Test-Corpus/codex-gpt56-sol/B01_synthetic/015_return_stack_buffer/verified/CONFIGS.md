# Configuration Surface

`Cargo.toml` has one feature combination: the empty set (`default = []`).
`c_src/CMakeLists.txt` has no options, compile definitions, or conditional
sources, so the C build also has one configuration.

The C source has no public header. The full public API below is the set of
defined dynamic symbols in `c_src/build/libdriver_c.so`. Static helpers are
covered through `bad` and `good`.

| # | entry point(s) | configuration (options set + input shape) | Passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `printLine` | Empty feature set; non-null C string; randomized lengths including empty, one byte, and many bytes | [x] |
| 2 | `bad` | Empty feature set; no input; GCC 11.5 `helperBad()` result | [x] |
| 3 | `good` | Empty feature set; no input; static helper string | [x] |
| 4 | `main`, `bad`, `printLine` | Empty feature set; `scanf("%d")` parses integer zero | [x] |
| 5 | `main`, `good`, `printLine` | Empty feature set; `scanf("%d")` parses a positive or negative nonzero integer | [x] |
| 6 | `main`, `bad`, `printLine` | Empty feature set; input is EOF or does not begin with a decimal integer, leaving initialized `x == 0` | [x] |
