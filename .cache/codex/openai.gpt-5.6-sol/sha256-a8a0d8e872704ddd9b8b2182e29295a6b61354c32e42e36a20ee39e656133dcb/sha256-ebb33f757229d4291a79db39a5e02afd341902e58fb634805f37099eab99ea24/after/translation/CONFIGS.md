# Configuration-surface table

Derived from the two exported entry points and every valid-input branch in
`c_src/src/lib.c`. There are no Cargo features and no C preprocessor feature
branches, so the sole build configuration is the default one.

The `flac_validate` rows cross the interacting public-input axes actually used
by the C branches:

- channel handling: independent (`0`), nonzero preserved (`channels == 2` and
  `bitdepth < 32`), or nonzero reset (`channels != 2` or `bitdepth == 32`);
- rice handling: explicit `1..=30`, default-to-14 (`0`, `bitdepth <= 16`), or
  default-to-30 (`0`, `bitdepth > 16`);
- partition handling: no increment because `min == max`, no increment because
  the initial divisor fails, one-or-more increments to the maximum, or partial
  increment followed by a non-divisible order.

All rows randomize every field not fixed by the stated configuration and cover
valid range boundaries.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tflac_size_memory` | every `u32` blocksize shape: zero, one, normal validation-range values, `2^30` multiplication-wrap boundaries, and `u32::MAX` | [x] |
| 2 | `flac_validate` | independent mode; explicit rice `1..=30`; partition fixed (`min == max`) | [x] |
| 3 | `flac_validate` | independent mode; explicit rice; partition increments through one or more divisible orders to `max` | [x] |
| 4 | `flac_validate` | independent mode; explicit rice; partition increments partially then stops on non-divisibility | [x] |
| 5 | `flac_validate` | independent mode; rice `0` with bitdepth `1..=16` defaults to `14`; partition fixed | [x] |
| 6 | `flac_validate` | independent mode; rice `0` with bitdepth `1..=16`; partition reaches `max` | [x] |
| 7 | `flac_validate` | independent mode; rice `0` with bitdepth `1..=16`; partition stops before `max` | [x] |
| 8 | `flac_validate` | independent mode; rice `0` with bitdepth `17..=32` defaults to `30`; partition fixed | [x] |
| 9 | `flac_validate` | independent mode; rice `0` with bitdepth `17..=32`; partition reaches `max` | [x] |
| 10 | `flac_validate` | independent mode; rice `0` with bitdepth `17..=32`; partition stops before `max` | [x] |
| 11 | `flac_validate` | nonzero valid mode `1..=3`, two channels, bitdepth `< 32`: mode preserved; explicit rice; partition fixed | [x] |
| 12 | `flac_validate` | nonzero valid mode, two channels, bitdepth `< 32`: mode preserved; explicit rice; partition reaches `max` | [x] |
| 13 | `flac_validate` | nonzero valid mode, two channels, bitdepth `< 32`: mode preserved; explicit rice; partition stops before `max` | [x] |
| 14 | `flac_validate` | out-of-range mode `4..=255`, two channels, bitdepth `< 32`: C still preserves the nonzero byte; rice default or explicit; all partition shapes | [x] |
| 15 | `flac_validate` | nonzero mode with channels other than two: mode reset to independent; rice default or explicit; all partition shapes | [x] |
| 16 | `flac_validate` | nonzero mode with two channels and bitdepth `32`: mode reset to independent; rice default or explicit; all partition shapes | [x] |
| 17 | `flac_validate` | valid scalar lower boundaries (`blocksize=16`, samplerate/channels/bitdepth `=1`) with arbitrary initial output fields | [x] |
| 18 | `flac_validate` | valid scalar upper boundaries (`blocksize=65535`, `samplerate=655350`, `channels=8`, `bitdepth=32`) with arbitrary initial output fields | [x] |
| 19 | `flac_validate` | blocksize divisibility at partition-order boundaries `0` and `15`, including odd and maximally divisible valid blocksizes | [x] |
| 20 | `flac_validate` | `min_partition_order < max_partition_order`, but blocksize is not divisible by the first tested divisor, so partition order does not increment | [x] |

The CMake project and Cargo crate build shared libraries only; there is no
binary-driver stdout surface to compare.
