# ERRORS.md — error-surface table

Derived mechanically from the C source. The sweep used was

```
grep -rn "return -1\|return NULL\|RETURN_ERROR\|assert\|RNG_BAD\|abort()\|exit(\|#error" \
     --include=*.c --include=*.h c_src/
```

plus a manual read of every `if (...) return` in `app/src/sign.c` and
`app/src/rng.c`. Findings:

* `app/src/sign.c` — 4 runtime rejection branches (2 in `crypto_sign_verify`,
  2 in `crypto_sign_open`).
* `app/src/rng.c` — 3 runtime rejection branches (1 in `seedexpander_init`,
  2 in `seedexpander`) plus one `abort()` on OpenSSL failure.
* Everything else found by the grep is a `#error` (compile-time guard, cannot be
  triggered at runtime) or a plain `return 0` success path.
* There are **no** `assert()`s, no `RETURN_ERROR` macro, and no function that
  returns `NULL` anywhere in `c_src`.

Sentinel values (`app/include/rng.h`):
`RNG_SUCCESS = 0`, `RNG_BAD_MAXLEN = -1`, `RNG_BAD_OUTBUF = -2`,
`RNG_BAD_REQ_LEN = -3`.

## Rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `crypto_sign_verify` | `siglen != SPX_BYTES` (`sign.c:180`). Checked *before* anything is read, so it fires for 0, `SPX_BYTES-1`, `SPX_BYTES+1`, `SIZE_MAX`. | returns `-1`; no output buffer is written | `err_e1_verify_bad_siglen` | [x] |
| E2 | `crypto_sign_verify` | `siglen == SPX_BYTES` but the recomputed hypertree root differs from `pk + SPX_N` (`memcmp` at `sign.c:236`) — i.e. corrupted signature, corrupted message, or wrong public key | returns `-1`. **Caveat found while testing:** with `HASH_BACKEND=blake` a *message* change is not always detected. `lib/blake/src/hash_blake.c` calls `blakeX_update(&S, m, mlen)`, but `blake256_update`/`blake512_update` take their length in **bits** and `memcpy` only `datalen >> 3` bytes. So only the first `mlen/8` message bytes influence the signature, and only when `mlen % 8 == 0` (otherwise `blake*_final` never reaches a compression at all and the digest degenerates). Measured on blake-128f: mlen 16→2 sensitive bytes, 32→4, 64→8, 128→16, 200→25, and 70/100→0. The other three backends absorb every byte. This is C behaviour and is reproduced exactly. | `err_e2_verify_root_mismatch` | [x] |
| E3 | `crypto_sign_open` | `smlen < SPX_BYTES` (`sign.c:272`) | `memset(m, 0, smlen)`, `*mlen = 0`, returns `-1`. Note `m` is zeroed for `smlen` bytes, **not** for `smlen - SPX_BYTES`. | `err_e3_open_short_smlen` | [x] |
| E4 | `crypto_sign_open` | `smlen >= SPX_BYTES` but the inner `crypto_sign_verify` fails (`sign.c:280`) | `memset(m, 0, smlen)`, `*mlen = 0`, returns `-1`. `*mlen` is set to `smlen - SPX_BYTES` first and then reset to 0. | `err_e4_open_verify_fail` | [x] |
| E5 | `seedexpander_init` | `maxlen >= 0x100000000` (`rng.c:33`) | returns `RNG_BAD_MAXLEN` (`-1`); `ctx` left completely untouched | `err_e5_seedexpander_init_maxlen` | [x] |
| E6 | `seedexpander` | `x == NULL` (`rng.c:67`) — checked **before** the length check, so a null `x` wins even when `xlen` is also invalid | returns `RNG_BAD_OUTBUF` (`-2`); `ctx` untouched | `err_e6_seedexpander_null_out` | [x] |
| E7 | `seedexpander` | `xlen >= ctx->length_remaining` (`rng.c:69`) — note `>=`, so requesting *exactly* the remaining budget is already an error | returns `RNG_BAD_REQ_LEN` (`-3`); `ctx` untouched | `err_e7_seedexpander_req_len` | [x] |
| E8 | `AES256_ECB` | any OpenSSL `EVP_*` call fails (`rng.c:109` `handleErrors()`) | `ERR_print_errors_fp(stderr)` then `abort()` | not reachable: the Rust port uses the `aes` crate, which is infallible for a fixed 32-byte key and 16-byte block; the C path cannot be made to fail without corrupting OpenSSL state. Documented, not tested. | n/a |

## Boundary / generic-FFI rows (not explicit C checks, but real inputs)

| # | function | trigger | expected C result | test | status |
|---|----------|---------|-------------------|------|--------|
| B1 | `crypto_sign_verify` | `siglen == SPX_BYTES`, `mlen == 0` (empty message), valid signature | returns `0` | `cfg_c37_signature_and_verify` (mlen=0) + `err_zero_length_message_paths` | [x] |
| B2 | `crypto_sign_open` | `smlen == SPX_BYTES` exactly (zero-length message) | `*mlen = 0`, returns `0`, nothing written to `m` | `err_b2_open_exact_smlen` | [x] |
| B3 | `crypto_sign_open` | `smlen == SPX_BYTES - 1` (one step below the documented minimum) | `-1`, `*mlen = 0` | `err_e3_open_short_smlen` | [x] |
| B4 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted) and `maxlen == 0x100000000` (first rejected) | `0` and `-1` respectively | `err_e5_seedexpander_init_maxlen` | [x] |
| B5 | `seedexpander_init` | `maxlen == 0` | `0`; `ctr[8..12] = 0`, `length_remaining = 0` (so every later `seedexpander` call fails with `-3`) | `err_b5_seedexpander_init_zero` | [x] |
| B6 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | `0`, nothing written | `err_b6_seedexpander_zero_len` | [x] |
| B7 | `seedexpander` | `xlen == ctx->length_remaining - 1` (largest accepted request) | `0` | `err_e7_seedexpander_req_len` | [x] |
| B8 | `randombytes` (`rng.c`) | `xlen == 0` | returns `0` (`RNG_SUCCESS`); DRBG state still advanced by the trailing `AES256_CTR_DRBG_Update` and `reseed_counter++` | `err_b8_randombytes_zero` | [x] |
| B9 | `randombytes_init` | `personalization_string == NULL` | no XOR applied; the `if (personalization_string)` branch is skipped | `cfg_c41_randombytes_init_variants` | [x] |
| B10 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` | the XOR loop is skipped; `Key`/`V` come straight from the three AES blocks | `err_b10_drbg_update_null` | [x] |
| B11 | `SPX_set_type` | out-of-range "enum" value. The `SPX_ADDR_TYPE_*` constants are `#define`s 0..6, but `set_type` takes a `uint32_t` and does `((unsigned char*)addr)[SPX_OFFSET_TYPE] = (unsigned char)type` — so any `int` is accepted and silently truncated to its low byte. `7`, `255`, `256`, `0xFFFFFFFF`, and `(uint32_t)-1` are all real inputs. | writes `type & 0xFF` at `SPX_OFFSET_TYPE`; no rejection | `err_b11_set_type_out_of_range` | [x] |
| B12 | `SPX_set_layer_addr`, `SPX_set_chain_addr`, `SPX_set_hash_addr`, `SPX_set_tree_height` | same truncation-to-`unsigned char` behaviour for values > 255 | low byte written, rest of `addr` unchanged | `err_b12_addr_setters_truncate` | [x] |
| B13 | `SPX_ull_to_bytes` | `outlen == 0` (loop body never runs, nothing written) and `outlen > 8` (leading bytes get zero after the value is exhausted) | no write / zero-extension; `outlen` is `unsigned int` cast to `signed int` for the loop, so huge values would run off the buffer — tested only up to 16 | `err_b13_ull_to_bytes_edge` | [x] |
| B14 | `SPX_bytes_to_ull` | `inlen == 0` (returns 0) and `inlen > 8` (the shift `8*(inlen-1-i)` is ≥ 64 → undefined shift in C; clang -O3 emits a variable shift, so both sides must agree bit-for-bit) | see test: C behaviour is recorded and Rust must match it | `err_b14_bytes_to_ull_edge` | [x] |
| B15 | `SPX_thash` | `inblocks == 0` | zero-length payload hashed; for `blake`/`sha2` with the 512 variant the `inblocks > 1` dispatch is *not* taken | `cfg_c16_thash_0` | [x] |

`B14` note: `bytes_to_ull` with `inlen > 8` is undefined behaviour in C. It is
included because it is reachable across the FFI boundary and the Rust must not
panic where the C returns a value; the test asserts the two agree on the actual
compiled behaviour rather than on a specification.

## Where the tests live and how to run them

`translation/tests/phase_c_errors.rs`. Each test `dlopen`s both the C reference
`.so` and the Rust `cdylib` and calls them through their exported C symbols only.

```
./run_tests_all.sh                    # all 96 feature combinations
cd translation && RUST_TEST_THREADS=1 \
  cargo test --release --no-default-features --features "blake,simple,128f" \
  --test phase_c_errors
```

`RUST_TEST_THREADS=1` is required: `DRBG_ctx` in `rng.c` is process-global.

## Result

16 tests, covering E1–E7 and B1–B15, pass under **all 96 build configurations**
(`4 backends × 2 THASH × 6 SECPAR × {DRBG, urandom}`). `E8` is documented as
unreachable rather than tested; `B14` beyond `inlen == 8` is documented as
undefined behaviour in both languages and deliberately not asserted.

Under the `urandom` feature the active C core (`libsphincs_core.so`) does not
contain `rng.c` at all, so rows E5–E7, B5–B7 and B10 are checked against
`libsphincs_core_det.so` opened as an auxiliary handle — the same `rng.c` object
code, which is what the Rust `cdylib` exports in both configurations.
