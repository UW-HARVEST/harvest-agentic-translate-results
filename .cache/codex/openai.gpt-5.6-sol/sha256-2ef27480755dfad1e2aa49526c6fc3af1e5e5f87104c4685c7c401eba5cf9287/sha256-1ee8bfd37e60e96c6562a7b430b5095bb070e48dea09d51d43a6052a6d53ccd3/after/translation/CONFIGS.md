# Configuration Surface

The crate declares no Cargo features, and the C API has no runtime option
structure, mode enum, byte-order flag, format, or element-type selector. The
configuration axes below come from the eleven exported entry points and the
branches on divisor, floating-point class, index ordering, array count,
callback, and pointer ordering.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `add_operation` | ordinary signed operands; unused arguments varied; no signed overflow | [x] |
| C2 | `multiply_operation` | ordinary signed operands; unused arguments varied; no signed overflow | [x] |
| C3 | `subtract_operation` | ordinary signed operands; unused arguments varied; no signed overflow | [x] |
| C4 | `modulo_operation` | nonzero positive divisor; mixed-sign dividend | [x] |
| C5 | `modulo_operation` | nonzero negative divisor; dividend excludes `INT32_MIN` when divisor is `-1` | [x] |
| C6 | `safe_double_to_int` | finite in-range values: negative, zero, positive, and fractional | [x] |
| C7 | `safe_double_to_int` | upper saturation boundary: exact/above `INT32_MAX` and positive infinity | [x] |
| C8 | `safe_double_to_int` | lower saturation boundary: exact/below `INT32_MIN` and negative infinity | [x] |
| C9 | `safe_double_to_int` | NaN | [x] |
| C10 | `compute_scaled_value` | finite product strictly inside integer range, including negative/zero/fractional scales | [x] |
| C11 | `compute_scaled_value` | product reaches upper saturation, including positive infinity | [x] |
| C12 | `compute_scaled_value` | product reaches lower saturation, including negative infinity | [x] |
| C13 | `compute_scaled_value` | product is NaN | [x] |
| C14 | `compare_results_in_array` | both indices valid and `idx1 < idx2` | [x] |
| C15 | `compare_results_in_array` | both indices valid and `idx1 == idx2` | [x] |
| C16 | `compare_results_in_array` | both indices valid and `idx1 > idx2` | [x] |
| C17 | `compare_results_in_array` | first index at/above count | [x] |
| C18 | `compare_results_in_array` | second index at/above count, first valid | [x] |
| C19 | `init_result_array` | negative count: stored unchanged; no values read | [x] |
| C20 | `init_result_array` | zero count, including null `values`: no values read | [x] |
| C21 | `init_result_array` | one element | [x] |
| C22 | `init_result_array` | many elements with `1 < count < 10` | [x] |
| C23 | `init_result_array` | exact capacity, `count == 10` | [x] |
| C24 | `init_result_array` | oversized request, `count > 10`: capped at ten | [x] |
| C25 | `process_with_foreach` + `add_operation` | empty array | [x] |
| C26 | `process_with_foreach` + `add_operation` | one element | [x] |
| C27 | `process_with_foreach` + `add_operation` | many elements with varied values/ranks | [x] |
| C28 | `process_with_foreach` + `multiply_operation` | empty array | [x] |
| C29 | `process_with_foreach` + `multiply_operation` | one element | [x] |
| C30 | `process_with_foreach` + `multiply_operation` | many elements with varied values/ranks | [x] |
| C31 | `process_with_foreach` + `subtract_operation` | empty array | [x] |
| C32 | `process_with_foreach` + `subtract_operation` | one element | [x] |
| C33 | `process_with_foreach` + `subtract_operation` | many elements with varied values/ranks | [x] |
| C34 | `process_with_foreach` + `modulo_operation` | empty array | [x] |
| C35 | `process_with_foreach` + `modulo_operation` | one element, rank zero triggers zero-divisor branch | [x] |
| C36 | `process_with_foreach` + `modulo_operation` | many elements; rank zero then nonzero ranks | [x] |
| C37 | `compute_weighted_sum` | negative count: loop does not execute | [x] |
| C38 | `compute_weighted_sum` | empty array | [x] |
| C39 | `compute_weighted_sum` | one element: `current == base`, weight one | [x] |
| C40 | `compute_weighted_sum` | many elements: base uses weight one, later elements use pointer-distance weights | [x] |
| C41 | `compute_weighted_sum` | values whose weighted products saturate high/low | [x] |
| C42 | `arrayfunc` | randomized ordinary four-parameter end-to-end inputs, avoiding C signed-overflow cases | [x] |
| C43 | `arrayfunc` | zero and mixed-sign inputs, including modulo divisors that become zero | [x] |
| C44 | `arrayfunc` | large-magnitude inputs that remain defined through all C arithmetic | [x] |
