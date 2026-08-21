# VERIFICATION.md — how the C↔Rust equivalence was established

## Configuration surface

The C build has three cache variables (`c_src/CMakeLists.txt`), mirrored 1:1 by
Cargo features with the same lowercase names:

| CMake | values | Cargo features |
|---|---|---|
| `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake` | `haraka`, `sha2`, `shake`, `blake` |
| `THASH` | `robust`, `simple` | `robust`, `simple` |
| `SECPAR` | `128s`, `128f`, `192s`, `192f`, `256s`, `256f` | `"128s"`, `"128f"`, `"192s"`, `"192f"`, `"256s"`, `"256f"` |

**4 × 2 × 6 = 48 valid combinations.** Exactly one value per axis is valid, so
there are no other combinations. Every script below iterates over all 48.

## Scripts

| script | what it does |
|---|---|
| `./build_c_all.sh` | builds the C reference (`cmake` + `cmake --build`) for all 48 combinations into `cbuild/<backend>-<thash>-<secpar>/` |
| `./check_all.sh`   | `cargo check --all-targets` for the default features + all 48 combinations |
| `./symdiff.sh`     | `nm -D --defined-only` diff of the C `.so`s vs the Rust `cdylib`, all 48 combinations |
| `./run_all.sh`     | builds the Rust `cdylib` per combination, snapshots it, and runs `tests/differential.rs` against the matching C `.so`s — all 48 combinations |
| `./kat_all.sh`     | runs the C `driver` and the Rust `driver` for all 48 combinations and compares the 32-byte KAT transcript digests (also against `expected_kat.txt`) |
| `./verify_all.sh`  | pre-existing: Rust `driver` digest vs `expected_kat.txt` |

> `cargo test` does **not** rebuild the `cdylib`, so `run_all.sh` builds it
> explicitly and points the harness at it with `SPHINCS_RUST_SO`. The harness
> also asserts that the loaded `.so` reports the expected `crypto_sign_bytes()`
> and exports the expected backend marker symbol, so a stale `.so` fails loudly
> instead of silently passing.

## How the differential tests avoid being vacuous

`tests/differential.rs` reaches **both** implementations only through
`dlopen`/`dlsym` — no Rust function is ever called directly, so the
`#[no_mangle]`/`extern "C"` export wrappers are part of what is tested.

Two hazards had to be designed around, both in `tests/common/mod.rs`:

1. **The test executable must not define any SPHINCS+ symbol.** The main
   executable sits at the front of the dynamic linker's global search scope, so
   if the `sphincsplus` crate were linked into the test binary the C libraries
   would bind *their* internal calls (`SPX_thash`, `SPX_prf_addr`, …) to the Rust
   implementations and every comparison would become a tautology. `common/mod.rs`
   therefore re-derives all parameters from `cfg!(feature = …)` instead of
   importing `sphincsplus::params`.
2. **Load order.** `libsphincsplus.so` is opened **first** with
   `RTLD_NOW | RTLD_LOCAL`: at that moment nothing in the process defines any of
   these symbols, so every one of its relocations binds to itself, and being
   `RTLD_LOCAL` it never enters the global scope. Only afterwards are
   `lib<backend>.so` and `libsphincs_core_det.so` opened with
   `RTLD_LAZY | RTLD_GLOBAL` (`RTLD_LAZY` is required because their dependency is
   circular: the backend needs `SPX_set_tree_index` from the core, the core needs
   `SPX_thash` from the backend). `libcrypto` is loaded **last**, so it can never
   shadow a SPHINCS+ symbol.

The harness additionally asserts `C_SPX_thash != Rust_SPX_thash` and
`C_crypto_sign_verify != Rust_crypto_sign_verify` (distinct addresses) before
running anything, and that both `.so`s report the expected
`crypto_sign_bytes()` and export the expected backend marker symbol.

### Negative controls (proof that the harness detects divergence)

| control | result |
|---|---|
| `blake/simple` Rust `.so` vs `blake/**robust**` C `.so` | `test result: FAILED. 61 passed; 41 failed` — every hash-dependent test reports `MISMATCH` (`thash cross-ctx`, `SPX_compute_root`, `SPX_fors_sign`, `crypto_sign_signature`, …); the 61 that still pass are the ones that legitimately do not depend on `thash` (address setters, `ull_to_bytes`, AES/DRBG, raw `blake*` primitives, …) |
| `blake/simple/**128f**` Rust `.so` vs `blake/simple/**192f**` C `.so` | FAILS immediately on the `crypto_sign_bytes()` configuration assert |
| `**shake**` Rust `.so` vs `blake` C `.so` | FAILS immediately: "`neg2.so` does not export `blake256`: it is not a blake build" |

So a passing run is a real byte-for-byte agreement, not a symbol-resolution
artefact or a silently-skipped test.

### Which C library is the reference

`libsphincs_core_det.so` (the `rng.c` DRBG variant) is the comparison target,
because that is the `randombytes` the Rust `cdylib` exports and the one the
reference `driver` links against — see `SYMBOLS.md`. `libcrypto` must be
`dlopen`ed by the harness because CMake links `crypto` only into the `driver`
executable, not into `libsphincs_core_det.so`.

## Results

```
$ ./check_all.sh
check_all: 49 configurations, 0 failed

$ ./symdiff.sh
TOTAL MISSING=0            # 48 combinations, 0 missing symbols each

$ ./run_all.sh
run_all: pass=48 fail=0    # 102 differential tests per combination

$ ./kat_all.sh
kat_all: pass=48 fail=0    # C driver digest == Rust driver digest == expected_kat.txt

$ cargo test --release --test urandom
test result: ok. 4 passed
```

* `SYMBOLS.md` — every symbol the C `.so`s export is exported by the Rust
  `.so`, in all 48 combinations; 0 missing, 0 undefined non-libc symbols.
* `CONFIGS.md` — 53 rows, every one exercised with randomized inputs
  (SplitMix64, fixed seed `0x5150_4849_4E43_5321`) in all 48 combinations.
* `ERRORS.md` — 57 rows; 54 have an executed differential test, 3 are C paths
  whose trigger is a crash or an effectively infinite loop in the C itself and
  are documented rather than executed (the Rust reproduces the same
  expression/UB).

## Fixes made to the Rust during verification (the C was never changed)

| # | problem | fix |
|---|---|---|
| 1 | `AES256_ECB` not exported (folded into a private helper) | `#[no_mangle] pub unsafe extern "C" fn AES256_ECB` in `src/rng.rs`; `seedexpander`/`randombytes`/`AES256_CTR_DRBG_Update` now route through it, as the C does |
| 2 | `DRBG_ctx` not exported (a private `Mutex<Drbg>` hid the observable global DRBG state) | `#[no_mangle] pub static mut DRBG_ctx: AES256_CTR_DRBG_struct`, operated on in place exactly like `rng.c` |
| 3 | `cst` not exported (`blake512.c` declares `const u64 cst[16]` **without** `static`) | `#[no_mangle] pub static cst: [u64; 16]` in `src/backends/blake/blake512.rs` |
| 4 | `seedexpander`: `16 - ctx->buffer_pos` underflows in C's `unsigned long`; Rust used plain `-` (panics in a debug profile) | `wrapping_sub`, so the branch taken matches the C for any `buffer_pos` (row E25) |
| 5 | `size_of::<SpxCtx>()` was 144 vs C's 72 for `sha2-128s`/`sha2-128f` (`state_seeded_512` was unconditional, but `context.h` guards it with `#if SPX_SHA512`) | field length is now `STATE_SEEDED_512_LEN = if SPX_SHA512 { 72 } else { 0 }`; sizes match C for all 12 backend×secpar pairs |
| 6 | `SPX_bytes_to_ull`: shift count `8*(inlen-1-i)` is ≥ 64 for `inlen > 8`; Rust's `<<` panics in a debug profile where the compiled C masks the count | `wrapping_shl`, which masks identically to x86-64 `shl` (row E35) |
| 7 | `SPX_compute_root`: `tree_height - 1` is `uint32_t` arithmetic in C and wraps for `tree_height == 0` | `wrapping_sub` (row E43) |

## A genuine C bug that is faithfully reproduced

`lib/blake/src/hash_blake.c` passes **byte** counts to `blake256_update` /
`blake512_update`, which take a length in **bits**
(`memcpy(..., (datalen >> 3) & 0x3F)`). Consequently, for `HASH_BACKEND=blake`
only about `mlen/8` bytes of the message reach the message digest, and
`crypto_sign_verify` **accepts** a signature after the message tail is modified.
This was discovered by the Phase-C test `err_verify_corrupt_msg` and confirmed
with a standalone C-only program linked against the reference `.so`s. The Rust
reproduces it exactly (byte-identical over 63 different `mlen` values in
`cfg_gen_message_random`/`cfg_hash_message`, and in the KAT transcripts). The
*test expectation* was corrected, never the C. See `ERRORS.md` (E6 and the
"Additional finding" section).
