# Error Surface

Derived from every `return -1` branch in `c_src/src/lib.c`. Each trigger below
assumes all checks before that branch receive valid values unless the row says
otherwise. `flac_validate` has no error enum: every explicit rejection returns
the integer `-1`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `flac_validate` | `t->blocksize < 16` | `-1` | [x] |
| 2 | `flac_validate` | `t->blocksize > 65535` | `-1` | [x] |
| 3 | `flac_validate` | `t->samplerate == 0` | `-1` | [x] |
| 4 | `flac_validate` | `t->samplerate > 655350` | `-1` | [x] |
| 5 | `flac_validate` | `t->channels == 0` | `-1` | [x] |
| 6 | `flac_validate` | `t->channels > 8` | `-1` | [x] |
| 7 | `flac_validate` | `t->bitdepth == 0` | `-1` | [x] |
| 8 | `flac_validate` | `t->bitdepth > 32` | `-1` | [x] |
| 9 | `flac_validate` | `t->max_rice_value != 0 && t->max_rice_value > 30` | `-1` | [x] |
| 10 | `flac_validate` | `t->max_partition_order > 15` | `-1` | [x] |
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` after `max_partition_order <= 15` | `-1` | [x] |

No C function accepts a length or a C enum parameter. The generic FFI boundary
cases are handled separately in the tests: a null `t` pointer is exercised in
isolated subprocesses, and out-of-range `channel_mode` byte values are accepted
inputs covered by `CONFIGS.md`.
