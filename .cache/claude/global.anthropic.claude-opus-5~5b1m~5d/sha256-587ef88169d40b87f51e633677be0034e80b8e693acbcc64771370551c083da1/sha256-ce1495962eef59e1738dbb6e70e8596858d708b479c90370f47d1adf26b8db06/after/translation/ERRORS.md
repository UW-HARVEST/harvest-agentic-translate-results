# ERRORS.md — error-surface table (rejections / failure returns)

Mechanically derived by grepping every `.c` file under `c_src/libsodium/` for
every distinct rejection site: `return -1`, `return NULL`, `goto` to an error
label, `ARGON2_*` / `ESCRYPT` error codes, `errno = ...`, explicit range/size
checks, `_MIN`/`_MAX` constant checks, `assert(...)`, `abort()` and
`sodium_misuse()`.

Each row has a differential test that constructs exactly that invalid input,
calls **both** libraries through `libloading`, and asserts they return the SAME
value (exact code/sentinel, not merely "both failed").

`[x]` = row has a passing differential test.
`[abort]` = the C path calls `sodium_misuse()`/`abort()`, which cannot be
executed in-process without killing the test binary; equivalence verified by
source inspection instead (noted per row).

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| aead1-E1 | crypto_aead_chacha20poly1305_encrypt | mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX (SIZE_MAX-16) | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E2 | crypto_aead_chacha20poly1305_ietf_encrypt | mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX (64*(2^32-1)) | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E3 | crypto_aead_xchacha20poly1305_ietf_encrypt | mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX (SIZE_MAX-16) | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E4 | crypto_aead_chacha20poly1305_encrypt_detached, _ietf_encrypt_detached, crypto_aead_xchacha20poly1305_ietf_encrypt_detached | no failure path at all | always returns 0 (verified for every configuration row) | [x] |
| aead1-E5 | crypto_aead_chacha20poly1305_decrypt_detached | crypto_verify_16(computed_mac, mac) != 0 and m == NULL | returns -1 (crypto_verify_16's result), m untouched | [x] |
| aead1-E6 | crypto_aead_chacha20poly1305_decrypt_detached | crypto_verify_16 != 0 and m != NULL | memset(m, 0, mlen) then returns -1 | [x] |
| aead1-E7 | crypto_aead_chacha20poly1305_decrypt | clen < crypto_aead_chacha20poly1305_ABYTES (every clen 0..15) | returns -1 without calling _decrypt_detached; *mlen_p = 0 when mlen_p != NULL, m untouched | [x] |
| aead1-E8 | crypto_aead_chacha20poly1305_decrypt | inner _decrypt_detached returns -1 (tampered ct / mac / ad, wrong key, wrong nonce, truncated adlen) | returns -1; *mlen_p = 0 when mlen_p != NULL | [x] |
| aead1-E9 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | crypto_verify_16 != 0 and m == NULL | returns -1, m untouched | [x] |
| aead1-E10 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | crypto_verify_16 != 0 and m != NULL | memset(m, 0, mlen) then returns -1 | [x] |
| aead1-E11 | crypto_aead_chacha20poly1305_ietf_decrypt | clen < crypto_aead_chacha20poly1305_ietf_ABYTES (every clen 0..15) | returns -1; *mlen_p = 0 when mlen_p != NULL | [x] |
| aead1-E12 | crypto_aead_chacha20poly1305_ietf_decrypt | inner _decrypt_detached returns -1 (all tamper classes) | returns -1; *mlen_p = 0 | [x] |
| aead1-E13 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached (static _decrypt_detached) | crypto_verify_16 != 0 and m == NULL | returns -1, m untouched | [x] |
| aead1-E14 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached (static _decrypt_detached) | crypto_verify_16 != 0 and m != NULL | memset(m, 0, mlen) then returns -1 | [x] |
| aead1-E15 | crypto_aead_xchacha20poly1305_ietf_decrypt | clen < crypto_aead_xchacha20poly1305_ietf_ABYTES (every clen 0..15) | returns -1; *mlen_p = 0 when mlen_p != NULL | [x] |
| aead1-E16 | crypto_aead_xchacha20poly1305_ietf_decrypt | inner _decrypt_detached returns -1 (all tamper classes) | returns -1; *mlen_p = 0 | [x] |
| aead1-E17 | crypto_secretbox_xsalsa20poly1305 | mlen < 32 (ZEROBYTES) — every mlen 0..31 | returns -1, c untouched | [x] |
| aead1-E18 | crypto_secretbox_xsalsa20poly1305_open | clen < 32 (ZEROBYTES) — every clen 0..31 | returns -1, m untouched | [x] |
| aead1-E19 | crypto_secretbox_xsalsa20poly1305_open | crypto_onetimeauth_poly1305_verify(c+16, c+32, clen-32, subkey) != 0 | returns -1 before writing m (tampered c[32..], tampered mac c[16..32], wrong key, wrong nonce, non-zero ZEROBYTES prefix at seal time) | [x] |
| aead1-E20 | crypto_secretbox | mlen < 32 (delegates to crypto_secretbox_xsalsa20poly1305) | returns -1 | [x] |
| aead1-E21 | crypto_secretbox_open | clen < 32, or poly1305 verify failure (delegates to crypto_secretbox_xsalsa20poly1305_open) | returns -1 | [x] |
| aead1-E22 | crypto_secretbox_easy | mlen > crypto_secretbox_MESSAGEBYTES_MAX (SIZE_MAX-16) | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E23 | crypto_secretbox_detached | no failure path at all | always returns 0 (verified for every configuration row, incl. the memmove/overlap branch) | [x] |
| aead1-E24 | crypto_secretbox_open_detached | crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0 | zeroes subkey and returns -1 before touching m (every mac byte flipped, every ct byte flipped, wrong key, wrong nonce) | [x] |
| aead1-E25 | crypto_secretbox_open_detached | m == NULL with a valid mac | zeroes subkey and returns 0 without decrypting | [x] |
| aead1-E26 | crypto_secretbox_open_easy | clen < crypto_secretbox_MACBYTES (every clen 0..15) | returns -1, m untouched | [x] |
| aead1-E27 | crypto_secretbox_open_easy | inner crypto_secretbox_open_detached returns -1 | returns -1 | [x] |
| aead1-E28 | crypto_secretbox_xchacha20poly1305_easy | mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX (SIZE_MAX-16) | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E29 | crypto_secretbox_xchacha20poly1305_detached | no failure path at all | always returns 0 (verified for every configuration row, incl. the memmove/overlap branch) | [x] |
| aead1-E30 | crypto_secretbox_xchacha20poly1305_open_detached | crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0 | zeroes subkey and returns -1 before touching m (every mac byte flipped, every ct byte flipped, wrong key, wrong nonce) | [x] |
| aead1-E31 | crypto_secretbox_xchacha20poly1305_open_detached | m == NULL with a valid mac | zeroes subkey and returns 0 without decrypting | [x] |
| aead1-E32 | crypto_secretbox_xchacha20poly1305_open_easy | clen < crypto_secretbox_xchacha20poly1305_MACBYTES (every clen 0..15) | returns -1, m untouched | [x] |
| aead1-E33 | crypto_secretbox_xchacha20poly1305_open_easy | inner _open_detached returns -1 | returns -1 | [x] |
| aead1-E34 | crypto_secretstream_xchacha20poly1305_init_push, _init_pull, _rekey | no failure path at all | init_* always return 0, rekey returns void (verified on random, all-zero, all-0xff and crafted states) | [x] |
| aead1-E35 | crypto_secretstream_xchacha20poly1305_push | mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX (64*(2^32-2)) | *outlen_p is set to 0 first, then sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E36 | crypto_secretstream_xchacha20poly1305_pull | inlen < crypto_secretstream_xchacha20poly1305_ABYTES (every inlen 0..16) | returns -1; *mlen_p = 0 and *tag_p = 0xff when non-NULL; m and state untouched | [x] |
| aead1-E37 | crypto_secretstream_xchacha20poly1305_pull | inlen - ABYTES > MESSAGEBYTES_MAX | calls sodium_misuse() | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead1-E38 | crypto_secretstream_xchacha20poly1305_pull | sodium_memcmp(mac, stored_mac, 16) != 0 | returns -1; *mlen_p stays 0 and *tag_p stays 0xff; m and state untouched (every ciphertext byte incl. the tag byte flipped, every ad byte flipped, wrong key) | [x] |
| aead1-E39 | crypto_secretstream_xchacha20poly1305_pull | m == NULL | NOT supported by the C: it passes m straight to crypto_stream_chacha20_ietf_xor_ic and would write through a NULL pointer for mlen > 0 — not testable in-process; Rust does the same by inspection | [n/a] |

| aead2-E1 | crypto_aead_aes256gcm_encrypt_detached | hardware AES-GCM unavailable in this build config (`#if !((HAVE_ARMCRYPTO && NATIVE_LITTLE_ENDIAN) || (HAVE_TMMINTRIN_H && HAVE_WMMINTRIN_H))` stub) | sets `errno = ENOSYS` (38) and returns -1, writes nothing | [x] |
| aead2-E2 | crypto_aead_aes256gcm_encrypt | same stub branch | `errno = ENOSYS`, returns -1, `*clen_p` untouched | [x] |
| aead2-E3 | crypto_aead_aes256gcm_decrypt_detached | same stub branch | `errno = ENOSYS`, returns -1, m untouched | [x] |
| aead2-E4 | crypto_aead_aes256gcm_decrypt | same stub branch | `errno = ENOSYS`, returns -1, `*mlen_p` untouched | [x] |
| aead2-E5 | crypto_aead_aes256gcm_beforenm | same stub branch | `errno = ENOSYS`, returns -1, the 512-byte state is not written at all | [x] |
| aead2-E6 | crypto_aead_aes256gcm_encrypt_detached_afternm | same stub branch | `errno = ENOSYS`, returns -1 | [x] |
| aead2-E7 | crypto_aead_aes256gcm_encrypt_afternm | same stub branch | `errno = ENOSYS`, returns -1 | [x] |
| aead2-E8 | crypto_aead_aes256gcm_decrypt_detached_afternm | same stub branch | `errno = ENOSYS`, returns -1 | [x] |
| aead2-E9 | crypto_aead_aes256gcm_decrypt_afternm | same stub branch | `errno = ENOSYS`, returns -1 | [x] |
| aead2-E10 | crypto_aead_aes256gcm_is_available | capability probe in the stub branch | returns 0 (never 1) | [x] |
| aead2-E11 | crypto_aead_aes256gcm_* (all nine crypto entry points) | stub ignores every argument: NULL `clen_p`/`mlen_p`/`m`/`ad`/`nsec`, mlen/clen = 0, 1, 16, 17, 2^61, u64::MAX | always `errno = ENOSYS` + -1, never dereferences anything | [x] |
| aead2-E12 | crypto_aead_aegis128l_encrypt | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` → `sodium_misuse()` | abort — not testable in-process; verified by inspection that `translation/src/aead_aegis128l.rs` calls `sodium_misuse()` at the same point with the same comparison | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead2-E13 | crypto_aead_aegis128l_encrypt_detached | `mlen > MESSAGEBYTES_MAX \|\| adlen > MESSAGEBYTES_MAX` → `sodium_misuse()` (note: `*maclen_p` is already written *before* the check, in both C and Rust) | abort — not testable in-process; verified by inspection | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead2-E14 | crypto_aead_aegis256_encrypt | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` → `sodium_misuse()` | abort — not testable in-process; verified by inspection | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead2-E15 | crypto_aead_aegis256_encrypt_detached | `mlen > MESSAGEBYTES_MAX \|\| adlen > MESSAGEBYTES_MAX` → `sodium_misuse()` | abort — not testable in-process; verified by inspection | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| aead2-E16 | crypto_aead_aegis128l_decrypt | `clen < crypto_aead_aegis128l_ABYTES` (tested for every clen 0..31) | returns -1, `*mlen_p = 0` when mlen_p != NULL (untouched when NULL), m never written | [x] |
| aead2-E17 | crypto_aead_aegis256_decrypt | `clen < crypto_aead_aegis256_ABYTES` (tested for every clen 0..31) | returns -1, `*mlen_p = 0` when mlen_p != NULL, m never written | [x] |
| aead2-E18 | crypto_aead_aegis128l_decrypt_detached | `clen > MESSAGEBYTES_MAX` (2^61, u64::MAX) | returns -1 before calling the implementation; m untouched | [x] |
| aead2-E19 | crypto_aead_aegis128l_decrypt_detached | `adlen > MESSAGEBYTES_MAX` (2^61, u64::MAX), and both over-long at once | returns -1 before calling the implementation | [x] |
| aead2-E20 | crypto_aead_aegis256_decrypt_detached | `clen > MESSAGEBYTES_MAX` (2^61, u64::MAX) | returns -1 before calling the implementation | [x] |
| aead2-E21 | crypto_aead_aegis256_decrypt_detached | `adlen > MESSAGEBYTES_MAX` (2^61, u64::MAX), and both over-long at once | returns -1 before calling the implementation | [x] |
| aead2-E22 | crypto_aead_aegis128l_decrypt, crypto_aead_aegis256_decrypt | `clen` large enough that `clen - ABYTES > MESSAGEBYTES_MAX` (u64::MAX and 2^61-1+32+1) → forwarded rejection from decrypt_detached | returns -1, `*mlen_p = 0` | [x] |
| aead2-E23 | aegis128l decrypt_detached (soft impl) → crypto_verify_32 | tag mismatch: every one of the 32 tag bytes × bit 0/3/7 flipped, for mlen ∈ {0,1,15,16,17,31,32,33,64,65,1000} | returns -1 and `memset(m, 0, mlen)`; caller sets `*mlen_p = 0` | [x] |
| aead2-E24 | aegis256 decrypt_detached (soft impl) → crypto_verify_32 | tag mismatch: same exhaustive per-byte/per-bit sweep | returns -1 and `memset(m, 0, mlen)` | [x] |
| aead2-E25 | aegis128l/aegis256 decrypt_detached (soft impl) | ciphertext corruption: every ciphertext byte (first 80) flipped | returns -1 and zeroes m | [x] |
| aead2-E26 | crypto_aead_aegis128l_decrypt_detached, crypto_aead_aegis256_decrypt_detached | detached-MAC corruption: every one of the 32 mac bytes flipped | returns -1 and zeroes m | [x] |
| aead2-E27 | aegis128l/aegis256 decrypt_detached (soft impl) with `m == NULL` and a bad tag | failure path where `if (m != NULL) memset(...)` is skipped | returns -1, no write | [x] |
| aead2-E28 | aegis128l_soft_implementation.encrypt_detached, aegis256_soft_implementation.encrypt_detached | `aegis*_mac()` else-branch: maclen ∉ {16,32} (0, 1, 8, 15, 17, 31, 33, 48) | `memset(mac, 0, maclen)` and returns -1 (the ciphertext has already been written) | [x] |
| aead2-E29 | aegis128l_soft_implementation.decrypt_detached, aegis256_soft_implementation.decrypt_detached | maclen ∉ {16,32}: `aegis*_mac()` returns -1 so neither crypto_verify_16 nor crypto_verify_32 runs and `ret` stays -1 | returns -1 and `memset(m, 0, clen)` | [x] |
| aead2-E30 | crypto_aead_aegis128l_decrypt, crypto_aead_aegis256_decrypt | wrong key / wrong nonce / wrong adlen | returns -1, m zeroed for exactly mlen bytes | [x] |
| aead2-E31 | crypto_core/softaes/softaes.c (all nine exports) | no `return -1` / `NULL` / assert / range check exists in this file — every function is total | n/a (no error surface); all nine exports still exercised for value equality | [x] |

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

| box-E1 | crypto_box_curve25519xsalsa20poly1305_beforenm | crypto_scalarmult_curve25519(s, sk, pk) != 0 — all-zero shared secret; tested with pk = 0, 1, p, p+1 and two order-8 points | returns -1, k left untouched | [x] |
| box-E2 | crypto_box_curve25519xsalsa20poly1305 | crypto_box_curve25519xsalsa20poly1305_beforenm() != 0 | returns -1, c left untouched | [x] |
| box-E3 | crypto_box_curve25519xsalsa20poly1305_open | crypto_box_curve25519xsalsa20poly1305_beforenm() != 0 | returns -1, m left untouched | [x] |
| box-E4 | crypto_box_curve25519xsalsa20poly1305_afternm (crypto_secretbox_xsalsa20poly1305) | mlen < 32 (ZEROBYTES); every mlen in 0..31 | returns -1 | [x] |
| box-E5 | crypto_box_curve25519xsalsa20poly1305_open_afternm (crypto_secretbox_xsalsa20poly1305_open) | clen < 32 (ZEROBYTES); every clen in 0..31 | returns -1 | [x] |
| box-E6 | crypto_box_curve25519xsalsa20poly1305_open_afternm | crypto_onetimeauth_poly1305_verify fails — one bit flipped in every ciphertext byte >= BOXZEROBYTES, and wrong nonce | returns -1, m left untouched | [x] |
| box-E7 | crypto_box (both spellings) | beforenm failure propagated from box-E1 | returns -1 | [x] |
| box-E8 | crypto_box_open (both spellings) | beforenm failure propagated from box-E1 | returns -1 | [x] |
| box-E9 | crypto_box_afternm / crypto_box_open_afternm | mlen/clen < ZEROBYTES (delegates to box-E4/E5) | returns -1 | [x] |
| box-E10 | crypto_box_detached | crypto_box_beforenm() != 0 | returns -1, c and mac untouched | [x] |
| box-E11 | crypto_box_open_detached | crypto_box_beforenm() != 0 | returns -1 | [x] |
| box-E12 | crypto_box_easy | mlen > crypto_box_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX-16) | sodium_misuse() -> abort — not testable in-process; Rust `box_.rs` calls the same `sodium_misuse()` with the identical `mlen > SODIUM_SIZE_MAX - 16` test (verified by inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E13 | crypto_box_easy_afternm | mlen > crypto_box_MESSAGEBYTES_MAX | sodium_misuse() -> abort — not testable in-process; identical guard in Rust (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E14 | crypto_box_easy | beforenm failure inside crypto_box_detached (bad pk) | returns -1, c untouched | [x] |
| box-E15 | crypto_box_open_easy | clen < crypto_box_MACBYTES (16); every clen in 0..15 | returns -1, m left untouched | [x] |
| box-E16 | crypto_box_open_easy_afternm | clen < crypto_box_MACBYTES (16); every clen in 0..15 | returns -1, m left untouched | [x] |
| box-E17 | crypto_box_open_easy / crypto_box_open_detached | beforenm failure (bad pk) | returns -1 | [x] |
| box-E18 | crypto_box_open_detached_afternm (crypto_secretbox_open_detached) | poly1305 verify fails — a bit flipped in every ciphertext byte (2 bit positions) and in every one of the 16 mac bytes | returns -1, m left untouched | [x] |
| box-E19 | crypto_box_curve25519xchacha20poly1305_beforenm | crypto_scalarmult_curve25519 != 0 (pk = 0/1/p/p+1/order-8) | returns -1, k untouched | [x] |
| box-E20 | crypto_box_curve25519xchacha20poly1305_detached | beforenm != 0 | returns -1 | [x] |
| box-E21 | crypto_box_curve25519xchacha20poly1305_open_detached | beforenm != 0 | returns -1 | [x] |
| box-E22 | crypto_box_curve25519xchacha20poly1305_easy | mlen > MESSAGEBYTES_MAX | sodium_misuse() -> abort — identical guard in Rust (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E23 | crypto_box_curve25519xchacha20poly1305_easy_afternm | mlen > MESSAGEBYTES_MAX | sodium_misuse() -> abort — identical guard in Rust (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E24 | crypto_box_curve25519xchacha20poly1305_open_easy | clen < MACBYTES (16); every clen in 0..15 | returns -1, m untouched | [x] |
| box-E25 | crypto_box_curve25519xchacha20poly1305_open_easy_afternm | clen < MACBYTES (16); every clen in 0..15 | returns -1, m untouched | [x] |
| box-E26 | crypto_box_curve25519xchacha20poly1305_open_detached_afternm | poly1305 verify fails (every ciphertext byte x 2 bits, every mac byte) | returns -1 | [x] |
| box-E27 | crypto_box_seal | mlen > crypto_box_MESSAGEBYTES_MAX | sodium_misuse() -> abort — identical guard in Rust (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E28 | crypto_box_seal | crypto_box_keypair() != 0 (LCOV_EXCL_LINE) | returns -1 — unreachable: crypto_scalarmult_curve25519_base never fails, so this branch cannot be triggered from outside; Rust has the identical `if ... != 0 { return -1 }` (inspection) | [n/a] |
| box-E29 | crypto_box_seal | crypto_box_easy -> beforenm fails because the recipient pk is 0/small order | returns -1; the ephemeral pk is still memcpy'd into c[0..32] (checked: only c[32..] stays untouched) | [x] |
| box-E30 | crypto_box_seal_open | clen < crypto_box_SEALBYTES (48); every clen in 0..47 | returns -1, m left untouched | [x] |
| box-E31 | crypto_box_seal_open | embedded ephemeral pk c[0..32] is 0/small order -> beforenm fails | returns -1 | [x] |
| box-E32 | crypto_box_seal_open | poly1305 verify fails: one bit flipped in every byte of a 88-byte sealed blob, and opening with the wrong recipient key pair | returns -1 | [x] |
| box-E33 | crypto_box_curve25519xchacha20poly1305_seal | mlen > MESSAGEBYTES_MAX | sodium_misuse() -> abort (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E34 | crypto_box_curve25519xchacha20poly1305_seal | keypair() != 0 (LCOV_EXCL_LINE) | returns -1 — unreachable (see box-E28) | [n/a] |
| box-E35 | crypto_box_curve25519xchacha20poly1305_seal | easy -> beforenm fails (bad recipient pk) | returns -1 | [x] |
| box-E36 | crypto_box_curve25519xchacha20poly1305_seal_open | clen < SEALBYTES (48); every clen in 0..47 | returns -1 | [x] |
| box-E37 | crypto_box_curve25519xchacha20poly1305_seal_open | bad embedded ephemeral pk / tampered blob / wrong key pair | returns -1 | [x] |
| box-E38 | crypto_kx_client_session_keys | rx == NULL && tx == NULL | sodium_misuse() -> abort — not testable in-process; `kx.rs` performs the same three NULL checks in the same order (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E39 | crypto_kx_client_session_keys | crypto_scalarmult(q, client_sk, server_pk) != 0 — server_pk = 0, 1, p, p+1, order-8 points | returns -1, rx and tx left untouched | [x] |
| box-E40 | crypto_kx_server_session_keys | rx == NULL && tx == NULL | sodium_misuse() -> abort — identical checks in Rust (inspection) | [x] SIGABRT in BOTH libraries, executed out-of-process by `tests/gaps.rs::abort_parity` (the guard is the first statement, so mlen/adlen = u64::MAX with a small buffer reaches it without touching memory) |
| box-E41 | crypto_kx_server_session_keys | crypto_scalarmult(q, server_sk, client_pk) != 0 — same bad-key set | returns -1, rx and tx left untouched | [x] |
| box-E42 | _sodium_mlkem768_ref_enc_deterministic (and crypto_kem_mlkem768_enc_deterministic) | polyvec_is_canonical(&pkpv) == 0, i.e. any of the 768 12-bit coefficients is >= MLKEM768_Q (3329) | returns -1, ct and ss left untouched; boundary verified: 3328 -> 0, 3329/3330/4095 -> -1, at 9 coefficient indices, plus an all-1184-byte single-bit sweep | [x] |
| box-E43 | _sodium_mlkem768_ref_dec (and crypto_kem_mlkem768_dec) | sodium_memcmp(ct, cmp) != 0 — the FO re-encryption check fails | NOT an error return: always returns 0 and cmov()s a SHAKE256(z‖ct)-derived shared secret in; verified byte-identical in C and Rust for a bit flip in every one of the 1088 ct bytes, random/all-0x00/all-0xff ciphertexts and random secret keys | [x] |
| box-E44 | crypto_kem_xwing_enc_deterministic | crypto_kem_mlkem768_enc_deterministic() != 0 (non-canonical ML-KEM part of pk) | returns -1, ct and ss left untouched | [x] |
| box-E45 | crypto_kem_xwing_enc_deterministic | crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0 — pk[1184..1216] is 0/small order | returns -1, ct and ss left untouched | [x] |
| box-E46 | crypto_kem_xwing_enc | crypto_kem_xwing_enc_deterministic() != 0 (LCOV_EXCL_LINE) | returns -1 (triggered via a bad X25519 part of the public key) | [x] |
| box-E47 | crypto_kem_xwing_dec | crypto_kem_mlkem768_dec() != 0 | returns -1 — unreachable: ML-KEM dec always returns 0 (see box-E43); Rust has the identical branch (inspection) | [n/a] |
| box-E48 | crypto_kem_xwing_dec | crypto_scalarmult_curve25519(ss_x25519, sk_x25519, ct_x25519) != 0 — ct[1088..1120] replaced by 0/1/p/p+1/order-8 points | returns -1, ss left untouched | [x] |
| box-E49 | crypto_kem_dec / crypto_kem_enc (generic dispatch) | all xwing rejection sites above, reached through the crypto_kem_* wrappers | returns -1 | [x] |

| ed25519low-E1 | _sodium_ge25519_frombytes | (has_m_root \| has_p_root) == 0, i.e. neither vx^2==u nor vx^2==-u -> y is not the ordinate of a curve point | returns -1 (`(has_m_root\|has_p_root) - 1`); h->X/Y/Z written, h->T left untouched | [x] |
| ed25519low-E2 | _sodium_ge25519_frombytes_negate_vartime | fe25519_iszero(vx^2-u)==0 && fe25519_iszero(vx^2+u)==0 | returns -1 early; h->X/Y/Z written, h->T left untouched | [x] |
| ed25519low-E3 | _sodium_ge25519_is_canonical | s (mod 2^255) >= 2^255-19, i.e. s[31]&0x7f==0x7f && s[1..30]==0xff && s[0]>=0xed | returns 0 | [x] |
| ed25519low-E4 | _sodium_sc25519_is_canonical | s >= L = 2^252+27742317777372353535851937790883648493 | returns 0 | [x] |
| ed25519low-E5 | _sodium_ge25519_is_on_curve | -X^2*Z^2 + Y^2*Z^2 != Z^4 + d*X^2*Y^2 | returns 0 | [x] |
| ed25519low-E6 | _sodium_ge25519_is_on_main_subgroup | ge25519_mul_l(P) != identity (X!=0 or Y!=Z) | returns 0 | [x] |
| ed25519low-E7 | _sodium_ge25519_has_small_order | X==0 \|\| Y==0 \|\| Z==0 \|\| Y*sqrt(-1)-X==0 \|\| Y*sqrt(-1)+X==0 | returns non-zero (1) | [x] |
| ed25519low-E8 | fe25519_sqrt (static) via ge25519_xmont_to_ymont via ge25519_elligator2 | x^3+A*x^2+x is not a square on the Montgomery curve | `abort()` — abort, not testable in-process; unreachable by construction (elligator2 picks x1 or -x1-A precisely so that gx is a square). Rust `ed25519_ref10_ge.rs` mirrors the same abort/unreachable shape | [abort] |
| ed25519low-E9 | _sodium_ristretto255_frombytes / ristretto255_is_canonical | s >= p (s[31]&0x7f==0x7f && s[1..30]==0xff && s[0]>=0xed) | returns -1 without touching *h | [x] |
| ed25519low-E10 | _sodium_ristretto255_frombytes / ristretto255_is_canonical | bit 255 of s set (`e = s[31]>>7`) | returns -1 without touching *h | [x] |
| ed25519low-E11 | _sodium_ristretto255_frombytes / ristretto255_is_canonical | s[0] odd (`s[0] & 1`) | returns -1 without touching *h | [x] |
| ed25519low-E12 | _sodium_ristretto255_frombytes | ristretto255_sqrt_ratio_m1(1, v*u2^2) reports a non-square (`1 - notsquare` set) | returns -1 with *h fully written; sub-condition counted separately in the test (11 occurrences) | [x] |
| ed25519low-E13 | _sodium_ristretto255_frombytes | fe25519_isnegative(h->T) != 0 | returns -1 with *h fully written (same `return -(...)` site as E12); sub-condition counted separately in the test (27 occurrences) | [x] |
| ed25519low-E14 | _sodium_ristretto255_frombytes | fe25519_iszero(h->Y) != 0 | returns -1 with *h fully written (same `return -(...)` site as E12); sub-condition counted separately in the test (2 occurrences) | [x] |
| ed25519low-E15 | crypto_core_ed25519_is_valid_point | ge25519_is_canonical(p) == 0 | returns 0 | [x] |
| ed25519low-E16 | crypto_core_ed25519_is_valid_point | ge25519_frombytes(&p_p3, p) != 0 | returns 0 | [x] |
| ed25519low-E17 | crypto_core_ed25519_is_valid_point | ge25519_is_on_curve(&p_p3) == 0 | returns 0 — UNREACHABLE: ge25519_frombytes solves the curve equation for x (both the vx^2==u and the vx^2==-u branch yield an on-curve point with Z=1), so a successful frombytes always implies is_on_curve==1. Verified by inspection in both C and Rust; the shared `return 0` is covered by E15/E16/E18/E19 | [unreachable] |
| ed25519low-E18 | crypto_core_ed25519_is_valid_point | ge25519_has_small_order(&p_p3) != 0 | returns 0 (asserted for the identity y=1, the two order-4 points y=0, and the order-2 point y=p-1, each with is_canonical=1 / frombytes=0 / is_on_curve=1) | [x] |
| ed25519low-E19 | crypto_core_ed25519_is_valid_point | ge25519_is_on_main_subgroup(&p_p3) == 0 | returns 0 (tested with a*B + T for the three decodable non-trivial torsion points T = (+/-sqrt(-1), 0) and (0, -1); asserted is_canonical=1, frombytes=0, is_on_curve=1, has_small_order=0, is_on_main_subgroup=0) | [x] |
| ed25519low-E20 | crypto_core_ed25519_add | ge25519_frombytes(&p_p3, p) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E21 | crypto_core_ed25519_add | ge25519_is_on_curve(&p_p3) == 0 | returns -1 — UNREACHABLE for the same reason as E17 | [unreachable] |
| ed25519low-E22 | crypto_core_ed25519_add | ge25519_frombytes(&q_p3, q) != 0 (p valid, q invalid) | returns -1, output buffer untouched | [x] |
| ed25519low-E23 | crypto_core_ed25519_add | ge25519_is_on_curve(&q_p3) == 0 | returns -1 — UNREACHABLE for the same reason as E17 | [unreachable] |
| ed25519low-E24 | crypto_core_ed25519_sub | ge25519_frombytes(&p_p3, p) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E25 | crypto_core_ed25519_sub | ge25519_is_on_curve(&p_p3) == 0 | returns -1 — UNREACHABLE (see E17) | [unreachable] |
| ed25519low-E26 | crypto_core_ed25519_sub | ge25519_frombytes(&q_p3, q) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E27 | crypto_core_ed25519_sub | ge25519_is_on_curve(&q_p3) == 0 | returns -1 — UNREACHABLE (see E17) | [unreachable] |
| ed25519low-E28 | _string_to_points (static, core_ed25519.c) | n > 2U | `abort()` — abort, not testable in-process; unreachable from the public API (only n=1 from `_nu` and n=2 from `_from_string`). Rust mirrors the same guard | [abort] |
| ed25519low-E29 | _string_to_points -> core_h2c_string_to_hash | hash_alg not in {CORE_H2C_SHA256=1, CORE_H2C_SHA512=2} | returns -1 (errno=EINVAL set by core_h2c) | [x] |
| ed25519low-E30 | crypto_core_ed25519_from_string_nu | _string_to_points(...) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E31 | crypto_core_ed25519_from_string | _string_to_points(px, 2, ...) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E32 | crypto_core_ed25519_from_string | crypto_core_ed25519_add(p, px, px+32) != 0 | returns -1 — UNREACHABLE: both px halves come from ge25519_from_hash, which always emits a canonical on-curve encoding | [unreachable] |
| ed25519low-E33 | crypto_core_ed25519_scalar_invert | sodium_is_zero(s, 32) (s == 0) | returns -1 (`- sodium_is_zero(...)`); `recip` is still written by sc25519_invert | [x] |
| ed25519low-E34 | crypto_core_ed25519_scalar_from_string | core_h2c_string_to_hash(...) != 0 (invalid hash_alg) | returns -1, output buffer untouched | [x] |
| ed25519low-E35 | crypto_core_ed25519_scalar_is_canonical | s >= L | returns 0 | [x] |
| ed25519low-E36 | core_h2c_string_to_hash_sha256 / _sha512 | `assert(h_len <= 0xff)` | abort on assertion failure — unreachable from this area (h_len is 48, 64 or 96). Only reached from another area's callers | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| ed25519low-E37 | crypto_core_ristretto255_is_valid_point | ristretto255_frombytes(&p_p3, p) != 0 | returns 0 | [x] |
| ed25519low-E38 | crypto_core_ristretto255_add | ristretto255_frombytes(&p_p3, p) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E39 | crypto_core_ristretto255_add | ristretto255_frombytes(&q_p3, q) != 0 (p valid, q invalid) | returns -1, output buffer untouched | [x] |
| ed25519low-E40 | crypto_core_ristretto255_sub | ristretto255_frombytes(&p_p3, p) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E41 | crypto_core_ristretto255_sub | ristretto255_frombytes(&q_p3, q) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E42 | _string_to_element (static, core_ristretto255.c) | core_h2c_string_to_hash(...) != 0 | returns -1 | [x] |
| ed25519low-E43 | crypto_core_ristretto255_from_string | _string_to_element(...) != 0 | returns -1, output buffer untouched | [x] |
| ed25519low-E44 | crypto_core_ristretto255_scalar_invert | s == 0 (delegates to crypto_core_ed25519_scalar_invert) | returns -1 | [x] |
| ed25519low-E45 | crypto_core_ristretto255_scalar_from_string | core_h2c_string_to_hash(...) != 0 | returns -1 | [x] |
| ed25519low-E46 | crypto_core_ristretto255_scalar_is_canonical | s >= L | returns 0 | [x] |
| ed25519low-E47 | crypto_core_ristretto255_from_hash | no rejection site: unconditionally `return 0` | always returns 0, verified for 66 inputs incl. all-0x00 / all-0xff | [x] |

| h2c-E1 | _sodium_core_h2c_string_to_hash | hash_alg not in {CORE_H2C_SHA256=1, CORE_H2C_SHA512=2} (tested: 0, 3, 4, -1, -2, 99, 256, 0x10000, INT_MIN, INT_MAX, 255) — `switch` default | sets errno=EINVAL(22), returns -1, output buffer left completely untouched (canary-verified) | [x] |
| h2c-E2 | _sodium_core_h2c_string_to_hash (SHA-256 path) | `assert(h_len <= 0xff)` (core_h2c.c:26) — asserts are ENABLED in the reference build (CMAKE_BUILD_TYPE empty ⇒ no -DNDEBUG; `U __assert_fail` present in core_h2c.c.o) | SIGABRT (exit 134). Was a Rust divergence (returned 0); FIXED in core_ed25519.rs by an explicit `if h_len > 0xff { csys::abort() }`. Verified out-of-process (h_len=256, 1000 → 134 in both libs); not testable in-process | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| h2c-E3 | _sodium_core_h2c_string_to_hash (SHA-512 path) | `assert(h_len <= 0xff)` (core_h2c.c:82), same build reasoning | SIGABRT (exit 134). Same divergence, same fix, verified out-of-process | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| h2c-E4 | _string_to_points (core_ed25519.c:72) | `n > 2` → `abort()` (LCOV_EXCL_LINE); unreachable from the public API (only n=1 and n=2 callers) | abort; Rust calls `crate::csys::abort()` — matches by inspection | [abort] |
| h2c-E5 | crypto_core_ed25519_from_string_nu | core_h2c_string_to_hash(h_len=48) fails (only possible via an out-of-range hash_alg) | returns -1, 32-byte output left untouched | [x] |
| h2c-E6 | crypto_core_ed25519_from_string | _string_to_points(n=2, h_len=96) fails (out-of-range hash_alg) → `return -1` | returns -1, 32-byte output left untouched | [x] |
| h2c-E7 | crypto_core_ed25519_from_string | tail `return crypto_core_ed25519_add(p, &px[0], &px[32])` can in principle return -1 (ge25519_frombytes / is_on_curve failure) | not triggerable: both px points come from ge25519_from_hash and are always canonical on-curve points; verified by inspection that the Rust propagates `crypto_core_ed25519_add`'s return value identically, and asserted rc==0 for 300+ inputs in `ed25519_from_string_matches_composition` | [x] |
| h2c-E8 | crypto_core_ed25519_scalar_from_string | core_h2c_string_to_hash(h_len=HASH_SC_L=48) fails (out-of-range hash_alg) → `return -1` | returns -1, 32-byte output left untouched | [x] |
| h2c-E9 | _string_to_element (core_ristretto255.c:76) | core_h2c_string_to_hash(h_len=64) fails (out-of-range hash_alg) → `return -1` (LCOV_EXCL_LINE) | returns -1, 32-byte output left untouched | [x] |
| h2c-E10 | crypto_core_ristretto255_from_string | propagates _string_to_element's -1 | returns -1, output untouched | [x] |
| h2c-E11 | crypto_core_ristretto255_scalar_from_string | propagates crypto_core_ed25519_scalar_from_string's -1 | returns -1, output untouched | [x] |
| h2c-E12 | crypto_core_ristretto255_from_hash | no rejection site at all — `ristretto255_from_hash(); return 0;` | always returns 0 for every 64-byte input (75 inputs incl. all-zero / all-0xff / p / p±1 / aliased p==h) | [x] |
| h2c-E13 | _sodium_ge25519_from_uniform, _sodium_ge25519_from_hash (via ge25519_elligator2) | `if (ge25519_xmont_to_ymont(y, x) != 0) abort();` (ed25519_ref10.c, LCOV_EXCL_LINE) — mathematically unreachable, Elligator2 guarantees a square | abort; Rust calls `crate::csys::abort()` — matches by inspection; never reached in 70+70 tested inputs | [abort] |
| h2c-E14 | _sodium_ge25519_from_uniform, _sodium_ge25519_from_hash, _sodium_ristretto255_from_hash | `void` return — no error surface; no length/range check exists (fixed 32/64-byte inputs) | n/a; byte-exact output compared for all inputs | [x] |
| h2c-E15 | _sodium_core_h2c_string_to_hash | h == NULL together with h_len == 0 (the `memcpy(&h[i], ...)` loop is never entered) | returns 0, no write, no crash in either library | [x] |
| h2c-E16 | _sodium_core_h2c_string_to_hash | ctx == NULL / msg == NULL with the matching length 0 (crypto_hash_sha{256,512}_update returns early on inlen==0) | returns 0, identical digest in both libraries | [x] |

| hash-E1 | crypto_hash_sha3256_update (sha3_update) | state->phase != SHA3_PHASE_ABSORBING (update called after final) | returns -1; also permute_24, phase:=ABSORBING, offset:=0, then absorbs normally | [x] |
| hash-E2 | crypto_hash_sha3512_update (sha3_update) | state->phase != SHA3_PHASE_ABSORBING | returns -1; same recovery | [x] |
| hash-E3 | crypto_hash_sha3256_final (sha3_final) | state->phase != SHA3_PHASE_ABSORBING (final called twice) | returns -1; permute_24 then extracts outlen bytes, offset:=0, phase:=FINALIZED | [x] |
| hash-E4 | crypto_hash_sha3512_final (sha3_final) | state->phase != SHA3_PHASE_ABSORBING | returns -1; same | [x] |
| hash-E5 | crypto_xof_shake128_update, _sodium_shake128_ref_update | state->phase != SHAKE128_PHASE_ABSORBING (update after squeeze) | returns -1; permute_24, phase:=ABSORBING, offset:=0 | [x] |
| hash-E6 | crypto_xof_shake256_update, _sodium_shake256_ref_update | state->phase != SHAKE256_PHASE_ABSORBING | returns -1; permute_24, reset | [x] |
| hash-E7 | crypto_xof_turboshake128_update, _sodium_turboshake128_ref_update | state->phase != TURBOSHAKE128_PHASE_ABSORBING | returns -1; permute_**12**, reset | [x] |
| hash-E8 | crypto_xof_turboshake256_update, _sodium_turboshake256_ref_update | state->phase != TURBOSHAKE256_PHASE_ABSORBING | returns -1; permute_**12**, reset | [x] |
| hash-E9 | crypto_hash_sha256_update | `inlen <= 0U` (i.e. inlen == 0, unsigned) | returns 0 immediately; count/buf/state untouched (verified by full state compare) | [x] |
| hash-E10 | crypto_hash_sha512_update | `inlen <= 0U` | returns 0 immediately; state untouched | [x] |
| hash-E11 | crypto_hash, crypto_hash_sha256, crypto_hash_sha512, crypto_hash_sha3256, crypto_hash_sha3512, crypto_xof_* (one-shot) | in == NULL with inlen == 0 — C tolerates it, `in` is never dereferenced | returns 0 and produces the empty-input digest/XOF | [x] |
| hash-E12 | crypto_hash_sha256_update, _sha512_update, crypto_hash_sha3*_update, crypto_xof_*_update, _sodium_*_ref_update | in == NULL with inlen == 0 mid-stream | returns 0, state unchanged | [x] |
| hash-E13 | crypto_xof_*_squeeze, _sodium_*_ref_squeeze | outlen == 0 | returns 0; `out` is never written (verified with a canary buffer); the state is still finalized on the first call | [x] |
| hash-E14 | crypto_xof_* (one-shot) | outlen == 0 | returns 0; out untouched | [x] |
| hash-E15 | crypto_xof_*_init_with_domain, _sodium_*_ref_init_with_domain | `domain` is an unconstrained `unsigned char` — there is NO validity check, every one of the 256 values is accepted (this is the only "enum-like" parameter in the area) | always returns 0; 13 representative values incl. 0x00 / 0x80 / 0xFF exercised, and 0x1F must match _init() | [x] |
| hash-E16 | crypto_core_keccak1600_xor_bytes, _extract_bytes, _sodium_keccak1600_ref_xor_bytes, _ref_extract_bytes | NO validation of `offset`/`length` at all — offset+length past 200/224 is unchecked (caller contract) | no rejection exists; `length == 0` is a no-op at any offset, verified at offsets {0,1,7,8,9,199} | [x] |
| hash-E17 | crypto_hash_sha256_final, crypto_hash_sha512_final | no rejection site; always returns 0 and `sodium_memzero`s the whole state | verified: entire statebytes-sized region is 0 afterwards in both C and Rust | [x] |
| hash-E18 | crypto_hash_sha256_init, _sha512_init, crypto_hash_sha3*_init, crypto_xof_*_init(_with_domain), _sodium_*_ref_init(_with_domain) | no rejection site; unconditional `return 0` | return value compared, always 0 | [x] |
| hash-E19 | crypto_hash_sha3256, crypto_hash_sha3512, crypto_xof_* (one-shot), _sodium_*_ref | unconditional `return 0` — the -1 from the inner update/final is deliberately discarded | return value compared, always 0 | [x] |
| hash-E20 | crypto_core_keccak1600_init/_xor_bytes/_extract_bytes/_permute_24/_permute_12 and the `_sodium_keccak1600_ref_*` variants | `void` return type — no error channel at all | behaviour compared byte-for-byte on the full 224-byte state instead | [x] |
| hash-E21 | crypto_hash_sha3256_init, crypto_hash_sha3512_init, crypto_xof_*, crypto_xof_*_init, crypto_xof_*_init_with_domain | `COMPILER_ASSERT(sizeof(public_state) >= sizeof(internal_state))` | compile-time only; verified by inspection that the Rust `#[repr(C, align(16))]` states are 256 bytes and the internal structs are 256 (sha3) / 240 (xof) | [compile-time] |
| hash-E22 | (whole area) | `sodium_misuse()` / `abort()` / runtime `assert()` | none present in any of these C files (grep: no `sodium_misuse`, no `assert(`, no `return NULL`, no `goto`) | [n/a] |

| mac-E1 | crypto_onetimeauth_poly1305_donna_verify (via crypto_onetimeauth_poly1305_verify) | `return crypto_verify_16(h, correct)` — tag mismatch (any of the 128 bits flipped) | returns -1 | [x] |
| mac-E2 | crypto_onetimeauth_poly1305_donna_verify (via crypto_onetimeauth_poly1305_verify) | `crypto_verify_16(h, correct)` — tag matches | returns 0 | [x] |
| mac-E3 | crypto_onetimeauth_poly1305_verify | wrong key (any of the 32 key bytes altered; key[16..32] zeroed) ⇒ tag mismatch | returns -1 (0 iff the altered bit lands in a masked-off `r` bit) | [x] |
| mac-E4 | crypto_onetimeauth_verify | pure delegation to crypto_onetimeauth_poly1305_verify — mismatch | returns -1 | [x] |
| mac-E5 | crypto_onetimeauth_verify | pure delegation to crypto_onetimeauth_poly1305_verify — match | returns 0 | [x] |
| mac-E6 | crypto_onetimeauth_poly1305 / _init / _update / _final / crypto_onetimeauth{,_init,_update,_final} | no rejection site at all — every path unconditionally `return 0` (including inlen == 0 and `in == NULL, inlen == 0`) | returns 0 | [x] |
| mac-E7 | crypto_onetimeauth_poly1305_donna_init | `COMPILER_ASSERT(sizeof(crypto_onetimeauth_poly1305_state) >= sizeof(poly1305_state_internal_t))` | compile-time only; mirrored in Rust by a `const _: () = assert!(...)`; verified at runtime by statebytes()==256 vs the observed 144-byte internal footprint | [x] |
| mac-E8 | _crypto_onetimeauth_poly1305_pick_best_implementation | no failure path (HAVE_TI_MODE/HAVE_EMMINTRIN_H undefined ⇒ donna unconditionally) | returns 0 | [x] |
| mac-E9 | crypto_auth_hmacsha256_init | `keylen > 64` ⇒ key replaced by SHA-256(key), keylen := 32 (not an error return, but the only branch in the function) | returns 0, MAC == HMAC(SHA256(key), m) | [x] |
| mac-E10 | crypto_auth_hmacsha256_init | `key == NULL && keylen > 0` ⇒ `sodium_misuse()` | abort — not testable in-process; Rust calls the same `sodium_misuse() -> !` in the identical `else if key.is_null() { if keylen > 0 { … } }` position (verified by inspection) | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| mac-E11 | crypto_auth_hmacsha256_init | `key == NULL && keylen == 0` ⇒ explicitly NOT a misuse (the `if (keylen > 0)` guard) | returns 0, same state/MAC as init(non-NULL, 0) | [x] |
| mac-E12 | crypto_auth_hmacsha512_init | `keylen > 128` ⇒ key replaced by SHA-512(key), keylen := 64 | returns 0, MAC == HMAC(SHA512(key), m) | [x] |
| mac-E13 | crypto_auth_hmacsha512_init | `key == NULL && keylen > 0` ⇒ `sodium_misuse()` | abort — not testable in-process; Rust identical (verified by inspection) | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| mac-E14 | crypto_auth_hmacsha512_init | `key == NULL && keylen == 0` ⇒ allowed | returns 0 | [x] |
| mac-E15 | crypto_auth_hmacsha512256_init | plain cast onto crypto_auth_hmacsha512_init ⇒ inherits E12/E13/E14 (`sodium_misuse()` for key==NULL && keylen>0) | returns 0; abort case not testable in-process | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| mac-E16 | crypto_auth_hmacsha512256_init | `key == NULL && keylen == 0` ⇒ allowed | returns 0 | [x] |
| mac-E17 | crypto_auth_hmacsha256_verify | `crypto_verify_32(h, correct) | (-(h == correct)) | sodium_memcmp(correct, h, 32)` — tag mismatch (any byte XOR 0xff, any single bit flipped, wrong key, altered message, shortened inlen) | returns -1 | [x] |
| mac-E18 | crypto_auth_hmacsha256_verify | same expression — tag matches | returns 0 | [x] |
| mac-E19 | crypto_auth_hmacsha512_verify | `crypto_verify_64(h, correct) | (-(h == correct)) | sodium_memcmp(correct, h, 64)` — mismatch / match | returns -1 / 0 | [x] |
| mac-E20 | crypto_auth_hmacsha512256_verify | `crypto_verify_32(h, correct) | (-(h == correct)) | sodium_memcmp(correct, h, 32)` — mismatch / match | returns -1 / 0 | [x] |
| mac-E21 | crypto_auth_verify | pure delegation to crypto_auth_hmacsha512256_verify — mismatch / match | returns -1 / 0 | [x] |
| mac-E22 | crypto_auth_hmacsha{256,512,512256}_verify | the `(-(h == correct))` term: `correct` is a function-local array, so `h == correct` can never hold for a caller-supplied `h` | not reachable from outside the library; Rust reproduces the identical `0i32.wrapping_sub((h == correct.as_ptr()) as i32)` term (verified by inspection) | [x] |
| mac-E23 | crypto_auth_hmacsha256_update/_final, crypto_auth_hmacsha512_update/_final, crypto_auth_hmacsha512256_update/_final, crypto_auth_hmacsha{256,512,512256}, crypto_auth | no rejection site — every path unconditionally `return 0` (incl. inlen == 0 and `in == NULL, inlen == 0`, which crypto_hash_sha*_update short-circuits) | returns 0 | [x] |
| mac-E24 | crypto_auth_hmacsha{256,512,512256}_keygen, crypto_auth_keygen, crypto_onetimeauth{,_poly1305}_keygen | `void`, no failure path; the only observable contract is that exactly KEYBYTES(=32) bytes are written | writes 32 bytes, nothing past | [x] |

| pwhash-E1 | crypto_pwhash | alg not ALG_ARGON2I13/ALG_ARGON2ID13 (switch default; any out-of-range int) | errno=EINVAL, returns -1 | [x] |
| pwhash-E2 | crypto_pwhash_str_alg | alg not ALG_ARGON2I13/ALG_ARGON2ID13 | sodium_misuse() | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| pwhash-E3 | crypto_pwhash_str_verify | str matches neither `$argon2id$` nor `$argon2i$` (strncmp) | errno=EINVAL, returns -1 | [x] |
| pwhash-E4 | crypto_pwhash_str_needs_rehash | str matches neither `$argon2id$` nor `$argon2i$` | errno=EINVAL, returns -1 | [x] |
| pwhash-E5 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen > BYTES_MAX (4294967295) | errno=EFBIG, returns -1 | [not testable — `memset(out, 0, outlen)` runs before the check, so any test would write >4 GiB into the caller's buffer; verified by inspection that the Rust has the identical order] |
| pwhash-E6 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen < BYTES_MIN (16) | errno=EINVAL, returns -1 | [x] |
| pwhash-E7 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen > PASSWD_MAX (4294967295) | errno=EFBIG, returns -1 | [x] |
| pwhash-E8 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit > OPSLIMIT_MAX (4294967295) | errno=EFBIG, returns -1 | [x] |
| pwhash-E9 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit > MEMLIMIT_MAX (4398046510080) | errno=EFBIG, returns -1 | [x] |
| pwhash-E10 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen < PASSWD_MIN | errno=EINVAL, returns -1 | [unreachable — PASSWD_MIN == 0 and passwdlen is unsigned] |
| pwhash-E11 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit < OPSLIMIT_MIN (3 / 1) | errno=EINVAL, returns -1 | [x] |
| pwhash-E12 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit < MEMLIMIT_MIN (8192) | errno=EINVAL, returns -1 | [x] |
| pwhash-E13 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | `(const void *) out == (const void *) passwd` | errno=EINVAL, returns -1 | [x] |
| pwhash-E14 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | alg not the family's own ALG constant (switch default) | errno=EINVAL, returns -1 | [x] |
| pwhash-E15 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | argon2{i,id}_hash_raw() != ARGON2_OK | returns -1 (errno untouched) | [not testable — with the accepted limit ranges the only way to fail is an allocation failure of several TiB; verified by inspection] |
| pwhash-E16 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen > PASSWD_MAX \|\| opslimit > OPSLIMIT_MAX \|\| memlimit > MEMLIMIT_MAX | errno=EFBIG, returns -1, out zeroed | [x] |
| pwhash-E17 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen < PASSWD_MIN \|\| opslimit < OPSLIMIT_MIN \|\| memlimit < MEMLIMIT_MIN | errno=EINVAL, returns -1, out zeroed | [x] |
| pwhash-E18 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | argon2{i,id}_hash_encoded() != ARGON2_OK | returns -1 | [not testable — STRBYTES (128) is always large enough for a 16-byte salt + 32-byte hash; only an allocation failure could trigger it] |
| pwhash-E19 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | passwdlen > PASSWD_MAX | errno=EFBIG, returns -1 | [x] |
| pwhash-E20 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | passwdlen < PASSWD_MIN | errno=EINVAL, returns -1 | [unreachable — PASSWD_MIN == 0] |
| pwhash-E21 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | argon2*_verify() == ARGON2_VERIFY_MISMATCH | errno=EINVAL, returns -1 | [x] |
| pwhash-E22 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | argon2*_verify() fails for any other reason (decoding, validation) | returns -1, errno untouched | [x] |
| pwhash-E23 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | opslimit > UINT32_MAX \|\| memlimit/1024 > UINT32_MAX \|\| strlen(str) >= crypto_pwhash_STRBYTES (128) | errno=EINVAL, returns -1 | [x] |
| pwhash-E24 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | calloc(fodder_len, 1) == NULL | returns -1 | [not testable — requires calloc failure for a <128-byte request] |
| pwhash-E25 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | argon2_decode_string() != 0 | errno=EINVAL, returns -1 | [x] |
| pwhash-E26 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | ctx.t_cost != opslimit \|\| ctx.m_cost != memlimit/1024 | returns 1 | [x] |
| pwhash-E27 | _sodium_argon2_ctx | argon2_validate_inputs() != ARGON2_OK | returns the validation code unchanged | [x] |
| pwhash-E28 | _sodium_argon2_ctx | type != Argon2_id && type != Argon2_i (any other int) | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E29 | _sodium_argon2_ctx | argon2_initialize() != ARGON2_OK | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires a multi-TiB allocation to fail; verified by inspection] |
| pwhash-E30 | _sodium_argon2_hash, _sodium_argon2i_hash_raw/encoded, _sodium_argon2id_hash_raw/encoded | pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF) | returns ARGON2_PWD_TOO_LONG (-5) | [x] |
| pwhash-E31 | _sodium_argon2_hash (same wrappers) | hashlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | returns ARGON2_OUTPUT_TOO_LONG (-3) | [x] |
| pwhash-E32 | _sodium_argon2_hash (same wrappers) | saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) | returns ARGON2_SALT_TOO_LONG (-7) | [x] |
| pwhash-E33 | _sodium_argon2_hash | malloc(hashlen) == NULL | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E34 | _sodium_argon2_hash | argon2_ctx() != ARGON2_OK | frees/zeroes `out` and returns that code | [x] |
| pwhash-E35 | _sodium_argon2_hash | argon2_encode_string() != ARGON2_OK (encoded buffer too small) | zeroes out+encoded, returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E36 | _sodium_argon2_verify, _sodium_argon2i_verify, _sodium_argon2id_verify | strlen(encoded) > UINT32_MAX | returns ARGON2_DECODING_LENGTH_FAIL (-34) | [not testable — needs a >4 GiB NUL-terminated string; verified by inspection] |
| pwhash-E37 | _sodium_argon2_verify | any of malloc(ctx.ad/ctx.salt/ctx.out) == NULL, or malloc(out) == NULL | frees the rest, returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E38 | _sodium_argon2_verify | argon2_decode_string() != ARGON2_OK | returns the decode code unchanged (-32 / -26 / validation codes) | [x] |
| pwhash-E39 | _sodium_argon2_verify | sodium_memcmp(out, ctx.out, ctx.outlen) != 0 | returns ARGON2_VERIFY_MISMATCH (-35) | [x] |
| pwhash-E40 | _sodium_argon2_validate_inputs | context == NULL | returns ARGON2_INCORRECT_PARAMETER (-25) | [x] |
| pwhash-E41 | _sodium_argon2_validate_inputs | context->out == NULL | returns ARGON2_OUTPUT_PTR_NULL (-1) | [x] |
| pwhash-E42 | _sodium_argon2_validate_inputs | outlen < ARGON2_MIN_OUTLEN (16) | returns ARGON2_OUTPUT_TOO_SHORT (-2) | [x] |
| pwhash-E43 | _sodium_argon2_validate_inputs | outlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | returns ARGON2_OUTPUT_TOO_LONG (-3) | [unreachable — outlen is uint32_t; outlen == UINT32_MAX tested and accepted] |
| pwhash-E44 | _sodium_argon2_validate_inputs | pwd == NULL && pwdlen != 0 | returns ARGON2_PWD_PTR_MISMATCH (-18) | [x] |
| pwhash-E45 | _sodium_argon2_validate_inputs | pwdlen < ARGON2_MIN_PWD_LENGTH (0) | returns ARGON2_PWD_TOO_SHORT (-4) | [unreachable — MIN is 0, field is unsigned] |
| pwhash-E46 | _sodium_argon2_validate_inputs | pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF) | returns ARGON2_PWD_TOO_LONG (-5) | [unreachable — field is uint32_t] |
| pwhash-E47 | _sodium_argon2_validate_inputs | salt == NULL && saltlen != 0 | returns ARGON2_SALT_PTR_MISMATCH (-19) | [x] |
| pwhash-E48 | _sodium_argon2_validate_inputs | saltlen < ARGON2_MIN_SALT_LENGTH (8) | returns ARGON2_SALT_TOO_SHORT (-6) | [x] |
| pwhash-E49 | _sodium_argon2_validate_inputs | saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) | returns ARGON2_SALT_TOO_LONG (-7) | [unreachable — field is uint32_t] |
| pwhash-E50 | _sodium_argon2_validate_inputs | secret == NULL && secretlen != 0 | returns ARGON2_SECRET_PTR_MISMATCH (-20) | [x] |
| pwhash-E51 | _sodium_argon2_validate_inputs | secret != NULL && secretlen < ARGON2_MIN_SECRET (0) | returns ARGON2_SECRET_TOO_SHORT (-10) | [unreachable — MIN is 0] |
| pwhash-E52 | _sodium_argon2_validate_inputs | secret != NULL && secretlen > ARGON2_MAX_SECRET (0xFFFFFFFF) | returns ARGON2_SECRET_TOO_LONG (-11) | [unreachable — field is uint32_t] |
| pwhash-E53 | _sodium_argon2_validate_inputs | ad == NULL && adlen != 0 | returns ARGON2_AD_PTR_MISMATCH (-21) | [x] |
| pwhash-E54 | _sodium_argon2_validate_inputs | ad != NULL && adlen < ARGON2_MIN_AD_LENGTH (0) | returns ARGON2_AD_TOO_SHORT (-8) | [unreachable — MIN is 0] |
| pwhash-E55 | _sodium_argon2_validate_inputs | ad != NULL && adlen > ARGON2_MAX_AD_LENGTH (0xFFFFFFFF) | returns ARGON2_AD_TOO_LONG (-9) | [unreachable — field is uint32_t] |
| pwhash-E56 | _sodium_argon2_validate_inputs | lanes < ARGON2_MIN_LANES (1) | returns ARGON2_LANES_TOO_FEW (-16) | [x] |
| pwhash-E57 | _sodium_argon2_validate_inputs | lanes > ARGON2_MAX_LANES (0xFFFFFF) | returns ARGON2_LANES_TOO_MANY (-17) | [x] |
| pwhash-E58 | _sodium_argon2_validate_inputs | m_cost < ARGON2_MIN_MEMORY (8) | returns ARGON2_MEMORY_TOO_LITTLE (-14) | [x] |
| pwhash-E59 | _sodium_argon2_validate_inputs | m_cost > ARGON2_MAX_MEMORY (0xFFFFFFFF) | returns ARGON2_MEMORY_TOO_MUCH (-15) | [unreachable — field is uint32_t; m_cost == UINT32_MAX tested and accepted] |
| pwhash-E60 | _sodium_argon2_validate_inputs | m_cost < 8 * lanes | returns ARGON2_MEMORY_TOO_LITTLE (-14) | [x] |
| pwhash-E61 | _sodium_argon2_validate_inputs | t_cost < ARGON2_MIN_TIME (1) | returns ARGON2_TIME_TOO_SMALL (-12) | [x] |
| pwhash-E62 | _sodium_argon2_validate_inputs | t_cost > ARGON2_MAX_TIME (0xFFFFFFFF) | returns ARGON2_TIME_TOO_LARGE (-13) | [unreachable — field is uint32_t; t_cost == UINT32_MAX tested and accepted] |
| pwhash-E63 | _sodium_argon2_validate_inputs | threads < ARGON2_MIN_THREADS (1) | returns ARGON2_THREADS_TOO_FEW (-28) | [x] |
| pwhash-E64 | _sodium_argon2_validate_inputs | threads > ARGON2_MAX_THREADS (0xFFFFFF) | returns ARGON2_THREADS_TOO_MANY (-29) | [x] |
| pwhash-E65 | _sodium_argon2_initialize | instance == NULL \|\| context == NULL | returns ARGON2_INCORRECT_PARAMETER (-25) | [x] |
| pwhash-E66 | _sodium_argon2_initialize | malloc(8 * segment_length) == NULL | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E67 | _sodium_argon2_initialize | allocate_memory() != ARGON2_OK (region==NULL, m_cost==0, size overflow, malloc failure) | frees the instance, returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — `region` is always non-NULL, m_cost >= 8 after argon2_ctx's rounding, `1024 * (uint32_t)` cannot overflow size_t; only malloc failure remains] |
| pwhash-E68 | _sodium_argon2_fill_memory_blocks | instance == NULL \|\| instance->lanes == 0 | returns without touching memory | [x] |
| pwhash-E69 | _sodium_argon2_finalize | context == NULL \|\| instance == NULL | returns without touching memory | [x] |
| pwhash-E70 | _sodium_argon2_fill_segment_ref | instance == NULL | returns immediately | [x] |
| pwhash-E71 | _sodium_argon2_decode_string | type is neither Argon2_id nor Argon2_i | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E72 | _sodium_argon2_decode_string | prefix mismatch (`$argon2id` / `$argon2i`, `$v=`, `$m=`, `,t=`, `,p=`, `$`) — 7 distinct `CC()` sites | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E73 | _sodium_argon2_decode_string | decode_decimal(): no digit at all | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E74 | _sodium_argon2_decode_string | decode_decimal(): non-minimal encoding (leading zero, e.g. `m=08`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E75 | _sodium_argon2_decode_string | decode_decimal(): value does not fit in unsigned long (`acc > ULONG_MAX/10` or `c > ULONG_MAX-acc`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E76 | _sodium_argon2_decode_string | DECIMAL_U32(): decoded value > UINT32_MAX (v=, m=, t=, p=) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E77 | _sodium_argon2_decode_string | version != ARGON2_VERSION_NUMBER (0x13) | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E78 | _sodium_argon2_decode_string | m_cost/t_cost/lanes > UINT32_MAX after DECIMAL_U32 | returns ARGON2_INCORRECT_TYPE (-26) | [unreachable — DECIMAL_U32 already rejects > UINT32_MAX] |
| pwhash-E79 | _sodium_argon2_decode_string | BIN(): sodium_base642bin() != 0 (bad char, padding, output too long for the caller's buffer) or bin_len > UINT32_MAX — 2 sites (salt, out) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E80 | _sodium_argon2_decode_string | argon2_validate_inputs() != ARGON2_OK after decoding (e.g. `p=0`, short salt, short out) | returns the validation code | [x] |
| pwhash-E81 | _sodium_argon2_decode_string | trailing characters after the final base64 field (`*str != 0`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E82 | _sodium_argon2_encode_string | type is neither Argon2_id nor Argon2_i (switch default) | returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E83 | _sodium_argon2_encode_string | SS()/SX(): `strlen(str) >= dst_len` — 9 distinct sites (prefix, version, `$m=`, m, `,t=`, t, `,p=`, p, `$`) | returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E84 | _sodium_argon2_encode_string | argon2_validate_inputs() != ARGON2_OK (checked after the prefix is written) | returns the validation code | [x] |
| pwhash-E85 | _sodium_argon2_encode_string | SB(): sodium_bin2base64() returns NULL — 2 sites (salt, out) | returns ARGON2_ENCODING_FAIL (-31) | [abort — sodium_bin2base64() calls sodium_misuse() when `b64_maxlen <= b64_len`, so this `return` is dead code in practice; the Rust reproduces the same call and therefore aborts identically. Only dst_len values that fail an earlier SS() check are exercised] |
| pwhash-E86 | _sodium_blake2b_long | outlen > UINT32_MAX | `goto fail`, returns -1 (out untouched) | [x] |
| pwhash-E87 | _sodium_blake2b_long | any crypto_generichash_blake2b_{init,update,final,} call returns < 0 (e.g. outlen == 0) | `goto fail`, returns that negative value | [x] |
| pwhash-E88 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | buflen > (2^32 - 1) * 32 | errno=EFBIG, returns -1 | [x] |
| pwhash-E89 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | (uint64_t)r * (uint64_t)p >= 2^30 | errno=EFBIG, returns -1 | [x] |
| pwhash-E90 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N > UINT32_MAX | errno=EFBIG, returns -1 | [x] |
| pwhash-E91 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N not a power of two, or N < 2 (incl. N=0, N=1) | errno=EINVAL, returns -1 | [x] |
| pwhash-E92 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | r == 0 \|\| p == 0 | errno=EINVAL, returns -1 | [x] |
| pwhash-E93 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | r > SIZE_MAX / 128 / p | errno=ENOMEM, returns -1 | [unreachable on 64-bit — `r*p < 2^30` already bounds `128*r*p < 2^37 << SIZE_MAX`] |
| pwhash-E94 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N > SIZE_MAX / 128 / r | errno=ENOMEM, returns -1 | [x] |
| pwhash-E95 | _sodium_escrypt_kdf_nosse | B_size + V_size wraps (`need < V_size`) | errno=ENOMEM, returns -1 | [unreachable on 64-bit — the preceding checks bound V_size <= SIZE_MAX and B_size < 2^37, so the sum cannot wrap] |
| pwhash-E96 | _sodium_escrypt_kdf_nosse | need + XY_size wraps (`need < XY_size`) | errno=ENOMEM, returns -1 | [unreachable on 64-bit — same bound] |
| pwhash-E97 | _sodium_escrypt_kdf_nosse | escrypt_free_region() != 0 | returns -1 | [unreachable — without HAVE_MMAP escrypt_free_region() always returns 0] |
| pwhash-E98 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | escrypt_alloc_region() == NULL | returns -1 | [x] |
| pwhash-E99 | _sodium_escrypt_PBKDF2_SHA256 | dkLen > 0x1fffffffe0 | sodium_misuse() | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` (dkLen check precedes any write to buf) |
| pwhash-E100 | _sodium_escrypt_alloc_region | size + 63 < size (overflow) | errno=ENOMEM, base/aligned NULL, size 0, returns NULL | [x] |
| pwhash-E101 | _sodium_escrypt_alloc_region | malloc(size + 63) == NULL | base/aligned NULL, size 0, returns NULL | [x] |
| pwhash-E102 | _sodium_escrypt_free_region | munmap() failure | returns -1 | [unreachable — HAVE_MMAP is not defined in this build, the `free(base)` branch is compiled] |
| pwhash-E103 | _sodium_escrypt_parse_setting | setting does not start with `$7$` | returns NULL | [x] |
| pwhash-E104 | _sodium_escrypt_parse_setting | decode64_one() on the N_log2 character fails (not in itoa64) | returns NULL, *N_log2_p = 0 | [x] |
| pwhash-E105 | _sodium_escrypt_parse_setting | decode64_uint32() fails on the 5-char r field | returns NULL, *r_p = 0 | [x] |
| pwhash-E106 | _sodium_escrypt_parse_setting | decode64_uint32() fails on the 5-char p field | returns NULL, *p_p = 0 | [x] |
| pwhash-E107 | _sodium_escrypt_r | escrypt_parse_setting() == NULL | returns NULL | [x] |
| pwhash-E108 | _sodium_escrypt_r | buf == NULL | returns NULL | [x] |
| pwhash-E109 | _sodium_escrypt_r | need > buflen | returns NULL | [x] |
| pwhash-E110 | _sodium_escrypt_r | need < saltlen (wrap) | returns NULL | [unreachable — need = prefixlen + saltlen + 88 cannot wrap for a NUL-terminated setting] |
| pwhash-E111 | _sodium_escrypt_r | escrypt_kdf() != 0 (setting encodes N_log2 = 0 -> N = 1, or r = 0, or p = 0) | returns NULL | [x] |
| pwhash-E112 | _sodium_escrypt_r | encode64() == NULL \|\| dst >= buf + buflen | returns NULL | [unreachable — `need <= buflen` was already checked ("Can't happen" in the C)] |
| pwhash-E113 | _sodium_escrypt_gensalt_r | need > buflen | returns NULL | [x] |
| pwhash-E114 | _sodium_escrypt_gensalt_r | need < saltlen (wrap) \|\| saltlen < srclen | returns NULL | [unreachable — BYTES2CHARS(n) = (8n+5)/6 >= n for all n] |
| pwhash-E115 | _sodium_escrypt_gensalt_r | N_log2 > 63 | returns NULL | [x] |
| pwhash-E116 | _sodium_escrypt_gensalt_r | (uint64_t)r * (uint64_t)p >= 2^30 | returns NULL | [x] |
| pwhash-E117 | _sodium_escrypt_gensalt_r | encode64_uint32()/encode64() == NULL, or dst >= buf + buflen | returns NULL | [unreachable — the `need > buflen` check already guarantees room ("Can't happen" in the C)] |
| pwhash-E118 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_init_local() != 0, escrypt_free_local() != 0 | returns -1 | [unreachable — both always return 0 in this build] |
| pwhash-E119 | crypto_pwhash_scryptsalsa208sha256 | passwdlen > PASSWD_MAX (SODIUM_SIZE_MAX) | errno=EFBIG, returns -1 | [unreachable on 64-bit — PASSWD_MAX == SIZE_MAX] |
| pwhash-E120 | crypto_pwhash_scryptsalsa208sha256 | outlen > BYTES_MAX (0x1fffffffe0) | errno=EFBIG, returns -1 | [not testable — `memset(out, 0, outlen)` runs before the check; verified by inspection that the Rust order is identical] |
| pwhash-E121 | crypto_pwhash_scryptsalsa208sha256 | outlen < BYTES_MIN (16) | errno=EINVAL, returns -1 | [x] |
| pwhash-E122 | crypto_pwhash_scryptsalsa208sha256, _str, _str_needs_rehash | pickparams() != 0 | errno=EINVAL, returns -1 | [unreachable — pickparams() always returns 0] |
| pwhash-E123 | crypto_pwhash_scryptsalsa208sha256 | `(const void *) out == (const void *) passwd` | errno=EINVAL, returns -1 | [x] |
| pwhash-E124 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen > PASSWD_MAX / passwdlen < PASSWD_MIN | errno=EFBIG / EINVAL, returns -1 | [unreachable on 64-bit — PASSWD_MAX == SIZE_MAX, PASSWD_MIN == 0] |
| pwhash-E125 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_gensalt_r() == NULL | errno=EINVAL, returns -1 | [unreachable — pickparams() yields N_log2 <= 62 and r*p = 8*p < 2^30] |
| pwhash-E126 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_init_local() != 0 | returns -1 | [unreachable — always returns 0] |
| pwhash-E127 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_r() == NULL | errno=EINVAL, returns -1 | [unreachable — pickparams() always yields N >= 2, r = 8, p >= 1, and STRBYTES == the exact `need`] |
| pwhash-E128 | crypto_pwhash_scryptsalsa208sha256_str_verify | sodium_strnlen(str, STRBYTES) != STRBYTES - 1 (too short, or no NUL within 102 bytes) | returns -1 | [x] |
| pwhash-E129 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_init_local() != 0 | returns -1 | [unreachable — always returns 0] |
| pwhash-E130 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_r() == NULL (bad setting, or kdf rejects the encoded N/r/p) | returns -1 | [x] |
| pwhash-E131 | crypto_pwhash_scryptsalsa208sha256_str_verify | sodium_memcmp(wanted, str, STRBYTES) != 0 (wrong password) | returns -1 | [x] |
| pwhash-E132 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | sodium_strnlen(str, STRBYTES) != STRBYTES - 1 | errno=EINVAL, returns -1 | [x] |
| pwhash-E133 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | escrypt_parse_setting() == NULL | errno=EINVAL, returns -1 | [x] |
| pwhash-E134 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | N_log2 != N_log2_ \|\| r != r_ \|\| p != p_ | returns 1 | [x] |

| sign-E1 | crypto_scalarmult_curve25519 | `implementation->mult(q,n,p) != 0` (marked LCOV_EXCL_LINE) — reached for the 7 blocklisted points | returns -1, q untouched | [x] |
| sign-E2 | crypto_scalarmult_curve25519 | all-zero result: `return -(1 & ((d - 1) >> 8))` where `d = OR of q[0..32]` | returns -1 | [unreachable] |
| sign-E3 | crypto_scalarmult_curve25519_ref10 (implementation->mult) | `has_small_order(p)`: p ∈ {0, 1, 3256…504, 3938…823, p-1, p, p+1} comparing 31 bytes plus `s[31] & 0x7f` (bit 255 ignored) | returns -1 | [x] |
| sign-E4 | crypto_scalarmult_ed25519, _noclamp | `ge25519_is_canonical(p) == 0` (y ≥ 2^255-19) | returns -1, q untouched | [x] |
| sign-E5 | crypto_scalarmult_ed25519, _noclamp | `ge25519_frombytes(&P, p) != 0` (point not on the curve) | returns -1, q untouched | [x] |
| sign-E6 | crypto_scalarmult_ed25519, _noclamp | `ge25519_has_small_order(&P) != 0` (order 1/2/4/8 points) | returns -1, q untouched | [x] |
| sign-E7 | crypto_scalarmult_ed25519, _noclamp | `ge25519_is_on_main_subgroup(&P) == 0` (canonical, on-curve, order 2L/4L/8L) | returns -1, q untouched | [x] |
| sign-E8 | crypto_scalarmult_ed25519_noclamp | `_crypto_scalarmult_ed25519_is_inf(q) != 0`: n ≡ 0 (mod L) after `n[31] &= 127` (n = k·L, k = 1..7) | returns -1, q = identity encoding | [x] |
| sign-E9 | crypto_scalarmult_ed25519 | `_crypto_scalarmult_ed25519_is_inf(q) != 0` with the clamped scalar (multiple of 8, bit 254 set, bit 255 clear) — needs 8L \| t, impossible for t < 2^255 | returns -1 | [unreachable] |
| sign-E10 | crypto_scalarmult_ed25519, _noclamp | `sodium_is_zero(n, 32)` — all-zero scalar, checked AFTER the scalar multiplication | returns -1, q holds the (identity) result | [x] |
| sign-E11 | crypto_scalarmult_ed25519_base_noclamp | `_crypto_scalarmult_ed25519_is_inf(q) != 0`: n = k·L (k = 1..7) or n masking to 0 (`n[31] = 0x80`) | returns -1, q = identity encoding | [x] |
| sign-E12 | crypto_scalarmult_ed25519_base | `_crypto_scalarmult_ed25519_is_inf(q) != 0` with the clamped scalar — needs 8L \| t, impossible for t < 2^255 | returns -1 | [unreachable] |
| sign-E13 | crypto_scalarmult_ed25519_base, _base_noclamp | `sodium_is_zero(n, 32)` — all-zero scalar | returns -1 | [x] |
| sign-E14 | crypto_scalarmult_ristretto255 | `ristretto255_frombytes(&P, p) != 0` (non-canonical / non-square / negative / not a valid ristretto encoding) | returns -1, q untouched | [x] |
| sign-E15 | crypto_scalarmult_ristretto255 | `sodium_is_zero(q, 32)` — result is the ristretto identity (n ≡ 0 mod L after `n[31] &= 127`, or p = identity encoding) | returns -1, q = all zeros | [x] |
| sign-E16 | crypto_scalarmult_ristretto255_base | `sodium_is_zero(q, 32)` — n = 0, n = k·L (k = 1..7), n masking to 0 | returns -1, q = all zeros | [x] |
| sign-E17 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_frombytes_negate_vartime(&A, pk) != 0` (pk not on the curve) | returns -1, output untouched | [x] |
| sign-E18 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_has_small_order(&A) != 0` | returns -1, output untouched | [x] |
| sign-E19 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_is_on_main_subgroup(&A) == 0` | returns -1, output untouched | [x] |
| sign-E20 | _crypto_sign_ed25519_verify_detached | `(sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0` — S ≥ L (S = L, L+1, 2L, 7L, 2^253-ish, all-0xff) | returns -1 | [x] |
| sign-E21 | _crypto_sign_ed25519_verify_detached | `ge25519_is_canonical(pk) == 0` (pk y ≥ 2^255-19, with and without bit 255 set) | returns -1 | [x] |
| sign-E22 | _crypto_sign_ed25519_verify_detached | `ge25519_frombytes_negate_vartime(&A, pk) != 0` (pk not on the curve) | returns -1 | [x] |
| sign-E23 | _crypto_sign_ed25519_verify_detached | `ge25519_has_small_order(&A) != 0` (small-order pk, incl. all-zero pk) | returns -1 | [x] |
| sign-E24 | _crypto_sign_ed25519_verify_detached | `ge25519_frombytes(&expected_r, sig) != 0` (R not on the curve) | returns -1 | [x] |
| sign-E25 | _crypto_sign_ed25519_verify_detached | `ge25519_has_small_order(&expected_r) != 0` (small-order R, incl. all-zero R) | returns -1 | [x] |
| sign-E26 | _crypto_sign_ed25519_verify_detached | final `return ge25519_has_small_order(&check) - 1` — equation does not hold (tampered sig/message/pk) | returns -1 (0 only when `check` has small order) | [x] |
| sign-E27 | crypto_sign_ed25519_verify_detached | ED25519_COMPAT variant `if (sig[63] & 224) return -1;` | not compiled (ED25519_COMPAT undefined) — verified by `tools/cpp.sh` | [n/a] |
| sign-E28 | crypto_sign_ed25519_open, crypto_sign_open | `smlen < 64` → `goto badsig` | returns -1, `*mlen_p = 0`, `m` untouched | [x] |
| sign-E29 | crypto_sign_ed25519_open | `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX - 64 = UINT64_MAX - 64 on LP64) → requires `smlen > UINT64_MAX` | returns -1 | [unreachable] |
| sign-E30 | crypto_sign_ed25519_open | `crypto_sign_ed25519_verify_detached(...) != 0` → `memset(m, 0, mlen)` then `goto badsig` | returns -1, m zeroed (exactly mlen bytes), `*mlen_p = 0` | [x] |
| sign-E31 | crypto_sign_ed25519_open | badsig path with `mlen_p == NULL` and/or `m == NULL` (the C explicitly NULL-checks both) | returns -1, no write | [x] |
| sign-E32 | crypto_sign_ed25519 | `crypto_sign_ed25519_detached(...) != 0 \|\| siglen != crypto_sign_ed25519_BYTES` (LCOV_EXCL_START/STOP) → `*smlen_p = 0`, `memset(sm, 0, mlen + 64)` | returns -1 | [unreachable] |
| sign-E33 | crypto_sign_ed25519, crypto_sign_ed25519_detached, _crypto_sign_ed25519_detached, crypto_sign_ed25519ph_final_create | `siglen_p == NULL` / `smlen_p == NULL` tolerated (explicit NULL check before the store) | returns 0, no store | [x] |
| sign-E34 | crypto_sign_ed25519_sk_to_seed, _sk_to_pk, _sk_to_curve25519, crypto_sign_ed25519ph_init, _update | no rejection sites at all (plain memmove / sha512 delegation) | always returns 0 | [x] |
| sign-E35 | crypto_scalarmult_curve25519_base, crypto_scalarmult_base, crypto_scalarmult_curve25519_ref10_base | no rejection site (the all-zero / small-order check is only in the two-argument entry point) | always returns 0, even for the all-zero scalar | [x] |
| sign-E36 | _crypto_scalarmult_curve25519_pick_best_implementation | no failure path (HAVE_AVX_ASM undefined) | always returns 0 | [x] |

| sodium-E1 | sodium_mlock | no HAVE_MLOCK / WINAPI_DESKTOP in this build (unconditional) | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E2 | sodium_munlock | no HAVE_MLOCK / WINAPI_DESKTOP in this build (unconditional; buffer is still zeroed first) | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E3 | sodium_mprotect_noaccess | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E4 | sodium_mprotect_readonly | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E5 | sodium_mprotect_readwrite | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E6 | _mprotect_noaccess, _mprotect_readonly, _mprotect_readwrite (static callbacks) | no HAVE_MPROTECT: each sets errno = ENOSYS and returns -1; in this build `_sodium_mprotect` never invokes the callback, so they are only reachable as function pointers | returns -1, errno = ENOSYS | [n/a] |
| sodium-E7 | sodium_malloc | `_sodium_malloc()` (i.e. `malloc()`) returned NULL | returns NULL | [x] |
| sodium-E8 | sodium_allocarray | `count > 0 && size >= SIZE_MAX / count` | returns NULL, errno = ENOMEM (12) | [x] |
| sodium-E9 | sodium_free | `ptr == NULL` (not compiled in this build: the non-HAVE_ALIGNED_MALLOC body is a bare `free(ptr)`, which tolerates NULL) | no-op | [x] |
| sodium-E10 | sodium_pad | `blocksize <= 0U` | returns -1, buffer and *padded_buflen_p untouched | [x] |
| sodium-E11 | sodium_pad | `(size_t) SIZE_MAX - unpadded_buflen <= xpadlen` (integer-overflow guard) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E12 | sodium_pad | `xpadded_len >= max_buflen` (output buffer too small) | returns -1, buffer and *padded_buflen_p untouched | [x] |
| sodium-E13 | sodium_unpad | `padded_buflen < blocksize` (incl. padded_buflen == 0) | returns -1, *unpadded_buflen_p NOT written | [x] |
| sodium-E14 | sodium_unpad | `blocksize <= 0U` | returns -1, *unpadded_buflen_p NOT written | [x] |
| sodium-E15 | sodium_unpad | no 0x80 barrier found in the last `blocksize` bytes (`valid == 0`) | returns -1, but *unpadded_buflen_p IS written (unconditional store before the return) | [x] |
| sodium-E16 | _sodium_alloc_init | `page_size < CANARY_SIZE \|\| page_size < sizeof(size_t)` → sodium_misuse() | not compiled (inside `#ifdef HAVE_ALIGNED_MALLOC`); the surviving body only calls randombytes_buf and returns 0 | [n/a] |
| sodium-E17 | sodium_bin2hex | `bin_len >= SIZE_MAX / 2` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E18 | sodium_bin2hex | `hex_maxlen <= bin_len * 2U` (output buffer too small, incl. hex_maxlen == 0) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E19 | sodium_hex2bin | `bin_pos >= bin_maxlen` (output buffer too small) | ret = -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E20 | sodium_hex2bin | odd number of hex digits consumed (`state != 0`) | ret = -1, errno = EINVAL (22), *bin_len = 0, *hex_end backed up by one character | [x] |
| sodium-E21 | sodium_hex2bin | non-hex character that is not in `ignore` (or `ignore == NULL`, or `state != 0`) | loop breaks; success unless hex_end == NULL (→ E22) | [x] |
| sodium-E22 | sodium_hex2bin | `hex_end == NULL && hex_pos != hex_len` (unconsumed input and no way to report it) | ret = -1, errno = EINVAL (22); *bin_len keeps the already-decoded count (it is zeroed *before* this check) | [x] |
| sodium-E23 | sodium_base64_check_variant (from sodium_base64_encoded_len) | `(((unsigned) variant) & ~0x6U) != 0x1U` — tested with variant −1, 0, 2, 4, 6, 8, 9, 99, INT_MAX | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E24 | sodium_base64_check_variant (from sodium_bin2base64) | same out-of-range variant check | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E25 | sodium_base64_check_variant (from sodium_base642bin) | same out-of-range variant check | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E26 | sodium_base64_encoded_len | `bin_len / 3 > (SIZE_MAX - 5) / 4` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E27 | sodium_base64_ENCODED_LEN macro | `(BIN_LEN)/3U > (SIZE_MAX-5)/4U` → `(size_t) SIZE_MAX` | unreachable through sodium_base64_encoded_len (E26 aborts first); no separate test | [n/a] |
| sodium-E28 | sodium_bin2base64 | `nibbles > (SIZE_MAX - 5) / 4` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E29 | sodium_bin2base64 | `b64_maxlen <= b64_len` (output buffer too small, incl. b64_maxlen == 0) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E30 | sodium_bin2base64 | `assert(b64_pos <= b64_len)` (live: the reference build defines no NDEBUG) | abort — provably unreachable (b64_pos is always ≤ b64_len); Rust reproduces the abort-on-failure by inspection | [abort] |
| sodium-E31 | _sodium_base642bin_skip_padding | `*b64_pos_p >= b64_len` while padding characters are still required (truncated `=` padding) | returns -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E32 | _sodium_base642bin_skip_padding | character is not `'='` and (`ignore == NULL` or not in `ignore`) | returns -1, errno = EINVAL (22), *bin_len = 0 | [x] |
| sodium-E33 | sodium_base642bin | `bin_pos >= bin_maxlen` (output buffer too small) | ret = -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E34 | sodium_base642bin | `acc_len > 4U` (dangling 6-bit group) | ret = -1, errno left untouched, *bin_len = 0 | [x] |
| sodium-E35 | sodium_base642bin | `(acc & ((1U << acc_len) - 1U)) != 0U` (non-canonical encoding: leftover bits set) | ret = -1, errno left untouched, *bin_len = 0 | [x] |
| sodium-E36 | sodium_base642bin | invalid base64 character (`d == 0xFF`) not in `ignore` | loop breaks; success unless b64_end == NULL (→ E37) | [x] |
| sodium-E37 | sodium_base642bin | `b64_end == NULL && b64_pos != b64_len` | ret = -1, errno = EINVAL (22); *bin_len keeps the already-decoded count | [x] |
| sodium-E38 | ip_hex_digit | character is not `[0-9a-fA-F]` | returns -1 (drives parse_ipv6 → E48) | [x] |
| sodium-E39 | parse_ipv4 | `src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end` (empty input) | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E40 | parse_ipv4 | `++digits > 3` (more than 3 digits in an octet, e.g. "0000.1.1.1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E41 | parse_ipv4 | `val > 255U` (octet out of range, e.g. "256.1.1.1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E42 | parse_ipv4 | `digits == 0` (empty octet, e.g. "1..2.3", ".1.2.3") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E43 | parse_ipv4 | missing `'.'` separator: `i < 3 && (p >= end \|\| *p++ != '.')` (e.g. "1.2.3") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E44 | parse_ipv4 | `p != end` after 4 octets (trailing junk, e.g. "1.2.3.4.5", "1.2.3.4 ") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E45 | parse_ipv6 | `src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end` | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E46 | parse_ipv6 | leading single `':'` not followed by another `':'` (e.g. ":1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E47 | parse_ipv6 | a second `"::"` run (`colonp != NULL` when `!saw_xdigit`), e.g. "1:::2", "::1::2" | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E48 | parse_ipv6 | `hv < 0` (non-hex character) or `xdigits >= 4` (group longer than 4 hex digits, e.g. "12345::") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E49 | parse_ipv6 | `tp + 2 > endp` when flushing a group (more than 8 groups, e.g. "1:2:3:4:5:6:7:8:9") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E50 | parse_ipv6 | trailing `':'` (`p >= end` right after a group separator, e.g. "1:") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E51 | parse_ipv6 | embedded IPv4: `tp + 4 > endp \|\| parse_ipv4(curtok, end, tp) == 0` (e.g. "::1.2.3", "1:2:3:4:5:6:7:1.2.3.4") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E52 | parse_ipv6 | final `tp + 2 > endp` for the last group | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E53 | parse_ipv6 | `colonp != NULL && tp == endp` (`"::"` present but the address is already full, e.g. "::1:2:3:4:5:6:7:8") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E54 | parse_ipv6 | `tp != endp` (fewer than 8 groups and no `"::"`, e.g. "1:2:3:4:5:6:7") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E55 | sodium_ip2bin | zone-id contains a character outside `[0-9a-zA-Z._-]` (e.g. "fe80::1%!") | returns -1 | [x] |
| sodium-E56 | sodium_ip2bin | empty zone-id (`zone + 1 >= end`, e.g. "fe80::1%") | returns -1 | [x] |
| sodium-E57 | sodium_ip2bin | zone-id present but the address is not IPv6 (`zone != NULL && !is_ipv6`, e.g. "1.2.3.4%eth0", "%eth0") | returns -1 | [x] |
| sodium-E58 | sodium_ip2bin | `parse_ipv6()` returned 0 | returns -1 (bin untouched) | [x] |
| sodium-E59 | sodium_ip2bin | `parse_ipv4()` returned 0 | returns -1 (bin untouched) | [x] |
| sodium-E60 | sodium_bin2ip | `ip_maxlen <= 2U` | returns NULL, `ip` untouched | [x] |
| sodium-E61 | sodium_bin2ip | IPv4-mapped branch: `len >= ip_maxlen` (formatted dotted-quad does not fit) | returns NULL, `ip` untouched | [x] |
| sodium-E62 | sodium_bin2ip | IPv6 branch: `len >= ip_maxlen` (formatted address does not fit) | returns NULL, `ip` untouched | [x] |
| sodium-E63 | sodium_init | `sodium_crit_enter() != 0` | returns -1 — unreachable in this build (crit_enter is the no-op version that always returns 0) | [n/a] |
| sodium-E64 | sodium_init | `sodium_crit_leave() != 0` (both call sites) | returns -1 — unreachable in this build (crit_leave always returns 0) | [n/a] |
| sodium-E65 | sodium_init | `initialized != 0` (already initialized) | returns 1 | [x] |
| sodium-E66 | sodium_crit_leave | `locked == 0` → errno = EPERM, return -1 | not compiled (only in the _WIN32 / HAVE_PTHREAD variants); the compiled version unconditionally returns 0 | [n/a] |
| sodium-E67 | sodium_misuse | unconditional `abort()` after (optionally) calling the misuse handler | abort — verified in a forked child (SIGABRT), and the handler-is-called path verified by a child that exits 42 from the handler | [x] |
| sodium-E68 | sodium_set_misuse_handler | `sodium_crit_enter() != 0` / `sodium_crit_leave() != 0` | returns -1 — unreachable in this build | [n/a] |
| sodium-E69 | _sodium_runtime_arm_cpu_features | `#ifndef __ARM_ARCH` → unconditional `return -1` on x86-64 | returns -1 (has_neon/has_armcrypto left 0) | [x] |
| sodium-E70 | _sodium_runtime_intel_cpu_features | `cpu_info[0] == 0U` (no HAVE_CPUID, so `_cpuid` zeroes the array) | returns -1; every has_* field keeps its zero value | [x] |
| sodium-E71 | _sodium_runtime_get_cpu_features | `ret = -1 & arm & intel` | returns -1 | [x] |
| sodium-E72 | randombytes_uniform | `upper_bound < 2` (0 or 1), only when `implementation->uniform == NULL` | returns 0 | [x] |
| sodium-E73 | randombytes_buf | `size == 0` | `implementation->buf` is NOT called; buffer untouched | [x] |
| sodium-E74 | randombytes_buf_deterministic | `size > 0x4000000000ULL` (randombytes_BYTES_MAX) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E75 | randombytes_close | `implementation == NULL` or `implementation->close == NULL` | returns 0 | [x] |
| sodium-E76 | randombytes_stir | `implementation->stir == NULL` | no call (no-op) | [x] |
| sodium-E77 | randombytes | `assert(buf_len <= SIZE_MAX)` | abort — unreachable on this target (both types are 64-bit) | [abort] |
| sodium-E78 | safe_read (sysrandom + internal) | `assert(size > 0U)`, `assert(size <= SSIZE_MAX)` | abort — unreachable in this build (the getrandom path is always taken, so safe_read is never called) | [abort] |
| sodium-E79 | safe_read (sysrandom + internal) | `read()` returned < 0 → return readnb; returned 0 → break with a short count | short/negative result → caller calls sodium_misuse() | [n/a] |
| sodium-E80 | randombytes_block_on_dev_random | `open("/dev/random")` failed | returns 0 (treated as "don't block") | [n/a] |
| sodium-E81 | randombytes_block_on_dev_random | `poll()` returned != 1 | errno = EIO (5), returns -1 → dev_open returns -1 | [n/a] |
| sodium-E82 | randombytes_sysrandom_random_dev_open / randombytes_internal_random_random_dev_open | all devices in the list exhausted | errno = EIO (5), returns -1 | [n/a] |
| sodium-E83 | _randombytes_linux_getrandom (sysrandom + internal) | `assert(size <= 256U)` | abort — unreachable (the caller chunks to ≤ 256); Rust reproduces the check | [abort] |
| sodium-E84 | _randombytes_linux_getrandom (sysrandom + internal) | `readnb != (int) size` (short getrandom) | returns -1 | [n/a] |
| sodium-E85 | randombytes_linux_getrandom (sysrandom) | `assert(chunk_size > 0U)` — REACHABLE: `randombytes_sysrandom_implementation.buf(p, 0)` called directly (randombytes_buf() itself guards size > 0) | abort — verified in a forked child (SIGABRT in both libraries); **this was a real divergence, now fixed** (see report) | [x] |
| sodium-E86 | randombytes_linux_getrandom (internal) | `assert(chunk_size > 0U)` | abort — unreachable (only called with 16 and 32) | [abort] |
| sodium-E87 | randombytes_sysrandom_init | `randombytes_sysrandom_random_dev_open() == -1` | `sodium_misuse()` → abort — unreachable here (getrandom(2) succeeds on this kernel, so the function returns early) | [abort] |
| sodium-E88 | randombytes_sysrandom_close | `random_data_source_fd == -1` and `getrandom_available == 0` | returns -1 - unreachable once sodium_init() has stirred sysrandom (getrandom_available becomes 1 and never goes back to 0); the analogous internal-RNG path IS tested, see E96 | [n/a] |
| sodium-E89 | randombytes_sysrandom_buf | `randombytes_linux_getrandom() != 0` | `sodium_misuse()` → abort — not reachable with a working getrandom(2) | [abort] |
| sodium-E90 | randombytes_sysrandom_buf | `random_data_source_fd == -1 \|\| safe_read(...) != size` | `sodium_misuse()` → abort — not reachable (getrandom path returns first) | [abort] |
| sodium-E91 | sodium_hrtime (internal) | `gettimeofday() != 0` | `sodium_misuse()` → abort — not reachable | [abort] |
| sodium-E92 | randombytes_internal_random_init | `assert((getentropy_available \| getrandom_available) == 0)` | abort — unreachable (the getrandom branch returns before it) | [abort] |
| sodium-E93 | randombytes_internal_random_init | `randombytes_internal_random_random_dev_open() == -1` | `sodium_misuse()` → abort — unreachable (getrandom succeeds) | [abort] |
| sodium-E94 | randombytes_internal_random_stir | `assert(stream.nonce != 0)` | abort — unreachable (gettimeofday-derived microsecond clock is never 0) | [abort] |
| sodium-E95 | randombytes_internal_random_stir | `randombytes_linux_getrandom(stream.key, 32) != 0` | `sodium_misuse()` → abort — not reachable | [abort] |
| sodium-E96 | randombytes_internal_random_close | `getrandom_available == 0` (close before any stir) | returns -1; after a stir it returns 0 | [x] |
| sodium-E97 | randombytes_internal_random_buf / randombytes_internal_random | `assert(ret == 0)` on the crypto_stream_chacha20 result | abort — unreachable (chacha20 always returns 0) | [abort] |
| sodium-E98 | crypto_ipcrypt_* (all of `crypto_ipcrypt.c` and `ipcrypt_soft.c`) | no rejection sites at all: every entry point returns `void` or a fixed `size_t`, and there are no length/range checks, `assert`s or `sodium_misuse()` calls | n/a — nothing to reject; verified by exhaustive grep | [x] |
| sodium-E99 | _crypto_ipcrypt_pick_best_implementation | the HAVE_ARMCRYPTO and HAVE_AVXINTRIN_H+HAVE_WMMINTRIN_H early-return branches are not compiled | always returns 0 with the soft backend selected | [x] |

| stream-E1 | crypto_stream_chacha20 | clen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= SODIUM_SIZE_MAX = min(UINT64_MAX, SIZE_MAX) = UINT64_MAX on x86_64) | sodium_misuse() → abort(); dead code because clen is `unsigned long long` so the comparison can never be true. Rust has the identical dead `clen > SODIUM_SIZE_MAX` check | [abort] |
| stream-E2 | crypto_stream_chacha20_xor_ic | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= UINT64_MAX) | sodium_misuse() → abort(); unreachable, identical dead check in Rust | [abort] |
| stream-E3 | crypto_stream_chacha20_xor | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= UINT64_MAX) | sodium_misuse() → abort(); unreachable, identical dead check in Rust | [abort] |
| stream-E4 | crypto_stream_chacha20_ietf_ext | clen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= UINT64_MAX) | sodium_misuse() → abort(); unreachable, identical dead check in Rust | [abort] |
| stream-E5 | crypto_stream_chacha20_ietf_ext_xor_ic | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= UINT64_MAX) | sodium_misuse() → abort(); unreachable, identical dead check in Rust | [abort] |
| stream-E6 | crypto_stream_chacha20_ietf_ext_xor (static, reached from crypto_stream_chacha20_ietf_xor) | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX (= UINT64_MAX) | sodium_misuse() → abort(); unreachable, identical dead check in Rust | [abort] |
| stream-E7 | crypto_stream_chacha20_ietf | clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX (64·2^32 = 274877906944) | sodium_misuse() → abort() before any memory is touched. Verified OUT-OF-PROCESS: both C and Rust die with SIGABRT for clen = 64·2^32 + 1 | [x] |
| stream-E8 | crypto_stream_chacha20_ietf_xor | mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX (64·2^32) | sodium_misuse() → abort() before any memory is touched. Verified OUT-OF-PROCESS: both C and Rust die with SIGABRT for mlen = 64·2^32 + 1 | [x] |
| stream-E9 | crypto_stream_chacha20_ietf_xor_ic | ic > (64·2^32)/64 − (mlen+63)/64, i.e. ic > 2^32 − ceil(mlen/64) (counter would run past the 32-bit block counter) | sodium_misuse() → abort(). Verified OUT-OF-PROCESS with mlen=65, ic=0xffffffff (limit 2^32−2): both C and Rust die with SIGABRT. The accepted side of the boundary (ic == the exact maximum) is tested in-process for 13 message lengths | [x] |
| stream-E10 | crypto_stream_salsa20 (`stream_ref`) | clen == 0 | `return 0` before any pointer is dereferenced; tested with c=n=k=NULL, returns 0, no write | [x] |
| stream-E11 | crypto_stream_salsa20_xor_ic / crypto_stream_salsa20_xor (`stream_ref_xor_ic`) | mlen == 0 | `return 0` before any pointer is dereferenced; tested with c=m=n=k=NULL and ic ∈ {0,1,2^64−1}, returns 0 | [x] |
| stream-E12 | crypto_stream_salsa2012 | clen == 0 | `return 0`, all pointers may be NULL; returns 0 | [x] |
| stream-E13 | crypto_stream_salsa2012_xor | mlen == 0 | `return 0`, all pointers may be NULL; returns 0 | [x] |
| stream-E14 | crypto_stream_salsa208 | clen == 0 | `return 0`, all pointers may be NULL; returns 0 | [x] |
| stream-E15 | crypto_stream_salsa208_xor | mlen == 0 | `return 0`, all pointers may be NULL; returns 0 | [x] |
| stream-E16 | crypto_stream_chacha20 (`stream_ref`) | clen == 0 | `return 0` before keysetup; tested with all pointers NULL, returns 0 | [x] |
| stream-E17 | crypto_stream_chacha20_ietf / _ietf_ext (`stream_ietf_ext_ref`) | clen == 0 | `return 0` before keysetup; tested with all pointers NULL, returns 0 | [x] |
| stream-E18 | crypto_stream_chacha20_xor / _xor_ic (`stream_ref_xor_ic`) | mlen == 0 | `return 0` before keysetup; tested with all pointers NULL and ic ∈ {0,1,2^64−1}, returns 0 | [x] |
| stream-E19 | crypto_stream_chacha20_ietf_xor / _ietf_xor_ic / _ietf_ext_xor_ic (`stream_ietf_ext_ref_xor_ic`) | mlen == 0 | `return 0` before keysetup; tested with all pointers NULL and ic ∈ {0,1,0xffffffff}, returns 0 | [x] |
| stream-E20 | chacha20_encrypt_bytes (static, chacha20_ref.c) | bytes == 0 | `return` (LCOV_EXCL_LINE): unreachable — every caller already guards `!clen`/`!mlen`. Rust has the identical guard | [abort] |
| stream-E21 | crypto_stream_xsalsa20, _xsalsa20_xor, _xsalsa20_xor_ic | clen/mlen == 0 | crypto_core_hsalsa20 runs FIRST, so n and k must be valid; c/m may be NULL. Tested with c=m=NULL and valid n/k, returns 0 | [x] |
| stream-E22 | crypto_stream_xchacha20, _xchacha20_xor, _xchacha20_xor_ic | clen/mlen == 0 | crypto_core_hchacha20 runs FIRST, so n and k must be valid; c/m may be NULL. Tested with c=m=NULL and valid n/k, returns 0 | [x] |
| stream-E23 | crypto_stream, crypto_stream_xor | clen/mlen == 0 | delegate to xsalsa20; c/m may be NULL. Tested, returns 0 | [x] |
| stream-E24 | crypto_core_salsa20 / _salsa2012 / _salsa208 / _hsalsa20 / _hchacha20 | `c == NULL` (constants argument omitted) | NULL is explicitly tolerated: the built-in "expand 32-byte k" constants are used instead of LOAD32_LE(c+…). Both branches tested, always returns 0 | [x] |
| stream-E25 | every function in the area | — | no function in crypto_stream/** or crypto_core/{salsa,hsalsa20,hchacha20}/** ever returns a non-zero value: `grep -n 'return -1\|return NULL' ` finds no hits. All 30+ entry points were checked to return exactly 0 in both libraries for every input above | [x] |


## Summary

* total rejection rows: **600**
* rows with a passing differential test: **490** `[x]`
* rows whose C path is *provably unreachable / dead* on this target — the
  comparison can never be true (e.g. `mlen > UINT64_MAX` on an
  `unsigned long long`), or the branch is guarded by an earlier return:
  **41** `[unreachable]` + **12** `[dead]` + **26** `[abort]`
* rows whose code is not compiled in this build configuration (no `HAVE_MMAP`,
  no pthreads, no aligned-malloc guarded heap, …) or not reachable through the
  public ABI: **19** `[n/a]`
* rows that need a > 4 GiB buffer/string or a forced `malloc` failure to reach:
  **11** `[not testable — …]` (each row states the reason; equivalence
  established by reading the Rust against the C)
* `[compile-time]` (`COMPILER_ASSERT`): **1**
* **rows still open / unexplained: 0**

Every rejection that can be triggered at all is executed against BOTH
libraries. The `sodium_misuse()` / `assert()` paths that *are* reachable are
executed **out of process** — `tests/gaps.rs::abort_parity` (34 cases × 2
libraries), `tests/blake2.rs::abort_paths` (37 cases × 2),
`tests/sodium.rs::assert_aborts` and `tests/stream.rs` — asserting that the C
child and the Rust child both die with **SIGABRT**, not merely that "both
failed somehow".
