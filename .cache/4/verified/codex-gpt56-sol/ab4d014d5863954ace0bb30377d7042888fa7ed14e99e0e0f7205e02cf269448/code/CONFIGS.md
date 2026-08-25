# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, implicit optional-dependency features,
or default features. `c_src/CMakeLists.txt` has no options, conditional
sources, or compile definitions. There is exactly one valid combination:

| # | Cargo invocation feature set | CMake configuration |
|---|------------------------------|---------------------|
| 1 | `--no-default-features` (empty set) | default |

## Runtime and Input Configurations

The C dynamic symbol table exposes both `run` and `driver`, although only
`driver` is declared in the public header. The process-global house starts at
`{ floors: 2, bedrooms: 5, bathrooms: 2.5 }`. There are no runtime options,
flags, enums, byte-order modes, element types, buffers, or count/length
parameters.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|-------------------------------------------|---------|
| 1 | `run` | Direct low-level call with negative, zero, and positive `int` bedroom deltas; one stateful update cycle emits all four house states | [x] |
| 2 | `driver` -> `run` | Plain base-10 digits in the inclusive `int` range; two stateful update cycles | [x] |
| 3 | `driver` -> `run` | Leading C-locale whitespace and optional `+`/`-`, followed by base-10 digits in range | [x] |
| 4 | `driver` -> `run` | In-range base-10 numeric prefix followed by nonnumeric bytes; accepted because C only checks `endp != str` | [x] |
| 5 | `driver` -> `run` | Exact `INT_MIN` and `INT_MAX` decimal boundary values | [x] |

Each row is exercised with a fixed-seed randomized input set. Because the API
uses mutable process-global state and C signed overflow is undefined, each
random case runs in fresh child processes and compares complete stdout bytes.
