# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/driver.c` (63 lines) and
`c_src/include/driver.h`. Grep inventory of every rejection construct in the C
source:

```
$ grep -n 'return\|assert\|NULL\|-1\|== 0\|!= 1\|<\|>' c_src/src/driver.c
29:    for (int i = 0; i < len; i++)          # loop guard == implicit len<=0 rejection
35:    if (len == 0) return 0;                # explicit early-return
41:    for (int i = 0; i < len; i++)          # loop guard
47:    return out[len-1];
53:    for (i = 0; i < 100; i++)              # hard cap: MAX 100 parsed integers
55:    if (sscanf(in, "%d%zn", &data[i], &nb) != 1) break;   # parse rejection
```

There are **no** `assert`s, **no** error enums, **no** `RETURN_ERROR`-style
macros, **no** null-pointer checks and **no** `-1`/`NULL` sentinel returns in
this library. The complete rejection surface is: one explicit early-return, two
loop guards, one hard size cap, and one `sscanf` return-value check (which has
three distinguishable sub-cases). Constants: `100` (max integers parsed by
`driver`), `0` (the `len` value `call_fma` special-cases).

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `call_fma` | `len == 0` — explicit `if (len == 0) return 0;` guard at driver.c:35. Reached before any VLA is declared or any pointer is dereferenced, so `data` may even be NULL. | returns `0`; `data` never dereferenced | [x] |
| 2 | `call_fma` | `len == 0` **and** `data == NULL` (the guard must fire *before* the deref) | returns `0`, no crash | [x] |
| 3 | `fma_array` | `len == 0` — loop body never executes (driver.c:29) | returns void; writes nothing to `out`; `out`/`mul1`/`mul2`/`add` never dereferenced (may be NULL) | [x] |
| 4 | `fma_array` | `len < 0` (e.g. `-1`, `INT_MIN`) — loop guard `i < len` false on entry | returns void; writes nothing; no deref | [x] |
| 5 | `fma_array` | all four pointers NULL with `len <= 0` | returns void, no crash | [x] |
| 6 | `driver` | input contains **no** parseable integer at all (e.g. `""`, `"abc"`, `"+"`, `"-"`, `"  "`) — first `sscanf` returns `EOF` or `0`, `!= 1` → `break` with `i == 0` → `call_fma(data, 0)` → `0` | prints `"0\n"` | [x] |
| 6a | `driver` | sub-case: `sscanf` returns `EOF` (-1) — input is empty or pure whitespace, i.e. matching failure at end-of-input | prints `"0\n"` | [x] |
| 6b | `driver` | sub-case: `sscanf` returns `0` — matching failure on a non-numeric character (`"abc"`, `"x1"`, `"+z"`) | prints `"0\n"` | [x] |
| 6c | `driver` | sub-case: `%d` matches a lone sign then fails (`"+"`, `"-"`, `"- 5"`) → returns `0`/`EOF`, no integer stored | prints `"0\n"` | [x] |
| 7 | `driver` | input has a valid prefix then unparseable trailing garbage (`"1 2 xyz"`) — loop breaks mid-way, `i` = count of successes; result is `data[i-1]` | prints the LAST successfully parsed integer (`"2\n"`) | [x] |
| 8 | `driver` | input has **more than 100** integers — `for (i = 0; i < 100; ...)` cap at driver.c:53 stops the loop at `i == 100`; integers 101+ are ignored | prints the **100th** integer, not the last one | [x] |
| 9 | `driver` | input has **exactly 100** integers — boundary one step below the cap | prints the 100th integer | [x] |
| 10 | `driver` | input has **exactly 101** integers — boundary one step past the cap | prints the 100th integer (101st ignored) | [x] |
| 11 | `driver` | integer token overflows `int` (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) — glibc `%d` parses into a `long` and stores only the **low 32 bits**, still returning 1, so this is **NOT** a rejection | prints the truncated value. Captured from the C `.so`: `2147483648`→`-2147483648`, `-2147483649`→`2147483647`, `99999999999999999999`→`-1`, `-99999999999999999999`→`0`, `4294967296`→`0`, `4294967297`→`1`, 64 KiB of `9`→`-1` | [x] |
| 12 | `driver` | token is `"0x10"` — `%d` is base-10, so it parses `0` and stops at `'x'`; the *next* iteration fails on `'x'` | prints `"0\n"` | [x] |
| 13 | `driver` | embedded NUL truncates the input (`"1 2\0 3 4"`) — `sscanf` stops at the NUL terminator | prints `"2\n"` | [x] |
| 14 | `driver` | input is a zero-length string `""` (boundary: minimum length) | prints `"0\n"` | [x] |

## Documented-UB rows (asserted only for the Rust side; C is nondeterministic)

| # | function | trigger | why it is excluded from byte-equality |
|---|----------|---------|----------------------------------------|
| U1 | `call_fma` | `len < 0` | `int out[len]` with negative `len` is undefined behaviour (C11 6.7.6.2p5 requires a VLA size > 0). Probed empirically: C returns stack garbage (`164115346`, then `32767`, then nothing on `len=-100`) — a different value on every run. The Rust translation returns a deterministic `0`. Asserted: Rust must not crash and must return `0`; C is not compared. |
| U2 | `call_fma` | `len > 0` but `data` shorter than `len`, or `data == NULL` with `len > 0` | out-of-bounds read / NULL deref — UB in C, segfaults both implementations identically. Not asserted. |
| U3 | `driver` | `in == NULL` | glibc `sscanf(NULL, ...)` segfaults. Both crash. Not asserted. |
| U4 | `call_fma` | `len` so large the VLAs overflow the stack (e.g. `1 << 24`) | stack overflow is UB in C; Rust heap-allocates instead. Not asserted (the C path is unreachable from `driver`, which caps `len` at 100). |
| U5 | `fma_array` | `out` aliases `mul1`/`mul2`/`add` while `len > 0` | violates the `restrict` qualifier on `out` → UB. Not asserted. |
| U6 | `fma_array` | signed overflow in `mul1[i]*mul2[i] + add[i]` | signed overflow is UB in C, but gcc/clang emit wrapping `imul`/`add`. The Rust uses `wrapping_mul`/`wrapping_add`. This IS asserted for byte-equality in Phase B (row C7) because the codegen is stable and observable. |
