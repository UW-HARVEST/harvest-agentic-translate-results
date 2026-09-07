# ERRORS.md — Phase C error-surface table

## How this table was derived

Mechanical grep over the *entire* C source for every rejection construct:

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|return 0|assert|errno|if *\(|switch|#ifdef|#if |MAX|MIN|<=|>=|!=|== *NULL' \
        c_src/src/driver.c c_src/include/driver.h
(no matches — exit status 1)
```

**Zero hits.** The C library contains no `RETURN_ERROR` macro, no error enum, no
`assert`, no `errno` use, no explicit range check, no null check, and no
min/max constant. `foo` has exactly one `return` (`return res;`, an unsigned
count, never a negative sentinel) and `driver` returns `void`.

Consequently the error surface consists of (a) the single *sentinel-driven*
control path the code does contain — `strchr` returning `NULL` — and (b) the
generic C-API boundaries the task requires covering even when absent from the
source. Rows below are the complete set; none are invented beyond that.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `foo` | `strchr(s, c)` finds no occurrence on the first iteration (`c` absent from `in`) — the NULL sentinel that terminates the loop | returns `0`; no crash | `err_row1_no_occurrence_returns_zero` | [x] |
| 2 | `foo` | `in` points to the empty string `""` (zero-length input, only the NUL) with `c != 0` | returns `0` | `err_row2_empty_string_returns_zero` | [x] |
| 3 | `foo` | `c == '\0'`: `strchr(s, 0)` matches the *terminator*, so it can **never** return the NULL sentinel — the `for` loop has no exit condition. Each iteration does `s++` past the previous match and rescans, so the pointer walks out of the object indefinitely | **unbounded out-of-bounds walk terminating in a fatal `SIGSEGV`**; the function never returns. (Established empirically against the built C `.so`, not assumed: `foo("hello world", 0)` → killed by signal 11.) | `err_row3_nul_needle_matches_terminator` (out-of-process, zero-padded arena, `alarm()` bounded, compares the signal) | [x] |
| 4 | `foo` | `in == NULL` — `strchr(NULL, c)` dereferences the null page | fatal `SIGSEGV`; the process dies, no return value | `err_row4_null_in_foo_segv` (out-of-process, compares signal) | [x] |
| 5 | `driver` | `in == NULL` — reaches the same null dereference through `foo(in, 'A')`, so *nothing* is printed before the fault | fatal `SIGSEGV`, empty stdout | `err_row5_null_in_driver_segv` (out-of-process, compares signal + stdout) | [x] |
| 6 | `foo` | needle byte outside the ASCII range, i.e. a high-bit value (`0x80`–`0xFF`) that is *negative* as a signed `char` — the one-past-valid-range value class for this parameter | matched by byte value; count of that byte in `in` | `err_row6_high_bit_needle` | [x] |
| 7 | `foo` | needle passed across the FFI boundary as a full-width `int` with garbage above bit 7 (e.g. `0x11223341`) — the "C enums/narrow params accept any int" case: the callee's `char` parameter must observe only the low 8 bits | behaves exactly as `c == (char)(v & 0xFF)`; upper bits ignored | `err_row7_wide_int_needle_truncates` | [x] |
| 8 | `foo` | oversized input: a very long string (1 MiB) where the needle occurs a large number of times — checks the `int res` accumulator and pointer walk at scale | returns the exact occurrence count, no wrap | `err_row8_oversized_input` | [x] |
| 9 | `foo` | needle occurs as the very last byte before the terminator, so the post-match `s++` lands exactly *on* the NUL and the next `strchr` must stop there rather than run past it | returns the count; terminates cleanly (no OOB read) | `err_row9_match_at_last_byte` | [x] |

Not applicable / infeasible, recorded for completeness:

- Negative or oversized *length* arguments: the API takes no length parameter.
- Out-of-range enum values: the API declares no enum type. Row 7 covers the
  equivalent "any int across the ABI" hazard for the narrow `char` parameter.
- `int` overflow of `res`: would require > 2^31 occurrences (≥ 2 GiB input);
  not exercised, and identical in both implementations by construction
  (`wrapping_add` on `c_int` vs. C `int++`).

## Divergence found and fixed

One row failed initially, and only in one build configuration:

- **Rows 4 and 5 (NULL `in`), debug profile / `-Cdebug-assertions=on`.**
  C: `Signalled(11)` (`SIGSEGV`). Rust: `Signalled(6)` (`SIGABRT`), plus
  `null pointer dereference occurred` on stderr. Cause: with debug assertions
  enabled, rustc inserts a null/alignment check in front of the plain `*s`
  dereference in the `strchr` reimplementation, converting C's hardware fault
  into a Rust panic. Release passed because the check is compiled out.
  Fix (`src/lib.rs`, `strchr`): perform the load with
  `core::ptr::read_volatile(s)`, which issues the same byte load with no
  inserted check. Fault behaviour is now identical to C in the debug profile,
  the release profile, and release with `-Cdebug-assertions=on` forced.

All rows now pass in every configuration.
