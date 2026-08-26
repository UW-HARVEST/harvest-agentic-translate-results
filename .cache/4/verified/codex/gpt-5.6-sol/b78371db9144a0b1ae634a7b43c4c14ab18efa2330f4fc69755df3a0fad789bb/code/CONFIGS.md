# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional definitions. There is exactly one valid build-time
configuration: `--no-default-features` with no features enabled.

The runtime rows below are derived from all three symbols in the C dynamic
symbol table and every data-dependent `if`, `for`, and `switch` branch. The
four `cleanup` arguments are processed identically and additively, so position
cross-products that differ only by permutation are represented by randomized
placement; the mixed row covers interactions among multiple case classes.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `cleanup_resources` | null pointer (the false side of `if (dynamic_str)`) | [x] |
| 2 | `cleanup_resources` | non-null allocation from the process C allocator (the true side of `if (dynamic_str)`) | [x] |
| 3 | `print_result` | empty, NUL-terminated label and arbitrary `int` result | [x] |
| 4 | `print_result` | non-empty, NUL-terminated labels of one/many bytes and arbitrary `int` results | [x] |
| 5 | `cleanup` | exactly one argument is `10`, randomized position; other arguments use the default switch arm | [x] |
| 6 | `cleanup` | exactly one argument is `20`, randomized position; other arguments use the default switch arm | [x] |
| 7 | `cleanup` | exactly one argument is `30`, randomized position; other arguments use the default switch arm | [x] |
| 8 | `cleanup` | exactly one argument is `40`, randomized position; other arguments use the default switch arm | [x] |
| 9 | `cleanup` | all arguments use the default switch arm, including negative, zero, and positive values | [x] |
| 10 | `cleanup` | multiple recognized switch values in randomized combinations and positions | [x] |
| 11 | `cleanup` | `INT_MIN`/`INT_MAX` appears once with remaining values chosen so C's cumulative signed addition remains in range | [x] |

Every `cleanup` row also validates the fixed four-element loop, successful
50-byte allocation, macro stringization to `"numbers"`, formatted output, and
cleanup of the allocation.
