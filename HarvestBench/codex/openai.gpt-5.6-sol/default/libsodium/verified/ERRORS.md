# Error Surface

Generated from every explicit error/sentinel return, assertion, misuse/abort path, error jump, and `errno` assignment in the C source. Trigger text is the nearest source condition and each row retains the exact source site for auditability.

| # | function | trigger (exact C condition/site) | expected C result | [ ] |
|---:|----------|----------------------------------|-------------------|:---:|
| 1 | `crypto_aead_aegis128l_encrypt` | `libsodium/crypto_aead/aegis128l/aead_aegis128l.c:70`: ` if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 2 | `crypto_aead_aegis128l_encrypt_detached` | `libsodium/crypto_aead/aegis128l/aead_aegis128l.c:121`: ` if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 3 | `crypto_aead_aegis128l_decrypt_detached` | `libsodium/crypto_aead/aegis128l/aead_aegis128l.c:139`: ` if (clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 4 | `<file scope>` | `libsodium/crypto_aead/aegis128l/aegis128l_common.h:64`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;             /* LCOV_EXCL_LINE */` | [x] |
| 5 | `crypto_aead_aegis256_encrypt` | `libsodium/crypto_aead/aegis256/aead_aegis256.c:70`: ` if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 6 | `crypto_aead_aegis256_encrypt_detached` | `libsodium/crypto_aead/aegis256/aead_aegis256.c:121`: ` if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 7 | `crypto_aead_aegis256_decrypt_detached` | `libsodium/crypto_aead/aegis256/aead_aegis256.c:138`: ` if (clen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 8 | `<file scope>` | `libsodium/crypto_aead/aegis256/aegis256_common.h:64`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;             /* LCOV_EXCL_LINE */` | [x] |
| 9 | `crypto_aead_aes256gcm_encrypt_detached` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:64`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 10 | `crypto_aead_aes256gcm_encrypt_detached` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:65`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 11 | `crypto_aead_aes256gcm_encrypt` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:74`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 12 | `crypto_aead_aes256gcm_encrypt` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:75`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 13 | `crypto_aead_aes256gcm_decrypt_detached` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:85`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 14 | `crypto_aead_aes256gcm_decrypt_detached` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:86`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 15 | `crypto_aead_aes256gcm_decrypt` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:95`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 16 | `crypto_aead_aes256gcm_decrypt` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:96`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 17 | `crypto_aead_aes256gcm_beforenm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:102`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 18 | `crypto_aead_aes256gcm_beforenm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:103`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 19 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:114`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 20 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:115`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 21 | `crypto_aead_aes256gcm_encrypt_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:125`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 22 | `crypto_aead_aes256gcm_encrypt_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:126`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 23 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:136`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 24 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:137`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 25 | `crypto_aead_aes256gcm_decrypt_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:147`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 26 | `crypto_aead_aes256gcm_decrypt_afternm` | `libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c:148`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 27 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:756`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 28 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:762`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 29 | `crypto_aead_aes256gcm_verify_mac` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:853`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 30 | `crypto_aead_aes256gcm_verify_mac` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:857`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 31 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:916`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 32 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:923`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 33 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:936`: ` if (crypto_verify_16(mac, computed_mac) != 0) {` | `return -1;` | [x] |
| 34 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:795`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 35 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:801`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 36 | `crypto_aead_aes256gcm_verify_mac` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:892`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 37 | `crypto_aead_aes256gcm_verify_mac` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:896`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 38 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:955`: ` if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) {` | `sodium_misuse();` | [x] |
| 39 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:962`: ` if (gh_required_blocks == 0) {` | `return -1;` | [x] |
| 40 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `libsodium/crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:975`: ` if (crypto_verify_16(mac, computed_mac) != 0) {` | `return -1;` | [x] |
| 41 | `crypto_aead_chacha20poly1305_encrypt` | `libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:90`: ` if (mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 42 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:178`: ` if (mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 43 | `crypto_aead_chacha20poly1305_decrypt_detached` | `libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:237`: ` if (ret != 0) {` | `return -1;` | [x] |
| 44 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `libsodium/crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:322`: ` if (ret != 0) {` | `return -1;` | [x] |
| 45 | `_decrypt_detached` | `libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:137`: ` if (ret != 0) {` | `return -1;` | [x] |
| 46 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `libsodium/crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:186`: ` if (mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 47 | `crypto_auth_hmacsha256_init` | `libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c:53`: ` if (keylen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 48 | `crypto_auth_hmacsha512_init` | `libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c:53`: ` if (keylen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 49 | `crypto_box_detached` | `libsodium/crypto_box/crypto_box_easy.c:31`: ` if (crypto_box_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 50 | `crypto_box_easy_afternm` | `libsodium/crypto_box/crypto_box_easy.c:45`: ` if (mlen > crypto_box_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 51 | `crypto_box_easy` | `libsodium/crypto_box/crypto_box_easy.c:57`: ` if (mlen > crypto_box_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 52 | `crypto_box_open_detached` | `libsodium/crypto_box/crypto_box_easy.c:83`: ` if (crypto_box_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 53 | `crypto_box_open_easy_afternm` | `libsodium/crypto_box/crypto_box_easy.c:97`: ` if (clen < crypto_box_MACBYTES) {` | `return -1;` | [x] |
| 54 | `crypto_box_open_easy` | `libsodium/crypto_box/crypto_box_easy.c:110`: ` if (clen < crypto_box_MACBYTES) {` | `return -1;` | [x] |
| 55 | `crypto_box_seal` | `libsodium/crypto_box/crypto_box_seal.c:34`: ` if (mlen > crypto_box_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 56 | `crypto_box_seal` | `libsodium/crypto_box/crypto_box_seal.c:37`: ` if (crypto_box_keypair(epk, esk) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 57 | `crypto_box_seal_open` | `libsodium/crypto_box/crypto_box_seal.c:56`: ` if (clen < crypto_box_SEALBYTES) {` | `return -1;` | [x] |
| 58 | `crypto_box_curve25519xchacha20poly1305_beforenm` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:49`: ` if (crypto_scalarmult_curve25519(s, sk, pk) != 0) {` | `return -1;` | [x] |
| 59 | `crypto_box_curve25519xchacha20poly1305_detached` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:77`: ` if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 60 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:94`: ` if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 61 | `crypto_box_curve25519xchacha20poly1305_easy` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:106`: ` if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 62 | `crypto_box_curve25519xchacha20poly1305_open_detached` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:132`: ` if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 63 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:147`: ` if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) {` | `return -1;` | [x] |
| 64 | `crypto_box_curve25519xchacha20poly1305_open_easy` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:160`: ` if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) {` | `return -1;` | [x] |
| 65 | `crypto_box_curve25519xchacha20poly1305_seal` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:40`: ` if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 66 | `crypto_box_curve25519xchacha20poly1305_seal` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:43`: ` if (crypto_box_curve25519xchacha20poly1305_keypair(epk, esk) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 67 | `crypto_box_curve25519xchacha20poly1305_seal_open` | `libsodium/crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:64`: ` if (clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES) {` | `return -1;` | [x] |
| 68 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | `libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:43`: ` if (crypto_scalarmult_curve25519(s, sk, pk) != 0) {` | `return -1;` | [x] |
| 69 | `crypto_box_curve25519xsalsa20poly1305` | `libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:82`: ` if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 70 | `crypto_box_curve25519xsalsa20poly1305_open` | `libsodium/crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:99`: ` if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) {` | `return -1;` | [x] |
| 71 | `crypto_core_ed25519_add` | `libsodium/crypto_core/ed25519/core_ed25519.c:36`: ` if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) {` | `return -1;` | [x] |
| 72 | `crypto_core_ed25519_sub` | `libsodium/crypto_core/ed25519/core_ed25519.c:52`: ` if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) {` | `return -1;` | [x] |
| 73 | `_string_to_points` | `libsodium/crypto_core/ed25519/core_ed25519.c:73`: ` if (n > 2U) {` | `abort(); /* LCOV_EXCL_LINE */` | [x] |
| 74 | `_string_to_points` | `libsodium/crypto_core/ed25519/core_ed25519.c:77`: ` if (core_h2c_string_to_hash(h_be, n * HASH_GE_L, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 75 | `crypto_core_ed25519_from_string` | `libsodium/crypto_core/ed25519/core_ed25519.c:109`: ` if (_string_to_points(px, 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` | `return -1;` | [x] |
| 76 | `crypto_core_ed25519_scalar_from_string` | `libsodium/crypto_core/ed25519/core_ed25519.c:251`: ` if (core_h2c_string_to_hash(h_be, sizeof h_be, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` | `return -1;` | [x] |
| 77 | `core_h2c_string_to_hash_sha256` | `libsodium/crypto_core/ed25519/core_h2c.c:26`: `unconditional at this source site or condition is more than 8 lines above` | `assert(h_len <= 0xff);` | [x] |
| 78 | `core_h2c_string_to_hash_sha512` | `libsodium/crypto_core/ed25519/core_h2c.c:82`: `unconditional at this source site or condition is more than 8 lines above` | `assert(h_len <= 0xff);` | [x] |
| 79 | `core_h2c_string_to_hash` | `libsodium/crypto_core/ed25519/core_h2c.c:130`: ` switch (hash_alg) {` | `errno = EINVAL;` | [x] |
| 80 | `core_h2c_string_to_hash` | `libsodium/crypto_core/ed25519/core_h2c.c:131`: ` switch (hash_alg) {` | `return -1;` | [x] |
| 81 | `crypto_core_ristretto255_add` | `libsodium/crypto_core/ed25519/core_ristretto255.c:34`: ` if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) {` | `return -1;` | [x] |
| 82 | `crypto_core_ristretto255_sub` | `libsodium/crypto_core/ed25519/core_ristretto255.c:50`: ` if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) {` | `return -1;` | [x] |
| 83 | `_string_to_element` | `libsodium/crypto_core/ed25519/core_ristretto255.c:76`: ` if (core_h2c_string_to_hash(h, sizeof h, ctx, ctx_len, msg, msg_len, hash_alg) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 84 | `ge25519_frombytes_negate_vartime` | `libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:395`: ` if (fe25519_iszero(p_root_check) == 0) {` | `return -1;` | [x] |
| 85 | `ge25519_elligator2` | `libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:2684`: ` if (ge25519_xmont_to_ymont(y, x) != 0) {` | `abort(); /* LCOV_EXCL_LINE */` | [x] |
| 86 | `ristretto255_frombytes` | `libsodium/crypto_core/ed25519/ref10/ed25519_ref10.c:2834`: ` if (ristretto255_is_canonical(s) == 0) {` | `return -1;` | [x] |
| 87 | `blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:131`: ` if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 88 | `blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:154`: ` if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 89 | `blake2b_init_key` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:185`: ` if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 90 | `blake2b_init_key` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:188`: ` if (!key \|\| !keylen \|\| keylen > BLAKE2B_KEYBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 91 | `blake2b_init_key` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:203`: ` if (blake2b_init_param(S, P) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 92 | `blake2b_init_key_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:223`: ` if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 93 | `blake2b_init_key_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:226`: ` if (!key \|\| !keylen \|\| keylen > BLAKE2B_KEYBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 94 | `blake2b_init_key_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:249`: ` if (blake2b_init_param(S, P) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 95 | `blake2b_final` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:297`: ` if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 96 | `blake2b_final` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:300`: ` if (blake2b_is_lastblock(S)) {` | `return -1;` | [x] |
| 97 | `blake2b_final` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:306`: ` if (S->buflen > BLAKE2B_BLOCKBYTES) {` | `assert(S->buflen <= BLAKE2B_BLOCKBYTES);` | [x] |
| 98 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:342`: ` if (NULL == in && inlen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 99 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:345`: ` if (NULL == out) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 100 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:348`: ` if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 101 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:351`: ` if (NULL == key && keylen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 102 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:354`: ` if (keylen > BLAKE2B_KEYBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 103 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:358`: ` if (blake2b_init_key(S, outlen, key, keylen) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 104 | `blake2b` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:362`: ` if (blake2b_init(S, outlen) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 105 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:380`: ` if (NULL == in && inlen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 106 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:383`: ` if (NULL == out) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 107 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:386`: ` if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 108 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:389`: ` if (NULL == key && keylen > 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 109 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:392`: ` if (keylen > BLAKE2B_KEYBYTES) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 110 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:397`: ` if (blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 111 | `blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/blake2b-ref.c:401`: ` if (blake2b_init_salt_personal(S, outlen, salt, personal) < 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 112 | `crypto_generichash_blake2b` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:18`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `return -1;` | [x] |
| 113 | `crypto_generichash_blake2b` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:20`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `assert(outlen <= UINT8_MAX);` | [x] |
| 114 | `crypto_generichash_blake2b` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:21`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `assert(keylen <= UINT8_MAX);` | [x] |
| 115 | `crypto_generichash_blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:35`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `return -1;` | [x] |
| 116 | `crypto_generichash_blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:37`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `assert(outlen <= UINT8_MAX);` | [x] |
| 117 | `crypto_generichash_blake2b_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:38`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) {` | `assert(keylen <= UINT8_MAX);` | [x] |
| 118 | `crypto_generichash_blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:52`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `return -1;` | [x] |
| 119 | `crypto_generichash_blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:54`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `assert(outlen <= UINT8_MAX);` | [x] |
| 120 | `crypto_generichash_blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:55`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `assert(keylen <= UINT8_MAX);` | [x] |
| 121 | `crypto_generichash_blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:59`: ` if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 122 | `crypto_generichash_blake2b_init` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:63`: ` if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 123 | `crypto_generichash_blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:76`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `return -1;` | [x] |
| 124 | `crypto_generichash_blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:78`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `assert(outlen <= UINT8_MAX);` | [x] |
| 125 | `crypto_generichash_blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:79`: ` if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) {` | `assert(keylen <= UINT8_MAX);` | [x] |
| 126 | `crypto_generichash_blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:83`: ` if (blake2b_init_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, salt, personal) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 127 | `crypto_generichash_blake2b_init_salt_personal` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:89`: ` if (blake2b_init_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, salt, personal) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 128 | `crypto_generichash_blake2b_final` | `libsodium/crypto_generichash/blake2b/ref/generichash_blake2b.c:107`: `unconditional at this source site or condition is more than 8 lines above` | `assert(outlen <= UINT8_MAX);` | [x] |
| 129 | `crypto_kdf_blake2b_derive_from_key` | `libsodium/crypto_kdf/blake2b/kdf_blake2b.c:45`: ` if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| subkey_len > crypto_kdf_blake2b_BYTES_MAX) {` | `errno = EINVAL;` | [x] |
| 130 | `crypto_kdf_blake2b_derive_from_key` | `libsodium/crypto_kdf/blake2b/kdf_blake2b.c:46`: ` if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| subkey_len > crypto_kdf_blake2b_BYTES_MAX) {` | `return -1;` | [x] |
| 131 | `crypto_kdf_hkdf_sha256_expand` | `libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:66`: ` if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX) {` | `errno = EINVAL;` | [x] |
| 132 | `crypto_kdf_hkdf_sha256_expand` | `libsodium/crypto_kdf/hkdf/kdf_hkdf_sha256.c:67`: ` if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX) {` | `return -1;` | [x] |
| 133 | `crypto_kdf_hkdf_sha512_expand` | `libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:66`: ` if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX) {` | `errno = EINVAL;` | [x] |
| 134 | `crypto_kdf_hkdf_sha512_expand` | `libsodium/crypto_kdf/hkdf/kdf_hkdf_sha512.c:67`: ` if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX) {` | `return -1;` | [x] |
| 135 | `mlkem768_ref_enc_deterministic` | `libsodium/crypto_kem/mlkem768/ref/kem_mlkem768_ref.c:746`: ` if (polyvec_is_canonical(&pkpv) == 0) {` | `return -1;` | [x] |
| 136 | `crypto_kem_xwing_enc_deterministic` | `libsodium/crypto_kem/xwing/kem_xwing.c:135`: ` if (crypto_kem_mlkem768_enc_deterministic(ct_mlkem, ss_mlkem, pk_mlkem, seed_mlkem) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 137 | `crypto_kem_xwing_enc_deterministic` | `libsodium/crypto_kem/xwing/kem_xwing.c:142`: ` if (crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0) {` | `return -1;                                 /* LCOV_EXCL_LINE */` | [x] |
| 138 | `crypto_kem_xwing_enc` | `libsodium/crypto_kem/xwing/kem_xwing.c:164`: ` if (crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed) != 0) {` | `return -1;                         /* LCOV_EXCL_LINE */` | [x] |
| 139 | `crypto_kem_xwing_dec` | `libsodium/crypto_kem/xwing/kem_xwing.c:191`: ` if (crypto_kem_mlkem768_dec(ss_mlkem, ct_mlkem, sk_mlkem) != 0) {` | `return -1;` | [x] |
| 140 | `crypto_kem_xwing_dec` | `libsodium/crypto_kem/xwing/kem_xwing.c:198`: ` if (crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0) {` | `return -1;` | [x] |
| 141 | `crypto_kx_client_session_keys` | `libsodium/crypto_kx/crypto_kx.c:52`: ` if (rx == NULL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 142 | `crypto_kx_client_session_keys` | `libsodium/crypto_kx/crypto_kx.c:55`: ` if (crypto_scalarmult(q, client_sk, server_pk) != 0) {` | `return -1;` | [x] |
| 143 | `crypto_kx_server_session_keys` | `libsodium/crypto_kx/crypto_kx.c:93`: ` if (rx == NULL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 144 | `crypto_kx_server_session_keys` | `libsodium/crypto_kx/crypto_kx.c:96`: ` if (crypto_scalarmult(q, server_sk, client_pk) != 0) {` | `return -1;` | [x] |
| 145 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:89`: ` if (region == NULL) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */` | [x] |
| 146 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:93`: ` if (m_cost == 0 \|\| memory_size / m_cost != sizeof(block)) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */` | [x] |
| 147 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:97`: ` if (*region == NULL) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */` | [x] |
| 148 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:109`: ` if ((errno = posix_memalign((void **) &base, 64, memory_size)) != 0) {` | `if ((errno = posix_memalign((void **) &base, 64, memory_size)) != 0) {` | [x] |
| 149 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:117`: ` if (memory_size + 63 < memory_size) {` | `errno = ENOMEM;` | [x] |
| 150 | `allocate_memory` | `libsodium/crypto_pwhash/argon2/argon2-core.c:128`: ` if (base == NULL) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR;` | [x] |
| 151 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:231`: ` if (NULL == context) {` | `return ARGON2_INCORRECT_PARAMETER;` | [x] |
| 152 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:235`: ` if (NULL == context->out) {` | `return ARGON2_OUTPUT_PTR_NULL;` | [x] |
| 153 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:240`: ` if (ARGON2_MIN_OUTLEN > context->outlen) {` | `return ARGON2_OUTPUT_TOO_SHORT;` | [x] |
| 154 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:244`: ` if (ARGON2_MAX_OUTLEN < context->outlen) {` | `return ARGON2_OUTPUT_TOO_LONG;` | [x] |
| 155 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:250`: ` if (0 != context->pwdlen) {` | `return ARGON2_PWD_PTR_MISMATCH;` | [x] |
| 156 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:255`: ` if (ARGON2_MIN_PWD_LENGTH > context->pwdlen) {` | `return ARGON2_PWD_TOO_SHORT;` | [x] |
| 157 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:259`: ` if (ARGON2_MAX_PWD_LENGTH < context->pwdlen) {` | `return ARGON2_PWD_TOO_LONG;` | [x] |
| 158 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:265`: ` if (0 != context->saltlen) {` | `return ARGON2_SALT_PTR_MISMATCH;` | [x] |
| 159 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:270`: ` if (ARGON2_MIN_SALT_LENGTH > context->saltlen) {` | `return ARGON2_SALT_TOO_SHORT;` | [x] |
| 160 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:274`: ` if (ARGON2_MAX_SALT_LENGTH < context->saltlen) {` | `return ARGON2_SALT_TOO_LONG;` | [x] |
| 161 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:280`: ` if (0 != context->secretlen) {` | `return ARGON2_SECRET_PTR_MISMATCH;` | [x] |
| 162 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:284`: ` if (ARGON2_MIN_SECRET > context->secretlen) {` | `return ARGON2_SECRET_TOO_SHORT;` | [x] |
| 163 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:288`: ` if (ARGON2_MAX_SECRET < context->secretlen) {` | `return ARGON2_SECRET_TOO_LONG;` | [x] |
| 164 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:295`: ` if (0 != context->adlen) {` | `return ARGON2_AD_PTR_MISMATCH;` | [x] |
| 165 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:299`: ` if (ARGON2_MIN_AD_LENGTH > context->adlen) {` | `return ARGON2_AD_TOO_SHORT;` | [x] |
| 166 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:303`: ` if (ARGON2_MAX_AD_LENGTH < context->adlen) {` | `return ARGON2_AD_TOO_LONG;` | [x] |
| 167 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:309`: ` if (ARGON2_MIN_LANES > context->lanes) {` | `return ARGON2_LANES_TOO_FEW;` | [x] |
| 168 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:313`: ` if (ARGON2_MAX_LANES < context->lanes) {` | `return ARGON2_LANES_TOO_MANY;` | [x] |
| 169 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:318`: ` if (ARGON2_MIN_MEMORY > context->m_cost) {` | `return ARGON2_MEMORY_TOO_LITTLE;` | [x] |
| 170 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:322`: ` if (ARGON2_MAX_MEMORY < context->m_cost) {` | `return ARGON2_MEMORY_TOO_MUCH;` | [x] |
| 171 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:326`: ` if (context->m_cost < 8 * context->lanes) {` | `return ARGON2_MEMORY_TOO_LITTLE;` | [x] |
| 172 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:331`: ` if (ARGON2_MIN_TIME > context->t_cost) {` | `return ARGON2_TIME_TOO_SMALL;` | [x] |
| 173 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:335`: ` if (ARGON2_MAX_TIME < context->t_cost) {` | `return ARGON2_TIME_TOO_LARGE;` | [x] |
| 174 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:340`: ` if (ARGON2_MIN_THREADS > context->threads) {` | `return ARGON2_THREADS_TOO_FEW;` | [x] |
| 175 | `argon2_validate_inputs` | `libsodium/crypto_pwhash/argon2/argon2-core.c:344`: ` if (ARGON2_MAX_THREADS < context->threads) {` | `return ARGON2_THREADS_TOO_MANY;` | [x] |
| 176 | `argon2_initialize` | `libsodium/crypto_pwhash/argon2/argon2-core.c:466`: ` if (instance == NULL \|\| context == NULL) {` | `return ARGON2_INCORRECT_PARAMETER; /* LCOV_EXCL_LINE */` | [x] |
| 177 | `argon2_initialize` | `libsodium/crypto_pwhash/argon2/argon2-core.c:473`: ` if ((instance->pseudo_rands = (uint64_t *)` | `return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */` | [x] |
| 178 | `decode_decimal` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:54`: ` if (acc > (ULONG_MAX / 10)) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 179 | `decode_decimal` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:58`: ` if ((unsigned long) c > (ULONG_MAX - acc)) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 180 | `decode_decimal` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:63`: ` if (str == orig \|\| (*orig == '0' && str != (orig + 1))) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 181 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:101`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_DECODING_FAIL;         \` | [x] |
| 182 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:124`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_DECODING_FAIL;   \` | [x] |
| 183 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:135`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_DECODING_FAIL;         \` | [x] |
| 184 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:149`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_DECODING_FAIL;                                         \` | [x] |
| 185 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:168`: ` if (type == Argon2_id) {` | `return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */` | [x] |
| 186 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:173`: ` if (version != ARGON2_VERSION_NUMBER) {` | `return ARGON2_INCORRECT_TYPE;` | [x] |
| 187 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:178`: ` if (ctx->m_cost > UINT32_MAX) {` | `return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */` | [x] |
| 188 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:183`: ` if (ctx->t_cost > UINT32_MAX) {` | `return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */` | [x] |
| 189 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:188`: ` if (ctx->lanes > UINT32_MAX) {` | `return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */` | [x] |
| 190 | `argon2_decode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:203`: ` if (*str == 0) {` | `return ARGON2_DECODING_FAIL;` | [x] |
| 191 | `argon2_encode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:248`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_ENCODING_FAIL; \` | [x] |
| 192 | `argon2_encode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:267`: `unconditional at this source site or condition is more than 8 lines above` | `return ARGON2_ENCODING_FAIL;                                            \` | [x] |
| 193 | `argon2_encode_string` | `libsodium/crypto_pwhash/argon2/argon2-encoding.c:282`: ` switch (type) {` | `return ARGON2_ENCODING_FAIL; /* LCOV_EXCL_LINE */` | [x] |
| 194 | `argon2_ctx` | `libsodium/crypto_pwhash/argon2/argon2.c:41`: ` if (type != Argon2_id && type != Argon2_i) {` | `return ARGON2_INCORRECT_TYPE; /* LCOV_EXCL_LINE */` | [x] |
| 195 | `argon2_hash` | `libsodium/crypto_pwhash/argon2/argon2.c:102`: ` if (pwdlen > ARGON2_MAX_PWD_LENGTH) {` | `return ARGON2_PWD_TOO_LONG; /* LCOV_EXCL_LINE */` | [x] |
| 196 | `argon2_hash` | `libsodium/crypto_pwhash/argon2/argon2.c:106`: ` if (hashlen > ARGON2_MAX_OUTLEN) {` | `return ARGON2_OUTPUT_TOO_LONG; /* LCOV_EXCL_LINE */` | [x] |
| 197 | `argon2_hash` | `libsodium/crypto_pwhash/argon2/argon2.c:110`: ` if (saltlen > ARGON2_MAX_SALT_LENGTH) {` | `return ARGON2_SALT_TOO_LONG; /* LCOV_EXCL_LINE */` | [x] |
| 198 | `argon2_hash` | `libsodium/crypto_pwhash/argon2/argon2.c:115`: ` if (!out) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR; /* LCOV_EXCL_LINE */` | [x] |
| 199 | `argon2_hash` | `libsodium/crypto_pwhash/argon2/argon2.c:152`: ` if (argon2_encode_string(encoded, encodedlen, &context, type) != ARGON2_OK) {` | `return ARGON2_ENCODING_FAIL;` | [x] |
| 200 | `argon2_verify` | `libsodium/crypto_pwhash/argon2/argon2.c:230`: ` if (encoded_len > UINT32_MAX) {` | `return ARGON2_DECODING_LENGTH_FAIL; /* LCOV_EXCL_LINE */` | [x] |
| 201 | `argon2_verify` | `libsodium/crypto_pwhash/argon2/argon2.c:244`: ` if (!ctx.out \|\| !ctx.salt \|\| !ctx.ad) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR;` | [x] |
| 202 | `argon2_verify` | `libsodium/crypto_pwhash/argon2/argon2.c:253`: ` if (!out) {` | `return ARGON2_MEMORY_ALLOCATION_ERROR;` | [x] |
| 203 | `blake2b_long` | `libsodium/crypto_pwhash/argon2/blake2b-long.c:21`: ` if (outlen > UINT32_MAX) {` | `goto fail; /* LCOV_EXCL_LINE */` | [x] |
| 204 | `blake2b_long` | `libsodium/crypto_pwhash/argon2/blake2b-long.c:31`: `unconditional at this source site or condition is more than 8 lines above` | `goto fail;   \` | [x] |
| 205 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:147`: ` if (outlen > crypto_pwhash_argon2i_BYTES_MAX) {` | `errno = EFBIG; /* LCOV_EXCL_LINE */` | [x] |
| 206 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:148`: ` if (outlen > crypto_pwhash_argon2i_BYTES_MAX) {` | `return -1;     /* LCOV_EXCL_LINE */` | [x] |
| 207 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:151`: ` if (outlen < crypto_pwhash_argon2i_BYTES_MIN) {` | `errno = EINVAL;` | [x] |
| 208 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:152`: ` if (outlen < crypto_pwhash_argon2i_BYTES_MIN) {` | `return -1;` | [x] |
| 209 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:157`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` | `errno = EFBIG;` | [x] |
| 210 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:158`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` | `return -1;` | [x] |
| 211 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:163`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` | `errno = EINVAL;` | [x] |
| 212 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:164`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` | `return -1;` | [x] |
| 213 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:167`: ` if ((const void *) out == (const void *) passwd) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 214 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:168`: ` if ((const void *) out == (const void *) passwd) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 215 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:176`: ` if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 216 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:180`: ` if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` | `errno = EINVAL;` | [x] |
| 217 | `crypto_pwhash_argon2i` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:181`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 218 | `crypto_pwhash_argon2i_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:197`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` | `errno = EFBIG;` | [x] |
| 219 | `crypto_pwhash_argon2i_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:198`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) {` | `return -1;` | [x] |
| 220 | `crypto_pwhash_argon2i_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:203`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` | `errno = EINVAL;` | [x] |
| 221 | `crypto_pwhash_argon2i_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:204`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) {` | `return -1;` | [x] |
| 222 | `crypto_pwhash_argon2i_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:211`: ` if (argon2i_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2i_STRBYTES) != ARGON2_OK) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 223 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:224`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX) {` | `errno = EFBIG;` | [x] |
| 224 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:225`: ` if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX) {` | `return -1;` | [x] |
| 225 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:229`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN) {` | `errno = EINVAL;` | [x] |
| 226 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:230`: ` if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN) {` | `return -1;` | [x] |
| 227 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:239`: ` if (verify_ret == ARGON2_VERIFY_MISMATCH) {` | `errno = EINVAL;` | [x] |
| 228 | `crypto_pwhash_argon2i_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:241`: ` if (verify_ret == ARGON2_VERIFY_MISMATCH) {` | `return -1;` | [x] |
| 229 | `_needs_rehash` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:257`: ` if (opslimit > UINT32_MAX \|\| memlimit > UINT32_MAX \|\| fodder_len >= crypto_pwhash_STRBYTES) {` | `errno = EINVAL;` | [x] |
| 230 | `_needs_rehash` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:258`: ` if (opslimit > UINT32_MAX \|\| memlimit > UINT32_MAX \|\| fodder_len >= crypto_pwhash_STRBYTES) {` | `return -1;` | [x] |
| 231 | `_needs_rehash` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:262`: ` if ((fodder = (unsigned char *) calloc(fodder_len, 1U)) == NULL) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 232 | `_needs_rehash` | `libsodium/crypto_pwhash/argon2/pwhash_argon2i.c:269`: ` if (argon2_decode_string(&ctx, str, type) != 0) {` | `errno = EINVAL;` | [x] |
| 233 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:143`: ` if (outlen > crypto_pwhash_argon2id_BYTES_MAX) {` | `errno = EFBIG; /* LCOV_EXCL_LINE */` | [x] |
| 234 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:144`: ` if (outlen > crypto_pwhash_argon2id_BYTES_MAX) {` | `return -1;     /* LCOV_EXCL_LINE */` | [x] |
| 235 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:147`: ` if (outlen < crypto_pwhash_argon2id_BYTES_MIN) {` | `errno = EINVAL;` | [x] |
| 236 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:148`: ` if (outlen < crypto_pwhash_argon2id_BYTES_MIN) {` | `return -1;` | [x] |
| 237 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:153`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` | `errno = EFBIG;` | [x] |
| 238 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:154`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` | `return -1;` | [x] |
| 239 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:159`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` | `errno = EINVAL;` | [x] |
| 240 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:160`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` | `return -1;` | [x] |
| 241 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:163`: ` if ((const void *) out == (const void *) passwd) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 242 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:164`: ` if ((const void *) out == (const void *) passwd) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 243 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:172`: ` if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 244 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:176`: ` if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) {` | `errno = EINVAL;` | [x] |
| 245 | `crypto_pwhash_argon2id` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:177`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 246 | `crypto_pwhash_argon2id_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:193`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` | `errno = EFBIG;` | [x] |
| 247 | `crypto_pwhash_argon2id_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:194`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) {` | `return -1;` | [x] |
| 248 | `crypto_pwhash_argon2id_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:199`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` | `errno = EINVAL;` | [x] |
| 249 | `crypto_pwhash_argon2id_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:200`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) {` | `return -1;` | [x] |
| 250 | `crypto_pwhash_argon2id_str` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:207`: ` if (argon2id_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2id_STRBYTES) != ARGON2_OK) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 251 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:220`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX) {` | `errno = EFBIG; /* LCOV_EXCL_LINE */` | [x] |
| 252 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:221`: ` if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX) {` | `return -1;     /* LCOV_EXCL_LINE */` | [x] |
| 253 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:225`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN) {` | `errno = EINVAL;` | [x] |
| 254 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:226`: ` if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN) {` | `return -1;` | [x] |
| 255 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:235`: ` if (verify_ret == ARGON2_VERIFY_MISMATCH) {` | `errno = EINVAL;` | [x] |
| 256 | `crypto_pwhash_argon2id_str_verify` | `libsodium/crypto_pwhash/argon2/pwhash_argon2id.c:237`: ` if (verify_ret == ARGON2_VERIFY_MISMATCH) {` | `return -1;` | [x] |
| 257 | `crypto_pwhash` | `libsodium/crypto_pwhash/crypto_pwhash.c:142`: ` switch (alg) {` | `errno = EINVAL;` | [x] |
| 258 | `crypto_pwhash` | `libsodium/crypto_pwhash/crypto_pwhash.c:143`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 259 | `crypto_pwhash_str_alg` | `libsodium/crypto_pwhash/crypto_pwhash.c:169`: ` switch (alg) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 260 | `crypto_pwhash_str_alg` | `libsodium/crypto_pwhash/crypto_pwhash.c:171`: `unconditional at this source site or condition is more than 8 lines above` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 261 | `crypto_pwhash_str_verify` | `libsodium/crypto_pwhash/crypto_pwhash.c:187`: ` if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` | `errno = EINVAL;` | [x] |
| 262 | `crypto_pwhash_str_verify` | `libsodium/crypto_pwhash/crypto_pwhash.c:189`: ` if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` | `return -1;` | [x] |
| 263 | `crypto_pwhash_str_needs_rehash` | `libsodium/crypto_pwhash/crypto_pwhash.c:204`: ` if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` | `errno = EINVAL;` | [x] |
| 264 | `crypto_pwhash_str_needs_rehash` | `libsodium/crypto_pwhash/crypto_pwhash.c:206`: ` if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) {` | `return -1;` | [x] |
| 265 | `encode64_uint32` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:41`: ` if (dstlen < 1) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 266 | `encode64` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:66`: ` if (!dnext) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 267 | `decode64_one` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:85`: ` if (ptr) {` | `return -1;` | [x] |
| 268 | `decode64_uint32` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:99`: ` if (decode64_one(&one, *src)) {` | `return NULL;` | [x] |
| 269 | `escrypt_parse_setting` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:116`: ` if (setting[0] != '$' \|\| setting[1] != '7' \|\| setting[2] != '$') {` | `return NULL;` | [x] |
| 270 | `escrypt_parse_setting` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:121`: ` if (decode64_one(N_log2_p, *src)) {` | `return NULL;` | [x] |
| 271 | `escrypt_parse_setting` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:127`: ` if (!src) {` | `return NULL;` | [x] |
| 272 | `escrypt_parse_setting` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:132`: ` if (!src) {` | `return NULL;` | [x] |
| 273 | `escrypt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:160`: ` if (!src) {` | `return NULL;` | [x] |
| 274 | `escrypt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:175`: ` if (buf == NULL \|\| need > buflen \|\| need < saltlen) {` | `return NULL;` | [x] |
| 275 | `escrypt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:185`: ` if (escrypt_kdf(local, passwd, passwdlen, salt, saltlen, N, r, p, hash, sizeof(hash))) {` | `return NULL;` | [x] |
| 276 | `escrypt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:195`: ` if (!dst \|\| dst >= buf + buflen) {` | `return NULL; /* Can't happen LCOV_EXCL_LINE */` | [x] |
| 277 | `escrypt_gensalt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:214`: ` if (need > buflen \|\| need < saltlen \|\| saltlen < srclen) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 278 | `escrypt_gensalt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:217`: ` if (N_log2 > 63 \|\| ((uint64_t) r * (uint64_t) p >= (1U << 30))) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 279 | `escrypt_gensalt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:228`: ` if (!dst) {` | `return NULL; /* Can't happen LCOV_EXCL_LINE */` | [x] |
| 280 | `escrypt_gensalt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:232`: ` if (!dst) {` | `return NULL; /* Can't happen LCOV_EXCL_LINE */` | [x] |
| 281 | `escrypt_gensalt_r` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:236`: ` if (!dst \|\| dst >= buf + buflen) {` | `return NULL; /* Can't happen LCOV_EXCL_LINE */` | [x] |
| 282 | `crypto_pwhash_scryptsalsa208sha256_ll` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:254`: ` if (escrypt_init_local(&local)) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 283 | `crypto_pwhash_scryptsalsa208sha256_ll` | `libsodium/crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:265`: ` if (escrypt_free_local(&local)) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 284 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:249`: ` if (buflen > (((uint64_t)(1) << 32) - 1) * 32) {` | `errno = EFBIG;` | [x] |
| 285 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:250`: ` if (buflen > (((uint64_t)(1) << 32) - 1) * 32) {` | `return -1;` | [x] |
| 286 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:254`: ` if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) {` | `errno = EFBIG;` | [x] |
| 287 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:255`: ` if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) {` | `return -1;` | [x] |
| 288 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:258`: ` if (N > UINT32_MAX) {` | `errno = EFBIG;` | [x] |
| 289 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:259`: ` if (N > UINT32_MAX) {` | `return -1;` | [x] |
| 290 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:262`: ` if (((N & (N - 1)) != 0) \|\| (N < 2)) {` | `errno = EINVAL;` | [x] |
| 291 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:263`: ` if (((N & (N - 1)) != 0) \|\| (N < 2)) {` | `return -1;` | [x] |
| 292 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:266`: ` if (r == 0 \|\| p == 0) {` | `errno = EINVAL;` | [x] |
| 293 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:267`: ` if (r == 0 \|\| p == 0) {` | `return -1;` | [x] |
| 294 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:274`: ` if ((r > SIZE_MAX / 128 / p) \|\| #if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| #endif (N > SIZE_MAX / 128 / r)) {` | `errno = ENOMEM;` | [x] |
| 295 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:275`: ` if ((r > SIZE_MAX / 128 / p) \|\| #if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| #endif (N > SIZE_MAX / 128 / r)) {` | `return -1;` | [x] |
| 296 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:283`: ` if (need < V_size) {` | `errno = ENOMEM;` | [x] |
| 297 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:284`: ` if (need < V_size) {` | `return -1;` | [x] |
| 298 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:289`: ` if (need < XY_size) {` | `errno = ENOMEM;` | [x] |
| 299 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:290`: ` if (need < XY_size) {` | `return -1;` | [x] |
| 300 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:294`: ` if (escrypt_free_region(local)) {` | `return -1;` | [x] |
| 301 | `escrypt_kdf_nosse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:297`: ` if (!escrypt_alloc_region(local, need)) {` | `return -1;` | [x] |
| 302 | `escrypt_PBKDF2_SHA256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c:64`: ` if (dkLen > 0x1fffffffe0ULL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 303 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:172`: ` if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\| outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX) {` | `errno = EFBIG; /* LCOV_EXCL_LINE */` | [x] |
| 304 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:173`: ` if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\| outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX) {` | `return -1;     /* LCOV_EXCL_LINE */` | [x] |
| 305 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:177`: ` if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 306 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:178`: ` if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 307 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:181`: ` if ((const void *) out == (const void *) passwd) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 308 | `crypto_pwhash_scryptsalsa208sha256` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:182`: ` if ((const void *) out == (const void *) passwd) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 309 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:205`: ` if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX) {` | `errno = EFBIG; /* LCOV_EXCL_LINE */` | [x] |
| 310 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:206`: ` if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX) {` | `return -1;     /* LCOV_EXCL_LINE */` | [x] |
| 311 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:210`: ` if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 312 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:211`: ` if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 313 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:216`: ` if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting, sizeof setting) == NULL) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 314 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:217`: ` if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting, sizeof setting) == NULL) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 315 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:220`: ` if (escrypt_init_local(&escrypt_local) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 316 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:227`: ` if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) setting, (uint8_t *) out, crypto_pwhash_scryptsalsa208sha256_STRBYTES) == NULL) {` | `errno = EINVAL;` | [x] |
| 317 | `crypto_pwhash_scryptsalsa208sha256_str` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:228`: ` if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) setting, (uint8_t *) out, crypto_pwhash_scryptsalsa208sha256_STRBYTES) == NULL) {` | `return -1;` | [x] |
| 318 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:255`: ` if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) {` | `return -1;` | [x] |
| 319 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:258`: ` if (escrypt_init_local(&escrypt_local) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 320 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:265`: ` if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) str, (uint8_t *) wanted, sizeof wanted) == NULL) {` | `return -1;` | [x] |
| 321 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:284`: ` if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 322 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:285`: ` if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 323 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:289`: ` if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) {` | `errno = EINVAL;` | [x] |
| 324 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:290`: ` if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) {` | `return -1;` | [x] |
| 325 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:294`: ` if (escrypt_parse_setting((const uint8_t *) str, &N_log2_, &r_, &p_) == NULL) {` | `errno = EINVAL; /* LCOV_EXCL_LINE */` | [x] |
| 326 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `libsodium/crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:295`: ` if (escrypt_parse_setting((const uint8_t *) str, &N_log2_, &r_, &p_) == NULL) {` | `return -1;      /* LCOV_EXCL_LINE */` | [x] |
| 327 | `escrypt_alloc_region` | `libsodium/crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c:56`: ` if ((errno = posix_memalign((void **) &base, 64, size)) != 0) {` | `if ((errno = posix_memalign((void **) &base, 64, size)) != 0) {` | [x] |
| 328 | `escrypt_alloc_region` | `libsodium/crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c:63`: ` if (size + 63 < size) {` | `errno = ENOMEM;` | [x] |
| 329 | `escrypt_free_region` | `libsodium/crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c:89`: ` if (munmap(region->base, region->size)) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 330 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:324`: ` if (buflen > (((uint64_t)(1) << 32) - 1) * 32) {` | `errno = EFBIG;` | [x] |
| 331 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:325`: ` if (buflen > (((uint64_t)(1) << 32) - 1) * 32) {` | `return -1;` | [x] |
| 332 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:330`: ` if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) {` | `errno = EFBIG;` | [x] |
| 333 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:331`: ` if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) {` | `return -1;` | [x] |
| 334 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:334`: ` if (N > UINT32_MAX) {` | `errno = EFBIG;` | [x] |
| 335 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:335`: ` if (N > UINT32_MAX) {` | `return -1;` | [x] |
| 336 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:338`: ` if (((N & (N - 1)) != 0) \|\| (N < 2)) {` | `errno = EINVAL;` | [x] |
| 337 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:339`: ` if (((N & (N - 1)) != 0) \|\| (N < 2)) {` | `return -1;` | [x] |
| 338 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:342`: ` if (r == 0 \|\| p == 0) {` | `errno = EINVAL;` | [x] |
| 339 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:343`: ` if (r == 0 \|\| p == 0) {` | `return -1;` | [x] |
| 340 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:351`: ` if ((r > SIZE_MAX / 128 / p) \|\| # if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| # endif (N > SIZE_MAX / 128 / r)) {` | `errno = ENOMEM;` | [x] |
| 341 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:352`: ` if ((r > SIZE_MAX / 128 / p) \|\| # if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| # endif (N > SIZE_MAX / 128 / r)) {` | `return -1;` | [x] |
| 342 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:362`: ` if (need < V_size) {` | `errno = ENOMEM;` | [x] |
| 343 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:363`: ` if (need < V_size) {` | `return -1;` | [x] |
| 344 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:370`: ` if (need < XY_size) {` | `errno = ENOMEM;` | [x] |
| 345 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:371`: ` if (need < XY_size) {` | `return -1;` | [x] |
| 346 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:376`: ` if (escrypt_free_region(local)) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 347 | `escrypt_kdf_sse` | `libsodium/crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:379`: ` if (!escrypt_alloc_region(local, need)) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 348 | `crypto_scalarmult_curve25519_ref10` | `libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c:107`: ` if (has_small_order(p)) {` | `return -1;` | [x] |
| 349 | `crypto_scalarmult_curve25519` | `libsodium/crypto_scalarmult/curve25519/scalarmult_curve25519.c:22`: ` if (implementation->mult(q, n, p) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 350 | `_crypto_scalarmult_ed25519` | `libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:41`: ` if (ge25519_is_canonical(p) == 0 \|\| ge25519_frombytes(&P, p) != 0 \|\| ge25519_has_small_order(&P) != 0 \|\| ge25519_is_on_main_subgroup(&P) == 0) {` | `return -1;` | [x] |
| 351 | `_crypto_scalarmult_ed25519` | `libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:54`: ` if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) {` | `return -1;` | [x] |
| 352 | `_crypto_scalarmult_ed25519_base` | `libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:92`: ` if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) {` | `return -1;` | [x] |
| 353 | `crypto_scalarmult_ristretto255` | `libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:19`: ` if (ristretto255_frombytes(&P, p) != 0) {` | `return -1;` | [x] |
| 354 | `crypto_scalarmult_ristretto255` | `libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:28`: ` if (sodium_is_zero(q, 32)) {` | `return -1;` | [x] |
| 355 | `crypto_scalarmult_ristretto255_base` | `libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:48`: ` if (sodium_is_zero(q, 32)) {` | `return -1;` | [x] |
| 356 | `crypto_secretbox_easy` | `libsodium/crypto_secretbox/crypto_secretbox_easy.c:98`: ` if (mlen > crypto_secretbox_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 357 | `crypto_secretbox_open_detached` | `libsodium/crypto_secretbox/crypto_secretbox_easy.c:129`: ` if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) {` | `return -1;` | [x] |
| 358 | `crypto_secretbox_open_easy` | `libsodium/crypto_secretbox/crypto_secretbox_easy.c:171`: ` if (clen < crypto_secretbox_MACBYTES) {` | `return -1;` | [x] |
| 359 | `crypto_secretbox_xchacha20poly1305_easy` | `libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:90`: ` if (mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 360 | `crypto_secretbox_xchacha20poly1305_open_detached` | `libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:122`: ` if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) {` | `return -1;` | [x] |
| 361 | `crypto_secretbox_xchacha20poly1305_open_easy` | `libsodium/crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:165`: ` if (clen < crypto_secretbox_xchacha20poly1305_MACBYTES) {` | `return -1;` | [x] |
| 362 | `crypto_secretbox_xsalsa20poly1305` | `libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:16`: ` if (mlen < 32) {` | `return -1;` | [x] |
| 363 | `crypto_secretbox_xsalsa20poly1305_open` | `libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:36`: ` if (clen < 32) {` | `return -1;` | [x] |
| 364 | `crypto_secretbox_xsalsa20poly1305_open` | `libsodium/crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:41`: ` if (crypto_onetimeauth_poly1305_verify(c + 16, c + 32, clen - 32, subkey) != 0) {` | `return -1;` | [x] |
| 365 | `crypto_secretstream_xchacha20poly1305_push` | `libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:129`: ` if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 366 | `crypto_secretstream_xchacha20poly1305_pull` | `libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:202`: ` if (inlen < crypto_secretstream_xchacha20poly1305_ABYTES) {` | `return -1;` | [x] |
| 367 | `crypto_secretstream_xchacha20poly1305_pull` | `libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:206`: ` if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 368 | `crypto_secretstream_xchacha20poly1305_pull` | `libsodium/crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:241`: ` if (sodium_memcmp(mac, stored_mac, sizeof mac) != 0) {` | `return -1;` | [x] |
| 369 | `crypto_sign_ed25519_pk_to_curve25519` | `libsodium/crypto_sign/ed25519/ref10/keypair.c:56`: ` if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 \|\| ge25519_has_small_order(&A) != 0 \|\| ge25519_is_on_main_subgroup(&A) == 0) {` | `return -1;` | [x] |
| 370 | `_crypto_sign_ed25519_verify_detached` | `libsodium/crypto_sign/ed25519/ref10/open.c:32`: ` if (sig[63] & 224) {` | `return -1;` | [x] |
| 371 | `_crypto_sign_ed25519_verify_detached` | `libsodium/crypto_sign/ed25519/ref10/open.c:37`: ` if ((sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0) {` | `return -1;` | [x] |
| 372 | `_crypto_sign_ed25519_verify_detached` | `libsodium/crypto_sign/ed25519/ref10/open.c:40`: ` if (ge25519_is_canonical(pk) == 0) {` | `return -1;` | [x] |
| 373 | `_crypto_sign_ed25519_verify_detached` | `libsodium/crypto_sign/ed25519/ref10/open.c:45`: ` if (ge25519_frombytes_negate_vartime(&A, pk) != 0 \|\| ge25519_has_small_order(&A) != 0) {` | `return -1;` | [x] |
| 374 | `_crypto_sign_ed25519_verify_detached` | `libsodium/crypto_sign/ed25519/ref10/open.c:49`: ` if (ge25519_frombytes(&expected_r, sig) != 0 \|\| ge25519_has_small_order(&expected_r) != 0) {` | `return -1;` | [x] |
| 375 | `crypto_sign_ed25519_open` | `libsodium/crypto_sign/ed25519/ref10/open.c:103`: ` if (mlen_p != NULL) {` | `return -1;` | [x] |
| 376 | `crypto_sign_ed25519` | `libsodium/crypto_sign/ed25519/ref10/sign.c:120`: ` if (smlen_p != NULL) {` | `return -1;` | [x] |
| 377 | `crypto_stream_chacha20` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:68`: ` if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 378 | `crypto_stream_chacha20_xor_ic` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:80`: ` if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 379 | `crypto_stream_chacha20_xor` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:91`: ` if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 380 | `crypto_stream_chacha20_ietf_ext` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:101`: ` if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 381 | `crypto_stream_chacha20_ietf_ext_xor_ic` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:113`: ` if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 382 | `crypto_stream_chacha20_ietf_ext_xor` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:124`: ` if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 383 | `crypto_stream_chacha20_ietf` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:134`: ` if (clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 384 | `crypto_stream_chacha20_ietf_xor_ic` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:147`: ` if ((unsigned long long) ic > (64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 385 | `crypto_stream_chacha20_ietf_xor` | `libsodium/crypto_stream/chacha20/stream_chacha20.c:158`: ` if (mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 386 | `<file scope>` | `libsodium/include/sodium/core.h:21`: `unconditional at this source site or condition is more than 8 lines above` | `void sodium_misuse(void)` | [x] |
| 387 | `sodium_hrtime` | `libsodium/randombytes/internal/randombytes_internal_random.c:173`: ` if (gettimeofday(&tv, NULL) != 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 388 | `randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:198`: ` if (CCRandomGenerateBytes(buf, size) != kCCSuccess) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 389 | `_randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:208`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= 256U);` | [x] |
| 390 | `_randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:211`: ` if (&getentropy == NULL) {` | `errno = ENOSYS;` | [x] |
| 391 | `_randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:212`: ` if (&getentropy == NULL) {` | `return -1;` | [x] |
| 392 | `_randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:216`: ` if (getentropy(buf, size) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 393 | `randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:230`: ` if (size < chunk_size) {` | `assert(chunk_size > (size_t) 0U);` | [x] |
| 394 | `randombytes_getentropy` | `libsodium/randombytes/internal/randombytes_internal_random.c:233`: ` if (_randombytes_getentropy(buf, chunk_size) != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 395 | `_randombytes_linux_getrandom` | `libsodium/randombytes/internal/randombytes_internal_random.c:249`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= 256U);` | [x] |
| 396 | `_randombytes_linux_getrandom` | `libsodium/randombytes/internal/randombytes_internal_random.c:252`: `unconditional at this source site or condition is more than 8 lines above` | `} while (readnb < 0 && (errno == EINTR \|\| errno == EAGAIN));` | [x] |
| 397 | `randombytes_linux_getrandom` | `libsodium/randombytes/internal/randombytes_internal_random.c:266`: ` if (size < chunk_size) {` | `assert(chunk_size > (size_t) 0U);` | [x] |
| 398 | `randombytes_linux_getrandom` | `libsodium/randombytes/internal/randombytes_internal_random.c:269`: ` if (_randombytes_linux_getrandom(buf, chunk_size) != 0) {` | `return -1;` | [x] |
| 399 | `randombytes_block_on_dev_random` | `libsodium/randombytes/internal/randombytes_internal_random.c:298`: ` if (fd == -1) {` | `} while (pret < 0 && (errno == EINTR \|\| errno == EAGAIN));` | [x] |
| 400 | `randombytes_block_on_dev_random` | `libsodium/randombytes/internal/randombytes_internal_random.c:301`: ` if (pret != 1) {` | `errno = EIO;` | [x] |
| 401 | `randombytes_block_on_dev_random` | `libsodium/randombytes/internal/randombytes_internal_random.c:302`: ` if (pret != 1) {` | `return -1;` | [x] |
| 402 | `randombytes_internal_random_random_dev_open` | `libsodium/randombytes/internal/randombytes_internal_random.c:324`: ` if (randombytes_block_on_dev_random() != 0) {` | `return -1;` | [x] |
| 403 | `randombytes_internal_random_random_dev_open` | `libsodium/randombytes/internal/randombytes_internal_random.c:337`: ` if (fstat(fd, &st) == 0 && (S_ISNAM(st.st_mode) \|\| S_ISCHR(st.st_mode))) {` | `} else if (errno == EINTR) {` | [x] |
| 404 | `randombytes_internal_random_random_dev_open` | `libsodium/randombytes/internal/randombytes_internal_random.c:343`: `unconditional at this source site or condition is more than 8 lines above` | `errno = EIO;` | [x] |
| 405 | `randombytes_internal_random_random_dev_open` | `libsodium/randombytes/internal/randombytes_internal_random.c:344`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 406 | `safe_read` | `libsodium/randombytes/internal/randombytes_internal_random.c:354`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size > (size_t) 0U);` | [x] |
| 407 | `safe_read` | `libsodium/randombytes/internal/randombytes_internal_random.c:355`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= SSIZE_MAX);` | [x] |
| 408 | `safe_read` | `libsodium/randombytes/internal/randombytes_internal_random.c:358`: `unconditional at this source site or condition is more than 8 lines above` | `(errno == EINTR \|\| errno == EAGAIN)); /* LCOV_EXCL_LINE */` | [x] |
| 409 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:389`: ` if (randombytes_getentropy(fodder, sizeof fodder) == 0) {` | `errno = errno_save;` | [x] |
| 410 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:399`: ` if (randombytes_linux_getrandom(fodder, sizeof fodder) == 0) {` | `errno = errno_save;` | [x] |
| 411 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:406`: `unconditional at this source site or condition is more than 8 lines above` | `assert((global.getentropy_available \| global.getrandom_available) == 0);` | [x] |
| 412 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:409`: ` if ((global.random_data_source_fd = randombytes_internal_random_random_dev_open()) == -1) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 413 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:411`: ` if ((global.random_data_source_fd = randombytes_internal_random_random_dev_open()) == -1) {` | `errno = errno_save;` | [x] |
| 414 | `randombytes_internal_random_init` | `libsodium/randombytes/internal/randombytes_internal_random.c:416`: `unconditional at this source site or condition is more than 8 lines above` | `sodium_misuse();` | [x] |
| 415 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:430`: `unconditional at this source site or condition is more than 8 lines above` | `assert(stream.nonce != (uint64_t) 0U);` | [x] |
| 416 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:446`: ` if (randombytes_getentropy(stream.key, sizeof stream.key) != 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 417 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:452`: ` if (randombytes_linux_getrandom(stream.key, sizeof stream.key) != 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 418 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:461`: ` if (global.random_data_source_fd == -1 \|\| safe_read(global.random_data_source_fd, stream.key, sizeof stream.key) != (ssize_t) sizeof stream.key) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 419 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:464`: ` if (global.random_data_source_fd == -1 \|\| safe_read(global.random_data_source_fd, stream.key, sizeof stream.key) != (ssize_t) sizeof stream.key) {` | `sodium_misuse();` | [x] |
| 420 | `randombytes_internal_random_stir` | `libsodium/randombytes/internal/randombytes_internal_random.c:469`: ` if (! RtlGenRandom((PVOID) stream.key, (ULONG) sizeof stream.key)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 421 | `randombytes_internal_random_stir_if_needed` | `libsodium/randombytes/internal/randombytes_internal_random.c:487`: ` if (stream.initialized == 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 422 | `randombytes_internal_random_buf` | `libsodium/randombytes/internal/randombytes_internal_random.c:599`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= ULLONG_MAX);` | [x] |
| 423 | `randombytes_internal_random_buf` | `libsodium/randombytes/internal/randombytes_internal_random.c:604`: `unconditional at this source site or condition is more than 8 lines above` | `assert(ret == 0);` | [x] |
| 424 | `randombytes_internal_random` | `libsodium/randombytes/internal/randombytes_internal_random.c:636`: ` if (stream.rnd32_outleft <= (size_t) 0U) {` | `assert(ret == 0);` | [x] |
| 425 | `randombytes_buf_deterministic` | `libsodium/randombytes/randombytes.c:222`: ` if (size > 0x4000000000ULL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 426 | `randombytes` | `libsodium/randombytes/randombytes.c:247`: `unconditional at this source site or condition is more than 8 lines above` | `assert(buf_len <= SIZE_MAX);` | [x] |
| 427 | `safe_read` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:134`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size > (size_t) 0U);` | [x] |
| 428 | `safe_read` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:135`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= SSIZE_MAX);` | [x] |
| 429 | `safe_read` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:138`: `unconditional at this source site or condition is more than 8 lines above` | `(errno == EINTR \|\| errno == EAGAIN)); /* LCOV_EXCL_LINE */` | [x] |
| 430 | `randombytes_block_on_dev_random` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:169`: ` if (fd == -1) {` | `} while (pret < 0 && (errno == EINTR \|\| errno == EAGAIN));` | [x] |
| 431 | `randombytes_block_on_dev_random` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:172`: ` if (pret != 1) {` | `errno = EIO;` | [x] |
| 432 | `randombytes_block_on_dev_random` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:173`: ` if (pret != 1) {` | `return -1;` | [x] |
| 433 | `randombytes_sysrandom_random_dev_open` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:195`: ` if (randombytes_block_on_dev_random() != 0) {` | `return -1;` | [x] |
| 434 | `randombytes_sysrandom_random_dev_open` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:216`: `unconditional at this source site or condition is more than 8 lines above` | `} else if (errno == EINTR) {` | [x] |
| 435 | `randombytes_sysrandom_random_dev_open` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:222`: `unconditional at this source site or condition is more than 8 lines above` | `errno = EIO;` | [x] |
| 436 | `randombytes_sysrandom_random_dev_open` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:223`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 437 | `_randombytes_linux_getrandom` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:233`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= 256U);` | [x] |
| 438 | `_randombytes_linux_getrandom` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:236`: `unconditional at this source site or condition is more than 8 lines above` | `} while (readnb < 0 && (errno == EINTR \|\| errno == EAGAIN));` | [x] |
| 439 | `randombytes_linux_getrandom` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:250`: ` if (size < chunk_size) {` | `assert(chunk_size > (size_t) 0U);` | [x] |
| 440 | `randombytes_linux_getrandom` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:253`: ` if (_randombytes_linux_getrandom(buf, chunk_size) != 0) {` | `return -1;` | [x] |
| 441 | `randombytes_sysrandom_init` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:274`: ` if (randombytes_linux_getrandom(fodder, sizeof fodder) == 0) {` | `errno = errno_save;` | [x] |
| 442 | `randombytes_sysrandom_init` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:283`: ` if ((stream.random_data_source_fd = randombytes_sysrandom_random_dev_open()) == -1) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 443 | `randombytes_sysrandom_init` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:285`: ` if ((stream.random_data_source_fd = randombytes_sysrandom_random_dev_open()) == -1) {` | `errno = errno_save;` | [x] |
| 444 | `randombytes_sysrandom_buf` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:346`: `unconditional at this source site or condition is more than 8 lines above` | `assert(size <= ULLONG_MAX);` | [x] |
| 445 | `randombytes_sysrandom_buf` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:353`: ` if (randombytes_linux_getrandom(buf, size) != 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 446 | `randombytes_sysrandom_buf` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:360`: ` if (stream.random_data_source_fd == -1 \|\| safe_read(stream.random_data_source_fd, buf, size) != (ssize_t) size) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 447 | `randombytes_sysrandom_buf` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:365`: ` if (size > (size_t) 0xffffffffUL) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 448 | `randombytes_sysrandom_buf` | `libsodium/randombytes/sysrandom/randombytes_sysrandom.c:368`: ` if (! RtlGenRandom((PVOID) buf, (ULONG) size)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 449 | `sodium_bin2hex` | `libsodium/sodium/codecs.c:24`: ` if (bin_len >= SIZE_MAX / 2 \|\| hex_maxlen <= bin_len * 2U) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 450 | `sodium_hex2bin` | `libsodium/sodium/codecs.c:73`: ` if (bin_pos >= bin_maxlen) {` | `errno = ERANGE;` | [x] |
| 451 | `sodium_hex2bin` | `libsodium/sodium/codecs.c:86`: ` if (state != 0U) {` | `errno = EINVAL;` | [x] |
| 452 | `sodium_hex2bin` | `libsodium/sodium/codecs.c:95`: ` if (hex_end != NULL) {` | `errno = EINVAL;` | [x] |
| 453 | `sodium_base64_check_variant` | `libsodium/sodium/codecs.c:169`: ` if ((((unsigned int) variant) & ~ 0x6U) != 0x1U) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 454 | `sodium_base64_encoded_len` | `libsodium/sodium/codecs.c:179`: ` if (bin_len / 3 > (SIZE_MAX - 5) / 4) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 455 | `sodium_bin2base64` | `libsodium/sodium/codecs.c:200`: ` if (nibbles > (SIZE_MAX - 5) / 4) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 456 | `sodium_bin2base64` | `libsodium/sodium/codecs.c:212`: ` if (b64_maxlen <= b64_len) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 457 | `sodium_bin2base64` | `libsodium/sodium/codecs.c:239`: ` if (acc_len > 0) {` | `assert(b64_pos <= b64_len);` | [x] |
| 458 | `_sodium_base642bin_skip_padding` | `libsodium/sodium/codecs.c:259`: ` if (*b64_pos_p >= b64_len) {` | `errno = ERANGE;` | [x] |
| 459 | `_sodium_base642bin_skip_padding` | `libsodium/sodium/codecs.c:260`: ` if (*b64_pos_p >= b64_len) {` | `return -1;` | [x] |
| 460 | `_sodium_base642bin_skip_padding` | `libsodium/sodium/codecs.c:267`: ` if (c == '=') {` | `errno = EINVAL;` | [x] |
| 461 | `_sodium_base642bin_skip_padding` | `libsodium/sodium/codecs.c:268`: ` if (c == '=') {` | `return -1;` | [x] |
| 462 | `sodium_base642bin` | `libsodium/sodium/codecs.c:311`: ` if (bin_pos >= bin_maxlen) {` | `errno = ERANGE;` | [x] |
| 463 | `sodium_base642bin` | `libsodium/sodium/codecs.c:336`: ` if (b64_end != NULL) {` | `errno = EINVAL;` | [x] |
| 464 | `ip_hex_digit` | `libsodium/sodium/codecs.c:354`: ` if (((unsigned int) ch \| 32U) >= 'a' && ((unsigned int) ch \| 32U) <= 'f') {` | `return -1;` | [x] |
| 465 | `parse_ipv4` | `libsodium/sodium/codecs.c:364`: ` if (src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end) {` | `return 0;` | [x] |
| 466 | `parse_ipv4` | `libsodium/sodium/codecs.c:373`: ` if (++digits > 3 \|\| val > 255U) {` | `return 0;` | [x] |
| 467 | `parse_ipv4` | `libsodium/sodium/codecs.c:377`: ` if (digits == 0) {` | `return 0;` | [x] |
| 468 | `parse_ipv4` | `libsodium/sodium/codecs.c:383`: ` if (p >= end \|\| *p++ != '.') {` | `return 0;` | [x] |
| 469 | `parse_ipv6` | `libsodium/sodium/codecs.c:406`: ` if (src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end) {` | `return 0;` | [x] |
| 470 | `parse_ipv6` | `libsodium/sodium/codecs.c:410`: ` if (++p >= end \|\| *p != ':') {` | `return 0;` | [x] |
| 471 | `parse_ipv6` | `libsodium/sodium/codecs.c:421`: ` if (colonp != NULL) {` | `return 0;` | [x] |
| 472 | `parse_ipv6` | `libsodium/sodium/codecs.c:428`: ` if (tp + 2 > endp) {` | `return 0;` | [x] |
| 473 | `parse_ipv6` | `libsodium/sodium/codecs.c:437`: ` if (p >= end) {` | `return 0;` | [x] |
| 474 | `parse_ipv6` | `libsodium/sodium/codecs.c:443`: ` if (tp + 4 > endp \|\| parse_ipv4(curtok, end, tp) == 0) {` | `return 0;` | [x] |
| 475 | `parse_ipv6` | `libsodium/sodium/codecs.c:451`: ` if (hv < 0 \|\| xdigits >= 4) {` | `return 0;` | [x] |
| 476 | `parse_ipv6` | `libsodium/sodium/codecs.c:460`: ` if (tp + 2 > endp) {` | `return 0;` | [x] |
| 477 | `parse_ipv6` | `libsodium/sodium/codecs.c:469`: ` if (tp == endp) {` | `return 0;` | [x] |
| 478 | `parse_ipv6` | `libsodium/sodium/codecs.c:476`: ` if (tp != endp) {` | `return 0;` | [x] |
| 479 | `sodium_ip2bin` | `libsodium/sodium/codecs.c:502`: ` if (!((*z >= '0' && *z <= '9') \|\| (*z >= 'a' && *z <= 'z') \|\| (*z >= 'A' && *z <= 'Z') \|\| *z == '-' \|\| *z == '_' \|\| *z == '.')) {` | `return -1;` | [x] |
| 480 | `sodium_ip2bin` | `libsodium/sodium/codecs.c:506`: ` if (zone + 1 >= end) {` | `return -1;` | [x] |
| 481 | `sodium_ip2bin` | `libsodium/sodium/codecs.c:512`: ` if (zone != NULL && !is_ipv6) {` | `return -1;` | [x] |
| 482 | `sodium_ip2bin` | `libsodium/sodium/codecs.c:518`: ` if (parse_ipv4(ip, end, v4) == 0) {` | `return -1;` | [x] |
| 483 | `sodium_bin2ip` | `libsodium/sodium/codecs.c:562`: ` if (ip_maxlen <= 2U) {` | `return NULL;` | [x] |
| 484 | `sodium_bin2ip` | `libsodium/sodium/codecs.c:573`: ` if (len >= ip_maxlen) {` | `return NULL;` | [x] |
| 485 | `sodium_bin2ip` | `libsodium/sodium/codecs.c:618`: ` if (len >= ip_maxlen) {` | `return NULL;` | [x] |
| 486 | `sodium_init` | `libsodium/sodium/core.c:31`: ` if (sodium_crit_enter() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 487 | `sodium_init` | `libsodium/sodium/core.c:35`: ` if (sodium_crit_leave() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 488 | `sodium_init` | `libsodium/sodium/core.c:53`: ` if (sodium_crit_leave() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 489 | `_sodium_crit_init` | `libsodium/sodium/core.c:80`: ` switch (status) {` | `return -1;` | [x] |
| 490 | `sodium_crit_enter` | `libsodium/sodium/core.c:88`: ` if (_sodium_crit_init() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 491 | `sodium_crit_enter` | `libsodium/sodium/core.c:91`: ` if (_sodium_crit_init() != 0) {` | `assert(locked == 0);` | [x] |
| 492 | `sodium_crit_leave` | `libsodium/sodium/core.c:102`: ` if (locked == 0) {` | `errno = EPERM;` | [x] |
| 493 | `sodium_crit_leave` | `libsodium/sodium/core.c:104`: ` if (locked == 0) {` | `return -1;` | [x] |
| 494 | `sodium_crit_enter` | `libsodium/sodium/core.c:122`: ` if ((ret = pthread_mutex_lock(&_sodium_lock)) == 0) {` | `assert(locked == 0);` | [x] |
| 495 | `sodium_crit_leave` | `libsodium/sodium/core.c:133`: ` if (locked == 0) {` | `errno = EPERM;` | [x] |
| 496 | `sodium_crit_leave` | `libsodium/sodium/core.c:135`: ` if (locked == 0) {` | `return -1;` | [x] |
| 497 | `sodium_misuse` | `libsodium/sodium/core.c:192`: `unconditional at this source site or condition is more than 8 lines above` | `sodium_misuse(void)` | [x] |
| 498 | `sodium_misuse` | `libsodium/sodium/core.c:204`: ` if (sodium_crit_leave() == 0 && handler != NULL) {` | `abort();` | [x] |
| 499 | `sodium_set_misuse_handler` | `libsodium/sodium/core.c:212`: ` if (sodium_crit_enter() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 500 | `sodium_set_misuse_handler` | `libsodium/sodium/core.c:216`: ` if (sodium_crit_leave() != 0) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 501 | `_sodium_runtime_arm_cpu_features` | `libsodium/sodium/runtime.c:67`: `unconditional at this source site or condition is more than 8 lines above` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 502 | `_sodium_runtime_intel_cpu_features` | `libsodium/sodium/runtime.c:209`: ` if (cpu_info[0] == 0U) {` | `return -1; /* LCOV_EXCL_LINE */` | [x] |
| 503 | `sodium_memzero` | `libsodium/sodium/utils.c:132`: ` if (len > 0U && memset_s(pnt, (rsize_t) len, 0, (rsize_t) len) != 0) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 504 | `_sodium_alloc_init` | `libsodium/sodium/utils.c:424`: ` if (page_size < CANARY_SIZE \|\| page_size < sizeof(size_t)) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 505 | `sodium_mlock` | `libsodium/sodium/utils.c:443`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 506 | `sodium_mlock` | `libsodium/sodium/utils.c:444`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 507 | `sodium_munlock` | `libsodium/sodium/utils.c:460`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 508 | `sodium_munlock` | `libsodium/sodium/utils.c:461`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 509 | `_mprotect_noaccess` | `libsodium/sodium/utils.c:474`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 510 | `_mprotect_noaccess` | `libsodium/sodium/utils.c:475`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 511 | `_mprotect_readonly` | `libsodium/sodium/utils.c:488`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 512 | `_mprotect_readonly` | `libsodium/sodium/utils.c:489`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 513 | `_mprotect_readwrite` | `libsodium/sodium/utils.c:502`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 514 | `_mprotect_readwrite` | `libsodium/sodium/utils.c:503`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 515 | `_out_of_bounds` | `libsodium/sodium/utils.c:522`: `unconditional at this source site or condition is more than 8 lines above` | `abort(); /* not something we want any higher-level API to catch */` | [x] |
| 516 | `_unprotected_ptr_from_user_ptr` | `libsodium/sodium/utils.c:582`: ` if (unprotected_ptr_u <= page_size * 2U) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 517 | `_sodium_malloc` | `libsodium/sodium/utils.c:608`: ` if (size >= (size_t) SIZE_MAX - page_size * 4U) {` | `errno = ENOMEM;` | [x] |
| 518 | `_sodium_malloc` | `libsodium/sodium/utils.c:609`: ` if (size >= (size_t) SIZE_MAX - page_size * 4U) {` | `return NULL;` | [x] |
| 519 | `_sodium_malloc` | `libsodium/sodium/utils.c:612`: ` if (page_size <= sizeof canary \|\| page_size < sizeof unprotected_size) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 520 | `_sodium_malloc` | `libsodium/sodium/utils.c:618`: ` if ((base_ptr = _alloc_aligned(total_size)) == NULL) {` | `return NULL; /* LCOV_EXCL_LINE */` | [x] |
| 521 | `_sodium_malloc` | `libsodium/sodium/utils.c:633`: `unconditional at this source site or condition is more than 8 lines above` | `assert(_unprotected_ptr_from_user_ptr(user_ptr) == unprotected_ptr);` | [x] |
| 522 | `sodium_malloc` | `libsodium/sodium/utils.c:645`: ` if ((ptr = _sodium_malloc(size)) == NULL) {` | `return NULL;` | [x] |
| 523 | `sodium_allocarray` | `libsodium/sodium/utils.c:656`: ` if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) {` | `errno = ENOMEM;` | [x] |
| 524 | `sodium_allocarray` | `libsodium/sodium/utils.c:657`: ` if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) {` | `return NULL;` | [x] |
| 525 | `_sodium_mprotect` | `libsodium/sodium/utils.c:707`: `unconditional at this source site or condition is more than 8 lines above` | `errno = ENOSYS;` | [x] |
| 526 | `_sodium_mprotect` | `libsodium/sodium/utils.c:708`: `unconditional at this source site or condition is more than 8 lines above` | `return -1;` | [x] |
| 527 | `sodium_pad` | `libsodium/sodium/utils.c:756`: ` if (blocksize <= 0U) {` | `return -1;` | [x] |
| 528 | `sodium_pad` | `libsodium/sodium/utils.c:765`: ` if ((size_t) SIZE_MAX - unpadded_buflen <= xpadlen) {` | `sodium_misuse(); /* LCOV_EXCL_LINE */` | [x] |
| 529 | `sodium_pad` | `libsodium/sodium/utils.c:769`: ` if (xpadded_len >= max_buflen) {` | `return -1;` | [x] |
| 530 | `sodium_unpad` | `libsodium/sodium/utils.c:798`: ` if (padded_buflen < blocksize \|\| blocksize <= 0U) {` | `return -1;` | [x] |
