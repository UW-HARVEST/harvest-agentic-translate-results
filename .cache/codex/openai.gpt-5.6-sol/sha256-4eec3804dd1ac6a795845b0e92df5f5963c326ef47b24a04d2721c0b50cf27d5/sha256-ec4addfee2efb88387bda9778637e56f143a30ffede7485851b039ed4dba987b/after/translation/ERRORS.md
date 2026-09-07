# Error surface

Rows are mechanically derived from explicit C error returns, null/range
checks in `app/src/sign.c` and `app/src/rng.c`. The production
`randombytes.c` has no error return: it retries opening `/dev/urandom` forever.
Compile-time `#error` guards are configuration validity constraints, not
runtime input rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E01 | `crypto_sign_verify` | `siglen != SPX_BYTES` (covers zero, one short, one long, and oversized) | [x] returns `-1` before reading the signature |
| E02 | `crypto_sign_verify` | signature length is exact but the recomputed root differs from `pk + SPX_N` (mutated signature, message, or public key) | [x] returns `-1` |
| E03 | `crypto_sign_open` | `smlen < SPX_BYTES` | [x] zeroes exactly `smlen` output bytes, sets `*mlen = 0`, returns `-1` |
| E04 | `crypto_sign_open` | `smlen >= SPX_BYTES` but detached verification fails | [x] zeroes exactly `smlen` output bytes, sets `*mlen = 0`, returns `-1` |
| E05 | `seedexpander_init` | `maxlen >= 0x100000000` | [x] returns `RNG_BAD_MAXLEN` (`-1`) |
| E06 | `seedexpander` | output pointer `x == NULL` | [x] returns `RNG_BAD_OUTBUF` (`-2`) |
| E07 | `seedexpander` | `xlen >= ctx->length_remaining` (equality is rejected) | [x] returns `RNG_BAD_REQ_LEN` (`-3`) without decrementing `length_remaining` |
The `AES256_ECB` helper also aborts if OpenSSL allocation, initialization, or
encryption fails. That is an environmental dependency failure, not a
caller-controlled input rejection, so it is not an error-surface row.

Generic FFI boundaries to test in addition to the explicit rows:

- null message pointers with zero length;
- null required pointers are not rejected by C; dereferencing them is undefined
  behavior and therefore has no defined C result to reproduce;
- zero and oversized lengths;
- values one step outside each documented size;
- address “type” values outside `0..=6` (the C ABI accepts any `uint32_t` and
  stores its low byte; there is no rejection).
