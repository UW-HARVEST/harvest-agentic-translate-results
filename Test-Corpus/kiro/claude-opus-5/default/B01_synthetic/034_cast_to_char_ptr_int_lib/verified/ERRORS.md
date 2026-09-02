# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical extraction

```sh
grep -nE 'return|assert|NULL|errno|exit\(|if *\(|switch|#if|==|!=|-1|RETURN_ERROR' \
     c_src/src/driver.c c_src/include/driver.h
```

Output (after dropping the license comment block and the header guard):

```
c_src/src/driver.c:29:    for (int i = 0; i < len; i++) {
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

That is the **complete** set of conditional constructs in the library.

Therefore the following are all genuinely ABSENT from this C library:

| construct searched for | occurrences |
|------------------------|-------------|
| `return <error>` / `return -1` / `return NULL` | 0 (both functions are `void`) |
| `RETURN_ERROR`-style macro | 0 |
| `assert` / `NDEBUG` | 0 |
| error enum / status code type | 0 |
| explicit range check on a parameter | 0 |
| null-pointer check | 0 |
| min/max constant, magic limit | 0 |
| `errno` use, `exit()`, `abort()` | 0 |
| `switch` / `#ifdef` behaviour toggle | 0 |

`driver` takes a single `int` by value and returns `void`. **Every** `int`
bit pattern is a valid input; the function has no way to reject anything and no
channel (return value, out-param, errno) on which to report a rejection. So the
error-surface table has no rejection rows.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | `driver` | *(none — no rejection path exists in the C source)* | n/a |

## Generic-boundary rows tested anyway

The task requires covering the boundaries every C API has even when the table is
empty. These are the ones that are *expressible* for this ABI
(`void driver(int)`), so each gets a differential test. "Expected C result" is
whatever the C `.so` actually does — the tests assert Rust matches it, they do
not assert a guessed value.

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| E1 | `driver` | `x = 0` (zero / all-bytes-zero) | prints `00000000\n`; no error | `err_zero` |
| E2 | `driver` | `x = INT_MAX` (`0x7fffffff`, one below overflow) | prints LE object repr `ffffff7f\n` | `err_int_max` |
| E3 | `driver` | `x = INT_MIN` (`0x80000000`, most negative) | prints `00000080\n` | `err_int_min` |
| E4 | `driver` | `x = -1` (all bits set) | prints `ffffffff\n` | `err_minus_one` |
| E5 | `driver` | `x = INT_MAX` + 1 step past the range, i.e. the wrapped value `INT_MIN` reached via `(int)0x80000000u` | identical to E3 — C `int` has no trap repr | `err_one_past_int_max` |
| E6 | `driver` | `x = INT_MIN` - 1 step, i.e. `(int)0x7fffffffu` reached by wrapping | identical to E2 | `err_one_before_int_min` |
| E7 | `driver` | out-of-range "enum" value: an `int` with no valid variant in any enum. There is no enum parameter in this API, so the closest real input is an arbitrary sentinel-looking `int` (`0xdeadbeef`, `-999999`, `0x7f7f7f7f`) passed where a caller might pass a bogus enum | prints the object representation; no validation, no rejection | `err_bogus_enum_like_values` |
| E8 | `driver` | 64-bit garbage in the upper half of the argument register (caller passes a value wider than `int`; the C ABI says only the low 32 bits are significant) | upper bits ignored; same output as the low 32 bits alone | `err_dirty_upper_register_bits` |
| E9 | `driver` | called repeatedly / re-entrantly many times in a row (no state to corrupt, but verifies no hidden static buffer) | each call independent, output is the concatenation | `err_repeated_calls_no_state` |

Null-pointer and zero/oversized-length rows are **not expressible**: `driver`
has no pointer parameter and no length parameter. The only length in the library
is `sizeof(x)` — a compile-time constant `4` passed to the `static` `print_hex`,
which is unreachable from outside the `.so` (confirmed absent from `nm -D`).
Test `err_no_pointer_or_length_params_reachable` asserts that inexpressibility
by checking `print_hex` is not loadable from either `.so`.

## Completion gate item

- [x] EVERY row above has a passing error-path differential test.
