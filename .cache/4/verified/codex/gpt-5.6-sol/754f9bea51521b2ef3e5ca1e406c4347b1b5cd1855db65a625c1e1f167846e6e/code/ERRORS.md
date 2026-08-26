# Error Surface

Mechanically derived from the explicit checks and returns in
`c_src/src/lib.c`. There are no assertions, error enums, null checks, or public
enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `get_bits` (internal, through `read_side_info`) | On the first field read, after adding requested width `n`, `bs->pos > bs->limit` (`bs.limit == initial bs.pos` in the public test) | Return `0` for that field and leave `bs->pos` advanced; with no parsed reservoir budget, the public call returns `-1` at its final budget check | [x] |
| E2 | `read_side_info` | Parsed `gr->big_values > 288` | Return `-1` immediately with prior state updates preserved | [x] |
| E3 | `read_side_info` | Window-switching flag is set and parsed `gr->block_type == 0` | Return `-1` immediately with prior state updates preserved | [x] |
| E4 | `read_side_info` | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` after all granules | Return `-1` | [x] |

## Generic FFI Boundaries

`bs`, `gr`, `hdr`, and `bs->buf` are dereferenced without null checks. A null
value therefore has no C error sentinel; tests compare the external process
termination behavior instead of treating it as a normal return. `bs_t` exposes
bit positions rather than a byte-length parameter, and the public API has no
enum parameters. Zero/truncated limits and the one-past-maximum
`big_values == 289` case are covered by E1/E2.
