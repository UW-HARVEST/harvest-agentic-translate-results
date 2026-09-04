| box-1 | crypto_box_seedbytes, crypto_box_publickeybytes, crypto_box_secretkeybytes, crypto_box_beforenmbytes, crypto_box_noncebytes, crypto_box_zerobytes, crypto_box_boxzerobytes, crypto_box_macbytes, crypto_box_messagebytes_max, crypto_box_sealbytes, crypto_box_primitive | constant getters, no inputs (also checked against the header constants) | [x] |
| box-2 | crypto_box_curve25519xsalsa20poly1305_{seedbytes,publickeybytes,secretkeybytes,beforenmbytes,noncebytes,zerobytes,boxzerobytes,macbytes,messagebytes_max} | constant getters, no inputs | [x] |
| box-3 | crypto_box_curve25519xchacha20poly1305_{seedbytes,publickeybytes,secretkeybytes,beforenmbytes,noncebytes,macbytes,messagebytes_max,sealbytes} | constant getters, no inputs (no ZEROBYTES/BOXZEROBYTES: this primitive has no low-level API) | [x] |
| box-4 | crypto_kx_{publickeybytes,secretkeybytes,seedbytes,sessionkeybytes,primitive} | constant getters, no inputs | [x] |
| box-5 | crypto_kem_{publickeybytes,secretkeybytes,ciphertextbytes,sharedsecretbytes,seedbytes,primitive} | constant getters, no inputs | [x] |
| box-6 | crypto_kem_mlkem768_{publickeybytes,secretkeybytes,ciphertextbytes,sharedsecretbytes,seedbytes} | constant getters, no inputs | [x] |
| box-7 | crypto_kem_xwing_{publickeybytes,secretkeybytes,ciphertextbytes,sharedsecretbytes,seedbytes} | constant getters, no inputs | [x] |
| box-8 | crypto_box_seed_keypair | 24 seeds incl. all-0x00 and all-0xff, byte-exact pk+sk, canary-guarded buffers | [x] |
| box-9 | crypto_box_curve25519xsalsa20poly1305_seed_keypair | 24 seeds incl. all-0x00/all-0xff, byte-exact pk+sk | [x] |
| box-10 | crypto_box_curve25519xchacha20poly1305_seed_keypair | 24 seeds incl. all-0x00/all-0xff, byte-exact pk+sk | [x] |
| box-11 | crypto_box_keypair, crypto_box_curve25519xsalsa20poly1305_keypair, crypto_box_curve25519xchacha20poly1305_keypair | randombytes-driven: return code only, plus cross-library DH (C sk x Rust pk == Rust sk x C pk) x8 each | [x] |
| box-12 | crypto_box_beforenm, crypto_box_curve25519xsalsa20poly1305_beforenm, crypto_box_curve25519xchacha20poly1305_beforenm | 24 random seed-derived key pairs, byte-exact shared key, both directions agree | [x] |
| box-13 | crypto_box | zero-padded API, mlen = ZEROBYTES + {0,1,15,16,17,31,32,33,63,64,65,1000}, byte-exact c, c[0..BOXZEROBYTES] == 0, canary tail | [x] |
| box-14 | crypto_box_open | clen = ZEROBYTES + {0,1,15,16,17,31,32,33,63,64,65,1000}, byte-exact m, m[0..ZEROBYTES] == 0, round trip | [x] |
| box-15 | crypto_box_afternm, crypto_box_open_afternm | same size matrix as box-13/14 with k from crypto_box_beforenm; output identical to the full call | [x] |
| box-16 | crypto_box, crypto_box_open | in-place (c == m), mlen = ZEROBYTES + {0,1,17,64,1000} | [x] |
| box-17 | crypto_box_curve25519xsalsa20poly1305, crypto_box_curve25519xsalsa20poly1305_open, _afternm, _open_afternm | full named-primitive repeat of box-13..box-16 | [x] |
| box-18 | crypto_box_easy | mlen = 0,1,15,16,17,31,32,33,47,48,49,63,64,65,1000, byte-exact c (mlen+MACBYTES), canary tail | [x] |
| box-19 | crypto_box_open_easy | clen = mlen+MACBYTES for the same mlen set, byte-exact m, round trip | [x] |
| box-20 | crypto_box_easy_afternm, crypto_box_open_easy_afternm | same mlen set, k from beforenm, byte-identical to easy/open_easy | [x] |
| box-21 | crypto_box_detached, crypto_box_open_detached | same mlen set, separate 16-byte mac buffer, byte-exact c and mac, layout easy == mac‖c | [x] |
| box-22 | crypto_box_detached_afternm, crypto_box_open_detached_afternm | same mlen set with k from beforenm, byte-identical to detached/open_detached | [x] |
| box-23 | crypto_box_easy, crypto_box_open_easy | in-place: encrypt with m == c+MACBYTES, decrypt with m == c; mlen = 0,1,16,17,64,1000; identical to out-of-place | [x] |
| box-24 | crypto_box_detached, crypto_box_open_detached | in-place (c == m), all mlen of box-18 | [x] |
| box-25 | crypto_box_open_easy, crypto_box_open_easy_afternm, crypto_box_open_detached, crypto_box_open_detached_afternm | m == NULL (verify-only; C tolerates it), all mlen of box-18 | [x] |
| box-26 | crypto_box_curve25519xchacha20poly1305_easy, _open_easy, _easy_afternm, _open_easy_afternm, _detached, _open_detached, _detached_afternm, _open_detached_afternm | full xchacha20poly1305 repeat of box-18..box-25 | [x] |
| box-27 | crypto_box_easy + crypto_box_open_easy (and the xchacha20poly1305 equivalents) | 24 randomized cases, random mlen in 0..600, fresh seed-derived key pairs; C-encrypt -> Rust-decrypt and Rust-encrypt -> C-decrypt | [x] |
| box-28 | crypto_box_seal, crypto_box_seal_open | mlen = 0,1,15,16,17,63,64,65,1000; seal is nondeterministic (ephemeral keypair) so return code only, then both libraries open both blobs byte-exactly | [x] |
| box-29 | crypto_box_seal_open | clen == SEALBYTES (empty message) and m == NULL | [x] |
| box-30 | crypto_box_seal, crypto_box_seal_open | 24 randomized cases (random mlen in 0..300, fresh key pair): C-seal -> Rust-open and Rust-seal -> C-open | [x] |
| box-31 | crypto_box_curve25519xchacha20poly1305_seal, _seal_open | full repeat of box-28..box-30 | [x] |
| box-32 | crypto_kx_seed_keypair | 24 seeds incl. all-0x00/all-0xff, byte-exact pk+sk | [x] |
| box-33 | crypto_kx_keypair | randombytes-driven: return code, plus cross-library handshake (C client x Rust server) x8 | [x] |
| box-34 | crypto_kx_client_session_keys, crypto_kx_server_session_keys | rx != NULL and tx != NULL, 23 deterministic key-pair combinations, byte-exact rx/tx, client.rx == server.tx and client.tx == server.rx | [x] |
| box-35 | crypto_kx_client_session_keys | rx == NULL (C aliases rx := tx; the `tx[i]` store is last so tx holds the tx half) | [x] |
| box-36 | crypto_kx_client_session_keys | tx == NULL (C aliases tx := rx; same aliasing result) | [x] |
| box-37 | crypto_kx_server_session_keys | rx == NULL and tx == NULL separately (store order is reversed there, so the rx half wins) | [x] |
| box-38 | crypto_kx_client_session_keys | rx == tx (same non-NULL pointer passed twice) | [x] |
| box-39 | crypto_kem_mlkem768_seed_keypair | 16 seeds (64 bytes) incl. all-0x00/all-0xff; byte-exact pk (1184) and sk (2400); structural checks sk = skpv‖pk‖SHA3-256(pk)‖z and all 768 pk coefficients < q | [x] |
| box-40 | crypto_kem_mlkem768_keypair | randombytes-driven: return code, plus Rust-encapsulate against C pk -> C-decapsulate | [x] |
| box-41 | crypto_kem_mlkem768_enc_deterministic | 6 key pairs x 4 random 32-byte seeds + all-0x00/all-0xff seeds; byte-exact ct (1088) and ss (32), canary tails | [x] |
| box-42 | crypto_kem_mlkem768_enc | randombytes-driven: return code, then cross-library decapsulation both ways | [x] |
| box-43 | crypto_kem_mlkem768_dec | valid ciphertexts from box-41: return code 0, byte-exact ss, ss == encapsulated ss | [x] |
| box-44 | crypto_kem_mlkem768_dec | implicit-rejection path: a single bit flipped in EVERY one of the 1088 ciphertext bytes; C and Rust must produce the SAME pseudorandom ss and both return 0 | [x] |
| box-45 | crypto_kem_mlkem768_dec | 8 fully random ciphertexts, all-0x00 and all-0xff ciphertexts | [x] |
| box-46 | crypto_kem_mlkem768_dec | 6 fully random secret keys, and all-0x00/all-0xff secret keys x ct in {0x00,0xff,0x5a} (exercises indcpa_dec/indcpa_enc with arbitrary coefficients) | [x] |
| box-47 | crypto_kem_mlkem768_enc_deterministic | non-canonical polyvec encoding boundary: coefficient set to q-1 (3328, accepted), q (3329), q+1 (3330) and 4095 (rejected) at indices 0,1,2,255,256,511,512,766,767 | [x] |
| box-48 | crypto_kem_mlkem768_enc_deterministic | 8 fully random 1184-byte public keys (essentially always non-canonical), plus the all-zero public key (all coefficients 0, therefore canonical and accepted) | [x] |
| box-49 | crypto_kem_mlkem768_enc_deterministic | single-bit sweep over ALL 1184 public-key bytes: return code and outputs must agree; the trailing 32-byte matrix seed never triggers rejection | [x] |
| box-50 | _sodium_mlkem768_ref_seed_keypair, _sodium_mlkem768_ref_keypair, _sodium_mlkem768_ref_enc, _sodium_mlkem768_ref_enc_deterministic, _sodium_mlkem768_ref_dec | the complete box-39..box-49 matrix run directly against the internal `ref` symbols | [x] |
| box-51 | crypto_kem_mlkem768_{seed_keypair,enc_deterministic,dec} vs _sodium_mlkem768_ref_* | wrapper is a pure pass-through: identical outputs inside each library, 4 random seeds | [x] |
| box-52 | crypto_kem_xwing_seed_keypair | 16 seeds (32 bytes) incl. all-0x00/all-0xff; byte-exact pk (1216) and sk (32); sk is literally the seed | [x] |
| box-53 | crypto_kem_xwing_keypair | randombytes-driven: return code, cross-library round trip, and the generated sk re-expands to the same pk | [x] |
| box-54 | crypto_kem_xwing_enc_deterministic | 6 key pairs x 4 random 64-byte seeds + all-0x00/all-0xff seeds; byte-exact ct (1120) and ss (32) | [x] |
| box-55 | crypto_kem_xwing_enc | randombytes-driven: return code, then cross-library decapsulation both ways | [x] |
| box-56 | crypto_kem_xwing_dec | valid ciphertexts: byte-exact ss, ss == encapsulated ss | [x] |
| box-57 | crypto_kem_xwing_dec | a single bit flipped in EVERY one of the 1120 ciphertext bytes (ML-KEM part -> implicit rejection, X25519 part -> different but valid ss); byte-exact ss | [x] |
| box-58 | crypto_kem_xwing_dec | 8 random (ct, sk) pairs, all-0x00/all-0xff ciphertexts, and a valid ciphertext with the wrong secret key | [x] |
| box-59 | crypto_kem_seed_keypair, crypto_kem_keypair, crypto_kem_enc, crypto_kem_dec | generic dispatch: the complete box-52..box-58 matrix, plus byte-equality with crypto_kem_xwing_* inside each library | [x] |
