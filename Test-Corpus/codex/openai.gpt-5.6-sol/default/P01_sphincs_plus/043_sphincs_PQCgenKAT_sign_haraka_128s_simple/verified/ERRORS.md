# Error surface

Rows are mechanically derived from explicit runtime rejection/error branches
in the public library sources (`app/src/sign.c`, `app/src/rng.c`, and
`app/src/randombytes.c`). Compile-time `#error` constraints and the standalone
KAT driver's private exit codes are build/application checks, not callable
shared-library input rejections.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `seedexpander_init` | `maxlen >= 0x100000000` | `RNG_BAD_MAXLEN` (`-1`); context is otherwise untouched | [ ] |
| 2 | `seedexpander` | output pointer `x == NULL` | `RNG_BAD_OUTBUF` (`-2`) | [ ] |
| 3 | `seedexpander` | `xlen >= ctx->length_remaining` (equality is rejected) | `RNG_BAD_REQ_LEN` (`-3`); remaining length is unchanged | [ ] |
| 4 | `crypto_sign_verify` | `siglen != SPX_BYTES`, including zero, one short, and one long | `-1` before reading signature contents | [ ] |
| 5 | `crypto_sign_verify` | computed final root differs from `pk + SPX_N` | `-1` | [ ] |
| 6 | `crypto_sign_open` | `smlen < SPX_BYTES` | zero exactly `smlen` output bytes, set `*mlen = 0`, return `-1` | [ ] |
| 7 | `crypto_sign_open` | embedded signature verification returns nonzero | zero exactly `smlen` output bytes, set `*mlen = 0`, return `-1` | [ ] |
| 8 | `AES256_ECB` | `EVP_CIPHER_CTX_new()` fails | print OpenSSL errors and `abort()` | [ ] |
| 9 | `AES256_ECB` | `EVP_EncryptInit_ex(...) != 1` | print OpenSSL errors and `abort()` | [ ] |
| 10 | `AES256_ECB` | `EVP_EncryptUpdate(...) != 1` | print OpenSSL errors and `abort()` | [ ] |
| 11 | OS `randombytes` | opening `/dev/urandom` returns `-1` | sleep one second and retry indefinitely | [ ] |
| 12 | OS `randombytes` | `read` returns less than 1 | sleep one second and retry without consuming requested length | [ ] |

There are no C enum-typed public parameters. Address types are `uint32_t`;
values outside the named `0..=6` constants are accepted and truncated to one
byte, so they belong to the valid configuration surface rather than this
rejection table.

