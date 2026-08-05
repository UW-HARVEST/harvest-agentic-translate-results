# authmac — Error / rejection paths

Family: AUTH / MAC / VERIFY (`crypto_verify_*`, `crypto_auth*`,
`crypto_onetimeauth*`). Return codes / sentinels verified byte-for-byte against
the C library through the FFI boundary.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | `crypto_verify_16` | inputs differ in any byte/bit | `-1` (constant-time) |
| 2 | `crypto_verify_16` | inputs equal | `0` |
| 3 | `crypto_verify_32` | inputs differ in any byte/bit | `-1` (constant-time) |
| 4 | `crypto_verify_32` | inputs equal | `0` |
| 5 | `crypto_verify_64` | inputs differ in any byte/bit | `-1` (constant-time) |
| 6 | `crypto_verify_64` | inputs equal | `0` |
| 7 | `crypto_auth_verify` (hmacsha512256) | tag bit flipped | `-1` |
| 8 | `crypto_auth_verify` | wrong key | `-1` |
| 9 | `crypto_auth_verify` | truncated message (verify over inlen-1) | `-1` |
| 10 | `crypto_auth_hmacsha256_verify` | tampered tag | `-1` |
| 11 | `crypto_auth_hmacsha256_verify` | wrong key | `-1` |
| 12 | `crypto_auth_hmacsha256_verify` | truncated message | `-1` |
| 13 | `crypto_auth_hmacsha512_verify` | tampered tag | `-1` |
| 14 | `crypto_auth_hmacsha512_verify` | wrong key | `-1` |
| 15 | `crypto_auth_hmacsha512_verify` | truncated message | `-1` |
| 16 | `crypto_auth_hmacsha512256_verify` | tampered tag | `-1` |
| 17 | `crypto_auth_hmacsha512256_verify` | wrong key | `-1` |
| 18 | `crypto_auth_hmacsha512256_verify` | truncated message | `-1` |
| 19 | `crypto_onetimeauth_verify` (poly1305) | tampered tag | `-1` |
| 20 | `crypto_onetimeauth_verify` | wrong (fresh) key | `-1` |
| 21 | `crypto_onetimeauth_verify` | truncated message | `-1` |
| 22 | `crypto_onetimeauth_poly1305_verify` | tampered tag | `-1` |
| 23 | `crypto_onetimeauth_poly1305_verify` | wrong (fresh) key | `-1` |
| 24 | `crypto_onetimeauth_poly1305_verify` | truncated message | `-1` |

## Notes on C behavior (from `c_src/libsodium`)

- All `*_verify` functions compute the correct tag and return
  `crypto_verify_N(h, correct)` (constant-time). A match returns `0`, any
  mismatch returns `-1`. The comparison is always full-length and
  constant-time; there is no early-out. (See
  `crypto_auth/hmacsha256/auth_hmacsha256.c:114` and the poly1305
  `onetimeauth_verify` path.)
- `crypto_verify_N` (portable path in `src/verify.rs`, translated from
  `crypto_verify/verify.c`) ORs all byte differences into an accumulator and
  maps non-zero to `-1` via `(1 & ((d - 1) >> 8)) - 1`. Equal ⇒ `0`,
  differing ⇒ `-1`.
- **Non-rejection misuse paths (not asserted as `-1`):** the HMAC `*_init`
  functions call `sodium_misuse()` (which `abort()`s) when `key == NULL` **and**
  `keylen > 0`. This is process-terminating, not a return-code path, so it is
  out of scope for a return-value differential and is not exercised. The
  `key == NULL, keylen == 0` case is a valid no-op and IS covered (klen=0 in the
  streaming tests).
- **poly1305 wrong-key caveat:** poly1305 clamps the `r` half of the key, so a
  single-bit key flip can land in a clamped bit and reproduce the same tag
  (giving `0`, matching on both libraries). The wrong-key error test therefore
  uses a fully independent fresh random key so the `-1` result is guaranteed
  (collision probability ~2^-128). C and Rust agree in both regimes.
