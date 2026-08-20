# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

Mechanically derived from `c_src/include/staticloop.h` (the *whole* public API)
and from every branch in `c_src/src/staticloop.c`.

## Build-time configuration axes

| source | axes found | enumeration |
|--------|------------|-------------|
| `Cargo.toml` | **no `[features]` table at all** | exactly one valid combination: the empty feature set |
| `c_src/CMakeLists.txt` | no `option()`, no `target_compile_definitions`, no `add_definitions` | single configuration |
| `c_src/src/staticloop.c` | `grep -nE '#if|#ifdef|#ifndef' c_src/src/staticloop.c` → **no matches** | no conditional compilation |
| `c_src/include/staticloop.h` | only the `STATICLOOP_H_` include guard | no conditional API |

**Complete list of valid feature combinations:**

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo check/test --no-default-features` (features: `""`) | the only combination — there are no features to enable |
| 2 | `cargo check/test` (default features) | identical to #1, because no `default` feature exists |

Both are verified by `./check_features.sh` (Phase D).

## Runtime configuration axes (what the C actually branches on)

There are **no** runtime option/mode/flag setters in the API — the header
exposes only two functions and neither takes a flag. The axes the C code's
behaviour actually depends on are therefore:

1. **Entry point** — `static_sum` (the low-level accumulator, called directly)
   and `driver` (the composed wrapper that calls `static_sum` 10× and `printf`s
   each result). Both are exercised directly; `driver` is *not* used as the only
   way to reach `static_sum`.
2. **Hidden library state** — the function-local `static int sum = 0;`. This is
   the library's real "mode": the result of any call depends on the accumulated
   state, which is shared by *both* entry points. Shapes: fresh (`0`), small
   positive, small negative, near `INT_MAX`, near `INT_MIN`, exactly `INT_MAX`,
   exactly `INT_MIN`.
3. **Argument value shape** — the single `int` argument. Shapes:
   `0`, `+1`, `-1`, small ±, large ±, powers of two, `INT_MAX`, `INT_MIN`,
   `INT_MAX ± 1`-class neighbours, and the `driver`-specific multiplication
   boundary `INT_MAX / 9 = 238609294` (largest `stride` for which `i * stride`
   never overflows, since `i` peaks at 9).
4. **Call-sequence shape** — "empty / one / many": 1 call, 2 calls, hundreds of
   calls, and *interleavings* of the two entry points (the state coupling
   between them is the only cross-function behaviour in the library).
5. **Output-format shape** (`driver` only, via `printf("%d\n", …)`) — printed
   value sign and digit width: `0`, 1-digit, multi-digit, 10-digit, negative
   with `-` sign, and the special-case `INT_MIN` (`-2147483648`, 11 chars).
   Compared byte-for-byte by redirecting fd 1 to a file around each call.
6. **Loop-trip shape** — hard-coded `for (int i = 0; i < 10; i++)`; the trip
   count is *not* caller-controlled, so "line count" is a fixed invariant to
   assert rather than an axis to vary.

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** (deterministic `SplitMix64`
seeded per row) unless the row is itself a specific boundary constant, and every
row compares the C `.so` and the Rust `.so` loaded via `libloading`.

| #   | entry point(s) | configuration (options set + input shape) | test fn | ✔ |
|-----|----------------|------------------------------------------|---------|---|
| C1  | `static_sum` | fresh/any state, `update = 0` (degenerate zero update), repeated | `cfg_c1_static_sum_zero_update` | [x] |
| C2  | `static_sum` | `update = +1` and small random positives `1..=1000`, 500 calls | `cfg_c2_static_sum_small_positive` | [x] |
| C3  | `static_sum` | `update = -1` and small random negatives `-1000..=-1`, 500 calls | `cfg_c3_static_sum_small_negative` | [x] |
| C4  | `static_sum` | mixed-sign random `-1000..=1000`, 1000 calls (state walks around 0, sign flips) | `cfg_c4_static_sum_mixed_small` | [x] |
| C5  | `static_sum` | full-range random `i32::MIN..=i32::MAX`, 2000 calls (wrap-heavy) | `cfg_c5_static_sum_full_range_random` | [x] |
| C6  | `static_sum` | boundary constants: `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `0`, `1`, `-1`, `±2^k` for all k | `cfg_c6_static_sum_boundary_constants` | [x] |
| C7  | `static_sum` | state driven to **exactly** `INT_MAX` then `+1`; to **exactly** `INT_MIN` then `-1` (precise wrap points) | `cfg_c7_static_sum_exact_wrap_points` | [x] |
| C8  | `static_sum` | call-count shape: 1 call, 2 calls, many identical calls with the same argument | `cfg_c8_static_sum_call_count_shapes` | [x] |
| C9  | `driver` | `stride = 0` — prints the current total 10× (output shape: repeated identical lines) | `cfg_c9_driver_stride_zero` | [x] |
| C10 | `driver` | `stride = 1` from fresh state — canonical `0,1,3,6,…,45` | `cfg_c10_driver_stride_one_fresh` | [x] |
| C11 | `driver` | `stride = -1` — output contains `-` signs (format shape) | `cfg_c11_driver_stride_negative_one` | [x] |
| C12 | `driver` | small random strides `-1000..=1000`, 200 calls, state = whatever accumulated | `cfg_c12_driver_small_random_strides` | [x] |
| C13 | `driver` | small random strides applied on top of a **deliberately pre-set non-zero state** (state × stride interaction) | `cfg_c13_driver_with_preset_state` | [x] |
| C14 | `driver` | large **positive** strides `238609296..=INT_MAX` ⇒ `i*stride` overflows inside the loop, 200 random | `cfg_c14_driver_large_positive_stride_overflow` | [x] |
| C15 | `driver` | large **negative** strides `INT_MIN..=-238609296` ⇒ `i*stride` overflows, 200 random | `cfg_c15_driver_large_negative_stride_overflow` | [x] |
| C16 | `driver` | boundary constants `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `±2^k` | `cfg_c16_driver_boundary_constants` | [x] |
| C17 | `driver` | multiplication boundary: `238609293`, `238609294` (largest safe), `238609295`, `238609296` and negatives | `cfg_c17_driver_multiply_boundary` | [x] |
| C18 | `driver` | called **many times in a row** (100×) with random strides — cross-call state accumulation | `cfg_c18_driver_repeated_calls` | [x] |
| C19 | `static_sum` + `driver` | randomly **interleaved** sequence of 1500 operations over the full value range (shared-state coupling, composed pipeline) | `cfg_c19_interleaved_random_operations` | [x] |
| C20 | `driver` | strides chosen so the printed lines span every digit width 1…10 plus signs | `cfg_c20_driver_all_digit_widths` | [x] |
| C21 | `static_sum` → `driver` | state set via `static_sum(x)` then observed via `driver(0)` — cross-entry-point state consistency for random `x` | `cfg_c21_state_visible_across_entry_points` | [x] |
| C22 | `driver` | stride chosen so the accumulator wraps **mid-loop** (partial wrap between iterations 0..9) | `cfg_c22_driver_wraps_mid_loop` | [x] |
| C23 | both | long mixed fuzz: 5000 randomized operations (choice of entry point, value drawn from a mix of small/large/boundary generators) | `cfg_c23_long_mixed_fuzz` | [x] |
| C24 | `driver` | printed value is exactly `INT_MIN` / `INT_MAX` (11-/10-char `%d` output, byte-exact) | `cfg_c24_driver_prints_int_extremes` | [x] |
| C25 | both | invariant: `driver` always emits exactly 10 `\n`-terminated lines and its net effect on the state is `45*stride` (wrapping) — checked for random strides | `cfg_c25_driver_line_count_and_net_effect` | [x] |
| C26 | both | **genuinely fresh library instance**: a pristine copy of each `.so` is `dlopen`ed from a unique path (so the loader creates a new mapping) — the hidden accumulator must start at exactly `0`, then stay in lockstep for a random sequence; 8 independent rounds | `cfg_c26_fresh_library_instances_start_at_zero` | [x] |

All rows are covered in `tests/differential.rs` (module `configs`) and pass for
both feature combinations and both build profiles.

## How the rows are executed

```
./run_tests.sh          # builds C + Rust .so, runs every row in every config
./check_features.sh     # cargo check for every feature combination
```

`run_tests.sh` runs the suite for the cross-product
`{feature combinations} × {dev, release}` — i.e. against the debug `.so` **and**
the optimised, `panic = "abort"` release `.so` — and finishes with the `nm -D`
symbol diff for both.

Test counts: **43 tests, 43 passing, 0 failing** in each configuration
(26 `configs` rows + 14 `errors` rows + 3 symbol/diagnostic checks).

## Notes on the differential method

* Both libraries are loaded with `libloading` and called **only** through their
  exported C symbols (`dlsym`), so the Rust `#[unsafe(no_mangle)] extern "C"`
  wrappers are themselves under test. No Rust function is ever called directly.
* `static_sum` owns a *hidden* `static int sum` per loaded library, so every
  operation is applied to C first and then to Rust under one process-wide mutex.
  That keeps the two accumulators in lockstep and makes any single-call
  divergence fail immediately.
* `driver`'s `printf` output is compared **byte-for-byte** by redirecting file
  descriptor 1 to a temporary file around each call (fd-level, so the C
  library's own `printf` is captured). `.cargo/config.toml` sets
  `RUST_TEST_THREADS=1` because fd 1 is a process-global resource.
* Randomised rows use a fixed-seed `SplitMix64` generator, so every run is
  reproducible.

## Harness sensitivity (mutation check)

To prove the suite is not vacuous, five deliberate mutations were injected into
`src/lib.rs` (then reverted); each was caught by a large number of rows:

| mutation | tests failed |
|----------|--------------|
| `wrapping_add` → `saturating_add` in `static_sum` | 37 |
| loop bound `i < 10` → `i < 9` in `driver`         | 40 |
| format string `"%d\n"` → `"%d \n"`                | 27 |
| `wrapping_mul` → `saturating_mul` for `i * stride`| 36 |
| off-by-one in the accumulator update              | 40 |

## Extra confirmation

The whole suite was also run against a C library rebuilt with `-O2`
(`cmake -DCMAKE_C_FLAGS=-O2`, built outside `c_src/`) via the
`STATICLOOP_C_SO` override: **43/43 pass**, so the Rust behaviour matches the C
library independently of how aggressively the C is optimised.
