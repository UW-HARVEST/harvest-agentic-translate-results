# Configuration-surface table

## Build-time configurations

The CMake axes and Cargo feature axes are isomorphic. A valid build selects
exactly one hash backend, one thash variant, and one parameter set:

- backend: `haraka`, `sha2`, `shake`, `blake`
- thash: `robust`, `simple`
- parameter set: `128f`, `128s`, `192f`, `192s`, `256f`, `256s`

This gives the following 48 valid combinations:

| # | Cargo features | CMake values |
|---|----------------|--------------|
| 1 | `haraka,robust,128f` | `HASH_BACKEND=haraka THASH=robust SECPAR=128f` |
| 2 | `haraka,robust,128s` | `HASH_BACKEND=haraka THASH=robust SECPAR=128s` |
| 3 | `haraka,robust,192f` | `HASH_BACKEND=haraka THASH=robust SECPAR=192f` |
| 4 | `haraka,robust,192s` | `HASH_BACKEND=haraka THASH=robust SECPAR=192s` |
| 5 | `haraka,robust,256f` | `HASH_BACKEND=haraka THASH=robust SECPAR=256f` |
| 6 | `haraka,robust,256s` | `HASH_BACKEND=haraka THASH=robust SECPAR=256s` |
| 7 | `haraka,simple,128f` | `HASH_BACKEND=haraka THASH=simple SECPAR=128f` |
| 8 | `haraka,simple,128s` | `HASH_BACKEND=haraka THASH=simple SECPAR=128s` |
| 9 | `haraka,simple,192f` | `HASH_BACKEND=haraka THASH=simple SECPAR=192f` |
| 10 | `haraka,simple,192s` | `HASH_BACKEND=haraka THASH=simple SECPAR=192s` |
| 11 | `haraka,simple,256f` | `HASH_BACKEND=haraka THASH=simple SECPAR=256f` |
| 12 | `haraka,simple,256s` | `HASH_BACKEND=haraka THASH=simple SECPAR=256s` |
| 13 | `sha2,robust,128f` | `HASH_BACKEND=sha2 THASH=robust SECPAR=128f` |
| 14 | `sha2,robust,128s` | `HASH_BACKEND=sha2 THASH=robust SECPAR=128s` |
| 15 | `sha2,robust,192f` | `HASH_BACKEND=sha2 THASH=robust SECPAR=192f` |
| 16 | `sha2,robust,192s` | `HASH_BACKEND=sha2 THASH=robust SECPAR=192s` |
| 17 | `sha2,robust,256f` | `HASH_BACKEND=sha2 THASH=robust SECPAR=256f` |
| 18 | `sha2,robust,256s` | `HASH_BACKEND=sha2 THASH=robust SECPAR=256s` |
| 19 | `sha2,simple,128f` | `HASH_BACKEND=sha2 THASH=simple SECPAR=128f` |
| 20 | `sha2,simple,128s` | `HASH_BACKEND=sha2 THASH=simple SECPAR=128s` |
| 21 | `sha2,simple,192f` | `HASH_BACKEND=sha2 THASH=simple SECPAR=192f` |
| 22 | `sha2,simple,192s` | `HASH_BACKEND=sha2 THASH=simple SECPAR=192s` |
| 23 | `sha2,simple,256f` | `HASH_BACKEND=sha2 THASH=simple SECPAR=256f` |
| 24 | `sha2,simple,256s` | `HASH_BACKEND=sha2 THASH=simple SECPAR=256s` |
| 25 | `shake,robust,128f` | `HASH_BACKEND=shake THASH=robust SECPAR=128f` |
| 26 | `shake,robust,128s` | `HASH_BACKEND=shake THASH=robust SECPAR=128s` |
| 27 | `shake,robust,192f` | `HASH_BACKEND=shake THASH=robust SECPAR=192f` |
| 28 | `shake,robust,192s` | `HASH_BACKEND=shake THASH=robust SECPAR=192s` |
| 29 | `shake,robust,256f` | `HASH_BACKEND=shake THASH=robust SECPAR=256f` |
| 30 | `shake,robust,256s` | `HASH_BACKEND=shake THASH=robust SECPAR=256s` |
| 31 | `shake,simple,128f` | `HASH_BACKEND=shake THASH=simple SECPAR=128f` |
| 32 | `shake,simple,128s` | `HASH_BACKEND=shake THASH=simple SECPAR=128s` |
| 33 | `shake,simple,192f` | `HASH_BACKEND=shake THASH=simple SECPAR=192f` |
| 34 | `shake,simple,192s` | `HASH_BACKEND=shake THASH=simple SECPAR=192s` |
| 35 | `shake,simple,256f` | `HASH_BACKEND=shake THASH=simple SECPAR=256f` |
| 36 | `shake,simple,256s` | `HASH_BACKEND=shake THASH=simple SECPAR=256s` |
| 37 | `blake,robust,128f` | `HASH_BACKEND=blake THASH=robust SECPAR=128f` |
| 38 | `blake,robust,128s` | `HASH_BACKEND=blake THASH=robust SECPAR=128s` |
| 39 | `blake,robust,192f` | `HASH_BACKEND=blake THASH=robust SECPAR=192f` |
| 40 | `blake,robust,192s` | `HASH_BACKEND=blake THASH=robust SECPAR=192s` |
| 41 | `blake,robust,256f` | `HASH_BACKEND=blake THASH=robust SECPAR=256f` |
| 42 | `blake,robust,256s` | `HASH_BACKEND=blake THASH=robust SECPAR=256s` |
| 43 | `blake,simple,128f` | `HASH_BACKEND=blake THASH=simple SECPAR=128f` |
| 44 | `blake,simple,128s` | `HASH_BACKEND=blake THASH=simple SECPAR=128s` |
| 45 | `blake,simple,192f` | `HASH_BACKEND=blake THASH=simple SECPAR=192f` |
| 46 | `blake,simple,192s` | `HASH_BACKEND=blake THASH=simple SECPAR=192s` |
| 47 | `blake,simple,256f` | `HASH_BACKEND=blake THASH=simple SECPAR=256f` |
| 48 | `blake,simple,256s` | `HASH_BACKEND=blake THASH=simple SECPAR=256s` |

## Runtime and input-shape configurations

Rows are derived from branches in the public C entry points and their direct
implementations. Each row applies to every build-time combination in which the
named symbol exists.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | four `crypto_sign_*bytes` queries | no input; compare all four configured sizes | [x] |
| 2 | `SPX_ull_to_bytes`, `SPX_bytes_to_ull` | lengths 0, 1, 2, 4, and 8; values 0, 1, and max | [x] |
| 3 | `SPX_u32_to_bytes` | values 0, 1, `0x01020304`, and `UINT32_MAX` | [x] |
| 4 | all eight address setters | zero and maximum input values on randomized prefilled addresses | [x] |
| 5 | `SPX_set_type` | documented types 0 through 6 and an out-of-range raw value | [x] |
| 6 | both address copy functions | distinct randomized source/destination words | [x] |
| 7 | `AES256_ECB` | randomized key/counter and all-zero key/counter | [x] |
| 8 | `AES256_CTR_DRBG_Update` | provided data non-null | [x] |
| 9 | `AES256_CTR_DRBG_Update` | provided data null | [x] |
| 10 | `randombytes_init`, `randombytes` | null personalization; requests 0, 1, 15, 16, 17, 48 bytes | [x] |
| 11 | `randombytes_init`, `randombytes` | non-null personalization; repeated requests and exported `DRBG_ctx` state | [x] |
| 12 | `seedexpander_init`, `seedexpander` | valid max length; requests smaller than remaining crossing 16-byte buffers | [x] |
| 13 | `blake256` | input lengths 0, 1, 54, 55, 56, 63, 64, 65, 127, 128, 129 | [x] |
| 14 | `blake512` | input lengths 0, 1, 110, 111, 112, 127, 128, 129, 255, 256, 257 | [x] |
| 15 | BLAKE incremental APIs | one-shot update versus split updates around block/padding boundaries | [x] |
| 16 | BLAKE `compress` APIs | exactly one randomized 64-byte/128-byte block from initialized state | [x] |
| 17 | both BLAKE MGF1 APIs | output lengths 0, 1, digest-1, digest, digest+1, and multiple digests | [x] |
| 18 | `SPX_initialize_hash_function` | randomized public/secret seeds | [x] |
| 19 | `SPX_prf_addr` | randomized initialized context and all address types | [x] |
| 20 | `SPX_gen_message_random` | message lengths 0, 1, block-1, block, block+1, and multi-block | [x] |
| 21 | `SPX_hash_message` | message lengths 0, 1, short-buffer threshold-1, threshold, threshold+1, multi-block | [x] |
| 22 | `SPX_thash` | `inblocks == 1` | [x] |
| 23 | `SPX_thash` | `inblocks == 2` | [x] |
| 24 | `SPX_thash` | `inblocks == SPX_WOTS_LEN` | [x] |
| 25 | `SPX_thash` | `inblocks == SPX_FORS_TREES` | [x] |
| 26 | SHA2 thash | `SPX_N < 24` and `inblocks > 1` (SHA-256 branch) | [x] |
| 27 | SHA2 thash | `SPX_N >= 24` and `inblocks > 1` (SHA-512 branch) | [x] |
| 28 | BLAKE thash | `SPX_N < 24` and `inblocks > 1` (BLAKE-256 branch) | [x] |
| 29 | BLAKE thash | `SPX_N >= 24` and `inblocks > 1` (BLAKE-512 branch) | [x] |
| 30 | Haraka thash | `inblocks == 1` specialized permutation path | [x] |
| 31 | Haraka thash | `inblocks > 1` sponge path | [x] |
| 32 | robust thash backends | bitmask/XOF path for each supported block count | [x] |
| 33 | simple thash backends | direct-input path for each supported block count | [x] |
| 34 | `SPX_chain_lengths` | all-zero, all-`0xff`, and randomized `SPX_N` messages | [x] |
| 35 | `SPX_wots_pk_from_sig` | randomized signature/message/context/address | [x] |
| 36 | `SPX_compute_root` | tree heights 1 and configured subtree height; even/odd leaf index | [x] |
| 37 | `SPX_treehash` | tree heights 0, 1, and configured subtree height with deterministic callback | [x] |
| 38 | `SPX_wots_gen_leafx1` | leaf differs from `wots_sign_leaf` | [x] |
| 39 | `SPX_wots_gen_leafx1` | leaf equals `wots_sign_leaf`, capturing WOTS signature | [x] |
| 40 | `SPX_wots_treehashx1` | randomized valid leaf and offset, with/without signature capture | [x] |
| 41 | `SPX_fors_gen_leafx1` | randomized valid leaf and address | [x] |
| 42 | `SPX_fors_treehashx1` | randomized valid leaf and offset | [x] |
| 43 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | randomized FORS message/context/address; direct sign then reconstruction | [x] |
| 44 | `SPX_merkle_gen_root` | randomized initialized secret/public seed context | [x] |
| 45 | `SPX_merkle_sign` | leaf 0, interior valid leaf, and maximum configured leaf | [x] |
| 46 | `crypto_sign_seed_keypair` | randomized fixed seeds | [x] |
| 47 | `crypto_sign_keypair` | deterministic RNG initialized with null personalization | [x] |
| 48 | `crypto_sign_signature`, `crypto_sign_verify` | messages of length 0, 1, 32, 33, and multi-block | [x] |
| 49 | `crypto_sign`, `crypto_sign_open` | messages of length 0, 1, 32, 33, and overlapping input/output-compatible buffers | [x] |
| 50 | C and Rust drivers | identical deterministic KAT seed and loop inputs; stdout byte-identical | [x] |
| 51 | SHA-256 public primitives | incremental and one-shot; padding boundary lengths 55, 56, 63, 64, 65 | [x] |
| 52 | SHA-512 public primitives | incremental and one-shot; padding boundary lengths 111, 112, 127, 128, 129 | [x] |
| 53 | SHA2 MGF1 | zero, partial-digest, exact-digest, and multi-digest output lengths | [x] |
| 54 | SHAKE public primitives | zero/partial/full-rate/multi-rate inputs and outputs; incremental and one-shot | [x] |
| 55 | Haraka public primitives | 256-bit, 512-bit, permutation, sponge, and incremental sponge entry points | [x] |
| 56 | backend exported read-only data | BLAKE `cst` contents byte-identical | [x] |
