# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the axes `c_src/src/driver.c` actually branches on.

## Public entry points (complete set)

`c_src/include/driver.h` exposes exactly one:

* `void driver(int x, int y, int z)`  — the only externally linkable symbol
  (`nm -D` confirms: `T driver` and nothing else).

Lower-level internals, reachable only *through* `driver` (both are `static`, so
they cannot be called directly by a consumer — they are still exercised by
driving `driver` through every branch):

* `static int multi_stage(int x, int z)` — the staged validator; returns 0/1/2/3.
* `static int y = 123;` — file-scope mutable global, written by `driver` on
  every call and read by `multi_stage`.

## Axes the C code branches on

| axis | values the code distinguishes | source of the branch |
|------|-------------------------------|----------------------|
| A1 `x` | `x == 1` vs `x != 1` | `if (x != 1)` (driver.c:33) |
| A2 `y` (set from `local_y`) | `y == 2` vs `y != 2` | `if (y != 2)` (driver.c:39) |
| A3 `z` | `z == 3` vs `z != 3` | `if (z != 3)` (driver.c:45) |
| A4 evaluation order / short-circuit | A1 is checked before A2 before A3; the first failure jumps to `fail:` and skips the rest | `goto fail` (driver.c:36,42,48) |
| A5 epilogue selection | success returns before `fail:`; failures fall into `fail:` and print an extra line | driver.c:51-56 |
| A6 global-state lifetime | `y` is file-scope `static`, assigned on every `driver` call (driver.c:60), so it is per-call, not per-process; its initialiser `123` is dead | driver.c:29,60 |
| A7 `int` value shape | any `int32_t`: negatives, `0`, the valid constants `1`/`2`/`3`, off-by-one neighbours, `INT_MIN`, `INT_MAX`, and arbitrary random values | parameter type `int` |
| A8 stdout formatting | fixed literals via `printf`/`puts`, plus one `%d` conversion of a value in `{0,1,2,3}` | driver.c:34,40,46,51,55,62 |

There are **no** runtime options/flags/modes, **no** `#ifdef`s, **no** setters,
**no** byte-order or element-type or count/size axes, and no buffers — the
configuration surface is exactly the cross product of A1×A2×A3 refined by the
value shapes in A7 and the call-sequencing in A6.

## Configuration table

Rows are the pruned cross product of A1×A2×A3 (8 combinations = every
distinguishable control-flow path), then the value-shape and sequencing rows the
code additionally distinguishes. `y` below means the effective value of the
static, i.e. the `local_y` argument.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|-------------------------------------------|-----|
| C1  | `driver` → `multi_stage` | `x==1, y==2, z==3` — the sole full-success path; `result=0`, `"Ok!"`, no `"Operation failed"` | [x] |
| C2  | `driver` → `multi_stage` | `x==1, y==2, z!=3` — stages 1,2 pass, stage 3 fails; randomized `z != 3` over full `i32` | [x] |
| C3  | `driver` → `multi_stage` | `x==1, y!=2, z==3` — stage 2 fails; `z` is valid but never reached; randomized `y != 2` | [x] |
| C4  | `driver` → `multi_stage` | `x==1, y!=2, z!=3` — stage 2 fails first, `z` check skipped (A4 short-circuit); randomized `y != 2`, `z != 3` | [x] |
| C5  | `driver` → `multi_stage` | `x!=1, y==2, z==3` — stage 1 fails; both later valid values unreached; randomized `x != 1` | [x] |
| C6  | `driver` → `multi_stage` | `x!=1, y==2, z!=3` — stage 1 fails first; randomized `x != 1`, `z != 3` | [x] |
| C7  | `driver` → `multi_stage` | `x!=1, y!=2, z==3` — stage 1 fails first, stage 2 skipped; randomized `x != 1`, `y != 2` | [x] |
| C8  | `driver` → `multi_stage` | `x!=1, y!=2, z!=3` — all three invalid, only stage 1 reported; randomized triple | [x] |
| C9  | `driver` | boundary shapes for `x` with `y==2, z==3`: `x ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` (off-by-one either side of the only valid value) | [x] |
| C10 | `driver` | boundary shapes for `y` with `x==1, z==3`: `y ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, 2, 3, 123, INT_MAX-1, INT_MAX}` (incl. `123`, the C static's dead initialiser) | [x] |
| C11 | `driver` | boundary shapes for `z` with `x==1, y==2`: `z ∈ {INT_MIN, INT_MIN+1, -1, 0, 2, 3, 4, INT_MAX-1, INT_MAX}` | [x] |
| C12 | `driver` | full cross product of the boundary/interesting set `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, 3, 4, 123, INT_MAX-1, INT_MAX}` on all three parameters (12³ = 1728 configurations, exhaustive over the special values) | [x] |
| C13 | `driver` | unconstrained randomized triples over the full `i32` range (seeded, reproducible) — value-independence of the non-branching bits | [x] |
| C14 | `driver` | biased randomized triples that hit the valid constants far more often (each parameter drawn from `{1,2,3,0,-1,random}`) so deep stages are reached frequently | [x] |
| C15 | `driver` (A6 state axis) | repeated calls in one process: success → success, i.e. `(1,2,3)` twice, verifying `y` is not corrupted and results are idempotent | [x] |
| C16 | `driver` (A6 state axis) | repeated calls in one process: failure then success, e.g. `(9,9,9)` then `(1,2,3)` — the static `y` left at `9` must be overwritten | [x] |
| C17 | `driver` (A6 state axis) | repeated calls in one process: success then failure, `(1,2,3)` then `(1,7,3)` — no stale `"Ok!"`/state leakage | [x] |
| C18 | `driver` (A6 state axis) | long randomized call SEQUENCE (hundreds of calls, one process, one `dlopen`) — the concatenated stdout of C and Rust must match byte-for-byte, proving no cross-call state divergence in the atomic-vs-plain global translation | [x] |
| C19 | `driver` (A6 fresh-process axis) | the very first call after `dlopen`, before the static has ever been written, for both a succeeding and a failing first call — confirms the `123` initialiser is unobservable in both implementations | [x] |
| C20 | `driver` (A8 formatting axis) | all four reachable `result` values `{0,1,2,3}` rendered by `"Result: %d\n"`, and the exact literal bytes of all five messages (C uses `puts` for the no-arg `printf`s, Rust uses `printf("%s")` — the emitted bytes must be identical) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds `add_library(driver SHARED ...)` only — there is no
`add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **The project builds no binary
driver**, so the "compare C and Rust binary stdout" clause has no applicable
target; stdout equivalence is instead verified through the `.so` boundary by
fd-redirect capture in every row above.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section ⇒ exactly one configuration
(default == `--no-default-features`). All rows are verified under it.

## Row → test mapping (Phase B, all passing)

Run with `RUST_TEST_THREADS=1 cargo test --release --test phase_b_valid_paths`.

| row | test in `tests/phase_b_valid_paths.rs` |
|-----|----------------------------------------|
| C1 | `c1_all_valid_success_path` |
| C2 | `c2_x_ok_y_ok_z_bad` |
| C3 | `c3_x_ok_y_bad_z_ok` |
| C4 | `c4_x_ok_y_bad_z_bad_short_circuits_on_y` |
| C5 | `c5_x_bad_y_ok_z_ok` |
| C6 | `c6_x_bad_y_ok_z_bad_short_circuits_on_x` |
| C7 | `c7_x_bad_y_bad_z_ok_short_circuits_on_x` |
| C8 | `c8_all_three_bad_reports_only_x` |
| C9 | `c9_x_boundary_sweep` |
| C10 | `c10_y_boundary_sweep` |
| C11 | `c11_z_boundary_sweep` |
| C12 | `c12_exhaustive_special_value_cross_product` |
| C13 | `c13_uniform_random_triples_full_i32_range` |
| C14 | `c14_biased_random_triples_reach_deep_stages` (includes a coverage guard asserting all four outcomes are actually reached) |
| C15 | `c15_repeated_success_calls_are_idempotent` |
| C16 | `c16_failure_then_success_static_is_overwritten` |
| C17 | `c17_success_then_failure_no_state_leak` |
| C18 | `c18_long_random_call_sequence_in_one_process` |
| C19 | `c19_first_call_after_dlopen_in_fresh_process` (+ its `c19_oneshot_worker` subprocess helper) |
| C20 | `c20_exact_message_bytes_and_result_codes` |
| (guard) | `project_builds_no_binary_executable` — asserts `c_src/CMakeLists.txt` still has no `add_executable` and `Cargo.toml` no `[[bin]]` |

Result: **22 passed, 0 failed.**

## Test-harness note

Every row calls BOTH `.so`s only via `dlopen`/`dlsym` on the exported `driver`
symbol (`libloading`); the Rust crate is never linked or called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrapper is itself under test. Because the
library's only output channel is libc `stdout`, the harness saves fd 1, redirects
it to a temp file around each call, flushes both the Rust `Stdout` LineWriter and
the libc `stdout` FILE buffer, then restores fd 1. That is process-global state,
so **`RUST_TEST_THREADS=1` is mandatory** (`common::require_serial_harness`
enforces it with a clear message); use `scripts/run_tests.sh`.
