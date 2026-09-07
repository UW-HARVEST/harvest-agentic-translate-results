# Configuration Surface

The public header declares only `complexmode`, while `nm -D` exposes six
additional low-level entry points. All seven exports are included below.
There are no Cargo features and no C preprocessor feature switches.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `create_result_string` | short non-empty operation; positive/zero value; formatted output fits in 64 bytes | [x] |
| 2 | `create_result_string` | empty operation and negative value; formatted output fits in 64 bytes | [x] |
| 3 | `create_result_string` | long operation; `snprintf` output reaches/truncates at the fixed 64-byte buffer boundary | [x] |
| 4 | `create_result_string` | `op == NULL`; platform `snprintf("%s")` handling through the FFI boundary | [x] |
| 5 | `check_permissions` | `required == 0` with arbitrary permission bits | [x] |
| 6 | `check_permissions` | one required bit is present | [x] |
| 7 | `check_permissions` | one required bit is absent | [x] |
| 8 | `check_permissions` | combined `READ_PERM \| WRITE_PERM` bits are both present, with or without extra bits | [x] |
| 9 | `check_permissions` | combined required bits are only partially present | [x] |
| 10 | `check_permissions` | execute/unrelated and high-bit masks, present versus absent | [x] |
| 11 | `safe_add` | exact read+write permissions; zero/positive/negative operands without overflow | [x] |
| 12 | `safe_add` | read+write plus extra permission bits; randomized operands | [x] |
| 13 | `safe_add` | no required permission bits | [x] |
| 14 | `safe_add` | only one of read/write is present | [x] |
| 15 | `safe_add` | sufficient permissions with signed-overflow boundary operands | [x] |
| 16 | `multiply_with_log` | positive operands and non-null output slot | [x] |
| 17 | `multiply_with_log` | zero and negative operands and non-null output slot | [x] |
| 18 | `multiply_with_log` | signed-overflow boundary operands and non-null output slot | [x] |
| 19 | `copy_and_sum` | non-null pointer with `count == 0` | [x] |
| 20 | `copy_and_sum` | one element | [x] |
| 21 | `copy_and_sum` | many elements containing zero, positive, and negative values | [x] |
| 22 | `copy_and_sum` | many elements whose signed sum crosses the integer boundary | [x] |
| 23 | `compare_operations` | equal non-empty strings | [x] |
| 24 | `compare_operations` | two empty strings | [x] |
| 25 | `compare_operations` | first string lexicographically less/greater than second | [x] |
| 26 | `compare_operations` | one string is a prefix of the other | [x] |
| 27 | `complexmode` | mode 1 addition with ordinary and boundary operand values; fixed permissions `0644` satisfy read+write | [x] |
| 28 | `complexmode` | mode 2 multiplication/logging with ordinary and boundary operand values | [x] |
| 29 | `complexmode` | mode 3 fixed three-element array sum with ordinary and boundary operand values | [x] |
| 30 | `complexmode` | mode 4 with fixed permissions `0644`; execute bit absent, so the additive branch is selected | [x] |
