# Error Surface

Mechanically derived from every rejecting branch in `c_src/src/lib.c`.
`flac_validate` dereferences its argument without a null check, so null is an
invalid FFI boundary case but is not a C rejection row.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
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
| 11 | `flac_validate` | `t->min_partition_order > t->max_partition_order` | `-1` | [x] |

No `assert`, error enum, `RETURN_ERROR`, `return NULL`, or other rejecting
construct occurs in the C source.
