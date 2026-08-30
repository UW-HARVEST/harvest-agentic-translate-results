# Error Surface

Mechanically derived from every `if` rejection branch and nonzero internal
result in `../c_src/src/driver.c`. The only public function returns `void`, so
the expected C result is its exact stdout byte sequence.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `driver` / `multi_stage` | `x != 1` | `Error: x != 1\nOperation failed\nResult: 1\n` | [x] |
| 2 | `driver` / `multi_stage` | `x == 1 && local_y != 2` (therefore static `y != 2`) | `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n` | [x] |
| 3 | `driver` / `multi_stage` | `x == 1 && local_y == 2 && z != 3` | `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n` | [x] |

There are no pointer, length, enum, assertion, error-macro, explicit range, or
min/max checks in the public API. All three parameters are by-value C `int`s.

All rows pass in `tests/differential.rs` under both the default build and
`--no-default-features`.
