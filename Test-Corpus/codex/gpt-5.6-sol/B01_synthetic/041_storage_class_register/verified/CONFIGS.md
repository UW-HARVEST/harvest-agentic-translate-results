# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional definitions. There is exactly one valid combination:
the empty feature set, exercised with `--no-default-features`.

## Runtime configurations

The C source has no runtime option, mode, flag, `if`, `switch`, or conditional
compilation branch. The rows below cover every public entry point and every
outcome distinguished by the only input operation, `scanf("%d", &x)`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|---|---|---|
| 1 | `driver` | direct scalar `int x`; randomized full-domain values including `INT_MIN`, `INT_MAX`, and compiled arithmetic-wrap cases | [x] |
| 2 | `main` -> `driver` | stdin contains a valid signed decimal `int`, with randomized full-domain values and accepted sign/whitespace forms | [x] |
| 3 | `main` -> `driver` | stdin begins with a nonmatching byte, so `scanf` returns 0 and leaves initialized `x == 0` | [x] |
| 4 | `main` -> `driver` | stdin is at EOF (empty or whitespace-only), so `scanf` returns EOF and leaves initialized `x == 0` | [x] |
