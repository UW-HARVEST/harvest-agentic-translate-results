| G4 | crypto_aead_aegis128l_keygen | fills 16-byte key from randombytes_buf; check length written and that two calls differ | [ ] |
| G4 | crypto_aead_aegis128l_keybytes / _nsecbytes / _npubbytes / _abytes / _messagebytes_max | constant getters; expect 16 / 0 / 16 / 32 / SODIUM_MIN(SIZE_MAX-32, 2^61-1) | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | ad == NULL, adlen == 0, nsec == NULL, clen_p non-NULL; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000 (RATE=32 -> 32/64 are rate multiples, 31/33/63/65 straddle) | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | ad non-NULL but adlen == 0 (pointer supplied, zero length) — must match ad==NULL result byte-for-byte | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 1, mlen sweep 0/16/32/33/1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 15 (sub-AES-block), mlen sweep 0/16/32/1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 16 (one AES block, half a RATE), mlen sweep 0/16/32/1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 17, mlen sweep 0/16/32/1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 31 / 32 / 33 (RATE-1 / RATE / RATE+1 -> exercises absorb + partial-block memset/memcpy) | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 63 / 64 / 65 (2*RATE boundary -> exercises the absorb2 double-rate loop) | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | adlen = 1000 (large, mixed absorb2 + absorb + tail) | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | clen_p == NULL (caller ignores output length) with mlen 0 and 1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt | in-place: c == m buffer with ABYTES tail room, mlen 0/32/33/1000 | [ ] |
| G4 | crypto_aead_aegis128l_decrypt | round-trip of every encrypt config above; clen = mlen+32, matching ad, mlen_p non-NULL | [ ] |
| G4 | crypto_aead_aegis128l_decrypt | clen == ABYTES exactly (32, empty message); expect *mlen_p == 0 and rc 0 | [ ] |
| G4 | crypto_aead_aegis128l_decrypt | mlen_p == NULL | [ ] |
| G4 | crypto_aead_aegis128l_decrypt | in-place: m == c | [ ] |
| G4 | crypto_aead_aegis128l_encrypt_detached | maclen_p non-NULL (expect 32), ad == NULL/adlen 0, mlen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt_detached | maclen_p == NULL | [ ] |
| G4 | crypto_aead_aegis128l_encrypt_detached | ad non-NULL, adlen sweep 1/15/16/17/31/32/33/64/1000, mlen 0/33/1000 | [ ] |
| G4 | crypto_aead_aegis128l_encrypt_detached | separate `mac` buffer vs `c + mlen`: verify detached mac == last 32 bytes of the combined ciphertext | [ ] |
| G4 | crypto_aead_aegis128l_decrypt_detached | m non-NULL, valid 32-byte mac, clen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000, adlen sweep 0/1/16/17/1000 | [ ] |
| G4 | crypto_aead_aegis128l_decrypt_detached | m == NULL (verify-only mode; declast writes into the scratch `dst` instead) | [ ] |
| G4 | crypto_aead_aegis128l_decrypt_detached | clen == 0 with ad non-empty (mac-over-ad-only) | [ ] |
| G4 | crypto_aead_aegis256_keygen | fills 32-byte key | [ ] |
| G4 | crypto_aead_aegis256_keybytes / _nsecbytes / _npubbytes / _abytes / _messagebytes_max | expect 32 / 0 / 32 / 32 / SODIUM_MIN(SIZE_MAX-32, 2^61-1) | [ ] |
| G4 | crypto_aead_aegis256_encrypt | ad == NULL, adlen == 0, nsec == NULL; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000 (RATE=16 -> 16/32/64 are rate multiples) | [ ] |
| G4 | crypto_aead_aegis256_encrypt | ad non-NULL, adlen == 0 — must equal the ad==NULL result | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 1, mlen sweep 0/16/17/1000 | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 15 (RATE-1), mlen sweep 0/16/1000 | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 16 (exactly RATE), mlen sweep 0/16/1000 | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 17 (RATE+1 -> absorb + partial tail) | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 31 / 32 / 33 (2*RATE boundary -> absorb2 loop) | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 63 / 64 / 65 | [ ] |
| G4 | crypto_aead_aegis256_encrypt | adlen = 1000 (large) | [ ] |
| G4 | crypto_aead_aegis256_encrypt | clen_p == NULL | [ ] |
| G4 | crypto_aead_aegis256_encrypt | in-place c == m with ABYTES tail room | [ ] |
| G4 | crypto_aead_aegis256_decrypt | round-trip of all the above; clen = mlen+32, matching ad | [ ] |
| G4 | crypto_aead_aegis256_decrypt | clen == 32 exactly (empty message) | [ ] |
| G4 | crypto_aead_aegis256_decrypt | mlen_p == NULL | [ ] |
| G4 | crypto_aead_aegis256_decrypt | in-place m == c | [ ] |
| G4 | crypto_aead_aegis256_encrypt_detached | maclen_p non-NULL (expect 32) / NULL; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000 | [ ] |
| G4 | crypto_aead_aegis256_encrypt_detached | ad non-NULL, adlen sweep 1/15/16/17/32/33/64/1000 | [ ] |
| G4 | crypto_aead_aegis256_decrypt_detached | m non-NULL, clen sweep 0,1,15,16,17,32,33,64,65,128,1000, adlen sweep 0/1/16/17/1000 | [ ] |
| G4 | crypto_aead_aegis256_decrypt_detached | m == NULL (verify-only) | [ ] |
| G4 | crypto_aead_aes256gcm_is_available | portable build (no HAVE_TMMINTRIN_H/HAVE_WMMINTRIN_H, no HAVE_ARMCRYPTO): expect 0 | [ ] |
| G4 | crypto_aead_aes256gcm_keybytes / _nsecbytes / _npubbytes / _abytes / _statebytes / _messagebytes_max | getters are ALWAYS compiled in (outside the availability #if): expect 32 / 0 / 12 / 16 / (sizeof(state)+15)&~15 = 512 / SODIUM_MIN(SIZE_MAX-16, 16*(2^32-2)) | [ ] |
| G4 | crypto_aead_aes256gcm_keygen | always compiled in; fills 32-byte key even though the cipher is unavailable | [ ] |
| G4 | crypto_aead_aes256gcm_encrypt | valid 32-byte k, 12-byte npub, nsec NULL, ad NULL/non-NULL, mlen sweep 0/1/15/16/17/63/64/65/128/1000 — in THIS build expect rc -1 with errno == ENOSYS for every shape, clen_p untouched | [ ] |
| G4 | crypto_aead_aes256gcm_encrypt_detached | same input sweep; expect -1/ENOSYS, maclen_p untouched | [ ] |
| G4 | crypto_aead_aes256gcm_decrypt | clen sweep 0/16/17/32/1016; expect -1/ENOSYS (no ABYTES pre-check reached, mlen_p untouched) | [ ] |
| G4 | crypto_aead_aes256gcm_decrypt_detached | m non-NULL and m == NULL; expect -1/ENOSYS | [ ] |
| G4 | crypto_aead_aes256gcm_beforenm | valid 32-byte key into a CRYPTO_ALIGN(16) crypto_aead_aes256gcm_state; expect -1/ENOSYS and state NOT initialized (callers must not proceed) | [ ] |
| G4 | crypto_aead_aes256gcm_encrypt_afternm | precomputed-state combined encrypt, st from beforenm, nsec NULL, ad NULL / adlen 0/1/16/17/1000, mlen sweep 0/1/16/17/64/65/1000; expect -1/ENOSYS | [ ] |
| G4 | crypto_aead_aes256gcm_encrypt_detached_afternm | precomputed-state detached encrypt, maclen_p NULL and non-NULL, same ad/mlen sweep; expect -1/ENOSYS | [ ] |
| G4 | crypto_aead_aes256gcm_decrypt_afternm | precomputed-state combined decrypt, clen sweep 0/15/16/17/1016, mlen_p NULL and non-NULL; expect -1/ENOSYS | [ ] |
| G4 | crypto_aead_aes256gcm_decrypt_detached_afternm | precomputed-state detached decrypt, m non-NULL and m == NULL (verify-only), ad sweep; expect -1/ENOSYS | [ ] |
| G4 | crypto_aead_aes256gcm_* (whole family) | reuse one `beforenm` state across many afternm calls (the intended amortization pattern) — behaviour must be identical to the non-afternm entry points; in this build all legs are -1/ENOSYS | [ ] |
| G4 | crypto_aead_chacha20poly1305_keygen | fills 32-byte key | [ ] |
| G4 | crypto_aead_chacha20poly1305_keybytes / _nsecbytes / _npubbytes / _abytes / _messagebytes_max | expect 32 / 0 / 8 / 16 / SIZE_MAX-16 | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | 8-byte npub, nsec NULL, ad == NULL, adlen == 0, clen_p non-NULL; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000 (64-byte ChaCha block boundaries) | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | ad non-NULL, adlen == 0 — must equal ad==NULL result (original construction appends LE64(adlen) with no 16-byte padding) | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | adlen = 1 / 15 / 16 / 17 (no Poly1305 zero-padding in the original construction, unlike ietf) | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | adlen = 63 / 64 / 65 and 1000 (large) | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | clen_p == NULL | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | in-place c == m with 16-byte tail room | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt | mlen > STREAM_POLY1305_CHUNK (131072): e.g. 131072, 131073, 262144 — exercises the multi-chunk loop and the `ic += cl/64` counter advance | [ ] |
| G4 | crypto_aead_chacha20poly1305_decrypt | round-trip; clen = mlen+16, matching ad, mlen_p non-NULL and NULL | [ ] |
| G4 | crypto_aead_chacha20poly1305_decrypt | clen == 16 exactly (empty message) | [ ] |
| G4 | crypto_aead_chacha20poly1305_decrypt | in-place m == c | [ ] |
| G4 | crypto_aead_chacha20poly1305_encrypt_detached | maclen_p non-NULL (expect 16) and NULL, mac in a separate buffer; mlen sweep 0,1,15,16,17,63,64,65,127,128,129,1000; adlen sweep 0/1/15/16/17/1000 | [ ] |
| G4 | crypto_aead_chacha20poly1305_decrypt_detached | m non-NULL, valid mac, clen sweep 0,1,15,16,17,63,64,65,128,1000; adlen sweep 0/1/16/17/1000 | [ ] |
| G4 | crypto_aead_chacha20poly1305_decrypt_detached | m == NULL (verify-only; returns the crypto_verify_16 result directly) | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_keygen | fills 32-byte key | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_keybytes / _nsecbytes / _npubbytes / _abytes / _messagebytes_max | expect 32 / 0 / 12 / 16 / SODIUM_MIN(SIZE_MAX-16, 64*(2^32-1)) | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | 12-byte npub, nsec NULL, ad == NULL, adlen == 0; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000 | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | ad non-NULL, adlen == 0 (`(0x10-0)&0xf == 0`, no pad emitted) | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | adlen = 1 (15 pad bytes), 15 (1 pad byte), 16 (0 pad bytes), 17 (15 pad bytes) — the `_pad0` alignment path | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | mlen = 15/16/17 combined with adlen 15/16/17 — both the ad pad and the message pad `(0x10-mlen)&0xf` are exercised | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | adlen = 1000 (large) | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | clen_p == NULL; in-place c == m with 16-byte tail room | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | mlen crossing STREAM_POLY1305_CHUNK (131072 / 131073 / 262144) — 32-bit `ic` chunk advance | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt | round-trip of all the above; mlen_p non-NULL and NULL; clen == 16 (empty message); in-place m == c | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt_detached | maclen_p non-NULL (16) and NULL; mlen sweep 0,1,15,16,17,63,64,65,127,128,129,1000; adlen sweep 0/1/15/16/17/1000 | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | m non-NULL, clen sweep 0,1,15,16,17,63,64,65,128,1000; adlen sweep 0/1/16/17/1000 | [ ] |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | m == NULL (verify-only) | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_keygen | fills 32-byte key | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_keybytes / _nsecbytes / _npubbytes / _abytes / _messagebytes_max | expect 32 / 0 / 24 / 16 / SIZE_MAX-16 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | 24-byte npub (hchacha20 over npub[0..16), then npub2 = 4 zero bytes ++ npub[16..24)), nsec NULL, ad == NULL/adlen 0; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | ad non-NULL, adlen == 0 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | adlen = 1 / 15 / 16 / 17 (Poly1305 `_pad0` alignment) | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | adlen = 63 / 64 / 65 and 1000 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | clen_p == NULL; in-place c == m with tail room | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | mlen crossing STREAM_POLY1305_CHUNK (131072/131073/262144) — the internal `_encrypt_detached` uses the ietf_ext stream, and switches to a single pass when mlen > 64*(2^32-2) | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt | round-trip; clen = mlen+16; mlen_p non-NULL and NULL; clen == 16 (empty); in-place m == c | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached | maclen_p non-NULL (16) and NULL; mlen sweep 0,1,15,16,17,63,64,65,128,1000; adlen sweep 0/1/15/16/17/1000 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | m non-NULL, clen sweep 0,1,15,16,17,63,64,65,128,1000; adlen sweep 0/1/16/17/1000 | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | m == NULL (verify-only) | [ ] |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt vs crypto_aead_chacha20poly1305_ietf_encrypt | equivalence check: hchacha20(npub[0..16), k) as the ietf key and (0,0,0,0 ++ npub[16..24)) as the ietf nonce must reproduce the xchacha20 ciphertext | [ ] |
| G4 | crypto_secretbox_keygen | fills 32-byte key | [ ] |
| G4 | crypto_secretbox_keybytes / _noncebytes / _macbytes / _zerobytes / _boxzerobytes / _messagebytes_max / _primitive | expect 32 / 24 / 16 / 32 / 16 / SIZE_MAX-16 / "xsalsa20poly1305" | [ ] |
| G4 | crypto_secretbox_easy | 24-byte nonce, 32-byte key; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,1000; output is 16+mlen | [ ] |
| G4 | crypto_secretbox_easy | in-place: c == m - MACBYTES layout (the documented overlap; hits the `memmove(c, m, mlen)` fixup in _detached), mlen 0/32/33/1000 | [ ] |
| G4 | crypto_secretbox_easy | mlen crossing STREAM_POLY1305_CHUNK (131072/131073) — the `ic += cl/64` salsa20 chunk loop | [ ] |
| G4 | crypto_secretbox_open_easy | round-trip of every _easy config; clen = mlen+16; clen == 16 exactly (empty message) | [ ] |
| G4 | crypto_secretbox_open_easy | in-place m == c overlap | [ ] |
| G4 | crypto_secretbox_detached | separate mac buffer, mlen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000; verify mac == first 16 bytes of the _easy output | [ ] |
| G4 | crypto_secretbox_detached | mlen crossing 32 (= 64 - ZEROBYTES, the `mlen0` cap) — mlen 31/32/33 is the block0-vs-chunk-loop split point | [ ] |
| G4 | crypto_secretbox_open_detached | m non-NULL, valid mac, clen sweep 0,1,15,16,17,31,32,33,63,64,65,128,1000 | [ ] |
| G4 | crypto_secretbox_open_detached | m == NULL (verify-only mode, returns 0 early) | [ ] |
| G4 | crypto_secretbox / crypto_secretbox_xsalsa20poly1305 (LOW-LEVEL NaCl zero-padded form) | m has crypto_secretbox_ZEROBYTES = 32 leading zero bytes; total mlen sweep 32,33,47,48,49,63,64,65,96,97,1032 (= 32 + payload 0,1,15,16,17,31,32,33,64,65,1000); output c has 16 leading zero bytes then mac then ciphertext | [ ] |
| G4 | crypto_secretbox_open / crypto_secretbox_xsalsa20poly1305_open (LOW-LEVEL NaCl form) | c has crypto_secretbox_BOXZEROBYTES = 16 leading zero bytes; clen sweep 32,33,47,48,49,64,65,96,1032; m gets 32 zeroed leading bytes | [ ] |
| G4 | crypto_secretbox / crypto_secretbox_open (LOW-LEVEL) | round-trip pair, and cross-check that `crypto_secretbox()` output[16..32) equals the `crypto_secretbox_easy()` mac and output[32..) equals its ciphertext | [ ] |
| G4 | crypto_secretbox_xsalsa20poly1305_keygen / _keybytes / _noncebytes / _zerobytes / _boxzerobytes / _macbytes / _messagebytes_max | getters + keygen (note _zerobytes/_boxzerobytes are marked deprecated) | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_keybytes / _noncebytes / _macbytes / _messagebytes_max | expect 32 / 24 / 16 / SIZE_MAX-16 (no keygen, no zerobytes/boxzerobytes in this variant) | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_easy | 24-byte nonce (hchacha20 over n[0..16), stream nonce n+16); mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,1000 | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_easy | mlen 31/32/33 straddling the `mlen0 = min(mlen, 64-32)` split (block0 XOR path vs `crypto_stream_chacha20_xor_ic(..., ic=1)` tail) | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_easy | in-place c == m - MACBYTES (memmove overlap fixup) | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_open_easy | round-trip; clen = mlen+16; clen == 16 exactly; in-place m == c | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_detached | separate mac buffer, mlen sweep 0,1,15,16,17,31,32,33,64,65,1000; mac must equal the _easy prefix | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_open_detached | m non-NULL, clen sweep 0,1,15,16,17,31,32,33,64,65,1000 | [ ] |
| G4 | crypto_secretbox_xchacha20poly1305_open_detached | m == NULL (verify-only, early return 0) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_keygen | fills 32-byte key | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_statebytes / _abytes / _headerbytes / _keybytes / _messagebytes_max | expect sizeof(state) / 17 / 24 / 32 / SODIUM_MIN(SIZE_MAX-17, 64*(2^32-2)) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_tag_message / _tag_push / _tag_rekey / _tag_final | expect 0x00 / 0x01 / 0x02 / 0x03 | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_init_push | fresh state + 32-byte key; emits a random 24-byte header, derives state->k = hchacha20(header[0..16), k), counter = 1, inonce = header[16..24), `_pad` zeroed | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_init_pull | the header produced by init_push; must reproduce the identical state (deterministic given the header) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_init_pull | an arbitrary caller-chosen 24-byte header (all-zero, all-0xff) — accepted unconditionally | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | tag = TAG_MESSAGE (0), ad == NULL, adlen == 0, outlen_p non-NULL; mlen sweep 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000; out length must be 17+mlen | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | tag = TAG_MESSAGE, ad non-NULL, adlen sweep 1/15/16/17/64/1000 (Poly1305 `_pad0` alignment on ad) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | tag = TAG_PUSH (1) — no rekey side effect; mlen 0/17/64/1000, with and without ad | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | tag = TAG_REKEY (2) — implicit rekey side effect (state->k and inonce replaced, counter reset to 1); assert the NEXT push differs from the no-rekey stream | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | tag = TAG_FINAL (3 = PUSH\|REKEY) — also triggers the implicit rekey via the REKEY bit | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | out-of-range tag byte 0x04 / 0x7f / 0x80 / 0xfe / 0xff — accepted verbatim (no validation); bit 0x02 determines whether a rekey happens (0x04 no, 0x82 yes) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | outlen_p == NULL | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | in-place: out == m - 1 (the documented 1-byte-offset overlap) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | mlen crossing 64 (the tag block is counter 1, message starts at counter 2) — mlen 63/64/65 and the odd `(0x10 - 64 + mlen) & 0xf` padding quirk | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_pull | round-trip of every push config; m non-NULL, mlen_p and tag_p non-NULL; inlen = 17+mlen; assert recovered tag byte matches | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_pull | inlen == 17 exactly (empty message, tag only) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_pull | mlen_p == NULL and/or tag_p == NULL | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_pull | ad supplied on pull matching the push ad; adlen sweep 0/1/15/16/17/1000 | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_pull | in-place: m == in + 1 | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push + _pull | multi-message sequence: 5..10 messages with mixed tags (MESSAGE, MESSAGE, PUSH, MESSAGE, REKEY, MESSAGE, FINAL) and mixed mlen/ad; assert the inonce XOR-with-mac chaining and the 4-byte counter increment keep both sides in lockstep | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_rekey | explicit rekey called on the push side after N messages and on the pull side at the same point; assert the streams stay in sync and the derived key/inonce match | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_rekey | explicit rekey called immediately after init_push (before any message) and immediately after an implicit REKEY-tag rekey (double rekey) | [ ] |
| G4 | crypto_secretstream_xchacha20poly1305_push | long run to exercise the 4-byte counter: push enough messages that `sodium_increment(counter,4)` advances the low bytes (e.g. 300+ messages) — carry propagation across the byte boundary | [ ] |
| G4 | crypto_stream_chacha20_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 8 / SODIUM_SIZE_MAX | [ ] |
| G4 | crypto_stream_chacha20 (keystream form `(c, clen, n, k)`) | 8-byte nonce; clen sweep 0,1,63,64,65,127,128,129,1000 (0 short-circuits to rc 0 with c untouched) | [ ] |
| G4 | crypto_stream_chacha20_xor | 8-byte nonce, ic implicitly 0; mlen sweep 0,1,63,64,65,127,128,129,1000; must equal keystream XOR m | [ ] |
| G4 | crypto_stream_chacha20_xor | in-place c == m | [ ] |
| G4 | crypto_stream_chacha20_xor_ic | ic = 0 — must be byte-identical to `_xor` for every mlen in the sweep | [ ] |
| G4 | crypto_stream_chacha20_xor_ic | ic = 1 — must equal the 64-byte-shifted keystream (bytes 64.. of `_xor` output) | [ ] |
| G4 | crypto_stream_chacha20_xor_ic | ic = large, e.g. 0x00000000deadbeef and 2^31 | [ ] |
| G4 | crypto_stream_chacha20_xor_ic | ic = 2^32-1 (0xffffffff) with mlen 64/65/128 — the low 32-bit counter word saturates and the ADD carries into j13 mid-message (no rejection in the non-ietf form) | [ ] |
| G4 | crypto_stream_chacha20_xor_ic | ic = 2^32 and 2^32+1 (high word non-zero) and ic = 2^64-1 (wraps j12/j13 to 0 mid-message) — 64-bit counter range | [ ] |
| G4 | crypto_stream_chacha20_ietf_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 12 / SODIUM_MIN(SIZE_MAX, 64*(2^32-1)) | [ ] |
| G4 | crypto_stream_chacha20_ietf (keystream form) | 12-byte nonce; clen sweep 0,1,63,64,65,127,128,129,1000 | [ ] |
| G4 | crypto_stream_chacha20_ietf_xor | 12-byte nonce, ic implicitly 0; mlen sweep 0,1,63,64,65,127,128,129,1000; in-place c == m | [ ] |
| G4 | crypto_stream_chacha20_ietf_xor_ic | ic = 0 — identical to `_ietf_xor` | [ ] |
| G4 | crypto_stream_chacha20_ietf_xor_ic | ic = 1 — 64-byte-shifted keystream | [ ] |
| G4 | crypto_stream_chacha20_ietf_xor_ic | ic = large but legal, e.g. 2^31 with mlen 1000, and ic = 2^32-2 with mlen 64 | [ ] |
| G4 | crypto_stream_chacha20_ietf_xor_ic | ic = 2^32-1 (0xffffffff, the max for uint32_t) with mlen 0 and mlen 1..64 — the largest ic that still satisfies `ic + ceil(mlen/64) <= 2^32` | [ ] |
| G4 | crypto_stream_chacha20_ietf_ext | internal/private entry point (`private/chacha20_ietf_ext.h`); 12-byte nonce, clen sweep 0,1,64,65,1000 — same as `_ietf` but capped at SODIUM_SIZE_MAX instead of the ietf max | [ ] |
| G4 | crypto_stream_chacha20_ietf_ext_xor_ic | internal entry point; uint32_t ic = 0 / 1 / 2^31 / 2^32-1 with mlen 64/65/128 — deliberately allows the 32-bit counter to carry into the IV (no misuse guard), which is what xchacha20poly1305 and secretstream rely on | [ ] |
| G4 | crypto_stream_salsa20_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 8 / SODIUM_SIZE_MAX | [ ] |
| G4 | crypto_stream_salsa20 (keystream form) | 8-byte nonce; clen sweep 0,1,63,64,65,127,128,129,1000 (0 short-circuits) | [ ] |
| G4 | crypto_stream_salsa20_xor | ic implicitly 0; mlen sweep 0,1,63,64,65,127,128,129,1000; in-place c == m | [ ] |
| G4 | crypto_stream_salsa20_xor_ic | ic = 0 — identical to `_xor` | [ ] |
| G4 | crypto_stream_salsa20_xor_ic | ic = 1 — 64-byte-shifted keystream | [ ] |
| G4 | crypto_stream_salsa20_xor_ic | ic = large, e.g. 0xdeadbeef and 2^31 | [ ] |
| G4 | crypto_stream_salsa20_xor_ic | ic = 2^32-1 with mlen 64/65/128 — byte-wise carry from in[11] into in[12] mid-message | [ ] |
| G4 | crypto_stream_salsa20_xor_ic | ic = 2^32, 2^40, 2^63, 2^64-1 (full 64-bit counter written little-endian into in[8..16), then byte-wise incremented and allowed to wrap) | [ ] |
| G4 | crypto_stream_salsa2012_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 8 / SODIUM_SIZE_MAX | [ ] |
| G4 | crypto_stream_salsa2012 (keystream form) | 8-byte nonce; clen sweep 0,1,63,64,65,127,128,129,1000; NOTE there is no `_xor_ic` entry point for salsa2012 (counter always starts at 0) | [ ] |
| G4 | crypto_stream_salsa2012_xor | mlen sweep 0,1,63,64,65,127,128,129,1000; must equal keystream XOR m; in-place c == m | [ ] |
| G4 | crypto_stream_salsa208_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 8 / SODIUM_SIZE_MAX (whole family is deprecated/LCOV-excluded but still exported) | [ ] |
| G4 | crypto_stream_salsa208 (keystream form) | 8-byte nonce; clen sweep 0,1,63,64,65,127,128,129,1000; no `_xor_ic` entry point | [ ] |
| G4 | crypto_stream_salsa208_xor | mlen sweep 0,1,63,64,65,127,128,129,1000; in-place c == m | [ ] |
| G4 | crypto_stream_xsalsa20_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 24 / SODIUM_SIZE_MAX | [ ] |
| G4 | crypto_stream_xsalsa20 (keystream form) | 24-byte nonce (subkey = hsalsa20(n[0..16), k, NULL); salsa20 nonce = n+16); clen sweep 0,1,63,64,65,127,128,129,1000 | [ ] |
| G4 | crypto_stream_xsalsa20_xor | mlen sweep 0,1,63,64,65,127,128,129,1000; in-place c == m | [ ] |
| G4 | crypto_stream_xsalsa20_xor_ic | ic = 0 (must equal `_xor`), 1, large (2^31 / 0xdeadbeef), 2^32-1, 2^32, 2^64-1 — full 64-bit ic passed through to salsa20 | [ ] |
| G4 | crypto_stream_xsalsa20 vs crypto_stream_salsa20 | equivalence: hsalsa20(n[0..16), k, NULL) as the salsa20 key with n[16..24) as the salsa20 nonce must reproduce the xsalsa20 keystream | [ ] |
| G4 | crypto_stream_xchacha20_keygen / _keybytes / _noncebytes / _messagebytes_max | expect 32 / 24 / SODIUM_SIZE_MAX | [ ] |
| G4 | crypto_stream_xchacha20 (keystream form) | 24-byte nonce (k2 = hchacha20(n[0..16), k, NULL); chacha20 nonce = n+16, 8 bytes); clen sweep 0,1,63,64,65,127,128,129,1000 | [ ] |
| G4 | crypto_stream_xchacha20_xor | mlen sweep 0,1,63,64,65,127,128,129,1000; in-place c == m | [ ] |
| G4 | crypto_stream_xchacha20_xor_ic | ic = 0 (must equal `_xor`), 1, large (2^31 / 0xdeadbeef), 2^32-1, 2^32, 2^64-1 — 64-bit ic forwarded to the non-ietf chacha20 (no ic guard) | [ ] |
| G4 | crypto_stream_xchacha20 vs crypto_stream_chacha20 | equivalence: hchacha20(n[0..16), k, NULL) as the chacha20 key with n[16..24) as the 8-byte chacha20 nonce | [ ] |
| G4 | crypto_stream_keybytes / _noncebytes / _messagebytes_max / _primitive | default-primitive getters; expect 32 / 24 / SODIUM_SIZE_MAX / "xsalsa20" | [ ] |
| G4 | crypto_stream_keygen | fills 32-byte key | [ ] |
| G4 | crypto_stream (default = xsalsa20, keystream form) | 24-byte nonce; clen sweep 0,1,63,64,65,128,1000; must be byte-identical to crypto_stream_xsalsa20 | [ ] |
| G4 | crypto_stream_xor (default = xsalsa20) | mlen sweep 0,1,63,64,65,128,1000; must be byte-identical to crypto_stream_xsalsa20_xor; in-place c == m | [ ] |
| G4 | crypto_core_salsa20 | out[64], in[16], k[32], `c == NULL` -> uses the "expand 32-byte k" sigma words; sweep in/k = all-zero, all-0xff, random | [ ] |
| G4 | crypto_core_salsa20 | `c != NULL` with a caller-supplied 16-byte constant (the sigma bytes themselves, all-zero, and random) — must match the c==NULL result exactly when c is the sigma bytes | [ ] |
| G4 | crypto_core_salsa20_outputbytes / _inputbytes / _keybytes / _constbytes | expect 64 / 16 / 32 / 16 | [ ] |
| G4 | crypto_core_salsa2012 | out[64], in[16], k[32], `c == NULL` (sigma) — 12 rounds; sweep all-zero / all-0xff / random inputs | [ ] |
| G4 | crypto_core_salsa2012 | `c != NULL` (sigma bytes, all-zero, random) | [ ] |
| G4 | crypto_core_salsa2012_outputbytes / _inputbytes / _keybytes / _constbytes | expect 64 / 16 / 32 / 16 | [ ] |
| G4 | crypto_core_salsa208 | out[64], in[16], k[32], `c == NULL` (sigma) — 8 rounds; sweep all-zero / all-0xff / random | [ ] |
| G4 | crypto_core_salsa208 | `c != NULL` (sigma bytes, all-zero, random) | [ ] |
| G4 | crypto_core_salsa208_outputbytes / _inputbytes / _keybytes / _constbytes | expect 64 / 16 / 32 / 16 | [ ] |
| G4 | crypto_core_hsalsa20 | out[32], in[16], k[32], `c == NULL` -> sigma constants; sweep all-zero / all-0xff / random | [ ] |
| G4 | crypto_core_hsalsa20 | `c != NULL` with a 16-byte constant (sigma bytes -> must match c==NULL; plus all-zero and random c) | [ ] |
| G4 | crypto_core_hsalsa20_outputbytes / _inputbytes / _keybytes / _constbytes | expect 32 / 16 / 32 / 16 | [ ] |
| G4 | crypto_core_hchacha20 | out[32], in[16], k[32], `c == NULL` -> sigma constants; sweep all-zero / all-0xff / random | [ ] |
| G4 | crypto_core_hchacha20 | `c != NULL` with a 16-byte constant (sigma bytes -> must match c==NULL; plus all-zero and random c) | [ ] |
| G4 | crypto_core_hchacha20_outputbytes / _inputbytes / _keybytes / _constbytes | expect 32 / 16 / 32 / 16 | [ ] |
