# Differential verification of the libsodium 1.0.23 C→Rust translation

Everything here is a **differential** result: both the reference C `libsodium.so`
and the translated Rust `liblibsodium.so` are loaded with `libloading` and every
call goes through `dlsym`, so the `#[no_mangle]`/`extern "C"` export wrappers are
exercised exactly as an external C consumer would exercise them. The Rust crate
is never called directly.

## How to reproduce

```sh
# 1. reference C shared library
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . -j

# 2. Rust shared library + the whole differential suite, all feature combos
cd ../../translation
./run-all-features.sh          # builds the cdylib, then runs every combination
./check-symbols.sh             # Phase-D symbol parity gate
./build-phaseA.sh              # regenerates ERRORS.md / CONFIGS.md from phaseA/
```

`cargo test --test <name>` does **not** rebuild the `cdylib` (the test binaries
reach it only through `dlsym`, so Cargo sees no dependency). The harness detects
this and hard-fails with a `STALE cdylib` message instead of silently testing an
old `.so`; always `cargo build --offline` first.

## Completion gate

| gate | result |
|------|--------|
| `SYMBOLS.md` — symbols exported by C but missing from Rust | **0** of 878 |
| `SYMBOLS.md` — undefined non-libc symbols in the Rust `.so` | **0** |
| `CONFIGS.md` — Phase-B rows verified | **1157 `[x]` + 13 `[~]` of 1170** |
| `ERRORS.md` — Phase-C rows with a resolved status | **774 of 774** |
| feature combinations covered | **1 of 1** (the crate declares no `[features]`) |
| tests | **614 passing, 0 failing**, across 36 test binaries |

The 13 `[~]` configuration rows are the `randombytes` sysrandom / internal
implementations, whose *output* comes from the OS and therefore cannot be
compared byte for byte between two processes' libraries. They are
contract-verified instead: identical descriptor shape (which callbacks are
NULL), identical return codes, non-constant output, `uniform(n) < n`, and no
out-of-bounds writes. Every deterministic randombytes path
(`randombytes_buf_deterministic`, the `uniform` rejection sampler, the whole
dispatch layer) *is* compared byte for byte, by installing a deterministic
custom implementation into both libraries.

## Phase A artifacts

* **`SYMBOLS.md`** — the full `nm -D` table, C vs Rust, mechanically generated.
* **`ERRORS.md`** — 774 rows, one per distinct rejection branch in the C
  (`return -1`, `return NULL`, error enums, `sodium_misuse()`, `assert`, every
  explicit range/null check and min/max constant), with the exact expected
  sentinel and the Phase-C status.
* **`CONFIGS.md`** — 1170 rows, one per meaningful combination of runtime
  options and input shapes the C actually branches on.
* Per-area fragments live in `phaseA/`; `build-phaseA.sh` concatenates them.

### ERRORS.md status breakdown

| status | rows | meaning |
|--------|------|---------|
| `verified` | 622 | a differential test constructs the exact invalid input and asserts both libraries return the same sentinel (and the same `errno`, where the C sets one) |
| `unreachable-from-public-API` | 87 | the branch exists but no input reachable through an exported symbol can trigger it (each row records the proof) |
| `OS-level-not-forceable` | 31 | needs a `/dev/urandom`, `getrandom`, `poll` or `close` syscall failure |
| `not-compiled-in-this-build` | 11 | inside an `#ifdef HAVE_*` the CMake build does not define |
| `dead-branch-on-this-build` | 11 | predicate provably false on LP64 (e.g. `mlen > UINT64_MAX`) |
| `compile-time-only` | 8 | `COMPILER_ASSERT`, or an `assert` that is a tautology |
| `undefined-behaviour-not-tested` | 6 | `__attribute__((nonnull))` violations — UB in C, so there is no defined result to compare |
| `checked-via-guard-only` | 4 | needs a ≥256 GiB buffer; driven with an over-limit length against a `PROT_NONE`-guarded mapping |
| `host-OOM-not-forceable` | 3 | needs `malloc` to fail |
| `verified-indirectly` | 3 | `static` helper, observed through all of its public callers |

## Verification techniques used beyond "call both, compare the output"

* **Full opaque-state comparison.** Wherever the C exposes `*_statebytes()`, the
  state is allocated at exactly that size in a guard-padded, over-aligned slab
  and the **entire** state image is compared between C and Rust after `init` and
  after **every** `update`/`squeeze`/`push`/`pull`/`rekey`. This is far stronger
  than comparing final digests and it also proves nothing is written past
  `statebytes()`.
* **Out-of-bounds-write detection.** Every output buffer is allocated with a
  32-byte trailing guard pattern (`padded()` / `check_pad()`).
* **Abort-path comparison.** `sodium_misuse()` and `assert()` paths cannot be
  observed in-process, so `eq_abort()` runs each side in a forked child (with
  core dumps disabled) and compares the process outcome — `exit:N` vs `sig:N`.
  A misuse handler that `_exit(77)`s is used to prove the handler is invoked
  *before* the `abort()`.
* **Randomness lockstep.** A deterministic RNG is installed into both libraries
  via `randombytes_set_implementation`, with one independent stream per library
  so the n-th random byte C consumes equals the n-th byte Rust consumes. The
  streams are **thread-local**, so parallel tests cannot desynchronise them.
  `uniform` is deliberately left NULL so the library's own rejection sampler is
  what gets tested.
* **Out-of-range enum values across the FFI boundary.** C enums accept any
  `int`, so values with no valid variant are real inputs. This found the one
  genuine translation bug (below).
* **Cross-library interop.** C-produced ciphertexts/signatures/keys are consumed
  by Rust and vice versa, in both directions.
* **Structural re-derivation and absolute KATs.** Where a shared bug could make
  a C-vs-Rust comparison vacuous, outputs are additionally re-derived from
  independent primitives (X-Wing's combiner, secretbox from `hsalsa20`+stream+
  Poly1305, ML-KEM's implicit-rejection secret as `SHAKE256(z‖ct)`) or pinned to
  published vectors (RFC 8032 §7.1, RFC 8439 §2.8.2, RFC 5869, SipHash-2-4,
  BLAKE2b, RFC 7748).
* **Negative controls.** Several areas deliberately mutated one bit of the Rust
  source, confirmed the suite went red, then restored and re-verified — proving
  the tests are not vacuous.

## Divergences found and fixed in the Rust translation

1. **`src/argon2_encoding.rs` — out-of-range `argon2_type` across the FFI
   boundary.** `_sodium_argon2_decode_string` and `_sodium_argon2_encode_string`
   declared their `type` parameter as a 2-variant `#[repr(C)] enum` and
   dispatched with an exhaustive `match`. The C parameter is an `int`, so an
   external caller may legally pass any value; passing `0` materialised an
   invalid discriminant and took the wrong arm:
   `argon2_decode_string(ctx, "$argon2id$…", 0)` returned `ARGON2_OK` where C
   returns `ARGON2_INCORRECT_TYPE` (-26), and `argon2_encode_string(…, 0)`
   returned 0 where C returns `ARGON2_ENCODING_FAIL` (-31). Fixed by taking
   `c_int` and comparing numerically with an explicit `else` mirroring the C
   `default:`, and by propagating the plain `int` through the FFI boundary of
   `_sodium_argon2_ctx` / `_sodium_argon2_hash` / `_sodium_argon2_verify` so an
   invalid enum value is never materialised.
2. **`src/blake2b.rs` — missing live `assert`.** The CMake build sets no
   `CMAKE_BUILD_TYPE`, so `NDEBUG` is absent and
   `crypto_generichash_blake2b_final`'s `assert(outlen <= UINT8_MAX)` is live.
   The port omitted it, so `outlen == 257` truncated to `1` and returned `0`
   where C dies on `SIGABRT`. Fixed with an explicit guard.

Everything else in the 51 kLOC library matched byte for byte, including opaque
state images, `errno` values, and all of the deliberately-preserved upstream
quirks listed below.

## Upstream quirks confirmed replicated (a "fix" here would be a bug)

* `crypto_aead_aes256gcm_*` is the ENOSYS stub in this build and, unlike every
  other AEAD, leaves `*clen_p`/`*mlen_p`/`*maclen_p` **unwritten** on failure.
* `crypto_kx_*_session_keys` with `tx == NULL` leaves the caller's `rx` buffer
  holding the **tx** key (client), with the server's loop order reversed.
* ed25519 verification is cofactored (`ge25519_has_small_order(&check) - 1`), so
  an off-main-subgroup key can produce an accepted signature.
* `verify_detached` checks `ge25519_is_canonical(pk)` but not
  `is_on_main_subgroup`; `pk_to_curve25519` checks the opposite.
* `crypto_scalarmult_curve25519_base` bypasses the wrapper's all-zero-output
  guard entirely and can never fail — not even for `n = 0`.
* ed25519/ristretto255 scalarmult stage the clamped scalar in the caller's `q`,
  so early rejects leave `q` untouched while late rejects leave it fully written.
* generichash never enforces `BYTES_MIN`/`KEYBYTES_MIN`; `final` with an
  `outlen` different from `init`'s is legal and emits a prefix.
* `crypto_generichash_blake2b` with `key == NULL, keylen > 0` **aborts**, while
  `..._init` with the same arguments silently does an unkeyed init.
* SHA-2 `final` zeroes the state, so double-`final` silently hashes from a zeroed
  IV and still returns 0.
* SHA-3/XOF `update`-after-finalize returns `-1` **and still absorbs the data**;
  a second `final` returns `-1` and still writes `outlen` bytes.
* TurboSHAKE's domain byte is completely unvalidated (`0x00` and `0x80` accepted).
* `crypto_stream_chacha20_ietf_xor_ic`'s guard underflows for `mlen > 2^38`, so
  it silently permits 32-bit counter overflow into the nonce.
* secretstream's Poly1305 padding expression
  `(0x10 - sizeof block + mlen) & 0xf` with `slen = 64 + mlen`.
* NaCl-style `crypto_secretbox` never verifies its leading zero padding.
* `crypto_pwhash_str_alg` **aborts** on an unknown `alg` rather than returning -1.
* Scrypt enforces no `opslimit`/`memlimit` minimum, unlike argon2.
* `crypto_pwhash_str_needs_rehash` compares only `t_cost` and `m_cost`.
* `sodium_unpad` writes `*unpadded_buflen_p` even when it returns `-1`.
* `sodium_base642bin` sets **no** `errno` for its two "bad trailing bits" rejects.
* `sodium_malloc` is plain `malloc` + `memset(0xdb)` in this build — no guard
  pages, no canary; `sodium_mlock`/`munlock`/`mprotect_*` are `-1`/`ENOSYS`
  (and `munlock` still zeroes the buffer first); `sodium_stackzero` is a no-op.

## Corrections made to the Phase-A tables while testing

Several table rows were derived from a careful but not infallible reading of the
C; where a test showed the C actually behaves differently, the **C is
authoritative** and the row was annotated with the real behaviour (search the
fragments for "Phase-C status notes" / "Coverage notes"). Examples: the
`sodium_bin2base64` short-buffer path aborts rather than returning NULL, so
`argon2_encode_string` aborts rather than returning -31; the NaCl
`crypto_box_open` guard is `clen < 32`, not `clen < 16`; AEAD-style zeroing of
`m` on MAC failure does *not* happen in `crypto_secretbox_open_detached`;
`ristretto255`/`elligator2` branch on bit 7 of `r[31]`, not bit 5.

## Notes on the test harness itself

Three harness defects were found and fixed during verification — each of which
had produced a *false* divergence or a flaky pass, so they are worth recording:

1. The deterministic RNG state was process-global, so parallel tests in one
   binary interleaved their draws and desynchronised the C stream from the Rust
   stream. Now thread-local.
2. `tests/a1_randombytes_impl.rs` swaps the process-global installed RNG, so its
   tests must not overlap; they now take a file-local mutex.
3. `tests/a3_generichash.rs::streaming_one_byte_updates` allocated a 32-byte key
   but passed `keylen = 64`, making the library read 32 bytes of heap garbage —
   which is why it failed only at certain thread counts. Both libraries agreed
   with each other throughout; the "divergence" was streaming-vs-one-shot within
   the C library, caused by the test itself.

The suite is stable: the full 614 tests pass at `--test-threads` of
1, 2, 3, 4, 6, 8 and 16, and across repeated runs.
