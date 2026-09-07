# ERRORS.md — Phase C error-surface table

Derived **mechanically** from the C source, not from docs or assumptions.

## Mechanical derivation

The complete library is `c_src/src/driver.c` (32 lines, 26 of which are the
licence header) plus `c_src/include/driver.h`. Exhaustive grep for every
rejection construct across **all** C files:

```sh
grep -nE 'return|assert|NULL|errno|ERROR|error|exit|abort|if *\(|switch|#ifdef|enum|INT_MAX|INT_MIN|malloc|free' \
     c_src/src/driver.c c_src/include/driver.h
```

Every hit is either inside the licence comment block, the `#include <stdio.h>`
line, the `#ifndef DRIVER_H_` include guard, or the loop condition `i < x`.

Consequently the library contains:

* **0** `return` statements (the function is `void` and falls off the end)
* **0** error-return macros / sentinels / error enums
* **0** `assert`s
* **0** range checks, null checks, min/max constants
* **0** pointer parameters (the sole parameter is `int x`)
* **0** enum parameters
* **0** allocations or resources that can fail
* **1** branch in total: the loop condition `i < x`

So the *explicit* rejection surface is empty. The table below therefore records
(a) the one implicit error condition the C code does react to — a failing
`printf` — and (b) the generic FFI boundary cases the task mandates covering
even when absent from the source.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|----------------------------------------------|-------------------|-----|
| E1 | `driver` | `x == 0` (zero "length") | returns normally, writes 0 bytes | [x] |
| E2 | `driver` | `x == -1` (one step past the low end of the producing range) | returns normally, writes 0 bytes; loop body never entered | [x] |
| E3 | `driver` | `x` negative, arbitrary magnitude (randomized) | returns normally, writes 0 bytes | [x] |
| E4 | `driver` | `x == INT_MIN` (`-2147483648`, extreme out-of-range low) | returns normally, writes 0 bytes; no overflow in `i < x` | [x] |
| E5 | `driver` | `x == INT_MIN + 1` | returns normally, writes 0 bytes | [x] |
| E6 | `driver` | `printf` fails every call: `stdout`'s fd is redirected to a **read-only** fd (`/dev/null` opened `O_RDONLY`) so `write(2)` returns `EBADF` | C ignores `printf`'s return value: the loop still runs to completion, the call returns normally, no crash, 0 bytes observable | [x] |
| E7 | `driver` | `printf` fails every call **and** the stream is left in an error state, then a second, normal call is made on a fresh `stdout` | identical observable byte stream from both libraries (the `FILE*` error flag is a property of shared libc `stdout`, not of the library) | [x] |
| E8 | `driver` | out-of-range "enum" value: `x` is a bit pattern with no meaningful interpretation, passed as raw `i32` across FFI (`0x80000000`, `0xFFFFFFFF`, `0x7FFFFFFF` reinterpreted) | interpreted as a plain two's-complement `int`; negatives produce no output; identical in Rust | [x] |
| E9 | `driver` | "oversized length": `x == INT_MAX` — `j += 2` signed-overflows at `i == 2^30` (UB in C) | both wrap identically; verified empirically by executing the full 2,147,483,647-iteration run for both libraries (46,096,159,855 bytes each, identical FNV-1a digest `0x285c20ffc2afca05`) — see CONFIGS.md C9c/C13 | [x] |

Notes:

* There is no null-pointer case to test: `driver` takes no pointer. Passing a
  pointer-shaped value is covered by E8 (it is just an `int` bit pattern).
* There is no error code or sentinel to compare, because the function returns
  `void`. "Same rejection" is therefore asserted as: same observable byte
  stream **and** both calls return control normally (no abort/panic/signal),
  which the test harness checks by running the call in-process and continuing.

## Gate

- [x] Every row above has a passing differential test in
      `translation/tests/error_paths.rs` (E9's empirical half lives in
      `translation/tests/overflow.rs`).

Result: `cargo test --release --test error_paths` → 10 passed, 0 failed.

Note on E6: with a failing `stdout` no bytes are observable from either side, so
that row is by construction insensitive to the loop arithmetic — it is the
"does the library survive and keep going" check, and `ferror(stdout)` is
asserted non-zero to prove the failing writes really were attempted.
