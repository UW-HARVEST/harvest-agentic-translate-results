# CONFIGS.md — configuration-surface table

The library exposes **no** runtime flags, modes or enums: `hashmap.h` and
`tree.h` declare only functions plus the four public structs, and the four
compile-time knobs (`HASHMAP_INITIAL_CAPACITY` 16, `HASHMAP_LOAD_FACTOR` 0.75,
`MAX_CHILDREN` 32, `MAX_DATA_LENGTH` 256) are fixed `#define`s with no
`#ifdef`/`#if` alternative anywhere in `c_src/`. `Cargo.toml` declares no
`[features]`, so there is exactly one build configuration (see the "Feature
combinations" section at the bottom).

The axes the C code actually branches on are therefore *data shapes* and
*sequences of operations*:

* `hashmap.c`: `map == NULL`; `should_resize()` (`(size+deleted_count)/capacity
  > 0.75`); `capacity` after 0/1/2/… doublings; per-slot `occupied` /
  `deleted` / `key ==` in the probe loop; `value == NULL` vs non-NULL;
  probe wrap-around (`(index+probe) % capacity`); probe exhaustion.
* `tree.c`: `tree == NULL`; `has_root == 0` (first node becomes the root and its
  `parent_id` is forced to 0) vs `!= 0`; `data == NULL` vs `strncpy` truncation
  at `MAX_DATA_LENGTH-1`; `child_count >= MAX_CHILDREN`; `id == root_id` in
  `tree_remove_node`; the child-list shift loop; recursion in
  `tree_remove_subtree` / `tree_get_height` / `tree_count_descendants` /
  `tree_print_helper`; `child_count == 0` (leaf) vs `> 0`; `length >
  max_length` in `tree_find_path`; the `length < 1000` cap.
* `main.c`: the 14 exported `test_*` one-shot wrappers and `main`.

Every row below is checked with many pseudo-random inputs (xorshift64\*, fixed
seed `0x243F6A8885A308D3`) unless the row is a fixed structural boundary, and
compares, between the C `.so` and the Rust `.so`: **every return value, the full
`hashmap_t` state (capacity/size/deleted_count and every slot's
key/occupied/deleted/value), the full `tree_t` state
(root_id/has_root/node_count) and every reachable `tree_node_t`
(id/parent_id/child_count/child_ids/NUL-terminated data)** — slot by slot, in
slot order, so a divergent probe sequence or a divergent hash is caught.

`[x]` = row passes. Implemented in `tests/phase_b_valid.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `hashmap_create` + `hashmap_size` + `hashmap_destroy` | fresh map: capacity 16, size 0, deleted 0, all 16 slots zeroed | [x] |
| C2 | `hashmap_put`/`get` | 1 entry; key ∈ {0, 1, 2, 0x7FFF…, 1<<63, u64::MAX, u64::MAX-1} × value non-NULL — one entry per run, 200 random keys | [x] |
| C3 | `hashmap_put`/`get` | 1…13 entries: the resize test runs *before* the insertion, so `(12+0)/16 == 0.75` is *not* `> 0.75` and the 13th entry still fits — capacity must stay 16 | [x] |
| C4 | `hashmap_put`/`get` | 14th insert → `13/16 > 0.75` → first resize to capacity 32 + full rehash; then the next doubling boundary (24, 25 → still 32; 26 → 64) | [x] |
| C5 | `hashmap_put`/`get` | repeated resizes 16→32→64→128→256 at every boundary (49 → 64, 50 → 128, 97 → 128, 98 → 256, 100 → 256); every key looked up afterwards | [x] |
| C6 | `hashmap_put` | keys chosen to collide in one bucket (same `hash % capacity`) → long linear-probe runs, including wrap-around past the end of the table | [x] |
| C7 | `hashmap_put` | duplicate key (update path): value replaced, `size` unchanged, slot reused in place | [x] |
| C8 | `hashmap_put` | `value == NULL` mixed with non-NULL values → occupies a slot, counts in `size`, yet `get`/`contains` report a miss | [x] |
| C9 | `hashmap_remove` | remove existing / colliding / already-removed keys → `deleted` slots, `size--`, `deleted_count++`; later `get` of keys *behind* the tombstone still found | [x] |
| C10 | `hashmap_put` after `hashmap_remove` | re-insert a removed key → "reuse deleted slot" branch (`deleted=0`, `size++`, `deleted_count--`), and insert of a *different* key into a tombstone | [x] |
| C11 | `hashmap_put` + `hashmap_remove` | tombstones drive `should_resize` (`size+deleted_count`) → resize that drops `deleted_count` back to 0 | [x] |
| C12 | `hashmap_clear` | on empty / partly filled / tombstoned / resized (capacity 32+) map → `occupied`/`deleted` cleared but `key`/`value` bytes left intact, capacity kept | [x] |
| C13 | `hashmap_clear` then `put`/`get` | reuse of a cleared map, including keys that were present before the clear | [x] |
| C14 | `hashmap_contains` | present / absent / removed / NULL-valued key on maps of capacity 16 and 128 | [x] |
| C15 | all 8 `hashmap_*` | randomized op sequence: 4000 ops (put/get/remove/contains/size/clear) over a 64-key space, full state compared after **every** op | [x] |
| C16 | `tree_create` + `tree_size` + `tree_delete` | fresh tree: `root_id 0`, `has_root 0`, `node_count 0`, embedded map capacity 16 | [x] |
| C17 | `tree_add_node` (first node) | `has_root == 0` path: `parent_id` argument is **ignored and forced to 0**; root ids ∈ {0, 1, u64::MAX, random} | [x] |
| C18 | `tree_add_node` | root + 1…31 children (`child_count` below the limit), random ids, insertion order preserved in `child_ids` | [x] |
| C19 | `tree_add_node` | root + exactly `MAX_CHILDREN` (32) children — the boundary that still succeeds | [x] |
| C20 | `tree_add_node` | chain of depth 1…50 (each node the only child of the previous) — drives the map through several resizes with node pointers being rehashed | [x] |
| C21 | `tree_add_node` | random tree, 200 nodes, random parents, random ids (incl. 0 and u64::MAX), fan-out up to 32 | [x] |
| C22 | `tree_add_node` | `data` shape: `NULL`, `""`, 1, 10, 254, 255 bytes (exact fit), 256, 257, 300, 1024 bytes (truncated at 255 + NUL) | [x] |
| C23 | `tree_add_node` | `data` bytes: all-`0xFF`, random 0x01..0xFF (invalid UTF-8), `%`/`\` and other `printf` metacharacters | [x] |
| C24 | `tree_get_node` + direct struct reads | every node of every shape above: `id`, `parent_id`, `child_count`, `child_ids[0..child_count]`, `data` | [x] |
| C25 | `tree_remove_node` | leaf; first / middle / last child of a 32-wide parent (child-list shift loop); node with a deep subtree; the root (`id == root_id` branch → `has_root=0`, `root_id=0`) | [x] |
| C26 | `tree_remove_node` + `tree_add_node` | remove root of a populated tree, then add a new root (re-rooting a used map: tombstones + a new `has_root`) | [x] |
| C27 | `tree_remove_node` | remove every node of a random 200-node tree in random order, comparing the full state after every removal | [x] |
| C28 | `tree_get_depth` | root (0), every node of a 50-deep chain, wide tree, node whose id equals `root_id == 0` | [x] |
| C29 | `tree_get_height` | leaf (0), root of a chain (49), root of a random tree, every intermediate node | [x] |
| C30 | `tree_count_descendants` | leaf (0), root of a 200-node random tree, every intermediate node | [x] |
| C31 | `tree_find_path` | `max_length` > len, `== len`, `< len`, `1`, `0`, negative, `i32::MAX`; path of length 1 (root) and length 50; buffer contents beyond the returned length must be untouched | [x] |
| C32 | `tree_print` | empty tree, root only, 50-deep chain (indentation), 32-wide star, complex tree of `main.c`, tree with truncated/high-bit `data`, tree after removals | [x] |
| C33 | `tree_*` + `hashmap_*` mixed | randomized op sequence: 3000 ops (add/remove/get/contains/size/depth/height/descendants/find_path/print) over a 40-id space, full tree+map state and every return value compared after **every** op | [x] |
| C34 | 14 × `test_*` exported one-shot wrappers | each called in a forked child with `stdout`/`stderr` captured separately; bytes + exit status compared | [x] |
| C36 | `tree_add_node` + `tree_find_path` + `tree_get_depth`/`height`/`count_descendants` | chains of 999 / 1000 / 1001 / 1200 nodes — the `temp_path[1000]` / `length < 1000` cap of `tree_find_path` and 1000-deep recursion; plus re-adding a child after `MAX_CHILDREN` was reached and one child removed | [x] |
| C35 | `main` | the whole program driven through the exported `main` symbol of both `.so`s, plus the two built executables (`c_src/build/driver` vs `target/{debug,release}/driver`) | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table and no optional dependencies, so the
complete set of valid feature combinations is:

| # | combination | command | result |
|---|-------------|---------|--------|
| 1 | *(none — this is also the default)* | `cargo check --no-default-features --all-targets`, `cargo test --no-default-features` | [x] |
| 1' | same, `--all-features` (identical, no features exist) | `cargo check --all-features --all-targets`, `cargo test` | [x] |
| 1'' | same, release profile (`panic = "abort"`, optimisations) | `cargo test --release` | [x] |

`c_src/CMakeLists.txt` likewise defines no options, no `target_compile_definitions`
and no build types beyond CMake's default (empty `CMAKE_BUILD_TYPE`, i.e. no
`-DNDEBUG`, so `assert()` in `main.c` is **enabled** — the C `.so` used for the
differential tests is compiled the same way). `grep -rn "#if\|#ifdef\|#ifndef"
c_src/` finds only the two header include guards.

## Row → test mapping

`tests/phase_b_valid.rs` implements the 36 rows as 32 harness rows (some rows
that share a fixture are checked together):

```
C1  C2  C3  C4  C5  C6  C7  C8  C9  C10 C11 C12 C13 C14 C15 C16 C17 C18 C19
C20 C21+C24 C22+C23 C25 C26 C27 C28+C29+C30 C31 C32 C33 C34 C35 C36
```

Latest run (both profiles): **32/32 rows passed, 0 failed, 78 313 individual
C-vs-Rust comparisons** (return values, `hashmap_t`/`tree_t`/`tree_node_t` state
and captured `stdout`/`stderr`).
