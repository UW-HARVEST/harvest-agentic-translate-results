# Configuration Surface

Build-time matrix: exactly one valid combination, `--no-default-features` (the manifest
declares no features). CMake compiles every C source without `HAVE_*` backend macros,
selecting the portable fallbacks. Rows cover every dynamic entry point and both outcomes
of every direct source branch in its body; impossible outcomes are exercised as rejection
rows in `ERRORS.md` rather than duplicated here.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|:---:|
| 1 | `_crypto_aead_aegis128l_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 2 | `_crypto_aead_aegis256_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 3 | `_crypto_generichash_blake2b_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 4 | `_crypto_ipcrypt_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 5 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 6 | `_crypto_pwhash_argon2_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 7 | `_crypto_scalarmult_curve25519_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 8 | `_crypto_sign_ed25519_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 9 | `_crypto_sign_ed25519_detached` | default portable build; source branch `if (siglen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:90`) | [x] |
| 10 | `_crypto_sign_ed25519_detached` | default portable build; source branch `if (siglen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:90`) | [x] |
| 11 | `_crypto_sign_ed25519_ref10_hinit` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 12 | `_crypto_sign_ed25519_ref10_hinit` | default portable build; source branch `if (prehashed) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:22`) | [x] |
| 13 | `_crypto_sign_ed25519_ref10_hinit` | default portable build; source branch `if (prehashed) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:22`) | [x] |
| 14 | `_crypto_sign_ed25519_verify_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 15 | `_crypto_sign_ed25519_verify_detached` | default portable build; source branch `if (sig[63] & 224) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:31`) | [x] |
| 16 | `_crypto_sign_ed25519_verify_detached` | default portable build; source branch `if ((sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:35`) | [x] |
| 17 | `_crypto_sign_ed25519_verify_detached` | default portable build; source branch `if (ge25519_is_canonical(pk) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:39`) | [x] |
| 18 | `_crypto_sign_ed25519_verify_detached` | default portable build; source branch `if (ge25519_frombytes_negate_vartime(&A, pk) != 0 \|\| ge25519_has_small_order(&A) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:43`) | [x] |
| 19 | `_crypto_sign_ed25519_verify_detached` | default portable build; source branch `if (ge25519_frombytes(&expected_r, sig) != 0 \|\| ge25519_has_small_order(&expected_r) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:47`) | [x] |
| 20 | `_crypto_stream_chacha20_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 21 | `_crypto_stream_chacha20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_neon()) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:202`) | [x] |
| 22 | `_crypto_stream_chacha20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_neon()) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:202`) | [x] |
| 23 | `_crypto_stream_salsa20_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 24 | `_crypto_stream_salsa20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_sse2()) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c:111`) | [x] |
| 25 | `_crypto_stream_salsa20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_sse2()) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c:111`) | [x] |
| 26 | `_crypto_stream_salsa20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_neon()) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c:118`) | [x] |
| 27 | `_crypto_stream_salsa20_pick_best_implementation` | default portable build; source branch `if (sodium_runtime_has_neon()) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c:118`) | [x] |
| 28 | `_sodium_alloc_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 29 | `_sodium_argon2_ctx` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 30 | `_sodium_argon2_decode_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 31 | `_sodium_argon2_encode_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 32 | `_sodium_argon2_fill_memory_blocks` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 33 | `_sodium_argon2_fill_segment_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 34 | `_sodium_argon2_finalize` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 35 | `_sodium_argon2_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 36 | `_sodium_argon2_initialize` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 37 | `_sodium_argon2_validate_inputs` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 38 | `_sodium_argon2_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 39 | `_sodium_argon2i_hash_encoded` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 40 | `_sodium_argon2i_hash_raw` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 41 | `_sodium_argon2i_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 42 | `_sodium_argon2id_hash_encoded` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 43 | `_sodium_argon2id_hash_raw` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 44 | `_sodium_argon2id_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 45 | `_sodium_blake2b` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 46 | `_sodium_blake2b_compress_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 47 | `_sodium_blake2b_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 48 | `_sodium_blake2b_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 49 | `_sodium_blake2b_init_key` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 50 | `_sodium_blake2b_init_key_salt_personal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 51 | `_sodium_blake2b_init_param` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 52 | `_sodium_blake2b_init_salt_personal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 53 | `_sodium_blake2b_long` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 54 | `_sodium_blake2b_pick_best_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 55 | `_sodium_blake2b_salt_personal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 56 | `_sodium_blake2b_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 57 | `_sodium_core_h2c_string_to_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 58 | `_sodium_escrypt_PBKDF2_SHA256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 59 | `_sodium_escrypt_alloc_region` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 60 | `_sodium_escrypt_free_local` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 61 | `_sodium_escrypt_free_region` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 62 | `_sodium_escrypt_gensalt_r` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 63 | `_sodium_escrypt_init_local` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 64 | `_sodium_escrypt_kdf_nosse` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 65 | `_sodium_escrypt_parse_setting` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 66 | `_sodium_escrypt_r` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 67 | `_sodium_fe25519_frombytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 68 | `_sodium_fe25519_invert` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 69 | `_sodium_fe25519_tobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 70 | `_sodium_ge25519_clear_cofactor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 71 | `_sodium_ge25519_double_scalarmult_vartime` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 72 | `_sodium_ge25519_from_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 73 | `_sodium_ge25519_from_uniform` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 74 | `_sodium_ge25519_frombytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 75 | `_sodium_ge25519_frombytes_negate_vartime` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 76 | `_sodium_ge25519_has_small_order` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 77 | `_sodium_ge25519_is_canonical` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 78 | `_sodium_ge25519_is_on_curve` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 79 | `_sodium_ge25519_is_on_main_subgroup` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 80 | `_sodium_ge25519_p1p1_to_p2` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 81 | `_sodium_ge25519_p1p1_to_p3` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 82 | `_sodium_ge25519_p2_to_p3` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 83 | `_sodium_ge25519_p3_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 84 | `_sodium_ge25519_p3_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 85 | `_sodium_ge25519_p3_tobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 86 | `_sodium_ge25519_scalarmult` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 87 | `_sodium_ge25519_scalarmult_base` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 88 | `_sodium_ge25519_tobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 89 | `_sodium_keccak1600_ref_extract_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 90 | `_sodium_keccak1600_ref_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 91 | `_sodium_keccak1600_ref_permute_12` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 92 | `_sodium_keccak1600_ref_permute_24` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 93 | `_sodium_keccak1600_ref_xor_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 94 | `_sodium_mlkem768_ref_dec` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 95 | `_sodium_mlkem768_ref_enc` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 96 | `_sodium_mlkem768_ref_enc_deterministic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 97 | `_sodium_mlkem768_ref_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 98 | `_sodium_mlkem768_ref_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 99 | `_sodium_ristretto255_from_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 100 | `_sodium_ristretto255_frombytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 101 | `_sodium_ristretto255_p3_tobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 102 | `_sodium_runtime_get_cpu_features` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 103 | `_sodium_sc25519_invert` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 104 | `_sodium_sc25519_is_canonical` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 105 | `_sodium_sc25519_mul` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 106 | `_sodium_sc25519_muladd` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 107 | `_sodium_sc25519_reduce` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 108 | `_sodium_shake128_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 109 | `_sodium_shake128_ref_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 110 | `_sodium_shake128_ref_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 111 | `_sodium_shake128_ref_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 112 | `_sodium_shake128_ref_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 113 | `_sodium_shake256_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 114 | `_sodium_shake256_ref_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 115 | `_sodium_shake256_ref_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 116 | `_sodium_shake256_ref_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 117 | `_sodium_shake256_ref_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 118 | `_sodium_softaes_block_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 119 | `_sodium_softaes_block_decryptlast` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 120 | `_sodium_softaes_block_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 121 | `_sodium_softaes_block_encryptlast` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 122 | `_sodium_softaes_expand_key128` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 123 | `_sodium_softaes_expand_key256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 124 | `_sodium_softaes_inv_mix_columns` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 125 | `_sodium_softaes_invert_key_schedule128` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 126 | `_sodium_softaes_invert_key_schedule256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 127 | `_sodium_turboshake128_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 128 | `_sodium_turboshake128_ref_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 129 | `_sodium_turboshake128_ref_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 130 | `_sodium_turboshake128_ref_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 131 | `_sodium_turboshake128_ref_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 132 | `_sodium_turboshake256_ref` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 133 | `_sodium_turboshake256_ref_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 134 | `_sodium_turboshake256_ref_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 135 | `_sodium_turboshake256_ref_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 136 | `_sodium_turboshake256_ref_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 137 | `aegis128l_soft_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 138 | `aegis256_soft_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 139 | `crypto_aead_aegis128l_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 140 | `crypto_aead_aegis128l_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 141 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (clen >= crypto_aead_aegis128l_ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:92`) | [x] |
| 142 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (clen >= crypto_aead_aegis128l_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:92`) | [x] |
| 143 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:97`) | [x] |
| 144 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:97`) | [x] |
| 145 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:98`) | [x] |
| 146 | `crypto_aead_aegis128l_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:98`) | [x] |
| 147 | `crypto_aead_aegis128l_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 148 | `crypto_aead_aegis128l_decrypt_detached` | default portable build; source branch `if (clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:137`) | [x] |
| 149 | `crypto_aead_aegis128l_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 150 | `crypto_aead_aegis128l_encrypt` | default portable build; source branch `if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:69`) | [x] |
| 151 | `crypto_aead_aegis128l_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:74`) | [x] |
| 152 | `crypto_aead_aegis128l_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:74`) | [x] |
| 153 | `crypto_aead_aegis128l_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:75`) | [x] |
| 154 | `crypto_aead_aegis128l_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:75`) | [x] |
| 155 | `crypto_aead_aegis128l_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 156 | `crypto_aead_aegis128l_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:116`) | [x] |
| 157 | `crypto_aead_aegis128l_encrypt_detached` | default portable build; source branch `if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:119`) | [x] |
| 158 | `crypto_aead_aegis128l_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 159 | `crypto_aead_aegis128l_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 160 | `crypto_aead_aegis128l_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 161 | `crypto_aead_aegis128l_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 162 | `crypto_aead_aegis128l_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 163 | `crypto_aead_aegis256_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 164 | `crypto_aead_aegis256_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 165 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (clen >= crypto_aead_aegis256_ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:92`) | [x] |
| 166 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (clen >= crypto_aead_aegis256_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:92`) | [x] |
| 167 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:97`) | [x] |
| 168 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:97`) | [x] |
| 169 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:98`) | [x] |
| 170 | `crypto_aead_aegis256_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:98`) | [x] |
| 171 | `crypto_aead_aegis256_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 172 | `crypto_aead_aegis256_decrypt_detached` | default portable build; source branch `if (clen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:136`) | [x] |
| 173 | `crypto_aead_aegis256_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 174 | `crypto_aead_aegis256_encrypt` | default portable build; source branch `if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:69`) | [x] |
| 175 | `crypto_aead_aegis256_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:74`) | [x] |
| 176 | `crypto_aead_aegis256_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:74`) | [x] |
| 177 | `crypto_aead_aegis256_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:75`) | [x] |
| 178 | `crypto_aead_aegis256_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:75`) | [x] |
| 179 | `crypto_aead_aegis256_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 180 | `crypto_aead_aegis256_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:116`) | [x] |
| 181 | `crypto_aead_aegis256_encrypt_detached` | default portable build; source branch `if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:119`) | [x] |
| 182 | `crypto_aead_aegis256_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 183 | `crypto_aead_aegis256_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 184 | `crypto_aead_aegis256_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 185 | `crypto_aead_aegis256_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 186 | `crypto_aead_aegis256_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 187 | `crypto_aead_aes256gcm_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 188 | `crypto_aead_aes256gcm_beforenm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 189 | `crypto_aead_aes256gcm_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 190 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 191 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (clen >= ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:991`) | [x] |
| 192 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (clen >= ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:991`) | [x] |
| 193 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:995`) | [x] |
| 194 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:995`) | [x] |
| 195 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:996`) | [x] |
| 196 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:996`) | [x] |
| 197 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (clen >= ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:952`) | [x] |
| 198 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (clen >= ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:952`) | [x] |
| 199 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:956`) | [x] |
| 200 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:956`) | [x] |
| 201 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:957`) | [x] |
| 202 | `crypto_aead_aes256gcm_decrypt_afternm` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:957`) | [x] |
| 203 | `crypto_aead_aes256gcm_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 204 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 205 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:954`) | [x] |
| 206 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:957`) | [x] |
| 207 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (gh_required_blocks == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:961`) | [x] |
| 208 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (crypto_verify_16(mac, computed_mac) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:972`) | [x] |
| 209 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:915`) | [x] |
| 210 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:918`) | [x] |
| 211 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (gh_required_blocks == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:922`) | [x] |
| 212 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | default portable build; source branch `if (crypto_verify_16(mac, computed_mac) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:933`) | [x] |
| 213 | `crypto_aead_aes256gcm_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 214 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:825`) | [x] |
| 215 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:825`) | [x] |
| 216 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:826`) | [x] |
| 217 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:826`) | [x] |
| 218 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:786`) | [x] |
| 219 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:786`) | [x] |
| 220 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:787`) | [x] |
| 221 | `crypto_aead_aes256gcm_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:787`) | [x] |
| 222 | `crypto_aead_aes256gcm_encrypt_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 223 | `crypto_aead_aes256gcm_encrypt_afternm` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:866`) | [x] |
| 224 | `crypto_aead_aes256gcm_encrypt_afternm` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:866`) | [x] |
| 225 | `crypto_aead_aes256gcm_encrypt_afternm` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:827`) | [x] |
| 226 | `crypto_aead_aes256gcm_encrypt_afternm` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:827`) | [x] |
| 227 | `crypto_aead_aes256gcm_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 228 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 229 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:791`) | [x] |
| 230 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:794`) | [x] |
| 231 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (gh_required_blocks == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:798`) | [x] |
| 232 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:811`) | [x] |
| 233 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:811`) | [x] |
| 234 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:752`) | [x] |
| 235 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:755`) | [x] |
| 236 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (gh_required_blocks == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:759`) | [x] |
| 237 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:772`) | [x] |
| 238 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:772`) | [x] |
| 239 | `crypto_aead_aes256gcm_is_available` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 240 | `crypto_aead_aes256gcm_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 241 | `crypto_aead_aes256gcm_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 242 | `crypto_aead_aes256gcm_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 243 | `crypto_aead_aes256gcm_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 244 | `crypto_aead_aes256gcm_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 245 | `crypto_aead_aes256gcm_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 246 | `crypto_aead_chacha20poly1305_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 247 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 248 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (clen >= crypto_aead_chacha20poly1305_ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:259`) | [x] |
| 249 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (clen >= crypto_aead_chacha20poly1305_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:259`) | [x] |
| 250 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:266`) | [x] |
| 251 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:266`) | [x] |
| 252 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:267`) | [x] |
| 253 | `crypto_aead_chacha20poly1305_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:267`) | [x] |
| 254 | `crypto_aead_chacha20poly1305_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 255 | `crypto_aead_chacha20poly1305_decrypt_detached` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:232`) | [x] |
| 256 | `crypto_aead_chacha20poly1305_decrypt_detached` | default portable build; source branch `if (ret != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:235`) | [x] |
| 257 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 258 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; source branch `if (mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:89`) | [x] |
| 259 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:97`) | [x] |
| 260 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:97`) | [x] |
| 261 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:98`) | [x] |
| 262 | `crypto_aead_chacha20poly1305_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:98`) | [x] |
| 263 | `crypto_aead_chacha20poly1305_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 264 | `crypto_aead_chacha20poly1305_encrypt_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:54`) | [x] |
| 265 | `crypto_aead_chacha20poly1305_encrypt_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:54`) | [x] |
| 266 | `crypto_aead_chacha20poly1305_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:69`) | [x] |
| 267 | `crypto_aead_chacha20poly1305_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:69`) | [x] |
| 268 | `crypto_aead_chacha20poly1305_ietf_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 269 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 270 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (clen >= crypto_aead_chacha20poly1305_ietf_ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:344`) | [x] |
| 271 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (clen >= crypto_aead_chacha20poly1305_ietf_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:344`) | [x] |
| 272 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:351`) | [x] |
| 273 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:351`) | [x] |
| 274 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:352`) | [x] |
| 275 | `crypto_aead_chacha20poly1305_ietf_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:352`) | [x] |
| 276 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 277 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:317`) | [x] |
| 278 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | default portable build; source branch `if (ret != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:320`) | [x] |
| 279 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 280 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; source branch `if (mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:177`) | [x] |
| 281 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:185`) | [x] |
| 282 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:185`) | [x] |
| 283 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:186`) | [x] |
| 284 | `crypto_aead_chacha20poly1305_ietf_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:186`) | [x] |
| 285 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 286 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:136`) | [x] |
| 287 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:136`) | [x] |
| 288 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:157`) | [x] |
| 289 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | default portable build; source branch `if (maclen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:157`) | [x] |
| 290 | `crypto_aead_chacha20poly1305_ietf_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 291 | `crypto_aead_chacha20poly1305_ietf_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 292 | `crypto_aead_chacha20poly1305_ietf_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 293 | `crypto_aead_chacha20poly1305_ietf_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 294 | `crypto_aead_chacha20poly1305_ietf_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 295 | `crypto_aead_chacha20poly1305_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 296 | `crypto_aead_chacha20poly1305_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 297 | `crypto_aead_chacha20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 298 | `crypto_aead_chacha20poly1305_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 299 | `crypto_aead_chacha20poly1305_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 300 | `crypto_aead_xchacha20poly1305_ietf_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 301 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 302 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (clen >= crypto_aead_xchacha20poly1305_ietf_ABYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:237`) | [x] |
| 303 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (clen >= crypto_aead_xchacha20poly1305_ietf_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:237`) | [x] |
| 304 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:244`) | [x] |
| 305 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:244`) | [x] |
| 306 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:245`) | [x] |
| 307 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:245`) | [x] |
| 308 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 309 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 310 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; source branch `if (mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:185`) | [x] |
| 311 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:190`) | [x] |
| 312 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; source branch `if (clen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:190`) | [x] |
| 313 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:191`) | [x] |
| 314 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | default portable build; source branch `if (ret == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:191`) | [x] |
| 315 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 316 | `crypto_aead_xchacha20poly1305_ietf_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 317 | `crypto_aead_xchacha20poly1305_ietf_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 318 | `crypto_aead_xchacha20poly1305_ietf_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 319 | `crypto_aead_xchacha20poly1305_ietf_npubbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 320 | `crypto_aead_xchacha20poly1305_ietf_nsecbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 321 | `crypto_auth` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 322 | `crypto_auth_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 323 | `crypto_auth_hmacsha256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 324 | `crypto_auth_hmacsha256_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 325 | `crypto_auth_hmacsha256_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 326 | `crypto_auth_hmacsha256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 327 | `crypto_auth_hmacsha256_init` | default portable build; source branch `if (keylen > 64) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c:45`) | [x] |
| 328 | `crypto_auth_hmacsha256_init` | default portable build; source branch `} else if (key == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c:51`) | [x] |
| 329 | `crypto_auth_hmacsha256_init` | default portable build; source branch `if (keylen > 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c:52`) | [x] |
| 330 | `crypto_auth_hmacsha256_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 331 | `crypto_auth_hmacsha256_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 332 | `crypto_auth_hmacsha256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 333 | `crypto_auth_hmacsha256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 334 | `crypto_auth_hmacsha256_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 335 | `crypto_auth_hmacsha512` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 336 | `crypto_auth_hmacsha512256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 337 | `crypto_auth_hmacsha512256_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 338 | `crypto_auth_hmacsha512256_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 339 | `crypto_auth_hmacsha512256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 340 | `crypto_auth_hmacsha512256_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 341 | `crypto_auth_hmacsha512256_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 342 | `crypto_auth_hmacsha512256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 343 | `crypto_auth_hmacsha512256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 344 | `crypto_auth_hmacsha512256_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 345 | `crypto_auth_hmacsha512_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 346 | `crypto_auth_hmacsha512_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 347 | `crypto_auth_hmacsha512_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 348 | `crypto_auth_hmacsha512_init` | default portable build; source branch `if (keylen > 128) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c:45`) | [x] |
| 349 | `crypto_auth_hmacsha512_init` | default portable build; source branch `} else if (key == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c:51`) | [x] |
| 350 | `crypto_auth_hmacsha512_init` | default portable build; source branch `if (keylen > 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c:52`) | [x] |
| 351 | `crypto_auth_hmacsha512_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 352 | `crypto_auth_hmacsha512_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 353 | `crypto_auth_hmacsha512_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 354 | `crypto_auth_hmacsha512_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 355 | `crypto_auth_hmacsha512_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 356 | `crypto_auth_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 357 | `crypto_auth_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 358 | `crypto_auth_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 359 | `crypto_auth_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 360 | `crypto_box` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 361 | `crypto_box_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 362 | `crypto_box_beforenm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 363 | `crypto_box_beforenmbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 364 | `crypto_box_boxzerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 365 | `crypto_box_curve25519xchacha20poly1305_beforenm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 366 | `crypto_box_curve25519xchacha20poly1305_beforenm` | default portable build; source branch `if (crypto_scalarmult_curve25519(s, sk, pk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:48`) | [x] |
| 367 | `crypto_box_curve25519xchacha20poly1305_beforenmbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 368 | `crypto_box_curve25519xchacha20poly1305_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 369 | `crypto_box_curve25519xchacha20poly1305_detached` | default portable build; source branch `if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:76`) | [x] |
| 370 | `crypto_box_curve25519xchacha20poly1305_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 371 | `crypto_box_curve25519xchacha20poly1305_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 372 | `crypto_box_curve25519xchacha20poly1305_easy` | default portable build; source branch `if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:105`) | [x] |
| 373 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 374 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | default portable build; source branch `if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:93`) | [x] |
| 375 | `crypto_box_curve25519xchacha20poly1305_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 376 | `crypto_box_curve25519xchacha20poly1305_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 377 | `crypto_box_curve25519xchacha20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 378 | `crypto_box_curve25519xchacha20poly1305_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 379 | `crypto_box_curve25519xchacha20poly1305_open_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 380 | `crypto_box_curve25519xchacha20poly1305_open_detached` | default portable build; source branch `if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:131`) | [x] |
| 381 | `crypto_box_curve25519xchacha20poly1305_open_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 382 | `crypto_box_curve25519xchacha20poly1305_open_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 383 | `crypto_box_curve25519xchacha20poly1305_open_easy` | default portable build; source branch `if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:159`) | [x] |
| 384 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 385 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | default portable build; source branch `if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:146`) | [x] |
| 386 | `crypto_box_curve25519xchacha20poly1305_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 387 | `crypto_box_curve25519xchacha20poly1305_seal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 388 | `crypto_box_curve25519xchacha20poly1305_seal` | default portable build; source branch `if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:39`) | [x] |
| 389 | `crypto_box_curve25519xchacha20poly1305_seal` | default portable build; source branch `if (crypto_box_curve25519xchacha20poly1305_keypair(epk, esk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:42`) | [x] |
| 390 | `crypto_box_curve25519xchacha20poly1305_seal_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 391 | `crypto_box_curve25519xchacha20poly1305_seal_open` | default portable build; source branch `if (clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:63`) | [x] |
| 392 | `crypto_box_curve25519xchacha20poly1305_sealbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 393 | `crypto_box_curve25519xchacha20poly1305_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 394 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 395 | `crypto_box_curve25519xchacha20poly1305_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 396 | `crypto_box_curve25519xsalsa20poly1305` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 397 | `crypto_box_curve25519xsalsa20poly1305` | default portable build; source branch `if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:81`) | [x] |
| 398 | `crypto_box_curve25519xsalsa20poly1305_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 399 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 400 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | default portable build; source branch `if (crypto_scalarmult_curve25519(s, sk, pk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:42`) | [x] |
| 401 | `crypto_box_curve25519xsalsa20poly1305_beforenmbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 402 | `crypto_box_curve25519xsalsa20poly1305_boxzerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 403 | `crypto_box_curve25519xsalsa20poly1305_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 404 | `crypto_box_curve25519xsalsa20poly1305_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 405 | `crypto_box_curve25519xsalsa20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 406 | `crypto_box_curve25519xsalsa20poly1305_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 407 | `crypto_box_curve25519xsalsa20poly1305_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 408 | `crypto_box_curve25519xsalsa20poly1305_open` | default portable build; source branch `if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:98`) | [x] |
| 409 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 410 | `crypto_box_curve25519xsalsa20poly1305_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 411 | `crypto_box_curve25519xsalsa20poly1305_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 412 | `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 413 | `crypto_box_curve25519xsalsa20poly1305_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 414 | `crypto_box_curve25519xsalsa20poly1305_zerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 415 | `crypto_box_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 416 | `crypto_box_detached` | default portable build; source branch `if (crypto_box_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:30`) | [x] |
| 417 | `crypto_box_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 418 | `crypto_box_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 419 | `crypto_box_easy` | default portable build; source branch `if (mlen > crypto_box_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:56`) | [x] |
| 420 | `crypto_box_easy_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 421 | `crypto_box_easy_afternm` | default portable build; source branch `if (mlen > crypto_box_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:44`) | [x] |
| 422 | `crypto_box_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 423 | `crypto_box_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 424 | `crypto_box_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 425 | `crypto_box_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 426 | `crypto_box_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 427 | `crypto_box_open_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 428 | `crypto_box_open_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 429 | `crypto_box_open_detached` | default portable build; source branch `if (crypto_box_beforenm(k, pk, sk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:82`) | [x] |
| 430 | `crypto_box_open_detached_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 431 | `crypto_box_open_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 432 | `crypto_box_open_easy` | default portable build; source branch `if (clen < crypto_box_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:109`) | [x] |
| 433 | `crypto_box_open_easy_afternm` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 434 | `crypto_box_open_easy_afternm` | default portable build; source branch `if (clen < crypto_box_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_easy.c:96`) | [x] |
| 435 | `crypto_box_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 436 | `crypto_box_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 437 | `crypto_box_seal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 438 | `crypto_box_seal` | default portable build; source branch `if (mlen > crypto_box_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_seal.c:33`) | [x] |
| 439 | `crypto_box_seal` | default portable build; source branch `if (crypto_box_keypair(epk, esk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_seal.c:36`) | [x] |
| 440 | `crypto_box_seal_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 441 | `crypto_box_seal_open` | default portable build; source branch `if (clen < crypto_box_SEALBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_box/crypto_box_seal.c:55`) | [x] |
| 442 | `crypto_box_sealbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 443 | `crypto_box_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 444 | `crypto_box_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 445 | `crypto_box_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 446 | `crypto_box_zerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 447 | `crypto_core_ed25519_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 448 | `crypto_core_ed25519_add` | default portable build; source branch `if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:34`) | [x] |
| 449 | `crypto_core_ed25519_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 450 | `crypto_core_ed25519_from_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 451 | `crypto_core_ed25519_from_string` | default portable build; source branch `if (_string_to_points(px, 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:108`) | [x] |
| 452 | `crypto_core_ed25519_from_string_nu` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 453 | `crypto_core_ed25519_hashbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 454 | `crypto_core_ed25519_is_valid_point` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 455 | `crypto_core_ed25519_is_valid_point` | default portable build; source branch `if (ge25519_is_canonical(p) == 0 \|\| ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_has_small_order(&p_p3) != 0 \|\| ge25519_is_on_main_subgroup(&p_p3) == 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:18`) | [x] |
| 456 | `crypto_core_ed25519_is_valid_point` | default portable build; source branch `if (ge25519_is_canonical(p) == 0 \|\| ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_has_small_order(&p_p3) != 0 \|\| ge25519_is_on_main_subgroup(&p_p3) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:18`) | [x] |
| 457 | `crypto_core_ed25519_nonreducedscalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 458 | `crypto_core_ed25519_random` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 459 | `crypto_core_ed25519_scalar_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 460 | `crypto_core_ed25519_scalar_complement` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 461 | `crypto_core_ed25519_scalar_from_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 462 | `crypto_core_ed25519_scalar_from_string` | default portable build; source branch `if (core_h2c_string_to_hash(h_be, sizeof h_be, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:249`) | [x] |
| 463 | `crypto_core_ed25519_scalar_invert` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 464 | `crypto_core_ed25519_scalar_is_canonical` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 465 | `crypto_core_ed25519_scalar_mul` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 466 | `crypto_core_ed25519_scalar_negate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 467 | `crypto_core_ed25519_scalar_random` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 468 | `crypto_core_ed25519_scalar_reduce` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 469 | `crypto_core_ed25519_scalar_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 470 | `crypto_core_ed25519_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 471 | `crypto_core_ed25519_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 472 | `crypto_core_ed25519_sub` | default portable build; source branch `if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:50`) | [x] |
| 473 | `crypto_core_ed25519_uniformbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 474 | `crypto_core_hchacha20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 475 | `crypto_core_hchacha20` | default portable build; source branch `if (c == NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/hchacha20/core_hchacha20.c:24`) | [x] |
| 476 | `crypto_core_hchacha20` | default portable build; source branch `if (c == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/hchacha20/core_hchacha20.c:24`) | [x] |
| 477 | `crypto_core_hchacha20_constbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 478 | `crypto_core_hchacha20_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 479 | `crypto_core_hchacha20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 480 | `crypto_core_hchacha20_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 481 | `crypto_core_hsalsa20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 482 | `crypto_core_hsalsa20` | default portable build; source branch `if (c == NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c:26`) | [x] |
| 483 | `crypto_core_hsalsa20` | default portable build; source branch `if (c == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c:26`) | [x] |
| 484 | `crypto_core_hsalsa20_constbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 485 | `crypto_core_hsalsa20_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 486 | `crypto_core_hsalsa20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 487 | `crypto_core_hsalsa20_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 488 | `crypto_core_keccak1600_extract_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 489 | `crypto_core_keccak1600_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 490 | `crypto_core_keccak1600_permute_12` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 491 | `crypto_core_keccak1600_permute_24` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 492 | `crypto_core_keccak1600_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 493 | `crypto_core_keccak1600_xor_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 494 | `crypto_core_ristretto255_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 495 | `crypto_core_ristretto255_add` | default portable build; source branch `if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:32`) | [x] |
| 496 | `crypto_core_ristretto255_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 497 | `crypto_core_ristretto255_from_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 498 | `crypto_core_ristretto255_from_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 499 | `crypto_core_ristretto255_hashbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 500 | `crypto_core_ristretto255_is_valid_point` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 501 | `crypto_core_ristretto255_is_valid_point` | default portable build; source branch `if (ristretto255_frombytes(&p_p3, p) != 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:20`) | [x] |
| 502 | `crypto_core_ristretto255_is_valid_point` | default portable build; source branch `if (ristretto255_frombytes(&p_p3, p) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:20`) | [x] |
| 503 | `crypto_core_ristretto255_nonreducedscalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 504 | `crypto_core_ristretto255_random` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 505 | `crypto_core_ristretto255_scalar_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 506 | `crypto_core_ristretto255_scalar_complement` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 507 | `crypto_core_ristretto255_scalar_from_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 508 | `crypto_core_ristretto255_scalar_invert` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 509 | `crypto_core_ristretto255_scalar_is_canonical` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 510 | `crypto_core_ristretto255_scalar_mul` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 511 | `crypto_core_ristretto255_scalar_negate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 512 | `crypto_core_ristretto255_scalar_random` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 513 | `crypto_core_ristretto255_scalar_reduce` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 514 | `crypto_core_ristretto255_scalar_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 515 | `crypto_core_ristretto255_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 516 | `crypto_core_ristretto255_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 517 | `crypto_core_ristretto255_sub` | default portable build; source branch `if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:48`) | [x] |
| 518 | `crypto_core_salsa20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 519 | `crypto_core_salsa2012` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 520 | `crypto_core_salsa2012_constbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 521 | `crypto_core_salsa2012_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 522 | `crypto_core_salsa2012_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 523 | `crypto_core_salsa2012_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 524 | `crypto_core_salsa208` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 525 | `crypto_core_salsa208_constbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 526 | `crypto_core_salsa208_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 527 | `crypto_core_salsa208_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 528 | `crypto_core_salsa208_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 529 | `crypto_core_salsa20_constbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 530 | `crypto_core_salsa20_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 531 | `crypto_core_salsa20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 532 | `crypto_core_salsa20_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 533 | `crypto_generichash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 534 | `crypto_generichash_blake2b` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 535 | `crypto_generichash_blake2b` | default portable build; source branch `if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:16`) | [x] |
| 536 | `crypto_generichash_blake2b_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 537 | `crypto_generichash_blake2b_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 538 | `crypto_generichash_blake2b_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 539 | `crypto_generichash_blake2b_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 540 | `crypto_generichash_blake2b_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 541 | `crypto_generichash_blake2b_init` | default portable build; source branch `if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:50`) | [x] |
| 542 | `crypto_generichash_blake2b_init` | default portable build; source branch `if (key == NULL \|\| keylen <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:57`) | [x] |
| 543 | `crypto_generichash_blake2b_init` | default portable build; source branch `if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:58`) | [x] |
| 544 | `crypto_generichash_blake2b_init` | default portable build; source branch `} else if (blake2b_init_key((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:61`) | [x] |
| 545 | `crypto_generichash_blake2b_init_salt_personal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 546 | `crypto_generichash_blake2b_init_salt_personal` | default portable build; source branch `if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:74`) | [x] |
| 547 | `crypto_generichash_blake2b_init_salt_personal` | default portable build; source branch `if (key == NULL \|\| keylen <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:80`) | [x] |
| 548 | `crypto_generichash_blake2b_init_salt_personal` | default portable build; source branch `if (blake2b_init_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, salt, personal) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:81`) | [x] |
| 549 | `crypto_generichash_blake2b_init_salt_personal` | default portable build; source branch `} else if (blake2b_init_key_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen, salt, personal) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:85`) | [x] |
| 550 | `crypto_generichash_blake2b_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 551 | `crypto_generichash_blake2b_keybytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 552 | `crypto_generichash_blake2b_keybytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 553 | `crypto_generichash_blake2b_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 554 | `crypto_generichash_blake2b_personalbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 555 | `crypto_generichash_blake2b_salt_personal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 556 | `crypto_generichash_blake2b_salt_personal` | default portable build; source branch `if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:33`) | [x] |
| 557 | `crypto_generichash_blake2b_saltbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 558 | `crypto_generichash_blake2b_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 559 | `crypto_generichash_blake2b_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 560 | `crypto_generichash_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 561 | `crypto_generichash_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 562 | `crypto_generichash_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 563 | `crypto_generichash_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 564 | `crypto_generichash_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 565 | `crypto_generichash_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 566 | `crypto_generichash_keybytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 567 | `crypto_generichash_keybytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 568 | `crypto_generichash_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 569 | `crypto_generichash_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 570 | `crypto_generichash_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 571 | `crypto_generichash_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 572 | `crypto_hash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 573 | `crypto_hash_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 574 | `crypto_hash_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 575 | `crypto_hash_sha256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 576 | `crypto_hash_sha256_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 577 | `crypto_hash_sha256_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 578 | `crypto_hash_sha256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 579 | `crypto_hash_sha256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 580 | `crypto_hash_sha256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 581 | `crypto_hash_sha256_update` | default portable build; source branch `if (inlen <= 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha256/cp/hash_sha256_cp.c:357`) | [x] |
| 582 | `crypto_hash_sha256_update` | default portable build; source branch `if (inlen <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha256/cp/hash_sha256_cp.c:357`) | [x] |
| 583 | `crypto_hash_sha256_update` | default portable build; source branch `if (inlen < 64 - r) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha256/cp/hash_sha256_cp.c:364`) | [x] |
| 584 | `crypto_hash_sha256_update` | default portable build; source branch `if (inlen < 64 - r) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha256/cp/hash_sha256_cp.c:364`) | [x] |
| 585 | `crypto_hash_sha3256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 586 | `crypto_hash_sha3256_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 587 | `crypto_hash_sha3256_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 588 | `crypto_hash_sha3256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 589 | `crypto_hash_sha3256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 590 | `crypto_hash_sha3256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 591 | `crypto_hash_sha3512` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 592 | `crypto_hash_sha3512_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 593 | `crypto_hash_sha3512_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 594 | `crypto_hash_sha3512_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 595 | `crypto_hash_sha3512_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 596 | `crypto_hash_sha3512_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 597 | `crypto_hash_sha512` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 598 | `crypto_hash_sha512_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 599 | `crypto_hash_sha512_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 600 | `crypto_hash_sha512_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 601 | `crypto_hash_sha512_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 602 | `crypto_hash_sha512_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 603 | `crypto_hash_sha512_update` | default portable build; source branch `if (inlen <= 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:219`) | [x] |
| 604 | `crypto_hash_sha512_update` | default portable build; source branch `if (inlen <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:219`) | [x] |
| 605 | `crypto_hash_sha512_update` | default portable build; source branch `if ((state->count[1] += bitlen[1]) < bitlen[1]) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:228`) | [x] |
| 606 | `crypto_hash_sha512_update` | default portable build; source branch `if ((state->count[1] += bitlen[1]) < bitlen[1]) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:228`) | [x] |
| 607 | `crypto_hash_sha512_update` | default portable build; source branch `if (inlen < 128 - r) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:233`) | [x] |
| 608 | `crypto_hash_sha512_update` | default portable build; source branch `if (inlen < 128 - r) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c:233`) | [x] |
| 609 | `crypto_ipcrypt_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 610 | `crypto_ipcrypt_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 611 | `crypto_ipcrypt_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 612 | `crypto_ipcrypt_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 613 | `crypto_ipcrypt_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 614 | `crypto_ipcrypt_nd_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 615 | `crypto_ipcrypt_nd_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 616 | `crypto_ipcrypt_nd_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 617 | `crypto_ipcrypt_nd_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 618 | `crypto_ipcrypt_nd_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 619 | `crypto_ipcrypt_nd_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 620 | `crypto_ipcrypt_nd_tweakbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 621 | `crypto_ipcrypt_ndx_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 622 | `crypto_ipcrypt_ndx_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 623 | `crypto_ipcrypt_ndx_inputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 624 | `crypto_ipcrypt_ndx_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 625 | `crypto_ipcrypt_ndx_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 626 | `crypto_ipcrypt_ndx_outputbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 627 | `crypto_ipcrypt_ndx_tweakbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 628 | `crypto_ipcrypt_pfx_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 629 | `crypto_ipcrypt_pfx_decrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 630 | `crypto_ipcrypt_pfx_encrypt` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 631 | `crypto_ipcrypt_pfx_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 632 | `crypto_ipcrypt_pfx_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 633 | `crypto_kdf_blake2b_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 634 | `crypto_kdf_blake2b_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 635 | `crypto_kdf_blake2b_contextbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 636 | `crypto_kdf_blake2b_derive_from_key` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 637 | `crypto_kdf_blake2b_derive_from_key` | default portable build; source branch `if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| subkey_len > crypto_kdf_blake2b_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/blake2b/kdf_blake2b.c:43`) | [x] |
| 638 | `crypto_kdf_blake2b_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 639 | `crypto_kdf_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 640 | `crypto_kdf_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 641 | `crypto_kdf_contextbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 642 | `crypto_kdf_derive_from_key` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 643 | `crypto_kdf_hkdf_sha256_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 644 | `crypto_kdf_hkdf_sha256_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 645 | `crypto_kdf_hkdf_sha256_expand` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 646 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:65`) | [x] |
| 647 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:72`) | [x] |
| 648 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:72`) | [x] |
| 649 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if ((left = out_len & (crypto_auth_hmacsha256_BYTES - 1U)) != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:83`) | [x] |
| 650 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if ((left = out_len & (crypto_auth_hmacsha256_BYTES - 1U)) != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:83`) | [x] |
| 651 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:85`) | [x] |
| 652 | `crypto_kdf_hkdf_sha256_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:85`) | [x] |
| 653 | `crypto_kdf_hkdf_sha256_extract` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 654 | `crypto_kdf_hkdf_sha256_extract_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 655 | `crypto_kdf_hkdf_sha256_extract_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 656 | `crypto_kdf_hkdf_sha256_extract_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 657 | `crypto_kdf_hkdf_sha256_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 658 | `crypto_kdf_hkdf_sha256_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 659 | `crypto_kdf_hkdf_sha256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 660 | `crypto_kdf_hkdf_sha512_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 661 | `crypto_kdf_hkdf_sha512_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 662 | `crypto_kdf_hkdf_sha512_expand` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 663 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:65`) | [x] |
| 664 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:72`) | [x] |
| 665 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:72`) | [x] |
| 666 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if ((left = out_len & (crypto_auth_hmacsha512_BYTES - 1U)) != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:83`) | [x] |
| 667 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if ((left = out_len & (crypto_auth_hmacsha512_BYTES - 1U)) != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:83`) | [x] |
| 668 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:85`) | [x] |
| 669 | `crypto_kdf_hkdf_sha512_expand` | default portable build; source branch `if (i != (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:85`) | [x] |
| 670 | `crypto_kdf_hkdf_sha512_extract` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 671 | `crypto_kdf_hkdf_sha512_extract_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 672 | `crypto_kdf_hkdf_sha512_extract_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 673 | `crypto_kdf_hkdf_sha512_extract_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 674 | `crypto_kdf_hkdf_sha512_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 675 | `crypto_kdf_hkdf_sha512_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 676 | `crypto_kdf_hkdf_sha512_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 677 | `crypto_kdf_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 678 | `crypto_kdf_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 679 | `crypto_kdf_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 680 | `crypto_kem_ciphertextbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 681 | `crypto_kem_dec` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 682 | `crypto_kem_enc` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 683 | `crypto_kem_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 684 | `crypto_kem_mlkem768_ciphertextbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 685 | `crypto_kem_mlkem768_dec` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 686 | `crypto_kem_mlkem768_enc` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 687 | `crypto_kem_mlkem768_enc_deterministic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 688 | `crypto_kem_mlkem768_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 689 | `crypto_kem_mlkem768_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 690 | `crypto_kem_mlkem768_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 691 | `crypto_kem_mlkem768_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 692 | `crypto_kem_mlkem768_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 693 | `crypto_kem_mlkem768_sharedsecretbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 694 | `crypto_kem_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 695 | `crypto_kem_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 696 | `crypto_kem_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 697 | `crypto_kem_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 698 | `crypto_kem_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 699 | `crypto_kem_sharedsecretbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 700 | `crypto_kem_xwing_ciphertextbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 701 | `crypto_kem_xwing_dec` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 702 | `crypto_kem_xwing_dec` | default portable build; source branch `if (crypto_kem_mlkem768_dec(ss_mlkem, ct_mlkem, sk_mlkem) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:188`) | [x] |
| 703 | `crypto_kem_xwing_dec` | default portable build; source branch `if (crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:194`) | [x] |
| 704 | `crypto_kem_xwing_enc` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 705 | `crypto_kem_xwing_enc` | default portable build; source branch `if (crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:162`) | [x] |
| 706 | `crypto_kem_xwing_enc_deterministic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 707 | `crypto_kem_xwing_enc_deterministic` | default portable build; source branch `if (crypto_kem_mlkem768_enc_deterministic(ct_mlkem, ss_mlkem, pk_mlkem, seed_mlkem) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:134`) | [x] |
| 708 | `crypto_kem_xwing_enc_deterministic` | default portable build; source branch `if (crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:140`) | [x] |
| 709 | `crypto_kem_xwing_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 710 | `crypto_kem_xwing_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 711 | `crypto_kem_xwing_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 712 | `crypto_kem_xwing_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 713 | `crypto_kem_xwing_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 714 | `crypto_kem_xwing_sharedsecretbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 715 | `crypto_kx_client_session_keys` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 716 | `crypto_kx_client_session_keys` | default portable build; source branch `if (rx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:45`) | [x] |
| 717 | `crypto_kx_client_session_keys` | default portable build; source branch `if (tx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:48`) | [x] |
| 718 | `crypto_kx_client_session_keys` | default portable build; source branch `if (rx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:51`) | [x] |
| 719 | `crypto_kx_client_session_keys` | default portable build; source branch `if (crypto_scalarmult(q, client_sk, server_pk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:54`) | [x] |
| 720 | `crypto_kx_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 721 | `crypto_kx_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 722 | `crypto_kx_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 723 | `crypto_kx_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 724 | `crypto_kx_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 725 | `crypto_kx_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 726 | `crypto_kx_server_session_keys` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 727 | `crypto_kx_server_session_keys` | default portable build; source branch `if (rx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:86`) | [x] |
| 728 | `crypto_kx_server_session_keys` | default portable build; source branch `if (tx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:89`) | [x] |
| 729 | `crypto_kx_server_session_keys` | default portable build; source branch `if (rx == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:92`) | [x] |
| 730 | `crypto_kx_server_session_keys` | default portable build; source branch `if (crypto_scalarmult(q, server_sk, client_pk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_kx/crypto_kx.c:95`) | [x] |
| 731 | `crypto_kx_sessionkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 732 | `crypto_onetimeauth` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 733 | `crypto_onetimeauth_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 734 | `crypto_onetimeauth_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 735 | `crypto_onetimeauth_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 736 | `crypto_onetimeauth_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 737 | `crypto_onetimeauth_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 738 | `crypto_onetimeauth_poly1305` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 739 | `crypto_onetimeauth_poly1305_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 740 | `crypto_onetimeauth_poly1305_donna_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 741 | `crypto_onetimeauth_poly1305_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 742 | `crypto_onetimeauth_poly1305_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 743 | `crypto_onetimeauth_poly1305_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 744 | `crypto_onetimeauth_poly1305_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 745 | `crypto_onetimeauth_poly1305_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 746 | `crypto_onetimeauth_poly1305_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 747 | `crypto_onetimeauth_poly1305_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 748 | `crypto_onetimeauth_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 749 | `crypto_onetimeauth_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 750 | `crypto_onetimeauth_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 751 | `crypto_onetimeauth_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 752 | `crypto_pwhash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 753 | `crypto_pwhash` | default portable build; source branch `switch (alg) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:134`) | [x] |
| 754 | `crypto_pwhash_alg_argon2i13` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 755 | `crypto_pwhash_alg_argon2id13` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 756 | `crypto_pwhash_alg_default` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 757 | `crypto_pwhash_argon2i` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 758 | `crypto_pwhash_argon2i` | default portable build; source branch `if (outlen > crypto_pwhash_argon2i_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:146`) | [x] |
| 759 | `crypto_pwhash_argon2i` | default portable build; source branch `if (outlen < crypto_pwhash_argon2i_BYTES_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:150`) | [x] |
| 760 | `crypto_pwhash_argon2i` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:154`) | [x] |
| 761 | `crypto_pwhash_argon2i` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:160`) | [x] |
| 762 | `crypto_pwhash_argon2i` | default portable build; source branch `if ((const void *) out == (const void *) passwd) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:166`) | [x] |
| 763 | `crypto_pwhash_argon2i` | default portable build; source branch `switch (alg) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:170`) | [x] |
| 764 | `crypto_pwhash_argon2i` | default portable build; source branch `if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:172`) | [x] |
| 765 | `crypto_pwhash_argon2i_alg_argon2i13` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 766 | `crypto_pwhash_argon2i_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 767 | `crypto_pwhash_argon2i_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 768 | `crypto_pwhash_argon2i_memlimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 769 | `crypto_pwhash_argon2i_memlimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 770 | `crypto_pwhash_argon2i_memlimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 771 | `crypto_pwhash_argon2i_memlimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 772 | `crypto_pwhash_argon2i_memlimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 773 | `crypto_pwhash_argon2i_opslimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 774 | `crypto_pwhash_argon2i_opslimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 775 | `crypto_pwhash_argon2i_opslimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 776 | `crypto_pwhash_argon2i_opslimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 777 | `crypto_pwhash_argon2i_opslimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 778 | `crypto_pwhash_argon2i_passwd_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 779 | `crypto_pwhash_argon2i_passwd_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 780 | `crypto_pwhash_argon2i_saltbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 781 | `crypto_pwhash_argon2i_str` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 782 | `crypto_pwhash_argon2i_str` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:194`) | [x] |
| 783 | `crypto_pwhash_argon2i_str` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:200`) | [x] |
| 784 | `crypto_pwhash_argon2i_str` | default portable build; source branch `if (argon2i_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2i_STRBYTES) != ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:207`) | [x] |
| 785 | `crypto_pwhash_argon2i_str_needs_rehash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 786 | `crypto_pwhash_argon2i_str_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 787 | `crypto_pwhash_argon2i_str_verify` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:223`) | [x] |
| 788 | `crypto_pwhash_argon2i_str_verify` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:228`) | [x] |
| 789 | `crypto_pwhash_argon2i_str_verify` | default portable build; source branch `if (verify_ret == ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:235`) | [x] |
| 790 | `crypto_pwhash_argon2i_str_verify` | default portable build; source branch `if (verify_ret == ARGON2_VERIFY_MISMATCH) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:238`) | [x] |
| 791 | `crypto_pwhash_argon2i_strbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 792 | `crypto_pwhash_argon2i_strprefix` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 793 | `crypto_pwhash_argon2id` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 794 | `crypto_pwhash_argon2id` | default portable build; source branch `if (outlen > crypto_pwhash_argon2id_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:142`) | [x] |
| 795 | `crypto_pwhash_argon2id` | default portable build; source branch `if (outlen < crypto_pwhash_argon2id_BYTES_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:146`) | [x] |
| 796 | `crypto_pwhash_argon2id` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:150`) | [x] |
| 797 | `crypto_pwhash_argon2id` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:156`) | [x] |
| 798 | `crypto_pwhash_argon2id` | default portable build; source branch `if ((const void *) out == (const void *) passwd) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:162`) | [x] |
| 799 | `crypto_pwhash_argon2id` | default portable build; source branch `switch (alg) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:166`) | [x] |
| 800 | `crypto_pwhash_argon2id` | default portable build; source branch `if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:168`) | [x] |
| 801 | `crypto_pwhash_argon2id_alg_argon2id13` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 802 | `crypto_pwhash_argon2id_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 803 | `crypto_pwhash_argon2id_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 804 | `crypto_pwhash_argon2id_memlimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 805 | `crypto_pwhash_argon2id_memlimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 806 | `crypto_pwhash_argon2id_memlimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 807 | `crypto_pwhash_argon2id_memlimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 808 | `crypto_pwhash_argon2id_memlimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 809 | `crypto_pwhash_argon2id_opslimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 810 | `crypto_pwhash_argon2id_opslimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 811 | `crypto_pwhash_argon2id_opslimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 812 | `crypto_pwhash_argon2id_opslimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 813 | `crypto_pwhash_argon2id_opslimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 814 | `crypto_pwhash_argon2id_passwd_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 815 | `crypto_pwhash_argon2id_passwd_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 816 | `crypto_pwhash_argon2id_saltbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 817 | `crypto_pwhash_argon2id_str` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 818 | `crypto_pwhash_argon2id_str` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:190`) | [x] |
| 819 | `crypto_pwhash_argon2id_str` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:196`) | [x] |
| 820 | `crypto_pwhash_argon2id_str` | default portable build; source branch `if (argon2id_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2id_STRBYTES) != ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:203`) | [x] |
| 821 | `crypto_pwhash_argon2id_str_needs_rehash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 822 | `crypto_pwhash_argon2id_str_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 823 | `crypto_pwhash_argon2id_str_verify` | default portable build; source branch `if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:219`) | [x] |
| 824 | `crypto_pwhash_argon2id_str_verify` | default portable build; source branch `if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:224`) | [x] |
| 825 | `crypto_pwhash_argon2id_str_verify` | default portable build; source branch `if (verify_ret == ARGON2_OK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:231`) | [x] |
| 826 | `crypto_pwhash_argon2id_str_verify` | default portable build; source branch `if (verify_ret == ARGON2_VERIFY_MISMATCH) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:234`) | [x] |
| 827 | `crypto_pwhash_argon2id_strbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 828 | `crypto_pwhash_argon2id_strprefix` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 829 | `crypto_pwhash_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 830 | `crypto_pwhash_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 831 | `crypto_pwhash_memlimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 832 | `crypto_pwhash_memlimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 833 | `crypto_pwhash_memlimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 834 | `crypto_pwhash_memlimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 835 | `crypto_pwhash_memlimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 836 | `crypto_pwhash_opslimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 837 | `crypto_pwhash_opslimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 838 | `crypto_pwhash_opslimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 839 | `crypto_pwhash_opslimit_moderate` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 840 | `crypto_pwhash_opslimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 841 | `crypto_pwhash_passwd_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 842 | `crypto_pwhash_passwd_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 843 | `crypto_pwhash_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 844 | `crypto_pwhash_saltbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 845 | `crypto_pwhash_scryptsalsa208sha256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 846 | `crypto_pwhash_scryptsalsa208sha256` | default portable build; source branch `if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\| outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:170`) | [x] |
| 847 | `crypto_pwhash_scryptsalsa208sha256` | default portable build; source branch `if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:175`) | [x] |
| 848 | `crypto_pwhash_scryptsalsa208sha256` | default portable build; source branch `if ((const void *) out == (const void *) passwd) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:180`) | [x] |
| 849 | `crypto_pwhash_scryptsalsa208sha256_bytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 850 | `crypto_pwhash_scryptsalsa208sha256_bytes_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 851 | `crypto_pwhash_scryptsalsa208sha256_ll` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 852 | `crypto_pwhash_scryptsalsa208sha256_ll` | default portable build; source branch `if (escrypt_init_local(&local)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:253`) | [x] |
| 853 | `crypto_pwhash_scryptsalsa208sha256_ll` | default portable build; source branch `if (escrypt_free_local(&local)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:264`) | [x] |
| 854 | `crypto_pwhash_scryptsalsa208sha256_memlimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 855 | `crypto_pwhash_scryptsalsa208sha256_memlimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 856 | `crypto_pwhash_scryptsalsa208sha256_memlimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 857 | `crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 858 | `crypto_pwhash_scryptsalsa208sha256_opslimit_interactive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 859 | `crypto_pwhash_scryptsalsa208sha256_opslimit_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 860 | `crypto_pwhash_scryptsalsa208sha256_opslimit_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 861 | `crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 862 | `crypto_pwhash_scryptsalsa208sha256_passwd_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 863 | `crypto_pwhash_scryptsalsa208sha256_passwd_min` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 864 | `crypto_pwhash_scryptsalsa208sha256_saltbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 865 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 866 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; source branch `if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:204`) | [x] |
| 867 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; source branch `if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:208`) | [x] |
| 868 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; source branch `if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting, sizeof setting) == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:214`) | [x] |
| 869 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; source branch `if (escrypt_init_local(&escrypt_local) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:219`) | [x] |
| 870 | `crypto_pwhash_scryptsalsa208sha256_str` | default portable build; source branch `if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) setting, (uint8_t *) out, crypto_pwhash_scryptsalsa208sha256_STRBYTES) == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:222`) | [x] |
| 871 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 872 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; source branch `if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:283`) | [x] |
| 873 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; source branch `if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:287`) | [x] |
| 874 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; source branch `if (escrypt_parse_setting((const uint8_t *) str, &N_log2_, &r_, &p_) == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:292`) | [x] |
| 875 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; source branch `if (N_log2 != N_log2_ \|\| r != r_ \|\| p != p_) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:297`) | [x] |
| 876 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | default portable build; source branch `if (N_log2 != N_log2_ \|\| r != r_ \|\| p != p_) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:297`) | [x] |
| 877 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 878 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | default portable build; source branch `if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:253`) | [x] |
| 879 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | default portable build; source branch `if (escrypt_init_local(&escrypt_local) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:257`) | [x] |
| 880 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | default portable build; source branch `if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) str, (uint8_t *) wanted, sizeof wanted) == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:261`) | [x] |
| 881 | `crypto_pwhash_scryptsalsa208sha256_strbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 882 | `crypto_pwhash_scryptsalsa208sha256_strprefix` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 883 | `crypto_pwhash_str` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 884 | `crypto_pwhash_str_alg` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 885 | `crypto_pwhash_str_alg` | default portable build; source branch `switch (alg) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:161`) | [x] |
| 886 | `crypto_pwhash_str_needs_rehash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 887 | `crypto_pwhash_str_needs_rehash` | default portable build; source branch `if (strncmp(str, crypto_pwhash_argon2id_STRPREFIX, sizeof crypto_pwhash_argon2id_STRPREFIX - 1) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:196`) | [x] |
| 888 | `crypto_pwhash_str_needs_rehash` | default portable build; source branch `if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:200`) | [x] |
| 889 | `crypto_pwhash_str_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 890 | `crypto_pwhash_str_verify` | default portable build; source branch `if (strncmp(str, crypto_pwhash_argon2id_STRPREFIX, sizeof crypto_pwhash_argon2id_STRPREFIX - 1) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:179`) | [x] |
| 891 | `crypto_pwhash_str_verify` | default portable build; source branch `if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:183`) | [x] |
| 892 | `crypto_pwhash_strbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 893 | `crypto_pwhash_strprefix` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 894 | `crypto_scalarmult` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 895 | `crypto_scalarmult_base` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 896 | `crypto_scalarmult_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 897 | `crypto_scalarmult_curve25519` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 898 | `crypto_scalarmult_curve25519` | default portable build; source branch `if (implementation->mult(q, n, p) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_scalarmult/curve25519/scalarmult_curve25519.c:21`) | [x] |
| 899 | `crypto_scalarmult_curve25519_base` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 900 | `crypto_scalarmult_curve25519_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 901 | `crypto_scalarmult_curve25519_ref10_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 902 | `crypto_scalarmult_curve25519_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 903 | `crypto_scalarmult_ed25519` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 904 | `crypto_scalarmult_ed25519_base` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 905 | `crypto_scalarmult_ed25519_base_noclamp` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 906 | `crypto_scalarmult_ed25519_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 907 | `crypto_scalarmult_ed25519_noclamp` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 908 | `crypto_scalarmult_ed25519_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 909 | `crypto_scalarmult_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 910 | `crypto_scalarmult_ristretto255` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 911 | `crypto_scalarmult_ristretto255` | default portable build; source branch `if (ristretto255_frombytes(&P, p) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:18`) | [x] |
| 912 | `crypto_scalarmult_ristretto255` | default portable build; source branch `if (sodium_is_zero(q, 32)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:27`) | [x] |
| 913 | `crypto_scalarmult_ristretto255_base` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 914 | `crypto_scalarmult_ristretto255_base` | default portable build; source branch `if (sodium_is_zero(q, 32)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:47`) | [x] |
| 915 | `crypto_scalarmult_ristretto255_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 916 | `crypto_scalarmult_ristretto255_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 917 | `crypto_scalarmult_scalarbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 918 | `crypto_secretbox` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 919 | `crypto_secretbox_boxzerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 920 | `crypto_secretbox_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 921 | `crypto_secretbox_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < mlen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < mlen)) { /* LCOV_EXCL_LINE */` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:40`) | [x] |
| 922 | `crypto_secretbox_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < mlen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < mlen)) { /* LCOV_EXCL_LINE */` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:40`) | [x] |
| 923 | `crypto_secretbox_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_ZEROBYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:50`) | [x] |
| 924 | `crypto_secretbox_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_ZEROBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:50`) | [x] |
| 925 | `crypto_secretbox_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:74`) | [x] |
| 926 | `crypto_secretbox_detached` | default portable build; source branch `if (cl > STREAM_POLY1305_CHUNK) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:74`) | [x] |
| 927 | `crypto_secretbox_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 928 | `crypto_secretbox_easy` | default portable build; source branch `if (mlen > crypto_secretbox_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:97`) | [x] |
| 929 | `crypto_secretbox_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 930 | `crypto_secretbox_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 931 | `crypto_secretbox_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 932 | `crypto_secretbox_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 933 | `crypto_secretbox_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 934 | `crypto_secretbox_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 935 | `crypto_secretbox_open_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 936 | `crypto_secretbox_open_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_ZEROBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:120`) | [x] |
| 937 | `crypto_secretbox_open_detached` | default portable build; source branch `if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:127`) | [x] |
| 938 | `crypto_secretbox_open_detached` | default portable build; source branch `if (m == NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:131`) | [x] |
| 939 | `crypto_secretbox_open_detached` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:131`) | [x] |
| 940 | `crypto_secretbox_open_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < clen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < clen)) { /* LCOV_EXCL_LINE */` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:145`) | [x] |
| 941 | `crypto_secretbox_open_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < clen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < clen)) { /* LCOV_EXCL_LINE */` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:145`) | [x] |
| 942 | `crypto_secretbox_open_detached` | default portable build; source branch `if (clen > mlen0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:156`) | [x] |
| 943 | `crypto_secretbox_open_detached` | default portable build; source branch `if (clen > mlen0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:156`) | [x] |
| 944 | `crypto_secretbox_open_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 945 | `crypto_secretbox_open_easy` | default portable build; source branch `if (clen < crypto_secretbox_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:170`) | [x] |
| 946 | `crypto_secretbox_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 947 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 948 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < mlen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < mlen)) { /* LCOV_EXCL_LINE */` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:42`) | [x] |
| 949 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < mlen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < mlen)) { /* LCOV_EXCL_LINE */` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:42`) | [x] |
| 950 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_xchacha20poly1305_ZEROBYTES) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:52`) | [x] |
| 951 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_xchacha20poly1305_ZEROBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:52`) | [x] |
| 952 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (mlen > mlen0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:69`) | [x] |
| 953 | `crypto_secretbox_xchacha20poly1305_detached` | default portable build; source branch `if (mlen > mlen0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:69`) | [x] |
| 954 | `crypto_secretbox_xchacha20poly1305_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 955 | `crypto_secretbox_xchacha20poly1305_easy` | default portable build; source branch `if (mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:89`) | [x] |
| 956 | `crypto_secretbox_xchacha20poly1305_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 957 | `crypto_secretbox_xchacha20poly1305_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 958 | `crypto_secretbox_xchacha20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 959 | `crypto_secretbox_xchacha20poly1305_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 960 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 961 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (mlen0 > 64U - crypto_secretbox_xchacha20poly1305_ZEROBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:113`) | [x] |
| 962 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:120`) | [x] |
| 963 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (m == NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:124`) | [x] |
| 964 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (m == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:124`) | [x] |
| 965 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < clen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < clen)) { /* LCOV_EXCL_LINE */` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:138`) | [x] |
| 966 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (((uintptr_t) c > (uintptr_t) m && (uintptr_t) c - (uintptr_t) m < clen) \|\| ((uintptr_t) m > (uintptr_t) c && (uintptr_t) m - (uintptr_t) c < clen)) { /* LCOV_EXCL_LINE */` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:138`) | [x] |
| 967 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (clen > mlen0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:148`) | [x] |
| 968 | `crypto_secretbox_xchacha20poly1305_open_detached` | default portable build; source branch `if (clen > mlen0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:148`) | [x] |
| 969 | `crypto_secretbox_xchacha20poly1305_open_easy` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 970 | `crypto_secretbox_xchacha20poly1305_open_easy` | default portable build; source branch `if (clen < crypto_secretbox_xchacha20poly1305_MACBYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:164`) | [x] |
| 971 | `crypto_secretbox_xsalsa20poly1305` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 972 | `crypto_secretbox_xsalsa20poly1305` | default portable build; source branch `if (mlen < 32) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:15`) | [x] |
| 973 | `crypto_secretbox_xsalsa20poly1305_boxzerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 974 | `crypto_secretbox_xsalsa20poly1305_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 975 | `crypto_secretbox_xsalsa20poly1305_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 976 | `crypto_secretbox_xsalsa20poly1305_macbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 977 | `crypto_secretbox_xsalsa20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 978 | `crypto_secretbox_xsalsa20poly1305_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 979 | `crypto_secretbox_xsalsa20poly1305_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 980 | `crypto_secretbox_xsalsa20poly1305_open` | default portable build; source branch `if (clen < 32) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:35`) | [x] |
| 981 | `crypto_secretbox_xsalsa20poly1305_open` | default portable build; source branch `if (crypto_onetimeauth_poly1305_verify(c + 16, c + 32, clen - 32, subkey) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:39`) | [x] |
| 982 | `crypto_secretbox_xsalsa20poly1305_zerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 983 | `crypto_secretbox_zerobytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 984 | `crypto_secretstream_xchacha20poly1305_abytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 985 | `crypto_secretstream_xchacha20poly1305_headerbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 986 | `crypto_secretstream_xchacha20poly1305_init_pull` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 987 | `crypto_secretstream_xchacha20poly1305_init_push` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 988 | `crypto_secretstream_xchacha20poly1305_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 989 | `crypto_secretstream_xchacha20poly1305_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 990 | `crypto_secretstream_xchacha20poly1305_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 991 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 992 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:195`) | [x] |
| 993 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (tag_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:198`) | [x] |
| 994 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (inlen < crypto_secretstream_xchacha20poly1305_ABYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:201`) | [x] |
| 995 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:205`) | [x] |
| 996 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (sodium_memcmp(mac, stored_mac, sizeof mac) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:239`) | [x] |
| 997 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if ((tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0 \|\| sodium_is_zero(STATE_COUNTER(state), crypto_secretstream_xchacha20poly1305_COUNTERBYTES)) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:250`) | [x] |
| 998 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if ((tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0 \|\| sodium_is_zero(STATE_COUNTER(state), crypto_secretstream_xchacha20poly1305_COUNTERBYTES)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:250`) | [x] |
| 999 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:255`) | [x] |
| 1000 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:255`) | [x] |
| 1001 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (tag_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:258`) | [x] |
| 1002 | `crypto_secretstream_xchacha20poly1305_pull` | default portable build; source branch `if (tag_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:258`) | [x] |
| 1003 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1004 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if (outlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:123`) | [x] |
| 1005 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:128`) | [x] |
| 1006 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if ((tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0 \|\| sodium_is_zero(STATE_COUNTER(state), crypto_secretstream_xchacha20poly1305_COUNTERBYTES)) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:168`) | [x] |
| 1007 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if ((tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0 \|\| sodium_is_zero(STATE_COUNTER(state), crypto_secretstream_xchacha20poly1305_COUNTERBYTES)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:168`) | [x] |
| 1008 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if (outlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:173`) | [x] |
| 1009 | `crypto_secretstream_xchacha20poly1305_push` | default portable build; source branch `if (outlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:173`) | [x] |
| 1010 | `crypto_secretstream_xchacha20poly1305_rekey` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1011 | `crypto_secretstream_xchacha20poly1305_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1012 | `crypto_secretstream_xchacha20poly1305_tag_final` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1013 | `crypto_secretstream_xchacha20poly1305_tag_message` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1014 | `crypto_secretstream_xchacha20poly1305_tag_push` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1015 | `crypto_secretstream_xchacha20poly1305_tag_rekey` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1016 | `crypto_shorthash` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1017 | `crypto_shorthash_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1018 | `crypto_shorthash_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1019 | `crypto_shorthash_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1020 | `crypto_shorthash_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1021 | `crypto_shorthash_siphash24` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1022 | `crypto_shorthash_siphash24` | default portable build; source branch `switch (left) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c:33`) | [x] |
| 1023 | `crypto_shorthash_siphash24` | default portable build; source branch `switch (left) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c:33`) | [x] |
| 1024 | `crypto_shorthash_siphash24_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1025 | `crypto_shorthash_siphash24_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1026 | `crypto_shorthash_siphashx24` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1027 | `crypto_shorthash_siphashx24` | default portable build; source branch `switch (left) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c:32`) | [x] |
| 1028 | `crypto_shorthash_siphashx24` | default portable build; source branch `switch (left) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c:32`) | [x] |
| 1029 | `crypto_shorthash_siphashx24_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1030 | `crypto_shorthash_siphashx24_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1031 | `crypto_sign` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1032 | `crypto_sign_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1033 | `crypto_sign_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1034 | `crypto_sign_ed25519` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1035 | `crypto_sign_ed25519` | default portable build; source branch `if (crypto_sign_ed25519_detached( sm, &siglen, sm + crypto_sign_ed25519_BYTES, mlen, sk) != 0 \|\| siglen != crypto_sign_ed25519_BYTES) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:113`) | [x] |
| 1036 | `crypto_sign_ed25519` | default portable build; source branch `if (smlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:116`) | [x] |
| 1037 | `crypto_sign_ed25519` | default portable build; source branch `if (smlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:124`) | [x] |
| 1038 | `crypto_sign_ed25519` | default portable build; source branch `if (smlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:124`) | [x] |
| 1039 | `crypto_sign_ed25519_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1040 | `crypto_sign_ed25519_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1041 | `crypto_sign_ed25519_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1042 | `crypto_sign_ed25519_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1043 | `crypto_sign_ed25519_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1044 | `crypto_sign_ed25519_open` | default portable build; source branch `if (smlen < 64 \|\| smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:81`) | [x] |
| 1045 | `crypto_sign_ed25519_open` | default portable build; source branch `if (smlen < 64 \|\| smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:81`) | [x] |
| 1046 | `crypto_sign_ed25519_open` | default portable build; source branch `if (crypto_sign_ed25519_verify_detached(sm, sm + 64, mlen, pk) != 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:85`) | [x] |
| 1047 | `crypto_sign_ed25519_open` | default portable build; source branch `if (crypto_sign_ed25519_verify_detached(sm, sm + 64, mlen, pk) != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:85`) | [x] |
| 1048 | `crypto_sign_ed25519_open` | default portable build; source branch `if (m != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:86`) | [x] |
| 1049 | `crypto_sign_ed25519_open` | default portable build; source branch `if (m != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:86`) | [x] |
| 1050 | `crypto_sign_ed25519_open` | default portable build; source branch `if (mlen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:91`) | [x] |
| 1051 | `crypto_sign_ed25519_open` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:91`) | [x] |
| 1052 | `crypto_sign_ed25519_open` | default portable build; source branch `if (m != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:94`) | [x] |
| 1053 | `crypto_sign_ed25519_open` | default portable build; source branch `if (mlen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:100`) | [x] |
| 1054 | `crypto_sign_ed25519_pk_to_curve25519` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1055 | `crypto_sign_ed25519_pk_to_curve25519` | default portable build; source branch `if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 \|\| ge25519_has_small_order(&A) != 0 \|\| ge25519_is_on_main_subgroup(&A) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_sign/ed25519/ref10/keypair.c:53`) | [x] |
| 1056 | `crypto_sign_ed25519_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1057 | `crypto_sign_ed25519_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1058 | `crypto_sign_ed25519_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1059 | `crypto_sign_ed25519_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1060 | `crypto_sign_ed25519_sk_to_curve25519` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1061 | `crypto_sign_ed25519_sk_to_pk` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1062 | `crypto_sign_ed25519_sk_to_seed` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1063 | `crypto_sign_ed25519_verify_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1064 | `crypto_sign_ed25519ph_final_create` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1065 | `crypto_sign_ed25519ph_final_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1066 | `crypto_sign_ed25519ph_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1067 | `crypto_sign_ed25519ph_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1068 | `crypto_sign_ed25519ph_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1069 | `crypto_sign_final_create` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1070 | `crypto_sign_final_verify` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1071 | `crypto_sign_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1072 | `crypto_sign_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1073 | `crypto_sign_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1074 | `crypto_sign_open` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1075 | `crypto_sign_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1076 | `crypto_sign_publickeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1077 | `crypto_sign_secretkeybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1078 | `crypto_sign_seed_keypair` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1079 | `crypto_sign_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1080 | `crypto_sign_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1081 | `crypto_sign_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1082 | `crypto_sign_verify_detached` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1083 | `crypto_stream` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1084 | `crypto_stream_chacha20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1085 | `crypto_stream_chacha20` | default portable build; source branch `if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:67`) | [x] |
| 1086 | `crypto_stream_chacha20_ietf` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1087 | `crypto_stream_chacha20_ietf` | default portable build; source branch `if (clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:133`) | [x] |
| 1088 | `crypto_stream_chacha20_ietf_ext` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1089 | `crypto_stream_chacha20_ietf_ext` | default portable build; source branch `if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:100`) | [x] |
| 1090 | `crypto_stream_chacha20_ietf_ext_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1091 | `crypto_stream_chacha20_ietf_ext_xor_ic` | default portable build; source branch `if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:112`) | [x] |
| 1092 | `crypto_stream_chacha20_ietf_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1093 | `crypto_stream_chacha20_ietf_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1094 | `crypto_stream_chacha20_ietf_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1095 | `crypto_stream_chacha20_ietf_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1096 | `crypto_stream_chacha20_ietf_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1097 | `crypto_stream_chacha20_ietf_xor` | default portable build; source branch `if (mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:157`) | [x] |
| 1098 | `crypto_stream_chacha20_ietf_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1099 | `crypto_stream_chacha20_ietf_xor_ic` | default portable build; source branch `if ((unsigned long long) ic > (64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:145`) | [x] |
| 1100 | `crypto_stream_chacha20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1101 | `crypto_stream_chacha20_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1102 | `crypto_stream_chacha20_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1103 | `crypto_stream_chacha20_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1104 | `crypto_stream_chacha20_ref_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 1105 | `crypto_stream_chacha20_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1106 | `crypto_stream_chacha20_xor` | default portable build; source branch `if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:90`) | [x] |
| 1107 | `crypto_stream_chacha20_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1108 | `crypto_stream_chacha20_xor_ic` | default portable build; source branch `if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c:79`) | [x] |
| 1109 | `crypto_stream_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1110 | `crypto_stream_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1111 | `crypto_stream_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1112 | `crypto_stream_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1113 | `crypto_stream_primitive` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1114 | `crypto_stream_salsa20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1115 | `crypto_stream_salsa2012` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1116 | `crypto_stream_salsa2012` | default portable build; source branch `if (!clen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:23`) | [x] |
| 1117 | `crypto_stream_salsa2012` | default portable build; source branch `if (!clen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:23`) | [x] |
| 1118 | `crypto_stream_salsa2012` | default portable build; source branch `if (clen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:46`) | [x] |
| 1119 | `crypto_stream_salsa2012` | default portable build; source branch `if (clen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:46`) | [x] |
| 1120 | `crypto_stream_salsa2012_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1121 | `crypto_stream_salsa2012_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1122 | `crypto_stream_salsa2012_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1123 | `crypto_stream_salsa2012_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1124 | `crypto_stream_salsa2012_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1125 | `crypto_stream_salsa2012_xor` | default portable build; source branch `if (!mlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:69`) | [x] |
| 1126 | `crypto_stream_salsa2012_xor` | default portable build; source branch `if (!mlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:69`) | [x] |
| 1127 | `crypto_stream_salsa2012_xor` | default portable build; source branch `if (mlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:96`) | [x] |
| 1128 | `crypto_stream_salsa2012_xor` | default portable build; source branch `if (mlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:96`) | [x] |
| 1129 | `crypto_stream_salsa208` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1130 | `crypto_stream_salsa208` | default portable build; source branch `if (!clen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:25`) | [x] |
| 1131 | `crypto_stream_salsa208` | default portable build; source branch `if (!clen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:25`) | [x] |
| 1132 | `crypto_stream_salsa208` | default portable build; source branch `if (clen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:48`) | [x] |
| 1133 | `crypto_stream_salsa208` | default portable build; source branch `if (clen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:48`) | [x] |
| 1134 | `crypto_stream_salsa208_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1135 | `crypto_stream_salsa208_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1136 | `crypto_stream_salsa208_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1137 | `crypto_stream_salsa208_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1138 | `crypto_stream_salsa208_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1139 | `crypto_stream_salsa208_xor` | default portable build; source branch `if (!mlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:71`) | [x] |
| 1140 | `crypto_stream_salsa208_xor` | default portable build; source branch `if (!mlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:71`) | [x] |
| 1141 | `crypto_stream_salsa208_xor` | default portable build; source branch `if (mlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:98`) | [x] |
| 1142 | `crypto_stream_salsa208_xor` | default portable build; source branch `if (mlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/crypto_stream/salsa208/ref/stream_salsa208_ref.c:98`) | [x] |
| 1143 | `crypto_stream_salsa20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1144 | `crypto_stream_salsa20_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1145 | `crypto_stream_salsa20_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1146 | `crypto_stream_salsa20_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1147 | `crypto_stream_salsa20_ref_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 1148 | `crypto_stream_salsa20_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1149 | `crypto_stream_salsa20_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1150 | `crypto_stream_xchacha20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1151 | `crypto_stream_xchacha20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1152 | `crypto_stream_xchacha20_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1153 | `crypto_stream_xchacha20_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1154 | `crypto_stream_xchacha20_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1155 | `crypto_stream_xchacha20_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1156 | `crypto_stream_xchacha20_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1157 | `crypto_stream_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1158 | `crypto_stream_xsalsa20` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1159 | `crypto_stream_xsalsa20_keybytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1160 | `crypto_stream_xsalsa20_keygen` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1161 | `crypto_stream_xsalsa20_messagebytes_max` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1162 | `crypto_stream_xsalsa20_noncebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1163 | `crypto_stream_xsalsa20_xor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1164 | `crypto_stream_xsalsa20_xor_ic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1165 | `crypto_verify_16` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1166 | `crypto_verify_16_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1167 | `crypto_verify_32` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1168 | `crypto_verify_32_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1169 | `crypto_verify_64` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1170 | `crypto_verify_64_bytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1171 | `crypto_xof_shake128` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1172 | `crypto_xof_shake128_blockbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1173 | `crypto_xof_shake128_domain_standard` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1174 | `crypto_xof_shake128_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1175 | `crypto_xof_shake128_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1176 | `crypto_xof_shake128_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1177 | `crypto_xof_shake128_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1178 | `crypto_xof_shake128_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1179 | `crypto_xof_shake256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1180 | `crypto_xof_shake256_blockbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1181 | `crypto_xof_shake256_domain_standard` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1182 | `crypto_xof_shake256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1183 | `crypto_xof_shake256_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1184 | `crypto_xof_shake256_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1185 | `crypto_xof_shake256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1186 | `crypto_xof_shake256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1187 | `crypto_xof_turboshake128` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1188 | `crypto_xof_turboshake128_blockbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1189 | `crypto_xof_turboshake128_domain_standard` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1190 | `crypto_xof_turboshake128_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1191 | `crypto_xof_turboshake128_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1192 | `crypto_xof_turboshake128_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1193 | `crypto_xof_turboshake128_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1194 | `crypto_xof_turboshake128_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1195 | `crypto_xof_turboshake256` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1196 | `crypto_xof_turboshake256_blockbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1197 | `crypto_xof_turboshake256_domain_standard` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1198 | `crypto_xof_turboshake256_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1199 | `crypto_xof_turboshake256_init_with_domain` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1200 | `crypto_xof_turboshake256_squeeze` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1201 | `crypto_xof_turboshake256_statebytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1202 | `crypto_xof_turboshake256_update` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1203 | `ipcrypt_soft_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 1204 | `randombytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1205 | `randombytes_buf` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1206 | `randombytes_buf` | default portable build; source branch `if (size > (size_t) 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:205`) | [x] |
| 1207 | `randombytes_buf` | default portable build; source branch `if (size > (size_t) 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:205`) | [x] |
| 1208 | `randombytes_buf_deterministic` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1209 | `randombytes_buf_deterministic` | default portable build; source branch `if (size > 0x4000000000ULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:221`) | [x] |
| 1210 | `randombytes_close` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1211 | `randombytes_close` | default portable build; source branch `if (implementation != NULL && implementation->close != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:238`) | [x] |
| 1212 | `randombytes_implementation_name` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1213 | `randombytes_internal_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 1214 | `randombytes_random` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1215 | `randombytes_seedbytes` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1216 | `randombytes_set_implementation` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1217 | `randombytes_stir` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1218 | `randombytes_stir` | default portable build; source branch `if (implementation->stir != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:173`) | [x] |
| 1219 | `randombytes_stir` | default portable build; source branch `if (implementation->stir != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:173`) | [x] |
| 1220 | `randombytes_sysrandom_implementation` | default portable build; exported data object initialization and ABI bytes | [x] |
| 1221 | `randombytes_uniform` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1222 | `randombytes_uniform` | default portable build; source branch `if (implementation->uniform != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:185`) | [x] |
| 1223 | `randombytes_uniform` | default portable build; source branch `if (implementation->uniform != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:185`) | [x] |
| 1224 | `randombytes_uniform` | default portable build; source branch `if (upper_bound < 2) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:188`) | [x] |
| 1225 | `randombytes_uniform` | default portable build; source branch `if (upper_bound < 2) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/randombytes/randombytes.c:188`) | [x] |
| 1226 | `sodium_add` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1227 | `sodium_allocarray` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1228 | `sodium_allocarray` | default portable build; source branch `if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:655`) | [x] |
| 1229 | `sodium_base642bin` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1230 | `sodium_base642bin` | default portable build; source branch `if (is_urlsafe) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:294`) | [x] |
| 1231 | `sodium_base642bin` | default portable build; source branch `if (is_urlsafe) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:294`) | [x] |
| 1232 | `sodium_base642bin` | default portable build; source branch `if (d == 0xFF) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:299`) | [x] |
| 1233 | `sodium_base642bin` | default portable build; source branch `if (d == 0xFF) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:299`) | [x] |
| 1234 | `sodium_base642bin` | default portable build; source branch `if (ignore != NULL && strchr(ignore, c) != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:300`) | [x] |
| 1235 | `sodium_base642bin` | default portable build; source branch `if (ignore != NULL && strchr(ignore, c) != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:300`) | [x] |
| 1236 | `sodium_base642bin` | default portable build; source branch `if (acc_len >= 8) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:308`) | [x] |
| 1237 | `sodium_base642bin` | default portable build; source branch `if (acc_len >= 8) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:308`) | [x] |
| 1238 | `sodium_base642bin` | default portable build; source branch `if (bin_pos >= bin_maxlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:310`) | [x] |
| 1239 | `sodium_base642bin` | default portable build; source branch `if (bin_pos >= bin_maxlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:310`) | [x] |
| 1240 | `sodium_base642bin` | default portable build; source branch `if (acc_len > 4U \|\| (acc & ((1U << acc_len) - 1U)) != 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:319`) | [x] |
| 1241 | `sodium_base642bin` | default portable build; source branch `if (acc_len > 4U \|\| (acc & ((1U << acc_len) - 1U)) != 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:319`) | [x] |
| 1242 | `sodium_base642bin` | default portable build; source branch `} else if (ret == 0 && (((unsigned int) variant) & VARIANT_NO_PADDING_MASK) == 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:321`) | [x] |
| 1243 | `sodium_base642bin` | default portable build; source branch `} else if (ret == 0 && (((unsigned int) variant) & VARIANT_NO_PADDING_MASK) == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:321`) | [x] |
| 1244 | `sodium_base642bin` | default portable build; source branch `if (ret != 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:326`) | [x] |
| 1245 | `sodium_base642bin` | default portable build; source branch `if (ret != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:326`) | [x] |
| 1246 | `sodium_base642bin` | default portable build; source branch `} else if (ignore != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:328`) | [x] |
| 1247 | `sodium_base642bin` | default portable build; source branch `} else if (ignore != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:328`) | [x] |
| 1248 | `sodium_base642bin` | default portable build; source branch `if (b64_end != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:333`) | [x] |
| 1249 | `sodium_base642bin` | default portable build; source branch `if (b64_end != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:333`) | [x] |
| 1250 | `sodium_base642bin` | default portable build; source branch `} else if (b64_pos != b64_len) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:335`) | [x] |
| 1251 | `sodium_base642bin` | default portable build; source branch `} else if (b64_pos != b64_len) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:335`) | [x] |
| 1252 | `sodium_base642bin` | default portable build; source branch `if (bin_len != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:339`) | [x] |
| 1253 | `sodium_base642bin` | default portable build; source branch `if (bin_len != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:339`) | [x] |
| 1254 | `sodium_base64_encoded_len` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1255 | `sodium_base64_encoded_len` | default portable build; source branch `if (bin_len / 3 > (SIZE_MAX - 5) / 4) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:178`) | [x] |
| 1256 | `sodium_bin2base64` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1257 | `sodium_bin2base64` | default portable build; source branch `if (nibbles > (SIZE_MAX - 5) / 4) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:199`) | [x] |
| 1258 | `sodium_bin2base64` | default portable build; source branch `if (remainder != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:204`) | [x] |
| 1259 | `sodium_bin2base64` | default portable build; source branch `if ((((unsigned int) variant) & VARIANT_NO_PADDING_MASK) == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:205`) | [x] |
| 1260 | `sodium_bin2base64` | default portable build; source branch `if (b64_maxlen <= b64_len) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:211`) | [x] |
| 1261 | `sodium_bin2base64` | default portable build; source branch `if ((((unsigned int) variant) & VARIANT_URLSAFE_MASK) != 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:214`) | [x] |
| 1262 | `sodium_bin2base64` | default portable build; source branch `if ((((unsigned int) variant) & VARIANT_URLSAFE_MASK) != 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:214`) | [x] |
| 1263 | `sodium_bin2base64` | default portable build; source branch `if (acc_len > 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:223`) | [x] |
| 1264 | `sodium_bin2base64` | default portable build; source branch `if (acc_len > 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:223`) | [x] |
| 1265 | `sodium_bin2base64` | default portable build; source branch `if (acc_len > 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:235`) | [x] |
| 1266 | `sodium_bin2hex` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1267 | `sodium_bin2hex` | default portable build; source branch `if (bin_len >= SIZE_MAX / 2 \|\| hex_maxlen <= bin_len * 2U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:23`) | [x] |
| 1268 | `sodium_bin2ip` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1269 | `sodium_bin2ip` | default portable build; source branch `if (ip_maxlen <= 2U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:561`) | [x] |
| 1270 | `sodium_bin2ip` | default portable build; source branch `if (memcmp(bin, ipv4_mapped_prefix, 12U) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:564`) | [x] |
| 1271 | `sodium_bin2ip` | default portable build; source branch `if (i != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:566`) | [x] |
| 1272 | `sodium_bin2ip` | default portable build; source branch `if (len >= ip_maxlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:572`) | [x] |
| 1273 | `sodium_bin2ip` | default portable build; source branch `if (word == 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:583`) | [x] |
| 1274 | `sodium_bin2ip` | default portable build; source branch `if (word == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:583`) | [x] |
| 1275 | `sodium_bin2ip` | default portable build; source branch `if (cur_start < 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:584`) | [x] |
| 1276 | `sodium_bin2ip` | default portable build; source branch `if (cur_start < 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:584`) | [x] |
| 1277 | `sodium_bin2ip` | default portable build; source branch `if (cur_len > best_len) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:589`) | [x] |
| 1278 | `sodium_bin2ip` | default portable build; source branch `if (cur_len > best_len) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:589`) | [x] |
| 1279 | `sodium_bin2ip` | default portable build; source branch `if (cur_len > best_len) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:597`) | [x] |
| 1280 | `sodium_bin2ip` | default portable build; source branch `if (cur_len > best_len) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:597`) | [x] |
| 1281 | `sodium_bin2ip` | default portable build; source branch `if (best_len < 2) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:601`) | [x] |
| 1282 | `sodium_bin2ip` | default portable build; source branch `if (best_len < 2) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:601`) | [x] |
| 1283 | `sodium_bin2ip` | default portable build; source branch `if (i == best_start) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:605`) | [x] |
| 1284 | `sodium_bin2ip` | default portable build; source branch `if (i == best_start) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:605`) | [x] |
| 1285 | `sodium_bin2ip` | default portable build; source branch `if (i != 0 && (best_start < 0 \|\| i != best_start + best_len)) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:611`) | [x] |
| 1286 | `sodium_bin2ip` | default portable build; source branch `if (len >= ip_maxlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:617`) | [x] |
| 1287 | `sodium_compare` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1288 | `sodium_crit_enter` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1289 | `sodium_crit_enter` | default portable build; source branch `if (_sodium_crit_init() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:87`) | [x] |
| 1290 | `sodium_crit_enter` | default portable build; source branch `if ((ret = pthread_mutex_lock(&_sodium_lock)) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:121`) | [x] |
| 1291 | `sodium_crit_leave` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1292 | `sodium_crit_leave` | default portable build; source branch `if (locked == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:100`) | [x] |
| 1293 | `sodium_crit_leave` | default portable build; source branch `if (locked == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:131`) | [x] |
| 1294 | `sodium_free` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1295 | `sodium_hex2bin` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1296 | `sodium_hex2bin` | default portable build; source branch `if ((c_num0 \| c_alpha0) == 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:63`) | [x] |
| 1297 | `sodium_hex2bin` | default portable build; source branch `if ((c_num0 \| c_alpha0) == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:63`) | [x] |
| 1298 | `sodium_hex2bin` | default portable build; source branch `if (ignore != NULL && state == 0U && strchr(ignore, c) != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:64`) | [x] |
| 1299 | `sodium_hex2bin` | default portable build; source branch `if (ignore != NULL && state == 0U && strchr(ignore, c) != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:64`) | [x] |
| 1300 | `sodium_hex2bin` | default portable build; source branch `if (bin_pos >= bin_maxlen) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:71`) | [x] |
| 1301 | `sodium_hex2bin` | default portable build; source branch `if (bin_pos >= bin_maxlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:71`) | [x] |
| 1302 | `sodium_hex2bin` | default portable build; source branch `if (state == 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:76`) | [x] |
| 1303 | `sodium_hex2bin` | default portable build; source branch `if (state == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:76`) | [x] |
| 1304 | `sodium_hex2bin` | default portable build; source branch `if (state != 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:84`) | [x] |
| 1305 | `sodium_hex2bin` | default portable build; source branch `if (state != 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:84`) | [x] |
| 1306 | `sodium_hex2bin` | default portable build; source branch `if (ret != 0) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:89`) | [x] |
| 1307 | `sodium_hex2bin` | default portable build; source branch `if (ret != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:89`) | [x] |
| 1308 | `sodium_hex2bin` | default portable build; source branch `if (hex_end != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:92`) | [x] |
| 1309 | `sodium_hex2bin` | default portable build; source branch `if (hex_end != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:92`) | [x] |
| 1310 | `sodium_hex2bin` | default portable build; source branch `} else if (hex_pos != hex_len) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:94`) | [x] |
| 1311 | `sodium_hex2bin` | default portable build; source branch `} else if (hex_pos != hex_len) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:94`) | [x] |
| 1312 | `sodium_hex2bin` | default portable build; source branch `if (bin_len != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:98`) | [x] |
| 1313 | `sodium_hex2bin` | default portable build; source branch `if (bin_len != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:98`) | [x] |
| 1314 | `sodium_increment` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1315 | `sodium_init` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1316 | `sodium_init` | default portable build; source branch `if (sodium_crit_enter() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:30`) | [x] |
| 1317 | `sodium_init` | default portable build; source branch `if (initialized != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:33`) | [x] |
| 1318 | `sodium_init` | default portable build; source branch `if (sodium_crit_leave() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:34`) | [x] |
| 1319 | `sodium_init` | default portable build; source branch `if (sodium_crit_leave() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:52`) | [x] |
| 1320 | `sodium_ip2bin` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1321 | `sodium_ip2bin` | default portable build; source branch `if (zone != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:498`) | [x] |
| 1322 | `sodium_ip2bin` | default portable build; source branch `if (!((*z >= '0' && *z <= '9') \|\| (*z >= 'a' && *z <= 'z') \|\| (*z >= 'A' && *z <= 'Z') \|\| *z == '-' \|\| *z == '_' \|\| *z == '.')) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:500`) | [x] |
| 1323 | `sodium_ip2bin` | default portable build; source branch `if (zone + 1 >= end) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:505`) | [x] |
| 1324 | `sodium_ip2bin` | default portable build; source branch `if (zone != NULL && !is_ipv6) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:511`) | [x] |
| 1325 | `sodium_ip2bin` | default portable build; source branch `if (is_ipv6) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:514`) | [x] |
| 1326 | `sodium_ip2bin` | default portable build; source branch `if (parse_ipv4(ip, end, v4) == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/codecs.c:517`) | [x] |
| 1327 | `sodium_is_zero` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1328 | `sodium_library_minimal` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1329 | `sodium_library_version_major` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1330 | `sodium_library_version_minor` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1331 | `sodium_malloc` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1332 | `sodium_malloc` | default portable build; source branch `if ((ptr = _sodium_malloc(size)) == NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:644`) | [x] |
| 1333 | `sodium_memcmp` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1334 | `sodium_memzero` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1335 | `sodium_memzero` | default portable build; source branch `if (len > 0U) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:141`) | [x] |
| 1336 | `sodium_memzero` | default portable build; source branch `if (len > 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:141`) | [x] |
| 1337 | `sodium_misuse` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1338 | `sodium_misuse` | default portable build; source branch `if (sodium_crit_enter() == 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:197`) | [x] |
| 1339 | `sodium_misuse` | default portable build; source branch `if (sodium_crit_leave() == 0 && handler != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:199`) | [x] |
| 1340 | `sodium_mlock` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1341 | `sodium_mprotect_noaccess` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1342 | `sodium_mprotect_readonly` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1343 | `sodium_mprotect_readwrite` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1344 | `sodium_munlock` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1345 | `sodium_pad` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1346 | `sodium_pad` | default portable build; source branch `if (blocksize <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:755`) | [x] |
| 1347 | `sodium_pad` | default portable build; source branch `if ((blocksize & (blocksize - 1U)) == 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:759`) | [x] |
| 1348 | `sodium_pad` | default portable build; source branch `if ((size_t) SIZE_MAX - unpadded_buflen <= xpadlen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:764`) | [x] |
| 1349 | `sodium_pad` | default portable build; source branch `if (xpadded_len >= max_buflen) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:768`) | [x] |
| 1350 | `sodium_pad` | default portable build; source branch `if (padded_buflen_p != NULL) {` evaluates true; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:772`) | [x] |
| 1351 | `sodium_pad` | default portable build; source branch `if (padded_buflen_p != NULL) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:772`) | [x] |
| 1352 | `sodium_runtime_has_aesni` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1353 | `sodium_runtime_has_armcrypto` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1354 | `sodium_runtime_has_avx` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1355 | `sodium_runtime_has_avx2` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1356 | `sodium_runtime_has_avx512f` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1357 | `sodium_runtime_has_neon` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1358 | `sodium_runtime_has_pclmul` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1359 | `sodium_runtime_has_rdrand` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1360 | `sodium_runtime_has_sse2` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1361 | `sodium_runtime_has_sse3` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1362 | `sodium_runtime_has_sse41` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1363 | `sodium_runtime_has_ssse3` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1364 | `sodium_set_misuse_handler` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1365 | `sodium_set_misuse_handler` | default portable build; source branch `if (sodium_crit_enter() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:211`) | [x] |
| 1366 | `sodium_set_misuse_handler` | default portable build; source branch `if (sodium_crit_leave() != 0) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/core.c:215`) | [x] |
| 1367 | `sodium_stackzero` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1368 | `sodium_sub` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1369 | `sodium_unpad` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
| 1370 | `sodium_unpad` | default portable build; source branch `if (padded_buflen < blocksize \|\| blocksize <= 0U) {` evaluates false; valid boundary-shaped inputs (`c_src/libsodium/sodium/utils.c:797`) | [x] |
| 1371 | `sodium_version_string` | default portable build; randomized valid inputs including empty, one, many, and documented boundaries | [x] |
