# Configuration Surface

## Build-time configurations

`Cargo.toml` declares `default = []` and no named features. CMake declares no
options or conditional definitions. Therefore the complete build-time feature
cross-product has one member:

| # | Cargo invocation | C configuration |
|---|------------------|-----------------|
| 1 | `--no-default-features` | default CMake configuration |

## Runtime and input configurations

The public entry points are the two globally defined symbols shown by
`nm -D`: `driver(int)` and `main(void)`. `print_hex` is `static` and is reached
through `driver`. There are no runtime modes, flags, element types, byte-order
options, or variable length/count parameters.

`driver` does not branch on its argument, so one randomized row spans the full
C `int` domain, including `INT_MIN`, zero, and `INT_MAX`. `main` always calls
`scanf("%d", &x)` and then `driver(x)`; its meaningful stream shapes are
listed separately because `scanf` consumes them differently.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` -> `print_hex` | no options; randomized full-domain `int`, including both bounds and zero | [x] |
| 2 | `main` -> `driver` -> `print_hex` | no options; randomized in-range decimal tokens with leading whitespace and optional `+`/`-` sign | [x] |
| 3 | `main` -> `driver` -> `print_hex` | no options; empty input (EOF before conversion) | [x] |
| 4 | `main` -> `driver` -> `print_hex` | no options; nonnumeric first byte (matching failure) | [x] |
| 5 | `main` -> `driver` -> `print_hex` | no options; numeric prefix followed by a nonnumeric suffix | [x] |
| 6 | repeated `main` calls -> `driver` -> `print_hex` | no options; many whitespace-separated decimal tokens in one stream | [x] |
| 7 | `main` -> `driver` -> `print_hex` | no options; decimal magnitude larger than C `int` (one step and far past each bound) | [x] |
