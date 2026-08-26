# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and the CMake file has no options or
preprocessor definitions. There is exactly one valid feature combination:

| # | Cargo features | C configuration | check command |
|---|----------------|-----------------|---------------|
| 1 | empty set | default | `cargo check --no-default-features` |

## Runtime configurations

The rows below are derived from all five library-defined symbols and the
`line != NULL` and `x != 0` branches in `c_src/src/main.c`. Failed `scanf`
conversion and EOF are separate input shapes because `x` retains its
initialized zero value in both cases.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `printLine` | non-null pointer to an empty NUL-terminated string | [x] |
| 2 | `printLine` | non-null pointer to a non-empty NUL-terminated string; randomized contents and lengths | [x] |
| 3 | `printIntLine` | randomized `int`, including zero, signs, and `INT_MIN`/`INT_MAX` | [x] |
| 4 | `bad` | fixed 10-byte allocation and ten-element zero source | [x] |
| 5 | `good` | fixed ten-`int` allocation and ten-element zero source | [x] |
| 6 | `main` | successful `%d` conversion with `x == 0`, selecting `bad` | [x] |
| 7 | `main` | successful `%d` conversion with randomized `x != 0`, selecting `good` | [x] |
| 8 | `main` | failed `%d` conversion, leaving initialized `x == 0` and selecting `bad` | [x] |
| 9 | `main` | EOF before conversion, leaving initialized `x == 0` and selecting `bad` | [x] |
