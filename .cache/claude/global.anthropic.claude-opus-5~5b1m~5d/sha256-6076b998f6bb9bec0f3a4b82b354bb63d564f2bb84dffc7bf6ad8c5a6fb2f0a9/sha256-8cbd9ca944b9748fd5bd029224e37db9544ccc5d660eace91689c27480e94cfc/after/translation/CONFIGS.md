# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

This library exposes **no runtime option/flag setters** and contains **no
`#ifdef`** conditionals. The configuration surface is therefore the cross
product of:

1. **Entry point** — all 12 exported symbols, *including* the lowest-level ones.
   `lib.h` only declares `mathop`, but the `.so` exports 11 more with external
   linkage; a real consumer can `dlsym` any of them, so all are in scope.
2. **`Operation` selector value** (`select_operation` `switch`, line 89): the 5
   `case`s + the `default:` branch = 6 distinct paths.
3. **Operand shape** for the arithmetic kernels: `0`, `+1`, `-1`, small
   positive, small negative, `INT_MAX`, `INT_MIN`, and the divisor-`== 0` guard.
4. **`char` domain** for `is_valid_operation`: NUL / below `'1'` / `'1'..'5'` /
   above `'5'` / negative (high bit set, since `char` is signed on x86-64).
5. **History state** for `perform_computation_with_history` (lines 122-132):
   `*history == NULL` vs already allocated × `*history_count` in
   `{0, 1..8, 9, 10, >10, negative}` — the `< 10` bound is the branch.
6. **`mathop` static state** (lines 138-139): `computation_history` /
   `history_count` are function-`static`, so `mathop` is **stateful across
   calls**. Each call appends 2 entries, so the printed `History entries:`
   walks 2 → 4 → 6 → 8 → 10 and then *saturates* at 10 forever. This is a
   sequence axis, not a per-call one, and requires C and Rust to be driven in
   **lockstep from a fresh process**.
7. **`mathop` stdout** (lines 168-171): 4 `printf` lines. Byte-for-byte stdout
   is part of the observable output and is compared, not just the return value.

## Rows

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `is_valid_operation` | **exhaustive** sweep of all 256 `char` bit patterns (covers NUL, `<'1'`, in-range `'1'..'5'`, `>'5'`, and all 128 negative values) | [x] |
| 2 | `get_operation_priority` | valid enum `1..5` | [x] |
| 3 | `get_operation_priority` | out-of-enum: `0`, `6`, `-1`, `-5`, `INT_MAX`, `INT_MIN`, plus randomized `i32` (`op*10` overflow) | [x] |
| 4 | `add_operation` | randomized `(a,b)` + boundary grid `{0,±1,±2,INT_MAX,INT_MIN}²`; 3rd `unused_param` varied to prove it is ignored | [x] |
| 5 | `subtract_operation` | randomized `(a,b)` + boundary grid; `unused_param` varied | [x] |
| 6 | `multiply_operation` | randomized `(a,b)` + boundary grid; `unused_param` varied | [x] |
| 7 | `divide_operation` | randomized `(a,b)` with `b != 0` + boundary grid excluding the `INT_MIN/-1` trap; `unused_param` varied | [x] |
| 8 | `modulo_operation` | randomized `(a,b)` with `b != 0` + boundary grid excluding `INT_MIN%-1`; `unused_param` varied | [x] |
| 9 | `select_operation` | each of the 5 valid `case`s — the returned **function pointer is invoked** through the FFI boundary and its results compared (pointer *values* differ between `.so`s by construction, so behaviour is what is compared) | [x] |
| 10 | `select_operation` | `default:` branch (`0`, `6`, `-1`, `INT_MIN`, `INT_MAX`, randomized) — returned pointer must behave as `add_operation` | [x] |
| 11 | `get_computation_timestamp` | no inputs; `time(NULL) >> 29` must agree between the two `.so`s (the `>>29` makes the value stable for ~17 years, so it is deterministic) | [x] |
| 12 | `allocate_results` | `count = 10` (the value `perform_computation_with_history` uses): pointer non-NULL, and all `count * sizeof(ComputationResult)` bytes must be **zeroed** by `calloc` | [x] |
| 13 | `allocate_results` | `count` in `{1, 2, 9, 10, 11, 100, 1000}`: zeroing + `sizeof(ComputationResult) == 24` layout agreement (`int` + 4 pad + `time_t` + `int` + 4 pad) | [x] |
| 14 | `perform_computation_with_history` | `*history == NULL`, `*history_count` garbage → allocates, resets count to 0, writes slot 0; for **each** of the 5 valid ops, randomized `(a,b)` | [x] |
| 15 | `perform_computation_with_history` | `*history` pre-allocated by the **same** `.so`'s `allocate_results`, count `0`, filling slots one at a time up to 9 — the full `ComputationResult` struct (`value`/`timestamp`/`status`) of every slot compared byte-for-byte | [x] |
| 16 | `perform_computation_with_history` | pre-allocated, driven **10 times** so the 10th call fills the last slot and the 11th+ hit the `< 10` bound; buffer + count compared after every call | [x] |
| 17 | `perform_computation_with_history` | pre-allocated, `*history_count` seeded to `9` (last writable slot — boundary value) then to `10` (first rejecting value) | [x] |
| 18 | `perform_computation_with_history` | out-of-enum `op` (`0`, `6`, `-1`, `INT_MIN`) with randomized `(a,b)` → must record `a + b` | [x] |
| 19 | `mathop` | `param3 % 5 + 1` sweeping **each** of the 5 valid first-ops × `param4` sweeping **each** of the 5 valid second-ops (5×5 grid), with `param1` chosen so the validation char is *valid* | [x] |
| 20 | `mathop` | same 5×5 op grid but with `param1` chosen so `(char)(param1 % 128)` is *invalid* (the dead-store branch) | [x] |
| 21 | `mathop` | randomized `(param1, param2, param3, param4)` over the full `i32` range, ~600 cases, C and Rust called in **lockstep** so their static histories stay in sync | [x] |
| 22 | `mathop` | boundary params `{0, ±1, ±2, ±4, ±5, ±6, INT_MAX, INT_MIN}` in all four positions (covers negative-modulo enum escape and `param4 + 1` overflow) | [x] |
| 23 | `mathop` | **stateful sequence from a fresh process**: 8 consecutive calls, asserting the `History entries:` line walks 2,4,6,8,10,10,10,10 identically in both | [x] |
| 24 | `mathop` | **stdout byte-for-byte**: all 4 `printf` lines captured via `dup2` redirection and compared for a fresh-process lockstep sequence | [x] |
| 25 | full pipeline | `select_operation` → returned pointer → `perform_computation_with_history` on a caller-owned buffer → `mathop`, composed in one process, so the composed pipeline (not just per-wrapper calls) is differentiated | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so there is exactly one
configuration. `cargo test` and `cargo test --no-default-features` build the
identical crate; both are run.
