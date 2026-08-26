# Error Surface

Mechanically derived from every C `return -1`, `return NULL`, Argon2 error-enum return,
`assert`, `sodium_misuse`, and `abort` rejection statement. The controlling source
condition and exact source location are retained in each row.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---:|----------|---------------------------------------------|-------------------|:---:|
| 1 | `crypto_aead_aegis128l_decrypt_detached` | if (clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) { (`c_src/libsodium/crypto_aead/aegis128l/aead_aegis128l.c:139`) | `-1` | [x] |
| 2 | `crypto_aead_aegis256_decrypt_detached` | if (clen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) { (`c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c:138`) | `-1` | [x] |
| 3 | `crypto_aead_aes256gcm_encrypt_detached` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:65`) | `-1` | [x] |
| 4 | `crypto_aead_aes256gcm_encrypt` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:75`) | `-1` | [x] |
| 5 | `crypto_aead_aes256gcm_decrypt_detached` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:86`) | `-1` | [x] |
| 6 | `crypto_aead_aes256gcm_decrypt` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:96`) | `-1` | [x] |
| 7 | `crypto_aead_aes256gcm_beforenm` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:103`) | `-1` | [x] |
| 8 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:115`) | `-1` | [x] |
| 9 | `crypto_aead_aes256gcm_encrypt_afternm` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:126`) | `-1` | [x] |
| 10 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:137`) | `-1` | [x] |
| 11 | `crypto_aead_aes256gcm_decrypt_afternm` | unconditional rejection at the cited source location (`c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:148`) | `-1` | [x] |
| 12 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:762`) | `-1` | [x] |
| 13 | `crypto_aead_aes256gcm_verify_mac` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:857`) | `-1` | [x] |
| 14 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:923`) | `-1` | [x] |
| 15 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (crypto_verify_16(mac, computed_mac) != 0) { (`c_src/libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:936`) | `-1` | [x] |
| 16 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:801`) | `-1` | [x] |
| 17 | `crypto_aead_aes256gcm_verify_mac` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:896`) | `-1` | [x] |
| 18 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (gh_required_blocks == 0) { (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:962`) | `-1` | [x] |
| 19 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (crypto_verify_16(mac, computed_mac) != 0) { (`c_src/libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:975`) | `-1` | [x] |
| 20 | `crypto_aead_chacha20poly1305_decrypt_detached` | if (ret != 0) { (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:237`) | `-1` | [x] |
| 21 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | if (ret != 0) { (`c_src/libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:322`) | `-1` | [x] |
| 22 | `_decrypt_detached` | if (ret != 0) { (`c_src/libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:137`) | `-1` | [x] |
| 23 | `crypto_box_detached` | if (crypto_box_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/crypto_box_easy.c:31`) | `-1` | [x] |
| 24 | `crypto_box_open_detached` | if (crypto_box_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/crypto_box_easy.c:83`) | `-1` | [x] |
| 25 | `crypto_box_open_easy_afternm` | if (clen < crypto_box_MACBYTES) { (`c_src/libsodium/crypto_box/crypto_box_easy.c:97`) | `-1` | [x] |
| 26 | `crypto_box_open_easy` | if (clen < crypto_box_MACBYTES) { (`c_src/libsodium/crypto_box/crypto_box_easy.c:110`) | `-1` | [x] |
| 27 | `crypto_box_seal` | if (crypto_box_keypair(epk, esk) != 0) { (`c_src/libsodium/crypto_box/crypto_box_seal.c:37`) | `-1` | [x] |
| 28 | `crypto_box_seal_open` | if (clen < crypto_box_SEALBYTES) { (`c_src/libsodium/crypto_box/crypto_box_seal.c:56`) | `-1` | [x] |
| 29 | `crypto_box_curve25519xchacha20poly1305_beforenm` | if (crypto_scalarmult_curve25519(s, sk, pk) != 0) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:49`) | `-1` | [x] |
| 30 | `crypto_box_curve25519xchacha20poly1305_detached` | if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:77`) | `-1` | [x] |
| 31 | `crypto_box_curve25519xchacha20poly1305_open_detached` | if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:132`) | `-1` | [x] |
| 32 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:147`) | `-1` | [x] |
| 33 | `crypto_box_curve25519xchacha20poly1305_open_easy` | if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:160`) | `-1` | [x] |
| 34 | `crypto_box_curve25519xchacha20poly1305_seal` | if (crypto_box_curve25519xchacha20poly1305_keypair(epk, esk) != 0) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:43`) | `-1` | [x] |
| 35 | `crypto_box_curve25519xchacha20poly1305_seal_open` | if (clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES) { (`c_src/libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:64`) | `-1` | [x] |
| 36 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | if (crypto_scalarmult_curve25519(s, sk, pk) != 0) { (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:43`) | `-1` | [x] |
| 37 | `crypto_box_curve25519xsalsa20poly1305` | if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:82`) | `-1` | [x] |
| 38 | `crypto_box_curve25519xsalsa20poly1305_open` | if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) { (`c_src/libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:99`) | `-1` | [x] |
| 39 | `crypto_core_ed25519_add` | if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:36`) | `-1` | [x] |
| 40 | `crypto_core_ed25519_sub` | if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:52`) | `-1` | [x] |
| 41 | `_string_to_points` | if (n > 2U) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:73`) | process termination | [x] |
| 42 | `_string_to_points` | if (core_h2c_string_to_hash(h_be, n * HASH_GE_L, ctx, ctx_len, msg, msg_len, hash_alg) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:77`) | `-1` | [x] |
| 43 | `crypto_core_ed25519_from_string` | if (_string_to_points(px, 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:109`) | `-1` | [x] |
| 44 | `crypto_core_ed25519_scalar_from_string` | if (core_h2c_string_to_hash(h_be, sizeof h_be, ctx, ctx_len, msg, msg_len, hash_alg) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ed25519.c:251`) | `-1` | [x] |
| 45 | `core_h2c_string_to_hash_sha256` | assertion is false: assert(h_len <= 0xff); (`c_src/libsodium/crypto_core/ed25519/core_h2c.c:26`) | assertion failure / process termination | [x] |
| 46 | `core_h2c_string_to_hash_sha512` | assertion is false: assert(h_len <= 0xff); (`c_src/libsodium/crypto_core/ed25519/core_h2c.c:82`) | assertion failure / process termination | [x] |
| 47 | `core_h2c_string_to_hash` | default: (`c_src/libsodium/crypto_core/ed25519/core_h2c.c:131`) | `-1` | [x] |
| 48 | `crypto_core_ristretto255_add` | if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:34`) | `-1` | [x] |
| 49 | `crypto_core_ristretto255_sub` | if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:50`) | `-1` | [x] |
| 50 | `_string_to_element` | if (core_h2c_string_to_hash(h, sizeof h, ctx, ctx_len, msg, msg_len, hash_alg) != 0) { (`c_src/libsodium/crypto_core/ed25519/core_ristretto255.c:76`) | `-1` | [x] |
| 51 | `ge25519_frombytes_negate_vartime` | if (fe25519_iszero(p_root_check) == 0) { (`c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:395`) | `-1` | [x] |
| 52 | `ge25519_elligator2` | if (ge25519_xmont_to_ymont(y, x) != 0) { (`c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:2684`) | process termination | [x] |
| 53 | `ristretto255_frombytes` | if (ristretto255_is_canonical(s) == 0) { (`c_src/libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:2834`) | `-1` | [x] |
| 54 | `blake2b_final` | if (blake2b_is_lastblock(S)) { (`c_src/libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:300`) | `-1` | [x] |
| 55 | `blake2b_final` | assertion is false: assert(S->buflen <= BLAKE2B_BLOCKBYTES); (`c_src/libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:306`) | assertion failure / process termination | [x] |
| 56 | `crypto_generichash_blake2b` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:18`) | `-1` | [x] |
| 57 | `crypto_generichash_blake2b` | assertion is false: assert(outlen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:20`) | assertion failure / process termination | [x] |
| 58 | `crypto_generichash_blake2b` | assertion is false: assert(keylen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:21`) | assertion failure / process termination | [x] |
| 59 | `crypto_generichash_blake2b_salt_personal` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:35`) | `-1` | [x] |
| 60 | `crypto_generichash_blake2b_salt_personal` | assertion is false: assert(outlen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:37`) | assertion failure / process termination | [x] |
| 61 | `crypto_generichash_blake2b_salt_personal` | assertion is false: assert(keylen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:38`) | assertion failure / process termination | [x] |
| 62 | `crypto_generichash_blake2b_init` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:52`) | `-1` | [x] |
| 63 | `crypto_generichash_blake2b_init` | assertion is false: assert(outlen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:54`) | assertion failure / process termination | [x] |
| 64 | `crypto_generichash_blake2b_init` | assertion is false: assert(keylen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:55`) | assertion failure / process termination | [x] |
| 65 | `crypto_generichash_blake2b_init` | if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:59`) | `-1` | [x] |
| 66 | `crypto_generichash_blake2b_init` | } else if (blake2b_init_key((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen) != 0) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:63`) | `-1` | [x] |
| 67 | `crypto_generichash_blake2b_init_salt_personal` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:76`) | `-1` | [x] |
| 68 | `crypto_generichash_blake2b_init_salt_personal` | assertion is false: assert(outlen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:78`) | assertion failure / process termination | [x] |
| 69 | `crypto_generichash_blake2b_init_salt_personal` | assertion is false: assert(keylen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:79`) | assertion failure / process termination | [x] |
| 70 | `crypto_generichash_blake2b_init_salt_personal` | if (blake2b_init_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, salt, personal) != 0) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:83`) | `-1` | [x] |
| 71 | `crypto_generichash_blake2b_init_salt_personal` | } else if (blake2b_init_key_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen, salt, personal) != 0) { (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:89`) | `-1` | [x] |
| 72 | `crypto_generichash_blake2b_final` | assertion is false: assert(outlen <= UINT8_MAX); (`c_src/libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:107`) | assertion failure / process termination | [x] |
| 73 | `crypto_kdf_blake2b_derive_from_key` | if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| subkey_len > crypto_kdf_blake2b_BYTES_MAX) { (`c_src/libsodium/crypto_kdf/blake2b/kdf_blake2b.c:46`) | `-1` | [x] |
| 74 | `crypto_kdf_hkdf_sha256_expand` | if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX) { (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:67`) | `-1` | [x] |
| 75 | `crypto_kdf_hkdf_sha512_expand` | if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX) { (`c_src/libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:67`) | `-1` | [x] |
| 76 | `mlkem768_ref_enc_deterministic` | if (polyvec_is_canonical(&pkpv) == 0) { (`c_src/libsodium/crypto_kem/mlkem768/ref/kem_mlkem768_ref.c:746`) | `-1` | [x] |
| 77 | `crypto_kem_xwing_enc_deterministic` | if (crypto_kem_mlkem768_enc_deterministic(ct_mlkem, ss_mlkem, pk_mlkem, seed_mlkem) != 0) { (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:135`) | `-1` | [x] |
| 78 | `crypto_kem_xwing_enc_deterministic` | if (crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0) { (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:142`) | `-1` | [x] |
| 79 | `crypto_kem_xwing_enc` | if (crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed) != 0) { (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:164`) | `-1` | [x] |
| 80 | `crypto_kem_xwing_dec` | if (crypto_kem_mlkem768_dec(ss_mlkem, ct_mlkem, sk_mlkem) != 0) { (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:191`) | `-1` | [x] |
| 81 | `crypto_kem_xwing_dec` | if (crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0) { (`c_src/libsodium/crypto_kem/xwing/kem_xwing.c:198`) | `-1` | [x] |
| 82 | `crypto_kx_client_session_keys` | if (crypto_scalarmult(q, client_sk, server_pk) != 0) { (`c_src/libsodium/crypto_kx/crypto_kx.c:55`) | `-1` | [x] |
| 83 | `crypto_kx_server_session_keys` | if (crypto_scalarmult(q, server_sk, client_pk) != 0) { (`c_src/libsodium/crypto_kx/crypto_kx.c:96`) | `-1` | [x] |
| 84 | `allocate_memory` | if (region == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:89`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 85 | `allocate_memory` | if (m_cost == 0 \|\| memory_size / m_cost != sizeof(block)) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:93`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 86 | `allocate_memory` | if (*region == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:97`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 87 | `allocate_memory` | if (base == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:128`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 88 | `argon2_validate_inputs` | if (NULL == context) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:231`) | `ARGON2_INCORRECT_PARAMETER` | [x] |
| 89 | `argon2_validate_inputs` | if (NULL == context->out) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:235`) | `ARGON2_OUTPUT_PTR_NULL` | [x] |
| 90 | `argon2_validate_inputs` | if (ARGON2_MIN_OUTLEN > context->outlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:240`) | `ARGON2_OUTPUT_TOO_SHORT` | [x] |
| 91 | `argon2_validate_inputs` | if (ARGON2_MAX_OUTLEN < context->outlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:244`) | `ARGON2_OUTPUT_TOO_LONG` | [x] |
| 92 | `argon2_validate_inputs` | if (0 != context->pwdlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:250`) | `ARGON2_PWD_PTR_MISMATCH` | [x] |
| 93 | `argon2_validate_inputs` | if (ARGON2_MIN_PWD_LENGTH > context->pwdlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:255`) | `ARGON2_PWD_TOO_SHORT` | [x] |
| 94 | `argon2_validate_inputs` | if (ARGON2_MAX_PWD_LENGTH < context->pwdlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:259`) | `ARGON2_PWD_TOO_LONG` | [x] |
| 95 | `argon2_validate_inputs` | if (0 != context->saltlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:265`) | `ARGON2_SALT_PTR_MISMATCH` | [x] |
| 96 | `argon2_validate_inputs` | if (ARGON2_MIN_SALT_LENGTH > context->saltlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:270`) | `ARGON2_SALT_TOO_SHORT` | [x] |
| 97 | `argon2_validate_inputs` | if (ARGON2_MAX_SALT_LENGTH < context->saltlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:274`) | `ARGON2_SALT_TOO_LONG` | [x] |
| 98 | `argon2_validate_inputs` | if (0 != context->secretlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:280`) | `ARGON2_SECRET_PTR_MISMATCH` | [x] |
| 99 | `argon2_validate_inputs` | if (ARGON2_MIN_SECRET > context->secretlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:284`) | `ARGON2_SECRET_TOO_SHORT` | [x] |
| 100 | `argon2_validate_inputs` | if (ARGON2_MAX_SECRET < context->secretlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:288`) | `ARGON2_SECRET_TOO_LONG` | [x] |
| 101 | `argon2_validate_inputs` | if (0 != context->adlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:295`) | `ARGON2_AD_PTR_MISMATCH` | [x] |
| 102 | `argon2_validate_inputs` | if (ARGON2_MIN_AD_LENGTH > context->adlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:299`) | `ARGON2_AD_TOO_SHORT` | [x] |
| 103 | `argon2_validate_inputs` | if (ARGON2_MAX_AD_LENGTH < context->adlen) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:303`) | `ARGON2_AD_TOO_LONG` | [x] |
| 104 | `argon2_validate_inputs` | if (ARGON2_MIN_LANES > context->lanes) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:309`) | `ARGON2_LANES_TOO_FEW` | [x] |
| 105 | `argon2_validate_inputs` | if (ARGON2_MAX_LANES < context->lanes) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:313`) | `ARGON2_LANES_TOO_MANY` | [x] |
| 106 | `argon2_validate_inputs` | if (ARGON2_MIN_MEMORY > context->m_cost) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:318`) | `ARGON2_MEMORY_TOO_LITTLE` | [x] |
| 107 | `argon2_validate_inputs` | if (ARGON2_MAX_MEMORY < context->m_cost) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:322`) | `ARGON2_MEMORY_TOO_MUCH` | [x] |
| 108 | `argon2_validate_inputs` | if (context->m_cost < 8 * context->lanes) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:326`) | `ARGON2_MEMORY_TOO_LITTLE` | [x] |
| 109 | `argon2_validate_inputs` | if (ARGON2_MIN_TIME > context->t_cost) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:331`) | `ARGON2_TIME_TOO_SMALL` | [x] |
| 110 | `argon2_validate_inputs` | if (ARGON2_MAX_TIME < context->t_cost) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:335`) | `ARGON2_TIME_TOO_LARGE` | [x] |
| 111 | `argon2_validate_inputs` | if (ARGON2_MIN_THREADS > context->threads) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:340`) | `ARGON2_THREADS_TOO_FEW` | [x] |
| 112 | `argon2_validate_inputs` | if (ARGON2_MAX_THREADS < context->threads) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:344`) | `ARGON2_THREADS_TOO_MANY` | [x] |
| 113 | `argon2_initialize` | if (instance == NULL \|\| context == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:466`) | `ARGON2_INCORRECT_PARAMETER` | [x] |
| 114 | `argon2_initialize` | if ((instance->pseudo_rands = (uint64_t *) malloc(sizeof(uint64_t) * instance->segment_length)) == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-core.c:473`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 115 | `decode_decimal` | if (acc > (ULONG_MAX / 10)) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:54`) | `NULL` | [x] |
| 116 | `decode_decimal` | if ((unsigned long) c > (ULONG_MAX - acc)) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:58`) | `NULL` | [x] |
| 117 | `decode_decimal` | if (str == orig \|\| (*orig == '0' && str != (orig + 1))) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:63`) | `NULL` | [x] |
| 118 | `argon2_decode_string` | if (strncmp(str, prefix, cc_len) != 0) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:101`) | `ARGON2_DECODING_FAIL` | [x] |
| 119 | `argon2_decode_string` | if (str == NULL) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:124`) | `ARGON2_DECODING_FAIL` | [x] |
| 120 | `argon2_decode_string` | if (str == NULL \|\| dec_x > UINT32_MAX) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:135`) | `ARGON2_DECODING_FAIL` | [x] |
| 121 | `argon2_decode_string` | if (sodium_base642bin((buf), (max_len), str, strlen(str), NULL, \ &bin_len, &str_end, \ sodium_base64_VARIANT_ORIGINAL_NO_PADDING) != 0 \|\| \ bin_len > UINT32_MAX) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:149`) | `ARGON2_DECODING_FAIL` | [x] |
| 122 | `argon2_decode_string` | } else if (type == Argon2_i) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:168`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 123 | `argon2_decode_string` | if (version != ARGON2_VERSION_NUMBER) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:173`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 124 | `argon2_decode_string` | if (ctx->m_cost > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:178`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 125 | `argon2_decode_string` | if (ctx->t_cost > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:183`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 126 | `argon2_decode_string` | if (ctx->lanes > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:188`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 127 | `argon2_decode_string` | if (*str == 0) { (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:203`) | `ARGON2_DECODING_FAIL` | [x] |
| 128 | `argon2_encode_string` | if (pp_len >= dst_len) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:248`) | `ARGON2_ENCODING_FAIL` | [x] |
| 129 | `argon2_encode_string` | if (sodium_bin2base64(dst, dst_len, (buf), (len), \ sodium_base64_VARIANT_ORIGINAL_NO_PADDING) == NULL) { \ (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:267`) | `ARGON2_ENCODING_FAIL` | [x] |
| 130 | `argon2_encode_string` | default: (`c_src/libsodium/crypto_pwhash/argon2/argon2-encoding.c:282`) | `ARGON2_ENCODING_FAIL` | [x] |
| 131 | `argon2_ctx` | if (type != Argon2_id && type != Argon2_i) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:41`) | `ARGON2_INCORRECT_TYPE` | [x] |
| 132 | `argon2_hash` | if (pwdlen > ARGON2_MAX_PWD_LENGTH) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:102`) | `ARGON2_PWD_TOO_LONG` | [x] |
| 133 | `argon2_hash` | if (hashlen > ARGON2_MAX_OUTLEN) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:106`) | `ARGON2_OUTPUT_TOO_LONG` | [x] |
| 134 | `argon2_hash` | if (saltlen > ARGON2_MAX_SALT_LENGTH) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:110`) | `ARGON2_SALT_TOO_LONG` | [x] |
| 135 | `argon2_hash` | if (!out) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:115`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 136 | `argon2_hash` | if (argon2_encode_string(encoded, encodedlen, &context, type) != ARGON2_OK) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:152`) | `ARGON2_ENCODING_FAIL` | [x] |
| 137 | `argon2_verify` | if (encoded_len > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:230`) | `ARGON2_DECODING_LENGTH_FAIL` | [x] |
| 138 | `argon2_verify` | if (!ctx.out \|\| !ctx.salt \|\| !ctx.ad) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:244`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 139 | `argon2_verify` | if (!out) { (`c_src/libsodium/crypto_pwhash/argon2/argon2.c:253`) | `ARGON2_MEMORY_ALLOCATION_ERROR` | [x] |
| 140 | `crypto_pwhash_argon2i` | if (outlen > crypto_pwhash_argon2i_BYTES_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:148`) | `-1` | [x] |
| 141 | `crypto_pwhash_argon2i` | if (outlen < crypto_pwhash_argon2i_BYTES_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:152`) | `-1` | [x] |
| 142 | `crypto_pwhash_argon2i` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:158`) | `-1` | [x] |
| 143 | `crypto_pwhash_argon2i` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:164`) | `-1` | [x] |
| 144 | `crypto_pwhash_argon2i` | if ((const void *) out == (const void *) passwd) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:168`) | `-1` | [x] |
| 145 | `crypto_pwhash_argon2i` | if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:176`) | `-1` | [x] |
| 146 | `crypto_pwhash_argon2i` | default: (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:181`) | `-1` | [x] |
| 147 | `crypto_pwhash_argon2i_str` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:198`) | `-1` | [x] |
| 148 | `crypto_pwhash_argon2i_str` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:204`) | `-1` | [x] |
| 149 | `crypto_pwhash_argon2i_str` | if (argon2i_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2i_STRBYTES) != ARGON2_OK) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:211`) | `-1` | [x] |
| 150 | `crypto_pwhash_argon2i_str_verify` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:225`) | `-1` | [x] |
| 151 | `crypto_pwhash_argon2i_str_verify` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:230`) | `-1` | [x] |
| 152 | `crypto_pwhash_argon2i_str_verify` | if (verify_ret == ARGON2_VERIFY_MISMATCH) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:241`) | `-1` | [x] |
| 153 | `_needs_rehash` | if (opslimit > UINT32_MAX \|\| memlimit > UINT32_MAX \|\| fodder_len >= crypto_pwhash_STRBYTES) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:258`) | `-1` | [x] |
| 154 | `_needs_rehash` | if ((fodder = (unsigned char *) calloc(fodder_len, 1U)) == NULL) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:262`) | `-1` | [x] |
| 155 | `crypto_pwhash_argon2id` | if (outlen > crypto_pwhash_argon2id_BYTES_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:144`) | `-1` | [x] |
| 156 | `crypto_pwhash_argon2id` | if (outlen < crypto_pwhash_argon2id_BYTES_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:148`) | `-1` | [x] |
| 157 | `crypto_pwhash_argon2id` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:154`) | `-1` | [x] |
| 158 | `crypto_pwhash_argon2id` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:160`) | `-1` | [x] |
| 159 | `crypto_pwhash_argon2id` | if ((const void *) out == (const void *) passwd) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:164`) | `-1` | [x] |
| 160 | `crypto_pwhash_argon2id` | if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:172`) | `-1` | [x] |
| 161 | `crypto_pwhash_argon2id` | default: (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:177`) | `-1` | [x] |
| 162 | `crypto_pwhash_argon2id_str` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:194`) | `-1` | [x] |
| 163 | `crypto_pwhash_argon2id_str` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:200`) | `-1` | [x] |
| 164 | `crypto_pwhash_argon2id_str` | if (argon2id_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2id_STRBYTES) != ARGON2_OK) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:207`) | `-1` | [x] |
| 165 | `crypto_pwhash_argon2id_str_verify` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:221`) | `-1` | [x] |
| 166 | `crypto_pwhash_argon2id_str_verify` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:226`) | `-1` | [x] |
| 167 | `crypto_pwhash_argon2id_str_verify` | if (verify_ret == ARGON2_VERIFY_MISMATCH) { (`c_src/libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:237`) | `-1` | [x] |
| 168 | `crypto_pwhash` | default: (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:143`) | `-1` | [x] |
| 169 | `crypto_pwhash_str_alg` | case crypto_pwhash_ALG_ARGON2ID13: (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:171`) | `-1` | [x] |
| 170 | `crypto_pwhash_str_verify` | if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) { (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:189`) | `-1` | [x] |
| 171 | `crypto_pwhash_str_needs_rehash` | if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) { (`c_src/libsodium/crypto_pwhash/crypto_pwhash.c:206`) | `-1` | [x] |
| 172 | `encode64_uint32` | if (dstlen < 1) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:41`) | `NULL` | [x] |
| 173 | `encode64` | if (!dnext) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:66`) | `NULL` | [x] |
| 174 | `decode64_one` | if (ptr) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:85`) | `-1` | [x] |
| 175 | `decode64_uint32` | if (decode64_one(&one, *src)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:99`) | `NULL` | [x] |
| 176 | `escrypt_parse_setting` | if (setting[0] != '$' \|\| setting[1] != '7' \|\| setting[2] != '$') { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:116`) | `NULL` | [x] |
| 177 | `escrypt_parse_setting` | if (decode64_one(N_log2_p, *src)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:121`) | `NULL` | [x] |
| 178 | `escrypt_parse_setting` | if (!src) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:127`) | `NULL` | [x] |
| 179 | `escrypt_parse_setting` | if (!src) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:132`) | `NULL` | [x] |
| 180 | `escrypt_r` | if (!src) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:160`) | `NULL` | [x] |
| 181 | `escrypt_r` | if (buf == NULL \|\| need > buflen \|\| need < saltlen) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:175`) | `NULL` | [x] |
| 182 | `escrypt_r` | if (escrypt_kdf(local, passwd, passwdlen, salt, saltlen, N, r, p, hash, sizeof(hash))) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:185`) | `NULL` | [x] |
| 183 | `escrypt_r` | if (!dst \|\| dst >= buf + buflen) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:195`) | `NULL` | [x] |
| 184 | `escrypt_gensalt_r` | if (need > buflen \|\| need < saltlen \|\| saltlen < srclen) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:214`) | `NULL` | [x] |
| 185 | `escrypt_gensalt_r` | if (N_log2 > 63 \|\| ((uint64_t) r * (uint64_t) p >= (1U << 30))) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:217`) | `NULL` | [x] |
| 186 | `escrypt_gensalt_r` | if (!dst) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:228`) | `NULL` | [x] |
| 187 | `escrypt_gensalt_r` | if (!dst) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:232`) | `NULL` | [x] |
| 188 | `escrypt_gensalt_r` | if (!dst \|\| dst >= buf + buflen) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:236`) | `NULL` | [x] |
| 189 | `crypto_pwhash_scryptsalsa208sha256_ll` | if (escrypt_init_local(&local)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:254`) | `-1` | [x] |
| 190 | `crypto_pwhash_scryptsalsa208sha256_ll` | if (escrypt_free_local(&local)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:265`) | `-1` | [x] |
| 191 | `escrypt_kdf_nosse` | if (buflen > (((uint64_t)(1) << 32) - 1) * 32) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:250`) | `-1` | [x] |
| 192 | `escrypt_kdf_nosse` | if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:255`) | `-1` | [x] |
| 193 | `escrypt_kdf_nosse` | if (N > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:259`) | `-1` | [x] |
| 194 | `escrypt_kdf_nosse` | if (((N & (N - 1)) != 0) \|\| (N < 2)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:263`) | `-1` | [x] |
| 195 | `escrypt_kdf_nosse` | if (r == 0 \|\| p == 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:267`) | `-1` | [x] |
| 196 | `escrypt_kdf_nosse` | if ((r > SIZE_MAX / 128 / p) \|\| #if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| #endif (N > SIZE_MAX / 128 / r)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:275`) | `-1` | [x] |
| 197 | `escrypt_kdf_nosse` | if (need < V_size) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:284`) | `-1` | [x] |
| 198 | `escrypt_kdf_nosse` | if (need < XY_size) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:290`) | `-1` | [x] |
| 199 | `escrypt_kdf_nosse` | if (escrypt_free_region(local)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:294`) | `-1` | [x] |
| 200 | `escrypt_kdf_nosse` | if (!escrypt_alloc_region(local, need)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:297`) | `-1` | [x] |
| 201 | `crypto_pwhash_scryptsalsa208sha256` | if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\| outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:173`) | `-1` | [x] |
| 202 | `crypto_pwhash_scryptsalsa208sha256` | if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:178`) | `-1` | [x] |
| 203 | `crypto_pwhash_scryptsalsa208sha256` | if ((const void *) out == (const void *) passwd) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:182`) | `-1` | [x] |
| 204 | `crypto_pwhash_scryptsalsa208sha256_str` | if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:206`) | `-1` | [x] |
| 205 | `crypto_pwhash_scryptsalsa208sha256_str` | if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:211`) | `-1` | [x] |
| 206 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting, sizeof setting) == NULL) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:217`) | `-1` | [x] |
| 207 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_init_local(&escrypt_local) != 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:220`) | `-1` | [x] |
| 208 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) setting, (uint8_t *) out, crypto_pwhash_scryptsalsa208sha256_STRBYTES) == NULL) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:228`) | `-1` | [x] |
| 209 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:255`) | `-1` | [x] |
| 210 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (escrypt_init_local(&escrypt_local) != 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:258`) | `-1` | [x] |
| 211 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) str, (uint8_t *) wanted, sizeof wanted) == NULL) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:265`) | `-1` | [x] |
| 212 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:285`) | `-1` | [x] |
| 213 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:290`) | `-1` | [x] |
| 214 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (escrypt_parse_setting((const uint8_t *) str, &N_log2_, &r_, &p_) == NULL) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:295`) | `-1` | [x] |
| 215 | `escrypt_free_region` | if (munmap(region->base, region->size)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c:89`) | `-1` | [x] |
| 216 | `escrypt_kdf_sse` | if (buflen > (((uint64_t)(1) << 32) - 1) * 32) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:325`) | `-1` | [x] |
| 217 | `escrypt_kdf_sse` | if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:331`) | `-1` | [x] |
| 218 | `escrypt_kdf_sse` | if (N > UINT32_MAX) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:335`) | `-1` | [x] |
| 219 | `escrypt_kdf_sse` | if (((N & (N - 1)) != 0) \|\| (N < 2)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:339`) | `-1` | [x] |
| 220 | `escrypt_kdf_sse` | if (r == 0 \|\| p == 0) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:343`) | `-1` | [x] |
| 221 | `escrypt_kdf_sse` | if ((r > SIZE_MAX / 128 / p) \|\| # if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| # endif (N > SIZE_MAX / 128 / r)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:352`) | `-1` | [x] |
| 222 | `escrypt_kdf_sse` | if (need < V_size) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:363`) | `-1` | [x] |
| 223 | `escrypt_kdf_sse` | if (need < XY_size) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:371`) | `-1` | [x] |
| 224 | `escrypt_kdf_sse` | if (escrypt_free_region(local)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:376`) | `-1` | [x] |
| 225 | `escrypt_kdf_sse` | if (!escrypt_alloc_region(local, need)) { (`c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:379`) | `-1` | [x] |
| 226 | `crypto_scalarmult_curve25519_ref10` | if (has_small_order(p)) { (`c_src/libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c:107`) | `-1` | [x] |
| 227 | `crypto_scalarmult_curve25519` | if (implementation->mult(q, n, p) != 0) { (`c_src/libsodium/crypto_scalarmult/curve25519/scalarmult_curve25519.c:22`) | `-1` | [x] |
| 228 | `_crypto_scalarmult_ed25519` | if (ge25519_is_canonical(p) == 0 \|\| ge25519_frombytes(&P, p) != 0 \|\| ge25519_has_small_order(&P) != 0 \|\| ge25519_is_on_main_subgroup(&P) == 0) { (`c_src/libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:41`) | `-1` | [x] |
| 229 | `_crypto_scalarmult_ed25519` | if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) { (`c_src/libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:54`) | `-1` | [x] |
| 230 | `_crypto_scalarmult_ed25519_base` | if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) { (`c_src/libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:92`) | `-1` | [x] |
| 231 | `crypto_scalarmult_ristretto255` | if (ristretto255_frombytes(&P, p) != 0) { (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:19`) | `-1` | [x] |
| 232 | `crypto_scalarmult_ristretto255` | if (sodium_is_zero(q, 32)) { (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:28`) | `-1` | [x] |
| 233 | `crypto_scalarmult_ristretto255_base` | if (sodium_is_zero(q, 32)) { (`c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:48`) | `-1` | [x] |
| 234 | `crypto_secretbox_open_detached` | if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) { (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:129`) | `-1` | [x] |
| 235 | `crypto_secretbox_open_easy` | if (clen < crypto_secretbox_MACBYTES) { (`c_src/libsodium/crypto_secretbox/crypto_secretbox_easy.c:171`) | `-1` | [x] |
| 236 | `crypto_secretbox_xchacha20poly1305_open_detached` | if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) { (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:122`) | `-1` | [x] |
| 237 | `crypto_secretbox_xchacha20poly1305_open_easy` | if (clen < crypto_secretbox_xchacha20poly1305_MACBYTES) { (`c_src/libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:165`) | `-1` | [x] |
| 238 | `crypto_secretbox_xsalsa20poly1305` | if (mlen < 32) { (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:16`) | `-1` | [x] |
| 239 | `crypto_secretbox_xsalsa20poly1305_open` | if (clen < 32) { (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:36`) | `-1` | [x] |
| 240 | `crypto_secretbox_xsalsa20poly1305_open` | if (crypto_onetimeauth_poly1305_verify(c + 16, c + 32, clen - 32, subkey) != 0) { (`c_src/libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:41`) | `-1` | [x] |
| 241 | `crypto_secretstream_xchacha20poly1305_pull` | if (inlen < crypto_secretstream_xchacha20poly1305_ABYTES) { (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:202`) | `-1` | [x] |
| 242 | `crypto_secretstream_xchacha20poly1305_pull` | if (sodium_memcmp(mac, stored_mac, sizeof mac) != 0) { (`c_src/libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:241`) | `-1` | [x] |
| 243 | `crypto_sign_ed25519_pk_to_curve25519` | if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 \|\| ge25519_has_small_order(&A) != 0 \|\| ge25519_is_on_main_subgroup(&A) == 0) { (`c_src/libsodium/crypto_sign/ed25519/ref10/keypair.c:56`) | `-1` | [x] |
| 244 | `_crypto_sign_ed25519_verify_detached` | if (sig[63] & 224) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:32`) | `-1` | [x] |
| 245 | `_crypto_sign_ed25519_verify_detached` | if ((sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:37`) | `-1` | [x] |
| 246 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_is_canonical(pk) == 0) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:40`) | `-1` | [x] |
| 247 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_frombytes_negate_vartime(&A, pk) != 0 \|\| ge25519_has_small_order(&A) != 0) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:45`) | `-1` | [x] |
| 248 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_frombytes(&expected_r, sig) != 0 \|\| ge25519_has_small_order(&expected_r) != 0) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:49`) | `-1` | [x] |
| 249 | `crypto_sign_ed25519_open` | if (mlen_p != NULL) { (`c_src/libsodium/crypto_sign/ed25519/ref10/open.c:103`) | `-1` | [x] |
| 250 | `crypto_sign_ed25519` | if (smlen_p != NULL) { (`c_src/libsodium/crypto_sign/ed25519/ref10/sign.c:120`) | `-1` | [x] |
| 251 | `randombytes_getentropy` | if (CCRandomGenerateBytes(buf, size) != kCCSuccess) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:198`) | `-1` | [x] |
| 252 | `_randombytes_getentropy` | assertion is false: assert(size <= 256U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:208`) | assertion failure / process termination | [x] |
| 253 | `_randombytes_getentropy` | if (&getentropy == NULL) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:212`) | `-1` | [x] |
| 254 | `_randombytes_getentropy` | if (getentropy(buf, size) != 0) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:216`) | `-1` | [x] |
| 255 | `randombytes_getentropy` | assertion is false: assert(chunk_size > (size_t) 0U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:230`) | assertion failure / process termination | [x] |
| 256 | `randombytes_getentropy` | if (_randombytes_getentropy(buf, chunk_size) != 0) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:233`) | `-1` | [x] |
| 257 | `_randombytes_linux_getrandom` | assertion is false: assert(size <= 256U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:249`) | assertion failure / process termination | [x] |
| 258 | `randombytes_linux_getrandom` | assertion is false: assert(chunk_size > (size_t) 0U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:266`) | assertion failure / process termination | [x] |
| 259 | `randombytes_linux_getrandom` | if (_randombytes_linux_getrandom(buf, chunk_size) != 0) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:269`) | `-1` | [x] |
| 260 | `randombytes_block_on_dev_random` | if (pret != 1) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:302`) | `-1` | [x] |
| 261 | `randombytes_internal_random_random_dev_open` | if (randombytes_block_on_dev_random() != 0) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:324`) | `-1` | [x] |
| 262 | `randombytes_internal_random_random_dev_open` | } else if (errno == EINTR) { (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:344`) | `-1` | [x] |
| 263 | `safe_read` | assertion is false: assert(size > (size_t) 0U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:354`) | assertion failure / process termination | [x] |
| 264 | `safe_read` | assertion is false: assert(size <= SSIZE_MAX); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:355`) | assertion failure / process termination | [x] |
| 265 | `randombytes_internal_random_init` | assertion is false: assert((global.getentropy_available \| global.getrandom_available) == 0); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:406`) | assertion failure / process termination | [x] |
| 266 | `randombytes_internal_random_stir` | assertion is false: assert(stream.nonce != (uint64_t) 0U); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:430`) | assertion failure / process termination | [x] |
| 267 | `randombytes_internal_random_buf` | assertion is false: assert(size <= ULLONG_MAX); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:599`) | assertion failure / process termination | [x] |
| 268 | `randombytes_internal_random_buf` | assertion is false: assert(ret == 0); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:604`) | assertion failure / process termination | [x] |
| 269 | `randombytes_internal_random` | assertion is false: assert(ret == 0); (`c_src/libsodium/randombytes/internal/randombytes_internal_random.c:636`) | assertion failure / process termination | [x] |
| 270 | `randombytes` | assertion is false: assert(buf_len <= SIZE_MAX); (`c_src/libsodium/randombytes/randombytes.c:247`) | assertion failure / process termination | [x] |
| 271 | `safe_read` | assertion is false: assert(size > (size_t) 0U); (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:134`) | assertion failure / process termination | [x] |
| 272 | `safe_read` | assertion is false: assert(size <= SSIZE_MAX); (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:135`) | assertion failure / process termination | [x] |
| 273 | `randombytes_block_on_dev_random` | if (pret != 1) { (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:173`) | `-1` | [x] |
| 274 | `randombytes_sysrandom_random_dev_open` | if (randombytes_block_on_dev_random() != 0) { (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:195`) | `-1` | [x] |
| 275 | `randombytes_sysrandom_random_dev_open` | } else if (errno == EINTR) { (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:223`) | `-1` | [x] |
| 276 | `_randombytes_linux_getrandom` | assertion is false: assert(size <= 256U); (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:233`) | assertion failure / process termination | [x] |
| 277 | `randombytes_linux_getrandom` | assertion is false: assert(chunk_size > (size_t) 0U); (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:250`) | assertion failure / process termination | [x] |
| 278 | `randombytes_linux_getrandom` | if (_randombytes_linux_getrandom(buf, chunk_size) != 0) { (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:253`) | `-1` | [x] |
| 279 | `randombytes_sysrandom_buf` | assertion is false: assert(size <= ULLONG_MAX); (`c_src/libsodium/randombytes/sysrandom/randombytes_sysrandom.c:346`) | assertion failure / process termination | [x] |
| 280 | `sodium_bin2base64` | assertion is false: assert(b64_pos <= b64_len); (`c_src/libsodium/sodium/codecs.c:239`) | assertion failure / process termination | [x] |
| 281 | `_sodium_base642bin_skip_padding` | if (*b64_pos_p >= b64_len) { (`c_src/libsodium/sodium/codecs.c:260`) | `-1` | [x] |
| 282 | `_sodium_base642bin_skip_padding` | } else if (ignore == NULL \|\| strchr(ignore, c) == NULL) { (`c_src/libsodium/sodium/codecs.c:268`) | `-1` | [x] |
| 283 | `ip_hex_digit` | if (((unsigned int) ch \| 32U) >= 'a' && ((unsigned int) ch \| 32U) <= 'f') { (`c_src/libsodium/sodium/codecs.c:354`) | `-1` | [x] |
| 284 | `sodium_ip2bin` | if (!((*z >= '0' && *z <= '9') \|\| (*z >= 'a' && *z <= 'z') \|\| (*z >= 'A' && *z <= 'Z') \|\| *z == '-' \|\| *z == '_' \|\| *z == '.')) { (`c_src/libsodium/sodium/codecs.c:502`) | `-1` | [x] |
| 285 | `sodium_ip2bin` | if (zone + 1 >= end) { (`c_src/libsodium/sodium/codecs.c:506`) | `-1` | [x] |
| 286 | `sodium_ip2bin` | if (zone != NULL && !is_ipv6) { (`c_src/libsodium/sodium/codecs.c:512`) | `-1` | [x] |
| 287 | `sodium_ip2bin` | if (parse_ipv4(ip, end, v4) == 0) { (`c_src/libsodium/sodium/codecs.c:518`) | `-1` | [x] |
| 288 | `sodium_bin2ip` | if (ip_maxlen <= 2U) { (`c_src/libsodium/sodium/codecs.c:562`) | `NULL` | [x] |
| 289 | `sodium_bin2ip` | if (len >= ip_maxlen) { (`c_src/libsodium/sodium/codecs.c:573`) | `NULL` | [x] |
| 290 | `sodium_bin2ip` | if (len >= ip_maxlen) { (`c_src/libsodium/sodium/codecs.c:618`) | `NULL` | [x] |
| 291 | `sodium_init` | if (sodium_crit_enter() != 0) { (`c_src/libsodium/sodium/core.c:31`) | `-1` | [x] |
| 292 | `sodium_init` | if (sodium_crit_leave() != 0) { (`c_src/libsodium/sodium/core.c:35`) | `-1` | [x] |
| 293 | `sodium_init` | if (sodium_crit_leave() != 0) { (`c_src/libsodium/sodium/core.c:53`) | `-1` | [x] |
| 294 | `_sodium_crit_init` | default: /* should never be reached */ (`c_src/libsodium/sodium/core.c:80`) | `-1` | [x] |
| 295 | `sodium_crit_enter` | if (_sodium_crit_init() != 0) { (`c_src/libsodium/sodium/core.c:88`) | `-1` | [x] |
| 296 | `sodium_crit_enter` | assertion is false: assert(locked == 0); (`c_src/libsodium/sodium/core.c:91`) | assertion failure / process termination | [x] |
| 297 | `sodium_crit_leave` | if (locked == 0) { (`c_src/libsodium/sodium/core.c:104`) | `-1` | [x] |
| 298 | `sodium_crit_enter` | assertion is false: assert(locked == 0); (`c_src/libsodium/sodium/core.c:122`) | assertion failure / process termination | [x] |
| 299 | `sodium_crit_leave` | if (locked == 0) { (`c_src/libsodium/sodium/core.c:135`) | `-1` | [x] |
| 300 | `sodium_misuse` | if (sodium_crit_leave() == 0 && handler != NULL) { (`c_src/libsodium/sodium/core.c:204`) | process termination | [x] |
| 301 | `sodium_set_misuse_handler` | if (sodium_crit_enter() != 0) { (`c_src/libsodium/sodium/core.c:212`) | `-1` | [x] |
| 302 | `sodium_set_misuse_handler` | if (sodium_crit_leave() != 0) { (`c_src/libsodium/sodium/core.c:216`) | `-1` | [x] |
| 303 | `_sodium_runtime_arm_cpu_features` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/runtime.c:67`) | `-1` | [x] |
| 304 | `_sodium_runtime_intel_cpu_features` | if (cpu_info[0] == 0U) { (`c_src/libsodium/sodium/runtime.c:209`) | `-1` | [x] |
| 305 | `sodium_mlock` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:444`) | `-1` | [x] |
| 306 | `sodium_munlock` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:461`) | `-1` | [x] |
| 307 | `_mprotect_noaccess` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:475`) | `-1` | [x] |
| 308 | `_mprotect_readonly` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:489`) | `-1` | [x] |
| 309 | `_mprotect_readwrite` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:503`) | `-1` | [x] |
| 310 | `_out_of_bounds` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:522`) | process termination | [x] |
| 311 | `_sodium_malloc` | if (size >= (size_t) SIZE_MAX - page_size * 4U) { (`c_src/libsodium/sodium/utils.c:609`) | `NULL` | [x] |
| 312 | `_sodium_malloc` | if ((base_ptr = _alloc_aligned(total_size)) == NULL) { (`c_src/libsodium/sodium/utils.c:618`) | `NULL` | [x] |
| 313 | `_sodium_malloc` | assertion is false: assert(_unprotected_ptr_from_user_ptr(user_ptr) == unprotected_ptr); (`c_src/libsodium/sodium/utils.c:633`) | assertion failure / process termination | [x] |
| 314 | `sodium_malloc` | if ((ptr = _sodium_malloc(size)) == NULL) { (`c_src/libsodium/sodium/utils.c:645`) | `NULL` | [x] |
| 315 | `sodium_allocarray` | if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) { (`c_src/libsodium/sodium/utils.c:657`) | `NULL` | [x] |
| 316 | `_sodium_mprotect` | unconditional rejection at the cited source location (`c_src/libsodium/sodium/utils.c:708`) | `-1` | [x] |
| 317 | `sodium_pad` | if (blocksize <= 0U) { (`c_src/libsodium/sodium/utils.c:756`) | `-1` | [x] |
| 318 | `sodium_pad` | if (xpadded_len >= max_buflen) { (`c_src/libsodium/sodium/utils.c:769`) | `-1` | [x] |
| 319 | `sodium_unpad` | if (padded_buflen < blocksize \|\| blocksize <= 0U) { (`c_src/libsodium/sodium/utils.c:798`) | `-1` | [x] |
