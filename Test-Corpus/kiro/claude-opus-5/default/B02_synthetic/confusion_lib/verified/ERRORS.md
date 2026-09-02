# ERRORS.md — Error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return NULL`, `return -1`,
early `return`, guarded branch, `switch` fallthrough, loop-break condition, and
every bit-width / modulus clamp in the C source gets one row.

There are no `assert`s, no error enums, and no `errno` use in the C source. The
complete set of rejection/limit sites found by grep is:

```
lib.c:60   if (state == NULL) { printf("Error: Failed to allocate memory for state\n"); return NULL; }
lib.c:78   if (state->buffer == NULL) { printf("Error: Failed to allocate buffer\n"); free(state); return NULL; }
lib.c:90   if (state != NULL) {            // destroy_state outer guard
lib.c:91       if (state->buffer != NULL) {// destroy_state inner guard
lib.c:99   if (state == NULL || state->buffer == NULL) { printf("Error: Null pointer in process_buffer\n"); return -1; }
lib.c:111      if (found == NULL) break;   // memchr miss
lib.c:126  if (state == NULL) return;      // update_flags
lib.c:143  if (state == NULL) return 0;    // confuse_types
lib.c:149  switch (operation) { case 0..3 } // no default -> falls through, result stays 0
lib.c:184  if (state == NULL) return -1;   // confusion
```

Clamps / magic limits: `& 0x1F` (counter), `& 0x7` (mode), `>> 3`,
bit-field widths 1/1/1/5/3/5/16, `capacity = 128` (hard-coded in `confusion`),
`param3 % 10`, `param4 % 4`, magic constant `1078530011`.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `create_state` | `malloc(sizeof(ProcessState))` (24 bytes) returns NULL | prints `Error: Failed to allocate memory for state`, returns `NULL` | [x] documented-unreachable |
| 2  | `create_state` | `capacity < 0` → `malloc((size_t)(int)capacity)` sign-extends to a huge `size_t`, allocation fails | prints `Error: Failed to allocate buffer`, `free`s state, returns `NULL` | [x] |
| 3  | `create_state` | `capacity == INT_MIN` (extreme negative, same path as #2) | prints `Error: Failed to allocate buffer`, returns `NULL` | [x] |
| 4  | `create_state` | `capacity == INT_MAX` (2147483647) — huge but non-negative request | allocation-dependent; C and Rust must agree (both NULL, or both non-NULL with identical buffer contents) | [x] |
| 5  | `create_state` | `capacity == 0` → `malloc(0)` returns a non-NULL minimal block; `snprintf(buf, 0, ...)` writes **nothing** | returns non-`NULL`; buffer left **uninitialized** (indeterminate) | [x] non-NULL parity only; contents are indeterminate and deliberately not byte-compared |
| 6  | `create_state` | `0 < capacity <= 16` → `snprintf` truncates `"State:%d:Mode:%d"` | returns non-`NULL`; buffer holds truncated NUL-terminated prefix | [x] |
| 7  | `destroy_state` | `state == NULL` | no-op, no output, no crash | [x] |
| 8  | `destroy_state` | `state->buffer == NULL` (state built by hand with a NULL buffer) | frees only `state`, no crash | [x] |
| 9  | `process_buffer` | `state == NULL` | prints `Error: Null pointer in process_buffer`, returns `-1` | [x] |
| 10 | `process_buffer` | `state != NULL` but `state->buffer == NULL` | prints `Error: Null pointer in process_buffer`, returns `-1` | [x] |
| 11 | `process_buffer` | `target` absent from buffer (`memchr` returns NULL on the first iteration) | `break` with `count == 0`, returns `0`, no `Operation:` lines | [x] |
| 12 | `process_buffer` | `target == '\0'` — the terminator is outside `strlen`'s span, so it is never found | returns `0` | [x] |
| 13 | `process_buffer` | `target` with the high bit set (negative `char`, e.g. `0x80`, `0xFF`); `memchr` compares as `unsigned char` | returns `0` for an ASCII buffer; must not match by sign-extension | [x] |
| 14 | `process_buffer` | empty buffer (`strlen == 0` ⇒ `remaining == 0`, loop body never entered) | returns `0` | [x] |
| 15 | `update_flags` | `state == NULL` | returns immediately, **no** `Debug:`/`Bit fields` output | [x] |
| 16 | `update_flags` | `param < 0` — `param >> 3` is an *arithmetic* (sign-propagating) shift, then `& 0x7` | `mode` = low 3 bits of the arithmetic shift; no error | [x] |
| 17 | `update_flags` | `param == INT_MIN` (shift of the most negative value) | `flag1..3 = 0`, `mode = (INT_MIN>>3)&7 = 0` | [x] |
| 18 | `update_flags` | called 32+ times — `counter` overflows its 5-bit field, `(counter+1) & 0x1F` wraps | `counter` wraps `31 → 0`; no error | [x] |
| 19 | `confuse_types` | `state == NULL` | returns `0`, no output | [x] |
| 20 | `confuse_types` | `operation` outside `{0,1,2,3}` — e.g. `4`, `5`, `100`, `INT_MAX`: no `case`, no `default` | returns `0`, **no output at all** | [x] |
| 21 | `confuse_types` | `operation` negative — e.g. `-1`, `-4`, `INT_MIN`: no `case` matches | returns `0`, no output | [x] |
| 22 | `confuse_types` | `operation == 1` with a bit pattern that reinterprets as NaN / ±Inf / out-of-`int`-range float; `(int)(f*100)` is UB | gcc emits `cvttss2si` ⇒ `INT_MIN` (`-2147483648`) for NaN/Inf/overflow; `printf("%f")` renders `nan`/`inf` | [x] |
| 23 | `confuse_types` | `operation == 3` with negative bytes — `char` is **signed** on x86-64 Linux | `bytes[0] + bytes[1]` sums sign-extended values (can be negative) | [x] |
| 24 | `confusion` | `create_state(param1, 128)` returns NULL | returns `-1` | [x] documented-unreachable (capacity is the constant 128) |
| 25 | `confusion` | `param3 < 0` → `param3 % 10` is negative (C truncating division) → `search_char = '0' + negative`, a non-digit | search finds nothing, `found_count == 0` | [x] |
| 26 | `confusion` | `param3 == INT_MIN` → `INT_MIN % 10 == -8` → `search_char == '('` | `found_count == 0` | [x] |
| 27 | `confusion` | `param4 < 0` → `param4 % 4` is negative (`-1`,`-2`,`-3`) → **no switch case matches** | `confusion_result == 0`, no `Set as`/`Read as` line printed | [x] |
| 28 | `confusion` | `param1 == INT_MIN` / `INT_MAX` — extreme value formatted into the buffer and reinterpreted as float | no error; result must match bit-for-bit | [x] |
| 29 | `confusion` | signed-`int` overflow while accumulating `result` (`found_count*10 + confusion_result + counter*5 + mode*3`) | gcc wraps two's-complement; Rust must use wrapping arithmetic | [x] |
| 30 | `confusion` | `search_char` computation overflows `char` range before the narrowing conversion | implementation-defined narrowing = truncation to low 8 bits | [x] |

## Verification result

Every row above has a differential test in `tests/phase_c_errors.rs` that
constructs the exact condition, calls BOTH `.so`s, and asserts the same
sentinel/error value **and** the same printed diagnostics. 35 tests, all
passing under both codegen profiles.

Row → test mapping:

| rows | test |
|------|------|
| 1, 24 | `row01_row24_state_malloc_failure_is_unreachable` |
| 2 | `row02_create_state_negative_capacity_returns_null` |
| 3 | `row03_create_state_capacity_int_min` |
| 4 | `row04_create_state_capacity_int_max` |
| 5 | `row05_create_state_capacity_zero` |
| 6 | `row06_create_state_capacity_truncating` |
| 7 | `row07_destroy_state_null_is_noop` |
| 8 | `row08_destroy_state_null_buffer` |
| 9 | `row09_process_buffer_null_state_returns_minus_one` |
| 10 | `row10_process_buffer_null_buffer_returns_minus_one` |
| 11 | `row11_process_buffer_target_absent` |
| 12 | `row12_process_buffer_nul_target_never_found` |
| 13 | `row13_process_buffer_high_bit_target` |
| 14 | `row14_process_buffer_empty_buffer` |
| 15 | `row15_update_flags_null_state_is_silent_noop` |
| 16 | `row16_update_flags_negative_param_arithmetic_shift` |
| 17 | `row17_update_flags_param_int_min` |
| 18 | `row18_update_flags_counter_wraps_at_32` |
| 19 | `row19_confuse_types_null_state_returns_zero` |
| 20 | `row20_confuse_types_operation_out_of_range_positive` |
| 21 | `row21_confuse_types_operation_negative` |
| 22 | `row22_confuse_types_op1_float_specials` |
| 23 | `row23_confuse_types_op3_signed_bytes` |
| 25 | `row25_confusion_negative_param3_non_digit_search_char` |
| 26 | `row26_confusion_param3_int_min` |
| 27 | `row27_confusion_negative_param4_matches_no_switch_case` |
| 28 | `row28_confusion_param1_extremes` |
| 29 | `row29_confusion_result_overflow_wraps` |
| 30 | `row30_confusion_search_char_narrowing` |
| generic boundaries | `generic_all_entry_points_with_null_pointers`, `generic_one_past_valid_ranges` |
| extra stress | `process_buffer_randomized_arbitrary_buffers`, `update_flags_preserves_unrelated_bitfields`, `confuse_types_operation_window_randomized` |
| negative control | `harness_detects_divergence` |

Rows 1 and 24 are the only rows that cannot be triggered through the public
API: `create_state` always requests a fixed 24 bytes for the state, and
`confusion` always passes the literal capacity `128`. They are asserted as
"both implementations agree that this succeeds" rather than faked.

### Notes on ABI facts confirmed against gcc, not assumed

- `(int)float` for NaN / ±Inf / out-of-range values yields `INT_MIN`
  (`cvttss2si` integer-indefinite), **not** a saturating cast. Rust's `as`
  saturates, so the translation needs the explicit `f32_to_c_int_trunc` helper.
- `char` is **signed** on x86-64 Linux, so `bytes[i]` sign-extends.
- Bit-field storage: `flag1`=bit 0, `flag2`=1, `flag3`=2, `counter`=3..7,
  `mode`=8..10, `status`=11..15, `reserved`=16..31 in one 4-byte unit.
- `%` in C truncates toward zero, matching Rust's `%` (not `rem_euclid`).
