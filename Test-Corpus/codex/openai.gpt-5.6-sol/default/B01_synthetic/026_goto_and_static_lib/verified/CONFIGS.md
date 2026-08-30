# Configuration Surface

Mechanically derived from the public header and the ordered `if` branches in
`../c_src/src/driver.c`. There are no runtime options, compile-time features,
pointer/length inputs, element types, formats, or byte-order modes. The full
public entry-point set is `{ driver }`; `multi_stage` is `static` and is not a
public entry point.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `driver` | scalar C `int`s; `x != 1`; `local_y` and `z` arbitrary and not inspected by `multi_stage` | [x] |
| 2 | `driver` | scalar C `int`s; `x == 1`, `local_y != 2`; `z` arbitrary and not inspected | [x] |
| 3 | `driver` | scalar C `int`s; `x == 1`, `local_y == 2`, `z != 3` | [x] |
| 4 | `driver` | scalar C `int`s; `x == 1`, `local_y == 2`, `z == 3` | [x] |

These four rows are the complete branch-pruned cross-product of the three
ordered equality checks. All rows pass in `tests/differential.rs` under both
the default build and `--no-default-features`.
