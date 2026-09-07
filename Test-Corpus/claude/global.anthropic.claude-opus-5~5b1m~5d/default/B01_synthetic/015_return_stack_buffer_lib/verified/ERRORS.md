# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived by grepping **all** of `c_src/src/driver.c` and
`c_src/include/driver.h` for every rejection construct. Exhaustive grep result:

```
$ grep -nE 'return|NULL|assert|if *\(|switch|errno|exit|abort|-1|<|>|==|!=' c_src/src/driver.c
30:    if (line != NULL)          <- the ONLY null check / guard in the library
32:        printf("%s\n", line);
39:    return charString;         <- returns address of automatic array (CWE-562)
44:    printLine(helperBad());
50:    return charString;         <- returns address of static array (well defined)
55:    printLine(helperGood1());
60:    if (useGood)               <- the ONLY branch on caller-supplied data
62:        good();
66:        bad();
```

Findings, stated precisely so the table below cannot be padded with invented rows:

* There is **no error enum, no error code, no sentinel return value** anywhere:
  every public function returns `void` (`printLine`, `bad`, `good`, `driver`).
* There is **no `assert`**, no `errno` use, no `exit`/`abort`, no allocation and
  therefore no allocation-failure path.
* There is **no range check, no min/max constant, no size/length parameter** and
  no `#ifdef`. `driver.h` declares a single function.
* Consequently the library's *entire* rejection surface is **one** guard: the
  `line != NULL` test in `printLine`. Rejection is expressed as **silence**
  (produce no output and return normally) rather than as a returned error.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` (`driver.c:30` guard fails) | `printf` is **not** called: zero bytes written to `stdout`, function returns normally (`void`), no crash, no `errno` change | [x] |
| 2 | `bad` | *unconditional* — `helperBad()` (`driver.c:36-40`) returns the address of the automatic array `charString`, i.e. a dangling pointer (CWE-562 "Return of Stack Variable Address"; GCC diagnoses it as `-Wreturn-local-addr`). GCC folds the return value to `NULL` (`117f: mov $0x0,%eax` in the reference `.so`), so `bad()` reaches error row #1 from the inside: `printLine(NULL)` | zero bytes written to `stdout`; the text `"helperBad string"` **never** appears in the output; returns normally, no crash | [x] |
| 3 | `driver` | `useGood == 0` (C falsity) — routes to the defective `bad()` path | zero bytes on `stdout` (per row #2); returns normally | [x] |

## Generic FFI boundary conditions (covered even though the C has no check for them)

| # | function | trigger | expected C result | status |
|---|----------|---------|-------------------|--------|
| 4 | `printLine` | NULL pointer (explicit re-test of row #1 through the `.so` export) | no output, returns | [x] |
| 5 | `printLine` | zero-length string — a valid non-NULL pointer to a lone `'\0'` | exactly one byte, `"\n"` (`puts("")`) — the empty string is **accepted**, not rejected; the guard is on the *pointer*, never on the length | [x] |
| 6 | `printLine` | oversized input: `1`, `4095`, `4096`, `4097`, `65535`, `1 MiB` byte strings | full string + `'\n'`; no truncation and no length limit exists in the C | [x] |
| 7 | `printLine` | non-UTF-8 / arbitrary binary payload (bytes `0x01..=0xFF`, high bytes, invalid UTF-8 sequences such as `0x80 0xFE 0xFF`, `char` values that are negative when `char` is signed) | bytes emitted verbatim + `'\n'`; the C never validates encoding (a Rust translation that routed through `str`/`String` would panic or lose bytes here) | [x] |
| 8 | `printLine` | embedded interior `'\0'` (pointer to `"aaa\0bbb"`) | output stops at the first NUL (`puts` semantics): `"aaa\n"` | [x] |
| 9 | `printLine` | payload containing `'\n'`, `'%'`, `"%s"`, `"%n"`, `"%d"` format-specifier bytes | emitted **verbatim** + `'\n'` — the data is the `%s` argument, never the format string; `%n` must not be interpreted | [x] |
| 10 | `printLine` | non-null but wildly out-of-range / unmapped pointer | undefined behaviour in C (segfault) — **intentionally not tested**; it is not a rejection the C library defines, and asserting on UB would assert on nothing. Row #1 covers the only pointer value the C actually inspects | n/a |
| 11 | `driver` | value one step past / outside the "documented" range. `useGood` is a bare `int` used only for C truthiness, so *every* `int` is in range. Tested: `0`, `1`, `-1`, `2`, `-2`, `i32::MIN`, `i32::MAX`, `i32::MIN+1`, `i32::MAX-1`, `0x8000_0000u32 as i32`, `0x0001_0000`, `0xFFFF_0000u32 as i32`, `0x7FFF_FFFE`, plus randomised values | any non-zero value (incl. negative, incl. `INT_MIN`) → `good()` output; **only exactly `0`** → `bad()` (silence) | [x] |
| 12 | `driver` | out-of-range *enum-style* values across the FFI boundary: `useGood` treated as if it were an enum with variants `{0, 1}` and passed `2`, `3`, `255`, `256`, `-1`, `i32::MIN`, `i32::MAX`. C enums/`int` params accept any `int`, so these are real inputs. A Rust translation using `match useGood { 0 => .., 1 => .. }` or a `#[repr(C)] enum` + `transmute` would be wrong/UB here | all non-zero values behave exactly like `1` (`good()`); no panic, no `unreachable!()` | [x] |
| 13 | `driver` | low 32 bits zero but garbage in the upper half of `rdi` (caller passes a 64-bit value whose low `int` is `0`) | `useGood` is `int`: only `%edi` is examined → `bad()` path (silence) | [x] |
| 14 | `bad` / `good` / `driver` | repeated invocation (100×), and interleaved `good`/`bad`/`printLine` call sequences | fully idempotent: `good()` prints the same 19 bytes every time (the `.data` copy is never mutated), `bad()` stays silent every time; no state leaks between calls | [x] |
| 15 | `good` | called after `printLine` has been handed the same underlying static buffer, and after `bad()` — i.e. any ordering | unchanged `"helperGood1 string\n"` | [x] |

All 15 rows have a passing differential test in `tests/differential.rs`
(`phase_c_*`); rows are checked off only after the test passed against **both**
`.so` files. Row 10 is documented-as-untestable UB rather than checked, with the
reason recorded above.

## Robustness of row #2 (the only UB in the library)

Row #2 is the single place where the C's behaviour is not guaranteed by the
standard, so the equivalence was checked against more than one C build rather
than assumed from one disassembly. `scripts/opt_levels.sh` rebuilds
`c_src/src/driver.c` at `-O0 -O1 -O2 -O3 -Os` and compares the observable output
of `bad()`, `good()`, `driver(0)`, `driver(1)` against the Rust `.so`:

```
C gcc -O0              [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
C gcc -O1              [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
C gcc -O2              [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
C gcc -O3              [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
C gcc -Os              [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
C cmake (reference)    [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
Rust debug             [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
Rust release           [bad][good]helperGood1 string[d0][d1]helperGood1 string[end]
OK: identical observable output across all builds
```

`[bad]` and `[d0]` are immediately followed by the next marker at every level:
the defective path emits nothing in **any** C build, and the Rust matches. The
`static`-storage `good()` path prints its 19 bytes in all builds.
