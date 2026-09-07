# Configuration surface

The table is derived from the three C dynamic symbols, the public struct
fields, the `bits / 8` truncation, the `pos % 64` addressing, the
`m->pos >= 64` wrap branch, its carry loop, the five fixed `update_md5`
iterations, sparse sample stride, and unsigned arithmetic boundaries.

`update_md5` rows use randomized 136-element sample arrays. That is the exact
input shape needed by the C access pattern: offsets `0..7`, `32..39`,
`64..71`, `96..103`, and `128..135`. Randomizing all elements verifies both
the consumed low bytes and that ignored/high bytes do not affect output.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tflac_pack_u64le` | writable 8-byte destination; randomized `u64`, including zero and extrema | [x] |
| 2 | `tflac_md5_addsample` | `pos < 64`, `bits = 0`: zero-byte advance, full 8-byte pack still occurs | [x] |
| 3 | `tflac_md5_addsample` | `pos < 64`, `bits = 1..7`: sub-byte count truncates to zero bytes | [x] |
| 4 | `tflac_md5_addsample` | byte-aligned `bits`, resulting `pos < 64`: no wrap branch | [x] |
| 5 | `tflac_md5_addsample` | non-byte-aligned `bits >= 8`, resulting `pos < 64`: byte count truncates | [x] |
| 6 | `tflac_md5_addsample` | `pos + bits/8 = 64`: exact wrap to zero, carry loop has zero iterations | [x] |
| 7 | `tflac_md5_addsample` | `pos + bits/8 = 65..71`: wrap with carry remainder `1..7` | [x] |
| 8 | `tflac_md5_addsample` | `pos + bits/8 = 72`: wrap with the maximum in-struct carry remainder, 8 | [x] |
| 9 | `tflac_md5_addsample` | initial `pos >= 64`, no `u32` overflow, safe final remainder `0..8`: modulo addressing and normalization | [x] |
| 10 | `tflac_md5_addsample` | initial `pos` near `u32::MAX`, addition wraps below 64: post-add wrap branch is skipped | [x] |
| 11 | `tflac_md5_addsample` | `total` near `u64::MAX`, positive `bits`: unsigned total wraps | [x] |
| 12 | `update_md5` | initial MD5 `pos 0..23` (no buffer wrap); `cur_blocksize * channels > 40` | [x] |
| 13 | `update_md5` | initial MD5 `pos 0..23` (no buffer wrap); product equals 40 | [x] |
| 14 | `update_md5` | initial MD5 `pos 0..23` (no buffer wrap); product below 40 and return subtraction wraps | [x] |
| 15 | `update_md5` | initial MD5 `pos 0..23` (no buffer wrap); multiplication itself wraps `u32` | [x] |
| 16 | `update_md5` | initial MD5 `pos = 24` (exact wrap on fifth add); product greater than 40 | [x] |
| 17 | `update_md5` | initial MD5 `pos = 24` (exact wrap on fifth add); product equals 40 | [x] |
| 18 | `update_md5` | initial MD5 `pos = 24` (exact wrap on fifth add); product below 40 | [x] |
| 19 | `update_md5` | initial MD5 `pos = 24` (exact wrap on fifth add); multiplication wraps | [x] |
| 20 | `update_md5` | initial MD5 `pos 25..31` (partial wrap on fifth add); product greater than 40 | [x] |
| 21 | `update_md5` | initial MD5 `pos 25..31` (partial wrap on fifth add); product equals 40 | [x] |
| 22 | `update_md5` | initial MD5 `pos 25..31` (partial wrap on fifth add); product below 40 | [x] |
| 23 | `update_md5` | initial MD5 `pos 25..31` (partial wrap on fifth add); multiplication wraps | [x] |
| 24 | `update_md5` | initial MD5 `pos` in `32,40,48,56` (exact wrap before the fifth add); product greater than 40 | [x] |
| 25 | `update_md5` | same pre-fifth exact-wrap positions; product equals 40 | [x] |
| 26 | `update_md5` | same pre-fifth exact-wrap positions; product below 40 | [x] |
| 27 | `update_md5` | same pre-fifth exact-wrap positions; multiplication wraps | [x] |
| 28 | `update_md5` | initial MD5 `pos 33..63`, excluding `40,48,56` (partial wrap before the fifth add); product greater than 40 | [x] |
| 29 | `update_md5` | same pre-fifth partial-wrap positions; product equals 40 | [x] |
| 30 | `update_md5` | same pre-fifth partial-wrap positions; product below 40 | [x] |
| 31 | `update_md5` | same pre-fifth partial-wrap positions; multiplication wraps | [x] |
| 32 | `update_md5` | initial MD5 `pos >= 64`, no `u32` overflow, first add normalizes it; product greater than 40 | [x] |
| 33 | `update_md5` | same initially non-normalized position; product equals 40 | [x] |
| 34 | `update_md5` | same initially non-normalized position; product below 40 | [x] |
| 35 | `update_md5` | same initially non-normalized position; multiplication wraps | [x] |
| 36 | `update_md5` | initial MD5 `pos` near `u32::MAX`, first add wraps below 64; product greater than 40 | [x] |
| 37 | `update_md5` | same near-`u32::MAX` position; product equals 40 | [x] |
| 38 | `update_md5` | same near-`u32::MAX` position; product below 40 | [x] |
| 39 | `update_md5` | same near-`u32::MAX` position; multiplication wraps | [x] |

There are no compile-time or Cargo feature branches. The feature combination
set therefore contains one member: the empty/default feature set.
