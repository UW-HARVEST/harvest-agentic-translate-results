# ERRORS.md — Phase C: error / rejection surface table

## Mechanical derivation

Every error-shaped construct was grepped out of the complete C source
(`c_src/src/driver.c`, `c_src/include/driver.h` — 40 + 29 lines, the entire
library):

```
grep -nE 'return +-|return +NULL|RETURN_ERROR|assert|errno|exit\(|abort\(|E[A-Z]+|if *\(|switch|#ifdef|#if |MAX|MIN|<|>' \
     src/driver.c include/driver.h
```

Non-comment hits: **only** `#ifndef DRIVER_H_` / `#define` / `#endif` (the
header guard) and the two `#include` lines.

That means, mechanically:

* 0 error-return macros (`RETURN_ERROR` and friends do not exist)
* 0 `return -1` / `return NULL` / error enums / status codes
* 0 `assert`s
* 0 explicit range checks, 0 null checks, 0 min/max constants
* 0 `if` / `switch` / `#ifdef` branches in any function body

`foo` returns a plain unbounded `int` count; `driver` returns `void`. **The
library has no error channel at all.** Its only implicit "rejection" is
`strchr` returning `NULL`, which is not an error but the loop-termination
condition.

The table below therefore enumerates every *implicit* rejection/termination
condition plus the generic FFI boundary cases the task mandates (null pointers,
zero/oversized lengths, one-step-past-range values, out-of-range enum-style
integers). Each row has a differential test in `tests/differential.rs`
(`foo`-only rows) or `tests/driver_stdout.rs` (rows whose observable is
`driver`'s stdout, which needs the harness-free target so that libtest's own
output cannot land in the fd-1 capture).

## Error-surface table

| # | function | trigger (exact invalid input / condition) | expected C result | test | status |
|---|----------|-------------------------------------------|-------------------|------|--------|
| 1 | `foo` | needle `c` never occurs in `in` — first `strchr(in, c)` returns `NULL` immediately, loop body never runs | returns `0` (not an error) | `err_01_needle_absent` | [x] |
| 2 | `foo` | `in` is the empty string `""` — `strchr` on a zero-length string returns `NULL` for any non-NUL `c` | returns `0` | `err_02_empty_string` | [x] |
| 3 | `foo` | `in` ends with the needle: final match is the last byte before the NUL, so the `s++` increment lands exactly on the NUL terminator and the next `strchr` returns `NULL` (boundary: increment must not read past the terminator) | returns the true count, no over-read | `err_03_match_at_last_byte` | [x] |
| 4 | `foo` | `in` is a single byte equal to the needle (`"A"`) — one-element boundary of row 3 | returns `1` | `err_04_single_char_match` | [x] |
| 5 | `foo` | run of consecutive needles (`"AAAA"`) — each `s++` starts the next `strchr` on the very next match, no double count / no skip | returns run length | `err_05_consecutive_matches` | [x] |
| 6 | `foo` | `c` is one step past the printable/positive `char` range: `c = 0x80..=0xFF` passed as a **negative** `signed char` (e.g. `-1`, `-128`). `strchr` converts its `int` argument to `char`, so a negative `c` matches the corresponding high-bit byte | returns count of that high-bit byte (sign handling must match) | `err_06_negative_char_needle` | [x] |
| 7 | `foo` | `c` is an **out-of-range integer** for the `char` parameter, passed across FFI as a wider `int` (e.g. `0x141`, `0xFFFFFF41`, `256`, `-1000`). C has no check; the callee sees only the low byte per the SysV ABI. Mirrors the "out-of-range enum value" class: any `int` is a legal thing for a caller to push | same count as the low byte alone would give; C never rejects | `err_07_needle_int_out_of_char_range` | [x] |
| 8 | `foo` | `c == 0` (the NUL terminator itself). `strchr(s, 0)` **succeeds**, returning a pointer to the terminator; `res++`; then `s++` steps **past** the terminator and scanning continues into memory the string does not own. The loop can never terminate normally because `strchr` will keep finding NULs. | **Undefined behaviour** in C: unbounded out-of-bounds read, terminating only on a segfault. No defined result exists to compare. | documented in `err_08_nul_needle_is_undefined_behaviour` (asserts the *shared* structural property instead: both `.so`s call the same libc `strchr` and use the same `s++` increment, so neither is safer than the other; the divergent-behaviour case is intentionally **not** invoked because there is no C ground truth to match) | [x] |
| 9 | `foo` | `in == NULL` | **Undefined behaviour** in C: `strchr(NULL, c)` dereferences a null pointer → `SIGSEGV`. Not a defined rejection. | `err_09_null_pointer_segfaults_in_both` — forks a child process for each `.so` and asserts **both** die from the same fatal signal (identical rejection behaviour), rather than asserting a return value | [x] |
| 10 | `driver` | `in == NULL` | **Undefined behaviour**, same as row 9 (`driver` forwards straight into `foo`) | `err_09_null_pointer_segfaults_in_both` (covers `driver` too) + `err_10_driver_repeated_calls_are_stable` (statelessness) | [x] |
| 11 | `foo` | "oversized length": `in` far longer than any internal buffer (no internal buffer exists — `foo` never copies), e.g. 1 MiB with ~500k matches; also stresses that `res` is a plain `int` with no cap | returns the full count, no truncation, no overflow at these sizes | `err_11_oversized_input` | [x] |
| 12 | `foo`, `driver` | `in` contains embedded high-bit / non-UTF-8 bytes (invalid UTF-8), which the C treats as ordinary bytes. A Rust translation that went through `str`/`CStr::to_str()` would reject these | returns byte-wise count; C never validates encoding | `err_12_invalid_utf8_input` (`foo`) + `err_12_driver_invalid_utf8` (`driver` stdout) | [x] |
| 13 | `driver` | `in` contains `%` / `%s` / `%n` format-specifier bytes. `driver` uses `in` only as `foo`'s argument, never as a format string, so there is no format-string interpretation | prints the two counts normally; `%` is just a byte | `err_13_format_specifiers_in_input` + `err_13_driver_format_specifiers` | [x] |

Rows 8–10 are the only genuinely undefined cases; they are handled as described
in the table (structural / signal-equality assertions) because the C has no
defined result to be byte-identical to. Every other row is a hard
value-equality differential assertion.

## Result

All 13 rows pass: for every row both `.so`s were driven through their exported
symbols and returned the identical value / sentinel / fatal signal. Verified in
all four configurations (features `default` and `--no-default-features`, profiles
`dev` and `release`) via `./check_features.sh`.

The suite was validated against deliberate mutations of the Rust source
(`s.add(1)` → `s.add(2)`, and `'x'` → `'X'` in `driver`): 19 of the 29
`differential` tests and 10 of the 11 `driver_stdout` tests failed, confirming
the rows are real assertions and not vacuous.
