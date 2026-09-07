# Configuration Surface

Mechanically derived from the public header, all externally visible functions
in `driver.c`, and every runtime `if`/preprocessor branch in the C source.
There are no Cargo features and no C compile-time feature branches. The sole
runtime option is `driver(useGood)`, whose two states are zero and nonzero.
`printLine` separately branches on pointer nullness; the null case is tracked
in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Direct low-level call with a non-null, NUL-terminated C string; randomized empty/non-empty byte strings without interior NULs. | [x] |
| 2 | `bad` | Direct low-level call; no arguments. `helperBad` returns its automatic-array pointer and the result is passed to `printLine`. | [x] |
| 3 | `good` | Direct low-level call; no arguments. `helperGood1` returns its static string and the result is passed to `printLine`. | [x] |
| 4 | `driver` → `bad` | `useGood == 0`. | [x] |
| 5 | `driver` → `good` | `useGood != 0`; randomized positive and negative nonzero C `int` values, including `INT_MIN` and `INT_MAX`. | [x] |

Feature combinations: default/no-feature build only (`Cargo.toml` declares no
features).
