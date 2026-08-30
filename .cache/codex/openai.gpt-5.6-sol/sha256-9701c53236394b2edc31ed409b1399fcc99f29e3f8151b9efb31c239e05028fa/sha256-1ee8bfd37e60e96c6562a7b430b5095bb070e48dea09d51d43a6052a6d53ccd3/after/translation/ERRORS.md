# Error surface

Mechanical inspection commands covered all C source and headers:

```text
rg -n "RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|if|switch|case|NULL|MIN|MAX|enum|[<>]=?" ../c_src/include ../c_src/src
```

The C implementation contains no explicit rejection, error return, assertion,
null check, range check, enum, or min/max constant. Therefore the source-derived
explicit rejection table has zero rows.

## Generic FFI boundary cases

The task additionally requires generic boundary coverage even when the C source
does not reject those inputs. The expected results below were measured in an
isolated subprocess against the built C shared object. `driver` and `fma_array`
return `void`, so normal completion/no mutation and process signal are the only
observable rejection outcomes.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `driver` | `data == NULL`, `len == 0` | returns normally; writes no output | [x] |
| 2 | `driver` | `data == NULL`, `len == 1` | process terminates with `SIGSEGV` | [x] |
| 3 | `driver` | non-null one-element `data`, `len == -1` | process terminates with `SIGSEGV` | [x] |
| 4 | `driver` | non-null one-element `data`, `len == INT_MAX` | process terminates with `SIGSEGV` | [x] |
| 5 | `fma_array` | all pointers `NULL`, `len == 0` | returns normally; no memory is accessed | [x] |
| 6 | `fma_array` | all pointers `NULL`, `len == -1` | returns normally; no memory is accessed | [x] |
| 7 | `fma_array` | `out == NULL`, other pointers valid, `len == 1` | process terminates with `SIGSEGV` | [x] |
| 8 | `fma_array` | `mul1 == NULL`, other pointers valid, `len == 1` | process terminates with `SIGSEGV` | [x] |
| 9 | `fma_array` | `mul2 == NULL`, other pointers valid, `len == 1` | process terminates with `SIGSEGV` | [x] |
| 10 | `fma_array` | `add == NULL`, other pointers valid, `len == 1` | process terminates with `SIGSEGV` | [x] |
| 11 | `fma_array` | all pointers reference one element, `len == INT_MAX` | process terminates with `SIGSEGV` | [x] |

There are no public enums and no documented finite maximum length, so there is
no enum-out-of-range case or documented-range-plus-one case beyond the
`INT_MAX` oversized-length probes above.
