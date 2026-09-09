# VERIFICATION.md — how this translation was verified

libsodium 1.0.23, C (`c_src/libsodium`) vs Rust (`translation/src`).

Everything below is reproducible with two commands:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . -j8
cd translation && ./verify.sh          # symbol parity + every feature combo + all suites
```

`./dt.sh [cargo-test-args]` runs a single suite (it rebuilds the cdylib first —
`cargo test` alone does **not**, because the lib target is `cdylib`-only, so a
bare `cargo test` would silently test a stale `.so`. The harness also panics
with `STALE Rust cdylib` if it detects that situation.)

## Method

Both libraries are loaded as shared objects with `libloading` and driven ONLY
through their dynamic symbol tables — the Rust code is never called directly, so
every `#[no_mangle] extern "C"` wrapper is exercised exactly as an external
consumer would exercise it.

`tests/common/mod.rs` is the shared harness:

| helper | purpose |
|---|---|
| `libs()` | both `Library` handles, both `sodium_init`ed |
| `pair::<F>("sym")` | the same symbol from C and Rust, typed |
| `eq_bytes` / `eq_i32` | byte-for-byte and return-value comparison with a hex diff on failure |
| `errno()` / `set_errno()` | compares the `errno` side effect (both libraries share the calling thread's `errno`) |
| `install_det_random()` / `det_reseed()` | installs ONE deterministic `randombytes_implementation` into BOTH libraries, making `*_keypair`, `*_keygen`, `*_random`, `crypto_pwhash_str*`, `crypto_secretstream_*_init_push`, `crypto_box_seal` fully byte-comparable |
| `diff_abort_case("case")` | re-executes the test binary in two child processes (one per library) and asserts they terminated with the same exit code / signal — the only way to compare `sodium_misuse()` / `assert()` / `_out_of_bounds()` rejections |
| `Rng::new(seed)` | fixed-seed input generator; every configuration is driven with many randomized inputs, not one hand-picked value |

Inputs are randomized from fixed seeds, so failures are reproducible.

## Phase A — surface maps

| artifact | content |
|---|---|
| `SYMBOLS.md` | all 890 dynamic symbols of the C `.so`, each confirmed present in the Rust `.so`. Diff empty both ways; 0 undefined non-libc symbols. |
| `ERRORS.md` | **739 rows** — every distinct rejection path in the C, grepped mechanically (`return -1`, `return NULL`, failure-`return 0`, `ARGON2_*` enums, `sodium_misuse()`, `abort()`, `assert(`, range/size/overflow checks, null checks, `errno =`), each with its exact trigger, its expected C result, and the test that covers it. |
| `CONFIGS.md` | **995 rows** — the valid-input mirror: every runtime option/mode/flag and every input shape the C branches on, crossed and pruned to the combinations the code actually distinguishes, including the lowest-level entry points (`*_beforenm`/`*_afternm`, `*_detached`, `*_xor_ic`, the NaCl zero-padded forms, `init`/`update`/`final` streaming APIs, `*_ll`). |

Both tables are regenerated (with the row→test mapping recomputed from the test
sources) by `python3 _logs/gen_tables.py`.

## Phase B / C — differential suites

| suite | phase | group | `#[test]`s |
|---|---|---|---|
| `tests/t00_smoke.rs` | — | harness self-check | 4 |
| `tests/t01_g1_utils.rs` | B | utils / codecs / core / runtime / verify | 18 |
| `tests/t13_g1_extra.rs` | B | the G1 rows `t01` does not reach | 13 |
| `tests/t02_g1_errors.rs` | C | utils / codecs / core / verify | 18 |
| `tests/t11_g2_pwhash.rs` | B | argon2 + scrypt | 22 |
| `tests/t12_g2_pwhash_errors.rs` | C | argon2 + scrypt | 21 |
| `tests/t03_g3_hash.rs` | B | blake2b / sha2 / sha3 / xof / hmac / poly1305 / siphash / kdf | 23 |
| `tests/t04_g3_hash_errors.rs` | C | same | 13 |
| `tests/t05_g4_aead.rs` | B | aead / secretbox / secretstream / stream / core | 65 |
| `tests/t06_g4_aead_errors.rs` | C | same | 32 |
| `tests/t07_g5_asym.rs` | B | box / kx / scalarmult / sign / ed25519+ristretto core | 23 |
| `tests/t08_g5_asym_errors.rs` | C | same | 20 |
| `tests/t09_g6_rand.rs` | B | randombytes / ML-KEM-768 / X-Wing / ipcrypt | 41 |
| `tests/t10_g6_rand_errors.rs` | C | same | 25 |
| `tests/t14_large_inputs.rs` | B | inputs past every internal chunk/rate boundary | 11 |
| **total** | | | **349** |

All 349 pass; `ERRORS.md` and `CONFIGS.md` have 0 uncovered rows.

## Divergences found and fixed

Four real bugs, all fixed in `translation/src` (the C was never touched):

1. **`src/blake2b.rs`, `crypto_generichash_blake2b_final`** — did `outlen as u8`,
   silently truncating, so `outlen = 288` returned 0 and wrote 32 bytes. The C
   hits `assert(outlen <= UINT8_MAX)` and dies with SIGABRT. Added the guard.
2. **`src/randombytes_sysrandom.rs`, `randombytes_linux_getrandom`** — missing
   the C `assert(chunk_size > 0)`, so calling the public
   `randombytes_sysrandom_implementation.buf(ptr, 0)` vtable entry returned
   quietly in Rust while the C aborted. Added the assert.
3. **`src/randombytes_internal.rs`** — the same missing assert in the identical
   chunking wrapper.
4. **`src/argon2.rs`, `src/argon2_encoding.rs`, `src/pwhash_argon2.rs`** —
   `argon2_type` was modelled as a 2-variant `#[repr(C)] enum`, which dropped
   the C's reachable `default:` arms and made out-of-range values UB. An
   out-of-range `type_` (a real input, since C enums accept any `int`) returned
   `0` in Rust but `ARGON2_INCORRECT_TYPE` in C. Replaced with a faithful
   `#[repr(transparent)] struct argon2_type(pub c_int)` and restored the
   `default:` arms in `argon2_ctx`, `argon2_decode_string` and
   `argon2_encode_string`.

Bugs 1-3 were only observable because the reference C build is configured with
an empty `CMAKE_BUILD_TYPE`: no `-DNDEBUG`, so every C `assert()` is live.

## Why the "everything else matched" result is trustworthy

Zero divergence in a translation this large is suspicious, so the suites were
mutation-tested: a single line of the Rust was deliberately broken and the
suites had to catch it. Independently verified mutants (each reverted
byte-identically afterwards):

| mutated | suite that caught it |
|---|---|
| `sodium_codecs.rs` hex digit `b'a'` → `b'A'` | `t01_g1_utils::g1_ip2bin_bin2ip` |
| `ed25519_ref10_ge.rs` sign bit `<< 7` → `<< 6` | `t07_g5_asym` |
| `scrypt.rs` salsa20 rotation `7` → `8` | `t11_g2_pwhash` |
| `softaes.rs` AES round `planes[5]` term | `t05_g4_aead` (7 tests), `t14_large_inputs` (2) |
| `softaes.rs` `SBOX[15]` `0x76` → `0x77` | `t09_g6_rand` (9 tests, via ipcrypt's `encryptlast`) |
| `ipcrypt.rs` degenerate-key `^ 0x5a` → `^ 0x5b` | `t09_g6_rand` |
| `kem_mlkem768_ref.rs` Montgomery constant `62209` → `62208` | `t09_g6_rand` |
| `auth_hmac.rs` ipad `0x36` → `0x37` | `t03_g3_hash` |
| `shorthash.rs` siphash rotation `13` → `14` | `t03_g3_hash` |
| `blake2b.rs` G-function rotation `32` → `31` | `t03_g3_hash` |
| `core_hchacha20.rs` round count `10` → `9` | `t05_g4_aead` |
| `core_ed25519.rs` `scalar_negate` output flip | `t07_g5_asym` |

One mutant was *not* caught and was analysed rather than papered over:
`aead_chacha20poly1305.rs`'s `CRYPTO_STREAM_CHUNK` `131072` → `65536`. That
constant is only the granularity of an internal loop (the C asserts it is a
multiple of 64), so any multiple of 64 produces identical output — an equivalent
mutant, not a missed bug. It did reveal that no suite fed a message past the
chunk boundary, which is why `tests/t14_large_inputs.rs` exists: it drives every
bulk entry point at `131072 ± 1/63/64/65`, `2 × 131072`, and 400000 bytes.

## Behaviour deliberately reproduced, not "fixed"

The C is ground truth even where it looks inconsistent. Notable quirks the Rust
mirrors and the tests pin:

- `crypto_aead_aes256gcm_*`: all 9 entry points set `errno = ENOSYS` and return
  `-1` in this build (no `HAVE_WMMINTRIN_H`/`HAVE_ARMCRYPTO`); the getters and
  `keygen` still work and `statebytes()` is 512.
- aegis `*_encrypt_detached` **aborts** on an oversized length while
  `*_decrypt_detached` **returns -1** for the same overflow.
- `crypto_secretstream_*_push` performs no tag validation: any byte 0x00-0xFF is
  accepted and only bit `0x02` (REKEY) is interpreted.
- `crypto_core_ed25519_add/sub` accept non-canonical encodings, the identity and
  small-order points, unlike `crypto_core_ed25519_is_valid_point`.
- `crypto_scalarmult_ed25519{,_base}(n = 0)` writes `q` *before* returning -1.
- `crypto_box_seal` writes the ephemeral public key into `c[0..32]` even when it
  returns -1.
- `crypto_sign_ed25519_pk_to_curve25519` omits the `ge25519_is_canonical` check
  that `verify_detached` performs.
- `crypto_core_ed25519_scalar_invert` only rejects literal byte-zero, so `s = L`
  returns 0 with a meaningless result.
- `crypto_kx_*_session_keys` aborts only when **both** `rx` and `tx` are NULL.
- argon2 `memlimit` is truncated by `/1024`, so 8192 and 8193 must produce
  identical digests; `OPSLIMIT_MIN` is 3 for argon2i but 1 for argon2id.
- scrypt's one-shot API does **not** range-check `opslimit`/`memlimit`:
  `(opslimit = 0, memlimit = 0)` succeeds with `N = 2`.
- ML-KEM-768 decapsulation never fails: a corrupted ciphertext yields a
  *specific* pseudorandom shared secret via implicit rejection, so the tests
  assert byte equality rather than "both failed".
- `sodium_base642bin`'s non-canonical-trailing-bits rejection, and
  `sodium_hex2bin`'s `ERANGE` vs `EINVAL` split, including which of them leaves
  `errno` untouched.

## Phase D — configurations

- `Cargo.toml` declares **no `[features]`**, so there is exactly one
  configuration. `verify.sh` extracts `[features]` from `Cargo.toml`
  mechanically and loops over `default` / `--no-default-features` / each feature
  / all features, so it will cover new combinations automatically if any are
  added; today it reports `no [features] declared -> single configuration`.
- Neither side builds an executable (`c_src/CMakeLists.txt` has no
  `add_executable`; `Cargo.toml` has no `[[bin]]`), so there is no binary stdout
  to compare.
- Symbol parity is re-checked inside `verify.sh` for every combination, and the
  missing-symbol diff must be empty for the run to pass.

## Test-suite hygiene

Two assertions in the `randombytes` suites were statistical smoke checks on REAL
kernel entropy applied at sizes where a false positive is likely (a single
random byte is `0x00` one time in 256). They were caught by a repeated
verification run and narrowed to sizes where a false positive has probability
2^-128 (`>= 16` bytes for "not all zeros", `>= 8` for "not a repeat"):
`t10_g6_rand_errors::g6e_entropy_backends_large_requests` and
`t09_g6_rand::g6_internal_pool_refill_and_ratchet`. Each suite was then re-run
5-6 times consecutively with no failures. Every other assertion in the project
is an exact C-vs-Rust comparison, which is deterministic by construction.

## Known limits of this verification

- The `internal` and `sysrandom` randombytes backends draw real kernel entropy;
  their raw output cannot be compared byte-for-byte. Everything deterministic
  (`randombytes_buf_deterministic`, `randombytes_uniform` under a sequenced
  custom implementation, and every consumer under `install_det_random()`) is.
- Rejection paths that require an injected allocation failure, an entropy-source
  failure, a 4 GiB+ output buffer, `2^32` secretstream messages, or a platform
  the build does not target (`_WIN32`, ARM crypto extensions, `getentropy`,
  `HAVE_GETPID`) are listed row by row in the tables with the reason, and the
  *accepted* boundary immediately beside each is asserted instead.
- Rows that are genuine NULL dereferences of `nonnull` parameters are not
  executed: there is no observable rejection to compare, only undefined
  behaviour. Where the C faults deterministically the comparison IS made through
  `diff_abort_case`.
