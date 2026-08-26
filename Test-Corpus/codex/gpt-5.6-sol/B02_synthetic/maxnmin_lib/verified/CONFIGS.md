# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, optional dependencies, or default
features. `c_src/CMakeLists.txt` has no options, compile definitions, source
conditions, or platform branches. Therefore there is exactly one valid build
combination:

| # | Rust features | C configuration | Check command |
|---|---------------|-----------------|---------------|
| 1 | none (`--no-default-features`) | CMake defaults with PIC enabled | `cargo check --no-default-features` |

## Runtime Configurations

Rows come from the loops and branches in `c_src/src/lib.c`, plus the input
shapes imposed by `MAX_NODES == 100` and `MAX_NAME_LEN == 50`. Rows whose
defined result is an error or sentinel are in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `find_node_by_id` | initial empty storage before any insertion | [x] |
| 2 | `add_node`, `find_node_by_id` | capacity available; empty NUL-terminated name | [x] |
| 3 | `add_node`, `find_node_by_id` | capacity available; name length 1 through 48 | [x] |
| 4 | `add_node`, `find_node_by_id` | capacity available; name length exactly 49 | [x] |
| 5 | `add_node`, `find_node_by_id` | capacity available; name longer than 49; stored name is truncated and terminated | [x] |
| 6 | `add_node`, `find_node_by_id` | arbitrary `id`, `parent_id`, and finite/special `double` bit patterns are stored and returned | [x] |
| 7 | `add_node`, `find_node_by_id` | final available slot (`node_count == 99`) succeeds at index 99 | [x] |
| 8 | `find_node_by_id` | active match is first, middle, or last loop element | [x] |
| 9 | `find_node_by_id` | duplicate active IDs return the first stored match | [x] |
| 10 | `find_node_by_id` | inactive matching node is skipped and a later active duplicate is returned | [x] |
| 11 | `get_children_count` | zero active children with unrelated records present | [x] |
| 12 | `get_children_count` | exactly one active child | [x] |
| 13 | `get_children_count` | multiple active children across first/middle/last records | [x] |
| 14 | `get_children_count` | matching inactive records are excluded | [x] |
| 15 | `calculate_subtree_sum` | active leaf node | [x] |
| 16 | `calculate_subtree_sum` | active internal node with one or multiple direct children | [x] |
| 17 | `calculate_subtree_sum` | active tree with multiple recursive depths and sibling branches | [x] |
| 18 | `calculate_subtree_sum` | inactive matching child is excluded from recursion | [x] |
| 19 | `process_string` | empty string | [x] |
| 20 | `process_string` | one-byte and multi-byte ASCII strings | [x] |
| 21 | `process_string` | bytes with the high bit set, using platform signed-`char` arithmetic | [x] |
| 22 | `safe_double_to_int` | finite negative/zero/positive values strictly inside the integer range; fractions truncate toward zero | [x] |
| 23 | `safe_double_to_int` | exact `INT_MIN` and `INT_MAX` boundaries | [x] |
| 24 | `maxnmin` | `param1 % 6 + 1` selects root, internal, or leaf nodes | [x] |
| 25 | `maxnmin` | negative `param1` yields both valid ID 1 and missing IDs `-4..0` | [x] |
| 26 | `maxnmin` | `param2 % 6 + 1` selects each stored node; finite multiplication by `param3` | [x] |
| 27 | `maxnmin` | negative `param2` produces both valid and missing second-node IDs | [x] |
| 28 | `maxnmin` | second-node multiplication exceeds the positive/negative `int` range and clamps | [x] |
| 29 | `maxnmin` | `param4 % 3 + 1` selects parents 1, 2, 3 (two, two, and one children) | [x] |
| 30 | `maxnmin` | negative `param4` produces parent IDs -1, 0, and 1 (zero or two children) | [x] |
| 31 | `maxnmin` | final calculation uses a finite nonzero positive or negative denominator | [x] |
| 32 | `maxnmin` | `param3 == -1`, zero denominator, nonzero numerator: signed infinity clamps | [x] |
| 33 | `maxnmin` | `param3 == -1`, zero denominator, zero numerator: NaN converts to zero | [x] |
| 34 | `maxnmin` | integer boundary and overflow-adjacent parameters in `param1 + param2` and `param3 + 1` | [x] |
| 35 | `maxnmin` | randomized cross-product of selected/missing first and second nodes, parent classes, denominator classes, and full-width integer values | [x] |
