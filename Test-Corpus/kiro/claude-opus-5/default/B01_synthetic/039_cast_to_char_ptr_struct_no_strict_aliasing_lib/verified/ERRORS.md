# ERRORS.md — error-surface table (Phase C gate)

## Mechanical derivation

Every rejection/error construct was grepped out of the whole C source
(`c_src/src/driver.c`, `c_src/include/driver.h`):

```
$ grep -cE 'return'      c_src/src/driver.c   -> 0
$ grep -cE 'assert'      c_src/src/driver.c   -> 0
$ grep -cE 'NULL'        c_src/src/driver.c   -> 0
$ grep -cE 'switch'      c_src/src/driver.c   -> 0
$ grep -cE 'enum'        c_src/src/driver.c   -> 0
$ grep -cE '#if'         c_src/src/driver.c   -> 0   (only the header guard in driver.h)
$ grep -nE 'if *\('      c_src/src/driver.c   -> (none)
$ grep -nE 'errno|exit|abort|RETURN_ERROR' c_src/src/driver.c -> (none)
```

The only comparison anywhere in the library is the `for` loop bound
`i < len` in `print_hex` (line 36), where `len` is always the compile-time
constant `sizeof(house_t)` == 16 — it is a loop condition, not an input check.

**Result: the C library has ZERO error returns, ZERO asserts, ZERO null
checks, ZERO range checks, ZERO error enums, and ZERO min/max constants.**
Both functions return `void`. There is no sentinel and no error code to
compare. The single public entry point `void driver(int x)` accepts every
one of the 2^32 `int` values without rejecting any of them.

## Table

Because no rejection branch exists, the table below records the *generic*
FFI boundaries every C API has (as required), with the C behaviour that was
actually observed rather than assumed. "Expected C result" for all rows is
"no error; prints the 16-byte struct image as 32 lowercase hex digits + `\n`".

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `driver` | `x == 0` (zero-value boundary) | no rejection; prints `00000000030000000000000000000040` |
| E2 | `driver` | `x == INT_MAX` (`2147483647`, one step below overflow) | no rejection; prints `ffffff7f0300...0040` |
| E3 | `driver` | `x == INT_MIN` (`-2147483648`, lowest valid `int`) | no rejection; prints `000000800300...0040` |
| E4 | `driver` | `x == -1` (all-bits-set / classic error sentinel value) | no rejection; prints `ffffffff0300...0040` |
| E5 | `driver` | `x == INT_MAX + 1` passed as the raw 32-bit pattern `0x80000000` (one step past the signed range; C reinterprets it, no check) | identical to E3 — `INT_MIN`; no rejection |
| E6 | `driver` | `x == 0xFFFFFFFF` passed as an unsigned/oversized value (one step past `UINT_MAX`'s signed reading) | identical to E4 — `-1`; no rejection |
| E7 | `driver` | out-of-range "enum-like" ints across the FFI boundary: `x` in {`INT_MIN`, `-2`, `-1`, `0`, `1`, `2`, `255`, `256`, `65535`, `65536`, `INT_MAX`} — the library declares no enum, so no value has "no valid variant"; every one must be accepted | no rejection for any value; each prints its own little-endian byte image |
| E8 | `driver` | oversized argument: a 64-bit value whose upper half is non-zero (`0x1_0000_0001`) passed in the argument register; the C ABI truncates to the low 32 bits | identical to `x == 1`; no rejection, no error |
| E9 | `driver` | null pointer arguments | NOT APPLICABLE — `driver` takes no pointer parameter, and the internal `print_hex` is `static` and only ever receives `&raw` (a live stack array), so no caller-reachable null-pointer path exists |
| E10 | `driver` | zero / oversized *length* arguments | NOT APPLICABLE — no length parameter is exposed; `print_hex`'s `len` is hard-wired to `sizeof(house_t)` |

Rows E1–E8 have differential tests in
`translation/tests/differential.rs` (`error_surface_*`). E9 and E10 are
unreachable by construction and are recorded as such rather than faked.

## Status

- [x] E1
- [x] E2
- [x] E3
- [x] E4
- [x] E5
- [x] E6
- [x] E7
- [x] E8
- [x] E9 (not applicable — no pointer parameter; documented)
- [x] E10 (not applicable — no length parameter; documented)
