# Configuration-surface table

The build matrix is the exact Cartesian product accepted by
`Cargo.toml [features]` and `c_src/CMakeLists.txt`: exactly one hash backend,
one thash variant, and one parameter set. Multiple selections on one axis are
not valid configurations because both implementations select a single source
or parameter header.

## Build-time combinations

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|---|---|:---:|
| 1 | all | `haraka, robust, 128s` | [x] |
| 2 | all | `haraka, robust, 128f` | [x] |
| 3 | all | `haraka, robust, 192s` | [x] |
| 4 | all | `haraka, robust, 192f` | [x] |
| 5 | all | `haraka, robust, 256s` | [x] |
| 6 | all | `haraka, robust, 256f` | [x] |
| 7 | all | `haraka, simple, 128s` | [x] |
| 8 | all | `haraka, simple, 128f` | [x] |
| 9 | all | `haraka, simple, 192s` | [x] |
| 10 | all | `haraka, simple, 192f` | [x] |
| 11 | all | `haraka, simple, 256s` | [x] |
| 12 | all | `haraka, simple, 256f` | [x] |
| 13 | all | `sha2, robust, 128s` (SHA-256 thash path) | [x] |
| 14 | all | `sha2, robust, 128f` (SHA-256 thash path) | [x] |
| 15 | all | `sha2, robust, 192s` (SHA-512 for multi-block thash) | [x] |
| 16 | all | `sha2, robust, 192f` (SHA-512 for multi-block thash) | [x] |
| 17 | all | `sha2, robust, 256s` (SHA-512 for multi-block thash) | [x] |
| 18 | all | `sha2, robust, 256f` (SHA-512 for multi-block thash) | [x] |
| 19 | all | `sha2, simple, 128s` (SHA-256 thash path) | [x] |
| 20 | all | `sha2, simple, 128f` (SHA-256 thash path) | [x] |
| 21 | all | `sha2, simple, 192s` (SHA-512 for multi-block thash) | [x] |
| 22 | all | `sha2, simple, 192f` (SHA-512 for multi-block thash) | [x] |
| 23 | all | `sha2, simple, 256s` (SHA-512 for multi-block thash) | [x] |
| 24 | all | `sha2, simple, 256f` (SHA-512 for multi-block thash) | [x] |
| 25 | all | `shake, robust, 128s` | [x] |
| 26 | all | `shake, robust, 128f` | [x] |
| 27 | all | `shake, robust, 192s` | [x] |
| 28 | all | `shake, robust, 192f` | [x] |
| 29 | all | `shake, robust, 256s` | [x] |
| 30 | all | `shake, robust, 256f` | [x] |
| 31 | all | `shake, simple, 128s` | [x] |
| 32 | all | `shake, simple, 128f` | [x] |
| 33 | all | `shake, simple, 192s` | [x] |
| 34 | all | `shake, simple, 192f` | [x] |
| 35 | all | `shake, simple, 256s` | [x] |
| 36 | all | `shake, simple, 256f` | [x] |
| 37 | all | `blake, robust, 128s` (BLAKE-256 thash path) | [x] |
| 38 | all | `blake, robust, 128f` (BLAKE-256 thash path) | [x] |
| 39 | all | `blake, robust, 192s` (BLAKE-512 for multi-block thash) | [x] |
| 40 | all | `blake, robust, 192f` (BLAKE-512 for multi-block thash) | [x] |
| 41 | all | `blake, robust, 256s` (BLAKE-512 for multi-block thash) | [x] |
| 42 | all | `blake, robust, 256f` (BLAKE-512 for multi-block thash) | [x] |
| 43 | all | `blake, simple, 128s` (BLAKE-256 thash path) | [x] |
| 44 | all | `blake, simple, 128f` (BLAKE-256 thash path) | [x] |
| 45 | all | `blake, simple, 192s` (BLAKE-512 for multi-block thash) | [x] |
| 46 | all | `blake, simple, 192f` (BLAKE-512 for multi-block thash) | [x] |
| 47 | all | `blake, simple, 256s` (BLAKE-512 for multi-block thash) | [x] |
| 48 | all | `blake, simple, 256f` (BLAKE-512 for multi-block thash) | [x] |

Parameter shapes are: `128s=(N=16,D=7,tree_height=9,FORS=12x14)`,
`128f=(16,22,3,6x33)`, `192s=(24,7,9,14x17)`,
`192f=(24,22,3,8x33)`, `256s=(32,8,8,14x22)`, and
`256f=(32,17,4,9x35)`.

## Runtime/data-shape combinations

These rows are run under every build-time combination when the named backend
entry point exists.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|---|---|:---:|
| 49 | four `crypto_sign_*bytes` queries | no input; compare all parameter-derived sizes | [x] |
| 50 | `SPX_ull_to_bytes`, `SPX_bytes_to_ull` | widths 0, 1, 2, 4, and 8; randomized values; big-endian truncation | [x] |
| 51 | `SPX_u32_to_bytes` | randomized 32-bit values including 0 and `UINT32_MAX` | [x] |
| 52 | all eight address setters | zero, in-range, and high-bit values; byte-truncating and full-width fields | [x] |
| 53 | both address copy functions | randomized source/destination bytes, including overlap-preserved fields | [x] |
| 54 | backend initialize/PRF | randomized public/secret seeds and all seven address types | [x] |
| 55 | backend message-random function | message lengths 0, 1, block-1, block, block+1, and multi-block | [x] |
| 56 | backend message hash | same message shapes; verify digest, masked tree index, and masked leaf index | [x] |
| 57 | `SPX_thash` | `inblocks=1` (F function) | [x] |
| 58 | `SPX_thash` | `inblocks=2` (Merkle/FORS internal node; wide hash when `N>=24`) | [x] |
| 59 | `SPX_thash` | `inblocks=SPX_WOTS_LEN` | [x] |
| 60 | `SPX_thash` | `inblocks=SPX_FORS_TREES` | [x] |
| 61 | `SPX_chain_lengths` | randomized `SPX_N`-byte messages, including all-zero/all-`0xff` | [x] |
| 62 | `SPX_wots_pk_from_sig` | randomized WOTS signature/message/context/address | [x] |
| 63 | `SPX_compute_root` | even leaf, nonzero offset, parameter-set tree height | [x] |
| 64 | `SPX_compute_root` | odd leaf, nonzero offset, parameter-set tree height | [x] |
| 65 | `SPX_treehash` | direct callback API, leaf indices at first/middle/last and nonzero offsets | [x] |
| 66 | `SPX_wots_gen_leafx1` | generated leaf is not the signing leaf | [x] |
| 67 | `SPX_wots_gen_leafx1` | generated leaf is the signing leaf; capture WOTS signature steps | [x] |
| 68 | `SPX_wots_treehashx1` | direct signing leaf and authentication path with nonzero offset | [x] |
| 69 | `SPX_fors_gen_leafx1` | randomized FORS address and nonzero index | [x] |
| 70 | `SPX_fors_treehashx1` | first/middle/last FORS leaf and nonzero tree offset | [x] |
| 71 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | randomized message digest/context/address; compare signature and recovered PK | [x] |
| 72 | `SPX_merkle_sign` | middle leaf; compare WOTS signature, auth path, and root | [x] |
| 73 | `SPX_merkle_gen_root` | randomized context seeds | [x] |
| 74 | `crypto_sign_seed_keypair` | randomized fixed-size seeds | [x] |
| 75 | `crypto_sign_signature`, `crypto_sign_verify` | deterministic RNG initialized identically; multi-block message | [x] |
| 76 | `crypto_sign`, `crypto_sign_open` | deterministic RNG initialized identically; overlapping message/output allocation | [x] |
| 77 | BLAKE one-shot functions | lengths 0, 1, 55/56, 63/64/65 and 111/112, 127/128/129 | [x] |
| 78 | BLAKE incremental functions | partial and multi-block updates crossing 64- and 128-byte compression boundaries; direct compression | [x] |
| 79 | BLAKE MGF1 functions | output lengths 0, 1, digest-1, digest, digest+1, and multiple digests | [x] |
| 80 | SHA-2 one-shot/incremental functions | empty, padding boundaries, one/many full blocks, and remainder finalization | [x] |
| 81 | SHA-2 MGF1/state seeding | short/exact/long outputs and both `N<24`/`N>=24` branches | [x] |
| 82 | SHAKE one-shot/incremental functions | empty, rate-1, rate, rate+1, and multi-rate absorb/squeeze | [x] |
| 83 | Haraka permutation/hash functions | 32-byte, 64-byte, incremental sponge, and partial squeeze shapes | [x] |
| 84 | deterministic RNG/AES functions | personalization absent/present; partial and many-block requests; direct AES/update; range/null boundaries | [x] |
