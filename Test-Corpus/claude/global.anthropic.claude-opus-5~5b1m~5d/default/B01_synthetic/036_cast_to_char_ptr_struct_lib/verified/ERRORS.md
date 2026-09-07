# ERRORS.md — Phase C error-surface table

## Mechanical grep of the entire C source for rejection/error constructs

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if *\(|else|switch|case|#if' \
      c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

The single match is the header's include guard — **not** a rejection.
Widening the pattern to include comparison operators adds exactly one more
match, `c_src/src/driver.c:35: for (int i = 0; i < len; i++)`, which is the
loop-bound comparison of `print_hex`'s `for` — a loop condition, not a
rejection either. There are no other conditionals anywhere in the library.

```
$ grep -c 'return'  c_src/src/driver.c    -> 0
$ grep -c 'assert'  c_src/src/driver.c    -> 0
$ grep -c 'NULL'    c_src/src/driver.c    -> 0
$ grep -c 'errno'   c_src/src/driver.c    -> 0
$ grep -c '#if'     c_src/src/driver.c    -> 0
```

## Error-surface table

The C library has **ZERO explicit rejection paths**:

* `void driver(int)` returns `void` — there is no error channel at all
  (no return code, no out-param status, no sentinel, no `errno` set).
* There are no `assert`s, no range checks, no null checks, no min/max
  constants, no error enums, and no `#ifdef`-gated error paths.
* `driver` takes an `int` **by value**; every one of the 2^32 bit patterns of
  `int` is a valid input and is stored verbatim into `house.floors`.
* The only pointer in the translation unit (`print_hex`'s `p`) is always
  `&house` of the caller's own stack frame, and `len` is always
  `sizeof(house_t)` — a compile-time constant. Neither is ever attacker
  controlled, so there is nothing to reject.

Therefore the table below has no "invalid input" rows to derive; instead it
records, one row per *candidate* rejection an API of this shape could have,
what the C **actually** does (accepts it and prints). Each row is a real
differential test in `tests/differential.rs`.

| # | function | trigger (the exact invalid / boundary input or condition) | expected C result | test |
|---|----------|-----------------------------------------------------------|-------------------|------|
| E1 | `driver` | `floors == 0` (zero-length-ish / falsy boundary) | no error; prints `00000000` + `03000000` + `0000000000000040` + `\n` | `err_e1_zero` |
| E2 | `driver` | `floors == INT_MAX` (`2147483647`, one past the largest "sane" count) | no error; prints `ffffff7f03000000` + `0000000000000040` | `err_e2_int_max` |
| E3 | `driver` | `floors == INT_MIN` (`-2147483648`, most negative, no positive counterpart) | no error; prints `00000080...` | `err_e3_int_min` |
| E4 | `driver` | `floors == -1` (all-bits-set / classic C error sentinel passed *in*) | no error; prints `ffffffff...` | `err_e4_minus_one` |
| E5 | `driver` | `floors` negative in general (a "count" that cannot exist) — **no validation**, value stored as-is | no error; two's-complement bytes printed | `err_e5_negatives` |
| E6 | `driver` | `floors == INT_MAX + 1` as seen by C, i.e. the caller passes `0x80000000` as an *unsigned* 32-bit value through the FFI boundary (wraps to `INT_MIN`) | no error; identical to E3 | `err_e6_unsigned_wraparound` |
| E7 | `driver` | `floors` = an out-of-range **enum-like** value: the caller passes an `int` that corresponds to no meaningful variant (e.g. `0x7ffffffe`, `12345678`, `-999999`). C enums accept any `int`; `driver`'s parameter is a bare `int` with no variant set at all, so every such value is accepted | no error; value printed verbatim | `err_e7_out_of_range_enum_values` |
| E8 | `driver` | value whose 4 bytes contain embedded NUL bytes (e.g. `0x00FF00FF`, `256`, `65536`) — would truncate a string-based impl | no error; NUL bytes printed as `00`, all 16 bytes always emitted | `err_e8_embedded_nul_bytes` |
| E9 | `driver` | `driver` called repeatedly / re-entrantly many times in a row (no init required, no global state to corrupt) | no error; each call prints an independent 33-byte line; no state carried over | `err_e9_repeated_calls_no_state` |
| E10 | `driver` (via `print_hex`) | `len` is never caller-controlled: `sizeof(house_t)` is hardcoded, so a 0 / negative / oversized length is **unreachable** from the public API. Verified structurally: the output line is *always* exactly `2*16 + 1 == 33` bytes for every input | no error; length invariant holds for all inputs | `err_e10_output_length_invariant` |
| E11 | `driver` | null pointer: the public API takes **no pointers**, so passing NULL is not expressible. Verified structurally (`driver` has exactly one `int` parameter in `include/driver.h`) | N/A — not reachable | documented (`err_e10_output_length_invariant` covers the pointer-free contract) |

All rows checked off — see the `err_*` tests in `tests/differential.rs`.

- [x] E1
- [x] E2
- [x] E3
- [x] E4
- [x] E5
- [x] E6
- [x] E7
- [x] E8
- [x] E9
- [x] E10
- [x] E11 (structurally unreachable; no pointer parameters in the public API)
