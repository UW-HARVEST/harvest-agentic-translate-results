# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on the C shared objects, for
**all 48 build configurations** (`HASH_BACKEND` × `THASH` × `SECPAR`).

## How the C side is built

`c_src` produces **three** shared objects per configuration:

| .so | sources | notes |
|-----|---------|-------|
| `app/libsphincs_core.so`     | `address.c fors.c merkle.c sign.c utils.c utilsx1.c wots.c wotsx1.c` + `randombytes.c` | non-deterministic `randombytes` (`/dev/urandom`) |
| `app/libsphincs_core_det.so` | same objects + `rng.c` | deterministic AES-256-CTR-DRBG `randombytes` (used by the KAT driver) |
| `lib/<backend>/lib<backend>.so` | backend hash + thash + (`utils.c` for blake/sha2) | |

The Rust crate produces **one** `cdylib` (`libsphincs_plus.so`) that must export
the **union** of those symbols. `randombytes.c` and `rng.c` both define a symbol
literally named `randombytes`; a single artifact cannot export it twice, so the
Rust crate exports the **deterministic (`rng.c`)** one — matching what the
`driver` executable links against — and keeps the `/dev/urandom` translation
available as `randombytes_urandom` (`src/randombytes.rs`).

## Symbol reference (union over all 48 configs)

### Always present (core, all 48 configs) — 38 symbols

| symbol | C source | Rust source |
|--------|----------|-------------|
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
| `SPX_set_layer_addr`         | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_addr`          | `app/src/address.c` | `src/address.rs` |
| `SPX_set_type`               | `app/src/address.c` | `src/address.rs` |
| `SPX_copy_subtree_addr`      | `app/src/address.c` | `src/address.rs` |
| `SPX_set_keypair_addr`       | `app/src/address.c` | `src/address.rs` |
| `SPX_copy_keypair_addr`      | `app/src/address.c` | `src/address.rs` |
| `SPX_set_chain_addr`         | `app/src/address.c` | `src/address.rs` |
| `SPX_set_hash_addr`          | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_height`        | `app/src/address.c` | `src/address.rs` |
| `SPX_set_tree_index`         | `app/src/address.c` | `src/address.rs` |
| `SPX_ull_to_bytes`           | `app/src/utils.c` | `src/utils.rs` |
| `SPX_u32_to_bytes`           | `app/src/utils.c` | `src/utils.rs` |
| `SPX_bytes_to_ull`           | `app/src/utils.c` | `src/utils.rs` |
| `SPX_compute_root`           | `app/src/utils.c` | `src/utils.rs` |
| `SPX_treehash`               | `app/src/utils.c` | `src/utils.rs` |
| `SPX_wots_treehashx1`        | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `SPX_fors_treehashx1`        | `app/src/utilsx1.c` | `src/utilsx1.rs` |
| `SPX_wots_pk_from_sig`       | `app/src/wots.c` | `src/wots.rs` |
| `SPX_chain_lengths`          | `app/src/wots.c` | `src/wots.rs` |
| `SPX_wots_gen_leafx1`        | `app/src/wotsx1.c` | `src/wotsx1.rs` |
| `SPX_fors_gen_leafx1`        | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_sign`              | `app/src/fors.c` | `src/fors.rs` |
| `SPX_fors_pk_from_sig`       | `app/src/fors.c` | `src/fors.rs` |
| `SPX_merkle_sign`            | `app/src/merkle.c` | `src/merkle.rs` |
| `SPX_merkle_gen_root`        | `app/src/merkle.c` | `src/merkle.rs` |
| `SPX_initialize_hash_function` | `lib/<b>/src/hash_<b>.c` | `src/<b>_hash.rs` |
| `SPX_prf_addr`               | `lib/<b>/src/hash_<b>.c` | `src/<b>_hash.rs` |
| `SPX_gen_message_random`     | `lib/<b>/src/hash_<b>.c` | `src/<b>_hash.rs` |
| `SPX_hash_message`           | `lib/<b>/src/hash_<b>.c` | `src/<b>_hash.rs` |
| `SPX_thash`                  | `lib/<b>/src/thash_<b>_<t>.c` | `src/<b>_thash.rs` |

### Deterministic RNG (`app/src/rng.c`, all 48 configs) — 7 symbols

| symbol | kind | Rust source |
|--------|------|-------------|
| `randombytes`             | fn (`int(u8*, unsigned long long)`) | `src/rng.rs` |
| `randombytes_init`        | fn (`void(u8*, u8*)`)               | `src/rng.rs` |
| `AES256_ECB`              | fn (`void(u8*, u8*, u8*)`)          | `src/rng.rs` |
| `AES256_CTR_DRBG_Update`  | fn (`void(u8*, u8*, u8*)`)          | `src/rng.rs` |
| `seedexpander_init`       | fn (`int(AES_XOF_struct*, u8*, u8*, unsigned long)`) | `src/rng.rs` |
| `seedexpander`            | fn (`int(AES_XOF_struct*, u8*, unsigned long)`)      | `src/rng.rs` |
| `DRBG_ctx`                | **data** (`AES256_CTR_DRBG_struct`, 52 bytes) | `src/rng.rs` |

### Backend-specific

#### `HASH_BACKEND=blake` (12 configs) — 11 extra symbols
`blake256`, `blake256_init`, `blake256_update`, `blake256_final`,
`blake256_compress`, `blake512`, `blake512_init`, `blake512_update`,
`blake512_final`, `blake512_compress`, `SPX_blake256_mgf1`,
`SPX_blake512_mgf1`, and the **data** symbol `cst`
(`const u64 cst[16]` in `lib/blake/src/blake512.c` — non-`static`, therefore
exported; note `blake256.c`'s `cst` *is* `static` and is **not** exported).
Rust: `src/blake256.rs`, `src/blake512.rs`, `src/blake_hash.rs`.

#### `HASH_BACKEND=sha2` (12 configs) — 11 extra symbols
`sha256`, `sha256_inc_init`, `sha256_inc_blocks`, `sha256_inc_finalize`,
`sha512`, `sha512_inc_init`, `sha512_inc_blocks`, `sha512_inc_finalize`,
`SPX_mgf1_256`, `SPX_mgf1_512`, `SPX_seed_state`.
Rust: `src/sha2.rs`, `src/sha2_hash.rs`.

#### `HASH_BACKEND=shake` (12 configs) — 7 extra symbols
`shake256`, `shake256_absorb`, `shake256_squeezeblocks`,
`shake256_inc_init`, `shake256_inc_absorb`, `shake256_inc_finalize`,
`shake256_inc_squeeze`.
Rust: `src/fips202.rs`, `src/shake_hash.rs`.
(`fips202.c` also *defines* `shake128*` / `sha3_*`, but those are `static`-free
yet **not** exported by `libshake.so` in this build — only the 7 above appear in
`nm -D`; the table is derived from `nm -D`, not from the header.)

#### `HASH_BACKEND=haraka` (12 configs) — 8 extra symbols
`SPX_tweak_constants`, `SPX_haraka_S`, `SPX_haraka_S_inc_init`,
`SPX_haraka_S_inc_absorb`, `SPX_haraka_S_inc_finalize`,
`SPX_haraka_S_inc_squeeze`, `SPX_haraka512_perm`, `SPX_haraka512`,
`SPX_haraka256`.
Rust: `src/haraka.rs`, `src/haraka_hash.rs`.

## Per-config diff results

Automated by `.verify/symdiff.sh` (builds the Rust cdylib for each of the 48
feature combos, diffs `nm -D` against the union of the three C `.so`s for the
same combo).

### Initial run (before fixes)

Every one of the 48 configs was missing the same 5 `rng.c` symbols, and the 12
`blake` configs additionally missed `cst`:

```
MISSING [*,*,*]        : AES256_CTR_DRBG_Update AES256_ECB DRBG_ctx seedexpander seedexpander_init
MISSING [blake,*,*]    : ... + cst
```

Cause classification (per the Phase A rule):
* `seedexpander`, `seedexpander_init` — implementation **existed** in
  `src/rng.rs` but had no `#[no_mangle]` / `extern "C"` wrapper → wrapper added.
* `AES256_ECB`, `AES256_CTR_DRBG_Update` — implementation **existed** as private
  Rust helpers (`aes256_ecb`, `aes256_ctr_drbg_update`) → wrappers added.
* `DRBG_ctx` — the DRBG state existed but was a private `Mutex<Drbg>`, so the C
  global was not observable. Restructured to a real exported
  `#[no_mangle] pub static mut DRBG_ctx: AES256_CTR_DRBG_struct` (`#[repr(C)]`,
  `Key[32] || V[16] || int reseed_counter`) which the Rust `randombytes` /
  `randombytes_init` now operate on directly, exactly as the C does.
* `cst` — the BLAKE-512 round constants existed as a private const in
  `src/blake512.rs` → re-exported as `#[no_mangle] pub static cst: [u64; 16]`.

No symbol required translating a previously-untranslated C module: all 8 C
`app/src` files, all 4 backends and both `thash` variants already have Rust
counterparts (`wc -l` on `translation/src` = 29 files / ~5.8 kloc vs. 8.5 kloc C
including the 24 params headers and the driver).

### Final run (after fixes)

`.verify/run_combo.sh` recomputes the diff for every combination as part of the
test run and writes `SYMBOLS OK` / `SYMBOLS MISSING: ...` to
`.verify/logs/<b>_<t>_<s>.log`:

```
$ grep -l "SYMBOLS OK" .verify/logs/*.log | wc -l
48
$ tot=0; for f in .verify/logs/csym_*.txt; do
    r=".verify/logs/rsym_${f#.verify/logs/csym_}"
    d=$(comm -23 "$f" "$r"); [ -n "$d" ] && { echo "MISSING: $d"; tot=$((tot+1)); }
  done; echo "combos with missing symbols: $tot / 48"
combos with missing symbols: 0 / 48
```

**0 missing symbols in all 48 configurations.**

## Undefined (imported) symbols

`nm -D --undefined-only` on `libsphincs_plus.so` lists only libc / libgcc /
`ld-linux` symbols (`memcpy`, `__errno_location`, `_Unwind_*`, …) plus Rust
`std` internals. The C `libsphincs_core*.so` additionally import the backend
hash symbols (`SPX_thash`, `SPX_prf_addr`, …) because CMake does **not** link
the backend into the core `.so` — only the `driver` executable links both. The
Rust cdylib is self-contained, so it has strictly fewer undefined symbols; there
are **0 undefined non-libc symbols** in the Rust `.so`.
