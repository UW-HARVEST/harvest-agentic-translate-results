# Error Surface

The C implementation has no error enum, error-return macro, `return -1`,
`return NULL`, `assert`, null check, allocation-failure check, or public
min/max constant. It handles the following invalid or rejected ranges without
undefined behavior.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E01 | `shift_array_data` | `shift_by <= 0`, so `shift_by > 0 && shift_by < size` is false | Return `void`; array bytes are unchanged. A null `arr` is not accessed. |
| [x] E02 | `shift_array_data` | `shift_by >= size`, so `shift_by > 0 && shift_by < size` is false (includes `shift_by == size`, one past the accepted maximum) | Return `void`; array bytes are unchanged. A null `arr` is not accessed. |
| [x] E03 | `manipulate_records` | `shift >= num_records`, so `shift > 0 && shift < num_records` is false and `num_records - shift <= 0` | No records are accessed; return `0`, including for null `records`. |
| [x] E04 | `manipulate_records` | `num_records <= 0` and `shift >= num_records` | No records are accessed; return `0`, including for null `records`. |
| [x] E05 | `compute_with_dynamic_memory` | `count < 0` | Both `i < count` loops execute zero times; `free` is called on the allocation result and the function returns `0`. |

## Unhandled Invalid Inputs

These are not rejection rows because the C code gives them no defined result:

- `apply_operation(NULL, ...)` calls a null function pointer.
- `shift_array_data(NULL, size, shift_by)` dereferences null when
  `shift_by > 0 && shift_by < size`.
- `process_pointer_data(NULL, ...)` unconditionally dereferences null.
- `manipulate_records(NULL, num_records, shift)` dereferences null when
  `num_records - shift > 0`.
- Allocation failure with a positive loop count in
  `compute_with_dynamic_memory` or `hatch` is followed by a null dereference.
- Positive lengths too large for the caller's allocation cause out-of-bounds
  access. The C API has no oversized-length rejection.
- There are no enum parameters, so no out-of-range enum case exists.
