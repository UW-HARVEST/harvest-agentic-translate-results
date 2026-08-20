# ERRORS.md — error-surface table (Phase C)

Mechanically derived from every rejection / early-return / error-print branch in
`c_src/src/lib.c`. There are no `assert`s, no error enums and no named
min/max constants in the C source; the complete rejection surface is the set of
`if (… == NULL)` guards, the `switch` fall-through, the loop-exit conditions and
the implementation-defined float→int conversion.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `create_state` | `malloc(sizeof(ProcessState))` returns `NULL` (line 60) — heap exhausted | prints `Error: Failed to allocate memory for state\n`, returns `NULL` | `err_01_create_state_state_malloc_fails` (forked child, `RLIMIT_AS` + heap exhaustion) | [x] |
| 2 | `create_state` | `malloc(capacity)` returns `NULL` (line 78) because `capacity < 0` sign-extends to a huge `size_t` (e.g. `-1` → `SIZE_MAX`) | prints `Error: Failed to allocate buffer\n`, `free(state)`, returns `NULL` | `err_02_create_state_negative_capacity` | [x] |
| 3 | `create_state` | `malloc(capacity)` returns `NULL` for a positive-but-unsatisfiable `capacity` (`INT_MAX`-ish) | same as #2 (or success — must agree with C either way) | `err_03_create_state_huge_capacity` | [x] |
| 4 | `destroy_state` | `state == NULL` (line 91) | no-op, no output, no crash | `err_04_destroy_state_null` | [x] |
| 5 | `destroy_state` | `state != NULL` but `state->buffer == NULL` (line 92) | skips `free(buffer)`, frees `state` only, no output | `err_05_destroy_state_null_buffer` | [x] |
| 6 | `process_buffer` | `state == NULL` (line 100, first disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` | `err_06_process_buffer_null_state` | [x] |
| 7 | `process_buffer` | `state != NULL` and `state->buffer == NULL` (line 100, second disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` | `err_07_process_buffer_null_buffer` | [x] |
| 8 | `process_buffer` | `strlen(buffer) == 0` → `remaining == 0`, loop body never runs (line 109) | returns `0`, no `Operation:` output | `err_08_process_buffer_empty` | [x] |
| 9 | `process_buffer` | `memchr` returns `NULL` → `break` (lines 110–114): target absent from the first `strlen` bytes | returns the count accumulated so far (`0` when never present) | `err_09_process_buffer_no_match` | [x] |
| 10 | `process_buffer` | `target == '\0'`: the NUL terminator is *outside* the `remaining = strlen()` window, so `memchr` can never match | returns `0` | `err_10_process_buffer_nul_target` | [x] |
| 11 | `update_flags` | `state == NULL` (line 127) | returns immediately, **no** `Debug:` / `Bit fields` output | `err_11_update_flags_null` | [x] |
| 12 | `confuse_types` | `state == NULL` (line 144) | returns `0`, no output | `err_12_confuse_types_null` | [x] |
| 13 | `confuse_types` | `operation` matches no `case` (line 150): any value `< 0` or `> 3`, incl. out-of-range "enum-like" ints such as `-1`, `4`, `INT_MIN`, `INT_MAX` | `result` stays `0`, no output, returns `0` | `err_13_confuse_types_out_of_range_operation` | [x] |
| 14 | `confuse_types` | `operation == 1` with a `float_val` whose `* 100.0f` product is NaN / ±Inf / outside `[INT_MIN, INT_MAX]` — the `(int)` cast is UB; gcc x86-64 emits `cvttss2si`, yielding the *integer indefinite* value `INT_MIN` | prints `Read as float: …` (`nan` / `inf` / `-inf` / huge) and returns `INT_MIN` | `err_14_confuse_types_float_cast_out_of_range` | [x] |
| 15 | `confusion` | `create_state(param1, 128)` returns `NULL` (line 187) — only reachable when the 24-byte `malloc` fails, since `capacity` is hard-coded `128` | returns `-1` | `err_15_confusion_create_state_null` (forked child, heap exhausted) | [x] |
| 16 | generic FFI boundary | `process_buffer` `target` byte with the high bit set (`0x80`–`0xFF`), i.e. a *negative* `char` promoted to `int` — `memchr` compares as `unsigned char` | same count as the unsigned-char interpretation | `err_16_process_buffer_high_bit_target` | [x] |
| 17 | generic FFI boundary | `create_state(_, 0)` — zero-length buffer: `malloc(0)` returns a non-`NULL` unique pointer and `snprintf(buf, 0, …)` writes nothing | returns non-`NULL` state with `capacity == 0` and an untouched buffer | `err_17_create_state_zero_capacity` | [x] |
| 18 | generic FFI boundary | `create_state(_, 1)` / tiny capacities — `snprintf` truncation at the buffer boundary | buffer holds the truncated, always-NUL-terminated prefix | `err_18_create_state_truncating_capacity` | [x] |
| 19 | generic FFI boundary | `confusion` with `param3 < 0` → `param3 % 10 < 0` → `search_char = '0' + negative` (a byte *below* `'0'`), and `param4 < 0` → `param4 % 4 < 0` → no `switch` case | well-defined: count of that byte, `confuse_types` contributes `0` | `err_19_confusion_negative_params` | [x] |
| 20 | generic FFI boundary | `INT_MIN` / `INT_MAX` in every `int` parameter of `confusion` (one step past every documented range, incl. `INT_MIN % 10` and `INT_MIN % 4`) | identical `int` result, including the signed-overflow wrap-around of the final `result +=` chain | `err_20_confusion_extreme_params` | [x] |
