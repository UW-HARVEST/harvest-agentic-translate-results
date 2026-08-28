# Error surface

Mechanically derived from every return and rejecting condition in
`../c_src/src/lib.c`. The public header contains no assertions, enums, explicit
null checks, or documented length ranges.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `get_bits` through `read_side_info` | A requested read makes `(bs->pos += n) > bs->limit`; the exercised truncated input has no reservoir allowance | exhausted reads produce zero and `read_side_info` ultimately returns `-1` | [x] |
| 2 | `read_side_info` | Decoded `gr->big_values > 288` (minimum rejecting value: 289) | `-1` | [x] |
| 3 | `read_side_info` | Window-switching flag is 1 and decoded `gr->block_type == 0` | `-1` | [x] |
| 4 | `read_side_info` | After all granules, `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` | `-1` | [x] |

The `scalefac_compress >= 500` check is a valid-path mode selection, not a
rejection, and is covered in `CONFIGS.md`.
