# CONFIGS.md — configuration surface table (Phase A -> Phase B)

## Axes the C code actually branches on

### Build-time axes (CMake cache vars -> Cargo features)
| axis | values | what it switches (C) |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | which `lib/<b>` is linked: `hash_<b>.c` + `thash_<b>_*.c`; also the **address-field offsets** (`sha2_offsets.h` uses a compressed 22-byte layout, the others 32-byte) and the **`spx_ctx` shape** (`#ifdef SPX_SHA2` / `#ifdef SPX_HARAKA` in `context.h`) |
| `THASH` | `robust`, `simple` | `thash_<b>_robust.c` (MGF1 bitmask XOR) vs `thash_<b>_simple.c` (direct hash) |
| `SECPAR` | `128s 128f 192s 192f 256s 256f` | `SPX_N` (16/24/32), `SPX_FULL_HEIGHT`, `SPX_D`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`; and via `SPX_N >= 24`: `SPX_SHA512` / `SPX_BLAKE512` -> `shaX`/`blakeX` = 512-bit variant in `gen_message_random`/`hash_message`, and the `inblocks > 1` -> `thash_512` branch |

4 x 2 x 6 = **48 build configurations**, all enumerated and exercised.

### Runtime axes (per configuration)
| axis | values the C distinguishes |
|---|---|
| `thash(inblocks)` | `0`, `1` (F function / SHA-256 / haraka512 / BLAKE-256 path), `2` (H function; `>1` -> 512-bit path when `SPX_SHA512`/`SPX_BLAKE512`, `haraka_S` path for haraka), `SPX_WOTS_LEN` (T_len, 35/51/67), `SPX_FORS_TREES` (14/17/22/33/35) |
| `mlen` (`gen_message_random`, `hash_message`, `crypto_sign*`) | `0`; `1`; just below / at / just above the SHA-2 block-fill boundaries `SPX_SHAX_BLOCK_BYTES - SPX_N` and `SPX_INBLOCKS*BLOCK - SPX_N - SPX_PK_BYTES` (`hash_sha2.c` `if (... < ...)` vs `else`); multi-block; also the incremental-absorb rate boundaries of SHAKE-256 (136), haraka-S (32), BLAKE-256 (64) / BLAKE-512 (128) |
| `leaf_idx` parity / position in `compute_root`, `treehash`, `wots_treehashx1`, `fors_treehashx1` | `0` (all-left), `max` (all-right), odd, even, `~0u` (the "don't generate an auth path" sentinel used by `merkle_gen_root`) |
| `idx_offset` | `0`, `i*(1<<SPX_FORS_HEIGHT)` (FORS), values that make `leaf_idx + idx_offset` overflow the low byte of the address |
| `leaf_info_x1.wots_sign_leaf` | `== leaf_idx` (signature branch, `wots_k_mask = 0`) vs `!= leaf_idx` (`wots_k_mask = ~0`, `wots_sig` never written) |
| `SPX_D == 1` in `hash_message` | never true for the 6 shipped parameter sets (`SPX_D` in {7,8,17,22}), so the `*tree = 0` branch is dead — noted, not testable |
| `ull_to_bytes(outlen)` / `bytes_to_ull(inlen)` | `0`, `1`, `4`, `8`, `>8` |
| address field values | `0`, small, `0xff`, `0x100`, `0xFFFFFFFF`, `u64::MAX` |
| `randombytes`/`seedexpander` lengths | `0`, `<16`, `16`, `17`, `>16` non-multiple, large |

## Rows (each is a differential test, both libs called through their `.so`)

Every row is exercised with **many randomised inputs** (splitmix64, fixed seed
`0x5EED_1234_ABCD_0001`, 8..64 iterations per row depending on cost) unless the
row is a fixed-value boundary.  A row is checked off only when it passes for
every iteration in every one of the 48 build configurations.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `SPX_ull_to_bytes` | random `in`, `outlen` in {0,1,2,3,4,5,6,7,8,9,16} | `b01_ull_to_bytes` | [x] |
| 2  | `SPX_u32_to_bytes` | random `u32` incl. `0`, `0xFFFFFFFF` | `b02_u32_to_bytes` | [x] |
| 3  | `SPX_bytes_to_ull` | random bytes, `inlen` in {0,1,..,8} | `b03_bytes_to_ull` | [x] |
| 4  | all 10 address setters | fresh zero addr + random addr, random field values incl. out-of-byte-range, applied in random order (sequence-dependent because fields share bytes: `CHAIN_ADDR == TREE_HGT`) | `b04_address_setters_random_sequences` | [x] |
| 5  | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random source/dest addrs (checks exactly which bytes are copied) | `b05_address_copies` | [x] |
| 6  | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the WHOLE `spx_ctx` byte image afterwards (haraka: 960 tweaked round-constant bytes; sha2: `state_seeded`[40] + `state_seeded_512`[72]; blake/shake: no-op) | `b06_initialize_hash_function` | [x] |
| 7  | `SPX_prf_addr` | random seeded ctx x random addr | `b07_prf_addr` | [x] |
| 8  | `SPX_thash` | `inblocks = 1` (F path), random ctx/addr/input | `b08_thash_1` | [x] |
| 9  | `SPX_thash` | `inblocks = 2` (H path; 512-bit variant for `SPX_N>=24` on sha2/blake, `haraka_S` for haraka) | `b09_thash_2` | [x] |
| 10 | `SPX_thash` | `inblocks = SPX_WOTS_LEN` (T_len, the widest input) | `b10_thash_wots_len` | [x] |
| 11 | `SPX_thash` | `inblocks = SPX_FORS_TREES` | `b11_thash_fors_trees` | [x] |
| 12 | `SPX_thash` | `inblocks = 0` and `3` (odd non-special widths) | `b12_thash_misc_widths` | [x] |
| 13 | `SPX_gen_message_random` | `mlen = 0` | `b13_gen_message_random_empty` | [x] |
| 14 | `SPX_gen_message_random` | `mlen` sweeping every block/rate boundary: 1..40, 31..33, 47..49, 63..65, 103..105, 127..129, 135..137, 168..170, 255..257 | `b14_gen_message_random_boundaries` | [x] |
| 15 | `SPX_gen_message_random` | random large `mlen` (up to 4096) | `b15_gen_message_random_large` | [x] |
| 16 | `SPX_hash_message` | `mlen = 0`; checks `digest`, `*tree`, `*leaf_idx` | `b16_hash_message_empty` | [x] |
| 17 | `SPX_hash_message` | same boundary sweep as row 14 (the SHA-2 `SPX_INBLOCKS*BLOCK - SPX_N - SPX_PK_BYTES` branch lives here) | `b17_hash_message_boundaries` | [x] |
| 18 | `SPX_hash_message` | random large `mlen`, random `R`/`pk` | `b18_hash_message_large` | [x] |
| 19 | `SPX_chain_lengths` | random `msg[SPX_N]`, incl. all-zero and all-`0xff` (checksum extremes) | `b19_chain_lengths` | [x] |
| 20 | `SPX_wots_pk_from_sig` | random sig/msg/addr; also all-zero and all-`0xff` msg (drives `gen_chain` start/steps to the extremes) | `b20_wots_pk_from_sig` | [x] |
| 21 | `SPX_wots_gen_leafx1` | `leaf_info.wots_sign_leaf == leaf_idx` (signature branch); random steps, checks `dest` AND the 
`wots_sig` buffer AND the mutated `leaf_addr`/`pk_addr` | `b21_wots_gen_leafx1_signing` | [x] |
| 22 | `SPX_wots_gen_leafx1` | `wots_sign_leaf != leaf_idx` (pk-only branch, `wots_sig` must stay untouched) | `b22_wots_gen_leafx1_pkonly` | [x] |
| 23 | `SPX_fors_gen_leafx1` | random ctx/addr/`addr_idx` incl. `0` and `0xFFFFFFFF` | `b23_fors_gen_leafx1` | [x] |
| 24 | `SPX_compute_root` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` = 0 / max / random odd / random even, `idx_offset` = 0 / `i<<h` | `b24_compute_root_fors_height` | [x] |
| 25 | `SPX_compute_root` | `tree_height = SPX_TREE_HEIGHT`, same leaf_idx/idx_offset shapes | `b25_compute_root_tree_height` | [x] |
| 26 | `SPX_compute_root` | `tree_height = 1` and `2` (minimal trees, exercises the "last iteration is exceptional" tail) | `b26_compute_root_small` | [x] |
| 27 | `SPX_treehash` (generic, function-pointer `gen_leaf`) | `gen_leaf` = a **C-ABI callback defined in the test** (so both libs call the same leaf generator); `tree_height` 1..4, `leaf_idx` 0/max/odd/even, `idx_offset` 0/random | `b27_treehash_callback` | [x] |
| 28 | `SPX_treehash` | `leaf_idx = ~0u` (no auth path node ever matches) | `b28_treehash_no_authpath` | [x] |
| 29 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` 0/max/random, `idx_offset` = `i*(1<<SPX_FORS_HEIGHT)` for random `i` | `b29_fors_treehashx1` | [x] |
| 30 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` = random (signing leaf) with `info.wots_sign_leaf = leaf_idx` | `b30_wots_treehashx1_signing` | [x] |
| 31 | `SPX_wots_treehashx1` | `leaf_idx = ~0u` / `wots_sign_leaf = ~0u` (the `merkle_gen_root` shape: no auth path, no signature) | `b31_wots_treehashx1_root_only` | [x] |
| 32 | `SPX_fors_sign` | random `m[SPX_FORS_MSG_BYTES]`, random `fors_addr`; compares full `sig[SPX_FORS_BYTES]` + `pk[SPX_N]` | `b32_fors_sign` | [x] |
| 33 | `SPX_fors_sign` | `m` = all-zero and all-`0xff` (index extremes: every `indices[i]` = 0 resp. max) | `b33_fors_sign_extremes` | [x] |
| 34 | `SPX_fors_pk_from_sig` | the signature produced in row 32 (round trip: derived pk must equal `fors_sign`'s pk) + random garbage signatures | `b34_fors_pk_from_sig` | [x] |
| 35 | `SPX_merkle_sign` | random ctx/addrs, `idx_leaf` random in `[0, 2^SPX_TREE_HEIGHT)`; compares `sig[SPX_WOTS_BYTES + SPX_TREE_HEIGHT*SPX_N]`, `root`, and both mutated addresses | `b35_merkle_sign` | [x] |
| 36 | `SPX_merkle_sign` | `idx_leaf = ~0u` (root-only shape) and `idx_leaf = 0` / `2^h - 1` (extremes) | `b36_merkle_sign_extremes` | [x] |
| 37 | `SPX_merkle_gen_root` | random seeded ctx | `b37_merkle_gen_root` | [x] |
| 38 | `crypto_sign_seed_keypair` | random 3N-byte seeds; compares `pk[2N]`, `sk[4N]`, return value | `b38_seed_keypair` | [x] |
| 39 | `crypto_sign_keypair` | `randombytes_init(entropy, NULL)` on BOTH libs first, then keypair; also compares the resulting `DRBG_ctx` image | `b39_keypair_via_drbg` | [x] |
| 40 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen` in {0,1,31,32,33,47,48,49,63,64,65,103,104,105,127,128,129,135,136,137,168,255,256,257,1000} — each: sign with both, compare sig bytes and `siglen`, then cross-verify (C sig vs Rust verify and vice versa) | `b40_sign_verify_mlen_sweep` | [x] |
| 41 | `crypto_sign_signature` | non-deterministic path: `randombytes` (optrand) is drawn from the shared DRBG, so both libs must be re-seeded identically before each call — verifies the `optrand` draw consumes the DRBG identically | `b41_sign_drbg_lockstep` | [x] |
| 42 | `crypto_sign` + `crypto_sign_open` | `mlen` sweep as row 40; compares `sm`, `*smlen`, recovered `m`, `*mlen`, return values | `b42_sign_open_roundtrip` | [x] |
| 43 | `randombytes_init` + `randombytes` | random 48-byte entropy, `personalization_string` = NULL and random; then a sequence of `randombytes` calls with lengths {0,1,15,16,17,31,32,48,100} comparing output AND the `DRBG_ctx` (`Key`,`V`,`reseed_counter`) after each | `b43_drbg_sequence` | [x] |
| 44 | `AES256_ECB` | random key/ctr (raw AES-256-ECB block, cross-checked against OpenSSL inside the C lib) | `b44_aes256_ecb` | [x] |
| 45 | `AES256_CTR_DRBG_Update` | random `Key`/`V`, `provided_data` = NULL and random 48 bytes | `b45_drbg_update` | [x] |
| 46 | `seedexpander_init` + `seedexpander` | random seed/diversifier, `maxlen` in {1,16,17,256,65536,0xFFFFFFFF}; then repeated `seedexpander` calls of lengths {1,5,15,16,17,33,100} comparing output AND the full `AES_XOF_struct` image after each | `b46_seedexpander_sequence` | [x] |
| 47 | blake backend primitives | `blake256_init/update/final`, `blake256`, `blake256_compress`, `blake512_*`, `blake512`, `SPX_blake256_mgf1`, `SPX_blake512_mgf1`, `cst` data symbol; random inputs over the same length sweep as row 14 (NOTE: this codebase calls `blakeX_update` with *byte* counts while `update` treats the argument as *bits* — the tests pin that exact behaviour) | `b47a_blake_oneshot`, `b47b_blake_incremental`, `b47c_blake_compress`, `b47d_blake_mgf1_and_cst` | [x] |
| 48 | sha2 backend primitives | `sha256`, `sha256_inc_init/blocks/finalize`, `sha512*`, `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`; length sweep incl. block boundaries 55/56/57/63/64/65/111/112/113/127/128/129 | `b48a_sha_oneshot`, `b48b_sha_incremental`, `b48c_mgf1_and_seed_state` | [x] |
| 49 | shake backend primitives | `shake256`, `shake256_absorb` + `shake256_squeezeblocks` (nblocks 0/1/2/3), `shake256_inc_init/absorb/finalize/squeeze` with multi-call absorb and split squeezes across the 136-byte rate | `b49a_shake256_oneshot`, `b49b_shake256_absorb_squeeze`, `b49c_shake256_incremental` | [x] |
| 50 | haraka backend primitives | `SPX_tweak_constants`, `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm`, `SPX_haraka_S` (outlen 0/1/31/32/33/64/100), `SPX_haraka_S_inc_*` with multi-call absorb / split squeeze across the 32-byte rate | `b50a_tweak_constants`, `b50b_haraka_permutations`, `b50c_haraka_sponge`, `b50d_haraka_incremental` | [x] |
| 51 | full pipeline, cross-library | C keypair -> Rust sign -> C verify, and Rust keypair -> C sign -> Rust verify (catches any state/format divergence invisible to per-function tests) | `b51_cross_library_pipeline` | [x] |
| 52 | `crypto_sign_*bytes` getters | no input; must equal the values derived from the C headers for this configuration | `b52_size_getters` | [x] |
