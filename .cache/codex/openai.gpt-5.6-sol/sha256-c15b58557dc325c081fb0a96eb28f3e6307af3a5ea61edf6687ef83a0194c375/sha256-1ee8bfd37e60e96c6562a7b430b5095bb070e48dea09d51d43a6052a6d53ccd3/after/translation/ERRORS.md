# Error Surface

Mechanical scan scope: `../c_src/include/lib.h` and
`../c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are zero rows. The C source has no error-return macro, error enum,
`assert`, explicit range check, null check, or rejection branch. `crc16`
requires `d` to identify `len` readable bytes when `len > 0`; violating that
precondition is undefined behavior, not a C rejection result. For `len == 0`,
the pointer is not dereferenced, including when it is null.

## Generic FFI Boundaries

| boundary | coverage | [ ] |
|----------|----------|-----|
| Null `d`, zero `len` | Both libraries return the initial CRC | [x] |
| Null `d`, nonzero `len` | Exact process outcomes compared in isolated subprocesses for lengths 1 and 8 | [x] |
| Zero length | Null and non-null pointers, randomized initial CRCs | [x] |
| Large representable length | 1,048,576 and 1,048,583 readable bytes | [x] |
| Maximum `uint32_t` length with null `d` | Exact undefined-behavior process outcomes compared in isolated subprocesses | [x] |
| One past documented range | Not representable: the C API exposes the full `uint32_t` domain | N/A |
| Out-of-range enum | N/A: the public API has no enum parameter | N/A |
