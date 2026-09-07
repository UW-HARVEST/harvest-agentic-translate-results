# Error surface

Mechanically derived from the checks and returns in `c_src/src/lib.c`.
`read_side_info` has no null checks or assertions; null `bs`, `gr`, or `hdr`
therefore invokes C undefined behavior rather than a library-defined error.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `get_bits` through `read_side_info` | A requested field crosses the bitstream boundary: after `bs->pos += n`, `bs->pos > bs->limit` | That field read returns `0` after advancing `bs->pos`; parsing continues, and the final reservoir check rejects with `-1` for the zero-length case used by the differential test. [x] |
| 2 | `read_side_info` | A parsed 9-bit `big_values` field is greater than `288` | Return `-1` immediately. [x] |
| 3 | `read_side_info` | `window_switching_flag` is 1 and the following 2-bit `block_type` is 0 | Return `-1` immediately. [x] |
| 4 | `read_side_info` | After all granules, `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` | Return `-1`. [x] |

Generic FFI boundaries additionally required by Phase C:

- [x] null `bs`, `gr`, and `hdr` pointers have matching process-level behavior;
- [x] zero `bs->limit`;
- [x] oversized but safely backed `bs->limit`;
- [x] one-past-valid `big_values == 289`;
- [x] all 2-bit sample-rate codes, including code 3, across all version-bit modes;
- [x] no public C enum parameters exist in this API.
