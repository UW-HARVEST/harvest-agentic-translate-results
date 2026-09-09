# Error surface

Generated mechanically from every C source rejection statement matching `RETURN_ERROR`, `return -1`, `return NULL`, error-enum returns, `assert`, or `sodium_misuse`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|----------|---------------------------------------------|-------------------|
| 1 | `crypto_aead_aegis128l_encrypt` | if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) ('crypto_aead/aegis128l/aead_aegis128l.c:70') | process aborts via sodium_misuse() |
| 2 | `crypto_aead_aegis128l_encrypt_detached` | if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) ('crypto_aead/aegis128l/aead_aegis128l.c:121') | process aborts via sodium_misuse() |
| 3 | `crypto_aead_aegis128l_decrypt_detached` | if (clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX) ('crypto_aead/aegis128l/aead_aegis128l.c:139') | returns '-1' |
| 4 | `crypto_aead_aegis256_encrypt` | if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) ('crypto_aead/aegis256/aead_aegis256.c:70') | process aborts via sodium_misuse() |
| 5 | `crypto_aead_aegis256_encrypt_detached` | if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) ('crypto_aead/aegis256/aead_aegis256.c:121') | process aborts via sodium_misuse() |
| 6 | `crypto_aead_aegis256_decrypt_detached` | if (clen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX) ('crypto_aead/aegis256/aead_aegis256.c:138') | returns '-1' |
| 7 | `crypto_aead_aes256gcm_encrypt_detached` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:65') | returns '-1' |
| 8 | `crypto_aead_aes256gcm_encrypt` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:75') | returns '-1' |
| 9 | `crypto_aead_aes256gcm_decrypt_detached` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:86') | returns '-1' |
| 10 | `crypto_aead_aes256gcm_decrypt` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:96') | returns '-1' |
| 11 | `crypto_aead_aes256gcm_beforenm` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:103') | returns '-1' |
| 12 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:115') | returns '-1' |
| 13 | `crypto_aead_aes256gcm_encrypt_afternm` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:126') | returns '-1' |
| 14 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:137') | returns '-1' |
| 15 | `crypto_aead_aes256gcm_decrypt_afternm` | unconditional/internal failure path ('crypto_aead/aes256gcm/aead_aes256gcm.c:148') | returns '-1' |
| 16 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:756') | process aborts via sodium_misuse() |
| 17 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:762') | returns '-1' |
| 18 | `crypto_aead_aes256gcm_verify_mac` | if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:853') | process aborts via sodium_misuse() |
| 19 | `crypto_aead_aes256gcm_verify_mac` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:857') | returns '-1' |
| 20 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:916') | process aborts via sodium_misuse() |
| 21 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:923') | returns '-1' |
| 22 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (crypto_verify_16(mac, computed_mac) != 0) ('crypto_aead/aes256gcm/aesni/aead_aes256gcm_aesni.c:936') | returns '-1' |
| 23 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:795') | process aborts via sodium_misuse() |
| 24 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:801') | returns '-1' |
| 25 | `crypto_aead_aes256gcm_verify_mac` | if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:892') | process aborts via sodium_misuse() |
| 26 | `crypto_aead_aes256gcm_verify_mac` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:896') | returns '-1' |
| 27 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:955') | process aborts via sodium_misuse() |
| 28 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (gh_required_blocks == 0) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:962') | returns '-1' |
| 29 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | if (crypto_verify_16(mac, computed_mac) != 0) ('crypto_aead/aes256gcm/armcrypto/aead_aes256gcm_armcrypto.c:975') | returns '-1' |
| 30 | `crypto_aead_chacha20poly1305_encrypt` | if (mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX) ('crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:90') | process aborts via sodium_misuse() |
| 31 | `crypto_aead_chacha20poly1305_ietf_encrypt` | if (mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX) ('crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:178') | process aborts via sodium_misuse() |
| 32 | `crypto_aead_chacha20poly1305_decrypt_detached` | if (ret != 0) ('crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:237') | returns '-1' |
| 33 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | if (ret != 0) ('crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:322') | returns '-1' |
| 34 | `_decrypt_detached` | if (ret != 0) ('crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:137') | returns '-1' |
| 35 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | if (mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX) ('crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:186') | process aborts via sodium_misuse() |
| 36 | `crypto_auth_hmacsha256_init` | if (keylen > 0) ('crypto_auth/hmacsha256/auth_hmacsha256.c:53') | process aborts via sodium_misuse() |
| 37 | `crypto_auth_hmacsha512_init` | if (keylen > 0) ('crypto_auth/hmacsha512/auth_hmacsha512.c:53') | process aborts via sodium_misuse() |
| 38 | `crypto_box_detached` | if (crypto_box_beforenm(k, pk, sk) != 0) ('crypto_box/crypto_box_easy.c:31') | returns '-1' |
| 39 | `crypto_box_easy_afternm` | if (mlen > crypto_box_MESSAGEBYTES_MAX) ('crypto_box/crypto_box_easy.c:45') | process aborts via sodium_misuse() |
| 40 | `crypto_box_easy` | if (mlen > crypto_box_MESSAGEBYTES_MAX) ('crypto_box/crypto_box_easy.c:57') | process aborts via sodium_misuse() |
| 41 | `crypto_box_open_detached` | if (crypto_box_beforenm(k, pk, sk) != 0) ('crypto_box/crypto_box_easy.c:83') | returns '-1' |
| 42 | `crypto_box_open_easy_afternm` | if (clen < crypto_box_MACBYTES) ('crypto_box/crypto_box_easy.c:97') | returns '-1' |
| 43 | `crypto_box_open_easy` | if (clen < crypto_box_MACBYTES) ('crypto_box/crypto_box_easy.c:110') | returns '-1' |
| 44 | `crypto_box_seal` | if (mlen > crypto_box_MESSAGEBYTES_MAX) ('crypto_box/crypto_box_seal.c:34') | process aborts via sodium_misuse() |
| 45 | `crypto_box_seal` | if (crypto_box_keypair(epk, esk) != 0) ('crypto_box/crypto_box_seal.c:37') | returns '-1' |
| 46 | `crypto_box_seal_open` | if (clen < crypto_box_SEALBYTES) ('crypto_box/crypto_box_seal.c:56') | returns '-1' |
| 47 | `crypto_box_curve25519xchacha20poly1305_beforenm` | if (crypto_scalarmult_curve25519(s, sk, pk) != 0) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:49') | returns '-1' |
| 48 | `crypto_box_curve25519xchacha20poly1305_detached` | if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:77') | returns '-1' |
| 49 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:94') | process aborts via sodium_misuse() |
| 50 | `crypto_box_curve25519xchacha20poly1305_easy` | if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:106') | process aborts via sodium_misuse() |
| 51 | `crypto_box_curve25519xchacha20poly1305_open_detached` | if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:132') | returns '-1' |
| 52 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:147') | returns '-1' |
| 53 | `crypto_box_curve25519xchacha20poly1305_open_easy` | if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) ('crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:160') | returns '-1' |
| 54 | `crypto_box_curve25519xchacha20poly1305_seal` | if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:40') | process aborts via sodium_misuse() |
| 55 | `crypto_box_curve25519xchacha20poly1305_seal` | if (crypto_box_curve25519xchacha20poly1305_keypair(epk, esk) != 0) ('crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:43') | returns '-1' |
| 56 | `crypto_box_curve25519xchacha20poly1305_seal_open` | if (clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES) ('crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:64') | returns '-1' |
| 57 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | if (crypto_scalarmult_curve25519(s, sk, pk) != 0) ('crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:43') | returns '-1' |
| 58 | `crypto_box_curve25519xsalsa20poly1305` | if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) ('crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:82') | returns '-1' |
| 59 | `crypto_box_curve25519xsalsa20poly1305_open` | if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) ('crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:99') | returns '-1' |
| 60 | `crypto_core_ed25519_add` | if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) ('crypto_core/ed25519/core_ed25519.c:36') | returns '-1' |
| 61 | `crypto_core_ed25519_sub` | if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| ge25519_frombytes(&q_p3, q) != 0 \|\| ge25519_is_on_curve(&q_p3) == 0) ('crypto_core/ed25519/core_ed25519.c:52') | returns '-1' |
| 62 | `_string_to_points` | if (core_h2c_string_to_hash(h_be, n * HASH_GE_L, ctx, ctx_len, msg, msg_len, hash_alg) != 0) ('crypto_core/ed25519/core_ed25519.c:77') | returns '-1' |
| 63 | `crypto_core_ed25519_from_string` | if (_string_to_points(px, 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0) ('crypto_core/ed25519/core_ed25519.c:109') | returns '-1' |
| 64 | `crypto_core_ed25519_scalar_from_string` | if (core_h2c_string_to_hash(h_be, sizeof h_be, ctx, ctx_len, msg, msg_len, hash_alg) != 0) ('crypto_core/ed25519/core_ed25519.c:251') | returns '-1' |
| 65 | `core_h2c_string_to_hash_sha256` | assertion false: h_len <= 0xff ('crypto_core/ed25519/core_h2c.c:26') | process aborts via failed C assert |
| 66 | `core_h2c_string_to_hash_sha512` | assertion false: h_len <= 0xff ('crypto_core/ed25519/core_h2c.c:82') | process aborts via failed C assert |
| 67 | `core_h2c_string_to_hash` | unconditional/internal failure path ('crypto_core/ed25519/core_h2c.c:131') | returns '-1' |
| 68 | `crypto_core_ristretto255_add` | if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) ('crypto_core/ed25519/core_ristretto255.c:34') | returns '-1' |
| 69 | `crypto_core_ristretto255_sub` | if (ristretto255_frombytes(&p_p3, p) != 0 \|\| ristretto255_frombytes(&q_p3, q) != 0) ('crypto_core/ed25519/core_ristretto255.c:50') | returns '-1' |
| 70 | `_string_to_element` | if (core_h2c_string_to_hash(h, sizeof h, ctx, ctx_len, msg, msg_len, hash_alg) != 0) ('crypto_core/ed25519/core_ristretto255.c:76') | returns '-1' |
| 71 | `ge25519_frombytes_negate_vartime` | if (fe25519_iszero(p_root_check) == 0) ('crypto_core/ed25519/ref10/ed25519_ref10.c:395') | returns '-1' |
| 72 | `ristretto255_frombytes` | if (ristretto255_is_canonical(s) == 0) ('crypto_core/ed25519/ref10/ed25519_ref10.c:2834') | returns '-1' |
| 73 | `blake2b_init` | if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) ('crypto_generichash/blake2b/ref/blake2b-ref.c:131') | process aborts via sodium_misuse() |
| 74 | `blake2b_init_salt_personal` | if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) ('crypto_generichash/blake2b/ref/blake2b-ref.c:154') | process aborts via sodium_misuse() |
| 75 | `blake2b_init_key` | if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) ('crypto_generichash/blake2b/ref/blake2b-ref.c:185') | process aborts via sodium_misuse() |
| 76 | `blake2b_init_key` | if (!key \|\| !keylen \|\| keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:188') | process aborts via sodium_misuse() |
| 77 | `blake2b_init_key` | if (blake2b_init_param(S, P) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:203') | process aborts via sodium_misuse() |
| 78 | `blake2b_init_key_salt_personal` | if ((!outlen) \|\| (outlen > BLAKE2B_OUTBYTES)) ('crypto_generichash/blake2b/ref/blake2b-ref.c:223') | process aborts via sodium_misuse() |
| 79 | `blake2b_init_key_salt_personal` | if (!key \|\| !keylen \|\| keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:226') | process aborts via sodium_misuse() |
| 80 | `blake2b_init_key_salt_personal` | if (blake2b_init_param(S, P) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:249') | process aborts via sodium_misuse() |
| 81 | `blake2b_final` | if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:297') | process aborts via sodium_misuse() |
| 82 | `blake2b_final` | if (blake2b_is_lastblock(S)) ('crypto_generichash/blake2b/ref/blake2b-ref.c:300') | returns '-1' |
| 83 | `blake2b_final` | assertion false: S->buflen <= BLAKE2B_BLOCKBYTES ('crypto_generichash/blake2b/ref/blake2b-ref.c:306') | process aborts via failed C assert |
| 84 | `blake2b` | if (NULL == in && inlen > 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:342') | process aborts via sodium_misuse() |
| 85 | `blake2b` | if (NULL == out) ('crypto_generichash/blake2b/ref/blake2b-ref.c:345') | process aborts via sodium_misuse() |
| 86 | `blake2b` | if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:348') | process aborts via sodium_misuse() |
| 87 | `blake2b` | if (NULL == key && keylen > 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:351') | process aborts via sodium_misuse() |
| 88 | `blake2b` | if (keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:354') | process aborts via sodium_misuse() |
| 89 | `blake2b` | if (blake2b_init_key(S, outlen, key, keylen) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:358') | process aborts via sodium_misuse() |
| 90 | `blake2b` | if (blake2b_init(S, outlen) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:362') | process aborts via sodium_misuse() |
| 91 | `blake2b_salt_personal` | if (NULL == in && inlen > 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:380') | process aborts via sodium_misuse() |
| 92 | `blake2b_salt_personal` | if (NULL == out) ('crypto_generichash/blake2b/ref/blake2b-ref.c:383') | process aborts via sodium_misuse() |
| 93 | `blake2b_salt_personal` | if (!outlen \|\| outlen > BLAKE2B_OUTBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:386') | process aborts via sodium_misuse() |
| 94 | `blake2b_salt_personal` | if (NULL == key && keylen > 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:389') | process aborts via sodium_misuse() |
| 95 | `blake2b_salt_personal` | if (keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/blake2b-ref.c:392') | process aborts via sodium_misuse() |
| 96 | `blake2b_salt_personal` | if (blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:397') | process aborts via sodium_misuse() |
| 97 | `blake2b_salt_personal` | if (blake2b_init_salt_personal(S, outlen, salt, personal) < 0) ('crypto_generichash/blake2b/ref/blake2b-ref.c:401') | process aborts via sodium_misuse() |
| 98 | `crypto_generichash_blake2b` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:18') | returns '-1' |
| 99 | `crypto_generichash_blake2b` | assertion false: outlen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:20') | process aborts via failed C assert |
| 100 | `crypto_generichash_blake2b` | assertion false: keylen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:21') | process aborts via failed C assert |
| 101 | `crypto_generichash_blake2b_salt_personal` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES \|\| inlen > UINT64_MAX) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:35') | returns '-1' |
| 102 | `crypto_generichash_blake2b_salt_personal` | assertion false: outlen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:37') | process aborts via failed C assert |
| 103 | `crypto_generichash_blake2b_salt_personal` | assertion false: keylen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:38') | process aborts via failed C assert |
| 104 | `crypto_generichash_blake2b_init` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:52') | returns '-1' |
| 105 | `crypto_generichash_blake2b_init` | assertion false: outlen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:54') | process aborts via failed C assert |
| 106 | `crypto_generichash_blake2b_init` | assertion false: keylen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:55') | process aborts via failed C assert |
| 107 | `crypto_generichash_blake2b_init` | if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:59') | returns '-1' |
| 108 | `crypto_generichash_blake2b_init` | } else if (blake2b_init_key((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen) != 0) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:63') | returns '-1' |
| 109 | `crypto_generichash_blake2b_init_salt_personal` | if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| keylen > BLAKE2B_KEYBYTES) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:76') | returns '-1' |
| 110 | `crypto_generichash_blake2b_init_salt_personal` | assertion false: outlen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:78') | process aborts via failed C assert |
| 111 | `crypto_generichash_blake2b_init_salt_personal` | assertion false: keylen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:79') | process aborts via failed C assert |
| 112 | `crypto_generichash_blake2b_init_salt_personal` | if (blake2b_init_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, salt, personal) != 0) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:83') | returns '-1' |
| 113 | `crypto_generichash_blake2b_init_salt_personal` | } else if (blake2b_init_key_salt_personal((blake2b_state *) (void *) state, (uint8_t) outlen, key, (uint8_t) keylen, salt, personal) != 0) ('crypto_generichash/blake2b/ref/generichash_blake2b.c:89') | returns '-1' |
| 114 | `crypto_generichash_blake2b_final` | assertion false: outlen <= UINT8_MAX ('crypto_generichash/blake2b/ref/generichash_blake2b.c:107') | process aborts via failed C assert |
| 115 | `crypto_kdf_blake2b_derive_from_key` | if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| subkey_len > crypto_kdf_blake2b_BYTES_MAX) ('crypto_kdf/blake2b/kdf_blake2b.c:46') | returns '-1' |
| 116 | `crypto_kdf_hkdf_sha256_expand` | if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX) ('crypto_kdf/hkdf/kdf_hkdf_sha256.c:67') | returns '-1' |
| 117 | `crypto_kdf_hkdf_sha512_expand` | if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX) ('crypto_kdf/hkdf/kdf_hkdf_sha512.c:67') | returns '-1' |
| 118 | `mlkem768_ref_enc_deterministic` | if (polyvec_is_canonical(&pkpv) == 0) ('crypto_kem/mlkem768/ref/kem_mlkem768_ref.c:746') | returns '-1' |
| 119 | `crypto_kem_xwing_enc_deterministic` | if (crypto_kem_mlkem768_enc_deterministic(ct_mlkem, ss_mlkem, pk_mlkem, seed_mlkem) != 0) ('crypto_kem/xwing/kem_xwing.c:135') | returns '-1' |
| 120 | `crypto_kem_xwing_enc_deterministic` | if (crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0) ('crypto_kem/xwing/kem_xwing.c:142') | returns '-1' |
| 121 | `crypto_kem_xwing_enc` | if (crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed) != 0) ('crypto_kem/xwing/kem_xwing.c:164') | returns '-1' |
| 122 | `crypto_kem_xwing_dec` | if (crypto_kem_mlkem768_dec(ss_mlkem, ct_mlkem, sk_mlkem) != 0) ('crypto_kem/xwing/kem_xwing.c:191') | returns '-1' |
| 123 | `crypto_kem_xwing_dec` | if (crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0) ('crypto_kem/xwing/kem_xwing.c:198') | returns '-1' |
| 124 | `crypto_kx_client_session_keys` | if (rx == NULL) ('crypto_kx/crypto_kx.c:52') | process aborts via sodium_misuse() |
| 125 | `crypto_kx_client_session_keys` | if (crypto_scalarmult(q, client_sk, server_pk) != 0) ('crypto_kx/crypto_kx.c:55') | returns '-1' |
| 126 | `crypto_kx_server_session_keys` | if (rx == NULL) ('crypto_kx/crypto_kx.c:93') | process aborts via sodium_misuse() |
| 127 | `crypto_kx_server_session_keys` | if (crypto_scalarmult(q, server_sk, client_pk) != 0) ('crypto_kx/crypto_kx.c:96') | returns '-1' |
| 128 | `allocate_memory` | if (region == NULL) ('crypto_pwhash/argon2/argon2-core.c:89') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 129 | `allocate_memory` | if (m_cost == 0 \|\| memory_size / m_cost != sizeof(block)) ('crypto_pwhash/argon2/argon2-core.c:93') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 130 | `allocate_memory` | if (*region == NULL) ('crypto_pwhash/argon2/argon2-core.c:97') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 131 | `allocate_memory` | if (base == NULL) ('crypto_pwhash/argon2/argon2-core.c:128') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 132 | `argon2_validate_inputs` | if (NULL == context) ('crypto_pwhash/argon2/argon2-core.c:231') | returns 'ARGON2_INCORRECT_PARAMETER' |
| 133 | `argon2_validate_inputs` | if (NULL == context->out) ('crypto_pwhash/argon2/argon2-core.c:235') | returns 'ARGON2_OUTPUT_PTR_NULL' |
| 134 | `argon2_validate_inputs` | if (ARGON2_MIN_OUTLEN > context->outlen) ('crypto_pwhash/argon2/argon2-core.c:240') | returns 'ARGON2_OUTPUT_TOO_SHORT' |
| 135 | `argon2_validate_inputs` | if (ARGON2_MAX_OUTLEN < context->outlen) ('crypto_pwhash/argon2/argon2-core.c:244') | returns 'ARGON2_OUTPUT_TOO_LONG' |
| 136 | `argon2_validate_inputs` | if (0 != context->pwdlen) ('crypto_pwhash/argon2/argon2-core.c:250') | returns 'ARGON2_PWD_PTR_MISMATCH' |
| 137 | `argon2_validate_inputs` | if (ARGON2_MIN_PWD_LENGTH > context->pwdlen) ('crypto_pwhash/argon2/argon2-core.c:255') | returns 'ARGON2_PWD_TOO_SHORT' |
| 138 | `argon2_validate_inputs` | if (ARGON2_MAX_PWD_LENGTH < context->pwdlen) ('crypto_pwhash/argon2/argon2-core.c:259') | returns 'ARGON2_PWD_TOO_LONG' |
| 139 | `argon2_validate_inputs` | if (0 != context->saltlen) ('crypto_pwhash/argon2/argon2-core.c:265') | returns 'ARGON2_SALT_PTR_MISMATCH' |
| 140 | `argon2_validate_inputs` | if (ARGON2_MIN_SALT_LENGTH > context->saltlen) ('crypto_pwhash/argon2/argon2-core.c:270') | returns 'ARGON2_SALT_TOO_SHORT' |
| 141 | `argon2_validate_inputs` | if (ARGON2_MAX_SALT_LENGTH < context->saltlen) ('crypto_pwhash/argon2/argon2-core.c:274') | returns 'ARGON2_SALT_TOO_LONG' |
| 142 | `argon2_validate_inputs` | if (0 != context->secretlen) ('crypto_pwhash/argon2/argon2-core.c:280') | returns 'ARGON2_SECRET_PTR_MISMATCH' |
| 143 | `argon2_validate_inputs` | if (ARGON2_MIN_SECRET > context->secretlen) ('crypto_pwhash/argon2/argon2-core.c:284') | returns 'ARGON2_SECRET_TOO_SHORT' |
| 144 | `argon2_validate_inputs` | if (ARGON2_MAX_SECRET < context->secretlen) ('crypto_pwhash/argon2/argon2-core.c:288') | returns 'ARGON2_SECRET_TOO_LONG' |
| 145 | `argon2_validate_inputs` | if (0 != context->adlen) ('crypto_pwhash/argon2/argon2-core.c:295') | returns 'ARGON2_AD_PTR_MISMATCH' |
| 146 | `argon2_validate_inputs` | if (ARGON2_MIN_AD_LENGTH > context->adlen) ('crypto_pwhash/argon2/argon2-core.c:299') | returns 'ARGON2_AD_TOO_SHORT' |
| 147 | `argon2_validate_inputs` | if (ARGON2_MAX_AD_LENGTH < context->adlen) ('crypto_pwhash/argon2/argon2-core.c:303') | returns 'ARGON2_AD_TOO_LONG' |
| 148 | `argon2_validate_inputs` | if (ARGON2_MIN_LANES > context->lanes) ('crypto_pwhash/argon2/argon2-core.c:309') | returns 'ARGON2_LANES_TOO_FEW' |
| 149 | `argon2_validate_inputs` | if (ARGON2_MAX_LANES < context->lanes) ('crypto_pwhash/argon2/argon2-core.c:313') | returns 'ARGON2_LANES_TOO_MANY' |
| 150 | `argon2_validate_inputs` | if (ARGON2_MIN_MEMORY > context->m_cost) ('crypto_pwhash/argon2/argon2-core.c:318') | returns 'ARGON2_MEMORY_TOO_LITTLE' |
| 151 | `argon2_validate_inputs` | if (ARGON2_MAX_MEMORY < context->m_cost) ('crypto_pwhash/argon2/argon2-core.c:322') | returns 'ARGON2_MEMORY_TOO_MUCH' |
| 152 | `argon2_validate_inputs` | if (context->m_cost < 8 * context->lanes) ('crypto_pwhash/argon2/argon2-core.c:326') | returns 'ARGON2_MEMORY_TOO_LITTLE' |
| 153 | `argon2_validate_inputs` | if (ARGON2_MIN_TIME > context->t_cost) ('crypto_pwhash/argon2/argon2-core.c:331') | returns 'ARGON2_TIME_TOO_SMALL' |
| 154 | `argon2_validate_inputs` | if (ARGON2_MAX_TIME < context->t_cost) ('crypto_pwhash/argon2/argon2-core.c:335') | returns 'ARGON2_TIME_TOO_LARGE' |
| 155 | `argon2_validate_inputs` | if (ARGON2_MIN_THREADS > context->threads) ('crypto_pwhash/argon2/argon2-core.c:340') | returns 'ARGON2_THREADS_TOO_FEW' |
| 156 | `argon2_validate_inputs` | if (ARGON2_MAX_THREADS < context->threads) ('crypto_pwhash/argon2/argon2-core.c:344') | returns 'ARGON2_THREADS_TOO_MANY' |
| 157 | `argon2_initialize` | if (instance == NULL \|\| context == NULL) ('crypto_pwhash/argon2/argon2-core.c:466') | returns 'ARGON2_INCORRECT_PARAMETER' |
| 158 | `argon2_initialize` | if ((instance->pseudo_rands = (uint64_t *) malloc(sizeof(uint64_t) * instance->segment_length)) == NULL) ('crypto_pwhash/argon2/argon2-core.c:473') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 159 | `decode_decimal` | if (acc > (ULONG_MAX / 10)) ('crypto_pwhash/argon2/argon2-encoding.c:54') | returns 'NULL' |
| 160 | `decode_decimal` | if ((unsigned long) c > (ULONG_MAX - acc)) ('crypto_pwhash/argon2/argon2-encoding.c:58') | returns 'NULL' |
| 161 | `decode_decimal` | if (str == orig \|\| (*orig == '0' && str != (orig + 1))) ('crypto_pwhash/argon2/argon2-encoding.c:63') | returns 'NULL' |
| 162 | `argon2_decode_string` | if (strncmp(str, prefix, cc_len) != 0) { \ ('crypto_pwhash/argon2/argon2-encoding.c:101') | returns 'ARGON2_DECODING_FAIL' |
| 163 | `argon2_decode_string` | if (str == NULL) { \ ('crypto_pwhash/argon2/argon2-encoding.c:124') | returns 'ARGON2_DECODING_FAIL' |
| 164 | `argon2_decode_string` | if (str == NULL \|\| dec_x > UINT32_MAX) { \ ('crypto_pwhash/argon2/argon2-encoding.c:135') | returns 'ARGON2_DECODING_FAIL' |
| 165 | `argon2_decode_string` | if (sodium_base642bin((buf), (max_len), str, strlen(str), NULL, \ &bin_len, &str_end, \ sodium_base64_VARIANT_ORIGINAL_NO_PADDING) != 0 \|\| \ bin_len > UINT32_MAX) { \ ('crypto_pwhash/argon2/argon2-encoding.c:149') | returns 'ARGON2_DECODING_FAIL' |
| 166 | `argon2_decode_string` | } else if (type == Argon2_i) ('crypto_pwhash/argon2/argon2-encoding.c:168') | returns 'ARGON2_INCORRECT_TYPE' |
| 167 | `argon2_decode_string` | if (version != ARGON2_VERSION_NUMBER) ('crypto_pwhash/argon2/argon2-encoding.c:173') | returns 'ARGON2_INCORRECT_TYPE' |
| 168 | `argon2_decode_string` | if (ctx->m_cost > UINT32_MAX) ('crypto_pwhash/argon2/argon2-encoding.c:178') | returns 'ARGON2_INCORRECT_TYPE' |
| 169 | `argon2_decode_string` | if (ctx->t_cost > UINT32_MAX) ('crypto_pwhash/argon2/argon2-encoding.c:183') | returns 'ARGON2_INCORRECT_TYPE' |
| 170 | `argon2_decode_string` | if (ctx->lanes > UINT32_MAX) ('crypto_pwhash/argon2/argon2-encoding.c:188') | returns 'ARGON2_INCORRECT_TYPE' |
| 171 | `argon2_decode_string` | if (*str == 0) ('crypto_pwhash/argon2/argon2-encoding.c:203') | returns 'ARGON2_DECODING_FAIL' |
| 172 | `argon2_encode_string` | if (pp_len >= dst_len) { \ ('crypto_pwhash/argon2/argon2-encoding.c:248') | returns 'ARGON2_ENCODING_FAIL' |
| 173 | `argon2_encode_string` | if (sodium_bin2base64(dst, dst_len, (buf), (len), \ sodium_base64_VARIANT_ORIGINAL_NO_PADDING) == NULL) { \ ('crypto_pwhash/argon2/argon2-encoding.c:267') | returns 'ARGON2_ENCODING_FAIL' |
| 174 | `argon2_encode_string` | unconditional/internal failure path ('crypto_pwhash/argon2/argon2-encoding.c:282') | returns 'ARGON2_ENCODING_FAIL' |
| 175 | `argon2_ctx` | if (type != Argon2_id && type != Argon2_i) ('crypto_pwhash/argon2/argon2.c:41') | returns 'ARGON2_INCORRECT_TYPE' |
| 176 | `argon2_hash` | if (pwdlen > ARGON2_MAX_PWD_LENGTH) ('crypto_pwhash/argon2/argon2.c:102') | returns 'ARGON2_PWD_TOO_LONG' |
| 177 | `argon2_hash` | if (hashlen > ARGON2_MAX_OUTLEN) ('crypto_pwhash/argon2/argon2.c:106') | returns 'ARGON2_OUTPUT_TOO_LONG' |
| 178 | `argon2_hash` | if (saltlen > ARGON2_MAX_SALT_LENGTH) ('crypto_pwhash/argon2/argon2.c:110') | returns 'ARGON2_SALT_TOO_LONG' |
| 179 | `argon2_hash` | if (!out) ('crypto_pwhash/argon2/argon2.c:115') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 180 | `argon2_hash` | if (argon2_encode_string(encoded, encodedlen, &context, type) != ARGON2_OK) ('crypto_pwhash/argon2/argon2.c:152') | returns 'ARGON2_ENCODING_FAIL' |
| 181 | `argon2_verify` | if (encoded_len > UINT32_MAX) ('crypto_pwhash/argon2/argon2.c:230') | returns 'ARGON2_DECODING_LENGTH_FAIL' |
| 182 | `argon2_verify` | if (!ctx.out \|\| !ctx.salt \|\| !ctx.ad) ('crypto_pwhash/argon2/argon2.c:244') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 183 | `argon2_verify` | if (!out) ('crypto_pwhash/argon2/argon2.c:253') | returns 'ARGON2_MEMORY_ALLOCATION_ERROR' |
| 184 | `crypto_pwhash_argon2i` | if (outlen > crypto_pwhash_argon2i_BYTES_MAX) ('crypto_pwhash/argon2/pwhash_argon2i.c:148') | returns '-1' |
| 185 | `crypto_pwhash_argon2i` | if (outlen < crypto_pwhash_argon2i_BYTES_MIN) ('crypto_pwhash/argon2/pwhash_argon2i.c:152') | returns '-1' |
| 186 | `crypto_pwhash_argon2i` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) ('crypto_pwhash/argon2/pwhash_argon2i.c:158') | returns '-1' |
| 187 | `crypto_pwhash_argon2i` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) ('crypto_pwhash/argon2/pwhash_argon2i.c:164') | returns '-1' |
| 188 | `crypto_pwhash_argon2i` | if ((const void *) out == (const void *) passwd) ('crypto_pwhash/argon2/pwhash_argon2i.c:168') | returns '-1' |
| 189 | `crypto_pwhash_argon2i` | if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2i.c:176') | returns '-1' |
| 190 | `crypto_pwhash_argon2i` | if (argon2i_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2i_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2i.c:181') | returns '-1' |
| 191 | `crypto_pwhash_argon2i_str` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX) ('crypto_pwhash/argon2/pwhash_argon2i.c:198') | returns '-1' |
| 192 | `crypto_pwhash_argon2i_str` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN) ('crypto_pwhash/argon2/pwhash_argon2i.c:204') | returns '-1' |
| 193 | `crypto_pwhash_argon2i_str` | if (argon2i_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2i_STRBYTES) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2i.c:211') | returns '-1' |
| 194 | `crypto_pwhash_argon2i_str_verify` | if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX) ('crypto_pwhash/argon2/pwhash_argon2i.c:225') | returns '-1' |
| 195 | `crypto_pwhash_argon2i_str_verify` | if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN) ('crypto_pwhash/argon2/pwhash_argon2i.c:230') | returns '-1' |
| 196 | `crypto_pwhash_argon2i_str_verify` | if (verify_ret == ARGON2_VERIFY_MISMATCH) ('crypto_pwhash/argon2/pwhash_argon2i.c:241') | returns '-1' |
| 197 | `_needs_rehash` | if (opslimit > UINT32_MAX \|\| memlimit > UINT32_MAX \|\| fodder_len >= crypto_pwhash_STRBYTES) ('crypto_pwhash/argon2/pwhash_argon2i.c:258') | returns '-1' |
| 198 | `_needs_rehash` | if ((fodder = (unsigned char *) calloc(fodder_len, 1U)) == NULL) ('crypto_pwhash/argon2/pwhash_argon2i.c:262') | returns '-1' |
| 199 | `crypto_pwhash_argon2id` | if (outlen > crypto_pwhash_argon2id_BYTES_MAX) ('crypto_pwhash/argon2/pwhash_argon2id.c:144') | returns '-1' |
| 200 | `crypto_pwhash_argon2id` | if (outlen < crypto_pwhash_argon2id_BYTES_MIN) ('crypto_pwhash/argon2/pwhash_argon2id.c:148') | returns '-1' |
| 201 | `crypto_pwhash_argon2id` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) ('crypto_pwhash/argon2/pwhash_argon2id.c:154') | returns '-1' |
| 202 | `crypto_pwhash_argon2id` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) ('crypto_pwhash/argon2/pwhash_argon2id.c:160') | returns '-1' |
| 203 | `crypto_pwhash_argon2id` | if ((const void *) out == (const void *) passwd) ('crypto_pwhash/argon2/pwhash_argon2id.c:164') | returns '-1' |
| 204 | `crypto_pwhash_argon2id` | if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2id.c:172') | returns '-1' |
| 205 | `crypto_pwhash_argon2id` | if (argon2id_hash_raw((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, (size_t) crypto_pwhash_argon2id_SALTBYTES, out, (size_t) outlen) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2id.c:177') | returns '-1' |
| 206 | `crypto_pwhash_argon2id_str` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\| opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX \|\| memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX) ('crypto_pwhash/argon2/pwhash_argon2id.c:194') | returns '-1' |
| 207 | `crypto_pwhash_argon2id_str` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\| opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN \|\| memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN) ('crypto_pwhash/argon2/pwhash_argon2id.c:200') | returns '-1' |
| 208 | `crypto_pwhash_argon2id_str` | if (argon2id_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), (uint32_t) 1U, passwd, (size_t) passwdlen, salt, sizeof salt, STR_HASHBYTES, out, crypto_pwhash_argon2id_STRBYTES) != ARGON2_OK) ('crypto_pwhash/argon2/pwhash_argon2id.c:207') | returns '-1' |
| 209 | `crypto_pwhash_argon2id_str_verify` | if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX) ('crypto_pwhash/argon2/pwhash_argon2id.c:221') | returns '-1' |
| 210 | `crypto_pwhash_argon2id_str_verify` | if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN) ('crypto_pwhash/argon2/pwhash_argon2id.c:226') | returns '-1' |
| 211 | `crypto_pwhash_argon2id_str_verify` | if (verify_ret == ARGON2_VERIFY_MISMATCH) ('crypto_pwhash/argon2/pwhash_argon2id.c:237') | returns '-1' |
| 212 | `crypto_pwhash` | unconditional/internal failure path ('crypto_pwhash/crypto_pwhash.c:143') | returns '-1' |
| 213 | `crypto_pwhash_str_alg` | unconditional/internal failure path ('crypto_pwhash/crypto_pwhash.c:169') | process aborts via sodium_misuse() |
| 214 | `crypto_pwhash_str_alg` | unconditional/internal failure path ('crypto_pwhash/crypto_pwhash.c:171') | returns '-1' |
| 215 | `crypto_pwhash_str_verify` | if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) ('crypto_pwhash/crypto_pwhash.c:189') | returns '-1' |
| 216 | `crypto_pwhash_str_needs_rehash` | if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, sizeof crypto_pwhash_argon2i_STRPREFIX - 1) == 0) ('crypto_pwhash/crypto_pwhash.c:206') | returns '-1' |
| 217 | `encode64_uint32` | if (dstlen < 1) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:41') | returns 'NULL' |
| 218 | `encode64` | if (!dnext) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:66') | returns 'NULL' |
| 219 | `decode64_one` | if (ptr) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:85') | returns '-1' |
| 220 | `decode64_uint32` | if (decode64_one(&one, *src)) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:99') | returns 'NULL' |
| 221 | `escrypt_parse_setting` | if (setting[0] != '$' \|\| setting[1] != '7' \|\| setting[2] != '$') ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:116') | returns 'NULL' |
| 222 | `escrypt_parse_setting` | if (decode64_one(N_log2_p, *src)) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:121') | returns 'NULL' |
| 223 | `escrypt_parse_setting` | if (!src) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:127') | returns 'NULL' |
| 224 | `escrypt_parse_setting` | if (!src) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:132') | returns 'NULL' |
| 225 | `escrypt_r` | if (!src) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:160') | returns 'NULL' |
| 226 | `escrypt_r` | if (buf == NULL \|\| need > buflen \|\| need < saltlen) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:175') | returns 'NULL' |
| 227 | `escrypt_r` | if (escrypt_kdf(local, passwd, passwdlen, salt, saltlen, N, r, p, hash, sizeof(hash))) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:185') | returns 'NULL' |
| 228 | `escrypt_r` | if (!dst \|\| dst >= buf + buflen) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:195') | returns 'NULL' |
| 229 | `escrypt_gensalt_r` | if (need > buflen \|\| need < saltlen \|\| saltlen < srclen) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:214') | returns 'NULL' |
| 230 | `escrypt_gensalt_r` | if (N_log2 > 63 \|\| ((uint64_t) r * (uint64_t) p >= (1U << 30))) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:217') | returns 'NULL' |
| 231 | `escrypt_gensalt_r` | if (!dst) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:228') | returns 'NULL' |
| 232 | `escrypt_gensalt_r` | if (!dst) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:232') | returns 'NULL' |
| 233 | `escrypt_gensalt_r` | if (!dst \|\| dst >= buf + buflen) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:236') | returns 'NULL' |
| 234 | `crypto_pwhash_scryptsalsa208sha256_ll` | if (escrypt_init_local(&local)) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:254') | returns '-1' |
| 235 | `crypto_pwhash_scryptsalsa208sha256_ll` | if (escrypt_free_local(&local)) ('crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:265') | returns '-1' |
| 236 | `escrypt_kdf_nosse` | if (buflen > (((uint64_t)(1) << 32) - 1) * 32) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:250') | returns '-1' |
| 237 | `escrypt_kdf_nosse` | if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:255') | returns '-1' |
| 238 | `escrypt_kdf_nosse` | if (N > UINT32_MAX) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:259') | returns '-1' |
| 239 | `escrypt_kdf_nosse` | if (((N & (N - 1)) != 0) \|\| (N < 2)) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:263') | returns '-1' |
| 240 | `escrypt_kdf_nosse` | if (r == 0 \|\| p == 0) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:267') | returns '-1' |
| 241 | `escrypt_kdf_nosse` | if ((r > SIZE_MAX / 128 / p) \|\| #if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| #endif (N > SIZE_MAX / 128 / r)) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:275') | returns '-1' |
| 242 | `escrypt_kdf_nosse` | if (need < V_size) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:284') | returns '-1' |
| 243 | `escrypt_kdf_nosse` | if (need < XY_size) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:290') | returns '-1' |
| 244 | `escrypt_kdf_nosse` | if (escrypt_free_region(local)) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:294') | returns '-1' |
| 245 | `escrypt_kdf_nosse` | if (!escrypt_alloc_region(local, need)) ('crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c:297') | returns '-1' |
| 246 | `escrypt_PBKDF2_SHA256` | if (dkLen > 0x1fffffffe0ULL) ('crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c:64') | process aborts via sodium_misuse() |
| 247 | `crypto_pwhash_scryptsalsa208sha256` | if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\| outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:173') | returns '-1' |
| 248 | `crypto_pwhash_scryptsalsa208sha256` | if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:178') | returns '-1' |
| 249 | `crypto_pwhash_scryptsalsa208sha256` | if ((const void *) out == (const void *) passwd) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:182') | returns '-1' |
| 250 | `crypto_pwhash_scryptsalsa208sha256_str` | if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:206') | returns '-1' |
| 251 | `crypto_pwhash_scryptsalsa208sha256_str` | if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\| pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:211') | returns '-1' |
| 252 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting, sizeof setting) == NULL) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:217') | returns '-1' |
| 253 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_init_local(&escrypt_local) != 0) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:220') | returns '-1' |
| 254 | `crypto_pwhash_scryptsalsa208sha256_str` | if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) setting, (uint8_t *) out, crypto_pwhash_scryptsalsa208sha256_STRBYTES) == NULL) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:228') | returns '-1' |
| 255 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:255') | returns '-1' |
| 256 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (escrypt_init_local(&escrypt_local) != 0) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:258') | returns '-1' |
| 257 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, (const uint8_t *) str, (uint8_t *) wanted, sizeof wanted) == NULL) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:265') | returns '-1' |
| 258 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:285') | returns '-1' |
| 259 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) != crypto_pwhash_scryptsalsa208sha256_STRBYTES - 1U) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:290') | returns '-1' |
| 260 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | if (escrypt_parse_setting((const uint8_t *) str, &N_log2_, &r_, &p_) == NULL) ('crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:295') | returns '-1' |
| 261 | `escrypt_free_region` | if (munmap(region->base, region->size)) ('crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c:89') | returns '-1' |
| 262 | `escrypt_kdf_sse` | if (buflen > (((uint64_t)(1) << 32) - 1) * 32) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:325') | returns '-1' |
| 263 | `escrypt_kdf_sse` | if ((uint64_t)(r) * (uint64_t)(p) >= ((uint64_t) 1 << 30)) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:331') | returns '-1' |
| 264 | `escrypt_kdf_sse` | if (N > UINT32_MAX) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:335') | returns '-1' |
| 265 | `escrypt_kdf_sse` | if (((N & (N - 1)) != 0) \|\| (N < 2)) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:339') | returns '-1' |
| 266 | `escrypt_kdf_sse` | if (r == 0 \|\| p == 0) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:343') | returns '-1' |
| 267 | `escrypt_kdf_sse` | if ((r > SIZE_MAX / 128 / p) \|\| # if SIZE_MAX / 256 <= UINT32_MAX (r > SIZE_MAX / 256) \|\| # endif (N > SIZE_MAX / 128 / r)) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:352') | returns '-1' |
| 268 | `escrypt_kdf_sse` | if (need < V_size) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:363') | returns '-1' |
| 269 | `escrypt_kdf_sse` | if (need < XY_size) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:371') | returns '-1' |
| 270 | `escrypt_kdf_sse` | if (escrypt_free_region(local)) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:376') | returns '-1' |
| 271 | `escrypt_kdf_sse` | if (!escrypt_alloc_region(local, need)) ('crypto_pwhash/scryptsalsa208sha256/sse/pwhash_scryptsalsa208sha256_sse.c:379') | returns '-1' |
| 272 | `crypto_scalarmult_curve25519_ref10` | if (has_small_order(p)) ('crypto_scalarmult/curve25519/ref10/x25519_ref10.c:107') | returns '-1' |
| 273 | `crypto_scalarmult_curve25519` | if (implementation->mult(q, n, p) != 0) ('crypto_scalarmult/curve25519/scalarmult_curve25519.c:22') | returns '-1' |
| 274 | `_crypto_scalarmult_ed25519` | if (ge25519_is_canonical(p) == 0 \|\| ge25519_frombytes(&P, p) != 0 \|\| ge25519_has_small_order(&P) != 0 \|\| ge25519_is_on_main_subgroup(&P) == 0) ('crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:41') | returns '-1' |
| 275 | `_crypto_scalarmult_ed25519` | if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) ('crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:54') | returns '-1' |
| 276 | `_crypto_scalarmult_ed25519_base` | if (_crypto_scalarmult_ed25519_is_inf(q) != 0 \|\| sodium_is_zero(n, 32)) ('crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:92') | returns '-1' |
| 277 | `crypto_scalarmult_ristretto255` | if (ristretto255_frombytes(&P, p) != 0) ('crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:19') | returns '-1' |
| 278 | `crypto_scalarmult_ristretto255` | if (sodium_is_zero(q, 32)) ('crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:28') | returns '-1' |
| 279 | `crypto_scalarmult_ristretto255_base` | if (sodium_is_zero(q, 32)) ('crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:48') | returns '-1' |
| 280 | `crypto_secretbox_easy` | if (mlen > crypto_secretbox_MESSAGEBYTES_MAX) ('crypto_secretbox/crypto_secretbox_easy.c:98') | process aborts via sodium_misuse() |
| 281 | `crypto_secretbox_open_detached` | if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) ('crypto_secretbox/crypto_secretbox_easy.c:129') | returns '-1' |
| 282 | `crypto_secretbox_open_easy` | if (clen < crypto_secretbox_MACBYTES) ('crypto_secretbox/crypto_secretbox_easy.c:171') | returns '-1' |
| 283 | `crypto_secretbox_xchacha20poly1305_easy` | if (mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:90') | process aborts via sodium_misuse() |
| 284 | `crypto_secretbox_xchacha20poly1305_open_detached` | if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0) ('crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:122') | returns '-1' |
| 285 | `crypto_secretbox_xchacha20poly1305_open_easy` | if (clen < crypto_secretbox_xchacha20poly1305_MACBYTES) ('crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:165') | returns '-1' |
| 286 | `crypto_secretbox_xsalsa20poly1305` | if (mlen < 32) ('crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:16') | returns '-1' |
| 287 | `crypto_secretbox_xsalsa20poly1305_open` | if (clen < 32) ('crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:36') | returns '-1' |
| 288 | `crypto_secretbox_xsalsa20poly1305_open` | if (crypto_onetimeauth_poly1305_verify(c + 16, c + 32, clen - 32, subkey) != 0) ('crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:41') | returns '-1' |
| 289 | `crypto_secretstream_xchacha20poly1305_push` | if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:129') | process aborts via sodium_misuse() |
| 290 | `crypto_secretstream_xchacha20poly1305_pull` | if (inlen < crypto_secretstream_xchacha20poly1305_ABYTES) ('crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:202') | returns '-1' |
| 291 | `crypto_secretstream_xchacha20poly1305_pull` | if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX) ('crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:206') | process aborts via sodium_misuse() |
| 292 | `crypto_secretstream_xchacha20poly1305_pull` | if (sodium_memcmp(mac, stored_mac, sizeof mac) != 0) ('crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:241') | returns '-1' |
| 293 | `crypto_sign_ed25519_pk_to_curve25519` | if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 \|\| ge25519_has_small_order(&A) != 0 \|\| ge25519_is_on_main_subgroup(&A) == 0) ('crypto_sign/ed25519/ref10/keypair.c:56') | returns '-1' |
| 294 | `_crypto_sign_ed25519_verify_detached` | if (sig[63] & 224) ('crypto_sign/ed25519/ref10/open.c:32') | returns '-1' |
| 295 | `_crypto_sign_ed25519_verify_detached` | if ((sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0) ('crypto_sign/ed25519/ref10/open.c:37') | returns '-1' |
| 296 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_is_canonical(pk) == 0) ('crypto_sign/ed25519/ref10/open.c:40') | returns '-1' |
| 297 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_frombytes_negate_vartime(&A, pk) != 0 \|\| ge25519_has_small_order(&A) != 0) ('crypto_sign/ed25519/ref10/open.c:45') | returns '-1' |
| 298 | `_crypto_sign_ed25519_verify_detached` | if (ge25519_frombytes(&expected_r, sig) != 0 \|\| ge25519_has_small_order(&expected_r) != 0) ('crypto_sign/ed25519/ref10/open.c:49') | returns '-1' |
| 299 | `crypto_sign_ed25519_open` | if (mlen_p != NULL) ('crypto_sign/ed25519/ref10/open.c:103') | returns '-1' |
| 300 | `crypto_sign_ed25519` | if (smlen_p != NULL) ('crypto_sign/ed25519/ref10/sign.c:120') | returns '-1' |
| 301 | `crypto_stream_chacha20` | if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:68') | process aborts via sodium_misuse() |
| 302 | `crypto_stream_chacha20_xor_ic` | if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:80') | process aborts via sodium_misuse() |
| 303 | `crypto_stream_chacha20_xor` | if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:91') | process aborts via sodium_misuse() |
| 304 | `crypto_stream_chacha20_ietf_ext` | if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:101') | process aborts via sodium_misuse() |
| 305 | `crypto_stream_chacha20_ietf_ext_xor_ic` | if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:113') | process aborts via sodium_misuse() |
| 306 | `crypto_stream_chacha20_ietf_ext_xor` | if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:124') | process aborts via sodium_misuse() |
| 307 | `crypto_stream_chacha20_ietf` | if (clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:134') | process aborts via sodium_misuse() |
| 308 | `crypto_stream_chacha20_ietf_xor_ic` | if ((unsigned long long) ic > (64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL) ('crypto_stream/chacha20/stream_chacha20.c:147') | process aborts via sodium_misuse() |
| 309 | `crypto_stream_chacha20_ietf_xor` | if (mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) ('crypto_stream/chacha20/stream_chacha20.c:158') | process aborts via sodium_misuse() |
| 310 | `sodium_hrtime` | if (gettimeofday(&tv, NULL) != 0) ('randombytes/internal/randombytes_internal_random.c:173') | process aborts via sodium_misuse() |
| 311 | `randombytes_getentropy` | if (CCRandomGenerateBytes(buf, size) != kCCSuccess) ('randombytes/internal/randombytes_internal_random.c:198') | returns '-1' |
| 312 | `_randombytes_getentropy` | assertion false: size <= 256U ('randombytes/internal/randombytes_internal_random.c:208') | process aborts via failed C assert |
| 313 | `_randombytes_getentropy` | if (&getentropy == NULL) ('randombytes/internal/randombytes_internal_random.c:212') | returns '-1' |
| 314 | `_randombytes_getentropy` | if (getentropy(buf, size) != 0) ('randombytes/internal/randombytes_internal_random.c:216') | returns '-1' |
| 315 | `randombytes_getentropy` | assertion false: chunk_size > (size_t) 0U ('randombytes/internal/randombytes_internal_random.c:230') | process aborts via failed C assert |
| 316 | `randombytes_getentropy` | if (_randombytes_getentropy(buf, chunk_size) != 0) ('randombytes/internal/randombytes_internal_random.c:233') | returns '-1' |
| 317 | `_randombytes_linux_getrandom` | assertion false: size <= 256U ('randombytes/internal/randombytes_internal_random.c:249') | process aborts via failed C assert |
| 318 | `randombytes_linux_getrandom` | assertion false: chunk_size > (size_t) 0U ('randombytes/internal/randombytes_internal_random.c:266') | process aborts via failed C assert |
| 319 | `randombytes_linux_getrandom` | if (_randombytes_linux_getrandom(buf, chunk_size) != 0) ('randombytes/internal/randombytes_internal_random.c:269') | returns '-1' |
| 320 | `randombytes_block_on_dev_random` | if (pret != 1) ('randombytes/internal/randombytes_internal_random.c:302') | returns '-1' |
| 321 | `randombytes_internal_random_random_dev_open` | if (randombytes_block_on_dev_random() != 0) ('randombytes/internal/randombytes_internal_random.c:324') | returns '-1' |
| 322 | `randombytes_internal_random_random_dev_open` | } else if (errno == EINTR) ('randombytes/internal/randombytes_internal_random.c:344') | returns '-1' |
| 323 | `safe_read` | assertion false: size > (size_t) 0U ('randombytes/internal/randombytes_internal_random.c:354') | process aborts via failed C assert |
| 324 | `safe_read` | assertion false: size <= SSIZE_MAX ('randombytes/internal/randombytes_internal_random.c:355') | process aborts via failed C assert |
| 325 | `randombytes_internal_random_init` | assertion false: (global.getentropy_available \| global.getrandom_available) == 0 ('randombytes/internal/randombytes_internal_random.c:406') | process aborts via failed C assert |
| 326 | `randombytes_internal_random_init` | if ((global.random_data_source_fd = randombytes_internal_random_random_dev_open()) == -1) ('randombytes/internal/randombytes_internal_random.c:409') | process aborts via sodium_misuse() |
| 327 | `randombytes_internal_random_init` | if ((global.random_data_source_fd = randombytes_internal_random_random_dev_open()) == -1) ('randombytes/internal/randombytes_internal_random.c:416') | process aborts via sodium_misuse() |
| 328 | `randombytes_internal_random_stir` | assertion false: stream.nonce != (uint64_t) 0U ('randombytes/internal/randombytes_internal_random.c:430') | process aborts via failed C assert |
| 329 | `randombytes_internal_random_stir` | if (randombytes_getentropy(stream.key, sizeof stream.key) != 0) ('randombytes/internal/randombytes_internal_random.c:446') | process aborts via sodium_misuse() |
| 330 | `randombytes_internal_random_stir` | if (randombytes_linux_getrandom(stream.key, sizeof stream.key) != 0) ('randombytes/internal/randombytes_internal_random.c:452') | process aborts via sodium_misuse() |
| 331 | `randombytes_internal_random_stir` | if (global.random_data_source_fd == -1 \|\| safe_read(global.random_data_source_fd, stream.key, sizeof stream.key) != (ssize_t) sizeof stream.key) ('randombytes/internal/randombytes_internal_random.c:461') | process aborts via sodium_misuse() |
| 332 | `randombytes_internal_random_stir` | if (global.random_data_source_fd == -1 \|\| safe_read(global.random_data_source_fd, stream.key, sizeof stream.key) != (ssize_t) sizeof stream.key) ('randombytes/internal/randombytes_internal_random.c:464') | process aborts via sodium_misuse() |
| 333 | `randombytes_internal_random_stir` | if (! RtlGenRandom((PVOID) stream.key, (ULONG) sizeof stream.key)) ('randombytes/internal/randombytes_internal_random.c:469') | process aborts via sodium_misuse() |
| 334 | `randombytes_internal_random_stir_if_needed` | } else if (global.pid != getpid()) ('randombytes/internal/randombytes_internal_random.c:487') | process aborts via sodium_misuse() |
| 335 | `randombytes_internal_random_buf` | assertion false: size <= ULLONG_MAX ('randombytes/internal/randombytes_internal_random.c:599') | process aborts via failed C assert |
| 336 | `randombytes_internal_random_buf` | assertion false: ret == 0 ('randombytes/internal/randombytes_internal_random.c:604') | process aborts via failed C assert |
| 337 | `randombytes_internal_random` | assertion false: ret == 0 ('randombytes/internal/randombytes_internal_random.c:636') | process aborts via failed C assert |
| 338 | `randombytes_buf_deterministic` | if (size > 0x4000000000ULL) ('randombytes/randombytes.c:222') | process aborts via sodium_misuse() |
| 339 | `randombytes` | assertion false: buf_len <= SIZE_MAX ('randombytes/randombytes.c:247') | process aborts via failed C assert |
| 340 | `safe_read` | assertion false: size > (size_t) 0U ('randombytes/sysrandom/randombytes_sysrandom.c:134') | process aborts via failed C assert |
| 341 | `safe_read` | assertion false: size <= SSIZE_MAX ('randombytes/sysrandom/randombytes_sysrandom.c:135') | process aborts via failed C assert |
| 342 | `randombytes_block_on_dev_random` | if (pret != 1) ('randombytes/sysrandom/randombytes_sysrandom.c:173') | returns '-1' |
| 343 | `randombytes_sysrandom_random_dev_open` | if (randombytes_block_on_dev_random() != 0) ('randombytes/sysrandom/randombytes_sysrandom.c:195') | returns '-1' |
| 344 | `randombytes_sysrandom_random_dev_open` | } else if (errno == EINTR) ('randombytes/sysrandom/randombytes_sysrandom.c:223') | returns '-1' |
| 345 | `_randombytes_linux_getrandom` | assertion false: size <= 256U ('randombytes/sysrandom/randombytes_sysrandom.c:233') | process aborts via failed C assert |
| 346 | `randombytes_linux_getrandom` | assertion false: chunk_size > (size_t) 0U ('randombytes/sysrandom/randombytes_sysrandom.c:250') | process aborts via failed C assert |
| 347 | `randombytes_linux_getrandom` | if (_randombytes_linux_getrandom(buf, chunk_size) != 0) ('randombytes/sysrandom/randombytes_sysrandom.c:253') | returns '-1' |
| 348 | `randombytes_sysrandom_init` | if ((stream.random_data_source_fd = randombytes_sysrandom_random_dev_open()) == -1) ('randombytes/sysrandom/randombytes_sysrandom.c:283') | process aborts via sodium_misuse() |
| 349 | `randombytes_sysrandom_buf` | assertion false: size <= ULLONG_MAX ('randombytes/sysrandom/randombytes_sysrandom.c:346') | process aborts via failed C assert |
| 350 | `randombytes_sysrandom_buf` | if (randombytes_linux_getrandom(buf, size) != 0) ('randombytes/sysrandom/randombytes_sysrandom.c:353') | process aborts via sodium_misuse() |
| 351 | `randombytes_sysrandom_buf` | if (stream.random_data_source_fd == -1 \|\| safe_read(stream.random_data_source_fd, buf, size) != (ssize_t) size) ('randombytes/sysrandom/randombytes_sysrandom.c:360') | process aborts via sodium_misuse() |
| 352 | `randombytes_sysrandom_buf` | if (size > (size_t) 0xffffffffUL) ('randombytes/sysrandom/randombytes_sysrandom.c:365') | process aborts via sodium_misuse() |
| 353 | `randombytes_sysrandom_buf` | if (! RtlGenRandom((PVOID) buf, (ULONG) size)) ('randombytes/sysrandom/randombytes_sysrandom.c:368') | process aborts via sodium_misuse() |
| 354 | `sodium_bin2hex` | if (bin_len >= SIZE_MAX / 2 \|\| hex_maxlen <= bin_len * 2U) ('sodium/codecs.c:24') | process aborts via sodium_misuse() |
| 355 | `sodium_base64_check_variant` | if ((((unsigned int) variant) & ~ 0x6U) != 0x1U) ('sodium/codecs.c:169') | process aborts via sodium_misuse() |
| 356 | `sodium_base64_encoded_len` | if (bin_len / 3 > (SIZE_MAX - 5) / 4) ('sodium/codecs.c:179') | process aborts via sodium_misuse() |
| 357 | `sodium_bin2base64` | if (nibbles > (SIZE_MAX - 5) / 4) ('sodium/codecs.c:200') | process aborts via sodium_misuse() |
| 358 | `sodium_bin2base64` | if (b64_maxlen <= b64_len) ('sodium/codecs.c:212') | process aborts via sodium_misuse() |
| 359 | `sodium_bin2base64` | assertion false: b64_pos <= b64_len ('sodium/codecs.c:239') | process aborts via failed C assert |
| 360 | `_sodium_base642bin_skip_padding` | if (*b64_pos_p >= b64_len) ('sodium/codecs.c:260') | returns '-1' |
| 361 | `_sodium_base642bin_skip_padding` | } else if (ignore == NULL \|\| strchr(ignore, c) == NULL) ('sodium/codecs.c:268') | returns '-1' |
| 362 | `ip_hex_digit` | if (((unsigned int) ch \| 32U) >= 'a' && ((unsigned int) ch \| 32U) <= 'f') ('sodium/codecs.c:354') | returns '-1' |
| 363 | `sodium_ip2bin` | if (!((*z >= '0' && *z <= '9') \|\| (*z >= 'a' && *z <= 'z') \|\| (*z >= 'A' && *z <= 'Z') \|\| *z == '-' \|\| *z == '_' \|\| *z == '.')) ('sodium/codecs.c:502') | returns '-1' |
| 364 | `sodium_ip2bin` | if (zone + 1 >= end) ('sodium/codecs.c:506') | returns '-1' |
| 365 | `sodium_ip2bin` | if (zone != NULL && !is_ipv6) ('sodium/codecs.c:512') | returns '-1' |
| 366 | `sodium_ip2bin` | if (parse_ipv4(ip, end, v4) == 0) ('sodium/codecs.c:518') | returns '-1' |
| 367 | `sodium_bin2ip` | if (ip_maxlen <= 2U) ('sodium/codecs.c:562') | returns 'NULL' |
| 368 | `sodium_bin2ip` | if (len >= ip_maxlen) ('sodium/codecs.c:573') | returns 'NULL' |
| 369 | `sodium_bin2ip` | if (len >= ip_maxlen) ('sodium/codecs.c:618') | returns 'NULL' |
| 370 | `sodium_init` | if (sodium_crit_enter() != 0) ('sodium/core.c:31') | returns '-1' |
| 371 | `sodium_init` | if (sodium_crit_leave() != 0) ('sodium/core.c:35') | returns '-1' |
| 372 | `sodium_init` | if (sodium_crit_leave() != 0) ('sodium/core.c:53') | returns '-1' |
| 373 | `_sodium_crit_init` | unconditional/internal failure path ('sodium/core.c:80') | returns '-1' |
| 374 | `sodium_crit_enter` | if (_sodium_crit_init() != 0) ('sodium/core.c:88') | returns '-1' |
| 375 | `sodium_crit_enter` | assertion false: locked == 0 ('sodium/core.c:91') | process aborts via failed C assert |
| 376 | `sodium_crit_leave` | if (locked == 0) ('sodium/core.c:104') | returns '-1' |
| 377 | `sodium_crit_enter` | assertion false: locked == 0 ('sodium/core.c:122') | process aborts via failed C assert |
| 378 | `sodium_crit_leave` | if (locked == 0) ('sodium/core.c:135') | returns '-1' |
| 379 | `sodium_set_misuse_handler` | if (sodium_crit_enter() != 0) ('sodium/core.c:212') | returns '-1' |
| 380 | `sodium_set_misuse_handler` | if (sodium_crit_leave() != 0) ('sodium/core.c:216') | returns '-1' |
| 381 | `_sodium_runtime_arm_cpu_features` | unconditional/internal failure path ('sodium/runtime.c:67') | returns '-1' |
| 382 | `_sodium_runtime_intel_cpu_features` | if (cpu_info[0] == 0U) ('sodium/runtime.c:209') | returns '-1' |
| 383 | `sodium_memzero` | if (len > 0U && memset_s(pnt, (rsize_t) len, 0, (rsize_t) len) != 0) ('sodium/utils.c:132') | process aborts via sodium_misuse() |
| 384 | `_sodium_alloc_init` | if (page_size < CANARY_SIZE \|\| page_size < sizeof(size_t)) ('sodium/utils.c:424') | process aborts via sodium_misuse() |
| 385 | `sodium_mlock` | unconditional/internal failure path ('sodium/utils.c:444') | returns '-1' |
| 386 | `sodium_munlock` | unconditional/internal failure path ('sodium/utils.c:461') | returns '-1' |
| 387 | `_mprotect_noaccess` | unconditional/internal failure path ('sodium/utils.c:475') | returns '-1' |
| 388 | `_mprotect_readonly` | unconditional/internal failure path ('sodium/utils.c:489') | returns '-1' |
| 389 | `_mprotect_readwrite` | unconditional/internal failure path ('sodium/utils.c:503') | returns '-1' |
| 390 | `_unprotected_ptr_from_user_ptr` | if (unprotected_ptr_u <= page_size * 2U) ('sodium/utils.c:582') | process aborts via sodium_misuse() |
| 391 | `_sodium_malloc` | if (size >= (size_t) SIZE_MAX - page_size * 4U) ('sodium/utils.c:609') | returns 'NULL' |
| 392 | `_sodium_malloc` | if (page_size <= sizeof canary \|\| page_size < sizeof unprotected_size) ('sodium/utils.c:612') | process aborts via sodium_misuse() |
| 393 | `_sodium_malloc` | if ((base_ptr = _alloc_aligned(total_size)) == NULL) ('sodium/utils.c:618') | returns 'NULL' |
| 394 | `_sodium_malloc` | assertion false: _unprotected_ptr_from_user_ptr(user_ptr) == unprotected_ptr ('sodium/utils.c:633') | process aborts via failed C assert |
| 395 | `sodium_malloc` | if ((ptr = _sodium_malloc(size)) == NULL) ('sodium/utils.c:645') | returns 'NULL' |
| 396 | `sodium_allocarray` | if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) ('sodium/utils.c:657') | returns 'NULL' |
| 397 | `_sodium_mprotect` | unconditional/internal failure path ('sodium/utils.c:708') | returns '-1' |
| 398 | `sodium_pad` | if (blocksize <= 0U) ('sodium/utils.c:756') | returns '-1' |
| 399 | `sodium_pad` | if ((size_t) SIZE_MAX - unpadded_buflen <= xpadlen) ('sodium/utils.c:765') | process aborts via sodium_misuse() |
| 400 | `sodium_pad` | if (xpadded_len >= max_buflen) ('sodium/utils.c:769') | returns '-1' |
| 401 | `sodium_unpad` | if (padded_buflen < blocksize \|\| blocksize <= 0U) ('sodium/utils.c:798') | returns '-1' |

## Phase C status

- [x] Rejection statements are covered by the shared-implementation equivalence test and targeted FFI error tests.
