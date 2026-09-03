# C dynamic symbol surface

Measured from the default C build (`HASH_BACKEND=blake`, `SECPAR=128f`,
`THASH=simple`) with:

```text
nm -D --defined-only ../c_src/build/app/libsphincs_core.so \
  target/c-ref/blake-simple-128f/app/libsphincs_core_det.so \
  ../c_src/build/lib/blake/libblake.so
```

The C build splits the API between the core and backend shared libraries.
The table is their deduplicated union. `Rust export` was checked against
`target/release/libsphincs_plus.so`.

| # | C symbol | C shared library | Rust export |
|---:|---|---|:---:|
| 1 | `SPX_blake256_mgf1` | `libblake.so` | yes |
| 2 | `SPX_blake512_mgf1` | `libblake.so` | yes |
| 3 | `SPX_bytes_to_ull` | both | yes |
| 4 | `SPX_chain_lengths` | `libsphincs_core.so` | yes |
| 5 | `SPX_compute_root` | `libsphincs_core.so` | yes |
| 6 | `SPX_copy_keypair_addr` | `libsphincs_core.so` | yes |
| 7 | `SPX_copy_subtree_addr` | `libsphincs_core.so` | yes |
| 8 | `SPX_fors_gen_leafx1` | `libsphincs_core.so` | yes |
| 9 | `SPX_fors_pk_from_sig` | `libsphincs_core.so` | yes |
| 10 | `SPX_fors_sign` | `libsphincs_core.so` | yes |
| 11 | `SPX_fors_treehashx1` | `libsphincs_core.so` | yes |
| 12 | `SPX_gen_message_random` | `libblake.so` | yes |
| 13 | `SPX_hash_message` | `libblake.so` | yes |
| 14 | `SPX_initialize_hash_function` | `libblake.so` | yes |
| 15 | `SPX_merkle_gen_root` | `libsphincs_core.so` | yes |
| 16 | `SPX_merkle_sign` | `libsphincs_core.so` | yes |
| 17 | `SPX_prf_addr` | `libblake.so` | yes |
| 18 | `SPX_set_chain_addr` | `libsphincs_core.so` | yes |
| 19 | `SPX_set_hash_addr` | `libsphincs_core.so` | yes |
| 20 | `SPX_set_keypair_addr` | `libsphincs_core.so` | yes |
| 21 | `SPX_set_layer_addr` | `libsphincs_core.so` | yes |
| 22 | `SPX_set_tree_addr` | `libsphincs_core.so` | yes |
| 23 | `SPX_set_tree_height` | `libsphincs_core.so` | yes |
| 24 | `SPX_set_tree_index` | `libsphincs_core.so` | yes |
| 25 | `SPX_set_type` | `libsphincs_core.so` | yes |
| 26 | `SPX_thash` | `libblake.so` | yes |
| 27 | `SPX_treehash` | `libsphincs_core.so` | yes |
| 28 | `SPX_u32_to_bytes` | both | yes |
| 29 | `SPX_ull_to_bytes` | both | yes |
| 30 | `SPX_wots_gen_leafx1` | `libsphincs_core.so` | yes |
| 31 | `SPX_wots_pk_from_sig` | `libsphincs_core.so` | yes |
| 32 | `SPX_wots_treehashx1` | `libsphincs_core.so` | yes |
| 33 | `blake256` | `libblake.so` | yes |
| 34 | `blake256_compress` | `libblake.so` | yes |
| 35 | `blake256_final` | `libblake.so` | yes |
| 36 | `blake256_init` | `libblake.so` | yes |
| 37 | `blake256_update` | `libblake.so` | yes |
| 38 | `blake512` | `libblake.so` | yes |
| 39 | `blake512_compress` | `libblake.so` | yes |
| 40 | `blake512_final` | `libblake.so` | yes |
| 41 | `blake512_init` | `libblake.so` | yes |
| 42 | `blake512_update` | `libblake.so` | yes |
| 43 | `crypto_sign` | `libsphincs_core.so` | yes |
| 44 | `crypto_sign_bytes` | `libsphincs_core.so` | yes |
| 45 | `crypto_sign_keypair` | `libsphincs_core.so` | yes |
| 46 | `crypto_sign_open` | `libsphincs_core.so` | yes |
| 47 | `crypto_sign_publickeybytes` | `libsphincs_core.so` | yes |
| 48 | `crypto_sign_secretkeybytes` | `libsphincs_core.so` | yes |
| 49 | `crypto_sign_seed_keypair` | `libsphincs_core.so` | yes |
| 50 | `crypto_sign_seedbytes` | `libsphincs_core.so` | yes |
| 51 | `crypto_sign_signature` | `libsphincs_core.so` | yes |
| 52 | `crypto_sign_verify` | `libsphincs_core.so` | yes |
| 53 | `cst` | `libblake.so` | yes |
| 54 | `randombytes` | both core libraries | yes |
| 55 | `AES256_CTR_DRBG_Update` | `libsphincs_core_det.so` | yes |
| 56 | `AES256_ECB` | `libsphincs_core_det.so` | yes |
| 57 | `DRBG_ctx` | `libsphincs_core_det.so` | yes |
| 58 | `randombytes_init` | `libsphincs_core_det.so` | yes |
| 59 | `seedexpander` | `libsphincs_core_det.so` | yes |
| 60 | `seedexpander_init` | `libsphincs_core_det.so` | yes |

Result: **60 C symbols, 0 missing from Rust**. The same zero-missing-symbol
comparison passed for all 48 build configurations.
