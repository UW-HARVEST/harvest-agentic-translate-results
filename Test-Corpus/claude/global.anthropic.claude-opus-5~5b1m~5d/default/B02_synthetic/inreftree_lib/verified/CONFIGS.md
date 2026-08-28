# CONFIGS.md — Phase A: configuration / valid-input surface table

## Axes the C code actually branches on

`c_src/src/lib.c` has **no** compile-time options (`#ifdef`), **no** runtime
option/flag setters and **no** init/teardown API. `Cargo.toml` declares **no
`[features]`**, so there is exactly one feature combination (`default` == empty
== `--no-default-features`). The "configuration" a caller can set is therefore
the *library-global mutable state* plus the *shape of the input*:

**A. Global state axes** (both are exported data symbols, so a caller can read
*and* write them directly):
* `node_count` — 0 / 1 / few / `MAX_NODES-1` / `MAX_NODES` (=50) / negative.
* `node_table` — contents of the 50×52-byte array, incl. leftovers from a
  previous, partially-failed `add_tree_node`.

**B. Tree-shape axes** (what `add_tree_node` / `calculate_tree_sum` branch on):
* parent mode: `parent_id == -1` (root) vs. existing parent vs. missing parent.
* parent occupancy: left free → left link; left taken, right free → right link;
  both taken → link dropped (no `else`).
* child sentinels: `left_child_id == -1` / `!= -1`, same for right ⇒ 4 recursion
  shapes in `calculate_tree_sum`.
* dangling child id (child link points at an id that is not in the table).
* duplicate ids (first match wins in `find_node_by_id`).
* depth: 1, 2, 3, … 50 (recursion depth of `calculate_tree_sum`).

**C. String-shape axes** (`strncpy` / `strchr` behaviour):
* label length 0, 1, 30, **31** (the `strncpy` bound), 32, >32; NUL padding.
* `parse_operation` haystack: NULL, empty, contains one of `+ * - / %`,
  contains several (tested precedence order `+`, `*`, `-`, `/`, `%`), contains
  none, operator not at index 0, non-ASCII/high-bit bytes.

**D. Scalar-shape axes** (`add/multiply/subtract/divide/modulo_op`, `inreftree`):
* sign combinations (+/+, +/−, −/+, −/−), zero operands, `INT_MIN`, `INT_MAX`,
  `±1`, wrap-around overflow, non-zero values in the two *unused* parameters.
* `inreftree` dispatch: `tree_sum % 4` ∈ {0, 1, 2, 3} (→ `+ * - %`) and
  ∈ {−1, −2, −3} (→ out-of-bounds `.rodata` read, all → `+`).
* `inreftree` target selection: `param2 == 0` (→ `target_id = 1`) vs
  `param2 != 0` (→ `target_id = 2`).

**E. Entry-point axis** — all 11 exported functions are exercised *directly*
through `dlsym`, not only the one-shot wrapper `inreftree` declared in
`include/lib.h`. The low-level ones (`add_tree_node`, `find_node_by_id`,
`calculate_tree_sum`, the five `*_op`s, `parse_operation`,
`get_operation_func`) are driven as a composed pipeline as well, with the full
`node_table`/`node_count` state compared byte-for-byte after every step.

Every row is verified by calling **both** `.so`s and comparing
`(return value, node_count, all 2600 bytes of node_table)`. Rows marked
*randomized* use ≥ 200 pseudo-random inputs from a fixed-seed xorshift64\*
generator (seed printed by the test) so the row covers value-dependent paths,
not one hand-picked value.

## Row table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `add_op` | both operands random full-range `i32`, unused params random non-zero — *randomized* | [x] |
| 2 | `add_op` | boundary grid: {INT_MIN, −2, −1, 0, 1, 2, INT_MAX}² incl. wrapping overflow | [x] |
| 3 | `multiply_op` | random full-range operands, wrapping products — *randomized* | [x] |
| 4 | `multiply_op` | boundary grid {INT_MIN, −1, 0, 1, INT_MAX}², incl. `INT_MIN*−1` wrap | [x] |
| 5 | `subtract_op` | random full-range operands — *randomized* | [x] |
| 6 | `subtract_op` | boundary grid incl. `0−INT_MIN` wrap | [x] |
| 7 | `divide_op` | random operands with `b != 0`, all four sign combinations (C truncating division) — *randomized* | [x] |
| 8 | `divide_op` | boundary grid {INT_MIN, −3, −1, 1, 3, INT_MAX} × same, `b != 0`, excluding the trapping `(INT_MIN,−1)` pair (that is ERRORS row 32) | [x] |
| 9 | `modulo_op` | random operands with `b != 0` — C remainder keeps the sign of the dividend — *randomized* | [x] |
| 10 | `modulo_op` | boundary grid, `b != 0`, excluding trapping `(INT_MIN,−1)` (ERRORS row 33) | [x] |
| 11 | all five `*_op` | unused params 3 and 4 set to random non-zero values — must not affect the result | [x] |
| 12 | `get_operation_func` | `op` = 1,2,3,4,5 → resolves to `add_op`, `multiply_op`, `subtract_op`, `divide_op`, `modulo_op` respectively (compared by symbol index, then invoked and its output compared) | [x] |
| 13 | `parse_operation` | each operator alone: `"+"`, `"*"`, `"-"`, `"/"`, `"%"` | [x] |
| 14 | `parse_operation` | operator not at index 0: `"a+"`, `"xx*"`, `"..-"`, `"zz/"`, `"q%"` | [x] |
| 15 | `parse_operation` | precedence: every 2-subset and the full `"+*-/%"`, plus reversed `"%/-*+"` — `+` beats `*` beats `-` beats `/` beats `%` | [x] |
| 16 | `parse_operation` | random strings of length 0–24 over the alphabet `{+,*,-,/,%,a,z,0,space}` — *randomized* | [x] |
| 17 | `parse_operation` | random byte strings incl. high-bit bytes 0x80–0xFF (signed-`char` comparison in `strchr`) — *randomized* | [x] |
| 18 | `find_node_by_id` | `node_count == 0`, any id → NULL | [x] |
| 19 | `find_node_by_id` | 1 node; probe the matching id and a non-matching id | [x] |
| 20 | `find_node_by_id` | 10 nodes, probe id at index 0 (first), 5 (middle), 9 (last), and an absent id | [x] |
| 21 | `find_node_by_id` | 50 nodes (table exactly full), probe every id 0..=49 plus absent ids | [x] |
| 22 | `find_node_by_id` | duplicate ids present (ids `7,7,7`) → offset of the FIRST match | [x] |
| 23 | `find_node_by_id` | ids that are negative / `0` / `-1` / `INT_MIN` / `INT_MAX` — *randomized* | [x] |
| 24 | `add_tree_node` | root only (`parent_id == -1`), label `"root"`; check return, `node_count`, full table bytes | [x] |
| 25 | `add_tree_node` | parent with left slot free → writes `parent->left_child_id` | [x] |
| 26 | `add_tree_node` | parent with left taken, right free → writes `parent->right_child_id` | [x] |
| 27 | `add_tree_node` | parent with both slots taken → third child linked nowhere, still returns index | [x] |
| 28 | `add_tree_node` | label lengths 0, 1, 5, 30, 31, 32, 40, 64 (crossing the `strncpy(...,31)` bound) — all 52 bytes of the node compared | [x] |
| 29 | `add_tree_node` | label containing `'l'` / not containing `'l'` (drives the `inreftree` scan) and labels containing operator chars | [x] |
| 30 | `add_tree_node` | fill the table one node at a time from 0 to 50 nodes, comparing return value + `node_count` + table bytes after every single insert | [x] |
| 31 | `add_tree_node` | `parent_id == -1` mixed with valid parents in one long randomized insert sequence (ids, values, parents, labels random) — *randomized*, table bytes compared after every insert | [x] |
| 32 | `calculate_tree_sum` | single node, no children (both sentinels `-1`) | [x] |
| 33 | `calculate_tree_sum` | left child only | [x] |
| 34 | `calculate_tree_sum` | right child only | [x] |
| 35 | `calculate_tree_sum` | both children (full binary, depth 2 and depth 3) | [x] |
| 36 | `calculate_tree_sum` | chain of depth 50 (maximum table) — deep recursion | [x] |
| 37 | `calculate_tree_sum` | dangling child id (`left_child_id` set to an id not in the table) → that branch contributes 0 | [x] |
| 38 | `calculate_tree_sum` | negative node values and values chosen so the sum wraps past `INT_MAX`/`INT_MIN` | [x] |
| 39 | `calculate_tree_sum` | random trees: random node count 1–50, random parent choice, random values, then sum every id — *randomized* | [x] |
| 40 | composed pipeline | `add_tree_node`×k → `find_node_by_id` → `calculate_tree_sum` → `parse_operation` → `get_operation_func` → invoke, all through the low-level exports, on random trees — *randomized* | [x] |
| 41 | `inreftree` | `(0,0,0,0)`: `tree_sum == 0` ⇒ `'+'`, `param2 == 0` ⇒ `target_id = 1` | [x] |
| 42 | `inreftree` | `tree_sum % 4 == 0,1,2,3` with `param2 != 0` (`target_id = 2`) — one case each | [x] |
| 43 | `inreftree` | `tree_sum % 4 == 0,1,2,3` with `param2 == 0` (`target_id = 1`) — one case each; note `%`-branch then computes `tree_sum % 1 == 0` | [x] |
| 44 | `inreftree` | negative `tree_sum`, residues −1, −2, −3, with `param2` zero and non-zero (out-of-bounds `.rodata` read path) | [x] |
| 45 | `inreftree` | boundary params: every combination drawn from {INT_MIN, −2, −1, 0, 1, 2, INT_MAX}⁴ (2401 cases, exhaustive) | [x] |
| 46 | `inreftree` | fully random `(p1,p2,p3,p4)` — *randomized*, ≥ 5000 cases | [x] |
| 47 | `inreftree` | called twice in a row / called after the table was dirtied by unrelated `add_tree_node` calls and after `node_count` was left at 50 — verifies the `node_count = 0` reset and the *stale* `node_table` bytes match | [x] |
| 48 | `node_table` / `node_count` | direct read-back of both exported data symbols after every scenario above: 2600 bytes + 4 bytes byte-identical | [x] |
| 49 | feature axis | the crate has no `[features]`; rows 1–53 are re-run under `--no-default-features` and against the `--release` `.so` (different codegen, `panic=abort`) | [x] |
| 50 | `find_node_by_id`, `calculate_tree_sum` | `node_count` (set from outside) LARGER than the number of real inserts, so the scan walks into still-zeroed slots whose `id` field is `0` — `find(0)` therefore succeeds and returns a zeroed slot. Counts 3, 4, 10, 49, 50 × ids 0…4 | [x] |
| 51 | `inreftree` | called with `node_count` pre-set to 0, 1, 6, 49, 50, −1, −7, `INT_MIN`, 123 — the `node_count = 0` reset must dominate whatever was there | [x] |
| 52 | `add_tree_node` | `label` buffer holding exactly 31 non-NUL bytes and **no terminator**, so `strncpy` stops on the count rather than on a NUL — *randomized* over 200 buffers | [x] |
| 53 | all 11 entry points | one long randomized session (8000 steps) interleaving `add_tree_node`, `find_node_by_id`, `calculate_tree_sum`, `parse_operation`, `get_operation_func` + invoke, `inreftree`, table resets and external `node_count` writes; the full 2604-byte state is compared after **every** step — *randomized* | [x] |

Cyclic tables (CONFIGS row 50 / ERRORS row 35) make `calculate_tree_sum` recurse
for ever in *both* libraries; `Pair::diff_sum` predicts that from the raw table
bytes and skips only those ids, after asserting that both libraries agree the
table is cyclic.
