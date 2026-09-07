# ERRORS.md — Phase C: ERROR-SURFACE TABLE

Derived mechanically from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h` — 37 + 29 lines, all of it comments except 14 code
lines). The grep for every rejection idiom:

```
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|if *\(|else|switch|case|#if|MAX|MIN|<=|>=' \
     src/driver.c include/driver.h
```

matches **only** `include/driver.h:24:#ifndef DRIVER_H_` (the include guard).

Therefore, mechanically:

- error-return macros (`RETURN_ERROR`, …): **0**
- `return -1` / `return NULL` / any value-returning `return`: **0**
  (`driver` and `print_hex` are both `void`; neither contains a `return`)
- error enums / status codes: **0**
- `assert`: **0** (`<assert.h>` is never included)
- explicit range checks / null checks: **0**
- `MIN`/`MAX` constants: **0**
- `#ifdef` feature branches in code: **0**

The C library has **no error/rejection surface**: `driver(int x)` is total over
its entire input domain (every one of the 2^32 `int` values is valid) and can
never fail or reject. The rows below are therefore the *implicit* rejection /
boundary conditions plus the generic FFI boundaries that every C API has, each
with the exact result the C actually produces.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `print_hex` (internal) | `len <= 0` — the only conditional in the library is the loop guard `i < len`, so a non-positive `len` skips the body entirely | prints nothing but the trailing `"\n"`; no read of `p`; no error. **Unreachable via the public API**: `driver` always passes the compile-time constant `sizeof(int)` (= 4 > 0). Verified structurally (Rust `while i < len` has identical guard) — not reachable at the FFI boundary in either library, so the two libraries are trivially identical on it. | structural / N/A | [x] |
| 2 | `print_hex` (internal) | `p == NULL` — no null check exists; C would dereference it | undefined behaviour in C. **Unreachable via the public API**: `driver` always passes `&x`, the address of its own by-value stack parameter, which is never null. No public entry point accepts a pointer, so *there is no null-pointer input to this library*. | structural / N/A | [x] |
| 3 | `driver` | `x = INT_MIN` (`-2147483648`) — lowest value one step past the negative end of the range | no rejection; prints the 4 little-endian bytes `00000080` + `"\n"` | `err_row3_int_min` | [x] |
| 4 | `driver` | `x = INT_MAX` (`2147483647`) — highest value, one step *below* the wrap-around | no rejection; prints `ffffff7f` + `"\n"` | `err_row4_int_max` | [x] |
| 5 | `driver` | `x = -1` (all bits set) — the classic "error sentinel" value a caller might pass | no rejection; prints `ffffffff` + `"\n"` | `err_row5_minus_one` | [x] |
| 6 | `driver` | `x = 0` — the "zero length / empty" analogue for a scalar API | no rejection; prints `00000000` + `"\n"` | `err_row6_zero` | [x] |
| 7 | `driver` | **Out-of-range value passed across the FFI boundary as a wider integer.** A C `int` parameter, like a C enum, accepts whatever bit pattern the ABI register holds. Calling `driver` through the signature `extern "C" fn(i64)` with a value whose bits 32..63 are garbage (e.g. `0x7FFF_FFFF_DEAD_BEEF`, `i64::MIN`, `u32::MAX as i64 + 1`) is a real input both libraries must handle identically. | SysV AMD64: the callee reads only `%edi`, i.e. the low 32 bits; the upper bits are ignored. Result is the low 32 bits printed little-endian, e.g. `0x7FFF_FFFF_DEAD_BEEF` -> `efbeadde` + `"\n"` | `err_row7_oversized_arg_truncation` | [x] |
| 8 | `driver` | Value one step past the *unsigned* range boundary, `0x1_0000_0000` (= `u32::MAX as i64 + 1`), passed as above | low 32 bits are all zero -> prints `00000000` + `"\n"` | `err_row7_oversized_arg_truncation` | [x] |
| 9 | `driver` | Repeated invocation / state corruption: the library has no global or `static` mutable state, so no call can put it into a bad state | every call is independent; output is a pure function of `x` | `err_row9_no_hidden_state` | [x] |

## Verdict

- [x] Every row above has a passing differential test (or is structurally
      unreachable at the FFI boundary and documented as such).
- [x] Both libraries agree byte-for-byte on every row — including the same
      *absence* of any error code, since neither has an error channel.
