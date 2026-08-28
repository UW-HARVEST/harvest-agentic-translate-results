# Configuration Surface

There are no Cargo features, C preprocessor configuration flags, runtime
options, variable lengths, element-type choices, byte-order choices, or format
choices. `DataBlock` has one fixed C layout and is copied as 40 raw bytes.

For `overunder`, the final row is a generated cross-product rather than a
single example. Its test corpus covers every feasible combination of these
C-observed axes:

- `a % 6`: switch cases `0` through `5`, plus the negative-remainder default.
- `(double)a * 1.5`: below `INT_MIN`, in range, or above `INT_MAX`.
- `(double)b * 2.7`: below `INT_MIN`, in range, or above `INT_MAX`.
- `d * d + a * a` as produced by the built C object: nonnegative (finite
  `sqrt`) or negative (`sqrt` returns NaN).

The matrix is pruned only where a combination cannot be represented by C
`int` inputs. Each feasible cell receives fixed boundary cases and randomized
cases.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|-------------------------------------------|--------|
| 1 | `safe_double_to_int` | Finite input in inclusive `INT_MIN..=INT_MAX`; cover exact endpoints, negative/positive fractions, signed zero, subnormals, and ordinary integers | [x] |
| 2 | `process_with_fallthrough` | `code == 0`; arbitrary `base_value`, result forced to zero | [x] |
| 3 | `process_with_fallthrough` | `code == 1`; arbitrary `base_value`, add 10 | [x] |
| 4 | `process_with_fallthrough` | `code == 2`; arbitrary `base_value`, fall through and add 20 + 10 | [x] |
| 5 | `process_with_fallthrough` | `code == 3`; arbitrary `base_value`, fall through and add 30 + 20 + 10 | [x] |
| 6 | `process_with_fallthrough` | `code == 4`; arbitrary `base_value`, fall through and add 40 + 30 + 20 + 10 | [x] |
| 7 | `process_with_fallthrough` | `code == 5`; arbitrary `base_value`, fall through and add 50 + 40 + 30 + 20 + 10 | [x] |
| 8 | `copy_data_block` | Distinct valid source/destination blocks; randomize all 40 bytes, including struct padding, all label bytes, embedded NUL bytes, floating-point bit patterns, and scalar boundaries | [x] |
| 9 | `handle_pointer_operations` | Full `int` domain, including values whose C arithmetic wraps in the built shared object | [x] |
| 10 | `overunder` | Full feasible cross-product matrix described above; randomize `c` over the full `int` domain and include arithmetic boundaries for all four arguments; compare return value and emitted bytes | [x] |

All five dynamic entry points are listed, including the four functions omitted
from the minimal public header but exported by the C shared object.
