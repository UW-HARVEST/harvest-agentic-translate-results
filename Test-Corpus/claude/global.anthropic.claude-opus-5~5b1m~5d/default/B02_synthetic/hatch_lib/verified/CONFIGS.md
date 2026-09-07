# CONFIGS.md — Phase B configuration surface table

## Axes the C actually branches on

Derived from `c_src/include/lib.h` (public: `hatch` only) plus every exported
`T` symbol in the `.so` (all 12 are callable by an external consumer, so all 12
are entry points — the low-level ones are tested directly, not only via `hatch`).

* **Runtime options / modes:** none (no flags, no `#ifdef`, no setters). The only
  *mutable configuration* is the pair of file-scope statics `global_counter` and
  `global_accumulator`, which persist across calls and are mutated by
  `increment_counter` / `update_accumulator` (and read by `complex_calc`,
  `process_pointer_data`, `hatch`). **Call-order / accumulated-state is therefore
  a first-class configuration axis.**
* **Input shapes:**
  * `shift_by` / `shift` vs. `size` / `num_records`: `<0`, `0`, `1`, `mid`,
    `size-1`, `size`, `>size` (the two guard branches).
  * array/record counts: `0`, `1`, `many`.
  * value magnitude: small, `INT_MAX`/`INT_MIN` neighbourhood (wrap-around).
  * `DataRecord` layout: 48 bytes / align 8, `name[32]` written by `snprintf`.
  * `count` in `compute_with_dynamic_memory`: `0`, `1`, `8` (the value `hatch`
    uses), large.
* **Cargo features:** `translation/Cargo.toml` declares **no `[features]`**, so
  the only feature combination is the default (= no features). Verified by
  looping `cargo check`/`cargo test` over `--no-default-features` too.
* **Binary executable:** the CMake project builds only `SHARED` (no `add_executable`)
  and the crate has no `[[bin]]` / `src/main.rs` ⇒ **no driver binary to diff**.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `add_three` | randomized full-range `int` triples (incl. `INT_MIN`/`INT_MAX`) | [x] |
| 2  | `multiply_add` | randomized full-range `int` triples (multiplication overflow) | [x] |
| 3  | `complex_calc` | randomized triples, `global_counter == 0` (fresh state) | [x] |
| 4  | `complex_calc` | randomized triples **after** randomized `increment_counter` calls (non-zero, wrapped `global_counter`) | [x] |
| 5  | `increment_counter` | randomized value sequences; observe accumulated effect via `complex_calc` | [x] |
| 6  | `update_accumulator` | randomized value sequences (`acc = acc*2 + v`, wraps); observe via `process_pointer_data` | [x] |
| 7  | `apply_operation` | `op = add_three` (pointer taken from the *same* `.so`), randomized triples | [x] |
| 8  | `apply_operation` | `op = multiply_add`, randomized triples | [x] |
| 9  | `apply_operation` | `op = complex_calc` with non-zero `global_counter` (state-dependent callee) | [x] |
| 10 | `apply_operation` | `op = increment_counter` reinterpreted as `operation_func` (ABI-mismatched but valid C call: returns garbage-but-defined `eax`, mutates the global) — checks the transmute path | [x] |
| 11 | `process_pointer_data` | randomized `*ptr` × `multiplier`, `global_accumulator == 0` | [x] |
| 12 | `process_pointer_data` | randomized, `global_accumulator` non-zero and wrapped | [x] |
| 13 | `shift_array_data` | `size` many (16..64), `0 < shift_by < size` randomized ⇒ guard taken; compare whole buffer bytes | [x] |
| 14 | `shift_array_data` | `size` many, `shift_by == 1` (min accepted) | [x] |
| 15 | `shift_array_data` | `size` many, `shift_by == size-1` (max accepted) | [x] |
| 16 | `shift_array_data` | `size == 1`, `shift_by == 0` ⇒ guard not taken | [x] |
| 17 | `shift_array_data` | `size == 2`, `shift_by == 1` (smallest guard-taken shape) | [x] |
| 18 | `shift_array_data` | guard-not-taken shapes: `shift_by <= 0`, `shift_by >= size`, `size <= 0` | [x] |
| 19 | `compute_with_dynamic_memory` | `count == 8` (the shape `hatch` uses), randomized `base` | [x] |
| 20 | `compute_with_dynamic_memory` | `count == 1`, randomized `base` | [x] |
| 21 | `compute_with_dynamic_memory` | `count` randomized 2..256, randomized `base` incl. overflow-inducing | [x] |
| 22 | `compute_with_dynamic_memory` | `count == 0` (malloc(0) path) | [x] |
| 23 | `get_time_based_value` | randomized small `seed` (no `seed*3600` overflow) | [x] |
| 24 | `get_time_based_value` | `seed` in the overflow region (`|seed| > 596523`), incl. `INT_MIN`/`INT_MAX` | [x] |
| 25 | `get_time_based_value` | `seed == 0` and `seed < 0` (negative `difftime`, truncation toward zero) | [x] |
| 26 | `manipulate_records` | `num_records` 1..8, `0 < shift < num_records` randomized ⇒ memmove taken; compare return **and** the full 48-byte-per-record buffer | [x] |
| 27 | `manipulate_records` | `shift == 1`, `num_records` many (min accepted shift) | [x] |
| 28 | `manipulate_records` | `shift == num_records - 1` (max accepted shift) | [x] |
| 29 | `manipulate_records` | `shift == 0`, `num_records` many ⇒ plain sum, no memmove | [x] |
| 30 | `manipulate_records` | `num_records == 5`, `shift == 2` (the exact shape `hatch` uses), randomized `value`s | [x] |
| 31 | `manipulate_records` | records whose `timestamp`/`name`/`id` fields are randomized garbage (checks 48-byte memmove moves *all* fields, not just `value`) | [x] |
| 32 | `hatch` | single call from fresh library state, randomized `param1..param4` (fresh `.so` handle per case so both libs start with zeroed globals) | [x] |
| 33 | `hatch` | **repeated** calls on one handle: identical randomized sequence driven into C and Rust, comparing every intermediate return (exercises persistent `global_counter`/`global_accumulator`) | [x] |
| 34 | `hatch` | all-zero params; and params at `INT_MIN`/`INT_MAX`/`±1` boundary values | [x] |
| 35 | `hatch` | long sequence (200+ calls) with large params so both globals wrap many times | [x] |
| 36 | mixed pipeline | interleave `increment_counter`, `update_accumulator`, `hatch`, `complex_calc`, `process_pointer_data` in one randomized script against both handles (composed-pipeline state coupling) | [x] |
| 37 | `DataRecord` ABI | assert `sizeof == 48`, `align == 8`, field offsets 0/4/8/16 agree between C and Rust by round-tripping a record through `manipulate_records` | [x] |
| 38 | feature combos | default features; `--no-default-features` (crate declares no features ⇒ both are the same build, both run the full suite) | [x] |
