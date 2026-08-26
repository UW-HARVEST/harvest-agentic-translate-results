# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Every distinct way `c_src/src/lib.c` rejects / errors on input, derived
mechanically by grepping the source for `if (... == NULL)`, `return NULL`,
`return -1`, `return 0`, `return;`, every explicit range check
(`opcode >= 0 && opcode < 4`, `count > 0`, `count > 4`) and every
`printf("Error: ...")`. There are **no** `assert`s and **no** error enums in the
C source; the entire rejection surface is null-checks, range-checks and
sentinel returns.

`"..."` in *expected C result* means the exact bytes the C emits on **stdout** —
Phase C asserts stdout equality as well as return-value equality, because these
diagnostics *are* the observable error report.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `get_operation` (lib.c:76-80) | `opcode < 0` — e.g. `-1` | returns `NULL`; no output | `e1_get_operation_negative_opcode` | [x] |
| E2 | `get_operation` (lib.c:76-80) | `opcode >= 4` — e.g. `4` (one past the valid range) | returns `NULL`; no output | `e2_get_operation_opcode_at_and_past_upper_bound` | [x] |
| E3 | `get_operation` (lib.c:76-80) | extreme out-of-range opcodes `INT_MIN`, `INT_MAX` (out-of-range "enum"-style ints across FFI) | returns `NULL`; no output | `e3_get_operation_extreme_opcodes` | [x] |
| E4 | `execute_operation` (lib.c:84-87) | `func == NULL`, `op_name` a valid string | prints `Error: Operation function pointer is NULL for <op_name>\n`; returns `0`; **does not** print the two `Variable ...` lines | `e4_execute_operation_null_func` | [x] |
| E5 | `execute_operation` (lib.c:84-87) | `func == NULL` **and** `op_name == NULL` (`%s` with `NULL`) | prints `Error: Operation function pointer is NULL for (null)\n` (glibc `%s`/`NULL`); returns `0` | `e5_execute_operation_null_func_null_name` | [x] |
| E6 | `execute_operation` (lib.c:84-87) | `func == NULL` and `op_name == ""` (empty, zero-length string) | prints `Error: Operation function pointer is NULL for \n`; returns `0` | `e6_execute_operation_null_func_empty_name` | [x] |
| E7 | `execute_operation` (lib.c:83) | `func == get_operation(<out-of-range>)` i.e. the `NULL` produced by E1/E2 fed straight back in — composed rejection | same as E4: error line, returns `0` | `e7_execute_operation_with_null_from_get_operation` | [x] |
| E8 | `compute_checksum` (lib.c:102) | `values == NULL`, `count > 0` | the fold is skipped entirely; returns `0 & MASK_LOWER` = `0`; **no** `MAGIC_NUMBER` mix-in; no output | `e8_compute_checksum_null_values` | [x] |
| E9 | `compute_checksum` (lib.c:102) | `count == 0` (zero length), `values` valid | fold skipped; returns `0`; no output | `e9_compute_checksum_zero_count` | [x] |
| E10 | `compute_checksum` (lib.c:102) | `count < 0` (negative length) — e.g. `-1`, `INT_MIN` | fold skipped; returns `0`; no output | `e10_compute_checksum_negative_count` | [x] |
| E11 | `compute_checksum` (lib.c:102) | `values == NULL` **and** `count <= 0` (both invalid) | fold skipped; returns `0` | `e11_compute_checksum_null_and_nonpositive` | [x] |
| E12 | `compute_checksum` (lib.c:103) | `count > 4` (oversized length, incl. `INT_MAX`) — clamped, **not** rejected: reads only 4 ints, never overruns `buffer[16]` | returns the checksum of the first 4 ints, identical to `count == 4` | `e12_compute_checksum_oversized_count_clamps` | [x] |
| E13 | `init_state` (lib.c:117-120) | `state == NULL` | prints `Error: state pointer is NULL in init_state\n`; returns (void); nothing written | `e13_init_state_null_state` | [x] |
| E14 | `apply_operation` (lib.c:130-133) | `state == NULL` (`func` valid) | prints `Error: state pointer is NULL in apply_operation\n`; returns; nothing written | `e14_apply_operation_null_state` | [x] |
| E15 | `apply_operation` (lib.c:135-138) | `func == NULL` (`state` valid) | prints `Error: operation function pointer is NULL in apply_operation\n`; returns; `accumulator` **and** `operation_count` left untouched | `e15_apply_operation_null_func` | [x] |
| E16 | `apply_operation` (lib.c:130-138) | `state == NULL` **and** `func == NULL` — precedence check: the `state` check comes first, so only the *state* message must appear | prints only `Error: state pointer is NULL in apply_operation\n` | `e16_apply_operation_null_state_and_func` | [x] |
| E17 | `checkshift` (lib.c:150-153) | `malloc(sizeof(ComputeState))` returns `NULL` | prints `\n=== Starting foo function ===\nParameters: ...\n` then `Error: Failed to allocate memory for state\n`; returns `-1`; **never** reaches `init_state` | `e17_checkshift_malloc_failure` — branch **actually executed** in both `.so`s via the `LD_PRELOAD` fault injector `tests/fixtures/failmalloc.c` (arms a failure for `malloc(12)` only around the single `checkshift` call, in a re-exec'd child process) | [x] |

## Generic FFI-boundary boundaries also covered in Phase C

(the C API has no enums, so "out-of-range enum value" maps onto the two
int-tagged dispatch inputs: `get_operation`'s `opcode` and the
`operation_func` pointer)

| # | boundary | test | also covered by | ✔ |
|---|----------|------|-----------------|---|
| G1 | null pointer in **every** pointer parameter of every exported function (`values`, `state`, `op_name`, `func`), incl. a **valid** `func` with a null `op_name` (the `%s`/NULL case on the *success* path) | `g_null_pointers_in_every_pointer_parameter` | E4, E5, E8, E11, E13, E14, E15, E16 | [x] |
| G2 | zero length | `e9_compute_checksum_zero_count` | E11 | [x] |
| G3 | oversized length (`count` = 5, 6, 8, 16, 17, 1024, `INT_MAX-1`, `INT_MAX`) | `e12_compute_checksum_oversized_count_clamps` | — | [x] |
| G4 | one step past a documented valid range (`opcode` = 4, `opcode` = -1, `count` = 5) | `e2_get_operation_opcode_at_and_past_upper_bound` | E1, E12 | [x] |
| G5 | out-of-range int-tagged dispatch value across FFI (`opcode` has only 4 valid variants; any other `int` is a real input) — dense sweep `-8..=11`, `INT_MIN`/`INT_MAX`, the `EDGES` list, and 4096 random `i32`s | `e3_get_operation_extreme_opcodes` | E1, E2, E7 | [x] |
| G6 | a *foreign* `operation_func`: pointer minted by the **other** `.so` (all 4 provider×callee combinations) and by the **test binary** itself, passed into `execute_operation` / `apply_operation` | `c25_cross_module_abi`, `c16_execute_operation_foreign_func` | Phase B row C25 | [x] |
| G7 | extreme scalars `INT_MIN` / `INT_MAX` / `0` / `-1` into every arithmetic entry point and into `checkshift` (wrap-around on `*`, `+`, `<<`) | `g_extreme_scalars_every_entry_point` | C2, C4, C6, C10, C24 | [x] |
| G8 | `ComputeState` object representation (size 12, align 4) written/read across the FFI boundary by each `.so` | `c25b_compute_state_layout_agrees` | C19, C20, C25 | [x] |
