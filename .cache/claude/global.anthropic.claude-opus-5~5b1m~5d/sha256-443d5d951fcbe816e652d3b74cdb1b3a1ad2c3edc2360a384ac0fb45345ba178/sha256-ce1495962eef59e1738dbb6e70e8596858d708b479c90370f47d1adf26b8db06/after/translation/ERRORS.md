# ERRORS.md — Error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h` by
grepping for every rejection mechanism a C library can use:

```
grep -nE "return|assert|NULL|errno|exit|abort|if |else|switch|#if|malloc|free|MAX|MIN" \
     c_src/src/driver.c c_src/include/driver.h
```

Result of that grep (excluding comments/include guards): the only matches are
`#include <stdio.h>`, `#include <string.h>`, the `for (int i = 0; i < len; i++)`
loop header, `printf("%02x", p[i])`, and `char raw[sizeof(house)]`.

## Mechanical findings

| mechanism searched for | occurrences in C |
|---|---|
| `RETURN_ERROR`-style macros | 0 |
| `return <error>` / `return -1` / `return NULL` | 0 (both functions are `void`; no `return` statement at all) |
| error enums / status codes | 0 |
| `assert` / `abort` / `exit` | 0 |
| explicit range checks / null checks on parameters | 0 |
| `MIN`/`MAX` constants, capacity limits | 0 |
| heap allocation that can fail (`malloc`) | 0 |

**Conclusion: the C library has an empty error surface.** `driver(int)` returns
`void`, validates nothing, and accepts every one of the 2^32 possible `int`
values as valid input. There is therefore no error code or sentinel to compare.

## Error-surface table

Because the C code contains zero rejection branches, there are no
"invalid input → error result" rows to write. What follows is the complete set of
*rejection-adjacent* conditions that do exist in the C — the implicit boundaries
of the only loop and the only parameter — plus the generic FFI boundary cases
Phase C mandates. Each row is covered by a differential test that asserts C and
Rust behave **identically** (same bytes on stdout, no crash, no trap), which is
the correct analogue of "same error code" for a `void`, non-validating API.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `driver` | `floors = 0` (zero / falsy boundary value) | no rejection; prints `00000000030000000000000000000040\n` | `err_e1_zero` |
| E2 | `driver` | `floors = -1` (all-bits-set; negative, would be an error sentinel in most APIs) | no rejection; prints `ffffffff...` | `err_e2_negative_one` |
| E3 | `driver` | `floors = INT_MIN` (`-2147483648`, one step past the negative end of the valid `int` range) | no rejection; prints `00000080...` | `err_e3_int_min` |
| E4 | `driver` | `floors = INT_MAX` (`2147483647`, the positive extreme) | no rejection; prints `ffffff7f...` | `err_e4_int_max` |
| E5 | `driver` | `floors = INT_MIN + 1` / `INT_MAX - 1` (one step inside each extreme) | no rejection; low 4 bytes are the two's-complement LE encoding | `err_e5_one_step_inside_extremes` |
| E6 | `driver` | out-of-range "enum-like" int passed across FFI: the C prototype has no enum, so *every* `int` is out of any conceivable valid variant set. Probe `-2, 3, 4, 255, 256, 65536, 0x7FFFFFFF, 0x80000000` reinterpreted as `int` | no rejection; each is copied verbatim into `house.floors` | `err_e6_out_of_range_enum_like_values` |
| E7 | `driver` | unsigned value `> INT_MAX` (`0xFFFFFFFFu`, `0x80000000u`) passed where a signed `int` is expected (implementation-defined conversion in the caller) | no rejection; identical 4 LE bytes | `err_e7_unsigned_overflow_values` |
| E8 | `print_hex` (internal) | `len <= 0` → loop body never runs, only `"\n"` is printed | unreachable from the public API: `driver` always passes `sizeof(house) == 16`. Verified indirectly: output is always exactly 33 bytes | `err_e8_len_is_always_16_no_empty_loop` |
| E9 | `print_hex` (internal) | `p == NULL` (the only pointer in the library) | unreachable from the public API: `driver` always passes `&raw`, a live stack array. `print_hex` is `static`, so no external caller can supply NULL — confirmed by `nm -D` (see `SYMBOLS.md`): the symbol is not exported by either `.so` | `err_e9_print_hex_not_reachable_externally` |
| E10 | `driver` | called repeatedly / after previously-observed values, probing for retained state that could make a later call reject | no state: `house` and `raw` are function-local; every call is independent | `err_e10_no_retained_state_between_calls` |

Notes on rows deliberately NOT invented: there is no row for "null pointer
argument to `driver`" or "zero/oversized length argument to `driver`" because
`driver` takes neither a pointer nor a length — its sole parameter is a
by-value `int`. Fabricating such rows would mean testing an API that does not
exist. The generic null/length boundaries are instead discharged at the only
places pointers and lengths occur in this library, rows E8 and E9.

## Status

- [x] E1 — [x] E2 — [x] E3 — [x] E4 — [x] E5 — [x] E6 — [x] E7 — [x] E8 — [x] E9 — [x] E10

All rows have a passing differential test against both `.so` files.
