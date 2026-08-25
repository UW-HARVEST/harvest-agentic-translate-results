# Configuration Surface

The CMake project has one unconditional shared-library target. `Cargo.toml`
contains no `[features]` table, and the C source has no conditional-compilation
branches. Therefore there is one build-time combination: the empty feature set
(`cargo ... --no-default-features`).

Runtime rows below are derived from all 11 C dynamic exports, the four callback
choices in `operations[]`, consumer-supplied callbacks, the branches in the
conversion/comparison helpers, and the loop/count shapes `0`, `1`, `2..9`, and
the fixed capacity `10`.
Negative counts are included only where the C loop condition treats them as
zero iterations. `process_with_foreach` uses `count_iter != size`, so a
negative count runs beyond the array and is outside defined C behavior. Inputs
that cause C signed overflow, invalid pointer arithmetic, or null dereferences
are likewise excluded.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `add_operation` | randomized integer operands in the defined, non-overflowing sum domain; ignored arguments varied | [x] |
| 2 | `multiply_operation` | randomized integer operands in the defined, non-overflowing product domain; ignored arguments varied | [x] |
| 3 | `subtract_operation` | randomized integer operands in the defined, non-overflowing difference domain; ignored arguments varied | [x] |
| 4 | `modulo_operation` | randomized nonzero positive and negative divisors, excluding `INT_MIN / -1`; ignored arguments varied | [x] |
| 5 | `safe_double_to_int` | finite values strictly inside `(INT32_MIN, INT32_MAX)`, including signs, zero, and fractions | [x] |
| 6 | `compute_scaled_value` | product is finite and strictly inside the integer limits | [x] |
| 7 | `compute_scaled_value` | product reaches/exceeds `INT32_MAX` or positive infinity | [x] |
| 8 | `compute_scaled_value` | product reaches/falls below `INT32_MIN` or negative infinity | [x] |
| 9 | `compute_scaled_value` | product is NaN | [x] |
| 10 | `compare_results_in_array` | valid equal indices (`idx1 == idx2`) | [x] |
| 11 | `compare_results_in_array` | valid ascending indices (`idx1 < idx2`) | [x] |
| 12 | `compare_results_in_array` | valid descending indices (`idx1 > idx2`) | [x] |
| 13 | `init_result_array` | negative `count`; stores it and initializes no elements | [x] |
| 14 | `init_result_array` | empty input (`count == 0`) | [x] |
| 15 | `init_result_array` | singleton input (`count == 1`) | [x] |
| 16 | `init_result_array` | multi-element input (`2 <= count < 10`) | [x] |
| 17 | `init_result_array` | exact-capacity input (`count == 10`) | [x] |
| 18 | `init_result_array` | oversized input (`count > 10`) capped to 10 | [x] |
| 19 | `process_with_foreach` | zero count; each of four operation callbacks; zero iterations, including a null callback | [x] |
| 20 | `process_with_foreach` | singleton count; each of four operation callbacks (modulo takes its rank-zero divisor branch) | [x] |
| 21 | `process_with_foreach` | multi-element count `2..9`; each of four operation callbacks | [x] |
| 22 | `process_with_foreach` | exact-capacity count `10`; each of four operation callbacks | [x] |
| 23 | `process_with_foreach` | consumer-supplied callback with singleton, multi-element, and exact-capacity counts | [x] |
| 24 | `compute_weighted_sum` | negative or zero count; zero iterations | [x] |
| 25 | `compute_weighted_sum` | singleton count; first-element weight branch (`weight = 1`) | [x] |
| 26 | `compute_weighted_sum` | multi-element count `2..9`; first and pointer-distance weight branches | [x] |
| 27 | `compute_weighted_sum` | exact-capacity count `10` | [x] |
| 28 | `arrayfunc` | randomized four-argument end-to-end pipeline through all callbacks, comparison orders, and final scaling | [x] |

## Completion

- [x] Every row passes byte-for-byte differential tests across randomized inputs.
