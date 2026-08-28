# Configuration Surface

Mechanical scan scope: every globally defined symbol in the C shared object,
the public header, and every branch and arithmetic boundary in
`../c_src/src/lib.c`.

There are no compile-time or Cargo feature switches. The runtime axes are:

- `tflac_pack_u64le`: the full `uint64_t` value range.
- `tflac_md5_addsample`: `bits / 8`, byte truncation, `pos + bytes`,
  the `pos >= 64` branch, zero/one/many spill-copy iterations, and unsigned
  wrapping of `pos` and `total`.
- `update_md5`: fixed five-iteration sample access at indices `0..7`,
  `32..39`, `64..71`, `96..103`, and `128..135`; the same MD5 position
  branches reached by five 64-bit additions; and normal, subtract-underflow,
  or multiply-overflow return arithmetic.

Rows C10-C21 are the cross-product of the four MD5 position paths and three
return-arithmetic paths that `update_md5` distinguishes. Each row includes
random full-range signed samples, whose upper 24 bits are intentionally
discarded by C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `tflac_pack_u64le` | writable 8-byte destination; randomized `uint64_t`, including `0` and `UINT64_MAX` | [x] |
| C2 | `tflac_md5_addsample` | `pos + bits/8 < 64`; byte-aligned bits; no unsigned overflow | [x] |
| C3 | `tflac_md5_addsample` | `pos + bits/8 < 64`; non-byte-aligned bits truncated by `/ 8` | [x] |
| C4 | `tflac_md5_addsample` | `pos + bits/8 == 64`; wraps to zero; spill-copy loop has zero iterations | [x] |
| C5 | `tflac_md5_addsample` | wraps to one; spill-copy loop has one iteration | [x] |
| C6 | `tflac_md5_addsample` | wraps to `2..8`; spill-copy loop has many iterations within the 8-byte spill area | [x] |
| C7 | `tflac_md5_addsample` | `uint32_t` position addition overflows to a value below 64, so the wrap branch is not taken | [x] |
| C8 | `tflac_md5_addsample` | `uint64_t total + bits` overflows; position remains below 64 | [x] |
| C9 | `tflac_md5_addsample` | initial `pos` is `56..63`; 8-byte write uses the tail/spill boundary and wraps to `0..7` | [x] |
| C10 | `update_md5` | MD5 no-wrap path (`pos 0..23`); product at least 40 with no arithmetic wrap | [x] |
| C11 | `update_md5` | MD5 wrap-to-zero path (`pos` a multiple of 8 in `24..56`); product at least 40 | [x] |
| C12 | `update_md5` | MD5 wrap-to-one path (`pos` congruent to 1 modulo 8 in `25..57`); product at least 40 | [x] |
| C13 | `update_md5` | MD5 wrap-to-many path (`pos` produces spill copy count `2..8`); product at least 40 | [x] |
| C14 | `update_md5` | MD5 no-wrap path; initial product below 40 and five subtractions underflow | [x] |
| C15 | `update_md5` | MD5 wrap-to-zero path; initial product below 40 | [x] |
| C16 | `update_md5` | MD5 wrap-to-one path; initial product below 40 | [x] |
| C17 | `update_md5` | MD5 wrap-to-many path; initial product below 40 | [x] |
| C18 | `update_md5` | MD5 no-wrap path; `cur_blocksize * channels` overflows `uint32_t` | [x] |
| C19 | `update_md5` | MD5 wrap-to-zero path; multiplication overflows | [x] |
| C20 | `update_md5` | MD5 wrap-to-one path; multiplication overflows | [x] |
| C21 | `update_md5` | MD5 wrap-to-many path; multiplication overflows | [x] |
| C22 | `update_md5` | `total` overflows during one of the five 64-bit additions | [x] |
| C23 | `tflac_md5_addsample` | `bits == UINT32_MAX`; `/ 8` truncates and the large byte count wraps modulo 64 | [x] |
