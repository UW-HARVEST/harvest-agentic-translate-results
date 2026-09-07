# CONFIGS.md -- configuration-surface table

## Axes the C code actually branches on

### A. Build-time (CMake cache variables -> cargo features)

| axis | values | what it switches in the C |
|------|--------|---------------------------|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | `lib/CMakeLists.txt: add_subdirectory(${HASH_BACKEND})`; picks `hash_*.c` + `thash_*` + the address-field offsets (`*_offsets.h`; sha2 uses a *different* set) and, in `context.h`, the shape of `spx_ctx` (`SPX_SHA2` -> `state_seeded[40]` (+`state_seeded_512[72]` when `SPX_SHA512`), `SPX_HARAKA` -> `tweaked512_rc64[10][8]` + `tweaked256_rc32[10][8]`) |
| `THASH` | `robust`, `simple` | `thash_<backend>_${THASH}.c` -- robust adds an MGF1/SHAKE/haraka_S bitmask XORed into the input |
| `SECPAR` | `128s`,`128f`,`192s`,`192f`,`256s`,`256f` | `SPX_N` (16/24/32), `SPX_FULL_HEIGHT`, `SPX_D`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`, and `SPX_SHA512`/`SPX_BLAKE512` (0 for 128, 1 for 192/256) which selects the 512-bit hash in `hash_*.c` (`#if SPX_N >= 24`) and the `inblocks > 1 -> thash_512` branch in `thash_{sha2,blake}_*.c` |

4 x 2 x 6 = **48 C configurations**; the Rust crate additionally accepts
`shake256` as an alias for `shake`, giving **60 cargo feature combinations**.

### B. Run-time input shapes the C special-cases

| axis | distinguished values | where |
|------|----------------------|-------|
| `mlen` | `0`; `< block`; exactly `block - SPX_N`; `>= block`; multi-block | `hash_sha2.c:94` `SPX_N + mlen < SPX_SHAX_BLOCK_BYTES`, `hash_sha2.c:156` `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*BLOCK`; blake/shake/haraka absorb-loop boundaries (64/128/136/32) |
| `inblocks` (`thash`) | `0`, `1`, `2`, `SPX_WOTS_LEN`, `SPX_FORS_TREES` | `thash_{sha2,blake}_*.c:23` `inblocks > 1 -> thash_512`; `thash_haraka_*.c:21` `inblocks == 1 -> haraka512/256 else haraka_S` |
| `leaf_idx` parity | even / odd | `compute_root` `leaf_idx & 1`, `*_treehashx1` `internal_idx & 1` |
| `leaf_idx == wots_sign_leaf` | equal / not equal | `wotsx1.c:28` -> `wots_k_mask = 0` (emit signature) vs `~0` (pk only) |
| `idx_leaf == (uint32_t)~0` | yes / no | `merkle_gen_root` passes `~0` so no auth path is produced |
| `tree_height` | `1`, `2`, `SPX_FORS_HEIGHT`, `SPX_TREE_HEIGHT` | `compute_root`, `treehash`, `wots_treehashx1`, `fors_treehashx1` (`idx == max_idx` special case) |
| `outlen` (MGF1 / SHAKE / haraka_S) | `< rate`, `== rate`, `> rate`, non-multiple of rate | `blake*_mgf1`, `shake*_squeeze`, `haraka_S_inc_squeeze` |
| address field values | `< 256` and `>= 256`; full 64-bit `tree` | `set_*` truncate to `unsigned char` (see ERRORS.md rows 15-19) |
| `outlen`/`inlen` = 0 | yes / no | `ull_to_bytes(0)`, `bytes_to_ull(0)`, `randombytes(0)`, `seedexpander(0)` |
| `siglen` | `== SPX_BYTES` / `!=` | `crypto_sign_verify` |
| `smlen` | `< SPX_BYTES` / `== SPX_BYTES` / `> SPX_BYTES` | `crypto_sign_open` |
| DRBG buffer position | `xlen <= 16 - buffer_pos` vs `>` | `seedexpander` refill loop |
| `personalization_string` / `provided_data` | `NULL` / non-`NULL` | `randombytes_init`, `AES256_CTR_DRBG_Update` |

### C. Full set of public entry points

Every symbol in `SYMBOLS.md` is an entry point, including the lowest-level ones
(`blake256_compress`, `sha256_inc_blocks`, `shake128_absorb`,
`SPX_haraka512_perm`, `SPX_ull_to_bytes`, ...), not just `crypto_sign*`.

---

## Rows (each is run for EVERY build-time configuration of §A)

Each row is driven with many pseudo-random inputs from a fixed-seed xorshift64\*
generator (`common::Rng`, seed `0x243F6A8885A308D3`), and the C and Rust outputs
are compared byte-for-byte (including every out-parameter and the whole
`spx_ctx` / DRBG state where applicable).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `SPX_ull_to_bytes` | `outlen` = 0,1,2,3,4,5,6,7,8,9,12,16 x 32 random `in` values (incl. `0`, `u64::MAX`) | [x] |
| 2  | `SPX_u32_to_bytes` | 64 random `u32` (incl. `0`, `u32::MAX`) | [x] |
| 3  | `SPX_bytes_to_ull` | `inlen` = 0..8 x 32 random byte strings | [x] |
| 4  | `SPX_set_layer_addr` | random addr x `layer` in {0,1,SPX_D-1,255,256,0x1FF,0xFFFFFFFF} + random | [x] |
| 5  | `SPX_set_tree_addr` | random addr x `tree` in {0,1,2^32,2^56,u64::MAX} + random | [x] |
| 6  | `SPX_set_type` | random addr x `type` in {0..6} (all valid) + {7,255,256,0x103,0xFFFFFFFF} | [x] |
| 7  | `SPX_set_keypair_addr` / `SPX_set_tree_index` | random addr x random `u32` (4-byte big-endian field) | [x] |
| 8  | `SPX_set_chain_addr` / `SPX_set_hash_addr` / `SPX_set_tree_height` | random addr x values {0,1,SPX_WOTS_LEN,SPX_WOTS_W-1,255,256,0xFFFFFFFF} | [x] |
| 9  | `SPX_copy_subtree_addr` / `SPX_copy_keypair_addr` | random `in`, random pre-filled `out` (checks the untouched bytes too) | [x] |
| 10 | `SPX_initialize_hash_function` | random `pub_seed`+`sk_seed`; whole `spx_ctx` compared (blake/shake: no-op; sha2: `seed_state`; haraka: `tweak_constants`) | [x] |
| 11 | `SPX_prf_addr` | random ctx x random addr (32 iterations) | [x] |
| 12 | `SPX_thash` | `inblocks` in {0,1,2,3,SPX_WOTS_LEN,SPX_FORS_TREES} x random ctx/in/addr; exercises simple/robust **and** the `inblocks>1 -> *_512` branch for 192/256 | [x] |
| 13 | `SPX_gen_message_random` | `mlen` in {0,1,15,16,17,31,32,33,47,48,63,64,65,71,72,79,80,103,104,111,112,127,128,135,136,137,167,168,169,255,256,1000} | [x] |
| 14 | `SPX_hash_message` | same `mlen` set; `tree` + `leaf_idx` out-params compared as well | [x] |
| 15 | `SPX_compute_root` | `tree_height` in {1,2,SPX_FORS_HEIGHT,SPX_TREE_HEIGHT} x `leaf_idx` even/odd/random x `idx_offset` 0/random | [x] |
| 16 | `SPX_treehash` (legacy, takes a `gen_leaf` callback) | callback = the library's own `SPX_fors_gen_leafx1`; `tree_height` in {1,2,3,SPX_FORS_HEIGHT} x `leaf_idx` in {0,1,2,mid,last,~0} x `idx_offset` 0/random | [x] |
| 17 | `SPX_chain_lengths` | 64 random `SPX_N`-byte messages | [x] |
| 18 | `SPX_wots_pk_from_sig` | random sig (`SPX_WOTS_BYTES`), random msg, random addr, random ctx | [x] |
| 19 | `SPX_wots_gen_leafx1` | `leaf_idx == info.wots_sign_leaf` (signature emitted) **and** `!=` (`wots_k_mask = ~0`); random `wots_steps` **and** `wots_steps` from `SPX_chain_lengths` | [x] |
| 20 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` in {0,1,mid,last,~0}, `idx_offset` 0/random | [x] |
| 21 | `SPX_fors_gen_leafx1` | random ctx, random `leaf_addrx`, `addr_idx` in {0,1,random,0xFFFFFFFF} | [x] |
| 22 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` in {0,1,mid,last,~0}, `idx_offset` in {0, i*2^FORS_HEIGHT} | [x] |
| 23 | `SPX_fors_sign` | random ctx, random `mhash` (`SPX_FORS_MSG_BYTES`), random `fors_addr`; both `sig` and `pk` compared | [x] |
| 24 | `SPX_fors_pk_from_sig` | random ctx, random `sig` (`SPX_FORS_BYTES`), random `mhash`, random addr | [x] |
| 25 | `SPX_merkle_sign` | random ctx, random `root`, random `wots_addr`/`tree_addr`, `idx_leaf` in {0,1,mid,last,~0}; `sig`, `root` and both addr arrays compared | [x] |
| 26 | `SPX_merkle_gen_root` | random ctx (uses `idx_leaf = ~0` internally) | [x] |
| 27 | `crypto_sign_secretkeybytes` / `publickeybytes` / `bytes` / `seedbytes` | no input | [x] |
| 28 | `crypto_sign_seed_keypair` | 4 random `CRYPTO_SEEDBYTES` seeds (incl. all-zero, all-`0xFF`) | [x] |
| 29 | `crypto_sign_keypair` | after identical `randombytes_init(entropy, NULL)`; also after `randombytes_init(entropy, ps)` | [x] |
| 30 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen` in {0,1,33,64,65,200}, DRBG re-seeded identically before each; `sig`, `siglen`, verify result compared | [x] |
| 31 | `crypto_sign` + `crypto_sign_open` | `mlen` in {0,1,33,64,200}; `sm`, `smlen`, recovered `m`, `mlen` compared | [x] |
| 32 | `randombytes_init` + `randombytes` | `personalization_string` = `NULL` and non-`NULL`; then a sequence of `xlen` in {0,1,15,16,17,31,32,33,48,100}; exported `DRBG_ctx` bytes compared after every call | [x] |
| 33 | `AES256_ECB` | 32 random (key, ctr) pairs | [x] |
| 34 | `AES256_CTR_DRBG_Update` | `provided_data` = `NULL` and non-`NULL`, random `Key`/`V` | [x] |
| 35 | `seedexpander_init` + `seedexpander` | `maxlen` in {16,17,100,4096,0xFFFFFFFF}; `xlen` sequences that stay inside the 16-byte buffer, exactly exhaust it, and cross it several times; whole `AES_XOF_struct` compared | [x] |
| 36 | blake: `blake256_init/update/final` | `inlen` in {0,1,2,54,55,56,57,63,64,65,111,127,128,129,255,256,1000}; multi-`update` splits | [x] |
| 37 | blake: `blake256` one-shot, `blake256_compress` | random `inlen`; `compress` on a random `blakestate256` + random 64-byte block | [x] |
| 38 | blake: `blake512_init/update/final`, `blake512`, `blake512_compress` | `inlen` in {0,1,111,112,113,127,128,129,239,240,255,256,1000}; multi-`update` splits; `compress` on random state + 128-byte block | [x] |
| 39 | blake: `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen` in {1,31,32,33,63,64,65,100,256} x `inlen` in {1,16,48,64,100} | [x] |
| 40 | blake: `cst` | the exported `const u64 cst[16]` compared word-for-word | [x] |
| 41 | sha2: `sha256_inc_init/inc_blocks/inc_finalize` | `nblocks` in {0,1,2,5} x final `inlen` in {0,1,54,55,56,57,63,64,65,200}; 40-byte state compared after every step | [x] |
| 42 | sha2: `sha256` one-shot | `inlen` in {0,1,55,56,63,64,65,119,120,128,1000} | [x] |
| 43 | sha2: `sha512_inc_init/inc_blocks/inc_finalize`, `sha512` | `nblocks` in {0,1,2,3} x final `inlen` in {0,1,110,111,112,113,127,128,129,300}; 72-byte state compared | [x] |
| 44 | sha2: `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen` in {1,31,32,33,63,64,65,100,256} x `inlen` in {1,16,48,64,100} | [x] |
| 45 | sha2: `SPX_seed_state` | random `pub_seed`; `state_seeded` (+ `state_seeded_512` when `SPX_SHA512`) compared | [x] |
| 46 | shake: `shake128_absorb` + `shake128_squeezeblocks` | `inlen` in {0,1,167,168,169,335,336,400} x `nblocks` in {1,2,3}; 25-word state compared -- **declared in `fips202.h` but NOT defined in `fips202.c`**, so there is no C side to compare against; the test skips itself (see note below) | n/a |
| 47 | shake: `shake128_inc_init/absorb/finalize/squeeze` | same shapes with the 168-byte rate -- **not defined in the C**, skipped (see note below) | n/a |
| 48 | shake: `shake256_absorb` + `shake256_squeezeblocks`, `shake256_inc_*` | same shapes with the 136-byte rate | [x] |
| 49 | shake: `shake256` one-shot (+ `shake128` if the C ever defines it) | `outlen` in {1,32,135,136,137,168,169,500} x `inlen` in {0,1,135,136,168,200}; plus a FIPS-202 `SHAKE256("abc")` anchor | [x] |
| 50 | shake: `sha3_256`, `sha3_256_inc_init/absorb/finalize` | **not defined in the C**, skipped (see note below) | n/a |
| 51 | shake: `sha3_512`, `sha3_512_inc_init/absorb/finalize` | **not defined in the C**, skipped (see note below) | n/a |
| 52 | haraka: `SPX_tweak_constants` | random `pub_seed`; both tweaked constant tables compared | [x] |
| 53 | haraka: `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | 32 random 64-/32-byte inputs, tweaked ctx | [x] |
| 54 | haraka: `SPX_haraka_S_inc_init/absorb/finalize/squeeze` | 1..3 `absorb` calls with lengths straddling the 32-byte rate x `outlen` in {1,16,32,33,64,100}; 65-byte state compared | [x] |
| 55 | haraka: `SPX_haraka_S` one-shot | `outlen` in {1,16,31,32,33,64,100} x `inlen` in {0,1,31,32,33,64,100} | [x] |
| 56 | driver (`PQCgenKAT_sign.c` <-> `src/main.rs`) | full KAT transcript, 7 keygen/sign/open rounds; digest compared against `reference_outputs.txt` for all 48 configurations | [x] |

## Note on rows 46, 47, 50, 51

`lib/shake/include/fips202.h` *declares* `shake128*`, `sha3_256*` and
`sha3_512*`, but `lib/shake/src/fips202.c` only *defines* the SHAKE-256 family
(everything else in that file is `static`).  `nm -D libshake.so` therefore lists
only:

```
SPX_gen_message_random SPX_hash_message SPX_initialize_hash_function
SPX_prf_addr SPX_thash
shake256 shake256_absorb shake256_squeezeblocks
shake256_inc_init shake256_inc_absorb shake256_inc_finalize shake256_inc_squeeze
```

The Rust port additionally exports the 15 declared-but-undefined entry points
(`shake128`, `shake128_absorb`, `shake128_squeezeblocks`,
`shake128_inc_{init,absorb,finalize,squeeze}`, `sha3_256`,
`sha3_256_inc_{init,absorb,finalize}`, `sha3_512`,
`sha3_512_inc_{init,absorb,finalize}`).  Extra exports do not break symbol
parity (the requirement is C ⊆ Rust), but there is no C implementation to
compare them with, so `tests/t08_shake.rs` gates those rows on
`libs().c.has(<symbol>)` and skips them instead of pretending to verify them.

## Status

All 52 applicable rows pass for **every one of the 60 cargo feature
combinations** (4+1 backends x 2 thash x 6 secpar), each driven by
`tests/t0*.rs` through the exported C symbols of both `.so`s.  Rows 46, 47, 50
and 51 are `n/a` because the C never defines those symbols (see the note above).

Reproduce with:

```sh
./run_tests.sh                    # all 60 combinations, 10 parallel workers
./run_tests.sh blake 128f simple  # one combination
./check_all.sh                    # cargo check on all 60 combinations
./symparity_all.sh                # nm -D parity over all 48 C configurations
./check_coverage.sh <backend>     # every C symbol is named by some test
```

`run_one.sh` performs, per combination:

1. `build_c.sh <backend> <secpar> <thash>` -- CMake build of
   `libsphincs_core.so`, `libsphincs_core_det.so`, `lib<backend>.so` and the
   KAT `driver`;
2. `cargo build --release --no-default-features --features <combo>`;
3. `cargo test --no-default-features --features <combo>` with `SPX_C_BUILD` and
   `SPX_RUST_SO` pointing at those artefacts -- 71 test functions, all of which
   `dlopen` both libraries and compare byte-for-byte;
4. the C and Rust KAT drivers are run and their transcript digests compared with
   each other **and** with `reference_outputs.txt`.
