# SYMBOLS.md — C `.so` vs Rust `.so` exported-symbol parity

Mechanically derived from `nm -D --defined-only` on the C shared libraries and
on `translation/target/release/libsphincs_plus.so`.

## How the C build maps onto the single Rust `cdylib`

`c_src` produces **three** shared libraries per configuration plus one
executable:

| C artifact | sources | Rust counterpart |
|---|---|---|
| `lib/<backend>/lib<backend>.so` | `lib/<backend>/src/*.c` + `app/src/utils.c` | `src/<backend>/*` + `src/utils.rs` |
| `app/libsphincs_core.so` | `sphincs_obj` objects + `app/src/randombytes.c` | see note on `randombytes` below |
| `app/libsphincs_core_det.so` | `sphincs_obj` objects + `app/src/rng.c` | `src/*.rs` incl. `src/rng.rs` |
| `app/driver` | `app/src/PQCgenKAT_sign.c` linked against `sphincs_core_det` | `src/main.rs` (`[[bin]] driver`) |

The Rust crate builds **one** `cdylib` that is the union of
`lib<backend>.so` + `libsphincs_core_det.so`, so parity is checked against the
union of the exported symbols of the three C libraries.

`randombytes` note — `app/src/randombytes.c` (`/dev/urandom`, returns `void`)
and `app/src/rng.c` (NIST AES-256-CTR DRBG, returns `int`) both define the
linker symbol `randombytes`; CMake keeps them apart by putting them in two
different `.so`s. A single Rust `cdylib` cannot export both, so the crate
exports the **deterministic** `rng.c` variant (matching `sphincs_core_det.so`,
which is what `driver` links) and keeps the urandom variant as the private
`crate::randombytes::randombytes_urandom`. This is the only intentional
deviation and it mirrors the linker semantics of the C build exactly.

## Symbol sets

### Common to every configuration (48 symbols)

From `app/src/{address,fors,merkle,sign,utils,utilsx1,wots,wotsx1}.c` and
`app/src/rng.c`:

```
AES256_CTR_DRBG_Update  AES256_ECB              DRBG_ctx
SPX_bytes_to_ull        SPX_chain_lengths       SPX_compute_root
SPX_copy_keypair_addr   SPX_copy_subtree_addr   SPX_fors_gen_leafx1
SPX_fors_pk_from_sig    SPX_fors_sign           SPX_fors_treehashx1
SPX_gen_message_random  SPX_hash_message        SPX_initialize_hash_function
SPX_merkle_gen_root     SPX_merkle_sign         SPX_prf_addr
SPX_set_chain_addr      SPX_set_hash_addr       SPX_set_keypair_addr
SPX_set_layer_addr      SPX_set_tree_addr       SPX_set_tree_height
SPX_set_tree_index      SPX_set_type            SPX_thash
SPX_treehash            SPX_u32_to_bytes        SPX_ull_to_bytes
SPX_wots_gen_leafx1     SPX_wots_pk_from_sig    SPX_wots_treehashx1
crypto_sign             crypto_sign_bytes       crypto_sign_keypair
crypto_sign_open        crypto_sign_publickeybytes
crypto_sign_secretkeybytes                      crypto_sign_seed_keypair
crypto_sign_seedbytes   crypto_sign_signature   crypto_sign_verify
randombytes             randombytes_init        seedexpander
seedexpander_init
```

`SPX_*` names come from `SPX_NAMESPACE(s) => SPX_##s` in the parameter headers,
i.e. they are macro-generated and must be reproduced verbatim.

### Backend-specific additions

| backend | extra symbols | total |
|---|---|---|
| `haraka` | `SPX_haraka256` `SPX_haraka512` `SPX_haraka512_perm` `SPX_haraka_S` `SPX_haraka_S_inc_absorb` `SPX_haraka_S_inc_finalize` `SPX_haraka_S_inc_init` `SPX_haraka_S_inc_squeeze` `SPX_tweak_constants` | 56 |
| `sha2` | `SPX_mgf1_256` `SPX_mgf1_512` `SPX_seed_state` `sha256` `sha256_inc_blocks` `sha256_inc_finalize` `sha256_inc_init` `sha512` `sha512_inc_blocks` `sha512_inc_finalize` `sha512_inc_init` | 58 |
| `shake` | `shake256` `shake256_absorb` `shake256_inc_absorb` `shake256_inc_finalize` `shake256_inc_init` `shake256_inc_squeeze` `shake256_squeezeblocks` | 54 |
| `blake` | `SPX_blake256_mgf1` `SPX_blake512_mgf1` `blake256` `blake256_compress` `blake256_final` `blake256_init` `blake256_update` `blake512` `blake512_compress` `blake512_final` `blake512_init` `blake512_update` `cst` | 60 |

`shake`: `lib/shake/include/fips202.h` *declares* `shake128*`, `sha3_256*` and
`sha3_512*`, but `lib/shake/src/fips202.c` never **defines** them — the only
non-`static` definitions in that file are the seven `shake256*` names above
(verified with `nm` on the built `libshake.so`, not assumed from the header).
Those declared-but-undefined names are therefore correctly absent from both
libraries.

`cst` (`lib/blake/src/blake512.c:45`, `const u64 cst[16]`) is **not** `static`
in the C source and therefore *is* an exported data symbol. `blake256.c` has a
`static const u32 cst[16]` which is correctly *not* exported.

## Findings and fixes

| symbol | status before | action taken |
|---|---|---|
| `cst` | MISSING from Rust `.so` in all 12 `blake` configurations. Implementation existed as a private `static CST: [u64;16]` in `src/blake/blake512.rs`; it was simply not exported. | Renamed to `#[unsafe(no_mangle)] pub static cst: [u64; 16]` (with an internal `use ... as CST` alias so the `round!` invocations are unchanged). Not a stub — it is the real constant table already used by `blake512_compress`. |
| everything else | present | none |

No whole C module was found untranslated: every one of the 15 C `.c` files that
CMake compiles into the three libraries has a corresponding Rust module
(`address`, `fors`, `merkle`, `sign`, `utils`, `utilsx1`, `wots`, `wotsx1`,
`rng`, `randombytes`, plus `<backend>/{hash,thash_*}` and the backend
primitives `blake256`/`blake512`/`sha2`/`fips202`/`haraka`).

## Verification

`scripts/sym_diff.sh` builds all 48 C configurations and all 48 matching Rust
feature combinations and diffs the symbol sets both ways.

```
$ ./scripts/sym_diff.sh
OK   haraka-robust-128s  (56 symbols)
...
OK   blake-simple-256f  (60 symbols)
$ echo $?
0
```

Result after the `cst` fix: **48/48 configurations report 0 missing and 0 extra
symbols.**

Undefined symbols in the Rust `.so` (`nm -D --undefined-only`) are all libc /
`libgcc` unwinder imports (`memcpy`, `malloc`, `read`, `_Unwind_*`, …) — there
are **0 undefined non-libc symbols**.
