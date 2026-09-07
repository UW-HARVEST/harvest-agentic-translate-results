# ERRORS.md — Error / rejection surface table (Phase C)

Mechanically derived from every rejection point in `c_src/src/lib.c`. Grep basis:

```
grep -n 'return\|== NULL\|is_null\|assert\|if (' c_src/src/lib.c
```

The C code has **no** error enum, no `errno` use, and no `assert`. It rejects in
exactly three ways: (a) print a message on `stdout` and return early, (b) return
`NULL` from `get_operation`, (c) return a sentinel value (`0` / `-1`).
Every distinct branch below is one row.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `execute_operation` | `func == NULL` (line 84) | prints `Error: Operation function pointer is NULL for %s\n` with `op_name`; returns `0`; does **not** call any op | `err_01_execute_operation_null_func` |
| 2 | `execute_operation` | `func == NULL` **and** `op_name` is an empty string `""` | prints `...NULL for \n`; returns `0` | `err_02_execute_operation_null_func_empty_name` |
| 3 | `compute_checksum` | `values == NULL`, `count > 0` (line 102, first conjunct false) | body skipped; returns `0 & MASK_LOWER` = `0`; no `memcpy` | `err_03_compute_checksum_null_values` |
| 4 | `compute_checksum` | `values != NULL`, `count == 0` (second conjunct false) | body skipped; returns `0` | `err_04_compute_checksum_zero_count` |
| 5 | `compute_checksum` | `values != NULL`, `count < 0` (e.g. `-1`, `INT_MIN`) | body skipped; returns `0` | `err_05_compute_checksum_negative_count` |
| 6 | `compute_checksum` | `values == NULL` **and** `count == 0` (both conjuncts false) | returns `0` | `err_06_compute_checksum_null_and_zero` |
| 7 | `compute_checksum` | `values == NULL` **and** `count < 0` | returns `0` | `err_07_compute_checksum_null_and_negative` |
| 8 | `init_state` | `state == NULL` (line 117) | prints `Error: state pointer is NULL in init_state\n`; returns; no write | `err_08_init_state_null` |
| 9 | `apply_operation` | `state == NULL` (line 130), any `func` (incl. `NULL`) | prints `Error: state pointer is NULL in apply_operation\n`; returns. State check precedes func check | `err_09_apply_operation_null_state` |
| 10 | `apply_operation` | `state != NULL`, `func == NULL` (line 135) | prints `Error: operation function pointer is NULL in apply_operation\n`; returns; `accumulator`/`operation_count` **unchanged** | `err_10_apply_operation_null_func` |
| 11 | `apply_operation` | `state == NULL` **and** `func == NULL` | only the *state* message is printed (ordering matters) | `err_11_apply_operation_both_null` |
| 12 | `get_operation` | `opcode < 0` (line 76, first conjunct false) — `-1`, `-2`, `INT_MIN` | returns `NULL` | `err_12_get_operation_negative` |
| 13 | `get_operation` | `opcode >= 4` (second conjunct false) — `4`, `5`, `0x7fffffff` | returns `NULL` | `err_13_get_operation_too_large` |
| 14 | `get_operation` | one step past each end of the valid range: `-1` and `4` exactly | returns `NULL` for both; `0..=3` non-NULL | `err_14_get_operation_range_edges` |
| 15 | `get_operation` | "out-of-range enum value" class: the `OP_*` macros are `1,2,3,4`, so `OP_SHIFT == 4` is itself **out of range** for the `ops[]` table, while opcode `0` has no `OP_*` name. Passing every `OP_*` constant plus arbitrary `int`s | `OP_ADD/OP_MULTIPLY/OP_XOR` → non-NULL, `OP_SHIFT` (`4`) → `NULL` | `err_15_get_operation_opcode_enum_values` |
| 16 | `checkshift` | `malloc(sizeof(ComputeState))` returns `NULL` (line 150) | prints `Error: Failed to allocate memory for state\n`; returns `-1`; `init_state` is never reached | `err_16_checkshift_malloc_failure` (real injection: the test binary **interposes `malloc`** and fails the 12-byte request for both libs) + `err_16_checkshift_malloc_failure_documented` (string presence in both `.so`s) |
| 17 | cross-cutting | `execute_operation` reached with a **valid** func but `op_name == NULL` | glibc `printf("%s")` prints `(null)`; return value is the op result. Both libs must agree | `err_17_execute_operation_null_name` |

## Constants / limits appearing in the code

| constant | value | where it clamps or masks |
|----------|-------|--------------------------|
| `MASK_LOWER` | `0x0000FFFF` | `compute_checksum` return is always masked to 16 bits |
| `MAGIC_NUMBER` | `0xDEADBEEF` | XORed in only when the guard passes |
| `4` (table size) | — | `get_operation` valid opcode range `[0, 4)` |
| `4` (copy clamp) | — | `compute_checksum`: `copy_count = min(count, 4)`; buffer is `sizeof(int)*4` = 16 bytes |
| `static_shift_amount` | `2` | `shift_with_static` shift distance (never `>= 32`, so no UB shift) |

## Checklist

- [x] 1 · [x] 2 · [x] 3 · [x] 4 · [x] 5 · [x] 6 · [x] 7 · [x] 8 · [x] 9
- [x] 10 · [x] 11 · [x] 12 · [x] 13 · [x] 14 · [x] 15 · [x] 16 · [x] 17

## Result

All 17 rows plus the three generic-boundary tests (`err_gen_*`: NULL pointers on
every pointer parameter, zero/oversized/negative lengths, and one-step-past-range
values including out-of-range "enum" opcodes) pass in every configuration.

### Divergence found and fixed during Phase C

Row 16 exposed a real translation bug. LLVM applies its built-in knowledge that
`malloc` returns non-null and had **deleted the entire `state == NULL` branch**
from the Rust `checkshift` — the diagnostic string was absent from the `.so` and
the `-1` sentinel was unreachable, whereas the C keeps the branch. With `malloc`
interposed to fail the 12-byte request, the C printed
`Error: Failed to allocate memory for state` and returned `-1` while the Rust
dereferenced the null pointer. Fixed in `src/lib.rs` by wrapping the allocation
in `core::hint::black_box`, which keeps the check reachable; `err_16_checkshift_
malloc_failure` now shows both libraries producing identical output and `-1`.
