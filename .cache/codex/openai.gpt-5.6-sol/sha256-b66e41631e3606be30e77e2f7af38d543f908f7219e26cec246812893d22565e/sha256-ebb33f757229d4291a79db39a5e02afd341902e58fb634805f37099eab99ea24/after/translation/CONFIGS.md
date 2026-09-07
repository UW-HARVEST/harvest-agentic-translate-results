# Configuration surface

The C library has no compile-time or Cargo feature switches. Its public
dynamic API consists of `stbds_hash_bytes` and `siphash`.

For `stbds_hash_bytes`, control flow is determined by the number of complete
`sizeof(size_t)` blocks and by each of the eight tail lengths selected by the
fall-through `switch`. On this build target `sizeof(size_t) == 8`. Each row is
tested over many randomized byte sequences and seeds; the "many" rows also
randomize the complete-block count.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `stbds_hash_bytes` | 0 complete blocks + tail 0 (length 0) | [x] |
| 2 | `stbds_hash_bytes` | 0 complete blocks + tail 1 (length 1) | [x] |
| 3 | `stbds_hash_bytes` | 0 complete blocks + tail 2 (length 2) | [x] |
| 4 | `stbds_hash_bytes` | 0 complete blocks + tail 3 (length 3) | [x] |
| 5 | `stbds_hash_bytes` | 0 complete blocks + tail 4 (length 4) | [x] |
| 6 | `stbds_hash_bytes` | 0 complete blocks + tail 5 (length 5) | [x] |
| 7 | `stbds_hash_bytes` | 0 complete blocks + tail 6 (length 6) | [x] |
| 8 | `stbds_hash_bytes` | 0 complete blocks + tail 7 (length 7) | [x] |
| 9 | `stbds_hash_bytes` | 1 complete block + tail 0 (length 8) | [x] |
| 10 | `stbds_hash_bytes` | 1 complete block + tail 1 (length 9) | [x] |
| 11 | `stbds_hash_bytes` | 1 complete block + tail 2 (length 10) | [x] |
| 12 | `stbds_hash_bytes` | 1 complete block + tail 3 (length 11) | [x] |
| 13 | `stbds_hash_bytes` | 1 complete block + tail 4 (length 12) | [x] |
| 14 | `stbds_hash_bytes` | 1 complete block + tail 5 (length 13) | [x] |
| 15 | `stbds_hash_bytes` | 1 complete block + tail 6 (length 14) | [x] |
| 16 | `stbds_hash_bytes` | 1 complete block + tail 7 (length 15) | [x] |
| 17 | `stbds_hash_bytes` | 2 or more complete blocks + tail 0 | [x] |
| 18 | `stbds_hash_bytes` | 2 or more complete blocks + tail 1 | [x] |
| 19 | `stbds_hash_bytes` | 2 or more complete blocks + tail 2 | [x] |
| 20 | `stbds_hash_bytes` | 2 or more complete blocks + tail 3 | [x] |
| 21 | `stbds_hash_bytes` | 2 or more complete blocks + tail 4 | [x] |
| 22 | `stbds_hash_bytes` | 2 or more complete blocks + tail 5 | [x] |
| 23 | `stbds_hash_bytes` | 2 or more complete blocks + tail 6 | [x] |
| 24 | `stbds_hash_bytes` | 2 or more complete blocks + tail 7 | [x] |
| 25 | `siphash` | nonnegative `init`; generated 64-byte sequence does not wrap at byte 255 | [x] |
| 26 | `siphash` | nonnegative `init`; generated 64-byte sequence wraps at byte 255 | [x] |
| 27 | `siphash` | negative `init`; signed-to-byte conversion in generated sequence | [x] |

The hash `seed` does not select a control-flow branch, but every hash row
randomizes it, including `0`, `SIZE_MAX`, and high-bit values. Randomized input
bytes likewise include all byte values, covering the C integer-promotion
behavior in word and tail assembly.
