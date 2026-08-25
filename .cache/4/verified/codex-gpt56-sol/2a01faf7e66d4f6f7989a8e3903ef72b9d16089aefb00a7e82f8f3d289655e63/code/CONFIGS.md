# Configuration Surface

The CMake file defines one executable target with no options, compile
definitions, or conditional sources. `Cargo.toml` initially defines no
features, so the complete build-time feature set is the empty combination
(`--no-default-features --features ''`).

The rows below are derived from every public dynamic symbol and every runtime
branch/input shape in `c_src/src/main.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null pointer to a NUL-terminated empty string. | [x] |
| 2 | `printLine` | Non-null pointer to randomized NUL-terminated byte strings without interior NUL bytes. | [x] |
| 3 | `bad` | No arguments; forwards the C automatic `data` pointer without initialization. | [x] |
| 4 | `good` | No arguments; fixed `"string"` data. | [x] |
| 5 | `main` | Valid decimal input converting to zero; takes the `x == 0` branch. | [x] |
| 6 | `main` | Valid randomized positive or negative nonzero `int` input; takes the `x != 0` branch. | [x] |
| 7 | `main` | Input with no integer conversion; initialized `x` remains zero. | [x] |
| 8 | `main` | EOF before conversion; initialized `x` remains zero. | [x] |

`printLine(NULL)` is the only rejected input and is tracked in `ERRORS.md`
rather than duplicated as a valid-path row.
