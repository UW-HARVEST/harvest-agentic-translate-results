# Error Surface

Mechanical searches covered `return`, `assert`, error macros/enums, null checks,
range checks, and min/max constants in `c_src/src/lib.c` and
`c_src/include/lib.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows because `crc16` contains no input rejection or error path. It
returns a `tflac_u16` CRC for every input satisfying its memory contract.

Boundary behavior that is defined by the C implementation:

| Boundary | C behavior | Coverage |
|----------|------------|----------|
| `d == NULL`, `len == 0` | Does not dereference `d`; returns the initial CRC unchanged. | [x] |
| `len == 0`, any pointer | Does not read memory; returns the initial CRC unchanged. | [x] |
| Large valid input (`len == 1 MiB + 7`) | Runs many slicing iterations plus the maximum remainder and returns a CRC. | [x] |

`len == UINT32_MAX` is valid only when `d` addresses that many readable bytes;
the C code has no oversized-length rejection. Processing 4 GiB twice is skipped
under the 600-second command limit. A one-past-range length cannot cross this
ABI because `len` is already `uint32_t`. Likewise, `d == NULL, len > 0` has
undefined behavior from dereferencing null, not a comparable C rejection.

The API has no enum parameters and therefore no out-of-range enum case.

- [x] Every rejection row and executable generic boundary has a passing
  differential test.
