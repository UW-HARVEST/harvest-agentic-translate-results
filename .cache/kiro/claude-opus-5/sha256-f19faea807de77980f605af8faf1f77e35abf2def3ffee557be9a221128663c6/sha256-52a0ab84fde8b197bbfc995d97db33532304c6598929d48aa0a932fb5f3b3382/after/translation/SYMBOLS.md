# SYMBOLS.md — dynamic symbol parity, C reference `.so` vs Rust `cdylib`

Generated mechanically from `nm -D --defined-only` (symbol classes `T`, `D`,
`B`, `R`, `W`) over every build-time configuration, not from the headers.

## What is compared against what

`c_src/app/CMakeLists.txt` and `c_src/lib/CMakeLists.txt` split the reference
build across **two** shared objects, while the Rust crate is a single `cdylib`.
The comparison therefore uses the union of the C pair:

| build-time selection | C shared objects | Rust `.so` |
|---|---|---|
| default (`rng.c` DRBG) | `cbuild/<b>_<t>_<s>/app/libsphincs_core_det.so` + `cbuild/<b>_<t>_<s>/lib/<b>/lib<b>.so` | `rsbuild/<b>_<t>_<s>/libsphincsplus.so` |
| `urandom` (`randombytes.c`) | `cbuild/<b>_<t>_<s>/app/libsphincs_core.so` + `.../lib<b>.so` | `rsbuild/<b>_<t>_<s>_urandom/libsphincsplus.so` |

Reproduce with `./build_c_all.sh` then `./build_rust_all.sh`; the per-config
verdicts land in `symbol_parity.txt` and the raw symbol lists in
`/tmp/symlogs/<tag>.{c,rs}.txt`.

## Result

* 4 `HASH_BACKEND` x 2 `THASH` x 6 `SECPAR` x 2 randombytes providers = **96
  configurations**.
* **0 configurations with a missing symbol.** `symbol_parity.txt` contains 96
  `OK` lines and no `MISS`/`BUILDFAIL` lines.
* 87 distinct symbol names occur across the union of all configurations; the
  Rust union is the same 87 names.

## The union of exported symbols

`configs` is the number of the 96 configurations in which the **C** side
exports the symbol. `also in Rust?` is the number of configurations in which the
Rust `.so` exports it.

| # | symbol | C source | C configs | Rust configs |
|---|--------|----------|-----------|--------------|
| 1 | `crypto_sign_secretkeybytes` | `app/src/sign.c` | 96 | 96 |
| 2 | `crypto_sign_publickeybytes` | `app/src/sign.c` | 96 | 96 |
| 3 | `crypto_sign_bytes` | `app/src/sign.c` | 96 | 96 |
| 4 | `crypto_sign_seedbytes` | `app/src/sign.c` | 96 | 96 |
| 5 | `crypto_sign_seed_keypair` | `app/src/sign.c` | 96 | 96 |
| 6 | `crypto_sign_keypair` | `app/src/sign.c` | 96 | 96 |
| 7 | `crypto_sign_signature` | `app/src/sign.c` | 96 | 96 |
| 8 | `crypto_sign_verify` | `app/src/sign.c` | 96 | 96 |
| 9 | `crypto_sign` | `app/src/sign.c` | 96 | 96 |
| 10 | `crypto_sign_open` | `app/src/sign.c` | 96 | 96 |
| 11 | `SPX_set_layer_addr` | `app/src/address.c` | 96 | 96 |
| 12 | `SPX_set_tree_addr` | `app/src/address.c` | 96 | 96 |
| 13 | `SPX_set_type` | `app/src/address.c` | 96 | 96 |
| 14 | `SPX_copy_subtree_addr` | `app/src/address.c` | 96 | 96 |
| 15 | `SPX_set_keypair_addr` | `app/src/address.c` | 96 | 96 |
| 16 | `SPX_copy_keypair_addr` | `app/src/address.c` | 96 | 96 |
| 17 | `SPX_set_chain_addr` | `app/src/address.c` | 96 | 96 |
| 18 | `SPX_set_hash_addr` | `app/src/address.c` | 96 | 96 |
| 19 | `SPX_set_tree_height` | `app/src/address.c` | 96 | 96 |
| 20 | `SPX_set_tree_index` | `app/src/address.c` | 96 | 96 |
| 21 | `SPX_ull_to_bytes` | `app/src/utils.c` | 96 | 96 |
| 22 | `SPX_u32_to_bytes` | `app/src/utils.c` | 96 | 96 |
| 23 | `SPX_bytes_to_ull` | `app/src/utils.c` | 96 | 96 |
| 24 | `SPX_compute_root` | `app/src/utils.c` | 96 | 96 |
| 25 | `SPX_treehash` | `app/src/utils.c` | 96 | 96 |
| 26 | `SPX_wots_treehashx1` | `app/src/utilsx1.c` | 96 | 96 |
| 27 | `SPX_fors_treehashx1` | `app/src/utilsx1.c` | 96 | 96 |
| 28 | `SPX_chain_lengths` | `app/src/wots.c` | 96 | 96 |
| 29 | `SPX_wots_pk_from_sig` | `app/src/wots.c` | 96 | 96 |
| 30 | `SPX_wots_gen_leafx1` | `app/src/wotsx1.c` | 96 | 96 |
| 31 | `SPX_fors_gen_leafx1` | `app/src/fors.c` | 96 | 96 |
| 32 | `SPX_fors_sign` | `app/src/fors.c` | 96 | 96 |
| 33 | `SPX_fors_pk_from_sig` | `app/src/fors.c` | 96 | 96 |
| 34 | `SPX_merkle_sign` | `app/src/merkle.c` | 96 | 96 |
| 35 | `SPX_merkle_gen_root` | `app/src/merkle.c` | 96 | 96 |
| 36 | `SPX_initialize_hash_function` | `lib/<b>/src/hash_<b>.c` | 96 | 96 |
| 37 | `SPX_prf_addr` | `lib/<b>/src/hash_<b>.c` | 96 | 96 |
| 38 | `SPX_gen_message_random` | `lib/<b>/src/hash_<b>.c` | 96 | 96 |
| 39 | `SPX_hash_message` | `lib/<b>/src/hash_<b>.c` | 96 | 96 |
| 40 | `SPX_thash` | `lib/<b>/src/thash_<b>_<t>.c` | 96 | 96 |
| 41 | `randombytes` | `app/src/rng.c` **or** `app/src/randombytes.c` | 96 | 96 |
| 42 | `randombytes_init` | `app/src/rng.c` | 48 | 96 |
| 43 | `seedexpander_init` | `app/src/rng.c` | 48 | 96 |
| 44 | `seedexpander` | `app/src/rng.c` | 48 | 96 |
| 45 | `AES256_ECB` | `app/src/rng.c` | 48 | 96 |
| 46 | `AES256_CTR_DRBG_Update` | `app/src/rng.c` | 48 | 96 |
| 47 | `DRBG_ctx` (data) | `app/src/rng.c` | 48 | 96 |
| 48 | `blake256` | `lib/blake/src/blake256.c` | 24 | 24 |
| 49 | `blake256_init` | `lib/blake/src/blake256.c` | 24 | 24 |
| 50 | `blake256_update` | `lib/blake/src/blake256.c` | 24 | 24 |
| 51 | `blake256_final` | `lib/blake/src/blake256.c` | 24 | 24 |
| 52 | `blake256_compress` | `lib/blake/src/blake256.c` | 24 | 24 |
| 53 | `SPX_blake256_mgf1` | `lib/blake/src/blake256.c` | 24 | 24 |
| 54 | `blake512` | `lib/blake/src/blake512.c` | 24 | 24 |
| 55 | `blake512_init` | `lib/blake/src/blake512.c` | 24 | 24 |
| 56 | `blake512_update` | `lib/blake/src/blake512.c` | 24 | 24 |
| 57 | `blake512_final` | `lib/blake/src/blake512.c` | 24 | 24 |
| 58 | `blake512_compress` | `lib/blake/src/blake512.c` | 24 | 24 |
| 59 | `SPX_blake512_mgf1` | `lib/blake/src/blake512.c` | 24 | 24 |
| 60 | `cst` (data, 16 x `u64`) | `lib/blake/src/blake512.c` | 24 | 24 |
| 61 | `sha256` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 62 | `sha256_inc_init` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 63 | `sha256_inc_blocks` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 64 | `sha256_inc_finalize` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 65 | `sha512` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 66 | `sha512_inc_init` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 67 | `sha512_inc_blocks` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 68 | `sha512_inc_finalize` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 69 | `SPX_mgf1_256` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 70 | `SPX_mgf1_512` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 71 | `SPX_seed_state` | `lib/sha2/src/sha2.c` | 24 | 24 |
| 72 | `shake256` | `lib/shake/src/fips202.c` | 24 | 24 |
| 73 | `shake256_absorb` | `lib/shake/src/fips202.c` | 24 | 24 |
| 74 | `shake256_squeezeblocks` | `lib/shake/src/fips202.c` | 24 | 24 |
| 75 | `shake256_inc_init` | `lib/shake/src/fips202.c` | 24 | 24 |
| 76 | `shake256_inc_absorb` | `lib/shake/src/fips202.c` | 24 | 24 |
| 77 | `shake256_inc_finalize` | `lib/shake/src/fips202.c` | 24 | 24 |
| 78 | `shake256_inc_squeeze` | `lib/shake/src/fips202.c` | 24 | 24 |
| 79 | `SPX_tweak_constants` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 80 | `SPX_haraka_S_inc_init` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 81 | `SPX_haraka_S_inc_absorb` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 82 | `SPX_haraka_S_inc_finalize` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 83 | `SPX_haraka_S_inc_squeeze` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 84 | `SPX_haraka_S` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 85 | `SPX_haraka512_perm` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 86 | `SPX_haraka512` | `lib/haraka/src/haraka.c` | 24 | 24 |
| 87 | `SPX_haraka256` | `lib/haraka/src/haraka.c` | 24 | 24 |

`<b>` is the `HASH_BACKEND` value, `<t>` the `THASH` value.  `SPX_NAMESPACE(s)`
expands to `SPX_##s`, which is why only part of the surface carries the prefix —
`blake256`, `sha256`, `shake256` and the `crypto_sign*` API are declared without
it in the C headers, and the Rust exports reproduce that exactly.

## Notes on the two rows that are not 1:1

* **`randombytes`** — `app/CMakeLists.txt` builds `sphincs_core` from
  `randombytes.c` and `sphincs_core_det` from `rng.c`.  Both define
  `randombytes`, so only one can be linked at a time.  The `urandom` Cargo
  feature selects the `/dev/urandom` version; the default matches the `driver`
  executable, which links `sphincs_core_det`.  Exported in all 96
  configurations on both sides, with the provider switched by the feature.
* **`randombytes_init`, `seedexpander{,_init}`, `AES256_ECB`,
  `AES256_CTR_DRBG_Update`, `DRBG_ctx`** — these live only in `rng.c`, so the C
  `libsphincs_core.so` (urandom) does not have them.  The Rust `cdylib` keeps
  them in every configuration, which makes the Rust side a strict **superset**.
  No C symbol is ever missing from the Rust `.so`, which is the property the
  gate requires; the extras are the entire NIST DRBG, still differentially
  tested in the `urandom` configurations against `rng.c` compiled into
  `libsphincs_core_det.so`.

## Fix applied during this phase

`lib/blake/src/blake512.c` declares its round-constant table as
`const u64 cst[16]` **without** `static` (unlike `blake256.c`, whose
`static const u32 cst[16]` stays internal), so `libblake.so` exports a
128-byte read-only data symbol named `cst`.  It was absent from the Rust
`.so`; `src/backend/blake/blake512.rs` now re-exports the already-translated
constant table under that name.  No other symbol needed adding and no C module
was missing from the translation.

## Re-verified inside the test suite

`translation/tests/a_symbols.rs` repeats this check in-process for whatever
feature combination cargo was invoked with, so it runs 96 times as part of
`run_all_tests.sh`:

* `every_c_symbol_is_exported_by_rust` — recomputes the `nm -D` sets, asserts the
  difference is empty, and additionally `dlsym`s every C symbol name against the
  Rust handle, so a name that appears in `nm` but is not actually resolvable
  would still fail.
* `rust_so_has_no_unresolved_non_libc_symbols` — asserts nothing in the Rust
  `.so`'s undefined-symbol list overlaps the SPHINCS+ surface; the `RTLD_NOW`
  open in the harness has already proved the remainder resolves against the
  platform C runtime.

`tests/common/mod.rs` also refuses to run if the loaded `libsphincsplus.so`
disagrees with the test binary's configuration (backend probe symbol plus the
four `crypto_sign_*bytes` sizes), so a stale artifact from a differently
configured build cannot make the suite pass by accident.
