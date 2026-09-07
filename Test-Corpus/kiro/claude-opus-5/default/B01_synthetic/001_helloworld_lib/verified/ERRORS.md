# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source. The greps below were run over
`c_src/src/` and `c_src/include/` (the complete C tree — 59 lines total):

```sh
grep -rn  "return" src include
grep -rEn "RETURN_ERROR|return -[0-9]|return NULL|errno|_ERROR|_ERR\b|enum " src include
grep -rn  "assert" src include
grep -rEn "\bif\b|\bswitch\b|\bwhile\b|\bfor\b|\?" src include
grep -rEn "^\s*#\s*(if|ifdef|ifndef|elif|else)" src include
grep -rEn "#define|MAX|MIN" src include
```

Results:

| grep | hits |
|------|------|
| `return` statements | **1** — `src/hello.c:30: return 0;` |
| error macros / `return -N` / `return NULL` / `errno` / error enums | **0** |
| `assert` | **0** |
| `if` / `switch` / `while` / `for` / ternary — i.e. any branch at all | **0** |
| `#if` / `#ifdef` / `#elif` / `#else` | 1, and it is the `#ifndef HELLO_H_` include guard only |
| `#define` / `MAX` / `MIN` constants | 1, and it is `#define HELLO_H_` (include guard) only |

The whole library, comments stripped, is:

```c
#include <stdio.h>
#include "hello.h"
int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

## The table

`helloworld` takes no parameters, performs no validation, has no branches, no
asserts, no range checks, no null checks, no error enum and no error return: it
has exactly one exit path, the unconditional `return 0`. So the set of "distinct
ways the C rejects or errors on input" derived from the source is **empty** —
there are zero `RETURN_ERROR`-style rows, and inventing any would violate the
"derive from what the C actually checks" rule.

What remains, and is genuinely testable, is the **generic FFI-boundary surface**
that the task requires be covered even when it is not in the table. For a
zero-argument, zero-branch function the applicable generic boundaries are the
ones below. Each has a differential test asserting C and Rust return the *same*
value / behave the same way — not merely "both didn't crash".

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `helloworld` | Nominal exit path, asserted exactly: the sole `return 0`. Baseline the other error rows are compared against. | returns `int` `0` | `e1_return_code_is_zero` | [x] |
| E2 | `helloworld` | Called through a **wrong-arity** function pointer with extra integer args. `hello.h:27` declares `int helloworld();` — an *empty* (non-prototype, K&R) parameter list, so C imposes no arity and any caller may pass extra arguments. Args land in unused SysV registers. This is a real input the C accepts. | ignores the extra args; returns `0`; prints the one line | `e2_extra_args_non_prototype` | [x] |
| E3 | `helloworld` | Called through a wrong-arity pointer with **extra pointer args, including `NULL`**. This is the "null pointer" generic boundary for a function that has no pointer parameter to nullify — passing `NULL` in the arg registers must be ignored, not dereferenced. | ignores them; returns `0`; no segfault | `e3_extra_null_pointer_args` | [x] |
| E4 | `helloworld` | Called through a wrong-arity pointer with **out-of-range enum values** (`-1`, `0`, `INT_MIN`, `INT_MAX`, `0x7FFFFFFF`, and `4242` — ints with no valid variant in any enum). C enums accept any `int`, so these are real inputs; there is no enum in this API, so the correct identical behaviour is to ignore them. | ignores them; returns `0` | `e4_out_of_range_enum_values` | [x] |
| E5 | `helloworld` | Called through a pointer whose **return type is mis-declared wider** (`i64`) — probes whether the Rust wrapper, like C, leaves only `eax` defined and does not write a full 64-bit `0`. Both must agree on the low 32 bits. | low 32 bits of return = `0` | `e5_return_width_low32` | [x] |
| E6 | `helloworld` | **Oversized/repeated invocation:** 100 000 successive calls, the "oversized length" analogue for a function with no length parameter. Probes for hidden state, counter overflow, or a one-shot init that fails on reuse. | returns `0` every time; prints the line every time | `e6_oversized_repeated_invocation` | [x] |
| E7 | `helloworld` | Called with **stdout closed** (fd 1 closed before the call). `puts` fails and sets the FILE error flag; the C still returns `0` because it discards `printf`'s return value. Rust must also return `0` and must not panic or abort. | returns `0`; no crash; no output | `e7_stdout_closed` | [x] |
| E8 | `helloworld` | Called with **stdout redirected to a full/unwritable sink** (fd 1 → `O_RDONLY` `/dev/null`, so writes get `EBADF`). Same as E7: the write error is discarded. `panic = "abort"` in `[profile.release]` makes any Rust panic here a process abort, so this specifically checks the Rust does not surface the I/O error. | returns `0`; no crash | `e8_stdout_unwritable` | [x] |
| E9 | `helloworld` | **Symbol resolved but library unloaded/reloaded** — `dlclose` then `dlopen` again and call. Probes destructor/atexit or TLS state that would break a second load. | returns `0`; prints the line | `e9_reload_after_dlclose` | [x] |

All 9 rows have a passing differential test (see `tests/differential.rs`).
