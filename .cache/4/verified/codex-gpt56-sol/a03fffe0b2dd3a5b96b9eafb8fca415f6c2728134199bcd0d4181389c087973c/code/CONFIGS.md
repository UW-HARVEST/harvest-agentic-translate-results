# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options or conditional definitions. The only build-time configuration is the
empty Cargo feature set, tested with `--no-default-features`.

The C source has no `if`, `switch`, preprocessor condition, runtime option,
mode, flag, pointer, length, enum, or data-format branch. Its full public entry
point surface consists of `driver` and `main`. The rows below enumerate the
distinct state transitions caused by the two sequential `%d` conversions in
`main`, including failed conversions whose return values C intentionally
ignores.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | Direct call with arbitrary full-range 32-bit C `int` values for `x` and `y`; output is the decimal rendering of `x bitor compl y` followed by `\n`. | [x] |
| 2 | `main` -> `driver` | Standard input contains two valid, representable decimal C `int` values; both `scanf` calls convert one value. | [x] |
| 3 | `main` -> `driver` | The first `scanf` reaches EOF or sees a nonmatching token; both zero-initialized operands remain zero because the second call sees the same EOF/token. | [x] |
| 4 | `main` -> `driver` | The first `scanf` converts one valid C `int`; the second reaches EOF or sees a nonmatching token, so `y` remains zero. | [x] |
