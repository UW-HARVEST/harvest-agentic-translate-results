# ERRORS.md — Error / rejection surface of `c_src`

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## How this table was derived

```sh
grep -nE 'return|NULL|assert|ERROR|errno|exit|abort|if|switch|<|>|==|!=' c_src/src/driver.c
grep -oE '[0-9]+' c_src/src/driver.c | sort -un      # -> 0 1 100 2025(copyright)
```

Findings of that grep, exhaustively:

| construct in C | line | kind |
|---|---|---|
| `if(line != NULL)` | `driver.c:32` | null check (guards the only output statement of `printLine`) |
| `if (data < 100)` | `driver.c:44` | signed range check (guards the copy block of `driver`) |
| `char source[100]` / `char dest[100]` | `driver.c:40-41` | the only size constants: **max = 100**, derived index bound **99** |
| `memset(source, 'A', 100-1)` | `driver.c:42` | fixed length, no check |
| `source[100-1] = '\0'` | `driver.c:43` | fixed index, no check |
| `dest[data] = '\0'` | `driver.c:47` | **unchecked** index (only lower-bounded by nothing, upper-bounded by the `data < 100` guard) |

There are **exactly two conditionals** in the whole library.

Absent by grep (so no rows can be derived from them):

* no `assert`, no `RETURN_ERROR`-style macro, no `errno` use, no `exit`/`abort`;
* no `return -1` / `return NULL` — **both public functions return `void`, so the
  library has no error-code or sentinel channel at all**;
* no `enum` and no `switch` anywhere in the header or the source, therefore
  there is no enum whose out-of-range integer values could be smuggled across
  the FFI boundary. The equivalent "any int is accepted" surface is `driver`'s
  `int data` parameter, which is covered exhaustively at its boundaries below
  (rows 2–9) — including values that carry no meaningful interpretation.

Because there is no error-return channel, a "rejection" in this library is
observable only as **the absence of the guarded side effect** (nothing written
to `stdout`, or an empty line written), or — for the unchecked negative
`data` — as **process termination by signal**. The differential tests assert on
exactly those observables, not on "both failed somehow".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `printLine` | `line == NULL` → `if(line != NULL)` at `driver.c:32` is false | `printf` is **not** called; **0 bytes** written to `stdout`; returns normally | `err_01_printline_null` | [x] |
| 2 | `driver` | `data == 100` — the exact first value rejected by `if (data < 100)` at `driver.c:44` | copy block skipped; `dest` stays the zero-initialised `""`; `printLine` prints **exactly `"\n"` (1 byte)**; exit 0 | `err_02_driver_data_eq_100` | [x] |
| 3 | `driver` | `data == 101` — one step past the boundary | same as row 2: **`"\n"` (1 byte)**, exit 0 | `err_03_driver_data_101` | [x] |
| 4 | `driver` | `data == INT_MAX` (`2147483647`) — maximum rejected value / "oversized length" | same as row 2: **`"\n"` (1 byte)**, exit 0 | `err_04_driver_data_int_max` | [x] |
| 5 | `driver` | `data` anywhere in `[100, INT_MAX]` (randomised sweep of the rejected half-line) | same as row 2 for every value: **`"\n"` (1 byte)**, exit 0 | `err_05_driver_rejected_range_random` | [x] |
| 6 | `driver` | `data == -1` — **unchecked**: passes the *signed* `data < 100` guard, then the implicit `int → size_t` conversion of `strncpy`'s 3rd argument sign-extends to `0xFFFF_FFFF_FFFF_FFFF` | `strncpy`'s NUL-padding walks off the end of `dest` → **process killed by `SIGSEGV` (11)**; `dest[data]` at `driver.c:47` is never reached | `err_06_driver_data_neg1` | [x] |
| 7 | `driver` | `data == -2` (second negative, confirms it is the sign and not the single value `-1`) | **`SIGSEGV` (11)** | `err_07_driver_data_neg2` | [x] |
| 8 | `driver` | `data == INT_MIN` (`-2147483648`) — one step past the low end of the range, worst-case sign extension to `0xFFFF_FFFF_8000_0000` | **`SIGSEGV` (11)** | `err_08_driver_data_int_min` | [x] |
| 9 | `driver` | `data` anywhere in `[INT_MIN, -1]` (randomised sweep of the negative half-line) | **`SIGSEGV` (11)** for every value | `err_09_driver_negative_range_random` | [x] |

### Notes on rows 6–9

`driver.c` deliberately performs **no** lower-bound check on `data` (this is the
CWE-listed defect the sample encodes). Per the task rules the Rust translation
reproduces it verbatim: the guard stays the signed `data < 100`, and `data as
usize` sign-extends exactly like the C implicit conversion. The differential
tests therefore run the call **in a forked child** and assert that the C `.so`
and the Rust `.so` die from the **same signal number**, not merely that both
died.

### Generic FFI boundaries also covered (beyond the table)

| boundary | where covered | note |
|---|---|---|
| null pointer | row 1 | `printLine` is the only function taking a pointer; `driver` takes none |
| zero length | `CONFIGS.md` row 3 (`data == 0`) | `strncpy(dest, source, 0)` copies nothing, `dest[0] = '\0'` → `"\n"` |
| oversized length | rows 3, 4, 5 | everything `>= 100` |
| one step past valid range | row 2 (`100`, one past `99`) and row 6 (`-1`, one below `0`) | both ends of the valid window |
| out-of-range enum value | n/a — **no enum exists in the C API** | the analogous full-`int` surface is swept in rows 5 and 9 and in `CONFIGS.md` |
