# Configuration surface

The crate declares no Cargo features, and the C API exposes no runtime options,
modes, flags, element types, formats, lengths, or byte-order controls.

For `cleanup`, each of four operands independently takes one of five switch
classes: default, `10`, `20`, `30`, or `40`. The switch is identical at every
array position and accumulation is additive, so the mechanically distinct
cross-product is represented by the 70 multisets of four classes below. Every
row randomizes operand order and every default-class value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `cleanup` | 4×default; randomized operand permutations and randomized non-special default values | [x] |
| 2 | `cleanup` | 3×default, 1×10; randomized operand permutations and randomized non-special default values | [x] |
| 3 | `cleanup` | 3×default, 1×20; randomized operand permutations and randomized non-special default values | [x] |
| 4 | `cleanup` | 3×default, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 5 | `cleanup` | 3×default, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 6 | `cleanup` | 2×default, 2×10; randomized operand permutations and randomized non-special default values | [x] |
| 7 | `cleanup` | 2×default, 1×10, 1×20; randomized operand permutations and randomized non-special default values | [x] |
| 8 | `cleanup` | 2×default, 1×10, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 9 | `cleanup` | 2×default, 1×10, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 10 | `cleanup` | 2×default, 2×20; randomized operand permutations and randomized non-special default values | [x] |
| 11 | `cleanup` | 2×default, 1×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 12 | `cleanup` | 2×default, 1×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 13 | `cleanup` | 2×default, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 14 | `cleanup` | 2×default, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 15 | `cleanup` | 2×default, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 16 | `cleanup` | 1×default, 3×10; randomized operand permutations and randomized non-special default values | [x] |
| 17 | `cleanup` | 1×default, 2×10, 1×20; randomized operand permutations and randomized non-special default values | [x] |
| 18 | `cleanup` | 1×default, 2×10, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 19 | `cleanup` | 1×default, 2×10, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 20 | `cleanup` | 1×default, 1×10, 2×20; randomized operand permutations and randomized non-special default values | [x] |
| 21 | `cleanup` | 1×default, 1×10, 1×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 22 | `cleanup` | 1×default, 1×10, 1×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 23 | `cleanup` | 1×default, 1×10, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 24 | `cleanup` | 1×default, 1×10, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 25 | `cleanup` | 1×default, 1×10, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 26 | `cleanup` | 1×default, 3×20; randomized operand permutations and randomized non-special default values | [x] |
| 27 | `cleanup` | 1×default, 2×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 28 | `cleanup` | 1×default, 2×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 29 | `cleanup` | 1×default, 1×20, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 30 | `cleanup` | 1×default, 1×20, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 31 | `cleanup` | 1×default, 1×20, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 32 | `cleanup` | 1×default, 3×30; randomized operand permutations and randomized non-special default values | [x] |
| 33 | `cleanup` | 1×default, 2×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 34 | `cleanup` | 1×default, 1×30, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 35 | `cleanup` | 1×default, 3×40; randomized operand permutations and randomized non-special default values | [x] |
| 36 | `cleanup` | 4×10; randomized operand permutations and randomized non-special default values | [x] |
| 37 | `cleanup` | 3×10, 1×20; randomized operand permutations and randomized non-special default values | [x] |
| 38 | `cleanup` | 3×10, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 39 | `cleanup` | 3×10, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 40 | `cleanup` | 2×10, 2×20; randomized operand permutations and randomized non-special default values | [x] |
| 41 | `cleanup` | 2×10, 1×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 42 | `cleanup` | 2×10, 1×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 43 | `cleanup` | 2×10, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 44 | `cleanup` | 2×10, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 45 | `cleanup` | 2×10, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 46 | `cleanup` | 1×10, 3×20; randomized operand permutations and randomized non-special default values | [x] |
| 47 | `cleanup` | 1×10, 2×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 48 | `cleanup` | 1×10, 2×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 49 | `cleanup` | 1×10, 1×20, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 50 | `cleanup` | 1×10, 1×20, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 51 | `cleanup` | 1×10, 1×20, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 52 | `cleanup` | 1×10, 3×30; randomized operand permutations and randomized non-special default values | [x] |
| 53 | `cleanup` | 1×10, 2×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 54 | `cleanup` | 1×10, 1×30, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 55 | `cleanup` | 1×10, 3×40; randomized operand permutations and randomized non-special default values | [x] |
| 56 | `cleanup` | 4×20; randomized operand permutations and randomized non-special default values | [x] |
| 57 | `cleanup` | 3×20, 1×30; randomized operand permutations and randomized non-special default values | [x] |
| 58 | `cleanup` | 3×20, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 59 | `cleanup` | 2×20, 2×30; randomized operand permutations and randomized non-special default values | [x] |
| 60 | `cleanup` | 2×20, 1×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 61 | `cleanup` | 2×20, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 62 | `cleanup` | 1×20, 3×30; randomized operand permutations and randomized non-special default values | [x] |
| 63 | `cleanup` | 1×20, 2×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 64 | `cleanup` | 1×20, 1×30, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 65 | `cleanup` | 1×20, 3×40; randomized operand permutations and randomized non-special default values | [x] |
| 66 | `cleanup` | 4×30; randomized operand permutations and randomized non-special default values | [x] |
| 67 | `cleanup` | 3×30, 1×40; randomized operand permutations and randomized non-special default values | [x] |
| 68 | `cleanup` | 2×30, 2×40; randomized operand permutations and randomized non-special default values | [x] |
| 69 | `cleanup` | 1×30, 3×40; randomized operand permutations and randomized non-special default values | [x] |
| 70 | `cleanup` | 4×40; randomized operand permutations and randomized non-special default values | [x] |
| 71 | `print_result` | non-null NUL-terminated label: empty, ASCII, percent-containing, and arbitrary non-NUL bytes × result zero, positive, negative, `INT_MIN`, and `INT_MAX` | [x] |
| 72 | `cleanup_resources` | `dynamic_str == NULL` | [x] |
| 73 | `cleanup_resources` | non-null pointer returned by libc `malloc` | [x] |

`cleanup` also receives explicit zero, negative, positive, `INT_MIN`, and
`INT_MAX` default-class values in boundary tests. The source has no executable
driver target.
