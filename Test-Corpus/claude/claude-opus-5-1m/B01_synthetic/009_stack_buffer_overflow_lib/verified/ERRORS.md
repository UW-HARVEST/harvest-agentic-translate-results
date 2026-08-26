# ERRORS.md — error / rejection surface (Phase A, verified in Phase C)

Derived mechanically from `c_src/src/driver.c` (the only C source) by grepping
for every construct that can reject or mis-handle an input:

```sh
grep -n -E "assert|return|NULL|ERROR|errno|exit|<|>|==|!=|\?|switch|case|#if" \
     c_src/src/driver.c c_src/include/driver.h
```

Findings, verbatim:

* `driver.c:31` `if(line != NULL)`               → the only null check.
* `driver.c:46` `if (data >= 0)` … `:57` `printLine("ERROR: Array index is negative.")`  (in `bad`)
* `driver.c:66` `if (data >= 0)` … `:77` `printLine("ERROR: Array index is negative.")`  (in `goodG2B`, `data` is the constant `7`)
* `driver.c:85` `if (data >= 0 && data < (10))` … `:96` `printLine("ERROR: Array index is out-of-bounds")` (in `goodB2G`)
* `driver.c:50,70,89` `for(i = 0; i < 10; i++)`  → the only other constants (`10` = `buffer` length).

There are **no** `assert`s, **no** `return` statements with a value (every
function is `void`), **no** error enums / error codes, **no** `errno` use, **no**
`exit()`, **no** allocation and therefore no allocation-failure path, and **no**
`switch`/`case` or `#if` other than the header guard.  Consequently every
"rejection" is observable **only through stdout**, which is what the Phase C
tests compare (byte-for-byte), together with process termination status.

## Error-surface table

| #  | function (entry point used in test) | trigger (the exact invalid input/condition) | expected C result | test |
|----|-------------------------------------|---------------------------------------------|-------------------|------|
| 1  | `printLine` | `line == NULL` (`c:31` false) | returns silently, **0 bytes** written to stdout | `err_01_print_line_null` |
| 2  | `bad` | `data == -1` (`c:46` false → `c:57`) | exactly `ERROR: Array index is negative.\n`, no `printIntLine` output | `err_02_bad_negative_one` |
| 3  | `bad` | `data == INT_MIN` (extreme negative, `c:46` false) | same as row 2 | `err_03_bad_int_min` |
| 4  | `bad` | randomised `data < 0` (200 values incl. `-1`, `INT_MIN`, `INT_MIN+1`) | same as row 2 for every value | `err_04_bad_negative_random` |
| 5  | `bad` | `data >= 10` — **no upper-bound check exists** (`c:46` accepts, `c:48` stores out of bounds) | *not* rejected: OOB 4-byte stack store `buffer[data] = 1`, then the ten in-bounds elements are printed unchanged → `0\n` ×10. Afterwards the frame is corrupted, so the process may be killed by `SIGSEGV`; which `data` values crash is stack-layout dependent (UB). | `err_05_bad_oob_not_rejected` |
| 6  | `bad` | `data == 10` (first index past the end) | as row 5: `0\n` ×10 | `err_06_bad_oob_first` |
| 7  | `bad` | `data == INT_MAX` (extreme positive, wildly OOB) | as row 5: `c:46` accepts, store is far off the stack | `err_07_bad_int_max` |
| 8  | `good` → `goodB2G` | `data == -1` (`c:85` first conjunct false → `c:96`) | `goodG2B`'s ten lines (`1` at index 7) followed by `ERROR: Array index is out-of-bounds\n` | `err_08_good_negative` |
| 9  | `good` → `goodB2G` | `data == INT_MIN` | same as row 8 | `err_09_good_int_min` |
| 10 | `good` → `goodB2G` | `data == 10` (first value failing `data < (10)`) | same as row 8 | `err_10_good_ten` |
| 11 | `good` → `goodB2G` | `data == INT_MAX` | same as row 8 | `err_11_good_int_max` |
| 12 | `good` → `goodB2G` | randomised out-of-range `data` (200 values from `[INT_MIN,-1] ∪ [10,INT_MAX]`) | same as row 8 for every value | `err_12_good_out_of_range_random` |
| 13 | `good` → `goodB2G` | `data == 9` (last accepted value — one step *inside* the range) | accepted: `1` printed at index 9, **no** error line | `err_13_good_nine_accepted` |
| 14 | `good` → `goodB2G` | `data == 0` (lower boundary, accepted) | accepted: `1` at index 0 | `err_14_good_zero_accepted` |
| 15 | `bad` | `data == 9` / `data == 0` (boundaries the missing check *would* have used) | accepted, in-bounds store | `err_15_bad_boundaries_accepted` |
| 16 | `goodG2B` (via `good`) | its `else` branch (`c:66` false → `c:77`, "negative" message) is **unreachable**: `data` is the literal `7` | the `negative` message is *never* printed by `good`/`driver` for any argument | `err_16_goodg2b_error_branch_unreachable` |
| 17 | `driver` | `goodData < 0` **and** `badData < 0` (both error branches at once) | fixed transcript: `Calling good()...`, ten lines, out-of-bounds error, `Finished good()`, `Calling bad()...`, negative error, `Finished bad()` | `err_17_driver_both_error_branches` |
| 18 | `driver` | `goodData` out of range, `badData` in range (only `good`'s check rejects) | out-of-bounds error for good, normal ten lines for bad | `err_18_driver_only_good_rejects` |
| 19 | `driver` | `goodData` in range, `badData < 0` (only `bad`'s check rejects) | normal ten lines for good, negative error for bad | `err_19_driver_only_bad_rejects` |
| 20 | `driver` | `goodData == INT_MIN`, `badData == INT_MIN` (extreme ints across FFI) | as row 17 | `err_20_driver_int_min_both` |
| 21 | `printLine` | non-NULL pointer to a `'\0'` byte (empty string — degenerate, not rejected) | prints just `\n` | `err_21_print_line_empty` |
| 22 | `printLine` | pointer into the *interior* of a buffer, and buffer with an embedded NUL (C stops at the first NUL) | prints only the bytes up to the first NUL, then `\n` | `err_22_print_line_embedded_nul` |
| 23 | `printLine` | data that looks like a format string (`%s %n %d %%`) — must be treated as **data**, since it is the *argument* of `printf("%s\n", …)`, never the format | the literal bytes then `\n`; no crash, no varargs read | `err_23_print_line_format_like_data` |
| 24 | `printIntLine` | `INT_MIN` / `INT_MAX` / `0` / `-1` — the out-of-range-value analogue for this API (there are **no enums** in this API, so the "invalid enum variant" class degenerates to arbitrary `int` values, all of which are valid inputs to `%d`) | `printf("%d\n", n)` of the exact value, incl. `-2147483648\n` | `err_24_print_int_line_extremes` |

## How the intentionally-UB rows (5, 6, 7) are compared

The store `buffer[data] = 1` with `data >= 10` is the injected CWE-129 / CWE-787
flaw.  What the C source *specifies* there is only "write 4 bytes at
`&buffer[0] + 4*data` on the stack"; whether that slot is frame padding, a saved
register, a return address or an unmapped page is decided by the **compiler's**
frame layout, not by the C program.  So those rows are compared as follows
(`common::assert_same_stdout_ub`, capture via `common::run_ub`):

* stdout is still fully determined — the ten in-bounds elements are never touched
  by the store — so if **both** processes survive, the two streams must be
  **byte-identical**;
* if the smashed frame kills one of them, the shorter stream must be an exact
  **prefix** of the longer one (identical output up to the moment one process
  died);
* the UB call runs in a dedicated child with `stdout` unbuffered (nothing is lost
  in libc's buffer when the process dies), core dumps disabled, a large
  sacrificial stack cushion above the callee's frame, and `_exit` instead of a
  return, so the *harness* can never be mistaken for the library.

Every **non-UB** row compares stdout byte-for-byte **and** requires identical
process termination (`Exit::Code(0)` on both sides).

Observed result: stdout is identical for every value tested in both cargo
profiles, except `driver(_, 12)` / `driver(_, 13)` in the `release` profile, where
the C's out-of-bounds store lands on gcc's saved `rbp` (harmless until `driver`
returns, so gcc prints the last line first) while it lands on the return address
of the optimised Rust `driver` (so it dies one line earlier).  That difference is
a property of the two compilers' frame layouts after the frame has been smashed,
not of the translation: the emitted bytes are a strict prefix, and every value in
the well-defined domain matches exactly.
