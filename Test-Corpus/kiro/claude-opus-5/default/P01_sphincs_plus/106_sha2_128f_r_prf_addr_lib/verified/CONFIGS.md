# CONFIGS.md — configuration-surface table

The mirror of `ERRORS.md` for **valid** inputs. Rows were derived from the axes
the C code actually branches on, found by reading the public headers and
grepping the `if` / `switch` / `#if` / `#ifdef` conditions in `c_src`.

## Axis 1 — build-time configuration (the CMake cache variables)

`c_src/CMakeLists.txt` exposes three cache variables; `c_src/lib/CMakeLists.txt`
does `add_subdirectory(${HASH_BACKEND})`, so exactly one backend is ever
compiled, and `app/CMakeLists.txt` turns `SECPAR` into
`-DPARAMS=sphincs-<backend>-<secpar>`.

| variable | values | count |
|---|---|---|
| `HASH_BACKEND` | `blake`, `sha2`, `shake`, `haraka` | 4 |
| `THASH` | `robust`, `simple` | 2 |
| `SECPAR` | `128s`, `128f`, `192s`, `192f`, `256s`, `256f` | 6 |

**48 combinations.** The Cargo features have the same names
(`shake256` is an accepted alias for `shake`, per the CMake cache docstring, so
`cargo check` covers 60 spellings of the same 48 configurations). Every row in
this table is executed under all 48.

### Derived build-time branches (not free axes — implied by the above)

| derived flag | condition | what it switches |
|---|---|---|
| `SPX_N` | 16 / 24 / 32 | every buffer size |
| `X512` (`SPX_SHA512`, `SPX_BLAKE512`) | `1` iff `SPX_N >= 24` **and** backend ∈ {`sha2`, `blake`} | `thash`'s `if (inblocks > 1) thash_512(...)` early-out, and the `shaX`/`blakeX` alias used by `gen_message_random` + `hash_message` |
| address layout | `sha2` → 22-byte compressed (`SPX_OFFSET_LAYER=0`, `TREE=1`, `TYPE=9`, …); others → 32-byte full (`LAYER=3`, `TREE=8`, `TYPE=19`, …) | every `set_*_addr` byte position |
| `spx_ctx` size | `blake`/`shake`: `2N`; `sha2`: `2N+40` (`+72` if `X512`); `haraka`: `2N+960` | the context the tests pass across FFI (`sizeof_spx_ctx` from `harness/dump_params.c`) |
| `SPX_D == 1` | never in any shipped parameter set (`D` ∈ 7…22) | `hash_message`'s `if (SPX_D == 1) *tree = 0;` — the `else` branch is always taken; recorded here so the dead branch is not mistaken for untested coverage |
| `SPX_WOTS_W` | `16` in every shipped set (`SPX_WOTS_LOGW = 4`) | `base_w`; the `#if SPX_WOTS_W == 256` arm is unreachable |

## Axis 2 — public entry points

The full exported API, lowest level first. Convenience wrappers are marked; the
table below drives the low-level ones **directly**, not only through the
wrappers.

| group | entry points | level |
|---|---|---|
| address | `SPX_set_layer_addr`, `SPX_set_tree_addr`, `SPX_set_type`, `SPX_set_keypair_addr`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height`, `SPX_set_tree_index`, `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | leaf |
| utils | `SPX_ull_to_bytes`, `SPX_u32_to_bytes`, `SPX_bytes_to_ull` | leaf |
| backend primitives | blake: `blake256{,_init,_update,_final,_compress}`, `blake512{…}`, `SPX_blake256_mgf1`, `SPX_blake512_mgf1`, `cst`; sha2: `sha256{,_inc_init,_inc_blocks,_inc_finalize}`, `sha512{…}`, `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`; shake: `shake256`, `shake256_absorb`, `shake256_squeezeblocks`, `shake256_inc_{init,absorb,finalize,squeeze}`; haraka: `SPX_tweak_constants`, `SPX_haraka256`, `SPX_haraka512`, `SPX_haraka512_perm`, `SPX_haraka_S`, `SPX_haraka_S_inc_{init,absorb,finalize,squeeze}` | leaf |
| DRBG | `AES256_ECB`, `AES256_CTR_DRBG_Update`, `DRBG_ctx`, `randombytes_init`, `randombytes`, `seedexpander_init`, `seedexpander` | leaf |
| hash hooks | `SPX_initialize_hash_function`, `SPX_prf_addr`, `SPX_thash`, `SPX_gen_message_random`, `SPX_hash_message` | mid |
| tree | `SPX_compute_root`, `SPX_treehash` (takes a `gen_leaf` **function pointer**), `SPX_wots_treehashx1`, `SPX_fors_treehashx1` | mid |
| WOTS / FORS | `SPX_chain_lengths`, `SPX_wots_pk_from_sig`, `SPX_wots_gen_leafx1`, `SPX_fors_sign`, `SPX_fors_pk_from_sig`, `SPX_fors_gen_leafx1` | mid |
| Merkle | `SPX_merkle_sign`, `SPX_merkle_gen_root` | high |
| API | `crypto_sign_{secretkeybytes,publickeybytes,bytes,seedbytes}`, `crypto_sign_seed_keypair`, `crypto_sign_keypair`, `crypto_sign_signature`, `crypto_sign_verify` | high |
| API (convenience wrappers over the above) | `crypto_sign`, `crypto_sign_open` | wrapper |

## Axis 3 — runtime options / input shapes the C special-cases

| axis | values the C distinguishes | where |
|---|---|---|
| `inblocks` (thash) | `0`, `1`, `2` (+`>1` → 512-bit path), `SPX_WOTS_LEN`, `SPX_FORS_TREES` | `thash_*.c` |
| `mlen` | `0`; `< BLOCK-N`; `== BLOCK-N` (branch flip); `> BLOCK-N`; multi-block | `hash_sha2.c:91`, `:154` |
| `leaf_idx` parity | even → left child, odd → right child | `utils.c:57`, `:74` |
| `idx_offset` | `0` vs non-zero vs odd (it is `>>= 1` in lockstep) | `utils.c:69` |
| `leaf_idx == ~0u` | "don't generate an auth path" sentinel from `merkle_gen_root` | `merkle.c:57` |
| `wots_sign_leaf == leaf_idx` | `wots_k_mask = 0` (emit signature) vs `~0` (public key only) | `wotsx1.c:27` |
| `idx == max_idx` | keep climbing past a left child at the right edge | `utilsx1.c:76` |
| `start + steps` vs `SPX_WOTS_W` | chain clamped at `W` | `wots.c:35` |
| `buflen` (BLAKE) | `< 440`, `== 440`, `> 440` bits; `== 0` → `nullt = 1` | `blake256.c:346` |
| `outlen % OUTPUT_BYTES` | `0` → tail branch skipped; `!= 0` → partial tail | `blake256.c:388`, `sha2.c` `mgf1_*` |
| `personalization_string` | `NULL` vs non-`NULL` | `rng.c:150` |
| `provided_data` | `NULL` vs non-`NULL` | `rng.c:186` |
| `V` / `ctr` carry | byte `0xff` → propagate to the next-higher byte | `rng.c:{83,157,172}` |
| `buffer_pos` | `16` (empty) vs partial vs exactly consumed | `rng.c:71` |
| `siglen` / `smlen` | see `ERRORS.md` rows 1, 3 | `sign.c:180`, `:272` |

## The table

One row per meaningful combination. Each row is exercised with **many
randomized inputs** (fixed seeds, so failures reproduce) rather than one
hand-picked value. `[x]` = passing in all 48 configurations.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| 0 | *(harness)* | Rust `.so` opened `RTLD_NOW\|RTLD_LOCAL` before the C libraries go global — assert no symbol interposition, i.e. the Rust outputs are unchanged whether or not the C `.so`s are loaded | `cfg00_no_symbol_interposition` | [x] |
| 1 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no input; also cross-checked against the C-preprocessor dump | `cfg01_size_getters` | [x] |
| 2 | `SPX_set_layer_addr` | addr = zeros / random / all-`0xFF`; `layer` ∈ {0, 1, `D-1`, 255, 256, `0xFFFFFFFF`, 256 random `u32`}; assert the other 31 bytes are untouched | `cfg02_set_layer_addr` | [x] |
| 3 | `SPX_set_tree_addr` | addr = zeros / random; `tree` ∈ {0, 1, `2^(TREE_HEIGHT*(D-1))-1`, `u64::MAX`, 256 random `u64`} | `cfg03_set_tree_addr` | [x] |
| 4 | `SPX_set_type` | `type` ∈ {0…6 (all valid variants), 7, 255, 256, 259, `0xFFFFFFFF`, 256 random} × addr {zeros, random} | `cfg04_set_type_all_variants_and_beyond` | [x] |
| 5 | `SPX_set_keypair_addr`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height`, `SPX_set_tree_index` | each with boundary + 256 random `u32`, addr random; byte-field vs 4-byte-field layouts differ between `sha2` and the rest | `cfg05_other_addr_setters` | [x] |
| 6 | `SPX_copy_subtree_addr`, `SPX_copy_keypair_addr` | random `in`, random pre-filled `out`; verifies exactly `SPX_OFFSET_TREE+8` bytes (and, for `copy_keypair_addr`, the extra 4 at `SPX_OFFSET_KP_ADDR`) are copied and nothing else | `cfg06_addr_copiers` | [x] |
| 7 | `SPX_ull_to_bytes` | `outlen` ∈ {0,1,2,3,4,7,8,9,16,32} × `in` ∈ {0, 1, `u64::MAX`, 64 random}; guard bytes after the buffer | `cfg07_ull_to_bytes` | [x] |
| 8 | `SPX_u32_to_bytes` | `in` ∈ {0, 1, `0xFFFFFFFF`, 256 random} | `cfg08_u32_to_bytes` | [x] |
| 9 | `SPX_bytes_to_ull` | `inlen` ∈ {0…8} × 64 random inputs (per `ERRORS.md` row 31, `inlen > 8` is C UB and is excluded from byte-equality) | `cfg09_bytes_to_ull` | [x] |
| 10 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; full `sizeof(spx_ctx)` byte compare. This is the row that covers `sha2`'s `seed_state` midstate precomputation and `haraka`'s `tweak_constants`; for `blake`/`shake` it is a no-op and the row asserts the context is unmodified | `cfg10_initialize_hash_function` | [x] |
| 11 | `SPX_prf_addr` | 200 random (ctx, addr) pairs, ctx built through `initialize_hash_function` | `cfg11_prf_addr` | [x] |
| 12 | `SPX_thash` | `inblocks` ∈ {0, 1, 2, 3, 4, 16, `SPX_WOTS_LEN`, `SPX_FORS_TREES`, 64} × random in/ctx/addr, 24 random inputs each. Covers both `THASH` variants and, for `X512=1`, both sides of `if (inblocks > 1)` and haraka's `inblocks == 1` F-function vs sponge split | `cfg12_thash_all_inblocks` | [x] |
| 13 | `SPX_gen_message_random` | `mlen` ∈ {0, 1, `N-1`, `N`, `N+1`, `BLOCK-N-1`, `BLOCK-N`, `BLOCK-N+1`, `BLOCK`, `BLOCK+1`, `2*BLOCK`, 1000, 5000} for `BLOCK` ∈ {64, 128} × random `sk_prf`/`optrand`/ctx. The output buffer is sized for the largest possible write and compared in full, because the BLAKE backend writes the whole digest into `R` rather than `SPX_N` bytes | `cfg13_gen_message_random` | [x] |
| 14 | `SPX_hash_message` | same `mlen` set, plus the `sha2`-specific boundary `INBLOCKS*BLOCK - N - PK_BYTES`; asserts `digest`, `*tree` **and** `*leaf_idx` (the masking at the end of `hash_message` is easy to get wrong) | `cfg14_hash_message` | [x] |
| 15 | `SPX_compute_root` | `tree_height` ∈ {1, 2, 3, `FORS_HEIGHT`, `TREE_HEIGHT`} × `leaf_idx` ∈ {even, odd, `2^h-1`, random} × `idx_offset` ∈ {0, 1, `0xFFFFFFFE`, random} × random leaf/auth_path/ctx/addr; also asserts `addr` is left in the same state by both | `cfg15_compute_root_shapes` | [x] |
| 16 | `SPX_treehash` | driven through a **neutral `extern "C"` `gen_leaf` callback defined in the test** (so both libraries generate identical leaves and only their own `thash`/traversal differs); `tree_height` ∈ {1, 2, 3, `FORS_HEIGHT` capped at 10} × `leaf_idx` ∈ {0, mid, last, `~0u`} × `idx_offset` ∈ {0, `k*2^h`, random}; asserts root, the whole auth path, and the final `tree_addr` | `cfg16_treehash_shapes` | [x] |
| 17 | `SPX_wots_treehashx1` | `tree_height = TREE_HEIGHT` × `leaf_idx` ∈ {0, 1, mid, `2^h-1`, `~0u`} × `idx_offset` ∈ {0, random} × `leaf_info_x1.wots_sign_leaf` = `leaf_idx` (signature path, `wots_k_mask = 0`) **and** `~0u` (pk-only path); asserts root, auth path, the emitted `wots_sig`, and the mutated `leaf_info_x1` | `cfg17_wots_treehashx1` | [x] |
| 18 | `SPX_fors_treehashx1` | `tree_height = FORS_HEIGHT` (capped) × `leaf_idx` ∈ {0, 1, mid, `2^h-1`} × `idx_offset` = `i*2^FORS_HEIGHT` for several `i`; asserts root, auth path and the mutated `fors_gen_leaf_info` | `cfg18_fors_treehashx1` | [x] |
| 19 | `SPX_chain_lengths` | 200 random `SPX_N`-byte messages; asserts all `SPX_WOTS_LEN` `unsigned int` outputs, incl. the `LEN2` checksum digits | `cfg19_chain_lengths` | [x] |
| 20 | `SPX_wots_pk_from_sig` | 40 random (sig, msg, addr, ctx); `msg` chosen to hit both `lengths[i] == 0` and `lengths[i] == W-1` (chain length 0 → `gen_chain` copies only) | `cfg20_wots_pk_from_sig` | [x] |
| 21 | `SPX_wots_gen_leafx1` | `leaf_idx == info.wots_sign_leaf` (emits `wots_sig`) and `leaf_idx != info.wots_sign_leaf` (`wots_k_mask = ~0`, no signature); `wots_steps` ∈ {all 0, all `W-1`, random}; asserts `dest`, `wots_sig` and the mutated `leaf_addr`/`pk_addr` | `cfg21_wots_gen_leafx1` | [x] |
| 22 | `SPX_fors_gen_leafx1` | random `addr_idx` incl. `0`, `2^FORS_HEIGHT-1`, `0xFFFFFFFF`; asserts leaf and the mutated `fors_gen_leaf_info.leaf_addrx` | `cfg22_fors_gen_leafx1` | [x] |
| 23 | `SPX_fors_sign` | 20 random (`m` of `SPX_FORS_MSG_BYTES`, ctx, `fors_addr`); asserts the full `SPX_FORS_BYTES` signature **and** the FORS pk | `cfg23_fors_sign` | [x] |
| 24 | `SPX_fors_pk_from_sig` | 20 random (sig, m, ctx, addr) — deliberately *not* only well-formed signatures, since the function has no validity check | `cfg24_fors_pk_from_sig` | [x] |
| 25 | `SPX_fors_sign` → `SPX_fors_pk_from_sig` | composed: sign then recover, assert both libraries agree on the recovered pk and that it equals the pk from `fors_sign` (catches pipeline bugs invisible per-wrapper) | `cfg25_fors_sign_then_recover` | [x] |
| 26 | `SPX_merkle_sign` | `idx_leaf` ∈ {0, 1, mid, `2^TREE_HEIGHT-1`, `~0u`} × random root/ctx/`wots_addr`/`tree_addr`; asserts `sig` (`WOTS_BYTES + TREE_HEIGHT*N`), the updated `root`, and both mutated addresses | `cfg26_merkle_sign` | [x] |
| 27 | `SPX_merkle_gen_root` | random ctx (built via `initialize_hash_function`) | `cfg27_merkle_gen_root` | [x] |
| 28 | `crypto_sign_seed_keypair` | 8 random `CRYPTO_SEEDBYTES` seeds + all-zero + all-`0xFF`; asserts `pk` and `sk` | `cfg28_seed_keypair` | [x] |
| 29 | `crypto_sign_keypair` | both DRBGs seeded identically via `randombytes_init`; asserts `pk`, `sk` **and** the resulting `DRBG_ctx` state | `cfg29_keypair_from_drbg` | [x] |
| 30 | `crypto_sign_signature` + `crypto_sign_verify` | `mlen` ∈ {0, 1, 2, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1000, 5000} with the DRBG re-seeded identically before each signature (it draws `optrand`); asserts signature bytes, `*siglen`, and `verify == 0` | `cfg30_signature_and_verify` | [x] |
| 31 | `crypto_sign` + `crypto_sign_open` | same `mlen` set; asserts `sm`, `*smlen`, recovered `m`, `*mlen` | `cfg31_sign_and_open` | [x] |
| 32 | cross-library verify | C-generated signature checked by Rust's `crypto_sign_verify`, and vice versa; likewise `crypto_sign_open` | `cfg30_signature_and_verify + cfg25_fors_sign_then_recover` | [x] |
| 33 | `SPX_ull_to_bytes` / `SPX_bytes_to_ull` | round-trip: `bytes_to_ull(ull_to_bytes(v, len), len)` for `len` ∈ 1…8 | `cfg33_ull_bytes_roundtrip` | [x] |
| 34 | *blake* `blake256` | one-shot, `inlen` ∈ {0, 1, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 127, 128, 129, 1000} + 32 random lengths ≤ 4096 | `cfg34_37_blake_one_shot` | [x] |
| 35 | *blake* `blake256_init` + `blake256_update`×k + `blake256_final` | incremental, split into 1…6 random chunks (`datalen` is in **bits**), covering all three `final` padding branches and `nullt` | `cfg35_37_blake_incremental` | [x] |
| 36 | *blake* `blake256_compress` | random 128-byte `blakestate256` + random 64-byte block, incl. `t[0]` near `2^32` to exercise the `t[1]++` carry | `cfg36_37_blake_compress` | [x] |
| 37 | *blake* `blake512` / `blake512_init`/`_update`/`_final`/`_compress` | same shapes with the 128-byte block and the 110/111/112-byte padding boundaries | `cfg34_37_blake_one_shot / cfg35_37 / cfg36_37` | [x] |
| 38 | *blake* `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen` ∈ {0, 1, 31, 32, 33, 63, 64, 65, 100, 256} × `inlen` ∈ {0, 1, 32, 64, random} | `cfg38_blake_mgf1` | [x] |
| 39 | *blake* `cst` | read all 16 `u64` from the exported read-only symbol and compare byte-for-byte with the C `.so`'s | `cfg39_blake_cst_global` | [x] |
| 40 | *sha2* `sha256`, `sha512` | one-shot, `inlen` ∈ {0, 1, 55, 56, 57, 63, 64, 65, 111, 112, 113, 127, 128, 129, 1000} + random | `cfg40_sha_one_shot` | [x] |
| 41 | *sha2* `sha256_inc_init`/`_inc_blocks`/`_inc_finalize` (and 512) | `inblocks` ∈ {0, 1, 2, 5} then `inc_finalize` with `inlen` ∈ {0, 1, `BLOCK-9`, `BLOCK-8`, `BLOCK-1`, `BLOCK`, `BLOCK+1`}; asserts the 40-/72-byte state after every step | `cfg41_sha_incremental` | [x] |
| 42 | *sha2* `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen`/`inlen` boundary grid as row 38 | `cfg42_sha_mgf1` | [x] |
| 43 | *sha2* `SPX_seed_state` | random `pub_seed`; asserts the whole `spx_ctx` including both midstates when `X512=1` | `cfg43_seed_state` | [x] |
| 44 | *shake* `shake256` | `outlen` ∈ {0, 1, 32, 135, 136, 137, 271, 272, 273} × `inlen` ∈ {0, 1, 135, 136, 137, 272, random} | `cfg44_shake256_one_shot` | [x] |
| 45 | *shake* `shake256_inc_init`/`_absorb`×k/`_finalize`/`_squeeze`×k | absorb split into 1…6 random chunks; squeeze split into 1…6 chunks crossing the 136-byte rate boundary; asserts the 200-byte state after each step | `cfg45_shake256_incremental` | [x] |
| 46 | *shake* `shake256_absorb` + `shake256_squeezeblocks` | `nblocks` ∈ {0, 1, 2, 5}; the non-incremental API, used by nothing in the library but exported | `cfg46_shake256_absorb_squeezeblocks` | [x] |
| 47 | *haraka* `SPX_tweak_constants` | random `pub_seed`/`sk_seed`; asserts all 1024 (or `2N+960`) context bytes, i.e. both `tweaked512_rc64` and `tweaked256_rc32` | `cfg47_tweak_constants` | [x] |
| 48 | *haraka* `SPX_haraka512`, `SPX_haraka512_perm`, `SPX_haraka256` | random 64-/32-byte inputs under a tweaked context | `cfg48_haraka_permutations` | [x] |
| 49 | *haraka* `SPX_haraka_S` | `outlen` ∈ {0, 1, 31, 32, 33, 64, 65, 200} × `inlen` ∈ {0, 1, 31, 32, 33, 64, random} | `cfg49_haraka_s_one_shot` | [x] |
| 50 | *haraka* `SPX_haraka_S_inc_init`/`_absorb`×k/`_finalize`/`_squeeze`×k | chunked absorb and squeeze across the 32-byte rate; asserts the 65-byte state after each step | `cfg50_haraka_s_incremental` | [x] |
| 51 | `AES256_ECB` | 64 random (key, ctr) pairs | `cfg51_aes256_ecb` | [x] |
| 52 | `AES256_CTR_DRBG_Update` | `provided_data` = `NULL` **and** non-`NULL`; `V` ∈ {zeros, `0xFF…FF` (full carry), `…FF` in the low bytes only, random}; asserts the updated `Key` and `V` | `cfg52_drbg_update` | [x] |
| 53 | `randombytes_init` | `personalization_string` = `NULL` and non-`NULL`; `entropy_input` random; asserts the whole `DRBG_ctx` (`Key`, `V`, `reseed_counter`) | `cfg53_randombytes_init` | [x] |
| 54 | `randombytes` | after identical `randombytes_init`: `xlen` ∈ {0, 1, 15, 16, 17, 31, 32, 48, 64, 1000} and a chain of 10 successive calls (state carries over); asserts output **and** `DRBG_ctx` after each call | `cfg54_randombytes_stream` | [x] |
| 55 | `seedexpander_init` + `seedexpander` | `maxlen` ∈ {1, 16, 17, 4096, `0xFFFFFFFF`}; then a sequence of `xlen` ∈ {0, 1, 15, 16, 17, 100} draws that crosses the 16-byte internal buffer boundary and the `ctr[12..16]` carry; asserts output and the whole 72-byte `AES_XOF_struct` after each call | `cfg55_seedexpander_stream` | [x] |
| 56 | end-to-end KAT | the C and Rust `driver` binaries (`app/src/PQCgenKAT_sign.c` vs `src/main.rs`) — 100 keygen/sign/verify rounds hashed into a SHAKE-256 transcript digest; compared per configuration | `run_tests.sh (c_driver vs rs_driver)` | [x] |

## Two C behaviours the rows had to be shaped around

`lib/blake/src/hash_blake.c` passes **byte** counts to `blake*_update`, whose
length argument is in **bits**, so the BLAKE backend absorbs only `SPX_N/8`
bytes of `R`, `SPX_PK_BYTES/8` bytes of `pk` and `mlen/8` bytes of the message
into the message hash. And `gen_message_random` for BLAKE finishes with
`blakeX_final(&S, R)`, writing the *whole* 32- or 64-byte digest into `R` rather
than `SPX_N` bytes (harmless in `sign.c`, which passes the `SPX_BYTES`-long
`sig`). Both are C behaviour, so both are required Rust behaviour; see the
corresponding note at the end of `ERRORS.md` for how rows 13 and 14 and the
error-path rows account for them.

## How to reproduce

```
./build_matrix.sh    # C .so's + Rust .so's + drivers + params.txt, all 48 combos
./symdiff.sh         # SYMBOLS.md gate: nm -D parity, all 48
./run_tests.sh       # Phases B + C + the KAT drivers, all 48
```

`run_tests.sh` runs `cargo test --release --no-default-features --features
"<backend>,<thash>,<secpar>"` for each combination and additionally asserts the
per-binary test counts (33 + 21 + 25 = 79), so a filtered-out or crashed test
binary cannot be mistaken for a pass.

## Gate

- [x] Every row above passes across its randomized inputs, in **all 48**
      configurations: `run_tests.sh` reports `combos passed: 48   failed: 0`,
      79 tests per combination, and an identical KAT transcript digest from the
      C and Rust drivers in every one.
- Rows 34–39 apply only to `HASH_BACKEND=blake`, 40–43 only to `sha2`, 44–46
  only to `shake`, 47–50 only to `haraka`. Each of those tests reads the active
  backend from `params.txt` (which comes from the C preprocessor) and returns
  early elsewhere rather than asserting anything vacuously; the remaining rows
  run in all 48.
