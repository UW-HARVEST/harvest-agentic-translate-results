# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Every axis below was derived by
grepping the C source for the `#if` / `#ifdef` / `if` / `switch` branches the
code actually takes, and the `#define`s the CMake cache variables feed into.

## Axis 1 — build-time: `HASH_BACKEND` (`c_src/lib/CMakeLists.txt`)

`add_subdirectory(${HASH_BACKEND})` selects one of 4 completely different
primitive families, each with its own `hash_*.c`, `thash_*.c` and
`*_offsets.h` (which *moves the address field offsets*, so even `set_*_addr`
behaves differently per backend):

| backend | primitives | `SPX_OFFSET_*` set (`*_offsets.h`) |
|---|---|---|
| `haraka` | `haraka256/512/512_perm`, `haraka_S` sponge, `tweak_constants` (AES round funcs) | `LAYER 3, TREE 8, TYPE 19, KP 20, CHAIN 27, HASH 31, TREE_HGT 27, TREE_INDEX 28` — verified per backend header |
| `sha2` | `sha256*`, `sha512*`, `mgf1_256`, `mgf1_512`, `seed_state` (precomputed midstate in `spx_ctx`) | `sha2_offsets.h`, plus `SPX_SHA256_ADDR_BYTES 22` (**compressed 22-byte address**, unlike the other 3 backends' 32) |
| `shake` | `shake256*` (Keccak) | `shake_offsets.h` |
| `blake` | `blake256*`, `blake512*`, `blake256_mgf1`, `blake512_mgf1` | `blake_offsets.h` |

## Axis 2 — build-time: `THASH` ∈ {`robust`, `simple`}

`thash_<backend>_${THASH}.c`. `robust` additionally derives an MGF1/SHAKE
`bitmask` of `inblocks * SPX_N` bytes and XORs it into the input before
hashing; `simple` concatenates directly. Different code, different output.

## Axis 3 — build-time: `SECPAR`, which sets every size constant

Read from `c_src/app/params/params-sphincs-<backend>-<secpar>.h`:

| secpar | `SPX_N` | `FULL_HEIGHT` | `D` | `TREE_HEIGHT` | `FORS_HEIGHT` | `FORS_TREES` | `WOTS_LEN` | secondary switch |
|---|---|---|---|---|---|---|---|---|
| `128s` | 16 | 63 | 7  | 9 | 12 | 14 | 35 | `SPX_SHA512 = 0`, `SPX_BLAKE512 = 0`, `SPX_N < 24` |
| `128f` | 16 | 66 | 22 | 3 | 6  | 33 | 35 | `SPX_SHA512 = 0`, `SPX_BLAKE512 = 0`, `SPX_N < 24` |
| `192s` | 24 | 63 | 7  | 9 | 14 | 17 | 51 | `SPX_SHA512 = 1`, `SPX_BLAKE512 = 1`, `SPX_N >= 24` |
| `192f` | 24 | 66 | 22 | 3 | 8  | 33 | 51 | `SPX_SHA512 = 1`, `SPX_BLAKE512 = 1`, `SPX_N >= 24` |
| `256s` | 32 | 64 | 8  | 8 | 14 | 22 | 67 | `SPX_SHA512 = 1`, `SPX_BLAKE512 = 1`, `SPX_N >= 24` |
| `256f` | 32 | 68 | 17 | 4 | 9  | 35 | 67 | `SPX_SHA512 = 1`, `SPX_BLAKE512 = 1`, `SPX_N >= 24` |

The "secondary switch" is the reason `secpar` is **not** just a size knob: for
`sha2`/`blake` it flips `thash` onto a *different hash function* for
`inblocks > 1` (`thash_512`), and flips `hash_blake.c`'s `blakeX` /
`hash_sha2.c`'s `shaX` aliases. `128s`/`128f` therefore exercise a genuinely
different code path from `192*`/`256*`.

`Cargo.toml` exposes exactly these three axes as features
(`haraka|sha2|shake|blake` with `shake256` as an alias for `shake`,
`robust|simple`, `128s|128f|192s|192f|256s|256f`), giving
**4 x 2 x 6 = 48 valid feature combinations**, all of which `cargo check`
cleanly.

## Axis 4 — runtime option/mode flags the public API can set

There is no global "options" struct; the runtime state and modes are:

| flag / state | where set | states it toggles |
|---|---|---|
| `spx_ctx.pub_seed`, `spx_ctx.sk_seed` | `crypto_sign_*`, or by hand for the low-level API | keys every hash |
| `spx_ctx.state_seeded` / `state_seeded_512` | `initialize_hash_function` → `seed_state` (**sha2 only**; a no-op for the other 3) | precomputed SHA midstate; if unset the sha2 backend produces garbage → must be driven through `initialize_hash_function` |
| `spx_ctx.tweaked512_rc64` / `tweaked256_rc32` | `initialize_hash_function` → `tweak_constants` (**haraka only**) | tweaked AES round constants |
| `addr[8]` type field | `set_type(addr, t)`, `t ∈ {WOTS 0, WOTSPK 1, HASHTREE 2, FORSTREE 3, FORSPK 4, WOTSPRF 5, FORSPRF 6}` | domain separation — 7 distinct valid values |
| `addr[8]` layer / tree / keypair / chain / hash / tree_height / tree_index | the 8 `set_*` / `copy_*` functions | address content |
| `leaf_info_x1.wots_sign_leaf == leaf_idx` vs `!=` | `merkle_sign` sets it to `idx_leaf`; `merkle_gen_root` sets it to `~0u` | `wots_gen_leafx1`: `wots_k_mask = 0` (emit signature into `wots_sig`) vs `~0u` (pk only, `wots_sig` untouched) — **two different code paths** |
| `provided_data == NULL` vs non-NULL | `AES256_CTR_DRBG_Update` | XOR-in of provided data |
| `personalization_string == NULL` vs non-NULL | `randombytes_init` | seed material |
| DRBG global `DRBG_ctx` seeded vs zero-initialised | `randombytes_init` called or not | `randombytes` output stream |

## Axis 5 — input shapes the C special-cases

| shape axis | distinguished values (from the C branches) |
|---|---|
| `mlen` (sign/verify/gen_message_random/hash_message) | `0`; `1`; `SPX_N`; one below / exactly / one above a hash block (`63/64/65` for sha256+blake256, `127/128/129` for sha512+blake512, `135/136/137` for the SHAKE256 rate, `31/32/33` for the Haraka sponge rate) ; multi-block (`1000`); the driver's `33*k` |
| `inblocks` (`thash`) | `0`; `1` (the **only** value that stays on the 256-bit path when `SPX_*512 = 1`); `2` (`compute_root`/`treehash` use this; first value that switches to `thash_512`); `SPX_WOTS_LEN`; `SPX_FORS_TREES` |
| `tree_height` (`compute_root`, `treehash`, `*_treehashx1`) | `0`; `1`; `2`; `SPX_FORS_HEIGHT`; `SPX_TREE_HEIGHT` |
| `leaf_idx` | `0`; `1` (odd → the `leaf_idx & 1` branch); `2^h - 1` (`max_idx`, the `idx < max_idx` exception path); `~0u` (`merkle_gen_root`'s sentinel) ; random in range |
| `idx_offset` | `0`; `i * (1 << SPX_FORS_HEIGHT)` (what `fors_sign` uses); odd values (the `>>= 1` path) |
| `outlen` (`ull_to_bytes`, mgf1, squeeze) | `0`; `1`; exact multiple of the block/rate; multiple + 1 (partial trailing block); `> 8` for `ull_to_bytes` |
| `inlen` (`bytes_to_ull`) | `0`; `1`; `8`; `> 8` (shift overflow) |
| incremental vs one-shot hashing | `blake*_update` in 1 chunk vs many chunks crossing the 64/128-byte buffer; `sha*_inc_blocks` with `0/1/many` blocks then `inc_finalize` with a `0`-to-`block-1` tail; `shake256_inc_absorb`/`_squeeze` in 1 vs many chunks; `haraka_S_inc_absorb`/`_squeeze` likewise |
| byte-value extremes | all-`0x00`, all-`0xFF`, and random for every keyed input (`pub_seed`, `sk_seed`, `msg`, `sig`, `addr`) — `chain_lengths`/`base_w` and the WOTS checksum are value-dependent |
| `xlen` (`randombytes`) | `0`; `15` (partial block); `16`; `17`; `48`; `1000` |
| `maxlen` / `xlen` (seedexpander) | see `ERRORS.md` rows 1-12 |

## Configuration-surface table

Every row is exercised for **all 48 build configurations** (Axes 1-3) unless the
row says otherwise. Randomised inputs: `RNG_SEED = 0xC0FFEE12345678` ChaCha-free
xorshift PRNG in `tests/common/mod.rs`, `N_ITER` per row (default 8; 3 for the
whole-signature rows because a `*s` signature takes seconds).

`entry point(s)` names the *exported* symbol(s) driven directly through
`libloading` from BOTH `.so`s.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `crypto_sign_secretkeybytes`, `crypto_sign_publickeybytes`, `crypto_sign_bytes`, `crypto_sign_seedbytes` | no inputs; the 4 size accessors must agree, and must equal `2N+PK`, `2N`, `SPX_BYTES`, `3N` for the active `secpar` | [x] |
| 2 | `SPX_ull_to_bytes` | `outlen ∈ {0,1,2,3,4,5,6,7,8,9,16}` x `in ∈ {0, 1, 0xFF, 0x0102030405060708, u64::MAX, random}` | [x] |
| 3 | `SPX_u32_to_bytes` | `in ∈ {0, 1, 0xFF, 0x01020304, u32::MAX, random}` | [x] |
| 4 | `SPX_bytes_to_ull` | `inlen ∈ {0,1,2,4,7,8,9,16}` x input bytes ∈ {all-00, all-FF, random} | [x] |
| 5 | `SPX_ull_to_bytes` ∘ `SPX_bytes_to_ull` | round-trip at `outlen = inlen ∈ {1..8}` with random values | [x] |
| 6 | `SPX_set_layer_addr` | random 32-byte `addr` x `layer ∈ {0, 1, 21, 255, 256, 0x1FF, u32::MAX, random}` | [x] |
| 7 | `SPX_set_tree_addr` | random `addr` x `tree ∈ {0, 1, 2^32, 2^63, u64::MAX, random}` (8-byte big-endian field) | [x] |
| 8 | `SPX_set_type` | random `addr` x `type ∈ {0,1,2,3,4,5,6}` (all 7 valid `SPX_ADDR_TYPE_*`) | [x] |
| 9 | `SPX_set_keypair_addr` | random `addr` x `keypair ∈ {0, 1, 0xFFFF, u32::MAX, random}` | [x] |
| 10 | `SPX_set_chain_addr` | random `addr` x `chain ∈ {0, 1, 15, 34, 66, 255, 256, u32::MAX, random}` (covers all `SPX_WOTS_LEN` values) | [x] |
| 11 | `SPX_set_hash_addr` | random `addr` x `hash ∈ {0, 1, 14, 15, 255, 256, u32::MAX, random}` (covers the WOTS chain `0..w-1`) | [x] |
| 12 | `SPX_set_tree_height` | random `addr` x `height ∈ {0, 1, 3, 9, 14, 255, 256, u32::MAX, random}` | [x] |
| 13 | `SPX_set_tree_index` | random `addr` x `index ∈ {0, 1, 2^13, u32::MAX, random}` | [x] |
| 14 | `SPX_copy_subtree_addr` | random `in`/`out` (checks exactly `SPX_OFFSET_TREE+8` bytes copied and the rest of `out` preserved — offset is backend-dependent) | [x] |
| 15 | `SPX_copy_keypair_addr` | random `in`/`out` (two disjoint `memcpy` ranges) | [x] |
| 16 | *all 8 setters composed* | apply layer→tree→type→keypair→chain→hash→tree_height→tree_index in sequence with random values; compare the full 32-byte `addr` (catches offset-aliasing between fields, e.g. `CHAIN_ADDR 27` vs `TREE_HGT 27`) | [x] |
| 17 | `SPX_initialize_hash_function` | random `pub_seed`/`sk_seed`; compare the **whole** `spx_ctx` byte image afterwards (this is the only way to check sha2's `state_seeded`/`state_seeded_512` and haraka's tweaked round-constant tables) | [x] |
| 17b | `SPX_initialize_hash_function` | ABI check: `sizeof(spx_ctx)` -- give both libraries a context buffer with a 256-byte marker tail and assert NEITHER writes past `CTX_BYTES`. Needed because the C only declares `state_seeded_512` under `# if SPX_SHA512`, so sha2/`128s`/`128f` have a **72**-byte context, not 144 | [x] |
| 18 | `SPX_prf_addr` | ctx from `initialize_hash_function` x `addr` ∈ {all-00, all-FF, random} x seeds ∈ {all-00, all-FF, random} | [x] |
| 19 | `SPX_thash` | `inblocks = 1`, random `in`, random `addr`, ctx initialised — the 256-bit path in every config | [x] |
| 20 | `SPX_thash` | `inblocks = 2` — the **`thash_512` switch** for `sha2`/`blake` at `192*`/`256*` | [x] |
| 21 | `SPX_thash` | `inblocks = SPX_WOTS_LEN` (35/51/67) — the WOTS-pk compression call | [x] |
| 22 | `SPX_thash` | `inblocks = SPX_FORS_TREES` (14/17/22/33/35) — the FORS-pk compression call | [x] |
| 23 | `SPX_thash` | `inblocks = 0` (degenerate) and `inblocks = 3` (odd, non-power-of-2) | [x] |
| 24 | `SPX_thash` | `in` = all-`0x00` and all-`0xFF` at `inblocks ∈ {1,2}` (value-extreme, exercises the `robust` bitmask XOR fully) | [x] |
| 25 | `SPX_gen_message_random` | random `sk_prf`/`optrand`, `mlen ∈ {0,1,31,32,33,63,64,65,127,128,129,135,136,137,1000}` | [x] |
| 26 | `SPX_hash_message` | random `R`/`pk`, `mlen` as in row 25; compare `digest`, `*tree` **and** `*leaf_idx` (the masking `>> (64 - SPX_TREE_BITS)` / `>> (32 - SPX_LEAF_BITS)` is config-dependent) | [x] |
| 27 | `SPX_hash_message` | `R`/`pk` = all-`0xFF` (drives `*tree`/`*leaf_idx` toward their masks) | [x] |
| 28 | `SPX_chain_lengths` | `msg` ∈ {all-`0x00`, all-`0xFF`, random} — all-`0x00` maximises and all-`0xFF` zeroes the WOTS checksum | [x] |
| 29 | `SPX_compute_root` | `tree_height ∈ {1, 2, SPX_FORS_HEIGHT, SPX_TREE_HEIGHT}` x `leaf_idx ∈ {0, 1, 2^h-1, random}` x `idx_offset ∈ {0, odd, i<<FORS_HEIGHT}`; random leaf/auth_path/addr | [x] |
| 30 | `SPX_compute_root` | `tree_height = 0` (degenerate: the `for i < tree_height-1` loop underflows to a huge count in C — **must be replicated**, so this row is asserted only for the `tree_height >= 1` cases and `0` is covered in `ERRORS.md` row 49) | [x] |
| 31 | `SPX_treehash` | with a `gen_leaf` callback supplied from the *test* (a C-ABI function pointer), `tree_height ∈ {0,1,2,3}`, `leaf_idx ∈ {0,1,2^h-1}`, `idx_offset ∈ {0, 8, odd}`; compares `root` **and** the full `auth_path` | [x] |
| 32 | `SPX_wots_gen_leafx1` | `leaf_info_x1` with `wots_sign_leaf == leaf_idx` (signature-emitting path, `wots_k_mask = 0`), random `wots_steps[0..WOTS_LEN]` in `0..w`, random `leaf_addr`/`pk_addr`; compares `dest` **and** the emitted `wots_sig` | [x] |
| 33 | `SPX_wots_gen_leafx1` | `wots_sign_leaf = !leaf_idx` (pk-only path, `wots_k_mask = ~0`), `wots_sig` left NULL as `merkle_gen_root` does | [x] |
| 34 | `SPX_wots_gen_leafx1` | `wots_steps[i] = 0` for all `i`, and `= SPX_WOTS_W - 1` for all `i` (chain-length extremes) | [x] |
| 35 | `SPX_wots_pk_from_sig` | random `sig` (`WOTS_BYTES`), `msg` ∈ {all-00, all-FF, random}, random `addr`; also asserts `addr` is left mutated identically (the C writes into the caller's `addr`) | [x] |
| 36 | `SPX_wots_treehashx1` | `tree_height = SPX_TREE_HEIGHT`, `leaf_idx ∈ {0, 1, 2^TH-1, ~0u}`, `idx_offset = 0`; full `leaf_info_x1`; compares `root`, `auth_path` and the emitted `wots_sig` | [x] |
| 37 | `SPX_fors_gen_leafx1` | `fors_gen_leaf_info` (32-byte `leaf_addrx`) x `addr_idx ∈ {0, 1, 2^FORS_HEIGHT-1, random}` | [x] |
| 38 | `SPX_fors_treehashx1` | `tree_height = SPX_FORS_HEIGHT`, `leaf_idx ∈ {0,1,2^h-1,random}`, `idx_offset ∈ {0, i<<FORS_HEIGHT}`, `info` = the 32-byte `fors_gen_leaf_info` the C actually passes; compares `root` + `auth_path` | [x] |
| 39 | `SPX_fors_sign` | random `m` (`SPX_FORS_MSG_BYTES`), random `fors_addr`; compares the full `sig` (`SPX_FORS_BYTES`) and `pk` | [x] |
| 40 | `SPX_fors_sign` | `m` = all-`0x00` (all indices 0) and all-`0xFF` (all indices `2^h-1`) — the extremes of `message_to_indices` | [x] |
| 41 | `SPX_fors_pk_from_sig` | fed the `sig` produced by row 39, plus an independently random `sig`; compares `pk` | [x] |
| 42 | `SPX_fors_sign` → `SPX_fors_pk_from_sig` | composed: the recovered `pk` must equal the signing `pk`, identically in C and Rust | [x] |
| 43 | `SPX_merkle_sign` | random `wots_addr`/`tree_addr`/`root`, `idx_leaf ∈ {0, 1, 2^TREE_HEIGHT-1, random}`; compares `sig` (`WOTS_BYTES + TREE_HEIGHT*N`), the updated `root`, and the mutated `tree_addr` | [x] |
| 44 | `SPX_merkle_sign` | `idx_leaf = ~0u` (the `merkle_gen_root` sentinel: no leaf matches, so no WOTS signature is emitted) | [x] |
| 45 | `SPX_merkle_gen_root` | ctx from random seeds via `initialize_hash_function`; compares `root` | [x] |
| 46 | `crypto_sign_seed_keypair` | seed ∈ {all-`0x00`, all-`0xFF`, random x N}; compares `pk` (2N) and `sk` (4N) | [x] |
| 47 | `crypto_sign_keypair` | after `randombytes_init(0..47, NULL)` on both sides so the DRBG streams match; compares `pk`/`sk` and the resulting `DRBG_ctx` | [x] |
| 48 | `crypto_sign_signature` | keypair from row 46; `mlen ∈ {0, 1, 32, 33, 64, 136, 1000}`; both sides `randombytes_init`-seeded identically so `optrand` matches; compares `*siglen` and the full `sig` | [x] |
| 49 | `crypto_sign_verify` | the `sig` from row 48 with `siglen = SPX_BYTES` → both return `0` | [x] |
| 50 | `crypto_sign_verify` | cross-check: verify the **C-produced** signature with the Rust `.so` and vice-versa → both `0` | [x] |
| 51 | `crypto_sign` | `mlen ∈ {0, 1, 33, 231}`; compares `*smlen` and the whole `sm` buffer | [x] |
| 52 | `crypto_sign_open` | on the `sm` from row 51; compares return, `*mlen`, and the recovered `m` | [x] |
| 53 | `crypto_sign` → `crypto_sign_open` | composed round-trip, C-signed/Rust-opened and Rust-signed/C-opened | [x] |
| 54 | `randombytes_init` + `randombytes` | `entropy_input = 0..47`, `ps = NULL`; then `randombytes` with `xlen ∈ {1, 15, 16, 17, 33, 48, 1000}` in sequence; compares every output block **and** the exported `DRBG_ctx` image after each call | [x] |
| 55 | `randombytes_init` + `randombytes` | `ps` non-NULL (48 random bytes) | [x] |
| 56 | `AES256_ECB` | `key`/`ctr` ∈ {all-00, all-FF, FIPS-197 vector, random x N} | [x] |
| 57 | `AES256_CTR_DRBG_Update` | `provided_data ∈ {NULL, 48 random bytes}` x `Key`/`V` ∈ {all-00, all-FF, random}; compares the updated `Key` and `V` | [x] |
| 58 | `seedexpander_init` + `seedexpander` | `maxlen ∈ {16, 17, 4096, 0xFFFFFFFF}` x request sequence `{1, 15, 16, 17, 33}`; compares output and the full `AES_XOF_struct` image after each call | [x] |
| 59 | `blake256_init/update/final`, `blake256` | one-shot vs incremental in chunks of `{1, 7, 63, 64, 65}` bytes; `inlen ∈ {0,1,55,56,63,64,65,128,1000}`; compares the 32-byte digest and the full `blakestate256` between steps *(blake only)* | [x] |
| 60 | `blake512_init/update/final`, `blake512` | same shape axis at the 128-byte block: `inlen ∈ {0,1,110,111,112,127,128,129,1000}` *(blake only)* | [x] |
| 61 | `blake256_compress`, `blake512_compress` | raw compression on a random state + random block *(blake only)* | [x] |
| 62 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1` | `outlen ∈ {0,1,31,32,33,64,65,100}` x `inlen ∈ {0,1,32,64,100}` *(blake only)* | [x] |
| 63 | `sha256_inc_init/inc_blocks/inc_finalize`, `sha256` | `inblocks ∈ {0,1,2,5}` then `inc_finalize` tail `∈ {0,1,55,56,63}`; one-shot `sha256` with `inlen ∈ {0,1,55,56,63,64,65,1000}`; compares the 40-byte state image *(sha2 only)* | [x] |
| 64 | `sha512_inc_init/inc_blocks/inc_finalize`, `sha512` | same at the 128-byte block, tail `∈ {0,1,111,112,127}`; 72-byte state image *(sha2 only)* | [x] |
| 65 | `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen`/`inlen` grid as row 62 *(sha2 only)* | [x] |
| 66 | `SPX_seed_state` | random `pub_seed`; compares the whole `spx_ctx` including `state_seeded`(40) and `state_seeded_512`(72) *(sha2 only)* | [x] |
| 67 | `shake256_inc_init/inc_absorb/inc_finalize/inc_squeeze` | absorb in `1` vs `{1,7,135,136,137}`-byte chunks; squeeze in `1` vs multi chunks; `outlen ∈ {0,1,135,136,137,272,300}`; compares the 26-`u64` state image *(shake only)* | [x] |
| 68 | `shake256_absorb` + `shake256_squeezeblocks`, `shake256` | `nblocks ∈ {0,1,2,3}`; one-shot `inlen ∈ {0,1,135,136,137,1000}` x `outlen` as row 67 *(shake only)* | [x] |
| 69 | `SPX_tweak_constants` | random `pub_seed` x `SPX_N ∈ {16,24,32}`; compares the full `tweaked512_rc64[10][8]` + `tweaked256_rc32[10][8]` *(haraka only)* | [x] |
| 70 | `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` | tweaked ctx x `in` ∈ {all-00, all-FF, random} *(haraka only)* | [x] |
| 71 | `SPX_haraka_S_inc_init/absorb/finalize/squeeze` | absorb in `1` vs `{1,7,31,32,33}`-byte chunks; `outlen ∈ {0,1,31,32,33,64,100}`; compares the 65-byte sponge state *(haraka only)* | [x] |
| 72 | `SPX_haraka_S` | `inlen ∈ {0,1,31,32,33,1000}` x `outlen ∈ {1,32,33,100}` *(haraka only)* | [x] |
| 73 | `driver` binary | C `cbuild/<cfg>/app/driver` vs Rust `target/release/driver` — stdout compared byte-for-byte, and both against `reference_outputs.txt` | [x] |

## Phase B result

All **74** rows above pass for **all 48** feature combinations.

```
$ ./run_tests.sh                       # per (backend,thash) chunk
PASS blake,robust,{128s,128f,192s,192f,256s,256f}   (107 tests each)
PASS blake,simple,{...}                             (107 tests each)
PASS haraka,robust,{...} / haraka,simple,{...}      (105 tests each)
PASS sha2,robust,{...}   / sha2,simple,{...}        (106 tests each)
PASS shake,robust,{...}  / shake,simple,{...}       (104 tests each)
ALL CONFIGS PASSED

48 log files, 0 containing "FAILED", 5064 passing test cases total.
```

(The per-config test count differs only because `tests/backend_prims.rs` and
`tests/errors.rs` have a different number of `#[cfg]`-gated tests per backend:
blake 4+4, sha2 4+3, haraka 4+2, shake 3+2.)

Row 73 (the `driver` binary):

```
$ ./run_drivers.sh
OK <all 48 configs>   exit=0
ALL DRIVERS MATCH
```

For every configuration the Rust `driver`'s stdout is byte-identical to the C
`driver`'s **and** to the corresponding line of `reference_outputs.txt`
(48/48 configurations have a reference line; all 48 match).

### Test-harness pitfall worth recording

`target/release/libsphincs_core_det.so` and `target/release/driver` are a
*single* path shared by all 48 feature sets. Any concurrent `cargo build` for a
different `SECPAR` silently swaps the artifact and the comparison then reports
dozens of bogus "divergences" (this happened twice during this verification).
Two mitigations are in place:

* `run_tests.sh` / `run_drivers.sh` copy the artifact to a per-configuration
  path (`gen/so/lib-<cfg>.so`, `gen/bin/driver-<cfg>`) and point the harness at
  it via `SPHINCS_RUST_SO`, each using its own `CARGO_TARGET_DIR`;
* `tests/common/mod.rs::libs()` calls `crypto_sign_bytes()` on **both** `.so`s at
  load time and hard-fails if it does not equal the `SPX_BYTES` of the
  configuration the test binary was compiled for.
