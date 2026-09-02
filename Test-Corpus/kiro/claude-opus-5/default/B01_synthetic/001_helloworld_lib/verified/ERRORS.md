# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Derived **mechanically** from the C source, not from docs or guesses. The
complete non-comment body of the library is:

```c
/* c_src/include/hello.h */
int helloworld();

/* c_src/src/hello.c */
#include <stdio.h>
#include "hello.h"

int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

## Mechanical grep for every rejection mechanism

Run over `c_src/**/*.{c,h}` (excluding `build/`):

| grep pattern | hits | note |
|---|---|---|
| `RETURN_ERROR` | 0 | no error macro |
| `return -` | 0 | no negative sentinel |
| `return NULL` | 0 | no null sentinel |
| `assert` | 0 | no assertions |
| `errno` | 0 | never sets/reads errno |
| `if (` | 0 | **no conditional branches at all** |
| `switch` | 0 | no dispatch |
| `enum` | 0 | no enum parameters to pass out-of-range values into |
| `ERROR` | 0 | no error identifiers |
| `MAX` / `MIN` | 0 | no range constants |
| `#ifdef` / `#if` | 0 | no compile-time variants |
| `#ifndef` | 1 | the `HELLO_H_` include guard only |
| `return` | 1 | `hello.c:30: return 0;` — the *only* return |

## Table

There is exactly **one** reachable exit from the library and it is
unconditional, so the classic error surface is empty. The rows below are
therefore the *generic* FFI-boundary rejection classes the instructions
require, instantiated for the actual signature. Each has a real test; each is
justified against the C rather than invented.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `helloworld` | *(no rejection path exists)* — the sole `return` is unconditional `return 0`, reached on every call with no guard | always returns `0`; never a negative / `NULL` / errno sentinel |
| E2 | `helloworld` | null pointer argument — **not applicable as a distinct path**, but reachable across FFI because `int helloworld();` is an *unprototyped* (K&R) declaration, so C callers may legally pass arguments. Call through a `extern "C" fn(*const c_void) -> c_int` pointer with `NULL`. | argument ignored (never dereferenced — no parameter is named or read); returns `0`, prints normally |
| E3 | `helloworld` | zero / oversized "length" argument — same unprototyped-declaration channel: pass `0` and `usize::MAX`/`-1` as a would-be size | argument ignored; returns `0`, prints normally |
| E4 | `helloworld` | out-of-range enum value across the FFI boundary — a C enum accepts any `int`, so pass `INT_MIN`, `-1`, `0`, `INT_MAX` through the unprototyped declaration as if they were enum selectors | no selector is read, no `switch` exists; returns `0`, prints normally |
| E5 | `helloworld` | one step past a "documented valid range" — the header documents no range; the extremes of every scalar register class (`i32::MIN`, `i32::MAX`, `u64::MAX`, `f64::NAN`) are pushed as extra arguments | all ignored; returns `0`, prints normally |
| E6 | `helloworld` | more arguments than the definition accepts (6 integer registers + stack spill), the ABI-level abuse the unprototyped declaration permits | extra args ignored by the SysV AMD64 callee; returns `0`, prints normally |
| E7 | `helloworld` | `stdout` is closed / not writable when `printf` runs (the only failure mode inside the body — `printf` can fail) | `printf`'s return value is **discarded** by the C, so the failure is swallowed: `helloworld` still returns `0` and does **not** set an error |
| E8 | `helloworld` | invoked concurrently from many threads (no lock, no guard in the C) | no rejection; every call returns `0`; libc `stdout` locking keeps each line intact |

Rows E2–E6 all probe the same C-language quirk (the empty parameter list in
`int helloworld();`) with different value classes; they are listed separately
because they are distinct inputs a real caller can present, and a Rust
translation that declared a *different* arity or a variadic signature would
diverge on them.

## Checklist (Phase C)

- [x] E1 — `errors_e1_return_is_unconditionally_zero`
- [x] E2 — `errors_e2_null_pointer_argument`
- [x] E3 — `errors_e3_zero_and_oversized_length_arguments`
- [x] E4 — `errors_e4_out_of_range_enum_values`
- [x] E5 — `errors_e5_scalar_extremes_one_past_range`
- [x] E6 — `errors_e6_excess_arguments_stack_spill`
- [x] E7 — `errors_e7_stdout_write_failure_is_swallowed`
- [x] E8 — `errors_e8_concurrent_calls`
