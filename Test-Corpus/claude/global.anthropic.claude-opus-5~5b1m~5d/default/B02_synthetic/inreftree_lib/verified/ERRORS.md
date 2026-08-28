# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (197 lines) — every `return` that
signals a failure, every guard, every sentinel, every explicit constant limit,
every implicit-truncation site. There are **no** `assert`s, no `errno` use, and
no error enum in this library; failures are signalled with the sentinels
`-1` (`int`), `0` (`int`), `NULL` (`TreeNode*`) and `OP_ADD` / `add_op`
(fallback values).

Constants: `MAX_NODES = 50`, label capacity `32` with `strncpy(...,31)` +
forced `label[31] = '\0'`, sentinel "no child"/"no parent" = `-1`,
`Operation` valid range = `1..=5`.

Every row below has a differential test in `tests/phase_c_errors.rs` (or, for
the rows that terminate the process, `tests/phase_c_crash.rs`) that constructs
exactly that condition, calls **both** `.so`s through `dlsym`, and asserts the
returned sentinel is identical (not merely "both failed").

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `divide_op` | `b == 0` (`lib.c:64`) | returns `0`, no trap | [x] |
| 2 | `modulo_op` | `b == 0` (`lib.c:69`) | returns `0`, no trap | [x] |
| 3 | `find_node_by_id` | loop completes without an `id` match (`lib.c:79`) — id absent from `node_table[0..node_count]` | returns `NULL` | [x] |
| 4 | `find_node_by_id` | `node_count == 0` (empty table): loop body never runs | returns `NULL` | [x] |
| 5 | `find_node_by_id` | `node_count < 0` (negative count set through the exported `node_count` symbol): `i < node_count` false immediately | returns `NULL` | [x] |
| 6 | `add_tree_node` | `node_count >= MAX_NODES` i.e. `>= 50` (`lib.c:83`) — table full | returns `-1`, `node_table` and `node_count` left completely unmodified | [x] |
| 7 | `add_tree_node` | `parent_id != -1` and no node with that id exists → `parent == NULL` (`lib.c:98`) | returns `-1`, **but** `node_table[node_count]` has already been overwritten with the new node (id/value/parent_id/-1/-1/label) and `node_count` is **not** incremented | [x] |
| 8 | `add_tree_node` | `parent_id != -1`, `find_node_by_id` returns non-NULL but `parent->id != parent_id` (`lib.c:98`, second disjunct) — unreachable by construction because `find_node_by_id` only returns id-matching nodes; documented as dead code | same as row 7 (`-1`) if ever reached | [x] |
| 9 | `add_tree_node` | `label` longer than 31 bytes (`strncpy(...,31)` + `label[31]='\0'`, `lib.c:93-94`) | silent truncation to 31 bytes + NUL; no error returned | [x] |
| 10 | `add_tree_node` | `label` exactly 31 bytes (boundary, one below capacity) | 31 bytes copied, `label[31]='\0'`; bytes 31.. are NOT padded beyond index 31 | [x] |
| 11 | `add_tree_node` | `label == ""` (empty) | whole 32-byte field NUL-filled (strncpy pads to n=31, then `[31]=0`) | [x] |
| 12 | `add_tree_node` | `parent_id == -1` (the "no parent" sentinel) — parent lookup skipped entirely | success; returns `node_count-1`; no parent link written | [x] |
| 13 | `add_tree_node` | parent already has BOTH `left_child_id != -1` and `right_child_id != -1` (`lib.c:102-106`, no `else`) | success (`node_count-1`), but the child link is **silently dropped** | [x] |
| 14 | `add_tree_node` | `id` duplicates an existing node's id | success; no duplicate check exists. `find_node_by_id` will thereafter return the FIRST match only | [x] |
| 15 | `add_tree_node` | `parent_id` equals the id of the node being inserted right now | `-1` — the new node is at index `node_count`, which the parent search (`i < node_count`) cannot see, so the parent is "not found" | [x] |
| 16 | `calculate_tree_sum` | `node_id` not present (`node == NULL`, `lib.c:116`) | returns `0` (indistinguishable from a real sum of 0) | [x] |
| 17 | `calculate_tree_sum` | `node_id == -1` (the child sentinel used as an id) | returns `0` via the `NULL` branch (unless a node with id `-1` was inserted) | [x] |
| 18 | `calculate_tree_sum` | `node != NULL` but `node->id != node_id` (`lib.c:116`, second disjunct) — unreachable, `find_node_by_id` guarantees the match; documented as dead code | `0` if ever reached | [x] |
| 19 | `calculate_tree_sum` | `node_count == 0` (empty table), any id | returns `0` | [x] |
| 20 | `parse_operation` | `op_str == NULL` (`lib.c:134`) — **accepted**, short-circuits before `strchr` | returns `OP_ADD` (`1`); must NOT crash | [x] |
| 21 | `parse_operation` | string with no `+ * - / %` at all (fall-through, `lib.c:149`) | returns `OP_ADD` (`1`) | [x] |
| 22 | `parse_operation` | empty string `""` | returns `OP_ADD` (`1`) — every `strchr` returns `NULL` | [x] |
| 23 | `get_operation_func` | `op == 0` (below the valid enum range) — `default:` (`lib.c:159`) | returns `add_op` | [x] |
| 24 | `get_operation_func` | `op == 6` (one past `OP_MODULO`) — `default:` | returns `add_op` | [x] |
| 25 | `get_operation_func` | `op == -1` (negative out-of-range enum value across the FFI boundary) | returns `add_op` | [x] |
| 26 | `get_operation_func` | `op == INT_MIN` / `op == INT_MAX` (extreme out-of-range enum values; C enums accept any `int`) | returns `add_op` | [x] |
| 27 | `inreftree` | selected `target` is `NULL` (`lib.c:180`, first disjunct) — unreachable in practice because the `'l'`-scan always finds `"left"` (id 2); reached only if `target_id` stayed `-1` | `target_id` forced to `1` | [x] |
| 28 | `inreftree` | selected `target->value == 0` (`lib.c:180`, second disjunct) — i.e. `param2 == 0` | `target_id` forced to `1` (instead of `2`) | [x] |
| 29 | `inreftree` | `tree_sum < 0` ⇒ `tree_sum % 4 ∈ {-1,-2,-3}` ⇒ `op_string[negative]` reads the three `.rodata` bytes **before** the `"+*-%"` literal (`lib.c:187`) — out-of-bounds read | with the CMake/gcc layout the preceding bytes are `'f'`, `'t'`, `'\0'` (tail of the `"left-left"` literal), none of which is an operator, so `parse_operation` yields `OP_ADD` for every negative `tree_sum` | [x] |
| 30 | `inreftree` | `tree_sum == INT_MIN` (`INT_MIN % 4 == 0`) | `op_string[0] == '+'` ⇒ `OP_ADD` | [x] |
| 31 | `inreftree` | signed overflow while summing the four params (`param1+param2+param3+param4`) — UB in C, wraps at `-O0`/`-O2` on x86-64 | wrapped 32-bit result, then the normal `% 4` dispatch | [x] |
| 32 | `divide_op` | `a == INT_MIN && b == -1` — signed-division overflow, UB in C; gcc emits `idiv`, the CPU raises `#DE` | process terminated by **SIGFPE** (exit status 136) | [x] |
| 33 | `modulo_op` | `a == INT_MIN && b == -1` — same `idiv`, same trap | process terminated by **SIGFPE** (exit status 136) | [x] |
| 34 | `add_tree_node` | `label == NULL` — `strncpy(dst, NULL, 31)` dereferences NULL | process terminated by **SIGSEGV** (exit status 139) | [x] |
| 35 | `add_tree_node` | insert id `X` as a root, then insert **another** node with the same id `X` naming `X` as its parent — `find_node_by_id(X)` returns the *first* slot, whose left slot is free, so it stores `left_child_id = X`: a link to itself | accepted (`node_count-1`); the table now contains a **cycle**, and `calculate_tree_sum(X)` recurses for ever. Reachable purely through the public API | [x] |

### Note on row 29 (the out-of-bounds `.rodata` read)

The three bytes that precede the `"+*-%"` literal were measured for
gcc/clang × `-O0 -O1 -O2 -O3 -Os` (10 builds of the unmodified
`c_src/CMakeLists.txt`):

| build | bytes at `op_string[-3..0]` |
|-------|------------------------------|
| gcc `-O0 -O1 -O2 -O3 -Os`, clang `-O0 -Os` | `'f'`, `'t'`, `'\0'` (tail of `"left-left"`) |
| clang `-O1 -O2 -O3` | `'\0'`, `'\0'`, `'\0'` (string pool is reordered) |

None of those bytes is one of `+ * - / %`, so **`parse_operation` yields
`OP_ADD` for every negative `tree_sum` under every one of those builds** — the
Rust translation's behaviour is therefore layout-independent even though the
read itself is not. The Rust side reproduces the default CMake build (gcc
`-O0`) byte-for-byte by keeping the five string literals in one `.rodata` blob
in emission order.

## Rows deliberately NOT differentially executed (memory-corrupting UB)

These are recorded for completeness; running them would corrupt or crash the
harness in a way that carries no comparable "result", and the C is the ground
truth precisely because it is *undefined*:

| condition | why not executed |
|-----------|------------------|
| `node_count` set `> 50` through the exported symbol, then `find_node_by_id` / `inreftree` | reads `node_table` out of bounds; both libraries read whatever follows their own `.bss`, which is by construction different data |
| `node_count` set `< 0` then `add_tree_node` | writes `node_table[negative]`, i.e. corrupts each library's own `.bss` before the array |
| `calculate_tree_sum` on a cycle (row 35, or any `node_count` that exposes a still-zeroed slot, whose `id == 0` and `left_child_id == 0` make it point at itself) | unbounded recursion → stack overflow in both. The *signal* differs only because the host process differs: a C harness dies with SIGSEGV, while a Rust harness's `std` stack-overflow guard turns it into SIGABRT — an artifact of the loader, not of the library. The tests instead assert that both libraries build the **same cyclic table** (`err35`) and skip the recursion itself via `Pair::diff_sum` |
| `parse_operation` with a non-NUL-terminated buffer | `strchr` reads past the buffer |

Note that row 5 (`node_count < 0` + `find_node_by_id`) *is* executed, because
the negative count makes the loop body unreachable, so no out-of-bounds access
happens.
