# ERRORS.md — Error / rejection surface table

Mechanically derived from `c_src/src/driver.c` (99 lines) and
`c_src/include/driver.h`. Every branch in the C source that *rejects*,
*guards against*, or *diverts* an input is one row.

## Mechanical inventory of the C source

```
$ grep -n 'return\|assert\|NULL\|== \|!= \|< \|> \|MAX\|MIN\|if\s*(\|else' c_src/src/driver.c
32:    if(line != NULL)              <- null-pointer guard
40:    printf("%02x\n", charHex);    <- no guard at all
46:    data = CHAR_MAX;              <- limits.h boundary constant
47:    if(data > 0)                  <- positivity guard (bad)
58:    if(data > 0)                  <- positivity guard (goodG2B)
70:    if(data > 0)                  <- positivity guard (goodB2G)
72:        if (data < (CHAR_MAX/2))  <- range check
79:        printLine("data value is too large ...")   <- rejection message
91:    if (useGood)                  <- mode dispatch
```

Notes on what is **absent** from the C source (so no rows exist for them):
`assert` — none. `return <errcode>` / `return NULL` — none: every function
returns `void`. Error enums / `RETURN_ERROR` macros — none. Allocation —
none. The *entire* error surface is "guard fails ⇒ produce different (or no)
output". Consequently, for every row below the observable "C result" is the
exact bytes written to `stdout` (and the fact that the call returns normally
without crashing), which is what the differential tests compare.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `printLine` | `line == NULL` (`driver.c:32` guard fails) | guard `line != NULL` is false ⇒ **no `printf` at all**, no crash, returns normally. stdout gets **0 bytes**. | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` = valid pointer to the empty string `""` (zero-length, degenerate but *accepted*) | guard passes ⇒ `printf("%s\n","")` ⇒ stdout gets exactly `"\n"` (1 byte) | `err_e2_print_line_empty` | [x] |
| E3 | `printLine` | `line` points at bytes that are **not valid UTF-8** (e.g. `0x80 0xFF 0xFE`) — invalid for a Rust `str`, perfectly legal for C | guard passes ⇒ raw bytes copied verbatim + `'\n'`; **no** replacement chars, **no** panic | `err_e3_print_line_invalid_utf8` | [x] |
| E4 | `printLine` | `line` points at a buffer whose NUL terminator is far away (32 KiB) and/or contains `%` / `%n` conversion characters (`printf("%s\n", line)` — the data is an *argument*, never a format) | guard passes ⇒ the `%` bytes are printed literally, not interpreted; full 32 KiB emitted | `err_e4_print_line_percent_and_long` | [x] |
| E5 | `printHexCharLine` | **no guard exists** (`driver.c:38-41`); `charHex` is negative, e.g. `-1`, `-2`, `CHAR_MIN` = `-128`. Default argument promotion makes it `int`, `%02x` reinterprets as `unsigned int` ⇒ 8 hex digits, and the `02` width is *not* honoured. | `-1` ⇒ `"ffffffff\n"`, `-2` ⇒ `"fffffffe\n"`, `-128` ⇒ `"ffffff80\n"` | `err_e5_print_hex_negative` | [x] |
| E6 | `printHexCharLine` | `charHex == 0` (boundary / falsy) | `"00\n"` (the `%02x` zero-pad path) | `err_e6_print_hex_zero` | [x] |
| E7 | `printHexCharLine` | `charHex` one step past the signed range as seen by the caller: caller passes the *int* `128` / `255` / `256` / `-129` / `INT_MAX` in the register. C truncates to `char` at the callee's ABI boundary. | value is truncated mod 256 then sign-extended: `128`⇒`"ffffff80\n"`, `255`⇒`"ffffffff\n"`, `256`⇒`"00\n"`, `-129`⇒`"7f\n"`, `INT_MAX`⇒`"ffffffff\n"` | `err_e7_print_hex_out_of_char_range` | [x] |

> **BUG FOUND AND FIXED BY ROW E7.** GCC's callee for `void printHexCharLine(char)`
> emits `mov %edi,%eax; mov %al,-0x4(%rbp); movsbl -0x4(%rbp),%eax` — it
> *truncates the incoming argument register to 8 bits itself*, so the C
> library's output depends only on the low byte. Rust's original
> `extern "C" fn(charHex: c_char)` tags the parameter `signext` and therefore
> assumes the caller already extended it; at `-O` LLVM elided the truncation
> (`mov %edi,%esi`) and the upper 24 bits leaked into `%02x`. Result:
> `printHexCharLine(128)` printed `"80\n"` in Rust `--release` vs `"ffffff80\n"`
> in C. Fixed by declaring the exported wrapper as taking `c_int` and
> truncating explicitly (`src/lib.rs`), which reproduces GCC's codegen for all
> 2^32 register values and is indistinguishable from `fn(c_char)` for any
> correctly-extending caller. `scripts/mutation_check.sh` carries a
> `printhex-no-abi-truncation` mutant to keep this from regressing.
| E8 | `bad` | the `if(data > 0)` guard at `driver.c:47` — `data` is hard-coded `CHAR_MAX`, so the guard **always passes**; the *unguarded* signed overflow `CHAR_MAX * 2` truncated to `char` is the CWE under test. Rust must NOT panic on this overflow (debug-mode `i8` arithmetic would). | one line `"fffffffe\n"` (127*2 = 254, truncated to `char` = -2, promoted to int = -2, `%02x` ⇒ `fffffffe`) | `err_e8_bad_overflow_no_panic` | [x] |
| E9 | `goodB2G` (via `good` / `driver(1)`) | the range check `if (data < (CHAR_MAX/2))` at `driver.c:72` **fails** (`data` = `CHAR_MAX` = 127, `CHAR_MAX/2` = 63, `127 < 63` is false) ⇒ the *rejection* branch | `printLine("data value is too large to perform arithmetic safely.")` ⇒ that string + `'\n'` (54 bytes). The multiplication is **never** performed. | `err_e9_good_b2g_rejects` | [x] |
| E10 | `goodB2G` | the dead store `data = ' '` at `driver.c:68` immediately overwritten by `data = CHAR_MAX` at `:69`. A translator that "fixes" the dead store would take the *accept* branch (32 < 63) and print `"40\n"`. | the dead store must have **no** observable effect: output is E9's rejection message, never `"40\n"` | `err_e10_good_b2g_dead_store_ignored` | [x] |
| E11 | `driver` | `useGood == 0` (the falsy value ⇒ `bad()`) | `bad()`'s output only: `"fffffffe\n"` | `err_e11_driver_zero` | [x] |
| E12 | `driver` | `useGood` is a **negative** int (`-1`, `INT_MIN`) — C `if(useGood)` tests the *whole* `int` for non-zero, not its sign and not its low byte | truthy ⇒ `good()` output | `err_e12_driver_negative` | [x] |
| E13 | `driver` | `useGood` is a non-zero int whose **low byte is zero** (`256`, `0x10000`, `INT_MIN` = `0x80000000`) — the classic "truncated to `char`/`bool`" mistranslation would take the `bad()` branch | truthy ⇒ `good()` output | `err_e13_driver_low_byte_zero` | [x] |
| E14 | `driver` | out-of-range "enum-like" values crossing FFI: `useGood` = `INT_MAX`, `INT_MIN`, `1<<31`-patterns, and every bit-pattern from a random sweep. `useGood` is a plain `int` so *any* of the 2^32 bit patterns is a legal input with no invalid variant to reject. | exactly two possible behaviours, partitioned by `== 0`; never UB, never a trap | `err_e14_driver_full_int_sweep` | [x] |
| E15 | all exported fns | called repeatedly / in interleaved order across the C and Rust `.so` in one process (shared `stdout` FILE): no hidden per-library state may make the Nth call differ from the 1st | every function is stateless ⇒ output is a pure function of the argument | `err_e15_statelessness` | [x] |

All 15 rows have a passing differential test (see
`tests/differential.rs`, `cargo test --release`).
