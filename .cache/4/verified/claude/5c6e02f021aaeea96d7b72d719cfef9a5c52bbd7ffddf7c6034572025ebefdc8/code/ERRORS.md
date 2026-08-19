# ERRORS.md — error / rejection surface table (Phase A, verified in Phase C)

Derived mechanically from `c_src/src/main.c` (85 lines, the whole library).
Exhaustive greps used:

```sh
grep -n -E '#if|#ifdef|#ifndef|assert|return -1|return NULL|errno|exit\(' \
        c_src/src/main.c c_src/CMakeLists.txt   # -> no matches (exit code 1)
grep -n -E 'if|return|NULL|<|>|<=|>=|==|!=' c_src/src/main.c
```

Findings: the C source contains **no** `assert`, **no** error enum, **no**
`RETURN_ERROR`-style macro, **no** `errno` use, **no** `exit()`, and **no**
`return -1` / `return NULL`. It contains exactly **two** conditional
statements (`if (line != NULL)` in `printLine`, `if (x)` in `main`), one
unchecked allocation per allocating function, and one library call whose failure
modes are observable (`scanf`). Each distinct rejection / failure condition
below is one row.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` (`if (line != NULL)` guard is false) | returns normally, prints **nothing** (0 bytes), no crash |
| 2 | `printLine` | `line` points at an empty string (`""`) — guard passes, `printf("%s\n","")`/`puts("")` | prints exactly `"\n"` (1 byte) |
| 3 | `printLine` | `line` contains `printf` conversion specifiers (e.g. `"%s %n"`) — passed as the *argument*, never as the format | bytes emitted verbatim + `"\n"`; no format interpretation, no crash |
| 4 | `printLine` | `line` contains non-UTF-8 / high bytes (e.g. `0xFF 0xFE 0x80`) | raw bytes emitted verbatim + `"\n"` (C is byte-oriented; must not be lossy-converted) |
| 5 | `printLine` | `line` longer than one stdio buffer (>4096 bytes, no interior newline) | full byte string + `"\n"` |
| 6 | `printIntLine` | `intNumber == INT_MIN` (`-2147483648`), the value with no positive counterpart | prints `-2147483648\n` |
| 7 | `printIntLine` | `intNumber == INT_MAX` (`2147483647`) | prints `2147483647\n` |
| 8 | `printIntLine` | out-of-range value passed across the FFI boundary as `int` (e.g. `0x80000000u` reinterpreted, `-1`) — C `int` accepts any 32-bit pattern | prints the two's-complement `%d` rendering of that bit pattern |
| 9 | `main` | stdin empty / immediate EOF → `scanf("%d", &x)` returns `EOF` (-1) and **does not assign** `x` | `x` keeps its initializer `0` → `bad()` runs → prints `0\n`, `main` returns `0` |
| 10 | `main` | stdin is whitespace only (`" \t\n"`) → whitespace skipped, then EOF → `scanf` returns `EOF`, no assignment | `x` stays `0` → `bad()` → `0\n`, returns `0` |
| 11 | `main` | stdin has non-numeric leading data (`"abc"`) → `scanf` **matching failure**, returns `0`, no assignment | `x` stays `0` → `bad()` → `0\n`, returns `0` |
| 12 | `main` | stdin is a lone sign (`"-"`, `"+"`, `"-x"`) → sign consumed, no digit → matching failure, returns `0` | `x` stays `0` → `bad()` → `0\n`, returns `0` |
| 13 | `main` | stdin value overflows `int` but fits `long` (`"4294967296"`) → `%d` is parsed by `strtol` into a `long`, then **truncated** into `int` (`0`) | truncated value `0` → `bad()` → `0\n`, returns `0` |
| 14 | `main` | stdin value overflows `long` (`"99999999999999999999"`) → `strtol` **saturates** to `LONG_MAX`, `ERANGE`, truncated to `int` (`-1`) | non-zero → `good()` → `0\n`, returns `0` |
| 15 | `main` | stdin value underflows `long` (`"-99999999999999999999"`) → saturates to `LONG_MIN`, truncated to `int` (`0`) | `0` → `bad()` → `0\n`, returns `0` |
| 16 | `main` | `x == 0` after the scan (any of rows 9–13, 15, or a literal `0`/`-0`/`000`) — the `if (x)` guard is false | takes the **`bad()`** branch: `alloca(10)` under-allocation, 40-byte write (CWE-131 / CWE-806 defect, undefined behaviour) | 
| 17 | `bad` | `alloca(10)` return value is used **without a NULL check**, and 10 `int`s (40 bytes) are written into a 10-byte allocation | no error is reported; C writes out of bounds and still prints `data[0]` == `0\n` (observed: no crash) |
| 18 | `good` | `alloca(10*sizeof(int))` return value is used **without a NULL check** (`data = NULL;` then immediately overwritten) | no error is reported; prints `0\n` |

## Generic FFI boundary cases also covered in Phase C

These are not distinct C branches but are the standard boundary inputs any C
ABI must tolerate; each has a differential test:

| # | case | expected |
|---|------|----------|
| G1 | `printLine(NULL)` (row 1, restated as the null-pointer case) | 0 bytes, no crash |
| G2 | `printLine("")` — zero length (row 2) | `"\n"` |
| G3 | `printLine(<64 KiB string>)` — oversized length (row 5) | full bytes + `"\n"` |
| G4 | `printIntLine` at each range endpoint and one step past the *decimal* boundaries (`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`) | `%d` rendering |
| G5 | `printIntLine` given a value with no "valid variant" — the C signature is `int`, so **every** 32-bit pattern is valid input; all 2^32 patterns are sampled randomly plus every power-of-two boundary | `%d` rendering |
| G6 | out-of-range *enum* values across FFI | **N/A — the C source declares no `enum` and no function taking one.** The only non-pointer parameter type in the whole API is `int` (`printIntLine`), which is covered by G4/G5; `bad`, `good` take no arguments and `main` takes none. |
| G7 | `bad()` / `good()` / `main` called repeatedly and interleaved (state leakage between calls) | identical output every time |
| G8 | write error on stdout: pipe with no reader | the C process is **killed by `SIGPIPE`** (wait status signal 13). Rust's runtime ignores `SIGPIPE` by default, so `src/main.rs` restores `SIG_DFL`; verified by CONFIGS.md row 38 |
| G9 | write error on stdout: `/dev/full` (`ENOSPC`) | `printf`'s return value is never checked -> exit 0, no diagnostic |
| G10 | read error on stdin: fd 0 closed, or a write-only descriptor (`EBADF`) | `scanf` reports `EOF`, `x` keeps `0` -> `bad()` -> `0\n`, exit 0 |

There are **no** `#ifdef`s in the C source and **no** `option()`/`if()` in
`c_src/CMakeLists.txt`, so this table is configuration-independent.
