# CONFIGS.md — Configuration / valid-input surface table (Phase B gate)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on.

## Axis inventory (derived from the source, not guessed)

**Runtime options / modes / flags.** The library exposes **no** option, mode or
flag setter — no `enum`, no bitmask, no `set_*` function, no environment lookup.
`grep -n '#if\|#ifdef\|switch' c_src/src/lib.c` → **no matches**: there is no
conditional compilation and no `switch`. So the configuration surface is not
"options" but **library state × input shape**, which is enumerated below.

**Hidden state (the real "mode" of this library).** Both `.so`s keep
`static Node node_storage[100]` + `static int node_count`, so *every* entry
point's result depends on accumulated state. State axes:

* `S0` pristine: `node_count == 0`
* `S1` a handful of nodes added by the caller
* `S2` state as left by `maxnmin`: `node_count == 6`, the 6 seeded nodes
* `S3` full: `node_count == 100`
* `S4` state mutated in place through the `Node*` returned by
  `find_node_by_id` (the only public write path into stored nodes — it hands
  out a **non-const** pointer): flipping `active`, rewriting `parent_id` to
  reshape the tree, rewriting `value`

**Input shapes the code special-cases.**

* `name` length: 0 / 1 / mid / 49 (`MAX_NAME_LEN-1`) / 50 / >50 → truncation
* `name` bytes: ASCII / high-bit `0x80..0xFF` (signed `char` → negative
  contributions in `process_string`) / interior bytes after a NUL
* `value`: 0.0 / -0.0 / small / negative / huge (>`INT_MAX`) / NaN / ±inf /
  values chosen so summation order is observable in the last mantissa bit
* `id`, `parent_id`: unique / duplicated / self-referencing / `0` / negative /
  `-1` (the seeded root's parent) / `INT_MIN` / `INT_MAX`
* node count: empty (0) / one / many / at the 100 boundary
* tree shape: single leaf / flat fan-out / deep chain / forest / node whose
  parent is absent / child stored *before* its parent (index order ≠ tree order)
* `maxnmin` params: all 6 residues of `param1 % 6` and `param2 % 6`
  (positive **and** negative), all 3 residues of `param4 % 3`, `param3` ∈
  {`-1` (÷0), `0`, positive, negative, `INT_MAX`, `INT_MIN`}

**Full set of public entry points** (all 7 from `SYMBOLS.md`), tested
**directly**, lowest level first — not only through the `maxnmin` one-shot
wrapper: `safe_double_to_int` → `process_string` → `add_node` →
`find_node_by_id` → `get_children_count` → `calculate_subtree_sum` → `maxnmin`.

## Configuration rows

Every row is exercised with **many randomized inputs** (fixed seed, own
xorshift PRNG — no external dep) against both `.so`s, byte-for-byte.
`[x]` = passes.

| # | entry point(s) | configuration (state + input shape) | [x] |
|---|----------------|--------------------------------------|-----|
| C01 | `safe_double_to_int` | stateless; randomized `f64` bit-patterns drawn from the whole space (incl. subnormals, huge, negative) | [x] |
| C02 | `safe_double_to_int` | randomized values tightly clustered around the `±INT_MAX`/`INT_MIN` boundaries (`±2^31 ± ε`) | [x] |
| C03 | `safe_double_to_int` | randomized small magnitudes with fractions, both signs (truncate-toward-zero) | [x] |
| C04 | `process_string` | `S0`; randomized ASCII strings, lengths 0..64 | [x] |
| C05 | `process_string` | `S0`; randomized **full-byte** strings (`0x01..0xFF`, high bit set) — signed-`char` sign extension | [x] |
| C06 | `process_string` | `S0`; long strings (1..4096 bytes) chosen so the `int` accumulator wraps | [x] |
| C07 | `process_string` | applied to a `name` **in situ** via the `Node*` from `find_node_by_id` (real consumer path, as `maxnmin` does) | [x] |
| C08 | `add_node` | `S0` → one node; randomized `id`/`parent_id`/`value`/`name`; check return index | [x] |
| C09 | `add_node` | `S0` → many nodes (1..99); randomized; check every return value in sequence | [x] |
| C10 | `add_node` | `S0` → exactly 100 nodes: the last legal add returns 99 (upper boundary, still valid) | [x] |
| C11 | `add_node` | randomized `name` lengths spanning 0,1,48,**49**,50,51,120 — truncation boundary | [x] |
| C12 | `add_node` | randomized `name` containing high-bit bytes and an embedded interior NUL | [x] |
| C13 | `add_node` | randomized `value` incl. `0.0`, `-0.0`, NaN, `±inf`, `1e300`, denormals — stored bits compared | [x] |
| C14 | `find_node_by_id` | `S1` random storage; query every stored id **and** ids known absent; compare NULL-ness and the pointer's node **index** (via pointer arithmetic, since absolute addresses differ per `.so`) | [x] |
| C15 | `find_node_by_id` | `S1` with **duplicate** ids → first-match-wins ordering | [x] |
| C16 | `find_node_by_id` | `S4`: some nodes deactivated through a previously returned `Node*`, then re-queried | [x] |
| C17 | `find_node_by_id` | `S2` (post-`maxnmin`), ids 1..6 plus 0 and 7 | [x] |
| C18 | `get_children_count` | `S1` random flat storage; query every `parent_id` present, plus absent ones, plus `-1`, `0`, `INT_MIN`, `INT_MAX` | [x] |
| C19 | `get_children_count` | `S1` fan-out shapes: 0, 1, and many children under one parent | [x] |
| C20 | `get_children_count` | `S4`: children deactivated in place, then recounted | [x] |
| C21 | `get_children_count` | `S3` (100 nodes) with parents assigned randomly — counts near the storage limit | [x] |
| C22 | `calculate_subtree_sum` | `S1` single leaf (no children) → just its own `value` | [x] |
| C23 | `calculate_subtree_sum` | `S1` flat fan-out; randomized `value`s picked so the **summation order** is observable in the low mantissa bits (bit-exact `f64` compare) | [x] |
| C24 | `calculate_subtree_sum` | `S1` deep chain (depth 1..40) — recursion depth | [x] |
| C25 | `calculate_subtree_sum` | `S1` randomized **forest**: children stored at indices *before* their parent, multiple roots, orphans whose parent id is absent | [x] |
| C26 | `calculate_subtree_sum` | `S1` with duplicate ids in the tree — C recurses on `node_storage[i].id`, so a duplicated id can be summed more than once | [x] |
| C27 | `calculate_subtree_sum` | `S1` with NaN / `±inf` / huge `value`s mixed in (bit-exact, incl. NaN payload) | [x] |
| C28 | `calculate_subtree_sum` | `S4`: subtree with interior nodes deactivated (prunes whole branches) | [x] |
| C29 | `calculate_subtree_sum` | `S2`, all of ids 1..6 — the exact shape `maxnmin` builds | [x] |
| C30 | `maxnmin` | all 6×6 combinations of `param1 % 6` × `param2 % 6` with **non-negative** params, randomized `param3`/`param4` | [x] |
| C31 | `maxnmin` | `param1`/`param2` **negative** across all residues (node_id ≤ 0 → NULL path) | [x] |
| C32 | `maxnmin` | all 3 residues of `param4 % 3`, positive and negative, incl. `parent_id == -1` hitting the root | [x] |
| C33 | `maxnmin` | `param3` ∈ {`-1`, `0`, `1`, small ±, `INT_MAX`, `INT_MIN`} × randomized others | [x] |
| C34 | `maxnmin` | fully randomized `(param1,param2,param3,param4)` over the whole `i32` range, 20 000 tuples | [x] |
| C35 | `maxnmin` | randomized tuples restricted to `|param| <= 12` (dense small-value sweep incl. every sign combination) | [x] |
| C36 | `maxnmin` | **composed pipeline / state interaction**: caller adds its own nodes, calls `maxnmin`, inspects state with the low-level entry points, calls `maxnmin` again — the whole sequence compared step by step | [x] |
| C37 | all 7 | **randomized operation-sequence fuzz**: a random program of 300 calls mixing every entry point (incl. in-place `Node` mutation), replayed identically against both `.so`s, comparing every single return value | [x] |
| C38 | `Node` ABI | struct layout parity: field offsets/size observed through the `Node*` (`id`@0, `parent_id`@4, `name`@8, `value`@64, `active`@72, stride 80) | [x] |
| C39 | `Node` ABI | the FULL 80-byte stored image (padding holes 58..64 and 76..80 included) for randomized nodes — anything a caller can read back through the returned `Node*` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the feature
power-set is exactly one element: the default build. There is likewise no
`[[bin]]` target and `c_src/CMakeLists.txt` has no `add_executable`, so there is
**no driver binary** whose stdout could be compared. `scripts/verify_all.sh`
enumerates the feature combinations from `Cargo.toml` programmatically and runs
the full suite for each, so the loop is automated and stays correct if features
are ever added.

## Row → test mapping

| rows | test file |
|---|---|
| C01..C29, C38, C39 | `tests/lowlevel_diff.rs` (31 tests; C39 is `c38b_node_full_byte_image_randomized`) |
| C27-deepened (exhaustive) | `tests/nan_propagation.rs` (4 tests) |
| C30..C37 | `tests/maxnmin_diff.rs` (8 tests) |

## Divergence found and fixed in Phase B

**`calculate_subtree_sum` returned the wrong NaN when two addends were both NaN.**

Found by row C27 (`wild_values` forests seeded with NaN/inf), then minimised by a
brute-force sweep over NaN/inf bit patterns.

* The C body is `sum += calculate_subtree_sum(...)`, which gcc compiles to
  `addsd %xmm1,%xmm0` where `%xmm0` holds the **recursive result** and `%xmm1`
  the accumulator — i.e. the child's value is `ADDSD`'s *destination* operand.
* Per Intel SDM Table 4-7, when both operands are NaN `ADDSD` returns the
  destination operand (quieted). So the C propagates the **child's** NaN sign and
  payload.
* Rust's `sum += child` puts the accumulator in the destination position, and
  because `fadd` is commutative LLVM reorders operands freely — so the Rust
  returned the **accumulator's** NaN instead. 282 of 1110 minimal combinations
  disagreed; the sign bit and the payload both differed.
* Fix: an `addsd(dst, src)` helper in `src/lib.rs` that reproduces the hardware
  rule explicitly (destination NaN wins, quieted; otherwise a plain add). Simply
  writing `child + sum` was **not** enough — LLVM commuted it straight back.

This is only observable through the exported `calculate_subtree_sum`, since
`maxnmin` funnels every NaN through `safe_double_to_int`, which maps all NaNs to
`0`. A `maxnmin`-only test suite would never have found it.

## Harness bug worth recording

An earlier version of the harness shared one `dlopen` handle across test
functions. `dlopen` de-duplicates by inode, so all tests in a binary got the
**same** library instance — and therefore the same `node_count` / `node_storage`
globals. Because cargo runs tests in parallel threads, concurrent `maxnmin` calls
(which reset `node_count` to 0 mid-flight) corrupted each other and produced four
phantom "divergences" in `maxnmin`. Every test now takes a `fresh_pair()`, which
`dlopen`s a private per-test copy of each `.so` and thus gets pristine statics.
The lesson is recorded in `tests/common/mod.rs`; there is deliberately no shared
loader.

## Result

**All 39 rows pass** across the randomized inputs (fixed seeds, reproducible).
`tests/lowlevel_diff.rs` + `tests/nan_propagation.rs` + `tests/maxnmin_diff.rs`
= 43 test functions, 0 failures.

## Independent cross-check (not using the Rust harness)

`scripts/xcheck.c` is a standalone C driver that `dlopen`s *one* library and
prints a transcript: digests over a 390 625-case exhaustive `maxnmin` sweep,
200 000 random `maxnmin` calls, 200 000 `safe_double_to_int` bit patterns, then a
60-node forest driven through `add_node` / `find_node_by_id` /
`get_children_count` / `calculate_subtree_sum` / `process_string`. Run against
each `.so`, its stdout is **byte-identical** (1943 bytes, 170 lines), which rules
out the possibility that the Rust harness passes vacuously (e.g. by accidentally
loading the C library twice).

## Negative control

`scripts/mutation_check.sh` injects 19 known bugs into `src/lib.rs` one at a time
and confirms the suite fails for each. **19/19 detected.** Two further mutants
were investigated and shown to be *behaviourally equivalent* (unobservable
through the public API), so their survival is correct, not a gap:

* `while i < MAX_NAME_LEN` instead of `MAX_NAME_LEN - 1` in the `strncpy` loop —
  byte 49 is unconditionally overwritten with `'\0'` on the next line either way.
* `d >= (double)INT_MAX` instead of `d >` in `safe_double_to_int` — at exactly
  `2147483647.0` the clamp returns `INT_MAX` and the fall-through `(int)d` cast
  also returns `2147483647`.
