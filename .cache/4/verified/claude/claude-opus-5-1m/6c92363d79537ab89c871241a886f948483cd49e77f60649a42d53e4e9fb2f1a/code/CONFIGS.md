# CONFIGS.md — configuration-surface table (Phase B)

Axes derived from the branches the C actually takes.

**Build-time axes:** none. `Cargo.toml` has no `[features]`; `c_src` has no
`#if`/`#ifdef` other than the `DAG_LIB_H` include guard. Every row below is
therefore run for both `cargo test --offline` and
`cargo test --offline --no-default-features` (see `run_all.sh`).

**Runtime axes the C branches on**

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `graph->node_count` | `0`, `1`, `2`, `>2`, `MAX_NODES-1` (99), `MAX_NODES` (100) | `add_node` L55/L61, `get_node_by_name` L328, `print_graph` L361, `free_graph` L373 |
| `city_name` length | `0`, `1`, `62`, `63` (= `MAX_CITY_NAME-1`), `64`, `65`, `>255` | `strncpy` L76 truncation, `strcmp` L62/L329 |
| `city_name` bytes | ASCII, spaces, digits, punctuation, bytes ≥ 0x80, shared prefixes | `strcmp` L62/L329, `%s` L63/L344 |
| `node->edge_count` | `0`, `1`, `2..9`, `10` (= `MAX_EDGES`) | `add_edge` L94/L105, `print_node` L346, Dijkstra L227 |
| `distance` | `0`, `1`, small, large, `INT_MAX`, negative | `add_edge` L99, relaxation L229/L250 |
| topology | isolated node, self-loop, chain, diamond with equal costs (tie-break), diamond with distinct costs, cycle, back edge, star, disconnected, cross-graph edge | Dijkstra loop L205–L265 |
| `start`/`end` relation | `start == end`, adjacent, multi-hop, reachable only via a longer first discovery (needs re-relaxation), unreachable, reachable only in the opposite direction (edges are directed) | L222, L250, L276 |
| Dijkstra `state_count` | `1`, `<MAX_NODES`, `== MAX_NODES` (only reachable with nodes from two graphs) | L240 |
| `ref_count` | `1` (fresh), `>1` (after `shallow_copy`), decremented but `>0`, `0` (frees) | `delete_node` L126, `print_node` L344 |
| `visited_count` in `shallow_copy` | node reached once, node reached twice (dedup), cyclic | L140–L149 |
| stream | `printf` → fully buffered stdout, `fprintf(stderr,…)` → unbuffered stderr | interleaving of the two |
| `main` `choice` | `1`..`8`, out of range, unparsable | `switch` L71 |
| `fgets` | line with `\n`, line without `\n` (EOF), line ≥ 256 bytes (split), EOF (NULL) | L62/L75/… |

`L#` = line in `c_src/src/lib.c` (or `main.c` for the `main` rows).

## Library rows — `tests/ffi_diff.rs`, both `.so`s via `libloading`

Every row is driven with many randomized inputs (fixed seed, deterministic
xorshift PRNG in `tests/common/mod.rs`) and compares: return values (pointers
canonicalized to creation indices), the full `node_t` (all 240 bytes of every
live node: `city_name[64]`, `ref_count`, `edges[]`, `edge_count`) and `graph_t`
(`node_count` + the `nodes[]` slots), plus captured stdout and stderr bytes.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `create_graph`, `free_graph` | empty graph, nothing added; `graph_t` contents inspected | `cfg_create_free_empty` | [x] |
| 2 | `create_graph`, `add_node`, `free_graph` | 1 node, name = 1 byte | `cfg_single_node` | [x] |
| 3 | `add_node` | name lengths 0,1,2,…,66 and 200/300 bytes (`strncpy` truncation boundary at 63) | `cfg_add_node_name_lengths` | [x] |
| 4 | `add_node` | random names with spaces, digits, punctuation and bytes ≥ 0x80 | `cfg_add_node_random_names` | [x] |
| 5 | `add_node` | names sharing long prefixes (63-byte prefix collisions) | `cfg_add_node_shared_prefixes` | [x] |
| 6 | `add_node` | 99 nodes, then the 100th (last accepted) | `cfg_add_node_fill_to_max` | [x] |
| 7 | `add_edge` | 0 → 1 edge; `distance = 0` | `cfg_add_edge_first` | [x] |
| 8 | `add_edge` | fill a node to exactly `MAX_EDGES` = 10 edges | `cfg_add_edge_fill_to_max` | [x] |
| 9 | `add_edge` | `distance` ∈ {0, 1, 2, 1000, `INT_MAX/2`, `INT_MAX`} | `cfg_add_edge_distance_values` | [x] |
| 10 | `add_edge` | self edge (`from == to`) | `cfg_add_edge_self` | [x] |
| 11 | `add_edge` | both directions between the same pair (`a→b` and `b→a`) | `cfg_add_edge_both_directions` | [x] |
| 12 | `get_node_by_name` | empty graph / 1 / many nodes; hit at first, middle, last slot; miss | `cfg_get_node_positions` | [x] |
| 13 | `get_node_by_name` | name that only matches after `strncpy` truncation (65-byte name looked up by its 63-byte prefix) | `cfg_get_node_truncated_lookup` | [x] |
| 14 | `print_node` | node with 0 edges, empty name | `cfg_print_node_no_edges` | [x] |
| 15 | `print_node` | node with 1..10 edges, distances incl. `INT_MAX`, destination names with high-bit bytes | `cfg_print_node_edges` | [x] |
| 16 | `print_node` | `ref_count` after 0/1/3 `shallow_copy` calls | `cfg_print_node_ref_counts` | [x] |
| 17 | `print_graph` | 0, 1, 2, 17 nodes, mixed edge counts | `cfg_print_graph_sizes` | [x] |
| 18 | `shallow_copy` | single node, no edges | `cfg_shallow_copy_single` | [x] |
| 19 | `shallow_copy` | chain, diamond (node reachable twice → visited dedup), cycle, self-loop | `cfg_shallow_copy_topologies` | [x] |
| 20 | `shallow_copy` | repeated calls (ref counts accumulate), called from a middle node (only the reachable subset is bumped) | `cfg_shallow_copy_repeated` | [x] |
| 21 | `find_shortest_path` | `start == end` | `cfg_fsp_start_is_end` | [x] |
| 22 | `find_shortest_path` | single edge; chain of 2..8 hops | `cfg_fsp_chain` | [x] |
| 23 | `find_shortest_path` | diamond with **equal** total cost (exercises the C's tie-breaking: strict `<` keeps the first) | `cfg_fsp_equal_cost_tie` | [x] |
| 24 | `find_shortest_path` | ≥ `MAX_NODES` distinct reachable nodes (edge across two graphs) → `state_count == MAX_NODES` branch | `cfg_fsp_state_full` | [x] |
| 25 | `find_shortest_path` | cycles, self-loops and back edges on the shortest path | `cfg_fsp_cycles` | [x] |
| 26 | `find_shortest_path` | target reachable only after re-relaxation (a cheaper route found after the node was first discovered) | `cfg_fsp_relaxation` | [x] |
| 27 | `find_shortest_path` | zero-weight edges only (all distances 0) | `cfg_fsp_zero_weights` | [x] |
| 28 | `find_shortest_path` | random directed graphs: 1..40 nodes, 0..10 edges/node, distances 0..10⁶, all (start, end) pairs | `cfg_fsp_random_graphs` | [x] |
| 29 | `find_shortest_path` | randomized dense graphs at the `MAX_EDGES` limit (every node with exactly 10 out-edges) | `cfg_fsp_dense_max_edges` | [x] |
| 30 | `find_shortest_path` | returned array is `free()`d by the caller (`malloc`-compatible), `*path_length` value checked | folded into every `cfg_fsp_*` row | [x] |
| 31 | `delete_node` | `ref_count` 3 → 2 → 1 (never frees), then `print_node` | `cfg_delete_node_positive` | [x] |
| 32 | `free_graph` | graph whose nodes all have `ref_count == 1` (all freed) | `cfg_free_graph_refcount_one` | [x] |
| 33 | `free_graph` | graph whose nodes have `ref_count > 1` (nothing freed, no crash) | `cfg_free_graph_refcount_many` | [x] |
| 34 | whole API | randomized operation sequences (250 runs × 60 ops) mixing `add_node`, `add_edge`, `get_node_by_name`, `shallow_copy`, `find_shortest_path`, `print_node`, `print_graph`, `delete_node`(ref>1) | `cfg_random_api_sequences` | [x] |

## Program rows — `tests/program_diff.rs`, `c_src/build/driver` vs `target/debug/driver`

Compared: stdout bytes, stderr bytes and the exit status.

| # | entry point(s) | configuration (command script shape) | test | [x] |
|---|----------------|--------------------------------------|------|-----|
| 35 | menu / choice 8 | immediate exit | `cfg_prog_exit_immediately` | [x] |
| 36 | menu / EOF | no input at all | `cfg_prog_empty_stdin` | [x] |
| 37 | choice 1 | add 1, 2, 3, 100 cities; names of length 0/1/63/64/65/300 | `cfg_prog_add_cities` | [x] |
| 38 | choice 2 | routes with distance 0/1/large/`INT_MAX`; 10 out-edges; self route; both directions | `cfg_prog_add_routes` | [x] |
| 39 | choice 3 | `print_graph` with 0/1/many cities and mixed edge counts | `cfg_prog_show_all` | [x] |
| 40 | choice 4 | city details for first/middle/last/missing city | `cfg_prog_show_details` | [x] |
| 41 | choice 5 | shortest path: same city, adjacent, multi-hop, tie, unreachable, reverse direction | `cfg_prog_shortest_path` | [x] |
| 42 | choice 6 | shallow copy from a leaf, from a hub, in a cycle; repeated | `cfg_prog_shallow_copy` | [x] |
| 43 | choice 7 | delete with `ref_count > 1` (never reaches 0 → fully defined) | `cfg_prog_delete_refcount_high` | [x] |
| 44 | choices 3/4/6 after 7 | `ref_count` drops to 0 → node freed while still in `graph->nodes[]` (C UB; glibc-behaviour model) | `cfg_prog_delete_to_zero_then_reuse` | [x] |
| 45 | mixed | stdout/stderr interleaving: a script that triggers library stderr messages between buffered stdout writes | `cfg_prog_stream_interleaving` | [x] |
| 46 | mixed | > 4096 bytes of stdout (crosses glibc's fully-buffered flush boundary) | `cfg_prog_large_output` | [x] |
| 47 | menu | 256-byte `fgets` boundary: menu line and city name of length 254/255/256/257/600 | `cfg_prog_fgets_boundary` | [x] |
| 48 | menu | last line without a trailing newline | `cfg_prog_no_final_newline` | [x] |
| 49 | whole program | randomized command scripts (300 runs × 80 commands) over choices 1–8 with random names/distances, never driving a `ref_count` to 0 | `cfg_prog_random_scripts` | [x] |
| 50 | whole program | randomized command scripts including choice 7 (may free nodes) — compared as far as the C is deterministic | `cfg_prog_random_scripts_with_delete` | [x] |
| 51 | choice 2 + 5 | distances near `INT_MAX` on every edge, so the Dijkstra relaxation overflows (randomized: 2-6 cities, 0-3 out-edges each, every start/end pair) | `cfg_prog_overflow_stack_overrun` | [x] |

## Results

`./run_all.sh` (both build configurations):

* `tests/ffi_diff.rs` — 52 cases, all passing. Rows 1-34 above plus the
  `ERRORS.md` library rows; ~4 800 individual C-vs-Rust comparisons of return
  values, `node_t`/`graph_t` memory, stdout and stderr.
* `tests/program_diff.rs` — 26 cases, all passing. Rows 35-51 above plus the
  `ERRORS.md` program rows.
  * row 49: 300 randomized scripts, **7 170 007 / 7 170 007 stdout bytes
    compared byte-for-byte** (0 scripts truncated).
  * row 50: 200 randomized scripts that deliberately release nodes; 189 of them
    reach the C's use-after-free and are compared up to that point.
  * row 51: 200 randomized scripts with near-`INT_MAX` distances; 30 of them
    drive the C into the `path[]` stack overrun.

The suite was validated by mutation: reverting the `strncpy` zero-padding in
`src/ffi.rs`, changing one stderr message's capitalisation, replacing
`path[count - 1 - i]` with `path[i]`, turning the Dijkstra tie-break `<` into
`<=` in `src/dag_lib.rs`, and changing `STDIO_BUFSIZE` from 4096 to 1024 in
`src/cio.rs` are each detected by at least one test.
