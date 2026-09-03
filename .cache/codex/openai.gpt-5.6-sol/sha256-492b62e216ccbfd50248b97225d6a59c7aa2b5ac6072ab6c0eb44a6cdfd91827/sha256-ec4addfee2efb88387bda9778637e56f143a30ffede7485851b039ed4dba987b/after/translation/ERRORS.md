# Error-surface table

This table comes from mechanical searches for negative/sentinel returns,
null/range checks, error constants, aborts, and compile-time `#error` guards.
Every row below is an input-triggerable API rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---:|---|---|---|:---:|
| 1 | `crypto_sign_verify` | `siglen != SPX_BYTES` | return `-1` before reading the signature | [x] |
| 2 | `crypto_sign_verify` | reconstructed root differs from `pk + SPX_N` (`memcmp(...) != 0`) | return `-1` | [x] |
| 3 | `crypto_sign_open` | `smlen < SPX_BYTES` | zero exactly `smlen` output bytes, set `*mlen = 0`, return `-1` | [x] |
| 4 | `crypto_sign_open` | embedded `crypto_sign_verify(...)` returns nonzero | zero exactly `smlen` output bytes, set `*mlen = 0`, return `-1` | [x] |
| 5 | `seedexpander_init` | `maxlen >= 0x100000000` | return `RNG_BAD_MAXLEN` (`-1`) | [x] |
| 6 | `seedexpander` | output pointer `x == NULL` | return `RNG_BAD_OUTBUF` (`-2`) | [x] |
| 7 | `seedexpander` | `xlen >= ctx->length_remaining` (equality is rejected) | return `RNG_BAD_REQ_LEN` (`-3`) without consuming bytes | [x] |

No C `assert` statements, `RETURN_ERROR` macros, `return NULL` statements, or
public enum-valued inputs exist in this source tree.

`AES256_ECB` also aborts if OpenSSL context allocation, initialization, or
encryption fails. Those are dependency failures rather than invalid FFI input
conditions, so they are not error-surface rows.

Build-time guards (not FFI inputs) reject: a BLAKE/SHA-256 output shorter than
`SPX_N`; subtree addressing needing more than 64 bits; message tree indices
needing more than 64 bits; SHA input parameters outside the implementation's
supported block-size assumptions.
