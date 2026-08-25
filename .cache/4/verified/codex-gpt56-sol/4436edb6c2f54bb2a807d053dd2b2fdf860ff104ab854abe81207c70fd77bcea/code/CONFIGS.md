# Configuration Surface

Mechanical source analysis found no preprocessor configurations, runtime
options, modes, flags, formats, element types, or byte-order choices. The hash
control flow branches on the number of complete `sizeof(size_t)` blocks and on
each tail length from 0 through 7 bytes. This table assumes the built platform,
where `sizeof(size_t) == 8`.

Every hash row uses randomized bytes, seeds, and representative lengths within
the stated shape. `siphash` is tested across randomized signed `int`
initializers and internally exercises lengths 0 through 63.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | empty input: 0 complete blocks, 0-byte tail (`len == 0`) | [x] |
| 2 | `stbds_hash_bytes` | short input: 0 complete blocks, 1-byte tail | [x] |
| 3 | `stbds_hash_bytes` | short input: 0 complete blocks, 2-byte tail | [x] |
| 4 | `stbds_hash_bytes` | short input: 0 complete blocks, 3-byte tail | [x] |
| 5 | `stbds_hash_bytes` | short input: 0 complete blocks, 4-byte tail | [x] |
| 6 | `stbds_hash_bytes` | short input: 0 complete blocks, 5-byte tail | [x] |
| 7 | `stbds_hash_bytes` | short input: 0 complete blocks, 6-byte tail | [x] |
| 8 | `stbds_hash_bytes` | short input: 0 complete blocks, 7-byte tail | [x] |
| 9 | `stbds_hash_bytes` | one-block input: 1 complete block, 0-byte tail (`len == 8`) | [x] |
| 10 | `stbds_hash_bytes` | one-block input: 1 complete block, 1-byte tail | [x] |
| 11 | `stbds_hash_bytes` | one-block input: 1 complete block, 2-byte tail | [x] |
| 12 | `stbds_hash_bytes` | one-block input: 1 complete block, 3-byte tail | [x] |
| 13 | `stbds_hash_bytes` | one-block input: 1 complete block, 4-byte tail | [x] |
| 14 | `stbds_hash_bytes` | one-block input: 1 complete block, 5-byte tail | [x] |
| 15 | `stbds_hash_bytes` | one-block input: 1 complete block, 6-byte tail | [x] |
| 16 | `stbds_hash_bytes` | one-block input: 1 complete block, 7-byte tail | [x] |
| 17 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 0-byte tail | [x] |
| 18 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 1-byte tail | [x] |
| 19 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 2-byte tail | [x] |
| 20 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 3-byte tail | [x] |
| 21 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 4-byte tail | [x] |
| 22 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 5-byte tail | [x] |
| 23 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 6-byte tail | [x] |
| 24 | `stbds_hash_bytes` | multi-block input: at least 2 complete blocks, 7-byte tail | [x] |
| 25 | `siphash` | signed `int init`; emits hashes for lengths 0 through 63 | [x] |
