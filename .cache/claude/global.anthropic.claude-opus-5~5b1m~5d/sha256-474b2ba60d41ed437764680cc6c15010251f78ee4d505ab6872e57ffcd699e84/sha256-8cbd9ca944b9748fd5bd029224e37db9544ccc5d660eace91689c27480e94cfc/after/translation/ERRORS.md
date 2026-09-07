# ERRORS.md — Error / rejection surface table

Mechanically derived from `c_src/src/driver.c`. This library has **no return
values at all** (every public function is `void`) and contains **no**
`return -1`, **no** `return NULL`, **no** error enum, and **no** `assert`.
Its entire "rejection" surface consists of *guard conditions that suppress
output* plus one *diagnostic-message* branch.

Therefore the observable "error result" for every row is defined as
**the bytes written to stdout** (nothing at all = silently rejected) — that is
the only channel through which this library reports anything.

Grep evidence (`grep -n 'NULL\|return\|assert\|CHAR_MAX\|> 0\|< (' src/driver.c`):

```
32:    if(line != NULL)            <- null-pointer rejection
46:    data = CHAR_MAX;
47:    if(data > 0)                <- positivity guard (bad)
58:    if(data > 0)                <- positivity guard (goodG2B)
69:    data = CHAR_MAX;
70:    if(data > 0)                <- positivity guard (goodB2G)
72:        if (data < (CHAR_MAX/2))<- explicit range check (goodB2G)
```

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` — the `if(line != NULL)` guard at driver.c:32 fails | Function returns having written **0 bytes**. No crash, no output. |
| 2 | `printLine` | `line` points at an empty string `""` (valid, but the degenerate/zero-length boundary of row 1) | Writes exactly one byte: `"\n"`. |
| 3 | `printLine` | `line` contains `printf` conversion specifiers (`%s`, `%d`, `%n`, `%%`) — must be treated as **data**, never as a format string | The literal bytes are printed verbatim followed by `\n`. No format-string interpretation, no crash. |
| 4 | `printLine` | `line` is a non-NUL-terminated-until-far-away / very long buffer (oversized length boundary) | Whole string printed verbatim + `\n`; length is irrelevant to the C code (`%s`/`puts` walk to the NUL). |
| 5 | `printLine` | `line` contains bytes `>= 0x80` (invalid UTF-8) — C `char` is byte-transparent, Rust must not validate UTF-8 | Raw bytes printed verbatim + `\n`. Must **not** reject or replace bytes. |
| 6 | `printLine` | `line` contains an embedded interior NUL (`"a\0b"`) | Printing stops at the first NUL: emits `"a\n"` only. Trailing bytes are silently dropped. |
| 7 | `printHexCharLine` | `charHex` is **negative** (`char` is signed on this platform, so any value `0x80..0xFF` / `-128..-1`). Default argument promotion widens it to a negative `int`, which `%02x` then reinterprets as `unsigned int`. | Prints **8** hex digits, e.g. `charHex == -2` → `"fffffffe\n"`, `-128` → `"ffffff80\n"`. It does **not** print 2 digits, and it does **not** error. This is the sign-extension trap the Rust must reproduce. |
| 8 | `printHexCharLine` | `charHex == 0` (zero boundary) | Zero-padding applies: `"00\n"`. |
| 9 | `printHexCharLine` | `charHex` in `1..=15` (values needing the `02` zero-pad) | Two digits with leading zero, e.g. `1` → `"01\n"`. |
| 10 | `printHexCharLine` | `charHex == CHAR_MAX` (`127`, one step past the largest value that survives `*2` in a `char`) | `"7f\n"`. |
| 11 | `bad` | *unconditional internal overflow*: `data = CHAR_MAX (127)`, passes `data > 0`, then `data * 2 == 254` is truncated on assignment to `char result`, yielding `-2` (CWE-197 truncation) | Always prints `"fffffffe\n"` (via row 7's sign-extension). Never prints `"fe\n"`. |
| 12 | `goodB2G` (via `good`/`driver(!=0)`) | the explicit range check `data < (CHAR_MAX/2)` i.e. `127 < 63` **fails** — the rejection branch | Prints the diagnostic line `"data value is too large to perform arithmetic safely.\n"` instead of a hex value. |
| 13 | `goodG2B` (via `good`/`driver(!=0)`) | positivity guard `data > 0` with `data = 2` — the **accepted** side of the guard (control row proving the guard is not inverted) | Prints `"04\n"`. |
| 14 | `driver` | `useGood == 0` — the false branch, dispatches to `bad()` | Exactly row 11's output: `"fffffffe\n"`. |
| 15 | `driver` | `useGood != 0` with an *out-of-range / non-boolean* `int`: `-1`, `2`, `INT_MIN`, `INT_MAX`, `0x100` (a C `int` parameter accepts **any** `int`, incl. values with no "valid" boolean meaning) | All non-zero values take the `good()` branch identically: `"04\ndata value is too large to perform arithmetic safely.\n"`. Notably `INT_MIN` and `0x100` must **not** be mistaken for false by a narrowing (`as i8`/`as u8`/`!= 0` on a truncated byte) bug. |
| 16 | `driver` | `useGood == 0x100` / `0x10000` — non-zero values whose **low byte is zero** (the specific narrowing-bug probe for row 15) | Takes the `good()` branch, **not** `bad()`. |

## Checklist

- [x] 1 `printLine(NULL)` → no output
- [x] 2 `printLine("")` → `"\n"`
- [x] 3 `printLine` format specifiers treated as data
- [x] 4 `printLine` very long string
- [x] 5 `printLine` high/non-UTF-8 bytes
- [x] 6 `printLine` interior NUL truncates
- [x] 7 `printHexCharLine` negative → 8 hex digits
- [x] 8 `printHexCharLine(0)` → `"00\n"`
- [x] 9 `printHexCharLine(1..15)` zero-padded
- [x] 10 `printHexCharLine(127)` → `"7f\n"`
- [x] 11 `bad()` truncation overflow → `"fffffffe\n"`
- [x] 12 `goodB2G` range-check rejection message
- [x] 13 `goodG2B` accepted guard → `"04\n"`
- [x] 14 `driver(0)` → bad branch
- [x] 15 `driver(non-zero incl. INT_MIN/INT_MAX)` → good branch
- [x] 16 `driver(0x100)` low-byte-zero narrowing probe → good branch
