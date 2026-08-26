# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return -1|return NULL|assert|if.*NULL|MIN|MAX|enum' c_src
```

The C source has one rejection branch. It has no error enum, assertion,
explicit input range check, public enum parameter, or null-input guard.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| E01 | `find_value_in_buffer` | `memchr(buffer, (char)search_val, size) == NULL`, including an empty prefix or a prefix containing no matching byte | `-1` | [x] |

## Generic FFI Boundaries

These rows record the mandatory generic boundaries separately from rejection
branches. They are based on the actual C parameter types and loop/memchr
behavior. No positive-length null pointer is listed because dereferencing it is
undefined in C and supplies no deterministic C result to compare.

| # | function | boundary input | expected C result | |
|---|----------|----------------|-------------------|-|
| G01 | `find_value_in_buffer` | `buffer == NULL`, `size == 0` | `-1`; no byte is read | [x] |
| G02 | `find_value_in_buffer` | `size == SIZE_MAX`, with the requested byte at offset zero | `0`; `memchr` stops at the first byte | [x] |
| G03 | `create_numeric_buffer` | `buffer == NULL`, `size == 0` or `size < 0` | returns without accessing `buffer` | [x] |
| G04 | `create_numeric_buffer` | signed length values one step around the loop boundary: `-1`, `0`, `1` | no write, no write, exactly one write | [x] |
| G05 | `convert_double_to_int` | values at/one representable-double step around the signed-int range, plus `NaN` and infinities | the exact `int` bit pattern returned by the built C shared object | [x] |

There are no enum-typed FFI parameters, no documented length maximum, and no
pointer/length parameters on the other four entry points.
