# Exported symbols per C source file

The C .so exports 890 symbols total. Each block lists the FINAL linker names
that the given C file must produce (T = function, D = data object, W = weak function).

## crypto_aead/aegis128l/aead_aegis128l.c
- _crypto_aead_aegis128l_pick_best_implementation  (T)
- crypto_aead_aegis128l_abytes  (T)
- crypto_aead_aegis128l_decrypt  (T)
- crypto_aead_aegis128l_decrypt_detached  (T)
- crypto_aead_aegis128l_encrypt  (T)
- crypto_aead_aegis128l_encrypt_detached  (T)
- crypto_aead_aegis128l_keybytes  (T)
- crypto_aead_aegis128l_keygen  (T)
- crypto_aead_aegis128l_messagebytes_max  (T)
- crypto_aead_aegis128l_npubbytes  (T)
- crypto_aead_aegis128l_nsecbytes  (T)

## crypto_aead/aegis128l/aegis128l_soft.c
- aegis128l_soft_implementation  (D)

## crypto_aead/aegis256/aead_aegis256.c
- _crypto_aead_aegis256_pick_best_implementation  (T)
- crypto_aead_aegis256_abytes  (T)
- crypto_aead_aegis256_decrypt  (T)
- crypto_aead_aegis256_decrypt_detached  (T)
- crypto_aead_aegis256_encrypt  (T)
- crypto_aead_aegis256_encrypt_detached  (T)
- crypto_aead_aegis256_keybytes  (T)
- crypto_aead_aegis256_keygen  (T)
- crypto_aead_aegis256_messagebytes_max  (T)
- crypto_aead_aegis256_npubbytes  (T)
- crypto_aead_aegis256_nsecbytes  (T)

## crypto_aead/aegis256/aegis256_soft.c
- aegis256_soft_implementation  (D)

## crypto_aead/aes256gcm/aead_aes256gcm.c
- crypto_aead_aes256gcm_abytes  (T)
- crypto_aead_aes256gcm_beforenm  (T)
- crypto_aead_aes256gcm_decrypt  (T)
- crypto_aead_aes256gcm_decrypt_afternm  (T)
- crypto_aead_aes256gcm_decrypt_detached  (T)
- crypto_aead_aes256gcm_decrypt_detached_afternm  (T)
- crypto_aead_aes256gcm_encrypt  (T)
- crypto_aead_aes256gcm_encrypt_afternm  (T)
- crypto_aead_aes256gcm_encrypt_detached  (T)
- crypto_aead_aes256gcm_encrypt_detached_afternm  (T)
- crypto_aead_aes256gcm_is_available  (T)
- crypto_aead_aes256gcm_keybytes  (T)
- crypto_aead_aes256gcm_keygen  (T)
- crypto_aead_aes256gcm_messagebytes_max  (T)
- crypto_aead_aes256gcm_npubbytes  (T)
- crypto_aead_aes256gcm_nsecbytes  (T)
- crypto_aead_aes256gcm_statebytes  (T)

## crypto_aead/chacha20poly1305/aead_chacha20poly1305.c
- crypto_aead_chacha20poly1305_abytes  (T)
- crypto_aead_chacha20poly1305_decrypt  (T)
- crypto_aead_chacha20poly1305_decrypt_detached  (T)
- crypto_aead_chacha20poly1305_encrypt  (T)
- crypto_aead_chacha20poly1305_encrypt_detached  (T)
- crypto_aead_chacha20poly1305_ietf_abytes  (T)
- crypto_aead_chacha20poly1305_ietf_decrypt  (T)
- crypto_aead_chacha20poly1305_ietf_decrypt_detached  (T)
- crypto_aead_chacha20poly1305_ietf_encrypt  (T)
- crypto_aead_chacha20poly1305_ietf_encrypt_detached  (T)
- crypto_aead_chacha20poly1305_ietf_keybytes  (T)
- crypto_aead_chacha20poly1305_ietf_keygen  (T)
- crypto_aead_chacha20poly1305_ietf_messagebytes_max  (T)
- crypto_aead_chacha20poly1305_ietf_npubbytes  (T)
- crypto_aead_chacha20poly1305_ietf_nsecbytes  (T)
- crypto_aead_chacha20poly1305_keybytes  (T)
- crypto_aead_chacha20poly1305_keygen  (T)
- crypto_aead_chacha20poly1305_messagebytes_max  (T)
- crypto_aead_chacha20poly1305_npubbytes  (T)
- crypto_aead_chacha20poly1305_nsecbytes  (T)

## crypto_aead/xchacha20poly1305/aead_xchacha20poly1305.c
- crypto_aead_xchacha20poly1305_ietf_abytes  (T)
- crypto_aead_xchacha20poly1305_ietf_decrypt  (T)
- crypto_aead_xchacha20poly1305_ietf_decrypt_detached  (T)
- crypto_aead_xchacha20poly1305_ietf_encrypt  (T)
- crypto_aead_xchacha20poly1305_ietf_encrypt_detached  (T)
- crypto_aead_xchacha20poly1305_ietf_keybytes  (T)
- crypto_aead_xchacha20poly1305_ietf_keygen  (T)
- crypto_aead_xchacha20poly1305_ietf_messagebytes_max  (T)
- crypto_aead_xchacha20poly1305_ietf_npubbytes  (T)
- crypto_aead_xchacha20poly1305_ietf_nsecbytes  (T)

## crypto_auth/crypto_auth.c
- crypto_auth  (T)
- crypto_auth_bytes  (T)
- crypto_auth_keybytes  (T)
- crypto_auth_keygen  (T)
- crypto_auth_primitive  (T)
- crypto_auth_verify  (T)

## crypto_auth/hmacsha256/auth_hmacsha256.c
- crypto_auth_hmacsha256  (T)
- crypto_auth_hmacsha256_bytes  (T)
- crypto_auth_hmacsha256_final  (T)
- crypto_auth_hmacsha256_init  (T)
- crypto_auth_hmacsha256_keybytes  (T)
- crypto_auth_hmacsha256_keygen  (T)
- crypto_auth_hmacsha256_statebytes  (T)
- crypto_auth_hmacsha256_update  (T)
- crypto_auth_hmacsha256_verify  (T)

## crypto_auth/hmacsha512/auth_hmacsha512.c
- crypto_auth_hmacsha512  (T)
- crypto_auth_hmacsha512_bytes  (T)
- crypto_auth_hmacsha512_final  (T)
- crypto_auth_hmacsha512_init  (T)
- crypto_auth_hmacsha512_keybytes  (T)
- crypto_auth_hmacsha512_keygen  (T)
- crypto_auth_hmacsha512_statebytes  (T)
- crypto_auth_hmacsha512_update  (T)
- crypto_auth_hmacsha512_verify  (T)

## crypto_auth/hmacsha512256/auth_hmacsha512256.c
- crypto_auth_hmacsha512256  (T)
- crypto_auth_hmacsha512256_bytes  (T)
- crypto_auth_hmacsha512256_final  (T)
- crypto_auth_hmacsha512256_init  (T)
- crypto_auth_hmacsha512256_keybytes  (T)
- crypto_auth_hmacsha512256_keygen  (T)
- crypto_auth_hmacsha512256_statebytes  (T)
- crypto_auth_hmacsha512256_update  (T)
- crypto_auth_hmacsha512256_verify  (T)

## crypto_box/crypto_box.c
- crypto_box  (T)
- crypto_box_afternm  (T)
- crypto_box_beforenm  (T)
- crypto_box_beforenmbytes  (T)
- crypto_box_boxzerobytes  (T)
- crypto_box_keypair  (T)
- crypto_box_macbytes  (T)
- crypto_box_messagebytes_max  (T)
- crypto_box_noncebytes  (T)
- crypto_box_open  (T)
- crypto_box_open_afternm  (T)
- crypto_box_primitive  (T)
- crypto_box_publickeybytes  (T)
- crypto_box_secretkeybytes  (T)
- crypto_box_seed_keypair  (T)
- crypto_box_seedbytes  (T)
- crypto_box_zerobytes  (T)

## crypto_box/crypto_box_easy.c
- crypto_box_detached  (T)
- crypto_box_detached_afternm  (T)
- crypto_box_easy  (T)
- crypto_box_easy_afternm  (T)
- crypto_box_open_detached  (T)
- crypto_box_open_detached_afternm  (T)
- crypto_box_open_easy  (T)
- crypto_box_open_easy_afternm  (T)

## crypto_box/crypto_box_seal.c
- crypto_box_seal  (T)
- crypto_box_seal_open  (T)
- crypto_box_sealbytes  (T)

## crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c
- crypto_box_curve25519xchacha20poly1305_beforenm  (T)
- crypto_box_curve25519xchacha20poly1305_beforenmbytes  (T)
- crypto_box_curve25519xchacha20poly1305_detached  (T)
- crypto_box_curve25519xchacha20poly1305_detached_afternm  (T)
- crypto_box_curve25519xchacha20poly1305_easy  (T)
- crypto_box_curve25519xchacha20poly1305_easy_afternm  (T)
- crypto_box_curve25519xchacha20poly1305_keypair  (T)
- crypto_box_curve25519xchacha20poly1305_macbytes  (T)
- crypto_box_curve25519xchacha20poly1305_messagebytes_max  (T)
- crypto_box_curve25519xchacha20poly1305_noncebytes  (T)
- crypto_box_curve25519xchacha20poly1305_open_detached  (T)
- crypto_box_curve25519xchacha20poly1305_open_detached_afternm  (T)
- crypto_box_curve25519xchacha20poly1305_open_easy  (T)
- crypto_box_curve25519xchacha20poly1305_open_easy_afternm  (T)
- crypto_box_curve25519xchacha20poly1305_publickeybytes  (T)
- crypto_box_curve25519xchacha20poly1305_secretkeybytes  (T)
- crypto_box_curve25519xchacha20poly1305_seed_keypair  (T)
- crypto_box_curve25519xchacha20poly1305_seedbytes  (T)

## crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c
- crypto_box_curve25519xchacha20poly1305_seal  (T)
- crypto_box_curve25519xchacha20poly1305_seal_open  (T)
- crypto_box_curve25519xchacha20poly1305_sealbytes  (T)

## crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c
- crypto_box_curve25519xsalsa20poly1305  (T)
- crypto_box_curve25519xsalsa20poly1305_afternm  (T)
- crypto_box_curve25519xsalsa20poly1305_beforenm  (T)
- crypto_box_curve25519xsalsa20poly1305_beforenmbytes  (T)
- crypto_box_curve25519xsalsa20poly1305_boxzerobytes  (T)
- crypto_box_curve25519xsalsa20poly1305_keypair  (T)
- crypto_box_curve25519xsalsa20poly1305_macbytes  (T)
- crypto_box_curve25519xsalsa20poly1305_messagebytes_max  (T)
- crypto_box_curve25519xsalsa20poly1305_noncebytes  (T)
- crypto_box_curve25519xsalsa20poly1305_open  (T)
- crypto_box_curve25519xsalsa20poly1305_open_afternm  (T)
- crypto_box_curve25519xsalsa20poly1305_publickeybytes  (T)
- crypto_box_curve25519xsalsa20poly1305_secretkeybytes  (T)
- crypto_box_curve25519xsalsa20poly1305_seed_keypair  (T)
- crypto_box_curve25519xsalsa20poly1305_seedbytes  (T)
- crypto_box_curve25519xsalsa20poly1305_zerobytes  (T)

## crypto_core/ed25519/core_ed25519.c
- crypto_core_ed25519_add  (T)
- crypto_core_ed25519_bytes  (T)
- crypto_core_ed25519_from_string  (T)
- crypto_core_ed25519_from_string_nu  (T)
- crypto_core_ed25519_hashbytes  (T)
- crypto_core_ed25519_is_valid_point  (T)
- crypto_core_ed25519_nonreducedscalarbytes  (T)
- crypto_core_ed25519_random  (T)
- crypto_core_ed25519_scalar_add  (T)
- crypto_core_ed25519_scalar_complement  (T)
- crypto_core_ed25519_scalar_from_string  (T)
- crypto_core_ed25519_scalar_invert  (T)
- crypto_core_ed25519_scalar_is_canonical  (T)
- crypto_core_ed25519_scalar_mul  (T)
- crypto_core_ed25519_scalar_negate  (T)
- crypto_core_ed25519_scalar_random  (T)
- crypto_core_ed25519_scalar_reduce  (T)
- crypto_core_ed25519_scalar_sub  (T)
- crypto_core_ed25519_scalarbytes  (T)
- crypto_core_ed25519_sub  (T)
- crypto_core_ed25519_uniformbytes  (T)

## crypto_core/ed25519/core_h2c.c
- _sodium_core_h2c_string_to_hash  (T)

## crypto_core/ed25519/core_ristretto255.c
- crypto_core_ristretto255_add  (T)
- crypto_core_ristretto255_bytes  (T)
- crypto_core_ristretto255_from_hash  (T)
- crypto_core_ristretto255_from_string  (T)
- crypto_core_ristretto255_hashbytes  (T)
- crypto_core_ristretto255_is_valid_point  (T)
- crypto_core_ristretto255_nonreducedscalarbytes  (T)
- crypto_core_ristretto255_random  (T)
- crypto_core_ristretto255_scalar_add  (T)
- crypto_core_ristretto255_scalar_complement  (T)
- crypto_core_ristretto255_scalar_from_string  (T)
- crypto_core_ristretto255_scalar_invert  (T)
- crypto_core_ristretto255_scalar_is_canonical  (T)
- crypto_core_ristretto255_scalar_mul  (T)
- crypto_core_ristretto255_scalar_negate  (T)
- crypto_core_ristretto255_scalar_random  (T)
- crypto_core_ristretto255_scalar_reduce  (T)
- crypto_core_ristretto255_scalar_sub  (T)
- crypto_core_ristretto255_scalarbytes  (T)
- crypto_core_ristretto255_sub  (T)

## crypto_core/ed25519/ref10/ed25519_ref10.c
- _sodium_fe25519_frombytes  (T)
- _sodium_fe25519_invert  (T)
- _sodium_fe25519_tobytes  (T)
- _sodium_ge25519_clear_cofactor  (T)
- _sodium_ge25519_double_scalarmult_vartime  (T)
- _sodium_ge25519_from_hash  (T)
- _sodium_ge25519_from_uniform  (T)
- _sodium_ge25519_frombytes  (T)
- _sodium_ge25519_frombytes_negate_vartime  (T)
- _sodium_ge25519_has_small_order  (T)
- _sodium_ge25519_is_canonical  (T)
- _sodium_ge25519_is_on_curve  (T)
- _sodium_ge25519_is_on_main_subgroup  (T)
- _sodium_ge25519_p1p1_to_p2  (T)
- _sodium_ge25519_p1p1_to_p3  (T)
- _sodium_ge25519_p2_to_p3  (T)
- _sodium_ge25519_p3_add  (T)
- _sodium_ge25519_p3_sub  (T)
- _sodium_ge25519_p3_tobytes  (T)
- _sodium_ge25519_scalarmult  (T)
- _sodium_ge25519_scalarmult_base  (T)
- _sodium_ge25519_tobytes  (T)
- _sodium_ristretto255_from_hash  (T)
- _sodium_ristretto255_frombytes  (T)
- _sodium_ristretto255_p3_tobytes  (T)
- _sodium_sc25519_invert  (T)
- _sodium_sc25519_is_canonical  (T)
- _sodium_sc25519_mul  (T)
- _sodium_sc25519_muladd  (T)
- _sodium_sc25519_reduce  (T)

## crypto_core/hchacha20/core_hchacha20.c
- crypto_core_hchacha20  (T)
- crypto_core_hchacha20_constbytes  (T)
- crypto_core_hchacha20_inputbytes  (T)
- crypto_core_hchacha20_keybytes  (T)
- crypto_core_hchacha20_outputbytes  (T)

## crypto_core/hsalsa20/core_hsalsa20.c
- crypto_core_hsalsa20_constbytes  (T)
- crypto_core_hsalsa20_inputbytes  (T)
- crypto_core_hsalsa20_keybytes  (T)
- crypto_core_hsalsa20_outputbytes  (T)

## crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c
- crypto_core_hsalsa20  (T)

## crypto_core/keccak1600/keccak1600.c
- crypto_core_keccak1600_extract_bytes  (T)
- crypto_core_keccak1600_init  (T)
- crypto_core_keccak1600_permute_12  (T)
- crypto_core_keccak1600_permute_24  (T)
- crypto_core_keccak1600_statebytes  (T)
- crypto_core_keccak1600_xor_bytes  (T)

## crypto_core/keccak1600/ref/keccak1600_ref.c
- _sodium_keccak1600_ref_extract_bytes  (T)
- _sodium_keccak1600_ref_init  (T)
- _sodium_keccak1600_ref_permute_12  (T)
- _sodium_keccak1600_ref_permute_24  (T)
- _sodium_keccak1600_ref_xor_bytes  (T)

## crypto_core/salsa/ref/core_salsa_ref.c
- crypto_core_salsa20  (T)
- crypto_core_salsa2012  (T)
- crypto_core_salsa2012_constbytes  (T)
- crypto_core_salsa2012_inputbytes  (T)
- crypto_core_salsa2012_keybytes  (T)
- crypto_core_salsa2012_outputbytes  (T)
- crypto_core_salsa208  (T)
- crypto_core_salsa208_constbytes  (T)
- crypto_core_salsa208_inputbytes  (T)
- crypto_core_salsa208_keybytes  (T)
- crypto_core_salsa208_outputbytes  (T)
- crypto_core_salsa20_constbytes  (T)
- crypto_core_salsa20_inputbytes  (T)
- crypto_core_salsa20_keybytes  (T)
- crypto_core_salsa20_outputbytes  (T)

## crypto_core/softaes/softaes.c
- _sodium_softaes_block_decrypt  (T)
- _sodium_softaes_block_decryptlast  (T)
- _sodium_softaes_block_encrypt  (T)
- _sodium_softaes_block_encryptlast  (T)
- _sodium_softaes_expand_key128  (T)
- _sodium_softaes_expand_key256  (T)
- _sodium_softaes_inv_mix_columns  (T)
- _sodium_softaes_invert_key_schedule128  (T)
- _sodium_softaes_invert_key_schedule256  (T)

## crypto_generichash/blake2b/generichash_blake2.c
- crypto_generichash_blake2b_bytes  (T)
- crypto_generichash_blake2b_bytes_max  (T)
- crypto_generichash_blake2b_bytes_min  (T)
- crypto_generichash_blake2b_keybytes  (T)
- crypto_generichash_blake2b_keybytes_max  (T)
- crypto_generichash_blake2b_keybytes_min  (T)
- crypto_generichash_blake2b_keygen  (T)
- crypto_generichash_blake2b_personalbytes  (T)
- crypto_generichash_blake2b_saltbytes  (T)
- crypto_generichash_blake2b_statebytes  (T)

## crypto_generichash/blake2b/ref/blake2b-compress-ref.c
- _sodium_blake2b_compress_ref  (T)

## crypto_generichash/blake2b/ref/blake2b-ref.c
- _sodium_blake2b  (T)
- _sodium_blake2b_final  (T)
- _sodium_blake2b_init  (T)
- _sodium_blake2b_init_key  (T)
- _sodium_blake2b_init_key_salt_personal  (T)
- _sodium_blake2b_init_param  (T)
- _sodium_blake2b_init_salt_personal  (T)
- _sodium_blake2b_pick_best_implementation  (T)
- _sodium_blake2b_salt_personal  (T)
- _sodium_blake2b_update  (T)

## crypto_generichash/blake2b/ref/generichash_blake2b.c
- _crypto_generichash_blake2b_pick_best_implementation  (T)
- crypto_generichash_blake2b  (T)
- crypto_generichash_blake2b_final  (T)
- crypto_generichash_blake2b_init  (T)
- crypto_generichash_blake2b_init_salt_personal  (T)
- crypto_generichash_blake2b_salt_personal  (T)
- crypto_generichash_blake2b_update  (T)

## crypto_generichash/crypto_generichash.c
- crypto_generichash  (T)
- crypto_generichash_bytes  (T)
- crypto_generichash_bytes_max  (T)
- crypto_generichash_bytes_min  (T)
- crypto_generichash_final  (T)
- crypto_generichash_init  (T)
- crypto_generichash_keybytes  (T)
- crypto_generichash_keybytes_max  (T)
- crypto_generichash_keybytes_min  (T)
- crypto_generichash_keygen  (T)
- crypto_generichash_primitive  (T)
- crypto_generichash_statebytes  (T)
- crypto_generichash_update  (T)

## crypto_hash/crypto_hash.c
- crypto_hash  (T)
- crypto_hash_bytes  (T)
- crypto_hash_primitive  (T)

## crypto_hash/sha256/cp/hash_sha256_cp.c
- crypto_hash_sha256  (T)
- crypto_hash_sha256_final  (T)
- crypto_hash_sha256_init  (T)
- crypto_hash_sha256_update  (T)

## crypto_hash/sha256/hash_sha256.c
- crypto_hash_sha256_bytes  (T)
- crypto_hash_sha256_statebytes  (T)

## crypto_hash/sha3/hash_sha3.c
- crypto_hash_sha3256  (T)
- crypto_hash_sha3256_bytes  (T)
- crypto_hash_sha3256_final  (T)
- crypto_hash_sha3256_init  (T)
- crypto_hash_sha3256_statebytes  (T)
- crypto_hash_sha3256_update  (T)
- crypto_hash_sha3512  (T)
- crypto_hash_sha3512_bytes  (T)
- crypto_hash_sha3512_final  (T)
- crypto_hash_sha3512_init  (T)
- crypto_hash_sha3512_statebytes  (T)
- crypto_hash_sha3512_update  (T)

## crypto_hash/sha512/cp/hash_sha512_cp.c
- crypto_hash_sha512  (T)
- crypto_hash_sha512_final  (T)
- crypto_hash_sha512_init  (T)
- crypto_hash_sha512_update  (T)

## crypto_hash/sha512/hash_sha512.c
- crypto_hash_sha512_bytes  (T)
- crypto_hash_sha512_statebytes  (T)

## crypto_ipcrypt/crypto_ipcrypt.c
- _crypto_ipcrypt_pick_best_implementation  (T)
- crypto_ipcrypt_bytes  (T)
- crypto_ipcrypt_decrypt  (T)
- crypto_ipcrypt_encrypt  (T)
- crypto_ipcrypt_keybytes  (T)
- crypto_ipcrypt_keygen  (T)
- crypto_ipcrypt_nd_decrypt  (T)
- crypto_ipcrypt_nd_encrypt  (T)
- crypto_ipcrypt_nd_inputbytes  (T)
- crypto_ipcrypt_nd_keybytes  (T)
- crypto_ipcrypt_nd_keygen  (T)
- crypto_ipcrypt_nd_outputbytes  (T)
- crypto_ipcrypt_nd_tweakbytes  (T)
- crypto_ipcrypt_ndx_decrypt  (T)
- crypto_ipcrypt_ndx_encrypt  (T)
- crypto_ipcrypt_ndx_inputbytes  (T)
- crypto_ipcrypt_ndx_keybytes  (T)
- crypto_ipcrypt_ndx_keygen  (T)
- crypto_ipcrypt_ndx_outputbytes  (T)
- crypto_ipcrypt_ndx_tweakbytes  (T)
- crypto_ipcrypt_pfx_bytes  (T)
- crypto_ipcrypt_pfx_decrypt  (T)
- crypto_ipcrypt_pfx_encrypt  (T)
- crypto_ipcrypt_pfx_keybytes  (T)
- crypto_ipcrypt_pfx_keygen  (T)

## crypto_ipcrypt/ipcrypt_soft.c
- ipcrypt_soft_implementation  (D)

## crypto_kdf/blake2b/kdf_blake2b.c
- crypto_kdf_blake2b_bytes_max  (T)
- crypto_kdf_blake2b_bytes_min  (T)
- crypto_kdf_blake2b_contextbytes  (T)
- crypto_kdf_blake2b_derive_from_key  (T)
- crypto_kdf_blake2b_keybytes  (T)

## crypto_kdf/crypto_kdf.c
- crypto_kdf_bytes_max  (T)
- crypto_kdf_bytes_min  (T)
- crypto_kdf_contextbytes  (T)
- crypto_kdf_derive_from_key  (T)
- crypto_kdf_keybytes  (T)
- crypto_kdf_keygen  (T)
- crypto_kdf_primitive  (T)

## crypto_kdf/hkdf/kdf_hkdf_sha256.c
- crypto_kdf_hkdf_sha256_bytes_max  (T)
- crypto_kdf_hkdf_sha256_bytes_min  (T)
- crypto_kdf_hkdf_sha256_expand  (T)
- crypto_kdf_hkdf_sha256_extract  (T)
- crypto_kdf_hkdf_sha256_extract_final  (T)
- crypto_kdf_hkdf_sha256_extract_init  (T)
- crypto_kdf_hkdf_sha256_extract_update  (T)
- crypto_kdf_hkdf_sha256_keybytes  (T)
- crypto_kdf_hkdf_sha256_keygen  (T)
- crypto_kdf_hkdf_sha256_statebytes  (T)

## crypto_kdf/hkdf/kdf_hkdf_sha512.c
- crypto_kdf_hkdf_sha512_bytes_max  (T)
- crypto_kdf_hkdf_sha512_bytes_min  (T)
- crypto_kdf_hkdf_sha512_expand  (T)
- crypto_kdf_hkdf_sha512_extract  (T)
- crypto_kdf_hkdf_sha512_extract_final  (T)
- crypto_kdf_hkdf_sha512_extract_init  (T)
- crypto_kdf_hkdf_sha512_extract_update  (T)
- crypto_kdf_hkdf_sha512_keybytes  (T)
- crypto_kdf_hkdf_sha512_keygen  (T)
- crypto_kdf_hkdf_sha512_statebytes  (T)

## crypto_kem/crypto_kem.c
- crypto_kem_ciphertextbytes  (T)
- crypto_kem_dec  (T)
- crypto_kem_enc  (T)
- crypto_kem_keypair  (T)
- crypto_kem_primitive  (T)
- crypto_kem_publickeybytes  (T)
- crypto_kem_secretkeybytes  (T)
- crypto_kem_seed_keypair  (T)
- crypto_kem_seedbytes  (T)
- crypto_kem_sharedsecretbytes  (T)

## crypto_kem/mlkem768/kem_mlkem768.c
- crypto_kem_mlkem768_ciphertextbytes  (T)
- crypto_kem_mlkem768_dec  (T)
- crypto_kem_mlkem768_enc  (T)
- crypto_kem_mlkem768_enc_deterministic  (T)
- crypto_kem_mlkem768_keypair  (T)
- crypto_kem_mlkem768_publickeybytes  (T)
- crypto_kem_mlkem768_secretkeybytes  (T)
- crypto_kem_mlkem768_seed_keypair  (T)
- crypto_kem_mlkem768_seedbytes  (T)
- crypto_kem_mlkem768_sharedsecretbytes  (T)

## crypto_kem/mlkem768/ref/kem_mlkem768_ref.c
- _sodium_mlkem768_ref_dec  (T)
- _sodium_mlkem768_ref_enc  (T)
- _sodium_mlkem768_ref_enc_deterministic  (T)
- _sodium_mlkem768_ref_keypair  (T)
- _sodium_mlkem768_ref_seed_keypair  (T)

## crypto_kem/xwing/kem_xwing.c
- crypto_kem_xwing_ciphertextbytes  (T)
- crypto_kem_xwing_dec  (T)
- crypto_kem_xwing_enc  (T)
- crypto_kem_xwing_enc_deterministic  (T)
- crypto_kem_xwing_keypair  (T)
- crypto_kem_xwing_publickeybytes  (T)
- crypto_kem_xwing_secretkeybytes  (T)
- crypto_kem_xwing_seed_keypair  (T)
- crypto_kem_xwing_seedbytes  (T)
- crypto_kem_xwing_sharedsecretbytes  (T)

## crypto_kx/crypto_kx.c
- crypto_kx_client_session_keys  (T)
- crypto_kx_keypair  (T)
- crypto_kx_primitive  (T)
- crypto_kx_publickeybytes  (T)
- crypto_kx_secretkeybytes  (T)
- crypto_kx_seed_keypair  (T)
- crypto_kx_seedbytes  (T)
- crypto_kx_server_session_keys  (T)
- crypto_kx_sessionkeybytes  (T)

## crypto_onetimeauth/crypto_onetimeauth.c
- crypto_onetimeauth  (T)
- crypto_onetimeauth_bytes  (T)
- crypto_onetimeauth_final  (T)
- crypto_onetimeauth_init  (T)
- crypto_onetimeauth_keybytes  (T)
- crypto_onetimeauth_keygen  (T)
- crypto_onetimeauth_primitive  (T)
- crypto_onetimeauth_statebytes  (T)
- crypto_onetimeauth_update  (T)
- crypto_onetimeauth_verify  (T)

## crypto_onetimeauth/poly1305/donna/poly1305_donna.c
- crypto_onetimeauth_poly1305_donna_implementation  (D)

## crypto_onetimeauth/poly1305/onetimeauth_poly1305.c
- _crypto_onetimeauth_poly1305_pick_best_implementation  (T)
- crypto_onetimeauth_poly1305  (T)
- crypto_onetimeauth_poly1305_bytes  (T)
- crypto_onetimeauth_poly1305_final  (T)
- crypto_onetimeauth_poly1305_init  (T)
- crypto_onetimeauth_poly1305_keybytes  (T)
- crypto_onetimeauth_poly1305_keygen  (T)
- crypto_onetimeauth_poly1305_statebytes  (T)
- crypto_onetimeauth_poly1305_update  (T)
- crypto_onetimeauth_poly1305_verify  (T)

## crypto_pwhash/argon2/argon2-core.c
- _crypto_pwhash_argon2_pick_best_implementation  (T)
- _sodium_argon2_fill_memory_blocks  (T)
- _sodium_argon2_finalize  (T)
- _sodium_argon2_initialize  (T)
- _sodium_argon2_validate_inputs  (T)

## crypto_pwhash/argon2/argon2-encoding.c
- _sodium_argon2_decode_string  (T)
- _sodium_argon2_encode_string  (T)

## crypto_pwhash/argon2/argon2-fill-block-ref.c
- _sodium_argon2_fill_segment_ref  (T)

## crypto_pwhash/argon2/argon2.c
- _sodium_argon2_ctx  (T)
- _sodium_argon2_hash  (T)
- _sodium_argon2_verify  (T)
- _sodium_argon2i_hash_encoded  (T)
- _sodium_argon2i_hash_raw  (T)
- _sodium_argon2i_verify  (T)
- _sodium_argon2id_hash_encoded  (T)
- _sodium_argon2id_hash_raw  (T)
- _sodium_argon2id_verify  (T)

## crypto_pwhash/argon2/blake2b-long.c
- _sodium_blake2b_long  (T)

## crypto_pwhash/argon2/pwhash_argon2i.c
- crypto_pwhash_argon2i  (T)
- crypto_pwhash_argon2i_alg_argon2i13  (T)
- crypto_pwhash_argon2i_bytes_max  (T)
- crypto_pwhash_argon2i_bytes_min  (T)
- crypto_pwhash_argon2i_memlimit_interactive  (T)
- crypto_pwhash_argon2i_memlimit_max  (T)
- crypto_pwhash_argon2i_memlimit_min  (T)
- crypto_pwhash_argon2i_memlimit_moderate  (T)
- crypto_pwhash_argon2i_memlimit_sensitive  (T)
- crypto_pwhash_argon2i_opslimit_interactive  (T)
- crypto_pwhash_argon2i_opslimit_max  (T)
- crypto_pwhash_argon2i_opslimit_min  (T)
- crypto_pwhash_argon2i_opslimit_moderate  (T)
- crypto_pwhash_argon2i_opslimit_sensitive  (T)
- crypto_pwhash_argon2i_passwd_max  (T)
- crypto_pwhash_argon2i_passwd_min  (T)
- crypto_pwhash_argon2i_saltbytes  (T)
- crypto_pwhash_argon2i_str  (T)
- crypto_pwhash_argon2i_str_needs_rehash  (T)
- crypto_pwhash_argon2i_str_verify  (T)
- crypto_pwhash_argon2i_strbytes  (T)
- crypto_pwhash_argon2i_strprefix  (T)
- crypto_pwhash_argon2id_str_needs_rehash  (T)

## crypto_pwhash/argon2/pwhash_argon2id.c
- crypto_pwhash_argon2id  (T)
- crypto_pwhash_argon2id_alg_argon2id13  (T)
- crypto_pwhash_argon2id_bytes_max  (T)
- crypto_pwhash_argon2id_bytes_min  (T)
- crypto_pwhash_argon2id_memlimit_interactive  (T)
- crypto_pwhash_argon2id_memlimit_max  (T)
- crypto_pwhash_argon2id_memlimit_min  (T)
- crypto_pwhash_argon2id_memlimit_moderate  (T)
- crypto_pwhash_argon2id_memlimit_sensitive  (T)
- crypto_pwhash_argon2id_opslimit_interactive  (T)
- crypto_pwhash_argon2id_opslimit_max  (T)
- crypto_pwhash_argon2id_opslimit_min  (T)
- crypto_pwhash_argon2id_opslimit_moderate  (T)
- crypto_pwhash_argon2id_opslimit_sensitive  (T)
- crypto_pwhash_argon2id_passwd_max  (T)
- crypto_pwhash_argon2id_passwd_min  (T)
- crypto_pwhash_argon2id_saltbytes  (T)
- crypto_pwhash_argon2id_str  (T)
- crypto_pwhash_argon2id_str_verify  (T)
- crypto_pwhash_argon2id_strbytes  (T)
- crypto_pwhash_argon2id_strprefix  (T)

## crypto_pwhash/crypto_pwhash.c
- crypto_pwhash  (T)
- crypto_pwhash_alg_argon2i13  (T)
- crypto_pwhash_alg_argon2id13  (T)
- crypto_pwhash_alg_default  (T)
- crypto_pwhash_bytes_max  (T)
- crypto_pwhash_bytes_min  (T)
- crypto_pwhash_memlimit_interactive  (T)
- crypto_pwhash_memlimit_max  (T)
- crypto_pwhash_memlimit_min  (T)
- crypto_pwhash_memlimit_moderate  (T)
- crypto_pwhash_memlimit_sensitive  (T)
- crypto_pwhash_opslimit_interactive  (T)
- crypto_pwhash_opslimit_max  (T)
- crypto_pwhash_opslimit_min  (T)
- crypto_pwhash_opslimit_moderate  (T)
- crypto_pwhash_opslimit_sensitive  (T)
- crypto_pwhash_passwd_max  (T)
- crypto_pwhash_passwd_min  (T)
- crypto_pwhash_primitive  (T)
- crypto_pwhash_saltbytes  (T)
- crypto_pwhash_str  (T)
- crypto_pwhash_str_alg  (T)
- crypto_pwhash_str_needs_rehash  (T)
- crypto_pwhash_str_verify  (T)
- crypto_pwhash_strbytes  (T)
- crypto_pwhash_strprefix  (T)

## crypto_pwhash/scryptsalsa208sha256/crypto_scrypt-common.c
- _sodium_escrypt_gensalt_r  (T)
- _sodium_escrypt_parse_setting  (T)
- _sodium_escrypt_r  (T)
- crypto_pwhash_scryptsalsa208sha256_ll  (T)

## crypto_pwhash/scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c
- _sodium_escrypt_kdf_nosse  (T)

## crypto_pwhash/scryptsalsa208sha256/pbkdf2-sha256.c
- _sodium_escrypt_PBKDF2_SHA256  (T)

## crypto_pwhash/scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c
- crypto_pwhash_scryptsalsa208sha256  (T)
- crypto_pwhash_scryptsalsa208sha256_bytes_max  (T)
- crypto_pwhash_scryptsalsa208sha256_bytes_min  (T)
- crypto_pwhash_scryptsalsa208sha256_memlimit_interactive  (T)
- crypto_pwhash_scryptsalsa208sha256_memlimit_max  (T)
- crypto_pwhash_scryptsalsa208sha256_memlimit_min  (T)
- crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive  (T)
- crypto_pwhash_scryptsalsa208sha256_opslimit_interactive  (T)
- crypto_pwhash_scryptsalsa208sha256_opslimit_max  (T)
- crypto_pwhash_scryptsalsa208sha256_opslimit_min  (T)
- crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive  (T)
- crypto_pwhash_scryptsalsa208sha256_passwd_max  (T)
- crypto_pwhash_scryptsalsa208sha256_passwd_min  (T)
- crypto_pwhash_scryptsalsa208sha256_saltbytes  (T)
- crypto_pwhash_scryptsalsa208sha256_str  (T)
- crypto_pwhash_scryptsalsa208sha256_str_needs_rehash  (T)
- crypto_pwhash_scryptsalsa208sha256_str_verify  (T)
- crypto_pwhash_scryptsalsa208sha256_strbytes  (T)
- crypto_pwhash_scryptsalsa208sha256_strprefix  (T)

## crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c
- _sodium_escrypt_alloc_region  (T)
- _sodium_escrypt_free_local  (T)
- _sodium_escrypt_free_region  (T)
- _sodium_escrypt_init_local  (T)

## crypto_scalarmult/crypto_scalarmult.c
- crypto_scalarmult  (T)
- crypto_scalarmult_base  (T)
- crypto_scalarmult_bytes  (T)
- crypto_scalarmult_primitive  (T)
- crypto_scalarmult_scalarbytes  (T)

## crypto_scalarmult/curve25519/ref10/x25519_ref10.c
- crypto_scalarmult_curve25519_ref10_implementation  (D)

## crypto_scalarmult/curve25519/scalarmult_curve25519.c
- _crypto_scalarmult_curve25519_pick_best_implementation  (T)
- crypto_scalarmult_curve25519  (T)
- crypto_scalarmult_curve25519_base  (T)
- crypto_scalarmult_curve25519_bytes  (T)
- crypto_scalarmult_curve25519_scalarbytes  (T)

## crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c
- crypto_scalarmult_ed25519  (T)
- crypto_scalarmult_ed25519_base  (T)
- crypto_scalarmult_ed25519_base_noclamp  (T)
- crypto_scalarmult_ed25519_bytes  (T)
- crypto_scalarmult_ed25519_noclamp  (T)
- crypto_scalarmult_ed25519_scalarbytes  (T)

## crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c
- crypto_scalarmult_ristretto255  (T)
- crypto_scalarmult_ristretto255_base  (T)
- crypto_scalarmult_ristretto255_bytes  (T)
- crypto_scalarmult_ristretto255_scalarbytes  (T)

## crypto_secretbox/crypto_secretbox.c
- crypto_secretbox  (T)
- crypto_secretbox_boxzerobytes  (T)
- crypto_secretbox_keybytes  (T)
- crypto_secretbox_keygen  (T)
- crypto_secretbox_macbytes  (T)
- crypto_secretbox_messagebytes_max  (T)
- crypto_secretbox_noncebytes  (T)
- crypto_secretbox_open  (T)
- crypto_secretbox_primitive  (T)
- crypto_secretbox_zerobytes  (T)

## crypto_secretbox/crypto_secretbox_easy.c
- crypto_secretbox_detached  (T)
- crypto_secretbox_easy  (T)
- crypto_secretbox_open_detached  (T)
- crypto_secretbox_open_easy  (T)

## crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c
- crypto_secretbox_xchacha20poly1305_detached  (T)
- crypto_secretbox_xchacha20poly1305_easy  (T)
- crypto_secretbox_xchacha20poly1305_keybytes  (T)
- crypto_secretbox_xchacha20poly1305_macbytes  (T)
- crypto_secretbox_xchacha20poly1305_messagebytes_max  (T)
- crypto_secretbox_xchacha20poly1305_noncebytes  (T)
- crypto_secretbox_xchacha20poly1305_open_detached  (T)
- crypto_secretbox_xchacha20poly1305_open_easy  (T)

## crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c
- crypto_secretbox_xsalsa20poly1305  (T)
- crypto_secretbox_xsalsa20poly1305_boxzerobytes  (T)
- crypto_secretbox_xsalsa20poly1305_keybytes  (T)
- crypto_secretbox_xsalsa20poly1305_keygen  (T)
- crypto_secretbox_xsalsa20poly1305_macbytes  (T)
- crypto_secretbox_xsalsa20poly1305_messagebytes_max  (T)
- crypto_secretbox_xsalsa20poly1305_noncebytes  (T)
- crypto_secretbox_xsalsa20poly1305_open  (T)
- crypto_secretbox_xsalsa20poly1305_zerobytes  (T)

## crypto_secretstream/xchacha20poly1305/secretstream_xchacha20poly1305.c
- crypto_secretstream_xchacha20poly1305_abytes  (T)
- crypto_secretstream_xchacha20poly1305_headerbytes  (T)
- crypto_secretstream_xchacha20poly1305_init_pull  (T)
- crypto_secretstream_xchacha20poly1305_init_push  (T)
- crypto_secretstream_xchacha20poly1305_keybytes  (T)
- crypto_secretstream_xchacha20poly1305_keygen  (T)
- crypto_secretstream_xchacha20poly1305_messagebytes_max  (T)
- crypto_secretstream_xchacha20poly1305_pull  (T)
- crypto_secretstream_xchacha20poly1305_push  (T)
- crypto_secretstream_xchacha20poly1305_rekey  (T)
- crypto_secretstream_xchacha20poly1305_statebytes  (T)
- crypto_secretstream_xchacha20poly1305_tag_final  (T)
- crypto_secretstream_xchacha20poly1305_tag_message  (T)
- crypto_secretstream_xchacha20poly1305_tag_push  (T)
- crypto_secretstream_xchacha20poly1305_tag_rekey  (T)

## crypto_shorthash/crypto_shorthash.c
- crypto_shorthash  (T)
- crypto_shorthash_bytes  (T)
- crypto_shorthash_keybytes  (T)
- crypto_shorthash_keygen  (T)
- crypto_shorthash_primitive  (T)

## crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c
- crypto_shorthash_siphash24  (T)

## crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c
- crypto_shorthash_siphashx24  (T)

## crypto_shorthash/siphash24/shorthash_siphash24.c
- crypto_shorthash_siphash24_bytes  (T)
- crypto_shorthash_siphash24_keybytes  (T)

## crypto_shorthash/siphash24/shorthash_siphashx24.c
- crypto_shorthash_siphashx24_bytes  (T)
- crypto_shorthash_siphashx24_keybytes  (T)

## crypto_sign/crypto_sign.c
- crypto_sign  (T)
- crypto_sign_bytes  (T)
- crypto_sign_detached  (T)
- crypto_sign_final_create  (T)
- crypto_sign_final_verify  (T)
- crypto_sign_init  (T)
- crypto_sign_keypair  (T)
- crypto_sign_messagebytes_max  (T)
- crypto_sign_open  (T)
- crypto_sign_primitive  (T)
- crypto_sign_publickeybytes  (T)
- crypto_sign_secretkeybytes  (T)
- crypto_sign_seed_keypair  (T)
- crypto_sign_seedbytes  (T)
- crypto_sign_statebytes  (T)
- crypto_sign_update  (T)
- crypto_sign_verify_detached  (T)

## crypto_sign/ed25519/ref10/keypair.c
- crypto_sign_ed25519_keypair  (T)
- crypto_sign_ed25519_pk_to_curve25519  (T)
- crypto_sign_ed25519_seed_keypair  (T)
- crypto_sign_ed25519_sk_to_curve25519  (T)

## crypto_sign/ed25519/ref10/open.c
- _crypto_sign_ed25519_verify_detached  (T)
- crypto_sign_ed25519_open  (T)
- crypto_sign_ed25519_verify_detached  (T)

## crypto_sign/ed25519/ref10/sign.c
- _crypto_sign_ed25519_detached  (T)
- _crypto_sign_ed25519_ref10_hinit  (T)
- crypto_sign_ed25519  (T)
- crypto_sign_ed25519_detached  (T)

## crypto_sign/ed25519/sign_ed25519.c
- crypto_sign_ed25519_bytes  (T)
- crypto_sign_ed25519_messagebytes_max  (T)
- crypto_sign_ed25519_publickeybytes  (T)
- crypto_sign_ed25519_secretkeybytes  (T)
- crypto_sign_ed25519_seedbytes  (T)
- crypto_sign_ed25519_sk_to_pk  (T)
- crypto_sign_ed25519_sk_to_seed  (T)
- crypto_sign_ed25519ph_final_create  (T)
- crypto_sign_ed25519ph_final_verify  (T)
- crypto_sign_ed25519ph_init  (T)
- crypto_sign_ed25519ph_statebytes  (T)
- crypto_sign_ed25519ph_update  (T)

## crypto_stream/chacha20/ref/chacha20_ref.c
- crypto_stream_chacha20_ref_implementation  (D)

## crypto_stream/chacha20/stream_chacha20.c
- _crypto_stream_chacha20_pick_best_implementation  (T)
- crypto_stream_chacha20  (T)
- crypto_stream_chacha20_ietf  (T)
- crypto_stream_chacha20_ietf_ext  (T)
- crypto_stream_chacha20_ietf_ext_xor_ic  (T)
- crypto_stream_chacha20_ietf_keybytes  (T)
- crypto_stream_chacha20_ietf_keygen  (T)
- crypto_stream_chacha20_ietf_messagebytes_max  (T)
- crypto_stream_chacha20_ietf_noncebytes  (T)
- crypto_stream_chacha20_ietf_xor  (T)
- crypto_stream_chacha20_ietf_xor_ic  (T)
- crypto_stream_chacha20_keybytes  (T)
- crypto_stream_chacha20_keygen  (T)
- crypto_stream_chacha20_messagebytes_max  (T)
- crypto_stream_chacha20_noncebytes  (T)
- crypto_stream_chacha20_xor  (T)
- crypto_stream_chacha20_xor_ic  (T)

## crypto_stream/crypto_stream.c
- crypto_stream  (T)
- crypto_stream_keybytes  (T)
- crypto_stream_keygen  (T)
- crypto_stream_messagebytes_max  (T)
- crypto_stream_noncebytes  (T)
- crypto_stream_primitive  (T)
- crypto_stream_xor  (T)

## crypto_stream/salsa20/ref/salsa20_ref.c
- crypto_stream_salsa20_ref_implementation  (D)

## crypto_stream/salsa20/stream_salsa20.c
- _crypto_stream_salsa20_pick_best_implementation  (T)
- crypto_stream_salsa20  (T)
- crypto_stream_salsa20_keybytes  (T)
- crypto_stream_salsa20_keygen  (T)
- crypto_stream_salsa20_messagebytes_max  (T)
- crypto_stream_salsa20_noncebytes  (T)
- crypto_stream_salsa20_xor  (T)
- crypto_stream_salsa20_xor_ic  (T)

## crypto_stream/salsa2012/ref/stream_salsa2012_ref.c
- crypto_stream_salsa2012  (T)
- crypto_stream_salsa2012_xor  (T)

## crypto_stream/salsa2012/stream_salsa2012.c
- crypto_stream_salsa2012_keybytes  (T)
- crypto_stream_salsa2012_keygen  (T)
- crypto_stream_salsa2012_messagebytes_max  (T)
- crypto_stream_salsa2012_noncebytes  (T)

## crypto_stream/salsa208/ref/stream_salsa208_ref.c
- crypto_stream_salsa208  (T)
- crypto_stream_salsa208_xor  (T)

## crypto_stream/salsa208/stream_salsa208.c
- crypto_stream_salsa208_keybytes  (T)
- crypto_stream_salsa208_keygen  (T)
- crypto_stream_salsa208_messagebytes_max  (T)
- crypto_stream_salsa208_noncebytes  (T)

## crypto_stream/xchacha20/stream_xchacha20.c
- crypto_stream_xchacha20  (T)
- crypto_stream_xchacha20_keybytes  (T)
- crypto_stream_xchacha20_keygen  (T)
- crypto_stream_xchacha20_messagebytes_max  (T)
- crypto_stream_xchacha20_noncebytes  (T)
- crypto_stream_xchacha20_xor  (T)
- crypto_stream_xchacha20_xor_ic  (T)

## crypto_stream/xsalsa20/stream_xsalsa20.c
- crypto_stream_xsalsa20  (T)
- crypto_stream_xsalsa20_keybytes  (T)
- crypto_stream_xsalsa20_keygen  (T)
- crypto_stream_xsalsa20_messagebytes_max  (T)
- crypto_stream_xsalsa20_noncebytes  (T)
- crypto_stream_xsalsa20_xor  (T)
- crypto_stream_xsalsa20_xor_ic  (T)

## crypto_verify/verify.c
- crypto_verify_16  (T)
- crypto_verify_16_bytes  (T)
- crypto_verify_32  (T)
- crypto_verify_32_bytes  (T)
- crypto_verify_64  (T)
- crypto_verify_64_bytes  (T)

## crypto_xof/shake128/ref/shake128_ref.c
- _sodium_shake128_ref  (T)
- _sodium_shake128_ref_init  (T)
- _sodium_shake128_ref_init_with_domain  (T)
- _sodium_shake128_ref_squeeze  (T)
- _sodium_shake128_ref_update  (T)

## crypto_xof/shake128/xof_shake128.c
- crypto_xof_shake128  (T)
- crypto_xof_shake128_blockbytes  (T)
- crypto_xof_shake128_domain_standard  (T)
- crypto_xof_shake128_init  (T)
- crypto_xof_shake128_init_with_domain  (T)
- crypto_xof_shake128_squeeze  (T)
- crypto_xof_shake128_statebytes  (T)
- crypto_xof_shake128_update  (T)

## crypto_xof/shake256/ref/shake256_ref.c
- _sodium_shake256_ref  (T)
- _sodium_shake256_ref_init  (T)
- _sodium_shake256_ref_init_with_domain  (T)
- _sodium_shake256_ref_squeeze  (T)
- _sodium_shake256_ref_update  (T)

## crypto_xof/shake256/xof_shake256.c
- crypto_xof_shake256  (T)
- crypto_xof_shake256_blockbytes  (T)
- crypto_xof_shake256_domain_standard  (T)
- crypto_xof_shake256_init  (T)
- crypto_xof_shake256_init_with_domain  (T)
- crypto_xof_shake256_squeeze  (T)
- crypto_xof_shake256_statebytes  (T)
- crypto_xof_shake256_update  (T)

## crypto_xof/turboshake128/ref/turboshake128_ref.c
- _sodium_turboshake128_ref  (T)
- _sodium_turboshake128_ref_init  (T)
- _sodium_turboshake128_ref_init_with_domain  (T)
- _sodium_turboshake128_ref_squeeze  (T)
- _sodium_turboshake128_ref_update  (T)

## crypto_xof/turboshake128/xof_turboshake128.c
- crypto_xof_turboshake128  (T)
- crypto_xof_turboshake128_blockbytes  (T)
- crypto_xof_turboshake128_domain_standard  (T)
- crypto_xof_turboshake128_init  (T)
- crypto_xof_turboshake128_init_with_domain  (T)
- crypto_xof_turboshake128_squeeze  (T)
- crypto_xof_turboshake128_statebytes  (T)
- crypto_xof_turboshake128_update  (T)

## crypto_xof/turboshake256/ref/turboshake256_ref.c
- _sodium_turboshake256_ref  (T)
- _sodium_turboshake256_ref_init  (T)
- _sodium_turboshake256_ref_init_with_domain  (T)
- _sodium_turboshake256_ref_squeeze  (T)
- _sodium_turboshake256_ref_update  (T)

## crypto_xof/turboshake256/xof_turboshake256.c
- crypto_xof_turboshake256  (T)
- crypto_xof_turboshake256_blockbytes  (T)
- crypto_xof_turboshake256_domain_standard  (T)
- crypto_xof_turboshake256_init  (T)
- crypto_xof_turboshake256_init_with_domain  (T)
- crypto_xof_turboshake256_squeeze  (T)
- crypto_xof_turboshake256_statebytes  (T)
- crypto_xof_turboshake256_update  (T)

## randombytes/internal/randombytes_internal_random.c
- randombytes_internal_implementation  (D)

## randombytes/randombytes.c
- randombytes  (T)
- randombytes_buf  (T)
- randombytes_buf_deterministic  (T)
- randombytes_close  (T)
- randombytes_implementation_name  (T)
- randombytes_random  (T)
- randombytes_seedbytes  (T)
- randombytes_set_implementation  (T)
- randombytes_stir  (T)
- randombytes_uniform  (T)

## randombytes/sysrandom/randombytes_sysrandom.c
- randombytes_sysrandom_implementation  (D)

## sodium/codecs.c
- sodium_base642bin  (T)
- sodium_base64_encoded_len  (T)
- sodium_bin2base64  (T)
- sodium_bin2hex  (T)
- sodium_bin2ip  (T)
- sodium_hex2bin  (T)
- sodium_ip2bin  (T)

## sodium/core.c
- sodium_crit_enter  (T)
- sodium_crit_leave  (T)
- sodium_init  (T)
- sodium_misuse  (T)
- sodium_set_misuse_handler  (T)

## sodium/runtime.c
- _sodium_runtime_get_cpu_features  (T)
- sodium_runtime_has_aesni  (W)
- sodium_runtime_has_armcrypto  (W)
- sodium_runtime_has_avx  (W)
- sodium_runtime_has_avx2  (W)
- sodium_runtime_has_avx512f  (W)
- sodium_runtime_has_neon  (W)
- sodium_runtime_has_pclmul  (W)
- sodium_runtime_has_rdrand  (W)
- sodium_runtime_has_sse2  (W)
- sodium_runtime_has_sse3  (W)
- sodium_runtime_has_sse41  (W)
- sodium_runtime_has_ssse3  (W)

## sodium/utils.c
- _sodium_alloc_init  (T)
- sodium_add  (T)
- sodium_allocarray  (T)
- sodium_compare  (T)
- sodium_free  (T)
- sodium_increment  (T)
- sodium_is_zero  (T)
- sodium_malloc  (T)
- sodium_memcmp  (T)
- sodium_memzero  (T)
- sodium_mlock  (T)
- sodium_mprotect_noaccess  (T)
- sodium_mprotect_readonly  (T)
- sodium_mprotect_readwrite  (T)
- sodium_munlock  (T)
- sodium_pad  (T)
- sodium_stackzero  (T)
- sodium_sub  (T)
- sodium_unpad  (T)

## sodium/version.c
- sodium_library_minimal  (T)
- sodium_library_version_major  (T)
- sodium_library_version_minor  (T)
- sodium_version_string  (T)
