# Error Surface

The C API has no error enum and performs no allocation-failure recovery. Inputs
outside the pointer/size contracts have C undefined behavior and therefore no C
result to reproduce. The rows below are every defined rejection/sentinel branch
reachable through an exported function. Internal `STBDS_ASSERT` sites check
implementation invariants after allocation/probing; none is an input-rejection
contract.

| # | function | trigger (the exact invalid input/condition) | expected C result | Test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `stbds_hmfree_func` | `a == NULL` | Return normally without changing state | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | Allocate the zero default entry, return the hash-view pointer, and write `-1` to `temp` | [x] |
| 3 | `stbds_hmget_key_ts` | Map has no hash table | Return the same map pointer and write `-1` to `temp` | [x] |
| 4 | `stbds_hmget_key_ts` | Key is absent and probing reaches an empty slot | Return the same map pointer and write `-1` to `temp` | [x] |
| 5 | `stbds_hmget_key` | Key is absent (including a map with no table) | Return the map pointer and store `-1` in the array header `temp` field | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | Return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | Map has no hash table | Return the same map pointer with header `temp == 0` | [x] |
| 8 | `stbds_hmdel_key` | Key is absent and probing reaches an empty slot | Return the same map pointer with header `temp == 0` | [x] |
| 9 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, and `min_cap == 0` | Return `NULL` without allocating | [x] |

## Mechanically Reviewed Non-Rejections

- `return -1` at C lines 610 and 621 are the two probe-loop paths represented
  by rows 4, 5, and 8 at the exported boundary.
- `STBDS_ASSERT` occurs at lines 401, 777, 828, 833, 847, 850, and 913. These
  assert generated table/capacity/probe/arena invariants, not rejected caller
  values.
- `STBDS_STRING_ARENA_BLOCKSIZE_MIN` is 512 and
  `STBDS_STRING_ARENA_BLOCKSIZE_MAX` is 1,048,576. Their valid boundary
  branches are covered in `CONFIGS.md`.
