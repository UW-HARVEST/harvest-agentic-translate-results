# Error Surface

The source contains no `assert`, `RETURN_ERROR`, `return -1`, `return NULL`,
error enum, public range check, or public null check. It has one rejection
branch, in the internal `get_bits`; that branch is reached through the public
`dequantize_granule` entry point.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `get_bits` via `dequantize_granule` | After `bs->pos += n`, the new position is greater than `bs->limit`; exercised for direct (`bitalloc < 17`) and grouped (`bitalloc >= 17`) reads | Return zero bits for that read, leave `bs->pos` advanced by `n`, continue dequantization, and return `group_size * 4` from `dequantize_granule` |

## Generic FFI Boundaries

- `group_size == 0` is defined by the loops and is covered in `CONFIGS.md`.
- A bit length one step beyond `bs.limit` is row 1 above.
- `total_bands` has no checked/documented range. The largest value that keeps
  `2 * total_bands` within `bitalloc[64]` is 32; larger values make the C
  program access beyond that array and therefore have no defined C result.
- The API defines no enum parameters, so there is no out-of-range enum case.
- Null `sci` is always dereferenced. Null `bs` is dereferenced when a nonzero
  allocation reads bits. Null `grbuf` is written when output is produced.
  The C implementation does not reject these inputs; those calls have
  undefined behavior rather than an error code or sentinel. Active null calls
  are isolated in child processes and their observed termination signals are
  compared. A null `bs` on the allocation-skip path is compared in-process.

## Verification

- [x] Row 1: direct and grouped overrun cases pass randomized differential tests.
- [x] Generic zero and oversized group lengths are covered by configuration rows.
- [x] Active null `grbuf`, `bs`, nested `buf`, and `sci` have matching process results.
- [x] No enum boundary exists in this API.
