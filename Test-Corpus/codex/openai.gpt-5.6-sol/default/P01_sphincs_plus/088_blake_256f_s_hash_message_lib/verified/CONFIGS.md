# Configuration surface

The build-time matrix is the full cross-product accepted by both
`Cargo.toml` and CMake: exactly one hash backend, one thash mode, and one
parameter set.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | all | `haraka,robust,128f` | [ ] |
| 2 | all | `haraka,robust,128s` | [ ] |
| 3 | all | `haraka,robust,192f` | [ ] |
| 4 | all | `haraka,robust,192s` | [ ] |
| 5 | all | `haraka,robust,256f` | [ ] |
| 6 | all | `haraka,robust,256s` | [ ] |
| 7 | all | `haraka,simple,128f` | [ ] |
| 8 | all | `haraka,simple,128s` | [ ] |
| 9 | all | `haraka,simple,192f` | [ ] |
| 10 | all | `haraka,simple,192s` | [ ] |
| 11 | all | `haraka,simple,256f` | [ ] |
| 12 | all | `haraka,simple,256s` | [ ] |
| 13 | all | `sha2,robust,128f` (SHA-256 thash path) | [ ] |
| 14 | all | `sha2,robust,128s` (SHA-256 thash path) | [ ] |
| 15 | all | `sha2,robust,192f` (SHA-256 for one block, SHA-512 for multiple blocks) | [ ] |
| 16 | all | `sha2,robust,192s` (SHA-256 for one block, SHA-512 for multiple blocks) | [ ] |
| 17 | all | `sha2,robust,256f` (SHA-256 for one block, SHA-512 for multiple blocks) | [ ] |
| 18 | all | `sha2,robust,256s` (SHA-256 for one block, SHA-512 for multiple blocks) | [ ] |
| 19 | all | `sha2,simple,128f` | [ ] |
| 20 | all | `sha2,simple,128s` | [ ] |
| 21 | all | `sha2,simple,192f` | [ ] |
| 22 | all | `sha2,simple,192s` | [ ] |
| 23 | all | `sha2,simple,256f` | [ ] |
| 24 | all | `sha2,simple,256s` | [ ] |
| 25 | all | `shake,robust,128f` | [ ] |
| 26 | all | `shake,robust,128s` | [ ] |
| 27 | all | `shake,robust,192f` | [ ] |
| 28 | all | `shake,robust,192s` | [ ] |
| 29 | all | `shake,robust,256f` | [ ] |
| 30 | all | `shake,robust,256s` | [ ] |
| 31 | all | `shake,simple,128f` | [ ] |
| 32 | all | `shake,simple,128s` | [ ] |
| 33 | all | `shake,simple,192f` | [ ] |
| 34 | all | `shake,simple,192s` | [ ] |
| 35 | all | `shake,simple,256f` | [ ] |
| 36 | all | `shake,simple,256s` | [ ] |
| 37 | all | `blake,robust,128f` (BLAKE-256) | [ ] |
| 38 | all | `blake,robust,128s` (BLAKE-256) | [ ] |
| 39 | all | `blake,robust,192f` (BLAKE-256 for one block, BLAKE-512 for multiple blocks) | [ ] |
| 40 | all | `blake,robust,192s` (BLAKE-256 for one block, BLAKE-512 for multiple blocks) | [ ] |
| 41 | all | `blake,robust,256f` (BLAKE-256 for one block, BLAKE-512 for multiple blocks) | [ ] |
| 42 | all | `blake,robust,256s` (BLAKE-256 for one block, BLAKE-512 for multiple blocks) | [ ] |
| 43 | all | `blake,simple,128f` (default) | [ ] |
| 44 | all | `blake,simple,128s` | [ ] |
| 45 | all | `blake,simple,192f` | [ ] |
| 46 | all | `blake,simple,192s` | [ ] |
| 47 | all | `blake,simple,256f` | [ ] |
| 48 | all | `blake,simple,256s` | [ ] |

The following runtime rows apply under every build row above. They are the
branch-distinct public entry points and input shapes found in the headers and
source.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 49 | `crypto_sign_*bytes` | four constant-size queries | [ ] |
| 50 | `SPX_ull_to_bytes`, `SPX_bytes_to_ull` | lengths `0,1,4,8`; values `0,1,2^32-1,2^64-1` | [ ] |
| 51 | `SPX_u32_to_bytes` | values `0,1,2^32-1` | [ ] |
| 52 | address setters/copies | zero, boundary, and all-ones values; named types `0..=6` and accepted unnamed types `7,255,256,u32::MAX` | [ ] |
| 53 | `SPX_initialize_hash_function` | zero and randomized seeds; backend-specific context state | [ ] |
| 54 | `SPX_prf_addr` | randomized public/secret seeds and addresses | [ ] |
| 55 | `SPX_gen_message_random` | message lengths `0,1,block-1,block,block+1,many` | [ ] |
| 56 | `SPX_hash_message` | message lengths `0,1,block-1,block,block+1,many`; tree/leaf masks | [ ] |
| 57 | `SPX_thash` | `inblocks=1` special path | [ ] |
| 58 | `SPX_thash` | `inblocks=2` internal-node path | [ ] |
| 59 | `SPX_thash` | `inblocks=SPX_WOTS_LEN` horizontal WOTS path | [ ] |
| 60 | `SPX_thash` | `inblocks=SPX_FORS_TREES` horizontal FORS path | [ ] |
| 61 | `SPX_chain_lengths` | all-zero, all-ones, alternating, randomized `SPX_N`-byte messages | [ ] |
| 62 | `SPX_wots_pk_from_sig` | randomized signatures/messages/contexts/addresses | [ ] |
| 63 | `SPX_compute_root` | even and odd leaf indices; zero and nonzero offsets; heights `1` and configured tree height | [ ] |
| 64 | `SPX_fors_gen_leafx1` | address indices `0,1,last`; randomized context/address | [ ] |
| 65 | `SPX_fors_sign`, `SPX_fors_pk_from_sig` | all-zero, all-ones, alternating, randomized FORS messages; round-trip public key | [ ] |
| 66 | `SPX_wots_gen_leafx1` | signing leaf selected versus not selected | [ ] |
| 67 | `SPX_wots_treehashx1` | even/odd/last leaf; zero/nonzero offset; configured tree height | [ ] |
| 68 | `SPX_fors_treehashx1` | even/odd/last leaf; per-tree nonzero offset; configured FORS height | [ ] |
| 69 | `SPX_treehash` | callback-driven height `1`, `2`, configured height; even/odd leaf and offset | [ ] |
| 70 | `SPX_merkle_sign` | even/odd/last leaf and randomized root/context/address | [ ] |
| 71 | `SPX_merkle_gen_root` | randomized context | [ ] |
| 72 | `crypto_sign_seed_keypair` | all-zero, all-ones, alternating, randomized `3*SPX_N` seeds | [ ] |
| 73 | `crypto_sign_signature`, `crypto_sign_verify` | empty, one-byte, boundary, and multi-block messages; valid detached signatures | [ ] |
| 74 | `crypto_sign`, `crypto_sign_open` | empty, one-byte, boundary, and multi-block messages; valid signed-message round trip | [ ] |
| 75 | `crypto_sign_keypair` | deterministic RNG initialized with zero, personalized, and randomized 48-byte entropy | [ ] |
| 76 | `seedexpander_init`, `seedexpander` | request `0`, partial buffer, exact block, multiple blocks, sequential requests | [ ] |
| 77 | `AES256_ECB`, `AES256_CTR_DRBG_Update`, `randombytes_init`, deterministic `randombytes` | zero, all-ones, alternating, randomized keys/seeds; lengths `0,1,15,16,17,many` | [ ] |
| 78 | BLAKE public API | one-shot and incremental; byte lengths `0,1,55/56,63/64,111/112,127/128,many`; MGF output `0,1,digest-1,digest,digest+1,many` | [ ] |
| 79 | SHA-2 public API | one-shot and incremental; finalization below/at/above 56 and 112 bytes; MGF partial/full blocks; seeded state | [ ] |
| 80 | SHAKE public API | one-shot, absorb/squeezeblocks, incremental absorb/finalize/squeeze; empty, rate boundaries, partial final output | [ ] |
| 81 | Haraka public API | tweak constants; 256/512 permutations; sponge one-shot and incremental; output multiple of 32 and remainder | [ ] |

