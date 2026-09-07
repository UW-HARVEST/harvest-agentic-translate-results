# CONFIGS.md — configuration-surface table

## Axes the C actually branches on

**Build-time (CMake cache variables → Cargo features).** Cross-product is
4 × 2 × 6 = **48** configurations, each of which is a *different library*:

| axis | values | what it toggles in C |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake` (a.k.a. `shake256`), `blake` | `lib/CMakeLists.txt` `add_subdirectory(${HASH_BACKEND})`; picks `hash_<b>.c` + `thash_<b>_*.c`; **also changes the address-field byte offsets** (`<b>_offsets.h`: sha2 uses LAYER=0/TREE=1/TYPE=9/KP=10/CHAIN=17/HASH=21/TREE_HGT=17/TREE_IDX=18, all others LAYER=3/TREE=8/TYPE=19/KP=20/CHAIN=27/HASH=31/TREE_HGT=27/TREE_IDX=28) |
| `THASH` | `robust`, `simple` | `thash()` either MGF1/haraka_S-masks the input first (robust) or hashes directly (simple) |
| `SECPAR` | `128s`,`128f`,`192s`,`192f`,`256s`,`256f` | `SPX_N`, `SPX_FULL_HEIGHT`, `SPX_D`, `SPX_TREE_HEIGHT`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`, `SPX_WOTS_LEN2`, and the `SPX_N >= 24` / `SPX_SHA512` / `SPX_BLAKE512` switch to the 512-bit primitive |
| randombytes provider | `rng.c` (DRBG, default) / `randombytes.c` (`/dev/urandom`, Cargo feature `urandom`) | `sphincs_core_det` vs `sphincs_core` |

Parameter values extracted from `c_src/app/params/params-sphincs-*.h`
(identical across backends):

| SECPAR | N | FULL_HEIGHT | D | TREE_HEIGHT | FORS_HEIGHT | FORS_TREES | WOTS_LEN | 512-primitive? |
|---|---|---|---|---|---|---|---|---|
| 128s | 16 | 63 | 7  | 9 | 12 | 14 | 35 | no  (`N < 24`) |
| 128f | 16 | 66 | 22 | 3 | 6  | 33 | 35 | no  (`N < 24`) |
| 192s | 24 | 63 | 7  | 9 | 14 | 17 | 51 | **yes** |
| 192f | 24 | 66 | 22 | 3 | 8  | 33 | 51 | **yes** |
| 256s | 32 | 64 | 8  | 8 | 14 | 22 | 67 | **yes** |
| 256f | 32 | 68 | 17 | 4 | 9  | 35 | 67 | **yes** |

`SPX_WOTS_W == 16` (`LOGW = 4`) in every params file, so the `W == 256` /
`LOGW = 8` branch of `base_w`/`SPX_WOTS_LEN2` is dead in this tree.
`SPX_D >= 7` everywhere, so the `if (SPX_D == 1) *tree = 0;` branch in every
`hash_message()` is likewise dead — but the Rust must still contain it.

**Run-time axes (from the `if`/`switch` the C takes on its arguments):**

| axis | values the C distinguishes |
|---|---|
| `thash` `inblocks` | `1` (F-function path: haraka uses `haraka512`/`haraka256`; blake & sha2 stay on the 256-bit primitive) vs `> 1` (haraka switches to `haraka_S`; blake/sha2 switch to `blake512`/`sha512` **iff** `SPX_BLAKE512`/`SPX_SHA512`).  Real call sites use `1`, `2`, `SPX_WOTS_LEN`, `SPX_FORS_TREES`. |
| `mlen` in `gen_message_random` (sha2) | `SPX_N + mlen < SPX_SHAX_BLOCK_BYTES` vs `>=` (`hash_sha2.c:96`) |
| `mlen` in `hash_message` (sha2) | `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*SPX_SHAX_BLOCK_BYTES` vs `>=` (`hash_sha2.c:159`) |
| `mlen` generally | `0`, `1`, one below / at / one above each block boundary (64/128 for sha2, 136 for shake256, 64/128 for blake, 32 rate for haraka_S), and multi-block |
| `leaf_idx & 1` in `compute_root` | even (leaf is left child) vs odd (right child) — different `memcpy` order (`utils.c:57`) |
| `leaf_idx` in `wots_gen_leafx1` | `leaf_idx == info->wots_sign_leaf` (emit WOTS signature, `wots_k_mask = 0`) vs `!=` (`wots_k_mask = ~0`, no signature).  `merkle_gen_root` passes `~0u` so the signing path never fires. |
| `idx < max_idx` in `wots_treehashx1` / `fors_treehashx1` | the last-leaf exception that keeps climbing the stack (`utilsx1.c:73`) |
| `tree_height` in `treehash` / `compute_root` | `1` (degenerate: `compute_root`'s `for i < tree_height-1` loop body never runs) vs `2..SPX_FORS_HEIGHT`/`SPX_TREE_HEIGHT` |
| `idx_offset` in `treehash` / `compute_root` / `*_treehashx1` | `0` vs non-zero (FORS uses `i * (1 << FORS_HEIGHT)`) |
| `outlen`/`inlen` in `ull_to_bytes` / `bytes_to_ull` | `0`, `1..8`, `> 8` |
| `addr` type field | all 7 `SPX_ADDR_TYPE_*` values (0–6), which select different address layouts consumed by `prf_addr`/`thash` |
| `personalization_string` / `provided_data` in `rng.c` | `NULL` vs non-`NULL` |
| `outlen`/`inlen` in the MGF1s (`blake256_mgf1`, `blake512_mgf1`, `mgf1_256`, `mgf1_512`) | `outlen` below / at / above one hash block; `inlen` arbitrary |
| incremental-hash entry points (`*_inc_absorb` etc.) | absorbing in one call vs many small calls straddling the rate boundary; `outlen` spanning multiple squeeze blocks |

## Rows

One row per combination the C treats differently.  Every row is exercised with
many randomized inputs (fixed seed `0x5EED_1234`), both `.so`s loaded via
`libloading`, outputs compared byte-for-byte.  `[x]` = passing.

### A. Lowest level — `utils` / `address` (backend-independent code, but the address
### offsets differ per backend, so these run under every backend)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| A1 | `SPX_ull_to_bytes` | `outlen ∈ {0,1,2,3,4,5,6,7,8,9,16}` × random `u64` `in` | [x] |
| A2 | `SPX_u32_to_bytes` | random `u32` including `0`, `0xFFFFFFFF` | [x] |
| A3 | `SPX_bytes_to_ull` | `inlen ∈ {0,1,…,8}` × random bytes | [x] |
| A4 | `SPX_bytes_to_ull` | `inlen ∈ {9,16}` (past the 64-bit width) × random bytes | [x] |
| A5 | `SPX_set_layer_addr` | random `addr[8]` × `layer ∈ {0,1,255,256,0xFFFFFFFF, random}` (byte truncation) | [x] |
| A6 | `SPX_set_tree_addr` | random `addr[8]` × `tree ∈ {0,1,2^32,2^63,u64::MAX, random}` (8-byte BE field) | [x] |
| A7 | `SPX_set_type` | random `addr[8]` × `type ∈ {0..6}` (all valid `SPX_ADDR_TYPE_*`) | [x] |
| A8 | `SPX_set_type` | random `addr[8]` × `type ∈ {7,8,255,256,1000,0xFFFFFFFF}` (out-of-range enum ints) | [x] |
| A9 | `SPX_set_keypair_addr` | random `addr[8]` × random `u32` keypair | [x] |
| A10 | `SPX_set_chain_addr` | random `addr[8]` × `chain ∈ {0,255,256,random}` | [x] |
| A11 | `SPX_set_hash_addr` | random `addr[8]` × `hash ∈ {0,255,256,random}` | [x] |
| A12 | `SPX_set_tree_height` | random `addr[8]` × `{0,255,256,random}` | [x] |
| A13 | `SPX_set_tree_index` | random `addr[8]` × random `u32` | [x] |
| A14 | `SPX_copy_subtree_addr` | random `in[8]`, random pre-filled `out[8]` (copies `SPX_OFFSET_TREE+8` bytes — 16 for sha2, 32 for others) | [x] |
| A15 | `SPX_copy_keypair_addr` | random `in[8]`, random pre-filled `out[8]` (copies subtree **plus** 4 bytes at `SPX_OFFSET_KP_ADDR`) | [x] |

### B. Backend primitives (one-shot and incremental entry points)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B1 | blake: `blake256` | `inlen ∈ {0,1,55,56,57,63,64,65,127,128,129,1000}` × random input | [x] |
| B2 | blake: `blake256_init`/`_update`/`_final` | random split of the input into 1–5 `update` calls straddling the 64-byte block | [x] |
| B3 | blake: `blake256_compress` | random 64-byte block against a random-but-valid `blakestate256` | [x] |
| B4 | blake: `blake512` | `inlen ∈ {0,1,111,112,113,127,128,129,255,256,257,1000}` | [x] |
| B5 | blake: `blake512_init`/`_update`/`_final` | random split into 1–5 `update` calls straddling the 128-byte block | [x] |
| B6 | blake: `blake512_compress` | random 128-byte block against a random-but-valid `blakestate512` | [x] |
| B7 | blake: `SPX_blake256_mgf1` | `outlen ∈ {1,31,32,33,64,100}` × `inlen ∈ {0,1,32,64,100}` | [x] |
| B8 | blake: `SPX_blake512_mgf1` | `outlen ∈ {1,63,64,65,128,200}` × `inlen ∈ {0,1,64,128,200}` | [x] |
| B9 | blake: exported data `cst` | 128 bytes at the symbol address must equal the C `.so`'s | [x] |
| B10 | sha2: `sha256` | `inlen ∈ {0,1,55,56,57,63,64,65,119,120,128,1000}` | [x] |
| B11 | sha2: `sha256_inc_init`/`_inc_blocks`/`_inc_finalize` | `inblocks ∈ {0,1,2,5}` then `inlen ∈ {0,1,55,56,63,64,120}` tail | [x] |
| B12 | sha2: `sha512` | `inlen ∈ {0,1,111,112,113,127,128,129,255,1000}` | [x] |
| B13 | sha2: `sha512_inc_init`/`_inc_blocks`/`_inc_finalize` | `inblocks ∈ {0,1,2,5}` then `inlen ∈ {0,1,111,112,127,128,240}` tail | [x] |
| B14 | sha2: `SPX_mgf1_256` | `outlen ∈ {1,31,32,33,64,100}` × `inlen ∈ {0,1,32,64,100}` | [x] |
| B15 | sha2: `SPX_mgf1_512` | `outlen ∈ {1,63,64,65,128,200}` × `inlen ∈ {0,1,64,128,200}` | [x] |
| B16 | sha2: `SPX_seed_state` | random `spx_ctx.pub_seed`; compare the resulting `state_seeded` (40 B) and, for `N >= 24`, `state_seeded_512` (72 B) | [x] |
| B17 | shake: `shake256` | `outlen ∈ {1,32,135,136,137,272,500}` × `inlen ∈ {0,1,135,136,137,272,1000}` | [x] |
| B18 | shake: `shake256_inc_init`/`_inc_absorb`/`_inc_finalize`/`_inc_squeeze` | absorb split into 1–5 chunks straddling the 136-byte rate; squeeze split into 1–4 chunks straddling the rate | [x] |
| B19 | shake: `shake256_absorb` + `shake256_squeezeblocks` | `inlen` a multiple and a non-multiple of 136; `nblocks ∈ {1,2,4}` | [x] |
| B20 | haraka: `SPX_tweak_constants` | random `pub_seed`/`sk_seed`; compare `tweaked512_rc64` (640 B) and `tweaked256_rc32` (320 B) | [x] |
| B21 | haraka: `SPX_haraka256` | random 32-byte input, tweaked ctx | [x] |
| B22 | haraka: `SPX_haraka512` | random 64-byte input, tweaked ctx | [x] |
| B23 | haraka: `SPX_haraka512_perm` | random 64-byte input, tweaked ctx | [x] |
| B24 | haraka: `SPX_haraka_S` | `outlen ∈ {1,16,31,32,33,64,100}` × `inlen ∈ {0,1,31,32,33,64,200}` | [x] |
| B25 | haraka: `SPX_haraka_S_inc_*` | absorb split into 1–5 chunks straddling the 32-byte rate; squeeze split into 1–4 chunks | [x] |

### C. Backend hooks (`hash.h` / `thash.h`) — the level `sign.c` calls

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `SPX_initialize_hash_function` | random `pub_seed`+`sk_seed`; compare the **whole** `spx_ctx` afterwards (no-op for blake/shake, `seed_state` for sha2, `tweak_constants` for haraka) | [x] |
| C2 | `SPX_prf_addr` | initialized ctx × random `addr[8]` × each `type ∈ {0..6}` | [x] |
| C3 | `SPX_prf_addr` | initialized ctx × `addr[8]` all-zero and all-`0xFF` | [x] |
| C4 | `SPX_thash` | `inblocks = 1` (F path) × random `in` × random `addr` | [x] |
| C5 | `SPX_thash` | `inblocks = 2` (H path; switches to the 512-bit primitive when `N >= 24`) | [x] |
| C6 | `SPX_thash` | `inblocks = SPX_WOTS_LEN` (WOTS-pk path, 35/51/67 blocks) | [x] |
| C7 | `SPX_thash` | `inblocks = SPX_FORS_TREES` (FORS-pk path, 14/17/22/33/35 blocks) | [x] |
| C8 | `SPX_thash` | `inblocks ∈ {3,4,8,16}` (intermediate widths, incl. crossing the primitive's block boundary) | [x] |
| C9 | `SPX_gen_message_random` | `mlen = 0` | [x] |
| C10 | `SPX_gen_message_random` | `mlen` just below the sha2 short/long split: `SPX_SHAX_BLOCK_BYTES - SPX_N - 1` | [x] |
| C11 | `SPX_gen_message_random` | `mlen` exactly at the split: `SPX_SHAX_BLOCK_BYTES - SPX_N` | [x] |
| C12 | `SPX_gen_message_random` | `mlen` just above the split, and much larger (`+1`, `1000`, `5000`) | [x] |
| C13 | `SPX_hash_message` | `mlen = 0`; check `digest`, `*tree`, `*leaf_idx` | [x] |
| C14 | `SPX_hash_message` | `mlen` at the sha2 `SPX_INBLOCKS` split `− 1`, `=`, `+ 1` | [x] |
| C15 | `SPX_hash_message` | `mlen ∈ {1, 135, 136, 137, 1000, 5000}` (shake rate / blake block boundaries) | [x] |
| C16 | `SPX_hash_message` | random `R`, random `pk` (both halves), verifying the `tree` mask `>> (64 - TREE_BITS)` and `leaf_idx` mask `>> (32 - TREE_HEIGHT)` over many samples | [x] |

### D. WOTS / FORS / Merkle (composed pipeline, low-level entry points)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| D1 | `SPX_chain_lengths` | random `msg[SPX_N]`, many samples (drives `base_w` + `wots_checksum`) | [x] |
| D2 | `SPX_chain_lengths` | `msg` all-zero (max checksum) and all-`0xFF` (zero checksum) | [x] |
| D3 | `SPX_wots_pk_from_sig` | random `sig[WOTS_BYTES]` × random `msg[N]` × random `addr`; `addr` mutated in place, so compare **both** `pk` and the final `addr` | [x] |
| D4 | `SPX_wots_gen_leafx1` | `leaf_info_x1.wots_sign_leaf == leaf_idx` (signature-emitting path, `wots_k_mask = 0`); compare `dest`, the `wots_sig` buffer, and both `leaf_addr`/`pk_addr` | [x] |
| D5 | `SPX_wots_gen_leafx1` | `wots_sign_leaf != leaf_idx` (`wots_k_mask = ~0`, no signature written) | [x] |
| D6 | `SPX_wots_gen_leafx1` | `wots_sign_leaf = ~0u` (the `merkle_gen_root` value) with `leaf_idx = ~0u`, i.e. the signing path fires on the last index | [x] |
| D7 | `SPX_wots_gen_leafx1` | `wots_steps` = random values in `0..SPX_WOTS_W`, plus the extremes `0` and `W-1` | [x] |
| D8 | `SPX_compute_root` | `leaf_idx` even and odd × `tree_height ∈ {1,2,3,SPX_TREE_HEIGHT,SPX_FORS_HEIGHT}` × `idx_offset ∈ {0, random}`; compare `root` and mutated `addr` | [x] |
| D9 | `SPX_treehash` | `tree_height ∈ {1,2,3}` (function-pointer `gen_leaf` supplied from the test) × `leaf_idx ∈ {0, 1, 2^h−1, random}` × `idx_offset ∈ {0, random}`; compare `root` **and** the full `auth_path` | [x] |
| D10 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT` × `leaf_idx ∈ {0, 1, max, random}` × `idx_offset ∈ {0, random}`; exercises the `idx < max_idx` last-leaf exception | [x] |
| D11 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT` × `leaf_idx ∈ {0,1,max,random}` × `idx_offset ∈ {0, i*(1<<FORS_HEIGHT)}` | [x] |
| D12 | `SPX_fors_gen_leafx1` | random `addr_idx` (incl. `0`, `0xFFFFFFFF`) × random `fors_gen_leaf_info.leaf_addrx`; compare `leaf` and the mutated `leaf_addrx` | [x] |
| D13 | `SPX_fors_sign` | random `m[SPX_FORS_MSG_BYTES]` × random `fors_addr`; compare the whole `sig[SPX_FORS_BYTES]` and `pk[SPX_N]` | [x] |
| D14 | `SPX_fors_sign` | `m` all-zero (all indices 0) and all-`0xFF` (all indices maximal) | [x] |
| D15 | `SPX_fors_pk_from_sig` | the `sig` produced by `fors_sign` (round-trip: `pk` must match) | [x] |
| D16 | `SPX_fors_pk_from_sig` | random (invalid) `sig` × random `m` — still a defined computation, outputs must match | [x] |
| D17 | `SPX_merkle_sign` | random `root[N]`, random `wots_addr`/`tree_addr`, `idx_leaf ∈ {0,1,(1<<TREE_HEIGHT)-1, random}`; compare `sig[WOTS_BYTES + TREE_HEIGHT*N]`, `root`, and both mutated addresses | [x] |
| D18 | `SPX_merkle_sign` | `idx_leaf = ~0u` (the `merkle_gen_root` "no auth path" value) | [x] |
| D19 | `SPX_merkle_gen_root` | random `pub_seed`/`sk_seed` ctx | [x] |

### E. Public API (`api.h`) — full end-to-end

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| E1 | `crypto_sign_secretkeybytes`, `_publickeybytes`, `_bytes`, `_seedbytes` | no input | [x] |
| E2 | `crypto_sign_seed_keypair` | random 3·`SPX_N` seed, many samples; compare `pk` and `sk` | [x] |
| E3 | `crypto_sign_seed_keypair` | seed all-zero and all-`0xFF` | [x] |
| E4 | `crypto_sign_keypair` | DRBG seeded identically via `randombytes_init` in both `.so`s → deterministic `pk`/`sk` must match | [x] |
| E5 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen = 0`; DRBG-seeded so `optrand` matches; compare `sig`, `siglen`, and the verify return | [x] |
| E6 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen ∈ {1, 32, 33, 63, 64, 65, 111, 112, 127, 128, 129, 135, 136, 137}` (every backend block/rate boundary) | [x] |
| E7 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen ∈ {1000, 5000}` (multi-block) | [x] |
| E8 | `crypto_sign` + `crypto_sign_open` | round trip, `mlen ∈ {0,1,33,64,136,1000}`; compare `sm`, `smlen`, recovered `m`, `mlen` | [x] |
| E9 | `crypto_sign_verify` | cross-check: signature from the C `.so` verified by the Rust `.so` and vice versa | [x] |
| E10 | `randombytes` (DRBG) | `randombytes_init(entropy, NULL)` then a sequence of `xlen ∈ {0,1,15,16,17,48,63,64,1000}` draws; compare every draw **and** the final `DRBG_ctx` bytes | [x] |
| E11 | `randombytes_init` | non-`NULL` 48-byte personalization string | [x] |
| E12 | `AES256_ECB` | random 32-byte key × random 16-byte counter | [x] |
| E13 | `AES256_CTR_DRBG_Update` | `provided_data = NULL` and non-`NULL`, random `Key`/`V` | [x] |
| E14 | `seedexpander_init` + `seedexpander` | `maxlen ∈ {1, 16, 17, 1024, 0xFFFFFFFF}` × draw sequences of `xlen ∈ {0,1,15,16,17,100}`; compare output and the whole `AES_XOF_struct` | [x] |
| E15 | `driver` binary | C `driver` vs Rust `driver` stdout, all 48 configurations | [x] |

### F. Build-time cross-product

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| F1 | all of A–E | every `(HASH_BACKEND, THASH, SECPAR)` triple — 48 configurations | [x] |
| F2 | `randombytes` | `urandom` feature selects `randombytes.c` semantics (`void` return, `/dev/urandom`); symbol presence + non-blocking behaviour only, output is non-deterministic by construction | [x] |

## Where each row is tested

| rows | file |
|---|---|
| A1–A15 | `translation/tests/diff_a_utils.rs` |
| B1–B9 | `translation/tests/diff_b_blake.rs` (`#[cfg(backend_blake)]`) |
| B10–B16 | `translation/tests/diff_b_sha2.rs` (`#[cfg(backend_sha2)]`) |
| B17–B19 | `translation/tests/diff_b_shake.rs` (`#[cfg(backend_shake)]`) |
| B20–B25 | `translation/tests/diff_b_haraka.rs` (`#[cfg(backend_haraka)]`) |
| C1–C16 | `translation/tests/diff_c_hooks.rs` |
| D1–D19 | `translation/tests/diff_d_wots_fors_merkle.rs` |
| E1–E14 | `translation/tests/diff_e_api.rs` |
| E15 | `./driversweep.sh` |
| F1 | `./run_backend_tests.sh <backend>` (drives all of the above per config) |
| F2 | `translation/tests/diff_f_urandom.rs` |

Every test loads **both** shared objects with `libloading` and calls only
exported symbols; nothing is called as a normal Rust function.
`translation/tests/common/mod.rs` holds the loader, the params table (re-derived
independently from the C headers, not imported from the crate), the
splitmix64 PRNG with fixed per-test seeds, and the `spx_ctx` /
`leaf_info_x1` / `AES_XOF_struct` layouts.

## Results

| check | command | result |
|---|---|---|
| `cargo check` on every combination | `translation/check_all.sh` | 120/120 (5 backend spellings × 2 THASH × 6 SECPAR × {default, urandom}) |
| exported-symbol parity | `./symsweep.sh` | 48/48 configs, symbol diff empty both ways |
| `driver` stdout byte-for-byte | `./driversweep.sh` | 48/48 configs identical |
| Phase B + C differential tests | `./run_backend_tests.sh {blake,haraka,sha2,shake}` | 96/96 configs, 6360 test cases, 0 failures |

## Notes on rows that turned out to be dead or degenerate in this tree

* `SPX_D >= 7` in every params header, so the `if (SPX_D == 1) *tree = 0;`
  branch in all four `hash_message()`s is unreachable.  Present in the Rust too.
* `SPX_WOTS_W == 16` everywhere, so the `W == 256` / `LOGW = 8` arms of
  `base_w()` and the `SPX_WOTS_LEN2` precomputation are unreachable.
* `SPX_N ∈ {16,24,32}`, all inside `(8, 136]`, so `SPX_WOTS_LEN2 == 3` always.
* Under the blake backend `hash_message` / `gen_message_random` pass byte counts
  to `blake*_update`, which wants **bit** counts, so only `mlen / 8` message
  bytes are absorbed.  See the note at the end of `ERRORS.md`; the behaviour is
  reproduced, not corrected, and row C13–C16 / E5–E9 assert C/Rust equality of
  the resulting digests rather than assuming full-message coverage.
