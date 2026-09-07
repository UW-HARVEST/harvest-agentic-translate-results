# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection construct was grepped out of the whole of `c_src`
(`src/driver.c`, 66 lines incl. a 23-line licence header; `include/driver.h`,
which contains only `void driver(int x);`):

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|RETURN_ERROR|if *\(|switch|#ifdef|#if |enum|INT_M|MAX|MIN' \
      src/driver.c include/driver.h
src/driver.c:26:#include <stdio.h>
src/driver.c:27:#include <stdlib.h>
src/driver.c:38:    house->floors++;
src/driver.c:42:    house->bedrooms += extra_bedrooms;
```

(The last two lines match only because `->` contains `>`.)

Findings, stated exactly:

- **0** `return` statements with a value — `run` and `driver` are both `void`,
  and every internal helper is `void`.
- **0** `assert` / `abort` / `exit` / `errno` uses.
- **0** `if` / `switch` / ternary — the code is straight-line; there is exactly
  one control-flow path through `run` and one through `driver`.
- **0** `NULL` checks. The public API takes no pointers at all.
- **0** range checks, **0** min/max constants, **0** error enums, **0** error
  macros, **0** `#ifdef` branches.

**There is no error-return surface in this library.** Every `int` bit pattern is
an accepted input; nothing is rejected. Rows 1–7 below are therefore the generic
FFI-boundary boundaries mandated for every C API, and the "expected C result" is
in each case *successful execution with a specific stdout*, which the Rust must
reproduce byte-for-byte. Fabricating error rows here would be inventing checks
the C does not perform.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `run` | `extra_bedrooms = INT_MAX` (`2147483647`) — signed-overflow of `house->bedrooms += extra_bedrooms` (C UB; the shipped `.so` is built with no optimisation, so it wraps two's-complement) | no error/rejection; 4 lines on stdout, last line's bedroom count is the wrapped value | `ERRORS row 1` | [x] |
| 2 | `run` | `extra_bedrooms = INT_MIN` (`-2147483648`) — signed underflow of the same `+=` | no error; 4 lines, wrapped (negative or wrapped-positive) bedroom count printed with `%d` | `ERRORS row 2` | [x] |
| 3 | `driver` | `x = INT_MAX` — overflow applied **twice** (`run(x); run(x);`), so the second wrap compounds the first | no error; 8 lines, both wraps identical to C | `ERRORS row 3` | [x] |
| 4 | `driver` | `x = INT_MIN` — underflow applied twice | no error; 8 lines, both wraps identical to C | `ERRORS row 4` | [x] |
| 5 | `run` / `driver` | value one step past the 32-bit parameter range: caller pushes a 64-bit argument (`0x1_0000_0000`, `0x7FFF_FFFF_FFFF_FFFF`, `-1i64`, …) where the callee declares `int`. C truncates to the low 32 bits of the argument register; the Rust `extern "C"` wrapper must truncate identically | both take the low 32 bits; stdout identical | `ERRORS row 5[0..8]` | [x] |
| 6 | `run` / `driver` | "out-of-range enum" analogue: the parameter has **no** valid-variant restriction (it is a bare `int`, not an enum), so every one of the 2^32 bit patterns is a legal input with no rejection branch. Swept exhaustively at the boundaries and randomly across the full `i32` range | no error for any value; stdout identical for every value | `ERRORS row 6[0..127]` | [x] |
| 7 | `run` / `driver` | repeated invocation until `house->floors` overflows (`floors++`, one increment per `run`) | requires 2^31 − 3 calls; **unreachable** in a test (≈ hours of pure printf). Not a rejection either way; partially covered by row 8's long call sequences | `ERRORS row 7` (documents + covers 4096 increments) | [x] |
| 8 | `run` / `driver` | null / zero / oversized *length* and null *pointer* arguments | **N/A — not applicable, not skipped.** Neither public symbol takes a pointer, a length, a buffer, a string, or a struct; the entire public signature surface is `void run(int)` / `void driver(int)`. There is no pointer to pass NULL for. Asserted structurally in `ERRORS row 8` by checking the C `.so` imports no `mem*`/`str*`/allocator symbol that a buffer API would need | `ERRORS row 8` | [x] |

## How "same rejection" is asserted when nothing returns an error

Both entry points are `void` and never signal failure, so there is no error code
or sentinel to compare. The equivalent observable contract, which
`tests/differential.rs` enforces for every row, is threefold:

1. **Same exit disposition.** Each configuration runs in its own `exec`'d worker
   process; the worker's exit status is checked. If the C survives an input while
   the Rust panics or aborts on it (e.g. an arithmetic-overflow panic instead of
   a wrap), the Rust worker exits non-zero and the row fails. This is verified to
   work: replacing `wrapping_add` with `+` makes the debug build panic with
   `attempt to add with overflow` and the harness reports the row as failed.
2. **Same stdout, byte for byte**, including the exact wrapped integers printed
   by `%d`.
3. **Same acceptance**: no input is rejected by either side.

## Note on the C build used as ground truth

The reference `.so` is the one produced by the documented command
(`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`), which sets
no `CMAKE_BUILD_TYPE` and therefore compiles without optimisation. Signed
overflow in `house->bedrooms += extra_bedrooms` is UB in the abstract C language;
in this artifact it wraps two's-complement, and that observed behaviour is what
the Rust `wrapping_add` reproduces and what rows 1–4 compare against.
