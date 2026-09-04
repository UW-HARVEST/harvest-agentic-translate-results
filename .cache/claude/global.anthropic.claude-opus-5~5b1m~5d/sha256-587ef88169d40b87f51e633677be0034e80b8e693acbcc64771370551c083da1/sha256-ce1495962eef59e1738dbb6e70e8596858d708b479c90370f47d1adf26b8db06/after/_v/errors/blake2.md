| blake2-E1 | crypto_generichash_blake2b | outlen <= 0U | returns -1, output untouched | [x] |
| blake2-E2 | crypto_generichash_blake2b | outlen > BLAKE2B_OUTBYTES (65, 100, 255, 256, 1000, SIZE_MAX) | returns -1, output untouched | [x] |
| blake2-E3 | crypto_generichash_blake2b | keylen > BLAKE2B_KEYBYTES (65, 100, 255, 256, 1000, SIZE_MAX) | returns -1, output untouched | [x] |
| blake2-E4 | crypto_generichash_blake2b | inlen > UINT64_MAX | dead branch: inlen is already uint64_t, so the comparison is always false; verified by inspection (Rust omits it with a comment) | [dead] |
| blake2-E5 | crypto_generichash_blake2b | assert(outlen <= UINT8_MAX) | unreachable: outlen > 64 already returned -1 | [dead] |
| blake2-E6 | crypto_generichash_blake2b | assert(keylen <= UINT8_MAX) | unreachable: keylen > 64 already returned -1 | [dead] |
| blake2-E7 | crypto_generichash | outlen 0 / > 64, keylen > 64 (delegates to crypto_generichash_blake2b) | returns -1 | [x] |
| blake2-E8 | crypto_generichash_blake2b_salt_personal | outlen <= 0U | returns -1 | [x] |
| blake2-E9 | crypto_generichash_blake2b_salt_personal | outlen > BLAKE2B_OUTBYTES | returns -1 | [x] |
| blake2-E10 | crypto_generichash_blake2b_salt_personal | keylen > BLAKE2B_KEYBYTES | returns -1 | [x] |
| blake2-E11 | crypto_generichash_blake2b_init | outlen <= 0U | returns -1, state untouched (still all zero) | [x] |
| blake2-E12 | crypto_generichash_blake2b_init | outlen > BLAKE2B_OUTBYTES (65, 100, 255, 256, SIZE_MAX) | returns -1, state untouched | [x] |
| blake2-E13 | crypto_generichash_blake2b_init | keylen > BLAKE2B_KEYBYTES (65, 100, 255, 256, SIZE_MAX) | returns -1, state untouched | [x] |
| blake2-E14 | crypto_generichash_blake2b_init | `blake2b_init(...) != 0` → return -1 (LCOV_EXCL_LINE) | unreachable: blake2b_init only ever returns 0 or aborts | [dead] |
| blake2-E15 | crypto_generichash_blake2b_init | `blake2b_init_key(...) != 0` → return -1 (LCOV_EXCL_LINE) | unreachable: blake2b_init_key only ever returns 0 or aborts | [dead] |
| blake2-E16 | crypto_generichash_init | outlen 0 / > 64 (delegates) | returns -1 | [x] |
| blake2-E17 | crypto_generichash_blake2b_init_salt_personal | outlen <= 0U | returns -1 | [x] |
| blake2-E18 | crypto_generichash_blake2b_init_salt_personal | outlen > BLAKE2B_OUTBYTES | returns -1 | [x] |
| blake2-E19 | crypto_generichash_blake2b_init_salt_personal | keylen > BLAKE2B_KEYBYTES | returns -1 | [x] |
| blake2-E20 | crypto_generichash_blake2b_init_salt_personal | `blake2b_init_salt_personal / _init_key_salt_personal != 0` (LCOV_EXCL_LINE) | unreachable | [dead] |
| blake2-E21 | crypto_generichash_blake2b_final | assert(outlen <= UINT8_MAX), outlen = 256 | abort — verified by re-executing the test binary as a child process; both C and Rust die with SIGABRT | [x] |
| blake2-E22 | crypto_generichash_blake2b_final | assert(outlen <= UINT8_MAX), outlen = 300 (would silently truncate to 44) | abort — child-process test; **required a Rust fix** (see report) | [x] |
| blake2-E23 | crypto_generichash_blake2b_final | outlen == 0 → blake2b_final → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E24 | crypto_generichash_blake2b_final | outlen == 65 → blake2b_final → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E25 | crypto_generichash_final | outlen == 0 (delegates) | abort — child-process test, both SIGABRT | [x] |
| blake2-E26 | crypto_generichash_blake2b_final / _sodium_blake2b_final | blake2b_is_lastblock(S) != 0 (second call to final) | returns -1, output buffer untouched, state unchanged | [x] |
| blake2-E27 | _sodium_blake2b_final | assert(S->buflen <= BLAKE2B_BLOCKBYTES) after the pre-compress | unreachable: buflen ∈ 129..256 before the subtraction ⇒ 1..128 after | [dead] |
| blake2-E28 | _sodium_blake2b_final | outlen == 0 → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E29 | _sodium_blake2b_final | outlen > BLAKE2B_OUTBYTES (65) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E30 | _sodium_blake2b_init | !outlen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E31 | _sodium_blake2b_init | outlen > BLAKE2B_OUTBYTES (65) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E32 | _sodium_blake2b_init_salt_personal | !outlen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E33 | _sodium_blake2b_init_salt_personal | outlen > BLAKE2B_OUTBYTES → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E34 | _sodium_blake2b_init_key | !outlen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E35 | _sodium_blake2b_init_key | outlen > BLAKE2B_OUTBYTES → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E36 | _sodium_blake2b_init_key | !key → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E37 | _sodium_blake2b_init_key | !keylen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E38 | _sodium_blake2b_init_key | keylen > BLAKE2B_KEYBYTES (65) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E39 | _sodium_blake2b_init_key | `blake2b_init_param(S, P) < 0` → sodium_misuse() (LCOV_EXCL_LINE) | unreachable: blake2b_init_param always returns 0 | [dead] |
| blake2-E40 | _sodium_blake2b_init_key_salt_personal | !outlen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E41 | _sodium_blake2b_init_key_salt_personal | outlen > BLAKE2B_OUTBYTES → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E42 | _sodium_blake2b_init_key_salt_personal | !key → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E43 | _sodium_blake2b_init_key_salt_personal | !keylen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E44 | _sodium_blake2b_init_key_salt_personal | keylen > BLAKE2B_KEYBYTES → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E45 | _sodium_blake2b_init_key_salt_personal | `blake2b_init_param(S, P) < 0` → sodium_misuse() (LCOV_EXCL_LINE) | unreachable | [dead] |
| blake2-E46 | _sodium_blake2b | in == NULL && inlen > 0 → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E47 | _sodium_blake2b | out == NULL → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E48 | _sodium_blake2b | !outlen → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E49 | _sodium_blake2b | outlen > BLAKE2B_OUTBYTES (65) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E50 | _sodium_blake2b | key == NULL && keylen > 0 → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E51 | _sodium_blake2b | keylen > BLAKE2B_KEYBYTES (65) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E52 | _sodium_blake2b | `blake2b_init_key(...) < 0` / `blake2b_init(...) < 0` → sodium_misuse() (LCOV_EXCL_LINE) | unreachable: both return 0 or abort earlier | [dead] |
| blake2-E53 | _sodium_blake2b_salt_personal | in == NULL && inlen > 0 → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E54 | _sodium_blake2b_salt_personal | out == NULL → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E55 | _sodium_blake2b_salt_personal | !outlen (and outlen > 64) → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E56 | _sodium_blake2b_salt_personal | key == NULL && keylen > 0 → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E57 | _sodium_blake2b_salt_personal | keylen > BLAKE2B_KEYBYTES → sodium_misuse() | abort — child-process test, both SIGABRT | [x] |
| blake2-E58 | _sodium_blake2b_salt_personal | `blake2b_init_key_salt_personal / _init_salt_personal < 0` → sodium_misuse() (LCOV_EXCL_LINE) | unreachable | [dead] |
| blake2-E59 | crypto_generichash_blake2b | in == NULL && inlen > 0 (reaches blake2b()'s misuse via the wrapper) | abort — child-process test, both SIGABRT | [x] |
| blake2-E60 | crypto_generichash_blake2b | key == NULL && keylen == 1 (passes the wrapper's keylen <= 64 check, then blake2b() misuses) | abort — child-process test, both SIGABRT | [x] |
| blake2-E61 | crypto_generichash_blake2b_salt_personal | in == NULL && inlen > 0 | abort — child-process test, both SIGABRT | [x] |
| blake2-E62 | crypto_generichash_blake2b_salt_personal | key == NULL && keylen == 1 | abort — child-process test, both SIGABRT | [x] |
| blake2-E63 | crypto_generichash | in == NULL && inlen > 0 (delegates) | abort — child-process test, both SIGABRT | [x] |
| blake2-E64 | _sodium_blake2b_long | outlen > UINT32_MAX (2^32, 2^32+1, 2^32+63, 2^40, SIZE_MAX) | returns -1 without writing to `pout` (the check is the first statement of the function, blake2b-long.c:20, so a small canary buffer suffices) | [x] `tests/gaps.rs::blake2b_long_outlen_over_u32max` |
| blake2-E65 | _sodium_blake2b_long | TRY(crypto_generichash_blake2b_init(..., outlen)) fails for outlen == 0 | returns -1, output untouched | [x] |
| blake2-E66 | _sodium_blake2b_long | remaining TRY(...) < 0 → `goto fail` | unreachable for outlen ∈ 1..=UINT32_MAX: update always returns 0, final/one-shot are called with valid outlen | [dead] |
| blake2-E67 | crypto_kdf_blake2b_derive_from_key | subkey_len < crypto_kdf_blake2b_BYTES_MIN (0, 1, 8, 15) | errno = EINVAL (22), returns -1, output untouched | [x] |
| blake2-E68 | crypto_kdf_blake2b_derive_from_key | subkey_len > crypto_kdf_blake2b_BYTES_MAX (65, 100, 1000, SIZE_MAX) | errno = EINVAL (22), returns -1, output untouched | [x] |
| blake2-E69 | crypto_kdf_derive_from_key | subkey_len out of [16, 64] (delegates) | errno = EINVAL (22), returns -1 | [x] |
| blake2-E70 | crypto_kdf_hkdf_sha256_expand | out_len > crypto_kdf_hkdf_sha256_BYTES_MAX (8161, 8162, 16320, SIZE_MAX) | errno = EINVAL (22), returns -1, output untouched | [x] |
| blake2-E71 | crypto_kdf_hkdf_sha512_expand | out_len > crypto_kdf_hkdf_sha512_BYTES_MAX (16321, 16322, 32640, SIZE_MAX) | errno = EINVAL (22), returns -1, output untouched | [x] |
| blake2-E72 | crypto_kdf_hkdf_sha256_extract_init / sha512 | returns crypto_auth_hmacsha{256,512}_init(...)'s value | always 0 for every salt_len tested (0..200); return value compared on every call | [x] |
| blake2-E73 | crypto_kdf_hkdf_sha256_extract_update / sha512 | returns crypto_auth_hmacsha{256,512}_update(...)'s value | always 0; return value compared on every chunked update | [x] |
| blake2-E74 | crypto_kdf_hkdf_sha256_extract_final / sha512 | unconditional `return 0` after zeroing the state | returns 0 and the whole state buffer is zeroed | [x] |
| blake2-E75 | crypto_shorthash_siphash24 / _siphashx24 / crypto_shorthash | no rejection sites at all (unconditional `return 0`) | returns 0 for every input length tested | [x] |
| blake2-E76 | crypto_verify_16 | x and y differ in any byte | returns -1 (checked for all 16×8 single-bit differences, both argument orders) | [x] |
| blake2-E77 | crypto_verify_32 | x and y differ in any byte | returns -1 (all 32×8 single-bit differences, both orders) | [x] |
| blake2-E78 | crypto_verify_64 | x and y differ in any byte | returns -1 (all 64×8 single-bit differences, both orders) | [x] |
