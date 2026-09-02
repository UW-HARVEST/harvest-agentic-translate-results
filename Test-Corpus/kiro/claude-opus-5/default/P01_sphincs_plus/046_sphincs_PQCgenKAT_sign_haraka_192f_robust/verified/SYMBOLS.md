# SYMBOLS.md — dynamic-symbol parity, C reference vs. Rust `cdylib`

Derived mechanically from `nm -D --defined-only` on the built shared objects.

## How the two builds map onto each other

`c_src` produces **three** shared objects per configuration; the Rust crate
produces **one** `cdylib` that contains all of them:

| C shared object | sources | Rust equivalent |
|---|---|---|
| `app/libsphincs_core.so` | `address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c` + `randombytes.c` | `src/{address,fors,merkle,sign,utils,utilsx1,wots,wotsx1,randombytes}.rs` |
| `app/libsphincs_core_det.so` | same core objects + `rng.c` | same, plus `src/rng.rs` |
| `lib/<backend>/lib<backend>.so` | backend `hash_*.c`, `thash_*_<THASH>.c`, primitives, and (blake/sha2) a second copy of `app/src/utils.c` | `src/backend/<backend>/*` + `src/{hash,thash}.rs` |

The required relation is therefore
`symbols(C core) ∪ symbols(C core_det) ∪ symbols(C backend) ⊆ symbols(Rust cdylib)`.
`randombytes` is the one name both C cores define, so exactly one of the two
bodies can be linked at a time; the Rust `urandom` feature selects
`randombytes.c` semantics and the default selects `rng.c` semantics, matching
which CMake target the `driver` links.

## Verification command

`./symdiff_all.sh` builds the Rust `cdylib` for all 96
`(HASH_BACKEND, THASH, SECPAR, randombytes-provider)` combinations, computes the
C union above for the matching `cbuild/<combo>/` directory (using
`libsphincs_core_det.so` for the DRBG builds and `libsphincs_core.so` for the
`urandom` builds), and prints any missing name.

```
$ ./symdiff_all.sh
symbol parity check done over 96 configurations, fail=0
```

`nm -D -u` on the Rust `.so` lists only `libc`/`libgcc_s` imports
(`memcpy`, `malloc`, `abort`, `_Unwind_*`, …) — zero unresolved project symbols,
checked across all 96 artifacts.

## Symbols present in every configuration (core, 43 names)

`app/libsphincs_core.so` ∪ `app/libsphincs_core_det.so`:

| # | symbol | kind | C source | Rust export site | status |
|---|--------|------|----------|------------------|--------|
| 1 | `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 2 | `crypto_sign_publickeybytes` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 3 | `crypto_sign_bytes` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 4 | `crypto_sign_seedbytes` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 5 | `crypto_sign_seed_keypair` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 6 | `crypto_sign_keypair` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 7 | `crypto_sign_signature` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 8 | `crypto_sign_verify` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 9 | `crypto_sign` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 10 | `crypto_sign_open` | T | `app/src/sign.c` | `src/sign.rs` | ok |
| 11 | `SPX_set_layer_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 12 | `SPX_set_tree_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 13 | `SPX_set_type` | T | `app/src/address.c` | `src/address.rs` | ok |
| 14 | `SPX_copy_subtree_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 15 | `SPX_set_keypair_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 16 | `SPX_copy_keypair_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 17 | `SPX_set_chain_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 18 | `SPX_set_hash_addr` | T | `app/src/address.c` | `src/address.rs` | ok |
| 19 | `SPX_set_tree_height` | T | `app/src/address.c` | `src/address.rs` | ok |
| 20 | `SPX_set_tree_index` | T | `app/src/address.c` | `src/address.rs` | ok |
| 21 | `SPX_ull_to_bytes` | T | `app/src/utils.c` | `src/utils.rs` | ok |
| 22 | `SPX_u32_to_bytes` | T | `app/src/utils.c` | `src/utils.rs` | ok |
| 23 | `SPX_bytes_to_ull` | T | `app/src/utils.c` | `src/utils.rs` | ok |
| 24 | `SPX_compute_root` | T | `app/src/utils.c` | `src/utils.rs` | ok |
| 25 | `SPX_treehash` | T | `app/src/utils.c` | `src/utils.rs` | ok |
| 26 | `SPX_wots_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` | ok |
| 27 | `SPX_fors_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` | ok |
| 28 | `SPX_chain_lengths` | T | `app/src/wots.c` | `src/wots.rs` | ok |
| 29 | `SPX_wots_pk_from_sig` | T | `app/src/wots.c` | `src/wots.rs` | ok |
| 30 | `SPX_wots_gen_leafx1` | T | `app/src/wotsx1.c` | `src/wotsx1.rs` | ok |
| 31 | `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | `src/fors.rs` | ok |
| 32 | `SPX_fors_sign` | T | `app/src/fors.c` | `src/fors.rs` | ok |
| 33 | `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | `src/fors.rs` | ok |
| 34 | `SPX_merkle_sign` | T | `app/src/merkle.c` | `src/merkle.rs` | ok |
| 35 | `SPX_merkle_gen_root` | T | `app/src/merkle.c` | `src/merkle.rs` | ok |
| 36 | `randombytes` | T | `app/src/rng.c` (det) / `app/src/randombytes.c` (urandom) | `src/rng.rs` / `src/randombytes.rs` | ok |
| 37 | `randombytes_init` | T | `app/src/rng.c` | `src/rng.rs` | ok |
| 38 | `seedexpander_init` | T | `app/src/rng.c` | `src/rng.rs` | ok |
| 39 | `seedexpander` | T | `app/src/rng.c` | `src/rng.rs` | ok |
| 40 | `AES256_ECB` | T | `app/src/rng.c` | `src/rng.rs` | ok |
| 41 | `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | `src/rng.rs` | ok |
| 42 | `DRBG_ctx` | B | `app/src/rng.c` | `src/rng.rs` | ok |
| 43 | `SPX_thash` | T | backend `thash_*_<THASH>.c` | `src/backend/*/thash_*.rs` | ok |

## Backend-specific symbols

### `HASH_BACKEND=blake` (22 names in `libblake.so`)

| # | symbol | kind | C source | Rust export site | status |
|---|--------|------|----------|------------------|--------|
| 44 | `SPX_initialize_hash_function` | T | `lib/blake/src/hash_blake.c` | `src/backend/blake/hash.rs` | ok |
| 45 | `SPX_prf_addr` | T | `lib/blake/src/hash_blake.c` | `src/backend/blake/hash.rs` | ok |
| 46 | `SPX_gen_message_random` | T | `lib/blake/src/hash_blake.c` | `src/backend/blake/hash.rs` | ok |
| 47 | `SPX_hash_message` | T | `lib/blake/src/hash_blake.c` | `src/backend/blake/hash.rs` | ok |
| 48 | `blake256_init` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 49 | `blake256_update` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 50 | `blake256_final` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 51 | `blake256_compress` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 52 | `blake256` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 53 | `SPX_blake256_mgf1` | T | `lib/blake/src/blake256.c` | `src/backend/blake/blake256.rs` | ok |
| 54 | `blake512_init` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 55 | `blake512_update` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 56 | `blake512_final` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 57 | `blake512_compress` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 58 | `blake512` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 59 | `SPX_blake512_mgf1` | T | `lib/blake/src/blake512.c` | `src/backend/blake/blake512.rs` | ok |
| 60 | `cst` | R | `lib/blake/src/blake512.c` (`const u64 cst[16]`, no `static`) | `src/backend/blake/blake512.rs` | **FIXED** — was missing; added `#[unsafe(no_mangle)] pub static cst` |

(`SPX_bytes_to_ull`, `SPX_u32_to_bytes`, `SPX_ull_to_bytes`, `SPX_compute_root`,
`SPX_treehash` also appear in `libblake.so` because `lib/blake/CMakeLists.txt`
compiles `app/src/utils.c` a second time; they are rows 21–25.)

### `HASH_BACKEND=haraka` (14 names in `libharaka.so`)

| # | symbol | kind | C source | Rust export site | status |
|---|--------|------|----------|------------------|--------|
| 61 | `SPX_initialize_hash_function` | T | `lib/haraka/src/hash_haraka.c` | `src/backend/haraka/hash.rs` | ok |
| 62 | `SPX_prf_addr` | T | `lib/haraka/src/hash_haraka.c` | `src/backend/haraka/hash.rs` | ok |
| 63 | `SPX_gen_message_random` | T | `lib/haraka/src/hash_haraka.c` | `src/backend/haraka/hash.rs` | ok |
| 64 | `SPX_hash_message` | T | `lib/haraka/src/hash_haraka.c` | `src/backend/haraka/hash.rs` | ok |
| 65 | `SPX_tweak_constants` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 66 | `SPX_haraka_S_inc_init` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 67 | `SPX_haraka_S_inc_absorb` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 68 | `SPX_haraka_S_inc_finalize` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 69 | `SPX_haraka_S_inc_squeeze` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 70 | `SPX_haraka_S` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 71 | `SPX_haraka512_perm` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 72 | `SPX_haraka512` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |
| 73 | `SPX_haraka256` | T | `lib/haraka/src/haraka.c` | `src/backend/haraka/haraka.rs` | ok |

### `HASH_BACKEND=sha2` (21 names in `libsha2.so`)

| # | symbol | kind | C source | Rust export site | status |
|---|--------|------|----------|------------------|--------|
| 74 | `SPX_initialize_hash_function` | T | `lib/sha2/src/hash_sha2.c` | `src/backend/sha2/hash.rs` | ok |
| 75 | `SPX_prf_addr` | T | `lib/sha2/src/hash_sha2.c` | `src/backend/sha2/hash.rs` | ok |
| 76 | `SPX_gen_message_random` | T | `lib/sha2/src/hash_sha2.c` | `src/backend/sha2/hash.rs` | ok |
| 77 | `SPX_hash_message` | T | `lib/sha2/src/hash_sha2.c` | `src/backend/sha2/hash.rs` | ok |
| 78 | `SPX_seed_state` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 79 | `SPX_mgf1_256` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 80 | `SPX_mgf1_512` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 81 | `sha256_inc_init` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 82 | `sha256_inc_blocks` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 83 | `sha256_inc_finalize` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 84 | `sha256` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 85 | `sha512_inc_init` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 86 | `sha512_inc_blocks` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 87 | `sha512_inc_finalize` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |
| 88 | `sha512` | T | `lib/sha2/src/sha2.c` | `src/backend/sha2/sha2.rs` | ok |

(plus rows 21–25 again, from the second compile of `app/src/utils.c`.)

### `HASH_BACKEND=shake` (12 names in `libshake.so`)

| # | symbol | kind | C source | Rust export site | status |
|---|--------|------|----------|------------------|--------|
| 89 | `SPX_initialize_hash_function` | T | `lib/shake/src/hash_shake.c` | `src/backend/shake/hash.rs` | ok |
| 90 | `SPX_prf_addr` | T | `lib/shake/src/hash_shake.c` | `src/backend/shake/hash.rs` | ok |
| 91 | `SPX_gen_message_random` | T | `lib/shake/src/hash_shake.c` | `src/backend/shake/hash.rs` | ok |
| 92 | `SPX_hash_message` | T | `lib/shake/src/hash_shake.c` | `src/backend/shake/hash.rs` | ok |
| 93 | `shake256_absorb` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 94 | `shake256_squeezeblocks` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 95 | `shake256_inc_init` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 96 | `shake256_inc_absorb` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 97 | `shake256_inc_finalize` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 98 | `shake256_inc_squeeze` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |
| 99 | `shake256` | T | `lib/shake/src/fips202.c` | `src/backend/shake/fips202.rs` | ok |

Note: `lib/shake/src/fips202.c` also *defines* `shake128*`, `sha3_256*` and
`sha3_512*`, but at `-O3` clang eliminates them because nothing in the shared
library references them, so they do not appear in `nm -D` and are therefore not
required of the Rust build. `shake256_absorb`/`shake256_squeezeblocks` do survive
and are exported by both.

## Result

`comm -23 <C union> <Rust>` is **empty for all 96 feature combinations**
(`./symdiff_all.sh` exit 0). The only defect found was `cst`, now fixed.
No stubs were introduced; every symbol is backed by a real translation.

## Dead files (not part of the compiled surface)

`src/blake/`, `src/haraka/`, `src/sha2/`, `src/shake/` and `src/tree.rs` are
earlier duplicates of `src/backend/*`. Neither `src/lib.rs` nor `src/main.rs`
declares them, so they are not compiled and contribute no symbols; the exported
implementations all come from `src/backend/<backend>/`. They were left in place
(removing them is out of scope for verification) but should not be mistaken for
the live translation.
