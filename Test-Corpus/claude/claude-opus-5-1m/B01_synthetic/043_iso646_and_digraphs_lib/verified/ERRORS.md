# ERRORS.md — Phase A: error-surface table

Mechanically derived from the C source. Grep counts over the **complete** C
source set (`c_src/src/driver.c`, `c_src/include/driver.h`), comments excluded:

| pattern grepped | occurrences in C source |
|---|---|
| `return` | 0 |
| `RETURN_ERROR` / any error macro | 0 |
| `assert` | 0 |
| `NULL` / null check | 0 |
| `errno` | 0 |
| `exit` / `abort` | 0 |
| `goto` | 0 |
| `if` / `switch` / `?:` (any branch) | 0 |
| `enum` | 0 |
| `malloc` / `calloc` / `realloc` / `free` | 0 |
| `sizeof` | 0 |
| `_MIN` / `_MAX` constants, explicit range check | 0 |
| `-1` or any sentinel literal | 0 |

The entire library body is:

```c
void driver(int x, int y) {
    int result = x | ~y;   /* `x bitor compl y` */
    printf("%d", result);
    puts("");
}
```

Consequences, taken directly from the source (not from docs or assumption):

* `driver` returns `void` — there is **no** error channel: no return code, no
  out-parameter, no `errno` write, no global status.
* Both parameters are `int` **by value**. There are no pointers, no lengths, no
  counts, no enums and no sizes, so the classic generic boundary classes (null
  pointer, zero length, oversized length, out-of-range enum variant) have **no
  parameter to attach to** — every one of the 2^64 `(int, int)` argument pairs
  is a *valid* input and must be accepted.
* `x | ~y` on a 32-bit two's-complement `int` cannot overflow and has no
  undefined-behaviour case (bitwise ops, no shifts, no division), so there is no
  input that makes the C trap or diverge.
* The only operations that *can* fail are `printf` and `puts`, and the C
  **discards both return values**. So a write failure must be silently ignored,
  identically, by the Rust.

Therefore the rejection table below has no rows sourced from explicit checks
(rows 1–0 do not exist), and instead enumerates the *only* failure/limit
conditions the C code can actually be subjected to, plus the generic FFI
boundary classes that this signature admits. Every row has a differential test
in `tests/phase_c_errors.rs`.

## Error / rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `driver` | *(no explicit rejection exists in the C source — documented for completeness; `printf`/`puts` return values are discarded, no `assert`, no `return`, no error enum)* | n/a — no error channel; call always returns normally | `no_error_channel_exists_in_c_source` | [x] |
| 2 | `driver` | `stdout` write fails with `ENOSPC` — fd 1 redirected to `/dev/full`; `printf`/`puts` return `< 0` | return value ignored; `driver` returns normally, no abort, no diagnostic on stdout | `err_stdout_write_fails_enospc_dev_full` | [x] |
| 3 | `driver` | `stdout` write fails with `EBADF` — fd 1 redirected to a **read-only** descriptor | return value ignored; `driver` returns normally, no abort | `err_stdout_write_fails_ebadf_readonly_fd` | [x] |
| 4 | `driver` | `stdout` discarded — fd 1 redirected to `/dev/null` (writes succeed, bytes vanish) | returns normally, produces no observable bytes | `err_stdout_discarded_dev_null` | [x] |
| 5 | `driver` | `y = 0` → `~y = -1` → result is `-1` for **every** `x` (the value-collapsing boundary) | prints `-1\n` regardless of `x` | `err_boundary_y_zero_collapses_to_minus_one` | [x] |
| 6 | `driver` | `x = -1` (all bits set) → result is `-1` for **every** `y` | prints `-1\n` regardless of `y` | `err_boundary_x_all_bits_set` | [x] |
| 7 | `driver` | `x = INT_MIN` (`-2147483648`, most negative, no positive counterpart) | prints `x \| ~y`; widest possible 11-char output when result is `INT_MIN` | `err_boundary_int_min` | [x] |
| 8 | `driver` | `x = INT_MAX` (`2147483647`) | prints `x \| ~y` (`-1` unless `y` = `INT_MIN`… see test) | `err_boundary_int_max` | [x] |
| 9 | `driver` | `y = INT_MIN` → `~y = INT_MAX` (sign flip at the boundary) | prints `x \| INT_MAX` | `err_boundary_y_int_min` | [x] |
| 10 | `driver` | `y = INT_MAX` → `~y = INT_MIN` (sign flip at the boundary) | prints `x \| INT_MIN` (always negative) | `err_boundary_y_int_max` | [x] |
| 11 | `driver` | one step past the `int` range: `INT_MAX + 1` / `INT_MIN - 1` supplied as 64-bit values through a mis-declared `extern "C" fn(i64, i64)` prototype — the C ABI truncates to the low 32 bits (the analogue of an out-of-range enum value: the callee is handed a bit pattern with no valid `int` reading) | callee reads only `edi`/`esi`; result identical to passing the truncated `int` | `err_out_of_int_range_args_truncate_via_i64_abi` | [x] |
| 12 | `driver` | garbage in the **upper** 32 bits of both argument registers with valid low halves (`(hi << 32) \| lo`) — every combination of hi/lo sign | upper bits ignored; identical to the `int` call | `err_upper_argument_register_bits_ignored` | [x] |
| 13 | `driver` | the 4 corner combinations of the parameter domain at once: `(INT_MIN, INT_MIN)`, `(INT_MIN, INT_MAX)`, `(INT_MAX, INT_MIN)`, `(INT_MAX, INT_MAX)` | prints the corresponding `x \| ~y` | `err_boundary_all_four_extreme_corners` | [x] |
| 14 | `driver` | result is exactly `0` — reachable only at `x = 0, y = -1`; the single input whose output has no `-` sign and a single digit | prints `0\n` | `err_boundary_only_zero_result_input` | [x] |
| 15 | `driver` | `stdout` set **unbuffered** (`setvbuf(_IONBF)`) so `printf` and `puts` each hit `write(2)` separately | same bytes, `<digits>` then `\n` | `err_stdout_unbuffered_setvbuf` | [x] |
| 16 | `driver` | `stdout` fully buffered with a **1-byte** buffer (`setvbuf(_IOFBF, size 1)`), forcing a flush per character | same bytes | `err_stdout_one_byte_buffer` | [x] |

Rows 5–10 and 13–14 are *limit* inputs rather than rejections: because the C has
no rejection logic, the boundary values of the parameter domain are the only
"edge" the API has, and the instructions require covering values at and one step
past the range regardless of whether the table names them.

## Status

All 16 rows have a passing differential test in `tests/phase_c_errors.rs`
(each row asserts the C `.so` and the Rust `.so` produce **byte-identical**
observable behaviour, and — where a failure is injected — that *both* survive it
and report it the same way, not merely that "both failed somehow").
