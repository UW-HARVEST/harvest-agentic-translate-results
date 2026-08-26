# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional definitions. There is exactly one valid combination:

| # | Rust features | C configuration | verified |
|---|---------------|-----------------|-----|
| B1 | No features (`--no-default-features`) | Default CMake configuration with position-independent code | [x] |

## Runtime and Input Configurations

The public header exposes no runtime option flags or enums. Rows below are the
cross-product pruned to distinctions present in C branches, loops, constants,
and pointer-identity comparisons. `MAX_CITY_NAME = 64`, `MAX_EDGES = 10`, and
`MAX_NODES = 100`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `create_graph` | Empty graph initialization: count 0 and all 100 node slots null | [x] |
| 2 | `add_node` | First node; empty city name (length 0) | [x] |
| 3 | `add_node` | First node; short city name (length 1..62) | [x] |
| 4 | `add_node` | First node; boundary city name (length 63) | [x] |
| 5 | `add_node` | First node; long city name (length >=64), truncated to 63 bytes and terminated | [x] |
| 6 | `add_node` | Nonempty graph below capacity; unique name exercises existing-node scan | [x] |
| 7 | `add_node`, `get_node_by_name` | Long-name asymmetry: stored truncated name is found by its 63-byte form, while the original long form is not equal | [x] |
| 8 | `add_node` | Boundary valid insertion at prior `node_count == 99`, producing 100 nodes | [x] |
| 9 | `get_node_by_name` | Match first node in a one-node graph | [x] |
| 10 | `get_node_by_name` | Match a later node after one or more failed comparisons | [x] |
| 11 | `add_edge` | Distinct nodes, first outgoing edge, zero distance | [x] |
| 12 | `add_edge` | Distinct nodes, first outgoing edge, positive distance | [x] |
| 13 | `add_edge` | Self-edge (`from == to`) | [x] |
| 14 | `add_edge` | Boundary valid insertion at prior `edge_count == 9`, producing 10 edges | [x] |
| 15 | `delete_node` | Reference count greater than 1: decrement without freeing | [x] |
| 16 | `shallow_copy` | Isolated start node: increment only the start | [x] |
| 17 | `shallow_copy` | Linear reachable chain with one or many edges | [x] |
| 18 | `shallow_copy` | Branches that merge on one destination: increment each pointer once | [x] |
| 19 | `shallow_copy` | Directed cycle/self-edge: visited-pointer guard terminates recursion and increments each node once | [x] |
| 20 | `find_shortest_path` | `start == end`: one-node path | [x] |
| 21 | `find_shortest_path` | Direct edge: two-node path | [x] |
| 22 | `find_shortest_path` | Linear chain: multi-node path reconstruction and reversal | [x] |
| 23 | `find_shortest_path` | Competing routes with different total distances: choose lower total | [x] |
| 24 | `find_shortest_path` | Equal-distance alternatives: retain first predecessor because update uses strict `<` | [x] |
| 25 | `find_shortest_path` | Zero-distance edges mixed with positive edges | [x] |
| 26 | `find_shortest_path` | Reachable graph containing a directed cycle | [x] |
| 27 | `find_shortest_path` | Boundary reachable state of 100 nodes | [x] |
| 28 | `print_node` | Node with zero outgoing edges | [x] |
| 29 | `print_node` | Node with one or many outgoing edges | [x] |
| 30 | `print_graph` | Empty graph | [x] |
| 31 | `print_graph` | Graph with one or many nodes and their edges | [x] |
| 32 | `free_graph` | Empty graph | [x] |
| 33 | `free_graph`, `delete_node` | Graph node at ref count 1: graph cleanup decrements to zero and frees it | [x] |
| 34 | `free_graph`, `shallow_copy` | Graph nodes with ref count greater than 1: graph cleanup decrements but does not free them | [x] |
| 35 | `add_node` | Repeat the same name of length >=64: duplicate scan compares the untruncated input, so a second node with the same truncated stored name is accepted | [x] |
| 36 | `add_edge`, `find_shortest_path` | Distance `INT_MAX`: edge insertion succeeds, but Dijkstra retains its `INT_MAX` unreachable sentinel and returns no path | [x] |
| 37 | `find_shortest_path` | 101 reachable node pointers: state fills at 100, the 101st neighbor is not added, and no path to it is returned | [x] |
| 38 | `delete_node` | Input reference count 0: decrement to -1 without freeing because the post-decrement value is not zero | [x] |
