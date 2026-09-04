| mac-1 | crypto_onetimeauth_bytes, crypto_onetimeauth_keybytes, crypto_onetimeauth_statebytes, crypto_onetimeauth_poly1305_bytes, crypto_onetimeauth_poly1305_keybytes, crypto_onetimeauth_poly1305_statebytes | constant getters; statebytes == sizeof(opaque[256]) == 256, bytes == 16, keybytes == 32 | [x] |
| mac-2 | crypto_onetimeauth_primitive | returned C string == "poly1305" | [x] |
| mac-3 | crypto_onetimeauth_poly1305 | one-shot, inlen ∈ {0,1,15,16,17,31,32,33,64,1000} × 3 random keys/messages each, canary-guarded 24-byte out | [x] |
| mac-4 | crypto_onetimeauth_poly1305 | one-shot, 24 random inlen in [0,600) | [x] |
| mac-5 | crypto_onetimeauth_poly1305 | in == NULL, inlen == 0 (header marks only out/k nonnull) | [x] |
| mac-6 | crypto_onetimeauth_poly1305 | key = all-zero, and key = all-0xff (drives the h >= p / `mask` selection branch in poly1305_finish) | [x] |
| mac-7 | crypto_onetimeauth | generic one-shot dispatcher; tag must equal crypto_onetimeauth_poly1305 for every length | [x] |
| mac-8 | crypto_onetimeauth_poly1305_init, _update, _final | streaming, random chunk plans (incl. 0-length and 1-byte chunks) for each inlen ∈ {0,1,15,16,17,31,32,33,64,1000}, 6 plans each | [x] |
| mac-9 | crypto_onetimeauth_poly1305_init, _update, _final | streaming, 40 random (inlen<400, random plan) cases | [x] |
| mac-10 | crypto_onetimeauth_poly1305_update | 21 explicit chunk plans that straddle the 16-byte block buffer: [15,1] [1,15] [15,2] [8,9] [16,1] [1,16] [17,15] [16,16,1] [3,0,13,0,1] [31,1,1] [7,9,16,0,1,15] … | [x] |
| mac-11 | crypto_onetimeauth_poly1305_update | 0-length update at start / middle / end of the stream, incl. `in == NULL, inlen == 0` | [x] |
| mac-12 | crypto_onetimeauth_poly1305_init, _update | FULL 256-byte opaque state buffer compared byte-for-byte after init and after every update (canary-prefilled; only the first 144 = sizeof(poly1305_state_internal_t) bytes may be touched, of which 137 carry data) | [x] |
| mac-13 | crypto_onetimeauth_poly1305_final | state after final: asserted that exactly bytes 0..144 are zeroed (sodium_memzero(st, sizeof *st)) and 144..256 keep the canary, in both libraries | [x] |
| mac-14 | crypto_onetimeauth_poly1305_final | final() called twice — second call runs on the memzero'd state (leftover/h/pad all 0); both libraries must produce the same tag | [x] |
| mac-15 | crypto_onetimeauth_poly1305, _init/_update/_final | streaming result == one-shot result for every plan | [x] |
| mac-16 | crypto_onetimeauth_poly1305_verify | correct tag, inlen ∈ {0,1,15,16,17,31,32,33,64,1000} → 0 | [x] |
| mac-17 | crypto_onetimeauth_poly1305_verify | each of the 128 tag bits flipped individually, per length → -1 | [x] |
| mac-18 | crypto_onetimeauth_poly1305_verify | each of the 32 key bytes altered (key[0..16] = r, masked; key[16..32] = pad) | [x] |
| mac-19 | crypto_onetimeauth_poly1305_verify | "truncated" key (key[16..32] zeroed) | [x] |
| mac-20 | crypto_onetimeauth_verify | generic dispatcher: correct tag → 0, all 128 flipped bits → -1 | [x] |
| mac-21 | crypto_onetimeauth_init, crypto_onetimeauth_update, crypto_onetimeauth_final | generic streaming dispatchers on crypto_onetimeauth_state, random plans per length, full state compare | [x] |
| mac-22 | crypto_onetimeauth_poly1305_donna_implementation (data symbol) | struct read out of both .so via both_data!, all five function pointers (.onetimeauth, .onetimeauth_verify, .onetimeauth_init, .onetimeauth_update, .onetimeauth_final) invoked through both libraries with full state compare | [x] |
| mac-23 | _crypto_onetimeauth_poly1305_pick_best_implementation | called 3× (no HAVE_TI_MODE / HAVE_EMMINTRIN_H ⇒ always donna, returns 0), then 20 random one-shot tags re-checked | [x] |
| mac-24 | crypto_onetimeauth_poly1305_keygen, crypto_onetimeauth_keygen | value is random, so the written extent is checked: exactly 32 bytes written, canary past byte 32 intact | [x] |
| mac-25 | crypto_onetimeauth_poly1305 | RFC 8439 §2.5.2 known-answer vector checked against BOTH libraries | [x] |
| mac-26 | crypto_auth_hmacsha256_bytes/_keybytes/_statebytes, crypto_auth_hmacsha512_*, crypto_auth_hmacsha512256_*, crypto_auth_bytes/_keybytes | constant getters; statebytes 208 / 416 / 416, bytes 32 / 64 / 32 | [x] |
| mac-27 | crypto_auth_primitive | returned C string == "hmacsha512256" | [x] |
| mac-28 | crypto_auth_hmacsha256_init/_update/_final | keylen ∈ {0,1,2,31,32,33,63,64,65,128,199} (< / == / > the 64-byte sha256 block) × inlen ∈ {0,1,63,64,65,127,128,129,200}, random chunk plan each, FULL 208-byte state compared after init and every update | [x] |
| mac-29 | crypto_auth_hmacsha512_init/_update/_final | keylen ∈ {0,1,2,31,32,33,127,128,129,256,391} (< / == / > the 128-byte sha512 block) × inlen ∈ {0,1,63,64,65,127,128,129,200}, random chunk plan each, FULL 416-byte state compared | [x] |
| mac-30 | crypto_auth_hmacsha512256_init/_update/_final | same keylen × inlen matrix as mac-29 (init/update are casts onto hmacsha512), FULL 416-byte state compared, out truncated to 32 bytes with canary check | [x] |
| mac-31 | crypto_auth_hmacsha256, crypto_auth_hmacsha512, crypto_auth_hmacsha512256 | one-shot (keylen fixed at KEYBYTES=32) over inlen ∈ {0,1,31,32,55,56,63,64,65,111,112,119,120,127,128,129,1000} (sha256/sha512 block and length-padding boundaries) × 3 random keys | [x] |
| mac-32 | crypto_auth_hmacsha*{,256,512,512256} vs _init/_update/_final | one-shot == init(k,32)/update(msg)/final for every length | [x] |
| mac-33 | crypto_auth_hmacsha256_verify, crypto_auth_hmacsha512_verify, crypto_auth_hmacsha512256_verify | correct tag → 0, per length | [x] |
| mac-34 | crypto_auth_hmacsha256_verify, crypto_auth_hmacsha512_verify, crypto_auth_hmacsha512256_verify | every tag byte XOR 0xff (per length), plus every single tag bit flipped (outlen*8 flips) for one message | [x] |
| mac-35 | crypto_auth_hmacsha256_verify, crypto_auth_hmacsha512_verify, crypto_auth_hmacsha512256_verify | every one of the 32 key bytes altered; message byte altered; shortened inlen | [x] |
| mac-36 | crypto_auth_hmacsha256, _verify, … | in == NULL, inlen == 0 for both the one-shot and verify entry points | [x] |
| mac-37 | crypto_auth_hmacsha256_init, crypto_auth_hmacsha512_init, crypto_auth_hmacsha512256_init | key == NULL with keylen == 0 (the only NULL-key case the C allows); must equal init(non-NULL ptr, 0) | [x] |
| mac-38 | crypto_auth_hmacsha256_init, crypto_auth_hmacsha512_init | keylen > blocksize: key pre-hashed through state->ictx; keylen ∈ {65,100,128,129,200,1000}; result cross-checked against HMAC(SHA-256/512(key)) | [x] |
| mac-39 | crypto_auth_hmacsha512256_init/_update vs crypto_auth_hmacsha512_init/_update | states must be byte-identical (plain cast), keylen ∈ {0,1,32,128,129,300} × inlen ∈ {0,1,64,127,128,129,500} | [x] |
| mac-40 | crypto_auth_hmacsha512256_final vs crypto_auth_hmacsha512_final | out == first 32 bytes of the 64-byte hmacsha512 tag (memcpy(out, out0, 32)), no write past byte 32 | [x] |
| mac-41 | crypto_auth_hmacsha256_keygen, crypto_auth_hmacsha512_keygen, crypto_auth_hmacsha512256_keygen, crypto_auth_keygen | random value, so written extent checked: exactly 32 bytes, canary past byte 32 intact | [x] |
| mac-42 | crypto_auth | generic dispatcher over inlen ∈ SHA_LENS × 3 keys; tag must equal crypto_auth_hmacsha512256 exactly; canary past byte 32 intact | [x] |
| mac-43 | crypto_auth_verify | generic dispatcher: correct tag → 0, each of 32 tag bytes flipped → -1, per length | [x] |
| mac-44 | crypto_auth_hmacsha256_init/_update/_final | RFC 4231 test case 2 ("Jefe" / "what do ya want for nothing?") known-answer checked against BOTH libraries | [x] |
