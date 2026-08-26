# Configuration Surface

`Cargo.toml` has one valid feature combination: the empty set
(`--no-default-features`). CMake declares no options or conditional sources.

The runtime axes below come directly from the branches and state mutations in
`c_src/src/main.c`: direct versus composed entry point, initial versus mutated
global house state, `extra_bedrooms` sign/range, `fgets` newline/EOF/99-byte
shapes, `strtol` whitespace/sign/digit-prefix shapes, and the inclusive
`INT_MIN`/`INT_MAX` checks.

| # | entry point(s) | configuration (options set + input shape) | test |
|---|----------------|-------------------------------------------|------|
| C1 | `run` | initial global state; `extra_bedrooms == 0` | [x] |
| C2 | `run` | mutated global state; randomized positive `extra_bedrooms` | [x] |
| C3 | `run` | mutated global state; randomized negative `extra_bedrooms` | [x] |
| C4 | `run` | mutated global state; values at and near the full C `int` boundaries | [x] |
| C5 | `run` | many consecutive calls, exercising accumulated floors, bedrooms, and bathrooms | [x] |
| C6 | `main` | unsigned decimal digits terminated by newline | [x] |
| C7 | `main` | one or more leading non-newline C-locale whitespace bytes before decimal digits | [x] |
| C8 | `main` | explicit `+` sign before decimal digits | [x] |
| C9 | `main` | explicit `-` sign before decimal digits | [x] |
| C10 | `main` | zero represented with randomized leading zeroes | [x] |
| C11 | `main` | valid decimal prefix followed by non-digit suffix bytes | [x] |
| C12 | `main` | valid decimal prefix followed by an embedded NUL and ignored bytes | [x] |
| C13 | `main` | valid decimal input ending at EOF without a newline | [x] |
| C14 | `main` | input reaches the `fgets` 99-byte payload limit before newline/EOF | [x] |
| C15 | `main` | parsed values at and near inclusive `INT_MIN` and `INT_MAX` | [x] |

Every row is exercised repeatedly with a fixed-seed generator through both
shared-library exports. No Rust implementation function is called directly.
