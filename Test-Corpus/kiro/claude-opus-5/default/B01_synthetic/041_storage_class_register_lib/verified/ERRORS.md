# Phase A.2 — Error-surface table

Derived mechanically from the C source, not from documentation.

## Mechanical grep for every rejection construct

Run over the whole C tree (`c_src/src/driver.c`, `c_src/include/driver.h`),
after discarding the 22-line MIT licence header comment that both files start
with:

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|if *\(|switch|#if|else|while|for *\(|goto|<|>|==|!=|RETURN_ERROR|-1|MAX|MIN|enum' \
     c_src/src/driver.c c_src/include/driver.h
```

Surviving hits, in full:

```
c_src/src/driver.c:26:#include <stdio.h>
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

Both hits are false positives of the pattern (`<stdio.h>` matched `<`/`>`;
`#ifndef DRIVER_H_` is the include guard).

## The complete C body, for reference

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

## Findings

The public API is a single `void`-returning function taking one `int` by value.
Consequently the C code contains, verifiably:

* **0** error-return macros (`RETURN_ERROR` or similar) — none defined anywhere.
* **0** `return` statements of any kind (the function falls off the end;
  its return type is `void`, so there is no error code or sentinel to return).
* **0** `assert` / `abort` / `exit` calls, and no `<assert.h>` include.
* **0** `if` / `switch` / `?:` / loop / `goto` statements — the function is
  straight-line code with no branches at all.
* **0** pointer parameters, therefore **0** null checks and no way to pass a
  null pointer across the FFI boundary.
* **0** explicit range checks and **0** min/max constants
  (no `INT_MAX`, no `<limits.h>`).
* **0** enum types in the API, therefore no enum-valued parameter that could
  receive an out-of-range integer discriminant.
* **0** `errno` reads/writes and no checking of `printf`'s return value, so a
  failed write is silently ignored — that non-check is itself behaviour the Rust
  must replicate (it does: it also discards `printf`'s result).

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(none — the C source contains no rejection path)* | — |

**The table is empty, and that emptiness is a derived result, not an
omission.** `driver` accepts the entire `int` domain: every one of the 2^32
representable `int` values is a valid input that is processed and printed. There
is no input the C rejects, so there is no error code, sentinel, or `errno`
setting for the Rust to match. Phase C therefore has zero table rows to check
off, and its gate is satisfied vacuously.

## Generic-boundary obligations carried into Phase C anyway

The task requires covering the boundaries every C API has even when absent from
the table. Mapped onto this API's actual shape:

| # | boundary class | how it applies here | test |
|---|----------------|---------------------|------|
| B1 | null pointers | **N/A** — no pointer parameter exists in the signature `void driver(int)`. Nothing can be nulled. | — |
| B2 | zero length | closest analogue is the zero value `x == 0` | `err_boundary_zero` |
| B3 | oversized length | closest analogue is the extremal magnitudes `INT_MAX` / `INT_MIN` | `err_boundary_extremes` |
| B4 | one step past a documented valid range | the header documents no range; the representable range's edges are `INT_MIN`/`INT_MAX`, and "one step past" them is the wrap point. `2*x` overflows signed `int` for every `x` with \|x\| > `INT_MAX/2`, and `y += 300` overflows for `x` in `[INT_MAX/2 - 149, INT_MAX/2]`. These are UB in ISO C but have a definite, observable result in the library **as compiled** (`-O0`, two's-complement `lea`/`add`), which the Rust must reproduce. | `err_boundary_overflow_2x`, `err_boundary_overflow_plus300` |
| B5 | out-of-range enum value across FFI | **N/A** — no enum in the API. The nearest equivalent is "an `int` bit pattern with no distinguished meaning", which B2–B4 already cover exhaustively by class. | — |
| B6 | ignored library error | `printf` failing (stdout closed / write error) must be silently ignored by both, with no crash and no differing side effect | `err_printf_failure_ignored` |

Every row above is exercised in `translation/tests/differential.rs` as a
differential test against both `.so`s, asserting byte-identical stdout and
identical (absent) failure behaviour.
