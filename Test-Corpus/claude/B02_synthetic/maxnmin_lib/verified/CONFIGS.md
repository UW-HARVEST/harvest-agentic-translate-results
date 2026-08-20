# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

The mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on.

## Full set of public entry points (from `nm -D` on the C `.so`)

Ordered lowest-level → highest-level (this is also the call hierarchy and the order
the tests attack them in):

| level | entry point | reads global state? | writes global state? |
|---|---|---|---|
| L0 | `safe_double_to_int(double)` | no | no |
| L0 | `process_string(char*)` | no | no |
| L1 | `add_node(int,int,const char*,double)` | `node_count` | `node_storage`, `node_count` |
| L1 | `find_node_by_id(int)` → `Node*` | both | no (but hands out a **mutable** pointer) |
| L1 | `get_children_count(int)` | both | no |
| L2 | `calculate_subtree_sum(int)` (recursive, calls `find_node_by_id`) | both | no |
| L3 | `maxnmin(int,int,int,int)` (the convenience one-shot; **resets** `node_count = 0` and seeds 6 nodes) | both | both |

There are **no runtime option/mode/flag setters** in this API. The "configuration"
of the library is therefore (a) the *global node-graph state* built up by
`add_node` + writes through the `Node*` handed back by `find_node_by_id`, and
(b) the *shape/class of the scalar arguments*. Those are the axes below.

## Axes the C code branches on

* **A — `node_count`**: `0` (empty) · `1` · `6` (what `maxnmin` seeds) · `99` ·
  `100` (`MAX_NODES`, full). Branch: `node_count >= MAX_NODES` (l.45) and every
  `i < node_count` loop bound.
* **B — graph shape** (drives `find_node_by_id` l.63-70, `get_children_count`
  l.72-80, the recursion at l.90-94): no edges (all orphans) · single root ·
  flat fan-out · deep chain (recursion depth = `node_count`) · balanced tree ·
  forest / multiple roots · duplicate `id`s (first match wins) · duplicate
  `parent_id`s · `parent_id == -1` sentinel · `id == 0` · negative ids ·
  `INT_MIN`/`INT_MAX` ids · self-parented leaf that is *never* summed.
* **C — `name` shape** (drives `strncpy` l.56 + `process_string` l.99-110):
  empty · 1 byte · 48 · 49 (exact boundary) · 50 · 200 (truncated) ·
  bytes `0x01`–`0x7F` · bytes `0x80`–`0xFF` (signed-`char` sign extension) ·
  embedded NUL (copy stops early).
* **D — `value` shape** (drives the FP accumulation at l.88-96 and the multiply at
  l.159): `0.0` · `-0.0` · small · negative · magnitudes chosen so the *order* of
  `+=` is observable (`1e16 + 1.0 + 1.0`) · `1e308` (sum overflows to `+inf`) ·
  `±INFINITY` (`inf + -inf` → NaN) · NaN · subnormal (`5e-324`).
* **E — `active`** (only reachable by writing through the returned `Node*`):
  `1` · `0` · `2` · `-1` · `INT_MIN` · `0x100`.
* **F — `safe_double_to_int` input class**: `< INT_MIN` · `== INT_MIN` · negative
  fractional · `-0.0` · `+0.0` · subnormal · positive fractional · `== INT_MAX` ·
  `> INT_MAX` · `±inf` · NaN.
* **G — `maxnmin` argument classes**: each of `param1..param4` ∈ {`INT_MIN`, `-2`,
  `-1`, `0`, `1`, `2`, `5`, `6`, `INT_MAX`, random} — the code branches on
  `param1 % 6`, `param2 % 6`, `param4 % 3` (each can be ≤ 0 → NULL branch),
  `param3 + 1 == 0` (division by zero), `param3 + 1` overflow, `param1 + param2`
  overflow.
* **H — call ordering / state carry-over**: fresh library (`node_count == 0`) ·
  after `add_node`s · after `maxnmin` (which clobbers to exactly 6 nodes) ·
  `maxnmin` → low-level calls · low-level calls → `maxnmin` → low-level calls.

Every test gets a **freshly `dlopen`ed copy** of each `.so` (copied to a unique
path so the loader allocates fresh, zeroed `.bss`), which is the only way to
observe `node_count == 0`; the C library exports no reset entry point.

## CONFIGURATION-SURFACE TABLE

Randomized rows use a fixed seed (SplitMix64, seed noted per test) and ≥ 256
iterations unless stated otherwise.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `safe_double_to_int` | axis F: all 11 hand-picked classes (`<INT_MIN`, `==INT_MIN`, `-frac`, `-0.0`, `+0.0`, subnormal, `+frac`, `==INT_MAX`, `>INT_MAX`, `±inf`, NaN) | [x] |
| C2 | `safe_double_to_int` | randomized: uniform **raw 64-bit patterns** reinterpreted as `f64` (covers every exponent incl. NaN/inf/subnormal), 20 000 iters | [x] |
| C3 | `safe_double_to_int` | randomized: doubles concentrated on the `INT_MIN`/`INT_MAX` boundaries (`±2147483647 ± ulp`, `±2^31 ± frac`), 4 096 iters | [x] |
| C4 | `process_string` | axis C: empty · 1 byte · 48 · 49 · 50 · 200 bytes, all-ASCII | [x] |
| C5 | `process_string` | axis C: bytes `0x80`–`0xFF` only (negative `char`), and mixed sign, random lengths 0…512, 2 000 iters | [x] |
| C6 | `process_string` | randomized full byte range `0x01`–`0xFF`, random length 0…4096, 2 000 iters | [x] |
| C7 | `process_string` | oversized: 17 000 000 bytes of `0x7F` (sum wraps past `INT_MAX`) — 1 iter | [x] |
| C8 | `add_node` + `find_node_by_id` | axis A=1, axis B single orphan; verify returned index, then read back **every field at its C offset** through the `Node*` (struct-layout check) | [x] |
| C9 | `add_node` + `find_node_by_id` | axis C × A: name lengths {0,1,48,49,50,200} incl. high-bit bytes and an embedded NUL; verify the 50-byte stored `name` buffer byte-for-byte | [x] |
| C10 | `add_node` | axis A: fill to 99, then 100 (`MAX_NODES`) — verify every return index `0..=99` | [x] |
| C11 | `add_node` + `find_node_by_id` | axis B duplicate `id`s (three nodes, same id) — first match wins | [x] |
| C12 | `add_node` + `find_node_by_id` | axis B extreme ids: `0`, `-1`, `INT_MIN`, `INT_MAX`; look each up plus near misses | [x] |
| C13 | `find_node_by_id` | axis A=0 (fresh lib, empty storage), 64 random ids → all `NULL` | [x] |
| C14 | `get_children_count` | axis A=0 (empty), random `parent_id`s | [x] |
| C15 | `get_children_count` | axis B flat fan-out: 1 parent with 0/1/2/50 children; query the parent, a non-parent, `-1`, `0`, `INT_MIN`, `INT_MAX` | [x] |
| C16 | `get_children_count` | axis B random forest (random ids/parents drawn from a small pool so collisions happen), 400 iters × all distinct `parent_id`s queried | [x] |
| C17 | `calculate_subtree_sum` | axis B single leaf (no children), axis D value classes {0.0, -0.0, small, neg, 1e308, inf, -inf, NaN, subnormal} — compare **raw bits** | [x] |
| C18 | `calculate_subtree_sum` | axis B flat fan-out, axis D order-sensitive values (`1e16, 1.0, 1.0, …`) — verifies the `+=` **iteration order** matches | [x] |
| C19 | `calculate_subtree_sum` | axis B deep chain of 99 nodes (recursion depth 99), random values — bit-exact | [x] |
| C20 | `calculate_subtree_sum` | axis B balanced tree (depth 3, fan-out 3), random values — bit-exact | [x] |
| C21 | `calculate_subtree_sum` | axis B **DAG / duplicate parent_id** so a subtree is counted more than once, plus `parent_id == id` on a node that is queried *from its parent* (bounded double-count) | [x] |
| C22 | `calculate_subtree_sum` | axis B random forest: 100 nodes, **unique** ids, each `parent_id` drawn from an *earlier* node's id or a dangling value (`-1`, `0`, unknown) ⇒ acyclic by construction, random values, queried at every id + misses, 200 iters — bit-exact | [x] |
| C23 | `calculate_subtree_sum` | axis D: mixture producing `inf + (-inf)` → NaN, and NaN propagation through 3 levels — compare raw bits | [x] |
| C23b | `calculate_subtree_sum` | axis D: the **NaN-payload tie-break** — every ordered pair of 10 distinct NaN bit patterns (+/- qNaN, +/- sNaN, custom payloads) as sibling values, as root+child, and mixed with `+/-inf`; plus 60 randomized 12-node trees of NaNs/infs — compared bit-for-bit | [x] |
| C24 | `find_node_by_id`/`get_children_count`/`calculate_subtree_sum` | axis E: `active` written through the `Node*` to `1`, `0`, `2`, `-1`, `INT_MIN`, `0x100`; re-query all three functions after each write | [x] |
| C25 | `maxnmin` | axis G: full cross-product of the 9 hand-picked classes per parameter, pruned to `param1,param2 ∈ {INT_MIN,-7,-6,-1,0,1,5,6,7,INT_MAX}` × `param3 ∈ {INT_MIN,-2,-1,0,1,INT_MAX}` × `param4 ∈ {INT_MIN,-3,-2,-1,0,1,2,3,INT_MAX}` (4 800 combos) | [x] |
| C26 | `maxnmin` | randomized: all four params uniform over the full `i32` range, 20 000 iters | [x] |
| C27 | `maxnmin` | randomized small params (`-16..=16`, hits every `%6`/`%3` residue incl. the NULL branches) exhaustively-ish, 20 000 iters | [x] |
| C28 | `maxnmin` | axis G: `param3 == -1` (÷0) crossed with `param1+param2` ∈ {0 → NaN, >0 → +inf, <0 → −inf} and `param4` ∈ {0 (inf*0 → NaN), ±1} | [x] |
| C29 | `maxnmin` | axis G overflow: `param3 == INT_MAX` (`param3+1` wraps), `param1 == param2 == INT_MAX` and `== INT_MIN` (`param1+param2` wraps), `result` accumulation wrap | [x] |
| C30 | `maxnmin` + all L0–L2 entry points | axis H: fresh lib → probe state → `maxnmin` → verify the 6 seeded nodes byte-for-byte (`id`, `parent_id`, `name`, `value`, `active`) via `find_node_by_id`, `get_children_count(-1,1,2,3)`, `calculate_subtree_sum(1..=6)` | [x] |
| C31 | `add_node` → `maxnmin` → low-level | axis H: pre-load 40 random nodes, call `maxnmin` (clobbers to 6), then re-probe every low-level function; then `add_node` 94 more and probe again | [x] |
| C32 | `maxnmin` | axis H: same params called 3× in a row (idempotence of the reset) and interleaved with different params, 512 random sequences | [x] |
| C33 | all 7 entry points | randomized **operation-sequence fuzz**: 300 random programs × 60 random ops each (`add_node`/`find_node_by_id` + field write / `get_children_count` / `calculate_subtree_sum` / `process_string` / `safe_double_to_int` / `maxnmin`), every return value and the full 80-byte-relevant field set compared after every op | [x] |

## Row → test mapping

| rows | test file |
|---|---|
| C1–C7 | `tests/configs_low.rs` |
| C8–C24b | `tests/configs_nodes.rs` |
| C25–C33 | `tests/configs_maxnmin.rs` |
| struct/ABI layout, symbol linkage | `tests/smoke.rs` |

All 34 rows pass under **both** the `dev` and the `release` profile, for the single
existing feature combination (`--no-default-features`). Driver: `./run_all.sh`.

## Suite adequacy (mutation testing)

The rows above are only worth something if they can actually fail. 15 single-edit
mutants were injected into `src/lib.rs`, rebuilt, and run against the suite:

| mutant | result |
|---|---|
| `strncpy` length `MAX_NAME_LEN-1` → `-2` | caught (1 test) |
| `char` sign-extension → zero-extension | caught (3) |
| `node_count >= MAX_NODES` → `>` | caught (1) |
| `param1 % 6` → `(param1 % 6).abs()` | caught (7) |
| `wrapping_add` → `saturating_add` for `param1+param2` | caught (5) |
| `param4 % 3` → `% 4` | caught (8) |
| `children * 10` → `* 11` | caught (9) |
| `param3 + 1` → `param3 + 2` | caught (9) |
| sNaN not quieted on propagation | caught (1) |
| `find_node_by_id` scans backwards (last match wins) | caught (2) |
| not-found returns `-0.0` instead of `+0.0` | caught (3) |
| `(param2 % 6) + 1` → `+ 2` | caught (9) |
| `Node` layout: `id`/`parent_id` swapped | caught (3) |
| `Node` layout: `name`/`value` swapped | caught (3) |
| `d > (double)INT_MAX` → `>=` | survived — **provably equivalent** (at `d == INT_MAX` both paths return `INT_MAX`; verified over 200 011 inputs) |
| drop `name[MAX_NAME_LEN-1] = '\0'` | survived — **provably equivalent** (the buffer is already zero-filled and `strncpy` writes at most bytes `0..=48`, so byte 49 is already NUL — the statement is redundant in the C too) |

13 of 13 non-equivalent mutants were caught.
