# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no CMake
options, conditional branches, compile definitions, or conditional sources.
There is exactly one valid build-time combination:

| # | Cargo feature set | Cargo command shape | C configuration | [ ] |
|---|-------------------|---------------------|-----------------|-----|
| 1 | Empty set | `cargo ... --no-default-features --features ''` | CMake defaults | [x] |

## Runtime and Input Configurations

The C source has no public header and no runtime option, mode, flag, enum,
element-type, byte-order, variable-length, or format selector. The complete
C-defined public entry-point set from `nm -D --defined-only` is `driver` and
`main`.

`driver` accepts one fixed-width C `int` and emits all `sizeof(int)` object
representation bytes in native byte order. `main` initializes that same shape
to zero and passes `%d` input through glibc `scanf`; the input classes below are
the distinct assignment outcomes of that call.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; arbitrary `int`, including zero, positive, negative, `INT_MIN`, and `INT_MAX`; fixed `sizeof(int)` bytes | [x] |
| 2 | `main` -> `driver` | No options; successful canonical decimal conversion across the full `int` range | [x] |
| 3 | `main` -> `driver` | No options; successful `%d` conversion with accepted whitespace/sign/leading-zero syntax or a nonnumeric suffix | [x] |
| 4 | `main` -> `driver` | No options; matching failure before assignment, leaving initialized `x == 0` | [x] |
| 5 | `main` -> `driver` | No options; input failure/EOF before assignment, leaving initialized `x == 0` | [x] |

Lowest-level testing starts at `driver`; composed testing then exercises
`main` and its call to `driver`.

Test mapping:

- Row 1: `phase_b_driver_all_int_shapes` (517 inputs).
- Row 2: `phase_b_main_canonical_decimal` (69 inputs).
- Row 3: `phase_b_main_accepted_syntax` (96 inputs).
- Row 4: `phase_b_main_matching_failure` (48 inputs).
- Row 5: `phase_b_main_input_failure_at_eof` (32 inputs).
