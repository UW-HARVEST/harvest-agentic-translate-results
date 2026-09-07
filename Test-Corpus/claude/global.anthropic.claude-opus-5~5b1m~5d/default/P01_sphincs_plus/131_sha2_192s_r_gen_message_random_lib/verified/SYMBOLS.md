# SYMBOLS.md — Public symbol surface (Phase A)

Mechanically derived from `nm -D --defined-only` on **every** C `.so` produced by
all 48 CMake configurations (4 backends x 2 thash x 6 secpar), and from
`nm -D --defined-only` on the Rust `cdylib`
(`translation/target/release/libsphincs_core_det.so`) built for each backend.

Reproduce with:

```bash
./build_c.sh <backend> <thash> <secpar>   # per config, into cbuild/
./symdiff.sh                              # dumps symdumps/{c,r}-<backend>.txt and diffs
```

## C shared libraries produced per configuration

| library | sources | notes |
|---|---|---|
| `app/libsphincs_core.so` | `address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c` + `randombytes.c` | non-deterministic RNG (`/dev/urandom`) |
| `app/libsphincs_core_det.so` | same objects + `rng.c` | NIST AES-256-CTR DRBG (deterministic); this is what the Rust `[lib] name = "sphincs_core_det"` corresponds to |
| `lib/<backend>/lib<backend>.so` | backend sources + `hash_<backend>.c` + `thash_<backend>_<THASH>.c` + `../../app/src/utils.c` | selected by `HASH_BACKEND` |

The Rust crate produces a **single** `libsphincs_core_det.so` that must export the
union of all three (for the active backend feature).

## Symbol-parity result

```
[blake]  MISSING_IN_RUST: <none>
[haraka] MISSING_IN_RUST: <none>
[sha2]   MISSING_IN_RUST: <none>
[shake]  MISSING_IN_RUST: <none>
```

`nm -D --undefined-only` on the Rust `.so` resolves only against libc / libgcc
(`memcpy`, `memset`, `malloc`, `abort`, `_Unwind_*`, `__cxa_finalize`, ...) — 0
missing non-libc symbols.

### Symbols that were MISSING and how they were fixed

| symbol | kind | C source | why it was missing | fix applied |
|---|---|---|---|---|
| `AES256_ECB` | `T` | `app/src/rng.c:117` (external linkage, forward-declared at `rng.c:17`) | implementation existed in Rust as the private `fn aes256_ecb` but had no `extern "C"` wrapper | added `#[unsafe(no_mangle)] pub unsafe extern "C" fn AES256_ECB` in `src/rng.rs::ffi` |
| `DRBG_ctx` | `B` | `app/src/rng.c:15` — `AES256_CTR_DRBG_struct DRBG_ctx;` (tentative definition, external linkage) | the Rust global was named `DRBG_CTX` and was not `#[no_mangle]` | renamed to `DRBG_ctx` and exported with `#[unsafe(no_mangle)] pub static mut`; `drbg_ctx()` still hands out `&mut` to *that same* object, so external writes through the exported symbol are observed by `randombytes`/`randombytes_init` exactly as in C |
| `cst` | `R` | `lib/blake/src/blake512.c:45` — `const u64 cst[16]` (**not** `static`, so external linkage) | the Rust translation had it as a private `static CST` | added `#[unsafe(no_mangle)] pub static cst: [u64;16] = CST;` in `src/backend/blake512.rs` |

No stubs were introduced; every symbol above is backed by the real translated
logic.

### Extra symbols in the Rust `.so` (Rust ⊃ C — does not violate the gate)

Only for the `shake` backend:

```
sha3_256 sha3_256_inc_absorb sha3_256_inc_finalize sha3_256_inc_init
sha3_512 sha3_512_inc_absorb sha3_512_inc_finalize sha3_512_inc_init
shake128 shake128_absorb shake128_inc_absorb shake128_inc_finalize
shake128_inc_init shake128_inc_squeeze shake128_squeezeblocks
```

These are *declared* in `c_src/lib/shake/include/fips202.h` but **not defined** in
`c_src/lib/shake/src/fips202.c` (verified: `fips202.c` defines only the
`shake256*` entry points). The Rust translation implemented the full header, so
they exist as extra exports. They are unreachable from any C entry point and do
not affect behaviour; the parity gate is `C ⊆ Rust`, which holds.

## Full symbol table

`kind` is the `nm` type letter: `T` = text (function), `B` = BSS data,
`R` = read-only data.

| # | symbol | kind | blake | haraka | sha2 | shake | in Rust .so |
|---|--------|------|-------|--------|------|-------|-------------|
| 1 | `AES256_CTR_DRBG_Update` | T |  yes | yes | yes | yes | yes |
| 2 | `AES256_ECB` | T |  yes | yes | yes | yes | yes |
| 3 | `DRBG_ctx` | B |  yes | yes | yes | yes | yes |
| 4 | `SPX_blake256_mgf1` | T |  yes | -- | -- | -- | yes |
| 5 | `SPX_blake512_mgf1` | T |  yes | -- | -- | -- | yes |
| 6 | `SPX_bytes_to_ull` | T |  yes | yes | yes | yes | yes |
| 7 | `SPX_chain_lengths` | T |  yes | yes | yes | yes | yes |
| 8 | `SPX_compute_root` | T |  yes | yes | yes | yes | yes |
| 9 | `SPX_copy_keypair_addr` | T |  yes | yes | yes | yes | yes |
| 10 | `SPX_copy_subtree_addr` | T |  yes | yes | yes | yes | yes |
| 11 | `SPX_fors_gen_leafx1` | T |  yes | yes | yes | yes | yes |
| 12 | `SPX_fors_pk_from_sig` | T |  yes | yes | yes | yes | yes |
| 13 | `SPX_fors_sign` | T |  yes | yes | yes | yes | yes |
| 14 | `SPX_fors_treehashx1` | T |  yes | yes | yes | yes | yes |
| 15 | `SPX_gen_message_random` | T |  yes | yes | yes | yes | yes |
| 16 | `SPX_haraka256` | T |  -- | yes | -- | -- | yes |
| 17 | `SPX_haraka512` | T |  -- | yes | -- | -- | yes |
| 18 | `SPX_haraka512_perm` | T |  -- | yes | -- | -- | yes |
| 19 | `SPX_haraka_S` | T |  -- | yes | -- | -- | yes |
| 20 | `SPX_haraka_S_inc_absorb` | T |  -- | yes | -- | -- | yes |
| 21 | `SPX_haraka_S_inc_finalize` | T |  -- | yes | -- | -- | yes |
| 22 | `SPX_haraka_S_inc_init` | T |  -- | yes | -- | -- | yes |
| 23 | `SPX_haraka_S_inc_squeeze` | T |  -- | yes | -- | -- | yes |
| 24 | `SPX_hash_message` | T |  yes | yes | yes | yes | yes |
| 25 | `SPX_initialize_hash_function` | T |  yes | yes | yes | yes | yes |
| 26 | `SPX_merkle_gen_root` | T |  yes | yes | yes | yes | yes |
| 27 | `SPX_merkle_sign` | T |  yes | yes | yes | yes | yes |
| 28 | `SPX_mgf1_256` | T |  -- | -- | yes | -- | yes |
| 29 | `SPX_mgf1_512` | T |  -- | -- | yes | -- | yes |
| 30 | `SPX_prf_addr` | T |  yes | yes | yes | yes | yes |
| 31 | `SPX_seed_state` | T |  -- | -- | yes | -- | yes |
| 32 | `SPX_set_chain_addr` | T |  yes | yes | yes | yes | yes |
| 33 | `SPX_set_hash_addr` | T |  yes | yes | yes | yes | yes |
| 34 | `SPX_set_keypair_addr` | T |  yes | yes | yes | yes | yes |
| 35 | `SPX_set_layer_addr` | T |  yes | yes | yes | yes | yes |
| 36 | `SPX_set_tree_addr` | T |  yes | yes | yes | yes | yes |
| 37 | `SPX_set_tree_height` | T |  yes | yes | yes | yes | yes |
| 38 | `SPX_set_tree_index` | T |  yes | yes | yes | yes | yes |
| 39 | `SPX_set_type` | T |  yes | yes | yes | yes | yes |
| 40 | `SPX_thash` | T |  yes | yes | yes | yes | yes |
| 41 | `SPX_treehash` | T |  yes | yes | yes | yes | yes |
| 42 | `SPX_tweak_constants` | T |  -- | yes | -- | -- | yes |
| 43 | `SPX_u32_to_bytes` | T |  yes | yes | yes | yes | yes |
| 44 | `SPX_ull_to_bytes` | T |  yes | yes | yes | yes | yes |
| 45 | `SPX_wots_gen_leafx1` | T |  yes | yes | yes | yes | yes |
| 46 | `SPX_wots_pk_from_sig` | T |  yes | yes | yes | yes | yes |
| 47 | `SPX_wots_treehashx1` | T |  yes | yes | yes | yes | yes |
| 48 | `blake256` | T |  yes | -- | -- | -- | yes |
| 49 | `blake256_compress` | T |  yes | -- | -- | -- | yes |
| 50 | `blake256_final` | T |  yes | -- | -- | -- | yes |
| 51 | `blake256_init` | T |  yes | -- | -- | -- | yes |
| 52 | `blake256_update` | T |  yes | -- | -- | -- | yes |
| 53 | `blake512` | T |  yes | -- | -- | -- | yes |
| 54 | `blake512_compress` | T |  yes | -- | -- | -- | yes |
| 55 | `blake512_final` | T |  yes | -- | -- | -- | yes |
| 56 | `blake512_init` | T |  yes | -- | -- | -- | yes |
| 57 | `blake512_update` | T |  yes | -- | -- | -- | yes |
| 58 | `crypto_sign` | T |  yes | yes | yes | yes | yes |
| 59 | `crypto_sign_bytes` | T |  yes | yes | yes | yes | yes |
| 60 | `crypto_sign_keypair` | T |  yes | yes | yes | yes | yes |
| 61 | `crypto_sign_open` | T |  yes | yes | yes | yes | yes |
| 62 | `crypto_sign_publickeybytes` | T |  yes | yes | yes | yes | yes |
| 63 | `crypto_sign_secretkeybytes` | T |  yes | yes | yes | yes | yes |
| 64 | `crypto_sign_seed_keypair` | T |  yes | yes | yes | yes | yes |
| 65 | `crypto_sign_seedbytes` | T |  yes | yes | yes | yes | yes |
| 66 | `crypto_sign_signature` | T |  yes | yes | yes | yes | yes |
| 67 | `crypto_sign_verify` | T |  yes | yes | yes | yes | yes |
| 68 | `cst` | R |  yes | -- | -- | -- | yes |
| 69 | `randombytes` | T |  yes | yes | yes | yes | yes |
| 70 | `randombytes_init` | T |  yes | yes | yes | yes | yes |
| 71 | `seedexpander` | T |  yes | yes | yes | yes | yes |
| 72 | `seedexpander_init` | T |  yes | yes | yes | yes | yes |
| 73 | `sha256` | T |  -- | -- | yes | -- | yes |
| 74 | `sha256_inc_blocks` | T |  -- | -- | yes | -- | yes |
| 75 | `sha256_inc_finalize` | T |  -- | -- | yes | -- | yes |
| 76 | `sha256_inc_init` | T |  -- | -- | yes | -- | yes |
| 77 | `sha512` | T |  -- | -- | yes | -- | yes |
| 78 | `sha512_inc_blocks` | T |  -- | -- | yes | -- | yes |
| 79 | `sha512_inc_finalize` | T |  -- | -- | yes | -- | yes |
| 80 | `sha512_inc_init` | T |  -- | -- | yes | -- | yes |
| 81 | `shake256` | T |  -- | -- | -- | yes | yes |
| 82 | `shake256_absorb` | T |  -- | -- | -- | yes | yes |
| 83 | `shake256_inc_absorb` | T |  -- | -- | -- | yes | yes |
| 84 | `shake256_inc_finalize` | T |  -- | -- | -- | yes | yes |
| 85 | `shake256_inc_init` | T |  -- | -- | -- | yes | yes |
| 86 | `shake256_inc_squeeze` | T |  -- | -- | -- | yes | yes |
| 87 | `shake256_squeezeblocks` | T |  -- | -- | -- | yes | yes |

## Phase D result

```
$ ./symdiff.sh
[blake]  MISSING_IN_RUST: <none>
[haraka] MISSING_IN_RUST: <none>
[sha2]   MISSING_IN_RUST: <none>
[shake]  MISSING_IN_RUST: <none>
```

`nm -D --undefined-only` on the Rust `.so`, after filtering libc / libgcc
(`memcpy`, `memset`, `malloc`, `abort`, `open64`, `pthread_*`, `_Unwind_*`,
`__cxa_finalize`, ...), leaves **0** unresolved symbols.

`cargo check --no-default-features --features <combo>` is clean for all 48
combinations, plus the `shake256` alias feature.
