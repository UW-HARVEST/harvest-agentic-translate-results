# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection path in a C library shows up as one of: an error-return macro, a
`return` of a sentinel, an error enum, an `assert`, an explicit range/null check,
or a min/max constant. Grepping the entire compiled C surface for all of them:

```sh
cd c_src
grep -nE 'return|assert|NULL|errno|exit\(|abort|if|switch|while|for|else|goto|#if|%:if|ERROR|error|<|>|MIN|MAX|limits' -r src include
```

The only hits are the include guard (`%:ifndef DRIVER_H_` / `%:endif`), the two
`%:include` lines, and the two brace digraphs of `driver`'s own body. Full list
of hits:

```
src/driver.c:26:%:include <stdio.h>
src/driver.c:27:%:include <iso646.h>
src/driver.c:29:void driver(int x, int y) <%
src/driver.c:33:%>
include/driver.h:24:%:ifndef DRIVER_H_
include/driver.h:29:%:endif //DRIVER_H_
```

Consequently, in the compiled library:

* error-return macros (`RETURN_ERROR`, …): **0**
* `return` statements of any kind: **0** (`driver` returns `void`)
* error enums / status codes: **0**
* `assert` / `abort` / `exit`: **0**
* explicit range checks, null checks, `if`/`switch`/loops: **0**
* min/max or `<limits.h>` constants: **0**
* pointer parameters (hence null-pointer rejections): **0**
* length/size/count parameters (hence zero/oversized-length rejections): **0**
* enum parameters (hence out-of-range-enum rejections): **0**

`driver` accepts two `int` parameters. Every one of the 2^32 bit patterns of a
32-bit `int` is a valid argument, `x | ~y` is total over them (bitwise ops on
`int` cannot overflow or trap), and `printf`/`puts` are total over the resulting
`int`. **The C library has an empty rejection surface: there is no input it
errors on.**

## Table

Rows 1–6 are the *derived* rows: because the C rejects nothing, the "expected C
result" for every row is "no rejection — prints `x | ~y` then `\n`". These rows
therefore assert the *absence* of a rejection, differentially: for each row the
Rust must also not reject, and must emit the same bytes. Rows 7–10 are the
generic FFI boundary probes Phase C mandates even when absent from the source.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `driver` | `x = INT_MIN` (`-2147483648`), the smallest representable argument — one step below is not representable | no rejection; prints `-1` for `y=0`, i.e. `x \| ~y`, then `\n` |
| 2  | `driver` | `y = INT_MIN`, smallest representable second argument (`~y` = `INT_MAX`) | no rejection; prints `x \| INT_MAX`, then `\n` |
| 3  | `driver` | `x = INT_MAX` (`2147483647`), the largest representable argument — one step above is not representable | no rejection; prints `x \| ~y`, then `\n` |
| 4  | `driver` | `y = INT_MAX`, largest representable second argument (`~y` = `INT_MIN`, sign bit forced on) | no rejection; prints a negative `x \| INT_MIN`, then `\n` |
| 5  | `driver` | `x = y = 0` — the "empty"/zero argument case | no rejection; prints `-1` then `\n` (`0 \| ~0` = `-1`) |
| 6  | `driver` | `y = -1` so `~y = 0`, and `x = 0`: the only argument pair yielding `0` | no rejection; prints `0` then `\n` |
| 7  | `driver` | out-of-`int`-range value passed across the FFI boundary: caller declares the parameter `int64_t`/`i64` and passes `INT_MAX + 1`, `INT_MIN - 1`, `0x1_0000_0000`, `0xFFFF_FFFF_FFFF_FFFF` — the SysV AMD64 ABI leaves the upper 32 bits of the register unspecified for an `int` parameter | no rejection; callee reads only the low 32 bits, so the result equals `driver((int)x, (int)y)`; Rust must truncate identically |
| 8  | `driver` | wrong-arity / extra-argument call across the FFI boundary (caller passes 4 args to the 2-arg `extern "C"` symbol) | no rejection; extra register arguments are ignored, output identical to the 2-arg call |
| 9  | `driver` | out-of-range "enum-like" `int` value — an `int` argument with no meaningful interpretation, e.g. every bit set (`0xFFFFFFFF` = `-1`) or a lone sign bit; C enums/ints accept any `int`, so these must not be rejected | no rejection; prints `x \| ~y` for those bit patterns, then `\n` |
| 10 | `driver` | repeated / high-volume invocation with no re-initialisation (no init/teardown API exists to misuse; calling `driver` "before init" or "after teardown" is therefore always legal) | no rejection on any call; each call appends `<x\|~y>\n` to `stdout` in order |

Rows explicitly **not** applicable (documented so the absence is deliberate, not
an oversight): null-pointer arguments, zero-length buffers, oversized lengths,
unterminated strings, invalid handles/contexts, unaligned pointers, and named
enum constants — the API has no pointer, length, handle, or enum parameter.

## Status

| # | test | result |
|---|------|--------|
| 1 | `err_row01_x_int_min` | [x] pass |
| 2 | `err_row02_y_int_min` | [x] pass |
| 3 | `err_row03_x_int_max` | [x] pass |
| 4 | `err_row04_y_int_max` | [x] pass |
| 5 | `err_row05_both_zero` | [x] pass |
| 6 | `err_row06_result_zero` | [x] pass |
| 7 | `err_row07_out_of_int_range_ffi` | [x] pass |
| 8 | `err_row08_extra_ffi_arguments` | [x] pass |
| 9 | `err_row09_out_of_range_enum_like_ints` | [x] pass |
| 10 | `err_row10_repeated_invocation_no_init` | [x] pass |
