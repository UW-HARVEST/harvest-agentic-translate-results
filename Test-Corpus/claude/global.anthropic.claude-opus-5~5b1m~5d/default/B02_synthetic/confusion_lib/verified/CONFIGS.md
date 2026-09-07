# CONFIGS.md — Configuration surface table (VALID inputs)

Axes derived mechanically from `c_src/src/lib.c` — every branch the C code
actually takes on a valid input.

## Axes

**A. Entry points (all 6 exports, lowest-level first)**
`create_state` → `update_flags` → `process_buffer` → `confuse_types` →
`destroy_state`, plus the one-shot wrapper `confusion` (the only symbol in
`include/lib.h`).

**B. `create_state` capacity shape** (branches at lines 76-86)
`capacity == 0` (malloc(0) ok, snprintf writes nothing) ·
`0 < capacity < 16` (snprintf **truncates**) ·
`capacity == 16..17` (exact fit boundary for `"State:%d:Mode:3"`) ·
`capacity == 128` (what `confusion` uses) · large capacity.

**C. `create_state` initial_val shape** — decides the union bits *and* the
decimal width of `"State:%d"`: `0` · small positive · negative (adds `'-'`) ·
`INT_MAX` · `INT_MIN` (11 chars) · values whose bits are a float NaN / Inf /
denormal / integral / huge · `1078530011` (the constant case 0 writes).

**D. `update_flags` param bit shape** (lines 131-135): the flag bits
`param & 7` (8 combinations) × `mode = (param >> 3) & 7` (8 combinations),
plus negative `param` (arithmetic `>>`).

**E. `update_flags` call count** — `counter` is a 5-bit saturating-free
accumulator: 0, 1, 2, 31, 32 (wrap to 0), 33 calls.

**F. `process_buffer` target shape** (lines 109-121): digit present once ·
present many times · present at buffer start · at buffer end ·
absent · `'\0'` · negative `char` (0x80..0xFF) · every byte 0..255.

**G. `process_buffer` buffer shape**: buffer produced by `create_state` ·
empty string · all-same-byte string · random bytes · long (>128) string ·
string whose only match is the last byte.

**H. `confuse_types` operation** (`switch`, lines 150-172): `0` (write int) ·
`1` (read float, `*100`, C cast) · `2` (read uint, `& 0xFF`) ·
`3` (read 4 signed bytes, `[0]+[1]`).

**I. `confusion` derived selectors**: `search_char = '0' + param3 % 10`
(10 non-negative residues + 10 negative residues) ×
`param4 % 4` ∈ {0,1,2,3,-1,-2,-3}.

**J. Cargo feature combinations**: `Cargo.toml` has **no `[features]`
section**, so the only build configuration is the default (no features). Every
row is therefore also the row for "every feature combination"; the test script
re-runs the suite under `--no-default-features` to prove it.

## Rows

Every row is exercised with many randomized inputs (fixed seed, deterministic
xorshift PRNG) and compared C-vs-Rust for **return value AND captured stdout,
byte-for-byte**.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `create_state` + `destroy_state` | `capacity = 128`, `initial_val` random over full `i32` range (1000 cases): compare returned `state` field-by-field (`flags` word, `data` word, `capacity`) and the `buffer` string bytes + stdout | [x] |
| 2  | `create_state` + `destroy_state` | `capacity = 128`, `initial_val` ∈ {0, 1, -1, 10, -10, `INT_MAX`, `INT_MIN`, 1078530011, 0x7F800000, 0x7FC00000, 0xFF800000, 0x00000001, 0x3F800000} (bit-pattern corner cases) | [x] |
| 3  | `create_state` | `capacity` truncating: 1..16 × `initial_val` ∈ {0, 12345, -12345, `INT_MIN`} — buffer must be truncated identically | [x] |
| 4  | `create_state` | `capacity == 0` — non-NULL state, `buffer` valid 0-byte block, `capacity` field 0 (buffer contents uninitialised, so only nullness/fields compared) | [x] |
| 5  | `create_state` | `capacity` large (256, 1024, 65536) × random `initial_val` | [x] |
| 6  | `update_flags` | fresh state (`counter=0, mode=3`), `param` = all 64 values `0..63` (full cross of `param&7` flags × `(param>>3)&7` mode) — compare flags word + stdout | [x] |
| 7  | `update_flags` | fresh state, `param` random over full `i32` (incl. negatives → arithmetic `>>`), 1000 cases | [x] |
| 8  | `update_flags` | repeated calls on the same state: 1, 2, 31, 32, 33, 64 calls (5-bit counter wrap) × random `param` sequence | [x] |
| 9  | `update_flags` | `param` ∈ {`INT_MIN`, `INT_MAX`, -1, -8, 7, 8, 56, 63, 64} (mode/flag field boundaries) | [x] |
| 10 | `process_buffer` | buffer from `create_state(v,128)`, `target` = every byte `0..=255`, for several `initial_val` | [x] |
| 11 | `process_buffer` | **synthetic `ProcessState`** (built in the test to the C layout) with buffer = random ASCII strings of length 0..64, `target` random, 2000 cases | [x] |
| 12 | `process_buffer` | synthetic state, buffer = random bytes incl. high bytes 0x80..0xFF, `target` random full-byte range (signed-`char`/`unsigned char` promotion path), 2000 cases | [x] |
| 13 | `process_buffer` | synthetic state, buffer shapes: empty · single char · all-same byte (len 1,2,31,64) · match only at first byte · match only at last byte · long 300-byte buffer | [x] |
| 14 | `confuse_types` | `operation = 0` (writes `1078530011`) on states with random `initial_val`; also verify the union word afterwards | [x] |
| 15 | `confuse_types` | `operation = 1` (float read) — `initial_val` random over full `i32`, 3000 cases, so NaN/Inf/denormal/huge/`cvttss2si`-indefinite all get hit | [x] |
| 16 | `confuse_types` | `operation = 1` with hand-picked float bit patterns: ±0, ±denormal min, ±1.0, ±FLT_MAX, ±Inf, quiet/signalling NaN, values whose `*100` lands just inside/outside `int` range (`0x4EFFFFFF`, `0x4F000000`, `0xCF000000`, …) | [x] |
| 17 | `confuse_types` | `operation = 2` (uint read, `& 0xFF`) — random full-`i32` `initial_val`, 1000 cases | [x] |
| 18 | `confuse_types` | `operation = 3` (4 signed bytes, `bytes[0]+bytes[1]`) — random full-`i32` `initial_val` incl. patterns making both bytes negative, 1000 cases | [x] |
| 19 | `confuse_types` | each `operation` 0..3 called **twice in sequence** on the same state (op 0 mutates the union, so ordered pairs 0→1, 0→2, 0→3, 1→0, 3→0, … all 16 ordered pairs) | [x] |
| 20 | `destroy_state` | destroy a state created by the *other* library's `create_state` (cross-allocator/layout check: C-created state freed by Rust `destroy_state` and vice versa) | [x] |
| 21 | full pipeline (low-level, hand-composed) | `create_state` → N× `update_flags` → `process_buffer` → `confuse_types` → `destroy_state`, all four params randomized, 2000 cases — mirrors what `confusion` does but with the low-level exports driven directly | [x] |
| 22 | `confusion` | full cross `param3 % 10` ∈ 0..9 × `param4 % 4` ∈ 0..3 with random `param1`, `param2` (40 combinations × many seeds) | [x] |
| 23 | `confusion` | negative selectors: `param3 < 0` (negative residues → non-digit search char) × `param4 < 0` (negative residues → no `switch` case) | [x] |
| 24 | `confusion` | all four params random over full `i32`, 5000 cases (return value + stdout byte-for-byte) | [x] |
| 25 | `confusion` | params at extremes: every element of {0, 1, -1, `INT_MAX`, `INT_MIN`, 1078530011} in each of the 4 positions (cartesian sample) | [x] |
| 26 | binary executable | `CMakeLists.txt` builds **only** `add_library(... SHARED)` and `Cargo.toml` declares only `crate-type = ["cdylib"]` — **no driver binary exists**, so there is no stdout-of-binary comparison to make. stdout is instead compared for every library call above via `dup2` capture. | [x] (N/A) |
| 27 | feature combos | no `[features]` in `Cargo.toml`; suite re-run with `--no-default-features` and with `--all-features` (both ≡ default) | [x] |

## Row → test mapping (all rows PASS)

All in `tests/phase_b_valid.rs` unless noted; run with `--test-threads=1`.

| row(s) | test |
|--------|------|
| 1 | `row01_create_state_cap128_random_initial` (1000 random `i32`) |
| 2 | `row02_create_state_cap128_interesting_initial` |
| 3 | `row03_create_state_truncating_capacity` |
| 4 | `row04_create_state_capacity_zero` |
| 5 | `row05_create_state_large_capacity` |
| 6 | `row06_update_flags_full_low_6_bits` |
| 7 | `row07_update_flags_random_i32` (1000 random) |
| 8 | `row08_update_flags_counter_wrap` |
| 9 | `row09_update_flags_boundary_params` |
| 10 | `row10_process_buffer_created_state_all_bytes` (7 states × 256 targets) |
| 11 | `row11_process_buffer_random_ascii` (2000 random) |
| 12 | `row12_process_buffer_random_high_bytes` (2000 random) |
| 13 | `row13_process_buffer_buffer_shapes` |
| 14 | `row14_confuse_types_op0` |
| 15 | `row15_confuse_types_op1_random` (3000 random) |
| 16 | `row16_confuse_types_op1_float_bit_patterns` (full 2048-point exponent/mantissa/sign sweep) |
| 17 | `row17_confuse_types_op2_random` |
| 18 | `row18_confuse_types_op3_random` |
| 19 | `row19_confuse_types_ordered_pairs` (all 16 ordered pairs + random sequences) |
| 20 | `row20_cross_library_destroy` (C-allocated state freed by Rust and vice versa) |
| 21 | `row21_full_pipeline_low_level` (2000 random full pipelines driven through the low-level exports) |
| 22 | `row22_confusion_selector_cross` |
| 23 | `row23_confusion_negative_selectors` |
| 24 | `row24_confusion_random_all_params` (5000 random 4-tuples) |
| 25 | `row25_confusion_extremes` |
| 26 | `phase_0_selfcheck.rs::no_driver_binary_exists_in_either_project` (asserts mechanically that neither project builds a driver, so the "compare binary stdout" gate is genuinely N/A and will start failing the moment one is added) |
| 27 | `run_tests.sh` — `--offline`, `--no-default-features`, `--all-features` × `release`/`debug` profiles |
| layout | `phase_a_struct_layout_parity` |

**26/26 valid-path tests pass** (plus 4 harness self-checks and 3 symbol-parity tests).

## Harness anti-vacuity guarantees (`tests/phase_0_selfcheck.rs`)

A differential suite is worthless if it silently compares nothing. These are
asserted every run:

* `capture_returns_the_actual_printf_bytes` — the captured stdout is non-empty
  and starts with the exact expected `DEBUG_VAR` lines and ends with
  `Final result: <ret>`.
* `capture_is_sensitive_to_differences` — two different inputs produce different
  captures (so equality assertions are not trivially true).
* `the_two_libraries_are_distinct_objects` — the C `.so` really comes from
  `c_src/build` and is not byte-identical to the Rust `.so`.
* `harness::find_rust_so` refuses to run if the Rust `.so` is older than
  `src/lib.rs`. **`cargo test` does not rebuild a `cdylib`**, so without this
  the suite would validate a stale artifact — this was actually observed and
  fixed during verification.

### Mutation check (harness sensitivity, verified manually)

Three independent defects were injected into `src/lib.rs` and the suite was
re-run; every one was caught:

| injected defect | caught by |
|-----------------|-----------|
| `mode` bit-field offset 8 → 9 | 23 Phase-B tests + 11 Phase-C tests |
| `f32_to_c_int` replaced by Rust's saturating `as c_int` | `row19_row20_confuse_types_float_cast_undefined_range` |
| `"Error: Null pointer"` → `"Error: null pointer"` | `row08_process_buffer_null_state`, `row09_process_buffer_null_buffer` |
