# ERRORS.md — Phase A error-surface table

Derived mechanically from every rejection / early-out / error return in
`c_src/src/lib.c`. There are no `assert`s, no error enums and no `RETURN_ERROR`
macros in this library; the rejection vocabulary is exactly: `return NULL`,
`return -1`, `return 0` (as a sentinel), the `default:` arm of a `switch`, and
the two NULL guards in `destroy_buffer`.

Line numbers refer to `c_src/src/lib.c`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `create_buffer` | `malloc(sizeof(StringBuffer))` fails (line 36 `if (!buffer)`) | returns `NULL` |
| 2  | `create_buffer` | `malloc(initial_capacity)` fails (line 41 `if (!buffer->data)`); reachable with `initial_capacity < 0`, which sign-extends to a huge `size_t` — e.g. `-1`, `-4096`, `INT_MIN` | frees the header, returns `NULL` |
| 3  | `create_buffer` | `initial_capacity` huge-but-positive so `malloc` fails (`INT_MAX`) | returns `NULL` (glibc may succeed with overcommit; both libs share the same allocator so both agree) |
| 4  | `create_buffer` | `initial_capacity == 0` — **not** rejected; `malloc(0)` returns a non-NULL minimal chunk, then line 48 writes `data[0] = '\0'` 1 byte out of bounds | returns non-`NULL`, `capacity == 0`, `length == 0` |
| 5  | `append_to_buffer` | `realloc` fails (line 61 `if (!new_data)`); reachable by making `required_capacity * 2` wrap negative, e.g. `length = INT_MAX - 4` / `INT_MAX - 1` / `2^30`, so the doubled value sign-extends to a ~2^64 request | returns `-1`, `buffer->data` / `capacity` / `length` left unchanged |
| 6  | `append_to_buffer` | `buffer == NULL` — **no** NULL check; line 55 dereferences it | SIGSEGV (identical fault in both libs; exercised out-of-process) |
| 7  | `append_to_buffer` | `str == NULL` — **no** NULL check; `strlen(NULL)` at line 54 | SIGSEGV (identical fault in both libs; exercised out-of-process) |
| 8  | `append_to_buffer` | `required_capacity <= buffer->capacity` (no growth) with an empty `str` (`str_len == 0`) | returns `0`, `length` unchanged, no realloc |
| 9  | `destroy_buffer` | `buffer == NULL` (line 76) | no-op, returns cleanly |
| 10 | `destroy_buffer` | `buffer != NULL` but `buffer->data == NULL` (line 77) | skips `free(data)`, frees the header only |
| 11 | `get_operation_name` | `op_code` outside `0..=3` — the `default:` arm (line 90). Includes every negative value (`-1`, `-2`, `-3` are what `x % 4` yields for negative `x`), `4`, `INT_MAX`, `INT_MIN`. This is the out-of-range-enum case across the FFI boundary. | returns the string `"unknown"` |
| 12 | `perform_operation` | `operation` matches none of `"add"`/`"subtract"`/`"multiply"`/`"divide"` — falls through to line 107. Includes `"unknown"` (what `get_operation_name` returns for out-of-range codes), `""`, `"ADD"`, `"add "`, `"addx"`, arbitrary bytes. | returns `0` |
| 13 | `perform_operation` | `operation == "divide"` and `b == 0` (line 102 guard fails) | returns `0` (division not attempted) |
| 14 | `perform_operation` | `operation == "divide"`, `a == INT_MIN`, `b == -1` — **not** guarded; `a / b` overflows | UB in C; compiles to `idiv` and raises SIGFPE. Rust reproduces with raw `idiv`. Exercised out-of-process comparing the signal. |
| 15 | `perform_operation` | `operation == NULL` — `strcmp(NULL, "add")` at line 95 | SIGSEGV (identical fault in both libs; exercised out-of-process) |
| 16 | `perform_operation` | signed-overflow arithmetic that is UB but must match: `"add"` with `INT_MAX,1`; `"subtract"` with `INT_MIN,1`; `"multiply"` with `INT_MIN,-1` and `INT_MAX,INT_MAX` | wraps two's-complement (`-2147483648`, `2147483647`, `-2147483648`, `1`) |
| 17 | `buffapp` | `create_buffer(32)` returns `NULL` — **not** checked; line 116 `log_buffer->length = 0` dereferences NULL | SIGSEGV. Unreachable in practice (32-byte malloc); documented, not tested. |
| 18 | `buffapp` | `intermediate3 == 0` (line 141 else branch) — e.g. any params where `param1 % 4` or `param3 % 4` selects `"unknown"`, or where either operand product is 0 | `result = param1 + param2 + param3 + param4` (wrapping) instead of `result / intermediate3` |
| 19 | `buffapp` | `result / intermediate3` at line 142 with `result == INT_MIN, intermediate3 == -1` | Unreachable: `intermediate3 == i1*i2 == -1` forces `{i1,i2} = {1,-1}` hence `result == i1+i2 == 0`. Documented; asserted unreachable by exhaustive reasoning + randomized search. |
| 20 | `append_to_buffer` | `buffer->length == INT_MAX`: `required_capacity` wraps to `INT_MIN`, which is **not** `> capacity`, so line 57's growth guard is bypassed entirely and line 69 runs `strcpy(data + INT_MAX, str)` | SIGSEGV (wild write ~2 GiB past the allocation; identical fault in both libs, exercised out-of-process) |
| 21 | `append_to_buffer` | `buffer->length` large negative (e.g. `INT_MIN / 2`): `required_capacity` stays negative, again bypassing the growth guard, and line 69 writes ~1 GiB *below* `data` | SIGSEGV (identical fault in both libs, exercised out-of-process) |
| 5b | `append_to_buffer` | `capacity == -1, length == -1, str == ""`: `required_capacity == 0 > -1`, so `new_capacity == 0` and the code calls `realloc(ptr, 0)` | glibc frees the block and returns `NULL`, so the function returns `-1` and leaves `data` dangling. Both libraries call the same glibc `realloc`, so the outcome is identical whatever glibc chooses. |

## Generic FFI boundary cases also covered (Phase C)

* NULL pointers for every pointer parameter (rows 6, 7, 9, 10, 15).
* Zero lengths / zero capacity (rows 4, 8).
* Oversized / negative lengths (rows 2, 3, 5).
* One step past a valid range: `get_operation_name(4)` and `get_operation_name(-1)`
  (row 11) — the "enum" accepts any `int`.
* Non-NUL-terminated / adversarial operation strings (row 12).

## Phase C status — every row has a passing differential test

Test file: `translation/tests/errors.rs` (`cargo test --test errors`). The two
rows that exercise `buffapp` live in `translation/tests/stdout_diff.rs`
(`cargo test --test stdout_diff`) because they capture stdout and must run
single-threaded.

| row | test | [x] |
|-----|------|-----|
| 1  | `row01_create_buffer_header_alloc_failure_shares_null_contract` | [x] |
| 2  | `row02_create_buffer_negative_capacity_returns_null` | [x] |
| 3  | `row03_create_buffer_huge_positive_capacity` | [x] |
| 4  | `row04_create_buffer_zero_capacity_not_rejected` | [x] |
| 5  | `row05_append_realloc_failure_returns_minus_one` | [x] |
| 5b | `row05b_append_realloc_zero_size_request` | [x] |
| 6  | `row06_append_null_buffer_faults_identically` (out-of-process, SIGSEGV) | [x] |
| 7  | `row07_append_null_str_faults_identically` (out-of-process, SIGSEGV) | [x] |
| 8  | `row08_append_empty_string_no_growth` | [x] |
| 9  | `row09_destroy_buffer_null_is_noop` | [x] |
| 10 | `row10_destroy_buffer_null_data_field` | [x] |
| 11 | `row11_get_operation_name_out_of_range_codes` | [x] |
| 12 | `row12_perform_operation_unmatched_operation_returns_zero` | [x] |
| 13 | `row13_perform_operation_divide_by_zero_returns_zero` | [x] |
| 14 | `row14_divide_int_min_by_minus_one_faults_identically` (out-of-process, SIGFPE) | [x] |
| 15 | `row15_perform_operation_null_operation_faults_identically` (out-of-process, SIGSEGV) | [x] |
| 16 | `row16_signed_overflow_wraps_identically` | [x] |
| 17 | `row17_buffapp_unchecked_create_buffer_is_unreachable` | [x] |
| 18 | `row18_buffapp_zero_product_takes_sum_branch` (in `tests/stdout_diff.rs`) | [x] |
| 19 | `row19_buffapp_never_divides_int_min_by_minus_one` (in `tests/stdout_diff.rs`) | [x] |
| 20 | `row20_append_length_int_max_wraps_past_the_growth_guard` (out-of-process, SIGSEGV) | [x] |
| 21 | `row21_append_large_negative_length_writes_below_allocation` (out-of-process, SIGSEGV) | [x] |
| generic | `generic_boundary_sweep` (NULL / zero / oversized / one-past-range) | [x] |
| harness | `crash_harness_control_case_exits_cleanly` — proves the out-of-process rows are not vacuous | [x] |

Notes on the out-of-process rows: `run_crash_case` re-execs the test binary with
`BUFFAPP_CRASH_CASE` / `BUFFAPP_CRASH_IMPL` set and compares the *terminating
signal*, so "both crash the same way" is asserted (SIGSEGV = 11, SIGFPE = 8),
not merely "both failed somehow". The control case asserts a clean `exit(0)`, so
a harness that always reported a signal could not make the rows pass.

### Divergence found and fixed

The only real divergence surfaced by Phase C was profile-dependent: with cargo's
default dev profile (`-C debug-assertions`) rustc inserts null/alignment checks
on raw-pointer dereferences, so `append_to_buffer(NULL, "x")` panicked and, being
a panic escaping `extern "C"`, aborted (SIGABRT/6) where the C faulted
(SIGSEGV/11). Fixed on the Rust side by disabling `debug-assertions` and
`overflow-checks` in `[profile.dev]` (`translation/Cargo.toml`); the crate uses
explicit `wrapping_*` arithmetic throughout, so nothing is lost. Verified by
`tests/profile_divergence.sh`, which runs the whole suite against both the
release and the dev cdylib: both are now clean.
