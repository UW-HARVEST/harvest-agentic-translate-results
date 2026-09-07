# ERRORS.md — error-surface table

Mechanically derived from every rejection / early-return / guard in
`c_src/src/lib.c`. There are no `assert`s in the C source and no error enums;
rejection is expressed with `return NULL`, `return -1`, `return 0`, bare
`return`, and `break`. Every `if` that guards an early exit gets a row, plus
every implicit rejection (a `switch` with no `default`).

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `create_state` | `malloc(sizeof(ProcessState))` returns NULL | prints `Error: Failed to allocate memory for state\n`, returns `NULL` |
| 2  | `create_state` | `capacity < 0` → `malloc((size_t)capacity)` is a huge sign-extended size and fails | prints `Error: Failed to allocate buffer\n`, `free(state)`, returns `NULL` |
| 3  | `create_state` | `capacity == INT_MIN` (extreme negative → `0xFFFFFFFF80000000`) | same as row 2: `NULL` |
| 4  | `create_state` | `capacity == INT_MAX` (2 GiB request; may succeed or fail, but must fail/succeed *identically* in C and Rust) | identical NULL-ness in both |
| 5  | `process_buffer` | `state == NULL` | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| 6  | `process_buffer` | `state != NULL` but `state->buffer == NULL` | prints `Error: Null pointer in process_buffer\n`, returns `-1` |
| 7  | `process_buffer` | `memchr` finds no occurrence → `found == NULL` → `break` | returns the count accumulated so far (`0` when never found) |
| 8  | `process_buffer` | `strlen(state->buffer) == 0` (empty buffer) → `remaining == 0`, loop never entered | returns `0`, prints nothing |
| 9  | `process_buffer` | `target == '\0'` — NUL is never inside the `strlen`-bounded region | returns `0`, prints nothing |
| 10 | `update_flags` | `state == NULL` | returns immediately, **prints nothing**, no side effect |
| 11 | `confuse_types` | `state == NULL` | returns `0`, prints nothing |
| 12 | `confuse_types` | `operation` not in `{0,1,2,3}` — `switch` has **no `default`** label (e.g. `4`, `5`, `-1`, `INT_MAX`, `INT_MIN`) | falls through the whole `switch`, returns `result == 0`, prints nothing, leaves `state->data` untouched |
| 13 | `destroy_state` | `state == NULL` | no-op, no output, no free |
| 14 | `destroy_state` | `state != NULL` but `state->buffer == NULL` | frees only `state`, no output |
| 15 | `confusion` | `create_state(param1, 128)` returns `NULL` | returns `-1` (unreachable in practice: capacity is the literal `128`) |
| 16 | `confusion` | `param3 < 0` → `param3 % 10` is negative in C99 (truncated division) → `search_char = '0' + negative`, a non-digit byte that is never present in the buffer | `found_count == 0`; **not** an error but the rejection-shaped path of `process_buffer` |
| 17 | `confusion` | `param4 < 0` and `param4 % 4 != 0` → `param4 % 4 ∈ {-1,-2,-3}` → hits the no-`default` `switch` (row 12) | `confusion_result == 0` |

## Generic FFI boundary cases (covered even though not distinct C branches)

| #  | function | trigger | expected C result |
|----|----------|---------|-------------------|
| G1 | `process_buffer` | NULL `state` (row 5) | `-1` |
| G2 | `update_flags` | NULL `state` (row 10) | no-op |
| G3 | `confuse_types` | NULL `state` (row 11) | `0` |
| G4 | `destroy_state` | NULL `state` (row 13) | no-op |
| G5 | `create_state` | `capacity == 0` → `malloc(0)` returns a non-NULL zero-size block; `snprintf(buf, 0, …)` writes **nothing** | non-NULL state, buffer untouched (contents indeterminate — see note) |
| G6 | `create_state` | `capacity == 1` → `snprintf` writes only the NUL terminator | non-NULL, `strlen(buffer) == 0` |
| G7 | `confuse_types` | out-of-range "enum" values one step past the valid range: `4`, `-1`, and the extremes `INT_MAX`, `INT_MIN` (a C `switch` on `int` accepts any `int`) | `0`, no output (row 12) |
| G8 | `process_buffer` | `target` with the high bit set (`char` is signed on x86-64; `memchr` compares as `unsigned char`), e.g. `-1` → `0xFF`, `-128` → `0x80` | `0` (not present in an ASCII buffer) |
| G9 | `create_state` | `initial_val` at the extremes `INT_MIN` / `INT_MAX` → longest `snprintf` output | identical buffer bytes |

Note on G5: with `capacity == 0` the C code leaves the 0-byte `malloc` block
uninitialised and unterminated, so any later `strlen(buffer)` is undefined
behaviour that reads heap bytes. The differential test therefore asserts only
on the *return value* of `create_state` (non-NULL in both) and does **not**
call `process_buffer` on a `capacity == 0` state, since neither implementation
has defined behaviour there.

## Verification record — every row has a passing differential test

| row | test in `tests/phase_c.rs` | result |
|-----|----------------------------|--------|
| 1  | `row01_state_alloc_failure_message_is_identical_in_both_binaries` | [x] |
| 2  | `row02_create_state_negative_capacity` (206 capacities) | [x] |
| 3  | `row03_create_state_capacity_int_min` | [x] |
| 4  | `row04_create_state_capacity_int_max` | [x] |
| 5  | `row05_process_buffer_null_state` — both return `-1` and print `Error: Null pointer in process_buffer\n` | [x] |
| 6  | `row06_process_buffer_null_buffer_field` — same, via a caller-built state | [x] |
| 7  | `row07_process_buffer_target_never_found` | [x] |
| 8  | `row08_process_buffer_empty_buffer` | [x] |
| 9  | `row09_process_buffer_nul_target_never_matches` | [x] |
| 10 | `row10_update_flags_null_state` — both silent, no side effect | [x] |
| 11 | `row11_confuse_types_null_state` — both return `0`, both silent | [x] |
| 12 | `row12_confuse_types_operation_out_of_range` (313 ops x 5 states) — both return `0`, print nothing, leave `data` untouched | [x] |
| 13 | `row13_destroy_state_null` | [x] |
| 14 | `row14_destroy_state_null_buffer_field` | [x] |
| 15 | `row15_confusion_create_state_failure_is_unreachable_but_consistent` | [x] |
| 16 | `row16_confusion_negative_param3_gives_non_digit_search_byte` | [x] |
| 17 | `row17_confusion_negative_param4_hits_no_switch_arm` | [x] |
| G1 | `row05_process_buffer_null_state` | [x] |
| G2 | `row10_update_flags_null_state` | [x] |
| G3 | `row11_confuse_types_null_state` | [x] |
| G4 | `row13_destroy_state_null` | [x] |
| G5 | `g05_create_state_capacity_zero` | [x] |
| G6 | `g06_create_state_capacity_one` | [x] |
| G7 | `row12_…` + `deep_op_out_of_range_wide` (9 out-of-range ops x 2,000 data words each) | [x] |
| G8 | `row15_process_buffer_high_bit_target` (Phase B) + `deep_process_buffer_wide` (all 256 targets) | [x] |
| G9 | `g09_create_state_extreme_initial_val` | [x] |

`extra_out_of_range_everything` additionally runs the full 9^4 = 6,561 cross
product of `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` through
`confusion`, comparing return value and stdout.

Note on row 1: no argument can make `malloc(sizeof(ProcessState))` fail —
the size is a fixed 24 bytes and both libraries call the identical libc
`malloc`. The row's testable content is that both `.so`s ship the identical
diagnostic string (asserted against the shipped binaries) and agree on the
struct layout the size derives from (asserted by `tests/interop.rs`, which
drives a state allocated by one `.so` through the other `.so`'s functions).

Both compilers rewrite `printf("literal\n")` with no conversions into
`puts("literal")`, so the stored literal has no trailing newline in either
binary — the runtime bytes are still identical, as the stdout comparisons in
rows 5, 6, 2 and 3 confirm.
