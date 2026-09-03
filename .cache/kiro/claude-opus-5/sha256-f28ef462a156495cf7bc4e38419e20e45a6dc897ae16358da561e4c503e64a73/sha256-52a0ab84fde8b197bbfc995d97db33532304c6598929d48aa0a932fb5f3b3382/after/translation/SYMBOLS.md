# SYMBOLS.md — C ↔ Rust exported-symbol parity

Derived mechanically from `nm -D --defined-only` on the C shared libraries and
on the Rust `cdylib`, for **every** one of the 48 build configurations
(4 `HASH_BACKEND` × 2 `THASH` × 6 `SECPAR`).

## What is compared

`c_src` produces three shared libraries per configuration
(`c_src/app/CMakeLists.txt`, `c_src/lib/<backend>/CMakeLists.txt`):

| C shared library | sources |
|---|---|
| `libsphincs_core.so`     | `app/src/{address,fors,merkle,sign,utils,utilsx1,wots,wotsx1}.c` + `app/src/randombytes.c` |
| `libsphincs_core_det.so` | same object files + `app/src/rng.c` |
| `lib<backend>.so`        | `lib/<backend>/src/*` (+ `app/src/utils.c` for `blake`/`sha2`) |

The Rust crate builds **one** `cdylib` (`libsphincs_plus.so`) that has to export
the **union** of all three. Both `randombytes.c` and `rng.c` define a symbol
named `randombytes`; the C build keeps them apart in two libraries, so a single
Rust `.so` can only export one of them. The crate exports the **deterministic**
(`rng.c`) one — that is the variant the `driver` executable links
(`target_link_libraries(driver sphincs_core_det ...)`), and it is the variant
with observable, testable behaviour. `app/src/randombytes.c` is still
translated (`src/randombytes.rs`, `randombytes_urandom`) but deliberately not
`#[no_mangle]`-exported, to avoid the duplicate-symbol clash.

## Verification

```
./build_matrix.sh   # builds C (3 .so + driver) and Rust (.so + driver) × 48
./symdiff.sh        # nm -D diff, union(C) vs Rust, for all 48
```

Result: **`checked 48 combos, 0 with missing symbols`**, and with
`SHOW_EXTRA=1` there are also **no extra** non-`_`-prefixed symbols in the Rust
`.so`. The diff is empty in both directions.

## The symbol table

`present` columns record which backend configurations export the symbol
(`T` = text/function, `B` = BSS data, `R` = read-only data, `-` = absent).
Because the C `.so` set and the Rust `.so` are byte-for-byte identical in
symbol names, one column per symbol suffices for Rust: every row below is
exported by the Rust `.so` in exactly the configurations marked non-`-`.

### Backend-independent — `app/src/*` (42 symbols, all 48 configs)

| symbol | kind | C source | Rust source |
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
| `SPX_wots_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `SPX_fors_treehashx1` | T | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `SPX_wots_pk_from_sig` | T | `app/src/wots.c` | `src/wots.rs` |
| `SPX_chain_lengths` | T | `app/src/wots.c` | `src/wots.rs` |
| `SPX_wots_gen_leafx1` | T | `app/src/wotsx1.c` | `src/wotsx1.rs` |
| `SPX_fors_sign` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_pk_from_sig` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_gen_leafx1` | T | `app/src/fors.c` | `src/fors.rs` |
| `SPX_merkle_sign` | T | `app/src/merkle.c` | `src/merkle.rs` |
| `SPX_merkle_gen_root` | T | `app/src/merkle.c` | `src/merkle.rs` |
| `randombytes` | T | `app/src/rng.c` | `src/rng.rs` |
| `randombytes_init` | T | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander` | T | `app/src/rng.c` | `src/rng.rs` |
| `seedexpander_init` | T | `app/src/rng.c` | `src/rng.rs` |
| `AES256_ECB` | T | `app/src/rng.c` | `src/rng.rs` |
| `AES256_CTR_DRBG_Update` | T | `app/src/rng.c` | `src/rng.rs` |
| `DRBG_ctx` | B | `app/src/rng.c` (global) | `src/rng.rs` (`static mut`) |

### Backend hooks — same names in all four backends (5 symbols)

| symbol | kind | C source | Rust source |
|---|---|---|---|
| `SPX_initialize_hash_function` | T | `lib/<b>/src/hash_<b>.c` | `src/<b>/hash.rs` |
| `SPX_prf_addr` | T | `lib/<b>/src/hash_<b>.c` | `src/<b>/hash.rs` |
| `SPX_gen_message_random` | T | `lib/<b>/src/hash_<b>.c` | `src/<b>/hash.rs` |
| `SPX_hash_message` | T | `lib/<b>/src/hash_<b>.c` | `src/<b>/hash.rs` |
| `SPX_thash` | T | `lib/<b>/src/thash_<b>_<thash>.c` | `src/<b>/thash_<b>_<thash>.rs` |

### `HASH_BACKEND=blake` only (13 symbols)

| symbol | kind | C source | Rust source |
|---|---|---|---|
| `blake256`, `blake256_init`, `blake256_update`, `blake256_final`, `blake256_compress` | T | `lib/blake/src/blake256.c` | `src/blake/blake256.rs` |
| `SPX_blake256_mgf1` | T | `lib/blake/src/blake256.c` | `src/blake/blake256.rs` |
| `blake512`, `blake512_init`, `blake512_update`, `blake512_final`, `blake512_compress` | T | `lib/blake/src/blake512.c` | `src/blake/blake512.rs` |
| `SPX_blake512_mgf1` | T | `lib/blake/src/blake512.c` | `src/blake/blake512.rs` |
| `cst` | **R** | `lib/blake/src/blake512.c` — `const u64 cst[16]`, **not** `static`, so it has a linker symbol (the BLAKE-256 `cst` in `blake256.c` *is* `static` and does not) | `src/blake/blake512.rs`, `#[no_mangle] pub static cst: [u64; 16]` |

`cst` was the one symbol the Rust `.so` was missing at the start of this
verification; the implementation (the constant table) was already present as a
private `static CST`, so the fix was to add the `#[no_mangle]` export, matching
both the name and the 0x80-byte size / read-only section of the C symbol.

### `HASH_BACKEND=sha2` only (11 symbols)

| symbol | kind | C source | Rust source |
|---|---|---|---|
| `sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize` | T | `lib/sha2/src/sha2.c` | `src/sha2/sha2.rs` |
| `sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize` | T | `lib/sha2/src/sha2.c` | `src/sha2/sha2.rs` |
| `SPX_mgf1_256`, `SPX_mgf1_512` | T | `lib/sha2/src/sha2.c` | `src/sha2/sha2.rs` |
| `SPX_seed_state` | T | `lib/sha2/src/sha2.c` | `src/sha2/sha2.rs` |

### `HASH_BACKEND=shake` only (7 symbols)

| symbol | kind | C source | Rust source |
|---|---|---|---|
| `shake256`, `shake256_absorb`, `shake256_squeezeblocks` | T | `lib/shake/src/fips202.c` | `src/shake/fips202.rs` |
| `shake256_inc_init`, `shake256_inc_absorb`, `shake256_inc_finalize`, `shake256_inc_squeeze` | T | `lib/shake/src/fips202.c` | `src/shake/fips202.rs` |

`lib/shake/include/fips202.h` also *declares* `shake128*`, `sha3_256*` and
`sha3_512*`, but `lib/shake/src/fips202.c` in this tree does not define them, so
they are absent from `libshake.so` and must be absent from the Rust `.so` too.
The Keccak core (`load64`, `store64`, `KeccakF1600_StatePermute`,
`keccak_absorb`, `keccak_squeezeblocks`, `keccak_inc_*`) is `static` in C and
therefore private in Rust as well.

### `HASH_BACKEND=haraka` only (9 symbols)

| symbol | kind | C source | Rust source |
|---|---|---|---|
| `SPX_tweak_constants` | T | `lib/haraka/src/haraka.c` | `src/haraka/haraka.rs` |
| `SPX_haraka_S`, `SPX_haraka_S_inc_init`, `SPX_haraka_S_inc_absorb`, `SPX_haraka_S_inc_finalize`, `SPX_haraka_S_inc_squeeze` | T | `lib/haraka/src/haraka.c` | `src/haraka/haraka.rs` |
| `SPX_haraka512`, `SPX_haraka512_perm`, `SPX_haraka256` | T | `lib/haraka/src/haraka.c` | `src/haraka/haraka.rs` |

The BearSSL-derived bit-sliced AES helpers in the first half of
`lib/haraka/src/haraka.c` are `static` in C → private `src/haraka/aes_ct.rs`.

## Per-configuration symbol counts (union of the 3 C `.so`, = Rust `.so`)

| `HASH_BACKEND` | symbols | = C ∪, = Rust |
|---|---|---|
| `blake`  | 60 | ✅ |
| `sha2`   | 58 | ✅ |
| `shake`  | 54 | ✅ |
| `haraka` | 56 | ✅ |

## Gate

- [x] `nm -D` shows **0 missing** symbols in the Rust `.so`, in all 48 configurations.
- [x] `nm -D` shows **0 extra** (non-`_`/`rust_`-prefixed) symbols in the Rust `.so`.
- [x] No symbol is a stub: every export routes to a real translation of the
      corresponding C function (verified behaviourally by the Phase B/C
      differential tests, which call each one through `dlopen`/`dlsym`).
