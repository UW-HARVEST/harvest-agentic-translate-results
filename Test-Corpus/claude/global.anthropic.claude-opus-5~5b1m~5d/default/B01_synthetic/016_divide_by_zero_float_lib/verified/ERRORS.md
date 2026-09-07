# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived from `c_src/src/driver.c`. Every function in this library
returns `void`, so there are **no error codes, no sentinel returns, no
`RETURN_ERROR` macros, no `assert`s, and no error enums**. Grep confirms it:

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|-1' c_src/src/driver.c
32:    if(line != NULL)
$ grep -nE 'if|switch|\?|#if' c_src/src/driver.c
32:    if(line != NULL)          <-- guard 1
61:    if (fabs(data) > 0.000001) <-- guard 2
```

So the C library has exactly **two explicit guards** plus a set of
*unguarded* paths where the input drives the code into C undefined /
implementation-defined behaviour. The observable behaviour of those unguarded
paths is part of the ground truth and is enumerated here too, because a caller
can reach them and the Rust must match byte-for-byte.

The single magic constant in the source is `0.000001` (line 61). There are no
other min/max constants, no length limits, and no allocation.

Observable "result" for every row = the exact bytes written to `stdout`.

`INT_MIN` below means the four-byte-decimal text `-2147483648`.

## Explicit guards

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` (`if(line != NULL)` fails, line 32) | returns immediately, writes **nothing** (0 bytes) |
| 2 | `goodB2G` (reached via `good`, `driver`) | `fabs((double)data) > 0.000001` is false, i.e. `data == 0.0f` | takes `else`, prints `This would result in a divide by zero\n` |
| 3 | `goodB2G` via `good`/`driver` | `data == -0.0f` (`fabs` → `+0.0`, not `> 1e-6`) | `This would result in a divide by zero\n` |
| 4 | `goodB2G` via `good`/`driver` | `data` = NaN — every `>` comparison with NaN is false, so the guard *fails* | `This would result in a divide by zero\n` (NOT the division path) |
| 5 | `goodB2G` via `good`/`driver` | `data == 1e-6f` = `9.99999997475e-7` as `double`, which is `< 0.000001` — the exact off-by-one-ULP boundary *below* the threshold | `This would result in a divide by zero\n` |
| 6 | `goodB2G` via `good`/`driver` | `data == 1.0000001e-6f` (bits `0x358637BE`, = `nextafter(1e-6f, +inf)`) — one ULP *past* the threshold, guard passes | division path, prints `99999988\n` (verified against the compiled C `.so`) |
| 7 | `goodB2G` via `good`/`driver` | `data` = smallest positive subnormal `1e-45f`, guard fails | `This would result in a divide by zero\n` |
| 8 | `goodB2G` via `good`/`driver` | `data == -1e-7f` (negative, magnitude under threshold) | `This would result in a divide by zero\n` |

## Unguarded paths (`bad` has **no** guard — this is the injected defect)

`bad` computes `(int)(100.0 / data)` with no check at all (lines 43–47).
Division by zero and out-of-range float→int conversion are C UB; the ground
truth is whatever the compiled C `.so` does on this target (x86-64,
`cvttsd2si`, which yields the "integer indefinite" value `INT_MIN`).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 9  | `bad` | `data == 0.0f` → `100.0/0.0` = `+inf`, `(int)+inf` out of range | prints `-2147483648\n` (`INT_MIN`) |
| 10 | `bad` | `data == -0.0f` → `-inf` | prints `-2147483648\n` |
| 11 | `bad` | `data` = NaN → quotient NaN, `(int)NaN` | prints `-2147483648\n` |
| 12 | `bad` | `data` = tiny subnormal `1e-45f` → `~1e47`, overflows `int` | prints `-2147483648\n` |
| 13 | `bad` | `data == 4.65661287e-8f` (bits `0x33480000`) → quotient exactly `2147483648.0` = `INT_MAX+1`, one step past the representable range | prints `-2147483648\n` (indefinite). One ULP up (`0x33480001`, q `2147483484.16`) prints `2147483484\n` — the guard is exact |
| 14 | `bad` | `data == -4.65661287e-8f` (bits `0xB3480000`) → quotient exactly `-2147483648.0`, the lowest *still-valid* value (boundary from the other side) | prints `-2147483648\n` (in-range, genuine result — must not be treated as overflow) |
| 15 | `bad` | `data == +inf` → `100.0/inf` = `+0.0` | prints `0\n` |
| 16 | `bad` | `data == -inf` → `-0.0`, `(int)-0.0` | prints `0\n` (not `-0`) |
| 17 | `bad` | `data` = negative NaN / signalling NaN bit patterns (e.g. `0xFFC00000`, `0x7FA00000`) | prints `-2147483648\n` |
| 18 | `bad` | truncation-toward-zero of a negative quotient, `data == -3.0f` → `-33.333…` | prints `-33\n` (truncate, not floor `-34`) |
| 19 | `driver` | `badData` invalid (any of rows 9–17) while `goodData` valid | the `good` output is still produced first, then the `bad` line; ordering must match |
| 20 | `driver` | both `goodData` and `badData` invalid | rows 2–8 output followed by rows 9–17 output, in C's order |

## Generic FFI boundary cases (covered even though not in the C source)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 21 | `printLine` | `line` = `""` (pointer to a lone NUL) — zero length | prints a single `\n` |
| 22 | `printLine` | `line` containing `%d`, `%s`, `%n` — format specifiers in the *argument*, which `printf("%s\n", line)` must NOT interpret | prints the literal text + `\n` |
| 23 | `printLine` | very long string (oversized length, 64 KiB) crossing `stdio` buffer boundaries | prints the whole string + `\n` |
| 24 | `printLine` | non-ASCII / high bytes `0x80`–`0xFF` (invalid UTF-8) — Rust must not validate | prints the raw bytes + `\n` |
| 25 | `printIntLine` | `INT_MIN` and `INT_MAX` — extremes of the value range | prints `-2147483648\n` / `2147483647\n` |
| 26 | `printIntLine` | `0` | prints `0\n` |
| 27 | *(enums)* | the public API declares **no enum type** — `driver.h` exposes only `float`/`int`/`const char*`. The analogue of "out-of-range enum value" here is an out-of-range/non-canonical *float* bit pattern, covered by rows 11, 12, 13, 17. | n/a |
| 28 | *(null)* | `printLine` is the only pointer-taking entry point; its NULL case is row 1. No other function accepts a pointer, so there is no further null-pointer surface. | n/a |
