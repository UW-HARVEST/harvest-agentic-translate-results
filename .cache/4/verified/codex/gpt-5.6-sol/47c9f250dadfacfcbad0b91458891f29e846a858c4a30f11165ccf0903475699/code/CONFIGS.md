# Configuration Surface

## Build-Time Configurations

`Cargo.toml` defines no optional features and `c_src/CMakeLists.txt` defines no
options or conditional compilation. The complete valid feature matrix is:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|-----|
| F01 | `cargo test --no-default-features` (empty feature set) | default, no options | [x] |

## Runtime Configurations

Rows come from public-header entry points crossed with branch axes in
`hashmap.c` and `tree.c`: occupancy, collision/probe position, tombstones,
load threshold, root state, node position, child count, data pointer/length,
tree shape, and path capacity.

Covered by integration tests `hashmap_valid_surface_randomized`,
`tree_valid_surface_randomized`, `tree_find_path_thousand_element_boundary`,
and `tree_print_bytes_match`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| C01 | `hashmap_create`, `hashmap_size` | newly created map: capacity `16`, zero live/deleted entries | [x] |
| C02 | `hashmap_put`, `hashmap_get` | insert into hash-selected empty slot; random keys and nonnull values | [x] |
| C03 | `hashmap_put`, `hashmap_get` | colliding keys requiring one or many linear probes | [x] |
| C04 | `hashmap_put`, `hashmap_size` | update an existing key; size must not change | [x] |
| C05 | `hashmap_put`, `hashmap_get`, `hashmap_contains` | live key whose stored value is null | [x] |
| C06 | `hashmap_remove` | remove a live key at its initial probe position | [x] |
| C07 | `hashmap_remove` | remove a collided key after one or many probes | [x] |
| C08 | `hashmap_put` | insert reuses the first deleted slot and decrements `deleted_count` | [x] |
| C09 | `hashmap_get` | successful lookup probes across a deleted slot | [x] |
| C10 | `hashmap_put` | active-entry load first exceeds `0.75`; next insertion doubles capacity and rehashes | [x] |
| C11 | `hashmap_put` | deleted entries make `(size + deleted_count) / capacity > 0.75`; next insertion rehashes | [x] |
| C12 | `hashmap_get`, `hashmap_contains` | absent key terminates at first never-occupied slot | [x] |
| C13 | `hashmap_remove` | absent key terminates at first never-occupied slot | [x] |
| C14 | `hashmap_size` | size after insert, update, remove, and tombstone reuse | [x] |
| C15 | `hashmap_clear`, `hashmap_size`, `hashmap_get` | clear empty map and populated map containing tombstones | [x] |
| C16 | `hashmap_destroy` | destroy empty and populated maps; values remain caller-owned | [x] |
| C17 | `tree_create`, `tree_size`, `tree_delete` | newly created empty tree, then deletion | [x] |
| C18 | `tree_add_node`, `tree_get_node` | first node becomes root and supplied `parent_id` is replaced with `0` | [x] |
| C19 | `tree_add_node`, `tree_get_node` | root data pointer is null | [x] |
| C20 | `tree_add_node`, `tree_get_node` | data is empty, short, exactly 255 bytes, and longer than 255 bytes | [x] |
| C21 | `tree_add_node`, `tree_get_node` | root/node IDs include `0`, `UINT64_MAX`, and randomized values | [x] |
| C22 | `tree_add_node`, `tree_get_node` | add child to existing parent; child appended in insertion order | [x] |
| C23 | `tree_add_node` | parent transitions from 31 to exactly 32 children | [x] |
| C24 | `tree_get_node`, `tree_contains` | present root, present descendant, and absent random ID | [x] |
| C25 | `tree_size` | empty, one-node, and many-node trees | [x] |
| C26 | `tree_remove_node` | remove only child/leaf | [x] |
| C27 | `tree_remove_node` | remove first, middle, and last child; remaining IDs shift left | [x] |
| C28 | `tree_remove_node` | remove an internal node with a multi-level subtree | [x] |
| C29 | `tree_remove_node` | remove root with descendants; tree becomes empty and root state resets | [x] |
| C30 | `tree_get_depth` | root depth `0`, direct child, and randomized deep chain | [x] |
| C31 | `tree_get_height` | leaf height `0`, chain, and branching tree selecting maximum child height | [x] |
| C32 | `tree_count_descendants` | leaf, internal subtree, and root in branching trees | [x] |
| C33 | `tree_find_path` | root and deep node with `max_length >=` full path length | [x] |
| C34 | `tree_find_path` | deep node with `0 < max_length <` full path length | [x] |
| C35 | `tree_find_path` | valid node with `max_length == 0` | [x] |
| C36 | `tree_find_path` | valid node with negative `max_length`; C returns that negative value | [x] |
| C37 | `tree_print` | empty tree | [x] |
| C38 | `tree_print` | populated branching tree; two-space indentation and insertion order | [x] |
| C39 | all hashmap entry points | randomized mixed operation sequences across resize/collision/tombstone states | [x] |
| C40 | all tree entry points | randomized valid trees and randomized query/removal sequences | [x] |
| C41 | `tree_find_path` | valid chain deeper than the fixed 1000-element temporary path; result caps at `1000` | [x] |
