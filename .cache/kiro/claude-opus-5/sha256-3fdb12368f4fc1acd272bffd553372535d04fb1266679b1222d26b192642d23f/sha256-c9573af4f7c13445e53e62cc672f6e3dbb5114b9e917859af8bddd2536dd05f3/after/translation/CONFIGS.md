# CONFIGS.md — configuration / valid-input surface table (Phase B gate)

Derived mechanically from the branches `c_src/src/lib.c` actually takes, not from
what looks important. The axes below are the complete set of things the C code
distinguishes.

## Axis inventory (from the source)

**Build-time configuration.** The C library has no `#ifdef`, no build options and
no compile-time flags (`c_src/CMakeLists.txt` lists one source and no
`target_compile_definitions`). `translation/Cargo.toml` declares no `[features]`.
There is therefore exactly **one** build configuration; `scripts/check_features.sh`
enumerates and re-runs everything for every combination it can find.

**Runtime configuration** (the "options" a caller can set):

| axis | values the C branches on | site |
|------|--------------------------|------|
| A. `apply_bitmask` `operation` | `0` (`&0xF0`), `1` (`&0x0F`), `2` (`\|0xAA`), `3` (`^0x55`), anything else (identity) | `switch` in `apply_bitmask` |
| B. `arity` dispatch `len` (low byte) | `<2` → `-1`; `==2` → `arity2`; `==3` → `arity3`; else → `arity4` | `if/else` chain in `arity` |
| C. `arity4` derived operation `param1 % 4` | `0,1,2,3` (param1 ≥ 0) and `-1,-2,-3` (param1 < 0, C truncating remainder) → 7 reachable values, 3 of which fall through to `default` | `apply_bitmask(result, param1 % 4)` |
| D. `arity4` scaling switch `param3 != 0` | off (`0`) / on-positive / on-negative | `if (param3 != 0)` |
| E. `arity4` offset switch `param4 != 0` | off (`0`) / on-positive / on-negative | `if (param4 != 0)` |
| F. `compare_allocations` bonus `val1 > 0` | bonus applied / not applied | ternary on `*uninit_ptr` |
| G. heap address ordering `ptr1 < ptr2` | ascending (`1`) / descending (`2`) — a real, observable runtime state of glibc's LIFO tcache, controlled in the tests by pre-seeding the 32-byte bin | `if (ptr1 < ptr2)` chain |
| H. `shift_array` shift amount | `1 .. size-1` (in range), plus the rejected values in `ERRORS.md` | guard in `shift_array` |

**Input shapes** the code special-cases:

| axis | values | site |
|------|--------|------|
| I. `shift_array` `size` | `2`, `3`, `4` (the size `arity4` uses), `5`, `8`, `64`, `1024` | loop/`memmove` length |
| J. `shift_array` overlap | destination always overlaps source (`memmove`, not `memcpy`) — full overlap (`positions == 1`) through minimal overlap (`positions == size-1`) | `memmove(arr+positions, arr, ...)` |
| K. `process_string` length | `1`, `2`, `5` (the literal `arity4` uses), `63`, `64`, `255`, `4096` | `strlen` |
| L. `process_string` byte values | ASCII, bytes ≥ 0x80 (`char` signedness), `0x01`, `0x7F`, `0xFF` | `*str` truthiness / `strlen` |
| M. `init_matrix` buffer | exactly 3×4, and a 3×4 window inside a larger sentinel-guarded buffer | `matrix[i][j]` writes |
| N. `int` magnitude | small (no overflow), boundary (`INT_MAX`, `INT_MIN`, `±100`, `±99`, `0x7fff0000`), fully random 32-bit | all arithmetic |
| O. entry-point level | low-level (`shift_array`, `process_string`, `apply_bitmask`, `init_matrix`, `compare_allocations`) → mid (`arity4`) → wrappers (`arity3`, `arity2`) → dispatcher (`arity`) | call hierarchy |

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (`common::Rng`), and — for every row that reaches
`compare_allocations` — under **both** heap-ordering states (axis G), so each row
is really run twice.

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `apply_bitmask` | `operation = 0` (`& 0xF0`) × 512 random + boundary `value`s | `valid_apply_bitmask_all_operations` | [x] |
| 2 | `apply_bitmask` | `operation = 1` (`& 0x0F`) × 512 random + boundary `value`s | `valid_apply_bitmask_all_operations` | [x] |
| 3 | `apply_bitmask` | `operation = 2` (`\| 0xAA`) × 512 random + boundary `value`s | `valid_apply_bitmask_all_operations` | [x] |
| 4 | `apply_bitmask` | `operation = 3` (`^ 0x55`) × 512 random + boundary `value`s | `valid_apply_bitmask_all_operations` | [x] |
| 5 | `apply_bitmask` | fully random `(value, operation)` pairs, both 32-bit — covers valid and `default` labels mixed | `valid_apply_bitmask_random_pairs` | [x] |
| 6 | `process_string` | length 1 string, every possible single byte `0x01..0xFF` (incl. ≥ 0x80, axis L) | `valid_process_string_single_byte_all_values` | [x] |
| 7 | `process_string` | random lengths 1..=255, random non-NUL bytes | `valid_process_string_random_lengths` | [x] |
| 8 | `process_string` | boundary lengths `1, 2, 5, 63, 64, 255, 4096` | `valid_process_string_boundary_lengths` | [x] |
| 9 | `shift_array` | `size = 4`, `positions = 1` — the exact configuration `arity4` uses, random contents | `valid_shift_array_arity4_config` | [x] |
| 10 | `shift_array` | `size ∈ {2,3,4,5,8,64,1024}` × **every** in-range `positions = 1..size-1` (full overlap → minimal overlap, axis J) × random contents | `valid_shift_array_all_sizes_and_positions` | [x] |
| 11 | `shift_array` | random `size` 2..=256 × random in-range `positions`, with guard bytes around the buffer to prove neither impl writes out of bounds | `valid_shift_array_random_with_guards` | [x] |
| 12 | `init_matrix` | exact 3×4 destination, buffer pre-filled with random garbage | `valid_init_matrix_exact_buffer` | [x] |
| 13 | `init_matrix` | 3×4 window inside a larger sentinel-guarded buffer (proves exactly 12 ints written) | `valid_init_matrix_guarded_window` | [x] |
| 14 | `compare_allocations` | `val1 > 0` (bonus on, axis F) × heap ascending (axis G) × random `val2` | `valid_compare_allocations_matrix` | [x] |
| 15 | `compare_allocations` | `val1 > 0` × heap descending | `valid_compare_allocations_matrix` | [x] |
| 16 | `compare_allocations` | `val1 == 0` (bonus off) × heap ascending / descending | `valid_compare_allocations_matrix` | [x] |
| 17 | `compare_allocations` | `val1 < 0` (bonus off) × heap ascending / descending × `INT_MIN` | `valid_compare_allocations_matrix` | [x] |
| 18 | `compare_allocations` | fully random `(val1, val2)` × both heap states | `valid_compare_allocations_random` | [x] |
| 19 | `arity4` | `param1 % 4 == 0` with `param1 > 0`; `param3 = 0`, `param4 = 0` (both switches off) × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 20 | `arity4` | `param1 % 4 == 1`; `param3 = 0`, `param4 = 0` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 21 | `arity4` | `param1 % 4 == 2`; `param3 = 0`, `param4 = 0` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 22 | `arity4` | `param1 % 4 == 3`; `param3 = 0`, `param4 = 0` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 23 | `arity4` | `param1 % 4 == -1` (`param1 < 0`, `default` label, bonus off) × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 24 | `arity4` | `param1 % 4 == -2` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 25 | `arity4` | `param1 % 4 == -3` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 26 | `arity4` | `param1 == 0` (mod 0, bonus off — distinct from row 19 which has the bonus on) × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 27 | `arity4` | each of the 7 `param1 % 4` values × `param3 > 0` (scaling on, positive) × `param4 = 0` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 28 | `arity4` | each `param1 % 4` × `param3 < 0` (scaling on, negative → negative numerator, truncation toward zero) × `param4 = 0` × both heap states | `valid_arity4_mod_and_switch_matrix` | [x] |
| 29 | `arity4` | each `param1 % 4` × `param3 = 0` × `param4 > 0` | `valid_arity4_mod_and_switch_matrix` | [x] |
| 30 | `arity4` | each `param1 % 4` × `param3 = 0` × `param4 < 0` | `valid_arity4_mod_and_switch_matrix` | [x] |
| 31 | `arity4` | each `param1 % 4` × `param3 != 0` × `param4 != 0` (both switches on, all four sign combinations) | `valid_arity4_mod_and_switch_matrix` | [x] |
| 32 | `arity4` | small-magnitude random params (no overflow anywhere) × both heap states | `valid_arity4_random_small` | [x] |
| 33 | `arity4` | fully random 32-bit params (overflow in the sum, in `result * param3`, and in `result + param4`) × both heap states | `valid_arity4_random_full_range` | [x] |
| 34 | `arity4` | params drawn from the boundary pool (`INT_MAX`, `INT_MIN`, `±100`, `±99`, `255`, `256`, `0x7fff0000`, …) in all four positions × both heap states | `valid_arity4_boundary_pool` | [x] |
| 35 | `arity3` | `param3 = 0` and `param3 != 0` (both signs) × all 7 `param1 % 4` × both heap states, small + full-range values | `valid_arity3_matrix` | [x] |
| 36 | `arity2` | all 7 `param1 % 4` × both heap states, small + full-range values | `valid_arity2_matrix` | [x] |
| 37 | `arity` | `len = 2` (dispatch → `arity2`); trailing `params[2..]` filled with garbage to prove they are not read | `valid_arity_dispatch_len2` | [x] |
| 38 | `arity` | `len = 3` (dispatch → `arity3`); `params[3]` garbage | `valid_arity_dispatch_len3` | [x] |
| 39 | `arity` | `len = 4` (dispatch → `arity4`) | `valid_arity_dispatch_len4` | [x] |
| 40 | `arity` | `len ∈ {5, 6, 7, 8, 100, 127, 128, 200, 254, 255}` — all take the `arity4` branch and read exactly 4 ints | `valid_arity_dispatch_len_above_four` | [x] |
| 41 | `arity` | `len` swept over all `0..=255` with a fixed 4-element buffer × both heap states | `valid_arity_full_len_sweep` | [x] |
| 42 | `arity` | random `len` × random 4-element `params` × both heap states (end-to-end, the way a real consumer drives the library) | `valid_arity_random_end_to_end` | [x] |
| 43 | composed pipeline | `shift_array` → `process_string` → `apply_bitmask` → `init_matrix` → `compare_allocations` driven **directly** in `arity4`'s exact order and with `arity4`'s exact arguments, then compared against `arity4` itself, for both impls — catches divergence that per-function tests hide | `valid_manual_pipeline_matches_arity4` | [x] |
| 44 | cross-impl mixing | C helpers + Rust helpers used interchangeably inside one hand-rolled pipeline (Rust `shift_array` on a buffer later summed by the C path, and vice versa) | `valid_cross_impl_pipeline` | [x] |
