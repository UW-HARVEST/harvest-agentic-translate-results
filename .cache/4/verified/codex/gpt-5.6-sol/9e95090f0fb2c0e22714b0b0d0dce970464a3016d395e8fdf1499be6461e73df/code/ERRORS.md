# Error Surface

Rows are derived from every error return, null check, capacity/range check,
allocation check, and not-found sentinel in `c_src/src/lib.c`. Internal
recursive guards are exercised through `shallow_copy`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `create_graph` | `malloc(sizeof(graph_t)) == NULL` | Return `NULL`. | [x] |
| 2 | `add_node` | `graph == NULL` or `city_name == NULL` | Return `NULL`. | [x] |
| 3 | `add_node` | `graph->node_count >= MAX_NODES` (100) | Return `NULL`. | [x] |
| 4 | `add_node` | An existing stored `city_name` is exactly equal to the input under `strcmp` | Return `NULL`. | [x] |
| 5 | `add_node` | `malloc(sizeof(node_t)) == NULL` | Return `NULL` without incrementing `node_count`. | [x] |
| 6 | `add_edge` | `from == NULL` or `to == NULL` | Return `-1`. | [x] |
| 7 | `add_edge` | `from->edge_count >= MAX_EDGES` (10) | Return `-1`. | [x] |
| 8 | `add_edge` | `distance < 0` | Return `-1`. | [x] |
| 9 | `add_edge` | An existing edge has `destination == to` | Return `-1`. | [x] |
| 10 | `delete_node` | `node == NULL` | Return normally without action. | [x] |
| 11 | `delete_node` | Decrement makes `node->ref_count == 0` | Free the node and return normally. | [x] |
| 12 | `increment_refs_recursive` via `shallow_copy` | Recursive `node == NULL` | Return from that recursive branch without action. | [x] |
| 13 | `increment_refs_recursive` via `shallow_copy` | Node pointer is already in `visited` | Return from that recursive branch without a second increment. | [x] |
| 14 | `increment_refs_recursive` via `shallow_copy` | `*visited_count >= MAX_NODES` | Do not append to `visited`; continue incrementing and traversing. | [x] |
| 15 | `shallow_copy` | `start == NULL` | Return `NULL`. | [x] |
| 16 | `find_shortest_path` | `start == NULL`, `end == NULL`, or `path_length == NULL` | Return `NULL`; do not write a length. | [x] |
| 17 | `find_shortest_path` | End node is absent from state | Set `*path_length = 0` and return `NULL`. | [x] |
| 18 | `find_shortest_path` | `malloc(sizeof(node_t*) * count) == NULL` | Set `*path_length = 0` and return `NULL`. | [x] |
| 19 | `get_node_by_name` | `graph == NULL` or `city_name == NULL` | Return `NULL`. | [x] |
| 20 | `get_node_by_name` | No stored name compares equal to `city_name` | Return `NULL`. | [x] |
| 21 | `print_node` | `node == NULL` | Write `NULL node\n` to stdout and return. | [x] |
| 22 | `print_graph` | `graph == NULL` | Write `NULL graph\n` to stdout and return. | [x] |
| 23 | `free_graph` | `graph == NULL` | Return normally without action. | [x] |
| 24 | `find_shortest_path` | End node is present in state but retains distance `INT_MAX` (for example, a direct edge of weight `INT_MAX`) | Set `*path_length = 0` and return `NULL`. | [x] |

Generic boundary audit: the public API has no length parameters and no enums.
Zero and oversized cases therefore apply to empty names, node count 100/101,
edge count 10/11, null pointers, and negative/out-of-range signed distances;
the differential suite covers each applicable case.
