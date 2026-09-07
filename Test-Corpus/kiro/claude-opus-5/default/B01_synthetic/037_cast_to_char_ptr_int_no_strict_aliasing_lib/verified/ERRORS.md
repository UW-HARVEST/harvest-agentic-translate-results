# ERRORS.md — Phase C error-surface table

Derived mechanically, not from assumptions. The whole library is 14 non-comment
lines (`c_src/src/driver.c`) plus a 3-line header. The exhaustive grep for every
rejection mechanism:

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|ERROR|<0|< 0|>=|<=|if *\(|switch|#if|goto' \
     c_src/src/driver.c c_src/include/driver.h
```

→ the **only** match in the entire C source tree is `include/driver.h:24:#ifndef DRIVER_H_`
(a header include guard, not an error path).

## Consequence

`void driver(int x)` is a `void` function that takes a single by-value `int`. It
performs **no** validation of any kind:

* no error-return macro or `return <sentinel>` statement — it cannot return a value at all;
* no `assert`;
* no explicit range check, no min/max constant;
* no null check — it accepts no pointer, so a null pointer is not a representable input;
* no error enum, and no enum parameter — so there is no out-of-range-enum input either;
* no `errno` use, no `exit`/`abort`;
* the only conditional in the file is the loop guard `i < len` in `print_hex`,
  and `len` is a compile-time `sizeof(x)` == 4, never attacker-controlled.

Every one of the 2^32 values of `x` is therefore a **valid** input with defined
output. The error surface is genuinely EMPTY, and the rows below record that
fact together with the tests that *prove* the C accepts these inputs rather
than rejecting them — i.e. each row asserts C and Rust agree that no rejection
occurs, which is the C's actual behaviour.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `driver` | `x = INT_MIN` (`-2147483648`) — one step past the negative end of the value range | no rejection: prints `00000080\n`, returns normally | `err_row1_int_min` | [x] |
| 2 | `driver` | `x = INT_MAX` (`2147483647`) — one step past the positive end of the value range | no rejection: prints `ffffff7f\n`, returns normally | `err_row2_int_max` | [x] |
| 3 | `driver` | `x = 0` — the "zero length / empty" analogue for the sole scalar argument | no rejection: prints `00000000\n`, returns normally | `err_row3_zero` | [x] |
| 4 | `driver` | `x = -1` — all-bits-set; every byte is `0xff`, the high-bit/sign-extension trap | no rejection: prints `ffffffff\n`, returns normally (NOT `ffffffffffffffff…`) | `err_row4_all_bits_set` | [x] |
| 5 | `driver` | out-of-range "enum-like" value: an `unsigned`/64-bit value passed across FFI in the `int` slot (e.g. `0xFFFFFFFF`, `0x1_0000_0000u64` truncated) — a C enum accepts any `int`, so this models a variant with no valid name | no rejection: the low 32 bits are reinterpreted; `0xFFFFFFFF` → `ffffffff\n`, `0x1_0000_0000` truncates to `00000000\n` | `err_row5_out_of_range_enum_like` | [x] |
| 6 | `print_hex` (static, unreachable) | `len <= 0`, or `p == NULL` with `len == 0` | unreachable from the public ABI: `print_hex` has internal linkage and its only call site passes `raw` (non-null) and `sizeof(raw)` == 4. Neither `.so` exports it, so no external caller can construct these conditions — asserted by `dlsym("print_hex")` failing on the C `.so` AND both Rust `.so`s. | `err_row6_print_hex_not_reachable` | [x] |

All 6 rows are covered by passing differential tests in
`translation/tests/differential.rs` (module `phase_c`).
