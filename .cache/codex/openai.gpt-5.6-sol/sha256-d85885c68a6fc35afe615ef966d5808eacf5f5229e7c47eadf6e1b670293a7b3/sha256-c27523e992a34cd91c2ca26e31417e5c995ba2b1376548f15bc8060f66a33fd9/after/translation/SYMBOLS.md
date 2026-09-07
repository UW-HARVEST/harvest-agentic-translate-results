# C dynamic-symbol surface

Mechanically derived from the union of `nm -D --defined-only` on the default
`libsphincs_core_det.so` and `libblake.so`. The normal
`libsphincs_core.so` is a subset except that its `randombytes` implementation
comes from `randombytes.c`. `T`, `B`, and `R` are the ELF symbol kinds.

| # | kind | symbol | Rust status |
|---|------|--------|-------------|
| 1 | T | `AES256_CTR_DRBG_Update` | present |
| 2 | T | `AES256_ECB` | present |
| 3 | B | `DRBG_ctx` | present |
| 4 | T | `SPX_blake256_mgf1` | present |
| 5 | T | `SPX_blake512_mgf1` | present |
| 6 | T | `SPX_bytes_to_ull` | present |
| 7 | T | `SPX_chain_lengths` | present |
| 8 | T | `SPX_compute_root` | present |
| 9 | T | `SPX_copy_keypair_addr` | present |
| 10 | T | `SPX_copy_subtree_addr` | present |
| 11 | T | `SPX_fors_gen_leafx1` | present |
| 12 | T | `SPX_fors_pk_from_sig` | present |
| 13 | T | `SPX_fors_sign` | present |
| 14 | T | `SPX_fors_treehashx1` | present |
| 15 | T | `SPX_gen_message_random` | present |
| 16 | T | `SPX_hash_message` | present |
| 17 | T | `SPX_initialize_hash_function` | present |
| 18 | T | `SPX_merkle_gen_root` | present |
| 19 | T | `SPX_merkle_sign` | present |
| 20 | T | `SPX_prf_addr` | present |
| 21 | T | `SPX_set_chain_addr` | present |
| 22 | T | `SPX_set_hash_addr` | present |
| 23 | T | `SPX_set_keypair_addr` | present |
| 24 | T | `SPX_set_layer_addr` | present |
| 25 | T | `SPX_set_tree_addr` | present |
| 26 | T | `SPX_set_tree_height` | present |
| 27 | T | `SPX_set_tree_index` | present |
| 28 | T | `SPX_set_type` | present |
| 29 | T | `SPX_thash` | present |
| 30 | T | `SPX_treehash` | present |
| 31 | T | `SPX_u32_to_bytes` | present |
| 32 | T | `SPX_ull_to_bytes` | present |
| 33 | T | `SPX_wots_gen_leafx1` | present |
| 34 | T | `SPX_wots_pk_from_sig` | present |
| 35 | T | `SPX_wots_treehashx1` | present |
| 36 | T | `blake256` | present |
| 37 | T | `blake256_compress` | present |
| 38 | T | `blake256_final` | present |
| 39 | T | `blake256_init` | present |
| 40 | T | `blake256_update` | present |
| 41 | T | `blake512` | present |
| 42 | T | `blake512_compress` | present |
| 43 | T | `blake512_final` | present |
| 44 | T | `blake512_init` | present |
| 45 | T | `blake512_update` | present |
| 46 | T | `crypto_sign` | present |
| 47 | T | `crypto_sign_bytes` | present |
| 48 | T | `crypto_sign_keypair` | present |
| 49 | T | `crypto_sign_open` | present |
| 50 | T | `crypto_sign_publickeybytes` | present |
| 51 | T | `crypto_sign_secretkeybytes` | present |
| 52 | T | `crypto_sign_seed_keypair` | present |
| 53 | T | `crypto_sign_seedbytes` | present |
| 54 | T | `crypto_sign_signature` | present |
| 55 | T | `crypto_sign_verify` | present |
| 56 | R | `cst` | present |
| 57 | T | `randombytes` | present |
| 58 | T | `randombytes_init` | present |
| 59 | T | `seedexpander` | present |
| 60 | T | `seedexpander_init` | present |

The initial diff found `cst` and `DRBG_ctx`. `cst` is the actual exported
BLAKE-512 constant table. `DRBG_ctx` is the actual C-layout RNG state and the
Rust RNG now reads and writes that exported state.
