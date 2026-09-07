# ERRORS.md — Error-surface table

Mechanical grep of the whole C source (`c_src/src/driver.c`, 32 lines, and
`c_src/include/driver.h`) for every rejection / error path:

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if|switch|<|>|==|!=|\?|goto|ERROR|MIN|MAX|#if' c_src/src/driver.c
(no matches other than the license text and `#include`)
```

Findings:

- no `return` statement (the function is `void`)
- no `assert`
- no `if` / `switch` / ternary / `goto` — **zero branches**
- no null checks (the function takes no pointers)
- no range checks, no `MIN`/`MAX` constants, no error enums, no `errno` use
- no error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, …)
- the only library call is `printf`, whose return value is discarded

**The C library therefore has an EMPTY error surface: there is no input value
of `int x` that it rejects.** Every one of the 2^32 possible arguments is
accepted and produces output. That is the ground truth the Rust must match, so
the table below records, instead of rejections, the boundary/extremal inputs
that a rejection would plausibly have guarded — each must behave *identically*
(no error, no panic, no abort) in both implementations.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `driver` | *no rejection path exists at all* (no `return`/`assert`/range check in the source) | n/a — nothing is ever rejected |
| 2  | `driver` | `x = 0` (degenerate/zero input) | no error; prints `300\n` |
| 3  | `driver` | `x = INT_MAX` (`2147483647`) — `2*x` signed overflow (C UB; the build wraps two's-complement, confirmed at both `-O0` and `-O2`) | no error; prints the wrapped value `298\n` |
| 4  | `driver` | `x = INT_MIN` (`-2147483648`) — `2*x` signed overflow, negative direction | no error; prints `300\n` (wrapped) |
| 5  | `driver` | `x = INT_MAX/2 = 1073741823` — largest `x` with `2*x` in range, but `2*x+300` overflows | no error; prints `-2147483350\n` (wrapped) |
| 6  | `driver` | `x = 1073741673` — last non-overflowing input (`2*x+300 == INT_MAX-1`; the result is always even so `INT_MAX` itself is unreachable) | no error; prints `2147483646\n` |
| 7  | `driver` | `x = 1073741674` — one step past the last non-overflowing input; `2*x+300` overflows by 2 | no error; prints `-2147483648\n` (wrapped) |
| 8  | `driver` | `x = INT_MIN/2 = -1073741824` — boundary of negative `2*x` overflow | no error; prints `-2147483348\n` (wrapped) |
| 9  | `driver` | `x = -1073741824 - 1` (`-1073741825`), one step past that boundary | no error; prints `-2147483350\n` (wrapped) |
| 10 | `driver` | `x = -150` — the exact root, `2*x+300 == 0` (prints `0`, not the empty string) | no error; prints `0\n` |
| 11 | `driver` | out-of-range "enum" value: the parameter is a plain `int`, so *every* bit pattern including `0x80000000` and `0xFFFFFFFF` is a legal argument with no valid-variant concept | no error; prints the wrapped arithmetic result |
| 12 | `driver` | called repeatedly / no init required (no hidden state, no "not initialised" error) | no error; each call independent |

Rows 2–12 are all covered by `tests/differential.rs`
(`errors_boundary_and_extremal_inputs`, `errors_no_rejection_path_is_reachable`,
`errors_repeated_calls_have_no_hidden_state`, `errors_full_int_bit_patterns`).
Row 1 is a statement about the source, verified by the grep above and by the
fact that no invalid input could be constructed for rows 2–12.

## Verification result

All rows have a passing differential test (`cargo test --offline`):

```
test errors_no_rejection_path_is_reachable ... ok    (row 1)
test errors_boundary_and_extremal_inputs ... ok      (rows 2-10)
test errors_expected_exact_bytes ... ok              (rows 2-10, exact bytes pinned)
test errors_full_int_bit_patterns ... ok             (row 11)
test errors_repeated_calls_have_no_hidden_state ... ok (row 12)
```

Note on "null pointers / zero and oversized lengths / out-of-range enum values":
`driver` takes a single `int` by value and no pointers, buffers, lengths or
enums, so those generic boundary classes collapse into "every `int` bit
pattern", which is covered by row 11 *and* by the exhaustive sweep
(`exhaustive.sh`: all 2^32 arguments compared, byte-for-byte identical).
