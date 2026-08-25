# Configuration Surface

The CMake file has no build options or preprocessor definitions. `Cargo.toml`
has no features, so the complete feature combination set is:

| combination | Cargo invocation |
|-------------|------------------|
| empty set | `cargo ... --no-default-features` |

The C source has one public entry point, no runtime options or flags, and one
successful input shape. `multi_stage` is `static` and therefore is not a public
entry point.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|-------------------------------------------|-----|
| 1 | `main` | no options; `scanf` converts all three signed decimal `int` values and produces `x == 1`, `y == 2`, `z == 3` (arbitrary accepted whitespace and sign spellings) | [x] |
