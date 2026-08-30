# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

## Mechanical derivation

Every rejection construct was grepped out of the *entire* C tree
(`c_src/include/driver.h`, `c_src/src/driver.c`), comments excluded:

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if|else|switch|while|for|\?|enum|#error' \
      c_src/src/driver.c c_src/include/driver.h        # (comments filtered)
c_src/include/driver.h:24:%:ifndef DRIVER_H_     <- include guard, not a runtime check
c_src/include/driver.h:29:%:endif  //DRIVER_H_   <- include guard, not a runtime check
c_src/src/driver.c:26:%:include <stdio.h>
c_src/src/driver.c:27:%:include <iso646.h>
c_src/src/driver.c:29:void driver(int x, int y) <%
c_src/src/driver.c:33:%>
```

Complete body of the only function (after digraph / `<iso646.h>` expansion):

```c
void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}
```

Therefore, mechanically:

* **0** `return` statements (function is `void` — there is no return value, no
  sentinel, no error code).
* **0** error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, …).
* **0** `assert`s.
* **0** explicit range / bounds checks.
* **0** null-pointer checks — there are **no pointer parameters** at all.
* **0** error enums / status enums; **no enum parameters** anywhere in the
  public header.
* **0** `MIN`/`MAX` constants, **0** `#error`, **0** `exit`/`abort`.
* **0** conditional branches of any kind (`if`/`else`/`switch`/`?:`/loops).

`driver` is **total**: it accepts the entire `int × int` domain and rejects
nothing. Every table row below is therefore a *"must NOT reject"* row: the
required behaviour is that C and Rust both accept the input and emit the
identical byte stream. A divergence here would show up as Rust panicking,
aborting, wrapping/overflow-trapping, or printing different bytes where C
prints a value.

The rows still cover the generic boundary classes the task mandates (extreme
and one-past-range values, out-of-range "enum"-like integers crossed over FFI,
ABI garbage in unused argument bits, and a failing output sink), because those
are the real inputs an external caller can produce even though the C source
contains no check for them.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `driver` | `x = INT_MIN` (extreme low, `0x80000000`); `y` swept | no rejection; prints `x \| ~y` then `\n`; no trap on the sign bit | `err_e1_x_int_min` | [x] |
| E2 | `driver` | `y = INT_MIN` — `~y` computes `~INT_MIN = INT_MAX`, the classic "negate INT_MIN" trap shape | no rejection; `~` is a bitwise op, cannot overflow; prints result | `err_e2_y_int_min` | [x] |
| E3 | `driver` | `x = INT_MAX`, `y = INT_MAX` (extreme high both) | no rejection; prints `-1`? no: `INT_MAX \| ~INT_MAX = INT_MAX \| INT_MIN = -1`; prints `-1\n` | `err_e3_both_int_max` | [x] |
| E4 | `driver` | `x = INT_MIN`, `y = INT_MIN` | no rejection; `INT_MIN \| INT_MAX = -1`; prints `-1\n` | `err_e4_both_int_min` | [x] |
| E5 | `driver` | `x = INT_MIN`, `y = INT_MAX` → result is exactly `INT_MIN`, the value whose `%d` text (`-2147483648`) has no positive counterpart | no rejection; prints `-2147483648\n` | `err_e5_result_int_min` | [x] |
| E6 | `driver` | one step past the range of a *narrower* int: `x = 32768`, `x = -32769`, `x = 65536`, `y` likewise (values a 16-bit `int` port would reject) | no rejection; full 32-bit `int` semantics | `err_e6_one_past_narrow_ranges` | [x] |
| E7 | `driver` | "out-of-range enum value" analogue: `driver` takes no enum, so the equivalent hostile FFI input is an `int` bit pattern with no meaningful interpretation — `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, `0xDEADBEEF`, `0xCAFEBABE`, `0xFFFFFFFE` — passed as `c_int` for both args | no rejection; every 32-bit pattern is a valid `int`; prints `x \| ~y` | `err_e7_out_of_range_enum_like_ints` | [x] |
| E8 | `driver` | ABI hostile: symbol called through a `extern "C" fn(u64, u64)` signature so the upper 32 bits of each 64-bit argument register are garbage (`0xDEADBEEF_00000005`) | both must ignore the high halves identically (System V AMD64: `int` occupies only the low 32 bits of `rdi`/`rsi`) | `err_e8_high_garbage_bits_in_arg_registers` | [x] |
| E9 | `driver` | zero-ish / degenerate arguments: `(0,0)`, `(0,-1)` (result `0`), `(-1,0)`, `(0,0)` repeated | no rejection; `(0,-1)` prints `0\n` | `err_e9_degenerate_zero_args` | [x] |
| E10 | `driver` | output sink fails: `stdout` redirected to `/dev/full`, so the `printf`/`puts` flush fails with `ENOSPC`; both ignore the `printf`/`puts` return values | no rejection, no crash, no abort; call returns normally; zero bytes land in the sink; `ferror(stdout)` set identically for C and Rust | `err_e10_failing_stdout_dev_full` (separate test binary) | [x] |
| E11 | `driver` | `stdout` fd closed entirely (`close(1)` → `write` fails `EBADF`) | no rejection, no crash; call returns normally | `err_e11_closed_stdout_fd` (separate test binary) | [x] |
| E12 | `driver` | "oversized length" analogue: the API has no length/size/count parameter and no buffer, so no length can be oversized. Documented as **N/A — vacuously satisfied**; the closest real analogue is the maximum-width output (10 digits + sign = 11 bytes) which is covered by E5 and `CONFIGS.md` rows C15/C16. | — | — | [x] N/A |
| E13 | `driver` | null-pointer analogue: the API has **no pointer parameters** and returns `void`, so no null can be passed and no null can be returned. Documented as **N/A — vacuously satisfied**. | — | — | [x] N/A |

## Notes on rows marked N/A

Rows E12 and E13 are recorded rather than silently dropped so the table is a
complete, auditable mapping of the mandated generic boundary classes onto this
API. They are N/A because the C signature (`void driver(int, int)`) exposes
neither a pointer, a length, a buffer, nor a return channel — this is verified
against `c_src/include/driver.h`, which contains exactly one declaration.

## Result

All rows pass (E12/E13 are the documented N/A rows, asserted structurally
against `driver.h` by `err_e12_e13_no_pointer_length_or_return_channel` so the
"nothing to check here" claim cannot silently rot if the header changes).

Rows E10 and E11 live in their own test binaries (`phase_c_dev_full.rs`,
`phase_c_closed_stdout.rs`) because they deliberately set the process-wide
`stdout` error indicator, which would otherwise leak into the other tests.

### Sensitivity evidence

Injecting a rejection that the C does not have —
`if x == INT_MIN || y == INT_MIN { return; }` — was caught by rows
E1, E2, E4, E5, E7, E8, E10 and E11. So these rows do detect a Rust-side
rejection that the C would not perform, which is the exact failure mode the
table exists to rule out.
