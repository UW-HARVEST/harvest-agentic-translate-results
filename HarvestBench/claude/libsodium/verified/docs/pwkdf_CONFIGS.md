# PWHASH + KDF configurations tested

Each row is a meaningful configuration exercised in `tests/pwkdf.rs` with many
randomized inputs (fixed seeds). `[x]` marks configs whose test passes
(all pass). Password-hashing params are kept MINIMAL for speed
(argon2 memlimit=8192, argon2i opslimit=3/4, argon2id opslimit=1/2/3;
scrypt _ll uses tiny N/r/p; scrypt raw/str use header MIN 32768/16MiB).

| # | entry point(s) | configuration (options+shape) | done |
|---|----------------|-------------------------------|------|
| 1 | `crypto_pwhash_argon2i` | ALG_ARGON2I13, outlen {16,24,32,40}, opslimit {3,4}, mem MIN, random pw(1..25)/salt(16) | [x] |
| 2 | `crypto_pwhash_argon2id` | ALG_ARGON2ID13, outlen {16,24,32,40}, opslimit {1,2,3}, mem MIN, random pw/salt | [x] |
| 3 | `crypto_pwhash` (generic) | dispatch to argon2i13 | [x] |
| 4 | `crypto_pwhash` (generic) | dispatch to argon2id13 | [x] |
| 5 | `crypto_pwhash_str_alg` + `crypto_pwhash_str_verify` | argon2i string hash, self + cross verify, wrong-pw reject | [x] |
| 6 | `crypto_pwhash_str_alg` + `crypto_pwhash_str_verify` | argon2id string hash, self + cross verify, wrong-pw reject | [x] |
| 7 | `crypto_pwhash_str` (default) + verify | default alg (argon2id), cross-verify C<->Rust | [x] |
| 8 | `crypto_pwhash_scryptsalsa208sha256_ll` | small (N,r,p) in {(16,1,1),(16,4,1),(32,2,2),(8,8,1),(64,1,3)}, buflen 16..64, random pw/salt | [x] |
| 9 | `crypto_pwhash_scryptsalsa208sha256` (raw) | opslimit/memlimit MIN, outlen {16,32,48}, salt 32, random pw | [x] |
| 10 | `crypto_pwhash_scryptsalsa208sha256_str` + `_str_verify` | MIN params, self + cross verify, wrong-pw reject | [x] |
| 11 | `crypto_kdf_derive_from_key` (generic) | subkey_len {16,17,24,32,48,63,64} x subkey_id {0,1,2,42,2^32-1,2^64-1} | [x] |
| 12 | `crypto_kdf_blake2b_derive_from_key` | same matrix as #11, blake2b-specific entry | [x] |
| 13 | `crypto_kdf_keygen` | fills 32-byte key (non-zero) | [x] |
| 14 | `crypto_kdf_hkdf_sha256_extract` + `_expand` | random salt(0..40)/ikm(1..49)/ctx(0..16), outlen {0,1,32,37,103} | [x] |
| 15 | `crypto_kdf_hkdf_sha512_extract` + `_expand` | random salt/ikm/ctx, outlen {0,1,64,69,199} | [x] |
| 16 | `crypto_kdf_hkdf_sha256_extract_init/_update/_final` | 3-chunk streaming, equals one-shot; C<->Rust match; statebytes match | [x] |
| 17 | `crypto_kdf_hkdf_sha512_extract_init/_update/_final` | 3-chunk streaming, equals one-shot; C<->Rust match; statebytes match | [x] |

## Error-path configs (see pwkdf_ERRORS.md for the full table)

| # | entry point(s) | configuration | done |
|---|----------------|---------------|------|
| E1 | `crypto_pwhash` | opslimit/memlimit below min, outlen below min, bad alg id | [x] |
| E2 | `crypto_pwhash_str_verify` | wrong pw, no-prefix, corrupt-body, empty string | [x] |
| E3 | `crypto_pwhash_scryptsalsa208sha256_ll` | N not pow2, N<2, r==0, p==0 | [x] |
| E4 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | wrong pw, invalid hash string | [x] |
| E5 | `crypto_kdf_derive_from_key` | subkey_len below/above bounds and 0 | [x] |
| E6 | `crypto_kdf_hkdf_{sha256,sha512}_expand` | out_len > BYTES_MAX | [x] |
