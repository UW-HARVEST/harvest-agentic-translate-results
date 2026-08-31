## Area 4 — crypto_auth + crypto_onetimeauth

Files analysed (READ-ONLY):
- `c_src/libsodium/crypto_auth/crypto_auth.c`
- `c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c`
- `c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c`
- `c_src/libsodium/crypto_auth/hmacsha512256/auth_hmacsha512256.c`
- `c_src/libsodium/crypto_onetimeauth/crypto_onetimeauth.c`
- `c_src/libsodium/crypto_onetimeauth/poly1305/onetimeauth_poly1305.c`
- `c_src/libsodium/crypto_onetimeauth/poly1305/donna/poly1305_donna.c` (`sse2/` is **not** selected: no `HAVE_TI_MODE`/`HAVE_EMMINTRIN_H`, so `poly1305_donna32.h` 32-bit limbs are used and `crypto_onetimeauth_poly1305_donna_implementation` is the only implementation ever installed)
- headers: `include/sodium/crypto_auth.h`, `crypto_auth_hmacsha256.h`, `crypto_auth_hmacsha512.h`, `crypto_auth_hmacsha512256.h`, `crypto_onetimeauth.h`, `crypto_onetimeauth_poly1305.h`

Return-value primitives used by this area (both take the portable, non-SSE2 branch in this build):
- `crypto_verify_16/32/64` (`crypto_verify/verify.c`) — constant-time, returns `0` iff all bytes equal, else `-1`.
- `sodium_memcmp` (`sodium/utils.c`) — constant-time, returns `0` iff equal, else `-1`.
- `sodium_misuse()` (`sodium/core.c`) — calls the installed misuse handler if any, then `abort()`; it **never returns**.

Every HMAC `*_verify` in this area returns the bitwise OR of three terms:
`crypto_verify_N(h, correct) | (-(h == correct)) | sodium_memcmp(correct, h, N)`
so any single failing term forces `-1`. The poly1305 verify uses only `crypto_verify_16`.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 4.1 | `crypto_auth_hmacsha256_verify` | at least one bit of `h[0..31]` differs from `HMAC-SHA-256(k, in)`; first OR-term `crypto_verify_32(h, correct)` yields `-1` | returns `-1` (constant time; no output written) | verified |
| 4.2 | `crypto_auth_hmacsha256_verify` | third OR-term `sodium_memcmp(correct, h, 32)` yields `-1` on the same mismatch — redundant second constant-time compare, must also be modelled so the result is `-1` even if `crypto_verify_32` were bypassed | returns `-1` | verified |
| 4.3 | `crypto_auth_hmacsha256_verify` | aliasing guard `-(h == correct)`: caller-supplied `h` pointer equal to the internal stack buffer `correct`. Unreachable from outside (the buffer is a fresh local), i.e. the term is always `0` in practice, but it is a *forced-rejection* branch: if it ever held, result is `-1` even for a matching tag | returns `-1` (dead branch; preserved semantics: pointer identity ⇒ reject) | unreachable-from-public-API |
| 4.4 | `crypto_auth_hmacsha512_verify` | at least one bit of `h[0..63]` differs from `HMAC-SHA-512(k, in)`; `crypto_verify_64(h, correct)` yields `-1` (64-byte compare, not 32) | returns `-1` | verified |
| 4.5 | `crypto_auth_hmacsha512_verify` | `sodium_memcmp(correct, h, 64)` yields `-1` (redundant second compare over the full 64 bytes) | returns `-1` | verified |
| 4.6 | `crypto_auth_hmacsha512_verify` | aliasing guard `-(h == correct)` holds | returns `-1` (dead branch) | unreachable-from-public-API |
| 4.7 | `crypto_auth_hmacsha512256_verify` | at least one bit of `h[0..31]` differs from the **first 32 bytes** of `HMAC-SHA-512(k, in)`; `crypto_verify_32(h, correct)` yields `-1`. Notably: passing bytes 32..63 of the untruncated SHA-512 tag also rejects | returns `-1` | verified |
| 4.8 | `crypto_auth_hmacsha512256_verify` | `sodium_memcmp(correct, h, 32)` yields `-1` | returns `-1` | verified |
| 4.9 | `crypto_auth_hmacsha512256_verify` | aliasing guard `-(h == correct)` holds | returns `-1` (dead branch) | unreachable-from-public-API |
| 4.10 | `crypto_auth_verify` (generic wrapper, `crypto_auth.c`) | tag mismatch — unconditionally delegates to `crypto_auth_hmacsha512256_verify`, so all of 4.7/4.8/4.9 propagate verbatim | returns `-1` | verified |
| 4.11 | `crypto_onetimeauth_poly1305_donna_verify` (reached via `crypto_onetimeauth_poly1305_verify` → `implementation->onetimeauth_verify`) | at least one bit of `h[0..15]` differs from `Poly1305(k, in)`; `crypto_verify_16(h, correct)` yields `-1`. **No** `sodium_memcmp` and **no** aliasing guard here — single OR-term only | returns `-1` | verified |
| 4.12 | `crypto_onetimeauth_verify` (generic wrapper, `crypto_onetimeauth.c`) | tag mismatch — unconditionally delegates to `crypto_onetimeauth_poly1305_verify`, which dispatches through the function-pointer table to 4.11 | returns `-1` | verified |
| 4.13 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `0 < keylen <= 64` (i.e. the `keylen > 64` branch is not taken, so the `else if (key == NULL)` arm runs and `keylen > 0`) → `sodium_misuse()` | never returns: misuse handler (if installed via `sodium_set_misuse_handler`) then `abort()`. Not an `int` error code | verified |
| 4.14 | `crypto_auth_hmacsha512_init` | `key == NULL` **and** `0 < keylen <= 128` (block size is 128 here, not 64) → `sodium_misuse()` | never returns: handler then `abort()` | verified |
| 4.15 | `crypto_auth_hmacsha512256_init` | same condition as 4.14 — the function is a pure cast-and-delegate to `crypto_auth_hmacsha512_init`, so `key == NULL && 0 < keylen <= 128` aborts | never returns: handler then `abort()` | verified |
| 4.16 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `keylen > 64`: the `keylen > 64` branch wins, so **no** `sodium_misuse()` fires; control reaches `crypto_hash_sha256_update(&state->ictx, NULL, keylen)` | **not a checked rejection** — undefined behaviour / NULL deref in C. Rust port must not treat this as a defined `-1`; document as unreachable/`debug_assert` | undefined-behaviour-not-tested |
| 4.17 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | `key == NULL` **and** `keylen > 128`: `keylen > 128` branch wins, no misuse, `crypto_hash_sha512_update(..., NULL, keylen)` | **not a checked rejection** — undefined behaviour, same treatment as 4.16 | undefined-behaviour-not-tested |
| 4.18 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `keylen == 0`: `else if (key == NULL)` taken but inner `if (keylen > 0)` is false → **no** misuse. Both XOR loops iterate zero times, so the HMAC is computed with an all-zero key | returns `0` (explicitly *not* an error — a rejection branch that is deliberately *not* taken) | verified |
| 4.19 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | `key == NULL` **and** `keylen == 0` → no misuse, all-zero 128-byte key material | returns `0` (not an error) | verified |
| 4.20 | `crypto_auth_hmacsha256_init` | key-length branch `keylen > 64` (non-error): `key` is replaced by `SHA-256(key)` and `keylen` is forced to `32`. `keylen == 64` exactly does **not** hash. Boundary must be `>` not `>=` | returns `0`; tag equals the tag for the 32-byte hashed key | verified |
| 4.21 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | key-length branch `keylen > 128` (non-error): `key` replaced by `SHA-512(key)`, `keylen` forced to `64`. `keylen == 128` exactly does **not** hash | returns `0`; tag equals the tag for the 64-byte hashed key | verified |
| 4.22 | `crypto_onetimeauth_poly1305_donna_init` | `COMPILER_ASSERT(sizeof(crypto_onetimeauth_poly1305_state) >= sizeof(poly1305_state_internal_t))` — expands to `(void) sizeof(char[(X) ? 1 : -1])`; a static/compile-time assertion, never a runtime rejection. Opaque state is 256 bytes vs. the donna32 internal struct | compile-time failure only; at runtime returns `0` | verified |
| 4.23 | all non-`verify` entry points in this area (`crypto_auth`, `crypto_auth_hmacsha256/512/512256`, all `_init`/`_update`/`_final`, `crypto_onetimeauth*`, `crypto_onetimeauth_poly1305*`, `_crypto_onetimeauth_poly1305_pick_best_implementation`) | no reachable rejection: they perform no length checks, no NULL checks (only `__attribute__((nonnull))` hints in the headers), and unconditionally `return 0`. `inlen` is `unsigned long long` and is never validated | always returns `0`; the only ways to fail are 4.13–4.15 (abort) and the `verify` paths | verified |
| 4.24 | `crypto_auth_keygen`, `crypto_auth_hmacsha256_keygen`, `crypto_auth_hmacsha512_keygen`, `crypto_auth_hmacsha512256_keygen`, `crypto_onetimeauth_keygen`, `crypto_onetimeauth_poly1305_keygen` | `void` return; only failure mode is inside `randombytes_buf` (out of scope for this area — it aborts on RNG failure rather than returning a code) | no error code exists; cannot return `-1` | verified |
