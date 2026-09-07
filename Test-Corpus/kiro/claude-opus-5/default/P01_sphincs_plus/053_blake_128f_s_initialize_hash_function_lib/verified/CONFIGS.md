# CONFIGS.md — configuration surface (valid inputs)

The mirror of `ERRORS.md`: every axis the C code actually branches on for
*valid* input, and the combinations it treats differently.

## Axis 1 — build-time configuration (the CMake cache variables)

`c_src/CMakeLists.txt` declares exactly three cache variables, and
`app/CMakeLists.txt` / `lib/<be>/CMakeLists.txt` turn them into a source-file
selection plus `-DPARAMS=sphincs-<be>-<secpar>`:

| CMake variable | values | what it selects |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | `lib/<be>/` sources, `<be>_offsets.h` (⇒ the address field offsets), `-D<BE>_TR=1` |
| `THASH` | `robust`, `simple` | `thash_<be>_<THASH>.c` |
| `SECPAR` | `128s`, `128f`, `192s`, `192f`, `256s`, `256f` | `app/params/params-sphincs-<be>-<secpar>.h` |

⇒ **4 × 2 × 6 = 48 valid combinations**, all of which are Cargo features of the
same name.  `Cargo.toml` additionally defines `shake256 = ["shake"]`, because the
CMake cache docstring lists `shake256` as a spelling of the SHAKE backend; that
alias is verified too (`shake256,{simple,robust},{128f,256s}`), bringing the
tested feature sets to 52.  Combinations that set two values of the same axis
(e.g. `128f` + `256s`) are not valid CMake configurations; `lib.rs` documents the
resolution order that applies if a caller does it anyway.

Parameter values the C headers actually differ on (extracted from
`app/params/params-sphincs-*.h`):

| secpar | `SPX_N` | `FULL_HEIGHT` | `SPX_D` | `TREE_HEIGHT` | `FORS_HEIGHT` | `FORS_TREES` | `WOTS_LEN` | `*512` |
|---|---|---|---|---|---|---|---|---|
| `128s` | 16 | 63 | 7  | 9  | 12 | 14 | 35 | 0 |
| `128f` | 16 | 66 | 22 | 3  | 6  | 33 | 35 | 0 |
| `192s` | 24 | 63 | 7  | 9  | 14 | 17 | 51 | 1 |
| `192f` | 24 | 66 | 22 | 3  | 8  | 33 | 51 | 1 |
| `256s` | 32 | 64 | 8  | 8  | 14 | 22 | 67 | 1 |
| `256f` | 32 | 68 | 17 | 4  | 9  | 35 | 67 | 1 |

`*512` is `SPX_SHA512` / `SPX_BLAKE512` — it flips `hash_sha2.c` /
`hash_blake.c` to the wide primitive and enables the `if (inblocks > 1)` branch
in `thash_sha2_*.c` / `thash_blake_*.c`.  `SPX_D == 1` never occurs in a shipped
parameter set, so the `if (SPX_D == 1) *tree = 0;` branch in every
`hash_message` is dead in all 48 configurations.  Address offsets differ between
`sha2` (compressed, 22-byte address) and the other three (full 32-byte).

## Axis 2 — runtime modes / flags the public API can set

SPHINCS+ has no options struct; the "modes" are carried in the ADRS word and in
the `leaf_info_x1` / `fors_gen_leaf_info` structures:

| mode / flag | values the C branches on | where |
|---|---|---|
| ADRS type (`SPX_set_type`) | `WOTS`=0, `WOTSPK`=1, `HASHTREE`=2, `FORSTREE`=3, `FORSPK`=4, `WOTSPRF`=5, `FORSPRF`=6 | mixed into every `thash`/`prf_addr` input |
| `thash` `inblocks` | `1` vs `> 1` | `thash_haraka_*` (F vs H), `thash_sha2_*` / `thash_blake_*` when `*512` |
| `leaf_info_x1.wots_sign_leaf` | `== leaf_idx` (emit WOTS signature) vs `!= leaf_idx` (`wots_k_mask = ~0`, pk only) | `wotsx1.c:28` |
| `leaf_info_x1.wots_sig` | non-null (signing) vs unused-and-null (`merkle_gen_root` passes `idx_leaf = ~0u`) | `merkle.c:59` |
| `compute_root` / `treehash` / `*_treehashx1` leaf parity | `leaf_idx & 1 == 0` vs `== 1` (left vs right child) | `utils.c:60,78`, `utilsx1.c:74` |
| `*_treehashx1` end-of-tree | `idx < max_idx` vs `idx == max_idx` (keep climbing) | `utilsx1.c:74,151` |
| `gen_message_random` (sha2 only) | `SPX_N + mlen < SPX_SHAX_BLOCK_BYTES` vs `>=` | `hash_sha2.c:94` |
| `hash_message` (sha2 only) | `SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS*BLOCK` vs `>=` | `hash_sha2.c:156` |
| `randombytes_init` personalization | `NULL` vs 48-byte string | `rng.c:143` |
| `AES256_CTR_DRBG_Update` `provided_data` | `NULL` vs 48 bytes | `rng.c:205` |
| `seedexpander` buffer state | `xlen <= 16 - buffer_pos` (serve from buffer) vs `>` (re-key, bump counter) | `rng.c:75` |
| `seedexpander` counter carry | `ctr[15..12]` == `0xff` (carry) vs not | `rng.c:92` |

## Axis 3 — input shapes the code special-cases

* message length `mlen`: `0`, `1`, the sha2 block-fill boundary `−1 / = / +1`,
  multi-block, and a length > one keccak/blake block;
* `inblocks`: `0`, `1`, `2`, `SPX_FORS_TREES`, `SPX_WOTS_LEN`;
* `tree_height`: `0`, `1`, `2`, `SPX_FORS_HEIGHT`, `SPX_TREE_HEIGHT`;
* `leaf_idx`: `0`, `1`, `2^h − 1`, and random interior values (both parities);
* `idx_offset`: `0` and `i * 2^FORS_HEIGHT` (the value `fors_sign` uses);
* `outlen`/`inlen` for `ull_to_bytes` / `bytes_to_ull`: `0 … 16`;
* mgf1 / shake / haraka-S `outlen`: shorter than, equal to and longer than one
  output block, and non-block-multiples;
* incremental hash `absorb` split points: byte-at-a-time vs one shot vs across
  the rate boundary;
* `WOTS` message bytes: all-`0x00`, all-`0xff`, random (drives `chain_lengths`
  to both extremes);
* keys/seeds: all-`0x00`, all-`0xff`, random.

## The rows

Each row is one meaningful combination of the axes above.  Every row is
exercised against **both** `.so`s through `libloading` — the Rust
implementation is never called directly, so the `#[no_mangle]` wrappers are
part of what is tested — with many randomized inputs (fixed seed
`0x5EED_C0FFEE`, so failures reproduce), and repeated for all 48 feature
combinations by `./test_all.sh`.

How the libraries are loaded (`tests/common/mod.rs`):

* the C side is ONE self-contained shared object, `cbuild_flat/libspx_<combo>.so`,
  linked by `./build_c_flat.sh` from the same sources and the same flags CMake
  uses.  CMake splits the library into `libsphincs_core[_det].so` and
  `lib<be>.so`, which reference each other's symbols and therefore can only be
  `dlopen`ed `RTLD_GLOBAL` — where they would interpose the identically named
  Rust exports and make every comparison vacuous.  Row **Z1** verifies the
  substitution is sound by loading the real CMake artifacts alongside and
  checking they agree with the flat object.
* both objects are opened `RTLD_LOCAL`, and the loader asserts that each shared
  symbol name resolves to two *different* addresses, and that both objects
  report the parameter set the test binary was compiled for (a stale
  `target/release/libsphincs_plus.so` from another `--features` run would
  otherwise silently corrupt every buffer).
* every test that touches the DRBG holds a process-wide lock, because `DRBG_ctx`
  is global state and `libtest` runs tests in parallel threads.

Running the tests: `cargo test` does **not** rebuild the `cdylib`, so run
`cargo build --release --no-default-features --features <combo>` first — or just
use `./test_all.sh`, which does it for you.  Forgetting is not silent: the
parameter-set guard fails every test with the exact command to run.

`entry point(s)` names are the exported (namespaced) symbols.

### Level 0 — `utils.c`, `address.c`

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C0 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | harness self-check: the test harness re-derives `SPX_N`, `SPX_D`, `SPX_BYTES`, … from the Cargo features independently of `src/params.rs`, and this asserts them against what the **C** library reports, so a wrong constant in the harness cannot silently weaken every other row | `c00_harness_constants_match_c` | [x] |
| C1 | `SPX_ull_to_bytes` | `outlen ∈ 0..=16` × random `u64` values incl. `0`, `1`, `u64::MAX` | `c01_ull_to_bytes` | [x] |
| C2 | `SPX_u32_to_bytes` | random `u32` incl. `0`, `u32::MAX` | `c02_u32_to_bytes` | [x] |
| C3 | `SPX_bytes_to_ull` | `inlen ∈ 0..=16` × random byte strings (incl. all-`0xff`) | `c03_bytes_to_ull` | [x] |
| C4 | round trip `SPX_ull_to_bytes` → `SPX_bytes_to_ull` | `outlen = inlen ∈ 1..=8` | `c04_ull_roundtrip` | [x] |
| C5 | all 10 ADRS setters (`set_layer_addr`, `set_tree_addr`, `set_type`, `set_keypair_addr`, `set_chain_addr`, `set_hash_addr`, `set_tree_height`, `set_tree_index`) applied in sequence to a random 32-byte ADRS | random values in range; compares the whole 32-byte word after each call | `c05_addr_setters_sequence` | [x] |
| C6 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random source/dest ADRS pairs (destination pre-filled with random bytes, so the *unwritten* bytes are checked too) | `c06_addr_copy` | [x] |
| C7 | `SPX_set_type` | every valid type `0..=6` on a random ADRS | `c07_set_type_all_valid` | [x] |
| C8 | `SPX_compute_root` | `tree_height ∈ {1, 2, SPX_FORS_HEIGHT, SPX_TREE_HEIGHT}` × `leaf_idx ∈ {0, 1, 2^h−1, random}` × `idx_offset ∈ {0, random}` × random leaf/auth-path, ADRS type `FORSTREE` and `HASHTREE` | `c08_compute_root` | [x] |
| C9 | `SPX_treehash` (function-pointer entry point) | `tree_height ∈ {0, 1, 2, 3}` × `leaf_idx` all values in range × `idx_offset ∈ {0, random}`, with a Rust `extern "C"` `gen_leaf` callback so the *same* callback drives both libraries; compares root **and** the full auth path | `c09_treehash_callback` | [x] |

### Level 0 — backend primitives (`cfg`-gated per backend)

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C10 | `blake256`, `blake512` | one-shot over `inlen ∈ {0,1,55,56,63,64,65,111,112,127,128,129,random}` (both padding branches of both block sizes) | `c10_blake_oneshot` | [x] |
| C11 | `blake256_init/_update/_final`, `blake512_…` | streaming: single update, byte-at-a-time, and random split points across the block boundary; also the `buflen`-remainder path | `c11_blake_streaming` | [x] |
| C12 | `blake256_compress`, `blake512_compress` | raw compression on a random state + block (state supplied through the same `blakestate*` layout) | `c12_blake_compress` | [x] |
| C13 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen ∈ {1, 31,32,33, 63,64,65, 100, N, inblocks*N}` × random `in` of length `SPX_N+32` | `c13_blake_mgf1` | [x] |
| C14 | `cst` (exported read-only table) | byte-compare the 128 bytes against the C object's copy | `c14_blake_cst` | [x] |
| C15 | `sha256`, `sha512` | one-shot over the same `inlen` set as C10 | `c15_sha_oneshot` | [x] |
| C16 | `sha256_inc_init/_inc_blocks/_inc_finalize`, `sha512_…` | `inc_blocks` with `inblocks ∈ {0,1,2,3}` then `inc_finalize` with a partial tail `∈ {0,1,rate−9,rate−8,rate−1,rate}` (covers both length-encoding branches) | `c16_sha_incremental` | [x] |
| C17 | `SPX_mgf1_256`, `SPX_mgf1_512` | same `outlen` set as C13 | `c17_sha_mgf1` | [x] |
| C18 | `SPX_seed_state` | random `pub_seed`; compares the whole `spx_ctx` byte image (`state_seeded`, and `state_seeded_512` when `SPX_SHA512`) | `c18_sha_seed_state` | [x] |
| C19 | `shake256` | `outlen ∈ {1, 135,136,137, 271,272, random}` × `inlen ∈ {0,1,135,136,137,random}` (rate boundary on both sides) | `c19_shake256_oneshot` | [x] |
| C20 | `shake256_absorb` + `shake256_squeezeblocks` | `nblocks ∈ {1,2,3}` on a random `inlen`; the low-level (non-incremental) API | `c20_shake_absorb_squeeze` | [x] |
| C21 | `shake256_inc_init/_inc_absorb/_inc_finalize/_inc_squeeze` | absorb split into 1..5 random chunks (straddling the 136-byte rate), squeeze in 1..3 chunks | `c21_shake_incremental` | [x] |
| C22 | `SPX_tweak_constants` | random `pub_seed`+`sk_seed`; compares the full 960-byte constant block in `spx_ctx` | `c22_haraka_tweak_constants` | [x] |
| C23 | `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm` | random 32/64-byte inputs under a tweaked `ctx` | `c23_haraka_perms` | [x] |
| C24 | `SPX_haraka_S` | `outlen ∈ {1,31,32,33,63,64,65,random}` × `inlen ∈ {0,1,31,32,33,random}` (32-byte sponge rate boundary) | `c24_haraka_S` | [x] |
| C25 | `SPX_haraka_S_inc_init/_absorb/_finalize/_squeeze` | absorb in 1..5 random chunks, squeeze in 1..3 chunks | `c25_haraka_S_incremental` | [x] |

### Level 1 — `hash.h` / `thash.h`

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C26 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compares the **entire** resulting `spx_ctx` image (no-op for shake/blake, `seed_state` for sha2, `tweak_constants` for haraka) | `c26_initialize_hash_function` | [x] |
| C27 | `SPX_prf_addr` | random ctx × random ADRS × every ADRS type `0..=6` | `c27_prf_addr` | [x] |
| C28 | `SPX_gen_message_random` | `mlen ∈ {0, 1, 2, 31, 32, 33, 63, 64, 65, 231, 1000}` ∪ `{B−N−1, B−N, B−N+1, 2(B−N), …}` ∪ `{K−1, K, K+1, 2K, …}` ∪ `{block−1, block, block+1, …}` where `B` is the backend block/rate — covers both branches of `hash_sha2.c:94`.  A 64-byte window past `R` is compared, because on the blake backend the C writes the full digest there (see `ERRORS.md`) | `c28_gen_message_random` | [x] |
| C29 | `SPX_hash_message` | `mlen ∈ {0, 1, K−1, K, K+1, 2K, 1000, random}` where `K = SPX_INBLOCKS*BLOCK − N − PK_BYTES` — covers both branches of `hash_sha2.c:156`; compares `digest`, `tree` **and** `leaf_idx` | `c29_hash_message` | [x] |
| C30 | `SPX_thash` | `inblocks ∈ {1, 2, 3, SPX_FORS_TREES, SPX_WOTS_LEN}` × ADRS type `0..=6` × random ctx/input — crosses the `inblocks > 1` / `inblocks == 1` branch in every backend | `c30_thash` | [x] |
| C31 | `SPX_thash` | `inblocks == 1` with each of the 7 ADRS types and `pub_seed` all-`0x00` / all-`0xff` / random | `c31_thash_edge_seeds` | [x] |

### Level 2 — WOTS / FORS building blocks

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C32 | `SPX_chain_lengths` | `msg` all-`0x00`, all-`0xff`, random (all `SPX_WOTS_LEN` outputs compared) | `c32_chain_lengths` | [x] |
| C33 | `SPX_wots_pk_from_sig` | random ctx/sig × `msg` all-`0x00` / all-`0xff` / random × ADRS with random keypair+tree; also compares the mutated ADRS afterwards | `c33_wots_pk_from_sig` | [x] |
| C34 | `SPX_wots_gen_leafx1` | **signing mode** (`wots_sign_leaf == leaf_idx`, `wots_sig` non-null): compares the leaf **and** the `SPX_WOTS_BYTES` signature **and** the mutated `leaf_addr`/`pk_addr` | `c34_wots_gen_leafx1_signing` | [x] |
| C35 | `SPX_wots_gen_leafx1` | **pk-only mode** (`wots_sign_leaf = ~0u`), `wots_steps` all-zero / random | `c35_wots_gen_leafx1_pkonly` | [x] |
| C36 | `SPX_fors_gen_leafx1` | random ctx × `addr_idx ∈ {0, 1, 2^FORS_HEIGHT−1, random}` × random `leaf_addrx`; compares leaf and mutated info | `c36_fors_gen_leafx1` | [x] |
| C37 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx ∈ {0, 1, 2^h−1, ~0u, random}`, `idx_offset ∈ {0, random}`, signing and pk-only modes; compares root **and** auth path **and** the emitted WOTS signature | `c37_wots_treehashx1` | [x] |
| C38 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT` and also `{1,2}`, `leaf_idx` in range incl. both parities, `idx_offset ∈ {0, i*2^h}` | `c38_fors_treehashx1` | [x] |

### Level 3 — FORS / Merkle

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C39 | `SPX_fors_sign` | random ctx × random `SPX_FORS_MSG_BYTES` message (plus all-`0x00`, all-`0xff`) × random `fors_addr` with random keypair; compares the full `SPX_FORS_BYTES` signature and the FORS pk | `c39_fors_sign` | [x] |
| C40 | `SPX_fors_pk_from_sig` | fed the signature produced by C40's counterpart *and* a freshly randomized signature; compares the derived pk | `c40_fors_pk_from_sig` | [x] |
| C41 | `SPX_fors_sign` → `SPX_fors_pk_from_sig` | cross-composition: C-signed → Rust-recovered and Rust-signed → C-recovered must both reproduce the signer's pk | `c41_fors_cross` | [x] |
| C42 | `SPX_merkle_sign` | random ctx × `idx_leaf ∈ {0, 1, 2^TREE_HEIGHT−1, random}` × random `wots_addr`/`tree_addr` (layer `0` and `SPX_D−1`); compares the `SPX_WOTS_BYTES + SPX_TREE_HEIGHT*SPX_N` signature, the in/out `root`, and the mutated `tree_addr` | `c42_merkle_sign` | [x] |
| C43 | `SPX_merkle_sign` | `idx_leaf == ~0u` (the "no auth path" mode `merkle_gen_root` uses) | `c43_merkle_sign_no_authpath` | [x] |
| C44 | `SPX_merkle_gen_root` | random ctx (`pub_seed`/`sk_seed` random, all-`0x00`, all-`0xff`) | `c44_merkle_gen_root` | [x] |

### Level 4 — `api.h`

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C45 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no inputs — the four size accessors | `c45_size_accessors` | [x] |
| C46 | `crypto_sign_seed_keypair` | random 3·N seeds, plus all-`0x00` and all-`0xff`; compares both `pk` and `sk` | `c46_seed_keypair` | [x] |
| C47 | `crypto_sign_keypair` | driven through the deterministic DRBG (`randombytes_init` with a fixed 48-byte entropy input on *both* libraries first), so the internal `randombytes` call is reproducible; compares `pk`, `sk` **and** the resulting `DRBG_ctx` | `c47_keypair_via_drbg` | [x] |
| C48 | `crypto_sign_signature` | DRBG-seeded so `optrand` is reproducible; `mlen ∈ {0, 1, 32, 33, 64, 128, 1000, random}`; compares `sig` byte-for-byte and `siglen` | `c48_signature_via_drbg` | [x] |
| C49 | `crypto_sign_signature` | signature reconstructed from the *low-level* entry points (`gen_message_random`, `hash_message`, `fors_sign`, `merkle_sign` in the exact `sign.c` order) and compared against the one-shot output — the composed-pipeline check, run on both libraries | `c49_signature_lowlevel_recompose` | [x] |
| C50 | `crypto_sign_verify` | valid signature (both C-produced and Rust-produced, cross-checked) × `mlen` set of C48 | `c50_verify_valid_cross` | [x] |
| C51 | `crypto_sign` | DRBG-seeded; `mlen ∈ {0,1,33,231,1000}`; compares `sm` (signature ‖ message) and `smlen` | `c51_sign_attached` | [x] |
| C52 | `crypto_sign_open` | valid `sm` from C51; compares return code, `*mlen` and the recovered message buffer | `c52_open_attached` | [x] |
| C53 | `crypto_sign_keypair` → `crypto_sign` → `crypto_sign_open` | full round trip repeated `LOOP_COUNT` times off one DRBG seeding, so the DRBG state chains across iterations exactly as `PQCgenKAT_sign.c` does | `c53_full_roundtrip_chained` | [x] |

### RNG (`rng.c`)

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C54 | `AES256_ECB` | random 32-byte key × random 16-byte block (plus all-`0x00`, all-`0xff`) | `c54_aes256_ecb` | [x] |
| C55 | `AES256_CTR_DRBG_Update` | `provided_data` non-null random / `NULL` × random `Key`/`V`, including `V` = all-`0xff` (carry propagation through all 16 bytes) | `c55_drbg_update` | [x] |
| C56 | `randombytes_init` + `randombytes` | personalization `NULL` and non-`NULL`; `xlen ∈ {1, 15, 16, 17, 48, 64, 1000}`; compares the output **and** the resulting `DRBG_ctx` (Key, V, reseed_counter) | `c56_randombytes` | [x] |
| C57 | `randombytes` | many successive calls off one seeding, to check the DRBG state chain and `reseed_counter` | `c57_randombytes_chained` | [x] |
| C58 | `seedexpander_init` + `seedexpander` | `maxlen ∈ {1, 16, 17, 256, 0xFFFF, 0xFFFFFFFF}` × `xlen` sequences that (a) stay inside the 16-byte buffer, (b) straddle it, (c) span several AES blocks; compares output **and** the full `AES_XOF_struct` after each call | `c58_seedexpander` | [x] |
| C59 | `seedexpander` | `diversifier` chosen so `ctr[15..12]` reaches `0xff` and carries | `c59_seedexpander_carry` | [x] |
| C60 | `DRBG_ctx` (exported global) | compare the 52-byte image of the C and Rust globals after identical `randombytes_init` + `randombytes` sequences | `c60_drbg_ctx_global` | [x] |

### Whole-program

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|--------------------------------------------|------|---|
| C61 | `driver` (`PQCgenKAT_sign.c` vs `src/main.rs`) | the NIST KAT loop: fixed 48-byte entropy, 7 iterations, `mlen = 33*(i+1)`, keypair→sign→open each time; compares the printed 32-byte transcript digest.  This is the only check that exercises the CMake-linked C binary end to end | `./kat_all.sh` | [x] |
| Z1 | `crypto_sign_{secretkey,publickey,,seed}bytes`, `crypto_sign_seed_keypair`, `crypto_sign_verify`, `SPX_initialize_hash_function`, `SPX_prf_addr`, `SPX_thash`, `SPX_merkle_gen_root`, `SPX_fors_sign`, `AES256_ECB` | the real CMake artifacts (`app/libsphincs_core_det.so` + `lib/<be>/lib<be>.so`) vs the flat object the other rows use — the anti-loophole check for the substitution described above, with a pointer-inequality guard so it cannot pass vacuously | `z01_cmake_libs_match_flat_lib` | [x] |

## Result

103 differential tests per feature combination, all passing for all 48
combinations plus the four `shake256`-alias sets:

| test target | tests | covers |
|---|---|---|
| `tests/level0.rs` | 10 | C1–C9 (+ a harness-constant cross-check against the C accessors) |
| `tests/backend.rs` | 3–5 (backend-dependent) | C10–C25 |
| `tests/hashes.rs` | 6 | C26–C31 |
| `tests/wots_fors.rs` | 13 | C32–C44 |
| `tests/api.rs` | 9 | C45–C53 |
| `tests/rng.rs` | 7 | C54–C60 |
| `tests/errors.rs` | 52 | every row of `ERRORS.md` |
| `tests/cmake_libs.rs` | 1 | Z1 |

Two C behaviours the valid-path tests pinned down and the Rust reproduces
exactly (both are described in `ERRORS.md`): `hash_blake.c` passes byte counts to
the bit-count `blake*_update` API, and `gen_message_random` on the blake backend
writes the full 32/64-byte digest into `R` rather than `SPX_N` bytes.
