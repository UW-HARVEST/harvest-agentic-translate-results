# CONFIGS.md — configuration-surface table (valid inputs)

## Axes the C code actually branches on

### Build-time axes (CMake cache variables → Cargo features)

| axis | values | what it changes in the C |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | which `lib/<backend>` is linked (`lib/CMakeLists.txt: add_subdirectory(${HASH_BACKEND})`); defines `SPX_HARAKA`/`SPX_SHA2`/`SPX_SHAKE`/`SPX_BLAKE`, which in turn selects the address-field offsets (`*_offsets.h`) and the `spx_ctx` extra fields (`context.h`) |
| `THASH` | `robust`, `simple` | `thash_<backend>_${THASH}.c` — completely different `thash` body (bitmask/MGF1 pre-whitening vs. plain) |
| `SECPAR` | `128s`,`128f`,`192s`,`192f`,`256s`,`256f` | `SPX_N` ∈ {16,24,32}, `SPX_FULL_HEIGHT`, `SPX_D`, `SPX_TREE_HEIGHT`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`, and the derived `SPX_SHA512`/`SPX_BLAKE512` flag (`SPX_N >= 24`) which switches the 512-bit primitive |

4 × 2 × 6 = **48 valid combinations**; every one is built and tested.
No other feature combination is valid (exactly one backend, exactly one thash,
exactly one secpar).

Derived branch conditions inside the C that follow from the axes:

* `#if SPX_N >= 24` in `hash_sha2.c` / `hash_blake.c` → `shaX`/`blakeX` is the
  512-bit primitive (affects `gen_message_random` and `hash_message`).
* `#if SPX_SHA512` / `#if SPX_BLAKE512` in `thash_*` → `if (inblocks > 1)`
  dispatch to `thash_512`. So for `192*`/`256*` parameter sets `thash` uses a
  **different hash function depending on `inblocks`**.
* `#ifdef SPX_SHA2` in `context.h` → `state_seeded[40]`, and
  `#if SPX_SHA512` → `state_seeded_512[72]`.
* `#ifdef SPX_HARAKA` in `context.h` → `tweaked512_rc64[10][8]`,
  `tweaked256_rc32[10][8]` (so `spx_ctx` must be seeded via
  `initialize_hash_function` before any hash call).
* `SPX_OFFSET_*`: `sha2` uses the compressed 22-byte address layout
  (`SPX_OFFSET_LAYER 0`, `TREE 1`, `TYPE 9`, `KP_ADDR 10`, `CHAIN 17`, `HASH 21`,
  `TREE_HGT 17`, `TREE_INDEX 18`); the other three backends use the full 32-byte
  layout (`3/8/19/20/27/31/27/28`). This changes what `set_*_addr` /
  `copy_subtree_addr` / `copy_keypair_addr` touch.
* `if (SPX_D == 1) *tree = 0;` in every `hash_message` (never taken for the 48
  shipped parameter sets, `SPX_D` ∈ {7,8,17,22} — kept as a documented row).
* `haraka`'s `thash` branches on `inblocks == 1` (F-function via
  `haraka512`/`haraka256`) vs `!= 1` (sponge via `haraka_S`).
* `haraka`'s `hash_message` absorbs `pk + SPX_N` (root only), the other backends
  absorb the whole `pk` (`SPX_PK_BYTES`).
* `sha2`'s `gen_message_random` / `hash_message` branch on whether
  `optrand‖m` (resp. `R‖pk‖m`) fills a whole block:
  `if (SPX_N + mlen < SPX_SHAX_BLOCK_BYTES)` and
  `if (SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*SPX_SHAX_BLOCK_BYTES)`.
* `blake*_update`'s `left && (((datalen>>3) & 0x7F) >= fill)` pre-fill branch and
  `blake*_final`'s three padding branches (`buflen == 440/888`, `< 440/888`,
  else two compressions) depend on the message length modulo the block size.
* `shake256_inc_absorb`/`squeeze` and `haraka_S_inc_absorb`/`squeeze` branch on
  the rate boundary (136 resp. 32 bytes).
* `wots.c: gen_chain` loop `for (i = start; i < start+steps && i < SPX_WOTS_W; i++)`
  — `steps == 0` and `start == SPX_WOTS_W-1` are distinct paths.
* `utils.c: compute_root` branches on the parity of `leaf_idx` at every level.
* `utilsx1.c: wots_treehashx1`/`fors_treehashx1` branch on
  `(internal_idx & 1) == 0 && idx < max_idx` (left child vs. flush-at-end) and on
  `(internal_idx ^ internal_leaf) == 0x01` (auth-path node).
* `wotsx1.c: wots_gen_leafx1` branches on `leaf_idx == info->wots_sign_leaf`
  (signing leaf → `wots_k_mask = 0`, writes into `info->wots_sig`) vs. not
  (`wots_k_mask = ~0`, no signature output). `merkle_gen_root` deliberately uses
  `idx_leaf = ~0u` so the second branch is always taken.

### Runtime axes (the public API has no option flags; the "options" are the
### pointer/length arguments and the address-word state)

| axis | distinct shapes the C treats differently |
|---|---|
| `mlen` | `0`; `1`; `SPX_N-1`; `SPX_N`; `SPX_N+1`; `block-1`, `block`, `block+1` for the sha2 block (64/128) and the blake block (64/128) and the shake rate (136) and the haraka rate (32); a multi-block length; a length that lands exactly on `SPX_INBLOCKS*BLOCK - SPX_N - SPX_PK_BYTES` |
| `inblocks` (`thash`) | `0`; `1`; `2`; `SPX_WOTS_LEN`; `SPX_FORS_TREES`; `255` |
| `addr[8]` | all-zero; all-`0xFF`; random; each `set_*` field at its min/max; every `SPX_ADDR_TYPE_*` value 0..6 plus out-of-range 7/255/`u32::MAX` |
| `tree_height` | `0`; `1`; `2`; `SPX_TREE_HEIGHT`; `SPX_FORS_HEIGHT` |
| `leaf_idx` / `idx_offset` | `0`; `1`; odd; even; `(1<<h)-1`; `~0u` (the "don't generate auth path" sentinel) |
| `siglen` / `smlen` | see `ERRORS.md` |
| `seed` (`crypto_sign_seed_keypair`) | all-zero; all-`0xFF`; random (`3*SPX_N` bytes) |
| DRBG state | freshly `randombytes_init`'ed; after N draws; `V` all-`0xFF`; entropy all-zero / all-`0xFF`; `personalization_string` NULL vs. non-NULL |

### Full set of public entry points (from the C headers)

Lowest level first — **all** of these are driven directly by the tests, not only
the one-shot wrappers:

* `utils.h`: `SPX_ull_to_bytes`, `SPX_u32_to_bytes`, `SPX_bytes_to_ull`,
  `SPX_compute_root`, `SPX_treehash` (**takes a `gen_leaf` function pointer**)
* `address.h`: `SPX_set_layer_addr`, `SPX_set_tree_addr`, `SPX_set_type`,
  `SPX_copy_subtree_addr`, `SPX_set_keypair_addr`, `SPX_set_chain_addr`,
  `SPX_set_hash_addr`, `SPX_copy_keypair_addr`, `SPX_set_tree_height`,
  `SPX_set_tree_index`
* `thash.h`: `SPX_thash`
* `hash.h`: `SPX_initialize_hash_function`, `SPX_prf_addr`,
  `SPX_gen_message_random`, `SPX_hash_message`
* `wots.h` / `wotsx1.h`: `SPX_chain_lengths`, `SPX_wots_pk_from_sig`,
  `SPX_wots_gen_leafx1`
* `fors.h` / `forsx1.h`: `SPX_fors_gen_leafx1`, `SPX_fors_sign`,
  `SPX_fors_pk_from_sig`
* `utilsx1.h`: `SPX_wots_treehashx1`, `SPX_fors_treehashx1`
* `merkle.h`: `SPX_merkle_sign`, `SPX_merkle_gen_root`
* `api.h`: `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`,
  `crypto_sign_bytes`, `crypto_sign_seedbytes`, `crypto_sign_seed_keypair`,
  `crypto_sign_keypair`, `crypto_sign_signature`, `crypto_sign_verify`,
  `crypto_sign`, `crypto_sign_open`
* `rng.h`: `AES256_CTR_DRBG_Update`, `AES256_ECB`, `seedexpander_init`,
  `seedexpander`, `randombytes_init`, `randombytes`, and the global `DRBG_ctx`
* backend primitives: `blake256{,_init,_update,_final,_compress}`,
  `blake512{,_init,_update,_final,_compress}`, `SPX_blake256_mgf1`,
  `SPX_blake512_mgf1`, `cst`; `sha256{,_inc_init,_inc_blocks,_inc_finalize}`,
  `sha512{...}`, `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`;
  `shake256{,_absorb,_squeezeblocks,_inc_init,_inc_absorb,_inc_finalize,_inc_squeeze}`;
  `SPX_tweak_constants`, `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm`,
  `SPX_haraka_S`, `SPX_haraka_S_inc_{init,absorb,finalize,squeeze}`

## The table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5150_4849_4E43_5321`, a SplitMix64 PRNG in the test harness) unless the row is
inherently a single fixed value, and every row is run for **all 48**
`(backend, thash, secpar)` combinations.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| C1 | `SPX_ull_to_bytes` | `outlen` ∈ {0,1,2,3,4,5,6,7,8,9,16} × random `u64` values incl. 0, 1, `u64::MAX`, `1<<63` | `cfg_ull_to_bytes` | [x] |
| C2 | `SPX_u32_to_bytes` | random `u32` incl. 0, `u32::MAX`, `0x0000_00FF`, `0xFF00_0000` | `cfg_u32_to_bytes` | [x] |
| C3 | `SPX_bytes_to_ull` | `inlen` ∈ {0..8} × random byte strings | `cfg_bytes_to_ull` | [x] |
| C4 | `SPX_set_*_addr` (all 8 setters) | random `addr[8]` start state × random values × every setter, applied in random order; checks the full 32-byte address word after each call (covers both the `sha2` 22-byte offsets and the 32-byte offsets) | `cfg_address_setters` | [x] |
| C5 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random source and destination addresses (verifies exactly `SPX_OFFSET_TREE+8` bytes, plus the extra 4 bytes at `SPX_OFFSET_KP_ADDR` for the keypair variant, and that nothing else changes) | `cfg_address_copy` | [x] |
| C6 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the **whole** resulting `spx_ctx` byte image (this is the only way the `sha2` `state_seeded`/`state_seeded_512` and the `haraka` tweaked round constants are observable) | `cfg_initialize_hash_function` | [x] |
| C7 | `SPX_prf_addr` | random ctx × random `addr[8]` (incl. all-zero and all-`0xFF`) | `cfg_prf_addr` | [x] |
| C8 | `SPX_thash`, `inblocks = 1` | random ctx × random `addr` × random input; **both** `robust` and `simple` bodies; for `SPX_N>=24` this is the 256-bit path | `cfg_thash_inblocks` | [x] |
| C9 | `SPX_thash`, `inblocks = 2` | as above; for `SPX_N>=24` this crosses into the `thash_512` branch, and for `haraka` into the `haraka_S` branch | `cfg_thash_inblocks` | [x] |
| C10 | `SPX_thash`, `inblocks ∈ {0,3,SPX_WOTS_LEN,SPX_FORS_TREES,255}` | random data of the matching size | `cfg_thash_inblocks` | [x] |
| C11 | `SPX_gen_message_random` (note: for the BLAKE backend the C writes the **full** 32-/64-byte digest into `R`, and passes byte counts where `blake*_update` wants bit counts — see ERRORS.md; the tests use an over-sized `R` and compare all of it) | `mlen` ∈ {0,1,15,16,17,23,24,25,31,32,33,63,64,65,71,72,73,79,80,81,95,96,97,111,112,113,127,128,129,135,136,137,199,200,255,256,1000} × random `sk_prf`/`optrand`/message. Covers every block/rate boundary of all four backends and the `sha2` `SPX_N + mlen < BLOCK` branch | `cfg_gen_message_random` | [x] |
| C12 | `SPX_hash_message` | same `mlen` set × random `R`, `pk`, message; checks `digest`, `tree` **and** `leaf_idx`. Covers the `sha2` `SPX_N+SPX_PK_BYTES+mlen < SPX_INBLOCKS*BLOCK` branch and the `tree`/`leaf_idx` masking | `cfg_hash_message` | [x] |
| C13 | `SPX_chain_lengths` | random `SPX_N`-byte messages + all-zero + all-`0xFF` (min/max checksum) | `cfg_chain_lengths` | [x] |
| C14 | `SPX_wots_pk_from_sig` | random ctx × random `addr` × random `SPX_WOTS_BYTES` signature × random `SPX_N` message digest; also compares the **mutated `addr`** afterwards (the C leaves `chain`/`hash` fields set) | `cfg_wots_pk_from_sig` | [x] |
| C15 | `SPX_wots_gen_leafx1`, non-signing leaf | `info.wots_sign_leaf = ~0u`, `wots_sig = NULL`, `wots_steps` = random steps; random `leaf_idx`; compares `dest` **and** the mutated `info` (`leaf_addr`, `pk_addr`) | `cfg_wots_gen_leafx1_nosig` | [x] |
| C16 | `SPX_wots_gen_leafx1`, signing leaf | `info.wots_sign_leaf == leaf_idx`, real `wots_sig` buffer; compares `dest`, the `SPX_WOTS_BYTES` signature and the mutated `info`. `wots_steps` from `SPX_chain_lengths` and also random in `0..SPX_WOTS_W` incl. 0 and `SPX_WOTS_W-1` | `cfg_wots_gen_leafx1_sig` | [x] |
| C17 | `SPX_treehash` (function-pointer API) | `tree_height` ∈ {0,1,2,3} × `leaf_idx` ∈ {0,1,2,3,`~0u`} × random `idx_offset`; `gen_leaf` is a **Rust callback in the test binary** so both libraries call back into the same generator — this exercises the FFI function-pointer boundary | `cfg_treehash` | [x] |
| C18 | `SPX_compute_root` | `tree_height` ∈ {1,2,3,SPX_FORS_HEIGHT,SPX_TREE_HEIGHT} × `leaf_idx` odd/even/`0`/`(1<<h)-1` × random `idx_offset` × random leaf/auth-path; compares `root` and the mutated `addr` | `cfg_compute_root` | [x] |
| C19 | `SPX_fors_gen_leafx1` | random ctx × random `info.leaf_addrx` × random `addr_idx` (0, 1, `(1<<SPX_FORS_HEIGHT)-1`, random); compares `leaf` and the mutated `info` | `cfg_fors_gen_leafx1` | [x] |
| C20 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT` and also 1..3; `leaf_idx` 0/1/max/random; random `idx_offset`; compares `root`, the whole `auth_path` and the mutated `tree_addr`/`info` | `cfg_fors_treehashx1` | [x] |
| C21 | `SPX_wots_treehashx1` | `tree_height` ∈ {1,2,SPX_TREE_HEIGHT}; `leaf_idx` 0/1/max/`~0u`/random; both signing and non-signing `info`; compares `root`, `auth_path`, `tree_addr`, `info` and the produced WOTS signature | `cfg_wots_treehashx1` | [x] |
| C22 | `SPX_fors_sign` | random ctx × random `fors_addr` × random `SPX_FORS_MSG_BYTES` message + all-zero + all-`0xFF`; compares the full `SPX_FORS_BYTES` signature and the `pk` | `cfg_fors_sign` | [x] |
| C23 | `SPX_fors_pk_from_sig` | fed with the signature produced by `SPX_fors_sign` (round trip) **and** with a random signature (must still agree byte-for-byte) | `cfg_fors_pk_from_sig` | [x] |
| C24 | `SPX_merkle_sign` | random ctx × random `wots_addr`/`tree_addr` × `idx_leaf` ∈ {0,1,max,`~0u`,random} × random `root` input; compares the `SPX_WOTS_BYTES + SPX_TREE_HEIGHT*SPX_N` output, the returned `root` and both mutated address words | `cfg_merkle_sign` | [x] |
| C25 | `SPX_merkle_gen_root` | random ctx (all-zero seeds, all-`0xFF` seeds, random) | `cfg_merkle_gen_root` | [x] |
| C26 | `crypto_sign_secretkeybytes/publickeybytes/bytes/seedbytes` | no inputs — the four size constants for the active parameter set | `cfg_sizes` | [x] |
| C27 | `crypto_sign_seed_keypair` | seed = all-zero, all-`0xFF`, random (`3*SPX_N` bytes) | `cfg_seed_keypair` | [x] |
| C28 | `crypto_sign_keypair` | driven from a `randombytes_init`-seeded DRBG so it is deterministic; also verifies both libraries advance `DRBG_ctx` identically | `cfg_keypair_from_drbg` | [x] |
| C29 | `crypto_sign_signature` | `mlen` ∈ {0,1,31,32,33,63,64,65,135,136,137,1000} × random key from `crypto_sign_seed_keypair`; DRBG re-seeded identically before each call so `optrand` matches; compares the full `SPX_BYTES` signature and `*siglen` | `cfg_sign_signature` | [x] |
| C30 | `crypto_sign_verify` | the signature from C verified by Rust and vice-versa (**cross**-verification), for the same `mlen` set | `cfg_verify_cross` | [x] |
| C31 | `crypto_sign` + `crypto_sign_open` | full round trip, `mlen` ∈ {0,1,32,64,136,1000}; compares `sm`, `*smlen`, the recovered `m` and `*mlen`; also C-signed/Rust-opened and Rust-signed/C-opened | `cfg_sign_open_roundtrip` | [x] |
| C32 | `randombytes_init` + `randombytes` | entropy all-zero / all-`0xFF` / random × `personalization_string` NULL / random × draw sizes {0,1,15,16,17,48,64,100} in sequence; compares every draw **and** the resulting `DRBG_ctx` image | `cfg_drbg_stream` | [x] |
| C33 | `AES256_CTR_DRBG_Update` | random `Key`/`V`/`provided_data`, plus `provided_data = NULL`, plus `V` = all-`0xFF` / all-zero | `cfg_drbg_update` | [x] |
| C34 | `AES256_ECB` | random 32-byte key × random 16-byte block, plus the NIST all-zero and all-`0xFF` vectors | `cfg_aes256_ecb` | [x] |
| C35 | `seedexpander_init` + `seedexpander` | random seed/diversifier × `maxlen` ∈ {1,16,17,1000,0xFFFFFFFF} × a sequence of draws of sizes {0,1,15,16,17,33} that cross the 16-byte buffer boundary; compares every draw and the whole `AES_XOF_struct` after each call | `cfg_seedexpander_stream` | [x] |
| C36 | `blake256`, `blake512` (one-shot) | `inlen` ∈ {0,1,54,55,56,57,63,64,65,110,111,112,113,127,128,129,255,256,1000} × random data — covers both `final` padding branches of both digests | `cfg_blake_oneshot` | [x] |
| C37 | `blake256_init/update/final`, `blake512_init/update/final` (incremental) | random split of a random message into 1..5 chunks (bit lengths that are multiples of 8), plus a `datalen == 0` update; compares the digest **and** the whole state struct after each step | `cfg_blake_incremental` | [x] |
| C38 | `blake256_compress`, `blake512_compress` | random state (`h`,`s`,`t`,`buflen`,`nullt`) × random 64/128-byte block; compares the whole state afterwards | `cfg_blake_compress` | [x] |
| C39 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `inlen` ∈ {1,16,32,48,64,100} × `outlen` ∈ {0,1,31,32,33,63,64,65,127,128,129,1000} | `cfg_blake_mgf1` | [x] |
| C40 | `cst` (exported `.rodata`) | reads the 16 `u64` values from both `.so`s and compares | `cfg_blake_cst` | [x] |
| C41 | `sha256`, `sha512` (one-shot) | `inlen` ∈ {0,1,55,56,57,63,64,65,111,112,113,127,128,129,255,1000} × random data | `cfg_sha_oneshot` | [x] |
| C42 | `sha256_inc_init/blocks/finalize`, `sha512_*` | random number of `inc_blocks` calls (0..4 blocks) followed by `inc_finalize` with `inlen` ∈ {0,1,55,56,57,111,112,113,random}; compares the digest **and** the 40/72-byte state after each call | `cfg_sha_incremental` | [x] |
| C43 | `SPX_mgf1_256`, `SPX_mgf1_512` | `inlen` × `outlen` grid as C39 | `cfg_sha_mgf1` | [x] |
| C44 | `SPX_seed_state` | random `pub_seed`; compares the full `spx_ctx` image (`state_seeded`, and `state_seeded_512` when `SPX_N>=24`) | `cfg_sha_seed_state` | [x] |
| C45 | `shake256` (one-shot) | `inlen` ∈ {0,1,135,136,137,271,272,273,1000} × `outlen` ∈ {1,32,135,136,137,272,1000} | `cfg_shake_oneshot` | [x] |
| C46 | `shake256_absorb` + `shake256_squeezeblocks` | `inlen` on/around the 136-byte rate × `nblocks` ∈ {0,1,2,3}; compares the squeezed output and the 25-`u64` state | `cfg_shake_absorb_squeeze` | [x] |
| C47 | `shake256_inc_init/absorb/finalize/squeeze` | 1..5 random absorb chunks (incl. 0-length) crossing the rate boundary, then squeezes of sizes {0,1,32,136,137,300}; compares every output and the 26-`u64` incremental state | `cfg_shake_incremental` | [x] |
| C48 | `SPX_tweak_constants` | random `pub_seed`/`sk_seed`; compares the whole `spx_ctx` (both tweaked constant tables) | `cfg_haraka_tweak_constants` | [x] |
| C49 | `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm` | tweaked ctx × random 32/64-byte inputs, plus all-zero and all-`0xFF` | `cfg_haraka_perm` | [x] |
| C50 | `SPX_haraka_S` (one-shot sponge) | `inlen` ∈ {0,1,31,32,33,63,64,65,100} × `outlen` ∈ {1,16,31,32,33,64,100} | `cfg_haraka_s` | [x] |
| C51 | `SPX_haraka_S_inc_init/absorb/finalize/squeeze` | 1..5 absorb chunks crossing the 32-byte rate, then squeezes of {0,1,16,32,33,100}; compares the 65-byte incremental state after each call | `cfg_haraka_s_incremental` | [x] |
| C52 | `spx_ctx` layout | asserts that both libraries agree on the observable field offsets by seeding the ctx through `SPX_initialize_hash_function` on a shared over-sized buffer and then using it for `SPX_prf_addr`/`SPX_thash` | `cfg_ctx_layout` | [x] |
| C53 | `SPX_D == 1` branch in `hash_message` | not reachable for any of the 48 shipped parameter sets (`SPX_D` ∈ {7,8,17,22}); both C and Rust compile the same `if`, so the branch is dead in both | *(documented, unreachable)* | [x] |

All rows are checked off only after passing across the randomized inputs for all
48 `(backend, thash, secpar)` combinations — see `run_all.sh`.

## Verification result

```
$ ./run_all.sh
PASS haraka:robust:128s (102 tests)
...  (48 lines, one per (backend, thash, secpar) combination)
PASS blake:simple:256f  (102 tests)
================================
run_all: pass=48 fail=0
```

Every row above is checked off only because it passed, across its randomized
inputs, in **all 48** combinations. The end-to-end `driver` KAT transcript digest
also matches the C reference for all 48 (`./kat_all.sh`, `kat_all: pass=48 fail=0`).

### Rows whose absolute expectation had to be relaxed (and why)

`cfg_gen_message_random` / `cfg_hash_message` initially used an `SPX_N`-sized
output buffer for `R`. The BLAKE backend's C writes the **full** 32-/64-byte
BLAKE digest into `R` (`blakeX_final(&S, R)`), which corrupted the test's heap.
The buffers are now over-sized *and fully compared*, so the over-write itself is
part of what is verified. See `ERRORS.md`.

`cfg_treehash` initially sized `auth_path` at `tree_height * SPX_N`. The C
`treehash` can write `auth_path + tree_height * SPX_N` (the final
`heights[offset-1]++` can reach `tree_height`), so the buffer now holds
`tree_height + 2` nodes plus a 64-byte sentinel guard that is compared too.
