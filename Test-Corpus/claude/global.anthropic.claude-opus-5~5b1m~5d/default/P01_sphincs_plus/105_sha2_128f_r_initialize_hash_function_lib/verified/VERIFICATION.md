# VERIFICATION.md — how to reproduce, and what was found

## Reproducing

```
# one configuration (builds the C .so + Rust cdylib, checks nm -D parity, runs all tests)
scripts/run_cfg.sh <backend> <secpar> <thash>          # e.g. blake 128f simple

# all 48 configurations, 12 at a time (each in its own CARGO_TARGET_DIR)
PAR=12 scripts/verify_all.sh

# just the compile matrix
scripts/check_all.sh
```

Prerequisites on this host: `clang`, `cmake`, `nm`, and `cargo --offline`
(`libloading 0.8` is already in the local registry cache). `rng.c` needs
OpenSSL headers, which come from the nix store; the *system* `libcrypto.so.3`
is linked (see `scripts/build_c.sh`).

## Harness design

* `tests/common/mod.rs` loads BOTH shared objects with `libloading` and resolves
  every function through `dlsym`. No function of the Rust crate is ever called
  directly, so the `#[no_mangle]` / `extern "C"` export wrappers are themselves
  under test.
* Parameters in the harness (`N`, `SPX_BYTES`, `WOTS_LEN`, ...) are re-derived
  from the C headers independently of `src/params.rs`, and `Pair::load()`
  cross-checks them against `crypto_sign_bytes()` from *both* libraries. This
  also guards against accidentally testing a stale artifact from a different
  feature set.
* All output buffers are over-allocated with a `0xA5` canary tail (`obuf()`) and
  compared in full, so "the C writes more bytes than expected" shows up as a
  diff instead of as heap corruption. This is exactly how divergence #1 below
  was caught.
* Randomised inputs use splitmix64 with a fixed per-test seed, so every failure
  is reproducible.
* `spx_ctx`, `leaf_info_x1`, `fors_gen_leaf_info`, `AES256_CTR_DRBG_struct`,
  `AES_XOF_struct` and the BLAKE states are built as raw C-layout byte images in
  the tests and compared in full after each call, so struct-layout and
  side-effect divergences are caught, not just return values.
* Tests that touch the process-wide `DRBG_ctx` take a mutex (cargo runs tests in
  parallel threads and `dlopen` of the same path shares one mapping).

## Divergences found and fixed (Rust side only; `c_src/` untouched)

1. **`gen_message_random` output length (BLAKE backend).**
   `lib/blake/src/hash_blake.c:68` calls `blakeX_final(&S, R)`, which writes the
   FULL digest — 32 bytes for the 128-bit level, 64 for 192/256 — into `R`, not
   `SPX_N` bytes. The Rust version wrote only `SPX_N`. Fixed in
   `src/blake_hash.rs` (and `src/sign.rs` now hands over the whole `sig` buffer,
   as the C does). Caught by the canary buffers in `b13`–`b15`.
2. **Missing `#[repr(C)]`** on every struct crossing the FFI boundary:
   `SpxCtx`, `LeafInfoX1`, `ForsGenLeafInfo`, `BlakeState256`, `BlakeState512`.
   Without it rustc may reorder fields, so a C caller's struct would be
   misinterpreted (`leaf_info_x1` in particular mixes pointers and `u32`s).
3. **Six symbols were implemented but not exported**: `AES256_ECB`,
   `AES256_CTR_DRBG_Update`, `seedexpander`, `seedexpander_init`, the `DRBG_ctx`
   global (which also had to be restructured from a `Mutex<...>` into a
   `#[repr(C)] static mut` with the exact C layout so external callers can read
   and patch it), and the `cst` read-only data symbol of `blake512.c`.
4. **`crypto_sign_verify` dereferenced before validating `siglen`.**
   `sign.c:179` returns `-1` on a wrong `siglen` *before* touching `m`/`pk`, so a
   caller may legitimately pass NULL for them. Rust now checks first
   (`src/sign.rs`).
5. **Over-broad exports removed**: the Rust `fips202` translation exported
   `shake128*`, `sha3_256*` and `sha3_512*`. `fips202.c` only *declares* those in
   the header — it contains no definitions, and `libshake.so` does not export
   them. They are no longer `#[no_mangle]`, so the exported surface now matches
   the C `.so` exactly in both directions.

## C behaviours deliberately reproduced (not "fixed")

* `hash_blake.c` passes **byte** counts to `blakeX_update`, which interprets its
  length argument as a **bit** count (only `blake256()`/`blake512()` multiply by
  8). Consequence: for the BLAKE backend only the first `mlen/8` message bytes
  reach the message digest. `e07_open_verify_fails` documents this — corrupting a
  trailing message byte legitimately still verifies.
* `hash_blake.c`'s `prf_addr` copies `sk_seed` into its buffer but hashes only
  `SPX_N + SPX_ADDR_BYTES` bytes, so `sk_seed` never reaches the PRF.
* `hash_haraka.c`'s `hash_message` absorbs `pk + SPX_N` (the root only), unlike
  the other backends which absorb all of `SPX_PK_BYTES`.
* `bytes_to_ull(in, inlen)` with `inlen > 8` shifts by ≥ 64, which is UB in C.
  `e32_bytes_to_ull_oversized` pins whatever the compiled C `.so` does and
  asserts the Rust `.so` agrees (it does on this toolchain — both end up with a
  masked shift).
* `thash(..., inblocks = 0)` is degenerate but accepted; both agree (`e38`).
* `randombytes(x, 0)` writes nothing but still runs the DRBG update and bumps
  `reseed_counter` (`e18`).

## Notes / limitations

* `app/src/randombytes.c` (the `/dev/urandom` variant) is translated as
  `randombytes_urandom` in `src/randombytes.rs` but is **not exported**: it and
  `rng.c` both define `randombytes`, only one can exist in a single object, and
  the KAT driver links the deterministic `rng.c` one. Its behaviour is
  nondeterministic and therefore not differentially testable.
* `app/src/PQCgenKAT_sign.c` is the `driver` executable's `main()`; it exports no
  library symbols and needs no translation. (The `driver` target also cannot be
  linked on this host — the only available `libcrypto` with headers requires a
  newer glibc than the system linker offers. No `.so` is affected.)
* Backend/thash/secpar selection uses `cfg(spx_backend=...)` etc. emitted by
  `build.rs` rather than raw `cfg(feature=...)`. This is what makes *all* 48
  feature combinations compile, including contradictory ones such as
  `--features haraka,blake,robust,simple,128s,256f`, by resolving them to a
  single selection with the same priority/default rules as the CMake cache
  variables.
* `RUST_MIN_STACK` is raised by `scripts/run_cfg.sh`: cargo's 2 MiB test-thread
  stacks can be tight for the deep sign/verify chains of the 256-bit parameter
  sets. This is a harness limit, not a translation difference.

## Independent code audit (in addition to the differential tests)

Every backend and every core module was additionally reviewed line-by-line
against its C counterpart, specifically hunting for the bug classes above
(output-buffer length mismatches, FFI integer widths, missing `#[repr(C)]`,
`<` vs `<=`, wrapping arithmetic, and read-length differences). Findings:

* `sha2.c`, `hash_sha2.c`, `thash_sha2_{simple,robust}.c`, `fips202.c`,
  `hash_shake.c`, `thash_shake_{simple,robust}.c` — no divergences.
* `haraka.c`, `hash_haraka.c`, `thash_haraka_{simple,robust}.c`, `blake256.c`,
  `blake512.c`, `wots.c`, `wotsx1.c`, `fors.c`, `merkle.c`, `utilsx1.c` — no
  divergences. Both bit-sliced AES S-boxes and both BLAKE compression functions
  were checked mechanically (the unrolled C `ROUND(...)` macros were re-derived
  and symbolically compared against the Rust `g()` + `SIGMA[r % 10]` form).
* Remaining differences are all on states unreachable from the public API
  (e.g. `compute_root(tree_height = 0)`, or a `blakestate` whose `buflen` has
  already been corrupted past the block size) and are recorded as rows 40-42 of
  `ERRORS.md`.
* One noteworthy C quirk confirmed: `utilsx1.c:104` declares `fors_treehashx1`'s
  `info` as `leaf_info_x1 *` but hands it to `fors_gen_leafx1`, which expects
  `fors_gen_leaf_info *` (the `IAN TODO` comment at `utilsx1.c:116`; it compiles
  with an incompatible-pointer warning). Only the first 32 bytes are ever
  touched, so typing the Rust wrapper as `*mut ForsGenLeafInfo` is
  behaviourally identical.

## Result

| gate | status |
|---|---|
| `cargo check` for all 48 feature combinations | 48/48 pass |
| `nm -D` C-vs-Rust symbol diff (both directions) | empty for 48/48 |
| Rust `nm -D --undefined-only` non-libc symbols | none |
| `CONFIGS.md` rows (52) — differential, randomised | all pass, 48/48 configs |
| `ERRORS.md` rows (42; 39 with tests, 3 documented unreachable) | all pass, 48/48 configs |
