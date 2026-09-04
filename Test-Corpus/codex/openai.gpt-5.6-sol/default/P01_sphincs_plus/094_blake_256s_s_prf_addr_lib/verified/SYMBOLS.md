# Dynamic symbol surface

Source: the sorted union of `nm -D --defined-only` from the default
`c_src/build/app/libsphincs_core.so` and
`c_src/build/lib/blake/libblake.so` (`blake,simple,128f`). Symbols supplied by
libc or the loader are not listed. Duplicate definitions in the two C shared
objects appear once.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `SPX_blake256_mgf1` | [x] |
| 2 | `SPX_blake512_mgf1` | [x] |
| 3 | `SPX_bytes_to_ull` | [x] |
| 4 | `SPX_chain_lengths` | [x] |
| 5 | `SPX_compute_root` | [x] |
| 6 | `SPX_copy_keypair_addr` | [x] |
| 7 | `SPX_copy_subtree_addr` | [x] |
| 8 | `SPX_fors_gen_leafx1` | [x] |
| 9 | `SPX_fors_pk_from_sig` | [x] |
| 10 | `SPX_fors_sign` | [x] |
| 11 | `SPX_fors_treehashx1` | [x] |
| 12 | `SPX_gen_message_random` | [x] |
| 13 | `SPX_hash_message` | [x] |
| 14 | `SPX_initialize_hash_function` | [x] |
| 15 | `SPX_merkle_gen_root` | [x] |
| 16 | `SPX_merkle_sign` | [x] |
| 17 | `SPX_prf_addr` | [x] |
| 18 | `SPX_set_chain_addr` | [x] |
| 19 | `SPX_set_hash_addr` | [x] |
| 20 | `SPX_set_keypair_addr` | [x] |
| 21 | `SPX_set_layer_addr` | [x] |
| 22 | `SPX_set_tree_addr` | [x] |
| 23 | `SPX_set_tree_height` | [x] |
| 24 | `SPX_set_tree_index` | [x] |
| 25 | `SPX_set_type` | [x] |
| 26 | `SPX_thash` | [x] |
| 27 | `SPX_treehash` | [x] |
| 28 | `SPX_u32_to_bytes` | [x] |
| 29 | `SPX_ull_to_bytes` | [x] |
| 30 | `SPX_wots_gen_leafx1` | [x] |
| 31 | `SPX_wots_pk_from_sig` | [x] |
| 32 | `SPX_wots_treehashx1` | [x] |
| 33 | `blake256` | [x] |
| 34 | `blake256_compress` | [x] |
| 35 | `blake256_final` | [x] |
| 36 | `blake256_init` | [x] |
| 37 | `blake256_update` | [x] |
| 38 | `blake512` | [x] |
| 39 | `blake512_compress` | [x] |
| 40 | `blake512_final` | [x] |
| 41 | `blake512_init` | [x] |
| 42 | `blake512_update` | [x] |
| 43 | `crypto_sign` | [x] |
| 44 | `crypto_sign_bytes` | [x] |
| 45 | `crypto_sign_keypair` | [x] |
| 46 | `crypto_sign_open` | [x] |
| 47 | `crypto_sign_publickeybytes` | [x] |
| 48 | `crypto_sign_secretkeybytes` | [x] |
| 49 | `crypto_sign_seed_keypair` | [x] |
| 50 | `crypto_sign_seedbytes` | [x] |
| 51 | `crypto_sign_signature` | [x] |
| 52 | `crypto_sign_verify` | [x] |
| 53 | `cst` | [x] |
| 54 | `randombytes` | [x] |

The original missing symbol was `cst`. It is the actual 16-word BLAKE
constant table, not a stub, and is now exported by Rust when `blake` is
enabled.

