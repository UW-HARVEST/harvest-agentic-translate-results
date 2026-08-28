# Configuration Surface

This table is derived from the public header and all non-error branches in
`c_src/src/lib.c`. Every `flac_validate` row also varies valid `blocksize`,
`samplerate`, `channels`, `bitdepth`, initial `partition_order`, and
`cur_blocksize` values where the named constraints permit.

Channel-mode classes:

- **M0**: `channel_mode == 0`; preserve it.
- **M1**: `channel_mode != 0`, `channels == 2`, and `bitdepth != 32`; preserve
  the byte. Inputs include declared values 1 through 3 and out-of-range bytes.
- **M2**: `channel_mode != 0` and `channels != 2`; normalize it to 0. Inputs
  include declared values 1 through 3 and out-of-range bytes.
- **M3**: `channel_mode != 0`, `channels == 2`, and `bitdepth == 32`; normalize
  it to 0. Inputs include declared values 1 through 3 and out-of-range bytes.

Rice-value classes:

- **R0**: `max_rice_value == 0` and `bitdepth <= 16`; set it to 14.
- **R1**: `max_rice_value == 0` and `bitdepth > 16`; set it to 30.
- **R2**: `max_rice_value` is 1 through 30; preserve it.

Partition classes:

- **P0**: `min_partition_order == max_partition_order`; output that order.
- **P1**: `min < max` and blocksize is not divisible by
  `2^(min + 1)`; output `min`.
- **P2**: `min < max` and divisibility advances the order all the way to
  `max`; output `max`.
- **P3**: divisibility advances at least once, then fails before `max`; output
  the intermediate order.

`M3/R0` is impossible because M3 fixes bitdepth at 32. No other cross-product
combination is pruned by the C branches.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `tflac_size_memory` | Full `uint32_t` input domain, including 0, small values, arithmetic boundaries, and `UINT32_MAX` | [x] |
| 2 | `flac_validate` | M0 + R0 + P0 | [x] |
| 3 | `flac_validate` | M0 + R0 + P1 | [x] |
| 4 | `flac_validate` | M0 + R0 + P2 | [x] |
| 5 | `flac_validate` | M0 + R0 + P3 | [x] |
| 6 | `flac_validate` | M0 + R1 + P0 | [x] |
| 7 | `flac_validate` | M0 + R1 + P1 | [x] |
| 8 | `flac_validate` | M0 + R1 + P2 | [x] |
| 9 | `flac_validate` | M0 + R1 + P3 | [x] |
| 10 | `flac_validate` | M0 + R2 + P0 | [x] |
| 11 | `flac_validate` | M0 + R2 + P1 | [x] |
| 12 | `flac_validate` | M0 + R2 + P2 | [x] |
| 13 | `flac_validate` | M0 + R2 + P3 | [x] |
| 14 | `flac_validate` | M1 + R0 + P0 | [x] |
| 15 | `flac_validate` | M1 + R0 + P1 | [x] |
| 16 | `flac_validate` | M1 + R0 + P2 | [x] |
| 17 | `flac_validate` | M1 + R0 + P3 | [x] |
| 18 | `flac_validate` | M1 + R1 + P0 | [x] |
| 19 | `flac_validate` | M1 + R1 + P1 | [x] |
| 20 | `flac_validate` | M1 + R1 + P2 | [x] |
| 21 | `flac_validate` | M1 + R1 + P3 | [x] |
| 22 | `flac_validate` | M1 + R2 + P0 | [x] |
| 23 | `flac_validate` | M1 + R2 + P1 | [x] |
| 24 | `flac_validate` | M1 + R2 + P2 | [x] |
| 25 | `flac_validate` | M1 + R2 + P3 | [x] |
| 26 | `flac_validate` | M2 + R0 + P0 | [x] |
| 27 | `flac_validate` | M2 + R0 + P1 | [x] |
| 28 | `flac_validate` | M2 + R0 + P2 | [x] |
| 29 | `flac_validate` | M2 + R0 + P3 | [x] |
| 30 | `flac_validate` | M2 + R1 + P0 | [x] |
| 31 | `flac_validate` | M2 + R1 + P1 | [x] |
| 32 | `flac_validate` | M2 + R1 + P2 | [x] |
| 33 | `flac_validate` | M2 + R1 + P3 | [x] |
| 34 | `flac_validate` | M2 + R2 + P0 | [x] |
| 35 | `flac_validate` | M2 + R2 + P1 | [x] |
| 36 | `flac_validate` | M2 + R2 + P2 | [x] |
| 37 | `flac_validate` | M2 + R2 + P3 | [x] |
| 38 | `flac_validate` | M3 + R1 + P0 | [x] |
| 39 | `flac_validate` | M3 + R1 + P1 | [x] |
| 40 | `flac_validate` | M3 + R1 + P2 | [x] |
| 41 | `flac_validate` | M3 + R1 + P3 | [x] |
| 42 | `flac_validate` | M3 + R2 + P0 | [x] |
| 43 | `flac_validate` | M3 + R2 + P1 | [x] |
| 44 | `flac_validate` | M3 + R2 + P2 | [x] |
| 45 | `flac_validate` | M3 + R2 + P3 | [x] |
