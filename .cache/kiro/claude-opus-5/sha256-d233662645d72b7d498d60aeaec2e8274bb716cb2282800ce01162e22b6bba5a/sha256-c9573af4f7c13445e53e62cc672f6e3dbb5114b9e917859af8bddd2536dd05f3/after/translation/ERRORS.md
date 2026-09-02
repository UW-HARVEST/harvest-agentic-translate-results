# ERRORS.md — Phase A error-surface table

## How this table was derived (mechanical grep of `c_src/`)

```sh
grep -rn "return"                     c_src/src c_src/include   # -> no matches
grep -rn "assert"                     c_src/src c_src/include   # -> no matches
grep -rn "NULL"                       c_src/src c_src/include   # -> no matches
grep -rniE "error|errno|fail|invalid" c_src/src c_src/include   # -> no matches (outside licence text)
grep -rnE "\b(if|else|switch|case|while|for)\b|#if|#ifdef|#ifndef" c_src/src c_src/include
                                                # -> only include/driver.h:24 `#ifndef DRIVER_H_`
grep -rnE "\b(enum|struct|typedef|union)\b"     c_src/src c_src/include   # -> no matches
grep -rnoE "[0-9]+" c_src/src/driver.c          # -> 2025 (licence), 2 (line 29), 300 (line 30)
```

The complete body of the library is:

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

**Result of the mechanical scan: the C library has an EMPTY explicit error
surface.** There are no `RETURN_ERROR`-style macros, no `return -1`, no
`return NULL`, no error enums, no `assert`, no range checks, no null checks and
no min/max constants — `driver` returns `void`, takes a single `int` by value,
dereferences nothing, allocates nothing, and rejects no input. Every `int` in
`[INT_MIN, INT_MAX]` is an accepted input.

Writing rows for checks the C does not perform would be inventing an error
surface, which the task forbids. The rows below are therefore the *actual*
failure-adjacent conditions reachable in this C code, plus the generic C-API /
FFI boundaries the task requires be covered even when absent from the table.
"Expected C result" is stated as the observable behaviour of the compiled C
`.so` (`printf` to `stdout`; no return value), because that is the only
observable the API has.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `driver` | Signed overflow of the *multiply* `2*x` on the positive side: `x > INT_MAX/2`, i.e. `x >= 1073741824`. Formally C UB; the compiled `.so` computes it with `lea (%rax,%rax,1),%ebx` on a 32-bit register. | No rejection, no diagnostic, no abort. Wraps mod 2^32; prints the wrapped `2*x + 300` followed by `\n`. E.g. `x = 1073741824` prints `-2147483348`. |
| E2 | `driver` | Signed overflow of the *multiply* `2*x` on the negative side: `x < INT_MIN/2`, i.e. `x <= -1073741825`. Formally C UB. | No rejection. Wraps mod 2^32 (the wrapped multiply then wraps again on the `+300`). E.g. `x = -1073741825` prints `-2147483350`. |
| E3 | `driver` | Signed overflow of the *addition* `y += 300` only (multiply in range, sum out of range): `1073741674 <= x <= 1073741823` (`add $0x12c,%ebx`). Formally C UB. | No rejection. Wraps mod 2^32. E.g. `x = 1073741674` prints `-2147483648`. |
| E4 | `driver` | Exact boundary values of the parameter type: `x = INT_MAX` (`2147483647`) and `x = INT_MIN` (`-2147483648`) — one step past these is not representable in the `int` parameter. | No rejection. `INT_MAX` prints `298`; `INT_MIN` prints `300`. |
| E5 | `driver` | One step past each internal overflow threshold — the "one past a documented valid range" probe for the only ranges the arithmetic distinguishes: add-only overflow at `x = 1073741673 -> 1073741674`, multiply overflow (positive) at `x = 1073741823 -> 1073741824`, multiply overflow (negative) at `x = -1073741824 -> -1073741825`. | No rejection at any threshold; output is continuous mod 2^32 across each threshold pair. |
| E6 | `driver` | Out-of-range "enum-like" / garbage-bit argument: the C prototype takes `int`, so a caller may pass a 64-bit value whose upper 32 bits are non-zero (e.g. calling through a `void(*)(long)` pointer, or an enum-typed argument with no valid variant — C enums accept any `int`). The callee reads only `%edi` (`mov %edi,-0x14(%rbp)`). | Upper 32 bits are ignored; behaves exactly as if called with the low 32 bits as an `int`. No rejection. |
| E7 | `driver` | `printf` itself fails (e.g. `stdout` redirected to a closed/full descriptor): its return value is discarded on line 30 of `driver.c`. | Failure is silently ignored; `driver` still returns normally (`void`) and does not set/report any status. No crash. |

## Sign-off

| # | differential test | status |
|---|-------------------|--------|
| E1 | `error_e1_multiply_overflow_positive` | [x] passes (C == Rust, debug + release, both feature configs) |
| E2 | `error_e2_multiply_overflow_negative` | [x] passes (C == Rust, debug + release, both feature configs) |
| E3 | `error_e3_add_only_overflow` | [x] passes (C == Rust, debug + release, both feature configs) |
| E4 | `error_e4_int_type_boundaries` | [x] passes (C == Rust, debug + release, both feature configs) |
| E5 | `error_e5_one_step_past_thresholds` | [x] passes (C == Rust, debug + release, both feature configs) |
| E6 | `error_e6_garbage_high_bits_and_out_of_range_enum` | [x] passes (C == Rust, debug + release, both feature configs) |
| E7 | `error_e7_printf_failure_ignored` | [x] passes (C == Rust, debug + release, both feature configs) |

Null-pointer probes are **not applicable**: the public API has no pointer
parameter and no out-parameter (`void driver(int)`), so there is no pointer to
pass `NULL` for. Likewise "zero and oversized lengths" are not applicable —
there is no length/size/count parameter and no buffer. The nearest meaningful
analogues (zero input, and the extreme representable magnitudes of the only
parameter) are covered by E4/E5 and by row C3/C9 of `CONFIGS.md`.
