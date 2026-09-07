# Error-surface table

Mechanically derived from explicit input-rejection returns and null/range
checks in the public C library. Pointer-taking APIs not listed below contain no
C null check; passing null where storage is dereferenced is C undefined
behavior rather than a library rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `seedexpander_init` | `maxlen >= 0x100000000` | `RNG_BAD_MAXLEN` (`-1`) | [x] |
| 2 | `seedexpander` | output pointer `x == NULL` | `RNG_BAD_OUTBUF` (`-2`) | [x] |
| 3 | `seedexpander` | `xlen >= ctx->length_remaining` (including equality) | `RNG_BAD_REQ_LEN` (`-3`) | [x] |
| 4 | `crypto_sign_verify` | `siglen != SPX_BYTES` (zero, one short, or one long included) | `-1` | [x] |
| 5 | `crypto_sign_verify` | reconstructed root differs from `pk + SPX_N` | `-1` | [x] |
| 6 | `crypto_sign_open` | `smlen < SPX_BYTES` | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` | [x] |
| 7 | `crypto_sign_open` | detached verification of the signed-message prefix fails | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` | [x] |

The same mechanical scan found the following non-input fault guards. They are
not Phase C rows because no public input can construct their condition:

- `AES256_ECB` prints OpenSSL errors and aborts if
  `EVP_CIPHER_CTX_new()`, `EVP_EncryptInit_ex()`, or
  `EVP_EncryptUpdate()` reports an OpenSSL allocation/provider failure.
- The deterministic C KAT driver returns `KAT_OVERFLOW` if its internally
  generated length exceeds its compile-time buffer, and
  `KAT_CRYPTO_FAILURE` if an internal signing call fails or its own
  sign/open consistency checks fail. The driver has no input that can request
  these states; its reachable execution is compared byte-for-byte in every
  build configuration.

No public C API uses an enum type. Address type is a raw `uint32_t`; all
values, including values outside `0..=6`, are accepted and encoded without an
error return.
