# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return` that is
not the normal success path, every null check, every implicit range/type
narrowing that can reject or clamp, and every `switch` that has no `default`.
There are no `assert`s and no error enums in this library; rejection is
expressed as `NULL` returns, `-1` / `0` sentinels, and silent no-ops.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `create_state` | `malloc(sizeof(ProcessState))` returns `NULL` (line 60) | prints `Error: Failed to allocate memory for state\n`, returns `NULL`. Not reachably testable (24-byte malloc); documented for completeness. |
| 2  | `create_state` | `capacity < 0` → `malloc(capacity)` converts the negative `int` to a huge `size_t` and fails (line 78) | prints `Error: Failed to allocate buffer\n`, frees `state`, returns `NULL` |
| 3  | `create_state` | `capacity == 0` → `malloc(0)` succeeds (glibc returns a unique non-NULL ptr), `snprintf(buf, 0, ...)` writes **nothing** | returns a **non-NULL** state whose `buffer` is a valid but *uninitialised* 0-byte block. Must NOT be rejected. |
| 4  | `create_state` | `0 < capacity < strlen("State:%d:Mode:3")` → `snprintf` truncates | returns non-NULL; buffer holds `capacity-1` chars + `NUL`. Must NOT be rejected. |
| 5  | `create_state` | `capacity == INT_MIN` (worst-case negative) | same as row 2: `NULL` |
| 6  | `destroy_state` | `state == NULL` (line 91) | no-op, no output, no crash |
| 7  | `destroy_state` | `state != NULL` but `state->buffer == NULL` (line 92) | skips `free(buffer)`, frees `state` only; no output |
| 8  | `process_buffer` | `state == NULL` (line 100, first disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| 9  | `process_buffer` | `state != NULL && state->buffer == NULL` (line 100, second disjunct) | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| 10 | `process_buffer` | `state->buffer` points to an empty string (`strlen == 0`) → `while (remaining > 0)` never entered | returns `0`, no `Operation:` lines |
| 11 | `process_buffer` | `target == '\0'` → `memchr` looks for `NUL` inside the first `strlen` bytes, never finds it | returns `0`, no `Operation:` lines |
| 12 | `process_buffer` | `target` is a *negative* `char` (0x80..0xFF, e.g. `(char)-1`) — `memchr` compares as `unsigned char`, so it matches the corresponding high byte, not "nothing" | count of matching high bytes (may be `> 0`); must match C exactly |
| 13 | `process_buffer` | `target` not present in buffer at all | returns `0` (loop `break`s on first `memchr == NULL`) |
| 14 | `update_flags` | `state == NULL` (line 127) | early `return`, no output, no crash |
| 15 | `update_flags` | `param` negative (e.g. `INT_MIN`) → `param >> 3` is an *arithmetic* shift of a negative value | `mode = (param >> 3) & 7`; no rejection |
| 16 | `update_flags` | `counter` already at its 5-bit max `31` → `(31 + 1) & 0x1F` | wraps to `0`; no rejection |
| 17 | `confuse_types` | `state == NULL` (line 144) | returns `0`, no output |
| 18 | `confuse_types` | `operation` outside `{0,1,2,3}` — `switch` has **no `default`** (lines 150-172). Includes out-of-range "enum-like" ints: `4`, `5`, `-1`, `-4`, `INT_MIN`, `INT_MAX` | returns `0`, **no output at all** |
| 19 | `confuse_types` | `operation == 1` and the union bits are a NaN / ±Inf / out-of-`int`-range float, so `(int)(float_val * 100)` is UB (x86-64 `cvttss2si` → "integer indefinite" `INT_MIN`) | prints `Read as float: %f` (`nan`/`inf`/`-inf`), returns `INT_MIN` (0x80000000) |
| 20 | `confuse_types` | `operation == 1` and `float_val * 100` overflows `float` range but not `int`… / underflows to `0` | truncation toward zero; result must match C bit-for-bit |
| 21 | `confuse_types` | `operation == 3` and `bytes[0] + bytes[1]` — `char` is *signed* on x86-64, so the sum can be negative | negative `int` result; must match C |
| 22 | `confusion` | `create_state(param1, 128)` returns `NULL` (line 187) | returns `-1`. Not reachable via the public API (128 always allocates); documented for completeness. |
| 23 | `confusion` | `param3` negative → `param3 % 10` is negative in C (truncated division), so `search_char = '0' + negative` is a **non-digit** byte (e.g. `param3 = -5` → `'+'`) | search char is that byte; no rejection. Must match C. |
| 24 | `confusion` | `param3 == INT_MIN` → `INT_MIN % 10 == -8` → `search_char = '0' - 8 = '('` | no rejection; must match C |
| 25 | `confusion` | `param4` negative → `param4 % 4` ∈ `{-1,-2,-3,0}`, so a negative selector reaches `confuse_types` and hits **no case** | `confuse_types` returns `0` silently (see row 18) |
| 26 | `confusion` | `param4 == INT_MIN` → `INT_MIN % 4 == 0` → case `0` | takes case `0`; must match C |
| 27 | `confusion` | `result += confusion_result` overflows `int` (signed-overflow UB; `-O0` wraps two's-complement), e.g. `param1` bits give a huge float and `param4 % 4 == 1` | wrapped two's-complement `int`; Rust must use wrapping arithmetic |
| 28 | `confusion` | `param1 == INT_MIN` / `INT_MAX` (extremes of the value written into the union) | full pipeline must match C |

## Boundary cases covered beyond the table

* NULL pointer passed to every pointer-taking export (`destroy_state`,
  `process_buffer`, `update_flags`, `confuse_types`) — rows 6, 8, 14, 17.
* Zero length: `create_state(_, 0)` (row 3), empty buffer in `process_buffer`
  (row 10).
* Oversized length: `create_state(_, INT_MIN)` and negative capacities (rows 2, 5).
* One step past a valid range: `confuse_types(state, 4)` and `(state, -1)`
  (row 18) — the C `switch` accepts any `int`, so these are real inputs.
* Out-of-range enum-like values across FFI: `confuse_types` operation
  `{4, 5, 100, -1, -2, INT_MIN, INT_MAX}` (row 18).

## Row → test mapping (all rows PASS)

| row(s) | test (`translation/tests/phase_c_errors.rs`) | status |
|--------|---------------------------------------------|--------|
| 1  | `row01_state_malloc_failure_unreachable_documented` | [x] accounted for (branch unreachable: a 24-byte `malloc` cannot be made to fail from a test; the sibling failure branch is covered by row 2) |
| 2, 5 | `row02_row05_create_state_negative_capacity_returns_null` | [x] both return `NULL` + identical `Error: Failed to allocate buffer\n` |
| 3  | `row03_create_state_capacity_zero_is_accepted` | [x] |
| 4  | `row04_create_state_truncation_is_accepted` | [x] |
| 6  | `row06_destroy_state_null_is_noop` | [x] |
| 7  | `row07_destroy_state_null_buffer` | [x] |
| 8  | `row08_process_buffer_null_state` | [x] both `-1` + identical message |
| 9  | `row09_process_buffer_null_buffer` | [x] both `-1` + identical message |
| 10 | `row10_process_buffer_empty_buffer_returns_zero` | [x] |
| 11 | `row11_process_buffer_nul_target_never_matches` | [x] |
| 12 | `row12_process_buffer_negative_char_target` | [x] |
| 13 | `row13_process_buffer_target_absent` | [x] |
| 14 | `row14_update_flags_null_state_is_noop` | [x] |
| 15 | `row15_update_flags_negative_param_arithmetic_shift` | [x] |
| 16 | `row16_update_flags_counter_at_max_wraps` | [x] (all 32 counter start values) |
| 17 | `row17_confuse_types_null_state_returns_zero` | [x] |
| 18 | `row18_confuse_types_out_of_range_operation` + `extra_confuse_types_one_past_valid_range` | [x] out-of-range enum ints return `0`, print nothing, leave the union unmodified |
| 19, 20 | `row19_row20_confuse_types_float_cast_undefined_range` | [x] full 512-point exponent sweep + NaN/Inf/FLT_MAX; both give `INT_MIN` for the out-of-range `cvttss2si` |
| 21 | `row21_confuse_types_signed_char_byte_sum` | [x] |
| 22 | `row22_confusion_create_state_failure_unreachable_documented` | [x] accounted for (`confusion` always passes capacity 128, so `state == NULL` is unreachable; the equivalent branch is covered on `create_state` by row 2) |
| 23, 24 | `row23_row24_confusion_negative_param3_nondigit_search_char` | [x] |
| 25, 26 | `row25_row26_confusion_negative_and_extreme_param4` | [x] |
| 27 | `row27_confusion_result_overflow_wraps` | [x] |
| 28 | `row28_confusion_param1_extremes` | [x] |
| extra | `extra_create_state_oversized_capacity` | [x] huge positive capacities (`INT_MAX`, 1 GiB, …) agree |

**26/26 error-path tests pass.**
