# Error-Surface Table

Mechanical source scan:

```text
30: if(line != NULL)
44: data = CHAR_MAX
45: if(data > 0)
56: if(data > 0)
67: data = CHAR_MAX
68: if(data > 0)
70: if(data < (CHAR_MAX/2))
90: scanf("%d", &x)
92: if(x)
```

The C source has no `RETURN_ERROR`, `return -1`, `return NULL`, assertions,
error enums, length parameters, or public enum parameters. `main` and its
callees always return normally. Rows 5-6 are the required one-step-past-range
FFI boundary probes; glibc's observed conversion is part of this fixture's C
behavior.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printLine` | `line == NULL` | no output; function returns |
| 2 | `main` | EOF before a `%d` conversion | `scanf` leaves `x == 0`; prints `fffffffe\n`; returns `0` |
| 3 | `main` | first non-whitespace byte is neither a decimal digit nor a valid signed decimal sequence | `scanf` leaves `x == 0`; prints `fffffffe\n`; returns `0` |
| 4 | `main` | input is `+` or `-` without a following decimal digit | conversion fails with `x == 0`; prints `fffffffe\n`; returns `0` |
| 5 | `main` | decimal value is `INT_MAX + 1` (`2147483648`) | glibc stores a nonzero narrowed value; prints the `good` output; returns `0` |
| 6 | `main` | decimal value is `INT_MIN - 1` (`-2147483649`) | glibc stores a nonzero narrowed value; prints the `good` output; returns `0` |
| 7 | `good` (`goodB2G`) | fixed `data == CHAR_MAX`, so `data < CHAR_MAX/2` is false | after `goodG2B` prints `04\n`, prints `data value is too large to perform arithmetic safely.\n` |

Test status: [x] 1, [x] 2, [x] 3, [x] 4, [x] 5, [x] 6, [x] 7
