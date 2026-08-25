# Configuration Surface

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options or conditional compilation. There is one valid build-time
configuration:

| # | Cargo feature combination | C configuration | verified |
|---|---------------------------|-----------------|----------|
| 1 | `--no-default-features` with an empty feature set | default CMake configuration | [x] |

The C source has one public entry point, `main`, and no runtime option or mode
flags. The rows below cover each successful input shape that the code delegates
to `strtod`, the value classes passed to `pow`, and the distinct `%.2f` output
classes. Error-producing branches are listed separately in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `main` | exactly three arguments; finite decimal base and finite integral exponent; ordinary finite result | [x] |
| 2 | `main` | exactly three arguments; finite decimal base and finite fractional exponent; ordinary finite result | [x] |
| 3 | `main` | exactly three arguments; leading ASCII whitespace accepted by `strtod` on base and/or exponent | [x] |
| 4 | `main` | exactly three arguments; explicit leading `+` or `-` sign on base and/or exponent | [x] |
| 5 | `main` | exactly three arguments; empty base and/or exponent string, which C accepts as zero because `errno` remains zero and the end pointer points at `'\0'` | [x] |
| 6 | `main` | exactly three arguments; hexadecimal floating-point base and/or exponent | [x] |
| 7 | `main` | exactly three arguments; infinity spelling accepted by `strtod`, with `pow` leaving `errno` clear | [x] |
| 8 | `main` | exactly three arguments; NaN or NaN-payload spelling accepted by `strtod` | [x] |
| 9 | `main` | exactly three arguments; positive or negative zero, including a sign-sensitive `pow` result | [x] |
| 10 | `main` | exactly three arguments; finite result whose `%.2f` conversion rounds, including carry into the integer part | [x] |
| 11 | `main` | exactly three arguments; successful `pow` result is positive or negative infinity without `errno == EDOM` or `ERANGE` | [x] |
