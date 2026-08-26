# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` declares no
options or compile definitions. There is exactly one valid feature
combination:

| # | Cargo invocation | CMake configuration | verified |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` | default | [x] |

## Runtime configurations

The public API consists only of `driver(int x, int y)`. The rows below are the
cross-product of the equivalence classes induced by the C branches
`x > 0 || y > 0`, `x == 1 && y == 4`, `x > 0`, `y == 0`, and `x < 3`.

For `x > 0, y < 0`, the C control flow does not terminate before eventually
encountering signed-overflow undefined behavior. Those rows compare a bounded,
deterministic output prefix and confirm that neither implementation returns
within that prefix.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; `x <= 0`, `y < 0` | [x] |
| 2 | `driver` | no options; `x <= 0`, `y == 0` | [x] |
| 3 | `driver` | no options; `x <= 0`, `1 <= y <= 3` | [x] |
| 4 | `driver` | no options; `x <= 0`, `y == 4` | [x] |
| 5 | `driver` | no options; `x <= 0`, `y >= 5` | [x] |
| 6 | `driver` | no options; `x == 1`, `y < 0` (nonterminating) | [x] |
| 7 | `driver` | no options; `x == 1`, `y == 0` | [x] |
| 8 | `driver` | no options; `x == 1`, `1 <= y <= 3` | [x] |
| 9 | `driver` | no options; `x == 1`, `y == 4` (special `goto label2`) | [x] |
| 10 | `driver` | no options; `x == 1`, `y >= 5` | [x] |
| 11 | `driver` | no options; `x == 2`, `y < 0` (nonterminating) | [x] |
| 12 | `driver` | no options; `x == 2`, `y == 0` | [x] |
| 13 | `driver` | no options; `x == 2`, `1 <= y <= 3` | [x] |
| 14 | `driver` | no options; `x == 2`, `y == 4` | [x] |
| 15 | `driver` | no options; `x == 2`, `y >= 5` | [x] |
| 16 | `driver` | no options; `x >= 3`, `y < 0` (nonterminating) | [x] |
| 17 | `driver` | no options; `x >= 3`, `y == 0` | [x] |
| 18 | `driver` | no options; `x >= 3`, `1 <= y <= 3` | [x] |
| 19 | `driver` | no options; `x >= 3`, `y == 4` | [x] |
| 20 | `driver` | no options; `x >= 3`, `y >= 5` | [x] |
