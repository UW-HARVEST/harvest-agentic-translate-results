# CONFIGS.md — Configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for the **valid** input space. Rows are the combinations of
option/mode/shape axes that the C source actually *branches on* — derived from
the public headers (`c_src/libsodium/include/sodium/*.h`, 749 exported
functions) plus the `if`/`switch`/loop-boundary branches in the C bodies.

Every row is exercised through BOTH `.so`s with **many randomized inputs**
(seeded xoshiro PRNG, fixed seed per test → reproducible) and asserted
byte-identical, not with a single hand-picked value.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|---|---|---|
| **message length** | `0`, `1`, block−1, block, block+1, multi-block, non-multiple tail | every stream/hash/AEAD loop |
| **block boundary** | 16 (poly1305/aes), 32 (aegis128l rate), 64 (salsa/chacha/sha256/blake2b), 128 (sha512/blake2b buf), 136/168 (keccak rate) | compress/`_xor` loops |
| **AD length** | `0`, `1`, 15, 16, 17, 63, 64, 65 | AEAD `_update`/pad16 branches |
| **key length** | `0` (unkeyed), 1, 32, 64, `> blocklen` (HMAC re-hash path) | `blake2b_init_key`, `hmac*_init` |
| **`outlen`** | MIN, MIN+1, mid, MAX−1, MAX | blake2b, hkdf, kdf, xof |
| **multipart split** | one-shot vs `init`/`update`×N/`final`, splits at/inside/across block boundaries | all `_update` state machines |
| **counter / `ic`** | `0`, `1`, `2^32−1`, `2^32`, `2^64−1` (64-bit ic); 32-bit ic wrap for ietf | `*_xor_ic` |
| **base64 variant** | `ORIGINAL(1)`, `ORIGINAL_NO_PADDING(3)`, `URLSAFE(5)`, `URLSAFE_NO_PADDING(7)` | `sodium_base64_check_variant` |
| **`ignore` set** | `NULL`, `""`, `" \n"`, `":"` | `sodium_hex2bin` / `base642bin` |
| **pwhash alg** | `ARGON2I13(1)`, `ARGON2ID13(2)` | `crypto_pwhash` switch |
| **h2c hash_alg** | `SHA256(1)`, `SHA512(2)` | `core_h2c_string_to_hash` switch |
| **secretstream tag** | `MESSAGE(0)`, `PUSH(1)`, `REKEY(2)`, `FINAL(3)` | `_push` tag handling + rekey branch |
| **scalar clamping** | clamped vs `_noclamp` variants | ed25519/ristretto scalarmult |
| **detached vs combined** | `_detached` (separate mac) vs combined (`_easy`, one-shot) | box/secretbox/aead/sign |
| **precomputed vs full** | `_beforenm`/`_afternm` vs one-shot | box, aes256gcm |
| **turboshake domain** | `0x01..0x7f` domain separation byte | `_init_with_domain` |
| **ip family** | IPv4, IPv6, IPv4-mapped, `%zone`, `::` compression | `sodium_ip2bin`/`bin2ip` |
| **padding blocksize** | 1, 2, 15, 16, 17, 64, `unpadded_buflen` itself | `sodium_pad`/`unpad` |
| **alignment / aliasing** | `c == m` (in-place) vs disjoint buffers | all `_xor` and box/secretbox |

---

## Rows

### A. Low-level primitives (called directly, not via wrappers)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `crypto_core_salsa20`, `_salsa2012`, `_salsa208` | random `in`/`k`/`c`, and `c == NULL` (sigma default) | [x] |
| 2 | `crypto_core_hsalsa20` | random `in`/`k`, `c == NULL` and `c` set | [x] |
| 3 | `crypto_core_hchacha20` | random `in`/`k`, `c == NULL` and `c` set | [x] |
| 4 | `crypto_core_keccak1600_init` + `_absorb`/`_squeeze`/`_permute` | rate 168 (shake128) & 136 (shake256), 0/partial/exact/multi-rate absorb | [x] |
| 5 | `crypto_verify_16/32/64` | equal, differ at each byte index, differ in every bit | [x] |
| 6 | `crypto_onetimeauth_poly1305` | `inlen` ∈ {0,1,15,16,17,31,32,33,64,1000} | [x] |
| 7 | `crypto_onetimeauth_poly1305_init/update/final` | multipart splits at 0/1/15/16/17/32 offsets, 1–8 updates | [x] |
| 8 | `crypto_onetimeauth_poly1305_verify` | correct mac, and mac with each byte flipped | [x] |
| 9 | `crypto_shorthash_siphash24` | `inlen` ∈ {0..24, 63,64,65,255} | [x] |
| 10 | `crypto_shorthash_siphashx24` | same shapes, 128-bit output | [x] |
| 11 | `crypto_scalarmult_curve25519` | random 32-byte scalar × random valid `p`; scalar with/without clamped bits | [x] |
| 12 | `crypto_scalarmult_curve25519_base` | random scalars incl. all-zero-except-clamp, all-0xff | [x] |
| 13 | `crypto_stream_salsa20_xor_ic` / `_salsa2012` / `_salsa208` (`salsa20` only has ic) | `ic` ∈ {0,1,2,2^32,2^64−1}, `mlen` at 0/63/64/65/128/1000 | [x] |
| 14 | `crypto_stream_chacha20_xor_ic` | 8-byte nonce, `ic` ∈ {0,1,2^32−1,2^32}, `mlen` boundaries | [x] |
| 15 | `crypto_stream_chacha20_ietf_xor_ic` | 12-byte nonce, `ic` ∈ {0,1,100,2^32−1−ceil(mlen/64)}, `mlen` boundaries | [x] |
| 16 | `crypto_stream_chacha20_ietf_ext`, `crypto_stream_chacha20_ietf_ext_xor_ic` | extended 16-byte-nonce variant, same `ic` sweep. (`..._ext_xor` is not exported.) | [x] |
| 17 | `crypto_stream_xsalsa20_xor_ic`, `crypto_stream_xchacha20_xor_ic` | 24-byte nonce, `ic` sweep | [x] |
| 18 | all `crypto_stream_*` keystream form (`_xor` with `m == NULL` equivalent: `crypto_stream_*`) | `clen` boundaries 0/1/63/64/65/1000 | [x] |
| 19 | `crypto_stream_*_xor` in-place (`c == m`) | vs disjoint; identical results required | [x] |
| 20 | `crypto_core_softaes` internals via `crypto_ipcrypt_*` | see row 62 | [x] |

### B. Hashes and XOFs (one-shot × multipart × length sweep)

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 21 | `crypto_hash_sha256` | `inlen` ∈ {0,1,55,56,57,63,64,65,119,120,127,128,1000} | [x] |
| 22 | `crypto_hash_sha256_init/update/final` | random splits (1–10 updates), incl. 0-length updates | [x] |
| 23 | `crypto_hash_sha512` / `crypto_hash` | `inlen` ∈ {0,1,111,112,113,127,128,129,255,256,1000} | [x] |
| 24 | `crypto_hash_sha512_init/update/final` | random splits, 0-length updates | [x] |
| 25 | `crypto_hash_sha3256` / `_sha3512` one-shot | `inlen` ∈ {0,1,rate−1,rate,rate+1,2·rate,1000} for rate 136/72 | [x] |
| 26 | `crypto_hash_sha3256_init/update/final`, `_sha3512_*` | random splits across rate boundary | [x] |
| 27 | `crypto_xof_shake128` / `_shake256` one-shot | `outlen` ∈ {0,1,31,32,33,168,169,336,1000}; `inlen` sweep | [x] |
| 28 | `crypto_xof_shake128_init/absorb/squeeze` | incremental absorb splits × incremental squeeze splits (both cross the rate) | [x] |
| 29 | `crypto_xof_turboshake128/256` one-shot | default domain, `outlen`/`inlen` sweep | [x] |
| 30 | `crypto_xof_turboshake128_init_with_domain` | domain ∈ {0x01,0x02,0x1f,0x7f}, absorb/squeeze splits | [x] |
| 31 | `crypto_xof_turboshake256_init_with_domain` | domain sweep as above | [x] |
| 32 | `crypto_generichash_blake2b` | `outlen` ∈ {1,16,31,32,33,63,64} × `keylen` ∈ {0,1,16,32,63,64} × `inlen` ∈ {0,1,127,128,129,1000} | [x] |
| 33 | `crypto_generichash_blake2b_salt_personal` | above × salt/personal ∈ {all-zero, random, `NULL`} | [x] |
| 34 | `crypto_generichash_blake2b_init/update/final` | keyed & unkeyed × outlen sweep × random update splits crossing 128 | [x] |
| 35 | `crypto_generichash_blake2b_init_salt_personal` + multipart | keyed/unkeyed × salt/personal set/`NULL` × splits | [x] |
| 36 | `crypto_generichash` / `_init` / `_update` / `_final` (generic) | same as 32/34 through the generic wrapper | [x] |
| 37 | `crypto_auth_hmacsha256` + `_init/_update/_final` | `keylen` ∈ {0,1,32,63,64,65,128,200} (crosses the >64 re-hash branch) × `inlen` sweep × splits | [x] |
| 38 | `crypto_auth_hmacsha512` + multipart | `keylen` ∈ {0,1,64,127,128,129,256} (crosses >128 re-hash) × splits | [x] |
| 39 | `crypto_auth_hmacsha512256` + multipart | same keylen sweep, truncated 32-byte output | [x] |
| 40 | `crypto_auth` / `crypto_auth_verify` (generic = hmacsha512256) | fixed 32-byte key, `inlen` sweep | [x] |

### C. KDFs

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 41 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len` ∈ {16,17,32,63,64} × `subkey_id` ∈ {0,1,2^32,2^64−1} × ctx random | [x] |
| 42 | `crypto_kdf_derive_from_key` (generic) | same sweep | [x] |
| 43 | `crypto_kdf_hkdf_sha256_extract` (one-shot) | `salt_len`/`ikm_len` ∈ {0,1,32,64,65,1000} | [x] |
| 44 | `crypto_kdf_hkdf_sha256_extract_init/update/final` | multipart ikm splits, 0-length updates | [x] |
| 45 | `crypto_kdf_hkdf_sha256_expand` | `out_len` ∈ {0,1,31,32,33,64,8160(MAX)} × `ctx_len` ∈ {0,1,64} | [x] |
| 46 | `crypto_kdf_hkdf_sha256_keygen` | (randomness) — length + non-degenerate only | [x] |
| 47 | `crypto_kdf_hkdf_sha512_extract` + multipart | as 43/44 with 64-byte PRK | [x] |
| 48 | `crypto_kdf_hkdf_sha512_expand` | `out_len` ∈ {0,1,63,64,65,16320(MAX)} × ctx sweep | [x] |

### D. AEAD — every variant, detached & combined, precomputed & one-shot

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 49 | `crypto_aead_chacha20poly1305_encrypt`/`_decrypt` | `mlen` ∈ {0,1,15,16,17,63,64,65,1000} × `adlen` ∈ {0,1,15,16,17,64} | [x] |
| 50 | `crypto_aead_chacha20poly1305_encrypt_detached`/`_decrypt_detached` | same sweep; `ad == NULL` when `adlen == 0`; `nsec` always `NULL` | [x] |
| 51 | `crypto_aead_chacha20poly1305_ietf_encrypt`/`_decrypt` | same sweep, 12-byte nonce | [x] |
| 52 | `crypto_aead_chacha20poly1305_ietf_*_detached` | same sweep | [x] |
| 53 | `crypto_aead_xchacha20poly1305_ietf_encrypt`/`_decrypt` | same sweep, 24-byte nonce | [x] |
| 54 | `crypto_aead_xchacha20poly1305_ietf_*_detached` | same sweep | [x] |
| 55 | `crypto_aead_aegis128l_encrypt`/`_decrypt` | `mlen` ∈ {0,1,31,32,33,63,64,65,1000} × `adlen` ∈ {0,1,31,32,33,64} | [x] |
| 56 | `crypto_aead_aegis128l_*_detached` | same sweep + `maclen_p` non-NULL/NULL | [x] |
| 57 | `crypto_aead_aegis256_encrypt`/`_decrypt` | same sweep, 32-byte nonce | [x] |
| 58 | `crypto_aead_aegis256_*_detached` | same sweep | [x] |
| 59 | `crypto_aead_aes256gcm_is_available` + all 9 aes256gcm entry points | portable build: all return `-1`/`ENOSYS`, `is_available()==0` (see ERRORS 131–140) | [x] |
| 60 | all AEAD `*_keygen` | output length + distinctness only (randomness) | [x] |
| 61 | AEAD in-place (`c == m`) | chacha20poly1305 + aegis, `mlen` sweep | [x] |

### E. ipcrypt

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 62 | `crypto_ipcrypt_encrypt`/`_decrypt` (deterministic) | random keys × IPv4/IPv6/IPv4-mapped/`::`/all-zero/all-0xff 16-byte inputs | [x] |
| 63 | `crypto_ipcrypt_nd_encrypt`/`_nd_decrypt` | random 8-byte tweak (deterministic-mode API) × same input shapes | [x] |
| 64 | `crypto_ipcrypt_ndx_encrypt`/`_ndx_decrypt` | random 16-byte tweak × same input shapes | [x] |
| 65 | `crypto_ipcrypt_str_encrypt`/`_str_decrypt` (+ nd/ndx str forms) | IPv4 / IPv6 / IPv4-mapped / zone-bearing / compressed text forms | [x] |
| 66 | `crypto_ipcrypt_*_keygen`, `*_bytes`/`*_keybytes` constants | length + constant values | [x] |

### F. secretbox

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 67 | `crypto_secretbox_xsalsa20poly1305` / `_open` (zero-padded low-level API) | `mlen` ∈ {32,33,47,48,49,64,96,1000} | [x] |
| 68 | `crypto_secretbox_easy` / `_open_easy` | `mlen` ∈ {0,1,15,16,17,31,32,33,64,1000} | [x] |
| 69 | `crypto_secretbox_detached` / `_open_detached` | same sweep | [x] |
| 70 | `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy` | same sweep | [x] |
| 71 | `crypto_secretbox_xchacha20poly1305_detached` / `_open_detached` | same sweep | [x] |
| 72 | secretbox in-place (`c == m` / `m == c`) | easy + detached, `mlen` sweep | [x] |

### G. box — full × precomputed × sealed, both primitives

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 73 | `crypto_box_keypair`, `_seed_keypair` | fixed seeds (deterministic) — byte-identical keypairs | [x] |
| 74 | `crypto_box_beforenm` + `_afternm` / `_open_afternm` | zero-padded low-level API, `mlen` ∈ {32,33,64,1000} | [x] |
| 75 | `crypto_box_easy` / `_open_easy` | `mlen` sweep {0,1,16,17,64,1000} | [x] |
| 76 | `crypto_box_detached` / `_open_detached` | `mlen` sweep | [x] |
| 77 | `crypto_box_easy_afternm` / `_open_easy_afternm` | `mlen` sweep, precomputed key | [x] |
| 78 | `crypto_box_detached_afternm` / `_open_detached_afternm` | `mlen` sweep | [x] |
| 79 | `crypto_box_seal` / `_seal_open` | `mlen` sweep; seal is randomized so only round-trip + length compared, `_seal_open` compared byte-exactly on C-produced ciphertext | [x] |
| 80 | `crypto_box_curve25519xsalsa20poly1305` / `_open` (raw) | `mlen` ∈ {32,33,64,1000} | [x] |
| 81 | `crypto_box_curve25519xsalsa20poly1305_beforenm`/`_afternm`/`_open_afternm` | `mlen` sweep | [x] |
| 82 | `crypto_box_curve25519xchacha20poly1305_easy`/`_open_easy` | `mlen` sweep | [x] |
| 83 | `crypto_box_curve25519xchacha20poly1305_detached`/`_open_detached` | `mlen` sweep | [x] |
| 84 | `crypto_box_curve25519xchacha20poly1305_beforenm` + all 4 `_afternm` forms | `mlen` sweep | [x] |
| 85 | `crypto_box_curve25519xchacha20poly1305_seal`/`_seal_open` | round-trip + cross-library `_seal_open` | [x] |

### H. sign

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 86 | `crypto_sign_seed_keypair`, `crypto_sign_ed25519_seed_keypair` | fixed seeds incl. all-zero, all-0xff | [x] |
| 87 | `crypto_sign_ed25519_sk_to_seed`, `_sk_to_pk` | round-trip from generated keys | [x] |
| 88 | `crypto_sign_ed25519_pk_to_curve25519`, `_sk_to_curve25519` | valid keys | [x] |
| 89 | `crypto_sign_ed25519_detached` / `_verify_detached` | `mlen` ∈ {0,1,31,32,33,63,64,65,127,128,1000} | [x] |
| 90 | `crypto_sign_ed25519` / `_open` (combined) | same `mlen` sweep, `mlen_p`/`smlen_p` NULL and non-NULL | [x] |
| 91 | `crypto_sign_ed25519ph_init/update/final_create` | random update splits (1–8), crossing 128-byte sha512 blocks | [x] |
| 92 | `crypto_sign_ed25519ph_final_verify` | valid sig, and each-byte-flipped sig | [x] |
| 93 | `crypto_sign_init/update/final_create/final_verify` (generic) | same as 91/92 | [x] |

### I. scalarmult / core group ops

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 94 | `crypto_scalarmult`, `_base` (generic = curve25519) | random scalars, random valid points | [x] |
| 95 | `crypto_scalarmult_ed25519`, `_base` (clamped) | random scalars/points from `_base` outputs | [x] |
| 96 | `crypto_scalarmult_ed25519_noclamp`, `_base_noclamp` | random scalars reduced mod L, and non-reduced | [x] |
| 97 | `crypto_scalarmult_ristretto255`, `_base` | random scalars, points from `_from_hash` | [x] |
| 98 | `crypto_core_ed25519_add`/`_sub` | valid point pairs from `crypto_scalarmult_ed25519_base` | [x] |
| 99 | `crypto_core_ed25519_from_string_nu` | non-uniform map; `hash_alg` ∈ {1,2} × ctx/msg sweep. (This build exports `_from_string`/`_from_string_nu`, **not** `_from_uniform`.) | [x] |
| 100 | `crypto_core_ed25519_from_string` | `hash_alg` ∈ {SHA256(1), SHA512(2)} × `ctx_len` ∈ {0,1,32,255} × `msg_len` ∈ {0,1,64,1000} | [x] |
| 101 | `crypto_core_ed25519_from_string` (2-point / random-oracle variant) | same sweep. `_from_string_ro` is **not** exported by this build. | [x] |
| 102 | `crypto_core_ed25519_random`, `_is_valid_point` | valid + hand-built invalid encodings | [x] |
| 103 | `crypto_core_ed25519_scalar_random/_invert/_negate/_complement/_add/_sub/_mul/_reduce/_is_canonical` | random 32/64-byte scalars, all-zero, L−1, L, L+1, all-0xff | [x] |
| 104 | `crypto_core_ed25519_scalar_from_string` | `hash_alg` ∈ {1,2} × ctx/msg length sweep | [x] |
| 105 | `crypto_core_ristretto255_add`/`_sub` | valid points from `crypto_core_ristretto255_from_hash` | [x] |
| 106 | `crypto_core_ristretto255_from_hash` | random 64-byte inputs | [x] |
| 107 | `crypto_core_ristretto255_from_string` | `hash_alg` ∈ {1,2} × ctx/msg sweep. `_from_string_ro` is **not** exported by this build. | [x] |
| 108 | `crypto_core_ristretto255_random`, `_is_valid_point` | valid + invalid encodings | [x] |
| 109 | `crypto_core_ristretto255_scalar_*` (invert/negate/complement/add/sub/mul/reduce/random/is_canonical) | same scalar corpus as 103 | [x] |

### J. kx / kem

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 110 | `crypto_kx_keypair`, `_seed_keypair` | fixed seeds | [x] |
| 111 | `crypto_kx_client_session_keys` | `rx`/`tx` both set; `tx == NULL` (aliases `rx`) | [x] |
| 112 | `crypto_kx_server_session_keys` | `rx`/`tx` both set; `tx == NULL` | [x] |
| 113 | `crypto_kem_mlkem768_seed_keypair` | fixed 64-byte seeds incl. all-zero/all-0xff | [x] |
| 114 | `crypto_kem_mlkem768_enc_deterministic` + `_dec` | fixed 32-byte coins × keypairs from fixed seeds | [x] |
| 115 | `crypto_kem_mlkem768_enc` + `_dec` | round-trip agreement (randomized enc) | [x] |
| 116 | `crypto_kem_xwing_seed_keypair` | fixed 32-byte seeds | [x] |
| 117 | `crypto_kem_xwing_enc_deterministic` + `_dec` | fixed 64-byte seeds | [x] |
| 118 | `crypto_kem_xwing_enc` + `_dec` | round-trip agreement | [x] |
| 119 | `crypto_kem_*_keypair` / generic `crypto_kem_*` wrappers | round-trip; constants | [x] |

### K. secretstream — the full state machine

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 120 | `..._init_push` + `_push` + `_init_pull` + `_pull` | tag `MESSAGE(0)` only, 1–8 messages, `mlen` ∈ {0,1,16,17,64,1000} | [x] |
| 121 | same | tag `PUSH(1)` interleaved | [x] |
| 122 | same | tag `REKEY(2)` — exercises the explicit-rekey branch | [x] |
| 123 | same | tag `FINAL(3)` as last message | [x] |
| 124 | same | `ad` ∈ {NULL/0, 1, 16, 17, 64} per message, varying per message | [x] |
| 125 | `..._rekey` called explicitly mid-stream on both sides | after N messages, then continue | [x] |
| 126 | `..._pull` with `mlen_p == NULL` / `tag_p == NULL` | valid stream | [x] |
| 127 | `..._keygen`, `_statebytes`, all `*bytes` constants | values | [x] |
| 128 | implicit counter rollover branch (`_counter_reset`/nonce increment) | 300+ sequential messages | [x] |

### L. pwhash

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 129 | `crypto_pwhash` | `alg = ARGON2I13(1)` × `outlen` ∈ {16,17,32,64} × `opslimit` ∈ {3,4} × `memlimit` ∈ {8192,16384} × `passwdlen` ∈ {0,1,32} | [x] |
| 130 | `crypto_pwhash` | `alg = ARGON2ID13(2)` × same sweep with `opslimit` ∈ {1,2,3} | [x] |
| 131 | `crypto_pwhash_argon2i` | direct, `alg = 1`, same sweep | [x] |
| 132 | `crypto_pwhash_argon2id` | direct, `alg = 2`, same sweep | [x] |
| 133 | `crypto_pwhash_argon2i_str` + `_str_verify` | verify C-produced string with Rust and vice versa (`_str` embeds random salt) | [x] |
| 134 | `crypto_pwhash_argon2id_str` + `_str_verify` | cross-library verify | [x] |
| 135 | `crypto_pwhash_str_alg` + `crypto_pwhash_str_verify` | `alg` ∈ {1,2}, cross-library verify | [x] |
| 136 | `crypto_pwhash_str_needs_rehash`, `_argon2i_str_needs_rehash`, `_argon2id_str_needs_rehash` | same/lower/higher opslimit & memlimit than embedded | [x] |
| 137 | `crypto_pwhash_scryptsalsa208sha256` | `outlen` ∈ {16,17,32,64} × `opslimit`/`memlimit` at `INTERACTIVE` and minimum valid | [x] |
| 138 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N` ∈ {2,4,16,1024} × `r` ∈ {1,2,8} × `p` ∈ {1,2,4} × `buflen` ∈ {1,32,64,100} × `saltlen` ∈ {0,1,32} | [x] |
| 139 | `crypto_pwhash_scryptsalsa208sha256_str` + `_str_verify` | cross-library verify | [x] |
| 140 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | matching / differing params | [x] |
| 141 | all pwhash `*_alg_argon2i13/_argon2id13/_alg_default`, `*_bytes_min/max`, `*_opslimit_*`, `*_memlimit_*` accessors | exact constant values | [x] |

### M. utils / codecs / runtime / version

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 142 | `sodium_bin2hex` / `sodium_hex2bin` | `bin_len` ∈ {0,1,2,31,32,1000} × `ignore` ∈ {NULL,`""`,`" :\n"`} × mixed-case input | [x] |
| 143 | `sodium_bin2base64` / `sodium_base642bin` | variant ∈ {1,3,5,7} × `bin_len` ∈ {0,1,2,3,4,5,31,32,33,1000} (all `% 3` residues) | [x] |
| 144 | `sodium_base64_encoded_len` | variant ∈ {1,3,5,7} × `bin_len` 0..64 | [x] |
| 145 | `sodium_base642bin` with `ignore` | variant sweep × whitespace-injected input | [x] |
| 146 | `sodium_base642bin` `b64_end` out-param | non-NULL and NULL | [x] |
| 147 | `sodium_ip2bin` / `sodium_bin2ip` | IPv4, IPv6 full, `::`-compressed, IPv4-mapped, `%zone`, leading/trailing forms | [x] |
| 148 | `sodium_pad` / `sodium_unpad` | `blocksize` ∈ {1,2,15,16,17,64} × `unpadded_buflen` ∈ {0,1,15,16,17,63,64,65} | [x] |
| 149 | `sodium_memcmp`, `sodium_compare`, `sodium_is_zero`, `sodium_increment`, `sodium_add`, `sodium_sub` | `len` ∈ {0,1,8,16,32,64} × all-zero/all-0xff/random (carry & borrow chains) | [x] |
| 150 | `sodium_increment` overflow, `sodium_add`/`sub` full-carry | all-0xff operands at each len | [x] |
| 151 | `sodium_stackzero`, `sodium_memzero`, `sodium_munlock`/`mlock` | no-crash + return value | [x] |
| 152 | `sodium_malloc`/`allocarray`/`free`/`mprotect_*` | sizes 0,1,4095,4096,4097; noaccess→readonly→readwrite transitions | [x] |
| 153 | `sodium_init` (idempotent), `sodium_set_misuse_handler` | repeated calls | [x] |
| 154 | `sodium_version_string`, `_library_version_major/minor`, `_library_minimal` | exact values | [x] |
| 155 | `sodium_runtime_has_*` (neon, sse2, sse3, ssse3, sse41, avx, avx2, avx512f, aesni, pclmul, rdrand, armcrypto, sha3, sm3, sm4) | all 15 predicates — C and Rust must agree on this host | [x] |
| 156 | all `crypto_*_bytes()` / `*_keybytes()` / `*_statebytes()` / `*_primitive()` accessors (≈300 symbols) | exact returned value / string for every one | [x] |
| 157 | `randombytes_buf_deterministic` | `size` ∈ {0,1,63,64,65,1000} × fixed seeds | [x] |
| 158 | `randombytes_implementation_name`, `_random`, `_uniform`, `_buf`, `_stir`, `_close`, `_set_implementation` | name string; `_uniform` upper bounds {0,1,2,3,255,256,2^31}; non-crash | [x] |
| 159 | `randombytes_SEEDBYTES`, `randombytes_seedbytes`, `randombytes_BYTES_MAX` | constants | [x] |
