# Error surface

This table comes from error returns and abort sites in `c_src/app/src/sign.c`
and `c_src/app/src/rng.c`. The KAT driver's private process exit codes are not
shared-library API symbols and are excluded. No C source contains `assert`,
`RETURN_ERROR`, or an error enum parameter.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `crypto_sign_verify` | `siglen != SPX_BYTES` | return `-1` before reading `sig` |
| 2 | `crypto_sign_verify` | computed root differs from `pk + SPX_N` | return `-1` |
| 3 | `crypto_sign_open` | `smlen < SPX_BYTES` | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` |
| 4 | `crypto_sign_open` | nested `crypto_sign_verify(...) != 0` | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` |
| 5 | `seedexpander_init` | `maxlen >= 0x100000000` | return `RNG_BAD_MAXLEN` (`-1`) without initializing `ctx` |
| 6 | `seedexpander` | `x == NULL` | return `RNG_BAD_OUTBUF` (`-2`) |
| 7 | `seedexpander` | `xlen >= ctx->length_remaining` (equality is rejected) | return `RNG_BAD_REQ_LEN` (`-3`) |
The source also has three environmental failure branches inside `AES256_ECB`:
`EVP_CIPHER_CTX_new() == NULL`, `EVP_EncryptInit_ex(...) != 1`, and
`EVP_EncryptUpdate(...) != 1`. Each prints the OpenSSL error queue and aborts.
They cannot be triggered by an invalid value within the function's C input
contract (32-byte key, 16-byte input, 16-byte output), so they are recorded
here but are not input-rejection rows.

## Boundary contracts

The following facts are also mechanically visible in the C declarations and
branches and are covered by the differential suite:

- Pointer parameters other than `seedexpander.x` have no null rejection.
  A null pointer is accepted only when the corresponding length is zero and
  the implementation does not dereference it; otherwise C behavior is
  undefined rather than an API error result.
- Length-zero inputs are valid for one-shot/streaming hashes, MGF output,
  message hashing, DRBG output, and the endian conversion loops.
- `seedexpander` uniquely rejects a zero request when
  `length_remaining == 0`, because its check is `>=`.
- There are no FFI enum parameters. `SPX_set_type` takes `uint32_t`; values
  outside the named constants `0..6` are valid and are truncated to one byte.
- The only explicit caller-visible maximum is
  `seedexpander_init.maxlen < 2^32`.
- SHA-256 and BLAKE-256 headers have compile-time `SPX_N <= 32` checks; all six
  valid parameter sets satisfy them.

Rows are checked only after both shared objects produce the listed result:

| row | status |
|-----|--------|
| 1 | [x] |
| 2 | [x] |
| 3 | [x] |
| 4 | [x] |
| 5 | [x] |
| 6 | [x] |
| 7 | [x] |
