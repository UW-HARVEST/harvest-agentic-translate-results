# Error Surface

Mechanical source scan:

```sh
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|assert|if\s*\(|NULL|MIN|MAX' \
  ../c_src/include ../c_src/src
```

The C source has no error macro, assertion, null check, enum validation,
explicit range rejection, `-1` return, or `NULL` return. Its only explicit
input-rejection branch is the failed integer scan in `driver`; the two rows
below distinguish its two observable states.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `driver` | The first `sscanf(in, "%d%zn", ...)` returns a value other than 1 because the input is empty, whitespace-only, or begins with a non-integer token. | Stop with zero accepted values; `call_fma(data, 0)` returns 0; print `0\n`. | [x] |
| 2 | `driver` | A later `sscanf(in, "%d%zn", ...)` returns a value other than 1 after one or more integers were accepted. | Stop at the malformed token; print the last integer accepted before it. | [x] |

Generic FFI boundary cases not represented by explicit C rejection branches
(null pointers, non-positive lengths, and oversized lengths) are covered by
the Phase C boundary tests. Passing a pointer that C dereferences as null, or a
negative/otherwise invalid VLA length to `call_fma`, has undefined C behavior
and therefore has no deterministic error code or sentinel to place in this
table.
