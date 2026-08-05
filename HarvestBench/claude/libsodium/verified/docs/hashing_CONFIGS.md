# HASHING family — configuration / input-shape coverage

Each row is a meaningful configuration exercised with many randomized inputs
(fixed-seed `Rng`) over lengths including 0, 1, block boundaries (55/56/63/64,
111/112/127/128, 135/136/137, 167/168/169), and large (up to 10000). All rows
assert BOTH return code AND output buffers match C byte-for-byte. `[x]` = test
passing.

| # | entry point(s) | configuration (options + shape) | done |
|---|----------------|---------------------------------|------|
| 1 | crypto_hash | one-shot; == sha512; all lengths | [x] |
| 2 | crypto_hash_sha512 | one-shot; all lengths | [x] |
| 3 | crypto_hash_sha512_init/update/final | streaming, random chunk splits; == one-shot | [x] |
| 4 | crypto_hash_sha256 | one-shot; all lengths | [x] |
| 5 | crypto_hash_sha256_init/update/final | streaming, random chunk splits; == one-shot | [x] |
| 6 | crypto_hash_sha3256 | one-shot; all lengths (rate 136 boundaries) | [x] |
| 7 | crypto_hash_sha3256_init/update/final | streaming, random chunk splits; == one-shot | [x] |
| 8 | crypto_hash_sha3512 | one-shot; all lengths (rate 72 boundaries) | [x] |
| 9 | crypto_hash_sha3512_init/update/final | streaming, random chunk splits; == one-shot | [x] |
| 10 | crypto_generichash / crypto_generichash_blake2b | one-shot; outlen {16,17,20,24,31,32,48,63,64} x keylen {0,1,16,24,32,63,64} x inlen {0,1,63,64,128,129,1000}; also gh==b2b | [x] |
| 11 | crypto_generichash_init/update/final | streaming; outlen {16,32,48,64} x keylen {0,16,32,64} x inlen {0,1,64,200,1000}; random chunks; == one-shot; 64-byte-aligned state | [x] |
| 12 | crypto_generichash_blake2b_salt_personal | one-shot; outlen {16,32,64} x keylen {0,16,32,64} x inlen {0,1,128,500}; random salt+personal; NULL salt/personal == plain blake2b | [x] |
| 13 | crypto_generichash_blake2b_init_salt_personal + update/final | streaming; outlen {16,32,64} x keylen {0,32,64} x inlen {0,100,1000}; == salt_personal one-shot | [x] |
| 14 | crypto_shorthash / crypto_shorthash_siphash24 | 8-byte output; random 16-byte key; all lengths x4 | [x] |
| 15 | crypto_shorthash_siphashx24 | 16-byte output; random 16-byte key; all lengths x4 | [x] |
| 16 | crypto_xof_shake128 | one-shot; outlen {0,1,16,32,167,168,169,336,500,1000} x inlen {0,1,100,168,169,1000} | [x] |
| 17 | crypto_xof_shake256 | one-shot; outlen boundaries at rate 136 x inlen | [x] |
| 18 | crypto_xof_turboshake128 | one-shot; outlen boundaries at rate 168 x inlen | [x] |
| 19 | crypto_xof_turboshake256 | one-shot; outlen boundaries at rate 136 x inlen | [x] |
| 20 | crypto_xof_*_init/update/squeeze | streaming; random update chunks + random incremental squeeze chunks; inlen {0,1,50,rate,rate+5,1000} x total_out {0,1,32,rate+7,500}; == one-shot; for all 4 XOFs | [x] |
| 21 | crypto_xof_*_init_with_domain | custom domains {0x01,0x06,0x07,0x1f,0x80,0xff,standard}; update+squeeze 100 bytes; for all 4 XOFs; domain_standard() parity | [x] |
| 22 | crypto_generichash (error) | outlen {0,65,100,1000} -> -1 | [x] |
| 23 | crypto_generichash (error) | keylen {65,100,200} -> -1 | [x] |
| 24 | crypto_generichash_init (error) | (outlen,keylen) in {(0,0),(65,0),(32,65),(0,65)} -> -1 | [x] |
| 25 | crypto_generichash_blake2b_salt_personal (error) | outlen {0,65}, keylen 65 -> -1 | [x] |
| 26 | crypto_hash_sha3256/512_final (error) | double final -> -1, state recovers | [x] |
| 27 | crypto_hash_sha3256/512_update (error) | update after final -> -1, state recovers | [x] |
| 28 | crypto_xof_*_update (error) | update after squeeze -> -1, continued squeeze still matches | [x] |
