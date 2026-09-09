| G3 | crypto_generichash_blake2b | `outlen == 0` (checked `outlen <= 0U`) | returns -1, `out` untouched |
| G3 | crypto_generichash_blake2b | `outlen > 64` (BLAKE2B_OUTBYTES / crypto_generichash_blake2b_BYTES_MAX), e.g. 65, 255, 256, SIZE_MAX | returns -1 |
| G3 | crypto_generichash_blake2b | `keylen > 64` (BLAKE2B_KEYBYTES / crypto_generichash_blake2b_KEYBYTES_MAX), e.g. 65, 255, 256 | returns -1 |
| G3 | crypto_generichash_blake2b | `inlen > UINT64_MAX` — dead branch (`inlen` is `unsigned long long`), unreachable on all supported platforms | never taken; documents intent only |
| G3 | crypto_generichash_blake2b | `assert(outlen <= UINT8_MAX)` / `assert(keylen <= UINT8_MAX)` after the range check — unreachable; in `-DNDEBUG`-less builds would `abort()` | unreachable (guarded by the `> 64` checks above) |
| G3 | crypto_generichash (wrapper) | any of: `outlen == 0`, `outlen > 64`, `keylen > 64` | returns -1 (delegates to crypto_generichash_blake2b) |
| G3 | crypto_generichash_blake2b_salt_personal | `outlen == 0` | returns -1 |
| G3 | crypto_generichash_blake2b_salt_personal | `outlen > 64` | returns -1 |
| G3 | crypto_generichash_blake2b_salt_personal | `keylen > 64` | returns -1 |
| G3 | crypto_generichash_blake2b_init | `outlen == 0` | returns -1, state left uninitialized |
| G3 | crypto_generichash_blake2b_init | `outlen > 64` | returns -1, state left uninitialized |
| G3 | crypto_generichash_blake2b_init | `keylen > 64` | returns -1, state left uninitialized |
| G3 | crypto_generichash_init (wrapper) | `outlen == 0`, `outlen > 64`, or `keylen > 64` | returns -1 |
| G3 | crypto_generichash_blake2b_init_salt_personal | `outlen == 0` | returns -1 |
| G3 | crypto_generichash_blake2b_init_salt_personal | `outlen > 64` | returns -1 |
| G3 | crypto_generichash_blake2b_init_salt_personal | `keylen > 64` | returns -1 |
| G3 | crypto_generichash_blake2b_init / _init_salt_personal | inner `blake2b_init*() != 0` (LCOV_EXCL_LINE) — `blake2b_init*` only ever returns 0 or calls `sodium_misuse()` | unreachable `return -1` |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `outlen == 0` — NOT checked in the wrapper; reaches `blake2b_final`'s `!outlen` guard | `sodium_misuse()` -> abort/handler (NOT -1) |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `65 <= outlen <= 255` — no wrapper check; `blake2b_final` sees `outlen > BLAKE2B_OUTBYTES` | `sodium_misuse()` -> abort/handler |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `outlen >= 256` where `outlen & 0xff == 0` (e.g. 256, 512): `(uint8_t) outlen == 0` after truncation | `sodium_misuse()` -> abort/handler |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `outlen > 255` triggers `assert(outlen <= UINT8_MAX)` in assert-enabled builds | `abort()` via failed assertion |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `outlen >= 256` with `1 <= (outlen & 0xff) <= 64` (e.g. 288 -> 32) in NDEBUG builds: silently truncated, no error | returns 0 with WRONG length (silent misuse, no rejection) |
| G3 | crypto_generichash_blake2b_final / crypto_generichash_final | `outlen` differs from the `outlen` passed to `_init` (e.g. init 32, final 64) — never validated | returns 0, digest for the init-time length parameter, no rejection |
| G3 | blake2b_final (via crypto_generichash_blake2b_final) | state already finalized: `blake2b_is_lastblock(S)` true (i.e. `_final` called twice, or `_update`+`_final` after a prior `_final`) | returns -1 (the only state-misuse guard in blake2b) |
| G3 | blake2b_final | internal invariant `assert(S->buflen <= BLAKE2B_BLOCKBYTES)` after the 128-byte drain | `abort()` if the buffer invariant is violated |
| G3 | blake2b_init | `outlen == 0` (`!outlen`) | `sodium_misuse()` |
| G3 | blake2b_init | `outlen > 64` (BLAKE2B_OUTBYTES) | `sodium_misuse()` |
| G3 | blake2b_init_salt_personal | `outlen == 0` | `sodium_misuse()` |
| G3 | blake2b_init_salt_personal | `outlen > 64` | `sodium_misuse()` |
| G3 | blake2b_init_key | `outlen == 0` | `sodium_misuse()` |
| G3 | blake2b_init_key | `outlen > 64` | `sodium_misuse()` |
| G3 | blake2b_init_key | `key == NULL` | `sodium_misuse()` |
| G3 | blake2b_init_key | `keylen == 0` (`!keylen`) | `sodium_misuse()` |
| G3 | blake2b_init_key | `keylen > 64` (BLAKE2B_KEYBYTES) | `sodium_misuse()` |
| G3 | blake2b_init_key | `blake2b_init_param(S, P) < 0` (LCOV_EXCL_LINE, unreachable) | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `outlen == 0` | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `outlen > 64` | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `key == NULL` | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `keylen == 0` | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `keylen > 64` | `sodium_misuse()` |
| G3 | blake2b_init_key_salt_personal | `blake2b_init_param(S, P) < 0` (unreachable) | `sodium_misuse()` |
| G3 | blake2b | `in == NULL && inlen > 0` | `sodium_misuse()` |
| G3 | blake2b | `out == NULL` | `sodium_misuse()` |
| G3 | blake2b | `outlen == 0` | `sodium_misuse()` |
| G3 | blake2b | `outlen > 64` | `sodium_misuse()` |
| G3 | blake2b | `key == NULL && keylen > 0` | `sodium_misuse()` |
| G3 | blake2b | `keylen > 64` | `sodium_misuse()` |
| G3 | blake2b | `blake2b_init_key(...) < 0` / `blake2b_init(...) < 0` (unreachable, LCOV_EXCL_LINE) | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `in == NULL && inlen > 0` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `out == NULL` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `outlen == 0` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `outlen > 64` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `key == NULL && keylen > 0` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `keylen > 64` | `sodium_misuse()` |
| G3 | blake2b_salt_personal | `blake2b_init_key_salt_personal(...) < 0` / `blake2b_init_salt_personal(...) < 0` (unreachable) | `sodium_misuse()` |
| G3 | blake2b_init_param | `COMPILER_ASSERT(sizeof *P == 64)` | compile-time failure only |
| G3 | crypto_generichash_blake2b_init | `COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state)` (state opaque[384]) | compile-time failure only |
| G3 | crypto_generichash* (all) | `state == NULL` / `out == NULL` where header declares `__attribute__((nonnull))` | no runtime check: undefined behavior / segfault, NOT -1 |
| G3 | crypto_generichash_blake2b (unkeyed path) | `key == NULL, keylen == 0` is VALID (unkeyed); but `key != NULL, keylen == 0` also silently takes the unkeyed path — no rejection | returns 0, unkeyed digest |
| G3 | crypto_generichash_blake2b | `0 < keylen < 16` (below crypto_generichash_blake2b_KEYBYTES_MIN) is NOT rejected — only `> 64` is checked | returns 0 (documented minimum not enforced) |
| G3 | crypto_generichash_blake2b | `0 < outlen < 16` (below crypto_generichash_blake2b_BYTES_MIN) is NOT rejected — only `== 0` and `> 64` are checked | returns 0 (documented minimum not enforced) |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len < 16` (crypto_kdf_blake2b_BYTES_MIN), e.g. 0, 1, 15 | sets `errno = EINVAL`, returns -1 |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len > 64` (crypto_kdf_blake2b_BYTES_MAX), e.g. 65, 128, SIZE_MAX | sets `errno = EINVAL`, returns -1 |
| G3 | crypto_kdf_blake2b_derive_from_key | `ctx == NULL`: the `memcpy(ctx_padded, ctx, 8)` happens BEFORE the `subkey_len` bound check | undefined behavior / segfault before any -1 can be returned |
| G3 | crypto_kdf_blake2b_derive_from_key | `key == NULL` — never checked; forwarded to blake2b as a 32-byte key with `keylen != 0` | `sodium_misuse()` inside `blake2b_init_key_salt_personal` (`!key`) |
| G3 | crypto_kdf_derive_from_key (wrapper) | `subkey_len < 16` or `subkey_len > 64` | `errno = EINVAL`, returns -1 |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_id` is a plain `uint64_t` stored LE in the blake2b salt — 0, 1, UINT64_MAX all valid, no bound check exists | returns 0 (no rejection path for subkey_id) |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len > crypto_kdf_hkdf_sha256_BYTES_MAX` = `0xff * 32` = 8160 (e.g. 8161) | sets `errno = EINVAL`, returns -1 |
| G3 | crypto_kdf_hkdf_sha512_expand | `out_len > crypto_kdf_hkdf_sha512_BYTES_MAX` = `0xff * 64` = 16320 (e.g. 16321) | sets `errno = EINVAL`, returns -1 |
| G3 | crypto_kdf_hkdf_sha256_expand / _sha512_expand | `ctx == NULL && ctx_len > 0` — never checked | undefined behavior inside `crypto_hash_sha*_update`, no -1 |
| G3 | crypto_kdf_hkdf_sha256_expand / _sha512_expand | `ctx_len` has NO upper bound check (any length accepted) | returns 0 |
| G3 | crypto_kdf_hkdf_sha256_extract_init | `salt == NULL && 0 < salt_len <= 64` -> `crypto_auth_hmacsha256_init` `key == NULL && keylen > 0` | `sodium_misuse()` |
| G3 | crypto_kdf_hkdf_sha512_extract_init | `salt == NULL && 0 < salt_len <= 128` -> `crypto_auth_hmacsha512_init` `key == NULL && keylen > 0` | `sodium_misuse()` |
| G3 | crypto_kdf_hkdf_sha256_extract_init | `salt == NULL && salt_len > 64`: takes the key-hashing branch FIRST, so the NULL check is skipped | NULL dereference / UB (no `sodium_misuse()`) |
| G3 | crypto_kdf_hkdf_sha512_extract_init | `salt == NULL && salt_len > 128`: key-hashing branch skips the NULL check | NULL dereference / UB |
| G3 | crypto_kdf_hkdf_sha256_extract_update / _sha512_extract_update | no validation at all (`ikm == NULL && ikm_len > 0` unchecked) | always returns 0 (UB on bad pointer) |
| G3 | crypto_kdf_hkdf_sha256_extract_final / _sha512_extract_final | no rejection path; zeroizes state unconditionally | always returns 0 |
| G3 | crypto_kdf_hkdf_sha256_extract / _sha512_extract | no rejection path (return value of `_extract_final`, always 0) | always returns 0 |
| G3 | crypto_kdf_hkdf_sha256_keygen / _sha512_keygen | `void` return, no failure mode | n/a |
| G3 | crypto_hash_sha256, crypto_hash_sha256_init/_update/_final | NO rejection paths exist (no length/NULL/state checks) | always returns 0 |
| G3 | crypto_hash_sha512, crypto_hash_sha512_init/_update/_final, crypto_hash | NO rejection paths exist | always returns 0 |
| G3 | crypto_hash_sha256_update / crypto_hash_sha512_update | `inlen == 0` early-returns before touching `in` — `in == NULL, inlen == 0` is accepted | returns 0 |
| G3 | crypto_hash_sha256/512_update | `in == NULL && inlen > 0` (header `nonnull(1)` only covers `state`) | undefined behavior, not -1 |
| G3 | crypto_hash_sha256/512_final | calling `_final` twice on the same state — no state-phase guard | returns 0 with a garbage/second digest (silent misuse) |
| G3 | crypto_hash_sha3256_update / crypto_hash_sha3512_update | called when `state->phase == SHA3_PHASE_FINALIZED` (i.e. `_update` after `_final`): permutes, resets phase to ABSORBING and offset to 0 | returns -1 but STILL absorbs the input (soft error, state is silently reset) |
| G3 | crypto_hash_sha3256_final / crypto_hash_sha3512_final | called when `state->phase == SHA3_PHASE_FINALIZED` (i.e. `_final` called twice): permutes without re-padding | returns -1 AND still writes `outlen` bytes to `out` |
| G3 | crypto_hash_sha3256 / crypto_hash_sha3512 (one-shot) | no rejection path; return values of init/update/final are discarded | always returns 0 |
| G3 | crypto_hash_sha3_384 | NOT PRESENT in libsodium 1.0.23 — only `crypto_hash_sha3256` (32B, rate 136) and `crypto_hash_sha3512` (64B, rate 72) are exported | n/a (no such symbol; do not test) |
| G3 | crypto_hash_sha3256/512_* | `state == NULL` / `out == NULL` (`__attribute__((nonnull))`) | UB, no runtime rejection |
| G3 | crypto_hash_sha3256_init / crypto_hash_sha3512_init | `COMPILER_ASSERT(sizeof(crypto_hash_sha3*_state) >= sizeof(sha3_state_internal))` (opaque[256]) | compile-time failure only |
| G3 | crypto_xof_shake128_update | `state->phase != SHAKE128_PHASE_ABSORBING` (update after squeeze): permutes, phase reset to ABSORBING, offset = 0 | returns -1 but still absorbs the input |
| G3 | crypto_xof_shake256_update | `state->phase != SHAKE256_PHASE_ABSORBING` (update after squeeze) | returns -1, still absorbs |
| G3 | crypto_xof_turboshake128_update | `state->phase != TURBOSHAKE128_PHASE_ABSORBING` (update after squeeze); uses `permute_12` | returns -1, still absorbs |
| G3 | crypto_xof_turboshake256_update | `state->phase != TURBOSHAKE256_PHASE_ABSORBING` (update after squeeze); uses `permute_12` | returns -1, still absorbs |
| G3 | crypto_xof_shake128/256_squeeze, crypto_xof_turboshake128/256_squeeze | NO rejection path: `outlen == 0`, huge `outlen`, squeeze-before-any-update are all accepted | always returns 0 |
| G3 | crypto_xof_shake128/256, crypto_xof_turboshake128/256 (one-shot) | no rejection path; return values of init/update/squeeze discarded | always returns 0 |
| G3 | crypto_xof_shake128/256_init_with_domain, crypto_xof_turboshake128/256_init_with_domain | `domain` byte is NOT validated: `0x00`, `0x80`..`0xFF` are accepted even though only `0x01`..`0x7F` are meaningful for TurboSHAKE; `domain == 0x80` XORs to 0x00 at the last-byte special case | always returns 0 (no rejection; degenerate/nonstandard padding) |
| G3 | crypto_xof_*_init / _init_with_domain | `COMPILER_ASSERT(sizeof(crypto_xof_*_state) >= sizeof(*_state_internal))` (STATEBYTES 256) | compile-time failure only |
| G3 | crypto_xof_*_update / _squeeze | `state == NULL` / `out == NULL` (`nonnull`) | UB, no runtime rejection |
| G3 | crypto_core_keccak1600_init/_xor_bytes/_extract_bytes/_permute_24/_permute_12 | all return `void`; NO bound check on `offset`/`length` (`offset + length > 200` writes/reads past the 1600-bit state) | no error signalling at all; out-of-range offset/length is UB |
| G3 | crypto_auth_hmacsha256_init | `key == NULL && 0 < keylen <= 64` | `sodium_misuse()` |
| G3 | crypto_auth_hmacsha256_init | `key == NULL && keylen > 64`: the `keylen > 64` key-hashing branch is taken first, bypassing the NULL check | NULL dereference / UB (no `sodium_misuse()`) |
| G3 | crypto_auth_hmacsha512_init | `key == NULL && 0 < keylen <= 128` | `sodium_misuse()` |
| G3 | crypto_auth_hmacsha512_init | `key == NULL && keylen > 128`: key-hashing branch bypasses the NULL check | NULL dereference / UB |
| G3 | crypto_auth_hmacsha512256_init | `key == NULL && 0 < keylen <= 128` (delegates to `crypto_auth_hmacsha512_init`) | `sodium_misuse()` |
| G3 | crypto_auth_hmacsha256_init / _hmacsha512_init | `key == NULL && keylen == 0` is ACCEPTED (all-zero HMAC key) — no rejection | returns 0 |
| G3 | crypto_auth_hmacsha256_update / _hmacsha512_update / _hmacsha512256_update | no validation of any kind | always returns 0 |
| G3 | crypto_auth_hmacsha256_final / _hmacsha512_final / _hmacsha512256_final | no state-phase guard; `_final` twice is not detected | always returns 0 (silent misuse) |
| G3 | crypto_auth_hmacsha256 / _hmacsha512 / _hmacsha512256 / crypto_auth | no rejection path; always uses `keylen = 32` (KEYBYTES) | always returns 0 |
| G3 | crypto_auth_hmacsha256_verify | `h` != computed 32-byte MAC -> `crypto_verify_32(h, correct)` returns -1 | returns -1 (constant-time; `correct` zeroized only implicitly) |
| G3 | crypto_auth_hmacsha512_verify | `h` != computed 64-byte MAC -> `crypto_verify_64(h, correct)` returns -1 | returns -1 |
| G3 | crypto_auth_hmacsha512256_verify / crypto_auth_verify | `h` != computed 32-byte truncated MAC -> `crypto_verify_32` returns -1 | returns -1 |
| G3 | crypto_auth_hmacsha256/512/512256_verify | hardening term `(-(h == correct))`: if the caller's `h` aliases the stack buffer `correct` (unreachable in practice) | forces -1 |
| G3 | crypto_auth_hmacsha256/512/512256_verify | belt-and-braces `sodium_memcmp(correct, h, N)` OR-ed in: any mismatch also contributes -1 | returns -1 (result is the bitwise OR of three checks; success requires all three == 0) |
| G3 | crypto_onetimeauth_poly1305_verify / crypto_onetimeauth_verify | `h` != computed 16-byte tag -> `crypto_verify_16(h, correct)` returns -1 | returns -1 |
| G3 | crypto_onetimeauth_poly1305 / _init / _update / _final and crypto_onetimeauth wrappers | NO rejection paths: no NULL check, no length check, no state-phase guard; `_final` twice is undetected | always returns 0 |
| G3 | crypto_onetimeauth_poly1305_donna_init | `COMPILER_ASSERT(sizeof(crypto_onetimeauth_poly1305_state) >= sizeof(poly1305_state_internal_t))` | compile-time failure only |
| G3 | crypto_shorthash_siphash24 / crypto_shorthash | NO rejection paths; `k` is read as exactly 16 bytes with no length parameter and no NULL check | always returns 0 |
| G3 | crypto_shorthash_siphashx24 | NO rejection paths; `k` read as exactly 16 bytes, `out` written as exactly 16 bytes | always returns 0 |
| G3 | crypto_shorthash_siphash24 / _siphashx24 | `in == NULL && inlen == 0` is safe (`end = in` when `inlen == 0`, `switch (left)` hits `case 0`) | returns 0 |
| G3 | crypto_shorthash_siphash24 / _siphashx24 | `in == NULL && inlen > 0` — unchecked | undefined behavior, not -1 |
| G3 | crypto_generichash_statebytes / crypto_generichash_blake2b_statebytes / crypto_hash_sha*_statebytes / crypto_xof_*_statebytes / crypto_auth_*_statebytes / crypto_onetimeauth_*_statebytes / crypto_kdf_hkdf_*_statebytes / crypto_core_keccak1600_statebytes | pure accessors, no failure mode | n/a |
| G3 | crypto_generichash_keygen / crypto_kdf_keygen / crypto_auth_*_keygen / crypto_onetimeauth_poly1305_keygen / crypto_shorthash_keygen / crypto_kdf_hkdf_*_keygen | `void` return; failure only via `randombytes` internal abort | n/a (no -1) |
