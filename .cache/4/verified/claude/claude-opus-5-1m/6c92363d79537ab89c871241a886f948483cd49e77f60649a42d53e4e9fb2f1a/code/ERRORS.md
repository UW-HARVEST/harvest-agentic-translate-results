# ERRORS.md — error-surface table (Phase C)

Mechanically derived from every `return NULL` / `return -1` / `return;` guard,
every `fprintf(stderr, ...)`, every `printf("NULL ...")` and every explicit
range / bound / null check in `c_src/src/lib.c` and `c_src/src/main.c`.
There are no `assert()`s and no error enums in this library.

Constants that define the ranges: `MAX_CITY_NAME 64`, `MAX_EDGES 10`,
`MAX_NODES 100` (`dag_lib.h`), `MAX_INPUT 256` (`main.c`), `INT_MAX`
(`find_shortest_path` sentinel distance).

`L#` = C source line. `[x]` = covered by a passing differential test.

## Library (`lib.c`) — exercised through both `.so`s by `tests/ffi_diff.rs`

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `create_graph` L35 | `malloc(sizeof(graph_t))` returns NULL | stderr `Error: Failed to allocate graph\n`, return `NULL` | not reachable without an allocator fault injector — documented, not tested | [n/a] |
| 2 | `add_node` L50 | `graph == NULL` (`city_name` valid) | stderr `Error: NULL parameter in add_node\n`, return `NULL` | `err_add_node_null_graph` | [x] |
| 3 | `add_node` L50 | `city_name == NULL` (`graph` valid) | stderr `Error: NULL parameter in add_node\n`, return `NULL` | `err_add_node_null_name` | [x] |
| 4 | `add_node` L50 | both `graph` and `city_name` NULL | stderr `Error: NULL parameter in add_node\n`, return `NULL` | `err_add_node_null_both` | [x] |
| 5 | `add_node` L55 | `graph->node_count >= MAX_NODES` (101st distinct city) | stderr `Error: Graph is full (max 100 nodes)\n`, return `NULL` | `err_add_node_graph_full` | [x] |
| 6 | `add_node` L62 | `city_name` equals an existing `nodes[i]->city_name` | stderr `Error: Node '<name>' already exists\n`, return `NULL` | `err_add_node_duplicate` | [x] |
| 7 | `add_node` L62 | duplicate only *after* `strncpy` truncation (two names sharing the first 63 bytes) | second call still succeeds (the stored name is the truncated one, so the *third* identical call is the duplicate) | `err_add_node_duplicate_truncated` | [x] |
| 8 | `add_node` L70 | `malloc(sizeof(node_t))` returns NULL | stderr `Error: Failed to allocate node\n`, return `NULL` | not reachable — documented, not tested | [n/a] |
| 9 | `add_edge` L89 | `from == NULL` | stderr `Error: NULL node in add_edge\n`, return `-1` | `err_add_edge_nulls` | [x] |
| 10 | `add_edge` L89 | `to == NULL` | stderr `Error: NULL node in add_edge\n`, return `-1` | `err_add_edge_nulls` | [x] |
| 11 | `add_edge` L89 | both NULL — checked *before* the negative-distance check, so a NULL node with `distance < 0` still reports the NULL error | stderr `Error: NULL node in add_edge\n`, return `-1` | `err_add_edge_nulls` | [x] |
| 12 | `add_edge` L94 | `from->edge_count >= MAX_EDGES` (11th edge) — checked *before* the distance check | stderr `Error: Node '<from name>' has maximum edges\n`, return `-1` | `err_add_edge_max_edges` | [x] |
| 13 | `add_edge` L94 | full node *and* negative distance → the "maximum edges" message wins | stderr `Error: Node '<from>' has maximum edges\n`, return `-1` | `err_add_edge_max_edges` | [x] |
| 14 | `add_edge` L99 | `distance < 0` (`-1`, `INT_MIN`) | stderr `Error: Negative distance not allowed\n`, return `-1` | `err_add_edge_negative_distance` | [x] |
| 15 | `add_edge` L105 | `to` already present in `from->edges[0..edge_count]` (same distance or different) | stderr `Error: Edge already exists\n`, return `-1` | `err_add_edge_duplicate` | [x] |
| 16 | `add_edge` L105 | duplicate *self* edge (`from == to` twice) | first call succeeds, second: stderr `Error: Edge already exists\n`, `-1` | `err_add_edge_duplicate` | [x] |
| 17 | `delete_node` L122 | `node == NULL` | silent `return`, no output, no crash | `err_delete_node_null` | [x] |
| 18 | `increment_refs_recursive` L136 | `node == NULL` (a NULL `edges[i].destination`) | silent `return` | unreachable through the public API (`add_edge` rejects NULL destinations) — covered indirectly by `err_add_edge_nulls`, documented | [n/a] |
| 19 | `increment_refs_recursive` L147 | `*visited_count >= MAX_NODES` — node is *not* recorded as visited | ref counts of already-visited nodes may be bumped again | unreachable: `graph_t` holds at most `MAX_NODES` distinct nodes, documented | [n/a] |
| 20 | `shallow_copy` L162 | `start == NULL` | stderr `Error: NULL node in shallow_copy\n`, return `NULL` | `err_shallow_copy_null` | [x] |
| 21 | `find_shortest_path` L187 | `start == NULL` | stderr `Error: NULL parameter in find_shortest_path\n`, `NULL`; `*path_length` **untouched** | `err_fsp_nulls` | [x] |
| 22 | `find_shortest_path` L187 | `end == NULL` | same as #21, `*path_length` untouched | `err_fsp_nulls` | [x] |
| 23 | `find_shortest_path` L187 | `path_length == NULL` | same message, `NULL` returned, no store | `err_fsp_nulls` | [x] |
| 24 | `find_shortest_path` L215 | `current_idx == -1` — `current` not found in `state` | `break` out of the Dijkstra loop | unreachable: `current` is only ever taken from `state[]`, documented | [n/a] |
| 25 | `find_shortest_path` L276 | `end_idx == -1` (`end` never reached, i.e. not in `state`) | stderr `No path found\n`, `*path_length = 0`, return `NULL` | `err_fsp_unreachable` | [x] |
| 26 | `find_shortest_path` L276 | `state[end_idx].distance == INT_MAX` (`end` was added as a neighbour but never relaxed below `INT_MAX`) | stderr `No path found\n`, `*path_length = 0`, `NULL` | `err_fsp_end_seen_but_infinite` | [x] |
| 27 | `find_shortest_path` L240 | `state_count >= MAX_NODES` — new neighbour cannot be recorded, `neighbor_idx` stays `-1`, relaxation skipped | that neighbour is silently ignored (can turn into "No path found") | `cfg_fsp_state_full` (CONFIGS row 24) | [x] |
| 28 | `find_shortest_path` L299 | `current_state_idx == -1` during path reconstruction | `break`, path truncated | unreachable: every `previous` value is a `state[].node`, documented | [n/a] |
| 29 | `find_shortest_path` L308 | `malloc(sizeof(node_t*) * count)` returns NULL | stderr `Error: Failed to allocate path\n`, `*path_length = 0`, `NULL` | not reachable — documented, not tested | [n/a] |
| 30 | `get_node_by_name` L324 | `graph == NULL` | return `NULL`, **no output at all** | `err_get_node_nulls_and_misses` | [x] |
| 31 | `get_node_by_name` L324 | `city_name == NULL` | return `NULL`, no output | `err_get_node_nulls_and_misses` | [x] |
| 32 | `get_node_by_name` L328 | name not present (incl. empty graph, empty string, 63+ byte name) | return `NULL`, no output | `err_get_node_nulls_and_misses` | [x] |
| 33 | `print_node` L339 | `node == NULL` | **stdout** `NULL node\n` (not stderr) | `err_print_and_free_nulls` | [x] |
| 34 | `print_graph` L355 | `graph == NULL` | **stdout** `NULL graph\n` | `err_print_and_free_nulls` | [x] |
| 35 | `free_graph` L368 | `graph == NULL` | silent `return` | `err_print_and_free_nulls` | [x] |

## Program (`main.c`) — exercised through both `driver`s by `tests/program_diff.rs`

| # | site | trigger | expected C result | test | [x] |
|---|------|---------|-------------------|------|-----|
| 36 | `main` L48 | `create_graph()` returns NULL | stderr `Failed to create graph\n`, exit status `1` | not reachable — documented, not tested | [n/a] |
| 37 | `main` L62 | EOF at the menu prompt (empty stdin) | loop `break` → `free_graph` → exit `0` | `cfg_prog_empty_stdin` | [x] |
| 38 | `main` L66 | `sscanf(input, "%d", &choice) != 1` (`abc`, `""`+`\n`, `+`, `-`, `x1`) | stdout `Invalid input\n`, `continue` | `err_prog_invalid_input` | [x] |
| 39 | `main` L257 | `choice` outside 1..8 (`0`, `9`, `-1`, `2147483647`, `-2147483648`) | stdout `Invalid choice\n` | `err_prog_invalid_choice` | [x] |
| 40 | `main` L66 | leading spaces / trailing garbage accepted by `%d` (`  3xyz`) | parsed as `3` | `err_prog_invalid_input` | [x] |
| 41 | `main` L66 | value out of `int` range (`99999999999999`) — glibc `%d` stores the truncated `long` | choice = truncated value → `Invalid choice` | `err_prog_invalid_input` | [x] |
| 42 | case 1 L75 | EOF at the "Enter city name" prompt | `break` out of the `switch`; the loop then re-prompts, hits EOF again and exits `0` | `err_prog_eof_at_prompts` | [x] |
| 43 | case 1 L85 | `add_node` returned NULL (duplicate / full graph) | stdout `Failed to add city\n` (after the library's stderr line) | `err_prog_route_and_city_failures` | [x] |
| 44 | case 2 L97/L103/L109 | EOF at the from / to / distance prompt | `break`, then EOF exit | `err_prog_eof_at_prompts` | [x] |
| 45 | case 2 L112 | `sscanf` on the distance fails | stdout `Invalid distance\n` | `err_prog_route_and_city_failures` | [x] |
| 46 | case 2 L120 | from-city not found — checked **after** both names *and* the distance are read | stdout `City '<from>' not found\n` | `err_prog_route_and_city_failures` | [x] |
| 47 | case 2 L124 | to-city not found (from exists) | stdout `City '<to>' not found\n` | `err_prog_route_and_city_failures` | [x] |
| 48 | case 2 L132 | `add_edge` returned `-1` (negative distance / duplicate / max edges) | stdout `Failed to add route\n` | `err_prog_route_and_city_failures` | [x] |
| 49 | case 4 L147 | EOF at the "Enter city name" prompt | `break`, then EOF exit | `err_prog_eof_at_prompts` | [x] |
| 50 | case 4 L155 | city not found | stdout `City '<name>' not found\n` | `cfg_prog_show_details` | [x] |
| 51 | case 5 L166/L172 | EOF at the start / end prompt | `break`, then EOF exit | `err_prog_eof_at_prompts` | [x] |
| 52 | case 5 L180/L184 | start / end city not found | stdout `City '<name>' not found\n` | `err_prog_missing_cities` | [x] |
| 53 | case 5 L198 | `find_shortest_path` returned NULL | stderr `No path found\n` (unbuffered, from the library) **and** stdout `No path found\n` | `err_prog_missing_cities` | [x] |
| 54 | case 6 L207 | EOF at the prompt | `break`, then EOF exit | `err_prog_eof_at_prompts` | [x] |
| 55 | case 6 L213 | city not found | stdout `City '<name>' not found\n` | `err_prog_missing_cities` | [x] |
| 56 | case 6 L223 | `shallow_copy` returned NULL | stdout `Failed to create shallow copy\n` | unreachable from `main` (`node` is non-NULL here) — documented | [n/a] |
| 57 | case 7 L232 | EOF at the prompt | `break`, then EOF exit | `err_prog_eof_at_prompts` | [x] |
| 58 | case 7 L238 | city not found | stdout `City '<name>' not found\n` | `err_prog_missing_cities` | [x] |
| 59 | case 7 L244 | `delete_node` drops `ref_count` to 0 → node is `free()`d while still in `graph->nodes[]` | *undefined behaviour* in C (use-after-free); the C prints the pre-decrement `ref_count` and continues | `cfg_prog_delete_to_zero_then_reuse` compares the deterministic prefix only; see note below | [x] |
| 60 | line-buffer boundary L62 | menu line longer than `MAX_INPUT-1` = 255 bytes | `fgets` splits it; the remainder is re-read as the *next* menu line | `cfg_prog_fgets_boundary` | [x] |
| 61 | line-buffer boundary L75 | city name longer than 255 bytes | truncated at 255 by `fgets`, then at 63 by `strncpy`; the tail becomes the next menu line | `cfg_prog_fgets_boundary` | [x] |
| 62 | `strcspn` L80 | input without a trailing newline (last line before EOF) | no newline to strip, whole line used | `cfg_prog_no_final_newline` | [x] |
| 63 | `strcspn` L80 | empty city name (bare `\n`) | `add_node(graph, "")` succeeds; `""` is then a lookup-able city | `err_prog_empty_city_name` | [x] |
| 64 | out-of-range "enum" | the `switch (choice)` receives every value in `-3..=12` plus `INT_MIN`/`INT_MAX` | 1..8 handled, everything else `Invalid choice\n` | `err_prog_invalid_choice` | [x] |

| 65 | `find_shortest_path` L287-L304 (reached from case 5) | distances near `INT_MAX` make `state[idx].distance + edge.distance` overflow, so a node can become its own predecessor; the reconstruction loop then writes past `node_t *path[MAX_NODES]` | *undefined behaviour*: the process dies from a fatal signal. Measured on the same input, 30 runs: SIGSEGV 19x, SIGBUS 11x - so not even the C is reproducible. What *is* reproducible is the stdout flushed before the crash (nothing is flushed by the signal) | `cfg_prog_overflow_stack_overrun` (compares stdout/stderr exactly, requires both to die from a fatal signal) | [x] |
| 66 | `main` L62 | a line that is exactly `MAX_INPUT-1` = 255 bytes plus the newline | the newline is left in the buffer and read as an empty next line -> `Invalid input` | `cfg_prog_fgets_boundary` | [x] |

### Note on rows 59 and the C's undefined behaviour

`delete_node()` `free()`s the node while `graph->nodes[]` still points at it, and
`main.c` keeps calling `get_node_by_name()`/`print_graph()` afterwards. The
result depends on what glibc writes into the freed chunk and on which chunk a
later `malloc()` recycles, i.e. it is not part of the language-defined
behaviour. `src/dag_lib.rs` models the glibc behaviour (tcache LIFO + unsorted
FIFO, and a freed name that can no longer be matched by `strcmp`), and
`tests/program_diff.rs` compares the two programs on such scripts too; where the
C's output is genuinely not reproducible the test only compares the output
produced up to the first divergence-prone operation (see
`UB_SCRIPT_PREFIX_ONLY`). Scripts that never drive a `ref_count` to 0 are
compared in full.

## Results

All 57 reachable rows have a passing differential test; the 9 rows marked
`[n/a]` are unreachable through the public API (documented per row) — 4 of them
require an allocator fault injector (`malloc` returning NULL), the other 5 are
branches the C can never take with any input.

Rows 59 and 65 are the two places where the C's own behaviour is not
reproducible (freed-node contents and the fatal signal of a stack overrun); both
are verified as far as the C is deterministic, which is documented in the test
itself.
