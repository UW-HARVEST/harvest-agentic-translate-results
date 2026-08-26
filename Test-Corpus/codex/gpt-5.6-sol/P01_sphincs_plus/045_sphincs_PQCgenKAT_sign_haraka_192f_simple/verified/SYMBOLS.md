# Dynamic symbol surface

Mechanically captured with `nm -D --defined-only` from the default C build:

- `c_src/build/lib/blake/libblake.so`
- `c_src/build/app/libsphincs_core.so`
- `c_src/build/app/libsphincs_core_det.so`

The table is the sorted union of global text/data/rodata symbols. `backend`
means `libblake.so`, `core` means both core variants, and `det` means only
`libsphincs_core_det.so`.

| # | C symbol | C provider | Rust default `.so` |
|---|----------|------------|--------------------|
| 1 | `AES256_CTR_DRBG_Update` | det | present |
| 2 | `AES256_ECB` | det | present |
| 3 | `DRBG_ctx` | det | present |
| 4 | `SPX_blake256_mgf1` | backend | present |
| 5 | `SPX_blake512_mgf1` | backend | present |
| 6 | `SPX_bytes_to_ull` | backend, core | present |
| 7 | `SPX_chain_lengths` | core | present |
| 8 | `SPX_compute_root` | backend, core | present |
| 9 | `SPX_copy_keypair_addr` | core | present |
| 10 | `SPX_copy_subtree_addr` | core | present |
| 11 | `SPX_fors_gen_leafx1` | core | present |
| 12 | `SPX_fors_pk_from_sig` | core | present |
| 13 | `SPX_fors_sign` | core | present |
| 14 | `SPX_fors_treehashx1` | core | present |
| 15 | `SPX_gen_message_random` | backend | present |
| 16 | `SPX_hash_message` | backend | present |
| 17 | `SPX_initialize_hash_function` | backend | present |
| 18 | `SPX_merkle_gen_root` | core | present |
| 19 | `SPX_merkle_sign` | core | present |
| 20 | `SPX_prf_addr` | backend | present |
| 21 | `SPX_set_chain_addr` | core | present |
| 22 | `SPX_set_hash_addr` | core | present |
| 23 | `SPX_set_keypair_addr` | core | present |
| 24 | `SPX_set_layer_addr` | core | present |
| 25 | `SPX_set_tree_addr` | core | present |
| 26 | `SPX_set_tree_height` | core | present |
| 27 | `SPX_set_tree_index` | core | present |
| 28 | `SPX_set_type` | core | present |
| 29 | `SPX_thash` | backend | present |
| 30 | `SPX_treehash` | backend, core | present |
| 31 | `SPX_u32_to_bytes` | backend, core | present |
| 32 | `SPX_ull_to_bytes` | backend, core | present |
| 33 | `SPX_wots_gen_leafx1` | core | present |
| 34 | `SPX_wots_pk_from_sig` | core | present |
| 35 | `SPX_wots_treehashx1` | core | present |
| 36 | `blake256` | backend | present |
| 37 | `blake256_compress` | backend | present |
| 38 | `blake256_final` | backend | present |
| 39 | `blake256_init` | backend | present |
| 40 | `blake256_update` | backend | present |
| 41 | `blake512` | backend | present |
| 42 | `blake512_compress` | backend | present |
| 43 | `blake512_final` | backend | present |
| 44 | `blake512_init` | backend | present |
| 45 | `blake512_update` | backend | present |
| 46 | `crypto_sign` | core | present |
| 47 | `crypto_sign_bytes` | core | present |
| 48 | `crypto_sign_keypair` | core | present |
| 49 | `crypto_sign_open` | core | present |
| 50 | `crypto_sign_publickeybytes` | core | present |
| 51 | `crypto_sign_secretkeybytes` | core | present |
| 52 | `crypto_sign_seed_keypair` | core | present |
| 53 | `crypto_sign_seedbytes` | core | present |
| 54 | `crypto_sign_signature` | core | present |
| 55 | `crypto_sign_verify` | core | present |
| 56 | `cst` | backend | present |
| 57 | `randombytes` | core, det | present |
| 58 | `randombytes_init` | det | present |
| 59 | `seedexpander` | det | present |
| 60 | `seedexpander_init` | det | present |

Default comparison:

- C union: 60 symbols
- Rust: 60 symbols
- Missing from Rust: 0
- Extra in Rust: 0
- Undefined non-libc/non-runtime symbols in Rust: 0

The C core libraries intentionally leave `SPX_gen_message_random`,
`SPX_hash_message`, `SPX_initialize_hash_function`, `SPX_prf_addr`, and
`SPX_thash` unresolved for the selected backend shared object to provide.

- [x] Final symbol parity holds for every configuration in `CONFIGS.md`.
