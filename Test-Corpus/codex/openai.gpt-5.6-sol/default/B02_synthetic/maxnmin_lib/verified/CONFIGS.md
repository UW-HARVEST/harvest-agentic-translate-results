# Configuration Surface

Derived from the public dynamic entry points and every data-dependent `if`,
loop, modulo selector, and conversion boundary in `../c_src/src/lib.c`. The
library has no Cargo features and the C source has no conditional compilation,
runtime mode, option, flag, public enum, byte-order branch, or element-type
branch.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C01 | `add_node` | empty storage; empty C string; finite value | [x] |
| C02 | `add_node` | available storage; name length 1 through 48 | [x] |
| C03 | `add_node` | available storage; name length exactly 49 | [x] |
| C04 | `add_node` | available storage; name length greater than 49, truncated and terminated at byte 49 | [x] |
| C05 | `add_node` | available storage; arbitrary IDs/parent IDs and NaN or infinity value | [x] |
| C06 | `find_node_by_id` | empty storage | [x] |
| C07 | `find_node_by_id` | one active matching node | [x] |
| C08 | `find_node_by_id` | duplicate IDs; first active match wins | [x] |
| C09 | `find_node_by_id` | matching inactive node is skipped in favor of a later active match | [x] |
| C10 | `get_children_count` | empty storage or no matching active parent IDs | [x] |
| C11 | `get_children_count` | exactly one active matching child | [x] |
| C12 | `get_children_count` | many active matching children, with inactive matches excluded | [x] |
| C13 | `calculate_subtree_sum` | active leaf node | [x] |
| C14 | `calculate_subtree_sum` | one-level tree with one active child | [x] |
| C15 | `calculate_subtree_sum` | multi-level tree with multiple active children | [x] |
| C16 | `calculate_subtree_sum` | inactive descendants excluded | [x] |
| C17 | `calculate_subtree_sum` | active tree containing NaN or infinity | [x] |
| C18 | `process_string` | empty C string | [x] |
| C19 | `process_string` | one-byte C string | [x] |
| C20 | `process_string` | many bytes ending at the first embedded NUL | [x] |
| C21 | `process_string` | bytes with the high bit set, using the platform C `char` signedness | [x] |
| C22 | `safe_double_to_int` | finite in-range positive/negative integral values, including `INT_MIN` and `INT_MAX` | [x] |
| C23 | `safe_double_to_int` | finite in-range positive/negative fractional values truncate toward zero | [x] |
| C24 | `maxnmin` | `param1` selects each node ID 1 through 6; leaf and non-leaf subtree/name branches | [x] |
| C25 | `maxnmin` | `param2` selects each node ID 1 through 6 for multiplied values | [x] |
| C26 | `maxnmin` | `param4 % 3` selects parent ID 1, 2, or 3 | [x] |
| C27 | `maxnmin` | negative `param4` selects parent ID 0 or -1 | [x] |
| C28 | `maxnmin` | `param3 == -1`, making the floating-point denominator zero | [x] |
| C29 | `maxnmin` | finite nonzero denominator and fractional final calculation | [x] |
| C30 | `maxnmin` | extreme integer parameters exercise multiplication/conversion clamps and wrapping machine arithmetic | [x] |
| C31 | `maxnmin` | repeated calls reset global storage to the fixed six-node tree | [x] |

The error-side selector combinations and capacity boundary are enumerated in
`ERRORS.md` and are not duplicated here.

All rows passed under the default and no-default-features configurations.
