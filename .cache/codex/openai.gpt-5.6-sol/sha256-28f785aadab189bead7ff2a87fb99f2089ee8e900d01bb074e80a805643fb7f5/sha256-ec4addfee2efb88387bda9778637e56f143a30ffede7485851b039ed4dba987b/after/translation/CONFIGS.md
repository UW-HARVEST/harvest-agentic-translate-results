# Configuration Surface

## Build-Time Matrix

The CMake axes are `HASH_BACKEND={haraka,sha2,shake,blake}`,
`THASH={robust,simple}`, and
`SECPAR={128f,128s,192f,192s,256f,256s}`. Cargo features mirror those axes and
enforce exactly one feature from each group. The full 48-combination matrix is:

| # | Cargo feature combination | `cargo check` |
|---|---------------------------|---------------|
| B01 | `haraka,robust,128f` | [x] |
| B02 | `haraka,robust,128s` | [x] |
| B03 | `haraka,robust,192f` | [x] |
| B04 | `haraka,robust,192s` | [x] |
| B05 | `haraka,robust,256f` | [x] |
| B06 | `haraka,robust,256s` | [x] |
| B07 | `haraka,simple,128f` | [x] |
| B08 | `haraka,simple,128s` | [x] |
| B09 | `haraka,simple,192f` | [x] |
| B10 | `haraka,simple,192s` | [x] |
| B11 | `haraka,simple,256f` | [x] |
| B12 | `haraka,simple,256s` | [x] |
| B13 | `sha2,robust,128f` | [x] |
| B14 | `sha2,robust,128s` | [x] |
| B15 | `sha2,robust,192f` | [x] |
| B16 | `sha2,robust,192s` | [x] |
| B17 | `sha2,robust,256f` | [x] |
| B18 | `sha2,robust,256s` | [x] |
| B19 | `sha2,simple,128f` | [x] |
| B20 | `sha2,simple,128s` | [x] |
| B21 | `sha2,simple,192f` | [x] |
| B22 | `sha2,simple,192s` | [x] |
| B23 | `sha2,simple,256f` | [x] |
| B24 | `sha2,simple,256s` | [x] |
| B25 | `shake,robust,128f` | [x] |
| B26 | `shake,robust,128s` | [x] |
| B27 | `shake,robust,192f` | [x] |
| B28 | `shake,robust,192s` | [x] |
| B29 | `shake,robust,256f` | [x] |
| B30 | `shake,robust,256s` | [x] |
| B31 | `shake,simple,128f` | [x] |
| B32 | `shake,simple,128s` | [x] |
| B33 | `shake,simple,192f` | [x] |
| B34 | `shake,simple,192s` | [x] |
| B35 | `shake,simple,256f` | [x] |
| B36 | `shake,simple,256s` | [x] |
| B37 | `blake,robust,128f` | [x] |
| B38 | `blake,robust,128s` | [x] |
| B39 | `blake,robust,192f` | [x] |
| B40 | `blake,robust,192s` | [x] |
| B41 | `blake,robust,256f` | [x] |
| B42 | `blake,robust,256s` | [x] |
| B43 | `blake,simple,128f` | [x] |
| B44 | `blake,simple,128s` | [x] |
| B45 | `blake,simple,192f` | [x] |
| B46 | `blake,simple,192s` | [x] |
| B47 | `blake,simple,256f` | [x] |
| B48 | `blake,simple,256s` | [x] |

The parameter shapes mechanically selected by the six `params-*.h` families
are:

| SECPAR | N | full height | D | subtree height | FORS height | FORS trees |
|--------|---|-------------|---|----------------|-------------|------------|
| 128f | 16 | 66 | 22 | 3 | 6 | 33 |
| 128s | 16 | 63 | 7 | 9 | 12 | 14 |
| 192f | 24 | 66 | 22 | 3 | 8 | 33 |
| 192s | 24 | 63 | 7 | 9 | 14 | 17 |
| 256f | 32 | 68 | 17 | 4 | 9 | 35 |
| 256s | 32 | 64 | 8 | 8 | 14 | 22 |

## Runtime Matrix

Every applicable row is repeated under all build combinations above. Length
sets include the exact branch boundaries found in the C loops and padding
code. Random-data rows use a fixed-seed generator and multiple samples.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| C01 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | all parameter sets; compare all size constants | [x] |
| C02 | `SPX_ull_to_bytes` | output lengths 0, 1, 4, 8, and 12; random `u64` | [x] |
| C03 | `SPX_u32_to_bytes` | random `u32`, including 0 and `UINT32_MAX` | [x] |
| C04 | `SPX_bytes_to_ull` | input lengths 0 through 8; random bytes | [x] |
| C05 | `SPX_set_layer_addr` | random address and values whose high bytes truncate | [x] |
| C06 | `SPX_set_tree_addr` | random address; 0, random, and `UINT64_MAX` | [x] |
| C07 | `SPX_set_type` | all seven documented types and out-of-range `u32` truncation | [x] |
| C08 | `SPX_set_keypair_addr` | random address and full `u32` range | [x] |
| C09 | `SPX_set_chain_addr` | random address and high-byte truncation | [x] |
| C10 | `SPX_set_hash_addr` | random address and high-byte truncation | [x] |
| C11 | `SPX_set_tree_height` | random address and high-byte truncation | [x] |
| C12 | `SPX_set_tree_index` | random address and full `u32` range | [x] |
| C13 | `SPX_copy_subtree_addr` | random source/destination, backend-specific address offsets | [x] |
| C14 | `SPX_copy_keypair_addr` | random source/destination, backend-specific address offsets | [x] |
| C15 | `SPX_initialize_hash_function` | random public/secret seeds; no-op, seeded SHA state, or tweaked Haraka constants | [x] |
| C16 | `SPX_prf_addr` | initialized context and random addresses | [x] |
| C17 | `SPX_gen_message_random` | messages of length 0, 1, block-boundary-1, block-boundary, block-boundary+1, and long | [x] |
| C18 | `SPX_hash_message` | messages crossing each backend's short/long branch; compare digest, tree, and leaf index | [x] |
| C19 | `SPX_thash` | `inblocks == 1` special case, robust and simple | [x] |
| C20 | `SPX_thash` | `inblocks == 2`, including wide-hash branch for N >= 24 | [x] |
| C21 | `SPX_thash` | `inblocks == SPX_WOTS_LEN` | [x] |
| C22 | `SPX_thash` | `inblocks == SPX_FORS_TREES` | [x] |
| C23 | `SPX_chain_lengths` | all-zero, all-`0xff`, and random N-byte messages | [x] |
| C24 | `SPX_wots_gen_leafx1` | requested leaf equals and differs from signing leaf | [x] |
| C25 | `SPX_wots_pk_from_sig` | random signatures/messages and initialized context | [x] |
| C26 | `SPX_compute_root` | even/odd leaf index; heights 1 and native subtree/FORS heights | [x] |
| C27 | `SPX_treehash` | deterministic callback; heights 1, 2, native subtree height; offset 0/nonzero | [x] |
| C28 | `SPX_wots_treehashx1` | valid leaf first/middle/last and offset 0/nonzero | [x] |
| C29 | `SPX_fors_gen_leafx1` | random initialized context/address and boundary indices | [x] |
| C30 | `SPX_fors_treehashx1` | valid leaf first/middle/last and per-tree offsets | [x] |
| C31 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | randomized messages/contexts/addresses; direct signing and reconstruction | [x] |
| C32 | `SPX_merkle_sign` | valid leaf first/middle/last in native subtree | [x] |
| C33 | `SPX_merkle_gen_root` | randomized initialized contexts | [x] |
| C34 | `crypto_sign_seed_keypair` | randomized seeds; compare complete public and secret keys | [x] |
| C35 | `crypto_sign_keypair` | deterministic RNG initialized identically before each call | [x] |
| C36 | `crypto_sign_signature` | deterministic RNG; empty, one-byte, boundary, and randomized messages | [x] |
| C37 | `crypto_sign_verify` | valid randomized detached signatures | [x] |
| C38 | `crypto_sign` | in-place/overlapping input and separate input; varied message lengths | [x] |
| C39 | `crypto_sign_open` | valid signed messages; output aliases and does not alias signed input | [x] |
| C40 | `AES256_ECB` | random keys and counter blocks | [x] |
| C41 | `AES256_CTR_DRBG_Update` | non-null random 48-byte provided data | [x] |
| C42 | `AES256_CTR_DRBG_Update` | null provided data | [x] |
| C43 | `randombytes_init`, `randombytes`, `DRBG_ctx` | null personalization; lengths 0, 1, 15, 16, 17, and multiple blocks | [x] |
| C44 | `randombytes_init`, `randombytes`, `DRBG_ctx` | non-null personalization; repeated calls and counter carry | [x] |
| C45 | `seedexpander_init` | max lengths 0, 1, block boundary, and `2^32-1`; random seed/diversifier | [x] |
| C46 | `seedexpander` | successful zero, partial-buffer, exact-block, cross-block, and repeated requests | [x] |
| C47 | `cst` | all 16 exported BLAKE constants | [x] |
| C48 | `blake256`, `blake512` | lengths 0, 1, padding boundary-1/boundary/+1, block boundary-1/boundary/+1, and multi-block | [x] |
| C49 | `blake256_init/update/final`, `blake512_init/update/final` | one-shot and split incremental updates at fill boundaries | [x] |
| C50 | `blake256_compress`, `blake512_compress` | initialized and randomized states with random full blocks | [x] |
| C51 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | output lengths 0, 1, digest-1/digest/digest+1, and multiple digests | [x] |
| C52 | `sha256`, `sha512` | lengths crossing 56/64 and 112/128 padding/block boundaries | [x] |
| C53 | `sha256_inc_init/blocks/finalize`, `sha512_inc_init/blocks/finalize` | zero/one/multiple blocks plus both final-padding branches | [x] |
| C54 | `SPX_mgf1_256`, `SPX_mgf1_512` | output lengths 0, 1, digest-1/digest/digest+1, and multiple digests | [x] |
| C55 | `SPX_seed_state` | N = 16 versus N >= 24 context layouts | [x] |
| C56 | `shake256` | input/output lengths 0, 1, rate-1/rate/rate+1, and multi-rate | [x] |
| C57 | `shake256_absorb`, `shake256_squeezeblocks` | zero/one/multiple absorb and squeeze blocks | [x] |
| C58 | `shake256_inc_init/absorb/finalize/squeeze` | split absorbs and split squeezes crossing the 136-byte rate | [x] |
| C59 | `SPX_tweak_constants` | N = 16, 24, and 32 public seeds | [x] |
| C60 | `SPX_haraka_S` | input/output lengths 0, 1, rate-1/rate/rate+1 and output remainder | [x] |
| C61 | `SPX_haraka_S_inc_init/absorb/finalize/squeeze` | split absorbs/squeezes crossing the 32-byte rate | [x] |
| C62 | `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | randomized full-width inputs and initialized contexts | [x] |

