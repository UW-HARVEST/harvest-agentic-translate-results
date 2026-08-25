# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no CMake
options, conditional source selection, compile definitions, or preprocessor
configuration. The complete feature combination set therefore contains one
member:

| # | Cargo feature set | C configuration | checked |
|---|-------------------|-----------------|---------|
| 1 | empty (`--no-default-features`) | default CMake configuration with PIC enabled | [x] |

## Runtime Configurations

There are no public headers and no runtime option setters, modes, flags, enums,
lengths, arrays, byte-order choices, or element-format choices. The complete
public API is the four global functions in `c_src/src/main.c`. The only
source-level data-dependent branch is `if (x)` in `main`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printIntPtrLine` | non-null pointer to a negative `int`, including `INT_MIN` | [x] |
| 2 | `printIntPtrLine` | non-null pointer to zero | [x] |
| 3 | `printIntPtrLine` | non-null pointer to a positive `int`, including `INT_MAX` | [x] |
| 4 | `good` -> `printIntPtrLine` | no input; fixed local `int` value `5` | [x] |
| 5 | `main` -> `good` -> `printIntPtrLine` | `scanf("%d")` succeeds with a negative nonzero integer | [x] |
| 6 | `main` -> `good` -> `printIntPtrLine` | `scanf("%d")` succeeds with a positive nonzero integer | [x] |
| 7 | `main` -> `bad` -> `printIntPtrLine` | `scanf("%d")` succeeds with zero | [x] |

Failed conversion, EOF, null-pointer, and the direct `bad` path are invalid or
indeterminate configurations and are tracked in `ERRORS.md`.
