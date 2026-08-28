# Configuration Surface

The built C library uses 64-bit `size_t`, so one complete input word is eight
bytes. `stbds_hash_bytes` branches on:

1. whether the complete-word loop executes zero, one, or multiple times; and
2. the exact tail size selected by `switch (len - i)`, from 0 through 7.

The table is that mechanically derived cross-product. Every hash row covers
random byte contents (including bytes below and above `0x80`), random seeds
(including `0` and `SIZE_MAX`), and pointer offsets 0 through 7. Multiple-word
rows also include large valid buffers because C defines no maximum length.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | zero complete words + 0 tail bytes (`len = 0`); null and non-null pointers | [x] |
| 2 | `stbds_hash_bytes` | zero complete words + 1 tail byte (`len = 1`) | [x] |
| 3 | `stbds_hash_bytes` | zero complete words + 2 tail bytes (`len = 2`) | [x] |
| 4 | `stbds_hash_bytes` | zero complete words + 3 tail bytes (`len = 3`) | [x] |
| 5 | `stbds_hash_bytes` | zero complete words + 4 tail bytes (`len = 4`) | [x] |
| 6 | `stbds_hash_bytes` | zero complete words + 5 tail bytes (`len = 5`) | [x] |
| 7 | `stbds_hash_bytes` | zero complete words + 6 tail bytes (`len = 6`) | [x] |
| 8 | `stbds_hash_bytes` | zero complete words + 7 tail bytes (`len = 7`) | [x] |
| 9 | `stbds_hash_bytes` | one complete word + 0 tail bytes (`len = 8`) | [x] |
| 10 | `stbds_hash_bytes` | one complete word + 1 tail byte (`len = 9`) | [x] |
| 11 | `stbds_hash_bytes` | one complete word + 2 tail bytes (`len = 10`) | [x] |
| 12 | `stbds_hash_bytes` | one complete word + 3 tail bytes (`len = 11`) | [x] |
| 13 | `stbds_hash_bytes` | one complete word + 4 tail bytes (`len = 12`) | [x] |
| 14 | `stbds_hash_bytes` | one complete word + 5 tail bytes (`len = 13`) | [x] |
| 15 | `stbds_hash_bytes` | one complete word + 6 tail bytes (`len = 14`) | [x] |
| 16 | `stbds_hash_bytes` | one complete word + 7 tail bytes (`len = 15`) | [x] |
| 17 | `stbds_hash_bytes` | multiple complete words + 0 tail bytes (`len = 8k`, `k >= 2`) | [x] |
| 18 | `stbds_hash_bytes` | multiple complete words + 1 tail byte (`len = 8k + 1`, `k >= 2`) | [x] |
| 19 | `stbds_hash_bytes` | multiple complete words + 2 tail bytes (`len = 8k + 2`, `k >= 2`) | [x] |
| 20 | `stbds_hash_bytes` | multiple complete words + 3 tail bytes (`len = 8k + 3`, `k >= 2`) | [x] |
| 21 | `stbds_hash_bytes` | multiple complete words + 4 tail bytes (`len = 8k + 4`, `k >= 2`) | [x] |
| 22 | `stbds_hash_bytes` | multiple complete words + 5 tail bytes (`len = 8k + 5`, `k >= 2`) | [x] |
| 23 | `stbds_hash_bytes` | multiple complete words + 6 tail bytes (`len = 8k + 6`, `k >= 2`) | [x] |
| 24 | `stbds_hash_bytes` | multiple complete words + 7 tail bytes (`len = 8k + 7`, `k >= 2`) | [x] |
| 25 | `siphash` | full operation: generate 64 wrapping bytes from any `int init` that avoids signed-overflow UB, hash lengths 0 through 63, and print all 64 rows | [x] |

## Other axes

| Axis | C treatment |
|------|-------------|
| Byte order | Input words and output bytes are explicitly assembled little-endian. |
| Element type | Input is always treated as `unsigned char *`. |
| Seed | `size_t`; no mode branch or restricted value. |
| `siphash` initialization | `int`; no mode branch or restricted value, except that signed overflow is undefined C behavior. |
| Runtime options, modes, flags | None. |
| Compile-time feature branches | None. |
| Rust Cargo features | None declared. |
