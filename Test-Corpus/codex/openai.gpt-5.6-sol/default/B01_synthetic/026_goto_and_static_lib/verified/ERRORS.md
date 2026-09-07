# Error-surface table

Derived mechanically from every conditional rejection in
`../c_src/src/driver.c`. The public function returns `void`, so the observable
C result is its exact stdout.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `driver` / `multi_stage` | `x != 1` (this rejects before inspecting `y` or `z`) | `Error: x != 1\nOperation failed\nResult: 1\n` | [x] |
| 2 | `driver` / `multi_stage` | `x == 1 && local_y != 2` (this rejects before inspecting `z`) | `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n` | [x] |
| 3 | `driver` / `multi_stage` | `x == 1 && local_y == 2 && z != 3` | `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n` | [x] |

Generic FFI boundary audit: this API has no pointer, length, buffer, or enum
parameters. All three parameters are C `int`; zero, `INT_MIN`, and `INT_MAX`
belong to the rejection classes above and are exercised by the differential
tests.
