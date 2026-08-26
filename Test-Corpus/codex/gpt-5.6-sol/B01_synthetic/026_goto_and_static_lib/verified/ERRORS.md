# Error Surface

This table is derived from every rejection branch in
`c_src/src/driver.c:33-49`. The public function returns `void`, so the
observable C result is its exact stdout byte sequence.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `driver` | `x != 1`; `local_y` and `z` are not inspected by `multi_stage` | `Error: x != 1\nOperation failed\nResult: 1\n` | [x] |
| 2 | `driver` | `x == 1 && local_y != 2`; `z` is not inspected by `multi_stage` | `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n` | [x] |
| 3 | `driver` | `x == 1 && local_y == 2 && z != 3` | `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n` | [x] |

Mechanical review also found no pointer, length, enum, assertion, explicit
range, or min/max validation in the public API. Consequently null pointers,
zero/oversized lengths, invalid enum discriminants, and one-past-range values
are not applicable. The `int` boundary values remain valid ABI inputs and are
covered in the differential tests.
