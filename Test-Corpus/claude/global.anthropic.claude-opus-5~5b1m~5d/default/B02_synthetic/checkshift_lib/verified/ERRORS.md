# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Mechanically derived from every rejection site in `c_src/src/lib.c`. There are no
`assert`s, no `errno` use, and no error enums in this library; rejection is
expressed as (a) a `NULL` return, (b) a sentinel numeric return (`0` / `-1`),
(c) an early `void` return, and always accompanied by a specific `printf`
diagnostic where the C prints one. Because stdout is part of the observable
behaviour, **every row asserts both the return value AND the byte-exact stdout**.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `get_operation` | `opcode < 0` (e.g. `-1`, `-4`, `INT_MIN`) — fails `opcode >= 0` at L76 | returns `NULL` (L80); no output | `err_e1_get_operation_negative_opcode` |
| E2 | `get_operation` | `opcode >= 4` (e.g. `4`, `5`, `INT_MAX`) — fails `opcode < 4` at L76 | returns `NULL` (L80); no output | `err_e2_get_operation_opcode_ge_4` |
| E3 | `get_operation` | out-of-range "enum" values: the `OP_*` macros are `1..4` but the accepted index range is `0..3`, so `OP_SHIFT == 4` is out of range | `OP_ADD/MULTIPLY/XOR` (1,2,3) → non-NULL; `OP_SHIFT` (4) → `NULL` | `err_e3_get_operation_op_macro_values` |
| E4 | `execute_operation` | `func == NULL` (L84), valid `op_name` | prints `Error: Operation function pointer is NULL for <op_name>\n`; returns `0` (L86) | `err_e4_execute_operation_null_func` |
| E5 | `execute_operation` | `func == NULL` **and** `op_name == NULL` → `%s` with a NULL pointer | glibc prints `...NULL for (null)\n`; returns `0` | `err_e5_execute_operation_null_func_null_name` |
| E6 | `execute_operation` | `func == NULL` and `op_name == ""` (empty string) | prints `...NULL for \n`; returns `0` | `err_e6_execute_operation_null_func_empty_name` |
| E7 | `compute_checksum` | `values == NULL` (fails `values != NULL` at L102), any `count` incl. positive | body skipped; returns `0 & MASK_LOWER == 0`; no output | `err_e7_compute_checksum_null_values` |
| E8 | `compute_checksum` | `count == 0` (fails `count > 0` at L102), valid `values` | body skipped; returns `0`; no output | `err_e8_compute_checksum_zero_count` |
| E9 | `compute_checksum` | `count < 0` (e.g. `-1`, `INT_MIN`), valid `values` | body skipped; returns `0`; no output | `err_e9_compute_checksum_negative_count` |
| E10 | `compute_checksum` | `count > 4` — oversized length clamped by `(count > 4) ? 4 : count` (L103) | reads only the first 4 ints; result identical to `count == 4`; never overruns the 16-byte buffer | `err_e10_compute_checksum_count_clamped_to_4` |
| E11 | `compute_checksum` | `values == NULL` **and** `count <= 0` (both guard operands false) | returns `0`; no output | `err_e11_compute_checksum_null_and_nonpositive` |
| E12 | `init_state` | `state == NULL` (L117) | prints `Error: state pointer is NULL in init_state\n`; returns void, writes nothing | `err_e12_init_state_null_state` |
| E13 | `apply_operation` | `state == NULL` (L130), `func` non-NULL | prints `Error: state pointer is NULL in apply_operation\n`; returns void; `func` is **not** called | `err_e13_apply_operation_null_state` |
| E14 | `apply_operation` | `state` non-NULL, `func == NULL` (L135) | prints `Error: operation function pointer is NULL in apply_operation\n`; returns void; state left **unmodified** (`operation_count` not incremented) | `err_e14_apply_operation_null_func` |
| E15 | `apply_operation` | `state == NULL` **and** `func == NULL` — check order matters: `state` is tested first (L130 before L135) | prints **only** the `state ... NULL in apply_operation` message, not the func one; returns void | `err_e15_apply_operation_both_null_order` |
| E16 | `checkshift` | `malloc(sizeof(ComputeState))` returns `NULL` (L150) | prints `Error: Failed to allocate memory for state\n`; returns `-1`; does **not** reach `init_state` or the closing banner | `err_e16_checkshift_malloc_failure_path` — reached for real with an `LD_PRELOAD` `malloc` interposer that fails allocations of exactly 12 bytes, driven out-of-process against both `.so`s (`tests/phase_c_malloc_failure.rs`). **This row found a genuine divergence — see FINDINGS.md #1.** |

## Generic FFI boundary cases (covered even though not distinct C branches)

| # | case | covered by |
|---|------|-----------|
| G1 | NULL pointer for every pointer parameter (`values`, `state`, `op_name`, `func`) | E4–E7, E12–E15 |
| G2 | Zero length / count | E8 |
| G3 | Oversized length (`count` far past the 4-element clamp, incl. `INT_MAX`) | E10 |
| G4 | One step past a valid range boundary (`opcode` = `-1` and `4`) | E1, E2 |
| G5 | Out-of-range enum-like value across FFI (`get_operation` with `INT_MIN`/`INT_MAX`/`0x7FFFFFFF`, and the `OP_*` macro mismatch) | E1, E2, E3 |
| G6 | Extreme integer inputs (`INT_MIN`, `INT_MAX`) to every arithmetic entry point — signed overflow in `*`, `+`, `<<` | Phase B rows C1–C4, C20 |
| G7 | Cross-library function pointers (a C `.so` `operation_func` handed to Rust's `execute_operation`/`apply_operation`, and vice versa) | Phase B rows C13, C18 |

## Completion

Every row has a passing differential test asserting the SAME sentinel/return
value AND byte-identical stdout from both `.so`s, under both the `debug` and
`release` profiles.

- [x] E1  - [x] E2  - [x] E3  - [x] E4  - [x] E5  - [x] E6  - [x] E7  - [x] E8
- [x] E9  - [x] E10 - [x] E11 - [x] E12 - [x] E13 - [x] E14 - [x] E15 - [x] E16
- [x] G1  - [x] G2  - [x] G3  - [x] G4  - [x] G5  - [x] G6  - [x] G7

Test files: `tests/phase_c_errors.rs` (E1–E15, G1–G7, 19 tests),
`tests/phase_c_malloc_failure.rs` (E16, 3 tests).

Run: `cargo test --test phase_c_errors --test phase_c_malloc_failure`
