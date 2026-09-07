# CONFIGS.md — Configuration-surface table (valid inputs)

The ABI is a single entry point with four `int` parameters:

```c
int jumpnode(int operation_mode, int node_id, int depth, int flags);
```

There is no init/teardown call, no option struct, no `#ifdef`, no env var, and
no feature gate in `Cargo.toml`. The "configuration" of this library is
therefore exactly the cross-product of the axes `jumpnode` branches on:

## Axes derived from the C source

| axis | values the C actually distinguishes | where it branches |
|---|---|---|
| `operation_mode` | `0001`, `0002`, `0003`, `0004`, anything else | `switch (operation_mode)` + `default:` |
| `node_id` | used by `find_node_by_id` (modes 1,2,4) and by `%d` in `sprintf` (mode 3) → distinguished by **decimal digit count and sign** | `sprintf("Node_%d_Depth_%d")` → `strlen` |
| `depth` | mode 1: loop bound `i < depth`; mode 2: `start_offset` into `process_backward`; mode 3: `%d` digit count/sign; mode 4: scale `1.0 + depth*0.1` | `for`, pointer arith, `sprintf` |
| `flags` | mode 2: `+ 16*flags` (wraps); mode 3: `+ (flags & 0177)` — only low 7 bits; modes 1,4: ignored | `result +=` |
| library state `node_count` | only ever `0` — `initialize_test_data` is `static` with no callers, so `find_node_by_id` always returns NULL | `if (node_count >= MAX_NODES)`, `if (node_count > 2)` |

Because `node_count` is pinned at 0, modes 1/2/4 collapse to their NULL-node
returns (see `ERRORS.md` rows 1–3) and **mode 3 is the only arm with
value-dependent output**. The rows below still cover modes 1/2/4 across
randomized `node_id`/`depth`/`flags` to prove the collapse is identical on
both sides rather than assumed.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jumpnode` | mode `0001`, randomized `node_id`,`depth`,`flags` over full `i32` range | [x] |
| 2 | `jumpnode` | mode `0001`, `node_id` ∈ {1..7} (the ids `initialize_test_data` *would* create), `depth` ∈ {0,1,2,3,10,-1,INT_MAX} | [x] |
| 3 | `jumpnode` | mode `0002`, randomized `node_id`,`depth`,`flags` over full `i32` range | [x] |
| 4 | `jumpnode` | mode `0002`, `depth` at `process_backward` boundaries {-1,0,1,4,15,16,17,19,20,INT_MIN,INT_MAX}; `flags` at {0,1,-1,INT_MIN,INT_MAX} (16*flags overflow) | [x] |
| 5 | `jumpnode` | mode `0004`, randomized `node_id`,`depth`,`flags`; `depth` also at {0,-10,10,INT_MIN,INT_MAX} (scale factor sign/overflow) | [x] |
| 6 | `jumpnode` | mode `0003`, `node_id`=0, `depth`=0 → shortest buffer `"Node_0_Depth_0"` (len 14) | [x] |
| 7 | `jumpnode` | mode `0003`, both `node_id` and `depth` positive, each digit count 1..10 (cross-product of `±10^k`, `±(10^k−1)`) | [x] |
| 8 | `jumpnode` | mode `0003`, `node_id` negative / `depth` positive (leading `-` adds 1 char) | [x] |
| 9 | `jumpnode` | mode `0003`, `node_id` positive / `depth` negative | [x] |
| 10 | `jumpnode` | mode `0003`, both negative | [x] |
| 11 | `jumpnode` | mode `0003`, `node_id`=`INT_MIN`, `depth`=`INT_MIN` → longest buffer (len 34), widest `%d` magnitude | [x] |
| 12 | `jumpnode` | mode `0003`, `node_id`/`depth`=`INT_MAX`, `INT_MIN+1`, `-1`, `1`, `-9`,`9`,`-10`,`10` | [x] |
| 13 | `jumpnode` | mode `0003`, randomized `node_id`,`depth` over full `i32` range × randomized `flags` | [x] |
| 14 | `jumpnode` | mode `0003`, `flags` mask boundaries: {0, 1, 0o176, 0o177, 0o200, 0o377, -1, INT_MIN, INT_MAX} with fixed `node_id`/`depth` | [x] |
| 15 | `jumpnode` | `default:` arm — `operation_mode` exhaustively over `-1024..=1024` (skipping 1..4) × randomized other args | [x] |
| 16 | `jumpnode` | `default:` arm — `operation_mode` ∈ {INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, ±2^k for k=0..31, 0x10001, 0x100000001 truncated} | [x] |
| 17 | `jumpnode` | full 4-axis randomized fuzz: `operation_mode` biased to {1,2,3,4,0,5} ∪ uniform i32, all other args uniform i32, fixed-seed, 200 000 cases | [x] |
| 18 | `jumpnode` | repeated invocation / state-leak check: the same call repeated and interleaved with other modes must give identical results every time (proves neither side lazily initialises `node_storage`) | [x] |

## Not applicable

- **Binary executable**: `CMakeLists.txt` builds only `SHARED` library; the Rust
  crate declares `crate-type = ["cdylib"]` and has no `src/main.rs`. There is no
  driver stdout to compare.
- **Feature combinations**: `Cargo.toml` has no `[features]`. Verified in
  Phase D that `--no-default-features` is the same build.
- **Pointer/length/enum-struct arguments**: none in the ABI.

## Additional rows: the ABI-hidden low-level surface

Rows 1–18 above drive the only public entry point. But because `node_count` is
pinned at 0, that entry point can never reach `add_node`, `process_backward`,
`safe_double_to_int`'s clamps, `compute_size_metric` with a caller-chosen
string, the `0001` parent-chain accumulation, or the `0004` sqrt/backward-sum
block. Those are the LOWEST-LEVEL entry points in the C, and testing only the
one reachable wrapper would leave most of the translation unverified.

They are reached from both sides without modifying `c_src/`:

- **C**: `tests/c_probe/probe.c` `#include`s the pristine `c_src/src/lib.c`
  (path passed via `-DLIB_C_PATH`) and re-exports each `static` under a
  `probe_` name. Compiled to its own separate `.so` at test time; the CMake
  `.so` is untouched.
- **Rust**: a `#[cfg(test)]`-gated `probe` module in `src/lib.rs`, reached with
  `#[path = "../src/lib.rs"]`. It compiles away in real builds — Phase D
  asserts the shipped `cdylib` still exports only `jumpnode`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 19 | `add_node` | 130 sequential inserts with randomized id/parent/value (crossing `MAX_NODES`=100); `node_storage` compared field-by-field, `double` bit-exactly | [x] |
| 20 | `add_node` | `node_count` forced to {0,50,98,99} (accepts) and {100,101,1000,INT_MAX-1,INT_MAX} (rejects) | [x] |
| 21 | `initialize_test_data` | first call and repeat call; all 7 nodes compared field-by-field incl. ids, parent ids, values, `data[0..4]` | [x] |
| 22 | `find_node_by_id` | empty storage; canonical 7-node set; ids −5..15; randomized ids; duplicate-id storage (first match must win) | [x] |
| 23 | `safe_double_to_int` | 200 000 uniform random f64 **bit patterns** (NaNs, subnormals, ±inf) + 200 000 in-and-out-of-range magnitudes + every integer straddling both clamps | [x] |
| 24 | `compute_size_metric` | strings of every length 0..300 with random non-NUL bytes (incl. the empty string the ABI can't produce) | [x] |
| 25 | `process_backward` | 20 000 random arrays × `size` 0..=20 × `start_offset` 0..=24 (incl. `start_offset > size`); the exact case-`0002` array shape at every offset 0..=20; saturating `int` sums of INT_MAX/INT_MIN elements | [x] |
| 26 | `sprintf`/`%d` | node_id × depth over both signs, every digit width 1..10, `INT_MIN`/`INT_MAX`, plus 200 000 randomized pairs; **raw bytes** compared, not just the length | [x] |
| 27 | `jumpnode` modes 1/2/3/4/default **with populated storage** | 8 storage scenarios (canonical 7-node set; empty; 1 node; 2 nodes; 30-deep chain; a parent **cycle**; extreme ±1e300 / near-clamp values; randomized 40-node graph) × modes × node_ids × depths × flags, plus a randomized sweep per scenario | [x] |
| 28 | `jumpnode` mode `0004` float sensitivity | `depth` swept densely up to the INT_MAX/INT_MIN clamps (~±1.6e7), where a sub-ULP difference in `2.718281828`, in `sqrt`, or in the clamp moves the truncated result by ~0.8 | [x] |
| 29 | `jumpnode` mode `0001` accumulation sensitivity | 400 randomized parent chains (40 deep) with values chosen to straddle integer truncation boundaries and to overflow past both clamps × node_ids × depths | [x] |

## Harness validation (why these rows are not vacuous)

Coverage claims were checked by mutation testing rather than assumed: 31
single-edit mutations were injected into `src/lib.rs` one at a time and the
suite re-run. **All 31 were caught.** They included every error constant, the
`0o177` mask, the NaN result, both clamps, the `process_backward` loop
comparison, the `2.718281828` and `1.5` and `0.1` float constants, the
`data[]` initialisers, `MAX_NODES`, both `sprintf` literals, the `node_count >
2` guard, the backward-walk trip count, `initialize_test_data`'s node values /
ids / parents / ordering, `find_node_by_id`'s first-match rule, and removing
`#[no_mangle]`.

Two blind spots were found and fixed this way rather than shipped:

1. `cargo test` does **not** build the `cdylib` (no test target links it), so
   the ABI tests were silently `dlopen`ing a **stale** `.so`. Dropping
   `#[no_mangle]` initially produced zero failures. The harness now runs
   `cargo build --lib` itself and refuses to run if the artifact is older than
   `src/lib.rs`.
2. `node_storage` contents were only ever read back from the C side, so a
   changed value in `initialize_test_data` went undetected. The Rust probe now
   exposes field readers and `assert_storage_identical` compares every node
   field on both sides, with the `double` compared by bit pattern.
