# SYMBOLS.md — dynamic-symbol parity between the C and the Rust shared objects

Generated mechanically from `nm -D --defined-only` on the C shared objects and
on the Rust `cdylib`, for **every one of the 48 build configurations**
(4 hash backends × 2 `thash` variants × 6 security parameters).

Reproduce with:

```sh
./verif/build_c_all.sh      # builds c_src/build-<backend>-<secpar>-<thash>/…
./verif/symbols_all.sh      # builds each Rust cdylib and diffs nm -D
```

`verif/symbols/<tag>.c.txt` / `verif/symbols/<tag>.rust.txt` hold the raw lists.

## What the C build produces

`c_src/CMakeLists.txt` splits the library in two pieces that are *not* linked
against each other, so the C surface is the **union** of three `.so` files:

| C artifact | sources |
|---|---|
| `app/libsphincs_core.so` | `address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c` + `randombytes.c` (`/dev/urandom`) |
| `app/libsphincs_core_det.so` | the same objects + `rng.c` (NIST AES-256-CTR-DRBG) — this is what `driver` links |
| `lib/<backend>/lib<backend>.so` | the backend's `hash_*.c`, `thash_*_<thash>.c`, its primitive (`blake256.c`/`sha2.c`/`fips202.c`/`haraka.c`) and, for `sha2`/`blake`, a second copy of `utils.c` |

`rng.c` needs OpenSSL headers, which are absent from `/usr/include` on this
host; `verif/build_c_all.sh` locates a nix-store OpenSSL and injects it through
the CMake flag variables, so **all four targets (both cores, the backend, and
`driver`) build for all 48 configurations**. `c_src` is never modified.

## Result

**0 missing symbols in every one of the 48 configurations.** The symbol count
per backend (union of the three C `.so` files):

| backend | C symbols | missing from Rust |
|---|---|---|
| `haraka` | 56 | 0 |
| `sha2`   | 58 | 0 |
| `shake`  | 54 | 0 |
| `blake`  | 60 | 0 |

### Symbols shared by every configuration (48)

From `app/src/sign.c` (plain names — `api.h` does *not* namespace them):

| symbol | Rust definition |
|---|---|
| `crypto_sign_secretkeybytes` | `src/sign.rs` |
| `crypto_sign_publickeybytes` | `src/sign.rs` |
| `crypto_sign_bytes` | `src/sign.rs` |
| `crypto_sign_seedbytes` | `src/sign.rs` |
| `crypto_sign_seed_keypair` | `src/sign.rs` |
| `crypto_sign_keypair` | `src/sign.rs` |
| `crypto_sign_signature` | `src/sign.rs` |
| `crypto_sign_verify` | `src/sign.rs` |
| `crypto_sign` | `src/sign.rs` |
| `crypto_sign_open` | `src/sign.rs` |

From `app/src/address.c` (namespaced `SPX_*` by `SPX_NAMESPACE`):

| symbol | Rust definition |
|---|---|
| `SPX_set_layer_addr` | `src/address.rs` |
| `SPX_set_tree_addr` | `src/address.rs` |
| `SPX_set_type` | `src/address.rs` |
| `SPX_copy_subtree_addr` | `src/address.rs` |
| `SPX_set_keypair_addr` | `src/address.rs` |
| `SPX_copy_keypair_addr` | `src/address.rs` |
| `SPX_set_chain_addr` | `src/address.rs` |
| `SPX_set_hash_addr` | `src/address.rs` |
| `SPX_set_tree_height` | `src/address.rs` |
| `SPX_set_tree_index` | `src/address.rs` |

From `app/src/utils.c`, `utilsx1.c`, `wots.c`, `wotsx1.c`, `fors.c`, `merkle.c`:

| symbol | Rust definition |
|---|---|
| `SPX_ull_to_bytes` | `src/utils.rs` |
| `SPX_u32_to_bytes` | `src/utils.rs` |
| `SPX_bytes_to_ull` | `src/utils.rs` |
| `SPX_compute_root` | `src/utils.rs` |
| `SPX_treehash` | `src/utils.rs` |
| `SPX_wots_treehashx1` | `src/utilsx1.rs` |
| `SPX_fors_treehashx1` | `src/utilsx1.rs` |
| `SPX_chain_lengths` | `src/wots.rs` |
| `SPX_wots_pk_from_sig` | `src/wots.rs` |
| `SPX_wots_gen_leafx1` | `src/wotsx1.rs` |
| `SPX_fors_gen_leafx1` | `src/fors.rs` |
| `SPX_fors_sign` | `src/fors.rs` |
| `SPX_fors_pk_from_sig` | `src/fors.rs` |
| `SPX_merkle_sign` | `src/merkle.rs` |
| `SPX_merkle_gen_root` | `src/merkle.rs` |

From `app/src/rng.c` (in `libsphincs_core_det.so`):

| symbol | Rust definition |
|---|---|
| `randombytes` | `src/rng.rs` |
| `randombytes_init` | `src/rng.rs` |
| `AES256_ECB` | `src/rng.rs` |
| `AES256_CTR_DRBG_Update` | `src/rng.rs` |
| `seedexpander_init` | `src/rng.rs` |
| `seedexpander` | `src/rng.rs` |
| `DRBG_ctx` (**data**) | `src/rng.rs`, `#[no_mangle] pub static mut DRBG_ctx` |

Backend-agnostic facade, provided by whichever `lib<backend>.so` is linked:

| symbol | Rust definition |
|---|---|
| `SPX_initialize_hash_function` | `src/{blake,sha2,shake,haraka}_hash.rs` |
| `SPX_prf_addr` | `src/{blake,sha2,shake,haraka}_hash.rs` |
| `SPX_gen_message_random` | `src/{blake,sha2,shake,haraka}_hash.rs` |
| `SPX_hash_message` | `src/{blake,sha2,shake,haraka}_hash.rs` |
| `SPX_thash` | `src/{blake,sha2,shake,haraka}_thash.rs` |

### Backend-specific symbols

`blake` (`lib/blake/src/blake256.c`, `blake512.c`):

`blake256`, `blake256_init`, `blake256_update`, `blake256_compress`,
`blake256_final`, `blake512`, `blake512_init`, `blake512_update`,
`blake512_compress`, `blake512_final`, `SPX_blake256_mgf1`,
`SPX_blake512_mgf1`, and the **data** symbol `cst`
(`blake512.c` declares its round-constant table as a *non-static*
`const u64 cst[16]`, so it lands in `.dynsym`). All in `src/blake256.rs` /
`src/blake512.rs`; `cst` is `#[no_mangle] pub static cst: [u64; 16]`.

`sha2` (`lib/sha2/src/sha2.c`):

`sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize`,
`sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize`,
`SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state` — all in `src/sha2.rs`.

`shake` (`lib/shake/src/fips202.c`):

`shake256`, `shake256_absorb`, `shake256_squeezeblocks`, `shake256_inc_init`,
`shake256_inc_absorb`, `shake256_inc_finalize`, `shake256_inc_squeeze` — all in
`src/fips202.rs`.

`haraka` (`lib/haraka/src/haraka.c`):

`SPX_tweak_constants`, `SPX_haraka_S_inc_init`, `SPX_haraka_S_inc_absorb`,
`SPX_haraka_S_inc_finalize`, `SPX_haraka_S_inc_squeeze`, `SPX_haraka_S`,
`SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256` — all in
`src/haraka.rs`.

## Fixes made during this phase

1. **`cst` was missing** from the Rust `.so` for the `blake` backend. The
   implementation existed (`C512` in `src/blake512.rs`) but was `const` and
   therefore not a linker symbol; a `#[no_mangle] pub static cst: [u64; 16]`
   alias was added. Verified byte-for-byte against the C table by
   `tests/diff_backend.rs::blake::cst_data_symbol_matches`.
2. **All of `rng.c` was effectively untranslated at the export level.** Only
   `randombytes` / `randombytes_init` were exported, and the DRBG state lived in
   a private `Mutex<Drbg>` with no `DRBG_ctx` symbol; `seedexpander`,
   `seedexpander_init`, `AES256_ECB` and `AES256_CTR_DRBG_Update` had no
   `extern "C"` wrappers at all (and `seedexpander` could not express the
   `x == NULL` rejection because it took a Rust slice). `src/rng.rs` was
   restructured so the DRBG lives in a `#[repr(C)] #[no_mangle] pub static mut
   DRBG_ctx` with exactly the C layout (`Key[32] ‖ V[16] ‖ int`), and all six
   functions plus the data symbol are exported with the C signatures. Covered by
   `tests/diff_rng.rs`.
3. **No FFI struct was `#[repr(C)]`.** `spx_ctx`, `blakestate256`,
   `blakestate512`, `leaf_info_x1`, `fors_gen_leaf_info` and `AES_XOF_struct`
   all cross the FFI boundary, and Rust's default layout is not the C one — this
   was a live miscompare (`blake256_compress` wrote `h` at offset 64 instead of
   0, and `leaf_info_x1` had `wots_sign_leaf` in the wrong place because rustc
   groups the two pointers first). `#[repr(C)]` was added to all six.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust cdylib lists only libc / libgcc entries
(`memcpy@GLIBC_*`, `malloc`, `free`, `_Unwind_*`, `__tls_get_addr`, …).
**0 undefined non-libc symbols.**

## Symbols the Rust `.so` exports in addition to the C `.so`

Only for the `shake` backend, and only because `lib/shake/include/fips202.h`
*declares* the following but `lib/shake/src/fips202.c` never *defines* them (so
they are missing from `libshake.so` and any C consumer using them would fail to
link):

`shake128`, `shake128_absorb`, `shake128_squeezeblocks`, `shake128_inc_init`,
`shake128_inc_absorb`, `shake128_inc_finalize`, `shake128_inc_squeeze`,
`sha3_256`, `sha3_256_inc_init`, `sha3_256_inc_absorb`,
`sha3_256_inc_finalize`, `sha3_512`, `sha3_512_inc_init`,
`sha3_512_inc_absorb`, `sha3_512_inc_finalize`.

They are real implementations of the declared interface, not stubs. Every other
configuration is an exact match in both directions.

## `randombytes` — the one name the two C cores disagree on

`randombytes.c` (`void`, `/dev/urandom`) and `rng.c` (`int`, AES-256-CTR-DRBG)
both define `randombytes`, and the C keeps them apart by linking them into two
separate `.so` files. A single Rust artifact can export the name once, so it
exports the **deterministic `rng.c` version** — the one `driver`, i.e. the
project's own entry point, links. The `/dev/urandom` translation is retained in
`src/randombytes.rs` as `randombytes_urandom`. Because the name is present
either way, `nm -D` parity holds against both C cores.
