# Configuration surface

The build axes come directly from `Cargo.toml`, top-level `CMakeLists.txt`, and
the 24 parameter headers. A valid build selects exactly one backend, one thash
mode, and one parameter set. This yields 48 combinations.

Parameter shapes:

| set | `N` | full height | layers `D` | tree height | FORS height | FORS trees |
|---|---:|---:|---:|---:|---:|---:|
| `128s` | 16 | 63 | 7 | 9 | 12 | 14 |
| `128f` | 16 | 66 | 22 | 3 | 6 | 33 |
| `192s` | 24 | 63 | 7 | 9 | 14 | 17 |
| `192f` | 24 | 66 | 22 | 3 | 8 | 33 |
| `256s` | 32 | 64 | 8 | 8 | 14 | 22 |
| `256f` | 32 | 68 | 17 | 4 | 9 | 35 |

## Build cross-product

Each row exercises all applicable API-shape rows below under that build.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C001 | all | `blake,simple,128s` | [x] |
| C002 | all | `blake,simple,128f` | [x] |
| C003 | all | `blake,simple,192s` | [x] |
| C004 | all | `blake,simple,192f` | [x] |
| C005 | all | `blake,simple,256s` | [x] |
| C006 | all | `blake,simple,256f` | [x] |
| C007 | all | `blake,robust,128s` | [x] |
| C008 | all | `blake,robust,128f` | [x] |
| C009 | all | `blake,robust,192s` | [x] |
| C010 | all | `blake,robust,192f` | [x] |
| C011 | all | `blake,robust,256s` | [x] |
| C012 | all | `blake,robust,256f` | [x] |
| C013 | all | `haraka,simple,128s` | [x] |
| C014 | all | `haraka,simple,128f` | [x] |
| C015 | all | `haraka,simple,192s` | [x] |
| C016 | all | `haraka,simple,192f` | [x] |
| C017 | all | `haraka,simple,256s` | [x] |
| C018 | all | `haraka,simple,256f` | [x] |
| C019 | all | `haraka,robust,128s` | [x] |
| C020 | all | `haraka,robust,128f` | [x] |
| C021 | all | `haraka,robust,192s` | [x] |
| C022 | all | `haraka,robust,192f` | [x] |
| C023 | all | `haraka,robust,256s` | [x] |
| C024 | all | `haraka,robust,256f` | [x] |
| C025 | all | `sha2,simple,128s` (SHA-256 path, 22-byte compressed address) | [x] |
| C026 | all | `sha2,simple,128f` (SHA-256 path, 22-byte compressed address) | [x] |
| C027 | all | `sha2,simple,192s` (SHA-512 path, 22-byte compressed address) | [x] |
| C028 | all | `sha2,simple,192f` (SHA-512 path, 22-byte compressed address) | [x] |
| C029 | all | `sha2,simple,256s` (SHA-512 path, 22-byte compressed address) | [x] |
| C030 | all | `sha2,simple,256f` (SHA-512 path, 22-byte compressed address) | [x] |
| C031 | all | `sha2,robust,128s` (SHA-256 path, 22-byte compressed address) | [x] |
| C032 | all | `sha2,robust,128f` (SHA-256 path, 22-byte compressed address) | [x] |
| C033 | all | `sha2,robust,192s` (SHA-512 path, 22-byte compressed address) | [x] |
| C034 | all | `sha2,robust,192f` (SHA-512 path, 22-byte compressed address) | [x] |
| C035 | all | `sha2,robust,256s` (SHA-512 path, 22-byte compressed address) | [x] |
| C036 | all | `sha2,robust,256f` (SHA-512 path, 22-byte compressed address) | [x] |
| C037 | all | `shake,simple,128s` | [x] |
| C038 | all | `shake,simple,128f` | [x] |
| C039 | all | `shake,simple,192s` | [x] |
| C040 | all | `shake,simple,192f` | [x] |
| C041 | all | `shake,simple,256s` | [x] |
| C042 | all | `shake,simple,256f` | [x] |
| C043 | all | `shake,robust,128s` | [x] |
| C044 | all | `shake,robust,128f` | [x] |
| C045 | all | `shake,robust,192s` | [x] |
| C046 | all | `shake,robust,192f` | [x] |
| C047 | all | `shake,robust,256s` | [x] |
| C048 | all | `shake,robust,256f` | [x] |

## Public API and data-shape branches

These rows are derived from public headers, dynamic exports, and explicit
`if`/loop branches in the C implementation. Fixed-seed randomized vectors are
used for each row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C049 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | constant queries for the selected parameter set | [x] |
| C050 | `SPX_ull_to_bytes`, `SPX_u32_to_bytes`, `SPX_bytes_to_ull` | lengths `0,1,4,8`; zero, one, high-bit, and max values; big-endian truncation | [x] |
| C051 | all `SPX_set_*_addr` | zero, in-range, and values with nonzero discarded high bytes | [x] |
| C052 | `SPX_set_tree_addr` | zero, one, random, and `UINT64_MAX` | [x] |
| C053 | `SPX_set_type` | each documented type `0..=6` and arbitrary/out-of-range `uint32_t` values (low-byte storage) | [x] |
| C054 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random source/destination including bytes outside copied fields | [x] |
| C055 | `SPX_chain_lengths` | all-zero, all-`0xff`, alternating, and randomized `N`-byte messages | [x] |
| C056 | `SPX_initialize_hash_function` | zero/random public and secret seeds; backend-specific seeded context | [x] |
| C057 | `SPX_prf_addr` | randomized context and address | [x] |
| C058 | `SPX_gen_message_random` | message lengths `0,1,block-1,block,block+1,many`; SHA2 short/long branch | [x] |
| C059 | `SPX_hash_message` | message lengths `0,1,block-boundaries,many`; digest/tree/leaf masking | [x] |
| C060 | `SPX_thash` | `inblocks=1` special case | [x] |
| C061 | `SPX_thash` | `inblocks=2` tree-node case | [x] |
| C062 | `SPX_thash` | `inblocks=SPX_WOTS_LEN` WOTS compression | [x] |
| C063 | `SPX_thash` | `inblocks=SPX_FORS_TREES` FORS compression | [x] |
| C064 | `SPX_compute_root` | even/odd leaf, nonzero offset, tree heights `1` and selected tree height | [x] |
| C065 | `SPX_wots_pk_from_sig` | zero/extreme/random message chain lengths and randomized signature | [x] |
| C066 | `SPX_wots_gen_leafx1` | leaf is not signing leaf (benign branch) | [x] |
| C067 | `SPX_wots_gen_leafx1` | leaf equals signing leaf; captures chain step values including `0` and `W-1` | [x] |
| C068 | `SPX_fors_gen_leafx1` | randomized context/address/index | [x] |
| C069 | `SPX_wots_treehashx1`, `SPX_fors_treehashx1`, `SPX_treehash` | left/right authentication siblings, offset zero/nonzero, selected tree/FORS heights | [x] |
| C070 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | zero, all-ones, alternating, and randomized FORS messages; recovered public key | [x] |
| C071 | `SPX_merkle_sign`, `SPX_merkle_gen_root` | boundary and randomized leaf indices | [x] |
| C072 | `crypto_sign_seed_keypair` | zero, incrementing, all-`0xff`, and randomized `3*N` seeds | [x] |
| C073 | `crypto_sign_signature`, `crypto_sign_verify` | valid detached signatures; message lengths `0,1,32,33,block-boundaries,many` | [x] |
| C074 | `crypto_sign`, `crypto_sign_open` | attached signatures with disjoint input/output buffers | [x] |
| C075 | `crypto_sign`, `crypto_sign_open` | overlapping/in-place buffers (C uses `memmove`) | [x] |
| C076 | `randombytes` | output lengths `0,1,15,16,17,many` | [x] |
| C077 | `seedexpander_init`, `seedexpander` | buffered reads `<16`, `=16`, `>16`, repeated reads, counter carry | [x] |
| C078 | BLAKE exports (`blake256*`, `blake512*`, `SPX_blake*_mgf1`, `cst`) | empty, partial-block, exact-block, multi-block, final-padding boundary, partial MGF output | [x] |
| C079 | SHA2 exports (`sha256*`, `sha512*`, `SPX_mgf1_*`, `SPX_seed_state`) | empty, partial/exact/multi-block, both finalize padding branches, partial MGF output | [x] |
| C080 | SHAKE exports (`shake256*`) | one-shot/incremental, empty, rate-1/rate/rate+1, partial and multi-block squeeze (the header declares SHAKE128/SHA3 names, but this C source/build does not define or export them) | [x] |
| C081 | Haraka exports (`SPX_tweak_constants`, `SPX_haraka*`) | one-shot/incremental, `inblocks=1` special paths, output multiple/nonmultiple of 32 | [x] |
