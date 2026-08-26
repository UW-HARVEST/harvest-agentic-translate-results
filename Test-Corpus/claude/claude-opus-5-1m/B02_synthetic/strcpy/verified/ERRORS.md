# ERRORS.md — error-surface table

Mechanically derived from every `return` that reports a rejection / "not found"
sentinel, every explicit check and every constant limit in `c_src/src/lib.c` and
`c_src/src/main.c`. The C code contains **no** `assert`, no error enum and no
`errno` use; rejections are expressed purely as negative/zero return values
(library) or as a `stderr` message + `return 1` (driver).

## Library — `process_strings` (`c_src/src/lib.c`)

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| 1  | `process_strings` L57 | `input == NULL` (any operation, even an invalid one) | `-1` | `err_input_null_all_ops` |
| 2  | `process_strings` L65 | `operation == 0` and `reference == NULL` | `-2` | `err_reference_null_op0` |
| 3  | `process_strings` L77 | `operation == 2` and `reference == NULL` | `-2` | `err_reference_null_op2` |
| 4  | `process_strings` L90 | `operation == 4` and `reference == NULL` | `-2` | `err_reference_null_op4` |
| 5  | `process_strings` L95 | `operation` not in `{0,1,2,3,4}` (incl. negative, `INT_MIN`, `INT_MAX`, out-of-range enum values) | `-3` | `err_bad_operation` |
| 6  | `process_strings` L84 | `operation == 3` and (`reference == NULL` **or** `ref_len == 0`) — *not* an error return, but the fallback that silently substitutes `':'` for the delimiter | delimiter `':'` (result then per rows 10-13) | `err_op3_null_ref_defaults_colon` |
| 7  | `validate_token` L115 | token differs from `expected` and from `"VALID"` and from `"OK"` | `0` | `err_validate_token_no_match` |
| 8  | `parse_command` L150 | buffer matches none of `START/STOP/PAUSE/RESUME/RESET` (neither `strncmp`+terminator nor `strcmp`) and is not `"ADMIN"` | `-1` | `err_parse_command_no_match` |
| 9  | `compare_prefix` L181 | `exact_match != 0` (`flags & 1`) and string equals neither the prefix nor prefix+`_v1/_v2/_old/_new/_tmp` | `0` | `err_compare_prefix_exact_no_match` |
| 10 | `compare_prefix` L187 | `exact_match == 0` and `strncmp(str, prefix, strlen(prefix)) != 0` | `0` | `err_compare_prefix_loose_no_match` |
| 11 | `find_delimiter` L196 | `len == 0` (checked **before** anything is read, so `data` is never dereferenced) | `-1` | `err_find_delimiter_zero_len` |
| 12 | `find_delimiter` L211 | delimiter is `'|'`, not found in `data[0..len)`, and `strcmp(data,"NONE") == 0` | `-2` | `err_find_delimiter_none` |
| 13 | `find_delimiter` L215 | delimiter is `':'`, not found in `data[0..len)`, and `strcmp(data,"EMPTY") == 0` | `-3` | `err_find_delimiter_empty` |
| 14 | `find_delimiter` L218 | delimiter not found and neither special pattern matches (also the "stopped at NUL before `len`" path) | `-1` | `err_find_delimiter_not_found` |
| 15 | `match_pattern` L294 | no exact / wildcard / substring / case-insensitive match | `0` | `err_match_pattern_no_match` |
| 16 | `match_pattern` L250 | `case_sensitive != 0` (`flags & 2`) and `strlen(text) < strlen(pattern)`: `text_len - pattern_len` underflows, the loop scans forward without bound | reads past the buffer until SIGSEGV (or an accidental match) | `err_match_pattern_underflow` |

Notes derived from the source, needed to keep the rows honest:

* row 1 is checked **before** the `switch`, so `input == NULL` beats an invalid
  operation (`-1`, not `-3`);
* rows 2/3/4 are checked **inside** their case, so a NULL `reference` with
  `operation == 1` or `operation == 3` is *not* rejected (op 1 ignores
  `reference`; op 3 falls back to `':'` — row 6);
* `input_len` is only used by op 1 (`buf_size >= cmd_len`) and op 3 (`len`);
  ops 0/2/4 ignore it completely, so a bogus length is not an error there;
* `ref_len` is only used by op 3 (`ref_len > 0`);
* there is no upper bound / no length validation anywhere in `lib.c`.

## Driver — `main` (`c_src/src/main.c`)

| #  | check | trigger | expected C result | test (`tests/exe_diff.rs`) |
|----|-------|---------|-------------------|----------------------------|
| 17 | `scanf("%d", &operation) != 1` | empty input / non-numeric first token | `stderr: "Error reading operation\n"`, exit 1 | `exe_err_reading_operation` |
| 18 | `scanf("%u", &flags) != 1` | missing/non-numeric second token | `stderr: "Error reading flags\n"`, exit 1 | `exe_err_reading_flags` |
| 19 | `scanf("%zu", &input_len) != 1` | missing/non-numeric third token | `stderr: "Error reading input length\n"`, exit 1 | `exe_err_reading_input_length` |
| 20 | `input_len > MAX_BUFFER_SIZE` | `input_len` 1025 … `SIZE_MAX` (incl. `-1` → `SIZE_MAX`) | `stderr: "Error: input length %zu exceeds maximum 1024\n"`, exit 1 | `exe_err_input_length_too_big` |
| 21 | `scanf("%u", &byte) != 1` (input loop) | fewer input bytes than `input_len` | `stderr: "Error reading input byte %zu\n"` (index), exit 1 | `exe_err_reading_input_byte` |
| 22 | `scanf("%zu", &ref_len) != 1` | missing/non-numeric token after input bytes | `stderr: "Error reading reference length\n"`, exit 1 | `exe_err_reading_ref_length` |
| 23 | `ref_len > MAX_BUFFER_SIZE` | `ref_len` 1025 … `SIZE_MAX` | `stderr: "Error: reference length %zu exceeds maximum 1024\n"`, exit 1 | `exe_err_ref_length_too_big` |
| 24 | `scanf("%u", &byte) != 1` (reference loop) | fewer reference bytes than `ref_len` | `stderr: "Error reading reference byte %zu\n"` (index), exit 1 | `exe_err_reading_ref_byte` |

Boundary values that are **not** errors and must therefore be accepted
identically: `input_len == 1024`, `ref_len == 1024`, `input_len == 0`,
`ref_len == 0`, `operation`/`flags` overflowing their type
(`scanf` saturates with `strtol`/`strtoul` and then truncates, e.g.
`99999999999999` → `-1` for `%d`, `-1` → `4294967295` for `%u`).
Covered by `exe_accepts_maximum_lengths` and `exe_scanf_shapes`.

## Generic C API boundaries (not a distinct `return` in the C source)

| #  | boundary | tests (`tests/ffi_errors.rs`) |
|----|----------|-------------------------------|
| G1 | `input == NULL` / `reference == NULL` / both NULL, for every operation (valid and invalid) and every flag combination | `err_input_null_all_ops`, `err_null_reference_ops_that_allow_it` |
| G2 | zero lengths with non-NULL buffers, and empty (`""`) strings | `err_zero_length_non_null_buffers` |
| G3 | oversized / nonsense lengths: `1025`, `4096`, `1<<32`, `SIZE_MAX/2`, `SIZE_MAX` for `input_len` and `ref_len`, every operation | `err_oversized_lengths` |
| G4 | out-of-range "enum" values for `operation` crossing the FFI boundary: `-1`, `5`, `6`, `99`, `1000`, `0x10000`, `INT_MAX`, `INT_MIN`, and 500 random `int`s | `err_bad_operation`, `err_operation_one_past_range` |
| G5 | one step past every documented range: `operation` `-1`/`5`, `flags` bit 2 and above (must be ignored), `input_len` `1024`/`1025` | `err_operation_one_past_range`, `cfg_op2_flag_bit1_ignored`, `exe_err_input_length_too_big` |
| G6 | a non-NULL but unmapped `data` pointer with `len == 0` (the C code returns `-1` before dereferencing it) | `err_find_delimiter_zero_len` |

## Status

All 16 library rows, all 8 driver rows and all 6 generic boundaries have a
passing differential test (`cargo test`: 20 tests in `tests/ffi_errors.rs`,
15 in `tests/exe_diff.rs`).
