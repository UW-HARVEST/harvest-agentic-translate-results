# Configuration Surface

`Cargo.toml` originally has no `[features]` table and CMake declares no
options, cache variables, or preprocessor definitions. Therefore the complete
build-time feature set is one combination: no features
(`--no-default-features`).

There are no runtime modes or flags. The C source branches only on the
`i < x` loop condition. Its meaningful input shapes are negative, zero, one,
and many iterations. `main` adds the `%d` conversion input shape and then
delegates to `driver`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `driver` | no options; `x < 0`, so the loop is initially false | [x] |
| 2 | `driver` | no options; `x == 0`, so the loop is initially false | [x] |
| 3 | `driver` | no options; `x == 1`, exactly one output line | [x] |
| 4 | `driver` | no options; `x > 1`, many output lines and repeated `i++`, `j += 2` | [x] |
| 5 | `main` | no options; valid `%d` input yielding `x < 0`, including optional whitespace/sign | [x] |
| 6 | `main` | no options; valid `%d` input yielding `x == 0`, including optional whitespace/sign | [x] |
| 7 | `main` | no options; valid `%d` input yielding `x == 1`, including optional whitespace/sign | [x] |
| 8 | `main` | no options; valid `%d` input yielding `x > 1`, including leading whitespace and trailing bytes | [x] |
