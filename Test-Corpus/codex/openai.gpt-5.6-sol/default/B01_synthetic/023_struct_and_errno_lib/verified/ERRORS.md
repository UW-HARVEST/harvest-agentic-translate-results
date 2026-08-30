# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|errno|INT_(MIN|MAX)|if\s*\(' ../c_src
```

The only rejection expression is `driver.c:64`; each failed conjunct is listed
separately. `run` contains no range check, null check, assertion, or error
return.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|----------------------------------------------|-------------------|---------|
| 1 | `driver` / `parse_val` | `endp == str`: `strtol` consumes no characters (for example `""`, whitespace-only, or sign-only input) | prints `An error occurred\n`; returns `void` | [x] |
| 2 | `driver` / `parse_val` | `errno != 0` after `strtol` (decimal magnitude outside the C `long` range produces `ERANGE`) | prints `An error occurred\n`; returns `void` | [x] |
| 3 | `driver` / `parse_val` | `tmp < INT_MIN` while conversion consumed characters and `errno == 0` | prints `An error occurred\n`; returns `void` | [x] |
| 4 | `driver` / `parse_val` | `tmp > INT_MAX` while conversion consumed characters and `errno == 0` | prints `An error occurred\n`; returns `void` | [x] |

Generic FFI boundaries not represented by an explicit C rejection branch are
still mandatory test cases: null pointers for `driver` and `run`, and integer
arithmetic boundaries for `run`. The C API has no length argument and no enum.
