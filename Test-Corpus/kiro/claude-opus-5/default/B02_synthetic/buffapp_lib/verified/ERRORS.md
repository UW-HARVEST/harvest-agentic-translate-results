# ERRORS.md — Phase A error-surface table

Derived mechanically from every rejection / error-return / guard in
`c_src/src/lib.c`. There are no `assert`s, no error enums, no `RETURN_ERROR`
macros and no min/max constants in the C source; the complete set of rejection
points is the `return NULL` / `return -1` / `return 0` guards and the `switch`
`default` arm listed below (grep of `return|if (|switch|case|default|assert`
over `lib.c` produced exactly the lines cited in the "C line" column).

Note on the `int` -> `size_t` conversions: `malloc(initial_capacity)` and
`realloc(.., new_capacity)` pass a *signed* `int`, so a negative value
sign-extends to a near-`SIZE_MAX` request and the allocator returns `NULL`.
That is the only reliably reachable allocation-failure path, and it is what
rows 2 and 4 use.

| # | function | trigger (the exact invalid input/condition) | expected C result | C line |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `create_buffer` | `malloc(sizeof(StringBuffer))` (16 bytes) fails | returns `NULL` | 36-37 |
| 2 | `create_buffer` | `malloc(initial_capacity)` fails — reachable with `initial_capacity < 0`, which sign-extends to a ~`SIZE_MAX` request (`-1`, `-16`, `INT_MIN`); the already-allocated struct is `free`d first | returns `NULL` (no leak of the struct) | 41-43 |
| 3 | `create_buffer` | `initial_capacity == 0`: `malloc(0)` succeeds, then `buffer->data[0] = '\0'` writes 1 byte past a 0-byte allocation | **no error** — returns a valid non-NULL buffer with `capacity == 0`, `length == 0` (out-of-bounds write is benign on glibc; behaviour must match, incl. `capacity` field value) | 46-48 |
| 4 | `append_to_buffer` | `realloc` fails: `new_capacity = required_capacity * 2` overflows `int` and becomes negative (e.g. `buffer->length = 2_000_000_000`, any non-empty `str`), sign-extending to a ~`SIZE_MAX` request | returns `-1`, and `buffer->data` / `buffer->capacity` / `buffer->length` are left UNCHANGED | 61-62 |
| 5 | `append_to_buffer` | `buffer == NULL` | no NULL check exists -> dereferences NULL -> SIGSEGV. **Not testable in-process**; asserted identical by inspection (neither impl checks). Rust has no `is_null` guard here either. | 55-56 |
| 6 | `append_to_buffer` | `str == NULL` | passed straight to `strlen` -> SIGSEGV. Not testable in-process; neither impl guards. | 55 |
| 7 | `append_to_buffer` | `required_capacity <= buffer->capacity` (string fits) | **no error** — no `realloc` call at all; `strcpy` in place, `length += str_len`, returns `0`. The `data` pointer must be unchanged. | 57, 69-72 |
| 8 | `destroy_buffer` | `buffer == NULL` | guarded: returns without touching anything (no crash, no free) | 76 |
| 9 | `destroy_buffer` | `buffer != NULL` but `buffer->data == NULL` | guarded: skips `free(buffer->data)`, still `free`s the struct | 77-79 |
| 10 | `get_operation_name` | `op_code` outside `0..=3` — i.e. every out-of-range "enum" value crossing the FFI boundary: `4`, `5`, `-1`, `-4`, `INT_MAX`, `INT_MIN`, and random `int`s | returns the string `"unknown"` (never `NULL`) | 90 |
| 11 | `perform_operation` | `operation` is not exactly one of `"add"`, `"subtract"`, `"multiply"`, `"divide"` — includes `""`, `"ADD"`, `"ad"`, `"adds"`, `"addx"`, `" add"`, `"divide0"`, arbitrary junk | returns `0` (silent sentinel, indistinguishable from a legitimate `0` result) | 107 |
| 12 | `perform_operation` | `operation == "divide"` and `b == 0` | returns `0` (division guarded) | 102-105 |
| 13 | `perform_operation` | `operation == NULL` | passed to `strcmp` -> SIGSEGV. Not testable in-process; neither impl guards. | 95 |
| 14 | `perform_operation` | `operation == "divide"`, `a == INT_MIN`, `b == -1` | `b != 0` so the guard passes; `INT_MIN / -1` is C UB and compiles to `idiv` -> **SIGFPE**. Not testable in-process; the Rust uses an inline `idiv` (`c_div`) specifically so it traps identically instead of wrapping. | 103 |
| 15 | `buffapp` | `create_buffer(32)` returns `NULL` | result is **never NULL-checked**; `log_buffer->length = 0` would SIGSEGV. Not reachable in practice (32-byte malloc); neither impl guards. | 122, 126 |
| 16 | `buffapp` | `intermediate3 == 0` (i.e. `intermediate1 == 0 || intermediate2 == 0`) | division is skipped; `result = param1 + param2 + param3 + param4` (wrapping) | 141-145 |
| 17 | `buffapp` | `intermediate3 != 0` but `result == INT_MIN && intermediate3 == -1` | `result / intermediate3` -> SIGFPE (same `idiv` UB as row 14). Not testable in-process. | 142 |

## Testability legend

* Rows **2, 3, 4, 7, 8, 9, 10, 11, 12, 16** are differentially testable in
  process and each has a test in `tests/error_paths.rs`.
* Rows **5, 6, 13, 15** are NULL-pointer dereferences: the C has no guard, so
  the only correct behaviour is to crash the same way. Verified by asserting
  the Rust code contains no NULL guard at those sites (a Rust `is_null()`
  early-return would be a *divergence*), and by a subprocess crash-parity test
  where practical.
* Rows **14, 17** are `INT_MIN / -1` SIGFPE. Verified out-of-process
  (`tests/error_paths.rs::sigfpe_parity`) by running each `.so` in a forked
  child and comparing the termination signal.
* Row **1** is an unconditional 16-byte `malloc` failure, not reachable without
  an allocator interposer; covered by inspection (identical NULL return).

## Phase C completion status — every row has a passing differential test

| # | test in `tests/error_paths.rs` | status |
|---|--------------------------------|--------|
| 1 | `err_01_struct_alloc_failure_is_structural` | [x] |
| 2 | `err_02_data_alloc_failure_returns_null`, `err_02b_huge_positive_capacity_agrees` | [x] |
| 3 | `err_03_zero_capacity_is_not_an_error` | [x] |
| 4 | `err_04_realloc_failure_returns_minus_one`, `err_04b_realloc_failure_randomized_lengths` | [x] |
| 5 | `err_05_append_null_buffer_crash_parity` (forked child; both SIGSEGV = signal 11) | [x] |
| 6 | `err_06_append_null_string_crash_parity` (forked child) | [x] |
| 7 | `err_07_no_grow_path_returns_zero` | [x] |
| 8 | `err_08_destroy_null_is_noop` | [x] |
| 9 | `err_09_destroy_with_null_data` | [x] |
| 10 | `err_10_get_operation_name_out_of_range` (~21 000 out-of-range enum values) | [x] |
| 11 | `err_11_unknown_operation_returns_zero` (~530 rejected names x 8 operand pairs) | [x] |
| 12 | `err_12_divide_by_zero_returns_zero` | [x] |
| 13 | `err_13_perform_operation_null_string_crash_parity` (forked child) | [x] |
| 14 | `err_14_int_min_div_minus_one_sigfpe_parity` (forked child; both SIGFPE = signal 8) | [x] |
| 15 | `err_15_buffapp_unchecked_create_is_unreachable` | [x] |
| 16 | `err_16_buffapp_zero_intermediate_branch` (return value + stdout) | [x] |
| 17 | `err_17_buffapp_int_min_div_minus_one_sigfpe_parity` (forked child; both SIGFPE) | [x] |

Generic boundaries beyond the table (also passing):
`generic_capacity_boundaries`, `generic_append_length_boundaries`,
`generic_one_past_valid_enum_range`, plus the NULL-pointer crash-parity tests
above and out-of-range enum coverage in row 10.

### Row 17 reachability (worked out, not assumed)

`result / intermediate3` traps only when `result == INT_MIN` and
`intermediate3 == -1`, i.e. `i1 + i2 == INT_MIN` and `i1 * i2 == -1 (mod 2^32)`.
Solving mod `2^32`: `i1 = -(2^30 - 1)`, `i2 = -(2^30 + 1)`, whose product is
`2^60 - 1 ≡ -1`. Both are producible from the `"add"` arm, so
`buffapp(-1073741824, 1, -1073741824, -1)` reaches the trap. Confirmed: the C
`.so` dies with SIGFPE and so does the Rust `.so`.

### Divergence found and fixed during Phase C

The **debug-profile** Rust `.so` turned rows 5/6/13 from SIGSEGV (signal 11)
into SIGABRT (signal 6): rustc's `-Cdebug-assertions` inserts a
"null pointer dereference occurred" panic, and a panic escaping an
`extern "C"` `cdylib` aborts. Since this crate deliberately reproduces the C's
unguarded NULL dereferences, the instrumentation was disabled for the `dev` and
`test` profiles in `Cargo.toml`. After the fix all four artifacts (release and
debug x default and no-default features) match the C exactly.
