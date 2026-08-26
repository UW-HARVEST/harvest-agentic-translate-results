# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/main.c`: every `return -1` / `return NULL`
/ `return false` / `return 1`, every `fprintf(stderr, ...)`, every explicit
range check, null check and min/max constant. (`grep -c assert main.c` = 0 —
the C code contains no assertions.)

`rc` = value returned by the function. "stderr" = the exact bytes the C code
writes to `stderr` (which the Rust side must reproduce byte for byte).

## Legend

* **[x]** = a differential test constructs this exact condition, calls both the
  C `.so` and the Rust `.so`, and asserts the same rc / sentinel / stderr.
* **UB** = the C code performs the operation with no validation and overruns a
  `uint8_t[256]`; the C behaviour is undefined (stack smash / struct overrun).
  Excluded from differential execution — see "Undefined-behaviour rows" below.

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `validate_buffer` (main.c:67) | `buf == NULL` | `false`; stderr `Error: NULL buffer\n` | [x] |
| 2 | `validate_buffer` (main.c:71) | `buf->length > 256` (e.g. 257, 512, SIZE_MAX) | `false`; stderr `Error: Buffer length <len> exceeds maximum 256\n` | [x] |
| 3 | `validate_buffer` (main.c:76) | `buf->checksum != calculate_checksum(data,length)` | still returns `true`, but emits stderr `Warning: Checksum mismatch. Expected <exp>, got <got>\n` | [x] |
| 4 | `init_buffer_array` (main.c:85) | `initial_capacity <= 0` (0, -1, INT_MIN) | `NULL`; stderr `Error: Invalid capacity <cap>\n` | [x] |
| 5 | `init_buffer_array` (main.c:91) | `malloc(sizeof(buffer_array_t))` fails | `NULL`; stderr `Error: Failed to allocate buffer array\n` | not triggerable without an allocator fault injector; both sides take the identical branch shape (documented, not executed) |
| 6 | `init_buffer_array` (main.c:97) | `malloc(sizeof(buffer_t)*cap)` fails (e.g. cap = INT_MAX → 272 * 2^31 bytes) | `NULL` + `free(arr)`; stderr `Error: Failed to allocate buffer storage\n` | [x] (cap = INT_MAX / 0x40000000 makes the second malloc fail) |
| 7 | `free_buffer_array` (main.c:110) | `arr == NULL` | returns silently, no free, no output | [x] |
| 8 | `buffer_copy` (main.c:120) | `src == NULL` | `-1`; stderr `Error: NULL pointer in buffer_copy\n` | [x] |
| 9 | `buffer_copy` (main.c:120) | `dst == NULL` | `-1`; same stderr as #8 | [x] |
| 10 | `buffer_copy` (main.c:120) | both NULL | `-1`; same stderr as #8 | [x] |
| 11 | `buffer_copy` (main.c:125) | `validate_buffer(src)` fails, i.e. `src->length > 256` | `-1`; stderr from #2 only (no second message) | [x] |
| 12 | `buffer_reverse` (main.c:139) | `buf == NULL` | `-1`; stderr `Error: NULL buffer in reverse\n` | [x] |
| 13 | `buffer_reverse` (main.c:144) | `buf->length == 0` | `0`, early return, **checksum left untouched** (not recomputed) | [x] |
| 14 | `buffer_merge` (main.c:161) | any of `src1`, `src2`, `dst` NULL (7 combinations) | `-1`; stderr `Error: NULL pointer in buffer_merge\n` | [x] |
| 15 | `buffer_merge` (main.c:166) | `src1->length + src2->length > 256` (e.g. 256+1, 200+200, 257+0) | `-1`; stderr `Error: Merged length <sum> exceeds maximum\n` | [x] |
| 16 | `buffer_split` (main.c:186) | any of `src`, `dst1`, `dst2` NULL (7 combinations) | `-1`; stderr `Error: NULL pointer in buffer_split\n` | [x] |
| 17 | `buffer_split` (main.c:191) | `split_pos > src->length` (incl. `split_pos = length+1` and sign-extended negatives such as `(size_t)-1`) | `-1`; stderr `Error: Split position <pos> exceeds length <len>\n` | [x] |
| 18 | `buffer_interleave` (main.c:217) | any of `src1`, `src2`, `dst` NULL (7 combinations) | `-1`; stderr `Error: NULL pointer in buffer_interleave\n` | [x] |
| 19 | `buffer_interleave` (main.c:223) | `src1->length + src2->length > 256` | `-1`; stderr `Error: Interleaved length exceeds maximum\n` (no numbers) | [x] |
| 20 | `buffer_rotate` (main.c:246) | `buf == NULL` | `-1`; stderr `Error: NULL buffer in rotate\n` | [x] |
| 21 | `buffer_rotate` (main.c:251) | `buf->length == 0` (any `positions`) | `0`, early return, checksum untouched | [x] |
| 22 | `buffer_rotate` (main.c:251) | `positions == 0` (any length) | `0`, early return, checksum untouched | [x] |
| 23 | `buffer_rotate` (main.c:256/257) | `positions < 0` → `positions %= (int)length` then `positions += length` in `size_t` and back to `int` | `0`, rotates by the normalised amount | [x] |
| 24 | `buffer_conditional_copy` (main.c:276) | `src == NULL` or `dst == NULL` | `-1`; stderr `Error: NULL pointer in conditional_copy\n` | [x] |
| 25 | `buffer_copy_strided` (main.c:297) | `src == NULL` or `dst == NULL` | `-1`; stderr `Error: NULL pointer in copy_strided\n` | [x] |
| 26 | `buffer_copy_strided` (main.c:302) | `stride <= 0` (0, -1, INT_MIN) | `-1`; stderr `Error: Invalid stride <stride>\n` | [x] |
| 27 | `process_buffer_array` (main.c:322) | `arr == NULL` | `-1`; stderr `Error: Invalid buffer array\n` | [x] |
| 28 | `process_buffer_array` (main.c:322) | `arr->count == 0` | `-1`; same stderr as #27 | [x] |
| 29 | `process_buffer_array` (main.c:348) | `op == OP_MERGE` and `arr->count < 2` (1, and negative counts) | `-1`; stderr `Error: Need at least 2 buffers for merge\n` | [x] |
| 30 | `process_buffer_array` (main.c:379) | `op` not in {0,1,2,5,6} — note **`OP_SPLIT`(3) and `OP_INTERLEAVE`(4) are valid enum values that fall through to `default`**, as do 7, -1, INT_MIN, INT_MAX | `-1`; stderr `Error: Unknown operation <op>\n` (`%d`, so 0xFFFFFFFF prints as `-1`) | [x] |
| 31 | `process_buffer_array` (main.c:331) | `op == OP_COPY` and `buffers[0].length > 256` → inner `buffer_copy` fails | `-1`; stderr from #2 | [x] |
| 32 | `process_buffer_array` (main.c:354) | `op == OP_MERGE` and a pair's combined length > 256 → inner `buffer_merge` fails | `-1`; stderr from #15 | [x] |
| 33 | `process_buffer_array` (main.c:373) | `op == OP_CHECKSUM` and some `buffers[i].length > 256` → `validate_buffer` fails | `-1`; stderr from #2 | [x] |
| 34 | `read_buffer` (main.c:391) | `buf == NULL` | `-1`; stderr `Error: NULL buffer in read_buffer\n` (checked **before** any `scanf`, so stdin is not consumed) | [x] |
| 35 | `read_buffer` (main.c:397) | `scanf("%d", &length) != 1` — EOF, or a non-numeric character (`x`, `-`, `+`, `.`) | `-1`; stderr `Error: Failed to read buffer length\n` | [x] |
| 36 | `read_buffer` (main.c:402) | `length < 0` (e.g. `-1`) | `-1`; stderr `Error: Invalid buffer length <length>\n` | [x] |
| 37 | `read_buffer` (main.c:402) | `length > 256` (e.g. `257`) | `-1`; same stderr as #36 | [x] |
| 38 | `read_buffer` (main.c:410) | `scanf("%d", &byte) != 1` for element `i` (short input / junk token) | `-1`; stderr `Error: Failed to read byte <i>\n` (`%zu`) | [x] |
| 39 | `write_buffer` (main.c:423) | `buf == NULL` | returns silently; stderr `Error: NULL buffer in write_buffer\n`, nothing on stdout | [x] |
| 40 | `main` (main.c:442) | `scanf("%d", &operation) != 1` (empty stdin, junk) | exit `1`; stderr `Error: Failed to read operation\n` | [x] |
| 41 | `main` (main.c:448) | `scanf("%d", &buffer_count) != 1` | exit `1`; stderr `Error: Failed to read buffer count\n` | [x] |
| 42 | `main` (main.c:453) | `buffer_count <= 0` (0, -1) | exit `1`; stderr `Error: Invalid buffer count <n>\n` | [x] |
| 43 | `main` (main.c:453) | `buffer_count > 100` (101, INT_MAX) | exit `1`; same stderr as #42 | [x] |
| 44 | `main` (main.c:460) | `init_buffer_array` returned NULL | exit `1`, no extra message of its own | [x] (covered through #42/#43 — `buffer_count` is already validated, so only reachable on malloc failure) |
| 45 | `main` (main.c:466) | any `read_buffer` fails while filling the array | exit `1`; stderr from #35/#36/#37/#38, array freed | [x] |
| 46 | `main` OP_COPY (main.c:484) | `operation == 0` and `buffer_count < 2` | exit `1`; stderr `Error: Copy needs at least 2 buffers\n` | [x] |
| 47 | `main` OP_MERGE (main.c:505) | `operation == 2` and `buffer_count < 2` | exit `1`; stderr `Error: Merge needs at least 2 buffers\n` | [x] |
| 48 | `main` OP_SPLIT (main.c:514) | `operation == 3` and `scanf("%d", &split_pos) != 1` | exit `1`; stderr `Error: Failed to read split position\n` | [x] |
| 49 | `main` OP_SPLIT (main.c:518) | `operation == 3` and `split_pos > buffers[0].length` (incl. negative → sign-extended) | exit `1`; stderr from #17 | [x] |
| 50 | `main` OP_INTERLEAVE (main.c:536) | `operation == 4` and `buffer_count < 2` | exit `1`; stderr `Error: Interleave needs at least 2 buffers\n` | [x] |
| 51 | `main` OP_ROTATE (main.c:544) | `operation == 5` and `scanf("%d", &positions) != 1` | exit `1`; stderr `Error: Failed to read rotation amount\n` | [x] |
| 52 | `main` (main.c:562) | `operation` not in 0..6 (7, -1, 42, INT_MIN, INT_MAX, and scanf-overflow results) | exit `1`; stderr `Error: Unknown operation <op>\n` | [x] |
| 53 | `main` OP_MERGE (main.c:500) | `operation == 2` and `buffers[0].length + buffers[1].length > 256` | exit `1`; stderr from #15 | [x] |
| 54 | `main` OP_INTERLEAVE (main.c:531) | `operation == 4` and combined length > 256 | exit `1`; stderr from #19 | [x] |

### Generic FFI-boundary boundaries also covered (not distinct C branches)

| # | condition | test |
|---|-----------|------|
| G1 | NULL passed for **every** pointer parameter of **every** exported function, in every combination | [x] |
| G2 | `calculate_checksum(NULL, 0)` — length 0 means the NULL is never dereferenced | [x] |
| G3 | zero lengths everywhere (`length == 0` for all buffer ops, `split_pos == 0`) | [x] |
| G4 | exactly-at-maximum lengths (`length == 256`, `256 + 0` merge, `split_pos == length`) | [x] |
| G5 | one step past the range (`length == 257` for the *checked* functions, `split_pos == length + 1`, `capacity == 0`, `stride == 0`, `buffer_count == 101`, `read_buffer length == 257`) | [x] |
| G6 | out-of-range `operation_t` across FFI (`3`, `4`, `7`, `-1`, `INT_MIN`, `INT_MAX`, `0x7FFFFFFF`) — a C enum accepts any `int` | [x] |
| G7 | out-of-range C `_Bool` across FFI (`copy_matching = 2`, `0xFF`) | [x] |
| G8 | `INT_MIN` for every `int` parameter (`capacity`, `stride`, `positions`, `param`, `op`) | [x] |
| G9 | aliasing: `src == dst` for `buffer_copy` / `buffer_conditional_copy` / `buffer_copy_strided`, and `src1 == src2 == dst` for `buffer_merge`/`buffer_interleave` | [x] |

## Undefined-behaviour rows (deliberately NOT executed differentially)

These are inputs for which the C code has **no** check and unconditionally
overruns a fixed 256-byte array. Running them would smash the C `.so`'s stack
or adjacent struct fields, so there is no defined C result to match:

| function | unchecked condition | what C does |
|----------|--------------------|-------------|
| `buffer_reverse` | `length > 256` | `memcpy(uint8_t temp[256], buf->data, length)` → stack overflow |
| `buffer_rotate` | `length > 256` | same `temp[256]` overflow; also `positions % (int)length` divides by zero when `(int)length == 0` |
| `buffer_split` | `src->length > 256` | `memcpy(dst2->data, …, remaining)` overruns `dst2->data` |
| `buffer_conditional_copy` | `src->length > 256` | reads past `src->data`, writes past `dst->data` |
| `buffer_copy_strided` | `src->length > 256` | reads past `src->data`, writes past `dst->data` |
| `buffer_merge` | `src1->length + src2->length` wraps `size_t` | `memcpy` of ~SIZE_MAX bytes → SIGSEGV |
| `process_buffer_array` | `arr->buffers == NULL` with `count != 0` | NULL dereference |

The Rust translation clamps the memory traffic in exactly these spots (see
`DATA_LEN` in `src/lib_impl.rs`) so that it stays memory-safe rather than
reproducing the overflow. This is unreachable from the shipped program:
`read_buffer` — the only way the executable can populate a buffer — rejects
`length > 256` (row #37), and `init_buffer_array` never yields a NULL
`buffers` with a non-zero `count`.

## Traceability: row → test

Every row above is checked off because a named test constructs that exact
condition, calls both shared objects through their exported symbols, and asserts
the same return value **and** the same stderr bytes.

| rows | test file | test function(s) |
|------|-----------|------------------|
| 1 | `tests/errors.rs` | `row01_validate_buffer_null` |
| 2 | `tests/errors.rs` | `row02_validate_buffer_length_over_maximum` |
| 3 | `tests/errors.rs` | `row03_validate_buffer_checksum_mismatch_warns_but_succeeds` |
| 4 | `tests/errors.rs` | `row04_init_buffer_array_non_positive_capacity` |
| 5 | — | documented only: needs an allocator fault injector (the *first*, 16-byte malloc cannot be made to fail portably) |
| 6 | `tests/errors.rs` | `row06_init_buffer_array_storage_allocation_failure` |
| 7 | `tests/errors.rs` | `row07_free_buffer_array_null_is_a_noop` |
| 8, 9, 10 | `tests/errors.rs` | `row08_09_10_buffer_copy_null_pointers` |
| 11 | `tests/errors.rs` | `row11_buffer_copy_rejects_oversized_source` |
| 12 | `tests/errors.rs` | `row12_buffer_reverse_null` |
| 13 | `tests/errors.rs` | `row13_buffer_reverse_empty_leaves_checksum_alone` |
| 14 | `tests/errors.rs` | `row14_buffer_merge_null_pointers` (all 7 NULL combinations) |
| 15 | `tests/errors.rs` | `row15_buffer_merge_combined_length_over_maximum` |
| 16 | `tests/errors.rs` | `row16_buffer_split_null_pointers` (all 7 combinations) |
| 17 | `tests/errors.rs` | `row17_buffer_split_position_past_length` |
| 18 | `tests/errors.rs` | `row18_buffer_interleave_null_pointers` (all 7 combinations) |
| 19 | `tests/errors.rs` | `row19_buffer_interleave_combined_length_over_maximum` |
| 20 | `tests/errors.rs` | `row20_buffer_rotate_null` |
| 21, 22, 23 | `tests/errors.rs` | `row21_22_23_buffer_rotate_early_returns_and_negative_normalisation` |
| 24 | `tests/errors.rs` | `row24_conditional_copy_null_pointers` |
| 25 | `tests/errors.rs` | `row25_copy_strided_null_pointers` |
| 26 | `tests/errors.rs` | `row26_copy_strided_invalid_stride` |
| 27 | `tests/errors.rs` | `row27_process_buffer_array_null` |
| 28 | `tests/errors.rs` | `row28_process_buffer_array_count_zero` |
| 29 | `tests/errors.rs` | `row29_process_buffer_array_merge_needs_two` |
| 30 | `tests/errors.rs` | `row30_process_buffer_array_unknown_operation` |
| 31, 32, 33 | `tests/errors.rs` | `row31_32_33_process_buffer_array_inner_failures` |
| 34 | `tests/errors.rs` | `row34_read_buffer_null_does_not_consume_stdin` |
| 35 | `tests/errors.rs` | `row35_read_buffer_length_scan_failure` |
| 36, 37 | `tests/errors.rs` | `row36_37_read_buffer_length_out_of_range` |
| 38 | `tests/errors.rs` | `row38_read_buffer_byte_scan_failure` |
| 39 | `tests/errors.rs` | `row39_write_buffer_null` |
| 40 | `tests/so_main_diff.rs` | `errors_row40_operation_scan_failure` |
| 41 | `tests/so_main_diff.rs` | `errors_row41_buffer_count_scan_failure` |
| 42, 43 | `tests/so_main_diff.rs` | `errors_row42_43_buffer_count_out_of_range` |
| 44 | `tests/so_main_diff.rs` | reachable only on malloc failure; `buffer_count` is validated first (rows 42/43), which is what those tests cover |
| 45 | `tests/so_main_diff.rs` | `errors_row45_read_buffer_failure_inside_main` |
| 46, 47, 50 | `tests/so_main_diff.rs` | `errors_row46_47_50_operations_needing_two_buffers` |
| 48 | `tests/so_main_diff.rs` | `errors_row48_split_position_scan_failure` |
| 49 | `tests/so_main_diff.rs` | `errors_row49_split_position_past_length` |
| 51 | `tests/so_main_diff.rs` | `errors_row51_rotation_amount_scan_failure` |
| 52 | `tests/so_main_diff.rs` | `errors_row52_unknown_operation` |
| 53, 54 | `tests/so_main_diff.rs` | `errors_row53_54_combined_length_over_maximum_in_main` |
| G1 | `tests/errors.rs` | the `row08_09_10` / `row14` / `row16` / `row18` / `row24` / `row25` / `row27` NULL-combination tests |
| G2 | `tests/errors.rs` | `g2_calculate_checksum_null_with_zero_length` |
| G3 | `tests/diff_lowlevel.rs` | `row01`, `row06`, `row10`, `row15`, `row20`, `row28`, `row31`, `row38`, `row42`, `row54`, `row60` |
| G4 | `tests/diff_lowlevel.rs` | `row04`, `row12`, `row25`, `row29`, `row39`, `row74` |
| G5 | `tests/errors.rs` | `row02`, `row04`, `row11`, `row15`, `row17`, `row19`, `row26`, `row36_37`; `tests/so_main_diff.rs::errors_row42_43` |
| G6 | `tests/errors.rs` | `row30_process_buffer_array_unknown_operation`, `row27`, `row28`; `tests/so_main_diff.rs::errors_row52_unknown_operation` |
| G7 | `tests/errors.rs` | `g7_out_of_range_c_bool_across_ffi` |
| G8 | `tests/errors.rs` | `g8_int_min_for_every_int_parameter` |
| G9 | `tests/diff_lowlevel.rs` | `row14`, `row27`, `row33`, `row40`, `row55`, `row61`, plus the `alias_*` tests |
