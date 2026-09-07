# Error-surface table

Derived mechanically from every rejection branch in `c_src/src/lib.c`.
For `flac_validate`, the expected result includes both the return value and all
bytes of the caller-owned `tflac` structure after the call.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `flac_validate` | `blocksize < 16` (including zero) | [x] return `-1`; structure unchanged |
| 2 | `flac_validate` | `blocksize > 65535` | [x] return `-1`; structure unchanged |
| 3 | `flac_validate` | valid blocksize and `samplerate == 0` | [x] return `-1`; structure unchanged |
| 4 | `flac_validate` | valid blocksize and `samplerate > 655350` | [x] return `-1`; structure unchanged |
| 5 | `flac_validate` | prior fields valid and `channels == 0` | [x] return `-1`; structure unchanged |
| 6 | `flac_validate` | prior fields valid and `channels > 8` | [x] return `-1`; structure unchanged |
| 7 | `flac_validate` | prior fields valid and `bitdepth == 0` | [x] return `-1`; structure unchanged |
| 8 | `flac_validate` | prior fields valid and `bitdepth > 32` | [x] return `-1`; structure unchanged |
| 9 | `flac_validate` | prior scalar ranges valid and `max_rice_value > 30` | [x] return `-1`; `channel_mode` may already have been reset to `0`; all later fields unchanged |
| 10 | `flac_validate` | prior checks valid and `max_partition_order > 15` | [x] return `-1`; channel-mode normalization and rice defaulting may already have occurred |
| 11 | `flac_validate` | prior checks valid, `max_partition_order <= 15`, and `min_partition_order > max_partition_order` | [x] return `-1`; channel-mode normalization and rice defaulting may already have occurred |
| 12 | `flac_validate` | `t == NULL` | [x] process is rejected by the same fatal signal/termination behavior as C |

Generic FFI boundaries that do not reject are tracked in `CONFIGS.md`, including
zero/oversized `tflac_size_memory` inputs and out-of-range channel-mode values.
