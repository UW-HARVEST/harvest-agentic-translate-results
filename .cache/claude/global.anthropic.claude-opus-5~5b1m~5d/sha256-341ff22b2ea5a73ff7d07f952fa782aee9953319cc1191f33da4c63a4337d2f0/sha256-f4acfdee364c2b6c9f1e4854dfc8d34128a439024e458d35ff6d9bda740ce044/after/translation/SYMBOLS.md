# SYMBOLS.md — C `nm -D` surface vs. Rust cdylib exports

Generated mechanically by `gen_symbols.sh` from
`nm -D --defined-only` on the C `libsphincs_core_det.so` + `lib<backend>.so`
pair produced by CMake, compared against the Rust `libsphincs_core_det.so`.

## Per-configuration parity (all 48 CMake configurations)

| config (backend-thash-secpar) | # C symbols | missing from Rust `.so` |
|---|---|---|
| `haraka-robust-128s` | 56 | *(none)* |
| `haraka-robust-128f` | 56 | *(none)* |
| `haraka-robust-192s` | 56 | *(none)* |
| `haraka-robust-192f` | 56 | *(none)* |
| `haraka-robust-256s` | 56 | *(none)* |
| `haraka-robust-256f` | 56 | *(none)* |
| `haraka-simple-128s` | 56 | *(none)* |
| `haraka-simple-128f` | 56 | *(none)* |
| `haraka-simple-192s` | 56 | *(none)* |
| `haraka-simple-192f` | 56 | *(none)* |
| `haraka-simple-256s` | 56 | *(none)* |
| `haraka-simple-256f` | 56 | *(none)* |
| `sha2-robust-128s` | 58 | *(none)* |
| `sha2-robust-128f` | 58 | *(none)* |
| `sha2-robust-192s` | 58 | *(none)* |
| `sha2-robust-192f` | 58 | *(none)* |
| `sha2-robust-256s` | 58 | *(none)* |
| `sha2-robust-256f` | 58 | *(none)* |
| `sha2-simple-128s` | 58 | *(none)* |
| `sha2-simple-128f` | 58 | *(none)* |
| `sha2-simple-192s` | 58 | *(none)* |
| `sha2-simple-192f` | 58 | *(none)* |
| `sha2-simple-256s` | 58 | *(none)* |
| `sha2-simple-256f` | 58 | *(none)* |
| `shake-robust-128s` | 54 | *(none)* |
| `shake-robust-128f` | 54 | *(none)* |
| `shake-robust-192s` | 54 | *(none)* |
| `shake-robust-192f` | 54 | *(none)* |
| `shake-robust-256s` | 54 | *(none)* |
| `shake-robust-256f` | 54 | *(none)* |
| `shake-simple-128s` | 54 | *(none)* |
| `shake-simple-128f` | 54 | *(none)* |
| `shake-simple-192s` | 54 | *(none)* |
| `shake-simple-192f` | 54 | *(none)* |
| `shake-simple-256s` | 54 | *(none)* |
| `shake-simple-256f` | 54 | *(none)* |
| `blake-robust-128s` | 60 | *(none)* |
| `blake-robust-128f` | 60 | *(none)* |
| `blake-robust-192s` | 60 | *(none)* |
| `blake-robust-192f` | 60 | *(none)* |
| `blake-robust-256s` | 60 | *(none)* |
| `blake-robust-256f` | 60 | *(none)* |
| `blake-simple-128s` | 60 | *(none)* |
| `blake-simple-128f` | 60 | *(none)* |
| `blake-simple-192s` | 60 | *(none)* |
| `blake-simple-192f` | 60 | *(none)* |
| `blake-simple-256s` | 60 | *(none)* |
| `blake-simple-256f` | 60 | *(none)* |

**Configurations with missing symbols: 0 / 48**

## Backend `haraka` — every C-exported symbol

| symbol | nm type | C translation unit (best-effort grep) | in Rust `.so` |
|---|---|---|---|
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | yes |
| `AES256_ECB` | T | `app/src/rng.c` | yes |
| `DRBG_ctx` | B | `(macro-generated / data)` | yes |
| `SPX_bytes_to_ull` | T | `app/src/utils.c` | yes |
| `SPX_chain_lengths` | T | `app/src/merkle.c` | yes |
| `SPX_compute_root` | T | `app/src/fors.c` | yes |
| `SPX_copy_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_copy_subtree_addr` | T | `app/src/address.c` | yes |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | yes |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | yes |
| `SPX_fors_sign` | T | `app/src/fors.c` | yes |
| `SPX_fors_treehashx1` | T | `app/src/fors.c` | yes |
| `SPX_gen_message_random` | T | `app/src/sign.c` | yes |
| `SPX_haraka256` | T | `lib/haraka/src/haraka.c` | yes |
| `SPX_haraka512` | T | `lib/haraka/src/haraka.c` | yes |
| `SPX_haraka512_perm` | T | `lib/haraka/src/haraka.c` | yes |
| `SPX_haraka_S` | T | `lib/haraka/src/haraka.c` | yes |
| `SPX_haraka_S_inc_absorb` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `SPX_haraka_S_inc_finalize` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `SPX_haraka_S_inc_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `SPX_haraka_S_inc_squeeze` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `SPX_hash_message` | T | `app/src/sign.c` | yes |
| `SPX_initialize_hash_function` | T | `app/src/sign.c` | yes |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | yes |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | yes |
| `SPX_prf_addr` | T | `app/src/fors.c` | yes |
| `SPX_set_chain_addr` | T | `app/src/address.c` | yes |
| `SPX_set_hash_addr` | T | `app/src/address.c` | yes |
| `SPX_set_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_set_layer_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_height` | T | `app/src/address.c` | yes |
| `SPX_set_tree_index` | T | `app/src/address.c` | yes |
| `SPX_set_type` | T | `app/src/address.c` | yes |
| `SPX_thash` | T | `app/src/fors.c` | yes |
| `SPX_treehash` | T | `app/src/utils.c` | yes |
| `SPX_tweak_constants` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `SPX_u32_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_ull_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_wots_gen_leafx1` | T | `app/src/utilsx1.c` | yes |
| `SPX_wots_pk_from_sig` | T | `app/src/sign.c` | yes |
| `SPX_wots_treehashx1` | T | `app/src/merkle.c` | yes |
| `crypto_sign` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_bytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_keypair` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_open` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_publickeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_seed_keypair` | T | `app/src/sign.c` | yes |
| `crypto_sign_seedbytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_signature` | T | `app/src/sign.c` | yes |
| `crypto_sign_verify` | T | `app/src/sign.c` | yes |
| `randombytes` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `randombytes_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `seedexpander` | T | `app/src/rng.c` | yes |
| `seedexpander_init` | T | `app/src/rng.c` | yes |

## Backend `sha2` — every C-exported symbol

| symbol | nm type | C translation unit (best-effort grep) | in Rust `.so` |
|---|---|---|---|
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | yes |
| `AES256_ECB` | T | `app/src/rng.c` | yes |
| `DRBG_ctx` | B | `(macro-generated / data)` | yes |
| `SPX_bytes_to_ull` | T | `app/src/utils.c` | yes |
| `SPX_chain_lengths` | T | `app/src/merkle.c` | yes |
| `SPX_compute_root` | T | `app/src/fors.c` | yes |
| `SPX_copy_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_copy_subtree_addr` | T | `app/src/address.c` | yes |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | yes |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | yes |
| `SPX_fors_sign` | T | `app/src/fors.c` | yes |
| `SPX_fors_treehashx1` | T | `app/src/fors.c` | yes |
| `SPX_gen_message_random` | T | `app/src/sign.c` | yes |
| `SPX_hash_message` | T | `app/src/sign.c` | yes |
| `SPX_initialize_hash_function` | T | `app/src/sign.c` | yes |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | yes |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | yes |
| `SPX_mgf1_256` | T | `lib/sha2/src/sha2.c` | yes |
| `SPX_mgf1_512` | T | `lib/sha2/src/sha2.c` | yes |
| `SPX_prf_addr` | T | `app/src/fors.c` | yes |
| `SPX_seed_state` | T | `lib/sha2/src/hash_sha2.c` | yes |
| `SPX_set_chain_addr` | T | `app/src/address.c` | yes |
| `SPX_set_hash_addr` | T | `app/src/address.c` | yes |
| `SPX_set_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_set_layer_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_height` | T | `app/src/address.c` | yes |
| `SPX_set_tree_index` | T | `app/src/address.c` | yes |
| `SPX_set_type` | T | `app/src/address.c` | yes |
| `SPX_thash` | T | `app/src/fors.c` | yes |
| `SPX_treehash` | T | `app/src/utils.c` | yes |
| `SPX_u32_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_ull_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_wots_gen_leafx1` | T | `app/src/utilsx1.c` | yes |
| `SPX_wots_pk_from_sig` | T | `app/src/sign.c` | yes |
| `SPX_wots_treehashx1` | T | `app/src/merkle.c` | yes |
| `crypto_sign` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_bytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_keypair` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_open` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_publickeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_seed_keypair` | T | `app/src/sign.c` | yes |
| `crypto_sign_seedbytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_signature` | T | `app/src/sign.c` | yes |
| `crypto_sign_verify` | T | `app/src/sign.c` | yes |
| `randombytes` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `randombytes_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `seedexpander` | T | `app/src/rng.c` | yes |
| `seedexpander_init` | T | `app/src/rng.c` | yes |
| `sha256` | T | `lib/sha2/src/sha2.c` | yes |
| `sha256_inc_blocks` | T | `lib/sha2/src/sha2.c` | yes |
| `sha256_inc_finalize` | T | `lib/sha2/src/hash_sha2.c` | yes |
| `sha256_inc_init` | T | `lib/sha2/src/sha2.c` | yes |
| `sha512` | T | `lib/sha2/src/sha2.c` | yes |
| `sha512_inc_blocks` | T | `lib/sha2/src/sha2.c` | yes |
| `sha512_inc_finalize` | T | `lib/sha2/src/sha2.c` | yes |
| `sha512_inc_init` | T | `lib/sha2/src/sha2.c` | yes |

## Backend `shake` — every C-exported symbol

| symbol | nm type | C translation unit (best-effort grep) | in Rust `.so` |
|---|---|---|---|
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | yes |
| `AES256_ECB` | T | `app/src/rng.c` | yes |
| `DRBG_ctx` | B | `(macro-generated / data)` | yes |
| `SPX_bytes_to_ull` | T | `app/src/utils.c` | yes |
| `SPX_chain_lengths` | T | `app/src/merkle.c` | yes |
| `SPX_compute_root` | T | `app/src/fors.c` | yes |
| `SPX_copy_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_copy_subtree_addr` | T | `app/src/address.c` | yes |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | yes |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | yes |
| `SPX_fors_sign` | T | `app/src/fors.c` | yes |
| `SPX_fors_treehashx1` | T | `app/src/fors.c` | yes |
| `SPX_gen_message_random` | T | `app/src/sign.c` | yes |
| `SPX_hash_message` | T | `app/src/sign.c` | yes |
| `SPX_initialize_hash_function` | T | `app/src/sign.c` | yes |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | yes |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | yes |
| `SPX_prf_addr` | T | `app/src/fors.c` | yes |
| `SPX_set_chain_addr` | T | `app/src/address.c` | yes |
| `SPX_set_hash_addr` | T | `app/src/address.c` | yes |
| `SPX_set_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_set_layer_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_height` | T | `app/src/address.c` | yes |
| `SPX_set_tree_index` | T | `app/src/address.c` | yes |
| `SPX_set_type` | T | `app/src/address.c` | yes |
| `SPX_thash` | T | `app/src/fors.c` | yes |
| `SPX_treehash` | T | `app/src/utils.c` | yes |
| `SPX_u32_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_ull_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_wots_gen_leafx1` | T | `app/src/utilsx1.c` | yes |
| `SPX_wots_pk_from_sig` | T | `app/src/sign.c` | yes |
| `SPX_wots_treehashx1` | T | `app/src/merkle.c` | yes |
| `crypto_sign` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_bytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_keypair` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_open` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_publickeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_seed_keypair` | T | `app/src/sign.c` | yes |
| `crypto_sign_seedbytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_signature` | T | `app/src/sign.c` | yes |
| `crypto_sign_verify` | T | `app/src/sign.c` | yes |
| `randombytes` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `randombytes_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `seedexpander` | T | `app/src/rng.c` | yes |
| `seedexpander_init` | T | `app/src/rng.c` | yes |
| `shake256` | T | `lib/shake/src/fips202.c` | yes |
| `shake256_absorb` | T | `lib/shake/src/fips202.c` | yes |
| `shake256_inc_absorb` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `shake256_inc_finalize` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `shake256_inc_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `shake256_inc_squeeze` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `shake256_squeezeblocks` | T | `lib/shake/src/fips202.c` | yes |

## Backend `blake` — every C-exported symbol

| symbol | nm type | C translation unit (best-effort grep) | in Rust `.so` |
|---|---|---|---|
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | yes |
| `AES256_ECB` | T | `app/src/rng.c` | yes |
| `DRBG_ctx` | B | `(macro-generated / data)` | yes |
| `SPX_blake256_mgf1` | T | `lib/blake/src/blake256.c` | yes |
| `SPX_blake512_mgf1` | T | `lib/blake/src/blake512.c` | yes |
| `SPX_bytes_to_ull` | T | `app/src/utils.c` | yes |
| `SPX_chain_lengths` | T | `app/src/merkle.c` | yes |
| `SPX_compute_root` | T | `app/src/fors.c` | yes |
| `SPX_copy_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_copy_subtree_addr` | T | `app/src/address.c` | yes |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | yes |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | yes |
| `SPX_fors_sign` | T | `app/src/fors.c` | yes |
| `SPX_fors_treehashx1` | T | `app/src/fors.c` | yes |
| `SPX_gen_message_random` | T | `app/src/sign.c` | yes |
| `SPX_hash_message` | T | `app/src/sign.c` | yes |
| `SPX_initialize_hash_function` | T | `app/src/sign.c` | yes |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | yes |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | yes |
| `SPX_prf_addr` | T | `app/src/fors.c` | yes |
| `SPX_set_chain_addr` | T | `app/src/address.c` | yes |
| `SPX_set_hash_addr` | T | `app/src/address.c` | yes |
| `SPX_set_keypair_addr` | T | `app/src/address.c` | yes |
| `SPX_set_layer_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_addr` | T | `app/src/address.c` | yes |
| `SPX_set_tree_height` | T | `app/src/address.c` | yes |
| `SPX_set_tree_index` | T | `app/src/address.c` | yes |
| `SPX_set_type` | T | `app/src/address.c` | yes |
| `SPX_thash` | T | `app/src/fors.c` | yes |
| `SPX_treehash` | T | `app/src/utils.c` | yes |
| `SPX_u32_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_ull_to_bytes` | T | `app/src/address.c` | yes |
| `SPX_wots_gen_leafx1` | T | `app/src/utilsx1.c` | yes |
| `SPX_wots_pk_from_sig` | T | `app/src/sign.c` | yes |
| `SPX_wots_treehashx1` | T | `app/src/merkle.c` | yes |
| `blake256` | T | `lib/blake/src/blake256.c` | yes |
| `blake256_compress` | T | `lib/blake/src/blake256.c` | yes |
| `blake256_final` | T | `lib/blake/src/blake256.c` | yes |
| `blake256_init` | T | `lib/blake/src/blake256.c` | yes |
| `blake256_update` | T | `lib/blake/src/blake256.c` | yes |
| `blake512` | T | `lib/blake/src/blake512.c` | yes |
| `blake512_compress` | T | `lib/blake/src/blake512.c` | yes |
| `blake512_final` | T | `lib/blake/src/blake512.c` | yes |
| `blake512_init` | T | `lib/blake/src/blake512.c` | yes |
| `blake512_update` | T | `lib/blake/src/blake512.c` | yes |
| `crypto_sign` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_bytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_keypair` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_open` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `crypto_sign_publickeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_seed_keypair` | T | `app/src/sign.c` | yes |
| `crypto_sign_seedbytes` | T | `app/src/sign.c` | yes |
| `crypto_sign_signature` | T | `app/src/sign.c` | yes |
| `crypto_sign_verify` | T | `app/src/sign.c` | yes |
| `cst` | R | `(macro-generated / data)` | yes |
| `randombytes` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `randombytes_init` | T | `app/src/PQCgenKAT_sign.c` | yes |
| `seedexpander` | T | `app/src/rng.c` | yes |
| `seedexpander_init` | T | `app/src/rng.c` | yes |

