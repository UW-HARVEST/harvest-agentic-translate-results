# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

## Mechanical derivation of the axes

The complete public API (`c_src/include/staticloop.h`):

```c
int  static_sum(int update);
void driver(int update);      /* definition names the parameter `stride` */
```

Branch/state inventory of `c_src/src/staticloop.c` (greps in `ERRORS.md`):

- runtime options / modes / flags: **none.** No setters, no config struct, no
  globals with external linkage, no environment reads, no `#ifdef` build flags
  (the header's include guard is the only preprocessor conditional), and
  `translation/Cargo.toml` has no `[features]`. So there is no option axis.
- control flow the C actually branches on: exactly one construct —
  `for (int i = 0; i < 10; i++)` (fixed trip count 10, not input dependent).
- **hidden state axis (the real one):** `static int sum = 0;` inside
  `static_sum` has static storage duration, so it is process-wide,
  zero-initialised once, and *mutated by every call*. Behaviour therefore
  depends on the whole call history, not on the current argument alone. This is
  the axis where interaction bugs can hide, so it is enumerated explicitly
  below (accumulator sign/magnitude class × call-sequence shape).
- **call-hierarchy axis:** `driver` is the convenience wrapper; `static_sum` is
  the lowest-level entry point and is *also* called internally by `driver`, so
  the two entry points share one accumulator. Rows therefore include the
  interleavings of the low-level and the wrapper entry point, not just each one
  alone.
- **observable-output axis:** `static_sum` yields a return value; `driver`
  yields libc `printf("%d\n", …)` bytes on stdout. Both are compared
  byte-for-byte (stdout is captured with `dup`/`dup2` + `fflush`).
- input shape axis for the single `int` parameter: sign (`0`, `+`, `-`),
  magnitude (small / large / extremal `INT_MIN`, `INT_MAX`), and whether it
  drives `i * stride` or `sum += update` past the `int` range.
- count axis: 0 / 1 / many calls (`empty / one / many`).

## Differential-test method (applies to every row)

Both `.so` files are `dlopen`ed in one process, so each keeps its *own* copy of
the accumulator. Every row drives **the identical call sequence into both
libraries in lockstep** and compares after each step, so the rows are
order-independent and no `dlclose` state reset is needed:

- `static_sum`: assert `c_static_sum(x) == rust_static_sum(x)` for each `x`.
- `driver`: capture the C stdout bytes and the Rust stdout bytes for the same
  `stride` and assert they are byte-identical.

Randomized rows use a fixed-seed SplitMix64 PRNG (seed noted per row) and
many inputs per row, per Phase B.

## Configuration-surface rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `static_sum` | count axis: exactly **one** call, from the pristine zero-initialised accumulator (`update = 0`) — proves both libs start at 0 | `initial_state.rs` (own process, so the accumulator is untouched) | [x] |
| C2 | `static_sum` | count axis: **zero** calls to `driver`, **many** (1000) sequential calls, small positive updates `1..=1000` | `configs::c2_many_small_positive` | [x] |
| C3 | `static_sum` | small **negative** updates only, many calls (accumulator driven negative) | `configs::c3_many_small_negative` | [x] |
| C4 | `static_sum` | **mixed sign** small updates, many calls (accumulator crosses zero repeatedly) | `configs::c4_mixed_sign_random` | [x] |
| C5 | `static_sum` | `update = 0` repeatedly (no-op shape, accumulator must be perfectly stable) | `configs::c5_repeated_zero` | [x] |
| C6 | `static_sum` | **full-range random** `int` updates (uniform over all 2^32 values), 4000 calls, seed `0x5EED_0001` — accumulator wanders and overflows in both directions | `configs::c6_full_range_random` | [x] |
| C7 | `static_sum` | **extremal** updates only: `INT_MAX`, `INT_MIN`, `INT_MAX`, `INT_MIN`, … (maximal wraparound pressure) | `configs::c7_extremal_alternating` | [x] |
| C8 | `static_sum` | accumulator parked **near `INT_MAX`**, then a sweep of `update ∈ {-2..2, ±INT_MAX, ±1}` (boundary neighbourhood) | `configs::c8_near_int_max_sweep` | [x] |
| C9 | `static_sum` | accumulator parked **near `INT_MIN`**, same sweep (opposite boundary neighbourhood) | `configs::c9_near_int_min_sweep` | [x] |
| C10 | `driver` | wrapper alone, **`stride = 1`** (canonical case: prints the triangular numbers) | `configs::c10_driver_stride_one` | [x] |
| C11 | `driver` | wrapper alone, **`stride = 0`** (degenerate shape: 10 identical lines) | `configs::c11_driver_stride_zero` | [x] |
| C12 | `driver` | wrapper alone, **small negative** stride (negative output formatting: `-` sign bytes) | `configs::c12_driver_small_negative` | [x] |
| C13 | `driver` | wrapper alone, **small positive random** strides, many rounds, seed `0x5EED_0002` | `configs::c13_driver_small_positive_random` | [x] |
| C14 | `driver` | wrapper alone, **full-range random** strides (`i * stride` overflows), 300 rounds, seed `0x5EED_0003` | `configs::c14_driver_full_range_random` | [x] |
| C15 | `driver` | wrapper alone, **extremal** strides `INT_MAX` / `INT_MIN` / `±1` | `configs::c15_driver_extremal_strides` | [x] |
| C16 | `driver` | count axis: **many consecutive** `driver` calls with the same stride (accumulator carried across calls — output of call *n* depends on calls `< n`) | `configs::c16_driver_repeated_same_stride` | [x] |
| C17 | `driver` + `static_sum` | **interleaving**: `static_sum` first (low-level entry point mutates the shared accumulator), then `driver` — proves `driver` reads the state the low-level API wrote | `configs::c17_static_sum_then_driver` | [x] |
| C18 | `driver` + `static_sum` | **interleaving**: `driver` first, then `static_sum` — proves the low-level API observes the 10 in-loop updates the wrapper performed | `configs::c18_driver_then_static_sum` | [x] |
| C19 | `driver` + `static_sum` | **fully randomized interleaving** of both entry points (random op choice + full-range random argument), 2000 ops, seed `0x5EED_0004` — the composed-pipeline row | `configs::c19_random_interleaved_pipeline` | [x] |
| C20 | `driver` + `static_sum` | interleaving with the accumulator deliberately parked at a **wrap boundary** before each `driver`, so the wrap happens *inside* the wrapper's loop | `configs::c20_driver_wrap_inside_loop` | [x] |
| C21 | `driver` | output-shape axis: strides producing **maximum-width** decimal output (10-digit + sign) so `printf` field widths / buffering are compared | `configs::c21_driver_max_width_output` | [x] |
| C22 | `static_sum` | return-value axis: sequence engineered so the return value hits `0`, `INT_MAX`, `INT_MIN`, `-1` exactly (every "interesting" return encoding) | `configs::c22_exact_return_landmarks` | [x] |

## Feature combinations (Phase D)

`translation/Cargo.toml` has no `[features]` table, so the powerset of features
is `{ {} }`: the default build *is* the only build. Verified mechanically by
`translation/check_all_features.sh`, which parses `Cargo.toml`, enumerates the
feature powerset, and runs `cargo check` + `cargo test` for each element
(default, and `--no-default-features`).

## Soak rows (the randomized rows above, at scale)

`translation/tests/soak.rs`. Scale factor via `STATICLOOP_SOAK` (default 1;
verified at 20 as well).

| # | entry point(s) | configuration | test | [x] |
|---|----------------|---------------|------|-----|
| S1 | `static_sum` | 2,000,000 full-`int`-domain random updates, seed `0xC0FFEE...0006` | `soak::soak_static_sum_full_domain` | [x] |
| S2 | `static_sum` | 200,000 updates drawn only from extremal/near-extremal values (maximal wrap traffic), seed `…0008` | `soak::soak_static_sum_boundary_biased` | [x] |
| S3 | `driver` + `static_sum` | 3,000 randomized interleaved ops, exact stdout bytes compared each time, seed `…0019` | `soak::soak_driver_and_interleaving` | [x] |
| S4 | `driver` | dense contiguous stride sweep `-300..=300` from a parked-zero accumulator | `soak::soak_driver_dense_small_strides` | [x] |

## How to run

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && ./check_all_features.sh          # every feature combo x debug+release
# or, manually:
cd translation && cargo build && cargo build --release && cargo test
STATICLOOP_SOAK=20 cargo test --test soak          # longer randomized sweep
```

Two harness properties are load-bearing and are enforced in code:

1. **`cargo test` does not relink the `cdylib`.** Verified experimentally: after
   editing `src/lib.rs`, `cargo test` left `target/debug/libStaticLoop.so`
   byte-identical (same md5), so the suite would have silently tested a stale
   library. `tests/common/mod.rs::assert_cdylib_is_fresh` now fails the run if the
   `.so` is older than `src/` or `Cargo.toml`, and `check_all_features.sh` always
   runs `cargo build` first.
2. **Tests must be single-threaded.** `driver`'s output is captured by rebinding
   fd 1, which is process-global; with parallel tests libtest's own
   `test … ok` progress lines landed inside a capture window and were mistaken
   for library output. `translation/.cargo/config.toml` sets
   `RUST_TEST_THREADS=1` (forced), so a plain `cargo test` is correct.

## Harness validation (negative controls)

A differential suite that passes is worthless if it cannot fail. Each of the
following mutations was applied to `translation/src/lib.rs`, rebuilt, and the
suite re-run; every one was caught (counts are matching failure/divergence lines
across the whole suite):

| mutation to the Rust translation | detected | signals |
|---|:--:|--:|
| `driver` loop bound `0..10` → `0..9` | yes | 43 |
| `printf` format `"%d\n"` → `"%u\n"` | yes | 21 |
| `printf` format `"%d\n"` → `"%d\n\n"` | yes | 25 |
| `i * stride` → `i * stride + 1` | yes | 43 |
| `SUM.wrapping_add` → `SUM.saturating_add` | yes | 35 |
| `static mut SUM: c_int = 0` → `= 1` | yes | 43 |
| `static_sum` result altered for the single input `update == 7` | yes | 43 |
| `#[no_mangle]` removed from `driver` (export wrapper gone) | yes | 43 |

So the suite is sensitive to arithmetic, wrapping behaviour, output formatting,
loop bounds, static initialisation, single-value special cases, and the presence
of the `#[no_mangle]` export wrappers themselves.

## C-compiler-variance cross-check

`sum += update` and `i * stride` can overflow, which is UB in C, so the same
source can in principle behave differently at different optimisation levels. The
full suite was re-run against C shared libraries built at `-O0`, `-O1`, `-O2`,
`-O3`, `-Os`, `-Ofast` and `-O2 -fno-strict-overflow` (via `STATICLOOP_C_SO`).
**0 divergences at every level** — gcc emits two's-complement wraparound in all
of them, which is what the Rust `wrapping_add` / `wrapping_mul` reproduce. The
CMake build (the ground truth, no `CMAKE_BUILD_TYPE`, i.e. no `-O` flag) is the
default the suite runs against.
