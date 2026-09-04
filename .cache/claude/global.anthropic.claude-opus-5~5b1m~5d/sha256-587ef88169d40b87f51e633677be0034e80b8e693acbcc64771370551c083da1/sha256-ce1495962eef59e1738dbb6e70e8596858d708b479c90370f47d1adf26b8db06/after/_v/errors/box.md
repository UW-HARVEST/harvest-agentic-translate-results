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
