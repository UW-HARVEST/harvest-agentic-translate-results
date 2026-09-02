# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h` by
grepping for every rejection construct:

```sh
grep -nE 'return|NULL|assert|errno|exit\(|abort|-1|<=|>=|<|>|MAX|MIN|if |else|switch|case |#if' \
    src/driver.c include/driver.h
```

Full result set from that grep, classified:

| grep hit | file:line | is it a rejection? |
|----------|-----------|--------------------|
| `#include <stdio.h>`  | `driver.c:26` | no — include directive |
| `#include <stdlib.h>` | `driver.c:27` | no — include directive |
| `if (line != NULL)`   | `driver.c:31` | **YES — the only rejection in the library** |
| `#ifndef DRIVER_H_`   | `driver.h:24` | no — include guard |
| `#endif`              | `driver.h:29` | no — include guard |

So the entire library has **exactly one** input-rejection site.

## What the C provably does NOT contain

Establishing these absences matters as much as listing the one present check,
because each absence is itself a behaviour the Rust must reproduce (namely: do
not reject, do not validate, do not abort):

- no error-return macro (`RETURN_ERROR`, `CHECK`, `GOTO_FAIL`, …)
- no `return <value>` of any kind — **every one of the five functions returns
  `void`**, so there is no error code or sentinel channel at all
- no `return NULL` (no function returns a pointer)
- no error `enum`, no status type, no `errno` read or write
- no `assert` / `static_assert` / `abort` / `exit`
- no numeric range check, no `MIN`/`MAX` constant, no length or size parameter
- no `switch`, no `enum` parameter — therefore **no enum-valued argument can be
  passed across the FFI boundary at all**; the out-of-range-enum bug class is
  structurally impossible here (see row 5)
- no allocation, so no allocation-failure path
- `printf`'s own return value is discarded, so I/O failure is silently ignored
  (row 4)

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` (literal null pointer) | `if (line != NULL)` is false → `printf` is **not** called → function returns normally, **zero bytes** written to stdout. No crash, no diagnostic, no status. |
| 2 | `printLine` | `line` points to a zero-length string (`""`, i.e. first byte is `\0`) — passes the null check but carries no payload | Accepted, **not** rejected. `printf("%s\n", "")` → exactly one byte, `"\n"`. Confirms the guard tests the *pointer*, not emptiness. |
| 3 | `printLine` | `line` contains `printf` conversion specifiers (`%s`, `%n`, `%d`, `%%`) | Accepted, **not** rejected and **not** interpreted. `line` is the *argument*, never the format string — the format is the fixed literal `"%s\n"`. Bytes are emitted verbatim. (A translation that used `printf(line)` or a Rust `format!`-style path would diverge here; this row exists to catch that.) |
| 4 | `printLine` | stdout is closed / unwritable (e.g. fd 1 closed, or a full device) | `printf` fails and returns negative; the C **discards the return value**, so `printLine` still returns normally. Failure is silently swallowed. Rust must also not panic, abort, or report. |
| 5 | *(none)* | out-of-range `enum` value across the FFI boundary | **Not applicable — vacuously satisfied.** No function in this library takes an `enum`, an `int`, or in fact any parameter other than `printLine`'s single `const char *`. `bad`, `good`, and `driver` are all `void(void)`. There is no integer input whose value could fall outside a valid variant set. |
| 6 | `bad`, `good`, `driver` | any attempt to supply an invalid argument | **Structurally impossible.** All three take no parameters, so they have no input to reject and no rejection path. Each unconditionally executes its fixed sequence of `printLine` calls. Correct behaviour = never fail. |

### Rows 1–4 vs. the "generic boundaries" checklist

The task asks to additionally cover null pointers, zero and oversized lengths,
and values one past a valid range, even when absent from the table:

- **null pointer** → row 1 (the library's only real error path).
- **zero length** → row 2.
- **oversized length** → no length parameter exists; the analogue is an
  extremely long NUL-terminated buffer, covered as a *valid* input in
  `CONFIGS.md` rows 7–9 (including the 4 KiB / `BUFSIZ` stdio-buffer boundary,
  where a divergence in buffering would surface).
- **one past a valid range** → no range exists (no numeric parameter). The
  nearest boundary is the NUL terminator position itself; row 2 plus
  `CONFIGS.md` rows 4–6 pin that down, and the tests deliberately place a
  guard byte *after* the terminator to prove neither implementation reads past
  it.
- **out-of-range enum** → row 5, vacuous, justified above.

## Status

| row | test | result |
|-----|------|--------|
| 1 | `test_err_01_null_pointer` | PASS |
| 2 | `test_err_02_empty_string` | PASS |
| 3 | `test_err_03_format_specifiers_not_interpreted` | PASS |
| 4 | `test_err_04_stdout_closed_is_silently_ignored` | PASS |
| 5 | `test_err_05_no_enum_or_integer_input_exists` (documents vacuity; asserts the `void(void)` signatures are callable with no args) | PASS |
| 6 | `test_err_06_argless_functions_have_no_rejection_path` | PASS |
