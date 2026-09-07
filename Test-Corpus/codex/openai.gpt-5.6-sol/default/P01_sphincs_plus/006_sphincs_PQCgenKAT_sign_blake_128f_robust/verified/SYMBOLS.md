# Dynamic symbol surface

Source command:

```text
{ nm -D --defined-only ../c_src/build/app/libsphincs_core.so;
  nm -D --defined-only ../c_src/build/lib/blake/libblake.so; } |
awk '$2 ~ /^[TRDBW]$/ {print $3}' | sort -u
```

The CMake project intentionally splits the default library into the core and
`blake` backend shared objects. This table is their de-duplicated exported
symbol union. `cst` is the exported BLAKE-512 constant table; all other entries
are functions.

The `symbol_parity` integration test repeats this union/diff mechanically for
the selected backend in every one of the 48 feature combinations. All diffs
are empty.

| C symbol | Rust export |
|---|---|
| `SPX_blake256_mgf1` | [x] |
| `SPX_blake512_mgf1` | [x] |
| `SPX_bytes_to_ull` | [x] |
| `SPX_chain_lengths` | [x] |
| `SPX_compute_root` | [x] |
| `SPX_copy_keypair_addr` | [x] |
| `SPX_copy_subtree_addr` | [x] |
| `SPX_fors_gen_leafx1` | [x] |
| `SPX_fors_pk_from_sig` | [x] |
| `SPX_fors_sign` | [x] |
| `SPX_fors_treehashx1` | [x] |
| `SPX_gen_message_random` | [x] |
| `SPX_hash_message` | [x] |
| `SPX_initialize_hash_function` | [x] |
| `SPX_merkle_gen_root` | [x] |
| `SPX_merkle_sign` | [x] |
| `SPX_prf_addr` | [x] |
| `SPX_set_chain_addr` | [x] |
| `SPX_set_hash_addr` | [x] |
| `SPX_set_keypair_addr` | [x] |
| `SPX_set_layer_addr` | [x] |
| `SPX_set_tree_addr` | [x] |
| `SPX_set_tree_height` | [x] |
| `SPX_set_tree_index` | [x] |
| `SPX_set_type` | [x] |
| `SPX_thash` | [x] |
| `SPX_treehash` | [x] |
| `SPX_u32_to_bytes` | [x] |
| `SPX_ull_to_bytes` | [x] |
| `SPX_wots_gen_leafx1` | [x] |
| `SPX_wots_pk_from_sig` | [x] |
| `SPX_wots_treehashx1` | [x] |
| `blake256` | [x] |
| `blake256_compress` | [x] |
| `blake256_final` | [x] |
| `blake256_init` | [x] |
| `blake256_update` | [x] |
| `blake512` | [x] |
| `blake512_compress` | [x] |
| `blake512_final` | [x] |
| `blake512_init` | [x] |
| `blake512_update` | [x] |
| `crypto_sign` | [x] |
| `crypto_sign_bytes` | [x] |
| `crypto_sign_keypair` | [x] |
| `crypto_sign_open` | [x] |
| `crypto_sign_publickeybytes` | [x] |
| `crypto_sign_secretkeybytes` | [x] |
| `crypto_sign_seed_keypair` | [x] |
| `crypto_sign_seedbytes` | [x] |
| `crypto_sign_signature` | [x] |
| `crypto_sign_verify` | [x] |
| `cst` | [x] |
| `randombytes` | [x] |
