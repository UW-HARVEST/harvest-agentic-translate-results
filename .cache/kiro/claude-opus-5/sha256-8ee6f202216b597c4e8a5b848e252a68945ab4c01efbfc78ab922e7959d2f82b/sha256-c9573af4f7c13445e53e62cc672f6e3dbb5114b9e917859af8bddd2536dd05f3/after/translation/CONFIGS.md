# CONFIGS.md — configuration / valid-input surface table (Phase A, gates Phase B)

## Axes, derived from the C source

**Compile-time configuration axes: none.** `grep '#if\|#ifdef\|#ifndef' c_src/src/lib.c`
→ no matches, and `translation/Cargo.toml` has no `[features]` section, so the
default (feature-less) build is the *only* configuration. `cargo check
--no-default-features` and the default build compile the same code (verified by
`scripts/check_feature_combos.sh`).

**Runtime option/mode axes.** The library exposes no init/config struct. Its
"mode" is carried by two pieces of hidden state plus a function-pointer
parameter:

* A1 — `global_counter` (`static int`, lib.c:30). Set only via
  `increment_counter`. Read by `complex_calc` (lib.c:56) and `hatch` (lib.c:174).
  States: `0` (pristine), positive, negative, wrapped past `INT_MAX`.
* A2 — `global_accumulator` (`static int`, lib.c:31). Set only via
  `update_accumulator`, which *doubles* it each call. Read by
  `process_pointer_data` (lib.c:75) and `hatch` (lib.c:174). States: `0`,
  positive, negative, wrapped.
* A3 — the `operation_func` passed to `apply_operation` (lib.c:43). Selectable
  values in the library: `add_three`, `multiply_add`, `complex_calc`. Because
  it is a plain C function pointer, a *caller-supplied* function and a
  *cross-library* function are also legal inputs.

**Input-shape axes.**

* A4 — `shift_array_data(arr, size, shift_by)` branch at lib.c:67: the sign of
  `shift_by` and its position relative to `size` (`<0`, `0`, `1`, interior,
  `size-1`, `size`, `>size`), and `size` itself (`0`, `1`, small, large).
* A5 — `manipulate_records(records, num_records, shift)` branch at lib.c:111 and
  the *independent* loop bound at lib.c:116 (`num_records - shift`), which is
  reached whether or not the guard fired. Shapes: `shift<0`, `0`, `1`,
  interior, `num_records-1`, `num_records`, `>num_records`; `num_records` `0`,
  `1`, many.
* A6 — `compute_with_dynamic_memory(base, count)` loop counts at lib.c:81/86:
  `count` `<0`, `0`, `1`, small, large; `base` sign and overflow proximity.
* A7 — `get_time_based_value(seed)`: `seed` `0`, small ±, magnitude where
  `seed*3600` stays in `int`, magnitude where it overflows, `INT_MIN`/`INT_MAX`.
* A8 — the pure `int` triples for `add_three` / `multiply_add` / `complex_calc`
  and the pair for `process_pointer_data`: full `i32` domain incl. `INT_MIN`,
  `INT_MAX`, `0`, `-1`.
* A9 — `hatch(param1..param4)`: full `i32` domain, *and* the number of prior
  calls (its result depends on A1/A2, which it itself mutates), *and* whether
  A1/A2 were pre-mutated by direct `increment_counter`/`update_accumulator`
  calls.

**Entry points.** All 12 exported symbols are in scope, including the lowest
level ones. `hatch` is the only entry point in the public header, i.e. the
"convenience one-shot wrapper"; the other 11 are the low-level API and are
driven **directly**, not only through `hatch`.

## Configuration table

One row per combination the C actually distinguishes. Every row is exercised
with many randomized inputs (SplitMix64, fixed seed `0x5EED_1234_ABCD_0001`)
against both `.so`s, plus the named boundary values.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `add_three` | full `i32` domain, randomized triples | [x] |
| 2 | `add_three` | boundary triples: `0`/`-1`/`INT_MIN`/`INT_MAX` cross-product (overflow wrap) | [x] |
| 3 | `multiply_add` | full `i32` domain, randomized triples | [x] |
| 4 | `multiply_add` | boundary triples incl. `INT_MIN * -1`, `INT_MAX * INT_MAX` | [x] |
| 5 | `increment_counter` | A1 pristine → positive `value`, repeated calls (accumulation) | [x] |
| 6 | `increment_counter` | A1 negative `value`s, and `value`s driving A1 past `INT_MAX` (wrap) | [x] |
| 7 | `increment_counter` | `unused_param` varied (must be ignored — incl. `INT_MIN`/`INT_MAX`) | [x] |
| 8 | `update_accumulator` | A2 pristine → single call, then 40 successive calls (doubling → guaranteed wrap) | [x] |
| 9 | `update_accumulator` | `unused_param` varied (must be ignored) | [x] |
| 10 | `complex_calc` | A1 = 0 (pristine), randomized `(a,b,c)` | [x] |
| 11 | `complex_calc` | A1 positive, randomized `(a,b,c)` — global read-back | [x] |
| 12 | `complex_calc` | A1 negative / wrapped, randomized `(a,b,c)` | [x] |
| 13 | `complex_calc` | A1 arbitrary + boundary `(a,b,c)` (`INT_MIN`/`INT_MAX`) | [x] |
| 14 | `process_pointer_data` | A2 = 0, randomized `*ptr` and `multiplier` | [x] |
| 15 | `process_pointer_data` | A2 non-zero / wrapped, randomized `*ptr`, `multiplier` | [x] |
| 16 | `process_pointer_data` | boundary `*ptr`/`multiplier` (`INT_MIN`, `INT_MAX`, `0`, `-1`) | [x] |
| 17 | `apply_operation` | A3 = `add_three` from the *same* library, randomized args | [x] |
| 18 | `apply_operation` | A3 = `multiply_add` from the same library, randomized args | [x] |
| 19 | `apply_operation` | A3 = `complex_calc` from the same library (also reads A1), randomized args, A1 varied | [x] |
| 20 | `apply_operation` | A3 = **cross-library** pointer (C's `add_three` into Rust's `apply_operation` and vice versa) | [x] |
| 21 | `apply_operation` | A3 = a **caller-supplied** `extern "C"` fn defined in the test binary (proves it is a real indirect call, not inlined) | [x] |
| 22 | `shift_array_data` | interior shift `0 < shift_by < size`, randomized `size` (2..64), `shift_by`, contents | [x] |
| 23 | `shift_array_data` | `shift_by == 1` (minimum moving shift), randomized size/contents | [x] |
| 24 | `shift_array_data` | `shift_by == size - 1` (maximum moving shift: 1 element moved, `size-1` zeroed) | [x] |
| 25 | `shift_array_data` | `size == 1` (no `shift_by` can satisfy the guard) | [x] |
| 26 | `shift_array_data` | `size == 2`, `shift_by == 1` (smallest non-degenerate move) | [x] |
| 27 | `shift_array_data` | large `size` (4096) with random interior `shift_by` (bulk `memmove`/`memset`) | [x] |
| 28 | `shift_array_data` | `size` smaller than the real buffer, non-zero trailing slack — asserts bytes past `size` are untouched by both | [x] |
| 29 | `manipulate_records` | interior `0 < shift < num_records`, randomized `num_records` (2..32), `shift`, record contents | [x] |
| 30 | `manipulate_records` | `shift == 1`, randomized `num_records`/contents (overlapping `memmove` of `num_records-1` records) | [x] |
| 31 | `manipulate_records` | `shift == num_records - 1` (single record moved, sum of 1 element) | [x] |
| 32 | `manipulate_records` | `shift == 0` (guard skipped, sums all `num_records`), randomized contents | [x] |
| 33 | `manipulate_records` | `num_records == 1`, `shift == 0` | [x] |
| 34 | `manipulate_records` | large `num_records` (512) with random interior `shift` | [x] |
| 35 | `manipulate_records` | full struct payload varied — `id`, `timestamp`, and `name[32]` bytes randomized, asserting `memmove` relocates all 48 bytes identically (not just `.value`) | [x] |
| 36 | `compute_with_dynamic_memory` | `count > 0` randomized (1..4096), randomized `base` | [x] |
| 37 | `compute_with_dynamic_memory` | `count == 1` (single element) | [x] |
| 38 | `compute_with_dynamic_memory` | `count` large (65536) — bulk allocation path | [x] |
| 39 | `compute_with_dynamic_memory` | `base` at `INT_MIN`/`INT_MAX` with `count` in 1..64 (per-element and sum wrap) | [x] |
| 40 | `get_time_based_value` | `seed == 0` | [x] |
| 41 | `get_time_based_value` | `seed > 0`, no `seed*3600` overflow (`1..596523`), randomized | [x] |
| 42 | `get_time_based_value` | `seed < 0`, no overflow (negative truncation direction), randomized | [x] |
| 43 | `get_time_based_value` | `seed` large enough that `seed*3600` overflows `int`, randomized full `i32` | [x] |
| 44 | `get_time_based_value` | `seed` = `INT_MIN`, `INT_MAX`, `±596523`, `±596524` (exact overflow threshold) | [x] |
| 45 | `hatch` | A1/A2 pristine, randomized `(p1..p4)` full `i32` — lockstepped so both libraries see the same call count | [x] |
| 46 | `hatch` | repeated back-to-back calls (A1/A2 accumulate; `global_accumulator` doubles each call → wraps) — 64 successive calls | [x] |
| 47 | `hatch` | boundary params: `0`, `1`, `-1`, `INT_MIN`, `INT_MAX` combinations | [x] |
| 48 | `hatch` | A1/A2 pre-mutated by direct `increment_counter`/`update_accumulator` calls before `hatch` (option-state × entry-point interaction) | [x] |
| 49 | composed pipeline | mutate A1/A2 → `apply_operation(complex_calc)` → `process_pointer_data` → `shift_array_data` → `manipulate_records` → `hatch`, all on shared buffers, randomized, asserting every intermediate result **and** final buffer bytes (bugs in the composed pipeline are invisible to per-function tests) | [x] |
| 50 | composed pipeline vs `hatch` | the exact call sequence `hatch` performs, driven manually through the 11 low-level exports, compared against the **other** library's real `hatch` (C pipeline == Rust `hatch`, and Rust pipeline == C `hatch`) — stronger than `hatch`-vs-`hatch`, which a consistently-wrong translation would pass | [x] |

Rows 1–48 live in `translation/tests/differential.rs` as `row01_…`–`row48_…`
(one `#[test]` per row). Row 49 is `row49_composed_pipeline_low_level` in the
same file; row 50 is `translation/tests/pipeline_vs_hatch.rs`, which gives each
of the four roles a private on-disk copy of the `.so` so it gets independent
`global_counter` / `global_accumulator` state.

`translation/tests/fuzz_walk.rs` adds a 1,500,000-step random walk that
interleaves all twelve exports in random order from the same fixed seed,
re-probing both hidden globals every 64 steps. That covers combinations across
rows that the per-row tests visit only in isolation.

## Feature combinations

`translation/Cargo.toml` declares no features, so the set of combinations is
`{default}` ≡ `{--no-default-features}`.
`translation/scripts/verify_all.sh` enumerates them mechanically from
`Cargo.toml` (it computes the power set, so it keeps working if features are
added later) and runs `cargo check` plus the full suite for each. It also widens
the matrix along two axes that genuinely change generated code:

| Rust cdylib profile | C build | result |
|---|---|---|
| `release` (opt, no UB checks) | default (`-O0`, the documented build) | PASS |
| `release` | `-DCMAKE_BUILD_TYPE=Release` (`-O3`) | PASS |
| `debug` (`overflow-checks = true`, UB checks on) | default (`-O0`) | PASS |
| `debug` | `-O3` | PASS |

The `debug` rows matter: with `overflow-checks = true`, any arithmetic in the
translation that is *not* explicitly wrapping aborts instead of silently
matching, so those runs prove every `int` operation was translated as a wrapping
operation. The `-O3` C rows confirm the C's signed-overflow-heavy paths land on
the same values under optimization. The optimized C build is written to
`translation/target/c_build_release/`, so nothing inside `c_src/` is modified.
