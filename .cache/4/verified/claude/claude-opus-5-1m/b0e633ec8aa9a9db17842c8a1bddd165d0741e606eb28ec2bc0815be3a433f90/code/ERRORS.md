# ERRORS.md — error-surface table (Phase C)

## Mechanical derivation

Every rejection/error construct was grepped out of the complete C source
(`c_src/src/driver.c`, `c_src/include/driver.h` — 31 + 28 lines, all of it):

```
$ grep -nE 'return|assert|NULL|errno|if *\(|switch|#ifdef|#if |ERROR|exit|abort|[<>]|==|!=' \
      c_src/src/driver.c c_src/include/driver.h
c_src/src/driver.c:26:#include <stdio.h>      <- the only hit; not an error path
```

Result: the C library contains

* **0** error-return macros (`RETURN_ERROR`, …) — none exist,
* **0** `return -1` / `return NULL` (the only function returns `void`),
* **0** error enums / status codes,
* **0** `assert` / `abort` / `exit`,
* **0** explicit range, null or size checks,
* **0** min/max constants,
* **0** branches of any kind (confirmed by disassembly: `driver` is a straight
  line `lea (%rax,%rax,1),%ebx; add $0x12c,%ebx; call printf@plt`),
* **0** pointer parameters (the only parameter is `int x`), so there is no
  null-pointer, length or buffer surface at all,
* **0** enum parameters, so there is no invalid-enum-value surface.

The whole public API is `void driver(int x)`, and **every** `int` bit pattern is
an accepted input: the function never rejects anything. Consequently the table
below has no "invalid input rejected" rows; instead it enumerates the generic
boundary conditions the task requires for every C API, i.e. the inputs that are
*extreme or that overflow*, together with the exact observable result the C
produces (its de-facto behaviour, which the Rust must reproduce bit-for-bit).

## Error / boundary surface

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `driver` | no explicit error path exists anywhere in the library (grep above) — there is no error code or sentinel to compare | n/a: `void` return, sole observable effect is the `printf("%d\n", …)` byte stream; both libs must emit identical bytes for every input | `errors::e1_no_error_paths_exist` |
| E2 | `driver` | `x = INT_MAX` (2147483647) → `2*x` signed-overflows (C UB); one step past the largest `x` for which `2*x` is representable | wraps mod 2^32: `2*x = -2`, `+300` → prints `298\n` | `errors::e2_int_max` |
| E3 | `driver` | `x = INT_MIN` (-2147483648) → `2*x` signed-overflows negatively (C UB) | wraps: `2*x = 0`, `+300` → prints `300\n` | `errors::e3_int_min` |
| E4 | `driver` | `x = 1073741824` = `INT_MAX/2 + 1`: first value one step past the valid range of the `2*x` multiply | wraps: `2*x = INT_MIN`, `+300` → prints `-2147483348\n` | `errors::e4_first_overflow_positive` |
| E5 | `driver` | `x = -1073741825` = `INT_MIN/2 - 1`: first value one step below the valid range of the `2*x` multiply | wraps: `2*x = 2147483646`, `+300` → prints `-2147483350\n` | `errors::e5_first_overflow_negative` |
| E6 | `driver` | `x = 1073741674` — largest `x` with `2*x` in range whose `y += 300` still overflows (`2*x = 2147483348`, `+300 > INT_MAX`) | the *addition* overflows and wraps → prints `-2147483648\n` | `errors::e6_add_overflow` |
| E7 | `driver` | `x = 1073741673 … 1073741823` sweep around the `y += 300` overflow edge (the exact step from in-range to out-of-range for the second arithmetic op) | last non-overflowing value prints `2147483646`, every following one wraps to negative | `errors::e7_add_overflow_edge_sweep` |
| E8 | `driver` | out-of-range *enum* value across the FFI boundary: no enum exists in the API, so the equivalent is an out-of-range **`int`** — every 32-bit pattern (including `0x80000000`, `0xFFFFFFFF`, `0x7FFFFFFF`) is passed and must be handled identically | no rejection; wrapped arithmetic result printed | `errors::e8_all_bit_patterns_accepted` |
| E9 | `driver` | ABI edge: caller supplies a 64-bit register whose upper 32 bits are garbage (`0xDEADBEEF_00000005`). C reads only `%edi` (`mov %edi,-0x14(%rbp)`), i.e. silently truncates instead of rejecting | truncates to the low 32 bits → prints `310\n` | `errors::e9_upper_bits_truncated` |
| E10 | `driver` | zero / "empty" input (`x = 0`), the degenerate boundary every API has | prints `300\n` | `errors::e10_zero` |
| E11 | `driver` | the library's single I/O call fails: fd 1 closed, so `printf` returns -1. C ignores the return value — it has no error path for this either | returns normally, child exit status 0, nothing printed; must not panic/abort in Rust | `errors::e11_write_error_stdout_closed` |

All eleven rows are covered by `tests/differential.rs` (module `errors`) and are
asserted against **both** `.so` files loaded through `libloading`.

Reproduce with:

```
./verify.sh                      # everything: C build, all feature combos, all rows
cargo test --offline -- --test-threads=1   # just the differential suite
```

Non-applicable generic boundaries, for the record: the API has **no** pointer
parameters (so no null-pointer row), **no** length/size parameters (so no zero-
or oversized-length row), **no** enum parameters (so the out-of-range-enum case
degenerates to E8's arbitrary 32-bit patterns), **no** handles/contexts and
**no** return value.
