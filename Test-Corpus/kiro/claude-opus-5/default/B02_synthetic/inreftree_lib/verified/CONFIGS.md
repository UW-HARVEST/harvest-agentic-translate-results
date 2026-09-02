# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on, not from what looks important.

## Axis inventory (from the source)

There is no init/config object, no flags word, and no `#ifdef` in `lib.c`, and
`translation/Cargo.toml` has no `[features]` table. The "runtime options" of this
library are therefore the two **writable exported data objects** plus the
argument shapes:

- **A1 — `node_count` (exported, writable)**: the library's only mode switch. It
  bounds every `find_node_by_id` scan (line 74) and the `inreftree` label scan
  (line 172), and gates `add_tree_node` (line 83). Distinct states:
  `0` / `1` / `2..48` / `49` / `50` (full).
- **A2 — `node_table` contents (exported, writable)**: `id`s unique vs.
  duplicated (line 75 returns the *first* match), `left_child_id` /
  `right_child_id` set vs. `-1` (lines 122, 126), `value` zero vs. non-zero
  (line 180), label containing `'l'` vs. not (line 173).
- **A3 — `add_tree_node` `parent_id` shape**: `-1` (root, skips the whole
  linking block, line 96) vs. an existing id vs. a *duplicated* existing id.
- **A4 — parent's child slots**: both free -> fills `left_child_id`; left taken
  -> fills `right_child_id`; both taken -> **neither is written and the call
  still succeeds** (line 102/104 have no `else`).
- **A5 — `label` length**: `""` (empty), 1, 4 ("root"), 30, exactly 31, 32, and
  > 32 — the `strncpy(.., 31)` + `label[31]=0` boundary (lines 92-93).
- **A6 — tree shape for `calculate_tree_sum`**: absent node, single leaf, one
  child (left only / right only), two children, deep chain, wide tree, the
  4-node shape `inreftree` builds.
- **A7 — `parse_operation` string shape**: which operator chars are present and
  in what position; the check order `+`, `*`, `-`, `/`, `%` (lines 134-148) means
  a string with several operators resolves to the *earliest check*, not the
  earliest character.
- **A8 — `get_operation_func` op value**: each of `1..=5`, plus the `default:`
  fallback set.
- **A9 — operand magnitude for the five `*_op` functions**: zeros, positives,
  negatives, mixed signs, `INT_MIN` / `INT_MAX` (wrapping overflow), and the
  `b == 0` / `b == -1` special cases.
- **A10 — `inreftree` parameter shape**: which of the four params are zero
  (only `param2` changes control flow, line 180), and the value of
  `tree_sum % 4` in `{0, 1, 2, 3, -1, -2, -3}` (line 189), plus sums that
  overflow `int`.

Entry points: **all 11** exported functions are covered, driven directly through
the `.so`, not only through the `inreftree` one-shot wrapper.

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (`SEED = 0x5EED_1234_ABCD_F00D`), not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `add_op` | A9: randomized `(a,b)` over the full `i32` range, 20k pairs | [x] |
| 2 | `add_op` | A9: boundary grid `{INT_MIN, -1, 0, 1, INT_MAX}²` — wrapping overflow both directions | [x] |
| 3 | `add_op` | A9: `unused1`/`unused2` set to junk (`INT_MIN`, random) — must be ignored | [x] |
| 4 | `multiply_op` | A9: randomized `(a,b)`, 20k pairs (wrapping multiply) | [x] |
| 5 | `multiply_op` | A9: boundary grid `{INT_MIN,-1,0,1,INT_MAX}²` + large-magnitude pairs that overflow | [x] |
| 6 | `subtract_op` | A9: randomized `(a,b)`, 20k pairs | [x] |
| 7 | `subtract_op` | A9: boundary grid, incl. `0 - INT_MIN` and `INT_MIN - 1` | [x] |
| 8 | `divide_op` | A9: randomized `(a,b)` with `b != 0`, mixed signs (C truncates toward zero) | [x] |
| 9 | `divide_op` | A9: `b == ±1`, `a == INT_MIN` with `b != -1`, `a == INT_MAX`, `|a| < |b|` | [x] |
| 10 | `modulo_op` | A9: randomized `(a,b)` with `b != 0`, mixed signs (C `%` keeps the dividend's sign) | [x] |
| 11 | `modulo_op` | A9: `b == ±1`, `a == INT_MIN` with `b != -1`, `|a| < |b|`, exact multiples | [x] |
| 12 | `find_node_by_id` | A1=0 (empty table) + randomized ids | [x] |
| 13 | `find_node_by_id` | A1=1, A2 unique id: hit on the single entry, and misses | [x] |
| 14 | `find_node_by_id` | A1=50 (full), A2 unique ids: hit at index 0, mid, 49; returned pointer's *offset* into `node_table` must match | [x] |
| 15 | `find_node_by_id` | A1=50, A2 **duplicated** ids: must return the FIRST match (line 75) | [x] |
| 16 | `find_node_by_id` | A1 truncated below the table's real contents (e.g. table filled to 50, `node_count` lowered to 10) — entries `>= node_count` must be invisible | [x] |
| 17 | `add_tree_node` | A3=-1 (root), A5 random label: fresh table, sequential appends 0..49 | [x] |
| 18 | `add_tree_node` | A3=existing parent, A4 both slots free -> writes `left_child_id` | [x] |
| 19 | `add_tree_node` | A3=existing parent, A4 left taken -> writes `right_child_id` | [x] |
| 20 | `add_tree_node` | A3=existing parent, A4 **both taken** -> succeeds, writes neither slot | [x] |
| 21 | `add_tree_node` | A3=**duplicated** parent id -> links under the FIRST matching parent | [x] |
| 22 | `add_tree_node` | A5: label `""` / 1 / 30 / 31 / 32 / 64 bytes — full 32-byte `label` field compared byte-for-byte incl. zero padding | [x] |
| 23 | `add_tree_node` | A5: label written over a slot that already held a LONGER label — `strncpy` zero-padding must scrub the stale tail | [x] |
| 24 | `add_tree_node` | A1: fill from 0 to 49 with randomized ids/values, comparing the whole 2600-byte `node_table` image + `node_count` after every call | [x] |
| 25 | `add_tree_node` | A3=`-1` with a *self-referential* id already present (duplicate ids allowed) | [x] |
| 26 | `calculate_tree_sum` | A6: absent id on a non-empty table | [x] |
| 27 | `calculate_tree_sum` | A6: single leaf (both children `-1`), randomized `value` | [x] |
| 28 | `calculate_tree_sum` | A6: left child only, and right child only | [x] |
| 29 | `calculate_tree_sum` | A6: both children, 2 levels, randomized values incl. overflowing sums | [x] |
| 30 | `calculate_tree_sum` | A6: deep left chain of 50 nodes (max depth the table allows) | [x] |
| 31 | `calculate_tree_sum` | A6: child id pointing at a node that does NOT exist (`!= -1` but unresolvable) -> contributes 0 | [x] |
| 32 | `calculate_tree_sum` | A6: randomized *forests* built via `add_tree_node`, summed from every root | [x] |
| 33 | `calculate_tree_sum` | A6: child id pointing at a node with a DUPLICATE id -> resolves to the first | [x] |
| 34 | `node_table` / `node_count` | A1/A2 as raw exported data: write the full 2600-byte image + `node_count` into both `.so`s and require identical reads back (defined range `0..=50` only) | [x] |
| 35 | `parse_operation` | A7: single-char strings for each of `+ * - / %` | [x] |
| 36 | `parse_operation` | A7: `""` and randomized strings drawn from a non-operator alphabet | [x] |
| 37 | `parse_operation` | A7: operator not in first position (`"ab+cd"`), and at the last position | [x] |
| 38 | `parse_operation` | A7: **multiple** operators — check-order precedence, e.g. `"%/-*+"` must give `OP_ADD`, `"%/-*"` -> `OP_MULTIPLY`, `"%/-"` -> `OP_SUBTRACT`, `"%/"` -> `OP_DIVIDE` | [x] |
| 39 | `parse_operation` | A7: randomized strings over the alphabet `+*-/%a1 ` (length 0..16), 20k strings — exercises the precedence cross-product | [x] |
| 40 | `get_operation_func` | A8: `op` in `1..=5` — identity of the returned fn ptr probed by calling it with a discriminating pair `(10, 3)` -> `13/30/7/3/1` | [x] |
| 41 | `get_operation_func` | A8: returned fn ptr called with randomized operands, cross-checked against the directly-exported `*_op` symbol of the same `.so` | [x] |
| 42 | `get_operation_func` | A8: randomized `op` over the full `i32` range (mostly the `default:` arm) | [x] |
| 43 | `inreftree` | A10: randomized `(p1,p2,p3,p4)` over the full `i32` range, 20k tuples | [x] |
| 44 | `inreftree` | A10: `tree_sum % 4` forced to each of `0,1,2,3` (positive sums) -> `add/mul/sub/mod` | [x] |
| 45 | `inreftree` | A10: `tree_sum % 4` forced to each of `-1,-2,-3` (negative sums) -> `.rodata` under-read path | [x] |
| 46 | `inreftree` | A10: `tree_sum == 0` exactly, via several different param combinations | [x] |
| 47 | `inreftree` | A10: `param2 == 0` (retarget to id 1) crossed with each `tree_sum % 4` value | [x] |
| 48 | `inreftree` | A10: params at `INT_MIN` / `INT_MAX` so the sum wraps, crossed with `param2 == 0` | [x] |
| 49 | `inreftree` | A10: called repeatedly (state carry-over) and interleaved with `add_tree_node` calls that dirty `node_table` first | [x] |
| 50 | `inreftree` | A10: post-conditions — full `node_table` image and `node_count` compared after the call, not just the return value | [x] |
| 51 | composed pipeline | `add_tree_node`* -> `find_node_by_id` -> `calculate_tree_sum` -> `parse_operation` -> `get_operation_func` -> call, hand-assembled from the low-level exports on randomized 1..50-node trees (the pipeline `inreftree` composes, driven from outside) | [x] |
| 52 | composed pipeline | randomized long op sequences (fuzz driver): random choice among all 11 entry points, 5k steps, comparing every return value + the whole `node_table`/`node_count` state after each step | [x] |
| 53 | `add_tree_node` | A2: writes over a **poisoned** (non-zero) table — every field the C stores must be stored, incl. the forced `label[31] = '\0'`; label lengths 0/1/5/29/30/31/32/33/60 | [x] |
| 54 | `add_tree_node` | A1 x A2: poisoned table with `node_count` at every index `0..=49`, one write per slot, full image compared | [x] |
| 55 | `inreftree` | A2: run over a poisoned table (`inreftree` resets only `node_count`, never the bytes), with a randomized starting `node_count` | [x] |

## Rows 53-55: why "poison" matters

Rows 1-52 all start from a zeroed `node_table`, which cannot tell "stored a 0"
apart from "stored nothing". Pre-filling both tables with a non-zero pattern
(`Pair::poison_both`) makes every omitted store visible in the image comparison.
This is not hypothetical: the mutation `label[31] NUL not forced` survives the
entire zero-initialised suite and is caught only by rows 53-55. See
`mutate.sh`.

## Input shapes deliberately excluded

A `left_child_id` / `right_child_id` that resolves back to an ancestor (reachable
by giving two table entries the same `id` and chaining a parent through them, or
by writing `node_table` directly) makes `calculate_tree_sum` recurse until the
stack is exhausted. The Rust translation does exactly the same thing, so this is
a property of the *input*, not a divergence — and it cannot be compared
in-process because it kills the harness. `Lib::sum_terminates` mirrors the
traversal with a step budget and the affected rows skip those inputs.

Likewise `add_tree_node` is never called with a negative `node_count`: the C
would evaluate `&node_table[node_count]` and write *before* the array. The two
libraries place `node_count` on opposite sides of `node_table`
(C: `node_table` then `node_count`; Rust: the reverse), so the corrupted bytes
would be unrelated and the write could take out the test process.

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so `cargo test` and
`cargo test --no-default-features` cover the complete configuration space.
`check_features.sh` parses the manifest, enumerates the power set of whatever
features it finds, and runs the build + `check_symbols.sh` + the full suite for
each; with no features declared that is 2 combinations, both passing.

Both cargo **profiles** are also verified: `[profile.dev]` sets
`debug-assertions = false` / `overflow-checks = false`, because Rust's debug-only
UB checks would otherwise turn the C library's deliberate undefined behavior
(the NULL-`label` fault) into a Rust panic in debug builds while release faulted
correctly. The suite passes against both the release and the debug `.so`
(`TRANSLATION_SO=target/debug/libinreftree_lib.so cargo test --release`).
