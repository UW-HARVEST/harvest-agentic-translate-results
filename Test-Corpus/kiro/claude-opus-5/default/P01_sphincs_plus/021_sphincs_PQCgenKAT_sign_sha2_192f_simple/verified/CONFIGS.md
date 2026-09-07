# CONFIGS.md — configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for inputs the C **accepts**. Axes derived from the
`#ifdef` / `if` / `switch` branches the C source actually takes, and from the
full set of public entry points in `app/include/*.h` and
`lib/<backend>/include/*.h` — not only the `crypto_sign*` convenience wrappers.

## Build-time axes (from `c_src/CMakeLists.txt` cache variables)

| axis | values | what it switches in the C |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | which `lib/<backend>` is linked; which `<backend>_offsets.h` sets `SPX_OFFSET_*`; which `spx_ctx` tail fields exist; which `-D<BACKEND>_TR=1` transcript the driver uses |
| `THASH` | `robust`, `simple` | `thash_<backend>_<thash>.c` — robust XORs an MGF1/sponge bitmask into the input first |
| `SECPAR` | `128s`,`128f`,`192s`,`192f`,`256s`,`256f` | `SPX_N` ∈ {16,24,32}, `SPX_FULL_HEIGHT`, `SPX_D`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`; and via `SPX_N >= 24` the `SPX_SHA512`/`SPX_BLAKE512` switch that routes `thash`/`hash_message` to the 512-bit primitive |

`shake256` is a Cargo-only alias of `shake` (the CMake cache docstring spells
the SHAKE backend `shake256`); it resolves to the identical code path.

Derived constants per SECPAR (from the parameter headers):

| SECPAR | N | FULL_HEIGHT | D | TREE_HEIGHT | FORS_HEIGHT | FORS_TREES | SHA512/BLAKE512 | ADDR offsets |
|---|---|---|---|---|---|---|---|---|
| 128s | 16 | 63 | 7 | 9 | 12 | 14 | 0 | sha2: 22-byte, else 32-byte |
| 128f | 16 | 66 | 22 | 3 | 6 | 33 | 0 | " |
| 192s | 24 | 63 | 7 | 9 | 14 | 17 | **1** | " |
| 192f | 24 | 66 | 22 | 3 | 8 | 33 | **1** | " |
| 256s | 32 | 64 | 8 | 8 | 14 | 22 | **1** | " |
| 256f | 32 | 68 | 17 | 4 | 9 | 35 | **1** | " |

Full build matrix = 4 × 2 × 6 = **48 configurations** (60 Cargo feature
combinations counting the `shake256` alias). Every row below is exercised in
each of them (see "Execution" at the bottom).

## Runtime axes (from the `if`/`switch` branches on the public API)

* `thash(inblocks)` — `1` vs `>1`: Haraka branches explicitly
  (`inblocks == 1` → `haraka512`/`haraka256`+`haraka512`, else `haraka_S`
  sponge); blake/sha2 branch on `SPX_BLAKE512`/`SPX_SHA512` **and**
  `inblocks > 1` to reach the 512-bit `thash_512`. Meaningful values the
  library itself uses: `1` (F), `2` (H, tree nodes), `SPX_WOTS_LEN` (T_len,
  WOTS pk compression), `SPX_FORS_TREES` (FORS pk compression). `0` is
  degenerate-but-accepted.
* `mlen` in `gen_message_random` / `hash_message` / `crypto_sign*` — sha2
  branches on `SPX_N + mlen < BLOCK` and
  `SPX_N + SPX_PK_BYTES + mlen < INBLOCKS*BLOCK`; shake/haraka sponges branch
  on rate-boundary fill. So: `0`, just-below the boundary, exactly the
  boundary, just-above, and multi-block.
* `compute_root(leaf_idx)` parity — `leaf_idx & 1` picks which half of the
  buffer the leaf goes into, and it is re-tested every level.
* `treehash` / `wots_treehashx1` / `fors_treehashx1` `leaf_idx` — in-range vs
  `~0u` (the "don't produce an auth path" sentinel used by `merkle_gen_root`).
* `wots_gen_leafx1` — `leaf_idx == info->wots_sign_leaf` (emit signature) vs
  `!=` (`wots_k_mask = ~0`, pk only).
* address field widths — 1-byte fields (`layer`, `type`, `chain`,
  `tree_height`), 4-byte big-endian fields (`keypair`, `tree_index`), 8-byte
  big-endian field (`tree`); plus the 22-byte vs 32-byte address layout.
* `seedexpander` — `xlen` smaller than the 16-byte buffer remainder vs
  spanning multiple AES blocks; and the `ctr[12..16]` carry/rollover.
* `randombytes` — `xlen` `< 16`, `== 16`, `> 16` non-multiple (the `xlen > 15`
  branch) and repeated calls (DRBG state advance).

## The table

Randomized: every row is driven with **32 pseudo-random input vectors** from a
fixed-seed SplitMix64 generator (seed `0x5PH1NCS_2024` per row index), not a
single hand-picked value. `[x]` = passes in all 48 build configurations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| **Address primitives (`app/src/address.c`) — lowest level** ||||
| 1 | `SPX_set_layer_addr` | random `addr[8]`, `layer` random full `u32` (exercises the 1-byte truncating store at `SPX_OFFSET_LAYER`; offset differs sha2 vs others) | [x] |
| 2 | `SPX_set_tree_addr` | random `addr[8]`, `tree` random full `u64` (8-byte big-endian store at `SPX_OFFSET_TREE`) | [x] |
| 3 | `SPX_set_type` | random `addr[8]`, `type` ∈ {0..6} (all seven `SPX_ADDR_TYPE_*`) | [x] |
| 4 | `SPX_copy_subtree_addr` | random `in[8]`, random pre-filled `out[8]` (copies `SPX_OFFSET_TREE+8` bytes — 16 for sha2, 40 for the others, i.e. *past* the 32-byte address for non-sha2) | [x] |
| 5 | `SPX_set_keypair_addr` | random `addr[8]`, `keypair` random full `u32` | [x] |
| 6 | `SPX_copy_keypair_addr` | random `in[8]`/`out[8]` (two-range copy: subtree part + 4-byte keypair field) | [x] |
| 7 | `SPX_set_chain_addr` | random `addr[8]`, `chain` ∈ 0..`SPX_WOTS_LEN-1` and random `u32` | [x] |
| 8 | `SPX_set_hash_addr` | random `addr[8]`, `hash` ∈ 0..`SPX_WOTS_W-1` and random `u32` | [x] |
| 9 | `SPX_set_tree_height` | random `addr[8]`, height ∈ 0..`SPX_TREE_HEIGHT` and random `u32` | [x] |
| 10 | `SPX_set_tree_index` | random `addr[8]`, index random full `u32` | [x] |
| 11 | all `set_*`/`copy_*` composed | apply the whole sequence used by `fors_sign` / `merkle_sign` to one `addr[8]` in order, comparing after each step (catches offset-aliasing between `CHAIN_ADDR`/`TREE_HGT` — both 27 for non-sha2, both 17 for sha2 — and between `HASH_ADDR` 31 / `TREE_INDEX` 28..31) | [x] |
| **Byte/integer utilities (`app/src/utils.c`)** ||||
| 12 | `SPX_ull_to_bytes` | `outlen` ∈ {1,2,4,8}, random `u64` value | [x] |
| 13 | `SPX_u32_to_bytes` | random `u32` | [x] |
| 14 | `SPX_bytes_to_ull` | `inlen` ∈ {1,2,4,8}, random bytes | [x] |
| 15 | `SPX_ull_to_bytes` → `SPX_bytes_to_ull` | round trip at each `outlen` | [x] |
| **`thash` (backend × THASH × inblocks)** ||||
| 16 | `SPX_thash` | `inblocks = 1` (F), random `pub_seed`/`sk_seed`/`in`/`addr`, context built via `SPX_initialize_hash_function` | [x] |
| 17 | `SPX_thash` | `inblocks = 2` (H) — the only value that reaches `thash_512` for `SPX_BLAKE512`/`SPX_SHA512` configs | [x] |
| 18 | `SPX_thash` | `inblocks = SPX_WOTS_LEN` (T_len, WOTS pk compression: 35/51/67 blocks) | [x] |
| 19 | `SPX_thash` | `inblocks = SPX_FORS_TREES` (FORS pk compression: 14…35 blocks) | [x] |
| 20 | `SPX_thash` | `inblocks = 3` (odd, > 1, not a library-internal value — guards against a hard-coded 2) | [x] |
| **`hash.h` backend entry points** ||||
| 21 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compare the **entire** `spx_ctx` byte image (no-op for shake/blake, `seed_state` midstates for sha2, `tweak_constants` round-constant tables for haraka) | [x] |
| 22 | `SPX_prf_addr` | random ctx + random `addr[8]` | [x] |
| 23 | `SPX_prf_addr` | random ctx + `addr[8] = 0` and `addr[8] = all-0xFF` (boundary address images) | [x] |
| 24 | `SPX_gen_message_random` | `mlen = 0` | [x] |
| 25 | `SPX_gen_message_random` | `mlen = SPX_SHAX_BLOCK_BYTES - SPX_N - 1` (just below the sha2 single-block branch) | [x] |
| 26 | `SPX_gen_message_random` | `mlen = SPX_SHAX_BLOCK_BYTES - SPX_N` (exactly the sha2 branch boundary → `else` arm) | [x] |
| 27 | `SPX_gen_message_random` | `mlen = SPX_SHAX_BLOCK_BYTES - SPX_N + 1` (just above) | [x] |
| 28 | `SPX_gen_message_random` | `mlen` random in 1..4096 (multi-block, spans shake/haraka sponge rates 136/32) | [x] |
| 29 | `SPX_hash_message` | `mlen = 0`; check `digest`, `*tree`, `*leaf_idx` | [x] |
| 30 | `SPX_hash_message` | `mlen` at the `SPX_INBLOCKS*BLOCK - SPX_N - SPX_PK_BYTES` boundary − 1 / = / + 1 (sha2 two-arm branch) | [x] |
| 31 | `SPX_hash_message` | `mlen` random in 1..4096; asserts `*tree` is masked to `SPX_TREE_BITS` and `*leaf_idx` to `SPX_TREE_HEIGHT` identically | [x] |
| **Backend primitives (`lib/<backend>/include/*.h`) — lowest level of all** ||||
| 32 | blake: `blake256_init`/`_update`/`_final`, `blake256` | `inlen` ∈ {0,1,55,56,64,65,119,120,128,random≤4096} (BLAKE-256 64-byte block + 55/56 padding boundary); incremental vs one-shot | [x] |
| 33 | blake: `blake512_init`/`_update`/`_final`, `blake512` | `inlen` ∈ {0,1,111,112,128,129,random} (128-byte block, 111/112 padding boundary) | [x] |
| 34 | blake: `blake256_compress`, `blake512_compress` | random state + random 64/128-byte block, single permutation | [x] |
| 35 | blake: `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen` ∈ {1, 31, 32, 33, 63, 64, 65, random ≤ 512} × `inlen` ∈ {0,1,random} (counter-block boundary) | [x] |
| 36 | blake: exported `cst` data symbol | compare the 128 bytes at the symbol address | [x] |
| 37 | sha2: `sha256_inc_init`/`_inc_blocks`/`_inc_finalize`, `sha256` | `inblocks` ∈ {0,1,2,5}; finalize `inlen` ∈ {0,1,55,56,63,64,65,random} | [x] |
| 38 | sha2: `sha512_inc_init`/`_inc_blocks`/`_inc_finalize`, `sha512` | `inblocks` ∈ {0,1,2}; finalize `inlen` ∈ {0,1,111,112,127,128,129,random} | [x] |
| 39 | sha2: `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen` × `inlen` grid as row 35 | [x] |
| 40 | sha2: `SPX_seed_state` | random ctx; compare `state_seeded` (+`state_seeded_512` when `SPX_SHA512`) | [x] |
| 41 | shake: `shake256_inc_init`/`_inc_absorb`/`_inc_finalize`/`_inc_squeeze` | absorb split into 1..4 chunks with sizes straddling the 136-byte rate; squeeze in 1..3 chunks | [x] |
| 42 | shake: `shake256_absorb` + `shake256_squeezeblocks` | `inlen` ∈ {0,1,135,136,137,272,random}; `nblocks` ∈ {0,1,2,3} | [x] |
| 43 | shake: `shake256` one-shot | `outlen` ∈ {1,135,136,137,random ≤ 512} × `inlen` ∈ {0,1,135,136,137,random} | [x] |
| 44 | haraka: `SPX_tweak_constants` | random `pub_seed`/`sk_seed`; compare both round-constant tables | [x] |
| 45 | haraka: `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm` | random 32/64-byte inputs against a tweaked ctx | [x] |
| 46 | haraka: `SPX_haraka_S_inc_init`/`_absorb`/`_finalize`/`_squeeze` | absorb split into 1..4 chunks straddling the 32-byte rate; squeeze in 1..3 chunks | [x] |
| 47 | haraka: `SPX_haraka_S` one-shot | `outlen` ∈ {1,31,32,33,random ≤ 512} × `inlen` ∈ {0,1,31,32,33,random} | [x] |
| **WOTS (`app/src/wots.c`, `wotsx1.c`)** ||||
| 48 | `SPX_chain_lengths` | random `SPX_N`-byte message; compare all `SPX_WOTS_LEN` lengths (`base_w` + checksum shift) | [x] |
| 49 | `SPX_chain_lengths` | all-`0x00` and all-`0xFF` message (checksum extremes) | [x] |
| 50 | `SPX_wots_pk_from_sig` | random ctx, random `SPX_WOTS_BYTES` signature, random `SPX_N` message, random `addr[8]`; compare pk **and** the mutated `addr[8]` | [x] |
| 51 | `SPX_wots_gen_leafx1` | `leaf_idx != wots_sign_leaf` (pk-only path, `wots_k_mask = ~0`); `wots_sig = NULL` | [x] |
| 52 | `SPX_wots_gen_leafx1` | `leaf_idx == wots_sign_leaf` (signature path); compare `dest`, the emitted `wots_sig`, and both mutated addresses in `leaf_info_x1` | [x] |
| **FORS (`app/src/fors.c`)** ||||
| 53 | `SPX_fors_gen_leafx1` | random ctx, random `addr_idx` incl. `0` and `2^FORS_HEIGHT*FORS_TREES-1`; compare `leaf` and the mutated `fors_gen_leaf_info` | [x] |
| 54 | `SPX_fors_sign` | random ctx, random `SPX_FORS_MSG_BYTES` message, random `fors_addr[8]`; compare the full `SPX_FORS_BYTES` signature and the `SPX_N` pk | [x] |
| 55 | `SPX_fors_sign` | all-`0x00` and all-`0xFF` message (index extremes: every tree index 0 / max) | [x] |
| 56 | `SPX_fors_pk_from_sig` | fed the signature produced in row 54 → must reproduce the row-54 pk in both implementations | [x] |
| 57 | `SPX_fors_pk_from_sig` | random (invalid) signature bytes — still a valid *input*, must produce identical garbage pk | [x] |
| **Merkle / treehash (`app/src/utils.c`, `utilsx1.c`, `merkle.c`)** ||||
| 58 | `SPX_compute_root` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` **even**, `idx_offset = 0` | [x] |
| 59 | `SPX_compute_root` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` **odd**, `idx_offset = 0` | [x] |
| 60 | `SPX_compute_root` | `tree_height = SPX_FORS_HEIGHT`, random `leaf_idx`, random non-zero `idx_offset` (FORS usage) | [x] |
| 61 | `SPX_compute_root` | `tree_height = 1` (single level, loop body never runs) | [x] |
| 62 | `SPX_treehash` (function-pointer variant) | `gen_leaf` = a deterministic `extern "C"` callback supplied by the test, `tree_height` ∈ {1,2,3}, `leaf_idx` random, `idx_offset` ∈ {0, random}; compare `root`, `auth_path`, mutated `tree_addr` | [x] |
| 63 | `SPX_treehash` | `gen_leaf` = `SPX_fors_gen_leafx1` **from the same `.so`** (real usage, `fors_gen_leaf_info` aliased over `tree_addr[8]`), `tree_height = min(SPX_FORS_HEIGHT, 4)` | [x] |
| 64 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx` random in range, `wots_sign_leaf = leaf_idx` (signing) | [x] |
| 65 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx = ~0u`, `wots_sign_leaf = ~0u`, `wots_sig = NULL` (the `merkle_gen_root` shape) | [x] |
| 66 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` random, `idx_offset = i * 2^FORS_HEIGHT` for random `i` | [x] |
| 67 | `SPX_merkle_sign` | random ctx, random `wots_addr`/`tree_addr`, `idx_leaf` random in `0..2^TREE_HEIGHT`; compare signature, root, and both mutated addresses | [x] |
| 68 | `SPX_merkle_sign` | `idx_leaf = ~0u` (the `merkle_gen_root` sentinel) | [x] |
| 69 | `SPX_merkle_gen_root` | random ctx | [x] |
| **Top-level API (`app/include/api.h`)** ||||
| 70 | `crypto_sign_secretkeybytes` / `_publickeybytes` / `_bytes` / `_seedbytes` | no input; the four size constants per configuration | [x] |
| 71 | `crypto_sign_seed_keypair` | random `CRYPTO_SEEDBYTES` seed; compare `pk` and `sk` | [x] |
| 72 | `crypto_sign_keypair` | seeded via `randombytes_init` (deterministic DRBG) so both sides consume identical randomness; compare `pk`, `sk` | [x] |
| 73 | `crypto_sign_signature` | keypair from row 71, `mlen = 0`, `optrand` from the seeded DRBG; compare `sig`, `siglen` | [x] |
| 74 | `crypto_sign_signature` | `mlen` ∈ {1, 32, 33, 64, 127, 128, 129, random ≤ 1024} (straddles every backend block/rate boundary) | [x] |
| 75 | `crypto_sign_verify` | valid `sig`/`m`/`pk` from rows 71+74 → `0` on both | [x] |
| 76 | `crypto_sign` | `mlen` ∈ {0, 1, random}; compare the full `sm` (signature ‖ message) and `smlen` | [x] |
| 77 | `crypto_sign_open` | `sm`/`smlen` from row 76 → `0`, identical recovered `m` and `mlen` | [x] |
| 78 | full pipeline, cross-implementation | C-signed → Rust-verified and Rust-signed → C-verified (both must accept) | [x] |
| **Deterministic DRBG (`app/src/rng.c`)** ||||
| 79 | `randombytes_init` + `DRBG_ctx` | random 48-byte entropy, `personalization_string = NULL`; compare the exported `DRBG_ctx` bytes (`Key`, `V`, `reseed_counter`) | [x] |
| 80 | `randombytes_init` | random 48-byte entropy **and** random 48-byte personalization string | [x] |
| 81 | `randombytes` | `xlen` ∈ {1, 15, 16, 17, 48, 100, random ≤ 4096}; compare output **and** `DRBG_ctx` after the call | [x] |
| 82 | `randombytes` | 10 successive calls of random length (DRBG state chaining, `V` carry across `0xff` bytes) | [x] |
| 83 | `AES256_ECB` | random 32-byte key, random 16-byte counter | [x] |
| 84 | `AES256_CTR_DRBG_Update` | random `Key`/`V`, `provided_data = NULL` | [x] |
| 85 | `AES256_CTR_DRBG_Update` | random `Key`/`V`, random 48-byte `provided_data` | [x] |
| 86 | `AES256_CTR_DRBG_Update` | `V` = all `0xff` (counter rollover on all 16 bytes) | [x] |
| 87 | `seedexpander_init` + `seedexpander` | random 32-byte seed, random 8-byte diversifier, `maxlen` ∈ {16, 256, 0xFFFFFFFF}; then `seedexpander` with `xlen` ∈ {1, 15, 16, 17, 33} — spans the `xlen <= 16-buffer_pos` fast path and the block-refill path | [x] |
| 88 | `seedexpander` | repeated calls until the internal counter `ctr[15]` rolls over (carry into `ctr[14]`) | [x] |
| **Binary driver (`app/src/PQCgenKAT_sign.c` vs `src/main.rs`)** ||||
| 89 | `driver` executable | no input; stdout compared byte-for-byte (the KAT transcript digest, which itself depends on backend, THASH and SECPAR) | [x] |

## Execution

* `translation/tests/configs.rs` implements rows 1–88; rows whose entry points
  exist only for one backend (32–47) are `#[cfg]`-gated on the backend feature,
  exactly as the C `.so` only exports them for that backend.
* Row 89 is `scripts/driver_diff.sh`.
* `scripts/test_all.sh` re-runs everything for all 48 configurations.
* `translation/tests/isolation.rs` is a **self-check on the harness itself**, not
  a `CONFIGS.md` row. The C `.so`s must be opened `RTLD_GLOBAL` (they are
  mutually recursive), which places a C definition of every exported symbol
  ahead of the Rust library in the dynamic-linker search order. Without
  `RTLD_DEEPBIND` the Rust library's own references (`DRBG_ctx`, `AES256_ECB`,
  `blake256`, `SPX_thash`, …) get interposed by the C ones and the entire suite
  silently degenerates into "C vs C". `isolation.rs` loads a second copy of the
  Rust `.so` with `dlmopen(LM_ID_NEWLM, …)` — a namespace that cannot see the C
  libraries at all — and asserts the normally-loaded copy produces identical
  results. This was a real defect in the first version of the harness, found via
  row 79 (`DRBG_ctx` read back as all zeros).

### Results

Per configuration the suite runs 3 smoke + 4 isolation + 69–71 config +
40 error tests (the config count varies with the backend-gated rows: 69 shake,
70 haraka/sha2, 71 blake).

```
$ ./scripts/test_all.sh          # all 48 configurations
OK   haraka-robust-128s (ok. 3 passed ok. 4 passed ok. 70 passed ok. 40 passed )
...
OK   blake-simple-256f  (ok. 3 passed ok. 4 passed ok. 71 passed ok. 40 passed )

$ ./scripts/driver_diff.sh       # row 89, all 48 configurations
OK   haraka-robust-128s  KAT transcript digest = 83F5A066...D532
...
OK   blake-simple-256f   KAT transcript digest = A152E392...4583
```

48/48 configurations pass every row, and the four `shake256`-alias combinations
were checked separately.

### Negative control

To confirm the suite is not vacuous, two deliberate single-token bugs were
injected into the Rust and the suite re-run:

| injected bug | caught by |
|---|---|
| `src/utils.rs:88` `compute_root`: `leaf_idx & 1 != 0` → `== 0` | `cfg_58`, `cfg_59`, `cfg_60`, `cfg_54_56`, `cfg_55`, `cfg_57`, `cfg_73_74_75`, `cfg_76_77`, `cfg_78`, `err_06` |
| `src/sign.rs:212` `crypto_sign_verify`: `siglen != SPX_BYTES` → `siglen < SPX_BYTES` | `err_03`, `err_04` |

Both were reverted afterwards and the suite returned to green.
