# ERRORS.md — Error / rejection surface table

Mechanically derived from the complete C source (`c_src/src/driver.c`, 50 lines,
`c_src/include/driver.h`). Both public functions return `void`; the library has **no**
error codes, no error enums, no `RETURN_ERROR` macros, no `assert`s, no `return -1` /
`return NULL` statements. Grep evidence:

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|<|>|==|!=' c_src/src/driver.c
32:    if(line != NULL)          # the only null check
44:    if (data < 100)           # the only range check
```

Therefore every "rejection" in this library is expressed as *silently skipping work*
(and the observable result is the bytes written to `stdout` plus the process exit
status). One row per distinct rejection / guard branch, plus the generic FFI boundary
cases required by the task.

| #  | function    | trigger (exact invalid input / condition)                                   | expected C result                                                                                       | status |
|----|-------------|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------|--------|
| E1 | `printLine` | `line == NULL` (`driver.c:32` `if(line != NULL)` false branch)              | no `printf` call at all; **zero bytes** written to stdout; returns normally                             | [x] |
| E2 | `driver`    | `data >= 100` (`driver.c:44` `if (data < 100)` false branch), e.g. `100`     | copy skipped; `dest` stays the zero-initialised `""`; prints exactly `"\n"` (1 byte)                    | [x] |
| E3 | `driver`    | `data == 100` exactly — the boundary value one step past the valid range      | same as E2: `"\n"` only                                                                                  | [x] |
| E4 | `driver`    | `data == INT_MAX` (`2147483647`) — maximal oversized length                   | same as E2: `"\n"` only (no overflow, branch not taken)                                                  | [x] |
| E5 | `driver`    | `data == 99` — largest value that still enters the branch (off-by-one edge)   | `strncpy(dest, source, 99)` copies 99 `'A'` with **no** NUL from src, then `dest[99]='\0'`; prints 99 `'A'` + `"\n"` | [x] |
| E6 | `driver`    | `data == 0` — zero length                                                    | `strncpy(...,0)` copies nothing, `dest[0]='\0'`; prints `"\n"` (1 byte)                                  | [x] |
| E7 | `driver`    | `data < 0` (e.g. `-1`, `-2`, `INT_MIN`) — **not** rejected by `data < 100`;   | UB, reproduced verbatim: `data` is converted to `size_t` giving `SIZE_MAX`-ish `n`, so `strncpy` runs off the end of the stack buffer → process dies with **SIGSEGV (11)**, and **no** stdout output is produced | [x] |
|    |             | the C has *no* lower-bound check, so this is a real reachable input          | (verified: exit status 139 = 128+11 from a C driver program)                                             |     |
| E8 | `driver`    | `data == INT_MIN` (`-2147483648`) — extreme negative                          | same as E7: SIGSEGV, no output                                                                           | [x] |
| E9 | `printLine` | non-NUL-terminated / unterminated buffer is *not* checked                     | `printf("%s\n")` reads until the first `0` byte wherever it is — reproduced identically (test uses an explicitly terminated tail so the read is well-defined) | [x] |
| E10| `printLine` | empty string `""` (degenerate but valid pointer)                              | prints exactly `"\n"`                                                                                    | [x] |

## Notes on the FFI-boundary generic cases

* **Null pointers** — only `printLine` takes a pointer: row E1.
* **Zero length** — row E6.
* **Oversized length** — rows E2/E3/E4.
* **One step past a documented valid range** — row E3 (`100`) and, on the other side,
  row E7 (`-1`).
* **Out-of-range enum values** — the API declares **no enums** (grep: `grep -c enum
  c_src/src/driver.c c_src/include/driver.h` → 0), so there is no enum-crossing case.
  The nearest analogue is the unconstrained `int` parameter of `driver`, which is
  exhaustively covered by rows E2–E8 plus the randomised sweep in `CONFIGS.md`.
* Neither function returns a value, so "same error code" is asserted as
  *same stdout bytes* **and** *same process exit status / terminating signal*.

## Test mapping (all rows have a passing differential test)

| row | test |
|-----|------|
| E1  | `tests/phase_c_errors.rs::e1_print_line_null` (in-process **and** out-of-process) |
| E2  | `e2_driver_data_ge_100` |
| E3  | `e3_driver_boundary_exactly_100` |
| E4  | `e4_driver_int_max` |
| E5  | `e5_driver_99_off_by_one` |
| E6  | `e6_driver_zero_length` |
| E7  | `e7_driver_negative_ub_matches` (20 negative values, compares terminating signal + stdout) |
| E8  | `e8_driver_int_min` |
| E9  | `e9_print_line_no_length_check` |
| E10 | `e10_print_line_empty_string` |
| generic boundaries | `generic_int_boundary_sweep` (every power of two ±1 in `0..=INT_MAX`) |

All rows pass in every build configuration (dev/release × default/`--no-default-features`).
