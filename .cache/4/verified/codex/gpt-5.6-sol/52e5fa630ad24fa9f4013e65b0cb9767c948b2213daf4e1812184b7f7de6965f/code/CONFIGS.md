# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` defines a selectable backend or
conditional source. The complete feature power set has one member:

| # | Cargo invocation feature set | C configuration | |
|---|------------------------------|-----------------|---|
| F1 | `--no-default-features --features ""` (no features) | default CMake target, no options | [x] |

## Runtime and Input Configurations

Mechanical searches of the complete C source found no `if`, `switch`, `case`,
preprocessor conditional, runtime option, mode, flag, format selector, element
type selector, byte-order selector, or public header. The complete public entry
point set comes from `nm -D --defined-only`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| C1 | `find_container_of_a` | pointer to `struct test.a`; randomized `a` and `b` across the full `int` value range; returns the enclosing object at member offset zero | [x] |
| C2 | `find_container_of_b` | pointer to `struct test.b`; randomized `a` and `b` across the full `int` value range; returns the enclosing object by subtracting the nonzero member offset | [x] |
| C3 | `main` | two valid decimal operand strings; randomized values whose mathematical sum is in the `int` range | [x] |
| C4 | `main` | `atoi` lexical shapes used by the C call: signs, leading whitespace, nondigit suffixes, empty/nondigit strings, and `int` boundary values | [x] |
| C5 | `main` | operand pairs whose machine `int` addition crosses `INT_MIN` or `INT_MAX` in the compiled C reference | [x] |
| C6 | `main` | valid `argv[1]` and `argv[2]` with ignored `argc` values (`INT_MIN`, `-1`, `0`, `2`, `3`, `INT_MAX`) and zero or many trailing arguments | [x] |

There are no option/data cross-products beyond these rows: the two low-level
entry points perform fixed pointer arithmetic, while `main` has no branch on
`argc` or any runtime option.
