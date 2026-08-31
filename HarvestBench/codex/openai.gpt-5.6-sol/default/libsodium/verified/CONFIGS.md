# Configuration Surface

Generated from every function symbol exported by the C shared object. Symbols also declared in non-private public headers are marked `public header`; all remaining exports are marked `low-level nm export`. Shape axes are assigned mechanically from the entry-point family and include direct low-level calls.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `_crypto_aead_aegis128l_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 2 | `_crypto_aead_aegis256_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 3 | `_crypto_generichash_blake2b_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 4 | `_crypto_ipcrypt_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 5 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 6 | `_crypto_pwhash_argon2_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 7 | `_crypto_scalarmult_curve25519_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 8 | `_crypto_sign_ed25519_detached` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 9 | `_crypto_sign_ed25519_ref10_hinit` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 10 | `_crypto_sign_ed25519_verify_detached` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 11 | `_crypto_stream_chacha20_pick_best_implementation` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 12 | `_crypto_stream_salsa20_pick_best_implementation` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 13 | `_sodium_alloc_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 14 | `_sodium_argon2_ctx` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 15 | `_sodium_argon2_decode_string` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 16 | `_sodium_argon2_encode_string` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 17 | `_sodium_argon2_fill_memory_blocks` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 18 | `_sodium_argon2_fill_segment_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 19 | `_sodium_argon2_finalize` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 20 | `_sodium_argon2_hash` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 21 | `_sodium_argon2_initialize` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 22 | `_sodium_argon2_validate_inputs` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 23 | `_sodium_argon2_verify` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 24 | `_sodium_argon2i_hash_encoded` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 25 | `_sodium_argon2i_hash_raw` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 26 | `_sodium_argon2i_verify` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 27 | `_sodium_argon2id_hash_encoded` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 28 | `_sodium_argon2id_hash_raw` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 29 | `_sodium_argon2id_verify` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 30 | `_sodium_blake2b` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 31 | `_sodium_blake2b_compress_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 32 | `_sodium_blake2b_final` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 33 | `_sodium_blake2b_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 34 | `_sodium_blake2b_init_key` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 35 | `_sodium_blake2b_init_key_salt_personal` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 36 | `_sodium_blake2b_init_param` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 37 | `_sodium_blake2b_init_salt_personal` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 38 | `_sodium_blake2b_long` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 39 | `_sodium_blake2b_pick_best_implementation` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 40 | `_sodium_blake2b_salt_personal` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 41 | `_sodium_blake2b_update` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 42 | `_sodium_core_h2c_string_to_hash` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 43 | `_sodium_escrypt_PBKDF2_SHA256` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 44 | `_sodium_escrypt_alloc_region` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 45 | `_sodium_escrypt_free_local` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 46 | `_sodium_escrypt_free_region` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 47 | `_sodium_escrypt_gensalt_r` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 48 | `_sodium_escrypt_init_local` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 49 | `_sodium_escrypt_kdf_nosse` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 50 | `_sodium_escrypt_parse_setting` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 51 | `_sodium_escrypt_r` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 52 | `_sodium_fe25519_frombytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 53 | `_sodium_fe25519_invert` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 54 | `_sodium_fe25519_tobytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 55 | `_sodium_ge25519_clear_cofactor` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 56 | `_sodium_ge25519_double_scalarmult_vartime` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 57 | `_sodium_ge25519_from_hash` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 58 | `_sodium_ge25519_from_uniform` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 59 | `_sodium_ge25519_frombytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 60 | `_sodium_ge25519_frombytes_negate_vartime` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 61 | `_sodium_ge25519_has_small_order` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 62 | `_sodium_ge25519_is_canonical` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 63 | `_sodium_ge25519_is_on_curve` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 64 | `_sodium_ge25519_is_on_main_subgroup` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 65 | `_sodium_ge25519_p1p1_to_p2` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 66 | `_sodium_ge25519_p1p1_to_p3` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 67 | `_sodium_ge25519_p2_to_p3` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 68 | `_sodium_ge25519_p3_add` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 69 | `_sodium_ge25519_p3_sub` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 70 | `_sodium_ge25519_p3_tobytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 71 | `_sodium_ge25519_scalarmult` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 72 | `_sodium_ge25519_scalarmult_base` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 73 | `_sodium_ge25519_tobytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 74 | `_sodium_keccak1600_ref_extract_bytes` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 75 | `_sodium_keccak1600_ref_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 76 | `_sodium_keccak1600_ref_permute_12` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 77 | `_sodium_keccak1600_ref_permute_24` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 78 | `_sodium_keccak1600_ref_xor_bytes` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 79 | `_sodium_mlkem768_ref_dec` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 80 | `_sodium_mlkem768_ref_enc` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 81 | `_sodium_mlkem768_ref_enc_deterministic` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 82 | `_sodium_mlkem768_ref_keypair` | low-level nm export; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 83 | `_sodium_mlkem768_ref_seed_keypair` | low-level nm export; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 84 | `_sodium_ristretto255_from_hash` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 85 | `_sodium_ristretto255_frombytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 86 | `_sodium_ristretto255_p3_tobytes` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 87 | `_sodium_runtime_get_cpu_features` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 88 | `_sodium_sc25519_invert` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 89 | `_sodium_sc25519_is_canonical` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 90 | `_sodium_sc25519_mul` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 91 | `_sodium_sc25519_muladd` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 92 | `_sodium_sc25519_reduce` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 93 | `_sodium_shake128_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 94 | `_sodium_shake128_ref_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 95 | `_sodium_shake128_ref_init_with_domain` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 96 | `_sodium_shake128_ref_squeeze` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 97 | `_sodium_shake128_ref_update` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 98 | `_sodium_shake256_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 99 | `_sodium_shake256_ref_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 100 | `_sodium_shake256_ref_init_with_domain` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 101 | `_sodium_shake256_ref_squeeze` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 102 | `_sodium_shake256_ref_update` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 103 | `_sodium_softaes_block_decrypt` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 104 | `_sodium_softaes_block_decryptlast` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 105 | `_sodium_softaes_block_encrypt` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 106 | `_sodium_softaes_block_encryptlast` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 107 | `_sodium_softaes_expand_key128` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 108 | `_sodium_softaes_expand_key256` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 109 | `_sodium_softaes_inv_mix_columns` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 110 | `_sodium_softaes_invert_key_schedule128` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 111 | `_sodium_softaes_invert_key_schedule256` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 112 | `_sodium_turboshake128_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 113 | `_sodium_turboshake128_ref_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 114 | `_sodium_turboshake128_ref_init_with_domain` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 115 | `_sodium_turboshake128_ref_squeeze` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 116 | `_sodium_turboshake128_ref_update` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 117 | `_sodium_turboshake256_ref` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 118 | `_sodium_turboshake256_ref_init` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 119 | `_sodium_turboshake256_ref_init_with_domain` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 120 | `_sodium_turboshake256_ref_squeeze` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 121 | `_sodium_turboshake256_ref_update` | low-level nm export; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 122 | `crypto_aead_aegis128l_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 123 | `crypto_aead_aegis128l_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 124 | `crypto_aead_aegis128l_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 125 | `crypto_aead_aegis128l_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 126 | `crypto_aead_aegis128l_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 127 | `crypto_aead_aegis128l_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 128 | `crypto_aead_aegis128l_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 129 | `crypto_aead_aegis128l_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 130 | `crypto_aead_aegis128l_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 131 | `crypto_aead_aegis128l_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 132 | `crypto_aead_aegis256_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 133 | `crypto_aead_aegis256_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 134 | `crypto_aead_aegis256_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 135 | `crypto_aead_aegis256_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 136 | `crypto_aead_aegis256_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 137 | `crypto_aead_aegis256_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 138 | `crypto_aead_aegis256_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 139 | `crypto_aead_aegis256_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 140 | `crypto_aead_aegis256_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 141 | `crypto_aead_aegis256_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 142 | `crypto_aead_aes256gcm_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 143 | `crypto_aead_aes256gcm_beforenm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 144 | `crypto_aead_aes256gcm_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 145 | `crypto_aead_aes256gcm_decrypt_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 146 | `crypto_aead_aes256gcm_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 147 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 148 | `crypto_aead_aes256gcm_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 149 | `crypto_aead_aes256gcm_encrypt_afternm` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 150 | `crypto_aead_aes256gcm_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 151 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 152 | `crypto_aead_aes256gcm_is_available` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 153 | `crypto_aead_aes256gcm_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 154 | `crypto_aead_aes256gcm_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 155 | `crypto_aead_aes256gcm_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 156 | `crypto_aead_aes256gcm_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 157 | `crypto_aead_aes256gcm_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 158 | `crypto_aead_aes256gcm_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 159 | `crypto_aead_chacha20poly1305_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 160 | `crypto_aead_chacha20poly1305_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 161 | `crypto_aead_chacha20poly1305_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 162 | `crypto_aead_chacha20poly1305_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 163 | `crypto_aead_chacha20poly1305_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 164 | `crypto_aead_chacha20poly1305_ietf_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 165 | `crypto_aead_chacha20poly1305_ietf_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 166 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 167 | `crypto_aead_chacha20poly1305_ietf_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 168 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 169 | `crypto_aead_chacha20poly1305_ietf_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 170 | `crypto_aead_chacha20poly1305_ietf_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 171 | `crypto_aead_chacha20poly1305_ietf_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 172 | `crypto_aead_chacha20poly1305_ietf_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 173 | `crypto_aead_chacha20poly1305_ietf_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 174 | `crypto_aead_chacha20poly1305_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 175 | `crypto_aead_chacha20poly1305_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 176 | `crypto_aead_chacha20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 177 | `crypto_aead_chacha20poly1305_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 178 | `crypto_aead_chacha20poly1305_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 179 | `crypto_aead_xchacha20poly1305_ietf_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 180 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 181 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 182 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 183 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 184 | `crypto_aead_xchacha20poly1305_ietf_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 185 | `crypto_aead_xchacha20poly1305_ietf_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 186 | `crypto_aead_xchacha20poly1305_ietf_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 187 | `crypto_aead_xchacha20poly1305_ietf_npubbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 188 | `crypto_aead_xchacha20poly1305_ietf_nsecbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 189 | `crypto_auth` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 190 | `crypto_auth_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 191 | `crypto_auth_hmacsha256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 192 | `crypto_auth_hmacsha256_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 193 | `crypto_auth_hmacsha256_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 194 | `crypto_auth_hmacsha256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 195 | `crypto_auth_hmacsha256_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 196 | `crypto_auth_hmacsha256_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 197 | `crypto_auth_hmacsha256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 198 | `crypto_auth_hmacsha256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 199 | `crypto_auth_hmacsha256_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 200 | `crypto_auth_hmacsha512` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 201 | `crypto_auth_hmacsha512256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 202 | `crypto_auth_hmacsha512256_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 203 | `crypto_auth_hmacsha512256_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 204 | `crypto_auth_hmacsha512256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 205 | `crypto_auth_hmacsha512256_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 206 | `crypto_auth_hmacsha512256_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 207 | `crypto_auth_hmacsha512256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 208 | `crypto_auth_hmacsha512256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 209 | `crypto_auth_hmacsha512256_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 210 | `crypto_auth_hmacsha512_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 211 | `crypto_auth_hmacsha512_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 212 | `crypto_auth_hmacsha512_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 213 | `crypto_auth_hmacsha512_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 214 | `crypto_auth_hmacsha512_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 215 | `crypto_auth_hmacsha512_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 216 | `crypto_auth_hmacsha512_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 217 | `crypto_auth_hmacsha512_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 218 | `crypto_auth_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 219 | `crypto_auth_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 220 | `crypto_auth_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 221 | `crypto_auth_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 222 | `crypto_box` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 223 | `crypto_box_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 224 | `crypto_box_beforenm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 225 | `crypto_box_beforenmbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 226 | `crypto_box_boxzerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 227 | `crypto_box_curve25519xchacha20poly1305_beforenm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 228 | `crypto_box_curve25519xchacha20poly1305_beforenmbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 229 | `crypto_box_curve25519xchacha20poly1305_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 230 | `crypto_box_curve25519xchacha20poly1305_detached_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 231 | `crypto_box_curve25519xchacha20poly1305_easy` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 232 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 233 | `crypto_box_curve25519xchacha20poly1305_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 234 | `crypto_box_curve25519xchacha20poly1305_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 235 | `crypto_box_curve25519xchacha20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 236 | `crypto_box_curve25519xchacha20poly1305_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 237 | `crypto_box_curve25519xchacha20poly1305_open_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 238 | `crypto_box_curve25519xchacha20poly1305_open_detached_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 239 | `crypto_box_curve25519xchacha20poly1305_open_easy` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 240 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 241 | `crypto_box_curve25519xchacha20poly1305_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 242 | `crypto_box_curve25519xchacha20poly1305_seal` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 243 | `crypto_box_curve25519xchacha20poly1305_seal_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 244 | `crypto_box_curve25519xchacha20poly1305_sealbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 245 | `crypto_box_curve25519xchacha20poly1305_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 246 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 247 | `crypto_box_curve25519xchacha20poly1305_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 248 | `crypto_box_curve25519xsalsa20poly1305` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 249 | `crypto_box_curve25519xsalsa20poly1305_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 250 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 251 | `crypto_box_curve25519xsalsa20poly1305_beforenmbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 252 | `crypto_box_curve25519xsalsa20poly1305_boxzerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 253 | `crypto_box_curve25519xsalsa20poly1305_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 254 | `crypto_box_curve25519xsalsa20poly1305_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 255 | `crypto_box_curve25519xsalsa20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 256 | `crypto_box_curve25519xsalsa20poly1305_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 257 | `crypto_box_curve25519xsalsa20poly1305_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 258 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 259 | `crypto_box_curve25519xsalsa20poly1305_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 260 | `crypto_box_curve25519xsalsa20poly1305_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 261 | `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 262 | `crypto_box_curve25519xsalsa20poly1305_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 263 | `crypto_box_curve25519xsalsa20poly1305_zerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 264 | `crypto_box_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 265 | `crypto_box_detached_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 266 | `crypto_box_easy` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 267 | `crypto_box_easy_afternm` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 268 | `crypto_box_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 269 | `crypto_box_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 270 | `crypto_box_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 271 | `crypto_box_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 272 | `crypto_box_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 273 | `crypto_box_open_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 274 | `crypto_box_open_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 275 | `crypto_box_open_detached_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 276 | `crypto_box_open_easy` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 277 | `crypto_box_open_easy_afternm` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 278 | `crypto_box_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 279 | `crypto_box_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 280 | `crypto_box_seal` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 281 | `crypto_box_seal_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 282 | `crypto_box_sealbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 283 | `crypto_box_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 284 | `crypto_box_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 285 | `crypto_box_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 286 | `crypto_box_zerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 287 | `crypto_core_ed25519_add` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 288 | `crypto_core_ed25519_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 289 | `crypto_core_ed25519_from_string` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 290 | `crypto_core_ed25519_from_string_nu` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 291 | `crypto_core_ed25519_hashbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 292 | `crypto_core_ed25519_is_valid_point` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 293 | `crypto_core_ed25519_nonreducedscalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 294 | `crypto_core_ed25519_random` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 295 | `crypto_core_ed25519_scalar_add` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 296 | `crypto_core_ed25519_scalar_complement` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 297 | `crypto_core_ed25519_scalar_from_string` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 298 | `crypto_core_ed25519_scalar_invert` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 299 | `crypto_core_ed25519_scalar_is_canonical` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 300 | `crypto_core_ed25519_scalar_mul` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 301 | `crypto_core_ed25519_scalar_negate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 302 | `crypto_core_ed25519_scalar_random` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 303 | `crypto_core_ed25519_scalar_reduce` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 304 | `crypto_core_ed25519_scalar_sub` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 305 | `crypto_core_ed25519_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 306 | `crypto_core_ed25519_sub` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 307 | `crypto_core_ed25519_uniformbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 308 | `crypto_core_hchacha20` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 309 | `crypto_core_hchacha20_constbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 310 | `crypto_core_hchacha20_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 311 | `crypto_core_hchacha20_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 312 | `crypto_core_hchacha20_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 313 | `crypto_core_hsalsa20` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 314 | `crypto_core_hsalsa20_constbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 315 | `crypto_core_hsalsa20_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 316 | `crypto_core_hsalsa20_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 317 | `crypto_core_hsalsa20_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 318 | `crypto_core_keccak1600_extract_bytes` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 319 | `crypto_core_keccak1600_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 320 | `crypto_core_keccak1600_permute_12` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 321 | `crypto_core_keccak1600_permute_24` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 322 | `crypto_core_keccak1600_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 323 | `crypto_core_keccak1600_xor_bytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 324 | `crypto_core_ristretto255_add` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 325 | `crypto_core_ristretto255_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 326 | `crypto_core_ristretto255_from_hash` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 327 | `crypto_core_ristretto255_from_string` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 328 | `crypto_core_ristretto255_hashbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 329 | `crypto_core_ristretto255_is_valid_point` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 330 | `crypto_core_ristretto255_nonreducedscalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 331 | `crypto_core_ristretto255_random` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 332 | `crypto_core_ristretto255_scalar_add` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 333 | `crypto_core_ristretto255_scalar_complement` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 334 | `crypto_core_ristretto255_scalar_from_string` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 335 | `crypto_core_ristretto255_scalar_invert` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 336 | `crypto_core_ristretto255_scalar_is_canonical` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 337 | `crypto_core_ristretto255_scalar_mul` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 338 | `crypto_core_ristretto255_scalar_negate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 339 | `crypto_core_ristretto255_scalar_random` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 340 | `crypto_core_ristretto255_scalar_reduce` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 341 | `crypto_core_ristretto255_scalar_sub` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 342 | `crypto_core_ristretto255_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 343 | `crypto_core_ristretto255_sub` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 344 | `crypto_core_salsa20` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 345 | `crypto_core_salsa2012` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 346 | `crypto_core_salsa2012_constbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 347 | `crypto_core_salsa2012_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 348 | `crypto_core_salsa2012_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 349 | `crypto_core_salsa2012_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 350 | `crypto_core_salsa208` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 351 | `crypto_core_salsa208_constbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 352 | `crypto_core_salsa208_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 353 | `crypto_core_salsa208_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 354 | `crypto_core_salsa208_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 355 | `crypto_core_salsa20_constbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 356 | `crypto_core_salsa20_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 357 | `crypto_core_salsa20_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 358 | `crypto_core_salsa20_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 359 | `crypto_generichash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 360 | `crypto_generichash_blake2b` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 361 | `crypto_generichash_blake2b_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 362 | `crypto_generichash_blake2b_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 363 | `crypto_generichash_blake2b_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 364 | `crypto_generichash_blake2b_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 365 | `crypto_generichash_blake2b_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 366 | `crypto_generichash_blake2b_init_salt_personal` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 367 | `crypto_generichash_blake2b_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 368 | `crypto_generichash_blake2b_keybytes_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 369 | `crypto_generichash_blake2b_keybytes_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 370 | `crypto_generichash_blake2b_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 371 | `crypto_generichash_blake2b_personalbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 372 | `crypto_generichash_blake2b_salt_personal` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 373 | `crypto_generichash_blake2b_saltbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 374 | `crypto_generichash_blake2b_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 375 | `crypto_generichash_blake2b_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 376 | `crypto_generichash_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 377 | `crypto_generichash_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 378 | `crypto_generichash_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 379 | `crypto_generichash_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 380 | `crypto_generichash_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 381 | `crypto_generichash_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 382 | `crypto_generichash_keybytes_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 383 | `crypto_generichash_keybytes_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 384 | `crypto_generichash_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 385 | `crypto_generichash_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 386 | `crypto_generichash_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 387 | `crypto_generichash_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 388 | `crypto_hash` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 389 | `crypto_hash_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 390 | `crypto_hash_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 391 | `crypto_hash_sha256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 392 | `crypto_hash_sha256_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 393 | `crypto_hash_sha256_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 394 | `crypto_hash_sha256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 395 | `crypto_hash_sha256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 396 | `crypto_hash_sha256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 397 | `crypto_hash_sha3256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 398 | `crypto_hash_sha3256_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 399 | `crypto_hash_sha3256_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 400 | `crypto_hash_sha3256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 401 | `crypto_hash_sha3256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 402 | `crypto_hash_sha3256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 403 | `crypto_hash_sha3512` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 404 | `crypto_hash_sha3512_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 405 | `crypto_hash_sha3512_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 406 | `crypto_hash_sha3512_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 407 | `crypto_hash_sha3512_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 408 | `crypto_hash_sha3512_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 409 | `crypto_hash_sha512` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 410 | `crypto_hash_sha512_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 411 | `crypto_hash_sha512_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 412 | `crypto_hash_sha512_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 413 | `crypto_hash_sha512_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 414 | `crypto_hash_sha512_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 415 | `crypto_ipcrypt_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 416 | `crypto_ipcrypt_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 417 | `crypto_ipcrypt_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 418 | `crypto_ipcrypt_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 419 | `crypto_ipcrypt_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 420 | `crypto_ipcrypt_nd_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 421 | `crypto_ipcrypt_nd_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 422 | `crypto_ipcrypt_nd_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 423 | `crypto_ipcrypt_nd_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 424 | `crypto_ipcrypt_nd_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 425 | `crypto_ipcrypt_nd_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 426 | `crypto_ipcrypt_nd_tweakbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 427 | `crypto_ipcrypt_ndx_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 428 | `crypto_ipcrypt_ndx_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 429 | `crypto_ipcrypt_ndx_inputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 430 | `crypto_ipcrypt_ndx_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 431 | `crypto_ipcrypt_ndx_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 432 | `crypto_ipcrypt_ndx_outputbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 433 | `crypto_ipcrypt_ndx_tweakbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 434 | `crypto_ipcrypt_pfx_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 435 | `crypto_ipcrypt_pfx_decrypt` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 436 | `crypto_ipcrypt_pfx_encrypt` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 437 | `crypto_ipcrypt_pfx_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 438 | `crypto_ipcrypt_pfx_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 439 | `crypto_kdf_blake2b_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 440 | `crypto_kdf_blake2b_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 441 | `crypto_kdf_blake2b_contextbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 442 | `crypto_kdf_blake2b_derive_from_key` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 443 | `crypto_kdf_blake2b_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 444 | `crypto_kdf_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 445 | `crypto_kdf_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 446 | `crypto_kdf_contextbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 447 | `crypto_kdf_derive_from_key` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 448 | `crypto_kdf_hkdf_sha256_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 449 | `crypto_kdf_hkdf_sha256_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 450 | `crypto_kdf_hkdf_sha256_expand` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 451 | `crypto_kdf_hkdf_sha256_extract` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 452 | `crypto_kdf_hkdf_sha256_extract_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 453 | `crypto_kdf_hkdf_sha256_extract_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 454 | `crypto_kdf_hkdf_sha256_extract_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 455 | `crypto_kdf_hkdf_sha256_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 456 | `crypto_kdf_hkdf_sha256_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 457 | `crypto_kdf_hkdf_sha256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 458 | `crypto_kdf_hkdf_sha512_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 459 | `crypto_kdf_hkdf_sha512_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 460 | `crypto_kdf_hkdf_sha512_expand` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 461 | `crypto_kdf_hkdf_sha512_extract` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 462 | `crypto_kdf_hkdf_sha512_extract_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 463 | `crypto_kdf_hkdf_sha512_extract_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 464 | `crypto_kdf_hkdf_sha512_extract_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 465 | `crypto_kdf_hkdf_sha512_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 466 | `crypto_kdf_hkdf_sha512_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 467 | `crypto_kdf_hkdf_sha512_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 468 | `crypto_kdf_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 469 | `crypto_kdf_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 470 | `crypto_kdf_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 471 | `crypto_kem_ciphertextbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 472 | `crypto_kem_dec` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 473 | `crypto_kem_enc` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 474 | `crypto_kem_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 475 | `crypto_kem_mlkem768_ciphertextbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 476 | `crypto_kem_mlkem768_dec` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 477 | `crypto_kem_mlkem768_enc` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 478 | `crypto_kem_mlkem768_enc_deterministic` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 479 | `crypto_kem_mlkem768_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 480 | `crypto_kem_mlkem768_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 481 | `crypto_kem_mlkem768_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 482 | `crypto_kem_mlkem768_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 483 | `crypto_kem_mlkem768_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 484 | `crypto_kem_mlkem768_sharedsecretbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 485 | `crypto_kem_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 486 | `crypto_kem_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 487 | `crypto_kem_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 488 | `crypto_kem_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 489 | `crypto_kem_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 490 | `crypto_kem_sharedsecretbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 491 | `crypto_kem_xwing_ciphertextbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 492 | `crypto_kem_xwing_dec` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 493 | `crypto_kem_xwing_enc` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 494 | `crypto_kem_xwing_enc_deterministic` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 495 | `crypto_kem_xwing_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 496 | `crypto_kem_xwing_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 497 | `crypto_kem_xwing_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 498 | `crypto_kem_xwing_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 499 | `crypto_kem_xwing_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 500 | `crypto_kem_xwing_sharedsecretbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 501 | `crypto_kx_client_session_keys` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 502 | `crypto_kx_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 503 | `crypto_kx_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 504 | `crypto_kx_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 505 | `crypto_kx_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 506 | `crypto_kx_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 507 | `crypto_kx_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 508 | `crypto_kx_server_session_keys` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 509 | `crypto_kx_sessionkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 510 | `crypto_onetimeauth` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 511 | `crypto_onetimeauth_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 512 | `crypto_onetimeauth_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 513 | `crypto_onetimeauth_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 514 | `crypto_onetimeauth_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 515 | `crypto_onetimeauth_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 516 | `crypto_onetimeauth_poly1305` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 517 | `crypto_onetimeauth_poly1305_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 518 | `crypto_onetimeauth_poly1305_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 519 | `crypto_onetimeauth_poly1305_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 520 | `crypto_onetimeauth_poly1305_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 521 | `crypto_onetimeauth_poly1305_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 522 | `crypto_onetimeauth_poly1305_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 523 | `crypto_onetimeauth_poly1305_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 524 | `crypto_onetimeauth_poly1305_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 525 | `crypto_onetimeauth_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 526 | `crypto_onetimeauth_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 527 | `crypto_onetimeauth_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 528 | `crypto_onetimeauth_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 529 | `crypto_pwhash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 530 | `crypto_pwhash_alg_argon2i13` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 531 | `crypto_pwhash_alg_argon2id13` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 532 | `crypto_pwhash_alg_default` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 533 | `crypto_pwhash_argon2i` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 534 | `crypto_pwhash_argon2i_alg_argon2i13` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 535 | `crypto_pwhash_argon2i_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 536 | `crypto_pwhash_argon2i_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 537 | `crypto_pwhash_argon2i_memlimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 538 | `crypto_pwhash_argon2i_memlimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 539 | `crypto_pwhash_argon2i_memlimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 540 | `crypto_pwhash_argon2i_memlimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 541 | `crypto_pwhash_argon2i_memlimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 542 | `crypto_pwhash_argon2i_opslimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 543 | `crypto_pwhash_argon2i_opslimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 544 | `crypto_pwhash_argon2i_opslimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 545 | `crypto_pwhash_argon2i_opslimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 546 | `crypto_pwhash_argon2i_opslimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 547 | `crypto_pwhash_argon2i_passwd_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 548 | `crypto_pwhash_argon2i_passwd_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 549 | `crypto_pwhash_argon2i_saltbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 550 | `crypto_pwhash_argon2i_str` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 551 | `crypto_pwhash_argon2i_str_needs_rehash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 552 | `crypto_pwhash_argon2i_str_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 553 | `crypto_pwhash_argon2i_strbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 554 | `crypto_pwhash_argon2i_strprefix` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 555 | `crypto_pwhash_argon2id` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 556 | `crypto_pwhash_argon2id_alg_argon2id13` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 557 | `crypto_pwhash_argon2id_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 558 | `crypto_pwhash_argon2id_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 559 | `crypto_pwhash_argon2id_memlimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 560 | `crypto_pwhash_argon2id_memlimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 561 | `crypto_pwhash_argon2id_memlimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 562 | `crypto_pwhash_argon2id_memlimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 563 | `crypto_pwhash_argon2id_memlimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 564 | `crypto_pwhash_argon2id_opslimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 565 | `crypto_pwhash_argon2id_opslimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 566 | `crypto_pwhash_argon2id_opslimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 567 | `crypto_pwhash_argon2id_opslimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 568 | `crypto_pwhash_argon2id_opslimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 569 | `crypto_pwhash_argon2id_passwd_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 570 | `crypto_pwhash_argon2id_passwd_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 571 | `crypto_pwhash_argon2id_saltbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 572 | `crypto_pwhash_argon2id_str` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 573 | `crypto_pwhash_argon2id_str_needs_rehash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 574 | `crypto_pwhash_argon2id_str_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 575 | `crypto_pwhash_argon2id_strbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 576 | `crypto_pwhash_argon2id_strprefix` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 577 | `crypto_pwhash_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 578 | `crypto_pwhash_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 579 | `crypto_pwhash_memlimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 580 | `crypto_pwhash_memlimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 581 | `crypto_pwhash_memlimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 582 | `crypto_pwhash_memlimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 583 | `crypto_pwhash_memlimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 584 | `crypto_pwhash_opslimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 585 | `crypto_pwhash_opslimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 586 | `crypto_pwhash_opslimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 587 | `crypto_pwhash_opslimit_moderate` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 588 | `crypto_pwhash_opslimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 589 | `crypto_pwhash_passwd_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 590 | `crypto_pwhash_passwd_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 591 | `crypto_pwhash_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 592 | `crypto_pwhash_saltbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 593 | `crypto_pwhash_scryptsalsa208sha256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 594 | `crypto_pwhash_scryptsalsa208sha256_bytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 595 | `crypto_pwhash_scryptsalsa208sha256_bytes_min` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 596 | `crypto_pwhash_scryptsalsa208sha256_ll` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 597 | `crypto_pwhash_scryptsalsa208sha256_memlimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 598 | `crypto_pwhash_scryptsalsa208sha256_memlimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 599 | `crypto_pwhash_scryptsalsa208sha256_memlimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 600 | `crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 601 | `crypto_pwhash_scryptsalsa208sha256_opslimit_interactive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 602 | `crypto_pwhash_scryptsalsa208sha256_opslimit_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 603 | `crypto_pwhash_scryptsalsa208sha256_opslimit_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 604 | `crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 605 | `crypto_pwhash_scryptsalsa208sha256_passwd_max` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 606 | `crypto_pwhash_scryptsalsa208sha256_passwd_min` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 607 | `crypto_pwhash_scryptsalsa208sha256_saltbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 608 | `crypto_pwhash_scryptsalsa208sha256_str` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 609 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 610 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 611 | `crypto_pwhash_scryptsalsa208sha256_strbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 612 | `crypto_pwhash_scryptsalsa208sha256_strprefix` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 613 | `crypto_pwhash_str` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 614 | `crypto_pwhash_str_alg` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 615 | `crypto_pwhash_str_needs_rehash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 616 | `crypto_pwhash_str_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 617 | `crypto_pwhash_strbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 618 | `crypto_pwhash_strprefix` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 619 | `crypto_scalarmult` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 620 | `crypto_scalarmult_base` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 621 | `crypto_scalarmult_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 622 | `crypto_scalarmult_curve25519` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 623 | `crypto_scalarmult_curve25519_base` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 624 | `crypto_scalarmult_curve25519_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 625 | `crypto_scalarmult_curve25519_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 626 | `crypto_scalarmult_ed25519` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 627 | `crypto_scalarmult_ed25519_base` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 628 | `crypto_scalarmult_ed25519_base_noclamp` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 629 | `crypto_scalarmult_ed25519_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 630 | `crypto_scalarmult_ed25519_noclamp` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 631 | `crypto_scalarmult_ed25519_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 632 | `crypto_scalarmult_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 633 | `crypto_scalarmult_ristretto255` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 634 | `crypto_scalarmult_ristretto255_base` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 635 | `crypto_scalarmult_ristretto255_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 636 | `crypto_scalarmult_ristretto255_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 637 | `crypto_scalarmult_scalarbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 638 | `crypto_secretbox` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 639 | `crypto_secretbox_boxzerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 640 | `crypto_secretbox_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 641 | `crypto_secretbox_easy` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 642 | `crypto_secretbox_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 643 | `crypto_secretbox_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 644 | `crypto_secretbox_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 645 | `crypto_secretbox_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 646 | `crypto_secretbox_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 647 | `crypto_secretbox_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 648 | `crypto_secretbox_open_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 649 | `crypto_secretbox_open_easy` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 650 | `crypto_secretbox_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 651 | `crypto_secretbox_xchacha20poly1305_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 652 | `crypto_secretbox_xchacha20poly1305_easy` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 653 | `crypto_secretbox_xchacha20poly1305_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 654 | `crypto_secretbox_xchacha20poly1305_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 655 | `crypto_secretbox_xchacha20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 656 | `crypto_secretbox_xchacha20poly1305_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 657 | `crypto_secretbox_xchacha20poly1305_open_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 658 | `crypto_secretbox_xchacha20poly1305_open_easy` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 659 | `crypto_secretbox_xsalsa20poly1305` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 660 | `crypto_secretbox_xsalsa20poly1305_boxzerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 661 | `crypto_secretbox_xsalsa20poly1305_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 662 | `crypto_secretbox_xsalsa20poly1305_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 663 | `crypto_secretbox_xsalsa20poly1305_macbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 664 | `crypto_secretbox_xsalsa20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 665 | `crypto_secretbox_xsalsa20poly1305_noncebytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 666 | `crypto_secretbox_xsalsa20poly1305_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 667 | `crypto_secretbox_xsalsa20poly1305_zerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 668 | `crypto_secretbox_zerobytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 669 | `crypto_secretstream_xchacha20poly1305_abytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 670 | `crypto_secretstream_xchacha20poly1305_headerbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 671 | `crypto_secretstream_xchacha20poly1305_init_pull` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 672 | `crypto_secretstream_xchacha20poly1305_init_push` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 673 | `crypto_secretstream_xchacha20poly1305_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 674 | `crypto_secretstream_xchacha20poly1305_keygen` | low-level nm export; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 675 | `crypto_secretstream_xchacha20poly1305_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 676 | `crypto_secretstream_xchacha20poly1305_pull` | low-level nm export; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 677 | `crypto_secretstream_xchacha20poly1305_push` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 678 | `crypto_secretstream_xchacha20poly1305_rekey` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 679 | `crypto_secretstream_xchacha20poly1305_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 680 | `crypto_secretstream_xchacha20poly1305_tag_final` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 681 | `crypto_secretstream_xchacha20poly1305_tag_message` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 682 | `crypto_secretstream_xchacha20poly1305_tag_push` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 683 | `crypto_secretstream_xchacha20poly1305_tag_rekey` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 684 | `crypto_shorthash` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 685 | `crypto_shorthash_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 686 | `crypto_shorthash_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 687 | `crypto_shorthash_keygen` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 688 | `crypto_shorthash_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 689 | `crypto_shorthash_siphash24` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 690 | `crypto_shorthash_siphash24_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 691 | `crypto_shorthash_siphash24_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 692 | `crypto_shorthash_siphashx24` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 693 | `crypto_shorthash_siphashx24_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 694 | `crypto_shorthash_siphashx24_keybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 695 | `crypto_sign` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 696 | `crypto_sign_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 697 | `crypto_sign_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 698 | `crypto_sign_ed25519` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 699 | `crypto_sign_ed25519_bytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 700 | `crypto_sign_ed25519_detached` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 701 | `crypto_sign_ed25519_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 702 | `crypto_sign_ed25519_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 703 | `crypto_sign_ed25519_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 704 | `crypto_sign_ed25519_pk_to_curve25519` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 705 | `crypto_sign_ed25519_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 706 | `crypto_sign_ed25519_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 707 | `crypto_sign_ed25519_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 708 | `crypto_sign_ed25519_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 709 | `crypto_sign_ed25519_sk_to_curve25519` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 710 | `crypto_sign_ed25519_sk_to_pk` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 711 | `crypto_sign_ed25519_sk_to_seed` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 712 | `crypto_sign_ed25519_verify_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 713 | `crypto_sign_ed25519ph_final_create` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 714 | `crypto_sign_ed25519ph_final_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 715 | `crypto_sign_ed25519ph_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 716 | `crypto_sign_ed25519ph_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 717 | `crypto_sign_ed25519ph_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 718 | `crypto_sign_final_create` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 719 | `crypto_sign_final_verify` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 720 | `crypto_sign_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 721 | `crypto_sign_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 722 | `crypto_sign_messagebytes_max` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 723 | `crypto_sign_open` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 724 | `crypto_sign_primitive` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 725 | `crypto_sign_publickeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 726 | `crypto_sign_secretkeybytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 727 | `crypto_sign_seed_keypair` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 728 | `crypto_sign_seedbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 729 | `crypto_sign_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 730 | `crypto_sign_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 731 | `crypto_sign_verify_detached` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 732 | `crypto_stream` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 733 | `crypto_stream_chacha20` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 734 | `crypto_stream_chacha20_ietf` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 735 | `crypto_stream_chacha20_ietf_ext` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 736 | `crypto_stream_chacha20_ietf_ext_xor_ic` | low-level nm export; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 737 | `crypto_stream_chacha20_ietf_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 738 | `crypto_stream_chacha20_ietf_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 739 | `crypto_stream_chacha20_ietf_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 740 | `crypto_stream_chacha20_ietf_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 741 | `crypto_stream_chacha20_ietf_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 742 | `crypto_stream_chacha20_ietf_xor_ic` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 743 | `crypto_stream_chacha20_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 744 | `crypto_stream_chacha20_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 745 | `crypto_stream_chacha20_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 746 | `crypto_stream_chacha20_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 747 | `crypto_stream_chacha20_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 748 | `crypto_stream_chacha20_xor_ic` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 749 | `crypto_stream_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 750 | `crypto_stream_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 751 | `crypto_stream_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 752 | `crypto_stream_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 753 | `crypto_stream_primitive` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 754 | `crypto_stream_salsa20` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 755 | `crypto_stream_salsa2012` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 756 | `crypto_stream_salsa2012_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 757 | `crypto_stream_salsa2012_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 758 | `crypto_stream_salsa2012_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 759 | `crypto_stream_salsa2012_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 760 | `crypto_stream_salsa2012_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 761 | `crypto_stream_salsa208` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 762 | `crypto_stream_salsa208_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 763 | `crypto_stream_salsa208_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 764 | `crypto_stream_salsa208_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 765 | `crypto_stream_salsa208_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 766 | `crypto_stream_salsa208_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 767 | `crypto_stream_salsa20_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 768 | `crypto_stream_salsa20_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 769 | `crypto_stream_salsa20_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 770 | `crypto_stream_salsa20_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 771 | `crypto_stream_salsa20_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 772 | `crypto_stream_salsa20_xor_ic` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 773 | `crypto_stream_xchacha20` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 774 | `crypto_stream_xchacha20_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 775 | `crypto_stream_xchacha20_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 776 | `crypto_stream_xchacha20_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 777 | `crypto_stream_xchacha20_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 778 | `crypto_stream_xchacha20_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 779 | `crypto_stream_xchacha20_xor_ic` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 780 | `crypto_stream_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 781 | `crypto_stream_xsalsa20` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 782 | `crypto_stream_xsalsa20_keybytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 783 | `crypto_stream_xsalsa20_keygen` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 784 | `crypto_stream_xsalsa20_messagebytes_max` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 785 | `crypto_stream_xsalsa20_noncebytes` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 786 | `crypto_stream_xsalsa20_xor` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 787 | `crypto_stream_xsalsa20_xor_ic` | public header; deterministic randomized input; empty, one-byte, block-edge, and many-byte message/AD; combined/detached where exposed | [x] |
| 788 | `crypto_verify_16` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 789 | `crypto_verify_16_bytes` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 790 | `crypto_verify_32` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 791 | `crypto_verify_32_bytes` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 792 | `crypto_verify_64` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 793 | `crypto_verify_64_bytes` | public header; valid and tampered input; empty, one-byte, block-edge, and many-byte message/AD; nullable documented outputs | [x] |
| 794 | `crypto_xof_shake128` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 795 | `crypto_xof_shake128_blockbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 796 | `crypto_xof_shake128_domain_standard` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 797 | `crypto_xof_shake128_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 798 | `crypto_xof_shake128_init_with_domain` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 799 | `crypto_xof_shake128_squeeze` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 800 | `crypto_xof_shake128_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 801 | `crypto_xof_shake128_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 802 | `crypto_xof_shake256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 803 | `crypto_xof_shake256_blockbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 804 | `crypto_xof_shake256_domain_standard` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 805 | `crypto_xof_shake256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 806 | `crypto_xof_shake256_init_with_domain` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 807 | `crypto_xof_shake256_squeeze` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 808 | `crypto_xof_shake256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 809 | `crypto_xof_shake256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 810 | `crypto_xof_turboshake128` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 811 | `crypto_xof_turboshake128_blockbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 812 | `crypto_xof_turboshake128_domain_standard` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 813 | `crypto_xof_turboshake128_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 814 | `crypto_xof_turboshake128_init_with_domain` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 815 | `crypto_xof_turboshake128_squeeze` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 816 | `crypto_xof_turboshake128_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 817 | `crypto_xof_turboshake128_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 818 | `crypto_xof_turboshake256` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 819 | `crypto_xof_turboshake256_blockbytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 820 | `crypto_xof_turboshake256_domain_standard` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 821 | `crypto_xof_turboshake256_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 822 | `crypto_xof_turboshake256_init_with_domain` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 823 | `crypto_xof_turboshake256_squeeze` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 824 | `crypto_xof_turboshake256_statebytes` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 825 | `crypto_xof_turboshake256_update` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 826 | `randombytes` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 827 | `randombytes_buf` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 828 | `randombytes_buf_deterministic` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 829 | `randombytes_close` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 830 | `randombytes_implementation_name` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 831 | `randombytes_random` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 832 | `randombytes_seedbytes` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 833 | `randombytes_set_implementation` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 834 | `randombytes_stir` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 835 | `randombytes_uniform` | public header; seeded and unseeded forms where exposed; zero, one, and many output elements | [x] |
| 836 | `sodium_add` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 837 | `sodium_allocarray` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 838 | `sodium_base642bin` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 839 | `sodium_base64_encoded_len` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 840 | `sodium_bin2base64` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 841 | `sodium_bin2hex` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 842 | `sodium_bin2ip` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 843 | `sodium_compare` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 844 | `sodium_crit_enter` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 845 | `sodium_crit_leave` | low-level nm export; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 846 | `sodium_free` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 847 | `sodium_hex2bin` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 848 | `sodium_increment` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 849 | `sodium_init` | public header; streaming state; empty, one-byte, block-edge, and many-byte chunks; one-shot equivalence | [x] |
| 850 | `sodium_ip2bin` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 851 | `sodium_is_zero` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 852 | `sodium_library_minimal` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 853 | `sodium_library_version_major` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 854 | `sodium_library_version_minor` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 855 | `sodium_malloc` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 856 | `sodium_memcmp` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 857 | `sodium_memzero` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 858 | `sodium_misuse` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 859 | `sodium_mlock` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 860 | `sodium_mprotect_noaccess` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 861 | `sodium_mprotect_readonly` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 862 | `sodium_mprotect_readwrite` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 863 | `sodium_munlock` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 864 | `sodium_pad` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 865 | `sodium_runtime_has_aesni` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 866 | `sodium_runtime_has_armcrypto` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 867 | `sodium_runtime_has_avx` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 868 | `sodium_runtime_has_avx2` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 869 | `sodium_runtime_has_avx512f` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 870 | `sodium_runtime_has_neon` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 871 | `sodium_runtime_has_pclmul` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 872 | `sodium_runtime_has_rdrand` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 873 | `sodium_runtime_has_sse2` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 874 | `sodium_runtime_has_sse3` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 875 | `sodium_runtime_has_sse41` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 876 | `sodium_runtime_has_ssse3` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
| 877 | `sodium_set_misuse_handler` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 878 | `sodium_stackzero` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 879 | `sodium_sub` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 880 | `sodium_unpad` | public header; direct exported entry point; valid source preconditions; zero, one, boundary, and many element shapes where length-bearing | [x] |
| 881 | `sodium_version_string` | public header; no-input metadata/runtime accessor; exact scalar or NUL-terminated bytes | [x] |
