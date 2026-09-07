# SYMBOLS.md — exported-symbol parity, C `.so` vs Rust `.so`

Derived mechanically from `nm -D --defined-only` on the C shared libraries and
on `translation/target/release/libsphincsplus.so`.

## How the C `.so` set is formed

`c_src` produces **three** shared objects per configuration plus one executable
(`c_src/app/CMakeLists.txt`, `c_src/lib/<backend>/CMakeLists.txt`):

| C artifact | sources | notes |
|---|---|---|
| `lib/<backend>/lib<backend>.so` | backend primitive + `hash_<b>.c` + `thash_<b>_<THASH>.c` (+ `app/src/utils.c` for blake/sha2) | one backend only — `lib/CMakeLists.txt` does `add_subdirectory(${HASH_BACKEND})` |
| `app/libsphincs_core.so` | `sphincs_obj` objects + `src/randombytes.c` | `/dev/urandom` `randombytes()` (returns `void`) |
| `app/libsphincs_core_det.so` | `sphincs_obj` objects + `src/rng.c` | NIST AES-256-CTR-DRBG `randombytes()` (returns `int`) |
| `app/driver` | `src/PQCgenKAT_sign.c` + `sphincs_core_det` + backend | KAT transcript executable |

The Rust crate is a **single** `cdylib`, so the required Rust export set is the
**union** of all three C `.so`s.  `randombytes` is provided by exactly one of
`randombytes.c` / `rng.c` in C; in Rust the `urandom` Cargo feature picks
`randombytes.rs`, otherwise `rng.rs` (see `build.rs` → `rand_urandom` /
`rand_drbg`).

## Verification command

`./symsweep.sh` — builds the Rust cdylib for each of the 48
`(HASH_BACKEND, THASH, SECPAR)` triples and diffs against the C union for the
same triple.

## Result

**48/48 configurations: symbol diff is empty in both directions**
(`MISSING_IN_RUST` and `EXTRA_IN_RUST` both empty; `C=60 RS=60` for blake,
`C=58 RS=58` for sha2, `C=56 RS=56` for haraka, `C=54 RS=54` for shake — the
union of the two core `.so`s plus the backend `.so`).

`nm -D -u` on the Rust `.so` shows only libc (`@GLIBC_*`), `_ITM_*`, `__cxa_*`,
`__gmon_start__`, and `_Unwind_*@GCC_*` (libgcc personality routine)
undefined symbols — **0 missing/undefined non-libc symbols**.

## Fix applied during this phase

| symbol | C origin | why it was missing | fix |
|---|---|---|---|
| `cst` | `c_src/lib/blake/src/blake512.c:45` — `const u64 cst[16]` has **external** linkage (unlike the `static const u32 cst[16]` in `blake256.c`), so it lands in `.rodata` as a public `R` symbol of `libblake.so` | Rust had it as a private `const CST` | added `#[no_mangle] pub static cst: [u64; 16] = CST;` in `translation/src/backend/blake/blake512.rs` |

No symbol required translating a missing module: every other C source file
already had a Rust counterpart (`translation/src/**` maps 1:1 onto
`c_src/app/src/*.c` and `c_src/lib/*/src/*.c`).

## Full symbol inventory

### `app/libsphincs_core.so` / `libsphincs_core_det.so` — 36 + 6 symbols
`SPX_NAMESPACE(s)` is `SPX_##s` in every params header, so the internal API is
exported with an `SPX_` prefix.

| C symbol | type | C definition | Rust `#[no_mangle]` wrapper |
|---|---|---|---|
| `crypto_sign_secretkeybytes` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_publickeybytes` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_bytes` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_seedbytes` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_seed_keypair` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_keypair` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_signature` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_verify` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign` | T | `app/src/sign.c` | `src/sign.rs` |
| `crypto_sign_open` | T | `app/src/sign.c` | `src/sign.rs` |
| `SPX_set_layer_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_type` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_copy_subtree_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_keypair_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_copy_keypair_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_chain_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_hash_addr` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_height` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_index` | T | `app/src/address.c` | `src/address.rs` |
| `SPX_ull_to_bytes` | T | `app/src/utils.c` | `src/utils.rs` |
| `SPX_u32_to_bytes` | T | `app/src/utils.c` | `src/utils.rs` |
| `SPX_bytes_to_ull` | T | `app/src/utils.c` | `src/utils.rs` |
| `SPX_compute_root` | T | `app/src/utils.c` | `src/utils.rs` |
| `SPX_treehash` | T | `app/src/utils.c` | `src/utils.rs` |
| `SPX_wots_pk_from_sig` | T | `app/src/wots.c` | `src/wots.rs` |
| `SPX_chain_lengths` | T | `app/src/wots.c` | `src/wots.rs` |
| `SPX_wots_gen_leafx1` | T | `app/src/wotsx1.c` | `src/wotsx1.rs` |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_sign` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | `src/merkle.rs` |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | `src/merkle.rs` |
| `SPX_wots_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `SPX_fors_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `randombytes` | T | `app/src/randombytes.c` **or** `app/src/rng.c` | `src/randombytes.rs` (`urandom`) / `src/rng.rs` (default) |
| `randombytes_init` | T | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander_init` | T | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander` | T | `app/src/rng.c` | `src/rng.rs` |
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | `src/rng.rs` |
| `AES256_ECB` | T | `app/src/rng.c` | `src/rng.rs` |
| `DRBG_ctx` | B | `app/src/rng.c` (global `AES256_CTR_DRBG_struct`) | `src/rng.rs` `#[no_mangle] pub static mut DRBG_ctx` |

### `lib/blake/libblake.so` — 23 symbols
`SPX_blake256_mgf1`, `SPX_blake512_mgf1`, `blake256`, `blake256_init`,
`blake256_update`, `blake256_final`, `blake256_compress`, `blake512`,
`blake512_init`, `blake512_update`, `blake512_final`, `blake512_compress`,
`cst` (R), plus the backend hooks `SPX_initialize_hash_function`,
`SPX_prf_addr`, `SPX_gen_message_random`, `SPX_hash_message`, `SPX_thash`, and
the re-linked `app/src/utils.c` copies `SPX_ull_to_bytes`, `SPX_u32_to_bytes`,
`SPX_bytes_to_ull`, `SPX_compute_root`, `SPX_treehash`.
→ Rust: `src/backend/blake/{blake256,blake512,hash,thash_simple,thash_robust}.rs`.

### `lib/haraka/libharaka.so` — 14 symbols
`SPX_tweak_constants`, `SPX_haraka_S_inc_init`, `SPX_haraka_S_inc_absorb`,
`SPX_haraka_S_inc_finalize`, `SPX_haraka_S_inc_squeeze`, `SPX_haraka_S`,
`SPX_haraka512_perm`, `SPX_haraka512`, `SPX_haraka256`, plus the five backend
hooks.  (haraka's CMakeLists does **not** re-link `utils.c`, so no
`SPX_ull_to_bytes`/`SPX_treehash`/… here.)
→ Rust: `src/backend/haraka/*.rs`.

### `lib/sha2/libsha2.so` — 21 symbols
`sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize`,
`sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize`,
`SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`, the five backend hooks, and
the re-linked `utils.c` five.
→ Rust: `src/backend/sha2/*.rs`.

### `lib/shake/libshake.so` — 12 symbols
`shake256`, `shake256_absorb`, `shake256_squeezeblocks`, `shake256_inc_init`,
`shake256_inc_absorb`, `shake256_inc_finalize`, `shake256_inc_squeeze`, plus
the five backend hooks.  `lib/shake/include/fips202.h` *declares* SHAKE-128 and
SHA3-256/512 entry points, but `lib/shake/src/fips202.c` **defines** only the
`shake256*` family (the `keccak_*` helpers are `static`), so those declared-only
functions are not symbols of `libshake.so`.  Confirmed with `nm`, not assumed.
→ Rust: `src/backend/shake/*.rs`.

## Re-verification after the fix

```
$ ./symsweep.sh
sweep rc=0
48
$ grep -v '^OK' /tmp/symsweep.txt   # no MISSING / no EXTRA lines
$
```

Per-configuration symbol counts (C union = Rust, exactly):

| backend | union of the three C `.so`s | Rust `.so` |
|---|---|---|
| blake  | 60 | 60 |
| sha2   | 58 | 58 |
| haraka | 56 | 56 |
| shake  | 54 | 54 |

`nm -D -u` on the Rust `.so`, non-libc filter:

```
_Unwind_Backtrace@GCC_3.3
_Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0
_Unwind_GetIPInfo@GCC_4.2.0
_Unwind_GetLanguageSpecificData@GCC_3.0
_Unwind_GetRegionStart@GCC_3.0
_Unwind_SetGR@GCC_3.0
_Unwind_SetIP@GCC_3.0
_Unwind_RaiseException@GCC_3.0
_Unwind_Resume@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0
```

These are libgcc's unwinder (the Rust panic personality routine), i.e. the
toolchain runtime rather than anything from `c_src/`.  **0 missing or undefined
non-libc symbols.**

## Note on building the C reference here

The host has `libcrypto.so.3` but no `openssl-devel`, so `c_src/app/src/rng.c`
(`#include <openssl/{conf,evp,err}.h>`) would not compile and the CMake targets
`sphincs_core_det` and `driver` would be missing.  `./build_c.sh` puts a minimal
declaration-only shim (`translation/c_compat/openssl/`) on the include path and
links against the system `libcrypto.so.3`.  Nothing in `c_src/` is modified; the
shim only declares `EVP_CIPHER_CTX_new`, `EVP_CIPHER_CTX_free`,
`EVP_aes_256_ecb`, `EVP_EncryptInit_ex`, `EVP_EncryptUpdate` and
`ERR_print_errors_fp`, all of which resolve to the real OpenSSL at link time.
This matters for correctness of the comparison: the C DRBG really does run
OpenSSL's AES-256-ECB, and the Rust port (which uses the `aes` crate) is
required to match it byte for byte (`diff_e_api.rs::e12_aes256_ecb`,
`e13_drbg_update`, `e10_e11_randombytes_drbg`).
