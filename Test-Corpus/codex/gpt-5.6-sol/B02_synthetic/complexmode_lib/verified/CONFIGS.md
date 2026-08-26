# Configuration Surface

Derived from all seven dynamic entry points, the public header, and every
value-dependent branch, loop shape, fixed output width, permission mask, and
mode branch in `c_src/src/lib.c`. Error configurations are tracked separately
in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `create_result_string` | empty operation string; signed `int` value; formatted result fits in 64 bytes | [x] |
| 2 | `create_result_string` | nonempty operation string; signed `int` value; formatted result fits in 64 bytes | [x] |
| 3 | `create_result_string` | operation/value formatting reaches or exceeds the 64-byte buffer and is truncated by `snprintf` | [x] |
| 4 | `check_permissions` | `required == 0`, which is always satisfied | [x] |
| 5 | `check_permissions` | every required bit present, with exact or additional permission bits | [x] |
| 6 | `check_permissions` | at least one required bit absent | [x] |
| 7 | `safe_add` | read and write bits both present; randomized signed operands including wrapping boundaries | [x] |
| 8 | `multiply_with_log` | valid output pointer; product representable as signed `int`; allocated log string | [x] |
| 9 | `multiply_with_log` | valid output pointer; signed multiplication wraps; allocated log string records wrapped product | [x] |
| 10 | `copy_and_sum` | nonnull source and `count == 0` (empty shape) | [x] |
| 11 | `copy_and_sum` | nonnull source and `count == 1` (singleton shape) | [x] |
| 12 | `copy_and_sum` | nonnull source and `count > 1` (many-element shape), representable sum | [x] |
| 13 | `copy_and_sum` | nonnull source and `count > 1`; signed accumulation wraps | [x] |
| 14 | `compare_operations` | two empty strings | [x] |
| 15 | `compare_operations` | equal nonempty strings | [x] |
| 16 | `compare_operations` | first differing unsigned byte in `op1` is less than that in `op2` | [x] |
| 17 | `compare_operations` | first differing unsigned byte in `op1` is greater than that in `op2` | [x] |
| 18 | `complexmode` | mode 1; fixed permissions `0644` satisfy read+write; randomized signed addition | [x] |
| 19 | `complexmode` | mode 2; multiplication representable; nonempty generated log | [x] |
| 20 | `complexmode` | mode 2; multiplication wraps; nonempty generated log | [x] |
| 21 | `complexmode` | mode 3; fixed three-element array; representable sum | [x] |
| 22 | `complexmode` | mode 3; fixed three-element array; signed accumulation wraps | [x] |
| 23 | `complexmode` | mode 4; fixed permissions `0644` fail the `0100` execute check, selecting `value1 + value2 + value3`; representable sum | [x] |
| 24 | `complexmode` | mode 4; same non-execute branch; signed accumulation wraps | [x] |

## Call Hierarchy

The direct low-level entry points are tested before the composed paths:

```text
create_result_string
check_permissions
safe_add -> check_permissions
multiply_with_log -> create_result_string
copy_and_sum
compare_operations
complexmode -> safe_add | multiply_with_log | copy_and_sum | check_permissions
```
