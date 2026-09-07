# ERRORS.md — Error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/driver.c`. Every `if`, `else`, null check,
range check and named constant in the file:

```
32:    if(line != NULL)              <- null check (printLine)
46:    data = CHAR_MAX;              <- limits.h constant (bad)
47:    if(data > 0)                  <- positivity guard (bad), no else
58:    if(data > 0)                  <- positivity guard (goodG2B), no else
69:    data = CHAR_MAX;              <- limits.h constant (goodB2G)
70:    if(data > 0)                  <- positivity guard (goodB2G), no else
72:        if (data < (CHAR_MAX/2))  <- explicit RANGE CHECK (goodB2G)
77:        else                      <- the rejection branch
91:    if (useGood)                  <- mode dispatch (driver)
95:    else                          <- the other mode
```

There are **no** `assert`s, **no** `return -1` / `return NULL`, **no** error
enums, **no** `errno` use, and **no** allocation in this translation unit. All
functions return `void`, so every "rejection" is observable **only** through the
bytes written to `stdout` (or the absence of bytes). The tests therefore compare
captured `stdout` byte-for-byte, plus the fact that neither library crashes.

Named constants: `CHAR_MAX` = `127` (x86-64 Linux `char` is signed);
`CHAR_MAX/2` = `63` (integer division).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `printLine` | `line == NULL` (line 32 false branch) | silent no-op; **zero bytes** written; no crash | [x] |
| E2 | `printLine` | `line` points at an empty string `""` (degenerate, passes null check) | writes exactly `"\n"` (1 byte) | [x] |
| E3 | `printLine` | `line` contains `printf` conversion specifiers (`%s %n %d %%`) — argument, not format, so must NOT be interpreted | writes the literal bytes + `"\n"` | [x] |
| E4 | `printHexCharLine` | negative `char` (e.g. `-1`, `-2`, `-128`) — no validation; varargs promotes to `int` and `%02x` reinterprets as `unsigned int` | 8 hex digits, e.g. `-1`→`ffffffff`, `-2`→`fffffffe`, `-128`→`ffffff80`, + `"\n"` | [x] |
| E5 | `printHexCharLine` | `0` (below the `data > 0` guard that protects the callers; reachable only via the exported symbol) | writes `"00\n"` (`%02x` zero-pads) | [x] |
| E6 | `printHexCharLine` | every one of the 256 possible `char` bit patterns (`-128..=127`) | exact `%02x` rendering for each; `<16` zero-padded to 2 digits, negative widened to 8 digits | [x] |
| E7 | `bad` | line 47 `data > 0` FALSE branch — unreachable by design (`data` is hard-coded `CHAR_MAX` = 127) and there is **no `else`** | never taken; `bad()` always prints exactly one line | [x] |
| E8 | `bad` | the *overflow* itself: `CHAR_MAX * 2` = 254 truncated into `char` → `-2` (signed overflow, reproduced verbatim, NOT fixed) | prints `"fffffffe\n"` | [x] |
| E9 | `goodG2B` (via `good`) | line 58 `data > 0` FALSE branch — unreachable (`data` = 2), no `else` | never taken | [x] |
| E10 | `goodB2G` (via `good`) | line 70 `data > 0` FALSE branch — unreachable (`data` = `CHAR_MAX`), no `else` | never taken | [x] |
| E11 | `goodB2G` (via `good`) | **range check** line 72: `data < CHAR_MAX/2` i.e. `127 < 63` is FALSE → rejection `else` at line 77 | prints `"data value is too large to perform arithmetic safely.\n"` and performs no multiply | [x] |
| E12 | `goodB2G` (via `good`) | dead store: `data = ' '` (32, which *would* pass the range check) is overwritten by `data = CHAR_MAX` on line 69 before any read | the `' '` value must have **no** effect — output must be the rejection line, not `40` | [x] |
| E13 | `driver` | `useGood == 0` → `else` branch line 95 | dispatches `bad()`; output `"fffffffe\n"` | [x] |
| E14 | `driver` | out-of-range "enum-like" `int` values across FFI: `1, -1, 2, 42, 256, 0x10000, INT_MAX, INT_MIN, 0x7FFFFF00` (non-zero **whole int**, incl. values whose low byte is 0) | all are truthy → `good()`; must NOT be truncated to `char`/`bool` on the Rust side | [x] |
| E15 | `driver` | `useGood == 0` supplied as a *fresh* zero after a non-zero call (no latched state) | `bad()` again — the library is stateless | [x] |

Rows are checked off in `tests/differential.rs::phase_c_*` — each asserts the two
libraries produce the **same** byte stream (the same sentinel output / same
rejection message), not merely that both "did something".

## Divergence found and fixed

One real divergence was found by the generic FFI-boundary tests (not by any
happy-path row):

**`printHexCharLine` — upper argument-register bits.** gcc compiles the `char`
parameter as a narrowing read of `edi`:

```
mov    %edi,%eax
mov    %al,-0x4(%rbp)      <- truncate to the low byte
movsbl -0x4(%rbp),%eax     <- sign-extend it back
```

so a caller that leaves garbage in the upper 24 bits (for example one using a
mismatched `void(int)` prototype, the `char` analogue of an out-of-range enum
value) still observes only the low byte. The original Rust took the parameter as
`c_char`; rustc tags that `signext i8`, assumes the caller already extended it,
and forwarded the whole register:

```
mov    %edi,%esi           <- no truncation
```

`printHexCharLine(0x100)` therefore printed `100` in Rust but `00` in C.
Fixed in `src/lib.rs` by taking the parameter as `c_int` (the same register on
the SysV x86-64 ABI the C library is built for) and truncating with
`as c_char` before use; the Rust `.so` now emits `movsbl %dil,%esi`, matching
gcc for every input. Regression test: `phase_c_generic_char_arg_widening`.
