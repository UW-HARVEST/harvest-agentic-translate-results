| aead1-1 | crypto_aead_chacha20poly1305_encrypt_detached | 8-byte nonce; ad=NULL & adlen=0; mlen = 0,1,15,16,17,63,64,65,1000; maclen_p=NULL | [x] |
| aead1-2 | crypto_aead_chacha20poly1305_encrypt_detached | ad=NULL & adlen=0; maclen_p != NULL (must be set to 16) | [x] |
| aead1-3 | crypto_aead_chacha20poly1305_encrypt_detached | ad != NULL with adlen = 0 (pointer given but zero length) | [x] |
| aead1-4 | crypto_aead_chacha20poly1305_encrypt_detached | adlen = 1, 16, and random 1..100 (non-block-aligned ad) | [x] |
| aead1-5 | crypto_aead_chacha20poly1305_encrypt_detached | nsec = NULL vs nsec != NULL (must be ignored, buffer untouched) | [x] |
| aead1-6 | crypto_aead_chacha20poly1305_encrypt | clen_p = NULL vs != NULL (= mlen+16); nsec NULL/non-NULL; all mlen | [x] |
| aead1-7 | crypto_aead_chacha20poly1305_encrypt | in-place (c == m), all mlen; result must equal out-of-place | [x] |
| aead1-8 | crypto_aead_chacha20poly1305_decrypt_detached | m != NULL, valid mac, nsec NULL/non-NULL, all mlen, all ad shapes | [x] |
| aead1-9 | crypto_aead_chacha20poly1305_decrypt_detached | m == NULL (verify-only path, returns crypto_verify_16 result) | [x] |
| aead1-10 | crypto_aead_chacha20poly1305_decrypt | mlen_p = NULL vs != NULL; nsec NULL/non-NULL; clen = mlen+16 | [x] |
| aead1-11 | crypto_aead_chacha20poly1305_{keybytes,npubbytes,nsecbytes,abytes,messagebytes_max} | constant getters (32/8/0/16/SIZE_MAX-16) | [x] |
| aead1-12 | crypto_aead_chacha20poly1305_keygen | 32-byte key, deterministic randombytes implementation installed in both libs | [x] |
| aead1-13 | crypto_aead_chacha20poly1305_ietf_encrypt_detached | 12-byte nonce; ad=NULL & adlen=0; mlen = 0,1,15,16,17,63,64,65,1000; maclen_p=NULL | [x] |
| aead1-14 | crypto_aead_chacha20poly1305_ietf_encrypt_detached | maclen_p != NULL; ad != NULL & adlen=0; adlen=1/16/random (exercises `(0x10-adlen)&0xf` pad) | [x] |
| aead1-15 | crypto_aead_chacha20poly1305_ietf_encrypt_detached | nsec = NULL vs nsec != NULL (ignored) | [x] |
| aead1-16 | crypto_aead_chacha20poly1305_ietf_encrypt | clen_p NULL/non-NULL; nsec NULL/non-NULL; in-place (c == m) | [x] |
| aead1-17 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | m != NULL / m == NULL; nsec NULL/non-NULL; all mlen and ad shapes | [x] |
| aead1-18 | crypto_aead_chacha20poly1305_ietf_decrypt | mlen_p NULL/non-NULL; nsec NULL/non-NULL | [x] |
| aead1-19 | crypto_aead_chacha20poly1305_ietf_{keybytes,npubbytes,nsecbytes,abytes,messagebytes_max}, ietf_keygen | constant getters (32/12/0/16/64*(2^32-1)) + keygen | [x] |
| aead1-20 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached | 24-byte nonce (hchacha20 subkey + npub2); ad=NULL & adlen=0; mlen = 0,1,15,16,17,63,64,65,1000 | [x] |
| aead1-21 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached | maclen_p NULL/non-NULL; ad != NULL & adlen=0; adlen = 1/16/random | [x] |
| aead1-22 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached | nsec = NULL vs nsec != NULL (ignored) | [x] |
| aead1-23 | crypto_aead_xchacha20poly1305_ietf_encrypt | clen_p NULL/non-NULL; nsec NULL/non-NULL; in-place (c == m) | [x] |
| aead1-24 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | m != NULL / m == NULL; nsec NULL/non-NULL; all mlen and ad shapes | [x] |
| aead1-25 | crypto_aead_xchacha20poly1305_ietf_decrypt | mlen_p NULL/non-NULL; nsec NULL/non-NULL | [x] |
| aead1-26 | crypto_aead_xchacha20poly1305_ietf_{keybytes,npubbytes,nsecbytes,abytes,messagebytes_max}, ietf_keygen | constant getters (32/24/0/16/SIZE_MAX-16) + keygen | [x] |
| aead1-27 | crypto_secretbox_xsalsa20poly1305 | zero-padded NaCl API: m[0..ZEROBYTES]=0, mlen = 32+{0,1,15,16,17,31,32,33,63,64,65,1000}; c[0..BOXZEROBYTES] must be zeroed | [x] |
| aead1-28 | crypto_secretbox_xsalsa20poly1305 | non-zero ZEROBYTES prefix (C performs no check; seals but never opens) | [x] |
| aead1-29 | crypto_secretbox_xsalsa20poly1305 | in-place (c == m) | [x] |
| aead1-30 | crypto_secretbox_xsalsa20poly1305_open | valid box, all clen; m[0..ZEROBYTES] must be zeroed; c[0..BOXZEROBYTES] ignored by verifier | [x] |
| aead1-31 | crypto_secretbox_xsalsa20poly1305_open | in-place (m == c) | [x] |
| aead1-32 | crypto_secretbox, crypto_secretbox_open | generic wrappers over the xsalsa20poly1305 pair, same full matrix | [x] |
| aead1-33 | crypto_secretbox_{keybytes,noncebytes,zerobytes,boxzerobytes,macbytes,messagebytes_max,primitive}, crypto_secretbox_keygen | constant getters + primitive string "xsalsa20poly1305" + keygen | [x] |
| aead1-34 | crypto_secretbox_xsalsa20poly1305_{keybytes,noncebytes,zerobytes,boxzerobytes,macbytes,messagebytes_max}, _keygen | constant getters + keygen | [x] |
| aead1-35 | crypto_secretbox_detached | mlen = 0,1,15,16,17,31,32,33,63,64,65,1000 (straddles the 64-ZEROBYTES=32 block0 clamp) | [x] |
| aead1-36 | crypto_secretbox_detached | overlapping m/c: c = base+0 & m = base+off, and c = base+off & m = base+0, off = 1,3,8,16 < mlen (memmove branch) | [x] |
| aead1-37 | crypto_secretbox_easy | c = mac||ct layout, all mlen; equals detached output | [x] |
| aead1-38 | crypto_secretbox_easy | in-place: easy(buf, buf+MACBYTES, mlen) | [x] |
| aead1-39 | crypto_secretbox_open_detached | m != NULL, valid mac, all clen (incl. 0) | [x] |
| aead1-40 | crypto_secretbox_open_detached | m == NULL (verify-only, returns 0) | [x] |
| aead1-41 | crypto_secretbox_open_detached | overlapping m/c (memmove branch), both directions, off = 1,3,8,16 | [x] |
| aead1-42 | crypto_secretbox_open_easy | clen = mlen+MACBYTES, all mlen; in-place open_easy(buf, buf, clen) | [x] |
| aead1-43 | crypto_secretbox_xchacha20poly1305_detached | mlen = 0,1,15,16,17,31,32,33,63,64,65,1000 (block0 stream length is mlen0+32 here, not 64) | [x] |
| aead1-44 | crypto_secretbox_xchacha20poly1305_detached | overlapping m/c (memmove branch), both directions, off = 1,3,8,16 | [x] |
| aead1-45 | crypto_secretbox_xchacha20poly1305_easy | all mlen; in-place easy(buf, buf+MACBYTES, mlen) | [x] |
| aead1-46 | crypto_secretbox_xchacha20poly1305_open_detached | m != NULL / m == NULL; overlapping m/c (memmove branch) | [x] |
| aead1-47 | crypto_secretbox_xchacha20poly1305_open_easy | all clen; in-place open_easy(buf, buf, clen) | [x] |
| aead1-48 | crypto_secretbox_xchacha20poly1305_{keybytes,noncebytes,macbytes,messagebytes_max} | constant getters (32/24/16/SIZE_MAX-16) | [x] |
| aead1-49 | crypto_secretstream_xchacha20poly1305_statebytes | 52 bytes; full state buffer compared byte-for-byte everywhere else | [x] |
| aead1-50 | crypto_secretstream_xchacha20poly1305_init_pull | random header+key (40 cases) + all-zero and all-0xff header/key; full state compared | [x] |
| aead1-51 | crypto_secretstream_xchacha20poly1305_init_push | deterministic randombytes: header AND state compared; cross-checked against init_pull of the same header | [x] |
| aead1-52 | crypto_secretstream_xchacha20poly1305_push | TAG_MESSAGE(0x00); mlen = 0,1,15,16,17,47,63,64,65,1000; 4-message session, ciphertext + state compared after every push | [x] |
| aead1-53 | crypto_secretstream_xchacha20poly1305_push | TAG_PUSH(0x01) | [x] |
| aead1-54 | crypto_secretstream_xchacha20poly1305_push | TAG_REKEY(0x02) — takes the implicit-rekey branch | [x] |
| aead1-55 | crypto_secretstream_xchacha20poly1305_push | TAG_FINAL(0x03) — takes the implicit-rekey branch | [x] |
| aead1-56 | crypto_secretstream_xchacha20poly1305_push | out-of-range tag bytes 0x04, 0x7f, 0x80, 0xff (C takes any unsigned char; 0x80/0xff also set the REKEY bit) | [x] |
| aead1-57 | crypto_secretstream_xchacha20poly1305_push | ad = NULL & adlen = 0; ad != NULL & adlen = 0; adlen = 1; adlen random 1..40 | [x] |
| aead1-58 | crypto_secretstream_xchacha20poly1305_push | outlen_p = NULL vs != NULL (= ABYTES + mlen) | [x] |
| aead1-59 | crypto_secretstream_xchacha20poly1305_pull | full session, m/mlen_p/tag_p compared, state compared after every pull; tags as in aead1-52..56 | [x] |
| aead1-60 | crypto_secretstream_xchacha20poly1305_pull | mlen_p = NULL and tag_p = NULL vs both non-NULL | [x] |
| aead1-61 | crypto_secretstream_xchacha20poly1305_rekey | explicit rekey, 4 consecutive times on a session state, plus crafted all-0x00 / all-0xff / all-0x5a states | [x] |
| aead1-62 | crypto_secretstream_xchacha20poly1305_push, _pull | crafted counter = 0xffffffff / 0xfffffffe / 0x00000000 / 0x00000001 x tag 0x00..0x03 x mlen 0,1,17,64 — exercises the `sodium_is_zero(counter)` wrap-rekey branch | [x] |
| aead1-63 | crypto_secretstream_xchacha20poly1305_{abytes,headerbytes,keybytes,messagebytes_max}, _tag_{message,push,rekey,final} | constant getters (17/24/32/64*(2^32-2)) and tag getters (0x00/0x01/0x02/0x03) | [x] |
| aead1-64 | crypto_secretstream_xchacha20poly1305_keygen | 32-byte key, deterministic randombytes implementation installed in both libs | [x] |
