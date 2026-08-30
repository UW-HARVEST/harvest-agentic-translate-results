# CONFIGS.md — the configuration-surface table (valid inputs)

The mirror image of `ERRORS.md`: every axis the C code actually branches on for
*valid* input, enumerated from the sources, and the pruned cross-product of
those axes.

## Axis 1 — build-time configuration (the CMake cache variables)

`c_src/CMakeLists.txt` exposes three cache variables; `Cargo.toml` mirrors them
as features and `build.rs` resolves them into `spx_backend` / `spx_thash` /
`spx_secpar`.

| variable | values | what it switches in the C |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | which `lib/<backend>` is compiled and linked (`hash_*.c`, `thash_*`, the primitive); which `<backend>_offsets.h` supplies the address-field offsets — **`sha2` uses a different, compressed layout** (`LAYER 0, TREE 1, TYPE 9, KP 10, CHAIN/HGT 17, INDEX 18, HASH 21`) than the other three (`3 / 8 / 19 / 20 / 27 / 28 / 31`); and the `-D{BLAKE,HARAKA,SHA2,SHAKE}_TR=1` define |
| `THASH` | `robust`, `simple` | `thash_<backend>_robust.c` (bitmask-XOR construction, one extra MGF1/sponge call) vs `thash_<backend>_simple.c` |
| `SECPAR` | `128s`, `128f`, `192s`, `192f`, `256s`, `256f` | `SPX_N` ∈ {16,24,32}, `SPX_FULL_HEIGHT` ∈ {63,66,64,68}, `SPX_D` ∈ {7,22,8,17}, `SPX_FORS_HEIGHT` ∈ {12,6,14,8,14,9}, `SPX_FORS_TREES` ∈ {14,33,17,33,22,35}; and hence `SPX_TREE_HEIGHT = FULL_HEIGHT/D` ∈ {9,3,9,3,8,4}, `SPX_WOTS_LEN = 2·N+3` ∈ {35,51,67}, `SPX_FORS_MSG_BYTES`, `SPX_BYTES` |

Two derived build-time switches follow from `SECPAR` and are separate code
paths, not just constants:

* `SPX_SHA512` (`sha2` sets it to 1 for 192/256): `hash_sha2.c` selects the
  `sha512_*` family for `gen_message_random` / `hash_message`, `seed_state`
  additionally seeds `state_seeded_512` (making `sizeof(spx_ctx)` 72 bytes
  larger), and `thash_sha2_*.c` routes `inblocks > 1` through a SHA-512 variant.
* `SPX_BLAKE512` (`blake` sets it to 1 for 192/256): `thash_blake_*.c` routes
  `inblocks > 1` to a static `thash_512`, and `hash_blake.c` (`SPX_N >= 24`)
  switches the whole `blakeX_*` family to BLAKE-512.

⇒ **48 build configurations**, each exercised in full. `verif/build_c_all.sh`
builds all 48 C variants; `verif/cargo_all.sh check` / `verif/run_tests.sh`
iterate the matching 48 Cargo feature sets.

## Axis 2 — runtime options / state the public API can set

| option | states it toggles |
|---|---|
| `spx_ctx.pub_seed`, `spx_ctx.sk_seed` | every hash is keyed by them |
| `initialize_hash_function(ctx)` | `sha2`: fills `state_seeded` (+ `state_seeded_512`); `haraka`: fills `tweaked512_rc64` / `tweaked256_rc32`; `shake`/`blake`: no-op. Different ctx *layout and size* per backend |
| `randombytes_init(entropy, personalization)` | DRBG `Key`/`V`/`reseed_counter`; `personalization == NULL` is a separate branch |
| `leaf_info_x1.wots_sign_leaf` | `wotsx1.c:28`: `leaf_idx == wots_sign_leaf` ⇒ also emit the WOTS signature (`wots_k_mask = 0`); otherwise public key only (`wots_k_mask = ~0`). `merkle_gen_root` deliberately passes `~0u` |
| `leaf_info_x1.wots_steps` | which chain position is captured into `wots_sig` |
| address `type` field | 7 documented values (`WOTS 0, WOTSPK 1, HASHTREE 2, FORSTREE 3, FORSPK 4, WOTSPRF 5, FORSPRF 6`) — plus any other `uint32_t` (see `ERRORS.md`) |
| `idx_offset` | `treehash`/`compute_root`/`*_treehashx1` shift it right each level; `0` vs non-zero vs a multiple of `2^tree_height` are different index arithmetic |
| `tree_height` | loop trip counts in `compute_root` (`tree_height-1` iterations then one more) and `treehash` (`2^tree_height` leaves) |
| `inblocks` | `thash`: `== 1` vs `> 1` is an explicit branch for `haraka` (F-function vs sponge) and for `sha2`/`blake` when `SPX_SHA512`/`SPX_BLAKE512` is 1 |
| `mlen` | `hash_sha2.c:94` `SPX_N + mlen < SPX_SHAX_BLOCK_BYTES` and `hash_sha2.c:156` `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*BLOCK` select "finalize only" vs "absorb a full block first". For every backend, `mlen` also crosses the primitive's own block/rate boundary |
| `seedexpander` `maxlen` / request sizes | `rng.c:76` `xlen <= 16 - buffer_pos` returns from the buffer, otherwise a fresh AES block is generated and `ctr[12..16]` is incremented |
| `smlen` in `crypto_sign_open` | `>= SPX_BYTES` vs `< SPX_BYTES`; `== SPX_BYTES` yields `*mlen == 0` |

`hash_*.c` also contains `if (SPX_D == 1) *tree = 0;` — unreachable for every
shipped parameter set (`min SPX_D == 7`), so it is a dead branch rather than a
row.

## Axis 3 — input shapes that are special-cased

* message length: `0, 1, 2, 7, 8, 15..17, 31..33, 47..49, 55, 56, 63..65, 71,
  72, 79, 80, 87, 88, 95, 96, 103, 104, 111, 112, 119, 120, 127..129, 135..137,
  200, 255..257, 500, 1000, 1023..1025` (SHA-256 block 64; SHA-512 / BLAKE-512
  block 128; SHAKE-256 rate 136; HARAKA-S rate 32; BLAKE-256 padding boundary
  55/56; BLAKE-512 padding boundary 111/112)
* `outlen` for the MGF1 / squeeze functions: `1`, `blk-1`, `blk`, `blk+1`,
  `2·blk`, `2·blk+1`, `5·blk+3`, and the real `SPX_DGST_BYTES`
* `inblocks`: `1, 2, 3, 4, 5, 8, 16, SPX_WOTS_LEN, SPX_FORS_TREES` (+ `0` and
  `max+1`, see `ERRORS.md`)
* `leaf_idx` parity (even ⇒ node is a left child, odd ⇒ right child — an
  explicit `if (leaf_idx & 1)` in `compute_root`), the two extremes `0` and
  `2^h - 1`, and values outside the tree
* `tree_index` / `keypair` / `tree` 32- and 64-bit fields at `0`, `0xff`,
  `0x100`, `0xffffffff`, `UINT64_MAX`
* WOTS message bytes all-`0x00` (chain lengths all 0 ⇒ 15 hash steps each) and
  all-`0xff` (chain lengths all 15 ⇒ 0 steps), plus random
* incremental hashing: 1–5 chunks of random length, and 1–4 successive squeezes

## Row table

Every row is executed for **all 48 build configurations** (Axis 1), with many
randomized inputs per row (fixed seeds — see the `Rng::new(...)` calls).
`✔` means it passed for all 48.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `SPX_ull_to_bytes` | `outlen` = 0..16 × {200 random `u64`} ∪ {0, 1, 0xff, 0x100, 2^63, MAX-1, MAX} | `diff_utils_address.rs::ull_to_bytes_all_outlens` | [x] |
| 2 | `SPX_u32_to_bytes` | 2000 random `u32` + {0, 1, 0xff, 0x100, 0xffff, 0x80000000, 0xffffffff} | `diff_utils_address.rs::u32_to_bytes_random` | [x] |
| 3 | `SPX_bytes_to_ull` | `inlen` = 0..8 × {300 random inputs} ∪ {all-00, all-ff, all-80, all-01} | `diff_utils_address.rs::bytes_to_ull_all_inlens` | [x] |
| 4 | all 10 `SPX_set_*` / `SPX_copy_*_addr` | each setter × {0..8, 0xff, 0x100, 0x101, 0xffff, 0xffffff00, 0xffffffff, 0x7fffffff, 0x80000000} ∪ 500 random, applied to a **random pre-existing address** so the untouched bytes are checked too; `set_tree_addr` additionally with 64-bit values; both `copy_*` with random src/dst | `diff_utils_address.rs::address_setters_random_and_boundary` | [x] |
| 5 | the `SPX_set_*` family, **composed** | the exact mutation sequence the signer performs (layer → tree → type → keypair → copy_subtree → copy_keypair → chain → hash → tree_height → tree_index), 300 random parameter tuples | `diff_utils_address.rs::address_setter_sequences` | [x] |
| 6 | `SPX_initialize_hash_function` | 50 random (pub_seed, sk_seed) + the all-00 / all-ff / mixed extremes; the **whole `spx_ctx`** is compared, which is what actually exercises the `sha2` `state_seeded`(+`_512`) and `haraka` tweaked-constant tables | `diff_utils_address.rs::initialize_hash_function_matches` | [x] |
| 7 | `SPX_seed_state` (sha2 only) | 100 random seeds, full ctx compared — covers `SPX_SHA512` = 0 and 1 | `diff_backend.rs::sha2::seed_state_matches` | [x] |
| 8 | `SPX_tweak_constants` (haraka only) | 100 random seeds, full ctx compared | `diff_backend.rs::haraka::tweak_constants_matches` | [x] |
| 9 | `SPX_prf_addr` | 20 ctx × 50 random addresses, plus every address `type` 0..8 on an otherwise-zero address | `diff_core.rs::prf_addr_random` | [x] |
| 10 | `SPX_gen_message_random` | 45 message lengths (see Axis 3) × 3 random (sk_prf, optrand, m); output buffer 96 bytes so the BLAKE backend's full 32/64-byte `blakeX_final(&S, R)` write is compared, not just `SPX_N` | `diff_core.rs::gen_message_random_all_lengths` | [x] |
| 11 | `SPX_hash_message` | 45 message lengths × 3 random (R, pk, m); `digest`, `*tree` **and** `*leaf_idx` compared. Straddles the `sha2` `3N+mlen < BLOCK` branch and every primitive's block boundary | `diff_core.rs::hash_message_all_lengths` | [x] |
| 12 | `SPX_thash` | `inblocks` ∈ {1, 2, 3, 4, 5, 8, 16, `SPX_WOTS_LEN`, `SPX_FORS_TREES`} × 40 random (input, address); also asserts the address is not mutated. Covers the `inblocks == 1` / `> 1` split and therefore the SHA-512 / BLAKE-512 sub-path of the 192/256 sets, under both `robust` and `simple` | `diff_core.rs::thash_all_inblocks` | [x] |
| 13 | `SPX_compute_root` | `tree_height` = 1..max(`TREE_HEIGHT`, `FORS_HEIGHT`) × 25 random (leaf, auth_path, address), with `leaf_idx` drawn from {0, `2^h-1`, in-tree random, full-range random} (both parities) and `idx_offset` from {0, random, multiple of `2^h`} | `diff_core.rs::compute_root_all_heights` | [x] |
| 14 | `SPX_treehash` (the generic, function-pointer form) | `tree_height` = 0..min(max(`TREE_HEIGHT`,`FORS_HEIGHT`), 10) × 10 random configurations, driven with a **shared** deterministic `gen_leaf` callback so the tree-building logic itself is what is compared; root *and* auth path *and* the mutated `tree_addr` are checked | `diff_core.rs::treehash_with_shared_gen_leaf` | [x] |
| 15 | `SPX_treehash` ∘ `SPX_fors_gen_leafx1` | the same sweep, but each library uses **its own** `gen_leaf` (the composed pipeline, not a per-wrapper test) | `diff_core.rs::treehash_with_native_fors_gen_leaf` | [x] |
| 16 | `SPX_chain_lengths` | 2000 random `SPX_N`-byte messages + the all-`00`/`ff`/`0f`/`f0`/`01`/`80` patterns (extreme checksums) | `diff_core.rs::chain_lengths_random` | [x] |
| 17 | `SPX_wots_pk_from_sig` | 20 random (signature, message, address) + the all-`00` and all-`ff` messages, which drive `gen_chain` to 15 steps and 0 steps respectively | `diff_core.rs::wots_pk_from_sig_random` | [x] |
| 18 | `SPX_wots_gen_leafx1` | 40 iterations covering **both** branches of `leaf_idx == info->wots_sign_leaf`: signing (`wots_sig` filled, compared) and pk-only (`wots_sign_leaf = ~0`); `wots_steps` taken alternately from `chain_lengths` and from random values in `0..SPX_WOTS_W`; `leaf_addr`/`pk_addr` compared after the call | `diff_core.rs::wots_gen_leafx1_both_branches` | [x] |
| 19 | `SPX_fors_gen_leafx1` | 300 random (`addr_idx`, `fors_gen_leaf_info`); the mutated info/address is compared too | `diff_core.rs::fors_gen_leafx1_random` | [x] |
| 20 | `SPX_fors_treehashx1` | `tree_height` = 1..min(`FORS_HEIGHT`, 10) × 5 random (leaf_idx in-tree, `idx_offset` a multiple of `2^h`, random `tree_addr`, random info); root, auth path, `tree_addr` and info all compared | `diff_core.rs::fors_treehashx1_all_heights` | [x] |
| 21 | `SPX_wots_treehashx1` | `tree_height` = 1..min(`TREE_HEIGHT`, 6) × both `wots_sign_leaf` modes (in-tree leaf / `~0`); root, auth path, `wots_sig`, `tree_addr`, `leaf_addr`, `pk_addr` compared | `diff_core.rs::wots_treehashx1_all_heights` | [x] |
| 22 | `SPX_fors_sign` + `SPX_fors_pk_from_sig` | 10 messages incl. all-`00` and all-`ff` `SPX_FORS_MSG_BYTES` (extreme `message_to_indices` results) × random `fors_addr`; `pk_from_sig` run both on the freshly produced signature and on a random one; plus the sign→verify round-trip | `diff_core.rs::fors_sign_and_pk_from_sig` | [x] |
| 23 | `SPX_merkle_sign` | 4 configurations covering `idx_leaf = ~0` (the `merkle_gen_root` case) and random in-tree `idx_leaf`; signature, updated `root`, `wots_addr` and `tree_addr` all compared | `diff_core.rs::merkle_sign_and_gen_root` | [x] |
| 24 | `SPX_merkle_gen_root` | 3 random contexts | `diff_core.rs::merkle_sign_and_gen_root` | [x] |
| 25 | `blake256` / `blake512` (one-shot) | 60 input lengths × 3 random inputs each | `diff_backend.rs::blake::blake256_oneshot`, `::blake512_oneshot` | [x] |
| 26 | `blake256_compress` / `blake512_compress` | 200 × (**random** 128/248-byte state, random block) — exercises non-zero `s`, `t` and `nullt`, which the init/update path never produces | `diff_backend.rs::blake::blake*_incremental_and_compress` | [x] |
| 27 | `blake256_init`/`_update`/`_final` | 200 × 1–5 random chunks (bit lengths), state compared after the updates *and* after `final`; plus the codebase's own quirky "byte count where a bit count is expected" call pattern at 0, 1, 8, 16, 24, 32, 64, 128, 136 | `diff_backend.rs::blake::blake*_incremental_and_compress` | [x] |
| 28 | `SPX_blake256_mgf1` / `SPX_blake512_mgf1` | `inlen` ∈ {1,4,16,32,48,64,96,2N+32,2N+64} × `outlen` ∈ {1, blk±1, blk, 2·blk±1, 5·blk+3, `SPX_DGST_BYTES`} | `diff_backend.rs::blake::blake_mgf1_all_lengths` | [x] |
| 29 | `cst` (exported **data** symbol, blake) | full 16×`u64` table compared | `diff_backend.rs::blake::cst_data_symbol_matches` | [x] |
| 30 | `sha256` / `sha512` (one-shot) | 60 input lengths × 3 random inputs | `diff_backend.rs::sha2::sha256_oneshot`, `::sha512_oneshot` | [x] |
| 31 | `sha256_inc_init`/`_inc_blocks`/`_inc_finalize` (and the 512 family) | `nblocks` ∈ {0,1,2,3,5} × tail length ∈ {0, 1, blk-9, blk-8, blk-1, blk, blk+1, 2·blk+5} — `blk-9`/`blk-8` straddle the length-encoding boundary inside `inc_finalize`; the 40/72-byte state is compared after every step | `diff_backend.rs::sha2::sha_incremental` | [x] |
| 32 | `SPX_mgf1_256` / `SPX_mgf1_512` | as row 28 | `diff_backend.rs::sha2::mgf1_all_lengths` | [x] |
| 33 | `shake256` (one-shot) | 60 input lengths × `outlen` ∈ {1, rate-1, rate, rate+1, 2·rate, `SPX_N`, `SPX_DGST_BYTES`} | `diff_backend.rs::shake::shake256_oneshot` | [x] |
| 34 | `shake256_absorb` + `shake256_squeezeblocks` (the low-level, rate-multiple form) | `nin` ∈ {1,2,3,5} rate-blocks × `nout` ∈ {1,2,3,8} rate-blocks; the 25-word state is compared after both calls | `diff_backend.rs::shake::shake256_absorb_squeezeblocks` | [x] |
| 35 | `shake256_inc_init`/`_absorb`/`_finalize`/`_squeeze` | 200 × (1–5 random absorb chunks, 1–4 successive squeezes of random length); 26-word state compared after every call; plus absorb lengths {0, 1, rate±1, rate, 2·rate, 2·rate+1} | `diff_backend.rs::shake::shake256_incremental` | [x] |
| 36 | `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | 500 random inputs each, under a tweaked context | `diff_backend.rs::haraka::haraka512_perm_and_512_and_256` | [x] |
| 37 | `SPX_haraka_S` (sponge one-shot) | 60 input lengths × `outlen` ∈ {1, rate-1, rate, rate+1, 2·rate, `SPX_N`, `SPX_DGST_BYTES`} | `diff_backend.rs::haraka::haraka_sponge_oneshot` | [x] |
| 38 | `SPX_haraka_S_inc_init`/`_absorb`/`_finalize`/`_squeeze` | 200 × (1–5 random absorb chunks, 1–4 squeezes); the 65-byte `s_inc` (including the `s_inc[64]` byte-count field) compared after every call; plus absorb lengths {0,1,rate±1,rate,2·rate,2·rate+1} | `diff_backend.rs::haraka::haraka_sponge_incremental` | [x] |
| 39 | `crypto_sign_secretkeybytes` / `publickeybytes` / `bytes` / `seedbytes` | all four, cross-checked against the header formulae recomputed independently in the harness | `diff_api.rs::size_getters_match` | [x] |
| 40 | `crypto_sign_seed_keypair` | 8 seeds incl. all-`00` and all-`ff`; `pk` **and** `sk` compared | `diff_api.rs::seed_keypair_matches` | [x] |
| 41 | `crypto_sign_keypair` (the `randombytes`-driven wrapper) | 4 × (`randombytes_init` with the same entropy on both sides, then keypair); `pk`/`sk` compared byte-for-byte | `diff_api.rs::keypair_matches_after_identical_reseed` | [x] |
| 42 | `crypto_sign_signature` + `crypto_sign_verify` | 5–13 message lengths (reduced for the `s` sets) × identical DRBG reseed; `sig` and `*siglen` compared, then **cross-verified**: the C signature must verify under Rust and vice versa | `diff_api.rs::signature_verify_and_cross_verify` | [x] |
| 43 | `crypto_sign` + `crypto_sign_open` | the same message lengths; `sm`, `*smlen`, and then `crypto_sign_open`'s `m`, `*mlen` and return value compared, plus the recovered message equals the original | `diff_api.rs::sign_and_open_matches` | [x] |
| 44 | `AES256_ECB` | 2000 random (key, block) + the four all-`00`/all-`ff` corner combinations | `diff_rng.rs::aes256_ecb_matches` | [x] |
| 45 | `AES256_CTR_DRBG_Update` | 500 × random (`Key`, `V`) alternating `provided_data` non-NULL / NULL, plus `V` values that force the increment loop to ripple (all-`ff`, `…ff`, `00 ff…ff`) | `diff_rng.rs::drbg_update_matches_with_and_without_provided_data` | [x] |
| 46 | `randombytes_init` + `randombytes` + the `DRBG_ctx` data symbol | 20 trials, alternating `personalization_string` NULL / non-NULL, × request lengths {0,1,15,16,17,31,32,33,48,63,64,100,1000} (the 16-byte AES block boundary), each from an identical reseed; then a 25-step **chain** of successive draws. Both the output bytes and the full 52-byte `DRBG_ctx` are compared after every call | `diff_rng.rs::randombytes_stream_and_drbg_ctx_state` | [x] |
| 47 | `seedexpander_init` + `seedexpander` | 100 random (seed, diversifier) × `maxlen` ∈ {1, 16, 17, 256, 0x10000, 0xfffffffe, 0xffffffff, 100000} × 8 chained requests of growing size, crossing the internal 16-byte buffer boundary and the `ctr[12..16]` increment; the whole 80-byte `AES_XOF_struct` is compared after every call | `diff_rng.rs::seedexpander_valid_paths` | [x] |
| 48 | the KAT driver end-to-end (`PQCgenKAT_sign.c` vs `src/main.rs`) | `randombytes_init` with the standard `0,1,…,47` entropy, 7 iterations, message lengths 33·1 … 33·7, each doing `crypto_sign_keypair` → `crypto_sign` → `crypto_sign_open`; the whole transcript is hashed and the digests compared | `verif/driver_all.sh` | [x] |

## Not represented as a row

* `if (SPX_D == 1) *tree = 0;` in all four `hash_*.c` — `SPX_D >= 7` for every
  shipped parameter set, so the branch is unreachable.
* `SPX_WOTS_W == 256` (`SPX_WOTS_LOGW 8`, `SPX_WOTS_LEN2 2`) — every
  `params-sphincs-*.h` sets `SPX_WOTS_W 16`, so the `#if` alternative is dead
  code in this project.
* `SPX_N > 32` (`#error Linking against BLAKE-256/SHA-256 with N larger than 32
  bytes is not supported`) — unreachable.
