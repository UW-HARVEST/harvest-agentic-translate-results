# Error Surface

Mechanical scan scope: all C source (`c_src/src/main.c`). The scan covered
`RETURN_ERROR`, negative and null returns, error enums, `assert`, explicit
range/min/max checks, null checks, and conditional rejection branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

The C source contains no explicit rejection or error path, so the table has
zero rows. In particular, `driver` does not validate either pointer, and
`main` does not check either `fgets` result before subtracting one from the
corresponding `strlen`.

Generic FFI boundaries not represented by an explicit C rejection row are
still covered in Phase C: null pointers and zero, boundary, and oversized input
lengths. This API has no enum parameters.

- [x] Null `driver` pointers produce the same child process status.
- [x] Zero, 1, 98, 99, 100, 101, 4096, and 1,048,576-byte inputs match.
- [x] Empty stdin/EOF produces the same child process status from `main`.
- [x] Out-of-range enum values are not applicable; the API has no enums.
