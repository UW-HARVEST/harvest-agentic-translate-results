# HASHING family — error / rejection paths

Ground truth is the C source in `c_src/libsodium`. "Result" is what the public
C entry point returns (or its observable behavior). `sodium_misuse()` calls in
the internal `blake2b`/`blake2b_init*` helpers are unreachable from the public
API because the public wrappers pre-validate `outlen`/`keylen` and return `-1`
first — so no test drives an abort; only the `-1` paths are exercised.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | crypto_generichash / crypto_generichash_blake2b | outlen == 0 | return -1 |
| 2 | crypto_generichash / crypto_generichash_blake2b | outlen > 64 (BLAKE2B_OUTBYTES) | return -1 |
| 3 | crypto_generichash / crypto_generichash_blake2b | keylen > 64 (BLAKE2B_KEYBYTES) | return -1 |
| 4 | crypto_generichash_blake2b_salt_personal | outlen == 0 | return -1 |
| 5 | crypto_generichash_blake2b_salt_personal | outlen > 64 | return -1 |
| 6 | crypto_generichash_blake2b_salt_personal | keylen > 64 | return -1 |
| 7 | crypto_generichash_init / crypto_generichash_blake2b_init | outlen == 0 | return -1 |
| 8 | crypto_generichash_init / crypto_generichash_blake2b_init | outlen > 64 | return -1 |
| 9 | crypto_generichash_init / crypto_generichash_blake2b_init | keylen > 64 | return -1 |
| 10 | crypto_generichash_blake2b_init_salt_personal | outlen == 0 \| outlen > 64 \| keylen > 64 | return -1 |
| 11 | crypto_generichash_blake2b_final | second call after finalize (blake2b_is_lastblock) | return -1 |
| 12 | crypto_hash_sha3256_final / crypto_hash_sha3512_final | called when state phase == FINALIZED (double final) | return -1 (and re-permutes, resets phase) |
| 13 | crypto_hash_sha3256_update / crypto_hash_sha3512_update | called when phase == FINALIZED (update after final) | return -1 (recovers: re-permutes, resets to ABSORBING, offset 0) |
| 14 | crypto_xof_shake128/256_update, crypto_xof_turboshake128/256_update | called when phase == SQUEEZING (update after squeeze) | return -1 (recovers: permutes, resets to ABSORBING) |
| 15 | crypto_xof_*_squeeze | squeeze then continue: first squeeze finalizes; subsequent squeezes always succeed | return 0 |

Notes on non-rejections (documented so tests don't over-assert):
- `crypto_generichash*` with `keylen == 0` and `key == NULL` is VALID (unkeyed).
- `crypto_generichash_blake2b_salt_personal` with `salt == NULL` and/or
  `personal == NULL` is VALID — C zero-fills those parameter fields, so the
  result equals plain `crypto_generichash_blake2b` for the same out/in/key.
- `crypto_xof_*` accept `outlen == 0` and `inlen == 0` and return 0.
- SHA-256/512 and SHA-3 one-shot functions always return 0 (no length limits
  besides pointer non-null contract, which is a caller precondition, not tested
  via null since `__attribute__((nonnull))` makes null UB rather than a checked
  error).
- `crypto_shorthash*` always return 0 for any input length; key is fixed 16
  bytes by contract (`nonnull`), so no in-band length rejection exists.
