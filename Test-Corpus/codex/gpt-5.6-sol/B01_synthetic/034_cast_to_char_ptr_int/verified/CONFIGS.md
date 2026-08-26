# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no options,
conditionals, or compile definitions and declares one unconditional target.
There is exactly one valid feature combination:

| # | Cargo features | CMake configuration | check command | [ ] |
|---|---|---|---|---|
| 1 | none (empty set) | default | `cargo check --no-default-features` | [x] |

## Runtime Configurations

Mechanical audit scope: public definitions in `c_src/src/main.c`, plus all
`if`, `switch`, preprocessor, and option/flag branches. There are no public
headers, runtime options, modes, flags, variable-size inputs, element types,
formats, or byte-order switches. `print_hex` is `static` and is not a public
entry point.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(int)` | no options; arbitrary C `int`, including zero, positive, negative, `INT_MIN`, and `INT_MAX`; emits all `sizeof(int)` native-representation bytes | [x] |
| 2 | `main()` | no options; stdin contains a valid decimal C `int`; runs input, `driver`, and successful return end to end | [x] |

Both rows are covered by `tests/differential.rs` with a fixed seed. Row 1 uses
the five named boundary values plus 1,024 randomized values. Row 2 uses 256
randomized values across plain, leading-whitespace, and explicit-sign decimal
forms.
