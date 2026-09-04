# ERRORS.md — error-surface table

Every distinct way the C in `c_src/` rejects or errors on input.  Derived
mechanically from

```
grep -rn "return -1\|return NULL\|assert\|RETURN_ERROR\|RNG_BAD\|return RNG\|abort()\|exit(" \
     --include=*.c --include=*.h c_src
```

plus the explicit `if (ptr)` / `if (ptr != NULL)` null tests and the min/max
constants those branches compare against.  The reference implementation has no
`assert`, no `RETURN_ERROR` macro and no `return NULL`: its whole rejection
surface is eight branches in two files (`app/src/rng.c`, `app/src/sign.c`).

Rows 1-8 are those branches, one row per branch.  Rows 9-30 are the generic
boundaries the task requires for any C API (null pointers, zero and oversized
lengths, one step past a range, out-of-range enum values crossing FFI) together
with the success sentinels that must also agree — the C validates none of them,
so "expected C result" states the behaviour that must be reproduced rather than
an error code.

Relevant constants: `RNG_SUCCESS 0`, `RNG_BAD_MAXLEN -1`, `RNG_BAD_OUTBUF -2`,
`RNG_BAD_REQ_LEN -3` (`app/include/rng.h`); `SPX_BYTES`, `SPX_PK_BYTES`,
`SPX_SK_BYTES`, `CRYPTO_SEEDBYTES = 3*SPX_N` (`app/include/api.h` and the
selected `app/params/params-*.h`).

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `seedexpander_init` | `maxlen >= 0x100000000` (`rng.c:32`) | returns `RNG_BAD_MAXLEN` (`-1`); `ctx` left untouched | [x] `tests/c_errors.rs::row01_10_seedexpander_init_maxlen_bound` |
| 2 | `seedexpander` | `x == NULL` (`rng.c:66`) | returns `RNG_BAD_OUTBUF` (`-2`); `ctx` left untouched | [x] `tests/c_errors.rs::row02_seedexpander_null_output` |
| 3 | `seedexpander` | `xlen >= ctx->length_remaining` (`rng.c:68`) | returns `RNG_BAD_REQ_LEN` (`-3`); `ctx` left untouched | [x] `tests/c_errors.rs::row03_14_15_16_seedexpander_length_bound` |
| 4 | `AES256_ECB` (via `handleErrors`) | an OpenSSL `EVP_*` call fails (`rng.c:107-109`) | `ERR_print_errors_fp(stderr)` then `abort()` | [x] `tests/c_errors.rs::row04_aes256_ecb_never_fails` |
| 5 | `crypto_sign_verify` | `siglen != SPX_BYTES` (`sign.c:179`) | returns `-1` before any hashing; no output buffer written | [x] `tests/c_errors.rs::row05_20_21_verify_wrong_siglen` |
| 6 | `crypto_sign_verify` | recomputed root differs from `pk + SPX_N`, i.e. `memcmp(root, pub_root, SPX_N) != 0` (`sign.c:235`) | returns `-1` | [x] `tests/c_errors.rs::row06_verify_root_mismatch` |
| 7 | `crypto_sign_open` | `smlen < SPX_BYTES` (`sign.c:270`) | `memset(m, 0, smlen)`, `*mlen = 0`, returns `-1` | [x] `tests/c_errors.rs::row07_22_24_open_short_smlen` |
| 8 | `crypto_sign_open` | `crypto_sign_verify(sm, SPX_BYTES, sm+SPX_BYTES, smlen-SPX_BYTES, pk)` is non-zero (`sign.c:278`) | `memset(m, 0, smlen)`, `*mlen = 0`, returns `-1`; note the `memset` covers `smlen`, not `*mlen` | [x] `tests/c_errors.rs::row08_25_26_open_invalid_signature` |
| 9 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted value; one below the row 1 bound) | returns `RNG_SUCCESS` (`0`); `ctx->length_remaining = 0xFFFFFFFF`, `ctr[8..12] = FF FF FF FF`, `ctr[12..16] = 0`, `buffer_pos = 16` | [x] `tests/c_errors.rs::row01_10_seedexpander_init_maxlen_bound` |
| 10 | `seedexpander_init` | `maxlen == 0x100000000` exactly (smallest rejected value) | `RNG_BAD_MAXLEN` | [x] `tests/c_errors.rs::row01_10_seedexpander_init_maxlen_bound` |
| 11 | `seedexpander_init` | `maxlen == 0` | `RNG_SUCCESS`, `length_remaining = 0`, so every later `seedexpander` call fails with `RNG_BAD_REQ_LEN` | [x] `tests/c_errors.rs::row11_seedexpander_init_maxlen_zero` |
| 12 | `seedexpander` | `xlen == 0` with `length_remaining == 0` (`0 >= 0`) | `RNG_BAD_REQ_LEN` — a zero-length request is *not* a no-op | [x] `tests/c_errors.rs::row11_seedexpander_init_maxlen_zero` |
| 13 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | `RNG_SUCCESS`, nothing written, `length_remaining` unchanged, `buffer_pos` unchanged | [x] `tests/c_errors.rs::row03_14_15_16_seedexpander_length_bound` |
| 14 | `seedexpander` | `xlen == length_remaining - 1` (largest accepted request) | `RNG_SUCCESS`, `length_remaining` becomes 1 | [x] `tests/c_errors.rs::row03_14_15_16_seedexpander_length_bound` |
| 15 | `seedexpander` | `xlen == length_remaining` exactly (smallest rejected request) | `RNG_BAD_REQ_LEN` | [x] `tests/c_errors.rs::row03_14_15_16_seedexpander_length_bound` |
| 16 | `seedexpander` | `xlen` far past the remaining length (e.g. `0xFFFFFFFF`) | `RNG_BAD_REQ_LEN`, no write, so an under-sized `x` must not be touched | [x] `tests/c_errors.rs::row03_14_15_16_seedexpander_length_bound` |
| 17 | `randombytes` | `xlen == 0` | returns `RNG_SUCCESS`; the `while (xlen > 0)` loop body never runs, but `AES256_CTR_DRBG_Update(NULL, Key, V)` still runs and `reseed_counter` still increments | [x] `tests/c_errors.rs::row17_randombytes_zero_length` |
| 18 | `randombytes_init` | `personalization_string == NULL` (`rng.c:157`) | the 48-byte XOR is skipped; `seed_material` is `entropy_input` verbatim | [x] `tests/c_errors.rs::row18_randombytes_init_null_personalization` |
| 19 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` (`rng.c:196`) | the 48-byte XOR is skipped; `Key`/`V` come straight from the three ECB blocks | [x] `tests/c_errors.rs::row19_drbg_update_null_provided_data` |
| 20 | `crypto_sign_verify` | `siglen == SPX_BYTES - 1` and `siglen == SPX_BYTES + 1` (one step either side of the only accepted length) | `-1` in both cases (row 5 branch) | [x] `tests/c_errors.rs::row05_20_21_verify_wrong_siglen` |
| 21 | `crypto_sign_verify` | `siglen == 0` | `-1` (row 5 branch) | [x] `tests/c_errors.rs::row05_20_21_verify_wrong_siglen` |
| 22 | `crypto_sign_open` | `smlen == SPX_BYTES - 1` (largest rejected) | `memset(m, 0, SPX_BYTES-1)`, `*mlen = 0`, `-1` | [x] `tests/c_errors.rs::row07_22_24_open_short_smlen` |
| 23 | `crypto_sign_open` | `smlen == SPX_BYTES` exactly (smallest accepted; zero-length message) | `*mlen = 0` and `0` on a valid signature | [x] `tests/c_errors.rs::row23_open_smlen_exactly_spx_bytes` |
| 24 | `crypto_sign_open` | `smlen == 0` | `memset(m, 0, 0)` (no write), `*mlen = 0`, `-1` | [x] `tests/c_errors.rs::row07_22_24_open_short_smlen` |
| 25 | `crypto_sign_open` | valid length, signature byte flipped | row 8 behaviour: `m` zeroed over `smlen`, `*mlen = 0`, `-1` | [x] `tests/c_errors.rs::row08_25_26_open_invalid_signature` |
| 26 | `crypto_sign_open` | valid length, public key byte flipped | row 8 behaviour | [x] `tests/c_errors.rs::row08_25_26_open_invalid_signature` |
| 27 | `set_type` (and `set_layer_addr`, `set_chain_addr`, `set_hash_addr`, `set_tree_height`) | out-of-range enum / value: `type = 7 .. 255` (no `SPX_ADDR_TYPE_*` variant), `type = 256`, `type = 0xFFFFFFFF` | no validation whatsoever: `((unsigned char *)addr)[SPX_OFFSET_*] = (unsigned char)value`, i.e. the value is truncated to its low 8 bits and stored | [x] `tests/c_errors.rs::row27_28_out_of_range_address_values` |
| 28 | `set_keypair_addr`, `set_tree_index` | `0xFFFFFFFF` | no validation: `u32_to_bytes` writes all four bytes big-endian | [x] `tests/c_errors.rs::row27_28_out_of_range_address_values` |
| 29 | `SPX_thash` | `inblocks == 0`, and `inblocks` larger than anything the library uses internally (`> max(SPX_WOTS_LEN, SPX_FORS_TREES)`) | no validation: the C `SPX_VLA` sizes the scratch buffers from `inblocks` at run time and hashes `SPX_N + SPX_ADDR_BYTES + inblocks*SPX_N` bytes | [x] `tests/c_errors.rs::row29_thash_unchecked_inblocks` |
| 30 | `SPX_ull_to_bytes` / `SPX_bytes_to_ull` / `SPX_blake256_mgf1` / `SPX_mgf1_256` / `shake256` | `outlen == 0` / `inlen == 0` | no validation and no error return: the loops simply execute zero times | [x] `tests/c_errors.rs::row30_zero_length_helpers` |

## Conditions that are C undefined behaviour, not error paths

Listed for completeness; they are deliberately **not** turned into differential
tests because the C reference dereferences without checking, so there is no
defined result to match.

* `seedexpander_init(ctx, NULL, NULL, maxlen)` with an accepted `maxlen` —
  `memcpy` from a null `seed`.  (With a *rejected* `maxlen` the C returns before
  the `memcpy`; that is row 1/row 10 and it *is* tested, which also pins down
  that the Rust must not touch the pointers before the check.)
* `randombytes_init(NULL, ...)` — unconditional `memcpy` of 48 bytes.
* Any `crypto_sign*` call with a null `pk`/`sk`/`sig` pointer.
* `compute_root(..., tree_height = 0, ...)` — `for (i = 0; i < tree_height - 1; i++)`
  with `uint32_t tree_height` wraps to `0xFFFFFFFF` iterations and walks off the
  end of `auth_path`.
* `crypto_sign_open` with an `m` buffer shorter than `smlen`, since the failure
  paths `memset(m, 0, smlen)`.

## Result

All 30 rows are checked off: each has a differential test in
`translation/tests/c_errors.rs` that constructs the exact condition, calls both
`.so` objects, and asserts the same sentinel **and** the same side effects
(`*mlen`, the `memset` extent, and the `AES_XOF_struct` / `DRBG_ctx` byte
images), not merely that both failed.

Those 16 tests pass in **all 96 build configurations**; see
`translation/test_results.txt` (96 `PASS`, 0 `FAIL`) and
`/tmp/testlogs/<tag>.log`.

No divergence was found on any error path — the Rust already returned the same
codes.  Two hardening changes were made to the `rng.c` wrappers so the C's
check *order* is observable rather than incidental:

* `seedexpander_init` now tests `maxlen >= 0x100000000` **before** turning
  `seed` and `diversifier` into slices, matching `rng.c`, which returns before
  its `memcpy`s.  Row 1/10 passes null pointers for both to enforce this.
* `seedexpander` now tests `xlen >= ctx->length_remaining` before materialising
  the output slice, so an over-long request against an under-sized `x` cannot
  form an out-of-bounds slice.  Rows 3 and 16 pass an 8-byte buffer with
  `xlen = 0xFFFFFFFF`.

One row is discharged by argument rather than by provoking it: row 4's
`handleErrors()`/`abort()` is only reachable if an OpenSSL `EVP_*` call fails
during a single AES-256-ECB block encryption, which no input to `AES256_ECB`
can cause.  `row04_aes256_ecb_never_fails` shows both sides agree over 512
random key/counter pairs plus the all-zero and all-`0xFF` extremes, and neither
aborts.
