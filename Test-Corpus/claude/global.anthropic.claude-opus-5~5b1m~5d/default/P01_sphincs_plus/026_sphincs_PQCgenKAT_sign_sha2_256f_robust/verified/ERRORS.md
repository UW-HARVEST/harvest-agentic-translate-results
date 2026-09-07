# ERRORS.md -- error-surface table

Derived mechanically from `c_src` by grepping for every `return -`, `return RNG_*`,
`== NULL` / `!= NULL` test, `abort()`, range check and min/max constant:

```
grep -rnE 'return *-|return *NULL|RETURN_ERROR|assert|RNG_BAD|NULL|#error|abort' c_src/app c_src/lib
```

The reference implementation has **no** `assert()`, no `errno`, and no error enum.
Every runtime rejection is one of

* `sign.c`  -> `return -1`
* `rng.c`   -> `RNG_BAD_MAXLEN (-1)`, `RNG_BAD_OUTBUF (-2)`, `RNG_BAD_REQ_LEN (-3)`
* `rng.c`   -> `abort()` (only reachable when OpenSSL itself fails)

All `#error` directives are *compile-time* assertions on the parameter sets; they
are listed at the bottom for completeness but cannot be triggered at run time
(all 48 shipped parameter combinations satisfy them).

`SPX_ADDR_TYPE_*` (`address.h`) is a set of `#define`d ints, not an `enum`, and
`set_type()` truncates its argument to `unsigned char` with **no** validation --
so an "out of range enum value" is not rejected, it is silently truncated.
Rows 15-19 pin that behaviour down, because "no rejection" is itself the
contract the Rust must reproduce.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `crypto_sign_verify` (sign.c:179) | `siglen != SPX_BYTES` (tested: `0`, `1`, `SPX_BYTES-1`, `SPX_BYTES+1`, `2*SPX_BYTES`, `SIZE_MAX`) | returns `-1`, `sig` never dereferenced | [x] |
| 2  | `crypto_sign_verify` (sign.c:235) | `siglen == SPX_BYTES` but recomputed root `!= pk + SPX_N` (corrupt R / FORS sig / WOTS sig / auth path / pk, or a changed `mlen`) | returns `-1` | [x] |
| 2b | `crypto_sign_verify` | flipping a bit of the *message content* while keeping `mlen` -- with the **blake** backend this is NOT rejected, because `lib/blake/src/hash_blake.c` passes BYTE counts to `blakeX_update()`, whose `datalen` is a BIT count; for short messages no BLAKE compression happens at all and the digest is independent of the message bytes. Ground truth: do not "fix" it. | backend-dependent; the test asserts C and Rust return the **same** value | [x] |
| 3  | `crypto_sign_verify` | valid signature (control row, must return `0`) | returns `0` | [x] |
| 4  | `crypto_sign_open` (sign.c:269) | `smlen < SPX_BYTES` (tested: `0`, `1`, `SPX_BYTES-1`) | `memset(m,0,smlen)`, `*mlen = 0`, returns `-1` | [x] |
| 5  | `crypto_sign_open` (sign.c:277) | `smlen >= SPX_BYTES` but inner `crypto_sign_verify` fails (corrupt `sm`, wrong `pk`) | `memset(m,0,smlen)`, `*mlen = 0`, returns `-1` | [x] |
| 6  | `crypto_sign_open` | `smlen == SPX_BYTES` exactly -> zero-length message, valid signature | `*mlen = 0`, returns `0` | [x] |
| 7  | `seedexpander_init` (rng.c:32) | `maxlen >= 0x100000000` (tested: `0x100000000`, `0xFFFFFFFFFFFFFFFF`) | returns `RNG_BAD_MAXLEN` = `-1`, `ctx` untouched | [x] |
| 8  | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted value, one below the check) | returns `RNG_SUCCESS` = `0` | [x] |
| 9  | `seedexpander` (rng.c:66) | `x == NULL` | returns `RNG_BAD_OUTBUF` = `-2` (checked *before* the length check) | [x] |
| 10 | `seedexpander` (rng.c:68) | `xlen >= ctx->length_remaining` (tested `xlen == length_remaining` and `xlen > length_remaining`) | returns `RNG_BAD_REQ_LEN` = `-3`, `ctx` untouched | [x] |
| 11 | `seedexpander` | `xlen == length_remaining - 1` (largest accepted) and `xlen == 0` | returns `RNG_SUCCESS` = `0` | [x] |
| 12 | `seedexpander` | `x == NULL` **and** `xlen >= length_remaining` (both triggers at once -> which wins?) | returns `RNG_BAD_OUTBUF` = `-2` | [x] |
| 13 | `randombytes_init` (rng.c:141) | `personalization_string == NULL` | no XOR applied; DRBG seeded from `entropy_input` alone | [x] |
| 14 | `AES256_CTR_DRBG_Update` (rng.c:205) | `provided_data == NULL` | no XOR applied; `Key`/`V` taken straight from the 3 AES blocks | [x] |
| 15 | `set_type` (address.c:33) | `type` outside `{0..6}` -- i.e. no valid `SPX_ADDR_TYPE_*` variant (tested `7`, `255`, `256`, `0x100 \| 3`, `0xFFFFFFFF`) | **no rejection**: `addr[SPX_OFFSET_TYPE] = (unsigned char)type` (truncation mod 256) | [x] |
| 16 | `set_layer_addr` (address.c:11) | `layer >= 256` / `layer > SPX_D` | **no rejection**: truncated to `unsigned char` | [x] |
| 17 | `set_chain_addr` / `set_hash_addr` (address.c:72,81) | value `>= 256` (e.g. `>= SPX_WOTS_LEN`, `>= SPX_WOTS_W`) | **no rejection**: truncated to `unsigned char` | [x] |
| 18 | `set_tree_height` (address.c:92) | `tree_height >= 256` / `> SPX_FULL_HEIGHT` | **no rejection**: truncated to `unsigned char` | [x] |
| 19 | `set_tree_addr` (address.c:19) | `tree` using all 64 bits (`0xFFFFFFFFFFFFFFFF`) although only `SPX_TREE_HEIGHT*(SPX_D-1)` bits are meaningful | **no rejection**: full 8 bytes written big-endian at `SPX_OFFSET_TREE` | [x] |
| 20 | `ull_to_bytes` (utils.c:12) | `outlen == 0` | writes nothing (loop `i = -1; i >= 0` never runs) | [x] |
| 21 | `bytes_to_ull` (utils.c:35) | `inlen == 0` | returns `0` (loop never runs) | [x] |
| 22 | `bytes_to_ull` | `inlen == 8` (largest well-defined value; `inlen > 8` shifts by `>= 64` = C UB, out of contract) | returns the full 64-bit big-endian value | [x] |
| 23 | `thash` (all backends) | `inblocks == 0` -> zero-length data part, VLA of size `SPX_N + SPX_ADDR_BYTES` | **no rejection**: hashes `pub_seed \|\| addr` only (haraka takes the `else` branch: `haraka_S` over `SPX_ADDR_BYTES`) | [x] |
| 24 | `crypto_sign_signature` / `crypto_sign` / `crypto_sign_keypair` / `crypto_sign_seed_keypair` | *no* input is ever rejected -- these four have a single unconditional `return 0` | always returns `0` | [x] |
| 25 | `randombytes` (rng.c:151, `rng.c` variant) | `xlen == 0` | returns `RNG_SUCCESS` = `0`; still runs one `AES256_CTR_DRBG_Update` and bumps `reseed_counter` | [x] |
| 26 | `gen_message_random` / `hash_message` (all backends) | `mlen == 0` (empty message) | **no rejection**; sha2 takes the "cannot fill a block" branch | [x] |
| 27 | `handleErrors` (rng.c:107) | OpenSSL `EVP_*` failure inside `AES256_ECB` | `ERR_print_errors_fp(stderr); abort()` -- **unreachable**: the Rust port has no OpenSSL, and `EVP_aes_256_ecb`/`EVP_EncryptUpdate` over one 16-byte block cannot fail. Not differentially testable. | n/a |

## Compile-time assertions (`#error`) -- cannot fire at run time

| file | condition |
|------|-----------|
| `app/params/params-sphincs-*.h:30/35` | `SPX_WOTS_W` not in `{16, 256}` |
| `app/params/params-sphincs-*.h:42/47/52/57` | `SPX_N` outside `{2,..,256}` (no precomputed `SPX_WOTS_LEN2`) |
| `app/params/params-sphincs-*.h:64/69` | `SPX_D` does not divide `SPX_FULL_HEIGHT` |
| `app/src/address.c:22` | `SPX_TREE_HEIGHT * (SPX_D - 1) > 64` |
| `lib/blake/include/blake.h:10` | `SPX_BLAKE256_OUTPUT_BYTES < SPX_N` |
| `lib/sha2/include/sha2.h:14` | `SPX_SHA256_OUTPUT_BYTES < SPX_N` |
| `lib/sha2/src/hash_sha2.c:79` | `SPX_N > SPX_SHAX_BLOCK_BYTES` |
| `lib/sha2/src/hash_sha2.c:139` | `SPX_SHAX_BLOCK_BYTES` not a power of two |
| `lib/{blake,sha2,shake,haraka}/src/hash_*.c` | `SPX_TREE_BITS > 64` |

## Dead run-time branch

`hash_message()` in all four backends contains `if (SPX_D == 1) { *tree = 0; }`.
`SPX_D` is `7`, `8`, `17` or `22` in every shipped parameter set, so the
`SPX_D == 1` arm is **dead code** in all 48 configurations; only the `else` arm
is differentially observable.

## Undefined behaviour in C, deliberately NOT tested

* `compute_root(tree_height = 0)` -> `for (i = 0; i < tree_height - 1; i++)` with
  `uint32_t` wraps to `0xFFFFFFFF` iterations and walks off `auth_path`.
* `bytes_to_ull(inlen > 8)` -> shift count `>= 64`.
* `treehash`/`*_treehashx1` with `tree_height >= 32` -> `1 << tree_height` on `int`.

## Status

Every row above has a passing differential test in `tests/t10_errors.rs`
(`err01`..`err26`), run for **all 60 cargo feature combinations**.  Each test
asserts the *same* return value / sentinel on both sides (`-1`, `-2`, `-3`, `0`)
plus the same side effects (`*mlen`, the `memset` extent, the `AES_XOF_struct`
and the exported `DRBG_ctx` bytes), not merely "both failed".

Generic boundaries additionally covered there: NULL `sig`/`m`/`pk`,
NULL `x`/`provided_data`/`personalization_string`, `xlen`/`outlen`/`inlen`/
`mlen`/`smlen`/`siglen`/`inblocks` = 0, `siglen` = `SIZE_MAX`,
`maxlen` = `2^32` and `2^64-1`, one step past every documented range, and
out-of-range "enum" values (`SPX_ADDR_TYPE_*` = 7, 255, 256, 0x103, 0xFFFFFFFF)
passed across the FFI boundary.
