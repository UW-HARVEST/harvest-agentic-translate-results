# Error Surface

The following mechanical searches were applied to `../c_src/include` and
`../c_src/src`:

```text
RETURN_ERROR
return -1
return NULL
assert(...)
if (...)
NULL
min / max
error
```

The implementation contains no rejection branch, assertion, explicit range or
null check, error enum, or error-return sentinel. Its only return statement
returns the computed hash.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

- [x] No explicit C rejection rows exist.

## Generic FFI boundaries

These are not error rows because the C implementation does not reject them:

| Boundary | C behavior | Coverage |
|----------|------------|----------|
| `stbds_hash_bytes(NULL, 0, seed)` | Valid: no pointer dereference; returns the empty-input hash. | [x] |
| Zero length with a non-null pointer | Valid: returns the empty-input hash. | [x] |
| Large length backed by an equally large allocation | Valid: no documented maximum; hashes all bytes. | [x] |
| Null pointer with positive length | Undefined behavior from dereferencing `NULL`; no stable C result exists to compare. | N/A |
| Length larger than the pointed-to allocation | Undefined behavior from an out-of-bounds read; no stable C result exists to compare. | N/A |
| Out-of-range enum | Neither public entry point accepts an enum. | N/A |
| One past documented range | No input range is documented or checked. | N/A |

Undefined-behavior cases are intentionally not invoked by the test process.
