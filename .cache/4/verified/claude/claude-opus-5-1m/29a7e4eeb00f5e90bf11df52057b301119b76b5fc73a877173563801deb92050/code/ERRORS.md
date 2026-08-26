# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h` by
grepping for **every** rejection construct:

```sh
grep -nE "return|assert|NULL|if|switch|==|!=|ERROR|errno|exit|abort|malloc|free|\[|sizeof|#if" \
     c_src/src/driver.c c_src/include/driver.h
```

Findings (the complete list — nothing else in the library rejects anything):

* `driver.c:31` — `if (line != NULL)` in `printLine`. The library's **only**
  conditional and its **only** input validation.
* No `assert`, no `RETURN_ERROR`-style macro, no error enum, no `return -1` /
  `return NULL` (every function is `void` and has no `return` statement at all).
* No range checks, no min/max constants, no array indexing, no allocation, no
  `errno` use, no `exit`/`abort`, no `#ifdef` config branches.
* `bad`, `good`, `driver` take **no parameters** (`void`), so they have no
  input to reject; they are unconditional straight-line code.

## Error-surface rows

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| E1 | `printLine` | `line == NULL` (`driver.c:31` guard fails) | returns normally, **writes zero bytes** to `stdout`, no crash | [x] |
| E2 | `printLine` | `line` = empty string `""` (non-NULL, first byte `\0`) — passes the guard, so *not* rejected | prints just `"\n"` (1 byte) | [x] |
| E3 | `printLine` | `line` points at a string containing `printf` format directives (`"%s"`, `"%d"`, `"%n"`, `"%%"`) | printed **literally**; never interpreted as a format string (the format is the fixed `"%s\n"`) | [x] |
| E4 | `printLine` | `line` = maximum-ish length input (64 KiB, 1 MiB) — no length limit exists in C | prints all bytes + `'\n'`, no truncation | [x] |
| E5 | `printLine` | `line` contains non-ASCII / invalid-UTF-8 bytes (`0x80`–`0xFF`) and control bytes (`0x01`–`0x1F`, `0x7F`) | bytes copied through verbatim (no UTF-8 validation, no panic) | [x] |
| E6 | `printLine` | called repeatedly / with `stdout` fully buffered and redirected | all output ordered and flushed identically | [x] |
| E7 | `bad` | *(no parameters — nothing can be rejected)*; must **not** invoke the dead `static helperBad()` | prints exactly `"bad()\n"`; `"helperBad()"` never appears | [x] |
| E8 | `good` | *(no parameters)* | prints exactly `"good()\nhelperGood()\n"` | [x] |
| E9 | `driver` | *(no parameters; header declares `void driver(void)`, definition is `void driver()`)* — called with extra/garbage args across the FFI boundary is not expressible, but calling via the `void driver(void)` prototype must work | prints the fixed 6-line banner sequence | [x] |

### Generic FFI boundaries also covered (not in the C table)

| # | condition | expected | status |
|---|-----------|----------|--------|
| G1 | `printLine(NULL)` (the null-pointer case) — same as E1 | zero bytes, no crash | [x] |
| G2 | zero length: `printLine("")` — same as E2 | `"\n"` | [x] |
| G3 | oversized length: 1 MiB string | full passthrough | [x] |
| G4 | one step past a valid byte range: byte values `0x01`…`0xFF` exhaustively (the full non-NUL alphabet; `0x00` terminates and is not expressible) | passthrough | [x] |
| G5 | out-of-range enum across FFI | **not applicable**: the library declares no `enum` and no integer parameters at all. Documented here so the gap is explicit rather than overlooked. | [x] |
| G6 | unaligned / interior string pointers | passthrough (byte-oriented, alignment-agnostic) | [x] |
| G7 | interleaving of the two libraries' writes on the same `stdout` | identical byte streams per call | [x] |

Every row is exercised by `tests/error_paths.rs` (rows E1–E9, G1–G7) against
**both** `.so`s loaded via `libloading`.
