# Error Surface

This table is derived from rejection branches in the shared-library C sources.
The KAT driver is an executable, not part of the shared-library API, so its
driver-only exit codes are excluded. Compile-time `#error` directives reject
invalid builds rather than runtime inputs and are covered by the feature matrix.
Pointers not explicitly checked by C have undefined behavior and are not
invented here as C rejection cases.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `seedexpander_init` | `maxlen >= 0x100000000` | `RNG_BAD_MAXLEN` (`-1`) | [x] |
| 2 | `seedexpander` | `x == NULL` | `RNG_BAD_OUTBUF` (`-2`) | [x] |
| 3 | `seedexpander` | `xlen >= ctx->length_remaining` (including equality) | `RNG_BAD_REQ_LEN` (`-3`) | [x] |
| 4 | `crypto_sign_verify` | `siglen != SPX_BYTES` | `-1` | [x] |
| 5 | `crypto_sign_verify` | computed root differs from `pk + SPX_N` | `-1` | [x] |
| 6 | `crypto_sign_open` | `smlen < SPX_BYTES` | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` | [x] |
| 7 | `crypto_sign_open` | embedded signature verification returns nonzero | zero `m[0..smlen]`, set `*mlen = 0`, return `-1` | [x] |

