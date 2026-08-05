# pubkey family — error / rejection paths

Every distinct rejection in the C source of the PUBLIC-KEY family
(`crypto_scalarmult*`, `crypto_sign_ed25519*`, `crypto_box*`, `crypto_kx*`,
`crypto_core_ed25519*`, `crypto_core_ristretto255*`), with the trigger a test
constructs and the expected C result. The Rust `.so` must return the same
sentinel and (where the buffer is defined) identical bytes.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | `crypto_scalarmult_curve25519` | point `p` of small order / identity (all-zero) so the product is the all-zero point | `-1` (constant-time all-zero-output check) |
| 2 | `crypto_scalarmult` (frontend) | same as #1 (delegates to curve25519) | `-1` |
| 3 | `crypto_scalarmult_ed25519` / `_noclamp` | `p` not a canonical / on-curve / main-subgroup point, or of small order | `-1` (`ge25519_is_canonical`/`frombytes`/`has_small_order`/`is_on_main_subgroup` gate) |
| 4 | `crypto_scalarmult_ed25519` / `_noclamp` | scalar `n` all-zero, or result is the point at infinity | `-1` (`_is_inf` / `sodium_is_zero(n)`) |
| 5 | `crypto_scalarmult_ed25519_base` / `_base_noclamp` | scalar `n` all-zero (result = infinity) | `-1` |
| 6 | `crypto_scalarmult_ristretto255` | `p` fails `ristretto255_frombytes` (not a valid ristretto encoding) | `-1` |
| 7 | `crypto_scalarmult_ristretto255` / `_base` | result decodes to all-zero (`sodium_is_zero(q)`) | `-1` |
| 8 | `crypto_sign_ed25519_verify_detached` | signature byte(s) tampered (bad `R` or `S`) | `-1` |
| 9 | `crypto_sign_ed25519_verify_detached` | message tampered after signing | `-1` |
| 10 | `crypto_sign_ed25519_verify_detached` | non-canonical `S` with high bits set, non-canonical `pk`, small-order `A`/`R`, or `frombytes` failure | `-1` |
| 11 | `crypto_sign_ed25519_open` | embedded signature/message tampered | `-1`, `*mlen_p = 0`, `m` zeroed |
| 12 | `crypto_sign_ed25519_open` | `smlen < 64` (too short to contain a signature) | `-1`, `*mlen_p = 0` |
| 13 | `crypto_box_open_easy` / `_open_easy_afternm` | `clen < crypto_box_MACBYTES` (16) | `-1` |
| 14 | `crypto_box_open_easy` / `_open_detached` / `_afternm` variants | ciphertext or MAC tampered (Poly1305 verify fails) | `-1` |
| 15 | `crypto_box_seal_open` | `clen < crypto_box_SEALBYTES` (48) | `-1` |
| 16 | `crypto_box_beforenm` / `crypto_box_easy` / `crypto_box_detached` | `crypto_scalarmult` of pk·sk yields all-zero (small-order pk) | `-1` (propagated) |
| 17 | `crypto_kx_client_session_keys` / `_server_session_keys` | peer public key produces zero shared secret (`crypto_scalarmult` returns `-1`) | `-1` |
| 18 | `crypto_core_ed25519_add` / `_sub` | either input not decodable / not on curve | `-1` |
| 19 | `crypto_core_ed25519_is_valid_point` | non-canonical, off-curve, small-order, or non-main-subgroup point | `0` (not `-1`; boolean predicate) |
| 20 | `crypto_core_ed25519_scalar_invert` | scalar `s` all-zero (no inverse) | `-1` |
| 21 | `crypto_core_ristretto255_add` / `_sub` | either input fails `ristretto255_frombytes` | `-1` |
| 22 | `crypto_core_ristretto255_is_valid_point` | input fails `ristretto255_frombytes` | `0` (boolean predicate) |
| 23 | `crypto_core_ristretto255_scalar_invert` | scalar all-zero | `-1` |

Notes:
- `crypto_core_*_is_valid_point` returns a boolean (`1`/`0`), not `-1`; the tests
  assert the exact `0`/`1` parity.
- `sodium_misuse()` paths (e.g. `mlen > MESSAGEBYTES_MAX` in `crypto_box_easy`)
  abort the process rather than return a sentinel, so they are not exercised as
  differential return-code tests.
