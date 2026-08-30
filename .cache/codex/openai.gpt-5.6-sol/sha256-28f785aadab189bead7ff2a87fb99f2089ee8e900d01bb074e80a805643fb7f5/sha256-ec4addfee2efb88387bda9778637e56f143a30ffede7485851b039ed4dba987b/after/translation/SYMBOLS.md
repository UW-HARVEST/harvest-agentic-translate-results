# Exported Symbol Surface

Generated from the union of `nm -D --defined-only` over the default C build:

- `c_src/build/app/libsphincs_core.so`
- `c_src/build/app/libsphincs_core_det.so`
- `c_src/build/lib/blake/libblake.so`

`T`, `B`, and `R` are the ELF symbol types reported by `nm`. Duplicate
definitions shared by the two core libraries or by the backend are listed once.

| # | C symbol | type | Rust export |
|---|----------|------|-------------|
| 1 | `AES256_CTR_DRBG_Update` | T | [x] |
| 2 | `AES256_ECB` | T | [x] |
| 3 | `DRBG_ctx` | B | [x] |
| 4 | `SPX_blake256_mgf1` | T | [x] |
| 5 | `SPX_blake512_mgf1` | T | [x] |
| 6 | `SPX_bytes_to_ull` | T | [x] |
| 7 | `SPX_chain_lengths` | T | [x] |
| 8 | `SPX_compute_root` | T | [x] |
| 9 | `SPX_copy_keypair_addr` | T | [x] |
| 10 | `SPX_copy_subtree_addr` | T | [x] |
| 11 | `SPX_fors_gen_leafx1` | T | [x] |
| 12 | `SPX_fors_pk_from_sig` | T | [x] |
| 13 | `SPX_fors_sign` | T | [x] |
| 14 | `SPX_fors_treehashx1` | T | [x] |
| 15 | `SPX_gen_message_random` | T | [x] |
| 16 | `SPX_hash_message` | T | [x] |
| 17 | `SPX_initialize_hash_function` | T | [x] |
| 18 | `SPX_merkle_gen_root` | T | [x] |
| 19 | `SPX_merkle_sign` | T | [x] |
| 20 | `SPX_prf_addr` | T | [x] |
| 21 | `SPX_set_chain_addr` | T | [x] |
| 22 | `SPX_set_hash_addr` | T | [x] |
| 23 | `SPX_set_keypair_addr` | T | [x] |
| 24 | `SPX_set_layer_addr` | T | [x] |
| 25 | `SPX_set_tree_addr` | T | [x] |
| 26 | `SPX_set_tree_height` | T | [x] |
| 27 | `SPX_set_tree_index` | T | [x] |
| 28 | `SPX_set_type` | T | [x] |
| 29 | `SPX_thash` | T | [x] |
| 30 | `SPX_treehash` | T | [x] |
| 31 | `SPX_u32_to_bytes` | T | [x] |
| 32 | `SPX_ull_to_bytes` | T | [x] |
| 33 | `SPX_wots_gen_leafx1` | T | [x] |
| 34 | `SPX_wots_pk_from_sig` | T | [x] |
| 35 | `SPX_wots_treehashx1` | T | [x] |
| 36 | `blake256` | T | [x] |
| 37 | `blake256_compress` | T | [x] |
| 38 | `blake256_final` | T | [x] |
| 39 | `blake256_init` | T | [x] |
| 40 | `blake256_update` | T | [x] |
| 41 | `blake512` | T | [x] |
| 42 | `blake512_compress` | T | [x] |
| 43 | `blake512_final` | T | [x] |
| 44 | `blake512_init` | T | [x] |
| 45 | `blake512_update` | T | [x] |
| 46 | `crypto_sign` | T | [x] |
| 47 | `crypto_sign_bytes` | T | [x] |
| 48 | `crypto_sign_keypair` | T | [x] |
| 49 | `crypto_sign_open` | T | [x] |
| 50 | `crypto_sign_publickeybytes` | T | [x] |
| 51 | `crypto_sign_secretkeybytes` | T | [x] |
| 52 | `crypto_sign_seed_keypair` | T | [x] |
| 53 | `crypto_sign_seedbytes` | T | [x] |
| 54 | `crypto_sign_signature` | T | [x] |
| 55 | `crypto_sign_verify` | T | [x] |
| 56 | `cst` | R | [x] |
| 57 | `randombytes` | T | [x] |
| 58 | `randombytes_init` | T | [x] |
| 59 | `seedexpander` | T | [x] |
| 60 | `seedexpander_init` | T | [x] |

Backend export counts, including common core and deterministic RNG exports:

| backend | C symbols | Rust symbols missing |
|---------|-----------|----------------------|
| Haraka | 56 | 0 |
| SHA-2 | 58 | 0 |
| SHAKE | 54 | 0 |
| BLAKE | 60 | 0 |

