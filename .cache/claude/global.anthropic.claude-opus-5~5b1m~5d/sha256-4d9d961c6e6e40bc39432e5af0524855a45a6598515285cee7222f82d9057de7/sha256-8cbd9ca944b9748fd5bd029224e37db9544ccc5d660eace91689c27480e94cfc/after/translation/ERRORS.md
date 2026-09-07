# ERRORS.md — Phase A: error-surface table

Mechanically derived from every rejection / error-return site in
`c_src/src/lib.c`. There are no `assert`s, no null-pointer parameters (the only
public function takes four `int`s by value), and no allocation failures.
Status constants: `STATUS_OK 0000=0`, `STATUS_WARNING 0001=1`,
`STATUS_ERROR 0002=2`, `STATUS_CRITICAL 0377=255`.

## Rejections reachable through the public `.so` boundary

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `jumpnode` (case `0001`, lib.c:124) | `operation_mode == 1` and `find_node_by_id(node_id) == NULL`. In the shipped `.so` `node_count` is permanently `0` (`initialize_test_data` is `static` and never called), so this fires for **every** `node_id`. | `STATUS_ERROR \| 0020` = `2\|16` = **18** | `err_row1_mode1_node_not_found` | [x] |
| 2 | `jumpnode` (case `0002`, lib.c:145) | `operation_mode == 2` and `find_node_by_id(node_id) == NULL` — again every `node_id` in the shipped `.so`. | `STATUS_ERROR \| 0040` = `2\|32` = **34** | `err_row2_mode2_node_not_found` | [x] |
| 3 | `jumpnode` (case `0004`, lib.c:174) | `operation_mode == 4` and `find_node_by_id(node_id) == NULL` — every `node_id` in the shipped `.so`. | `STATUS_ERROR \| 0100` = `2\|64` = **66** | `err_row3_mode4_node_not_found` | [x] |
| 4 | `jumpnode` (`default:`, lib.c:202) | `operation_mode` is not one of `1,2,3,4` — i.e. any other `int`, including `0`, negatives, `5`, `INT_MIN`, `INT_MAX`, and out-of-range "enum" values passed across FFI. | `STATUS_ERROR \| 0200` = `2\|128` = **130** | `err_row4_default_mode`, `err_out_of_range_mode_exhaustive`, `err_mode_boundaries` | [x] |
| 5 | `add_node` (lib.c:56) | `node_count >= MAX_NODES` (100) on entry. Note this is a **signed `int`** comparison. | returns `STATUS_ERROR` = **2**, node not stored, `node_count` unchanged | `err_row5_add_node_capacity`, `err_add_node_capacity_guard_is_a_signed_compare` (via the C harness; unreachable from the public `.so` because `add_node` is `static` and only ever called 7 times by dead code) | [x] |

## Implicit clamps / saturations (non-erroring rejections of out-of-range values)

| # | function | trigger | expected C result | test | ✔ |
|---|----------|---------|-------------------|------|---|
| 6 | `safe_double_to_int` (lib.c:101) | `value > 2147483647.0` | clamped to `2147483647` | `err_row6_clamp_high` (mode 4, huge `depth`, via harness) | [x] |
| 7 | `safe_double_to_int` (lib.c:104) | `value < -2147483648.0` | clamped to `-2147483648` | `err_row7_clamp_low` (mode 4, very negative `depth`, via harness) | [x] |
| 8 | `process_backward` (lib.c:81) | `start_offset >= size` (i.e. `depth >= 16` in case `0002`) ⇒ `ptr > start` false immediately | sum contribution **0** (no iterations); result is `16 * flags` only | `err_row8_depth_ge_size` (via harness) | [x] |

## Documented C undefined behaviour — excluded from differential assertions

Recorded for completeness; these are *not* rejections, they are UB, so
byte-identical behaviour is not required and not asserted.

| # | site | trigger | why excluded |
|---|------|---------|--------------|
| U1 | `process_backward` (lib.c:78-84) | `start_offset < 0` in case `0002` ⇒ `start` is before `array`, so the loop walks off the bottom of the 20-`int` stack array reading uninitialised/other stack memory | out-of-bounds read; value depends on the compiler's stack layout. Reachable only when a node exists, i.e. only through the test harness, and only for negative `depth`. |
| U2 | `jumpnode` case `0002` (lib.c:161) | `(int)array_size * flags` overflows `int` | signed overflow UB; Rust uses `wrapping_mul`, which is what both compilers actually emit. Randomized tests keep `flags` small enough to avoid it, and a separate wrapping cross-check is done in Phase B. |
| U3 | `safe_double_to_int` (lib.c:108) | `value` is NaN | `(int)NaN` is UB in C. **Measured**: C returns `-2147483648` (x86-64 `cvttsd2si` sentinel), Rust's `as` returns `0`. This is the *only* input on which the two disagree, and it is unreachable from the public API: `add_node` hardcodes `data[] = {0100,0200,0300,0400}` so `sqrt` never sees a negative argument, and case `0001` only ever sums finite stored `value`s, so no `inf - inf` can arise. Left as a faithful non-UB translation rather than hard-coding a platform-specific UB result. Asserted unreachable by `nan_divergence_is_unreachable_from_public_api`. |
| U4 | `add_node` (lib.c:56-69) | `node_count < 0` on entry | C's guard is a **signed** compare, so a negative count does not trip it and the function stores into `node_storage[negative]`. In this link `node_storage[-1]` overlaps `node_count` *exactly* (`nm`: node_count @ 0x4040, node_storage @ 0x4060 — precisely `sizeof(Node)` = 32 apart), so `node_storage[node_count].id = id` overwrites `node_count` with `id`; every later field assignment re-reads that clobbered index and stores ~135 KB past the 3200-byte array, and the C **SIGSEGVs**. Not differentially testable — the C cannot survive the input. **A real translation bug was found and fixed here anyway**: the Rust had `NODE_COUNT as usize >= MAX_NODES`, whose unsigned cast rejects (returns `STATUS_ERROR`) inputs the C accepts (returns `STATUS_OK`); it is now the signed `NODE_COUNT >= MAX_NODES as c_int`. Unreachable from the public API, where `node_count` only ever starts at 0 and is incremented. |
| U5 | `process_backward` (lib.c:78) | `size >= 2^61` via the test hook | `array + size` overflows the address computation. C silently wraps; Rust's `ptr::offset` precondition check aborts when built with `-C debug-assertions=on`. Both are UB, and `jumpnode` only ever passes the constant `0o20`, so this is a build-config artifact rather than a semantic difference. Tests cap `size` at `u16::MAX`. |
