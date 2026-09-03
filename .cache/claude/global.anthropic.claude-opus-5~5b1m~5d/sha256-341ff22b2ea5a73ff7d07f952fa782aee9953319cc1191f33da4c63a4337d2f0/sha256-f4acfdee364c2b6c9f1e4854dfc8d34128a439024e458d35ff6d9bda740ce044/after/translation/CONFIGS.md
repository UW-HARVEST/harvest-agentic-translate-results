# CONFIGS.md — configuration-surface table (valid inputs)

## The axes, derived from the C source

**Build-time axes** (the CMake cache variables in `c_src/CMakeLists.txt`, mapped
1:1 onto Cargo features by `build.rs`):

| axis | CMake variable | values | what it changes in the C |
|---|---|---|---|
| A1 | `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | `add_subdirectory(${HASH_BACKEND})`; selects `hash_*.c` + `thash_*.c`; also selects `<backend>_offsets.h`, and **`sha2` moves every `SPX_OFFSET_*` field** (LAYER 3→0, TREE 8→1, TYPE 19→9, KP_ADDR 20→10, CHAIN 27→17, HASH 31→21, TREE_HGT 27→17, TREE_INDEX 28→18). `sha2` additionally puts a seeded hash state inside `spx_ctx`; `haraka` puts tweaked round constants there |
| A2 | `THASH` | `robust`, `simple` | `thash_<backend>_${THASH}.c`. `robust` XORs an MGF1 bitmask into the input and hashes `addr‖masked-in`; `simple` hashes `pub_seed‖addr‖in` |
| A3 | `SECPAR` | `128s`,`128f`,`192s`,`192f`,`256s`,`256f` | `SPX_N` ∈ {16,24,32}; `SPX_D`; `SPX_TREE_HEIGHT`; `SPX_FORS_HEIGHT/TREES`; `SPX_WOTS_LEN`; **and the `SPX_BLAKE512`/`SPX_SHA512` flag (0 for 128-bit, 1 for 192/256-bit) which switches `thash`/`gen_message_random`/`hash_message` between the 256-bit and the 512-bit primitive** |

→ **48 build configurations** (4 × 2 × 6).  `shake256` is a Cargo alias of
`shake` (`shake256 = ["shake"]`), so it adds no distinct C configuration; the
alias is still exercised by `run_all.sh`, giving 60 Rust feature combinations.

Exact per-`SECPAR` shapes (computed from `c_src/app/params/params-sphincs-*.h`):

| SECPAR | N | FULL_HEIGHT | D | TREE_HEIGHT | FORS_HEIGHT | FORS_TREES | WOTS_LEN | WOTS_BYTES | FORS_MSG_BYTES | FORS_BYTES | SPX_BYTES | PK | SK | SEED | TREE_BITS | DGST_BYTES | 512-flag |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 128s | 16 | 63 | 7 | 9 | 12 | 14 | 35 | 560 | 21 | 2912 | 7856 | 32 | 64 | 48 | 54 | 30 | 0 |
| 128f | 16 | 66 | 22 | 3 | 6 | 33 | 35 | 560 | 25 | 3696 | 17088 | 32 | 64 | 48 | 63 | 34 | 0 |
| 192s | 24 | 63 | 7 | 9 | 14 | 17 | 51 | 1224 | 30 | 6120 | 16224 | 48 | 96 | 72 | 54 | 39 | 1 |
| 192f | 24 | 66 | 22 | 3 | 8 | 33 | 51 | 1224 | 33 | 7128 | 35664 | 48 | 96 | 72 | 63 | 42 | 1 |
| 256s | 32 | 64 | 8 | 8 | 14 | 22 | 67 | 2144 | 39 | 10560 | 29792 | 64 | 128 | 96 | 56 | 47 | 1 |
| 256f | 32 | 68 | 17 | 4 | 9 | 35 | 67 | 2144 | 40 | 11200 | 49856 | 64 | 128 | 96 | 64 | 49 | 1 |

**Run-time axes** (every `if` / `switch` / loop-shape the C actually branches
on, per entry point):

| axis | where | values that take different branches |
|---|---|---|
| R1 | `thash(out,in,inblocks,…)` | `inblocks` = `0`, `1`, `2`, `SPX_WOTS_LEN`, `SPX_FORS_TREES`, large. **`inblocks > 1` selects the 512-bit primitive when the 512-flag is 1** (`thash_*_{simple,robust}.c:22-27`) |
| R2 | `compute_root` (`utils.c:60,78`) | `leaf_idx & 1` (even/odd) at *every* level → the whole bit pattern of `leaf_idx` matters; `idx_offset` `0` vs non-zero; `tree_height` `1` (loop body never runs) vs `>1` |
| R3 | `treehash` (`utils.c:126,131,147`) | auth-path-node test `(leaf_idx^1)==idx`; stack-merge loop `heights[off-1]==heights[off-2]`; `tree_height` `0`,`1`,`2`,`FORS_HEIGHT`; `leaf_idx` inside / outside `[0,2^h)`; `idx_offset` `0` vs non-zero |
| R4 | `wots_treehashx1` / `fors_treehashx1` (`utilsx1.c:52,62,74`) | `h == tree_height` (root); `(internal_idx^internal_leaf)==1` (auth path); `(internal_idx&1)==0 && idx<max_idx` (left child, non-final) — the `idx == max_idx` exception is a distinct path; `leaf_idx = ~0u` (never matches, as `merkle_gen_root` does) vs a real leaf |
| R5 | `wots_gen_leafx1` (`wotsx1.c:28-35,57`) | `leaf_idx == info->wots_sign_leaf` → `wots_k_mask = 0` (signature *is* emitted, `info->wots_sig` written) vs `!=` → `wots_k_mask = ~0u` (`wots_k` can never equal `k`, no signature); and `info->wots_steps[i]` = `0`, `1`, `SPX_WOTS_W-1`, `>= SPX_WOTS_W` |
| R6 | `gen_chain` (`wots.c:34`) | `start` = `0`…`W-1`; `steps` = `0` (no hashing) … `W-1-start`; the `i < SPX_WOTS_W` clamp fires when `start+steps > W` |
| R7 | `chain_lengths` / `base_w` / `wots_checksum` (`wots.c:45-91`) | message bytes: all-zero (checksum maximal), all-`0xff` (checksum 0), random; only `SPX_WOTS_LOGW = 4` is instantiated |
| R8 | `hash_message` (`hash_*.c`) | `mlen` = `0`, `1`, block−1, block, block+1, multi-block, > `SPX_DGST_BYTES`; the `SPX_D == 1` branch is dead for all 6 param sets (`D >= 7`) but is still a source branch |
| R9 | `gen_message_random` | `mlen` = `0`, `1`, spanning the internal 64/128-byte buffer boundary |
| R10 | `blake256_update`/`blake256_final` (`blake256.c`) | `buflen == 440`, `< 440`, `> 440` → 1 vs 2 compressions; `nullt` set when `buflen == 0`; `t[0]` 32-bit wraparound. Exercised via `inlen` = 0,1,55,56,63,64,65,119,120,128,1000 |
| R11 | `blake512_update`/`blake512_final` (`blake512.c`) | same, with the 888/1024-bit boundaries → `inlen` = 0,1,111,112,127,128,129,239,240,256,1000 |
| R12 | `blake256_mgf1`/`blake512_mgf1` (`blake256.c:381`) | `outlen` = 0, 1, 31, 32, 33, 64, 65, 100 (i.e. `< block`, `== block`, `> block`, non-multiple) and `inlen` = 0,1,32,64 |
| R13 | `seedexpander` (`rng.c:74-100`) | `xlen <= 16-buffer_pos` (served from buffer, early return) vs `>` (re-key loop); `buffer_pos` `0`,`1`,`15`,`16`; counter-carry when `ctr[15]==0xff` (and cascading to `ctr[12]`) |
| R14 | `randombytes` (`rng.c:158-178`) | `xlen > 15` (full 16-byte copy) vs `<= 15` (partial); `xlen` = 1,15,16,17,31,32,48,1000; `V` all-`0xff` carry cascade |
| R15 | `randombytes_init` (`rng.c:143`) | `personalization_string` NULL vs non-NULL |
| R16 | `crypto_sign` / `crypto_sign_open` | `mlen` = 0, 1, 32, 33, 1000 |
| R17 | address setters | field-offset interaction: `copy_subtree_addr` copies `SPX_OFFSET_TREE+8` bytes, `copy_keypair_addr` copies that *plus* 4 bytes at `SPX_OFFSET_KP_ADDR`. Because `sha2` moves TREE to offset 1, the two backends copy 16 vs 11 bytes → distinct byte layouts must be compared, starting from a fully-random `addr` (not zeros) |

## The rows

Each row is checked with **many randomized inputs** (fixed seed, see
`tests/common/mod.rs::Rng`) and under **every one of the 48 build
configurations** (driven by `run_all.sh`).  Both the C `.so` and the Rust `.so`
are called through their exported symbols via `libloading`; nothing is called
directly.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `SPX_ull_to_bytes` | `outlen` ∈ {0,1,2,3,4,5,6,7,8,9,16,32} × 64 random `in` values incl. 0 and `UINT64_MAX` | `cfg01_ull_to_bytes` | [x] |
| 2 | `SPX_u32_to_bytes` | 256 random `u32` + {0,1,0x7fffffff,0xffffffff} | `cfg02_u32_to_bytes` | [x] |
| 3 | `SPX_bytes_to_ull` | `inlen` ∈ {0..9,16} × random byte strings + all-zero + all-`0xff` | `cfg03_bytes_to_ull` | [x] |
| 4 | all 10 address setters/copiers | start from a **random** 32-byte `addr` (so every offset is observable); random `layer`/`tree`/`type`/`keypair`/`chain`/`hash`/`tree_height`/`tree_index`; 256 iterations; compares the full 32 bytes. Covers axis R17 and the sha2-vs-rest offset split (A1) | `cfg04_address_setters` | [x] |
| 5 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the **entire** `spx_ctx` byte image afterwards (this is where sha2's `state_seeded`/`state_seeded_512` and haraka's tweaked round constants are produced) | `cfg05_initialize_hash_function` | [x] |
| 6 | `SPX_prf_addr` | random ctx × random `addr` (incl. all-zero and all-`0xff` addr), 64 iterations | `cfg06_prf_addr` | [x] |
| 7 | `SPX_thash` | `inblocks` ∈ {0,1,2,3,`SPX_WOTS_LEN`,`SPX_FORS_TREES`, `max(WOTS_LEN,FORS_TREES)+1`, `+8`, `2x`, 200} × random ctx/in/addr, 16 iterations each. Axis R1 + A2 (robust vs simple) + the 512-flag switch. The over-max values matter because the C sizes its scratch with `SPX_VLA` and therefore has **no** upper bound on `inblocks` | `cfg07_thash` | [x] |
| 8 | `SPX_gen_message_random` | `mlen` ∈ {0,1,31,32,33,55,56,63,64,65,111,112,127,128,129,1000} × random `sk_prf`/`optrand`/`m`. Axes R9,R10,R11. **The output buffer is 64+ bytes and compared in full**: for blake with `SPX_N >= 24`, `hash_blake.c:68` ends in `blake512_final(&S, R)`, which writes all 64 digest bytes through the caller's `R` pointer even though every in-tree caller only owns `SPX_N` | `cfg08_gen_message_random` | [x] |
| 9 | `SPX_hash_message` | same `mlen` set + `{DGST_BYTES-1, DGST_BYTES, DGST_BYTES+1}`; compares `digest`, `tree` **and** `leaf_idx`. Axis R8 | `cfg09_hash_message` | [x] |
| 10 | `SPX_chain_lengths` | random `msg`, all-zero `msg`, all-`0xff` `msg`; compares all `SPX_WOTS_LEN` lengths. Axis R7 | `cfg10_chain_lengths` | [x] |
| 11 | `SPX_wots_pk_from_sig` | random `sig`/`msg`/ctx/`addr`; compares `pk` (`SPX_WOTS_BYTES`) **and** the mutated `addr`. Axes R6,R7 | `cfg11_wots_pk_from_sig` | [x] |
| 12 | `SPX_wots_gen_leafx1` | `leaf_info_x1` with (a) `wots_sign_leaf == leaf_idx` and a real `wots_sig` buffer, (b) `wots_sign_leaf = ~0u` and `wots_sig = NULL`; `wots_steps` = all-0, all-`W-1`, random, and out-of-range (`>= W`); compares `dest`, `wots_sig`, and both addresses in the info struct. Axis R5 | `cfg12_wots_gen_leafx1` | [x] |
| 13 | `SPX_fors_gen_leafx1` | random ctx, random `fors_gen_leaf_info.leaf_addrx`, `addr_idx` ∈ {0,1,`2^FORS_HEIGHT-1`, random}; compares `leaf` and the mutated `leaf_addrx` | `cfg13_fors_gen_leafx1` | [x] |
| 14 | `SPX_compute_root` | `tree_height` ∈ {1,2,3,`FORS_HEIGHT`,`TREE_HEIGHT`} × `leaf_idx` ∈ {0,1,2,3,`2^h-1`, random} × `idx_offset` ∈ {0, random} ; compares `root` and the mutated `addr`. Axis R2 | `cfg14_compute_root` | [x] |
| 15 | `SPX_treehash` (with a C **callback**) | `tree_height` ∈ {0,1,2,3, min(FORS_HEIGHT,4)} × `leaf_idx` ∈ {0,1,`2^h-1`, `2^h` (out of range), `~0u`} × `idx_offset` ∈ {0, random}. The `gen_leaf` function pointer is supplied by the *test* and must be invoked with identical arguments in identical order by both libraries — the test records the call sequence. Axis R3 | `cfg15_treehash` | [x] |
| 16 | `SPX_wots_treehashx1` | driven exactly as `merkle_sign` does: `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` ∈ {0,1,`2^h-1`,`~0u`, random}, `idx_offset` ∈ {0, random}, `info.wots_steps` from `chain_lengths(root)`; compares `root`, the full auth path, `info.wots_sig`, and the mutated `tree_addr`. Axis R4 | `cfg16_wots_treehashx1` | [x] |
| 17 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` ∈ {0,1,`2^h-1`, random}, `idx_offset` ∈ {0, `i*2^FORS_HEIGHT`}; compares `root`, auth path, and the mutated `tree_addr` + `leaf_addrx`. Axis R4 | `cfg17_fors_treehashx1` | [x] |
| 18 | `SPX_fors_sign` | random ctx × random `m` (`SPX_FORS_MSG_BYTES`) incl. all-zero and all-`0xff` × random `fors_addr`; compares the whole `SPX_FORS_BYTES` signature and the `pk` | `cfg18_fors_sign` | [x] |
| 19 | `SPX_fors_pk_from_sig` | (a) on the signature produced by row 18 (valid), (b) on a fully random `sig`; random `m`/`addr`; compares `pk` | `cfg19_fors_pk_from_sig` | [x] |
| 20 | `SPX_merkle_sign` | random ctx, `wots_addr`/`tree_addr` set up as `crypto_sign_signature` does for each layer `i ∈ 0..SPX_D`, `idx_leaf` ∈ {0,1,`2^TREE_HEIGHT-1`,`~0u`, random}; compares the `SPX_WOTS_BYTES + TREE_HEIGHT*N` output, the updated `root`, and both addresses | `cfg20_merkle_sign` | [x] |
| 21 | `SPX_merkle_gen_root` | random ctx (`pub_seed`,`sk_seed`); compares the `SPX_N`-byte root | `cfg21_merkle_gen_root` | [x] |
| 22 | `crypto_sign_{secretkeybytes,publickeybytes,bytes,seedbytes}` | no inputs; the four size constants must agree, and agree with the table above | `cfg22_size_constants` | [x] |
| 23 | `crypto_sign_seed_keypair` | random 3·N-byte seeds, incl. all-zero and all-`0xff`; compares `pk` and `sk` | `cfg23_seed_keypair` | [x] |
| 24 | `randombytes_init` + `crypto_sign_keypair` | seed the DRBG identically in both libraries, then let `crypto_sign_keypair` pull its own randomness through `randombytes()`; compares `pk`/`sk` **and** the resulting `DRBG_ctx` image. This is the only entry point that consumes global RNG state | `cfg24_keypair_via_drbg` | [x] |
| 25 | `crypto_sign_signature` | DRBG seeded identically (it calls `randombytes(optrand)`), `mlen` ∈ {0,1,32,33,64,1000}; compares `sig`, `siglen`, and `DRBG_ctx` | `cfg25_sign_signature` | [x] |
| 26 | `crypto_sign_verify` | on the row-25 signature (expect 0) and with `siglen` = `SPX_BYTES` but a corrupted `sig`/`m`/`pk` (expect −1) | `cfg26_sign_verify` | [x] |
| 27 | `crypto_sign` | `mlen` ∈ {0,1,32,1000}; compares the whole `sm` (signature ‖ message) and `smlen` | `cfg27_crypto_sign` | [x] |
| 28 | `crypto_sign_open` | on the row-27 `sm` (expect 0, message recovered) for the same `mlen` set; compares `m` and `mlen` | `cfg28_crypto_sign_open` | [x] |
| 29 | full pipeline cross-check | C-produced signature verified by Rust and vice versa, for random seeds/messages — catches any asymmetry the same-library round trips would hide | `cfg29_cross_verify` | [x] |
| 30 | `AES256_ECB` | 64 random (key, ctr) pairs + all-zero + all-`0xff` | `cfg30_aes256_ecb` | [x] |
| 31 | `AES256_CTR_DRBG_Update` | `provided_data` NULL and non-NULL; `Key`/`V` random, all-zero, and `V` = all-`0xff` (carry cascade). Axis R15/B18 | `cfg31_drbg_update` | [x] |
| 32 | `randombytes_init` + `randombytes` + `DRBG_ctx` | `entropy_input` random; `personalization_string` NULL / random; then a *sequence* of `randombytes` calls with `xlen` ∈ {1,15,16,17,31,32,48,1000}; after every call compares the output **and** the full `DRBG_ctx` (Key, V, reseed_counter). Axis R14 | `cfg32_drbg_stream` | [x] |
| 33 | `seedexpander_init` + `seedexpander` | `maxlen` ∈ {1,16,17,256,0xFFFFFFFF} × a sequence of `seedexpander` calls with `xlen` ∈ {1,2,15,16,17,31,32,33,100} that walks `buffer_pos` through 0..16 and forces the `ctr` carry; compares output and the full `AES_XOF_struct` after each call. Axis R13 | `cfg33_seedexpander_stream` | [x] |
| 34 | `blake256_init/update/final/compress`, `blake256` | *(blake only)* `inlen` ∈ {0,1,2,55,56,57,63,64,65,119,120,127,128,129,1000} random data; incremental updates in random chunk sizes vs one-shot; `blake256_compress` on random 64-byte blocks with a random state. Axis R10 | `cfg34_blake256` | [x] |
| 35 | `blake512_init/update/final/compress`, `blake512` | *(blake only)* `inlen` ∈ {0,1,2,111,112,113,127,128,129,239,240,255,256,257,1000}; incremental vs one-shot; `blake512_compress` on random 128-byte blocks. Axis R11 | `cfg35_blake512` | [x] |
| 36 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1`, `cst` | *(blake only)* `outlen` ∈ {0,1,31,32,33,63,64,65,100,200,512,1000} × `inlen` ∈ {0,1,16,32,64,192,256,257,1000} (the large `inlen`s cross the stack/heap-fallback threshold that stands in for the C VLA); plus the `cst` data symbol compared byte-for-byte. Axis R12 | `cfg36_blake_mgf1` | [x] |
| 37 | `sha256_inc_init/blocks/finalize`, `sha256` | *(sha2 only)* `inlen` ∈ {0,1,55,56,57,63,64,65,119,120,1000}; incremental block feeding vs one-shot | `cfg37_sha256` | [x] |
| 38 | `sha512_inc_init/blocks/finalize`, `sha512` | *(sha2 only)* `inlen` ∈ {0,1,111,112,113,127,128,129,1000} | `cfg38_sha512` | [x] |
| 39 | `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state` | *(sha2 only)* the same `outlen` × `inlen` grid as row 36; `SPX_seed_state` on random ctx compared over the whole ctx image | `cfg39_sha2_mgf1_seed_state` | [x] |
| 40 | `shake256`, `shake256_absorb/squeezeblocks`, `shake256_inc_*` | *(shake only)* `inlen` ∈ {0,1,135,136,137,271,272,1000} × `outlen` ∈ {0,1,32,135,136,137,272}; one-shot vs absorb+squeezeblocks vs incremental | `cfg40_shake256` | [x] |
| 41 | `SPX_haraka_S`, `SPX_haraka_S_inc_*`, `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm`, `SPX_tweak_constants` | *(haraka only)* `SPX_tweak_constants` on random seeds first (it fills the ctx round constants), then `haraka256`/`haraka512`/`haraka512_perm` on random blocks, and `haraka_S` with `inlen` ∈ {0,1,31,32,33,63,64,65,1000} × `outlen` ∈ {0,1,32,33,64,100,640,1000} | `cfg41_haraka` | [x] |

Rows 34–41 are backend-specific: each is compiled in only for the configurations
whose backend exports those symbols (`#[cfg(feature = …)]`), which is exactly the
set of configurations in which the C exports them (see `SYMBOLS.md`).

## How to reproduce

From the repository root:

```sh
./build_c_all.sh     # C reference: 48 CMake configurations + one combined .so each
./build_rust_all.sh  # Rust cdylib for each configuration
./symdiff.sh         # nm -D parity check for all 48
./gen_symbols.sh     # regenerates translation/SYMBOLS.md
./run_all.sh         # everything above + Phase B/C tests for all 60 feature combos
```

To drive a single configuration by hand:

```sh
cd translation
cargo build --release --no-default-features --features blake,simple,128f
cp target/release/libsphincs_core_det.so ../rbuild/blake-simple-128f/
cargo test --no-default-features --features blake,simple,128f
```

The tests locate the two shared objects at
`../cbuild/<combo>/libcsphincs_all.so` and
`../rbuild/<combo>/libsphincs_core_det.so`; both paths can be overridden with
the `SPX_C_LIB` and `SPX_RUST_LIB` environment variables.

The `--offline` flag is needed in sandboxes without crates.io access
(`libloading` and `aes` are already in the local cargo cache).
