# ERRORS.md — error / rejection surface table

Mechanical grep of the whole C source (`c_src/src/driver.c`, `c_src/include/driver.h`)
for every rejection mechanism:

```sh
$ grep -nE 'return|assert|NULL|errno|exit|abort|ERROR|-1|<|>|==|!=' c_src/src/driver.c
30:    while (x > 0 || y > 0) {
33:        if (x == 1 && y == 4) {
38:        if (x > 0) {
44:        if (y == 0) {
49:        if (x < 3) {
```

Findings:

* `return` statements: **0** (the function is `void` and falls off the end).
* `assert` / `NULL` checks / `errno` / `exit` / `abort` / error enums / error
  macros / min-max constants: **0 occurrences**.
* Pointer parameters: **none** (both parameters are by-value `int`), so there is
  no null-pointer or length/size validation surface at all.
* Enum parameters: **none**, so there is no out-of-range-enum surface either.

The library therefore has **no explicit error-return surface**. Its entire
"rejection" behaviour is *implicit*: the guard conditions that decide whether
work happens, plus the degenerate/extreme argument values a caller can push
across the FFI boundary. Those are enumerated below as one row per distinct
implicit rejection / boundary condition, each with a differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | loop guard rejects the call outright: `x <= 0 && y <= 0` (line 30 `while (x > 0 \|\| y > 0)` false on entry) | returns immediately, **no output at all** (0 bytes), no crash | `err_e1_guard_rejects_nonpositive` | [x] |
| E2 | `driver` | one step past the guard on the `x` axis: `x == 0, y == 0` vs `x == 1, y == 0` | `x=0,y=0` → 0 bytes; `x=1,y=0` → exactly `loop\nx\n` | `err_e2_one_past_x_guard` | [x] |
| E3 | `driver` | one step past the guard on the `y` axis: `y == 0` vs `y == 1` with `x <= 0` | `y=0` → 0 bytes; `y=1` → exactly `loop\ny\n` | `err_e3_one_past_y_guard` | [x] |
| E4 | `driver` | most-negative arguments (extreme out-of-any-sane-range values): `x = INT_MIN`, `y = INT_MIN` | guard false → 0 bytes, no decrement, no overflow reached | `err_e4_int_min_both` | [x] |
| E5 | `driver` | `x = INT_MIN`, `y > 0` (negative `x` never satisfies `x > 0`, so `x--` is never reached and `x < 3` is always true) | prints `loop\n` then `y\n` repeated `y` times, then stops | `err_e5_int_min_x_positive_y` | [x] |
| E6 | `driver` | `x > 0`, `y = INT_MIN` (negative-but-nonzero `y` passes the `y == 0` check at line 44, so `y--` runs on a negative value) | **not differentially testable**: the C loops ~2^31 times and then signed-overflows `y--` (undefined behaviour). Both implementations are checked on the *reachable prefix* of this path instead (see E7). | `err_e6_negative_y_prefix` (bounded) | [x] |
| E7 | `driver` | `y < 0` with `x <= 0` (guard false because `x > 0` and `y > 0` are both false even though `y != 0`) | returns immediately, 0 bytes | `err_e7_negative_y_nonpositive_x` | [x] |
| E8 | `driver` | `x = INT_MAX` / `y = INT_MAX` (largest representable arguments) | terminating but ~2^31 iterations of output; **not differentially testable within the time budget**. Covered by the largest feasible values instead (`x`,`y` up to 3000) plus E4/E5 for the opposite extreme. | `err_e8_large_but_feasible` | [x] |
| E9 | `driver` | `x == 1 && y == 4` — the only input for which line 33 takes the `goto label2` branch and *skips* the `label1` block, i.e. the special-cased "rejected" first pass | prints `loop\ny\nx\ny\ny\ny\n` (verified byte-for-byte against C) | `err_e9_goto_label2_special_case` | [x] |

Rows E6 and E8 are the two conditions the C code cannot be *fully* driven
through (2^31 iterations, and signed-overflow UB in E6). They are marked as
covered by a bounded prefix / largest-feasible substitute rather than skipped:
the reachable behaviour on those exact paths is compared byte-for-byte.
