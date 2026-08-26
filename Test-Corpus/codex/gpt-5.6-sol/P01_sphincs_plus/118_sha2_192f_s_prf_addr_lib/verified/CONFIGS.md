# Configuration surface

## Build-time matrix

The CMake axes are `HASH_BACKEND`, `THASH`, and `SECPAR`. The implementation
has backend directories for `haraka`, `sha2`, `shake`, and `blake`, producing
4 x 2 x 6 = 48 distinct libraries. CMake's cache docstring says `shake256`,
but `add_subdirectory(${HASH_BACKEND})` and the parameter headers require the
actual C value `shake`. Cargo additionally accepts `shake256` as an alias for
`shake`, so the Rust command matrix has 60 invocations but 48 distinct
configurations.

Every build row means: run every runtime row in the next table against C and
Rust shared objects built with that configuration.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | all exports | `haraka`, `robust`, `128s` | [x] |
| 2 | all exports | `haraka`, `robust`, `128f` | [x] |
| 3 | all exports | `haraka`, `robust`, `192s` | [x] |
| 4 | all exports | `haraka`, `robust`, `192f` | [x] |
| 5 | all exports | `haraka`, `robust`, `256s` | [x] |
| 6 | all exports | `haraka`, `robust`, `256f` | [x] |
| 7 | all exports | `haraka`, `simple`, `128s` | [x] |
| 8 | all exports | `haraka`, `simple`, `128f` | [x] |
| 9 | all exports | `haraka`, `simple`, `192s` | [x] |
| 10 | all exports | `haraka`, `simple`, `192f` | [x] |
| 11 | all exports | `haraka`, `simple`, `256s` | [x] |
| 12 | all exports | `haraka`, `simple`, `256f` | [x] |
| 13 | all exports | `sha2`, `robust`, `128s` | [x] |
| 14 | all exports | `sha2`, `robust`, `128f` | [x] |
| 15 | all exports | `sha2`, `robust`, `192s` | [x] |
| 16 | all exports | `sha2`, `robust`, `192f` | [x] |
| 17 | all exports | `sha2`, `robust`, `256s` | [x] |
| 18 | all exports | `sha2`, `robust`, `256f` | [x] |
| 19 | all exports | `sha2`, `simple`, `128s` | [x] |
| 20 | all exports | `sha2`, `simple`, `128f` | [x] |
| 21 | all exports | `sha2`, `simple`, `192s` | [x] |
| 22 | all exports | `sha2`, `simple`, `192f` | [x] |
| 23 | all exports | `sha2`, `simple`, `256s` | [x] |
| 24 | all exports | `sha2`, `simple`, `256f` | [x] |
| 25 | all exports | `shake`, `robust`, `128s` (`shake256` Cargo alias too) | [x] |
| 26 | all exports | `shake`, `robust`, `128f` (`shake256` Cargo alias too) | [x] |
| 27 | all exports | `shake`, `robust`, `192s` (`shake256` Cargo alias too) | [x] |
| 28 | all exports | `shake`, `robust`, `192f` (`shake256` Cargo alias too) | [x] |
| 29 | all exports | `shake`, `robust`, `256s` (`shake256` Cargo alias too) | [x] |
| 30 | all exports | `shake`, `robust`, `256f` (`shake256` Cargo alias too) | [x] |
| 31 | all exports | `shake`, `simple`, `128s` (`shake256` Cargo alias too) | [x] |
| 32 | all exports | `shake`, `simple`, `128f` (`shake256` Cargo alias too) | [x] |
| 33 | all exports | `shake`, `simple`, `192s` (`shake256` Cargo alias too) | [x] |
| 34 | all exports | `shake`, `simple`, `192f` (`shake256` Cargo alias too) | [x] |
| 35 | all exports | `shake`, `simple`, `256s` (`shake256` Cargo alias too) | [x] |
| 36 | all exports | `shake`, `simple`, `256f` (`shake256` Cargo alias too) | [x] |
| 37 | all exports | `blake`, `robust`, `128s` | [x] |
| 38 | all exports | `blake`, `robust`, `128f` | [x] |
| 39 | all exports | `blake`, `robust`, `192s` | [x] |
| 40 | all exports | `blake`, `robust`, `192f` | [x] |
| 41 | all exports | `blake`, `robust`, `256s` | [x] |
| 42 | all exports | `blake`, `robust`, `256f` | [x] |
| 43 | all exports | `blake`, `simple`, `128s` | [x] |
| 44 | all exports | `blake`, `simple`, `128f` (default) | [x] |
| 45 | all exports | `blake`, `simple`, `192s` | [x] |
| 46 | all exports | `blake`, `simple`, `192f` | [x] |
| 47 | all exports | `blake`, `simple`, `256s` | [x] |
| 48 | all exports | `blake`, `simple`, `256f` | [x] |

## Runtime and input-shape matrix

These rows are the branch-derived runtime cross-section. Fixed-seed randomized
values are required in addition to each listed boundary. Backend-only rows are
run for every build row selecting that backend. The two core variants are both
loaded where their `randombytes` behavior differs.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| R1 | `crypto_sign_{secretkeybytes,publickeybytes,bytes,seedbytes}` | all parameter sets; compare all four constants | [x] |
| R2 | `SPX_ull_to_bytes`, `SPX_bytes_to_ull` | lengths `0,1,4,8`; random values; big-endian truncation | [x] |
| R3 | `SPX_u32_to_bytes` | `0,1,0xff,0x100,UINT32_MAX` and random values | [x] |
| R4 | `SPX_set_layer_addr`, `SPX_set_type`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height` | `0,1,0xff,0x100,UINT32_MAX`; byte truncation; SHA2/plain offsets | [x] |
| R5 | `SPX_set_tree_addr`, `SPX_set_keypair_addr`, `SPX_set_tree_index` | zero, boundary, maximum, and random 32/64-bit values | [x] |
| R6 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | randomized full addresses; SHA2/plain copy spans | [x] |
| R7 | `blake256` | message lengths `0,1,54,55,56,63,64,65,127,128,129` and random bytes | [x] |
| R8 | `blake256_{init,update,final}` | same messages split at `0,1,63,64,65`; buffered/full-block/final padding branches | [x] |
| R9 | `blake256_compress` | initialized and randomized state; one 64-byte block | [x] |
| R10 | `SPX_blake256_mgf1` | input `0,1,SPX_N`; output `0,1,31,32,33,64,65` | [x] |
| R11 | `blake512` | message lengths `0,1,110,111,112,127,128,129,255,256,257` and random bytes | [x] |
| R12 | `blake512_{init,update,final}` | same messages split at `0,1,127,128,129`; buffered/full-block/final padding branches | [x] |
| R13 | `blake512_compress` | initialized and randomized state; one 128-byte block | [x] |
| R14 | `SPX_blake512_mgf1` | input `0,1,SPX_N`; output `0,1,63,64,65,128,129` | [x] |
| R15 | `sha256`, `sha256_inc_{init,blocks,finalize}` | lengths around `55/56/63/64/65`; zero/one/many blocks and randomized chunking | [x] |
| R16 | `sha512`, `sha512_inc_{init,blocks,finalize}` | lengths around `111/112/127/128/129`; zero/one/many blocks and randomized chunking | [x] |
| R17 | `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state` | zero/full/partial output blocks; both `N=16` and `N>=24` parameter families | [x] |
| R18 | `shake256` | input around rate `0,1,r-1,r,r+1,2r`; output `0,1,r-1,r,r+1` | [x] |
| R19 | `shake256_{absorb,squeezeblocks}` | zero/one/many absorb and squeeze blocks | [x] |
| R20 | `shake256_inc_{init,absorb,finalize,squeeze}` | empty and randomized multi-call chunks crossing the rate | [x] |
| R21 | `SPX_tweak_constants`, `SPX_haraka{256,512,512_perm}` | random seeds/blocks; initialized Haraka context | [x] |
| R22 | `SPX_haraka_S` | input/output `0,1,31,32,33,rate-1,rate,rate+1` | [x] |
| R23 | `SPX_haraka_S_inc_{init,absorb,finalize,squeeze}` | empty and randomized chunks crossing the rate | [x] |
| R24 | `SPX_initialize_hash_function` | randomized public/secret seed context; compare complete backend-specific context bytes | [x] |
| R25 | `SPX_prf_addr` | randomized context and all address fields | [x] |
| R26 | `SPX_gen_message_random` | message `0,1,block-N-1,block-N,block-N+1,2block`; random seeds/optrand | [x] |
| R27 | `SPX_hash_message` | message `0,1,block-(N+PK)-1`, boundary, above boundary, multi-block; digest/tree/leaf masks | [x] |
| R28 | `SPX_thash` | `inblocks=0,1,2,SPX_WOTS_LEN,SPX_FORS_TREES`; simple/robust and `N=16/24/32` branches | [x] |
| R29 | `SPX_chain_lengths` | all-zero, all-`0xff`, and randomized `SPX_N` messages | [x] |
| R30 | `SPX_wots_pk_from_sig` | randomized signature/message/context/address | [x] |
| R31 | `SPX_wots_gen_leafx1` | signing leaf equals/does not equal `leaf_idx`; null/non-null signature buffer | [x] |
| R32 | `SPX_compute_root` | even/odd leaf; zero/nonzero offset; heights `1` and `SPX_TREE_HEIGHT` | [x] |
| R33 | `SPX_treehash` | deterministic callback; heights `1,2,SPX_TREE_HEIGHT`; first/middle/last leaf and offsets | [x] |
| R34 | `SPX_wots_treehashx1`, `SPX_fors_treehashx1` | heights `1` and native height; first/middle/last leaf; signing/non-signing leaf | [x] |
| R35 | `SPX_fors_gen_leafx1` | zero, boundary, and randomized leaf indices/address | [x] |
| R36 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | randomized messages/contexts/addresses; recovered PK equals generated PK | [x] |
| R37 | `SPX_merkle_sign` | first/middle/last leaf; randomized context/tree address | [x] |
| R38 | `SPX_merkle_gen_root` | randomized context for each parameter/backend combination | [x] |
| R39 | `seedexpander_init` | `maxlen=0,1,15,16,17,UINT32_MAX`; random seed/diversifier | [x] |
| R40 | `seedexpander` | requests `0,1,15,16,17`, buffered and counter-wrap states, multiple calls | [x] |
| R41 | `AES256_ECB` | randomized keys and counters | [x] |
| R42 | `AES256_CTR_DRBG_Update` | `provided_data=NULL` and randomized 48-byte data; counter carry states | [x] |
| R43 | `randombytes_init` | null and randomized personalization strings | [x] |
| R44 | deterministic `randombytes` | lengths `0,1,15,16,17,31,32,33`; repeated calls after identical initialization | [x] |
| R45 | system `randombytes` | lengths `0,1,1048575,1048576,1048577`; completion/write extent only (bytes are nondeterministic) | [x] |
| R46 | `crypto_sign_seed_keypair` | fixed-seed randomized seeds; compare PK/SK byte-for-byte | [x] |
| R47 | `crypto_sign_keypair` | identically initialized deterministic DRBG; compare PK/SK byte-for-byte | [x] |
| R48 | `crypto_sign_signature`, `crypto_sign_verify` | messages `0,1,block boundary,many`; deterministic DRBG; valid and altered message/signature/key | [x] |
| R49 | `crypto_sign`, `crypto_sign_open` | same message shapes; in-place/overlapping source and destination permitted by `memmove` | [x] |
| R50 | `DRBG_ctx`, backend constant globals (`cst` where exported) | initial and post-operation object bytes | [x] |

`fips202.h` also declares SHAKE128 and SHA3 functions, but `fips202.c` defines
none of them and `nm -D` confirms that they are not shared-library entry
points. They therefore cannot be invoked through either C `.so`.
