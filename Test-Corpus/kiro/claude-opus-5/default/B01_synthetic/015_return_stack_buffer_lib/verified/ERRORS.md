# ERRORS.md — error / rejection surface of `c_src/src/driver.c`

Mechanically derived. The whole library is 68 lines with **one** translation
unit, so the rejection surface is small; the table below is exhaustive rather
than a selection. Grep evidence for the claim of exhaustiveness:

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|if *\(|switch|#if' c_src/src/driver.c
```

yields only:

* `if (line != NULL)` — the single guard in `printLine` (line 30)
* `return charString;` ×2 — value returns from the two static helpers, not error returns
* `if (useGood)` — the dispatch branch in `driver` (line 59)

There are **no** `RETURN_ERROR`-style macros, no error enums, no `assert`, no
`errno` use, no range checks, no min/max constants, no allocation (hence no
`return NULL` on failure), and no function in the library returns a status code —
every external-linkage function returns `void`. "Rejection" therefore means
*silently declining to print*, and it is observed through stdout.

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | guard `if (line != NULL)` fails → function returns, **zero bytes** written to stdout, no crash |
| 2 | `printLine` | `line` points at a zero-length string (`""`) — passes the null guard but has no payload | writes exactly one byte, `"\n"` |
| 3 | `printLine` | `line` non-null but the first byte is `\0` mid-buffer (embedded-NUL truncation) | writes only the bytes before the first `\0`, then `"\n"` |
| 4 | `bad` | *unconditional* — `helperBad()` returns the address of the automatic array `charString`, i.e. a dead stack object (CWE-562, UB). gcc 11.5.0 diagnoses `-Wreturn-local-addr` and materialises `0` for the return value at **every** optimisation level (`-O0 mov $0x0,%eax`, `-O1 mov $0x0,%edi`, `-O2/-O3/-Os xor %edi,%edi`) | `printLine(NULL)` → row 1 → **zero bytes** written to stdout |
| 5 | `driver` | `useGood == 0` | takes the `else` branch → `bad()` → row 4 → **zero bytes** written |
| 6 | `driver` | `useGood` is any non-zero `int` with no "valid variant" meaning — the parameter is a bare `int`, so out-of-range/enum-like values such as `2`, `-1`, `0x7FFFFFFF`, `0x80000000` are all accepted | C truthiness: every non-zero value takes the `if` branch → `good()` → writes `"helperGood1 string\n"` |
| 7 | `printLine` | non-terminated buffer / oversized length | *not checkable*: `printLine` takes no length argument and the C performs no bound check, so it reads until a `\0`. Both implementations must read to the same terminator; tested with buffers whose terminator sits at 1 B … 64 KiB and at stdio buffer boundaries (see `CONFIGS.md` rows 8–10) rather than by feeding genuinely unterminated memory (that would be UB in *both* objects and has no defined ground truth). |

## Generic FFI boundary cases also tested (Phase C)

| # | case | expectation |
|---|------|-------------|
| 8  | `printLine(NULL)` repeated / interleaved with successful calls | never emits output, never disturbs the surrounding byte stream |
| 9  | `driver` with the full set of `int` corner values `{0, 1, -1, 2, -2, i32::MIN, i32::MAX, i32::MIN+1, i32::MAX-1}` | `0` → silent, everything else → `"helperGood1 string\n"` |
| 10 | `driver` with randomised `i32` (fixed seed) | matches C for every value; only `0` is silent |
| 11 | misaligned / interior pointer (`&buf[k]` for arbitrary `k`) passed to `printLine` | no alignment requirement for `char*`; prints from that offset |
| 12 | `printLine` on a string containing printf format specifiers (`%s %n %d %%`) | the argument is *data*, never a format string (C passes it as the `%s` operand / to `puts`) → printed verbatim, no format-string interpretation, no crash |

## Ground-truth boundary for row 4

`helperBad` is undefined behaviour in C, so its "correct" result is whatever the
reference build produces. That was established by disassembly rather than
assumption, against the compiler the reference `CMakeLists.txt` actually picks up
(gcc 11.5.0; clang is not installed on this machine):

```
-O0   mov  $0x0,%eax          # helperBad returns 0
-O1   mov  $0x0,%edi          # inlined: printLine(NULL)
-O2   xor  %edi,%edi ; jmp printLine@plt
-O3   xor  %edi,%edi ; jmp printLine@plt
-Os   xor  %edi,%edi ; jmp printLine@plt
```

Every level funnels into `printLine(NULL)`, i.e. row 1, i.e. no output. If some
other toolchain instead returned the live stack address, the C would print
whatever bytes happened to remain in that frame — non-deterministic, so no
translation could be byte-identical to it. `NULL` is therefore both the observed
and the only reproducible ground truth, and `err04` asserts it explicitly on the
C side rather than only comparing the two implementations to each other.
