# Configuration surface

This table is derived from all six definitions exported by the C shared
library and every `if`/`switch` branch in `src/lib.c`. There are no Cargo
features and no C preprocessor feature flags, so the only build configuration
is the default one.

For `buffapp`, the source independently selects an operation from `param1 % 4`
and `param3 % 4`, then branches on whether the product of the two intermediate
results is zero. The operation classes below are:

- `A`: add (`param % 4 == 0`)
- `S`: subtract (`param % 4 == 1`)
- `M`: multiply (`param % 4 == 2`)
- `D`: divide (`param % 4 == 3`, divisor nonzero)
- `D0`: divide with a zero divisor
- `U`: unknown (negative remainder, default switch arm)

`NZ` means both intermediates and their product are nonzero. `Z` means at
least one intermediate is zero. `D0` and `U` always produce a zero
intermediate, so their impossible `NZ` combinations are pruned.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `create_buffer` | positive capacity; allocation succeeds | [x] |
| 2 | `create_buffer` | zero capacity boundary (target libc returns a writable non-null allocation) | [x] |
| 3 | `destroy_buffer` | null buffer pointer | [x] |
| 4 | `destroy_buffer` | non-null buffer with non-null data | [x] |
| 5 | `destroy_buffer` | non-null buffer with null data | [x] |
| 6 | `append_to_buffer` | empty string, required capacity below current capacity (no growth) | [x] |
| 7 | `append_to_buffer` | nonempty string, required capacity exactly equals current capacity (no growth) | [x] |
| 8 | `append_to_buffer` | nonempty string, spare capacity remains (no growth) | [x] |
| 9 | `append_to_buffer` | append to empty buffer and required capacity exceeds capacity (growth) | [x] |
| 10 | `append_to_buffer` | append to nonempty buffer and required capacity exceeds capacity (growth) | [x] |
| 11 | `append_to_buffer` | repeated appends cross multiple growth boundaries | [x] |
| 12 | `get_operation_name` | code `0` selects `add` | [x] |
| 13 | `get_operation_name` | code `1` selects `subtract` | [x] |
| 14 | `get_operation_name` | code `2` selects `multiply` | [x] |
| 15 | `get_operation_name` | code `3` selects `divide` | [x] |
| 16 | `get_operation_name` | any other `int` selects `unknown` | [x] |
| 17 | `perform_operation` | `add`, randomized operands | [x] |
| 18 | `perform_operation` | `subtract`, randomized operands | [x] |
| 19 | `perform_operation` | `multiply`, randomized operands | [x] |
| 20 | `perform_operation` | `divide`, nonzero divisor | [x] |
| 21 | `perform_operation` | `divide`, zero divisor | [x] |
| 22 | `perform_operation` | unknown operation string | [x] |
| 23 | `buffapp` | `A x A`, final branch `NZ` | [x] |
| 24 | `buffapp` | `A x S`, final branch `NZ` | [x] |
| 25 | `buffapp` | `A x M`, final branch `NZ` | [x] |
| 26 | `buffapp` | `A x D`, final branch `NZ` | [x] |
| 27 | `buffapp` | `S x A`, final branch `NZ` | [x] |
| 28 | `buffapp` | `S x S`, final branch `NZ` | [x] |
| 29 | `buffapp` | `S x M`, final branch `NZ` | [x] |
| 30 | `buffapp` | `S x D`, final branch `NZ` | [x] |
| 31 | `buffapp` | `M x A`, final branch `NZ` | [x] |
| 32 | `buffapp` | `M x S`, final branch `NZ` | [x] |
| 33 | `buffapp` | `M x M`, final branch `NZ` | [x] |
| 34 | `buffapp` | `M x D`, final branch `NZ` | [x] |
| 35 | `buffapp` | `D x A`, final branch `NZ` | [x] |
| 36 | `buffapp` | `D x S`, final branch `NZ` | [x] |
| 37 | `buffapp` | `D x M`, final branch `NZ` | [x] |
| 38 | `buffapp` | `D x D`, final branch `NZ` | [x] |
| 39 | `buffapp` | `A x A`, final branch `Z` | [x] |
| 40 | `buffapp` | `A x S`, final branch `Z` | [x] |
| 41 | `buffapp` | `A x M`, final branch `Z` | [x] |
| 42 | `buffapp` | `A x D`, final branch `Z` | [x] |
| 43 | `buffapp` | `A x D0`, final branch `Z` | [x] |
| 44 | `buffapp` | `A x U`, final branch `Z` | [x] |
| 45 | `buffapp` | `S x A`, final branch `Z` | [x] |
| 46 | `buffapp` | `S x S`, final branch `Z` | [x] |
| 47 | `buffapp` | `S x M`, final branch `Z` | [x] |
| 48 | `buffapp` | `S x D`, final branch `Z` | [x] |
| 49 | `buffapp` | `S x D0`, final branch `Z` | [x] |
| 50 | `buffapp` | `S x U`, final branch `Z` | [x] |
| 51 | `buffapp` | `M x A`, final branch `Z` | [x] |
| 52 | `buffapp` | `M x S`, final branch `Z` | [x] |
| 53 | `buffapp` | `M x M`, final branch `Z` | [x] |
| 54 | `buffapp` | `M x D`, final branch `Z` | [x] |
| 55 | `buffapp` | `M x D0`, final branch `Z` | [x] |
| 56 | `buffapp` | `M x U`, final branch `Z` | [x] |
| 57 | `buffapp` | `D x A`, final branch `Z` | [x] |
| 58 | `buffapp` | `D x S`, final branch `Z` | [x] |
| 59 | `buffapp` | `D x M`, final branch `Z` | [x] |
| 60 | `buffapp` | `D x D`, final branch `Z` | [x] |
| 61 | `buffapp` | `D x D0`, final branch `Z` | [x] |
| 62 | `buffapp` | `D x U`, final branch `Z` | [x] |
| 63 | `buffapp` | `D0 x A`, final branch `Z` | [x] |
| 64 | `buffapp` | `D0 x S`, final branch `Z` | [x] |
| 65 | `buffapp` | `D0 x M`, final branch `Z` | [x] |
| 66 | `buffapp` | `D0 x D`, final branch `Z` | [x] |
| 67 | `buffapp` | `D0 x D0`, final branch `Z` | [x] |
| 68 | `buffapp` | `D0 x U`, final branch `Z` | [x] |
| 69 | `buffapp` | `U x A`, final branch `Z` | [x] |
| 70 | `buffapp` | `U x S`, final branch `Z` | [x] |
| 71 | `buffapp` | `U x M`, final branch `Z` | [x] |
| 72 | `buffapp` | `U x D`, final branch `Z` | [x] |
| 73 | `buffapp` | `U x D0`, final branch `Z` | [x] |
| 74 | `buffapp` | `U x U`, final branch `Z` | [x] |
