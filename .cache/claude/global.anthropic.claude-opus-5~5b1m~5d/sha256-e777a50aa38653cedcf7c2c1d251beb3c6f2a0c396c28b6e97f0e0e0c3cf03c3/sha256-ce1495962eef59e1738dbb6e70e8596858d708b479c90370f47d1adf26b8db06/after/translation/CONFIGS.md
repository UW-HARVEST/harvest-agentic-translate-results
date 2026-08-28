# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from
`c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

This library has **no compile-time options** (`#ifdef` count: 0 — only the two
object-like macros `MAX_NODES 100` and `MAX_NAME_LEN 50`) and **no runtime
option/flag/mode arguments**. Its "configuration" is therefore

1. **Which entry point** is called. All 7 exported functions are public API;
   `maxnmin` (the only one in `include/lib.h`) is the one-shot convenience
   wrapper, the other 6 are the low-level entry points and are exercised
   directly, not only through `maxnmin`:
   `add_node`, `find_node_by_id`, `get_children_count`,
   `calculate_subtree_sum`, `process_string`, `safe_double_to_int`.
2. **The mutable static state** the functions branch on — `node_count`
   (`0`, `1`, `few`, `99`, `100` = `MAX_NODES`) and the contents of
   `node_storage` (`id`, `parent_id`, `active`, `value`, `name`). The state is
   reachable/observable three ways: `add_node` appends, `maxnmin` *resets*
   `node_count` to 0 and re-seeds 6 nodes, and `find_node_by_id` hands the
   caller a **mutable `Node *`** into the static array, so a real consumer can
   flip `active`, rewrite `id`/`parent_id`/`value`/`name` in place.
3. **Tree shape** encoded by `parent_id`, which drives the linear scans
   (`get_children_count`) and the recursion (`calculate_subtree_sum`):
   no children / flat fan-out / deep chain / forest / forward references /
   duplicate ids / dangling parents.
4. **Input shapes** the code special-cases:
   * `name`: length `0`, `1`, `< 49`, `49`, `50`, `> 50` (the `strncpy`
     truncation boundary), byte values incl. `0x80..0xFF` (signed `char`).
   * `value` / `d`: `±0.0`, in-range fractional (truncation direction),
     the exact `(double)INT_MAX` / `(double)INT_MIN` boundaries, out-of-range,
     `±inf`, NaN, subnormal.
   * `str` for `process_string`: empty, short, long, high-bit bytes, early NUL.
   * `int` params: `0`, `±1`, all residues mod 6 and mod 3 (both signs — C `%`
     truncates toward zero), `INT_MIN`, `INT_MAX`.

## Rows (cross-product, pruned to combinations the C distinguishes)

Every row is driven by **many randomized inputs with a fixed seed**
(`tests/common/mod.rs`: `Rng::new(<row seed>)`), both libraries loaded from
**private copies** of the `.so`s so each row starts from pristine static state.
`[x]` = passing byte-for-byte (ints compared exactly, doubles compared by
`to_bits()`, `name[50]` compared byte-for-byte, `Node *` compared as
null/non-null **and** as pointer deltas → storage index).

Test file: `translation/tests/valid_paths.rs` (row `Cn` -> test `cn_...`).

Note on the randomized tree shapes: the C `calculate_subtree_sum` has no visited
set, so a `parent_id` cycle (including a node whose `id == parent_id`) recurses
until the stack dies (`ERRORS.md` E39). The generators therefore only ever point
a node's `parent_id` at `-1`, at an *already inserted* node, or at a value that
is permanently excluded from being an id — which keeps the child relation a
forest while still covering dangling parents, duplicates and forward references.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `safe_double_to_int` | in-range **positive** fractional doubles, random in `[0, 2^31)` — truncation toward zero | [x] |
| C2 | `safe_double_to_int` | in-range **negative** fractional doubles, random in `(-2^31, 0]` — truncation toward zero | [x] |
| C3 | `safe_double_to_int` | exact integral values: `±0.0`, `±1`, `±2^k` for k≤31, `2147483647`, `-2147483648` | [x] |
| C4 | `safe_double_to_int` | boundary neighbourhood: `nextafter` steps around `(double)INT_MAX` and `(double)INT_MIN`, `±(2^31±0.5)` | [x] |
| C5 | `safe_double_to_int` | out-of-range magnitudes up to `1e308` and `±inf` (both clamp branches) | [x] |
| C6 | `safe_double_to_int` | NaN family: quiet/signalling, both signs, random mantissa payloads | [x] |
| C7 | `safe_double_to_int` | subnormals / tiny (`±5e-324`, `±1e-300`) and `-0.0` | [x] |
| C8 | `safe_double_to_int` | **random raw 64-bit patterns** reinterpreted as `double` (hits every class incl. weird NaNs) | [x] |
| C9 | `process_string` | zero-length string (the `if (*str)` guard) | [x] |
| C10 | `process_string` | random printable ASCII, lengths 1..64 | [x] |
| C11 | `process_string` | random non-zero bytes over the full `0x01..0xFF` range (signed-`char` sign extension), lengths 1..256 | [x] |
| C12 | `process_string` | homogeneous buffers `0x7F` / `0x80` / `0xFF` / `0x01` at lengths 1, 2, 49, 50, 51, 1000 | [x] |
| C13 | `process_string` | long buffers, 4 KiB..64 KiB of random non-zero bytes | [x] |
| C14 | `process_string` | NUL embedded at a random interior offset (scan stops early) | [x] |
| C15 | `find_node_by_id`, `get_children_count`, `calculate_subtree_sum` | **pristine store** (`node_count == 0`), random ids/parent_ids incl. `0`, `±1`, `INT_MIN`, `INT_MAX` | [x] |
| C16 | `add_node` + all 3 queries | **single node**, `parent_id = -1` (root only); query its own id, its parent id, and absent ids | [x] |
| C17 | `add_node` | sequential inserts `n = 1..20`, random `id`/`parent_id`/`name`/`value`; assert every return value **and** every stored field via `find_node_by_id` | [x] |
| C18 | `add_node` + `find_node_by_id` | `name` shapes: lengths 0,1,2,10,48,**49**,**50**,51,120 incl. random high-bit bytes → verify all 50 stored bytes (truncation + zero padding + forced `name[49]=0`) | [x] |
| C19 | `add_node` + `calculate_subtree_sum` | `value` shapes: `0.0`, `-0.0`, `±inf`, NaN, `±1e308` (accumulator overflow), subnormal, random bit patterns | [x] |
| C20 | `add_node` + `find_node_by_id` | `id` shapes: `0`, negatives, `INT_MIN`, `INT_MAX`, **duplicate ids** (first-active-match wins; compared via pointer delta) | [x] |
| C21 | `add_node` + `get_children_count` + `calculate_subtree_sum` | **flat tree**: 1 root + k children, k ∈ {0,1,2,7,50}; query every parent_id present *and* absent | [x] |
| C22 | `add_node` + `calculate_subtree_sum` | **deep chain**, depth 1..99 (`parent_id` = previous id) — recursion depth; sum queried from every level | [x] |
| C23 | `add_node` + `get_children_count` | **forest**: m roots sharing `parent_id = -1`, m ∈ {2,5,30}; plus several disjoint subtrees | [x] |
| C24 | all 4 state entry points | **random forest**, n ≤ 100 nodes, parents drawn from {existing ids} ∪ {absent ids} ∪ {-1}; then `get_children_count` + `calculate_subtree_sum` for every id seen and a batch of random ids | [x] |
| C25 | `add_node` | **near-full**: fill to 99, then the 100th (last legal slot) → index 99 | [x] |
| C26 | all 4 state entry points | **exactly full** store (`node_count == MAX_NODES == 100`): queries over all 100 entries | [x] |
| C27 | `find_node_by_id` → write `active = 0` → re-query | deactivation through the returned `Node *`, for 1, several and *all* nodes | [x] |
| C28 | `find_node_by_id` → rewrite `id` / `parent_id` / `value` / `name` in place → re-query | in-place mutation through the FFI-returned pointer, then re-scan with all 3 query functions | [x] |
| C29 | `find_node_by_id` → write `active` = `2` / `-1` / `INT_MIN` / `0x7fffffff` | non-`0/1` truthiness of `active` across all three scans | [x] |
| C30 | `add_node` + queries | **forward references**: children inserted before their parents (linear-scan order dependence) | [x] |
| C31 | `maxnmin` | all 6 residues of `param1` × all 6 residues of `param2` (non-negative), `param3 = 1`, `param4 = 0` | [x] |
| C32 | `maxnmin` | negative `param1`/`param2` ∈ `-1..-13` (node lookup misses, both blocks skipped) | [x] |
| C33 | `maxnmin` | `param3` ∈ {`0`,`1`,`-1`,`2`,`-2`,`5`,`-7`,`INT_MAX`,`INT_MIN`,`±10^9`} — divisor `param3+1` incl. `0` and overflow | [x] |
| C34 | `maxnmin` | `param4` ∈ all 3 residues both signs (`parent_id` ∈ {-1,0,1,2,3}) × a few `param1`/`param2` | [x] |
| C35 | `maxnmin` | full cross-product of `{INT_MIN, INT_MIN+1, -7, -1, 0, 1, 7, INT_MAX-1, INT_MAX}` over all 4 params (9⁴ = 6561 cases) | [x] |
| C36 | `maxnmin` | fully random 32-bit params, large randomized sweep (≥ 20 000 cases) | [x] |
| C37 | `maxnmin` + `add_node` + queries | `maxnmin` called **repeatedly** and interleaved with `add_node` — the `node_count = 0` reset and storage reuse | [x] |
| C38 | `maxnmin` after mutation | store previously filled to 100 and/or nodes deactivated/mutated, then `maxnmin` → reset semantics, then queries | [x] |
| C39 | **all 7 entry points** | long randomized **operation sequences** (state-machine fuzz, 200 ops × 200 runs, pristine library pair per run) mixing inserts, pointer mutations, queries, `maxnmin` resets | [x] |
| C40 | `add_node` / `find_node_by_id` | struct-layout probe: the **whole raw 80-byte image** of every stored `Node` compared C↔Rust — each field at its C offset (0,4,8,64,72) *and* the 6 bytes of inter-field padding at 58..64 plus the 4 tail bytes at 76..80 | [x] |
| C41 | `add_node` + `calculate_subtree_sum` | NaN **payload/sign** propagation through nested accumulation: every ordered pair of 8 distinct NaN bit patterns (quiet/signalling, both signs, non-zero payloads) at chain depth 3, plus randomized fan-out with grandchildren. Regression row for the one real divergence found (see `VERIFICATION.md`) | [x] |

## Results

All 41 rows pass. `tests/valid_paths.rs` = 41 tests, ~1.9 s, seeds fixed
(`Rng::new(0xC0nn)` per row), each row driven through **both** `.so`s only via
their exported C symbols.

Completeness evidence (`./check_coverage.sh`, gcov on an instrumented copy of
`c_src/src/lib.c` driven by this suite):

```
Lines executed:100.00% of 75
Branches executed:100.00% of 38
Taken at least once:97.37% of 38     <- 37/38 directions; the one gap is
Calls executed:100.00% of 16            lib.c:145 `if (*name_ptr)` FALSE,
                                        which is unreachable (ERRORS.md E29)
```

## Feature combinations

No `[features]` in `Cargo.toml` → the only combinations are the empty default,
`--no-default-features` and `--all-features`; all three are run by
`translation/check_features.sh` (see `SYMBOLS.md`).
