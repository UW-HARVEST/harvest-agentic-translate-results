# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically by grepping every rejection / failure branch in
`c_src/src/lib.c`. There are no `assert`s, no error enums, no `errno` use, no
`RETURN_ERROR`-style macros and no explicit numeric range checks in the C —
every rejection is a null check or an allocation-failure check. The complete
set of `if`/`return` sites is:

```
$ grep -n 'return\|assert\|if (' c_src/src/lib.c
47:    if (!arr) return NULL;              # init_array: malloc(DynamicArray) failed
50:    if (!arr->data) {                   # init_array: malloc(data) failed
52:        return NULL;
61:    if (!arr) return 0;                 # expand_array: null arr
66:    if (!new_data) {                    # expand_array: realloc failed
67:        return 0;
76:    if (!arr) return 0;                 # add_element: null arr
79:        if (!expand_array(arr)) {       # add_element: growth failed
80:            return 0;
89:    if (arr) {                          # free_array: null guard (no-op)
154:    if (!arr) {                         # matrixsum: init_array(2) failed
155:        return -1;
```

One row per distinct rejection branch, plus the generic FFI-boundary boundaries
required by the task (null pointers, zero / oversized lengths, one-step-past
range values, out-of-range "enum"-style ints).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `init_array` | `malloc(sizeof(DynamicArray))` returns NULL (`lib.c:46-47`) | returns `NULL` | not reachable from the FFI boundary — a 24-byte `malloc` cannot be made to fail deterministically without an allocator hook. Covered structurally: row 2 exercises the same `free(arr); return NULL` cleanup path. | n/a |
| 2 | `init_array` | `malloc(initial_capacity * sizeof(int))` returns NULL, i.e. `initial_capacity` so large the byte count is unallocatable. `SIZE_MAX`, `SIZE_MAX/4`, `SIZE_MAX/2`, `1<<62 + 1` | returns `NULL` (and frees the header) | `err_init_array_oversized_capacity_returns_null` | [x] |
| 3 | `init_array` | `initial_capacity == 0` → `malloc(0)`: glibc returns a **non-NULL** unique pointer, so this is *not* an error. Boundary "zero length" case; resulting array has `capacity == 0`, `size == 0` | returns non-NULL, `size=0`, `capacity=0` | `err_init_array_zero_capacity_is_not_an_error` | [x] |
| 4 | `init_array` | `initial_capacity` such that `initial_capacity * sizeof(int)` **wraps to 0** (`1<<62` on LP64) → `malloc(0)` succeeds. Wrapping-overflow boundary; C does no overflow check, so this must NOT be rejected | returns non-NULL with `capacity == 1<<62` | `err_init_array_capacity_multiply_wraps_to_zero` | [x] |
| 5 | `expand_array` | `arr == NULL` (`lib.c:61`) | returns `0` | `err_expand_array_null_returns_zero` | [x] |
| 6 | `expand_array` | `realloc` fails: `arr->capacity * 2 * sizeof(int)` unallocatable (capacity forged to `SIZE_MAX/8`, `SIZE_MAX/4`, `1<<61`) | returns `0`, leaves `data`/`capacity` untouched | `err_expand_array_realloc_failure_returns_zero` | [x] |
| 7 | `expand_array` | `arr->capacity == 0` → `new_capacity == 0` → `realloc(p, 0)`, which under glibc frees `p` and returns NULL | returns `0` (buffer freed, `arr->data` left dangling — C does not clear it) | `err_expand_array_zero_capacity_realloc_zero` | [x] |
| 8 | `add_element` | `arr == NULL` (`lib.c:76`) | returns `0` | `err_add_element_null_returns_zero` | [x] |
| 9 | `add_element` | `arr` valid but full **and** growth fails (`size >= capacity` and `expand_array` returns 0 — capacity forged to `SIZE_MAX/8`) | returns `0`, `size` unchanged | `err_add_element_growth_failure_returns_zero` | [x] |
| 10 | `add_element` | `arr->capacity == 0` (zero-length buffer): `size >= capacity` immediately, `expand_array` returns 0 via row 7 | returns `0` | `err_add_element_zero_capacity_returns_zero` | [x] |
| 11 | `free_array` | `arr == NULL` (`lib.c:89`) | no-op, no crash, returns void | `err_free_array_null_is_noop` | [x] |
| 12 | `matrixsum` | `init_array(2)` returns NULL (`lib.c:154-155`) | returns `-1` | not reachable from the FFI boundary — `matrixsum` hard-codes `init_array(2)` (a 24-byte + 8-byte allocation pair) with no caller-controlled size. Documented as unreachable; both implementations contain the identical `-1` branch. | n/a |
| 13 | `process_flags` | out-of-range "enum"/flag ints: bits outside the documented `FLAG_READ..FLAG_DELETE` nibble (`0x10`, `0xFF`, `-1`, `INT_MIN`, `INT_MAX`, `0x7FFFFFF0`) — C has **no** validation, so these are accepted and only the low 4 bits count | returns `0..4`, high bits ignored; never rejects | `err_process_flags_out_of_range_values` | [x] |
| 14 | `matrixsum` | extreme / one-past-range `int` arguments (`INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`, mixed signs) causing signed overflow in `sum`, `sum*0x10` and the final adds — C has no range check and wraps at `-O0` | returns the wrapped `int`; never rejects | `err_matrixsum_extreme_int_arguments` | [x] |
| 15 | `calculate_matrix_checksum` | takes no arguments — the only "invalid input" reachable is a `matrix` global mutated to overflow the `int` accumulator (all 12 cells `INT_MAX`) | returns the wrapped sum; never rejects | `err_matrix_checksum_overflowing_global` | [x] |

All 15 rows are either covered by a passing differential test or explicitly
documented as unreachable across the FFI boundary (rows 1 and 12 — small
fixed-size `malloc` failures that cannot be provoked without an allocator
interposer; both implementations were read and contain the same branch).
