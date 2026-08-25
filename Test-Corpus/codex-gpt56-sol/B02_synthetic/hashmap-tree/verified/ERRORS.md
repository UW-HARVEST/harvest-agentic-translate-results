# Error Surface

Rows are derived from every externally observable rejection/sentinel branch in
`c_src/src/hashmap.c` and `c_src/src/tree.c`. Allocation failures are included
because the C source has distinct failure returns for them. Static helper
failures are listed through the public call that exposes their result.

Covered by integration tests `non_allocation_error_surface_matches` and
`allocation_failure_surface_matches`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 [x] | `hashmap_create` | `malloc(sizeof(hashmap_t))` returns null | null |
| E02 [x] | `hashmap_create` | map allocation succeeds, then entry `calloc` returns null | null; map is freed |
| E03 [x] | `hashmap_destroy` | `map == NULL` | no-op |
| E04 [x] | `hashmap_put` | `map == NULL` | `-1` |
| E05 [x] | `hashmap_put` | load exceeds `0.75` and resize `calloc` fails | `-1`; old table retained |
| E06 [x] | `hashmap_put` | every slot is occupied and resize is not requested | `-1` |
| E07 [x] | `hashmap_get` | `map == NULL` | null |
| E08 [x] | `hashmap_get` | probe reaches an unoccupied slot before finding key | null |
| E09 [x] | `hashmap_get` | full-capacity probe finds no matching live key | null |
| E10 [x] | `hashmap_remove` | `map == NULL` | null |
| E11 [x] | `hashmap_remove` | probe reaches an unoccupied slot before finding key | null |
| E12 [x] | `hashmap_remove` | full-capacity probe finds no matching live key | null |
| E13 [x] | `hashmap_contains` | `map == NULL` | `0` |
| E14 [x] | `hashmap_size` | `map == NULL` | `0` |
| E15 [x] | `hashmap_clear` | `map == NULL` | no-op |
| E16 [x] | `tree_create` | `malloc(sizeof(tree_t))` returns null | null |
| E17 [x] | `tree_create` | tree allocation succeeds but `hashmap_create` returns null | null; tree is freed |
| E18 [x] | `tree_delete` | `tree == NULL` | no-op |
| E19 [x] | `tree_add_node` | `tree == NULL` | `-1` |
| E20 [x] | `tree_add_node` | `tree_contains(tree, id)` is true | `-1`; tree unchanged |
| E21 [x] | `tree_add_node` | node `malloc` returns null | `-1`; tree unchanged |
| E22 [x] | `tree_add_node` | tree has a root and `parent_id` is absent | `-1`; allocated node freed |
| E23 [x] | `tree_add_node` | parent `child_count >= MAX_CHILDREN` (`32`) | `-1`; allocated node freed |
| E24 [x] | `tree_add_node` | internal `hashmap_put` fails | `-1`; allocated node freed |
| E25 [x] | `tree_remove_node` | `tree == NULL` | `-1` |
| E26 [x] | `tree_remove_node` | requested `id` is absent | `-1` |
| E27 [x] | `tree_get_node` | `tree == NULL` | null |
| E28 [x] | `tree_get_node` | requested `id` is absent | null |
| E29 [x] | `tree_contains` | `tree == NULL` | `0` |
| E30 [x] | `tree_size` | `tree == NULL` | `0` |
| E31 [x] | `tree_print` | `tree == NULL` | writes `"(empty tree)\n"` |
| E32 [x] | `tree_get_depth` | `tree == NULL` | `-1` |
| E33 [x] | `tree_get_depth` | requested `id` is absent | `-1` |
| E34 [x] | `tree_get_depth` | ancestor lookup becomes null while walking to root | `-1` |
| E35 [x] | `tree_get_height` | `tree == NULL` or requested `id` is absent | `-1` |
| E36 [x] | `tree_count_descendants` | `tree == NULL` or requested `id` is absent | `-1` |
| E37 [x] | `tree_find_path` | `tree == NULL` | `-1` |
| E38 [x] | `tree_find_path` | `path == NULL` | `-1` |
| E39 [x] | `tree_find_path` | requested `id` is absent | `-1` |
| E40 [x] | `tree_find_path` | ancestor lookup becomes null while walking to root | `-1` |

No C enum types are present in the public API, so there is no out-of-range enum
surface. The test-driver assertions in `src/main.c` are consumers of this API,
not library input checks, and are therefore not exported error branches.
