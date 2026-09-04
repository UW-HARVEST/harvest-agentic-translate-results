# CONFIGS.md — configuration-surface table (valid inputs)

The mirror of `ERRORS.md`.  Axes were read off the branches the C actually
takes, not guessed: the CMake cache variables, the `#if` / `#ifdef` tests those
variables drive, and every run-time `if` / `switch` in `c_src/app/src/*.c` and
`c_src/lib/*/src/*.c` that depends on an argument.

## Build-time axes (the outer product; 96 configurations)

| axis | source | values |
|---|---|---|
| `HASH_BACKEND` | `c_src/CMakeLists.txt`, `lib/CMakeLists.txt` `add_subdirectory(${HASH_BACKEND})` | `blake`, `haraka`, `sha2`, `shake` |
| `THASH` | `lib/<b>/CMakeLists.txt` `src/thash_<b>_${THASH}.c` | `robust`, `simple` |
| `SECPAR` | `app/CMakeLists.txt` `PARAMS=sphincs-${HASH_BACKEND}-${SECPAR}` | `128s`, `128f`, `192s`, `192f`, `256s`, `256f` |
| `randombytes` provider | `app/CMakeLists.txt` targets `sphincs_core` (`randombytes.c`) vs `sphincs_core_det` (`rng.c`) | urandom, NIST DRBG |

Derived compile-time branches these switch on, all covered by the product
above: `SPX_BLAKE512` / `SPX_SHA512` (0 for `128*`, 1 for `192*`/`256*`, which
is exactly `SPX_N >= 24`), `SPX_N` in {16, 24, 32}, `SPX_D` in {7, 8, 17, 22},
`SPX_TREE_HEIGHT` in {3, 8, 9}, `SPX_FORS_HEIGHT` in {6, 8, 9, 12, 14},
`SPX_FORS_TREES` in {14, 17, 22, 33, 35}, `SPX_WOTS_LEN` in {35, 51, 67}, and
the per-backend `SPX_OFFSET_*` address layout (`sha2` differs from the other
three).  `if (SPX_D == 1)` in every `hash_message` is dead in all 24 parameter
sets (min `SPX_D` is 7) and is recorded as such rather than tested.

## Run-time axes

* **`mlen` sweep** (used by every row that takes a message).  The block-fill
  branches are `SPX_N + mlen < SPX_SHAX_BLOCK_BYTES` in `hash_sha2.c`'s
  `gen_message_random`, `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS *
  SPX_SHAX_BLOCK_BYTES` in its `hash_message`, `left && ((datalen>>3) & 0x3F)
  >= fill` in `blake256_update` / `blake512_update`, the `buflen == 440` /
  `< 440` / `else` three-way split in `blake256_final` (`888` in
  `blake512_final`), and the rate-crossing tests in `keccak_inc_absorb`
  (rate 136) and `haraka_S_inc_absorb` (rate 32).  The sweep
  `{0, 1, 2, 15, 16, 17, 31, 32, 33, 47, 48, 49, 55, 56, 57, 63, 64, 65, 71,
  72, 73, 95, 96, 97, 103, 104, 105, 127, 128, 129, 135, 136, 137, 167, 168,
  169, 191, 192, 193, 255, 256, 257, 1000, 4096}` hits every one of those
  thresholds and its neighbours in all 24 parameter sets.
* **`inblocks`** for `thash`: `1` (the `F` function; the only value for which
  `thash_haraka_*` takes its first branch and for which `thash_blake_*` /
  `thash_sha2_*` stay on the narrow primitive when `SPX_N >= 24`), `2` (Merkle
  node), `SPX_WOTS_LEN` (WOTS public key compression), `SPX_FORS_TREES` (FORS
  root compression), `0`, and `max(SPX_WOTS_LEN, SPX_FORS_TREES) + k` for the
  `SPX_VLA` sizes no internal caller ever reaches.
* **`leaf_idx` parity and `idx_offset`** in `compute_root` (`if (leaf_idx & 1)`)
  and in `wots_treehashx1` / `fors_treehashx1`
  (`(internal_idx & 1) == 0 && idx < max_idx`).
* **`info->wots_sign_leaf`** in `wots_gen_leafx1`: equal to `leaf_idx`
  (`wots_k_mask = 0`, the WOTS signature is emitted) or not (`wots_k_mask = ~0`,
  public keys only).  `merkle_gen_root` drives the second case with the
  sentinel `(uint32_t)~0`.
* **message digits** for `chain_lengths` / `wots_pk_from_sig`: all-zero and
  all-`0xFF` messages put the base-`w` digits and the WOTS checksum at both
  extremes, which is where `gen_chain`'s `i < SPX_WOTS_W` clamp and
  `wots_checksum`'s shift matter.
* **address `type`** values `0..6` (`SPX_ADDR_TYPE_WOTS` .. `FORSPRF`).
* **`xlen` / `outlen` shapes** for the DRBG and the MGF1/XOF helpers: `0`,
  `< block`, `== block`, `block + 1`, several blocks, non-multiples, and the
  `0xFF` carry chains in `randombytes`' `V` and `seedexpander`'s `ctr[12..16]`.
* **incremental vs one-shot** hashing of the backend primitives.

## The table

One row per combination the C treats differently.  `[ ]` is checked off only
after the row passes across its randomised inputs (fixed seed) in **all** 96
build configurations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `SPX_set_layer_addr`, `SPX_set_type`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height` | single-byte setters; random 32-byte start address, value sweep `0..=255` plus `0x100`, `0xDEADBEEF`, `0xFFFFFFFF` (truncation) | [x] `tests/b_lowlevel.rs::row01_single_byte_setters` |
| 2 | `SPX_set_tree_addr` | 8-byte big-endian field; `tree` = 0, 1, `2^k` for k in 0..63, `0xFFFF_FFFF_FFFF_FFFF`, random | [x] `tests/b_lowlevel.rs::row02_set_tree_addr` |
| 3 | `SPX_set_keypair_addr`, `SPX_set_tree_index` | 4-byte big-endian field; 0, 1, `0x7FFFFFFF`, `0xFFFFFFFF`, random | [x] `tests/b_lowlevel.rs::row03_four_byte_setters` |
| 4 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random `in`, random pre-filled `out`; verifies the backend-specific `SPX_OFFSET_TREE + 8` / `+ KP_ADDR` copy windows and that the remaining bytes of `out` survive | [x] `tests/b_lowlevel.rs::row04_copiers` |
| 5 | `SPX_ull_to_bytes` | `outlen` 0, 1, 2, 4, 7, 8, 9, 16, 32 x `in` = 0, 1, `0xFF`, `0x0102030405060708`, `u64::MAX`, random (covers `outlen > 8`, where the top bytes must be zero) | [x] `tests/b_lowlevel.rs::row05_ull_to_bytes` |
| 6 | `SPX_u32_to_bytes` | `in` = 0, 1, `0xFFFFFFFF`, random | [x] `tests/b_lowlevel.rs::row06_u32_to_bytes` |
| 7 | `SPX_bytes_to_ull` | `inlen` 0, 1, 4, 7, 8 x random input (round-trips against row 5) | [x] `tests/b_lowlevel.rs::row07_bytes_to_ull` |
| 8 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the **whole** `spx_ctx` byte image, which is where `sha2`'s `state_seeded`/`state_seeded_512` and `haraka`'s `tweaked512_rc64`/`tweaked256_rc32` are produced (no-op for `blake`/`shake`) | [x] `tests/b_hash.rs::row08_initialize_hash_function` |
| 9 | `SPX_prf_addr` | initialised ctx x random address x random seeds | [x] `tests/b_hash.rs::row09_prf_addr` |
| 10 | `SPX_gen_message_random` | random `sk_prf`, `optrand`, initialised ctx x full `mlen` sweep | [x] `tests/b_hash.rs::row10_gen_message_random` |
| 11 | `SPX_hash_message` | random `R`, `pk` x full `mlen` sweep; compares `digest`, `*tree` and `*leaf_idx` | [x] `tests/b_hash.rs::row11_hash_message` |
| 12 | `SPX_thash`, `THASH=simple` | `inblocks` = 1 | [x] `tests/b_thash.rs::row12_inblocks_1` |
| 13 | `SPX_thash`, `THASH=simple` | `inblocks` = 2 (the `> 1` wide-primitive branch when `SPX_N >= 24`) | [x] `tests/b_thash.rs::row13_inblocks_2` |
| 14 | `SPX_thash`, `THASH=simple` | `inblocks` = `SPX_WOTS_LEN` | [x] `tests/b_thash.rs::row14_inblocks_wots_len` |
| 15 | `SPX_thash`, `THASH=simple` | `inblocks` = `SPX_FORS_TREES` | [x] `tests/b_thash.rs::row15_inblocks_fors_trees` |
| 16 | `SPX_thash`, `THASH=simple` | `inblocks` = 0 | [x] `tests/b_thash.rs::row16_inblocks_zero` |
| 17 | `SPX_thash`, `THASH=simple` | `inblocks` = `max(SPX_WOTS_LEN, SPX_FORS_TREES) + 1`, `+ 17`, `+ 96` (past every internal use) | [x] `tests/b_thash.rs::row17_inblocks_past_internal_max` |
| 18 | `SPX_thash`, `THASH=robust` | rows 12-17 repeated; the robust variant additionally derives a bitmask of `inblocks * SPX_N` bytes with MGF1/XOF, so `inblocks` also drives the mask length | [x] `tests/b_thash.rs::row18_inblocks_dense_sweep` |
| 19 | `SPX_compute_root` | `tree_height` = 1 x `leaf_idx` even/odd x `idx_offset` = 0 | [x] `tests/b_tree.rs::row19_compute_root_height1_parity` |
| 20 | `SPX_compute_root` | `tree_height` = `SPX_TREE_HEIGHT` x random `leaf_idx` in `0..2^h` x `idx_offset` = 0 | [x] `tests/b_tree.rs::row20_compute_root_tree_height` |
| 21 | `SPX_compute_root` | `tree_height` = `SPX_FORS_HEIGHT` x random `leaf_idx` x `idx_offset` = `i * 2^h` for random `i` (the FORS cross-tree index offset) | [x] `tests/b_tree.rs::row21_compute_root_fors_height_with_offset` |
| 22 | `SPX_compute_root` | `tree_height` = 2..`SPX_TREE_HEIGHT` sweep, `leaf_idx` = 0, `2^h - 1` (both extremes) and random | [x] `tests/b_tree.rs::row22_compute_root_height_sweep` |
| 23 | `SPX_treehash` + `SPX_fors_gen_leafx1` as the `gen_leaf` callback | `tree_height` = 1 and 2, `leaf_idx` = 0 and 1, `idx_offset` = 0; exercises the exported function-pointer signature across FFI | [x] `tests/b_tree.rs::row23_treehash_small` |
| 24 | `SPX_treehash` + `SPX_fors_gen_leafx1` | `tree_height` = `SPX_FORS_HEIGHT` x random `leaf_idx` x `idx_offset` = `i * 2^h`; compares `root` **and** the full `auth_path` | [x] `tests/b_tree.rs::row24_treehash_fors_height` |
| 25 | `SPX_fors_treehashx1` | `tree_height` = `SPX_FORS_HEIGHT`, random `leaf_idx` in range, `idx_offset` = `i * 2^h`; compares `root`, `auth_path` and the mutated `tree_addr` and `leaf_addrx` | [x] `tests/b_tree.rs::row25_fors_treehashx1_fors_height` |
| 26 | `SPX_fors_treehashx1` | `tree_height` = 1, 2, 3 (small trees, where `idx == max_idx` forces the extra combining iterations) | [x] `tests/b_tree.rs::row26_fors_treehashx1_small_trees` |
| 27 | `SPX_wots_gen_leafx1` | `leaf_idx != info.wots_sign_leaf` (`wots_k_mask = ~0`, no signature written); random `wots_steps` | [x] `tests/b_tree.rs::row27_wots_gen_leafx1_not_signing` |
| 28 | `SPX_wots_gen_leafx1` | `leaf_idx == info.wots_sign_leaf` (`wots_k_mask = 0`); `wots_steps[i]` = 0, `SPX_WOTS_W - 1` and random in `0..w`, so `k == wots_k` is hit at the first, last and middle chain positions; compares `dest`, `wots_sig`, `leaf_addr`, `pk_addr` | [x] `tests/b_tree.rs::row28_wots_gen_leafx1_signing` |
| 29 | `SPX_wots_treehashx1` | `tree_height` = `SPX_TREE_HEIGHT`, `leaf_idx` random in range, `info.wots_sign_leaf = leaf_idx` (signing) | [x] `tests/b_tree.rs::row29_wots_treehashx1_signing` |
| 30 | `SPX_wots_treehashx1` | `tree_height` = `SPX_TREE_HEIGHT`, `info.wots_sign_leaf = (uint32_t)~0` (the `merkle_gen_root` sentinel: no leaf ever matches) | [x] `tests/b_tree.rs::row30_wots_treehashx1_sentinel` |
| 31 | `SPX_wots_treehashx1` | `tree_height` = 1 and 2 | [x] `tests/b_tree.rs::row31_wots_treehashx1_small` |
| 32 | `SPX_chain_lengths` | `msg` = all zero, all `0xFF`, random; checks both `SPX_WOTS_LEN1` base-`w` digits and the `SPX_WOTS_LEN2` checksum digits at their extremes | [x] `tests/b_wots_fors.rs::row32_chain_lengths` |
| 33 | `SPX_wots_pk_from_sig` | random `sig`, `msg` = all zero / all `0xFF` / random (drives `gen_chain` `start`/`steps` from 0 to `w-1`) | [x] `tests/b_wots_fors.rs::row33_wots_pk_from_sig` |
| 34 | `SPX_fors_gen_leafx1` | random `addr_idx` including 0 and `0xFFFFFFFF`, random `leaf_addrx`; compares `leaf` and the mutated `leaf_addrx` | [x] `tests/b_wots_fors.rs::row34_fors_gen_leafx1` |
| 35 | `SPX_fors_sign` | random `m` (`SPX_FORS_MSG_BYTES`), random `fors_addr`, initialised ctx; compares the whole `SPX_FORS_BYTES` signature and the `pk` | [x] `tests/b_wots_fors.rs::row35_fors_sign_random` |
| 36 | `SPX_fors_sign` | `m` = all zero and all `0xFF` (`message_to_indices` at both index extremes: every index 0 vs every index `2^SPX_FORS_HEIGHT - 1`) | [x] `tests/b_wots_fors.rs::row36_fors_sign_index_extremes` |
| 37 | `SPX_fors_pk_from_sig` | the signature produced by row 35 (round-trip: derived `pk` must equal `fors_sign`'s `pk`) and independent random signatures | [x] `tests/b_wots_fors.rs::row37_fors_pk_from_sig` |
| 38 | `SPX_merkle_sign` | random `wots_addr`/`tree_addr`, `idx_leaf` random in `0..2^SPX_TREE_HEIGHT`; compares `sig` (`SPX_WOTS_BYTES + SPX_TREE_HEIGHT*SPX_N`), `root`, and both mutated addresses | [x] `tests/b_wots_fors.rs::row38_merkle_sign_random` |
| 39 | `SPX_merkle_sign` | `idx_leaf` = 0, `2^SPX_TREE_HEIGHT - 1`, and `(uint32_t)~0` (the `merkle_gen_root` sentinel) | [x] `tests/b_wots_fors.rs::row39_merkle_sign_extremes` |
| 40 | `SPX_merkle_gen_root` | random ctx; the top-layer root, i.e. `merkle_sign` composed with layer `SPX_D - 1` and the `~0` sentinel | [x] `tests/b_wots_fors.rs::row40_merkle_gen_root` |
| 41 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no arguments; the four size constants must agree, and are used to size every other row's buffers | [x] `tests/b_api.rs::row41_size_functions` |
| 42 | `crypto_sign_seed_keypair` | random `CRYPTO_SEEDBYTES` seed x many seeds; compares `pk` and `sk` | [x] `tests/b_api.rs::row42_seed_keypair` |
| 43 | `crypto_sign_keypair` | DRBG seeded identically on both sides via `randombytes_init`; compares `pk`, `sk` and the resulting `DRBG_ctx` state | [x] `tests/b_api.rs::row43_keypair_from_drbg` |
| 44 | `crypto_sign_signature` | key from row 42, DRBG re-seeded identically before each call, full `mlen` sweep; compares `sig` and `*siglen` | [x] `tests/b_api.rs::row44_45_46_signature_verify_open` |
| 45 | `crypto_sign_verify` | valid `(sig, m, pk)` from row 44 x full `mlen` sweep; both must return 0 | [x] `tests/b_api.rs::row44_45_46_signature_verify_open` |
| 46 | `crypto_sign` / `crypto_sign_open` | one-shot combined form, full `mlen` sweep; compares `sm`, `*smlen`, then the recovered `m` and `*mlen` | [x] `tests/b_api.rs::row44_45_46_signature_verify_open` |
| 47 | cross-library round trip | C signs, Rust verifies, and Rust signs, C verifies (the only comparison available under the `urandom` feature, where `optrand` is non-deterministic) | [x] `tests/b_api.rs::row47_cross_library_round_trip` |
| 48 | `blake256` / `blake512` one-shot | `inlen` sweep `0..=200` plus 255, 256, 257, 440/8, 511, 512, 513, 888/8, 1000, 4096 (the `blake*_final` `buflen` three-way split lives at 440 and 888 **bits**) | [x] `tests/b_backend.rs::blake::row48_blake_one_shot + row48b_cst_data_symbol` |
| 49 | `blake256_init` + `blake256_update` x k + `blake256_final` (and the 512 pair) | random chunking of the same input into 1..6 `update` calls, chunk sizes chosen to straddle the 64-byte (128-byte) block fill; result must equal the one-shot of row 48. Note `update` takes its length in **bits** | [x] `tests/b_backend.rs::blake::row49_blake_incremental` |
| 50 | `blake256_compress` / `blake512_compress` | random state x random 64-byte (128-byte) block, `nullt` = 0 and 1 | [x] `tests/b_backend.rs::blake::row50_blake_compress` |
| 51 | `SPX_blake256_mgf1` / `SPX_blake512_mgf1` | `outlen` = 0, 1, 31, 32, 33, 63, 64, 65, 200 x `inlen` = 0, 1, 32, 48, `SPX_N + SPX_ADDR_BYTES` (covers the `(i+1)*OUT <= outlen` loop and the partial tail) | [x] `tests/b_backend.rs::blake::row51_blake_mgf1` |
| 52 | `sha256` / `sha512` one-shot | `inlen` sweep as row 48 (the padding split in `sha*_inc_finalize` is at 56 / 112 bytes of the final block) | [x] `tests/b_backend.rs::sha2::row52_sha_one_shot` |
| 53 | `sha256_inc_init` + `sha256_inc_blocks` x k + `sha256_inc_finalize` (and the 512 pair) | `inblocks` = 0, 1, 2, 5 followed by a tail of 0..130 bytes; compares the 40-byte (72-byte) state after every step as well as the digest | [x] `tests/b_backend.rs::sha2::row53_sha_incremental` |
| 54 | `SPX_mgf1_256` / `SPX_mgf1_512` | as row 51 with 32/64-byte output blocks | [x] `tests/b_backend.rs::sha2::row54_mgf1` |
| 55 | `SPX_seed_state` | random `pub_seed`; compares the resulting `state_seeded` and, for `SPX_N >= 24`, `state_seeded_512` | [x] `tests/b_backend.rs::sha2::row55_seed_state` |
| 56 | `shake256` one-shot | `outlen` = 0, 1, 32, 135, 136, 137, 272, 300 x `inlen` sweep (rate 136) | [x] `tests/b_backend.rs::shake::row56_shake256_one_shot` |
| 57 | `shake256_absorb` + `shake256_squeezeblocks` | the non-incremental API: `inlen` sweep x `nblocks` = 0, 1, 2, 3; compares the squeezed output and the 25-word state | [x] `tests/b_backend.rs::shake::row57_shake256_absorb_squeezeblocks` |
| 58 | `shake256_inc_init` + `shake256_inc_absorb` x k + `shake256_inc_finalize` + `shake256_inc_squeeze` x k | absorb split into 1..4 chunks straddling the 136-byte rate, squeeze split into 1..4 chunks; compares the 26-word state after every step | [x] `tests/b_backend.rs::shake::row58_shake256_incremental` |
| 59 | `SPX_tweak_constants` | random `pub_seed`; compares `tweaked512_rc64` (640 B) and `tweaked256_rc32` (320 B) | [x] `tests/b_backend.rs::haraka::row59_tweak_constants` |
| 60 | `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | tweaked ctx x random 64/32-byte inputs | [x] `tests/b_backend.rs::haraka::row60_haraka_perm_and_blocks` |
| 61 | `SPX_haraka_S` one-shot | `outlen` = 0, 1, 31, 32, 33, 64, 100 x `inlen` sweep (rate 32) | [x] `tests/b_backend.rs::haraka::row61_haraka_s_one_shot` |
| 62 | `SPX_haraka_S_inc_init` + `_inc_absorb` x k + `_inc_finalize` + `_inc_squeeze` x k | absorb and squeeze split into 1..4 chunks straddling the 32-byte rate; compares the 65-byte `s_inc` after every step | [x] `tests/b_backend.rs::haraka::row62_haraka_s_incremental` |
| 63 | `randombytes_init` + `randombytes` | `personalization_string` = NULL and random; `xlen` = 0, 1, 15, 16, 17, 31, 32, 48, 100, 1000 in sequence so `V` advances across an AES block boundary; compares output and `DRBG_ctx` (`Key`, `V`, `reseed_counter`) after every call | [x] `tests/b_rng.rs::row63_randombytes_init_and_draw` |
| 64 | `randombytes_init` + `randombytes` | `V` forced to `FF..FF` by seeding then draining, so the `if (V[j] == 0xff)` carry chain in both `randombytes` and `AES256_CTR_DRBG_Update` propagates | [x] `tests/b_rng.rs::row64_randombytes_carry_chain` |
| 65 | `AES256_ECB` | random 32-byte key x random 16-byte counter, plus all-zero and all-`0xFF` | [x] `tests/b_rng.rs::row65_aes256_ecb` |
| 66 | `AES256_CTR_DRBG_Update` | `provided_data` = NULL and random 48 bytes; `V` = zero, random, `FF..FF` | [x] `tests/b_rng.rs::row66_drbg_update` |
| 67 | `seedexpander_init` + `seedexpander` | `maxlen` = 1, 2, 16, 17, 4096, `0xFFFFFFFF`; `xlen` = 1, 15, 16, 17, 32, 100 issued repeatedly so `buffer_pos` walks 16 -> 0 -> partial and `ctr[12..16]` increments; compares output and the whole `AES_XOF_struct` after every call | [x] `tests/b_rng.rs::row67_seedexpander` |
| 68 | `seedexpander` | `ctr[12..16]` pre-set to `FF FF FF FF` so the counter carry chain in the `for (i=15; i>=12; i--)` loop rolls over | [x] `tests/b_rng.rs::row68_seedexpander_counter_carry` |

## Result

Every row above is checked off, meaning its differential test passed against
both `.so` objects across its randomised inputs (fixed seeds, one per row) in
**all 96 build configurations**.  Reproduce with:

```
./build_c_all.sh              # 48 C configurations into cbuild/
translation/run_all_tests.sh  # 96 build+test runs, verdicts in test_results.txt
```

`translation/test_results.txt` holds 96 `PASS` lines and no `FAIL`; the raw
per-configuration output is in `/tmp/testlogs/<tag>.log`.

Rows 48-62 are backend specific by construction — `lib/CMakeLists.txt` compiles
exactly one backend, so `blake256`/`cst` exist only in the 24 blake
configurations, `sha256`/`SPX_mgf1_*`/`SPX_seed_state` only in the 24 sha2 ones,
`shake256*` only in the 24 shake ones and `SPX_haraka*` only in the 24 haraka
ones.  The tests are `cfg`-gated to match, which is why the suite reports 74
tests for blake, 73 for sha2 and haraka, and 72 for shake.

Rows 43, 44 and 46 compare `randombytes`-driven output only when the
deterministic `rng.c` provider is selected; under the `urandom` feature
`optrand` is genuinely non-deterministic, and row 47's cross-library round trip
is what covers those entry points there.  Row 64 likewise applies only to the
deterministic provider.

## Divergences found and fixed

| where | symptom | cause | fix |
|---|---|---|---|
| all eight `thash_*_{robust,simple}.rs` | `SPX_thash` panicked (slice index out of range) for any `inblocks` above `max(SPX_WOTS_LEN, SPX_FORS_TREES)`, where the C sized its `SPX_VLA` at run time and returned a digest — rows 17, 18, 29 | the scratch buffers were fixed-size arrays dimensioned by `SPX_THASH_MAX_INBLOCKS`, an internal-use bound the exported API does not enforce | added `src/vla.rs` (`Vla<N>`: stack-allocated while the length fits, heap beyond) and switched every `thash` scratch buffer to it |

## C behaviour deliberately preserved

Recorded here because these look like bugs and a future reader may be tempted to
"fix" the Rust:

* `hash_blake.c` passes **byte** counts to `blake256_update` / `blake512_update`,
  which interpret their length argument as **bits**.  Only the first `mlen/8`
  bytes of a message therefore reach `gen_message_random` and `hash_message`
  under the blake backend, so flipping a later message byte still verifies.
  The message-corruption cases of `ERRORS.md` rows 8/25/26 account for this
  rather than asserting a rejection the C does not make.
* `hash_blake.c`'s `gen_message_random` ends in `blakeX_final(&S, R)`, writing
  `SPX_BLAKE{256,512}_OUTPUT_BYTES` (32 or 64) rather than `SPX_N` bytes into
  `R`.  `sign.c` gets away with it because `R` is the head of an `SPX_BYTES`
  buffer.  The Rust wrapper slices `R` to the same width, and row 10 compares a
  256-byte sentinel-filled buffer so the write width is part of the comparison.
* `if (SPX_D == 1)` in every `hash_message` is unreachable: the smallest `SPX_D`
  across the 24 parameter sets is 7.
* None of `set_type`, `set_layer_addr`, `set_chain_addr`, `set_hash_addr`,
  `set_tree_height`, `set_keypair_addr` or `set_tree_index` validates its
  argument; they truncate and store.  Rows 1, 3 and `ERRORS.md` rows 27-28 pin
  that down.
