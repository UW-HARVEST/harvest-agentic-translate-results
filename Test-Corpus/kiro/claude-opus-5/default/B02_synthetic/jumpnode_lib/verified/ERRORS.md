# ERRORS.md — error-surface table

Every distinct rejection / error return in `c_src/src/lib.c`, found by grepping
for `return`, `NULL`, `>=`, `<`, `>` range checks and the `STATUS_*` constants.
No rows invented; each cites the C line it comes from.

`STATUS_ERROR` is `0002` octal = `2` decimal.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `add_node` | `node_count >= MAX_NODES` (100) | returns `STATUS_ERROR` = `2` |
| 2 | `find_node_by_id` | no element of `node_storage[0..node_count]` has `.id == id` (always true, `node_count==0`) | returns `NULL` |
| 3 | `jumpnode` mode `0001` | `find_node_by_id(node_id) == NULL` — i.e. **every** `node_id`, since `node_count==0` | returns `STATUS_ERROR \| 0020` = `2 \| 16` = **18** |
| 4 | `jumpnode` mode `0002` | `find_node_by_id(node_id) == NULL` — every `node_id` | returns `STATUS_ERROR \| 0040` = `2 \| 32` = **34** |
| 5 | `jumpnode` mode `0004` | `find_node_by_id(node_id) == NULL` — every `node_id` | returns `STATUS_ERROR \| 0100` = `2 \| 64` = **66** |
| 6 | `jumpnode` `default:` arm | `operation_mode` is any `int` other than `1`, `2`, `3`, `4` | returns `STATUS_ERROR \| 0200` = `2 \| 128` = **130** |
| 7 | `safe_double_to_int` | `value > 2147483647.0` (upper clamp) | value replaced by `2147483647.0`, returns `2147483647` |
| 8 | `safe_double_to_int` | `value < -2147483648.0` (lower clamp) | value replaced by `-2147483648.0`, returns `-2147483648` |
| 9 | `process_backward` | `start_offset >= (int)size` so `ptr > start` is false on entry | loop body never runs, returns `0` |
| 10 | `process_backward` | `start_offset < 0` — `start` is before `array`, loop reads `array[start_offset .. size)` out of bounds below the buffer | UB in C; unreachable from the public API (row 4 fires first) |

## Generic FFI boundary cases (not `RETURN_ERROR` rows, tested anyway)

| # | trigger | expected C result |
|---|---------|-------------------|
| G1 | `operation_mode = 0` (the `STATUS_OK` value, no `case` for it) | `130` (default arm) |
| G2 | `operation_mode = 5` (one past the last valid `case 0004`) | `130` |
| G3 | `operation_mode = -1` | `130` |
| G4 | `operation_mode = INT_MIN` / `INT_MAX` | `130` |
| G5 | `operation_mode = 0o377` (`STATUS_CRITICAL`), `0o200`, `0o177` — out-of-range "enum" values with no matching variant | `130` |
| G6 | `node_id = INT_MIN` / `INT_MAX` / `0` / `-1` in mode `0003` (widest `%d` output) | `18`/`34`/`66` for modes 1/2/4; mode 3 computes from the formatted length |
| G7 | `depth = INT_MIN` / `INT_MAX` / negative in every mode | per-mode; mode 3 length changes, modes 1/2/4 unaffected (early return) |
| G8 | `flags = INT_MIN` / `INT_MAX` / `-1` (mode 3 masks with `0177`, mode 2 multiplies by 16 → signed overflow) | mode 3: `+ (flags & 0177)`; modes 1/2/4: unaffected (early return) |

`jumpnode` takes four `int`s by value and no pointers, so there is no
null-pointer or length argument to abuse at the public boundary; the pointer
paths (`process_backward`, `find_node_by_id`, `compute_size_metric`) are only
reachable internally and are covered through `jumpnode`.

## Status

All rows have passing differential tests in `tests/differential.rs`
(`phase_c_*`). Rows 1, 2, 7, 8, 9, 10 are not directly observable through the
single exported symbol because the null-node early returns (rows 3–5) preempt
them; they are covered indirectly, and each is documented in the test file with
the reason it cannot be reached. See `tests/differential.rs::phase_c_unreachable_rows`.

- [x] 1 — unreachable through public ABI (documented); `add_node` only called by dead `initialize_test_data`
- [x] 2 — covered by rows 3/4/5 (every `node_id`, randomized)
- [x] 3 — `phase_c_row3_mode1_null_node`
- [x] 4 — `phase_c_row4_mode2_null_node`
- [x] 5 — `phase_c_row5_mode4_null_node`
- [x] 6 — `phase_c_row6_default_arm`, `phase_c_out_of_range_enum_values`
- [x] 7 — unreachable (mode 1/4 return early); clamp verified by code inspection + `phase_c_unreachable_rows`
- [x] 8 — unreachable (same reason)
- [x] 9 — unreachable (mode 2 returns early)
- [x] 10 — unreachable (mode 2 returns early); NOT exercised, since triggering it in C is UB
- [x] G1–G8 — `phase_c_generic_boundaries`, `phase_c_out_of_range_enum_values`

## Negative control (why these checkmarks mean something)

Passing tests alone do not prove a test suite has teeth. `./mutation_check.sh`
perturbs the Rust translation in one place at a time and requires the suite to
fail. Result: **17 mutations caught, 7 escaped, 0 unexpected escapes.**

The first version of the harness caught **zero** of them. Root cause:
`cargo test` does not relink a `cdylib` artifact, so the tests were `dlopen`ing a
stale `.so` left behind by an earlier `cargo build --release`. The harness now
runs a nested `cargo build --lib` into its own `--target-dir` on every run, so
the loaded `.so` always matches `src/lib.rs`.

### The 7 escapes, each with a verified reason

| mutation | why it cannot be observed |
|----------|----------------------------|
| `mode2 array_size 0o20 -> 0o17` | mode `0002` always returns `34` first (row 4); everything after is dead code |
| `mode4 constant 2.718281828 -> 2.7` | mode `0004` always returns `66` first (row 5) |
| `mode1 parent weight 1.5 -> 2.5` | mode `0001` always returns `18` first (row 3) |
| `safe_double_to_int upper clamp off by one` | only called from modes `0001`/`0004`, both of which return early |
| `%d digits base 10 -> base 9` | mode `0003` passes the buffer to `compute_size_metric`, which uses only `strlen` — digit *content* is unobservable, only *length* is |
| `%d zero prints empty` (`'0'` → `' '`) | same: one byte either way, so `strlen` is unchanged |
| `c_strlen off by one` (`n == 0 \|\|`) | semantic no-op: the formatted buffer is never empty (shortest output `"Node_0_Depth_0"`, 14 bytes) |

The two "content is unobservable" rows are not a coverage gap: three separate
mutations that change the formatted **length** by ±1 (`%d emits an EXTRA digit`,
`%d zero emits NO digit`, `sprintf literal _Depth_ LENGTHENED`) are all caught,
which proves the length is genuinely under test. Likewise
`NODE_COUNT starts at 7` — the mutation that simulates
`initialize_test_data()` actually running — is caught, which is the positive
evidence that the dead-code claims above are real and not an excuse.
