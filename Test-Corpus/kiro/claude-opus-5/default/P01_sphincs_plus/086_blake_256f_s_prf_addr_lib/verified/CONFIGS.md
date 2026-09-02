# CONFIGS.md — configuration-surface table (valid inputs)

Everything below is derived from branches that actually exist in `c_src`, not
from a guess about which options matter.

## Build-time axes (the CMake cache variables)

`c_src/CMakeLists.txt` exposes three cache variables; `app/CMakeLists.txt` builds
two different cores on top of them. All four map to Cargo features.

| axis | values | what it toggles in the C |
|---|---|---|
| `HASH_BACKEND` | `blake`, `haraka`, `sha2`, `shake` | `lib/CMakeLists.txt` does `add_subdirectory(${HASH_BACKEND})`, so a different `hash_*.c`/`thash_*.c`/primitive set is compiled; each backend's `*_offsets.h` also changes **every** `SPX_OFFSET_*` (sha2 uses a 22-byte compressed address: LAYER 0/TREE 1/TYPE 9/KP 10/CHAIN 17/HASH 21/HGT 17/INDEX 18; the other three use LAYER 3/TREE 8/TYPE 19/KP 20/CHAIN 27/HASH 31/HGT 27/INDEX 28) |
| `THASH` | `robust`, `simple` | selects `thash_<backend>_<THASH>.c`; robust XORs a bitmask derived by MGF1/SHAKE/haraka_S from `pub_seed‖addr` before hashing, simple does not |
| `SECPAR` | `128s 128f 192s 192f 256s 256f` | selects `app/params/params-sphincs-<backend>-<SECPAR>.h`: `SPX_N ∈ {16,24,32}`, `SPX_D ∈ {7,22,7,22,8,17}`, `SPX_FULL_HEIGHT ∈ {63,66,63,66,64,68}`, `SPX_FORS_HEIGHT`, `SPX_FORS_TREES`. `SPX_N >= 24` additionally flips `SPX_BLAKE512`/`SPX_SHA512` to 1, which enables a *second* code path inside `thash` and switches `hash_*.c` to the 512-bit primitive |
| randombytes provider | `rng.c` (CMake `sphincs_core_det`, Cargo default) / `randombytes.c` (CMake `sphincs_core`, Cargo `urandom`) | which `randombytes()` `sign.c` links against: NIST AES-256-CTR DRBG vs `/dev/urandom` |

4 × 2 × 6 × 2 = **96 build configurations**, all of which `cargo check` cleanly
and all of which the test matrix below is run under.

## Runtime axes the C branches on

* `thash(inblocks)` — haraka has an explicit `if (inblocks == 1)`; blake and sha2
  have `#if SPX_{BLAKE,SHA}512 → if (inblocks > 1) → thash_512()`; shake has no
  branch. Distinct values used by the library: `1` (F), `2` (H),
  `SPX_WOTS_LEN` (T_len, from `wots_gen_leafx1`/`crypto_sign_verify`),
  `SPX_FORS_TREES` (T_k, from `fors_sign`/`fors_pk_from_sig`). Plus `0`.
* `compute_root` — `if (leaf_idx & 1)` at every level, `idx_offset` zero vs
  non-zero, `tree_height`.
* `treehash` (function-pointer variant, only used by external callers) —
  `(leaf_idx ^ 1) == idx` and `(leaf_idx >> h) ^ 1 == tree_idx` auth-path
  branches, `tree_height`, `idx_offset`.
* `wots_treehashx1` / `fors_treehashx1` — `(internal_idx ^ internal_leaf) == 1`,
  `(internal_idx & 1) == 0 && idx < max_idx`, and the `leaf_idx == ~0u` mode used
  by `merkle_gen_root` in which no auth-path node is ever written.
* `wots_gen_leafx1` — `leaf_idx == info->wots_sign_leaf` decides
  `wots_k_mask = 0` (emit a WOTS signature into `info->wots_sig`) vs `~0`
  (public key only, `info->wots_sig` never touched).
* `gen_chain` — `for (i = start; i < start+steps && i < SPX_WOTS_W; i++)`; the
  `start`/`steps` pair is entirely message-dependent, and `chain_lengths` can
  produce `0` and `15` at both ends.
* `gen_message_random` — sha2 only: `if (SPX_N + mlen < SPX_SHAX_BLOCK_BYTES)`.
* `hash_message` — sha2 only:
  `if (SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES)`;
  all backends: `if (SPX_D == 1)` (never taken by any shipped parameter set, all
  have `SPX_D >= 7`, so the `else` is the only reachable arm).
* `blake256_update` — `if (left && ((datalen>>3 & 0x3F) >= fill))`, the
  `while (datalen >= 512)` block loop, and the residue copy.
  `blake256_final` — `buflen == 440` / `< 440` / `>= 440` (three-way).
  `blake512_*` — same with 888/1024.
* `mgf1` (all four flavours) — `for (i; (i+1)*OUT <= outlen; i++)` plus the
  `if (outlen > i*OUT)` tail: outlen a multiple of the digest size vs not.
* `sha256_inc_finalize` / `sha512_inc_finalize` — padding straddles a block or
  not (`inlen % BLOCK` above/below the 8/16-byte length field).
* `shake256_inc_absorb` / `_inc_squeeze` — partial-rate absorb/squeeze across the
  136-byte rate; `shake256_absorb`+`shake256_squeezeblocks` is a separate
  (non-incremental) entry point.
* `haraka_S_inc_absorb` / `_inc_squeeze` — 32-byte rate crossing;
  `haraka512`/`haraka256`/`haraka512_perm` are separately exported.
* `randombytes` (`rng.c`) — `if (xlen > 15)` block/residue split and the
  byte-wise carry when `V[15] == 0xff`.
* `seedexpander` — `if (xlen <= 16 - buffer_pos)` fast path vs the refill loop,
  and the `ctr[12..16]` carry.
* `randombytes_init` — `if (personalization_string)`.
* `AES256_CTR_DRBG_Update` — `if (provided_data != NULL)`.
* `crypto_sign_open` / `crypto_sign` — `mlen` 0 vs non-zero; `memmove` overlap.
* Address setters — byte-write (truncating) vs 4-byte vs 8-byte fields, and the
  offset-dependent copy lengths of `copy_subtree_addr` (`SPX_OFFSET_TREE+8`) and
  `copy_keypair_addr` (that plus 4 bytes at `SPX_OFFSET_KP_ADDR`).

## Rows

Every row is exercised under **all 96 build configurations** and, unless noted,
with many randomized inputs from a fixed-seed xoshiro-style PRNG (seed
`0x5150_5058_2b2b_0001`), asserting byte-identical outputs from the C `.so` and
the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no inputs; the four compile-time sizes for the active `SECPAR` | [x] |
| C02 | `SPX_ull_to_bytes` | `outlen ∈ {1,2,3,4,5,6,7,8}` × random `in`; also `in` values `0`, `1`, `u64::MAX`, `0x0102…08` | [x] |
| C03 | `SPX_ull_to_bytes` | `outlen ∈ {0, 9, 12, 16}` — zero-length write and zero-extension past 8 bytes | [x] |
| C04 | `SPX_u32_to_bytes` | random `u32`, plus `0`, `1`, `u32::MAX` | [x] |
| C05 | `SPX_bytes_to_ull` | `inlen ∈ {0,1,…,8}` × random input bytes | [x] |
| C06 | `SPX_set_layer_addr`, `SPX_set_type`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height` | single-byte fields; random 32-byte starting address × random `u32` value (exercises the `(unsigned char)` truncation and the backend-specific offset) | [x] |
| C07 | `SPX_set_tree_addr` | 8-byte big-endian field at `SPX_OFFSET_TREE`; `tree ∈ {0, 1, 2^32, u64::MAX}` ∪ random | [x] |
| C08 | `SPX_set_keypair_addr`, `SPX_set_tree_index` | 4-byte big-endian fields; random `u32` ∪ `{0, u32::MAX}` | [x] |
| C09 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random source and destination addresses; verifies the offset-dependent copy lengths (`SPX_OFFSET_TREE+8`, plus 4 at `SPX_OFFSET_KP_ADDR`) | [x] |
| C10 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the *whole* `spx_ctx` afterwards (no-op for blake/shake, `seed_state` for sha2 — including `state_seeded_512` when `SPX_N>=24` —, `tweak_constants` for haraka) | [x] |
| C11 | `SPX_prf_addr` | initialized ctx × random address | [x] |
| C12 | `SPX_thash` | `inblocks = 1` (F function; the haraka `inblocks==1` arm and the non-512 arm of blake/sha2) × random `in`, random `addr`, both `THASH` values | [x] |
| C13 | `SPX_thash` | `inblocks = 2` (H function; first `inblocks > 1` value, so the 512 dispatch is taken for blake/sha2 when `SPX_N>=24`) | [x] |
| C14 | `SPX_thash` | `inblocks = SPX_WOTS_LEN` (T_len) | [x] |
| C15 | `SPX_thash` | `inblocks = SPX_FORS_TREES` (T_k) | [x] |
| C16 | `SPX_thash` | `inblocks = 0` (degenerate; empty payload, no 512 dispatch) | [x] |
| C17 | `SPX_gen_message_random` | `mlen ∈ {0, 1, SPX_N-1, SPX_N, 55, 63, 64, 65, 127, 128, 129, 1000}` — spans the sha2 `SPX_N + mlen < BLOCK` boundary in both directions and the blake/haraka/shake rate boundaries. Output buffer sized for blake's full `SPX_BLAKEX_OUTPUT_BYTES` write. | [x] |
| C18 | `SPX_hash_message` | `mlen ∈ {0, 1, 16, 31, 32, 33, 63, 64, 65, 95, 96, 127, 128, 129, 1000}` — spans sha2's `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*BLOCK` boundary. Compares `digest`, `*tree` and `*leaf_idx`. | [x] |
| C19 | `SPX_chain_lengths` | random `SPX_N`-byte message ∪ all-`0x00` ∪ all-`0xFF` (drives `base_w` and the `wots_checksum` shift to both extremes) | [x] |
| C20 | `SPX_wots_pk_from_sig` | random signature (`SPX_WOTS_BYTES`) × random message × random address; the per-chain `start`/`steps` are message-derived and include `0` and `15` | [x] |
| C21 | `SPX_wots_gen_leafx1` | `leaf_idx != info.wots_sign_leaf` (`wots_k_mask = ~0`, public-key-only path); `wots_sig = NULL`-equivalent (unused buffer), random `wots_steps` | [x] |
| C22 | `SPX_wots_gen_leafx1` | `leaf_idx == info.wots_sign_leaf` (`wots_k_mask = 0`, signature path); compares `dest` **and** the `SPX_WOTS_BYTES` written into `info.wots_sig`, over random `wots_steps ∈ [0,15]` | [x] |
| C23 | `SPX_fors_gen_leafx1` | random `addr_idx` × random `leaf_addrx`; compares the leaf **and** the mutated `info.leaf_addrx` | [x] |
| C24 | `SPX_compute_root` | `tree_height = 1`, `leaf_idx` even and odd, `idx_offset = 0` | [x] |
| C25 | `SPX_compute_root` | `tree_height = SPX_FORS_HEIGHT`, random `leaf_idx < 2^h`, `idx_offset = i * 2^h` non-zero (the FORS use) | [x] |
| C26 | `SPX_compute_root` | `tree_height = SPX_TREE_HEIGHT`, random `leaf_idx`, `idx_offset = 0` (the hypertree use) | [x] |
| C27 | `SPX_treehash` | function-pointer variant with a deterministic test `gen_leaf`; `tree_height ∈ {1,2,3,4}`, `leaf_idx` in range (auth path fully written) | [x] |
| C28 | `SPX_treehash` | same, `leaf_idx = ~0u` and `idx_offset != 0` (no auth-path node ever matches) | [x] |
| C29 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx` random in range, `idx_offset = t * 2^h`, `info.leaf_addrx` seeded from a random address; compares root, auth path and the mutated `tree_addr`/`info` | [x] |
| C30 | `SPX_wots_treehashx1` | `tree_height ∈ {1,2,3}` (kept small — each leaf is a full WOTS key), `leaf_idx` in range with `wots_sign_leaf == leaf_idx` so the WOTS signature is emitted | [x] |
| C31 | `SPX_wots_treehashx1` | same, `leaf_idx = ~0u` / `wots_sign_leaf = ~0u` — the `merkle_gen_root` mode where no auth path and no signature are produced | [x] |
| C32 | `SPX_merkle_sign` | `idx_leaf` random `< 2^SPX_TREE_HEIGHT`; compares the `SPX_WOTS_BYTES + SPX_TREE_HEIGHT*SPX_N` signature, the updated `root`, and both mutated addresses | [x] |
| C33 | `SPX_merkle_gen_root` | full top-subtree root from a random initialized ctx (`idx_leaf = ~0u` internally) | [x] |
| C34 | `SPX_fors_sign` | random `SPX_FORS_MSG_BYTES` message × random `fors_addr`; compares the `SPX_FORS_BYTES` signature and the FORS pk | [x] |
| C35 | `SPX_fors_pk_from_sig` | the signature produced by C34 (round-trip: pk must equal C34's pk) **and** an independent random signature | [x] |
| C36 | `crypto_sign_seed_keypair` | random `3*SPX_N` seed; compares `pk` and `sk` byte-for-byte | [x] |
| C37 | `crypto_sign_signature` + `crypto_sign_verify` | DRBG seeded identically on both sides (`randombytes_init`), `mlen ∈ {0, 1, 33, 64, 128, 1000}`; compares `sig`, `siglen`, and the verify return value | [x] |
| C38 | `crypto_sign` + `crypto_sign_open` | one-shot attached form, `mlen ∈ {0, 1, 33, 1000}`; compares `sm`, `*smlen`, recovered `m`, `*mlen` and return codes | [x] |
| C39 | `crypto_sign_keypair` | DRBG-seeded (default feature set): fully deterministic, compares `pk`/`sk`. Under `urandom`: nondeterministic, so instead cross-checks C-generated keys verify under Rust and vice versa | [x] |
| C40 | `randombytes` (`rng.c`) | DRBG seeded identically; `xlen ∈ {0,1,15,16,17,31,32,33,48,255,1000}` in sequence, so `V` carries propagate and `reseed_counter` advances; compares output **and** the exported `DRBG_ctx` afterwards | [x] |
| C41 | `randombytes_init` | with and without a personalization string; compares `DRBG_ctx.Key`, `.V`, `.reseed_counter` | [x] |
| C42 | `AES256_ECB` | random key × random counter block | [x] |
| C43 | `AES256_CTR_DRBG_Update` | `provided_data` non-NULL and NULL, random `Key`/`V` including `V` all-`0xff` | [x] |
| C44 | `seedexpander_init` + `seedexpander` | `maxlen ∈ {16, 100, 4096, 0xFFFFFFFF}` × a sequence of requests `{1, 15, 16, 17, 32, 100}` that refills the 16-byte buffer several times and carries `ctr`; compares output and the full `AES_XOF_struct` | [x] |
| C45 | blake only: `blake256_init`/`_update`/`_final` | incremental hashing with chunk sequences that hit `buflen` 0, 55·8, 440, 448, 504, 512 bits and multi-block updates; compares digest and the whole `blakestate256` after each step | [x] |
| C46 | blake only: `blake256`, `blake512` | one-shot, `inlen ∈ {0,1,54,55,56,63,64,65,111,112,119,120,127,128,129,1000}` (covers both `final` three-way arms for 256 and 512) | [x] |
| C47 | blake only: `blake256_compress`, `blake512_compress` | direct permutation call on a random state × random 64/128-byte block | [x] |
| C48 | blake only: `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen ∈ {0,1,31,32,33,63,64,65,96,200}` × `inlen ∈ {0,1,32,48,64}` (multiple-of-digest and remainder tails) | [x] |
| C49 | blake only: `blake512_init`/`_update`/`_final` | incremental, chunks hitting `buflen` 888, 896, 1016, 1024 bits | [x] |
| C50 | sha2 only: `sha256_inc_init`/`_inc_blocks`/`_inc_finalize`, `sha256` | `inblocks ∈ {0,1,2,3}` then `inlen ∈ {0,1,55,56,63,64,65,119,120,128,1000}`; compares the 40-byte state after every step | [x] |
| C51 | sha2 only: `sha512_inc_init`/`_inc_blocks`/`_inc_finalize`, `sha512` | `inblocks ∈ {0,1,2}` then `inlen ∈ {0,1,111,112,127,128,129,239,240,256,1000}`; compares the 72-byte state | [x] |
| C52 | sha2 only: `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen ∈ {0,1,31,32,33,63,64,65,96,200}` × `inlen ∈ {0,1,32,48,64}` | [x] |
| C53 | sha2 only: `SPX_seed_state` | random `pub_seed`; compares `state_seeded` (and `state_seeded_512` when `SPX_N>=24`) | [x] |
| C54 | shake only: `shake256_inc_init`/`_inc_absorb`/`_inc_finalize`/`_inc_squeeze` | multiple absorbs with sizes `{0,1,135,136,137,271,272}` and squeezes `{1,135,136,137,272}`; compares the 26-word state after every step | [x] |
| C55 | shake only: `shake256_absorb` + `shake256_squeezeblocks` | non-incremental entry point; `inlen ∈ {0,1,135,136,137}` (note: this variant requires `inlen < rate` semantics of the reference code) × `nblocks ∈ {1,2,3}` | [x] |
| C56 | shake only: `shake256` | `outlen ∈ {1,32,135,136,137,272,1000}` × `inlen ∈ {0,1,135,136,137,1000}` | [x] |
| C57 | haraka only: `SPX_tweak_constants` | random `pub_seed`/`sk_seed`; compares the full 10×8 `u64` and 10×8 `u32` tweaked round-constant tables | [x] |
| C58 | haraka only: `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | tweaked ctx × random 64/32-byte input | [x] |
| C59 | haraka only: `SPX_haraka_S_inc_init`/`_inc_absorb`/`_inc_finalize`/`_inc_squeeze` | absorb sizes `{0,1,31,32,33,64,65}` (32-byte rate) and squeeze sizes `{1,31,32,33,64}`; compares the 65-byte state after every step | [x] |
| C60 | haraka only: `SPX_haraka_S` | `outlen ∈ {1,16,32,33,64,100}` × `inlen ∈ {0,1,31,32,33,100}` | [x] |

## Where the tests live and how to run them

* `translation/tests/common/mod.rs` — harness: `dlopen`s the C reference
  `.so`(s) and the Rust `cdylib`, re-derives every parameter independently from
  the Cargo features, and provides the fixed-seed PRNG.
* `translation/tests/phase_b_core.rs` — rows C01–C44 (42 tests).
* `translation/tests/phase_b_backend.rs` — rows C45–C60 (16 tests; the ones for
  inactive backends are no-ops).

```
./run_tests_all.sh                     # all 96 feature combinations
./run_tests_all.sh shake simple 128f   # one combination
```

Two harness details that matter for correctness of the *test*, not the library:

* The Rust `cdylib` is opened with `RTLD_DEEPBIND`. The C libraries are in the
  global scope and define the same names (`SPX_thash`, `DRBG_ctx`, …) with
  default visibility, so without it the Rust library's own GOT/PLT entries get
  interposed by the C definitions and the comparison silently degenerates to
  C-against-C. The harness asserts that the two resolved addresses differ.
* `cargo test` does not reliably rebuild a `crate-type = ["cdylib"]` artifact, so
  the runner always does an explicit `cargo build --release` first, and the
  harness cross-checks `crypto_sign_bytes()` from both `.so`s against its own
  derivation to catch a stale library.

## Result

All 60 rows pass under **all 96 build configurations**
(`4 backends × 2 THASH × 6 SECPAR × {DRBG, urandom}`): 96 log files in
`testlogs/`, each with 74 passing tests (42 + 16 + 16 error-path) and no failures.

Non-vacuity was confirmed by mutation: injecting a one-byte error into
`shake256()` for `outlen == 137` makes `cfg_c56_shake256_one_shot` fail, and biasing
`bytes_to_ull()` for `inlen == 7` makes `cfg_c05_bytes_to_ull` fail. Both mutations were
reverted.

As an independent whole-program check, the `PQCgenKAT_sign` driver's 100-iteration
transcript digest was compared between the C and Rust executables and matches for
`blake/simple/128f`, `blake/robust/192f`, `sha2/simple/128f`, `shake/robust/256f`
and `haraka/simple/192f`.
