# sodiumutils family — tested configurations

Family: SODIUM UTILS + CODECS + RUNTIME. Test file: `tests/sodiumutils.rs`.
Every row is checked off once its differential test (C vs Rust) passes.

| # | entry point(s) | configuration (options + shape) | [x] |
|---|----------------|---------------------------------|-----|
| 1 | sodium_memcmp | random equal buffers, len 0..64 | [x] |
| 2 | sodium_memcmp | random differing buffers (flipped byte) | [x] |
| 3 | sodium_memcmp | len == 0 (always equal) | [x] |
| 4 | sodium_compare | random buffers, len 0..40, LE ordering | [x] |
| 5 | sodium_compare | forced-equal / off-by-one-limb cases | [x] |
| 6 | sodium_compare | all-0xff vs all-0x00 across lens {1,2,8,12,16,24,32} | [x] |
| 7 | sodium_is_zero | all-zero and mostly-zero buffers, len 0..48 | [x] |
| 8 | sodium_is_zero | empty buffer -> 1 | [x] |
| 9 | sodium_increment | random multi-precision, len 0..40 | [x] |
| 10 | sodium_increment | carry-chain edges (0x00/0xff) at lens {0,1,8,12,16,24,32} | [x] |
| 11 | sodium_add | random a+b, len 0..40 | [x] |
| 12 | sodium_add | all-0xff carry edges at lens {0,1,8,12,24,32,64} | [x] |
| 13 | sodium_sub | random a-b, len 0..80 | [x] |
| 14 | sodium_sub | borrow edges (0x00 - 0x01) at lens {0,1,8,12,24,32,64} | [x] |
| 15 | sodium_memzero | random buffers len 0..128 | [x] |
| 16 | sodium_memzero | len == 0 no-op | [x] |
| 17 | sodium_stackzero | lens {0,16,64,256} (no-op in this build config) | [x] |
| 18 | sodium_bin2hex | random bin len 0..64, exact-min hex_maxlen, return-ptr check | [x] |
| 19 | sodium_hex2bin | valid pure-hex, all combos of bin_len/hex_end present/absent | [x] |
| 20 | sodium_hex2bin | ignore chars (`:` / space) interspersed | [x] |
| 21 | sodium_hex2bin | uppercase / mixed case input | [x] |
| 22 | sodium_hex2bin | trailing non-hex char, with & without hex_end | [x] |
| 23 | sodium_hex2bin | odd nibble count (dangling) -> EINVAL | [x] |
| 24 | sodium_hex2bin | bin_maxlen too small {0,1,2,3} -> ERANGE | [x] |
| 25 | sodium_hex2bin | hex_len shorter than string (0..len) | [x] |
| 26 | sodium_base64_encoded_len | bin_len 0..300 + large {1000,4096,65535,1e6}, all 4 variants | [x] |
| 27 | sodium_bin2base64 | random bin len 0..64, all 4 variants, encoded_len-sized buffer | [x] |
| 28 | sodium_base642bin | encode(C)->decode(both) round-trip, all 4 variants, bin_len/b64_end combos | [x] |
| 29 | sodium_base642bin | whitespace ignore set injected, all 4 variants | [x] |
| 30 | sodium_base642bin | invalid/edge inputs (bad chars, padding), all variants, ptr-param combos | [x] |
| 31 | sodium_base642bin | bin_maxlen too small {0,1,2,3} -> ERANGE, all variants | [x] |
| 32 | sodium_base642bin | b64_len shorter than string (0..len), all variants | [x] |
| 33 | sodium_pad | random blocksize 1..32 (pow2 & non-pow2), unpadded 0..128, generous max | [x] |
| 34 | sodium_pad | max_buflen too small -> -1 | [x] |
| 35 | sodium_pad | blocksize == 0 -> -1 (null padded_buflen_p) | [x] |
| 36 | sodium_pad | null padded_buflen_p on success path | [x] |
| 37 | sodium_unpad | pad-then-unpad round-trip recovers length, blocksize 1..32 | [x] |
| 38 | sodium_unpad | random buffers direct unpad (mostly invalid padding) | [x] |
| 39 | sodium_unpad | padded_buflen < blocksize -> -1 | [x] |
| 40 | sodium_unpad | blocksize == 0 -> -1 | [x] |
| 41 | sodium_unpad | explicit valid full-block padding at bs {8,16,17} | [x] |
| 42 | sodium_runtime_has_* | all 12 features (sse2/3/ssse3/sse41/avx/avx2/avx512f/neon/aesni/pclmul/rdrand/armcrypto) | [x] |
| 43 | sodium_version_string / _version_major / _version_minor / _library_minimal | value comparison C vs Rust | [x] |

Build-config note: the C `.so` is compiled with NO `HAVE_*` feature macros
(equivalent to `--disable-asm`), so `_sodium_runtime_get_cpu_features` takes the
portable fallback and every `sodium_runtime_has_*` returns `0`. The Rust
translation hardcodes the same zeros; they match on this x86_64 host.
