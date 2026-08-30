# Error Surface

Mechanical source scan:

```sh
rg -n 'return|assert|if|switch|case|NULL|MIN|MAX|ERROR|enum|#define|#ifdef|#if' \
  ../c_src/include ../c_src/src
```

The C source has no error returns, assertions, enums, lengths, or min/max
constants. Its two input-rejection paths are the null guard and the guarded
division. Float zero and non-finite values passed to unguarded `bad` are not
rejected by the C implementation and are therefore valid configuration cases.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `printLine` | `line == NULL` | Return `void` without output. |
| [x] 2 | `good` / internal `goodB2G` | `!(fabs(data) > 0.000001)`, comprising `fabs(data) <= 0.000001` and unordered `NaN` | Print `50\nThis would result in a divide by zero\n`, then return `void`. |
| [x] 3 | `driver` through `good` / internal `goodB2G` | `!(fabs(goodData) > 0.000001)`, comprising `fabs(goodData) <= 0.000001` and unordered `NaN` | Print the normal driver framing, `50\n`, and the divide-by-zero warning before continuing through `bad`. |

## Generic FFI Boundaries

- [x] Null pointer: applicable to `printLine`; row 1.
- [x] Zero and signed zero: applicable to float arguments; covered as guarded
  rejection for `good`/`driver` and as valid input for `bad`.
- [x] Integer minimum and maximum: applicable to `printIntLine`.
- [x] Exact and one-step threshold boundaries: applicable to `good` and
  `driver`.
- [x] Oversized lengths: not applicable; no API accepts a length.
- [x] Out-of-range enums: not applicable; no API accepts an enum.
