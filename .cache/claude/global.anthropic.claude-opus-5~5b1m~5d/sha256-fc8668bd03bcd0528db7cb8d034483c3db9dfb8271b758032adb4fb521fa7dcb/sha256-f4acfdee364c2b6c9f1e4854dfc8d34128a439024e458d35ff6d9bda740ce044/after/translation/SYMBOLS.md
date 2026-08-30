# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this table was produced

The C project is built by CMake into **three** shared objects (see
`c_src/app/CMakeLists.txt`, `c_src/lib/<backend>/CMakeLists.txt`):

| C shared object | contents |
|---|---|
| `libsphincs_core.so`      | `address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c` + `randombytes.c` (`/dev/urandom`) |
| `libsphincs_core_det.so`  | the same core objects + `rng.c` (deterministic AES-256-CTR-DRBG; this is what `driver` links) |
| `lib<backend>.so`         | the hash backend (`hash_*.c`, `thash_*_<THASH>.c`, primitives) + `utils.c` |

`libsphincs_core*.so` is *not* self-contained (it has undefined `SPX_thash`,
`SPX_prf_addr`, ... which the backend `.so` provides).  The Rust crate is a
single `cdylib`, so for symbol comparison and for the differential tests we
also link one **self-contained** C `.so` per configuration out of exactly the
same translation units (`scripts/build_c.sh`):

```
app/src/{address,fors,merkle,sign,utils,utilsx1,wots,wotsx1,rng}.c
lib/<backend>/src/{...backend sources..., thash_<backend>_<thash>.c}
```

`randombytes.c` and `rng.c` both define `randombytes`; only one can live in a
single object, and the KAT driver links the deterministic one, so `rng.c` is the
one used (this matches the Rust crate, whose `randombytes.rs` translation of the
`/dev/urandom` variant is kept under the internal name `randombytes_urandom`).

Reference command (documented default configuration):

```
cd c_src && mkdir -p build && cd build && \
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DHASH_BACKEND=blake -DSECPAR=128f -DTHASH=simple && \
cmake --build .
```

(`rng.c` needs OpenSSL headers; on this host they come from the nix store, see
`scripts/build_c.sh`. The `driver` executable cannot be linked here because the
nix `libcrypto.so` wants a newer glibc than the system linker offers — this does
not affect any `.so`.)

Symbol sets are compared with:

```
nm -D --defined-only <lib> | awk '{print $3}' | sort -u
```

`scripts/build_all.sh` does this for **all 48 feature combinations**
(4 backends x 2 thash x 6 secpar) and fails if any diff is non-empty.

## Result

**0 symbols missing in Rust, 0 extra symbols, for all 48 configurations.**
`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc_s
(`memcpy`, `malloc`, `_Unwind_*`, ...) — no unresolved project symbols.

Symbol counts per backend (identical for every secpar / thash):

| backend | # exported symbols |
|---|---|
| haraka | 56 |
| sha2   | 58 |
| shake  | 54 |
| blake  | 60 |

## Symbols common to every configuration (47)

| symbol | C source | Rust source |
|---|---|---|
| `SPX_set_layer_addr`     | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_tree_addr`      | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_type`           | `app/src/address.c`  | `src/address.rs` |
| `SPX_copy_subtree_addr`  | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_keypair_addr`   | `app/src/address.c`  | `src/address.rs` |
| `SPX_copy_keypair_addr`  | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_chain_addr`     | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_hash_addr`      | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_tree_height`    | `app/src/address.c`  | `src/address.rs` |
| `SPX_set_tree_index`     | `app/src/address.c`  | `src/address.rs` |
| `SPX_ull_to_bytes`       | `app/src/utils.c`    | `src/utils.rs` |
| `SPX_u32_to_bytes`       | `app/src/utils.c`    | `src/utils.rs` |
| `SPX_bytes_to_ull`       | `app/src/utils.c`    | `src/utils.rs` |
| `SPX_compute_root`       | `app/src/utils.c`    | `src/utils.rs` |
| `SPX_treehash`           | `app/src/utils.c`    | `src/utils.rs` |
| `SPX_wots_treehashx1`    | `app/src/utilsx1.c`  | `src/utilsx1.rs` |
| `SPX_fors_treehashx1`    | `app/src/utilsx1.c`  | `src/utilsx1.rs` |
| `SPX_chain_lengths`      | `app/src/wots.c`     | `src/wots.rs` |
| `SPX_wots_pk_from_sig`   | `app/src/wots.c`     | `src/wots.rs` |
| `SPX_wots_gen_leafx1`    | `app/src/wotsx1.c`   | `src/wotsx1.rs` |
| `SPX_fors_gen_leafx1`    | `app/src/fors.c`     | `src/fors.rs` |
| `SPX_fors_sign`          | `app/src/fors.c`     | `src/fors.rs` |
| `SPX_fors_pk_from_sig`   | `app/src/fors.c`     | `src/fors.rs` |
| `SPX_merkle_sign`        | `app/src/merkle.c`   | `src/merkle.rs` |
| `SPX_merkle_gen_root`    | `app/src/merkle.c`   | `src/merkle.rs` |
| `crypto_sign_secretkeybytes` | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_publickeybytes` | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_bytes`          | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_seedbytes`      | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_seed_keypair`   | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_keypair`        | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_signature`      | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_verify`         | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign`                | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_open`           | `app/src/sign.c` | `src/sign.rs` |
| `SPX_initialize_hash_function` | `lib/*/src/hash_*.c` | `src/*_hash.rs` |
| `SPX_prf_addr`                 | `lib/*/src/hash_*.c` | `src/*_hash.rs` |
| `SPX_gen_message_random`       | `lib/*/src/hash_*.c` | `src/*_hash.rs` |
| `SPX_hash_message`             | `lib/*/src/hash_*.c` | `src/*_hash.rs` |
| `SPX_thash`                    | `lib/*/src/thash_*.c`| `src/*_thash.rs` |
| `randombytes_init`       | `app/src/rng.c` | `src/rng.rs` |
| `randombytes`            | `app/src/rng.c` | `src/rng.rs` |
| `AES256_ECB`             | `app/src/rng.c` | `src/rng.rs` |
| `AES256_CTR_DRBG_Update` | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander_init`      | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander`           | `app/src/rng.c` | `src/rng.rs` |
| `DRBG_ctx` (data, BSS)   | `app/src/rng.c` | `src/rng.rs` |

## Backend-specific symbols

### haraka (+9)
`SPX_tweak_constants`, `SPX_haraka_S_inc_init`, `SPX_haraka_S_inc_absorb`,
`SPX_haraka_S_inc_finalize`, `SPX_haraka_S_inc_squeeze`, `SPX_haraka_S`,
`SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256`
(`lib/haraka/src/haraka.c` -> `src/haraka.rs`).

### sha2 (+11)
`sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize`,
`sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize`,
`SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`
(`lib/sha2/src/sha2.c`, `hash_sha2.c` -> `src/sha2.rs`, `src/sha2_hash.rs`).

### shake (+7)
`shake256`, `shake256_absorb`, `shake256_squeezeblocks`, `shake256_inc_init`,
`shake256_inc_absorb`, `shake256_inc_finalize`, `shake256_inc_squeeze`
(`lib/shake/src/fips202.c` -> `src/fips202.rs`).

NOTE: `lib/shake/include/fips202.h` also *declares* `shake128*`, `sha3_256*`
and `sha3_512*`, but `fips202.c` contains **no definitions** for them, so
`libshake.so` does not export them.  The Rust crate therefore deliberately does
not export them either (the internal Rust implementations exist but are not
`#[no_mangle]`), keeping the surface byte-for-byte identical to the C `.so`.

### blake (+13)
`blake256`, `blake256_init`, `blake256_update`, `blake256_final`,
`blake256_compress`, `blake512`, `blake512_init`, `blake512_update`,
`blake512_final`, `blake512_compress`, `SPX_blake256_mgf1`,
`SPX_blake512_mgf1`, and the read-only data symbol `cst`
(`const u64 cst[16]` in `lib/blake/src/blake512.c` -> `pub static cst` in
`src/blake512.rs`).

## Not part of any `.so`

`app/src/PQCgenKAT_sign.c` only provides `main()` for the `driver`
executable, so it exports no library symbols and needs no translation.
