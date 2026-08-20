# CONFIGS.md — Configuration surface table (Phase A → Phase B)

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Public entry points

`c_src/include/lib.h` declares exactly one:

```c
int jumpnode(int operation_mode, int node_id, int depth, int flags);
```

There are no other externally linkable functions (`nm -D` on the C `.so`
confirms one symbol — see `SYMBOLS.md`), and there is no lower-level public API:
`find_node_by_id`, `add_node`, `process_backward`, `compute_size_metric`,
`safe_double_to_int` and `initialize_test_data` are all `static`. They are
therefore exercised *through* `jumpnode` — the lowest-level entry point reachable
by an external caller is `jumpnode` itself.

## Axes the C actually branches on

| axis | source of the branch | distinct values the C treats differently |
|------|----------------------|------------------------------------------|
| `operation_mode` | `switch (operation_mode)` line 121 | `0001`, `0002`, `0003`, `0004`, everything else (`default:`) |
| `node_id` | `find_node_by_id(node_id)` lines 123/144/173; `sprintf("%d", node_id)` line 165 | for modes 1/2/4: any value (always not-found). For mode 3: **decimal width & sign** of the `%d` conversion — this is the only axis that changes the result: `0`, 1-digit, 2-digit … 10-digit, negative (adds `-`), `INT_MIN`, `INT_MAX` |
| `depth` | loop bound line 130; `process_backward` start offset line 159; `sprintf("%d", depth)` line 165; `1.0 + depth*0.1` line 183 | for mode 3: **decimal width & sign** of `%d`. For modes 1/2/4: unreachable (early error), but boundary values still probed at the public boundary |
| `flags` | `(int)array_size * flags` line 161; `flags & 0177` line 169 | mode 3: only the low 7 bits (`0..=127`) matter; sign/high bits must be discarded identically. Mode 2: multiplier (unreachable) |
| library state | `node_count` / `node_storage`, lines 37-38 | permanently `0` / all-zero because `initialize_test_data` is never called (see `ERRORS.md`). Not caller-settable — one state only |
| build config | no `#ifdef`, no CMake `option()`, no cargo `[features]` | one configuration only (see `SYMBOLS.md`) |

`STATUS_WARNING` (0001) and `STATUS_CRITICAL` (0377) are defined but never
referenced, so they contribute no branch.

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG in `tests/common/mod.rs`) in the axis positions the row leaves
free, plus the row's pinned boundary values.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jumpnode` | `operation_mode = 0001` (node walk), randomized `node_id`/`depth`/`flags` over full `i32` range | [x] |
| 2 | `jumpnode` | `operation_mode = 0001`, `depth` pinned to boundary shapes `{INT_MIN, -1, 0, 1, 2, 3, 100, INT_MAX}` × randomized `node_id`/`flags` | [x] |
| 3 | `jumpnode` | `operation_mode = 0002` (array backward-sum), randomized `node_id`/`depth`/`flags` over full `i32` range | [x] |
| 4 | `jumpnode` | `operation_mode = 0002`, `depth` pinned to the `process_backward` offset boundaries `{INT_MIN, -1, 0, 1, 4, 15, 16, 17, INT_MAX}` × randomized `flags` | [x] |
| 5 | `jumpnode` | `operation_mode = 0002`, `flags` pinned to multiplier boundaries `{0, 1, -1, INT_MIN, INT_MAX, 0x7f, 0x80, 134217728}` (overflow of `16 * flags`) × randomized `node_id`/`depth` | [x] |
| 6 | `jumpnode` | `operation_mode = 0003` (sprintf/strlen metric), randomized `node_id`/`depth`/`flags` over full `i32` range — the primary value-dependent path | [x] |
| 7 | `jumpnode` | `operation_mode = 0003`, `node_id` swept over **every decimal width**: `0`, ±1-digit … ±10-digit, `INT_MIN`, `INT_MAX`, and all powers-of-ten ±1 × randomized `depth`/`flags` | [x] |
| 8 | `jumpnode` | `operation_mode = 0003`, `depth` swept over every decimal width / powers-of-ten ±1 / `INT_MIN` / `INT_MAX` × randomized `node_id`/`flags` | [x] |
| 9 | `jumpnode` | `operation_mode = 0003`, **cross-product** of `node_id` × `depth` boundary sets (both widths varying simultaneously) | [x] |
| 10 | `jumpnode` | `operation_mode = 0003`, `flags` swept over all 128 mask residues plus `{INT_MIN, INT_MAX, -1, 0}` and randomized high bits with fixed low bits (verifies `& 0177` discards sign/high bits identically) | [x] |
| 11 | `jumpnode` | `operation_mode = 0004` (sqrt accumulation + backward scan), randomized `node_id`/`depth`/`flags` over full `i32` range | [x] |
| 12 | `jumpnode` | `operation_mode = 0004`, `depth` pinned to `{INT_MIN, -100, -11, -10, -9, -1, 0, 1, 10, INT_MAX}` (the `1.0 + depth*0.1` scale, incl. the sign-flip at `depth = -10`) × randomized `node_id`/`flags` | [x] |
| 13 | `jumpnode` | `operation_mode` = every value in `-8..=8` plus `{INT_MIN, INT_MIN+1, INT_MAX, 5, 8, 0x1_0001, 0o1000}` — mode dispatch incl. out-of-range "enum" ints × randomized other args | [x] |
| 14 | `jumpnode` | `operation_mode` randomized over the full `i32` range (mostly `default:`) × randomized other args — unbiased dispatch fuzz | [x] |
| 15 | `jumpnode` | full 4-axis fuzz: all four arguments randomized over the full `i32` range, with `operation_mode` biased into `1..=4` so the real work paths dominate | [x] |
| 16 | `jumpnode` | full 4-axis boundary cross-product: each argument drawn from `{INT_MIN, INT_MIN+1, -1, 0, 1, 2, 3, 4, 16, 127, 128, INT_MAX}` — exhaustive over the pruned boundary set | [x] |
| 17 | `jumpnode` | repeated / interleaved invocation (state-leak check): the same call issued many times interleaved across modes, asserting results stay identical (both libraries hold `static` mutable state) | [x] |
| 18 | `jumpnode` | identical sequence replayed against a **freshly `dlopen`ed** pair of libraries, asserting first-call results equal steady-state results (static-initialiser check) | [x] |

## Low-level entry points (rows 19-30)

Rows 1-18 drive the only entry point an external consumer has. But `jumpnode`
is a *dispatcher*, and because `initialize_test_data()` is never called, modes
1/2/4 return their sentinel before executing any real work — so rows 1-18 leave
most of the library's code paths unexercised.

Rows 19-30 therefore reach the `static` helpers directly, and drive `jumpnode`
itself over **populated node storage**, via the `shadow_probe` probe build
(`tests/deep_paths.rs`; see `SYMBOLS.md` for how it avoids touching `c_src/`).
These are the lowest-level entry points in the library, not convenience wrappers.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 19 | `sizeof(Node)`, `STATUS_*`, `MAX_NODES` | struct layout + every constant, compared across the FFI boundary (`#[repr(C)]` fidelity) | [x] |
| 20 | `safe_double_to_int` | `F64_BOUNDARIES` (26 values straddling both clamps), 200 000 randomized `f64` incl. NaN/±Inf/subnormal bit patterns, and ulp-by-ulp sweeps of 6 000 values around `±2^31` and `0.0` | [x] |
| 21 | `compute_size_metric` | every string length `0..=512`, plus `{1000, 4095, 4096, 65535, 65536, 100000}`; interior-NUL buffers proving `strlen` stops at the first NUL | [x] |
| 22 | `process_backward` | `size` `0..=40` × `start_offset` `0..=size+6` × 40 randomized arrays each; plus `INT_MAX`/`INT_MIN` element arrays to force identical wrapping of `sum +=`; plus the exact `size=16` shape mode 2 uses | [x] |
| 23 | `add_node` + `find_node_by_id` | 300 rounds × `0..30` nodes drawn from a deliberately small id space so **duplicate ids** occur (first-match-wins), with every stored field (`id`, `parent_id`, `value` bit pattern, `data[0..4]`) compared | [x] |
| 24 | `add_node` capacity | fill to exactly `MAX_NODES` (100) then overflow by 25 — asserts `STATUS_OK` below the limit and `STATUS_ERROR` at/after it, and that `node_count` saturates | [x] |
| 25 | `jumpnode` mode `0001`, populated | 250 rounds × proper trees (root `parent_id == -1`) of 1-20 nodes, randomized `f64` values, `depth` unbounded incl. `INT_MIN`/`INT_MAX` — the real parent walk | [x] |
| 26 | `jumpnode` mode `0001`, populated | 400 rounds × arbitrary parent links producing **dangling ids, self-loops and cycles**, `depth` bounded (a cycle iterates exactly `depth` times) | [x] |
| 27 | `jumpnode` mode `0002`, populated | 300 rounds × 1-10 nodes × `depth` `0..=22` (the in-bounds `process_backward` offset domain) × `flags` boundary set incl. `INT_MIN`/`INT_MAX`/`2^27` | [x] |
| 28 | `jumpnode` mode `0004`, populated | `node_count` `0..=6` (straddles the `node_count > 2` and `iter > node_storage` scan guards) × 120 rounds × `depth` boundary set incl. the `depth = -10` sign flip of `1.0 + depth*0.1` | [x] |
| 29 | `jumpnode` mode `0004`, populated | high-resolution `depth` sweep: 80 000 randomized depths over `±1.7e8`, the exhaustive clamp neighbourhood `160669700..160669830`, and dense `-20000..20000` — makes the result sensitive to a 4.9e-8 error in the sqrt constant | [x] |
| 30 | `probe_init` (= `initialize_test_data`) + `jumpnode` | the never-called initializer's real 7-node state, full 4-axis `jumpnode` sweep over it, init idempotence, then `probe_reset` restoring the pristine `.bss` state where the public sentinels reappear | [x] |
| 31 | `jumpnode`, all modes, populated | end-to-end fuzz: 600 rounds × random storage (0-15 nodes, random ids/parents/values) × 60 random `(mode, node_id, depth, flags)` calls per round, with all stored fields re-compared each round | [x] |

## Phase B rule

A row is checked off only after C and Rust agree **byte-for-byte** across all
randomized inputs for that row — `i32` equality on returned values, and `f64`
**bit-pattern** equality (`to_bits`) wherever a `double` crosses the boundary, so
that `-0.0` vs `0.0` and differing NaN payloads cannot slip through.

All rows pass under all four feature combinations (rows 19-31 require
`shadow_probe`; rows 1-18 run in every configuration).
