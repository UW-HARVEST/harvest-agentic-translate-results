# Configuration Surface

The rows below come from the exported entry points and branches in
`c_src/src/lib.c`. `S` is the first operation selected by
`(param3 % 5) + 1`; `T` is the second operation selected by
`((param4 + 1) % 5) + 1`. C remainder semantics make both ranges
`{-3,-2,-1,0,1,2,3,4,5}`. Every `mathop` matrix row also exercises both
validation-character branches and the initial, partially filled, and full
static-history states.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| C1 | `is_valid_operation` | valid characters `'1'` through `'5'` | [x] |
| C2 | `get_operation_priority` | valid operations 1 through 5 and arbitrary signed operation values | [x] |
| C3 | `add_operation` | signed operands: zero, mixed signs, boundaries, and wrapping overflow | [x] |
| C4 | `multiply_operation` | signed operands: zero, mixed signs, boundaries, and wrapping overflow | [x] |
| C5 | `subtract_operation` | signed operands: zero, mixed signs, boundaries, and wrapping overflow | [x] |
| C6 | `divide_operation` | nonzero positive divisor; positive dividend | [x] |
| C7 | `divide_operation` | nonzero positive divisor; negative dividend | [x] |
| C8 | `divide_operation` | negative divisor; positive dividend | [x] |
| C9 | `divide_operation` | negative divisor; negative dividend, excluding `INT_MIN / -1` | [x] |
| C10 | `modulo_operation` | nonzero positive divisor; positive dividend | [x] |
| C11 | `modulo_operation` | nonzero positive divisor; negative dividend | [x] |
| C12 | `modulo_operation` | negative divisor; positive dividend | [x] |
| C13 | `modulo_operation` | negative divisor; negative dividend, excluding `INT_MIN % -1` | [x] |
| C14 | `select_operation` | `OP_ADD` (1), invoke returned function | [x] |
| C15 | `select_operation` | `OP_MULTIPLY` (2), invoke returned function | [x] |
| C16 | `select_operation` | `OP_SUBTRACT` (3), invoke returned function | [x] |
| C17 | `select_operation` | `OP_DIVIDE` (4), invoke returned function | [x] |
| C18 | `select_operation` | `OP_MODULO` (5), invoke returned function | [x] |
| C19 | `select_operation` | out-of-range operation, default to addition | [x] |
| C20 | `get_computation_timestamp` | no-input clock query and right shift by 29 | [x] |
| C21 | `allocate_results` | count 0 | [x] |
| C22 | `allocate_results` | count 1 | [x] |
| C23 | `allocate_results` | count greater than 1 | [x] |
| C24 | `perform_computation_with_history` | add; null history (allocate) | [x] |
| C25 | `perform_computation_with_history` | add; existing history below capacity | [x] |
| C26 | `perform_computation_with_history` | add; existing history at/above capacity | [x] |
| C27 | `perform_computation_with_history` | multiply; null history (allocate) | [x] |
| C28 | `perform_computation_with_history` | multiply; existing history below capacity | [x] |
| C29 | `perform_computation_with_history` | multiply; existing history at/above capacity | [x] |
| C30 | `perform_computation_with_history` | subtract; null history (allocate) | [x] |
| C31 | `perform_computation_with_history` | subtract; existing history below capacity | [x] |
| C32 | `perform_computation_with_history` | subtract; existing history at/above capacity | [x] |
| C33 | `perform_computation_with_history` | divide; null history (allocate) | [x] |
| C34 | `perform_computation_with_history` | divide; existing history below capacity | [x] |
| C35 | `perform_computation_with_history` | divide; existing history at/above capacity | [x] |
| C36 | `perform_computation_with_history` | modulo; null history (allocate) | [x] |
| C37 | `perform_computation_with_history` | modulo; existing history below capacity | [x] |
| C38 | `perform_computation_with_history` | modulo; existing history at/above capacity | [x] |
| C39 | `perform_computation_with_history` | out-of-range operation (default add); null history | [x] |
| C40 | `perform_computation_with_history` | out-of-range operation (default add); existing history below capacity | [x] |
| C41 | `perform_computation_with_history` | out-of-range operation (default add); existing history at/above capacity | [x] |
| M-3,-3 | `mathop` | `S=-3`, `T=-3` | [x] |
| M-3,-2 | `mathop` | `S=-3`, `T=-2` | [x] |
| M-3,-1 | `mathop` | `S=-3`, `T=-1` | [x] |
| M-3,0 | `mathop` | `S=-3`, `T=0` | [x] |
| M-3,1 | `mathop` | `S=-3`, `T=1` | [x] |
| M-3,2 | `mathop` | `S=-3`, `T=2` | [x] |
| M-3,3 | `mathop` | `S=-3`, `T=3` | [x] |
| M-3,4 | `mathop` | `S=-3`, `T=4` | [x] |
| M-3,5 | `mathop` | `S=-3`, `T=5` | [x] |
| M-2,-3 | `mathop` | `S=-2`, `T=-3` | [x] |
| M-2,-2 | `mathop` | `S=-2`, `T=-2` | [x] |
| M-2,-1 | `mathop` | `S=-2`, `T=-1` | [x] |
| M-2,0 | `mathop` | `S=-2`, `T=0` | [x] |
| M-2,1 | `mathop` | `S=-2`, `T=1` | [x] |
| M-2,2 | `mathop` | `S=-2`, `T=2` | [x] |
| M-2,3 | `mathop` | `S=-2`, `T=3` | [x] |
| M-2,4 | `mathop` | `S=-2`, `T=4` | [x] |
| M-2,5 | `mathop` | `S=-2`, `T=5` | [x] |
| M-1,-3 | `mathop` | `S=-1`, `T=-3` | [x] |
| M-1,-2 | `mathop` | `S=-1`, `T=-2` | [x] |
| M-1,-1 | `mathop` | `S=-1`, `T=-1` | [x] |
| M-1,0 | `mathop` | `S=-1`, `T=0` | [x] |
| M-1,1 | `mathop` | `S=-1`, `T=1` | [x] |
| M-1,2 | `mathop` | `S=-1`, `T=2` | [x] |
| M-1,3 | `mathop` | `S=-1`, `T=3` | [x] |
| M-1,4 | `mathop` | `S=-1`, `T=4` | [x] |
| M-1,5 | `mathop` | `S=-1`, `T=5` | [x] |
| M0,-3 | `mathop` | `S=0`, `T=-3` | [x] |
| M0,-2 | `mathop` | `S=0`, `T=-2` | [x] |
| M0,-1 | `mathop` | `S=0`, `T=-1` | [x] |
| M0,0 | `mathop` | `S=0`, `T=0` | [x] |
| M0,1 | `mathop` | `S=0`, `T=1` | [x] |
| M0,2 | `mathop` | `S=0`, `T=2` | [x] |
| M0,3 | `mathop` | `S=0`, `T=3` | [x] |
| M0,4 | `mathop` | `S=0`, `T=4` | [x] |
| M0,5 | `mathop` | `S=0`, `T=5` | [x] |
| M1,-3 | `mathop` | `S=1`, `T=-3` | [x] |
| M1,-2 | `mathop` | `S=1`, `T=-2` | [x] |
| M1,-1 | `mathop` | `S=1`, `T=-1` | [x] |
| M1,0 | `mathop` | `S=1`, `T=0` | [x] |
| M1,1 | `mathop` | `S=1`, `T=1` | [x] |
| M1,2 | `mathop` | `S=1`, `T=2` | [x] |
| M1,3 | `mathop` | `S=1`, `T=3` | [x] |
| M1,4 | `mathop` | `S=1`, `T=4` | [x] |
| M1,5 | `mathop` | `S=1`, `T=5` | [x] |
| M2,-3 | `mathop` | `S=2`, `T=-3` | [x] |
| M2,-2 | `mathop` | `S=2`, `T=-2` | [x] |
| M2,-1 | `mathop` | `S=2`, `T=-1` | [x] |
| M2,0 | `mathop` | `S=2`, `T=0` | [x] |
| M2,1 | `mathop` | `S=2`, `T=1` | [x] |
| M2,2 | `mathop` | `S=2`, `T=2` | [x] |
| M2,3 | `mathop` | `S=2`, `T=3` | [x] |
| M2,4 | `mathop` | `S=2`, `T=4` | [x] |
| M2,5 | `mathop` | `S=2`, `T=5` | [x] |
| M3,-3 | `mathop` | `S=3`, `T=-3` | [x] |
| M3,-2 | `mathop` | `S=3`, `T=-2` | [x] |
| M3,-1 | `mathop` | `S=3`, `T=-1` | [x] |
| M3,0 | `mathop` | `S=3`, `T=0` | [x] |
| M3,1 | `mathop` | `S=3`, `T=1` | [x] |
| M3,2 | `mathop` | `S=3`, `T=2` | [x] |
| M3,3 | `mathop` | `S=3`, `T=3` | [x] |
| M3,4 | `mathop` | `S=3`, `T=4` | [x] |
| M3,5 | `mathop` | `S=3`, `T=5` | [x] |
| M4,-3 | `mathop` | `S=4`, `T=-3` | [x] |
| M4,-2 | `mathop` | `S=4`, `T=-2` | [x] |
| M4,-1 | `mathop` | `S=4`, `T=-1` | [x] |
| M4,0 | `mathop` | `S=4`, `T=0` | [x] |
| M4,1 | `mathop` | `S=4`, `T=1` | [x] |
| M4,2 | `mathop` | `S=4`, `T=2` | [x] |
| M4,3 | `mathop` | `S=4`, `T=3` | [x] |
| M4,4 | `mathop` | `S=4`, `T=4` | [x] |
| M4,5 | `mathop` | `S=4`, `T=5` | [x] |
| M5,-3 | `mathop` | `S=5`, `T=-3` | [x] |
| M5,-2 | `mathop` | `S=5`, `T=-2` | [x] |
| M5,-1 | `mathop` | `S=5`, `T=-1` | [x] |
| M5,0 | `mathop` | `S=5`, `T=0` | [x] |
| M5,1 | `mathop` | `S=5`, `T=1` | [x] |
| M5,2 | `mathop` | `S=5`, `T=2` | [x] |
| M5,3 | `mathop` | `S=5`, `T=3` | [x] |
| M5,4 | `mathop` | `S=5`, `T=4` | [x] |
| M5,5 | `mathop` | `S=5`, `T=5` | [x] |

