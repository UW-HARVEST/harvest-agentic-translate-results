# Error Surface

The inventory was derived from every `return -1`, `return NULL`, explicit
range check, and NaN check in `c_src/src/lib.c`. The source has no assertions,
error enums, `RETURN_ERROR` macros, or length parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1. [x] | `add_node` | `node_count >= MAX_NODES` (`MAX_NODES == 100`) | `-1`; storage and count are unchanged |
| 2. [x] | `find_node_by_id` | no stored node has both `id == requested_id` and `active != 0` | `NULL` |
| 3. [x] | `calculate_subtree_sum` | `find_node_by_id(node_id) == NULL` | `0.0` |
| 4. [x] | `safe_double_to_int` | `d > (double)INT_MAX` (including positive infinity) | `INT_MAX` |
| 5. [x] | `safe_double_to_int` | `d < (double)INT_MIN` (including negative infinity) | `INT_MIN` |
| 6. [x] | `safe_double_to_int` | `d != d` (NaN) after both range comparisons are false | `0` |

## Generic FFI Boundaries

- `add_node(NULL)` below capacity and `process_string(NULL)` dereference the
  null pointer in C. They have undefined behavior rather than a C rejection
  result, so subprocess tests compare the observable process termination
  behavior without adding fictitious rows to the explicit error table.
- A null `name` passed to `add_node` at full capacity is short-circuited by the
  capacity check and returns `-1`; this is covered by row 1.
- There are no length/count parameters, enums, or documented numeric ranges in
  the API. Zero-length strings and the fixed 49-byte name boundary are valid
  configurations listed in `CONFIGS.md`.
