# CONFIGS.md — Phase A configuration-surface table (valid inputs)

The library has **no** runtime flags, no `#ifdef`s, no environment lookups, and
`translation/Cargo.toml` declares **no `[features]`** — so the only
"configuration" axes are the ones the C code branches on structurally.

## Axes derived from the C source

| axis | values the C actually distinguishes | source of the branch |
|------|-------------------------------------|----------------------|
| A. `initial_capacity` shape | `0`, `1`, `< needed`, `== needed`, `> needed` (32 is what `buffapp` uses) | `create_buffer` line 40/48; `append_to_buffer` line 57 |
| B. append growth decision | `required_capacity <= capacity` (no realloc) vs `> capacity` (realloc, `capacity = required*2`) | `append_to_buffer` line 57 |
| C. `str` shape | empty (`""`), 1 byte, fits-exactly, longer than capacity, embedded-NUL prefix, long (≫ capacity), repeated appends accumulating `length` | `strlen`/`strcpy` lines 54, 69 |
| D. buffer state mutated by caller | fresh, after N appends, `length` forced to 0 (what `buffapp` does at line 116), `length` forced non-zero, `capacity` forced smaller than reality | struct is public/opaque-by-convention; `buffapp` itself tampers with `length` |
| E. `op_code` | `0`, `1`, `2`, `3` (the four `case`s) and everything else (`default`) | `get_operation_name` switch, lines 85-90 |
| F. `operation` string | `"add"`, `"subtract"`, `"multiply"`, `"divide"`, non-matching | `perform_operation` strcmp chain, lines 95-107 |
| G. divisor | `b == 0` vs `b != 0` | `perform_operation` line 102 |
| H. operand magnitude | small, negative, `INT_MAX`, `INT_MIN`, overflow-inducing pairs | wrapping arithmetic lines 96-103 |
| I. `param1 % 4` / `param3 % 4` residue | `0,1,2,3` (positive params) and `0,-1,-2,-3` (negative params → `default` → `"unknown"` → `0`) — 7 distinct residues each | `buffapp` lines 119, 127 |
| J. `intermediate3` | `!= 0` (divide path) vs `== 0` (sum path) | `buffapp` line 141 |
| K. observable channel | return value; `StringBuffer` fields (`data` bytes, `capacity`, `length`); stdout bytes from `printf` | `buffapp` line 150 |

## Rows — pruned cross-product of the combinations the C treats differently

Every row is driven through the `.so` exports of **both** libraries and compared
byte-for-byte, with many randomized inputs per row (fixed seed, see
`translation/tests/differential.rs` and `translation/tests/stdout_diff.rs`).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `create_buffer` + `destroy_buffer` | `initial_capacity == 1` (minimum in-bounds); compare `capacity`, `length`, `data[0]` | [x] |
| 2  | `create_buffer` + `destroy_buffer` | `initial_capacity == 0` (the OOB `data[0]` write, axis A boundary) | [x] |
| 3  | `create_buffer` + `destroy_buffer` | randomized `initial_capacity` in `1..=4096` | [x] |
| 4  | `create_buffer` + `destroy_buffer` | large valid `initial_capacity` (`1<<20`, `1<<24`) | [x] |
| 5  | `create_buffer` + `append_to_buffer` | fresh buffer, `str == ""` → `required = 1`; grows only when `capacity == 0` (axes B×C boundary) | [x] |
| 6  | `create_buffer` + `append_to_buffer` | fresh buffer, `strlen(str) + 1 < capacity` → **no** realloc; assert `capacity` unchanged | [x] |
| 7  | `create_buffer` + `append_to_buffer` | fresh buffer, `strlen(str) + 1 == capacity` → exact fit, **no** realloc (off-by-one boundary of line 57) | [x] |
| 8  | `create_buffer` + `append_to_buffer` | fresh buffer, `strlen(str) + 1 == capacity + 1` → realloc by exactly one byte over | [x] |
| 9  | `create_buffer` + `append_to_buffer` | fresh buffer, `str` much longer than capacity → realloc, `capacity == (len+1)*2` | [x] |
| 10 | `create_buffer` + `append_to_buffer` ×N | randomized sequence of 1..32 randomized appends on one buffer; assert `data`, `length`, `capacity` after **every** step (axis D: accumulating state, mix of growth/no-growth) | [x] |
| 11 | `create_buffer` + `append_to_buffer` | caller forces `length = 0` mid-stream, then appends (exactly what `buffapp` does) — overwrite from the start, `capacity` retained | [x] |
| 12 | `create_buffer` + `append_to_buffer` | caller forces `length` to a valid interior offset → append writes into the middle | [x] |
| 13 | `create_buffer` + `append_to_buffer` | caller forces `capacity` **smaller** than the real allocation → forces a realloc that shrinks/grows to `required*2` | [x] |
| 14 | `append_to_buffer` | `str` with bytes ≥ 0x80 and a 0x01..0x1f control-byte payload (strlen/strcpy are byte-exact, not UTF-8) | [x] |
| 15 | `get_operation_name` | each of `op_code == 0,1,2,3` → `"add"`,`"subtract"`,`"multiply"`,`"divide"`; compare returned C strings byte-for-byte | [x] |
| 16 | `get_operation_name` | randomized `op_code` over the full `i32` range incl. `INT_MIN`/`INT_MAX` (axis E `default`) | [x] |
| 17 | `perform_operation` | `operation == "add"`, randomized `a,b` full `i32` range (includes wrapping overflow, axis H) | [x] |
| 18 | `perform_operation` | `operation == "subtract"`, randomized `a,b` full range | [x] |
| 19 | `perform_operation` | `operation == "multiply"`, randomized `a,b` full range | [x] |
| 20 | `perform_operation` | `operation == "divide"`, randomized `a,b` with `b != 0` (axis G true) | [x] |
| 21 | `perform_operation` | `operation == "divide"`, `b == 0` (axis G false) | [x] |
| 22 | `perform_operation` | `operation` = pointer returned by `get_operation_name` (composition: F fed by E, incl. `"unknown"`) × randomized `a,b` | [x] |
| 23 | `perform_operation` | `operation` = randomized non-matching byte strings (axis F `default`) | [x] |
| 24 | `perform_operation` | small-magnitude exhaustive grid `a,b ∈ -8..=8` × all five operation names (all of F×G×H interactions at boundaries) | [x] |
| 25 | `buffapp` | full pipeline, all 7×7 = 49 residue combinations of `param1 % 4` × `param3 % 4` (axis I), randomized magnitudes per cell; compare return value **and** stdout bytes | [x] |
| 26 | `buffapp` | params forcing `intermediate3 == 0` (axis J false → sum path) | [x] |
| 27 | `buffapp` | params forcing `intermediate3 != 0` (axis J true → divide path) | [x] |
| 28 | `buffapp` | randomized full-`i32` params (axis H inside the pipeline: wrapping ops, long decimal renderings that stress the 64-byte `temp` and the 32-byte initial buffer) | [x] |
| 29 | `buffapp` | all params equal / all zero / all `INT_MIN` / all `INT_MAX` corner tuples | [x] |
| 30 | `buffapp` | stdout byte-for-byte comparison of the whole `"Computation Log:\n…"` block (axis K) for every tuple used in rows 25-29 | [x] |
| 31 | end-to-end composition | build a buffer with `create_buffer`, append the same log lines `buffapp` would, `destroy_buffer` — i.e. drive the low-level API to reproduce `buffapp`'s internal pipeline and compare against the C's low-level API step by step | [x] |
| 32 | cross-library ABI | `StringBuffer` created by the **C** `.so` passed to the **Rust** `append_to_buffer`/`destroy_buffer` and vice versa (validates `#[repr(C)]` layout and shared allocator) | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table and no optional dependencies,
so the only build configuration is the default one. `--no-default-features` is
therefore identical to the default. This is verified mechanically by
`translation/tests/feature_combos.sh`.

## Phase B status — row → test mapping

Rows 1-24 live in `translation/tests/differential.rs` (libtest harness);
rows 25-32 plus three extra property checks live in
`translation/tests/stdout_diff.rs`, a `harness = false` binary that runs
strictly sequentially on one thread. Every test drives both `.so` files through
`dlsym` only.

The split is required, not cosmetic: `buffapp` logs with `printf`, so comparing
its output means redirecting fd 1, which is process-global. With libtest's
default thread pool, libtest's own progress lines were landing inside the
capture window and producing intermittent false divergences. Owning `main()`
eliminates the race.

| row | test fn | randomized inputs |
|-----|---------|-------------------|
| 1  | `row01_create_buffer_capacity_one` | fixed boundary |
| 2  | `row02_create_buffer_capacity_zero` | fixed boundary |
| 3  | `row03_create_buffer_randomized_small_capacities` | 2000 |
| 4  | `row04_create_buffer_large_valid_capacities` | 4 sizes |
| 5  | `row05_append_empty_string` | 5 capacities |
| 6  | `row06_append_fits_without_growth` | 1500 |
| 7  | `row07_append_exact_fit_no_growth` | 800 |
| 8  | `row08_append_one_byte_over` | 800 |
| 9  | `row09_append_much_longer_than_capacity` | 600 |
| 10 | `row10_append_randomized_sequences_state_checked_each_step` | 300 buffers × 1-32 appends, state compared after every append |
| 11 | `row11_append_after_length_forced_to_zero` | 300 |
| 12 | `row12_append_after_length_forced_to_interior_offset` | 300 |
| 13 | `row13_append_with_capacity_field_understated` | 400 |
| 14 | `row14_append_high_and_control_bytes` | 600 |
| 15 | `row15_get_operation_name_valid_codes` | 4 codes, bytes compared |
| 16 | `row16_get_operation_name_randomized_full_i32` | 5013 |
| 17 | `row17_perform_operation_add` | 5000 |
| 18 | `row18_perform_operation_subtract` | 5000 |
| 19 | `row19_perform_operation_multiply` | 5000 |
| 20 | `row20_perform_operation_divide_nonzero_divisor` | 5000 |
| 21 | `row21_perform_operation_divide_by_zero` | 2000 |
| 22 | `row22_perform_operation_with_names_from_get_operation_name` | 6000, each also cross-fed the peer library's string pointer |
| 23 | `row23_perform_operation_non_matching_operation_strings` | 15 near-misses + 3000 random |
| 24 | `row24_perform_operation_small_exhaustive_grid` | 5 ops × 17 × 17 exhaustive + overflow corners |
| 25 | `row25_buffapp_all_residue_combinations` | 49 residue cells × 20, return **and** stdout compared |
| 26 | `row26_buffapp_intermediate3_zero_sum_path` | 355 |
| 27 | `row27_buffapp_intermediate3_nonzero_divide_path` | 335 |
| 28 | `row28_buffapp_randomized_full_range` | 1500 |
| 28b| `row28b_buffapp_randomized_full_range_stdout` | 400, stdout compared |
| 29 | `row29_buffapp_corner_tuples` | 9^4 = 6561 exhaustive corner tuples |
| 30 | `row30_buffapp_stdout_exact_text` | pins the exact log text |
| 31 | `row31_low_level_pipeline_composition` | 200, snapshot compared after *every* low-level step and reconciled against the one-shot wrapper |
| 32 | `row32_cross_library_abi_interchange` | 200, C-created buffers driven by Rust and vice versa |

Randomness comes from a splitmix64 PRNG seeded per test with a hard-coded
constant, so every failure is reproducible.

Divergences found in Phase B: none. Three test-side bugs were found and fixed
(an over-strong `capacity` assertion in row 13, and two byte strings —
`b"add\0extra\0"`, `b"subtract\0x\0"` — that are `strcmp`-equal to a valid
operation name and so were wrongly listed as unmatched); in each case the
C↔Rust comparison itself had already passed.
