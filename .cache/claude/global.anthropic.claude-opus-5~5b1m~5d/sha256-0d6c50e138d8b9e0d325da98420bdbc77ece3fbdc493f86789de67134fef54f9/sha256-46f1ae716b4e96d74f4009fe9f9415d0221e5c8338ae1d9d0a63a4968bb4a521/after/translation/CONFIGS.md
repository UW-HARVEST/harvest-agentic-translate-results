# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Rows are the cross-product of the
axes the C code actually branches on, pruned to combinations the C treats
differently.

## Axes derived from the C source

### A1. `HASH_BACKEND` (CMake cache var → `lib/CMakeLists.txt: add_subdirectory(${HASH_BACKEND})`)
`haraka` | `sha2` | `shake` | `blake`. Selects `hash_<b>.c` + `thash_<b>_<t>.c`
**and** a different `<b>_offsets.h`, which changes the address layout:

| backend | LAYER | TREE | TYPE | KP_ADDR | CHAIN/TREE_HGT | HASH_ADDR/TREE_INDEX | addr bytes hashed |
|---------|-------|------|------|---------|----------------|----------------------|-------------------|
| haraka | 3 | 8 | 19 | 20 | 27 | 31 / 28 | 32 |
| shake  | 3 | 8 | 19 | 20 | 27 | 31 / 28 | 32 |
| blake  | 3 | 8 | 19 | 20 | 27 | 31 / 28 | 32 |
| sha2   | **0** | **1** | **9** | **10** | **17** | **21 / 18** | **22** (`SPX_SHA256_ADDR_BYTES`) |

`sha2` is the odd one out — it uses a *compressed* 22-byte address. Every
`address.c` setter therefore writes to different offsets.

### A2. `THASH` (`thash_<b>_robust.c` vs `thash_<b>_simple.c`)
`robust` XORs the input with an MGF1 bitmask keyed on `pub_seed‖addr` before
hashing; `simple` hashes `pub_seed‖addr‖in` directly. Completely different
digests. Also changes what `initialize_hash_function` must precompute.

### A3. `SECPAR` (`params-sphincs-<b>-<s>.h`)

| secpar | N | FULL_HEIGHT | D | TREE_HEIGHT | FORS_HEIGHT | FORS_TREES | SHA512/BLAKE512 | SPX_BYTES (blake) |
|--------|---|-------------|---|-------------|-------------|------------|-----------------|--------|
| 128s | 16 | 63 | 7  | 9 | 12 | 14 | 0 | 7856 |
| 128f | 16 | 66 | 22 | 3 | 6  | 33 | 0 | 17088 |
| 192s | 24 | 63 | 7  | 9 | 14 | 17 | **1** | 16224 |
| 192f | 24 | 66 | 22 | 3 | 8  | 33 | **1** | 35664 |
| 256s | 32 | 64 | 8  | 8 | 14 | 22 | **1** | 29792 |
| 256f | 32 | 68 | 17 | 4 | 9  | 35 | **1** | 49856 |

Distinct code paths keyed on these:
* `SPX_N >= 24` in `hash_<b>.c` and in the driver ⇒ 512-bit primitive for
  `gen_message_random` / `hash_message` / the KAT transcript.
* `SPX_SHA512` / `SPX_BLAKE512` (`1` for 192/256) ⇒ `thash` takes a
  **second, 512-bit branch** when `inblocks > 1`. For 128-bit params that branch
  does not exist at all.
* `SPX_D == 1` special case in `hash_message` (`*tree = 0`) — **not reachable**
  in any of the 6 shipped param sets (min `D` is 7), documented for completeness.
* `SPX_WOTS_LEN` = 35 (N=16), 51 (N=24), 67 (N=32) ⇒ different `thash` inblocks
  count for the WOTS-pk compression, and different `base_w` / checksum widths
  (`SPX_WOTS_LEN2` = 3 for N=16/24/32 since all are ≤136).

⇒ **48 build configurations** = 4 × 2 × 6, all of which build in C and compile in
Rust.

### A4. Runtime options / modes the public API can set
There are **no** runtime flags — SPHINCS+ is fully compile-time configured. The
runtime "modes" are instead:
* the 8-word `addr[8]` **address type** (`set_type`, 7 valid values
  `SPX_ADDR_TYPE_WOTS..FORSPRF`) and every other address field
  (layer, tree, keypair, chain, hash, tree_height, tree_index);
* `thash`'s `inblocks` argument (1, 2, `SPX_WOTS_LEN`, `SPX_FORS_TREES`);
* `leaf_info_x1.wots_sign_leaf` = `~0u` (pk-only) vs. a real leaf index
  (sign mode) in `wots_gen_leafx1` / `wots_treehashx1`;
* `randombytes_init`'s `personalization_string` = `NULL` vs. 48 bytes;
* the DRBG global state (`DRBG_ctx`) — output depends on call history.

### A5. Input shapes the code special-cases
* `mlen`: 0, 1, 32, 33 (driver base), 55/56 and 111/112 (BLAKE padding
  boundaries), 63/64/65 and 127/128/129 (SHA/BLAKE block boundaries), 135/136
  (SHAKE256 rate), 200 (multi-block), 231 (driver max).
* `siglen` / `smlen`: exactly `SPX_BYTES`, `SPX_BYTES + mlen`.
* `thash inblocks`: 0, 1, 2, `SPX_WOTS_LEN`, `SPX_FORS_TREES`.
* `tree_height`: 1, 2, `SPX_FORS_HEIGHT`, `SPX_TREE_HEIGHT`.
* `leaf_idx`: 0, 1, `2^h - 1`, `~0u`; `idx_offset`: 0 and `i * 2^FORS_HEIGHT`.
* `outlen` for MGF1 / squeeze: 0, 1, block-1, block, block+1, many blocks.
* `maxlen` for `seedexpander_init`: 0, 1, `0xFFFFFFFF`.

## Public entry points (the FULL set, lowest level first)

Layer 0 (pure byte utilities): `SPX_ull_to_bytes`, `SPX_u32_to_bytes`,
`SPX_bytes_to_ull`.
Layer 0 (address setters): `SPX_set_layer_addr`, `SPX_set_tree_addr`,
`SPX_set_type`, `SPX_copy_subtree_addr`, `SPX_set_keypair_addr`,
`SPX_copy_keypair_addr`, `SPX_set_chain_addr`, `SPX_set_hash_addr`,
`SPX_set_tree_height`, `SPX_set_tree_index`.
Layer 0 (raw hash primitives): per backend — `blake256*`, `blake512*`,
`sha256*`, `sha512*`, `shake256*`, `SPX_haraka*`, plus `cst`.
Layer 1 (keyed hash façade): `SPX_initialize_hash_function`, `SPX_prf_addr`,
`SPX_thash`, `SPX_gen_message_random`, `SPX_hash_message`,
`SPX_blake256_mgf1`/`SPX_blake512_mgf1`/`SPX_mgf1_256`/`SPX_mgf1_512`/
`SPX_seed_state`/`SPX_tweak_constants`.
Layer 2 (tree/OTS internals): `SPX_chain_lengths`, `SPX_wots_pk_from_sig`,
`SPX_wots_gen_leafx1`, `SPX_fors_gen_leafx1`, `SPX_compute_root`,
`SPX_treehash`, `SPX_wots_treehashx1`, `SPX_fors_treehashx1`.
Layer 3 (scheme internals): `SPX_fors_sign`, `SPX_fors_pk_from_sig`,
`SPX_merkle_sign`, `SPX_merkle_gen_root`.
Layer 4 (public API): `crypto_sign_{secretkey,publickey,,seed}bytes`,
`crypto_sign_seed_keypair`, `crypto_sign_keypair`, `crypto_sign_signature`,
`crypto_sign_verify`, `crypto_sign`, `crypto_sign_open`.
Layer 4 (RNG): `randombytes_init`, `randombytes`, `AES256_ECB`,
`AES256_CTR_DRBG_Update`, `seedexpander_init`, `seedexpander`, `DRBG_ctx`.
Binary: `driver` (`PQCgenKAT_sign.c`).

## Configuration-surface table

Every row is executed for **all 48 feature combinations** (the runner stages a
per-combo Rust cdylib + driver + test binaries and then runs the 48 combos in
parallel — see `.verify/build_all_rust.sh`, `.verify/run_combo.sh`,
`.verify/run_all.sh`). Rows that name a backend-specific symbol are
`#[cfg(spx_backend = ...)]`-gated and therefore run in the 12 combos where that
backend is active. All randomized rows use a fixed-seed `SplitMix64` PRNG
(`tests/common/mod.rs`) so failures are reproducible.

Both libraries are always reached through `dlopen`/`dlsym` (`libloading`, with
`RTLD_NOW | RTLD_LOCAL` so the identically-named exports cannot interpose on one
another) — the Rust side is **never** called directly, so the `#[no_mangle]`
export wrappers are themselves under test.

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|--------------------------------------------|-----|
| 1 | `SPX_ull_to_bytes` | `outlen` ∈ {0,1,2,4,8} × random `u64` incl. 0, 1, `u64::MAX` (256 randomized) | [x] |
| 2 | `SPX_ull_to_bytes` | `outlen` ∈ {9,16,32} (> 8, `in` exhausted, remaining bytes must be 0) | [x] |
| 3 | `SPX_u32_to_bytes` | random `u32` incl. 0, 1, `u32::MAX` (256 randomized) | [x] |
| 4 | `SPX_bytes_to_ull` | `inlen` ∈ {0,1,2,4,8} × random bytes (256 randomized) | [x] |
| 5 | `SPX_bytes_to_ull` | `inlen` ∈ {9,12,16} — shift count ≥ 64 (C UB, must still match) | [x] |
| 6 | `SPX_set_layer_addr`, `SPX_set_type`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height` | random pre-filled `addr[8]` × random `u32` value; verifies the backend-specific byte offset (differs for `sha2`) | [x] |
| 7 | `SPX_set_tree_addr` | random pre-filled `addr[8]` × random `u64` incl. 0, `u64::MAX` | [x] |
| 8 | `SPX_set_keypair_addr`, `SPX_set_tree_index` | random `addr[8]` × random `u32` (4-byte big-endian fields) | [x] |
| 9 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random `in[8]` into random pre-filled `out[8]` (checks exactly which bytes are preserved) | [x] |
| 10 | `SPX_set_*` composition | apply all 10 setters in the order `sign.c` uses them, on the same `addr[8]`, with random values (catches offset-aliasing between CHAIN_ADDR/TREE_HGT and HASH_ADDR/TREE_INDEX) | [x] |
| 11 | `blake256` / `blake512` (blake only) | `inlen` ∈ {0,1,55,56,57,63,64,65,111,112,113,127,128,129,200} × random input | [x] |
| 12 | `blake256_init`+`update`×k+`final`, `blake512_*` (blake only) | incremental API: random split of a random message into 1..5 chunks; must equal the one-shot | [x] |
| 13 | `blake256_compress` / `blake512_compress` (blake only) | random 64/128-byte block against a random-but-valid state struct | [x] |
| 14 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1` (blake only) | `outlen` ∈ {0,1,31,32,33,64,65,100} × `inlen` ∈ {0,1,16,32,48,64} random | [x] |
| 15 | `sha256` / `sha512` (sha2 only) | `inlen` ∈ {0,1,55,56,57,63,64,65,111,112,113,127,128,129,200} × random | [x] |
| 16 | `sha256_inc_init/blocks/finalize`, `sha512_*` (sha2 only) | `inblocks` ∈ {0,1,2,3} then `finalize` with `inlen` ∈ {0,1,55,56,63,64,111,112,127,128} | [x] |
| 17 | `SPX_mgf1_256`, `SPX_mgf1_512` (sha2 only) | `outlen` ∈ {0,1,31,32,33,64,65,100} × `inlen` ∈ {0,1,16,32,48,64} | [x] |
| 18 | `SPX_seed_state` (sha2 only) | random `pub_seed`; also checks the `state_seeded_512` half only exists for 192/256 | [x] |
| 19 | `shake256` (shake only) | `outlen` ∈ {0,1,135,136,137,272,300} × `inlen` ∈ {0,1,135,136,137,200} | [x] |
| 20 | `shake256_absorb` + `shake256_squeezeblocks` (shake only) | `nblocks` ∈ {0,1,2,3} × `inlen` ∈ {0,1,135,136,137,200} | [x] |
| 21 | `shake256_inc_init/absorb×k/finalize/squeeze` (shake only) | random split absorb (1..5 chunks) × repeated `squeeze` calls of random sizes (tests the partial-block carry in `s_inc[25]`) | [x] |
| 22 | `SPX_tweak_constants` (haraka only) | random `pub_seed`/`sk_seed`; compares all 10×8 `tweaked512_rc64` and `tweaked256_rc32` words | [x] |
| 23 | `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm` (haraka only) | random 32/64-byte inputs × ctx from a random tweak | [x] |
| 24 | `SPX_haraka_S` (haraka only) | `outlen` ∈ {0,1,31,32,33,64,100} × `inlen` ∈ {0,1,31,32,33,64,200} | [x] |
| 25 | `SPX_haraka_S_inc_init/absorb×k/finalize/squeeze` (haraka only) | random split absorb × repeated squeezes of random sizes | [x] |
| 26 | `SPX_initialize_hash_function` | random `pub_seed`+`sk_seed`; full `spx_ctx` byte comparison (size/layout differs per backend: haraka has the 400-byte tweak tables, sha2 has 40+72 bytes of seeded state) | [x] |
| 27 | `SPX_prf_addr` | random ctx × random `addr[8]` (64 randomized) | [x] |
| 28 | `SPX_thash` | `inblocks == 1` (the only path for 128-bit; the 256-bit path for 192/256) × random ctx, random `addr`, random input | [x] |
| 29 | `SPX_thash` | `inblocks == 2` — for 192/256 this selects the **512-bit** branch; for 128 the same 256-bit branch | [x] |
| 30 | `SPX_thash` | `inblocks == SPX_WOTS_LEN` (35/51/67) — the WOTS-pk compression shape | [x] |
| 31 | `SPX_thash` | `inblocks == SPX_FORS_TREES` (14/33/17/33/22/35) — the FORS-pk shape | [x] |
| 32 | `SPX_thash` | `inblocks == 0` (degenerate but legal) | [x] |
| 33 | `SPX_gen_message_random` | `mlen` ∈ {0,1,32,33,55,56,63,64,65,111,112,127,128,135,136,200,231} × random `sk_prf`/`optrand` | [x] |
| 34 | `SPX_hash_message` | same `mlen` set × random `R`/`pk`; compares `digest`, `*tree` **and** `*leaf_idx` (the masking `>> (64-SPX_TREE_BITS)` differs per secpar) | [x] |
| 35 | `SPX_chain_lengths` | random `msg[SPX_N]` + the extremes all-`0x00` / all-`0xFF` (checksum boundary) | [x] |
| 36 | `SPX_wots_pk_from_sig` | random ctx × random `sig[SPX_WOTS_BYTES]` × random `msg[SPX_N]` × random `addr`; also `msg` all-0 / all-0xFF (max/min chain steps) | [x] |
| 37 | `SPX_wots_gen_leafx1` | `leaf_info_x1.wots_sign_leaf == leaf_idx` (**sign** mode, `wots_sig` written) × random `wots_steps` | [x] |
| 38 | `SPX_wots_gen_leafx1` | `leaf_info_x1.wots_sign_leaf == ~0u` (**pk-only** mode, `wots_sig` untouched) | [x] |
| 39 | `SPX_fors_gen_leafx1` | random ctx × random `addr_idx` × random `fors_gen_leaf_info.leaf_addrx` | [x] |
| 40 | `SPX_compute_root` | `tree_height` ∈ {1,2,`SPX_FORS_HEIGHT`,`SPX_TREE_HEIGHT`} × `leaf_idx` ∈ {0,1,odd,even,`2^h−1`} × random `idx_offset` × random auth path | [x] |
| 41 | `SPX_treehash` (function-pointer `gen_leaf`) | `tree_height` ∈ {0,1,2,3} × `leaf_idx` ∈ {0,1,`2^h−1`} × `idx_offset` ∈ {0, random}; `gen_leaf` = `SPX_fors_gen_leafx1` (the only shape a real caller uses) | [x] |
| 42 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT` × `idx_leaf` ∈ {0,1,`2^h−1`,`~0u`} × `idx_offset` = 0; compares `root` **and** the full `auth_path` **and** the written `wots_sig` | [x] |
| 43 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT` × `leaf_idx` ∈ {0,1,`2^h−1`} × `idx_offset` ∈ {0, `i*2^FORS_HEIGHT`}; compares `root` + `auth_path` | [x] |
| 44 | `SPX_fors_sign` | random ctx × random `m[SPX_FORS_MSG_BYTES]` × random `fors_addr[8]`; compares the whole `sig[SPX_FORS_BYTES]` and `pk[SPX_N]`; incl. `m` all-0 / all-0xFF | [x] |
| 45 | `SPX_fors_pk_from_sig` | round-trip: the `sig` produced by row 44 must yield the same `pk`; plus random `sig` (garbage) must still agree byte-for-byte | [x] |
| 46 | `SPX_merkle_sign` | random ctx × random `wots_addr`/`tree_addr` × `idx_leaf` ∈ {0,1,`2^TREE_HEIGHT−1`, `~0u`}; compares `sig[SPX_WOTS_BYTES + TREE_HEIGHT*N]` and the in/out `root` | [x] |
| 47 | `SPX_merkle_gen_root` | random ctx (this is `SPX_D-1`-layer top tree, `idx_leaf = ~0u`) | [x] |
| 48 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no input — one call each, per config (validates the param table) | [x] |
| 49 | `crypto_sign_seed_keypair` | random 3·`SPX_N` seed (64 randomized); compares `pk` and `sk` | [x] |
| 50 | `crypto_sign_keypair` | `randombytes_init` with a fixed 48-byte entropy first, so the internal `randombytes(seed)` is deterministic; compares `pk`/`sk` and the resulting `DRBG_ctx` | [x] |
| 51 | `crypto_sign_signature` | deterministic DRBG (for `optrand`) × `mlen` ∈ {0,1,32,33,64,128,231} random messages; compares `sig` and `*siglen` | [x] |
| 52 | `crypto_sign_verify` | the row-51 signature (must return 0) with `siglen == SPX_BYTES` | [x] |
| 53 | `crypto_sign` / `crypto_sign_open` | full round trip, `mlen` ∈ {0,1,32,33,64,128,231}; compares `sm`, `*smlen`, recovered `m`, `*mlen`, and both return codes | [x] |
| 54 | `randombytes_init` + `randombytes` | `personalization_string = NULL`, then a sequence of `xlen` ∈ {0,1,15,16,17,32,48,64,1000} calls; compares every output **and** the final `DRBG_ctx` bytes (state must stay in lockstep) | [x] |
| 55 | `randombytes_init` | `personalization_string != NULL` (random 48 bytes) — the XOR branch; then compare `DRBG_ctx` and a squeeze | [x] |
| 56 | `AES256_ECB` | random 32-byte key × random 16-byte block (256 randomized) | [x] |
| 57 | `AES256_CTR_DRBG_Update` | `provided_data = NULL` and `= random 48 bytes`, × random Key/V incl. V all-`0xFF` (carry propagation) | [x] |
| 58 | `seedexpander_init` + `seedexpander` | `maxlen` ∈ {1,16,17,256,65536,0xFFFFFFFF} × a sequence of `xlen` calls ∈ {0,1,15,16,17,31,32,100}; compares outputs and the full 80-byte `AES_XOF_struct` after each call | [x] |
| 59 | `seedexpander` | `xlen` large enough to force the `ctr[12..16]` counter to roll over (`ctr` pre-set to `..FF FF FF FF`) | [x] |
| 60 | `driver` binary | C `driver` vs Rust `driver`, stdout byte-for-byte + exit code, for each of the 48 configs | [x] |


## Result

```
$ bash .verify/run_all.sh
pass: 48   fail: 0
```

Per-combination test counts: 86 (blake), 84 (sha2), 84 (haraka), 83 (shake)
— the difference is the number of backend-gated tests (blake additionally has
`blake512_cst_data_symbol` and separate 256/512 incremental tests; shake and
haraka expose no `mgf1`).

The 48 KAT driver digests are **48 distinct values**, confirming that each
configuration really does exercise a different code path (they are not
accidentally identical), and that C and Rust agree on every one:

| backend | thash | 128s / 128f / 192s / 192f / 256s / 256f |
|---------|-------|------------------------------------------|
| all 4   | both  | see `.verify/logs/<b>_<t>_<s>.log` (`DRIVER OK rc=0 KAT transcript digest = ...`) |

Divergences found and fixed while working through this table:

1. **`SpxCtx` / `LeafInfoX1` / `BlakeState256` / `BlakeState512` /
   `ForsGenLeafInfo` were not `#[repr(C)]`.** Rust's default layout reorders
   fields by descending alignment, so e.g. `leaf_info_x1` (`u8* , u32, u32*,
   u32[8], u32[8]`) had `wots_steps` at offset 8 instead of 16, and the haraka
   `spx_ctx` put the 640-byte `tweaked512_rc64` table at offset 0 instead of
   after the two seeds. Every FFI entry point that takes one of these structs
   (`SPX_prf_addr`, `SPX_thash`, `SPX_wots_gen_leafx1`, `SPX_wots_treehashx1`,
   `SPX_merkle_sign`, `blake256_*`, …) was reading the wrong bytes.
   Fixed by adding `#[repr(C)]` (rows 22, 26, 37, 42, 12, 13).
2. **`SPX_gen_message_random` under the BLAKE backend wrote only `SPX_N` bytes**
   where the C writes the whole `blakeX` digest (32 bytes for `SPX_N < 24`, 64
   for `SPX_N >= 24`), because `blakeX_final(&S, R)` does not truncate
   (row 33). `sign.c` hides this because the following `SPX_FORS_BYTES` are
   overwritten immediately, but an FFI caller sees it.
3. **`DRBG_ctx`, `AES256_ECB`, `AES256_CTR_DRBG_Update`, `seedexpander`,
   `seedexpander_init` and `cst` were not exported** (rows 56-59 and the blake
   data-symbol row); `DRBG_ctx` in particular had to stop being a private
   `Mutex<Drbg>` and become the real exported global the C code uses.
