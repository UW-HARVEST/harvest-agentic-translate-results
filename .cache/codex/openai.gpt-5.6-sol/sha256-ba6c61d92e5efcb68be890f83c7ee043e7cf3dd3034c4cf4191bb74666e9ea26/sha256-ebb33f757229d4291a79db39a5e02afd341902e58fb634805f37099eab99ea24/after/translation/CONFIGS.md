# Configuration surface

Generated mechanically from the full C dynamic-symbol set and the direct `if`/`switch`/`case` axes in each matching C function. Each row denotes the entry point with randomized empty/one/many/boundary inputs chosen to exercise both outcomes of every listed direct predicate.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|-------------------------------------------|:---:|
| 1 | `_crypto_aead_aegis128l_pick_best_implementation` | `crypto_aead/aegis128l/aead_aegis128l.c:146`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_armcrypto()); if (sodium_runtime_has_aesni() & sodium_runtime_has_avx()) | x |
| 2 | `_crypto_aead_aegis256_pick_best_implementation` | `crypto_aead/aegis256/aead_aegis256.c:145`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_armcrypto()); if (sodium_runtime_has_aesni() & sodium_runtime_has_avx()) | x |
| 3 | `_crypto_generichash_blake2b_pick_best_implementation` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:113`; fixed-shape or branch-free valid invocation across randomized values | x |
| 4 | `_crypto_ipcrypt_pick_best_implementation` | `crypto_ipcrypt/crypto_ipcrypt.c:186`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_armcrypto()); if (sodium_runtime_has_aesni()) | x |
| 5 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:81`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_sse2()) | x |
| 6 | `_crypto_pwhash_argon2_pick_best_implementation` | `crypto_pwhash/argon2/argon2-core.c:540`; fixed-shape or branch-free valid invocation across randomized values | x |
| 7 | `_crypto_scalarmult_curve25519_pick_best_implementation` | `crypto_scalarmult/curve25519/scalarmult_curve25519.c:50`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_avx()) | x |
| 8 | `_crypto_sign_ed25519_detached` | `crypto_sign/ed25519/ref10/sign.c:50`; empty/one/many and boundary shapes; both outcomes of: if (siglen_p != NULL) | x |
| 9 | `_crypto_sign_ed25519_ref10_hinit` | `crypto_sign/ed25519/ref10/sign.c:12`; empty/one/many and boundary shapes; both outcomes of: if (prehashed) | x |
| 10 | `_crypto_sign_ed25519_verify_detached` | `crypto_sign/ed25519/ref10/open.c:15`; empty/one/many and boundary shapes; both outcomes of: if (sig[63] & 224); if ((sig[63] & 240) != 0 &&; if (ge25519_is_canonical(pk) == 0); if (ge25519_frombytes_negate_vartime(&A, pk) != 0 \|\|; … (5 direct branch axes total) | x |
| 11 | `_crypto_stream_chacha20_pick_best_implementation` | `crypto_stream/chacha20/stream_chacha20.c:176`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_avx512f()); if (sodium_runtime_has_avx2()); if (sodium_runtime_has_ssse3()); if (sodium_runtime_has_neon()) | x |
| 12 | `_crypto_stream_salsa20_pick_best_implementation` | `crypto_stream/salsa20/stream_salsa20.c:87`; empty/one/many and boundary shapes; both outcomes of: if (sodium_runtime_has_avx512f()); if (sodium_runtime_has_avx2()); if (sodium_runtime_has_sse2()); if (sodium_runtime_has_neon()) | x |
| 13 | `_sodium_alloc_init` | `sodium/utils.c:408`; empty/one/many and boundary shapes; both outcomes of: if (page_size_ > 0L); if (page_size < CANARY_SIZE \|\| page_size < sizeof(size_t)) | x |
| 14 | `_sodium_argon2_ctx` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 15 | `_sodium_argon2_decode_string` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 16 | `_sodium_argon2_encode_string` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 17 | `_sodium_argon2_fill_memory_blocks` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 18 | `_sodium_argon2_fill_segment_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 19 | `_sodium_argon2_finalize` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 20 | `_sodium_argon2_hash` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 21 | `_sodium_argon2_initialize` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 22 | `_sodium_argon2_validate_inputs` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 23 | `_sodium_argon2_verify` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 24 | `_sodium_argon2i_hash_encoded` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 25 | `_sodium_argon2i_hash_raw` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 26 | `_sodium_argon2i_verify` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 27 | `_sodium_argon2id_hash_encoded` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 28 | `_sodium_argon2id_hash_raw` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 29 | `_sodium_argon2id_verify` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 30 | `_sodium_blake2b` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 31 | `_sodium_blake2b_compress_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 32 | `_sodium_blake2b_final` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 33 | `_sodium_blake2b_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 34 | `_sodium_blake2b_init_key` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 35 | `_sodium_blake2b_init_key_salt_personal` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 36 | `_sodium_blake2b_init_param` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 37 | `_sodium_blake2b_init_salt_personal` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 38 | `_sodium_blake2b_long` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 39 | `_sodium_blake2b_pick_best_implementation` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 40 | `_sodium_blake2b_salt_personal` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 41 | `_sodium_blake2b_update` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 42 | `_sodium_core_h2c_string_to_hash` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 43 | `_sodium_escrypt_PBKDF2_SHA256` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 44 | `_sodium_escrypt_alloc_region` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 45 | `_sodium_escrypt_free_local` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 46 | `_sodium_escrypt_free_region` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 47 | `_sodium_escrypt_gensalt_r` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 48 | `_sodium_escrypt_init_local` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 49 | `_sodium_escrypt_kdf_nosse` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 50 | `_sodium_escrypt_parse_setting` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 51 | `_sodium_escrypt_r` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 52 | `_sodium_fe25519_frombytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 53 | `_sodium_fe25519_invert` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 54 | `_sodium_fe25519_tobytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 55 | `_sodium_ge25519_clear_cofactor` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 56 | `_sodium_ge25519_double_scalarmult_vartime` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 57 | `_sodium_ge25519_from_hash` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 58 | `_sodium_ge25519_from_uniform` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 59 | `_sodium_ge25519_frombytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 60 | `_sodium_ge25519_frombytes_negate_vartime` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 61 | `_sodium_ge25519_has_small_order` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 62 | `_sodium_ge25519_is_canonical` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 63 | `_sodium_ge25519_is_on_curve` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 64 | `_sodium_ge25519_is_on_main_subgroup` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 65 | `_sodium_ge25519_p1p1_to_p2` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 66 | `_sodium_ge25519_p1p1_to_p3` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 67 | `_sodium_ge25519_p2_to_p3` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 68 | `_sodium_ge25519_p3_add` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 69 | `_sodium_ge25519_p3_sub` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 70 | `_sodium_ge25519_p3_tobytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 71 | `_sodium_ge25519_scalarmult` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 72 | `_sodium_ge25519_scalarmult_base` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 73 | `_sodium_ge25519_tobytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 74 | `_sodium_keccak1600_ref_extract_bytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 75 | `_sodium_keccak1600_ref_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 76 | `_sodium_keccak1600_ref_permute_12` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 77 | `_sodium_keccak1600_ref_permute_24` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 78 | `_sodium_keccak1600_ref_xor_bytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 79 | `_sodium_mlkem768_ref_dec` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 80 | `_sodium_mlkem768_ref_enc` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 81 | `_sodium_mlkem768_ref_enc_deterministic` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 82 | `_sodium_mlkem768_ref_keypair` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 83 | `_sodium_mlkem768_ref_seed_keypair` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 84 | `_sodium_ristretto255_from_hash` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 85 | `_sodium_ristretto255_frombytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 86 | `_sodium_ristretto255_p3_tobytes` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 87 | `_sodium_runtime_get_cpu_features` | `sodium/runtime.c:319`; fixed-shape or branch-free valid invocation across randomized values | x |
| 88 | `_sodium_sc25519_invert` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 89 | `_sodium_sc25519_is_canonical` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 90 | `_sodium_sc25519_mul` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 91 | `_sodium_sc25519_muladd` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 92 | `_sodium_sc25519_reduce` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 93 | `_sodium_shake128_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 94 | `_sodium_shake128_ref_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 95 | `_sodium_shake128_ref_init_with_domain` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 96 | `_sodium_shake128_ref_squeeze` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 97 | `_sodium_shake128_ref_update` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 98 | `_sodium_shake256_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 99 | `_sodium_shake256_ref_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 100 | `_sodium_shake256_ref_init_with_domain` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 101 | `_sodium_shake256_ref_squeeze` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 102 | `_sodium_shake256_ref_update` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 103 | `_sodium_softaes_block_decrypt` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 104 | `_sodium_softaes_block_decryptlast` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 105 | `_sodium_softaes_block_encrypt` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 106 | `_sodium_softaes_block_encryptlast` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 107 | `_sodium_softaes_expand_key128` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 108 | `_sodium_softaes_expand_key256` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 109 | `_sodium_softaes_inv_mix_columns` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 110 | `_sodium_softaes_invert_key_schedule128` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 111 | `_sodium_softaes_invert_key_schedule256` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 112 | `_sodium_turboshake128_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 113 | `_sodium_turboshake128_ref_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 114 | `_sodium_turboshake128_ref_init_with_domain` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 115 | `_sodium_turboshake128_ref_squeeze` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 116 | `_sodium_turboshake128_ref_update` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 117 | `_sodium_turboshake256_ref` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 118 | `_sodium_turboshake256_ref_init` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 119 | `_sodium_turboshake256_ref_init_with_domain` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 120 | `_sodium_turboshake256_ref_squeeze` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 121 | `_sodium_turboshake256_ref_update` | ABI-resolved low-level or macro-generated function; randomized valid inputs according to its public declaration | x |
| 122 | `aegis128l_soft_implementation` | exported data object; compare all bytes after loader initialization | x |
| 123 | `aegis256_soft_implementation` | exported data object; compare all bytes after loader initialization | x |
| 124 | `crypto_aead_aegis128l_abytes` | `crypto_aead/aegis128l/aead_aegis128l.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 125 | `crypto_aead_aegis128l_decrypt` | `crypto_aead/aegis128l/aead_aegis128l.c:84`; empty/one/many and boundary shapes; both outcomes of: if (clen >= crypto_aead_aegis128l_ABYTES); if (mlen_p != NULL); if (ret == 0) | x |
| 126 | `crypto_aead_aegis128l_decrypt_detached` | `crypto_aead/aegis128l/aead_aegis128l.c:128`; empty/one/many and boundary shapes; both outcomes of: if (clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| | x |
| 127 | `crypto_aead_aegis128l_encrypt` | `crypto_aead/aegis128l/aead_aegis128l.c:61`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX); if (clen_p != NULL); if (ret == 0) | x |
| 128 | `crypto_aead_aegis128l_encrypt_detached` | `crypto_aead/aegis128l/aead_aegis128l.c:107`; empty/one/many and boundary shapes; both outcomes of: if (maclen_p != NULL); if (mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX \|\| | x |
| 129 | `crypto_aead_aegis128l_keybytes` | `crypto_aead/aegis128l/aead_aegis128l.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 130 | `crypto_aead_aegis128l_keygen` | `crypto_aead/aegis128l/aead_aegis128l.c:55`; fixed-shape or branch-free valid invocation across randomized values | x |
| 131 | `crypto_aead_aegis128l_messagebytes_max` | `crypto_aead/aegis128l/aead_aegis128l.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 132 | `crypto_aead_aegis128l_npubbytes` | `crypto_aead/aegis128l/aead_aegis128l.c:37`; fixed-shape or branch-free valid invocation across randomized values | x |
| 133 | `crypto_aead_aegis128l_nsecbytes` | `crypto_aead/aegis128l/aead_aegis128l.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 134 | `crypto_aead_aegis256_abytes` | `crypto_aead/aegis256/aead_aegis256.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 135 | `crypto_aead_aegis256_decrypt` | `crypto_aead/aegis256/aead_aegis256.c:84`; empty/one/many and boundary shapes; both outcomes of: if (clen >= crypto_aead_aegis256_ABYTES); if (mlen_p != NULL); if (ret == 0) | x |
| 136 | `crypto_aead_aegis256_decrypt_detached` | `crypto_aead/aegis256/aead_aegis256.c:128`; empty/one/many and boundary shapes; both outcomes of: if (clen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| | x |
| 137 | `crypto_aead_aegis256_encrypt` | `crypto_aead/aegis256/aead_aegis256.c:61`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX); if (clen_p != NULL); if (ret == 0) | x |
| 138 | `crypto_aead_aegis256_encrypt_detached` | `crypto_aead/aegis256/aead_aegis256.c:107`; empty/one/many and boundary shapes; both outcomes of: if (maclen_p != NULL); if (mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX \|\| | x |
| 139 | `crypto_aead_aegis256_keybytes` | `crypto_aead/aegis256/aead_aegis256.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 140 | `crypto_aead_aegis256_keygen` | `crypto_aead/aegis256/aead_aegis256.c:55`; fixed-shape or branch-free valid invocation across randomized values | x |
| 141 | `crypto_aead_aegis256_messagebytes_max` | `crypto_aead/aegis256/aead_aegis256.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 142 | `crypto_aead_aegis256_npubbytes` | `crypto_aead/aegis256/aead_aegis256.c:37`; fixed-shape or branch-free valid invocation across randomized values | x |
| 143 | `crypto_aead_aegis256_nsecbytes` | `crypto_aead/aegis256/aead_aegis256.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 144 | `crypto_aead_aes256gcm_abytes` | `crypto_aead/aes256gcm/aead_aes256gcm.c:27`; fixed-shape or branch-free valid invocation across randomized values | x |
| 145 | `crypto_aead_aes256gcm_beforenm` | `crypto_aead/aes256gcm/aead_aes256gcm.c:100`; fixed-shape or branch-free valid invocation across randomized values | x |
| 146 | `crypto_aead_aes256gcm_decrypt` | `crypto_aead/aes256gcm/aead_aes256gcm.c:90`; fixed-shape or branch-free valid invocation across randomized values | x |
| 147 | `crypto_aead_aes256gcm_decrypt_afternm` | `crypto_aead/aes256gcm/aead_aes256gcm.c:141`; fixed-shape or branch-free valid invocation across randomized values | x |
| 148 | `crypto_aead_aes256gcm_decrypt_detached` | `crypto_aead/aes256gcm/aead_aes256gcm.c:79`; fixed-shape or branch-free valid invocation across randomized values | x |
| 149 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | `crypto_aead/aes256gcm/aead_aes256gcm.c:130`; fixed-shape or branch-free valid invocation across randomized values | x |
| 150 | `crypto_aead_aes256gcm_encrypt` | `crypto_aead/aes256gcm/aead_aes256gcm.c:69`; fixed-shape or branch-free valid invocation across randomized values | x |
| 151 | `crypto_aead_aes256gcm_encrypt_afternm` | `crypto_aead/aes256gcm/aead_aes256gcm.c:119`; fixed-shape or branch-free valid invocation across randomized values | x |
| 152 | `crypto_aead_aes256gcm_encrypt_detached` | `crypto_aead/aes256gcm/aead_aes256gcm.c:58`; fixed-shape or branch-free valid invocation across randomized values | x |
| 153 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | `crypto_aead/aes256gcm/aead_aes256gcm.c:107`; fixed-shape or branch-free valid invocation across randomized values | x |
| 154 | `crypto_aead_aes256gcm_is_available` | `crypto_aead/aes256gcm/aead_aes256gcm.c:152`; fixed-shape or branch-free valid invocation across randomized values | x |
| 155 | `crypto_aead_aes256gcm_keybytes` | `crypto_aead/aes256gcm/aead_aes256gcm.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 156 | `crypto_aead_aes256gcm_keygen` | `crypto_aead/aes256gcm/aead_aes256gcm.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 157 | `crypto_aead_aes256gcm_messagebytes_max` | `crypto_aead/aes256gcm/aead_aes256gcm.c:39`; fixed-shape or branch-free valid invocation across randomized values | x |
| 158 | `crypto_aead_aes256gcm_npubbytes` | `crypto_aead/aes256gcm/aead_aes256gcm.c:21`; fixed-shape or branch-free valid invocation across randomized values | x |
| 159 | `crypto_aead_aes256gcm_nsecbytes` | `crypto_aead/aes256gcm/aead_aes256gcm.c:15`; fixed-shape or branch-free valid invocation across randomized values | x |
| 160 | `crypto_aead_aes256gcm_statebytes` | `crypto_aead/aes256gcm/aead_aes256gcm.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 161 | `crypto_aead_chacha20poly1305_abytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:415`; fixed-shape or branch-free valid invocation across randomized values | x |
| 162 | `crypto_aead_chacha20poly1305_decrypt` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:246`; empty/one/many and boundary shapes; both outcomes of: if (clen >= crypto_aead_chacha20poly1305_ABYTES); if (mlen_p != NULL); if (ret == 0) | x |
| 163 | `crypto_aead_chacha20poly1305_decrypt_detached` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:195`; empty/one/many and boundary shapes; both outcomes of: if (m == NULL); if (ret != 0) | x |
| 164 | `crypto_aead_chacha20poly1305_encrypt` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:76`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX); if (clen_p != NULL); if (ret == 0) | x |
| 165 | `crypto_aead_chacha20poly1305_encrypt_detached` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:23`; empty/one/many and boundary shapes; both outcomes of: if (cl > STREAM_POLY1305_CHUNK); if (maclen_p != NULL) | x |
| 166 | `crypto_aead_chacha20poly1305_ietf_abytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:379`; fixed-shape or branch-free valid invocation across randomized values | x |
| 167 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:331`; empty/one/many and boundary shapes; both outcomes of: if (clen >= crypto_aead_chacha20poly1305_ietf_ABYTES); if (mlen_p != NULL); if (ret == 0) | x |
| 168 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:276`; empty/one/many and boundary shapes; both outcomes of: if (m == NULL); if (ret != 0) | x |
| 169 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:164`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX); if (clen_p != NULL); if (ret == 0) | x |
| 170 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:107`; empty/one/many and boundary shapes; both outcomes of: if (cl > STREAM_POLY1305_CHUNK); if (maclen_p != NULL) | x |
| 171 | `crypto_aead_chacha20poly1305_ietf_keybytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:361`; fixed-shape or branch-free valid invocation across randomized values | x |
| 172 | `crypto_aead_chacha20poly1305_ietf_keygen` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:391`; fixed-shape or branch-free valid invocation across randomized values | x |
| 173 | `crypto_aead_chacha20poly1305_ietf_messagebytes_max` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:385`; fixed-shape or branch-free valid invocation across randomized values | x |
| 174 | `crypto_aead_chacha20poly1305_ietf_npubbytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:367`; fixed-shape or branch-free valid invocation across randomized values | x |
| 175 | `crypto_aead_chacha20poly1305_ietf_nsecbytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:373`; fixed-shape or branch-free valid invocation across randomized values | x |
| 176 | `crypto_aead_chacha20poly1305_keybytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:397`; fixed-shape or branch-free valid invocation across randomized values | x |
| 177 | `crypto_aead_chacha20poly1305_keygen` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:427`; fixed-shape or branch-free valid invocation across randomized values | x |
| 178 | `crypto_aead_chacha20poly1305_messagebytes_max` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:421`; fixed-shape or branch-free valid invocation across randomized values | x |
| 179 | `crypto_aead_chacha20poly1305_npubbytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:403`; fixed-shape or branch-free valid invocation across randomized values | x |
| 180 | `crypto_aead_chacha20poly1305_nsecbytes` | `crypto_aead/chacha20poly1305/aead_chacha20poly1305.c:409`; fixed-shape or branch-free valid invocation across randomized values | x |
| 181 | `crypto_aead_xchacha20poly1305_ietf_abytes` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:272`; fixed-shape or branch-free valid invocation across randomized values | x |
| 182 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:224`; empty/one/many and boundary shapes; both outcomes of: if (clen >= crypto_aead_xchacha20poly1305_ietf_ABYTES); if (mlen_p != NULL); if (ret == 0) | x |
| 183 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:200`; fixed-shape or branch-free valid invocation across randomized values | x |
| 184 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:172`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX); if (clen_p != NULL); if (ret == 0) | x |
| 185 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:146`; fixed-shape or branch-free valid invocation across randomized values | x |
| 186 | `crypto_aead_xchacha20poly1305_ietf_keybytes` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:254`; fixed-shape or branch-free valid invocation across randomized values | x |
| 187 | `crypto_aead_xchacha20poly1305_ietf_keygen` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:284`; fixed-shape or branch-free valid invocation across randomized values | x |
| 188 | `crypto_aead_xchacha20poly1305_ietf_messagebytes_max` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:278`; fixed-shape or branch-free valid invocation across randomized values | x |
| 189 | `crypto_aead_xchacha20poly1305_ietf_npubbytes` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:260`; fixed-shape or branch-free valid invocation across randomized values | x |
| 190 | `crypto_aead_xchacha20poly1305_ietf_nsecbytes` | `crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c:266`; fixed-shape or branch-free valid invocation across randomized values | x |
| 191 | `crypto_auth` | `crypto_auth/crypto_auth.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 192 | `crypto_auth_bytes` | `crypto_auth/crypto_auth.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 193 | `crypto_auth_hmacsha256` | `crypto_auth/hmacsha256/auth_hmacsha256.c:101`; fixed-shape or branch-free valid invocation across randomized values | x |
| 194 | `crypto_auth_hmacsha256_bytes` | `crypto_auth/hmacsha256/auth_hmacsha256.c:14`; fixed-shape or branch-free valid invocation across randomized values | x |
| 195 | `crypto_auth_hmacsha256_final` | `crypto_auth/hmacsha256/auth_hmacsha256.c:86`; fixed-shape or branch-free valid invocation across randomized values | x |
| 196 | `crypto_auth_hmacsha256_init` | `crypto_auth/hmacsha256/auth_hmacsha256.c:38`; empty/one/many and boundary shapes; both outcomes of: if (keylen > 64); } else if (key == NULL); if (keylen > 0) | x |
| 197 | `crypto_auth_hmacsha256_keybytes` | `crypto_auth/hmacsha256/auth_hmacsha256.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 198 | `crypto_auth_hmacsha256_keygen` | `crypto_auth/hmacsha256/auth_hmacsha256.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 199 | `crypto_auth_hmacsha256_statebytes` | `crypto_auth/hmacsha256/auth_hmacsha256.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 200 | `crypto_auth_hmacsha256_update` | `crypto_auth/hmacsha256/auth_hmacsha256.c:77`; fixed-shape or branch-free valid invocation across randomized values | x |
| 201 | `crypto_auth_hmacsha256_verify` | `crypto_auth/hmacsha256/auth_hmacsha256.c:114`; fixed-shape or branch-free valid invocation across randomized values | x |
| 202 | `crypto_auth_hmacsha512` | `crypto_auth/hmacsha512/auth_hmacsha512.c:101`; fixed-shape or branch-free valid invocation across randomized values | x |
| 203 | `crypto_auth_hmacsha512256` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:69`; fixed-shape or branch-free valid invocation across randomized values | x |
| 204 | `crypto_auth_hmacsha512256_bytes` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:14`; fixed-shape or branch-free valid invocation across randomized values | x |
| 205 | `crypto_auth_hmacsha512256_final` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:56`; fixed-shape or branch-free valid invocation across randomized values | x |
| 206 | `crypto_auth_hmacsha512256_init` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:39`; fixed-shape or branch-free valid invocation across randomized values | x |
| 207 | `crypto_auth_hmacsha512256_keybytes` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 208 | `crypto_auth_hmacsha512256_keygen` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 209 | `crypto_auth_hmacsha512256_statebytes` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 210 | `crypto_auth_hmacsha512256_update` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:47`; fixed-shape or branch-free valid invocation across randomized values | x |
| 211 | `crypto_auth_hmacsha512256_verify` | `crypto_auth/hmacsha512256/auth_hmacsha512256.c:83`; fixed-shape or branch-free valid invocation across randomized values | x |
| 212 | `crypto_auth_hmacsha512_bytes` | `crypto_auth/hmacsha512/auth_hmacsha512.c:14`; fixed-shape or branch-free valid invocation across randomized values | x |
| 213 | `crypto_auth_hmacsha512_final` | `crypto_auth/hmacsha512/auth_hmacsha512.c:86`; fixed-shape or branch-free valid invocation across randomized values | x |
| 214 | `crypto_auth_hmacsha512_init` | `crypto_auth/hmacsha512/auth_hmacsha512.c:38`; empty/one/many and boundary shapes; both outcomes of: if (keylen > 128); } else if (key == NULL); if (keylen > 0) | x |
| 215 | `crypto_auth_hmacsha512_keybytes` | `crypto_auth/hmacsha512/auth_hmacsha512.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 216 | `crypto_auth_hmacsha512_keygen` | `crypto_auth/hmacsha512/auth_hmacsha512.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 217 | `crypto_auth_hmacsha512_statebytes` | `crypto_auth/hmacsha512/auth_hmacsha512.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 218 | `crypto_auth_hmacsha512_update` | `crypto_auth/hmacsha512/auth_hmacsha512.c:77`; fixed-shape or branch-free valid invocation across randomized values | x |
| 219 | `crypto_auth_hmacsha512_verify` | `crypto_auth/hmacsha512/auth_hmacsha512.c:114`; fixed-shape or branch-free valid invocation across randomized values | x |
| 220 | `crypto_auth_keybytes` | `crypto_auth/crypto_auth.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 221 | `crypto_auth_keygen` | `crypto_auth/crypto_auth.c:38`; fixed-shape or branch-free valid invocation across randomized values | x |
| 222 | `crypto_auth_primitive` | `crypto_auth/crypto_auth.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 223 | `crypto_auth_verify` | `crypto_auth/crypto_auth.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 224 | `crypto_box` | `crypto_box/crypto_box.c:101`; fixed-shape or branch-free valid invocation across randomized values | x |
| 225 | `crypto_box_afternm` | `crypto_box/crypto_box.c:85`; fixed-shape or branch-free valid invocation across randomized values | x |
| 226 | `crypto_box_beforenm` | `crypto_box/crypto_box.c:78`; fixed-shape or branch-free valid invocation across randomized values | x |
| 227 | `crypto_box_beforenmbytes` | `crypto_box/crypto_box.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 228 | `crypto_box_boxzerobytes` | `crypto_box/crypto_box.c:41`; fixed-shape or branch-free valid invocation across randomized values | x |
| 229 | `crypto_box_curve25519xchacha20poly1305_beforenm` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:41`; empty/one/many and boundary shapes; both outcomes of: if (crypto_scalarmult_curve25519(s, sk, pk) != 0) | x |
| 230 | `crypto_box_curve25519xchacha20poly1305_beforenmbytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:186`; fixed-shape or branch-free valid invocation across randomized values | x |
| 231 | `crypto_box_curve25519xchacha20poly1305_detached` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:66`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) | x |
| 232 | `crypto_box_curve25519xchacha20poly1305_detached_afternm` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:58`; fixed-shape or branch-free valid invocation across randomized values | x |
| 233 | `crypto_box_curve25519xchacha20poly1305_easy` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:101`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) | x |
| 234 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:87`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX) | x |
| 235 | `crypto_box_curve25519xchacha20poly1305_keypair` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 236 | `crypto_box_curve25519xchacha20poly1305_macbytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:198`; fixed-shape or branch-free valid invocation across randomized values | x |
| 237 | `crypto_box_curve25519xchacha20poly1305_messagebytes_max` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:204`; fixed-shape or branch-free valid invocation across randomized values | x |
| 238 | `crypto_box_curve25519xchacha20poly1305_noncebytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:192`; fixed-shape or branch-free valid invocation across randomized values | x |
| 239 | `crypto_box_curve25519xchacha20poly1305_open_detached` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:123`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_curve25519xchacha20poly1305_beforenm(k, pk, sk) != 0) | x |
| 240 | `crypto_box_curve25519xchacha20poly1305_open_detached_afternm` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:114`; fixed-shape or branch-free valid invocation across randomized values | x |
| 241 | `crypto_box_curve25519xchacha20poly1305_open_easy` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:155`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) | x |
| 242 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:142`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_curve25519xchacha20poly1305_MACBYTES) | x |
| 243 | `crypto_box_curve25519xchacha20poly1305_publickeybytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:174`; fixed-shape or branch-free valid invocation across randomized values | x |
| 244 | `crypto_box_curve25519xchacha20poly1305_seal` | `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:30`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX); if (crypto_box_curve25519xchacha20poly1305_keypair(epk, esk) != 0) | x |
| 245 | `crypto_box_curve25519xchacha20poly1305_seal_open` | `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:56`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES) | x |
| 246 | `crypto_box_curve25519xchacha20poly1305_sealbytes` | `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c:78`; fixed-shape or branch-free valid invocation across randomized values | x |
| 247 | `crypto_box_curve25519xchacha20poly1305_secretkeybytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:180`; fixed-shape or branch-free valid invocation across randomized values | x |
| 248 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 249 | `crypto_box_curve25519xchacha20poly1305_seedbytes` | `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c:168`; fixed-shape or branch-free valid invocation across randomized values | x |
| 250 | `crypto_box_curve25519xsalsa20poly1305` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:72`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) | x |
| 251 | `crypto_box_curve25519xsalsa20poly1305_afternm` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:52`; fixed-shape or branch-free valid invocation across randomized values | x |
| 252 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:35`; empty/one/many and boundary shapes; both outcomes of: if (crypto_scalarmult_curve25519(s, sk, pk) != 0) | x |
| 253 | `crypto_box_curve25519xsalsa20poly1305_beforenmbytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:126`; fixed-shape or branch-free valid invocation across randomized values | x |
| 254 | `crypto_box_curve25519xsalsa20poly1305_boxzerobytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:144`; fixed-shape or branch-free valid invocation across randomized values | x |
| 255 | `crypto_box_curve25519xsalsa20poly1305_keypair` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 256 | `crypto_box_curve25519xsalsa20poly1305_macbytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:150`; fixed-shape or branch-free valid invocation across randomized values | x |
| 257 | `crypto_box_curve25519xsalsa20poly1305_messagebytes_max` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:156`; fixed-shape or branch-free valid invocation across randomized values | x |
| 258 | `crypto_box_curve25519xsalsa20poly1305_noncebytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:132`; fixed-shape or branch-free valid invocation across randomized values | x |
| 259 | `crypto_box_curve25519xsalsa20poly1305_open` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:91`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) != 0) | x |
| 260 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:62`; fixed-shape or branch-free valid invocation across randomized values | x |
| 261 | `crypto_box_curve25519xsalsa20poly1305_publickeybytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:114`; fixed-shape or branch-free valid invocation across randomized values | x |
| 262 | `crypto_box_curve25519xsalsa20poly1305_secretkeybytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:120`; fixed-shape or branch-free valid invocation across randomized values | x |
| 263 | `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 264 | `crypto_box_curve25519xsalsa20poly1305_seedbytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:108`; fixed-shape or branch-free valid invocation across randomized values | x |
| 265 | `crypto_box_curve25519xsalsa20poly1305_zerobytes` | `crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c:138`; fixed-shape or branch-free valid invocation across randomized values | x |
| 266 | `crypto_box_detached` | `crypto_box/crypto_box_easy.c:21`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_beforenm(k, pk, sk) != 0) | x |
| 267 | `crypto_box_detached_afternm` | `crypto_box/crypto_box_easy.c:13`; fixed-shape or branch-free valid invocation across randomized values | x |
| 268 | `crypto_box_easy` | `crypto_box/crypto_box_easy.c:52`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_MESSAGEBYTES_MAX) | x |
| 269 | `crypto_box_easy_afternm` | `crypto_box/crypto_box_easy.c:40`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_MESSAGEBYTES_MAX) | x |
| 270 | `crypto_box_keypair` | `crypto_box/crypto_box.c:72`; fixed-shape or branch-free valid invocation across randomized values | x |
| 271 | `crypto_box_macbytes` | `crypto_box/crypto_box.c:47`; fixed-shape or branch-free valid invocation across randomized values | x |
| 272 | `crypto_box_messagebytes_max` | `crypto_box/crypto_box.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 273 | `crypto_box_noncebytes` | `crypto_box/crypto_box.c:29`; fixed-shape or branch-free valid invocation across randomized values | x |
| 274 | `crypto_box_open` | `crypto_box/crypto_box.c:109`; fixed-shape or branch-free valid invocation across randomized values | x |
| 275 | `crypto_box_open_afternm` | `crypto_box/crypto_box.c:93`; fixed-shape or branch-free valid invocation across randomized values | x |
| 276 | `crypto_box_open_detached` | `crypto_box/crypto_box_easy.c:74`; empty/one/many and boundary shapes; both outcomes of: if (crypto_box_beforenm(k, pk, sk) != 0) | x |
| 277 | `crypto_box_open_detached_afternm` | `crypto_box/crypto_box_easy.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 278 | `crypto_box_open_easy` | `crypto_box/crypto_box_easy.c:105`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_MACBYTES) | x |
| 279 | `crypto_box_open_easy_afternm` | `crypto_box/crypto_box_easy.c:92`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_MACBYTES) | x |
| 280 | `crypto_box_primitive` | `crypto_box/crypto_box.c:59`; fixed-shape or branch-free valid invocation across randomized values | x |
| 281 | `crypto_box_publickeybytes` | `crypto_box/crypto_box.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 282 | `crypto_box_seal` | `crypto_box/crypto_box_seal.c:25`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_box_MESSAGEBYTES_MAX); if (crypto_box_keypair(epk, esk) != 0) | x |
| 283 | `crypto_box_seal_open` | `crypto_box/crypto_box_seal.c:49`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_box_SEALBYTES) | x |
| 284 | `crypto_box_sealbytes` | `crypto_box/crypto_box_seal.c:67`; fixed-shape or branch-free valid invocation across randomized values | x |
| 285 | `crypto_box_secretkeybytes` | `crypto_box/crypto_box.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 286 | `crypto_box_seed_keypair` | `crypto_box/crypto_box.c:65`; fixed-shape or branch-free valid invocation across randomized values | x |
| 287 | `crypto_box_seedbytes` | `crypto_box/crypto_box.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 288 | `crypto_box_zerobytes` | `crypto_box/crypto_box.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 289 | `crypto_core_ed25519_add` | `crypto_core/ed25519/core_ed25519.c:29`; empty/one/many and boundary shapes; both outcomes of: if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| | x |
| 290 | `crypto_core_ed25519_bytes` | `crypto_core/ed25519/core_ed25519.c:264`; fixed-shape or branch-free valid invocation across randomized values | x |
| 291 | `crypto_core_ed25519_from_string` | `crypto_core/ed25519/core_ed25519.c:101`; empty/one/many and boundary shapes; both outcomes of: if (_string_to_points(px, 2, ctx, ctx_len, msg, msg_len, hash_alg) != 0) | x |
| 292 | `crypto_core_ed25519_from_string_nu` | `crypto_core/ed25519/core_ed25519.c:92`; fixed-shape or branch-free valid invocation across randomized values | x |
| 293 | `crypto_core_ed25519_hashbytes` | `crypto_core/ed25519/core_ed25519.c:282`; fixed-shape or branch-free valid invocation across randomized values | x |
| 294 | `crypto_core_ed25519_is_valid_point` | `crypto_core/ed25519/core_ed25519.c:14`; empty/one/many and boundary shapes; both outcomes of: if (ge25519_is_canonical(p) == 0 \|\| | x |
| 295 | `crypto_core_ed25519_nonreducedscalarbytes` | `crypto_core/ed25519/core_ed25519.c:270`; fixed-shape or branch-free valid invocation across randomized values | x |
| 296 | `crypto_core_ed25519_random` | `crypto_core/ed25519/core_ed25519.c:116`; fixed-shape or branch-free valid invocation across randomized values | x |
| 297 | `crypto_core_ed25519_scalar_add` | `crypto_core/ed25519/core_ed25519.c:188`; fixed-shape or branch-free valid invocation across randomized values | x |
| 298 | `crypto_core_ed25519_scalar_complement` | `crypto_core/ed25519/core_ed25519.c:168`; fixed-shape or branch-free valid invocation across randomized values | x |
| 299 | `crypto_core_ed25519_scalar_from_string` | `crypto_core/ed25519/core_ed25519.c:240`; empty/one/many and boundary shapes; both outcomes of: if (core_h2c_string_to_hash(h_be, sizeof h_be, ctx, ctx_len, msg, msg_len, | x |
| 300 | `crypto_core_ed25519_scalar_invert` | `crypto_core/ed25519/core_ed25519.c:135`; fixed-shape or branch-free valid invocation across randomized values | x |
| 301 | `crypto_core_ed25519_scalar_is_canonical` | `crypto_core/ed25519/core_ed25519.c:232`; fixed-shape or branch-free valid invocation across randomized values | x |
| 302 | `crypto_core_ed25519_scalar_mul` | `crypto_core/ed25519/core_ed25519.c:213`; fixed-shape or branch-free valid invocation across randomized values | x |
| 303 | `crypto_core_ed25519_scalar_negate` | `crypto_core/ed25519/core_ed25519.c:150`; fixed-shape or branch-free valid invocation across randomized values | x |
| 304 | `crypto_core_ed25519_scalar_random` | `crypto_core/ed25519/core_ed25519.c:125`; fixed-shape or branch-free valid invocation across randomized values | x |
| 305 | `crypto_core_ed25519_scalar_reduce` | `crypto_core/ed25519/core_ed25519.c:220`; fixed-shape or branch-free valid invocation across randomized values | x |
| 306 | `crypto_core_ed25519_scalar_sub` | `crypto_core/ed25519/core_ed25519.c:203`; fixed-shape or branch-free valid invocation across randomized values | x |
| 307 | `crypto_core_ed25519_scalarbytes` | `crypto_core/ed25519/core_ed25519.c:288`; fixed-shape or branch-free valid invocation across randomized values | x |
| 308 | `crypto_core_ed25519_sub` | `crypto_core/ed25519/core_ed25519.c:45`; empty/one/many and boundary shapes; both outcomes of: if (ge25519_frombytes(&p_p3, p) != 0 \|\| ge25519_is_on_curve(&p_p3) == 0 \|\| | x |
| 309 | `crypto_core_ed25519_uniformbytes` | `crypto_core/ed25519/core_ed25519.c:276`; fixed-shape or branch-free valid invocation across randomized values | x |
| 310 | `crypto_core_hchacha20` | `crypto_core/hchacha20/core_hchacha20.c:17`; empty/one/many and boundary shapes; both outcomes of: if (c == NULL) | x |
| 311 | `crypto_core_hchacha20_constbytes` | `crypto_core/hchacha20/core_hchacha20.c:90`; fixed-shape or branch-free valid invocation across randomized values | x |
| 312 | `crypto_core_hchacha20_inputbytes` | `crypto_core/hchacha20/core_hchacha20.c:78`; fixed-shape or branch-free valid invocation across randomized values | x |
| 313 | `crypto_core_hchacha20_keybytes` | `crypto_core/hchacha20/core_hchacha20.c:84`; fixed-shape or branch-free valid invocation across randomized values | x |
| 314 | `crypto_core_hchacha20_outputbytes` | `crypto_core/hchacha20/core_hchacha20.c:72`; fixed-shape or branch-free valid invocation across randomized values | x |
| 315 | `crypto_core_hsalsa20` | `crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c:17`; empty/one/many and boundary shapes; both outcomes of: if (c == NULL) | x |
| 316 | `crypto_core_hsalsa20_constbytes` | `crypto_core/hsalsa20/core_hsalsa20.c:19`; fixed-shape or branch-free valid invocation across randomized values | x |
| 317 | `crypto_core_hsalsa20_inputbytes` | `crypto_core/hsalsa20/core_hsalsa20.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 318 | `crypto_core_hsalsa20_keybytes` | `crypto_core/hsalsa20/core_hsalsa20.c:14`; fixed-shape or branch-free valid invocation across randomized values | x |
| 319 | `crypto_core_hsalsa20_outputbytes` | `crypto_core/hsalsa20/core_hsalsa20.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 320 | `crypto_core_keccak1600_extract_bytes` | `crypto_core/keccak1600/keccak1600.c:40`; fixed-shape or branch-free valid invocation across randomized values | x |
| 321 | `crypto_core_keccak1600_init` | `crypto_core/keccak1600/keccak1600.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 322 | `crypto_core_keccak1600_permute_12` | `crypto_core/keccak1600/keccak1600.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 323 | `crypto_core_keccak1600_permute_24` | `crypto_core/keccak1600/keccak1600.c:48`; fixed-shape or branch-free valid invocation across randomized values | x |
| 324 | `crypto_core_keccak1600_statebytes` | `crypto_core/keccak1600/keccak1600.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 325 | `crypto_core_keccak1600_xor_bytes` | `crypto_core/keccak1600/keccak1600.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 326 | `crypto_core_ristretto255_add` | `crypto_core/ed25519/core_ristretto255.c:27`; empty/one/many and boundary shapes; both outcomes of: if (ristretto255_frombytes(&p_p3, p) != 0 \|\| | x |
| 327 | `crypto_core_ristretto255_bytes` | `crypto_core/ed25519/core_ristretto255.c:173`; fixed-shape or branch-free valid invocation across randomized values | x |
| 328 | `crypto_core_ristretto255_from_hash` | `crypto_core/ed25519/core_ristretto255.c:59`; fixed-shape or branch-free valid invocation across randomized values | x |
| 329 | `crypto_core_ristretto255_from_string` | `crypto_core/ed25519/core_ristretto255.c:84`; fixed-shape or branch-free valid invocation across randomized values | x |
| 330 | `crypto_core_ristretto255_hashbytes` | `crypto_core/ed25519/core_ristretto255.c:185`; fixed-shape or branch-free valid invocation across randomized values | x |
| 331 | `crypto_core_ristretto255_is_valid_point` | `crypto_core/ed25519/core_ristretto255.c:16`; empty/one/many and boundary shapes; both outcomes of: if (ristretto255_frombytes(&p_p3, p) != 0) | x |
| 332 | `crypto_core_ristretto255_nonreducedscalarbytes` | `crypto_core/ed25519/core_ristretto255.c:179`; fixed-shape or branch-free valid invocation across randomized values | x |
| 333 | `crypto_core_ristretto255_random` | `crypto_core/ed25519/core_ristretto255.c:93`; fixed-shape or branch-free valid invocation across randomized values | x |
| 334 | `crypto_core_ristretto255_scalar_add` | `crypto_core/ed25519/core_ristretto255.c:129`; fixed-shape or branch-free valid invocation across randomized values | x |
| 335 | `crypto_core_ristretto255_scalar_complement` | `crypto_core/ed25519/core_ristretto255.c:122`; fixed-shape or branch-free valid invocation across randomized values | x |
| 336 | `crypto_core_ristretto255_scalar_from_string` | `crypto_core/ed25519/core_ristretto255.c:163`; fixed-shape or branch-free valid invocation across randomized values | x |
| 337 | `crypto_core_ristretto255_scalar_invert` | `crypto_core/ed25519/core_ristretto255.c:108`; fixed-shape or branch-free valid invocation across randomized values | x |
| 338 | `crypto_core_ristretto255_scalar_is_canonical` | `crypto_core/ed25519/core_ristretto255.c:157`; fixed-shape or branch-free valid invocation across randomized values | x |
| 339 | `crypto_core_ristretto255_scalar_mul` | `crypto_core/ed25519/core_ristretto255.c:143`; fixed-shape or branch-free valid invocation across randomized values | x |
| 340 | `crypto_core_ristretto255_scalar_negate` | `crypto_core/ed25519/core_ristretto255.c:115`; fixed-shape or branch-free valid invocation across randomized values | x |
| 341 | `crypto_core_ristretto255_scalar_random` | `crypto_core/ed25519/core_ristretto255.c:102`; fixed-shape or branch-free valid invocation across randomized values | x |
| 342 | `crypto_core_ristretto255_scalar_reduce` | `crypto_core/ed25519/core_ristretto255.c:150`; fixed-shape or branch-free valid invocation across randomized values | x |
| 343 | `crypto_core_ristretto255_scalar_sub` | `crypto_core/ed25519/core_ristretto255.c:136`; fixed-shape or branch-free valid invocation across randomized values | x |
| 344 | `crypto_core_ristretto255_scalarbytes` | `crypto_core/ed25519/core_ristretto255.c:191`; fixed-shape or branch-free valid invocation across randomized values | x |
| 345 | `crypto_core_ristretto255_sub` | `crypto_core/ed25519/core_ristretto255.c:43`; empty/one/many and boundary shapes; both outcomes of: if (ristretto255_frombytes(&p_p3, p) != 0 \|\| | x |
| 346 | `crypto_core_salsa20` | `crypto_core/salsa/ref/core_salsa_ref.c:98`; fixed-shape or branch-free valid invocation across randomized values | x |
| 347 | `crypto_core_salsa2012` | `crypto_core/salsa/ref/core_salsa_ref.c:132`; fixed-shape or branch-free valid invocation across randomized values | x |
| 348 | `crypto_core_salsa2012_constbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:158`; fixed-shape or branch-free valid invocation across randomized values | x |
| 349 | `crypto_core_salsa2012_inputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:146`; fixed-shape or branch-free valid invocation across randomized values | x |
| 350 | `crypto_core_salsa2012_keybytes` | `crypto_core/salsa/ref/core_salsa_ref.c:152`; fixed-shape or branch-free valid invocation across randomized values | x |
| 351 | `crypto_core_salsa2012_outputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:140`; fixed-shape or branch-free valid invocation across randomized values | x |
| 352 | `crypto_core_salsa208` | `crypto_core/salsa/ref/core_salsa_ref.c:164`; fixed-shape or branch-free valid invocation across randomized values | x |
| 353 | `crypto_core_salsa208_constbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:190`; fixed-shape or branch-free valid invocation across randomized values | x |
| 354 | `crypto_core_salsa208_inputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:178`; fixed-shape or branch-free valid invocation across randomized values | x |
| 355 | `crypto_core_salsa208_keybytes` | `crypto_core/salsa/ref/core_salsa_ref.c:184`; fixed-shape or branch-free valid invocation across randomized values | x |
| 356 | `crypto_core_salsa208_outputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:172`; fixed-shape or branch-free valid invocation across randomized values | x |
| 357 | `crypto_core_salsa20_constbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:124`; fixed-shape or branch-free valid invocation across randomized values | x |
| 358 | `crypto_core_salsa20_inputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:112`; fixed-shape or branch-free valid invocation across randomized values | x |
| 359 | `crypto_core_salsa20_keybytes` | `crypto_core/salsa/ref/core_salsa_ref.c:118`; fixed-shape or branch-free valid invocation across randomized values | x |
| 360 | `crypto_core_salsa20_outputbytes` | `crypto_core/salsa/ref/core_salsa_ref.c:106`; fixed-shape or branch-free valid invocation across randomized values | x |
| 361 | `crypto_generichash` | `crypto_generichash/crypto_generichash.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 362 | `crypto_generichash_blake2b` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:12`; empty/one/many and boundary shapes; both outcomes of: if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| | x |
| 363 | `crypto_generichash_blake2b_bytes` | `crypto_generichash/blake2b/generichash_blake2.c:15`; fixed-shape or branch-free valid invocation across randomized values | x |
| 364 | `crypto_generichash_blake2b_bytes_max` | `crypto_generichash/blake2b/generichash_blake2.c:10`; fixed-shape or branch-free valid invocation across randomized values | x |
| 365 | `crypto_generichash_blake2b_bytes_min` | `crypto_generichash/blake2b/generichash_blake2.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 366 | `crypto_generichash_blake2b_final` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:104`; fixed-shape or branch-free valid invocation across randomized values | x |
| 367 | `crypto_generichash_blake2b_init` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:46`; empty/one/many and boundary shapes; both outcomes of: if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\|; if (key == NULL \|\| keylen <= 0U); if (blake2b_init((blake2b_state *) (void *) state, (uint8_t) outlen) != 0); } else if (blake2b_init_key((blake2b_state *) (void *) state, (uint8_t) outlen, key, | x |
| 368 | `crypto_generichash_blake2b_init_salt_personal` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:69`; empty/one/many and boundary shapes; both outcomes of: if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\|; if (key == NULL \|\| keylen <= 0U); if (blake2b_init_salt_personal((blake2b_state *) (void *) state,; } else if (blake2b_init_key_salt_personal((blake2b_state *) (void *) state, | x |
| 369 | `crypto_generichash_blake2b_keybytes` | `crypto_generichash/blake2b/generichash_blake2.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 370 | `crypto_generichash_blake2b_keybytes_max` | `crypto_generichash/blake2b/generichash_blake2.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 371 | `crypto_generichash_blake2b_keybytes_min` | `crypto_generichash/blake2b/generichash_blake2.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 372 | `crypto_generichash_blake2b_keygen` | `crypto_generichash/blake2b/generichash_blake2.c:52`; fixed-shape or branch-free valid invocation across randomized values | x |
| 373 | `crypto_generichash_blake2b_personalbytes` | `crypto_generichash/blake2b/generichash_blake2.c:40`; fixed-shape or branch-free valid invocation across randomized values | x |
| 374 | `crypto_generichash_blake2b_salt_personal` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:28`; empty/one/many and boundary shapes; both outcomes of: if (outlen <= 0U \|\| outlen > BLAKE2B_OUTBYTES \|\| | x |
| 375 | `crypto_generichash_blake2b_saltbytes` | `crypto_generichash/blake2b/generichash_blake2.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 376 | `crypto_generichash_blake2b_statebytes` | `crypto_generichash/blake2b/generichash_blake2.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 377 | `crypto_generichash_blake2b_update` | `crypto_generichash/blake2b/ref/generichash_blake2b.c:95`; fixed-shape or branch-free valid invocation across randomized values | x |
| 378 | `crypto_generichash_bytes` | `crypto_generichash/crypto_generichash.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 379 | `crypto_generichash_bytes_max` | `crypto_generichash/crypto_generichash.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 380 | `crypto_generichash_bytes_min` | `crypto_generichash/crypto_generichash.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 381 | `crypto_generichash_final` | `crypto_generichash/crypto_generichash.c:80`; fixed-shape or branch-free valid invocation across randomized values | x |
| 382 | `crypto_generichash_init` | `crypto_generichash/crypto_generichash.c:62`; fixed-shape or branch-free valid invocation across randomized values | x |
| 383 | `crypto_generichash_keybytes` | `crypto_generichash/crypto_generichash.c:36`; fixed-shape or branch-free valid invocation across randomized values | x |
| 384 | `crypto_generichash_keybytes_max` | `crypto_generichash/crypto_generichash.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 385 | `crypto_generichash_keybytes_min` | `crypto_generichash/crypto_generichash.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 386 | `crypto_generichash_keygen` | `crypto_generichash/crypto_generichash.c:88`; fixed-shape or branch-free valid invocation across randomized values | x |
| 387 | `crypto_generichash_primitive` | `crypto_generichash/crypto_generichash.c:42`; fixed-shape or branch-free valid invocation across randomized values | x |
| 388 | `crypto_generichash_statebytes` | `crypto_generichash/crypto_generichash.c:48`; fixed-shape or branch-free valid invocation across randomized values | x |
| 389 | `crypto_generichash_update` | `crypto_generichash/crypto_generichash.c:71`; fixed-shape or branch-free valid invocation across randomized values | x |
| 390 | `crypto_hash` | `crypto_hash/crypto_hash.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 391 | `crypto_hash_bytes` | `crypto_hash/crypto_hash.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 392 | `crypto_hash_primitive` | `crypto_hash/crypto_hash.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 393 | `crypto_hash_sha256` | `crypto_hash/sha256/cp/hash_sha256_cp.c:405`; fixed-shape or branch-free valid invocation across randomized values | x |
| 394 | `crypto_hash_sha256_bytes` | `crypto_hash/sha256/hash_sha256.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 395 | `crypto_hash_sha256_final` | `crypto_hash/sha256/cp/hash_sha256_cp.c:392`; fixed-shape or branch-free valid invocation across randomized values | x |
| 396 | `crypto_hash_sha256_init` | `crypto_hash/sha256/cp/hash_sha256_cp.c:336`; fixed-shape or branch-free valid invocation across randomized values | x |
| 397 | `crypto_hash_sha256_statebytes` | `crypto_hash/sha256/hash_sha256.c:10`; fixed-shape or branch-free valid invocation across randomized values | x |
| 398 | `crypto_hash_sha256_update` | `crypto_hash/sha256/cp/hash_sha256_cp.c:350`; empty/one/many and boundary shapes; both outcomes of: if (inlen <= 0U); if (inlen < 64 - r) | x |
| 399 | `crypto_hash_sha3256` | `crypto_hash/sha3/hash_sha3.c:153`; fixed-shape or branch-free valid invocation across randomized values | x |
| 400 | `crypto_hash_sha3256_bytes` | `crypto_hash/sha3/hash_sha3.c:120`; fixed-shape or branch-free valid invocation across randomized values | x |
| 401 | `crypto_hash_sha3256_final` | `crypto_hash/sha3/hash_sha3.c:147`; fixed-shape or branch-free valid invocation across randomized values | x |
| 402 | `crypto_hash_sha3256_init` | `crypto_hash/sha3/hash_sha3.c:132`; fixed-shape or branch-free valid invocation across randomized values | x |
| 403 | `crypto_hash_sha3256_statebytes` | `crypto_hash/sha3/hash_sha3.c:126`; fixed-shape or branch-free valid invocation across randomized values | x |
| 404 | `crypto_hash_sha3256_update` | `crypto_hash/sha3/hash_sha3.c:140`; fixed-shape or branch-free valid invocation across randomized values | x |
| 405 | `crypto_hash_sha3512` | `crypto_hash/sha3/hash_sha3.c:199`; fixed-shape or branch-free valid invocation across randomized values | x |
| 406 | `crypto_hash_sha3512_bytes` | `crypto_hash/sha3/hash_sha3.c:166`; fixed-shape or branch-free valid invocation across randomized values | x |
| 407 | `crypto_hash_sha3512_final` | `crypto_hash/sha3/hash_sha3.c:193`; fixed-shape or branch-free valid invocation across randomized values | x |
| 408 | `crypto_hash_sha3512_init` | `crypto_hash/sha3/hash_sha3.c:178`; fixed-shape or branch-free valid invocation across randomized values | x |
| 409 | `crypto_hash_sha3512_statebytes` | `crypto_hash/sha3/hash_sha3.c:172`; fixed-shape or branch-free valid invocation across randomized values | x |
| 410 | `crypto_hash_sha3512_update` | `crypto_hash/sha3/hash_sha3.c:186`; fixed-shape or branch-free valid invocation across randomized values | x |
| 411 | `crypto_hash_sha512` | `crypto_hash/sha512/cp/hash_sha512_cp.c:274`; fixed-shape or branch-free valid invocation across randomized values | x |
| 412 | `crypto_hash_sha512_bytes` | `crypto_hash/sha512/hash_sha512.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 413 | `crypto_hash_sha512_final` | `crypto_hash/sha512/cp/hash_sha512_cp.c:261`; fixed-shape or branch-free valid invocation across randomized values | x |
| 414 | `crypto_hash_sha512_init` | `crypto_hash/sha512/cp/hash_sha512_cp.c:196`; fixed-shape or branch-free valid invocation across randomized values | x |
| 415 | `crypto_hash_sha512_statebytes` | `crypto_hash/sha512/hash_sha512.c:10`; fixed-shape or branch-free valid invocation across randomized values | x |
| 416 | `crypto_hash_sha512_update` | `crypto_hash/sha512/cp/hash_sha512_cp.c:211`; empty/one/many and boundary shapes; both outcomes of: if (inlen <= 0U); if ((state->count[1] += bitlen[1]) < bitlen[1]); if (inlen < 128 - r) | x |
| 417 | `crypto_ipcrypt_bytes` | `crypto_ipcrypt/crypto_ipcrypt.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 418 | `crypto_ipcrypt_decrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:128`; fixed-shape or branch-free valid invocation across randomized values | x |
| 419 | `crypto_ipcrypt_encrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:120`; fixed-shape or branch-free valid invocation across randomized values | x |
| 420 | `crypto_ipcrypt_keybytes` | `crypto_ipcrypt/crypto_ipcrypt.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 421 | `crypto_ipcrypt_keygen` | `crypto_ipcrypt/crypto_ipcrypt.c:96`; fixed-shape or branch-free valid invocation across randomized values | x |
| 422 | `crypto_ipcrypt_nd_decrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:145`; fixed-shape or branch-free valid invocation across randomized values | x |
| 423 | `crypto_ipcrypt_nd_encrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:136`; fixed-shape or branch-free valid invocation across randomized values | x |
| 424 | `crypto_ipcrypt_nd_inputbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:48`; fixed-shape or branch-free valid invocation across randomized values | x |
| 425 | `crypto_ipcrypt_nd_keybytes` | `crypto_ipcrypt/crypto_ipcrypt.c:36`; fixed-shape or branch-free valid invocation across randomized values | x |
| 426 | `crypto_ipcrypt_nd_keygen` | `crypto_ipcrypt/crypto_ipcrypt.c:102`; fixed-shape or branch-free valid invocation across randomized values | x |
| 427 | `crypto_ipcrypt_nd_outputbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 428 | `crypto_ipcrypt_nd_tweakbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:42`; fixed-shape or branch-free valid invocation across randomized values | x |
| 429 | `crypto_ipcrypt_ndx_decrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:162`; fixed-shape or branch-free valid invocation across randomized values | x |
| 430 | `crypto_ipcrypt_ndx_encrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:153`; fixed-shape or branch-free valid invocation across randomized values | x |
| 431 | `crypto_ipcrypt_ndx_inputbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:72`; fixed-shape or branch-free valid invocation across randomized values | x |
| 432 | `crypto_ipcrypt_ndx_keybytes` | `crypto_ipcrypt/crypto_ipcrypt.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 433 | `crypto_ipcrypt_ndx_keygen` | `crypto_ipcrypt/crypto_ipcrypt.c:108`; fixed-shape or branch-free valid invocation across randomized values | x |
| 434 | `crypto_ipcrypt_ndx_outputbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:78`; fixed-shape or branch-free valid invocation across randomized values | x |
| 435 | `crypto_ipcrypt_ndx_tweakbytes` | `crypto_ipcrypt/crypto_ipcrypt.c:66`; fixed-shape or branch-free valid invocation across randomized values | x |
| 436 | `crypto_ipcrypt_pfx_bytes` | `crypto_ipcrypt/crypto_ipcrypt.c:90`; fixed-shape or branch-free valid invocation across randomized values | x |
| 437 | `crypto_ipcrypt_pfx_decrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:178`; fixed-shape or branch-free valid invocation across randomized values | x |
| 438 | `crypto_ipcrypt_pfx_encrypt` | `crypto_ipcrypt/crypto_ipcrypt.c:170`; fixed-shape or branch-free valid invocation across randomized values | x |
| 439 | `crypto_ipcrypt_pfx_keybytes` | `crypto_ipcrypt/crypto_ipcrypt.c:84`; fixed-shape or branch-free valid invocation across randomized values | x |
| 440 | `crypto_ipcrypt_pfx_keygen` | `crypto_ipcrypt/crypto_ipcrypt.c:114`; fixed-shape or branch-free valid invocation across randomized values | x |
| 441 | `crypto_kdf_blake2b_bytes_max` | `crypto_kdf/blake2b/kdf_blake2b.c:14`; fixed-shape or branch-free valid invocation across randomized values | x |
| 442 | `crypto_kdf_blake2b_bytes_min` | `crypto_kdf/blake2b/kdf_blake2b.c:8`; fixed-shape or branch-free valid invocation across randomized values | x |
| 443 | `crypto_kdf_blake2b_contextbytes` | `crypto_kdf/blake2b/kdf_blake2b.c:20`; fixed-shape or branch-free valid invocation across randomized values | x |
| 444 | `crypto_kdf_blake2b_derive_from_key` | `crypto_kdf/blake2b/kdf_blake2b.c:31`; empty/one/many and boundary shapes; both outcomes of: if (subkey_len < crypto_kdf_blake2b_BYTES_MIN \|\| | x |
| 445 | `crypto_kdf_blake2b_keybytes` | `crypto_kdf/blake2b/kdf_blake2b.c:26`; fixed-shape or branch-free valid invocation across randomized values | x |
| 446 | `crypto_kdf_bytes_max` | `crypto_kdf/crypto_kdf.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 447 | `crypto_kdf_bytes_min` | `crypto_kdf/crypto_kdf.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 448 | `crypto_kdf_contextbytes` | `crypto_kdf/crypto_kdf.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 449 | `crypto_kdf_derive_from_key` | `crypto_kdf/crypto_kdf.c:36`; fixed-shape or branch-free valid invocation across randomized values | x |
| 450 | `crypto_kdf_hkdf_sha256_bytes_max` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:115`; fixed-shape or branch-free valid invocation across randomized values | x |
| 451 | `crypto_kdf_hkdf_sha256_bytes_min` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:109`; fixed-shape or branch-free valid invocation across randomized values | x |
| 452 | `crypto_kdf_hkdf_sha256_expand` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:55`; empty/one/many and boundary shapes; both outcomes of: if (out_len > crypto_kdf_hkdf_sha256_BYTES_MAX); if (i != (size_t) 0U); if ((left = out_len & (crypto_auth_hmacsha256_BYTES - 1U)) != (size_t) 0U); if (i != (size_t) 0U) | x |
| 453 | `crypto_kdf_hkdf_sha256_extract` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 454 | `crypto_kdf_hkdf_sha256_extract_final` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 455 | `crypto_kdf_hkdf_sha256_extract_init` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 456 | `crypto_kdf_hkdf_sha256_extract_update` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 457 | `crypto_kdf_hkdf_sha256_keybytes` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:103`; fixed-shape or branch-free valid invocation across randomized values | x |
| 458 | `crypto_kdf_hkdf_sha256_keygen` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 459 | `crypto_kdf_hkdf_sha256_statebytes` | `crypto_kdf/hkdf/kdf_hkdf_sha256.c:120`; fixed-shape or branch-free valid invocation across randomized values | x |
| 460 | `crypto_kdf_hkdf_sha512_bytes_max` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:115`; fixed-shape or branch-free valid invocation across randomized values | x |
| 461 | `crypto_kdf_hkdf_sha512_bytes_min` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:109`; fixed-shape or branch-free valid invocation across randomized values | x |
| 462 | `crypto_kdf_hkdf_sha512_expand` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:55`; empty/one/many and boundary shapes; both outcomes of: if (out_len > crypto_kdf_hkdf_sha512_BYTES_MAX); if (i != (size_t) 0U); if ((left = out_len & (crypto_auth_hmacsha512_BYTES - 1U)) != (size_t) 0U); if (i != (size_t) 0U) | x |
| 463 | `crypto_kdf_hkdf_sha512_extract` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 464 | `crypto_kdf_hkdf_sha512_extract_final` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 465 | `crypto_kdf_hkdf_sha512_extract_init` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 466 | `crypto_kdf_hkdf_sha512_extract_update` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 467 | `crypto_kdf_hkdf_sha512_keybytes` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:103`; fixed-shape or branch-free valid invocation across randomized values | x |
| 468 | `crypto_kdf_hkdf_sha512_keygen` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 469 | `crypto_kdf_hkdf_sha512_statebytes` | `crypto_kdf/hkdf/kdf_hkdf_sha512.c:120`; fixed-shape or branch-free valid invocation across randomized values | x |
| 470 | `crypto_kdf_keybytes` | `crypto_kdf/crypto_kdf.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 471 | `crypto_kdf_keygen` | `crypto_kdf/crypto_kdf.c:46`; fixed-shape or branch-free valid invocation across randomized values | x |
| 472 | `crypto_kdf_primitive` | `crypto_kdf/crypto_kdf.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 473 | `crypto_kem_ciphertextbytes` | `crypto_kem/crypto_kem.c:16`; fixed-shape or branch-free valid invocation across randomized values | x |
| 474 | `crypto_kem_dec` | `crypto_kem/crypto_kem.c:59`; fixed-shape or branch-free valid invocation across randomized values | x |
| 475 | `crypto_kem_enc` | `crypto_kem/crypto_kem.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 476 | `crypto_kem_keypair` | `crypto_kem/crypto_kem.c:47`; fixed-shape or branch-free valid invocation across randomized values | x |
| 477 | `crypto_kem_mlkem768_ciphertextbytes` | `crypto_kem/mlkem768/kem_mlkem768.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 478 | `crypto_kem_mlkem768_dec` | `crypto_kem/mlkem768/kem_mlkem768.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 479 | `crypto_kem_mlkem768_enc` | `crypto_kem/mlkem768/kem_mlkem768.c:47`; fixed-shape or branch-free valid invocation across randomized values | x |
| 480 | `crypto_kem_mlkem768_enc_deterministic` | `crypto_kem/mlkem768/kem_mlkem768.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 481 | `crypto_kem_mlkem768_keypair` | `crypto_kem/mlkem768/kem_mlkem768.c:41`; fixed-shape or branch-free valid invocation across randomized values | x |
| 482 | `crypto_kem_mlkem768_publickeybytes` | `crypto_kem/mlkem768/kem_mlkem768.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 483 | `crypto_kem_mlkem768_secretkeybytes` | `crypto_kem/mlkem768/kem_mlkem768.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 484 | `crypto_kem_mlkem768_seed_keypair` | `crypto_kem/mlkem768/kem_mlkem768.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 485 | `crypto_kem_mlkem768_seedbytes` | `crypto_kem/mlkem768/kem_mlkem768.c:29`; fixed-shape or branch-free valid invocation across randomized values | x |
| 486 | `crypto_kem_mlkem768_sharedsecretbytes` | `crypto_kem/mlkem768/kem_mlkem768.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 487 | `crypto_kem_primitive` | `crypto_kem/crypto_kem.c:34`; fixed-shape or branch-free valid invocation across randomized values | x |
| 488 | `crypto_kem_publickeybytes` | `crypto_kem/crypto_kem.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 489 | `crypto_kem_secretkeybytes` | `crypto_kem/crypto_kem.c:10`; fixed-shape or branch-free valid invocation across randomized values | x |
| 490 | `crypto_kem_seed_keypair` | `crypto_kem/crypto_kem.c:40`; fixed-shape or branch-free valid invocation across randomized values | x |
| 491 | `crypto_kem_seedbytes` | `crypto_kem/crypto_kem.c:28`; fixed-shape or branch-free valid invocation across randomized values | x |
| 492 | `crypto_kem_sharedsecretbytes` | `crypto_kem/crypto_kem.c:22`; fixed-shape or branch-free valid invocation across randomized values | x |
| 493 | `crypto_kem_xwing_ciphertextbytes` | `crypto_kem/xwing/kem_xwing.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 494 | `crypto_kem_xwing_dec` | `crypto_kem/xwing/kem_xwing.c:172`; empty/one/many and boundary shapes; both outcomes of: if (crypto_kem_mlkem768_dec(ss_mlkem, ct_mlkem, sk_mlkem) != 0); if (crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0) | x |
| 495 | `crypto_kem_xwing_enc` | `crypto_kem/xwing/kem_xwing.c:157`; empty/one/many and boundary shapes; both outcomes of: if (crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed) != 0) | x |
| 496 | `crypto_kem_xwing_enc_deterministic` | `crypto_kem/xwing/kem_xwing.c:120`; empty/one/many and boundary shapes; both outcomes of: if (crypto_kem_mlkem768_enc_deterministic(ct_mlkem, ss_mlkem, pk_mlkem, seed_mlkem) != 0); if (crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0) | x |
| 497 | `crypto_kem_xwing_keypair` | `crypto_kem/xwing/kem_xwing.c:107`; fixed-shape or branch-free valid invocation across randomized values | x |
| 498 | `crypto_kem_xwing_publickeybytes` | `crypto_kem/xwing/kem_xwing.c:56`; fixed-shape or branch-free valid invocation across randomized values | x |
| 499 | `crypto_kem_xwing_secretkeybytes` | `crypto_kem/xwing/kem_xwing.c:62`; fixed-shape or branch-free valid invocation across randomized values | x |
| 500 | `crypto_kem_xwing_seed_keypair` | `crypto_kem/xwing/kem_xwing.c:86`; fixed-shape or branch-free valid invocation across randomized values | x |
| 501 | `crypto_kem_xwing_seedbytes` | `crypto_kem/xwing/kem_xwing.c:80`; fixed-shape or branch-free valid invocation across randomized values | x |
| 502 | `crypto_kem_xwing_sharedsecretbytes` | `crypto_kem/xwing/kem_xwing.c:74`; fixed-shape or branch-free valid invocation across randomized values | x |
| 503 | `crypto_kx_client_session_keys` | `crypto_kx/crypto_kx.c:34`; empty/one/many and boundary shapes; both outcomes of: if (rx == NULL); if (tx == NULL); if (rx == NULL); if (crypto_scalarmult(q, client_sk, server_pk) != 0) | x |
| 504 | `crypto_kx_keypair` | `crypto_kx/crypto_kx.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 505 | `crypto_kx_primitive` | `crypto_kx/crypto_kx.c:140`; fixed-shape or branch-free valid invocation across randomized values | x |
| 506 | `crypto_kx_publickeybytes` | `crypto_kx/crypto_kx.c:116`; fixed-shape or branch-free valid invocation across randomized values | x |
| 507 | `crypto_kx_secretkeybytes` | `crypto_kx/crypto_kx.c:122`; fixed-shape or branch-free valid invocation across randomized values | x |
| 508 | `crypto_kx_seed_keypair` | `crypto_kx/crypto_kx.c:13`; fixed-shape or branch-free valid invocation across randomized values | x |
| 509 | `crypto_kx_seedbytes` | `crypto_kx/crypto_kx.c:128`; fixed-shape or branch-free valid invocation across randomized values | x |
| 510 | `crypto_kx_server_session_keys` | `crypto_kx/crypto_kx.c:75`; empty/one/many and boundary shapes; both outcomes of: if (rx == NULL); if (tx == NULL); if (rx == NULL); if (crypto_scalarmult(q, server_sk, client_pk) != 0) | x |
| 511 | `crypto_kx_sessionkeybytes` | `crypto_kx/crypto_kx.c:134`; fixed-shape or branch-free valid invocation across randomized values | x |
| 512 | `crypto_onetimeauth` | `crypto_onetimeauth/crypto_onetimeauth.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 513 | `crypto_onetimeauth_bytes` | `crypto_onetimeauth/crypto_onetimeauth.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 514 | `crypto_onetimeauth_final` | `crypto_onetimeauth/crypto_onetimeauth.c:55`; fixed-shape or branch-free valid invocation across randomized values | x |
| 515 | `crypto_onetimeauth_init` | `crypto_onetimeauth/crypto_onetimeauth.c:38`; fixed-shape or branch-free valid invocation across randomized values | x |
| 516 | `crypto_onetimeauth_keybytes` | `crypto_onetimeauth/crypto_onetimeauth.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 517 | `crypto_onetimeauth_keygen` | `crypto_onetimeauth/crypto_onetimeauth.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 518 | `crypto_onetimeauth_poly1305` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 519 | `crypto_onetimeauth_poly1305_bytes` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:56`; fixed-shape or branch-free valid invocation across randomized values | x |
| 520 | `crypto_onetimeauth_poly1305_donna_implementation` | exported data object; compare all bytes after loader initialization | x |
| 521 | `crypto_onetimeauth_poly1305_final` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 522 | `crypto_onetimeauth_poly1305_init` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:34`; fixed-shape or branch-free valid invocation across randomized values | x |
| 523 | `crypto_onetimeauth_poly1305_keybytes` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:62`; fixed-shape or branch-free valid invocation across randomized values | x |
| 524 | `crypto_onetimeauth_poly1305_keygen` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:74`; fixed-shape or branch-free valid invocation across randomized values | x |
| 525 | `crypto_onetimeauth_poly1305_statebytes` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 526 | `crypto_onetimeauth_poly1305_update` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:41`; fixed-shape or branch-free valid invocation across randomized values | x |
| 527 | `crypto_onetimeauth_poly1305_verify` | `crypto_onetimeauth/poly1305/onetimeauth_poly1305.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 528 | `crypto_onetimeauth_primitive` | `crypto_onetimeauth/crypto_onetimeauth.c:63`; fixed-shape or branch-free valid invocation across randomized values | x |
| 529 | `crypto_onetimeauth_statebytes` | `crypto_onetimeauth/crypto_onetimeauth.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 530 | `crypto_onetimeauth_update` | `crypto_onetimeauth/crypto_onetimeauth.c:46`; fixed-shape or branch-free valid invocation across randomized values | x |
| 531 | `crypto_onetimeauth_verify` | `crypto_onetimeauth/crypto_onetimeauth.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 532 | `crypto_pwhash` | `crypto_pwhash/crypto_pwhash.c:129`; empty/one/many and boundary shapes; both outcomes of: switch (alg); case crypto_pwhash_ALG_ARGON2I13:; case crypto_pwhash_ALG_ARGON2ID13: | x |
| 533 | `crypto_pwhash_alg_argon2i13` | `crypto_pwhash/crypto_pwhash.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 534 | `crypto_pwhash_alg_argon2id13` | `crypto_pwhash/crypto_pwhash.c:15`; fixed-shape or branch-free valid invocation across randomized values | x |
| 535 | `crypto_pwhash_alg_default` | `crypto_pwhash/crypto_pwhash.c:21`; fixed-shape or branch-free valid invocation across randomized values | x |
| 536 | `crypto_pwhash_argon2i` | `crypto_pwhash/argon2/pwhash_argon2i.c:140`; empty/one/many and boundary shapes; both outcomes of: if (outlen > crypto_pwhash_argon2i_BYTES_MAX); if (outlen < crypto_pwhash_argon2i_BYTES_MIN); if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\|; if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\|; … (8 direct branch axes total) | x |
| 537 | `crypto_pwhash_argon2i_alg_argon2i13` | `crypto_pwhash/argon2/pwhash_argon2i.c:22`; fixed-shape or branch-free valid invocation across randomized values | x |
| 538 | `crypto_pwhash_argon2i_bytes_max` | `crypto_pwhash/argon2/pwhash_argon2i.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 539 | `crypto_pwhash_argon2i_bytes_min` | `crypto_pwhash/argon2/pwhash_argon2i.c:28`; fixed-shape or branch-free valid invocation across randomized values | x |
| 540 | `crypto_pwhash_argon2i_memlimit_interactive` | `crypto_pwhash/argon2/pwhash_argon2i.c:110`; fixed-shape or branch-free valid invocation across randomized values | x |
| 541 | `crypto_pwhash_argon2i_memlimit_max` | `crypto_pwhash/argon2/pwhash_argon2i.c:97`; fixed-shape or branch-free valid invocation across randomized values | x |
| 542 | `crypto_pwhash_argon2i_memlimit_min` | `crypto_pwhash/argon2/pwhash_argon2i.c:90`; fixed-shape or branch-free valid invocation across randomized values | x |
| 543 | `crypto_pwhash_argon2i_memlimit_moderate` | `crypto_pwhash/argon2/pwhash_argon2i.c:122`; fixed-shape or branch-free valid invocation across randomized values | x |
| 544 | `crypto_pwhash_argon2i_memlimit_sensitive` | `crypto_pwhash/argon2/pwhash_argon2i.c:134`; fixed-shape or branch-free valid invocation across randomized values | x |
| 545 | `crypto_pwhash_argon2i_opslimit_interactive` | `crypto_pwhash/argon2/pwhash_argon2i.c:104`; fixed-shape or branch-free valid invocation across randomized values | x |
| 546 | `crypto_pwhash_argon2i_opslimit_max` | `crypto_pwhash/argon2/pwhash_argon2i.c:83`; fixed-shape or branch-free valid invocation across randomized values | x |
| 547 | `crypto_pwhash_argon2i_opslimit_min` | `crypto_pwhash/argon2/pwhash_argon2i.c:76`; fixed-shape or branch-free valid invocation across randomized values | x |
| 548 | `crypto_pwhash_argon2i_opslimit_moderate` | `crypto_pwhash/argon2/pwhash_argon2i.c:116`; fixed-shape or branch-free valid invocation across randomized values | x |
| 549 | `crypto_pwhash_argon2i_opslimit_sensitive` | `crypto_pwhash/argon2/pwhash_argon2i.c:128`; fixed-shape or branch-free valid invocation across randomized values | x |
| 550 | `crypto_pwhash_argon2i_passwd_max` | `crypto_pwhash/argon2/pwhash_argon2i.c:49`; fixed-shape or branch-free valid invocation across randomized values | x |
| 551 | `crypto_pwhash_argon2i_passwd_min` | `crypto_pwhash/argon2/pwhash_argon2i.c:42`; fixed-shape or branch-free valid invocation across randomized values | x |
| 552 | `crypto_pwhash_argon2i_saltbytes` | `crypto_pwhash/argon2/pwhash_argon2i.c:56`; fixed-shape or branch-free valid invocation across randomized values | x |
| 553 | `crypto_pwhash_argon2i_str` | `crypto_pwhash/argon2/pwhash_argon2i.c:186`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX \|\|; if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN \|\|; if (argon2i_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), | x |
| 554 | `crypto_pwhash_argon2i_str_needs_rehash` | `crypto_pwhash/argon2/pwhash_argon2i.c:283`; fixed-shape or branch-free valid invocation across randomized values | x |
| 555 | `crypto_pwhash_argon2i_str_verify` | `crypto_pwhash/argon2/pwhash_argon2i.c:217`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_argon2i_PASSWD_MAX); if (passwdlen < crypto_pwhash_argon2i_PASSWD_MIN); if (verify_ret == ARGON2_OK); if (verify_ret == ARGON2_VERIFY_MISMATCH) | x |
| 556 | `crypto_pwhash_argon2i_strbytes` | `crypto_pwhash/argon2/pwhash_argon2i.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 557 | `crypto_pwhash_argon2i_strprefix` | `crypto_pwhash/argon2/pwhash_argon2i.c:70`; fixed-shape or branch-free valid invocation across randomized values | x |
| 558 | `crypto_pwhash_argon2id` | `crypto_pwhash/argon2/pwhash_argon2id.c:136`; empty/one/many and boundary shapes; both outcomes of: if (outlen > crypto_pwhash_argon2id_BYTES_MAX); if (outlen < crypto_pwhash_argon2id_BYTES_MIN); if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\|; if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\|; … (8 direct branch axes total) | x |
| 559 | `crypto_pwhash_argon2id_alg_argon2id13` | `crypto_pwhash/argon2/pwhash_argon2id.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 560 | `crypto_pwhash_argon2id_bytes_max` | `crypto_pwhash/argon2/pwhash_argon2id.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 561 | `crypto_pwhash_argon2id_bytes_min` | `crypto_pwhash/argon2/pwhash_argon2id.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 562 | `crypto_pwhash_argon2id_memlimit_interactive` | `crypto_pwhash/argon2/pwhash_argon2id.c:106`; fixed-shape or branch-free valid invocation across randomized values | x |
| 563 | `crypto_pwhash_argon2id_memlimit_max` | `crypto_pwhash/argon2/pwhash_argon2id.c:93`; fixed-shape or branch-free valid invocation across randomized values | x |
| 564 | `crypto_pwhash_argon2id_memlimit_min` | `crypto_pwhash/argon2/pwhash_argon2id.c:86`; fixed-shape or branch-free valid invocation across randomized values | x |
| 565 | `crypto_pwhash_argon2id_memlimit_moderate` | `crypto_pwhash/argon2/pwhash_argon2id.c:118`; fixed-shape or branch-free valid invocation across randomized values | x |
| 566 | `crypto_pwhash_argon2id_memlimit_sensitive` | `crypto_pwhash/argon2/pwhash_argon2id.c:130`; fixed-shape or branch-free valid invocation across randomized values | x |
| 567 | `crypto_pwhash_argon2id_opslimit_interactive` | `crypto_pwhash/argon2/pwhash_argon2id.c:100`; fixed-shape or branch-free valid invocation across randomized values | x |
| 568 | `crypto_pwhash_argon2id_opslimit_max` | `crypto_pwhash/argon2/pwhash_argon2id.c:79`; fixed-shape or branch-free valid invocation across randomized values | x |
| 569 | `crypto_pwhash_argon2id_opslimit_min` | `crypto_pwhash/argon2/pwhash_argon2id.c:72`; fixed-shape or branch-free valid invocation across randomized values | x |
| 570 | `crypto_pwhash_argon2id_opslimit_moderate` | `crypto_pwhash/argon2/pwhash_argon2id.c:112`; fixed-shape or branch-free valid invocation across randomized values | x |
| 571 | `crypto_pwhash_argon2id_opslimit_sensitive` | `crypto_pwhash/argon2/pwhash_argon2id.c:124`; fixed-shape or branch-free valid invocation across randomized values | x |
| 572 | `crypto_pwhash_argon2id_passwd_max` | `crypto_pwhash/argon2/pwhash_argon2id.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 573 | `crypto_pwhash_argon2id_passwd_min` | `crypto_pwhash/argon2/pwhash_argon2id.c:38`; fixed-shape or branch-free valid invocation across randomized values | x |
| 574 | `crypto_pwhash_argon2id_saltbytes` | `crypto_pwhash/argon2/pwhash_argon2id.c:52`; fixed-shape or branch-free valid invocation across randomized values | x |
| 575 | `crypto_pwhash_argon2id_str` | `crypto_pwhash/argon2/pwhash_argon2id.c:182`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX \|\|; if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN \|\|; if (argon2id_hash_encoded((uint32_t) opslimit, (uint32_t) (memlimit / 1024U), | x |
| 576 | `crypto_pwhash_argon2id_str_needs_rehash` | `crypto_pwhash/argon2/pwhash_argon2i.c:290`; fixed-shape or branch-free valid invocation across randomized values | x |
| 577 | `crypto_pwhash_argon2id_str_verify` | `crypto_pwhash/argon2/pwhash_argon2id.c:213`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_argon2id_PASSWD_MAX); if (passwdlen < crypto_pwhash_argon2id_PASSWD_MIN); if (verify_ret == ARGON2_OK); if (verify_ret == ARGON2_VERIFY_MISMATCH) | x |
| 578 | `crypto_pwhash_argon2id_strbytes` | `crypto_pwhash/argon2/pwhash_argon2id.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 579 | `crypto_pwhash_argon2id_strprefix` | `crypto_pwhash/argon2/pwhash_argon2id.c:66`; fixed-shape or branch-free valid invocation across randomized values | x |
| 580 | `crypto_pwhash_bytes_max` | `crypto_pwhash/crypto_pwhash.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 581 | `crypto_pwhash_bytes_min` | `crypto_pwhash/crypto_pwhash.c:27`; fixed-shape or branch-free valid invocation across randomized values | x |
| 582 | `crypto_pwhash_memlimit_interactive` | `crypto_pwhash/crypto_pwhash.c:99`; fixed-shape or branch-free valid invocation across randomized values | x |
| 583 | `crypto_pwhash_memlimit_max` | `crypto_pwhash/crypto_pwhash.c:87`; fixed-shape or branch-free valid invocation across randomized values | x |
| 584 | `crypto_pwhash_memlimit_min` | `crypto_pwhash/crypto_pwhash.c:81`; fixed-shape or branch-free valid invocation across randomized values | x |
| 585 | `crypto_pwhash_memlimit_moderate` | `crypto_pwhash/crypto_pwhash.c:111`; fixed-shape or branch-free valid invocation across randomized values | x |
| 586 | `crypto_pwhash_memlimit_sensitive` | `crypto_pwhash/crypto_pwhash.c:123`; fixed-shape or branch-free valid invocation across randomized values | x |
| 587 | `crypto_pwhash_opslimit_interactive` | `crypto_pwhash/crypto_pwhash.c:93`; fixed-shape or branch-free valid invocation across randomized values | x |
| 588 | `crypto_pwhash_opslimit_max` | `crypto_pwhash/crypto_pwhash.c:75`; fixed-shape or branch-free valid invocation across randomized values | x |
| 589 | `crypto_pwhash_opslimit_min` | `crypto_pwhash/crypto_pwhash.c:69`; fixed-shape or branch-free valid invocation across randomized values | x |
| 590 | `crypto_pwhash_opslimit_moderate` | `crypto_pwhash/crypto_pwhash.c:105`; fixed-shape or branch-free valid invocation across randomized values | x |
| 591 | `crypto_pwhash_opslimit_sensitive` | `crypto_pwhash/crypto_pwhash.c:117`; fixed-shape or branch-free valid invocation across randomized values | x |
| 592 | `crypto_pwhash_passwd_max` | `crypto_pwhash/crypto_pwhash.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 593 | `crypto_pwhash_passwd_min` | `crypto_pwhash/crypto_pwhash.c:39`; fixed-shape or branch-free valid invocation across randomized values | x |
| 594 | `crypto_pwhash_primitive` | `crypto_pwhash/crypto_pwhash.c:210`; fixed-shape or branch-free valid invocation across randomized values | x |
| 595 | `crypto_pwhash_saltbytes` | `crypto_pwhash/crypto_pwhash.c:51`; fixed-shape or branch-free valid invocation across randomized values | x |
| 596 | `crypto_pwhash_scryptsalsa208sha256` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:158`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX \|\|; if (outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN \|\|; if ((const void *) out == (const void *) passwd) | x |
| 597 | `crypto_pwhash_scryptsalsa208sha256_bytes_max` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:74`; fixed-shape or branch-free valid invocation across randomized values | x |
| 598 | `crypto_pwhash_scryptsalsa208sha256_bytes_min` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 599 | `crypto_pwhash_scryptsalsa208sha256_ll` | `crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c:244`; empty/one/many and boundary shapes; both outcomes of: if (escrypt_init_local(&local)); if (escrypt_free_local(&local)) | x |
| 600 | `crypto_pwhash_scryptsalsa208sha256_memlimit_interactive` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:140`; fixed-shape or branch-free valid invocation across randomized values | x |
| 601 | `crypto_pwhash_scryptsalsa208sha256_memlimit_max` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:128`; fixed-shape or branch-free valid invocation across randomized values | x |
| 602 | `crypto_pwhash_scryptsalsa208sha256_memlimit_min` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:122`; fixed-shape or branch-free valid invocation across randomized values | x |
| 603 | `crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:152`; fixed-shape or branch-free valid invocation across randomized values | x |
| 604 | `crypto_pwhash_scryptsalsa208sha256_opslimit_interactive` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:134`; fixed-shape or branch-free valid invocation across randomized values | x |
| 605 | `crypto_pwhash_scryptsalsa208sha256_opslimit_max` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:116`; fixed-shape or branch-free valid invocation across randomized values | x |
| 606 | `crypto_pwhash_scryptsalsa208sha256_opslimit_min` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:110`; fixed-shape or branch-free valid invocation across randomized values | x |
| 607 | `crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:146`; fixed-shape or branch-free valid invocation across randomized values | x |
| 608 | `crypto_pwhash_scryptsalsa208sha256_passwd_max` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:86`; fixed-shape or branch-free valid invocation across randomized values | x |
| 609 | `crypto_pwhash_scryptsalsa208sha256_passwd_min` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:80`; fixed-shape or branch-free valid invocation across randomized values | x |
| 610 | `crypto_pwhash_scryptsalsa208sha256_saltbytes` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:92`; fixed-shape or branch-free valid invocation across randomized values | x |
| 611 | `crypto_pwhash_scryptsalsa208sha256_str` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:191`; empty/one/many and boundary shapes; both outcomes of: if (passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX); if (passwdlen < crypto_pwhash_scryptsalsa208sha256_PASSWD_MIN \|\|; if (escrypt_gensalt_r(N_log2, r, p, salt, sizeof salt, (uint8_t *) setting,; if (escrypt_init_local(&escrypt_local) != 0); … (5 direct branch axes total) | x |
| 612 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:275`; empty/one/many and boundary shapes; both outcomes of: if (pickparams(opslimit, memlimit, &N_log2, &p, &r) != 0); if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) !=; if (escrypt_parse_setting((const uint8_t *) str,; if (N_log2 != N_log2_ \|\| r != r_ \|\| p != p_) | x |
| 613 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:245`; empty/one/many and boundary shapes; both outcomes of: if (sodium_strnlen(str, crypto_pwhash_scryptsalsa208sha256_STRBYTES) !=; if (escrypt_init_local(&escrypt_local) != 0); if (escrypt_r(&escrypt_local, (const uint8_t *) passwd, (size_t) passwdlen, | x |
| 614 | `crypto_pwhash_scryptsalsa208sha256_strbytes` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:98`; fixed-shape or branch-free valid invocation across randomized values | x |
| 615 | `crypto_pwhash_scryptsalsa208sha256_strprefix` | `crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c:104`; fixed-shape or branch-free valid invocation across randomized values | x |
| 616 | `crypto_pwhash_str` | `crypto_pwhash/crypto_pwhash.c:148`; fixed-shape or branch-free valid invocation across randomized values | x |
| 617 | `crypto_pwhash_str_alg` | `crypto_pwhash/crypto_pwhash.c:157`; empty/one/many and boundary shapes; both outcomes of: switch (alg); case crypto_pwhash_ALG_ARGON2I13:; case crypto_pwhash_ALG_ARGON2ID13: | x |
| 618 | `crypto_pwhash_str_needs_rehash` | `crypto_pwhash/crypto_pwhash.c:193`; empty/one/many and boundary shapes; both outcomes of: if (strncmp(str, crypto_pwhash_argon2id_STRPREFIX,; if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, | x |
| 619 | `crypto_pwhash_str_verify` | `crypto_pwhash/crypto_pwhash.c:175`; empty/one/many and boundary shapes; both outcomes of: if (strncmp(str, crypto_pwhash_argon2id_STRPREFIX,; if (strncmp(str, crypto_pwhash_argon2i_STRPREFIX, | x |
| 620 | `crypto_pwhash_strbytes` | `crypto_pwhash/crypto_pwhash.c:57`; fixed-shape or branch-free valid invocation across randomized values | x |
| 621 | `crypto_pwhash_strprefix` | `crypto_pwhash/crypto_pwhash.c:63`; fixed-shape or branch-free valid invocation across randomized values | x |
| 622 | `crypto_scalarmult` | `crypto_scalarmult/crypto_scalarmult.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 623 | `crypto_scalarmult_base` | `crypto_scalarmult/crypto_scalarmult.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 624 | `crypto_scalarmult_bytes` | `crypto_scalarmult/crypto_scalarmult.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 625 | `crypto_scalarmult_curve25519` | `crypto_scalarmult/curve25519/scalarmult_curve25519.c:15`; empty/one/many and boundary shapes; both outcomes of: if (implementation->mult(q, n, p) != 0) | x |
| 626 | `crypto_scalarmult_curve25519_base` | `crypto_scalarmult/curve25519/scalarmult_curve25519.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 627 | `crypto_scalarmult_curve25519_bytes` | `crypto_scalarmult/curve25519/scalarmult_curve25519.c:38`; fixed-shape or branch-free valid invocation across randomized values | x |
| 628 | `crypto_scalarmult_curve25519_ref10_implementation` | exported data object; compare all bytes after loader initialization | x |
| 629 | `crypto_scalarmult_curve25519_scalarbytes` | `crypto_scalarmult/curve25519/scalarmult_curve25519.c:44`; fixed-shape or branch-free valid invocation across randomized values | x |
| 630 | `crypto_scalarmult_ed25519` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 631 | `crypto_scalarmult_ed25519_base` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:98`; fixed-shape or branch-free valid invocation across randomized values | x |
| 632 | `crypto_scalarmult_ed25519_base_noclamp` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:105`; fixed-shape or branch-free valid invocation across randomized values | x |
| 633 | `crypto_scalarmult_ed25519_bytes` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:112`; fixed-shape or branch-free valid invocation across randomized values | x |
| 634 | `crypto_scalarmult_ed25519_noclamp` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:67`; fixed-shape or branch-free valid invocation across randomized values | x |
| 635 | `crypto_scalarmult_ed25519_scalarbytes` | `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c:118`; fixed-shape or branch-free valid invocation across randomized values | x |
| 636 | `crypto_scalarmult_primitive` | `crypto_scalarmult/crypto_scalarmult.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 637 | `crypto_scalarmult_ristretto255` | `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:10`; empty/one/many and boundary shapes; both outcomes of: if (ristretto255_frombytes(&P, p) != 0); if (sodium_is_zero(q, 32)) | x |
| 638 | `crypto_scalarmult_ristretto255_base` | `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:34`; empty/one/many and boundary shapes; both outcomes of: if (sodium_is_zero(q, 32)) | x |
| 639 | `crypto_scalarmult_ristretto255_bytes` | `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 640 | `crypto_scalarmult_ristretto255_scalarbytes` | `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 641 | `crypto_scalarmult_scalarbytes` | `crypto_scalarmult/crypto_scalarmult.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 642 | `crypto_secretbox` | `crypto_secretbox/crypto_secretbox.c:48`; fixed-shape or branch-free valid invocation across randomized values | x |
| 643 | `crypto_secretbox_boxzerobytes` | `crypto_secretbox/crypto_secretbox.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 644 | `crypto_secretbox_detached` | `crypto_secretbox/crypto_secretbox_easy.c:19`; empty/one/many and boundary shapes; both outcomes of: if (((uintptr_t) c > (uintptr_t) m &&; if (mlen0 > 64U - crypto_secretbox_ZEROBYTES); if (cl > STREAM_POLY1305_CHUNK) | x |
| 645 | `crypto_secretbox_easy` | `crypto_secretbox/crypto_secretbox_easy.c:93`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_secretbox_MESSAGEBYTES_MAX) | x |
| 646 | `crypto_secretbox_keybytes` | `crypto_secretbox/crypto_secretbox.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 647 | `crypto_secretbox_keygen` | `crypto_secretbox/crypto_secretbox.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 648 | `crypto_secretbox_macbytes` | `crypto_secretbox/crypto_secretbox.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 649 | `crypto_secretbox_messagebytes_max` | `crypto_secretbox/crypto_secretbox.c:36`; fixed-shape or branch-free valid invocation across randomized values | x |
| 650 | `crypto_secretbox_noncebytes` | `crypto_secretbox/crypto_secretbox.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 651 | `crypto_secretbox_open` | `crypto_secretbox/crypto_secretbox.c:56`; fixed-shape or branch-free valid invocation across randomized values | x |
| 652 | `crypto_secretbox_open_detached` | `crypto_secretbox/crypto_secretbox_easy.c:105`; empty/one/many and boundary shapes; both outcomes of: if (mlen0 > 64U - crypto_secretbox_ZEROBYTES); if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0); if (m == NULL); if (((uintptr_t) c > (uintptr_t) m &&; … (5 direct branch axes total) | x |
| 653 | `crypto_secretbox_open_easy` | `crypto_secretbox/crypto_secretbox_easy.c:166`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_secretbox_MACBYTES) | x |
| 654 | `crypto_secretbox_primitive` | `crypto_secretbox/crypto_secretbox.c:42`; fixed-shape or branch-free valid invocation across randomized values | x |
| 655 | `crypto_secretbox_xchacha20poly1305_detached` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:19`; empty/one/many and boundary shapes; both outcomes of: if (((uintptr_t) c > (uintptr_t) m &&; if (mlen0 > 64U - crypto_secretbox_xchacha20poly1305_ZEROBYTES); if (mlen > mlen0) | x |
| 656 | `crypto_secretbox_xchacha20poly1305_easy` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:83`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX) | x |
| 657 | `crypto_secretbox_xchacha20poly1305_keybytes` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:173`; fixed-shape or branch-free valid invocation across randomized values | x |
| 658 | `crypto_secretbox_xchacha20poly1305_macbytes` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:185`; fixed-shape or branch-free valid invocation across randomized values | x |
| 659 | `crypto_secretbox_xchacha20poly1305_messagebytes_max` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:191`; fixed-shape or branch-free valid invocation across randomized values | x |
| 660 | `crypto_secretbox_xchacha20poly1305_noncebytes` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:179`; fixed-shape or branch-free valid invocation across randomized values | x |
| 661 | `crypto_secretbox_xchacha20poly1305_open_detached` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:97`; empty/one/many and boundary shapes; both outcomes of: if (mlen0 > 64U - crypto_secretbox_xchacha20poly1305_ZEROBYTES); if (crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0); if (m == NULL); if (((uintptr_t) c > (uintptr_t) m &&; … (5 direct branch axes total) | x |
| 662 | `crypto_secretbox_xchacha20poly1305_open_easy` | `crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c:158`; empty/one/many and boundary shapes; both outcomes of: if (clen < crypto_secretbox_xchacha20poly1305_MACBYTES) | x |
| 663 | `crypto_secretbox_xsalsa20poly1305` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:8`; empty/one/many and boundary shapes; both outcomes of: if (mlen < 32) | x |
| 664 | `crypto_secretbox_xsalsa20poly1305_boxzerobytes` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:70`; fixed-shape or branch-free valid invocation across randomized values | x |
| 665 | `crypto_secretbox_xsalsa20poly1305_keybytes` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:52`; fixed-shape or branch-free valid invocation across randomized values | x |
| 666 | `crypto_secretbox_xsalsa20poly1305_keygen` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:88`; fixed-shape or branch-free valid invocation across randomized values | x |
| 667 | `crypto_secretbox_xsalsa20poly1305_macbytes` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:76`; fixed-shape or branch-free valid invocation across randomized values | x |
| 668 | `crypto_secretbox_xsalsa20poly1305_messagebytes_max` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:82`; fixed-shape or branch-free valid invocation across randomized values | x |
| 669 | `crypto_secretbox_xsalsa20poly1305_noncebytes` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:58`; fixed-shape or branch-free valid invocation across randomized values | x |
| 670 | `crypto_secretbox_xsalsa20poly1305_open` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:27`; empty/one/many and boundary shapes; both outcomes of: if (clen < 32); if (crypto_onetimeauth_poly1305_verify(c + 16, c + 32, | x |
| 671 | `crypto_secretbox_xsalsa20poly1305_zerobytes` | `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 672 | `crypto_secretbox_zerobytes` | `crypto_secretbox/crypto_secretbox.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 673 | `crypto_secretstream_xchacha20poly1305_abytes` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:271`; fixed-shape or branch-free valid invocation across randomized values | x |
| 674 | `crypto_secretstream_xchacha20poly1305_headerbytes` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:277`; fixed-shape or branch-free valid invocation across randomized values | x |
| 675 | `crypto_secretstream_xchacha20poly1305_init_pull` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 676 | `crypto_secretstream_xchacha20poly1305_init_push` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 677 | `crypto_secretstream_xchacha20poly1305_keybytes` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:283`; fixed-shape or branch-free valid invocation across randomized values | x |
| 678 | `crypto_secretstream_xchacha20poly1305_keygen` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:36`; fixed-shape or branch-free valid invocation across randomized values | x |
| 679 | `crypto_secretstream_xchacha20poly1305_messagebytes_max` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:289`; fixed-shape or branch-free valid invocation across randomized values | x |
| 680 | `crypto_secretstream_xchacha20poly1305_pull` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:180`; empty/one/many and boundary shapes; both outcomes of: if (mlen_p != NULL); if (tag_p != NULL); if (inlen < crypto_secretstream_xchacha20poly1305_ABYTES); if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX); … (8 direct branch axes total) | x |
| 681 | `crypto_secretstream_xchacha20poly1305_push` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:111`; empty/one/many and boundary shapes; both outcomes of: if (outlen_p != NULL); if (mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX); if ((tag & crypto_secretstream_xchacha20poly1305_TAG_REKEY) != 0 \|\|; if (outlen_p != NULL) | x |
| 682 | `crypto_secretstream_xchacha20poly1305_rekey` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:83`; fixed-shape or branch-free valid invocation across randomized values | x |
| 683 | `crypto_secretstream_xchacha20poly1305_statebytes` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:265`; fixed-shape or branch-free valid invocation across randomized values | x |
| 684 | `crypto_secretstream_xchacha20poly1305_tag_final` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:313`; fixed-shape or branch-free valid invocation across randomized values | x |
| 685 | `crypto_secretstream_xchacha20poly1305_tag_message` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:295`; fixed-shape or branch-free valid invocation across randomized values | x |
| 686 | `crypto_secretstream_xchacha20poly1305_tag_push` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:301`; fixed-shape or branch-free valid invocation across randomized values | x |
| 687 | `crypto_secretstream_xchacha20poly1305_tag_rekey` | `crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c:307`; fixed-shape or branch-free valid invocation across randomized values | x |
| 688 | `crypto_shorthash` | `crypto_shorthash/crypto_shorthash.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 689 | `crypto_shorthash_bytes` | `crypto_shorthash/crypto_shorthash.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 690 | `crypto_shorthash_keybytes` | `crypto_shorthash/crypto_shorthash.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 691 | `crypto_shorthash_keygen` | `crypto_shorthash/crypto_shorthash.c:31`; fixed-shape or branch-free valid invocation across randomized values | x |
| 692 | `crypto_shorthash_primitive` | `crypto_shorthash/crypto_shorthash.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 693 | `crypto_shorthash_siphash24` | `crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c:6`; empty/one/many and boundary shapes; both outcomes of: switch (left); case 7:; case 6:; case 5:; … (9 direct branch axes total) | x |
| 694 | `crypto_shorthash_siphash24_bytes` | `crypto_shorthash/siphash24/shorthash_siphash24.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 695 | `crypto_shorthash_siphash24_keybytes` | `crypto_shorthash/siphash24/shorthash_siphash24.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 696 | `crypto_shorthash_siphashx24` | `crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c:6`; empty/one/many and boundary shapes; both outcomes of: switch (left); case 7:; case 6:; case 5:; … (9 direct branch axes total) | x |
| 697 | `crypto_shorthash_siphashx24_bytes` | `crypto_shorthash/siphash24/shorthash_siphashx24.c:4`; fixed-shape or branch-free valid invocation across randomized values | x |
| 698 | `crypto_shorthash_siphashx24_keybytes` | `crypto_shorthash/siphash24/shorthash_siphashx24.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 699 | `crypto_sign` | `crypto_sign/crypto_sign.c:60`; fixed-shape or branch-free valid invocation across randomized values | x |
| 700 | `crypto_sign_bytes` | `crypto_sign/crypto_sign.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 701 | `crypto_sign_detached` | `crypto_sign/crypto_sign.c:76`; fixed-shape or branch-free valid invocation across randomized values | x |
| 702 | `crypto_sign_ed25519` | `crypto_sign/ed25519/ref10/sign.c:105`; empty/one/many and boundary shapes; both outcomes of: if (crypto_sign_ed25519_detached(; if (smlen_p != NULL); if (smlen_p != NULL) | x |
| 703 | `crypto_sign_ed25519_bytes` | `crypto_sign/ed25519/sign_ed25519.c:15`; fixed-shape or branch-free valid invocation across randomized values | x |
| 704 | `crypto_sign_ed25519_detached` | `crypto_sign/ed25519/ref10/sign.c:97`; fixed-shape or branch-free valid invocation across randomized values | x |
| 705 | `crypto_sign_ed25519_keypair` | `crypto_sign/ed25519/ref10/keypair.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 706 | `crypto_sign_ed25519_messagebytes_max` | `crypto_sign/ed25519/sign_ed25519.c:39`; fixed-shape or branch-free valid invocation across randomized values | x |
| 707 | `crypto_sign_ed25519_open` | `crypto_sign/ed25519/ref10/open.c:75`; empty/one/many and boundary shapes; both outcomes of: if (smlen < 64 \|\| smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX); if (crypto_sign_ed25519_verify_detached(sm, sm + 64, mlen, pk) != 0); if (m != NULL); if (mlen_p != NULL); … (6 direct branch axes total) | x |
| 708 | `crypto_sign_ed25519_pk_to_curve25519` | `crypto_sign/ed25519/ref10/keypair.c:46`; empty/one/many and boundary shapes; both outcomes of: if (ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0 \|\| | x |
| 709 | `crypto_sign_ed25519_publickeybytes` | `crypto_sign/ed25519/sign_ed25519.c:27`; fixed-shape or branch-free valid invocation across randomized values | x |
| 710 | `crypto_sign_ed25519_secretkeybytes` | `crypto_sign/ed25519/sign_ed25519.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 711 | `crypto_sign_ed25519_seed_keypair` | `crypto_sign/ed25519/ref10/keypair.c:13`; fixed-shape or branch-free valid invocation across randomized values | x |
| 712 | `crypto_sign_ed25519_seedbytes` | `crypto_sign/ed25519/sign_ed25519.c:21`; fixed-shape or branch-free valid invocation across randomized values | x |
| 713 | `crypto_sign_ed25519_sk_to_curve25519` | `crypto_sign/ed25519/ref10/keypair.c:71`; fixed-shape or branch-free valid invocation across randomized values | x |
| 714 | `crypto_sign_ed25519_sk_to_pk` | `crypto_sign/ed25519/sign_ed25519.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 715 | `crypto_sign_ed25519_sk_to_seed` | `crypto_sign/ed25519/sign_ed25519.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 716 | `crypto_sign_ed25519_verify_detached` | `crypto_sign/ed25519/ref10/open.c:66`; fixed-shape or branch-free valid invocation across randomized values | x |
| 717 | `crypto_sign_ed25519ph_final_create` | `crypto_sign/ed25519/sign_ed25519.c:75`; fixed-shape or branch-free valid invocation across randomized values | x |
| 718 | `crypto_sign_ed25519ph_final_verify` | `crypto_sign/ed25519/sign_ed25519.c:88`; fixed-shape or branch-free valid invocation across randomized values | x |
| 719 | `crypto_sign_ed25519ph_init` | `crypto_sign/ed25519/sign_ed25519.c:61`; fixed-shape or branch-free valid invocation across randomized values | x |
| 720 | `crypto_sign_ed25519ph_statebytes` | `crypto_sign/ed25519/sign_ed25519.c:9`; fixed-shape or branch-free valid invocation across randomized values | x |
| 721 | `crypto_sign_ed25519ph_update` | `crypto_sign/ed25519/sign_ed25519.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 722 | `crypto_sign_final_create` | `crypto_sign/crypto_sign.c:104`; fixed-shape or branch-free valid invocation across randomized values | x |
| 723 | `crypto_sign_final_verify` | `crypto_sign/crypto_sign.c:111`; fixed-shape or branch-free valid invocation across randomized values | x |
| 724 | `crypto_sign_init` | `crypto_sign/crypto_sign.c:91`; fixed-shape or branch-free valid invocation across randomized values | x |
| 725 | `crypto_sign_keypair` | `crypto_sign/crypto_sign.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 726 | `crypto_sign_messagebytes_max` | `crypto_sign/crypto_sign.c:35`; fixed-shape or branch-free valid invocation across randomized values | x |
| 727 | `crypto_sign_open` | `crypto_sign/crypto_sign.c:68`; fixed-shape or branch-free valid invocation across randomized values | x |
| 728 | `crypto_sign_primitive` | `crypto_sign/crypto_sign.c:41`; fixed-shape or branch-free valid invocation across randomized values | x |
| 729 | `crypto_sign_publickeybytes` | `crypto_sign/crypto_sign.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 730 | `crypto_sign_secretkeybytes` | `crypto_sign/crypto_sign.c:29`; fixed-shape or branch-free valid invocation across randomized values | x |
| 731 | `crypto_sign_seed_keypair` | `crypto_sign/crypto_sign.c:47`; fixed-shape or branch-free valid invocation across randomized values | x |
| 732 | `crypto_sign_seedbytes` | `crypto_sign/crypto_sign.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 733 | `crypto_sign_statebytes` | `crypto_sign/crypto_sign.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 734 | `crypto_sign_update` | `crypto_sign/crypto_sign.c:97`; fixed-shape or branch-free valid invocation across randomized values | x |
| 735 | `crypto_sign_verify_detached` | `crypto_sign/crypto_sign.c:84`; fixed-shape or branch-free valid invocation across randomized values | x |
| 736 | `crypto_stream` | `crypto_stream/crypto_stream.c:30`; fixed-shape or branch-free valid invocation across randomized values | x |
| 737 | `crypto_stream_chacha20` | `crypto_stream/chacha20/stream_chacha20.c:64`; empty/one/many and boundary shapes; both outcomes of: if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) | x |
| 738 | `crypto_stream_chacha20_ietf` | `crypto_stream/chacha20/stream_chacha20.c:130`; empty/one/many and boundary shapes; both outcomes of: if (clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) | x |
| 739 | `crypto_stream_chacha20_ietf_ext` | `crypto_stream/chacha20/stream_chacha20.c:97`; empty/one/many and boundary shapes; both outcomes of: if (clen > crypto_stream_chacha20_MESSAGEBYTES_MAX) | x |
| 740 | `crypto_stream_chacha20_ietf_ext_xor_ic` | `crypto_stream/chacha20/stream_chacha20.c:107`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) | x |
| 741 | `crypto_stream_chacha20_ietf_keybytes` | `crypto_stream/chacha20/stream_chacha20.c:48`; fixed-shape or branch-free valid invocation across randomized values | x |
| 742 | `crypto_stream_chacha20_ietf_keygen` | `crypto_stream/chacha20/stream_chacha20.c:164`; fixed-shape or branch-free valid invocation across randomized values | x |
| 743 | `crypto_stream_chacha20_ietf_messagebytes_max` | `crypto_stream/chacha20/stream_chacha20.c:58`; fixed-shape or branch-free valid invocation across randomized values | x |
| 744 | `crypto_stream_chacha20_ietf_noncebytes` | `crypto_stream/chacha20/stream_chacha20.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 745 | `crypto_stream_chacha20_ietf_xor` | `crypto_stream/chacha20/stream_chacha20.c:153`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX) | x |
| 746 | `crypto_stream_chacha20_ietf_xor_ic` | `crypto_stream/chacha20/stream_chacha20.c:140`; empty/one/many and boundary shapes; both outcomes of: if ((unsigned long long) ic > | x |
| 747 | `crypto_stream_chacha20_keybytes` | `crypto_stream/chacha20/stream_chacha20.c:32`; fixed-shape or branch-free valid invocation across randomized values | x |
| 748 | `crypto_stream_chacha20_keygen` | `crypto_stream/chacha20/stream_chacha20.c:170`; fixed-shape or branch-free valid invocation across randomized values | x |
| 749 | `crypto_stream_chacha20_messagebytes_max` | `crypto_stream/chacha20/stream_chacha20.c:42`; fixed-shape or branch-free valid invocation across randomized values | x |
| 750 | `crypto_stream_chacha20_noncebytes` | `crypto_stream/chacha20/stream_chacha20.c:37`; fixed-shape or branch-free valid invocation across randomized values | x |
| 751 | `crypto_stream_chacha20_ref_implementation` | exported data object; compare all bytes after loader initialization | x |
| 752 | `crypto_stream_chacha20_xor` | `crypto_stream/chacha20/stream_chacha20.c:86`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) | x |
| 753 | `crypto_stream_chacha20_xor_ic` | `crypto_stream/chacha20/stream_chacha20.c:74`; empty/one/many and boundary shapes; both outcomes of: if (mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX) | x |
| 754 | `crypto_stream_keybytes` | `crypto_stream/crypto_stream.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 755 | `crypto_stream_keygen` | `crypto_stream/crypto_stream.c:46`; fixed-shape or branch-free valid invocation across randomized values | x |
| 756 | `crypto_stream_messagebytes_max` | `crypto_stream/crypto_stream.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 757 | `crypto_stream_noncebytes` | `crypto_stream/crypto_stream.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 758 | `crypto_stream_primitive` | `crypto_stream/crypto_stream.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 759 | `crypto_stream_salsa20` | `crypto_stream/salsa20/stream_salsa20.c:57`; fixed-shape or branch-free valid invocation across randomized values | x |
| 760 | `crypto_stream_salsa2012` | `crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:14`; empty/one/many and boundary shapes; both outcomes of: if (!clen); if (clen) | x |
| 761 | `crypto_stream_salsa2012_keybytes` | `crypto_stream/salsa2012/stream_salsa2012.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |
| 762 | `crypto_stream_salsa2012_keygen` | `crypto_stream/salsa2012/stream_salsa2012.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 763 | `crypto_stream_salsa2012_messagebytes_max` | `crypto_stream/salsa2012/stream_salsa2012.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 764 | `crypto_stream_salsa2012_noncebytes` | `crypto_stream/salsa2012/stream_salsa2012.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 765 | `crypto_stream_salsa2012_xor` | `crypto_stream/salsa2012/ref/stream_salsa2012_ref.c:59`; empty/one/many and boundary shapes; both outcomes of: if (!mlen); if (mlen) | x |
| 766 | `crypto_stream_salsa208` | `crypto_stream/salsa208/ref/stream_salsa208_ref.c:16`; empty/one/many and boundary shapes; both outcomes of: if (!clen); if (clen) | x |
| 767 | `crypto_stream_salsa208_keybytes` | `crypto_stream/salsa208/stream_salsa208.c:7`; fixed-shape or branch-free valid invocation across randomized values | x |
| 768 | `crypto_stream_salsa208_keygen` | `crypto_stream/salsa208/stream_salsa208.c:25`; fixed-shape or branch-free valid invocation across randomized values | x |
| 769 | `crypto_stream_salsa208_messagebytes_max` | `crypto_stream/salsa208/stream_salsa208.c:19`; fixed-shape or branch-free valid invocation across randomized values | x |
| 770 | `crypto_stream_salsa208_noncebytes` | `crypto_stream/salsa208/stream_salsa208.c:13`; fixed-shape or branch-free valid invocation across randomized values | x |
| 771 | `crypto_stream_salsa208_xor` | `crypto_stream/salsa208/ref/stream_salsa208_ref.c:61`; empty/one/many and boundary shapes; both outcomes of: if (!mlen); if (mlen) | x |
| 772 | `crypto_stream_salsa20_keybytes` | `crypto_stream/salsa20/stream_salsa20.c:39`; fixed-shape or branch-free valid invocation across randomized values | x |
| 773 | `crypto_stream_salsa20_keygen` | `crypto_stream/salsa20/stream_salsa20.c:81`; fixed-shape or branch-free valid invocation across randomized values | x |
| 774 | `crypto_stream_salsa20_messagebytes_max` | `crypto_stream/salsa20/stream_salsa20.c:51`; fixed-shape or branch-free valid invocation across randomized values | x |
| 775 | `crypto_stream_salsa20_noncebytes` | `crypto_stream/salsa20/stream_salsa20.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 776 | `crypto_stream_salsa20_ref_implementation` | exported data object; compare all bytes after loader initialization | x |
| 777 | `crypto_stream_salsa20_xor` | `crypto_stream/salsa20/stream_salsa20.c:73`; fixed-shape or branch-free valid invocation across randomized values | x |
| 778 | `crypto_stream_salsa20_xor_ic` | `crypto_stream/salsa20/stream_salsa20.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 779 | `crypto_stream_xchacha20` | `crypto_stream/xchacha20/stream_xchacha20.c:29`; fixed-shape or branch-free valid invocation across randomized values | x |
| 780 | `crypto_stream_xchacha20_keybytes` | `crypto_stream/xchacha20/stream_xchacha20.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 781 | `crypto_stream_xchacha20_keygen` | `crypto_stream/xchacha20/stream_xchacha20.c:65`; fixed-shape or branch-free valid invocation across randomized values | x |
| 782 | `crypto_stream_xchacha20_messagebytes_max` | `crypto_stream/xchacha20/stream_xchacha20.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 783 | `crypto_stream_xchacha20_noncebytes` | `crypto_stream/xchacha20/stream_xchacha20.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 784 | `crypto_stream_xchacha20_xor` | `crypto_stream/xchacha20/stream_xchacha20.c:57`; fixed-shape or branch-free valid invocation across randomized values | x |
| 785 | `crypto_stream_xchacha20_xor_ic` | `crypto_stream/xchacha20/stream_xchacha20.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 786 | `crypto_stream_xor` | `crypto_stream/crypto_stream.c:38`; fixed-shape or branch-free valid invocation across randomized values | x |
| 787 | `crypto_stream_xsalsa20` | `crypto_stream/xsalsa20/stream_xsalsa20.c:8`; fixed-shape or branch-free valid invocation across randomized values | x |
| 788 | `crypto_stream_xsalsa20_keybytes` | `crypto_stream/xsalsa20/stream_xsalsa20.c:45`; fixed-shape or branch-free valid invocation across randomized values | x |
| 789 | `crypto_stream_xsalsa20_keygen` | `crypto_stream/xsalsa20/stream_xsalsa20.c:63`; fixed-shape or branch-free valid invocation across randomized values | x |
| 790 | `crypto_stream_xsalsa20_messagebytes_max` | `crypto_stream/xsalsa20/stream_xsalsa20.c:57`; fixed-shape or branch-free valid invocation across randomized values | x |
| 791 | `crypto_stream_xsalsa20_noncebytes` | `crypto_stream/xsalsa20/stream_xsalsa20.c:51`; fixed-shape or branch-free valid invocation across randomized values | x |
| 792 | `crypto_stream_xsalsa20_xor` | `crypto_stream/xsalsa20/stream_xsalsa20.c:37`; fixed-shape or branch-free valid invocation across randomized values | x |
| 793 | `crypto_stream_xsalsa20_xor_ic` | `crypto_stream/xsalsa20/stream_xsalsa20.c:22`; fixed-shape or branch-free valid invocation across randomized values | x |
| 794 | `crypto_verify_16` | `crypto_verify/verify.c:89`; fixed-shape or branch-free valid invocation across randomized values | x |
| 795 | `crypto_verify_16_bytes` | `crypto_verify/verify.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 796 | `crypto_verify_32` | `crypto_verify/verify.c:95`; fixed-shape or branch-free valid invocation across randomized values | x |
| 797 | `crypto_verify_32_bytes` | `crypto_verify/verify.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 798 | `crypto_verify_64` | `crypto_verify/verify.c:101`; fixed-shape or branch-free valid invocation across randomized values | x |
| 799 | `crypto_verify_64_bytes` | `crypto_verify/verify.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 800 | `crypto_xof_shake128` | `crypto_xof/shake128/xof_shake128.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 801 | `crypto_xof_shake128_blockbytes` | `crypto_xof/shake128/xof_shake128.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 802 | `crypto_xof_shake128_domain_standard` | `crypto_xof/shake128/xof_shake128.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 803 | `crypto_xof_shake128_init` | `crypto_xof/shake128/xof_shake128.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 804 | `crypto_xof_shake128_init_with_domain` | `crypto_xof/shake128/xof_shake128.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 805 | `crypto_xof_shake128_squeeze` | `crypto_xof/shake128/xof_shake128.c:63`; fixed-shape or branch-free valid invocation across randomized values | x |
| 806 | `crypto_xof_shake128_statebytes` | `crypto_xof/shake128/xof_shake128.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 807 | `crypto_xof_shake128_update` | `crypto_xof/shake128/xof_shake128.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 808 | `crypto_xof_shake256` | `crypto_xof/shake256/xof_shake256.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 809 | `crypto_xof_shake256_blockbytes` | `crypto_xof/shake256/xof_shake256.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 810 | `crypto_xof_shake256_domain_standard` | `crypto_xof/shake256/xof_shake256.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 811 | `crypto_xof_shake256_init` | `crypto_xof/shake256/xof_shake256.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 812 | `crypto_xof_shake256_init_with_domain` | `crypto_xof/shake256/xof_shake256.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 813 | `crypto_xof_shake256_squeeze` | `crypto_xof/shake256/xof_shake256.c:63`; fixed-shape or branch-free valid invocation across randomized values | x |
| 814 | `crypto_xof_shake256_statebytes` | `crypto_xof/shake256/xof_shake256.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 815 | `crypto_xof_shake256_update` | `crypto_xof/shake256/xof_shake256.c:53`; fixed-shape or branch-free valid invocation across randomized values | x |
| 816 | `crypto_xof_turboshake128` | `crypto_xof/turboshake128/xof_turboshake128.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 817 | `crypto_xof_turboshake128_blockbytes` | `crypto_xof/turboshake128/xof_turboshake128.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 818 | `crypto_xof_turboshake128_domain_standard` | `crypto_xof/turboshake128/xof_turboshake128.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 819 | `crypto_xof_turboshake128_init` | `crypto_xof/turboshake128/xof_turboshake128.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 820 | `crypto_xof_turboshake128_init_with_domain` | `crypto_xof/turboshake128/xof_turboshake128.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 821 | `crypto_xof_turboshake128_squeeze` | `crypto_xof/turboshake128/xof_turboshake128.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 822 | `crypto_xof_turboshake128_statebytes` | `crypto_xof/turboshake128/xof_turboshake128.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 823 | `crypto_xof_turboshake128_update` | `crypto_xof/turboshake128/xof_turboshake128.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 824 | `crypto_xof_turboshake256` | `crypto_xof/turboshake256/xof_turboshake256.c:24`; fixed-shape or branch-free valid invocation across randomized values | x |
| 825 | `crypto_xof_turboshake256_blockbytes` | `crypto_xof/turboshake256/xof_turboshake256.c:6`; fixed-shape or branch-free valid invocation across randomized values | x |
| 826 | `crypto_xof_turboshake256_domain_standard` | `crypto_xof/turboshake256/xof_turboshake256.c:18`; fixed-shape or branch-free valid invocation across randomized values | x |
| 827 | `crypto_xof_turboshake256_init` | `crypto_xof/turboshake256/xof_turboshake256.c:33`; fixed-shape or branch-free valid invocation across randomized values | x |
| 828 | `crypto_xof_turboshake256_init_with_domain` | `crypto_xof/turboshake256/xof_turboshake256.c:43`; fixed-shape or branch-free valid invocation across randomized values | x |
| 829 | `crypto_xof_turboshake256_squeeze` | `crypto_xof/turboshake256/xof_turboshake256.c:64`; fixed-shape or branch-free valid invocation across randomized values | x |
| 830 | `crypto_xof_turboshake256_statebytes` | `crypto_xof/turboshake256/xof_turboshake256.c:12`; fixed-shape or branch-free valid invocation across randomized values | x |
| 831 | `crypto_xof_turboshake256_update` | `crypto_xof/turboshake256/xof_turboshake256.c:54`; fixed-shape or branch-free valid invocation across randomized values | x |
| 832 | `ipcrypt_soft_implementation` | exported data object; compare all bytes after loader initialization | x |
| 833 | `randombytes` | `randombytes/randombytes.c:245`; fixed-shape or branch-free valid invocation across randomized values | x |
| 834 | `randombytes_buf` | `randombytes/randombytes.c:202`; empty/one/many and boundary shapes; both outcomes of: if (size > (size_t) 0U) | x |
| 835 | `randombytes_buf_deterministic` | `randombytes/randombytes.c:211`; empty/one/many and boundary shapes; both outcomes of: if (size > 0x4000000000ULL) | x |
| 836 | `randombytes_close` | `randombytes/randombytes.c:236`; empty/one/many and boundary shapes; both outcomes of: if (implementation != NULL && implementation->close != NULL) | x |
| 837 | `randombytes_implementation_name` | `randombytes/randombytes.c:156`; fixed-shape or branch-free valid invocation across randomized values | x |
| 838 | `randombytes_internal_implementation` | exported data object; compare all bytes after loader initialization | x |
| 839 | `randombytes_random` | `randombytes/randombytes.c:163`; fixed-shape or branch-free valid invocation across randomized values | x |
| 840 | `randombytes_seedbytes` | `randombytes/randombytes.c:230`; fixed-shape or branch-free valid invocation across randomized values | x |
| 841 | `randombytes_set_implementation` | `randombytes/randombytes.c:149`; fixed-shape or branch-free valid invocation across randomized values | x |
| 842 | `randombytes_stir` | `randombytes/randombytes.c:170`; empty/one/many and boundary shapes; both outcomes of: if (implementation->stir != NULL) | x |
| 843 | `randombytes_sysrandom_implementation` | exported data object; compare all bytes after loader initialization | x |
| 844 | `randombytes_uniform` | `randombytes/randombytes.c:179`; empty/one/many and boundary shapes; both outcomes of: if (implementation->uniform != NULL); if (upper_bound < 2) | x |
| 845 | `sodium_add` | `sodium/utils.c:317`; empty/one/many and boundary shapes; both outcomes of: if (len == 12U); } else if (len == 24U); } else if (len == 8U) | x |
| 846 | `sodium_allocarray` | `sodium/utils.c:653`; empty/one/many and boundary shapes; both outcomes of: if (count > (size_t) 0U && size >= (size_t) SIZE_MAX / count) | x |
| 847 | `sodium_base642bin` | `sodium/codecs.c:276`; empty/one/many and boundary shapes; both outcomes of: if (is_urlsafe); if (d == 0xFF); if (ignore != NULL && strchr(ignore, c) != NULL); if (acc_len >= 8); … (12 direct branch axes total) | x |
| 848 | `sodium_base64_encoded_len` | `sodium/codecs.c:174`; empty/one/many and boundary shapes; both outcomes of: if (bin_len / 3 > (SIZE_MAX - 5) / 4) | x |
| 849 | `sodium_bin2base64` | `sodium/codecs.c:185`; empty/one/many and boundary shapes; both outcomes of: if (nibbles > (SIZE_MAX - 5) / 4); if (remainder != 0); if ((((unsigned int) variant) & VARIANT_NO_PADDING_MASK) == 0U); if (b64_maxlen <= b64_len); … (7 direct branch axes total) | x |
| 850 | `sodium_bin2hex` | `sodium/codecs.c:15`; empty/one/many and boundary shapes; both outcomes of: if (bin_len >= SIZE_MAX / 2 \|\| hex_maxlen <= bin_len * 2U) | x |
| 851 | `sodium_bin2ip` | `sodium/codecs.c:550`; empty/one/many and boundary shapes; both outcomes of: if (ip_maxlen <= 2U); if (memcmp(bin, ipv4_mapped_prefix, 12U) == 0); if (i != 0); if (len >= ip_maxlen); … (12 direct branch axes total) | x |
| 852 | `sodium_compare` | `sodium/utils.c:227`; fixed-shape or branch-free valid invocation across randomized values | x |
| 853 | `sodium_crit_enter` | `sodium/core.c:117`; empty/one/many and boundary shapes; both outcomes of: if ((ret = pthread_mutex_lock(&_sodium_lock)) == 0) | x |
| 854 | `sodium_crit_leave` | `sodium/core.c:129`; empty/one/many and boundary shapes; both outcomes of: if (locked == 0) | x |
| 855 | `sodium_free` | `sodium/utils.c:664`; fixed-shape or branch-free valid invocation across randomized values | x |
| 856 | `sodium_hex2bin` | `sodium/codecs.c:42`; empty/one/many and boundary shapes; both outcomes of: if ((c_num0 \| c_alpha0) == 0U); if (ignore != NULL && state == 0U && strchr(ignore, c) != NULL); if (bin_pos >= bin_maxlen); if (state == 0U); … (9 direct branch axes total) | x |
| 857 | `sodium_increment` | `sodium/utils.c:270`; empty/one/many and boundary shapes; both outcomes of: if (nlen == 12U); } else if (nlen == 24U); } else if (nlen == 8U) | x |
| 858 | `sodium_init` | `sodium/core.c:28`; empty/one/many and boundary shapes; both outcomes of: if (sodium_crit_enter() != 0); if (initialized != 0); if (sodium_crit_leave() != 0); if (sodium_crit_leave() != 0) | x |
| 859 | `sodium_ip2bin` | `sodium/codecs.c:484`; empty/one/many and boundary shapes; both outcomes of: if (zone != NULL); if (!((*z >= '0' && *z <= '9') \|\| (*z >= 'a' && *z <= 'z') \|\|; if (zone + 1 >= end); if (zone != NULL && !is_ipv6); … (6 direct branch axes total) | x |
| 860 | `sodium_is_zero` | `sodium/utils.c:258`; fixed-shape or branch-free valid invocation across randomized values | x |
| 861 | `sodium_library_minimal` | `sodium/version.c:23`; fixed-shape or branch-free valid invocation across randomized values | x |
| 862 | `sodium_library_version_major` | `sodium/version.c:11`; fixed-shape or branch-free valid invocation across randomized values | x |
| 863 | `sodium_library_version_minor` | `sodium/version.c:17`; fixed-shape or branch-free valid invocation across randomized values | x |
| 864 | `sodium_malloc` | `sodium/utils.c:640`; empty/one/many and boundary shapes; both outcomes of: if ((ptr = _sodium_malloc(size)) == NULL) | x |
| 865 | `sodium_memcmp` | `sodium/utils.c:187`; fixed-shape or branch-free valid invocation across randomized values | x |
| 866 | `sodium_memzero` | `sodium/utils.c:126`; empty/one/many and boundary shapes; both outcomes of: if (len > 0U && memset_s(pnt, (rsize_t) len, 0, (rsize_t) len) != 0); if (len > 0U) | x |
| 867 | `sodium_misuse` | `sodium/core.c:192`; empty/one/many and boundary shapes; both outcomes of: if (sodium_crit_enter() == 0); if (sodium_crit_leave() == 0 && handler != NULL) | x |
| 868 | `sodium_mlock` | `sodium/utils.c:433`; fixed-shape or branch-free valid invocation across randomized values | x |
| 869 | `sodium_mprotect_noaccess` | `sodium/utils.c:727`; fixed-shape or branch-free valid invocation across randomized values | x |
| 870 | `sodium_mprotect_readonly` | `sodium/utils.c:733`; fixed-shape or branch-free valid invocation across randomized values | x |
| 871 | `sodium_mprotect_readwrite` | `sodium/utils.c:739`; fixed-shape or branch-free valid invocation across randomized values | x |
| 872 | `sodium_munlock` | `sodium/utils.c:449`; fixed-shape or branch-free valid invocation across randomized values | x |
| 873 | `sodium_pad` | `sodium/utils.c:745`; empty/one/many and boundary shapes; both outcomes of: if (blocksize <= 0U); if ((blocksize & (blocksize - 1U)) == 0U); if ((size_t) SIZE_MAX - unpadded_buflen <= xpadlen); if (xpadded_len >= max_buflen); … (5 direct branch axes total) | x |
| 874 | `sodium_runtime_has_aesni` | `sodium/runtime.c:391`; fixed-shape or branch-free valid invocation across randomized values | x |
| 875 | `sodium_runtime_has_armcrypto` | `sodium/runtime.c:337`; fixed-shape or branch-free valid invocation across randomized values | x |
| 876 | `sodium_runtime_has_avx` | `sodium/runtime.c:367`; fixed-shape or branch-free valid invocation across randomized values | x |
| 877 | `sodium_runtime_has_avx2` | `sodium/runtime.c:373`; fixed-shape or branch-free valid invocation across randomized values | x |
| 878 | `sodium_runtime_has_avx512f` | `sodium/runtime.c:379`; fixed-shape or branch-free valid invocation across randomized values | x |
| 879 | `sodium_runtime_has_neon` | `sodium/runtime.c:331`; fixed-shape or branch-free valid invocation across randomized values | x |
| 880 | `sodium_runtime_has_pclmul` | `sodium/runtime.c:385`; fixed-shape or branch-free valid invocation across randomized values | x |
| 881 | `sodium_runtime_has_rdrand` | `sodium/runtime.c:397`; fixed-shape or branch-free valid invocation across randomized values | x |
| 882 | `sodium_runtime_has_sse2` | `sodium/runtime.c:343`; fixed-shape or branch-free valid invocation across randomized values | x |
| 883 | `sodium_runtime_has_sse3` | `sodium/runtime.c:349`; fixed-shape or branch-free valid invocation across randomized values | x |
| 884 | `sodium_runtime_has_sse41` | `sodium/runtime.c:361`; fixed-shape or branch-free valid invocation across randomized values | x |
| 885 | `sodium_runtime_has_ssse3` | `sodium/runtime.c:355`; fixed-shape or branch-free valid invocation across randomized values | x |
| 886 | `sodium_set_misuse_handler` | `sodium/core.c:209`; empty/one/many and boundary shapes; both outcomes of: if (sodium_crit_enter() != 0); if (sodium_crit_leave() != 0) | x |
| 887 | `sodium_stackzero` | `sodium/utils.c:160`; fixed-shape or branch-free valid invocation across randomized values | x |
| 888 | `sodium_sub` | `sodium/utils.c:366`; empty/one/many and boundary shapes; both outcomes of: if (len == 64U) | x |
| 889 | `sodium_unpad` | `sodium/utils.c:786`; empty/one/many and boundary shapes; both outcomes of: if (padded_buflen < blocksize \|\| blocksize <= 0U) | x |
| 890 | `sodium_version_string` | `sodium/version.c:5`; fixed-shape or branch-free valid invocation across randomized values | x |

## Feature combinations

- [x] Default feature set. `Cargo.toml` declares no optional features, so this is the only feature combination.
