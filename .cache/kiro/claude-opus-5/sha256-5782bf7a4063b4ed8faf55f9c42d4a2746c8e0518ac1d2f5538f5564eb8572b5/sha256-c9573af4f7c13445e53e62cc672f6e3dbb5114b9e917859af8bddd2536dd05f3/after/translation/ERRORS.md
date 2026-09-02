# ERRORS.md — error / rejection surface

Derived mechanically. The entire library is `c_src/src/driver.c` (59 lines, 34
of which are the licence header). An exhaustive grep for every rejection
construct:

```sh
grep -nE 'assert|return|NULL|<|>|==|!=|\[|ERROR|errno|exit|abort' c_src/src/driver.c
# 26:#include <stdio.h>
# 30:    if (line != NULL)
```

That is the complete result. The C source contains:

* **one** conditional rejection: `if (line != NULL)` in `printLine` (line 30);
* **zero** `assert`s, `RETURN_ERROR`-style macros, error enums, `return -1`,
  `return NULL`, `errno` writes, `exit`/`abort` calls;
* **zero** range checks, min/max constants, array indexing, or length
  parameters;
* **zero** value-returning functions — all four exported functions are `void`,
  so there is no error code to compare. The *only* observable channel is bytes
  written to `stdout` (plus process termination), and every row below is
  asserted on exactly that channel, byte-for-byte, for both libraries.

`if (useGood)` in `driver` (line 49) is a **mode selection**, not a rejection;
it is covered by `CONFIGS.md` rows 12–19, not here.

## Rejection table

Rows 1–2 are the real rejection branches the C code contains. Rows 3–13 are the
generic C-API boundaries the task requires regardless of whether the source
checks them, including out-of-range values passed across the FFI boundary for
the `int` mode selector (a C `int` parameter accepts any 32-bit value, so a
value that is neither 0 nor 1 is a real input the C handles and the Rust must
handle identically).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` — the false arm of `if (line != NULL)` | returns silently; **zero bytes** written to stdout |
| 2 | `printLine` | `line != NULL` (true arm, for contrast — the branch that is *not* rejected) | `puts(line)`: the NUL-terminated bytes at `line`, then one `\n` |
| 3 | `bad` | uninitialized `char *data` (CWE-457) whose stack slot holds `0` | forwards `NULL` to `printLine`, which rejects it → **zero bytes**. Note: reached through `driver` in a *fresh* process the lazy PLT resolver overwrites that slot first, so the absolute expectation is only asserted once the PLT is bound |
| 4 | `bad` | uninitialized `char *data` whose stack slot holds a valid non-NULL pointer | forwards it → those bytes + `\n`; **no rejection**, the defect is reproduced not fixed |
| 5 | `driver` | `useGood == 0` (the false arm — selects the defective path) | behaves exactly as `bad()` (rows 3/4) |
| 6 | `driver` | `useGood == -1` (negative, one step below the "documented" 0/1 range) | non-zero → `good()` → `string\n` |
| 7 | `driver` | `useGood == 2` (one step past the documented valid range) | non-zero → `good()` → `string\n` |
| 8 | `driver` | `useGood == INT_MIN` (`-2147483648`) | non-zero → `good()` → `string\n` |
| 9 | `driver` | `useGood == INT_MAX` (`2147483647`) | non-zero → `good()` → `string\n` |
| 10 | `driver` | `useGood == 0x100` / `0x0000FF00` — **non-zero int whose low byte is 0**; catches a translation that tests `al`/a `bool` instead of the full `cmpl [rbp-4], 0` | non-zero → `good()` → `string\n` |
| 11 | `driver` | `useGood == 0xFFFFFF00u as i32` — non-zero, low byte 0, sign bit set | non-zero → `good()` → `string\n` |
| 12 | `printLine` | zero-length input: `line` points at `"\0"` (empty string, not NULL) | not rejected: `puts("")` → exactly one `\n` |
| 13 | `printLine` | oversized input: 1 MiB of non-NUL bytes then `\0` | not rejected: all 1 MiB + `\n` |
| 14 | `printLine` | wild / unmapped non-NULL pointer (e.g. `0x1`) — passes the NULL check, then dereferenced | **not rejected**: `puts` faults; process dies on `SIGSEGV` (tested in a forked child so the same fatal signal is observed from both libraries) |
| 15 | `printLine` | pointer to bytes containing `printf` conversion specifiers (`%s %n %d`) | not rejected and **not interpreted**: gcc lowers `printf("%s\n", line)` to `puts(line)`, so the specifiers are emitted literally |

## Status

All 15 rows are covered by `translation/tests/differential.rs`
(`phase_c_*` tests) and pass for both the C `.so` and the Rust `.so`:

- [x] 1 — `phase_c_row01_02_printline_null_vs_nonnull`
- [x] 2 — `phase_c_row01_02_printline_null_vs_nonnull`
- [x] 3 — `phase_c_row03_bad_with_null_residue`
- [x] 4 — `phase_c_row04_bad_with_valid_residue`
- [x] 5 — `phase_c_row05_driver_zero_is_bad`
- [x] 6..11 — `phase_c_row06_11_driver_out_of_range_selectors`
- [x] 12 — `phase_c_row12_printline_empty_string`
- [x] 13 — `phase_c_row13_printline_oversized`
- [x] 14 — `phase_c_row14_printline_wild_pointer_same_fatal_signal`
- [x] 15 — `phase_c_row15_printline_format_specifiers_not_interpreted`
