# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or backend preprocessor definitions. The complete valid feature
matrix therefore contains one combination:

| # | Cargo feature set | CMake configuration | status |
|---|-------------------|---------------------|--------|
| B1 | empty set (`--no-default-features --features ''`) | default shared-library build with position-independent code | [x] |

## Runtime State Legend

`buffapp` selects each operation from the C remainder of parameter 1 or 3
modulo four. The following states are the mechanically distinct combination
of that selection, operation-internal branches, and whether the intermediate
result drives the final zero/nonzero branch:

| code | selected branch | paired operand shape | intermediate |
|------|-----------------|----------------------|--------------|
| `AZ` | add (`p % 4 == 0`) | `p + q == 0` | zero |
| `AN` | add (`p % 4 == 0`) | `p + q != 0` | nonzero |
| `SZ` | subtract (`p % 4 == 1`) | `p - q == 0` | zero |
| `SN` | subtract (`p % 4 == 1`) | `p - q != 0` | nonzero |
| `MZ` | multiply (`p % 4 == 2`) | `q == 0` | zero |
| `MN` | multiply (`p % 4 == 2`) | `q != 0` | nonzero |
| `DZ` | divide (`p % 4 == 3`) | `q == 0` | zero via divide-by-zero branch |
| `DQ` | divide (`p % 4 == 3`) | `q != 0` and `p / q == 0` | zero quotient |
| `DN` | divide (`p % 4 == 3`) | `q != 0` and `p / q != 0` | nonzero |
| `UZ` | default (`p % 4` is `-1`, `-2`, or `-3`) | any `q` | zero |

## Direct API Configurations

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `create_buffer` | successful allocation with randomized positive capacity `1..=256` | [x] |
| 2 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | empty C string; required capacity remains within allocation | [x] |
| 3 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | nonempty string with `required_capacity < capacity` | [x] |
| 4 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | nonempty string with `required_capacity == capacity` | [x] |
| 5 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | nonempty string with `required_capacity > capacity`; realloc/growth branch | [x] |
| 6 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | repeated appends: nonempty existing buffer, first append fits, later append grows | [x] |
| 7 | `create_buffer`, `append_to_buffer`, `destroy_buffer` | randomized non-NUL byte strings, including non-ASCII bytes | [x] |
| 8 | `destroy_buffer` | ordinary non-null object with non-null data | [x] |
| 9 | `get_operation_name` | `op_code == 0` returns add | [x] |
| 10 | `get_operation_name` | `op_code == 1` returns subtract | [x] |
| 11 | `get_operation_name` | `op_code == 2` returns multiply | [x] |
| 12 | `get_operation_name` | `op_code == 3` returns divide | [x] |
| 13 | `perform_operation` | add with randomized signed operands | [x] |
| 14 | `perform_operation` | subtract with randomized signed operands | [x] |
| 15 | `perform_operation` | multiply with randomized signed operands | [x] |
| 16 | `perform_operation` | divide with nonzero operands and nonnegative quotient | [x] |
| 17 | `perform_operation` | divide with nonzero operands and C truncation toward zero across sign combinations | [x] |

## Composed `buffapp` Configurations

Each row is one state for operation 1 crossed with one state for operation 2.
Rows containing `DZ`, `DQ`, or `DN` separately exercise the nested divide
condition. The final `intermediate3 != 0` path is taken exactly when both state
codes end in `N`; every other row takes the fallback sum path.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 18 | `buffapp` | operation states `AZ x AZ` | [x] |
| 19 | `buffapp` | operation states `AZ x AN` | [x] |
| 20 | `buffapp` | operation states `AZ x SZ` | [x] |
| 21 | `buffapp` | operation states `AZ x SN` | [x] |
| 22 | `buffapp` | operation states `AZ x MZ` | [x] |
| 23 | `buffapp` | operation states `AZ x MN` | [x] |
| 24 | `buffapp` | operation states `AZ x DZ` | [x] |
| 25 | `buffapp` | operation states `AZ x DQ` | [x] |
| 26 | `buffapp` | operation states `AZ x DN` | [x] |
| 27 | `buffapp` | operation states `AZ x UZ` | [x] |
| 28 | `buffapp` | operation states `AN x AZ` | [x] |
| 29 | `buffapp` | operation states `AN x AN` | [x] |
| 30 | `buffapp` | operation states `AN x SZ` | [x] |
| 31 | `buffapp` | operation states `AN x SN` | [x] |
| 32 | `buffapp` | operation states `AN x MZ` | [x] |
| 33 | `buffapp` | operation states `AN x MN` | [x] |
| 34 | `buffapp` | operation states `AN x DZ` | [x] |
| 35 | `buffapp` | operation states `AN x DQ` | [x] |
| 36 | `buffapp` | operation states `AN x DN` | [x] |
| 37 | `buffapp` | operation states `AN x UZ` | [x] |
| 38 | `buffapp` | operation states `SZ x AZ` | [x] |
| 39 | `buffapp` | operation states `SZ x AN` | [x] |
| 40 | `buffapp` | operation states `SZ x SZ` | [x] |
| 41 | `buffapp` | operation states `SZ x SN` | [x] |
| 42 | `buffapp` | operation states `SZ x MZ` | [x] |
| 43 | `buffapp` | operation states `SZ x MN` | [x] |
| 44 | `buffapp` | operation states `SZ x DZ` | [x] |
| 45 | `buffapp` | operation states `SZ x DQ` | [x] |
| 46 | `buffapp` | operation states `SZ x DN` | [x] |
| 47 | `buffapp` | operation states `SZ x UZ` | [x] |
| 48 | `buffapp` | operation states `SN x AZ` | [x] |
| 49 | `buffapp` | operation states `SN x AN` | [x] |
| 50 | `buffapp` | operation states `SN x SZ` | [x] |
| 51 | `buffapp` | operation states `SN x SN` | [x] |
| 52 | `buffapp` | operation states `SN x MZ` | [x] |
| 53 | `buffapp` | operation states `SN x MN` | [x] |
| 54 | `buffapp` | operation states `SN x DZ` | [x] |
| 55 | `buffapp` | operation states `SN x DQ` | [x] |
| 56 | `buffapp` | operation states `SN x DN` | [x] |
| 57 | `buffapp` | operation states `SN x UZ` | [x] |
| 58 | `buffapp` | operation states `MZ x AZ` | [x] |
| 59 | `buffapp` | operation states `MZ x AN` | [x] |
| 60 | `buffapp` | operation states `MZ x SZ` | [x] |
| 61 | `buffapp` | operation states `MZ x SN` | [x] |
| 62 | `buffapp` | operation states `MZ x MZ` | [x] |
| 63 | `buffapp` | operation states `MZ x MN` | [x] |
| 64 | `buffapp` | operation states `MZ x DZ` | [x] |
| 65 | `buffapp` | operation states `MZ x DQ` | [x] |
| 66 | `buffapp` | operation states `MZ x DN` | [x] |
| 67 | `buffapp` | operation states `MZ x UZ` | [x] |
| 68 | `buffapp` | operation states `MN x AZ` | [x] |
| 69 | `buffapp` | operation states `MN x AN` | [x] |
| 70 | `buffapp` | operation states `MN x SZ` | [x] |
| 71 | `buffapp` | operation states `MN x SN` | [x] |
| 72 | `buffapp` | operation states `MN x MZ` | [x] |
| 73 | `buffapp` | operation states `MN x MN` | [x] |
| 74 | `buffapp` | operation states `MN x DZ` | [x] |
| 75 | `buffapp` | operation states `MN x DQ` | [x] |
| 76 | `buffapp` | operation states `MN x DN` | [x] |
| 77 | `buffapp` | operation states `MN x UZ` | [x] |
| 78 | `buffapp` | operation states `DZ x AZ` | [x] |
| 79 | `buffapp` | operation states `DZ x AN` | [x] |
| 80 | `buffapp` | operation states `DZ x SZ` | [x] |
| 81 | `buffapp` | operation states `DZ x SN` | [x] |
| 82 | `buffapp` | operation states `DZ x MZ` | [x] |
| 83 | `buffapp` | operation states `DZ x MN` | [x] |
| 84 | `buffapp` | operation states `DZ x DZ` | [x] |
| 85 | `buffapp` | operation states `DZ x DQ` | [x] |
| 86 | `buffapp` | operation states `DZ x DN` | [x] |
| 87 | `buffapp` | operation states `DZ x UZ` | [x] |
| 88 | `buffapp` | operation states `DQ x AZ` | [x] |
| 89 | `buffapp` | operation states `DQ x AN` | [x] |
| 90 | `buffapp` | operation states `DQ x SZ` | [x] |
| 91 | `buffapp` | operation states `DQ x SN` | [x] |
| 92 | `buffapp` | operation states `DQ x MZ` | [x] |
| 93 | `buffapp` | operation states `DQ x MN` | [x] |
| 94 | `buffapp` | operation states `DQ x DZ` | [x] |
| 95 | `buffapp` | operation states `DQ x DQ` | [x] |
| 96 | `buffapp` | operation states `DQ x DN` | [x] |
| 97 | `buffapp` | operation states `DQ x UZ` | [x] |
| 98 | `buffapp` | operation states `DN x AZ` | [x] |
| 99 | `buffapp` | operation states `DN x AN` | [x] |
| 100 | `buffapp` | operation states `DN x SZ` | [x] |
| 101 | `buffapp` | operation states `DN x SN` | [x] |
| 102 | `buffapp` | operation states `DN x MZ` | [x] |
| 103 | `buffapp` | operation states `DN x MN` | [x] |
| 104 | `buffapp` | operation states `DN x DZ` | [x] |
| 105 | `buffapp` | operation states `DN x DQ` | [x] |
| 106 | `buffapp` | operation states `DN x DN` | [x] |
| 107 | `buffapp` | operation states `DN x UZ` | [x] |
| 108 | `buffapp` | operation states `UZ x AZ` | [x] |
| 109 | `buffapp` | operation states `UZ x AN` | [x] |
| 110 | `buffapp` | operation states `UZ x SZ` | [x] |
| 111 | `buffapp` | operation states `UZ x SN` | [x] |
| 112 | `buffapp` | operation states `UZ x MZ` | [x] |
| 113 | `buffapp` | operation states `UZ x MN` | [x] |
| 114 | `buffapp` | operation states `UZ x DZ` | [x] |
| 115 | `buffapp` | operation states `UZ x DQ` | [x] |
| 116 | `buffapp` | operation states `UZ x DN` | [x] |
| 117 | `buffapp` | operation states `UZ x UZ` | [x] |
