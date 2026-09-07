# Configuration surface

The C library has no compile-time feature flags or runtime option object. Its
configuration axes are the exported entry point, callback selected by
`process_with_foreach`, array count shape, index ordering, and numeric input
shape. The rows below are the pruned cross-product of branches present in
`../c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `add_operation` | randomized four-argument calls; result depends on `a` and `b`, unused arguments varied | [x] |
| 2 | `multiply_operation` | randomized four-argument calls; result depends on `a` and `b`, unused arguments varied | [x] |
| 3 | `subtract_operation` | randomized four-argument calls; result depends on `a` and `b`, unused arguments varied | [x] |
| 4 | `modulo_operation` | `b != 0`, randomized positive/negative dividends and divisors, excluding the C-overflow pair `INT_MIN % -1` | [x] |
| 5 | `safe_double_to_int` | finite values strictly between `INT32_MIN` and `INT32_MAX`, including negative, zero, positive, and fractional values | [x] |
| 6 | `compute_scaled_value` → `safe_double_to_int` | randomized base/scale pairs whose product remains strictly inside the integer range | [x] |
| 7 | `compare_results_in_array` | valid indices with `idx1 < idx2` | [x] |
| 8 | `compare_results_in_array` | valid indices with `idx1 > idx2` | [x] |
| 9 | `compare_results_in_array` | valid indices with `idx1 == idx2` | [x] |
| 10 | `init_result_array` | empty shape, `count == 0` (`count < 10` arm) | [x] |
| 11 | `init_result_array` | singleton shape, `count == 1` (`count < 10` arm) | [x] |
| 12 | `init_result_array` | many-element shape, `2 <= count <= 9` (`count < 10` arm) | [x] |
| 13 | `init_result_array` | exact-capacity shape, `count == 10` (else arm) | [x] |
| 14 | `init_result_array` | over-capacity shape, `count > 10` (else arm, capped to ten) | [x] |
| 15 | `process_with_foreach` + `add_operation` | empty array (`count == 0`) | [x] |
| 16 | `process_with_foreach` + `add_operation` | singleton array (`count == 1`) | [x] |
| 17 | `process_with_foreach` + `add_operation` | many-element array (`2 <= count <= 10`) | [x] |
| 18 | `process_with_foreach` + `multiply_operation` | empty array (`count == 0`) | [x] |
| 19 | `process_with_foreach` + `multiply_operation` | singleton array (`count == 1`) | [x] |
| 20 | `process_with_foreach` + `multiply_operation` | many-element array (`2 <= count <= 10`) | [x] |
| 21 | `process_with_foreach` + `subtract_operation` | empty array (`count == 0`) | [x] |
| 22 | `process_with_foreach` + `subtract_operation` | singleton array (`count == 1`) | [x] |
| 23 | `process_with_foreach` + `subtract_operation` | many-element array (`2 <= count <= 10`) | [x] |
| 24 | `process_with_foreach` + `modulo_operation` | empty array (`count == 0`) | [x] |
| 25 | `process_with_foreach` + `modulo_operation` | singleton array; rank zero selects the callback's `b == 0` branch | [x] |
| 26 | `process_with_foreach` + `modulo_operation` | many-element array; rank zero and nonzero callback-divisor branches both occur | [x] |
| 27 | `compute_weighted_sum` | empty array (`count == 0`) | [x] |
| 28 | `compute_weighted_sum` | singleton array; `current == base`, so weight is `1` | [x] |
| 29 | `compute_weighted_sum` | many-element array; first element uses weight `1`, later elements use pointer distance/index | [x] |
| 30 | `arrayfunc` | fixed eight-element end-to-end pipeline with randomized mixed-sign and boundary integer parameters | [x] |

Feature combinations from `Cargo.toml`: no named features. Both the default
invocation and `--no-default-features` must pass.
