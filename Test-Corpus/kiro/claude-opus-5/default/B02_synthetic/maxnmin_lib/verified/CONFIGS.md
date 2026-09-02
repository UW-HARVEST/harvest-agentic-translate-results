# CONFIGS.md — configuration / valid-input surface table (Phase A, gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

There are no runtime option/mode/flag setters and no `#ifdef`s in this library.
The "configuration" of this API is entirely **mutable process-wide state plus
input shape**:

- **A1 — store population** (`node_count`): `0` (empty) · `1` · `6` (the shape
  `maxnmin` builds) · many (~50) · `99` · `100` (`MAX_NODES`, full).
- **A2 — tree topology**: leaf-only · single parent + children · 3-level tree
  (as in `maxnmin`) · forest (several roots) · duplicate `id`s (L65 returns the
  *first* match) · duplicate `parent_id`s · `parent_id` pointing at a
  non-existent node · `parent_id == -1` (root sentinel).
- **A3 — `active` flag** (only reachable by writing through the `Node*` that
  `find_node_by_id` hands back — this also validates the `#[repr(C)]` layout):
  all active · some children deactivated · root deactivated · all deactivated.
- **A4 — name shape** (`strncpy` with `n = MAX_NAME_LEN - 1 = 49`): empty ·
  1 byte · 48 · **49** (fills, no source NUL copied) · 50 · 200 (truncated) ·
  bytes ≥ 0x80 (signed `char` ⇒ negative in `process_string`) · byte `0x01`,
  `0x7F`.
- **A5 — `value` shape**: `0.0` · `-0.0` · small positive · negative ·
  fractional · `1e300` · subnormal · `±inf` · NaN.
- **A6 — `safe_double_to_int` input class**: in-range integral · in-range
  fractional positive (truncates toward 0) · in-range fractional negative
  (truncates toward 0) · exact `(double)INT_MAX` / `(double)INT_MIN` ·
  above/below range · `±inf` · NaN · `±0.0` · subnormal.
- **A7 — `maxnmin` parameter classes**: `param1 % 6` ∈ {0..5} (positive) and
  {0,-1..-5} (negative, ⇒ NULL node) · same for `param2` · `param3` ∈
  {0, -1 (÷0), positive, negative, `INT_MAX`, `INT_MIN`} · `param4 % 3` ∈
  {0,1,2} and {0,-1,-2} · `param4 == 0` (⇒ `× 0.0`).
- **A8 — call ordering / statefulness**: fresh store · after `add_node`s ·
  after a `maxnmin` (which resets `node_count = 0`) · `maxnmin` twice in a row ·
  `add_node` after `maxnmin` (appends at index 6).

## Entry points

All 7 exported symbols are driven directly through the `.so`, lowest level
first: `add_node` → `find_node_by_id` → `get_children_count` →
`calculate_subtree_sum` → `process_string` → `safe_double_to_int` → `maxnmin`
(the composed one-shot wrapper).

## Table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_5678_9ABC`, xorshift64\* PRNG) against **both** `.so`s, comparing
return values bit-for-bit (`f64::to_bits` for doubles) and, where a `Node*` is
returned, comparing null-ness plus every field of the pointed-to struct.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `safe_double_to_int` | A6: random in-range integral doubles | `cfg_01_sdti_in_range_integral` | [x] |
| 2  | `safe_double_to_int` | A6: random in-range fractional, positive — truncation toward zero | `cfg_02_sdti_fractional_positive` | [x] |
| 3  | `safe_double_to_int` | A6: random in-range fractional, negative — truncation toward zero | `cfg_03_sdti_fractional_negative` | [x] |
| 4  | `safe_double_to_int` | A6: random *arbitrary bit patterns* reinterpreted as `f64` (covers inf/NaN/subnormal/huge in one sweep) | `cfg_04_sdti_random_bit_patterns` | [x] |
| 5  | `safe_double_to_int` | A6: dense sweep of `nextafter` neighbourhoods around `(double)INT_MAX`, `(double)INT_MIN`, `0.0`, `±1.0` | `cfg_05_sdti_boundary_neighbourhoods` | [x] |
| 6  | `process_string` | A4: random ASCII strings, lengths 1..48 | `cfg_06_process_string_ascii` | [x] |
| 7  | `process_string` | A4: random bytes 0x01..0xFF (mixed sign), lengths 1..48 | `cfg_07_process_string_full_byte_range` | [x] |
| 8  | `process_string` | A4: length exactly 49 and exactly 50, all-0xFF and all-0x7F | `cfg_08_process_string_length_boundaries` | [x] |
| 9  | `process_string` | A4: long strings (up to 4 KiB) driving the accumulator far | `cfg_09_process_string_long` | [x] |
| 10 | `add_node` + `find_node_by_id` | A1=1, A4=empty name | `cfg_10_add_find_single_empty_name` | [x] |
| 11 | `add_node` + `find_node_by_id` | A1=1, A4 = 1/48/49/50/200-byte names, A5 = assorted values | `cfg_11_add_find_name_length_matrix` | [x] |
| 12 | `add_node` + `find_node_by_id` | A1=many, A2 = unique random ids incl. `INT_MIN`/`INT_MAX`/`0`/negatives; look up every id + random misses | `cfg_12_add_find_random_ids` | [x] |
| 13 | `add_node` + `find_node_by_id` | A2 = **duplicate ids** — first match must win; verified by mutating `value` through the returned pointer | `cfg_13_find_duplicate_ids_first_wins` | [x] |
| 14 | `add_node` + `find_node_by_id` | A1 = 99 then 100 (`MAX_NODES`) — returned index sequence and full-store lookups | `cfg_14_add_to_capacity` | [x] |
| 15 | `find_node_by_id` | A3: struct layout probe — read back `id`/`parent_id`/`name`/`value`/`active` through the returned pointer for random nodes | `cfg_15_node_struct_layout_readback` | [x] |
| 16 | `find_node_by_id` | A3: write `active = 0` through the returned pointer, then re-query | `cfg_16_deactivate_through_pointer` | [x] |
| 17 | `find_node_by_id` | A3: write `active` to a non-1 truthy value (e.g. `2`, `-1`, `INT_MIN`) — C tests `active` for truthiness, not `== 1` | `cfg_17_active_truthy_nonone` | [x] |
| 18 | `get_children_count` | A1=0 (empty) and A1=1 | `cfg_18_children_count_trivial` | [x] |
| 19 | `get_children_count` | A2 = one parent with 1..30 children; query the parent, a sibling, and random non-parents | `cfg_19_children_count_single_parent` | [x] |
| 20 | `get_children_count` | A2 = forest, random `parent_id`s from a small pool so counts collide; query every distinct `parent_id` | `cfg_20_children_count_forest` | [x] |
| 21 | `get_children_count` | A2 = every node shares one `parent_id` (count == `node_count`), at A1=100 | `cfg_21_children_count_all_same_parent` | [x] |
| 22 | `get_children_count` | A3 = mixed active/inactive children (random mask applied through pointers) | `cfg_22_children_count_mixed_active` | [x] |
| 23 | `get_children_count` | A2: `parent_id == -1` (root sentinel), `0`, `INT_MIN`, `INT_MAX` | `cfg_23_children_count_sentinel_parents` | [x] |
| 24 | `calculate_subtree_sum` | A2 = single leaf, A5 = random finite values | `cfg_24_subtree_sum_leaf` | [x] |
| 25 | `calculate_subtree_sum` | A2 = 2-level (parent + N children), A5 random — **summation order matters** for FP | `cfg_25_subtree_sum_two_level` | [x] |
| 26 | `calculate_subtree_sum` | A2 = 3-level tree identical in shape to `maxnmin`'s, A5 random | `cfg_26_subtree_sum_three_level` | [x] |
| 27 | `calculate_subtree_sum` | A2 = random deep chain (depth up to 40) — recursion depth | `cfg_27_subtree_sum_deep_chain` | [x] |
| 28 | `calculate_subtree_sum` | A2 = random forest, sum queried at every node id | `cfg_28_subtree_sum_random_forest` | [x] |
| 29 | `calculate_subtree_sum` | A2 = **duplicate ids** with children — recursion re-enters via `id`, so a duplicated id is visited more than once; must match exactly | `cfg_29_subtree_sum_duplicate_ids` | [x] |
| 30 | `calculate_subtree_sum` | A5 = values chosen to expose non-associative FP addition (`1e16`, `1.0`, `-1e16`, …) | `cfg_30_subtree_sum_fp_association` | [x] |
| 31 | `calculate_subtree_sum` | A3 = random active mask over a 3-level tree | `cfg_31_subtree_sum_active_mask` | [x] |
| 32 | `maxnmin` | A7: full cross-product of `param1 % 6` × `param2 % 6` × `param4 % 3` for positive params, `param3` = 1 | `cfg_32_maxnmin_residue_cross_product` | [x] |
| 33 | `maxnmin` | A7: same cross-product with negative `param1`/`param2`/`param4` (C truncating `%`) | `cfg_33_maxnmin_negative_residues` | [x] |
| 34 | `maxnmin` | A7: `param3` ∈ {`INT_MIN`, -2, -1, 0, 1, 2, `INT_MAX`} × representative `param1/2/4` | `cfg_34_maxnmin_param3_classes` | [x] |
| 35 | `maxnmin` | A7: `param4` ∈ {`INT_MIN`, -1, 0, 1, `INT_MAX`} × representative others | `cfg_35_maxnmin_param4_classes` | [x] |
| 36 | `maxnmin` | A7: 20 000 fully random `(i32, i32, i32, i32)` quadruples | `cfg_36_maxnmin_random_quadruples` | [x] |
| 37 | `maxnmin` | A7: quadruples drawn from a boundary pool (`0, ±1, ±2, ±3, ±5, ±6, ±7, INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, ±0x7FFF…`) — full 4-way cross-product | `cfg_37_maxnmin_boundary_pool_cross_product` | [x] |
| 38 | `maxnmin` | A7: params that make the final `(p1+p2)/(p3+1)*p4` term land exactly on / past the int range | `cfg_38_maxnmin_final_term_saturation` | [x] |
| 39 | all 7 | A8: interleaved random *call sequence* (random op, random args) driving both `.so`s in lock-step, comparing after every step — catches state divergence the per-function tests cannot | `cfg_39_random_interleaved_call_sequence` | [x] |
| 40 | `add_node` → `maxnmin` | A8: populate the store, then call `maxnmin` (which resets `node_count`), then observe the store via `find_node_by_id`/`get_children_count` | `cfg_40_state_add_then_maxnmin` | [x] |
| 41 | `maxnmin` → `add_node` | A8: `maxnmin` first, then `add_node` — must land at index 6 and be visible to the other functions | `cfg_41_state_maxnmin_then_add` | [x] |
| 42 | `maxnmin` ×N | A8: `maxnmin` called repeatedly with the same args — idempotent because it resets state | `cfg_42_maxnmin_idempotent_repeat` | [x] |
| 43 | `add_node` past capacity → `maxnmin` | A8: fill to 100, get `-1`s, then `maxnmin` recovers the store to 6 nodes; further `add_node`s succeed | `cfg_43_state_recover_after_capacity` | [x] |
| 44 | `find_node_by_id` + `process_string` | A8: feed the `name` field of a live `Node*` straight into `process_string` (the exact composition `maxnmin` performs), for random names incl. high-bit bytes | `cfg_44_compose_find_then_process_string` | [x] |
| 45 | `add_node` + `calculate_subtree_sum` | A5 × A5: **full cross-product** of the non-finite pool (±qNaN/±sNaN with 4 distinct payloads, ±inf, ±0.0, subnormals, `MAX`/`MIN`) as (root value, child value) — the two-operand `addsd` case | `cfg_45_subtree_sum_nan_propagation_exhaustive_pairs` | [x] |
| 46 | `add_node` + `calculate_subtree_sum` | A5 × A2: multi-child and 3-level trees drawn from the non-finite pool — the accumulator is fed repeatedly, so the per-step operand order decides which NaN survives | `cfg_46_subtree_sum_nan_propagation_multi_child` | [x] |
| 47 | `add_node` + `calculate_subtree_sum` + `safe_double_to_int` | A5 × A2 × A3: random acyclic forests of non-finite values with a random `active` mask, sums piped into `safe_double_to_int` | `cfg_47_subtree_sum_nan_propagation_deep_and_masked` | [x] |

## Axis note: floating-point operand order

Rows 45–47 exist because C's `sum += calculate_subtree_sum(...)` lowers to
`addsd %xmm1,%xmm0` with the **child's** value in the destination register.
x86 `addsd` returns the destination operand (quieted) when it is a NaN, so the
child's NaN sign and payload win over the accumulator's — the opposite of what
`sum += child` produces in Rust. This is the one divergence Phase B found; see
the `addsd` helper in `src/lib.rs`.

The other FP sites are not order-sensitive: `maxnmin`'s division is
non-commutative by definition, and its two multiplications never see a NaN in
their second operand (`param3` / `param4` are converted `int`s, and
`second_node->value` is always one of the six hard-coded constants because
`maxnmin` rebuilds the store on entry).
