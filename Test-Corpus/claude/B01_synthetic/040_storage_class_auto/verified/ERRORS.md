# ERRORS.md — error-surface table (Phase A / Phase C)

Mechanically derived from every statement of `c_src/src/main.c`:

```c
void driver(int x) {
    auto int y = 2*x;      /* signed overflow possible -- no check          */
    y += 300;              /* signed overflow possible -- no check          */
    printf("%d\n", y);     /* return value IGNORED -> no error propagation  */
}

int main() {
    int x = 0;             /* initial value observable on scanf failure     */
    scanf("%d", &x);       /* return value IGNORED -> x stays 0 on failure  */
    driver(x);
    return 0;              /* the ONLY exit status the program can return   */
}
```

Grep results for rejection constructs: **no** `assert`, **no** `return -1`,
**no** `return NULL`, **no** error enum, **no** `errno` check, **no** range
check, **no** null check, **no** min/max constant, **no** `#ifdef`. The entire
error surface therefore consists of

1. the failure modes of `scanf("%d", &x)` whose return value the C **ignores**
   (so every failure degrades to "`x` keeps its initial value `0`"),
2. the failure modes of `printf` (also ignored), and
3. unchecked signed-integer overflow in `driver`.

Both public entry points also have no pointer parameters and no enum
parameters, which bounds the generic-boundary surface (rows 24–26).

`driver(x)` prints `(int)(2*x + 300)` with wrap-around, so "expected C result"
below is given as the exact bytes printed on stdout plus the exit status.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `main` (`scanf`) | stdin is empty → immediate EOF, *input failure*, `scanf` returns `EOF`, ignored | `x` stays `0` → prints `300\n`, exit `0` |
| 2  | `main` (`scanf`) | stdin contains only whitespace (`" \t\n\v\f\r"`) then EOF → input failure | prints `300\n`, exit `0` |
| 3  | `main` (`scanf`) | first non-whitespace byte is not `[+-0-9]` (e.g. `"abc"`, `".5"`, `"x"`, `"/"`) → *matching failure*, returns `0`, byte pushed back | prints `300\n`, exit `0` |
| 4  | `main` (`scanf`) | `"+"` or `"-"` followed by EOF → input failure after partial match | prints `300\n`, exit `0` |
| 5  | `main` (`scanf`) | sign followed by a non-digit (`"+-3"`, `"- 5"`, `"+a"`, `"++"`) → matching failure | prints `300\n`, exit `0` |
| 6  | `main` (`scanf`) | fd 0 is closed / unreadable → `read` fails with `EBADF` → input failure | prints `300\n`, exit `0` |
| 7  | `main` (`scanf`) | non-ASCII / binary first byte (`0xFF`, `0x00`, `0x80`) → matching failure | prints `300\n`, exit `0` |
| 8  | `main` (`scanf`) | `"0x10"` — `%d` is decimal-only, conversion stops at `x` | parses `0` → prints `300\n` |
| 9  | `main` (`scanf`) | `"1e5"` — exponent not part of `%d` | parses `1` → prints `302\n` |
| 10 | `main` (`scanf`) | digits > `LONG_MAX` (`"9999…"`, `"9223372036854775808"`, `"1234567890…"`) → glibc saturates to `LONG_MAX`, then truncated to `int` = `-1` | prints `298\n` |
| 11 | `main` (`scanf`) | digits < `LONG_MIN` (`"-99999999999999999999"`, `"-9223372036854775809"`) → saturates to `LONG_MIN`, truncated to `int` = `0` | prints `300\n` |
| 12 | `main` (`scanf`) | `INT_MAX` < value ≤ `LONG_MAX` (`"2147483648"`, `"4294967296"`, `"4294967297"`) → `long`→`int` truncation mod 2³² | `2147483648`→`300\n`, `4294967296`→`300\n`, `4294967297`→`302\n` |
| 13 | `main` (`scanf`) | value < `INT_MIN` but ≥ `LONG_MIN` (`"-2147483649"`, `"-4294967296"`) → truncation | `-2147483649`→`298\n`, `-4294967296`→`300\n` |
| 14 | `main` (`scanf`) | valid digits immediately followed by garbage (`"12abc"`, `"1 2"`, `"5)"`) — conversion stops, remainder never read | uses the prefix: `12`→`324\n`, `1`→`302\n`, `5`→`310\n` |
| 15 | `main` (`scanf`) | `LONG_MAX`-overflowing digit run **longer than one stdio buffer** (≥ 4097 bytes of `9`) | prints `298\n` |
| 16 | `driver` | `x = INT_MAX` → `2*x` signed overflow (UB, wraps as compiled) | `-2` + `300` → `298\n` |
| 17 | `driver` | `x = INT_MIN` → `2*x` signed overflow | `0` + `300` → `300\n` |
| 18 | `driver` | `x ≥ 1073741824` (`2*x` overflows, e.g. `INT_MAX/2+1`) | wrapped `2*x` then `+300` |
| 19 | `driver` | `x ≤ -1073741824` (`2*x` underflows) | wrapped `2*x` then `+300` |
| 20 | `driver` | `x ∈ [1073741674, 1073741823]`: `2*x` fits but `y += 300` overflows | e.g. `1073741674` → `-2147483648\n` |
| 21 | `driver` | `printf` fails because fd 1 is closed (`EBADF`), return ignored | no output, exit `0` |
| 22 | `driver`/`main` | `printf`/flush hits a pipe whose reader is gone → `SIGPIPE` at its default disposition | process killed by signal 13 (no exit status) |
| 23 | `driver` (FFI) | argument bit pattern with the sign bit set / "out of range" value passed as `int` (`0x80000000`, `0xFFFFFFFF`) — every 32-bit pattern is a valid `int`, none is rejected | wrapped `2*x + 300` |
| 24 | `driver`/`main` (FFI) | null-pointer boundary: **not applicable** — neither exported function takes a pointer; asserted by inspecting the two prototypes | n/a (documented, no test possible) |
| 25 | `driver`/`main` (FFI) | out-of-range enum boundary: **not applicable** — no enum appears in `c_src/`; the only parameter is a plain `int`, exhaustively covered by rows 16–23 | n/a (row 23 is the `int` analogue) |
| 26 | `main` (FFI) | zero-length / oversized "length" boundary: **not applicable** — no length parameter exists; the input-size analogue is stdin length `0` (row 1) and stdin longer than the stdio buffer (row 15) | covered by rows 1 and 15 |

## Verification status — every row has a passing differential test

Each test constructs the exact invalid input/condition, calls **both** shared
libraries through their exported symbols (`libloading` + `dlsym`, one forked
child per call) and asserts the same stdout bytes *and* the same termination
(exit status or killing signal). Rows that need process-wide effects are
additionally verified against the two real executables.

| # | test | status |
|---|------|--------|
| 1 | `error_paths::row01_empty_stdin` | [x] PASS |
| 2 | `error_paths::row02_whitespace_only` (incl. 4095/4096/4097/9000-byte runs) | [x] PASS |
| 3 | `error_paths::row03_leading_non_numeric` (15 inputs) | [x] PASS |
| 4 | `error_paths::row04_sign_then_eof` (incl. sign at the chunk boundary) | [x] PASS |
| 5 | `error_paths::row05_sign_then_non_digit` (10 inputs) | [x] PASS |
| 6 | `error_paths::row06_stdin_closed` + `binary_diff::errors_row06_closed_stdin` | [x] PASS |
| 7 | `error_paths::row07_binary_first_byte` (`0xFF`, NUL, `0x80`, UTF-8, `0x7F`) | [x] PASS |
| 8 | `error_paths::row08_hex_prefix_stops_at_x` | [x] PASS |
| 9 | `error_paths::row09_exponent_not_consumed` | [x] PASS |
| 10 | `error_paths::row10_above_long_max` | [x] PASS |
| 11 | `error_paths::row11_below_long_min` | [x] PASS |
| 12 | `error_paths::row12_between_int_max_and_long_max` | [x] PASS |
| 13 | `error_paths::row13_between_long_min_and_int_min` | [x] PASS |
| 14 | `error_paths::row14_valid_prefix_then_garbage` | [x] PASS |
| 15 | `error_paths::row15_overflowing_digit_run_across_buffers` | [x] PASS |
| 16 | `error_paths::row16_driver_int_max` | [x] PASS |
| 17 | `error_paths::row17_driver_int_min` | [x] PASS |
| 18 | `error_paths::row18_driver_double_overflow_threshold` (+64 random) | [x] PASS |
| 19 | `error_paths::row19_driver_double_underflow_threshold` (+64 random) | [x] PASS |
| 20 | `error_paths::row20_driver_plus_300_overflow` | [x] PASS |
| 21 | `error_paths::row21_driver_closed_stdout` + `binary_diff::errors_row21_closed_stdout` | [x] PASS |
| 22 | `error_paths::row22_broken_pipe_signal` + `binary_diff::errors_row22_broken_pipe_kills_both` (C dies with signal 13; the Rust binary now does too) | [x] PASS |
| 23 | `error_paths::row23_ffi_out_of_range_bit_patterns` | [x] PASS |
| 24 | `error_paths::rows24to26_absent_boundaries_are_really_absent` — asserts structurally that the C source still has no pointer parameter | [x] PASS (vacuous) |
| 25 | same test — asserts the C source still contains no `enum` | [x] PASS (vacuous) |
| 26 | same test + `row01_empty_stdin` / `row15_…across_buffers` for the size analogues | [x] PASS |

### Fixes made to the Rust translation as a result

* **Row 22:** the Rust binary exited 0 on a broken pipe because Rust's runtime
  installs `SIG_IGN` for `SIGPIPE` before `main`, while a C program starts with
  the default disposition. `src/main.rs` now restores `SIG_DFL` for `SIGPIPE`,
  so both binaries are killed by signal 13 identically.
* **Row 21:** printing is done with `write!` + ignored error (like C's ignored
  `printf` return) instead of `println!`, which would panic on `EBADF`.
