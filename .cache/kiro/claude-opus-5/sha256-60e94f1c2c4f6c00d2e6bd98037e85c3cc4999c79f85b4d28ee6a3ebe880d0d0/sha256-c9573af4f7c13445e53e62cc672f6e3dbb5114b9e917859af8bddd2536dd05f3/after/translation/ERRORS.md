# ERRORS.md — Error / rejection surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/driver.c`. Every guard, null check,
comparison against a limit constant, and every branch that *suppresses* output
is listed. Grep basis:

```sh
grep -n 'return\|assert\|NULL\|CHAR_MAX\|if *(\|else' c_src/src/driver.c
```

Findings of that grep:

* `return` statements: **none** (every function is `void`, falls off the end).
* `assert`: **none**.
* error enums / `-1` / `NULL` sentinels returned: **none** — the library has no
  error *return* channel at all. Its only observable output is the byte stream
  written to `stdout`.
* limit constants referenced: `CHAR_MAX` (`<limits.h>`, `= 127` for the
  platform's signed `char`), `CHAR_MAX/2` (`= 63`), literals `2` and `' '`.
* null check: `if(line != NULL)` in `printLine` (line 30).
* guards: `if(data > 0)` (lines 43, 55, 67), `if (data < (CHAR_MAX/2))`
  (line 69), `if (useGood)` (line 88).

Consequently "the same error/rejection" for this library means **the same
stdout byte stream (including the empty stream) and no crash**. Each row below
asserts C and Rust agree on that observable, byte-for-byte.

| # | function | trigger (exact invalid input / condition) | expected C result | [x] |
|---|----------|-------------------------------------------|-------------------|-----|
| E1 | `printLine` | `line == NULL` (the explicit `if(line != NULL)` null check fails) | rejected: function returns having written **0 bytes**; no crash | [x] |
| E2 | `printLine` | `line` points at an immediate NUL (`""`) — passes the null check, degenerate length | accepted: writes exactly `"\n"` (1 byte) | [x] |
| E3 | `printLine` | `line` contains `printf` conversion specifiers (`"%s %n %d %p"`). It is the *argument*, never the format, so no interpretation occurs | accepted: bytes copied verbatim + `"\n"` | [x] |
| E4 | `printLine` | `line` contains non-ASCII / non-UTF-8 bytes (e.g. `0x80 0xFF 0xFE`) | accepted: bytes copied verbatim + `"\n"`; no UTF-8 validation | [x] |
| E5 | `printLine` | oversized input: 64 KiB string with no interior NUL | accepted: all bytes + `"\n"`; no length cap in the C | [x] |
| E6 | `printLine` | interior NUL (`"ab\0cd"`): C stops at the first NUL | accepted: writes `"ab\n"` only — trailing bytes discarded | [x] |
| E7 | `printHexCharLine` | negative `char` (e.g. `-1`, `-128`): promoted to `int` by varargs, then read by `%02x` as `unsigned int` | accepted: prints 8 hex digits, e.g. `ffffffff`, `ffffff80` | [x] |
| E8 | `printHexCharLine` | `0` — degenerate/zero value, hits the `%02x` zero-pad path | accepted: prints `00` | [x] |
| E9 | `printHexCharLine` | value one step past the signed range as seen by the caller (`0x80 … 0xFF` passed as unsigned bytes) — C `char` is signed here, so these are *not* representable as positive | accepted: reinterpreted as negative, prints `ffffff80` … `ffffffff` | [x] |
| E10 | `printHexCharLine` | caller leaves the upper 24 bits of the argument register dirty (out-of-range int passed where `char` is declared) | accepted: callee sign-extends the low byte only (`movsbl`); upper bits ignored | [x] |
| E11 | `bad` | unreachable rejection branch: `if(data > 0)` with `data = CHAR_MAX`; the *false* arm can never be taken | the guard is always true → always prints the overflowed value; `127*2` truncates to `char` `-2`, which the varargs int promotion + `%02x` renders as `fffffffe` | [x] |
| E12 | `goodG2B` (via `good`) | unreachable rejection branch: `if(data > 0)` with `data = 2` | guard always true → prints `04` | [x] |
| E13 | `goodB2G` (via `good`) | **the library's one real range rejection**: `data = CHAR_MAX (127)` fails `data < (CHAR_MAX/2)` (`127 < 63` is false) | rejected: takes the `else` arm, prints the diagnostic line `data value is too large to perform arithmetic safely.` and performs **no** multiplication | [x] |
| E14 | `goodB2G` (via `good`) | dead store `data = ' '` (32) immediately overwritten by `data = CHAR_MAX` — the value 32 *would* satisfy `32 < 63`, so a translation that honoured the dead store would take the other branch | the dead store must have **no** effect; the `else` arm is taken (see E13) | [x] |
| E15 | `driver` | `useGood == 0` (the falsy selector) | dispatches to `bad()` → output `fffffffe\n` | [x] |
| E16 | `driver` | out-of-range / non-boolean enum-style selector across FFI: `-1`, `2`, `INT_MIN`, `INT_MAX`, `0x100`, `0xFFFFFF00`. C `if (useGood)` accepts any `int`; there is no valid-variant check | every non-zero value is truthy → dispatches to `good()`; only exact `0` selects `bad()` | [x] |
| E17 | `driver` | `useGood` whose *low byte* is zero but which is non-zero overall (`0x100`, `0x10000`, `INT_MIN`) — the classic truncation bug | still truthy → `good()`; a Rust translation testing only the low byte would wrongly call `bad()` | [x] |

All 17 rows have a passing differential test — see
`tests/differential.rs` (`phase_c_*` tests).
