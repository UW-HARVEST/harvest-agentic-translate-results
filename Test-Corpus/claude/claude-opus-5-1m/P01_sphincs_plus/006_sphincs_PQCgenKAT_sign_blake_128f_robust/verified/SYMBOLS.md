# SYMBOLS.md — exported-symbol parity, C `.so` vs Rust `.so`

## How this was produced

The C project builds **three** shared libraries per configuration
(`c_src/lib/CMakeLists.txt`, `c_src/app/CMakeLists.txt`):

| C shared library | sources |
|---|---|
| `lib/<backend>/lib<backend>.so` | the hash backend + `thash_<backend>_${THASH}.c` (+ `app/src/utils.c` for `sha2`/`blake`) |
| `app/libsphincs_core_det.so`    | `address/fors/merkle/sign/utils/utilsx1/wots/wotsx1.c` + `app/src/rng.c` (NIST AES-256 CTR_DRBG) |
| `app/libsphincs_core.so`        | the same core objects + `app/src/randombytes.c` (`/dev/urandom`) |

The Rust crate builds a **single** `cdylib` (`target/release/libsphincsplus.so`)
which must export the union of all three.

The reference symbol set is the union of `nm -D --defined-only` over the three C
libraries. The Rust set is `nm -D --defined-only target/release/libsphincsplus.so`.
`./symdiff.sh` performs this diff mechanically for **all 48**
`(HASH_BACKEND, THASH, SECPAR)` combinations; `./build_c_all.sh` builds the C
side for all 48.

## Result

```
$ ./symdiff.sh
haraka:robust:128s     missing(0):
...  (48 lines, one per combination)
blake:simple:256f      missing(0):
TOTAL MISSING=0
```

**0 missing symbols in every one of the 48 configurations.** The Rust `.so` also
has no undefined non-libc/non-Rust-runtime symbols (checked in `symdiff.sh`,
column `UNDEF:` — always empty).

## Symbols that had to be added during verification

| symbol | why it was missing | fix |
|---|---|---|
| `AES256_ECB` | `rng.c` defines it non-`static`, so it is exported; the Rust translation had folded it into a private helper `aes256_ecb`. | Re-exported as `#[no_mangle] pub unsafe extern "C" fn AES256_ECB(key, ctr, buffer)` in `src/rng.rs`, and `seedexpander`/`randombytes`/`AES256_CTR_DRBG_Update` now call it (as the C does). |
| `DRBG_ctx` | `rng.c` has the global `AES256_CTR_DRBG_struct DRBG_ctx;` (a `.bss` object callers can read/write). The Rust translation used a private `Mutex<Drbg>`, so the observable global state was not exposed. | Replaced with `#[no_mangle] pub static mut DRBG_ctx: AES256_CTR_DRBG_struct` in `src/rng.rs`; `randombytes_init`/`randombytes` now operate on it in place, exactly like the C. |
| `cst` | `blake512.c` declares `const u64 cst[16]` **without** `static` (unlike `blake256.c`, where it *is* `static`), so BLAKE-512's constant table is an exported `.rodata` symbol. The Rust had it as a private `const CST`. | Renamed to `#[no_mangle] pub static cst: [u64; 16]` in `src/backends/blake/blake512.rs`. |

No symbol was stubbed: every export is backed by the real translated
implementation.

## Source-file coverage (no C file was skipped)

Every one of the 27 C source files has a Rust counterpart, so no symbol was
missing because a whole module had been skipped:

| C source | Rust |
|---|---|
| `app/src/address.c` | `src/address.rs` |
| `app/src/utils.c` | `src/utils.rs` |
| `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `app/src/wots.c` | `src/wots.rs` |
| `app/src/wotsx1.c` | `src/wotsx1.rs` |
| `app/src/fors.c` | `src/fors.rs` |
| `app/src/merkle.c` | `src/merkle.rs` |
| `app/src/sign.c` | `src/sign.rs` |
| `app/src/rng.c` | `src/rng.rs` |
| `app/src/randombytes.c` | `src/randombytes.rs` |
| `app/src/PQCgenKAT_sign.c` | `src/main.rs` |
| `app/include/context.h` | `src/context.rs` |
| `app/params/params-sphincs-*.h` + `lib/*/include/*_offsets.h` | `src/params.rs` |
| `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` |
| `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` |
| `lib/blake/src/hash_blake.c` | `src/backends/blake/hash_blake.rs` |
| `lib/blake/src/thash_blake_{robust,simple}.c` | `src/backends/blake/thash_blake.rs` |
| `lib/haraka/src/haraka.c` | `src/backends/haraka/haraka.rs` |
| `lib/haraka/src/hash_haraka.c` | `src/backends/haraka/hash_haraka.rs` |
| `lib/haraka/src/thash_haraka_{robust,simple}.c` | `src/backends/haraka/thash_haraka.rs` |
| `lib/sha2/src/sha2.c` | `src/backends/sha2/sha2.rs` |
| `lib/sha2/src/hash_sha2.c` | `src/backends/sha2/hash_sha2.rs` |
| `lib/sha2/src/thash_sha2_{robust,simple}.c` | `src/backends/sha2/thash_sha2.rs` |
| `lib/shake/src/fips202.c` | `src/backends/shake/fips202.rs` |
| `lib/shake/src/hash_shake.c` | `src/backends/shake/hash_shake.rs` |
| `lib/shake/src/thash_shake_{robust,simple}.c` | `src/backends/shake/thash_shake.rs` |

## `spx_ctx` ABI parity

`app/include/context.h` guards `state_seeded_512[72]` with `#if SPX_SHA512`
(false for `sha2-128s`/`sha2-128f`). The Rust `SpxCtx` originally carried the
field unconditionally, so `size_of::<SpxCtx>()` was 144 where C's
`sizeof(spx_ctx)` is 72. Fixed by giving the field the length
`STATE_SEEDED_512_LEN = if SPX_SHA512 { 72 } else { 0 }`. Measured
(`sizeof`/`size_of`) after the fix — exact match everywhere:

| backend | `128*` | `192*` | `256*` |
|---|---|---|---|
| `haraka` | 992 / 992 | 1008 / 1008 | 1024 / 1024 |
| `sha2`   | 72 / 72   | 160 / 160   | 176 / 176   |
| `shake`  | 32 / 32   | 48 / 48     | 64 / 64     |
| `blake`  | 32 / 32   | 48 / 48     | 64 / 64     |

(C / Rust; all field offsets also match, which is additionally checked at
run time by the `cfg_ctx_layout` differential test that seeds an `spx_ctx` with
one implementation and consumes it with the other.)

## Notes on `randombytes`

`randombytes` is defined **twice** in the C project with different semantics:

* `app/src/rng.c` → `int randombytes(unsigned char*, unsigned long long)`, the
  deterministic NIST CTR_DRBG (used by `libsphincs_core_det.so` and by the
  `driver` KAT executable);
* `app/src/randombytes.c` → `void randombytes(unsigned char*, unsigned long long)`,
  reading `/dev/urandom` (used by `libsphincs_core.so`).

A single Rust `cdylib` cannot export one name twice, so the exported
`randombytes` is the **deterministic** one (matching `libsphincs_core_det.so`,
the variant the reference `driver` links against and the only one whose output
is reproducible/testable). The `/dev/urandom` variant is still translated, in
`src/randombytes.rs`, as `randombytes_urandom`.

All differential tests therefore compare against `libsphincs_core_det.so`.

## Full symbol table (default configuration `blake` / `simple` / `128f`)

| # | symbol | kind | C .so(s) | C source | Rust module | in Rust .so |
|---|--------|------|----------|----------|-------------|-------------|
| 1 | `AES256_CTR_DRBG_Update` | T | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (T) |
| 2 | `AES256_ECB` | T | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (T) |
| 3 | `DRBG_ctx` | B | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (B) |
| 4 | `SPX_blake256_mgf1` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 5 | `SPX_blake512_mgf1` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 6 | `SPX_bytes_to_ull` | T | libblake.so, libsphincs_core.so, libsphincs_core_det.so | `app/src/utils.c` | `src/utils.rs` | YES (T) |
| 7 | `SPX_chain_lengths` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/wots.c` | `src/wots.rs` | YES (T) |
| 8 | `SPX_compute_root` | T | libblake.so, libsphincs_core.so, libsphincs_core_det.so | `app/src/utils.c` | `src/utils.rs` | YES (T) |
| 9 | `SPX_copy_keypair_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 10 | `SPX_copy_subtree_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 11 | `SPX_fors_gen_leafx1` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/fors.c` | `src/fors.rs` | YES (T) |
| 12 | `SPX_fors_pk_from_sig` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/fors.c` | `src/fors.rs` | YES (T) |
| 13 | `SPX_fors_sign` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/fors.c` | `src/fors.rs` | YES (T) |
| 14 | `SPX_fors_treehashx1` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/utilsx1.c` | `src/utilsx1.rs` | YES (T) |
| 15 | `SPX_gen_message_random` | T | libblake.so | `lib/<backend>/src/hash_<backend>.c` | `src/backends/<backend>/hash_<backend>.rs` | YES (T) |
| 16 | `SPX_hash_message` | T | libblake.so | `lib/<backend>/src/hash_<backend>.c` | `src/backends/<backend>/hash_<backend>.rs` | YES (T) |
| 17 | `SPX_initialize_hash_function` | T | libblake.so | `lib/<backend>/src/hash_<backend>.c` | `src/backends/<backend>/hash_<backend>.rs` | YES (T) |
| 18 | `SPX_merkle_gen_root` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/merkle.c` | `src/merkle.rs` | YES (T) |
| 19 | `SPX_merkle_sign` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/merkle.c` | `src/merkle.rs` | YES (T) |
| 20 | `SPX_prf_addr` | T | libblake.so | `lib/<backend>/src/hash_<backend>.c` | `src/backends/<backend>/hash_<backend>.rs` | YES (T) |
| 21 | `SPX_set_chain_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 22 | `SPX_set_hash_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 23 | `SPX_set_keypair_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 24 | `SPX_set_layer_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 25 | `SPX_set_tree_addr` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 26 | `SPX_set_tree_height` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 27 | `SPX_set_tree_index` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 28 | `SPX_set_type` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/address.c` | `src/address.rs` | YES (T) |
| 29 | `SPX_thash` | T | libblake.so | `lib/<backend>/src/thash_<backend>_{robust,simple}.c` | `src/backends/<backend>/thash_<backend>.rs` | YES (T) |
| 30 | `SPX_treehash` | T | libblake.so, libsphincs_core.so, libsphincs_core_det.so | `app/src/utils.c` | `src/utils.rs` | YES (T) |
| 31 | `SPX_u32_to_bytes` | T | libblake.so, libsphincs_core.so, libsphincs_core_det.so | `app/src/utils.c` | `src/utils.rs` | YES (T) |
| 32 | `SPX_ull_to_bytes` | T | libblake.so, libsphincs_core.so, libsphincs_core_det.so | `app/src/utils.c` | `src/utils.rs` | YES (T) |
| 33 | `SPX_wots_gen_leafx1` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/wotsx1.c` | `src/wotsx1.rs` | YES (T) |
| 34 | `SPX_wots_pk_from_sig` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/wots.c` | `src/wots.rs` | YES (T) |
| 35 | `SPX_wots_treehashx1` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/utilsx1.c` | `src/utilsx1.rs` | YES (T) |
| 36 | `blake256` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 37 | `blake256_compress` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 38 | `blake256_final` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 39 | `blake256_init` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 40 | `blake256_update` | T | libblake.so | `lib/blake/src/blake256.c` | `src/backends/blake/blake256.rs` | YES (T) |
| 41 | `blake512` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 42 | `blake512_compress` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 43 | `blake512_final` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 44 | `blake512_init` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 45 | `blake512_update` | T | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (T) |
| 46 | `crypto_sign` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 47 | `crypto_sign_bytes` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 48 | `crypto_sign_keypair` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 49 | `crypto_sign_open` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 50 | `crypto_sign_publickeybytes` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 51 | `crypto_sign_secretkeybytes` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 52 | `crypto_sign_seed_keypair` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 53 | `crypto_sign_seedbytes` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 54 | `crypto_sign_signature` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 55 | `crypto_sign_verify` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/sign.c` | `src/sign.rs` | YES (T) |
| 56 | `cst` | R | libblake.so | `lib/blake/src/blake512.c` | `src/backends/blake/blake512.rs` | YES (R) |
| 57 | `randombytes` | T | libsphincs_core.so, libsphincs_core_det.so | `app/src/rng.c (det) / app/src/randombytes.c (urandom)` | `src/rng.rs (+ src/randombytes.rs)` | YES (T) |
| 58 | `randombytes_init` | T | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (T) |
| 59 | `seedexpander` | T | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (T) |
| 60 | `seedexpander_init` | T | libsphincs_core_det.so | `app/src/rng.c` | `src/rng.rs` | YES (T) |

Total distinct C symbols: 60; present in Rust .so: 60; missing: 0

## Per-backend extra symbols

The backend library contributes different symbols per `HASH_BACKEND`; all are
present in the Rust `.so` for the matching feature (verified by `symdiff.sh`):

| backend | extra exported symbols |
|---|---|
| `haraka` | `SPX_tweak_constants`, `SPX_haraka_S_inc_init`, `SPX_haraka_S_inc_absorb`, `SPX_haraka_S_inc_finalize`, `SPX_haraka_S_inc_squeeze`, `SPX_haraka_S`, `SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` |
| `sha2`   | `sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize`, `sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize`, `SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state` |
| `shake`  | `shake256`, `shake256_absorb`, `shake256_squeezeblocks`, `shake256_inc_init`, `shake256_inc_absorb`, `shake256_inc_finalize`, `shake256_inc_squeeze` |
| `blake`  | `blake256`, `blake256_init`, `blake256_update`, `blake256_final`, `blake256_compress`, `SPX_blake256_mgf1`, `blake512`, `blake512_init`, `blake512_update`, `blake512_final`, `blake512_compress`, `SPX_blake512_mgf1`, `cst` |
