# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/driver.c`. Every rejection / error path in
the C source is listed, one row per distinct branch.

## How the C code signals errors

`driver` returns `void`. There is **no** error return code, no `errno`, no
`RETURN_ERROR` macro, no `assert`, no `return NULL`, no allocation and no
pointer parameter. The library's *entire* observable behaviour — success **and**
failure — is the byte stream it writes to `stdout`.

Therefore "expected C result" below is the exact stdout byte sequence, which is
what the differential tests compare. The internal `result` code
(`multi_stage`'s return value) is observable indirectly through the final
`Result: %d` line.

Grep inventory of every branch/exit in the C source:

```
$ grep -n 'if (\|goto\|return\|printf' c_src/src/driver.c
33:    if (x != 1) {                                     -> rejection #1
34:        printf("Error: x != 1\n");
36:        goto fail;
39:    if (y != 2) {                                     -> rejection #2
40:        printf("Error: x == 1 but y != 2\n");
42:        goto fail;
45:    if (z != 3) {                                     -> rejection #3
46:        printf("Error: x == 1 and y == 2, but z != 3\n");
48:        goto fail;
51:    printf("Ok!\n");                                   -> success path #4
52:    return result;
54: fail:                                                 -> shared failure epilogue
55:    printf("Operation failed\n");
56:    return result;
62:    printf("Result: %d\n", result);
```

There are exactly **3** rejection branches, **1** success branch, and **1**
shared failure epilogue (`fail:`), reached only from the 3 rejections.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| E1 | `multi_stage` via `driver(x, y, z)` | `x != 1` (any `y`, any `z` — checked first, short-circuits) | `result = 1`; stdout = `"Error: x != 1\n" + "Operation failed\n" + "Result: 1\n"` | [x] |
| E2 | `multi_stage` via `driver(x, y, z)` | `x == 1` **and** `y != 2` (any `z`) | `result = 2`; stdout = `"Error: x == 1 but y != 2\n" + "Operation failed\n" + "Result: 2\n"` | [x] |
| E3 | `multi_stage` via `driver(x, y, z)` | `x == 1` **and** `y == 2` **and** `z != 3` | `result = 3`; stdout = `"Error: x == 1 and y == 2, but z != 3\n" + "Operation failed\n" + "Result: 3\n"` | [x] |
| E4 | `multi_stage` via `driver(x, y, z)` | shared `fail:` epilogue is reached from E1/E2/E3 and **only** from them — the success path must NOT print `"Operation failed\n"` | `"Operation failed\n"` present iff `result != 0`; absent when `result == 0` | [x] |
| E5 | `driver` | check ORDER / short-circuit: two or three conditions invalid at once (e.g. `x!=1 && y!=2 && z!=3`) must report only the FIRST failing one (`x` before `y` before `z`) — exactly one `Error: ...` line is ever printed | lowest-numbered failing check wins; never more than one `Error:` line | [x] |

## Generic FFI boundary cases (required by Phase C even though not in the table)

The signature is `void driver(int, int, int)`: there are no pointers, no
lengths, no buffers, no enums and no `const char*`, so the classic null-pointer /
zero-length / oversized-length / bad-enum-value classes have no representable
form in this API. What *is* representable, and is therefore tested, is the full
range and boundary set of the three `int` parameters:

| #  | trigger | expected C result | [x] |
|----|---------|-------------------|-----|
| E6 | `x = INT_MIN`, `INT_MAX`, `0`, `-1`, `2` (i.e. one step past and far past the single valid value `1`) | rejection E1 | [x] |
| E7 | `x = 1`, `y = INT_MIN`, `INT_MAX`, `0`, `1`, `3`, `-2`, `123` (one step past / far past the single valid value `2`; `123` is the C static's initialiser value) | rejection E2 | [x] |
| E8 | `x = 1`, `y = 2`, `z = INT_MIN`, `INT_MAX`, `0`, `2`, `4`, `-3` (one step past / far past the single valid value `3`) | rejection E3 | [x] |
| E9 | out-of-range "enum-like" ints: the API accepts any `int`; every one of the 2^32 values is a legal input and must be classified identically by C and Rust (verified over randomized full-`i32`-range sweeps) | classification per E1/E2/E3/success | [x] |
| E10 | repeated / interleaved calls: the file-scope `static int y` is written by every `driver` call, so no state can leak across calls; a failing call followed by a succeeding call must still succeed (and vice versa) | each call's stdout depends only on its own arguments | [x] |

There are no `malloc`/`calloc` calls, so allocation-failure paths do not exist.
There are no `assert`s, so abort paths do not exist.

## Row → test mapping (Phase C, all passing)

Run with `RUST_TEST_THREADS=1 cargo test --release --test phase_c_error_paths`.

| row | test in `tests/phase_c_error_paths.rs` |
|-----|----------------------------------------|
| E1 | `e1_x_not_one_rejects_with_code_1` |
| E2 | `e2_y_not_two_rejects_with_code_2` |
| E3 | `e3_z_not_three_rejects_with_code_3` |
| E4 | `e4_operation_failed_epilogue_only_on_error_paths` |
| E5 | `e5_only_first_failing_check_is_reported` |
| E6 | `e6_x_extremes_and_off_by_one` |
| E7 | `e7_y_extremes_and_off_by_one` |
| E8 | `e8_z_extremes_and_off_by_one` |
| E9 | `e9_arbitrary_int_bit_patterns_classified_identically` |
| E10 | `e10_error_then_success_and_interleavings` |
| (guard) | `no_pointer_length_or_enum_parameters_exist_in_the_api` — asserts the API still has no pointer / length / enum / assert / malloc surface, so the "not representable" claims above cannot silently rot |

Each test asserts BOTH the exact rejection message bytes AND the numeric
`result` sentinel parsed from the `Result: %d` line, for the C `.so` and the
Rust `.so` independently — never merely "both failed somehow".

Result: **11 passed, 0 failed.**
