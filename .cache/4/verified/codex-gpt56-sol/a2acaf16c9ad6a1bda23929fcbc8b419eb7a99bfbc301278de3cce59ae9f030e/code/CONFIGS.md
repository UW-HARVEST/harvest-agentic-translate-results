# Configuration Surface

## Build-Time Configurations

`Cargo.toml` declares no features and `c_src/CMakeLists.txt` declares no CMake
options or conditional sources. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | [x] |
|---|-------------------|-----------------|-----|
| B01 | empty (`--no-default-features`) | default, `src/lib.c` | [x] |

## Runtime Configurations

Rows are derived from the public dynamic symbols and the `if`, `else if`,
`switch`, loop, recursive-child, parser-precedence, and modulo-index branches
in `c_src/src/lib.c`. Arithmetic randomization excludes C operations with
undefined signed overflow and `INT_MIN / -1`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | `add_op` | randomized `int` operands; both ignored arguments varied | [x] |
| C02 | `multiply_op` | randomized `int` operands; both ignored arguments varied | [x] |
| C03 | `subtract_op` | randomized `int` operands; both ignored arguments varied | [x] |
| C04 | `divide_op` | randomized nonzero divisor | [x] |
| C05 | `modulo_op` | randomized nonzero divisor | [x] |
| C06 | `find_node_by_id` | empty table | [x] |
| C07 | `find_node_by_id` | requested ID is the first table entry | [x] |
| C08 | `find_node_by_id` | requested ID is a later table entry | [x] |
| C09 | `find_node_by_id` | populated table with no matching ID | [x] |
| C10 | `add_tree_node` | root (`parent_id == -1`) with empty or short label | [x] |
| C11 | `add_tree_node` | root with exactly 31 label bytes | [x] |
| C12 | `add_tree_node` | root with more than 31 label bytes (truncation) | [x] |
| C13 | `add_tree_node` | existing parent has no children; set left child | [x] |
| C14 | `add_tree_node` | existing parent has only a left child; set right child | [x] |
| C15 | `add_tree_node` | existing parent already has two children; insert node without attaching it | [x] |
| C16 | `add_tree_node`, `find_node_by_id` | duplicate ID; lookup returns first matching entry | [x] |
| C17 | `calculate_tree_sum` | leaf node | [x] |
| C18 | `calculate_tree_sum` | node with only a left child | [x] |
| C19 | `calculate_tree_sum` | node with only a right child | [x] |
| C20 | `calculate_tree_sum` | node with both children | [x] |
| C21 | `calculate_tree_sum` | nested descendants exercise recursive accumulation | [x] |
| C22 | `parse_operation` | string contains `+` (highest precedence), including mixed operators | [x] |
| C23 | `parse_operation` | no `+`, string contains `*` | [x] |
| C24 | `parse_operation` | no `+`/`*`, string contains `-` | [x] |
| C25 | `parse_operation` | no `+`/`*`/`-`, string contains `/` | [x] |
| C26 | `parse_operation` | only recognized operator is `%` | [x] |
| C27 | `parse_operation` | empty string or no recognized operator | [x] |
| C28 | `get_operation_func` | operation `1` dispatches to addition | [x] |
| C29 | `get_operation_func` | operation `2` dispatches to multiplication | [x] |
| C30 | `get_operation_func` | operation `3` dispatches to subtraction | [x] |
| C31 | `get_operation_func` | operation `4` dispatches to division | [x] |
| C32 | `get_operation_func` | operation `5` dispatches to modulo | [x] |
| C33 | `node_table`, `node_count` | globals are writable and observed by tree entry points | [x] |
| C34 | `inreftree` | `param2 != 0`, nonnegative tree sum, `tree_sum % 4 == 0` (`+`, target ID 2) | [x] |
| C35 | `inreftree` | `param2 != 0`, nonnegative tree sum, `tree_sum % 4 == 1` (`*`, target ID 2) | [x] |
| C36 | `inreftree` | `param2 != 0`, nonnegative tree sum, `tree_sum % 4 == 2` (`-`, target ID 2) | [x] |
| C37 | `inreftree` | `param2 != 0`, nonnegative tree sum, `tree_sum % 4 == 3` (`%`, target ID 2) | [x] |
| C38 | `inreftree` | `param2 == 0`, nonnegative tree sum, `tree_sum % 4 == 0` (`+`, fallback target ID 1) | [x] |
| C39 | `inreftree` | `param2 == 0`, nonnegative tree sum, `tree_sum % 4 == 1` (`*`, fallback target ID 1) | [x] |
| C40 | `inreftree` | `param2 == 0`, nonnegative tree sum, `tree_sum % 4 == 2` (`-`, fallback target ID 1) | [x] |
| C41 | `inreftree` | `param2 == 0`, nonnegative tree sum, `tree_sum % 4 == 3` (`%`, fallback target ID 1) | [x] |
| C42 | `inreftree` | negative tree sum gives C remainder `-1` and reads one byte before `"+*-%"` | [x] |
| C43 | `inreftree` | negative tree sum gives C remainder `-2` and reads two bytes before `"+*-%"` | [x] |
| C44 | `inreftree` | negative tree sum gives C remainder `-3` and reads three bytes before `"+*-%"` | [x] |
| C45 | `inreftree`, `node_count` | pre-existing global tree state is discarded before constructing four fixed nodes | [x] |
| C46 | `add_tree_node`, `node_count` | `node_count == 49`; insert into the final valid table slot and advance count to 50 | [x] |
