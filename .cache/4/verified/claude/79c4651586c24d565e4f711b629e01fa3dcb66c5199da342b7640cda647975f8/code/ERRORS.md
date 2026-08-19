# ERRORS.md — Error-surface table (Phase C gate)

Mechanically derived from `c_src/src/driver.c`. Every rejection/error path the C
code contains is listed below, one row per distinct branch.

## Mechanical grep of the rejection surface

```
$ grep -n 'return\|goto\|assert\|Error\|failed\|NULL\|-1' c_src/src/driver.c
33:    if (x != 1) {            -> printf("Error: x != 1\n");                     result = 1; goto fail;
39:    if (y != 2) {            -> printf("Error: x == 1 but y != 2\n");          result = 2; goto fail;
45:    if (z != 3) {            -> printf("Error: x == 1 and y == 2, but z != 3\n"); result = 3; goto fail;
52:    return result;           -> success return (result == 0)
54: fail:  printf("Operation failed\n"); return result;
```

Facts that bound the error surface:

* There are **no** `assert`s, **no** pointer parameters (so no null checks), no
  allocation (so no `NULL`/OOM path), no `errno` use, no `exit`/`abort`.
* `driver` returns `void`; the only observable result is the byte stream printed
  to `stdout`, whose last line is `Result: <code>` where `<code>` is
  `multi_stage`'s return value (0, 1, 2 or 3).
* Every parameter is a plain `int`, so **the entire `int` range is a valid
  input** — there is no range check, no min/max constant, and no enum. Values
  such as `INT_MIN`/`INT_MAX` are therefore not rejected; they simply take the
  "not equal" branch. Rows 7–10 below pin that down, because "out of range" for
  this API means "not the one accepted value".
* The three checks are **ordered and short-circuiting** via `goto fail`, so the
  later error messages are unreachable once an earlier one fires. Rows 4–6
  assert that ordering (the C never prints two `Error:` lines).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `multi_stage` via `driver(x,y,z)` | `x != 1` (e.g. `driver(0, 2, 3)`) | stdout `"Error: x != 1\nOperation failed\nResult: 1\n"`, `result = 1` | `err_row01_x_not_1` | [x] |
| 2 | `multi_stage` via `driver(x,y,z)` | `x == 1 && y != 2` (e.g. `driver(1, 0, 3)`) | stdout `"Error: x == 1 but y != 2\nOperation failed\nResult: 2\n"`, `result = 2` | `err_row02_y_not_2` | [x] |
| 3 | `multi_stage` via `driver(x,y,z)` | `x == 1 && y == 2 && z != 3` (e.g. `driver(1, 2, 0)`) | stdout `"Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n"`, `result = 3` | `err_row03_z_not_3` | [x] |
| 4 | `multi_stage` ordering | `x != 1` **and** `y != 2` simultaneously — the `x` check must win, the `y` message must NOT be printed | only the `x != 1` message, `Result: 1` | `err_row04_x_check_wins_over_y` | [x] |
| 5 | `multi_stage` ordering | `x != 1` **and** `z != 3` simultaneously — the `x` check must win | only the `x != 1` message, `Result: 1` | `err_row05_x_check_wins_over_z` | [x] |
| 6 | `multi_stage` ordering | `x == 1`, `y != 2` **and** `z != 3` simultaneously — the `y` check must win, the `z` message must NOT be printed | only the `y != 2` message, `Result: 2` | `err_row06_y_check_wins_over_z` | [x] |
| 7 | `driver` boundary | `x` one step past the single accepted value: `x = 0` and `x = 2` | both take the `x != 1` path, `Result: 1` | `err_row07_x_off_by_one` | [x] |
| 8 | `driver` boundary | `y` one step past the single accepted value: `y = 1` and `y = 3` (with `x = 1`) | both take the `y != 2` path, `Result: 2` | `err_row08_y_off_by_one` | [x] |
| 9 | `driver` boundary | `z` one step past the single accepted value: `z = 2` and `z = 4` (with `x = 1, y = 2`) | both take the `z != 3` path, `Result: 3` | `err_row09_z_off_by_one` | [x] |
| 10 | `driver` extreme ints | `INT_MIN` / `INT_MAX` / `-1` / `0` in each of the three positions (no range check exists, so these are *not* rejected specially — they just fail the equality test) | same error path as any other non-matching value; no crash, no UB divergence | `err_row10_extreme_ints` | [x] |
| 11 | `driver` "out-of-range enum" analogue | The C prototype takes plain `int`s with exactly one accepted value each; an arbitrary `int` with no valid meaning (e.g. `driver(0x7fffffff, -0x80000000, 12345)`) is a real FFI input. C accepts the call and reports the first failing stage. | identical byte stream from both libraries | `err_row11_no_valid_variant` | [x] |
| 12 | `driver` `void` return | `driver` has no error return channel at all — it can never signal failure to the caller; every input, valid or not, returns normally after printing | no return value; both libraries return normally for every input | `err_row12_void_return_never_traps` | [x] |
| 13 | static state after error | An error path leaves `static int y` set to the last `local_y` (the assignment `y = local_y` happens *before* validation, and is never rolled back) | the next call's behaviour is unaffected because `driver` reassigns `y` first; state must match between C and Rust | `err_row13_state_not_rolled_back` | [x] |

## Notes on unreachable C code

`multi_stage`'s success path (`printf("Ok!\n"); return result;`) returns
`result`, which is still `0` at that point — the `fail:` label is only reached by
`goto`, so `Result: 0` is printed exactly and only for `x==1, y==2, z==3`.
This is not a bug to fix; it is reproduced verbatim.
