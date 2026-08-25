# Error Surface

Mechanical source scan:

```sh
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|\
if[[:space:]]*\(|switch[[:space:]]*\(|NULL|enum|MIN|MAX' c_src/src c_src/include
```

The only match is `if (sum > 0.0f)`, which selects normalization and does not
reject input. `gaussian_kernel` returns `void`; the C source has no error
return, sentinel, assertion, explicit range check, null check, enum, or
min/max constant. Consequently, there are no defined rejection rows:

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

Generic FFI boundary obligations:

| # | function | boundary | expected C result | status |
|---|----------|----------|-------------------|--------|
| G1 | `gaussian_kernel` | `dest == NULL`, `size <= -2` | returns normally because the loop performs no access | [x] |
| G2 | `gaussian_kernel` | `size == 0`, valid one-float destination | writes one unnormalized element and returns | [x] |
| G3 | `gaussian_kernel` | oversized practical length (`size == 65536`), destination has `size + 1` floats | completes and preserves C's even-size tail behavior | [x] |

`dest == NULL` with `size >= -1` is undefined behavior in C because line 19
dereferences it; it has no C error result to compare. There are no documented
numeric ranges to step past and no enum parameters.
