# CONFIGS.md — configuration-surface table (valid inputs)

Mechanically derived from the C sources in `c_src/libsodium/` (the axes each
`if`/`switch`/parameter actually makes the C branch on), one row per meaningful
combination of options × input shape that the C treats differently.

Every row is exercised by a differential test in `tests/<area>.rs` that calls
**both** the C `libsodium.so` and the Rust `liblibsodium.so` through
`libloading` and compares return values and full output buffers byte-for-byte,
over many randomized inputs with a fixed seed.

`[x]` = row passes (C output == Rust output for every randomized input).

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section**: the crate has no
optional features and no `cfg`-gated code, so there is exactly ONE build
configuration (`--no-default-features` and any `--features` combination are
identical to the default). Verified with:

```
$ grep -c '^\[features\]' translation/Cargo.toml   # -> 0
$ cargo test --release --no-default-features        # same result as default
```

The C build likewise defines no `HAVE_*` macros (see `c_src/CMakeLists.txt`), so
the portable/reference implementation is the only code path in both libraries;
`sodium_runtime_has_*()` returning identical values in both `.so`s is asserted in
`tests/sodium.rs`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
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

| aead2-1 | crypto_aead_aes256gcm_keybytes, _nsecbytes, _npubbytes, _abytes, _messagebytes_max | constant getters, values compared C vs Rust and against the header macros (32/0/12/16, min(SIZE_MAX-16, 16*(2^32-2))) | [x] |
| aead2-2 | crypto_aead_aes256gcm_statebytes | `(sizeof(crypto_aead_aes256gcm_state)+15) & ~15` with `CRYPTO_ALIGN(16) unsigned char opaque[512]` → 512 on both; state buffer 16-byte aligned by the caller | [x] |
| aead2-3 | crypto_aead_aes256gcm_is_available | no HAVE_TMMINTRIN_H/HAVE_WMMINTRIN_H/HAVE_ARMCRYPTO → stub branch compiled → returns 0 on both | [x] |
| aead2-4 | crypto_aead_aes256gcm_encrypt | stub: mlen=64, adlen=20, clen_p non-NULL; verifies rc=-1, errno=ENOSYS, c/clen_p untouched | [x] |
| aead2-5 | crypto_aead_aes256gcm_decrypt | stub: clen=64, adlen=20, mlen_p non-NULL, nsec=NULL; rc=-1, errno=ENOSYS, m/mlen_p untouched | [x] |
| aead2-6 | crypto_aead_aes256gcm_encrypt_detached | stub: mac buffer + maclen_p non-NULL; rc=-1, errno=ENOSYS, c/mac/maclen_p untouched | [x] |
| aead2-7 | crypto_aead_aes256gcm_decrypt_detached | stub: mac non-NULL, nsec=NULL; rc=-1, errno=ENOSYS, m untouched | [x] |
| aead2-8 | crypto_aead_aes256gcm_beforenm | stub: 512-byte 16-aligned state, key=32B; rc=-1, errno=ENOSYS, whole state buffer byte-identical to the 0xA5 canary (no write at all) | [x] |
| aead2-9 | crypto_aead_aes256gcm_encrypt_afternm | stub: precomputed-state form, clen_p non-NULL; rc=-1, errno=ENOSYS, no buffer writes | [x] |
| aead2-10 | crypto_aead_aes256gcm_decrypt_afternm | stub: precomputed-state form, mlen_p non-NULL; rc=-1, errno=ENOSYS, no buffer writes | [x] |
| aead2-11 | crypto_aead_aes256gcm_encrypt_detached_afternm | stub: precomputed-state form, mac + maclen_p; rc=-1, errno=ENOSYS, no buffer writes | [x] |
| aead2-12 | crypto_aead_aes256gcm_decrypt_detached_afternm | stub: precomputed-state form, mac non-NULL; rc=-1, errno=ENOSYS, no buffer writes | [x] |
| aead2-13 | crypto_aead_aes256gcm_encrypt, crypto_aead_aes256gcm_decrypt | argument-insensitivity of the stub: clen_p/mlen_p/m/nsec/ad all NULL, mlen/clen ∈ {0,1,16,17,2^61,u64::MAX}; always rc=-1 + errno=ENOSYS | [x] |
| aead2-14 | crypto_aead_aes256gcm_keygen | deterministic `randombytes_implementation` injected in both libraries; writes exactly 32 bytes, canary tail intact, 8 rounds | [x] |
| aead2-15 | crypto_aead_aegis128l_keybytes, _nsecbytes, _npubbytes, _abytes, _messagebytes_max | constant getters: 16/0/16/32 and min(SIZE_MAX-32, 2^61-1) = 2^61-1 | [x] |
| aead2-16 | crypto_aead_aegis256_keybytes, _nsecbytes, _npubbytes, _abytes, _messagebytes_max | constant getters: 32/0/32/32 and min(SIZE_MAX-32, 2^61-1) = 2^61-1 | [x] |
| aead2-17 | _crypto_aead_aegis128l_pick_best_implementation, _crypto_aead_aegis256_pick_best_implementation | no HAVE_ARMCRYPTO / HAVE_AVXINTRIN_H+HAVE_WMMINTRIN_H → always selects `*_soft_implementation`, returns 0; called 3× and encryption re-verified after each call | [x] |
| aead2-18 | crypto_aead_aegis128l_encrypt, crypto_aead_aegis128l_decrypt | full size matrix mlen × adlen over {0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000} (RATE=32: straddles 32/64-byte absorb, absorb2 and partial-block branches), 3 random k/npub/m/ad per cell = 675 cases; clen_p/mlen_p checked, output canary checked, plaintext round-trips | [x] |
| aead2-19 | crypto_aead_aegis256_encrypt, crypto_aead_aegis256_decrypt | same full size matrix (RATE=16: straddles 16/32-byte absorb, absorb2 and partial-block branches), 675 cases | [x] |
| aead2-20 | crypto_aead_aegis128l_encrypt, crypto_aead_aegis256_encrypt | clen_p == NULL and nsec != NULL (must be ignored); output compared against the clen_p != NULL run | [x] |
| aead2-21 | crypto_aead_aegis128l_encrypt_detached, crypto_aead_aegis256_encrypt_detached | maclen_p non-NULL (must be set to ABYTES=32) and maclen_p == NULL; detached (c,mac) compared against the combined ciphertext, over the whole size matrix | [x] |
| aead2-22 | crypto_aead_aegis128l_decrypt, crypto_aead_aegis256_decrypt | mlen_p == NULL, and m == NULL (ciphertext verified but not written out — takes the `else` branch of `if (m != NULL)` in the soft impl), over the whole size matrix | [x] |
| aead2-23 | crypto_aead_aegis128l_decrypt_detached, crypto_aead_aegis256_decrypt_detached | mac supplied separately, nsec != NULL (ignored and left untouched), and m == NULL variant, over the whole size matrix | [x] |
| aead2-24 | crypto_aead_aegis128l_encrypt/_decrypt, crypto_aead_aegis256_encrypt/_decrypt | ad == NULL with adlen == 0 (all adlen==0 cells of the matrix pass a NULL ad pointer) | [x] |
| aead2-25 | crypto_aead_aegis128l_encrypt/_decrypt, crypto_aead_aegis256_encrypt/_decrypt | fully in-place: c == m for encrypt and m == c for decrypt, whole size matrix; result compared against the out-of-place ciphertext | [x] |
| aead2-26 | crypto_aead_aegis128l_decrypt, crypto_aead_aegis256_decrypt | authentication failure via wrong key, wrong nonce and wrong adlen (adlen+1); rc=-1, m zeroed for exactly mlen bytes, canary tail intact | [x] |
| aead2-27 | aegis128l_soft_implementation (data object), .encrypt_detached / .decrypt_detached | both function pointers of the exported struct called directly through both .so's; maclen ∈ {0,1,8,15,16,17,31,32,33,48} × mlen ∈ {0,1,15,16,17,31,32,33,63,64,65,129} × adlen ∈ {0,1,16,17,32,33,64,65}; exercises the maclen==16, maclen==32 and `else` branches of aegis128l_mac() | [x] |
| aead2-28 | aegis256_soft_implementation (data object), .encrypt_detached / .decrypt_detached | same maclen × mlen × adlen sweep through the exported struct's function pointers; exercises all three branches of aegis256_mac() | [x] |
| aead2-29 | aegis128l_soft_implementation.decrypt_detached, aegis256_soft_implementation.decrypt_detached | implementation-level decrypt with m == NULL for every maclen/mlen/adlen combination | [x] |
| aead2-30 | crypto_aead_aegis128l_keygen, crypto_aead_aegis256_keygen | deterministic `randombytes_implementation` injected in both libraries; writes exactly 16 / 32 bytes, canary tail intact, 8 rounds each | [x] |
| aead2-31 | _sodium_softaes_expand_key128 | AES-128 key schedule: keys 0x00*16, 0xFF*16, 0..15 and 64 random keys; all 11 round keys compared byte-for-byte + 2 canary blocks past the end | [x] |
| aead2-32 | _sodium_softaes_expand_key256 | AES-256 key schedule (both the `i%8==0` RCON and the `i%8==4` sub_word branch): 3 fixed + 64 random keys; all 15 round keys compared + canary block | [x] |
| aead2-33 | _sodium_softaes_invert_key_schedule128 | in-place inverse-MixColumns over rkeys[1..10) (indices 0 and 10 must stay untouched); applied to every expanded 128-bit schedule above | [x] |
| aead2-34 | _sodium_softaes_invert_key_schedule256 | in-place inverse-MixColumns over rkeys[1..14) (indices 0 and 14 must stay untouched); applied to every expanded 256-bit schedule above | [x] |
| aead2-35 | _sodium_softaes_inv_mix_columns | struct-by-value in/out; 0, all-ones, every byte value 0..255 in each of the four byte positions, plus 2000 random blocks | [x] |
| aead2-36 | _sodium_softaes_block_encrypt | non-FAVOR_PERFORMANCE SRM-1R bitsliced round (the `#else` branch); 3028 (block, rk) pairs: edge values, all 256 byte values × 4 positions, 2000 random | [x] |
| aead2-37 | _sodium_softaes_block_decrypt | INV_SBOX + inv_mix_column round; same 3028 (block, rk) pairs | [x] |
| aead2-38 | _sodium_softaes_block_encryptlast | SOFTAES_STRIDE=16 stride-table SBOX form (HAVE_INLINE_ASM undefined → no barriers); same 3028 (block, rk) pairs | [x] |
| aead2-39 | _sodium_softaes_block_decryptlast | INV_SBOX-only last round; same 3028 (block, rk) pairs | [x] |
| aead2-40 | _sodium_softaes_expand_key128 + _sodium_softaes_block_encrypt + _block_encryptlast + _invert_key_schedule128 + _block_decrypt + _block_decryptlast | full 10-round AES-128 encrypt and decrypt composed from the exported primitives, 32 random key/block pairs, round-trip verified | [x] |
| aead2-41 | _sodium_softaes_expand_key256 + _sodium_softaes_block_encrypt + _block_encryptlast + _invert_key_schedule256 + _block_decrypt + _block_decryptlast | full 14-round AES-256 encrypt and decrypt composed from the exported primitives, 32 random key/block pairs, round-trip verified | [x] |
| aead2-42 | _sodium_softaes_expand_key128 + _sodium_softaes_block_encrypt + _block_encryptlast | FIPS-197 C.1 known-answer vector checked independently against BOTH libraries (pins softaes to real AES, not just C↔Rust agreement) | [x] |

| blake2-1 | crypto_generichash_bytes_min/max/bytes, crypto_generichash_keybytes_min/max/keybytes, crypto_generichash_statebytes, crypto_generichash_primitive | all compile-time getters (16/64/32, 16/64/32, 384, "blake2b") | [x] |
| blake2-2 | crypto_generichash_blake2b_bytes_min/max/bytes, _keybytes_min/max/keybytes, _saltbytes, _personalbytes, _statebytes | all compile-time getters (16/64/32, 16/64/32, 16, 16, 384) | [x] |
| blake2-3 | crypto_generichash, crypto_generichash_blake2b | outlen ∈ {1,15,16,17,31,32,33,63,64} × keylen ∈ {0,1,15,16,31,32,33,63,64} × inlen ∈ {0,1,2,7,8,63,64,127,128,129,191,192,255,256,257,383,384,385,1000,4096}; canary-guarded output | [x] |
| blake2-4 | crypto_generichash_blake2b | key != NULL with keylen == 0 → unkeyed path, must equal key == NULL | [x] |
| blake2-5 | crypto_generichash_blake2b | in == NULL with inlen == 0 (misuse check does not fire) | [x] |
| blake2-6 | crypto_generichash_blake2b_salt_personal | salt/personal ∈ {NULL, random, all-zero}² (5 combinations incl. NULL==zeros equivalence) × keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,64,128,129,256,257,1000} | [x] |
| blake2-7 | crypto_generichash_init, crypto_generichash_update, crypto_generichash_final | keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,63,64,127,128,129,191,192,255,256,257,300,512,1000,4096}, randomized chunk splits including leading/interior/trailing 0-length updates; whole 384-byte opaque state compared after init, after every update and after final; digest cross-checked against the one-shot API | [x] |
| blake2-8 | crypto_generichash_blake2b_init, _update, _final | same matrix as blake2-7 through the blake2b-specific entry points, randomized chunk splits, full state compare | [x] |
| blake2-9 | crypto_generichash_blake2b_init_salt_personal, _update, _final | salt × personal ∈ {NULL, set}² (4 combinations) × keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,128,256,257,700}, randomized chunk splits, full state compare, digest == crypto_generichash_blake2b_salt_personal | [x] |
| blake2-10 | crypto_generichash_blake2b_init, _init_salt_personal | key == NULL with keylen ∈ {1,16,32,64} selects the *unkeyed* branch; state must equal keylen == 0 | [x] |
| blake2-11 | crypto_generichash_blake2b_update, _final | update after final (state has f[0] != 0); update with in == NULL, inlen == 0 | [x] |
| blake2-12 | _sodium_blake2b_init | every valid outlen 1..=64, full state compare | [x] |
| blake2-13 | _sodium_blake2b_init_salt_personal | salt × personal ∈ {NULL, set}² × outlen ∈ {1,16,32,64}, full state compare | [x] |
| blake2-14 | _sodium_blake2b_init_key | keylen 1..=64 × outlen ∈ {1,32,64}, full state compare (includes the 128-byte key-block update) | [x] |
| blake2-15 | _sodium_blake2b_init_key_salt_personal | keylen 1..=64 × outlen ∈ {1,32,64} × salt/personal set and NULL, full state compare | [x] |
| blake2-16 | _sodium_blake2b_init_param, _sodium_blake2b_update, _sodium_blake2b_final | 40 fully random 64-byte parameter blocks (arbitrary digest_length/key_length/fanout/depth/leaf_length/node_offset/node_depth/inner_length/reserved/salt/personal), random inlen 0..600, random outlen 1..64 | [x] |
| blake2-17 | _sodium_blake2b_update, _sodium_blake2b_final (blake2b_set_lastnode) | state with last_node = 1 poked directly at offset 360 (unreachable via public API) × inlen ∈ {0,1,128,257,600} | [x] |
| blake2-18 | _sodium_blake2b | keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,127,128,129,256,257,1024}; in = NULL when inlen = 0; result == crypto_generichash_blake2b | [x] |
| blake2-19 | _sodium_blake2b_salt_personal | same matrix as blake2-18 × salt/personal ∈ {NULL, set}² | [x] |
| blake2-20 | _sodium_blake2b_compress_ref | 64 fully random 384-byte states (arbitrary h/t/f including carry and both finalisation flags) × random 128-byte blocks, plus all-zero and all-0xff states | [x] |
| blake2-21 | _sodium_blake2b_pick_best_implementation, _crypto_generichash_blake2b_pick_best_implementation | no HAVE_*INTRIN_H macros ⇒ always selects blake2b_compress_ref; return value + hashing still correct afterwards | [x] |
| blake2-22 | _sodium_blake2b_long | outlen ∈ {0,1,16,32,63,64} (single-pass branch), {65,66,95,96} (chained, no loop iteration), {97,127,128,129,160,192,200,1000} (chained with 1..n loop iterations) × inlen ∈ {0,1,64,128,257,1000} | [x] |
| blake2-23 | crypto_generichash_keygen, crypto_generichash_blake2b_keygen, crypto_kdf_keygen, crypto_shorthash_keygen, crypto_kdf_hkdf_sha256_keygen, crypto_kdf_hkdf_sha512_keygen | writes exactly KEYBYTES (32/32/32/16/32/64) bytes, canary beyond untouched (bytes are randombytes_buf output, so not comparable) | [x] |
| blake2-24 | crypto_kdf_bytes_min/max, crypto_kdf_contextbytes, crypto_kdf_keybytes, crypto_kdf_primitive, crypto_kdf_blake2b_bytes_min/max, _contextbytes, _keybytes | all compile-time getters (16/64/8/32/"blake2b") | [x] |
| blake2-25 | crypto_kdf_blake2b_derive_from_key, crypto_kdf_derive_from_key | subkey_len ∈ {16,17,31,32,33,63,64} × subkey_id ∈ {0,1,2,0xff,0x100,0xffffffff,0x100000000,u64::MAX,0x0123456789abcdef} × random 32-byte key and 8-byte ctx; result independently reproduced through crypto_generichash_blake2b_salt_personal(salt=LE64(id)‖0, personal=ctx‖0) | [x] |
| blake2-26 | crypto_kdf_blake2b_derive_from_key | ctx containing embedded NUL bytes (ctx is a fixed 8-byte array, not a C string) | [x] |
| blake2-27 | crypto_kdf_hkdf_sha256_keybytes/bytes_min/bytes_max/statebytes | getters (32 / 0 / 0xff*32 = 8160 / 208) | [x] |
| blake2-28 | crypto_kdf_hkdf_sha256_extract | salt_len ∈ {0,1,16,32,55,63,64,65,100,128,200} × ikm_len ∈ {0,1,16,32,64,100,127,128,129,500} (crosses the 64-byte HMAC key-compression boundary) | [x] |
| blake2-29 | crypto_kdf_hkdf_sha256_extract_init, _extract_update, _extract_final | same salt/ikm matrix, randomized chunk splits including 0-length updates; whole 208-byte state compared after init and after every update; extract_final must zero the state; streamed prk == one-shot prk | [x] |
| blake2-30 | crypto_kdf_hkdf_sha256_extract | salt == NULL / salt_len == 0; ikm == NULL / ikm_len == 0 | [x] |
| blake2-31 | crypto_kdf_hkdf_sha256_expand | out_len ∈ {0,1,31,32,33,63,64,65,96,100,255,1000,8159,8160} (0, exactly-one-block, one-past-block, multi-block, `left != 0` tail, and BYTES_MAX) × ctx_len ∈ {0,1,8,32,64,200}, canary-guarded output | [x] |
| blake2-32 | crypto_kdf_hkdf_sha256_expand | ctx == NULL with ctx_len == 0 | [x] |
| blake2-33 | crypto_kdf_hkdf_sha512_keybytes/bytes_min/bytes_max/statebytes | getters (64 / 0 / 0xff*64 = 16320 / 416) | [x] |
| blake2-34 | crypto_kdf_hkdf_sha512_extract | salt_len ∈ {0,1,16,32,55,63,64,65,100,128,200} × ikm_len ∈ {0,1,16,32,64,100,127,128,129,500} (crosses the 128-byte HMAC key-compression boundary) | [x] |
| blake2-35 | crypto_kdf_hkdf_sha512_extract_init, _extract_update, _extract_final | same matrix, randomized chunk splits including 0-length updates, whole 416-byte state compared after init/each update, state zeroed by final, streamed == one-shot | [x] |
| blake2-36 | crypto_kdf_hkdf_sha512_extract | salt == NULL / salt_len == 0; ikm == NULL / ikm_len == 0 | [x] |
| blake2-37 | crypto_kdf_hkdf_sha512_expand | out_len ∈ {0,1,63,64,65,127,128,129,192,100,255,1000,16319,16320} × ctx_len ∈ {0,1,8,32,64,200}, canary-guarded output | [x] |
| blake2-38 | crypto_kdf_hkdf_sha512_expand | ctx == NULL with ctx_len == 0 | [x] |
| blake2-39 | crypto_shorthash_bytes, _keybytes, _primitive, crypto_shorthash_siphash24_bytes/_keybytes, crypto_shorthash_siphashx24_bytes/_keybytes | getters (8/16/"siphash24", 8/16, 16/16) | [x] |
| blake2-40 | crypto_shorthash_siphash24, crypto_shorthash | every inlen 0..=80 (all 8 `left` residues at every 8n boundary) plus {100,127,128,129,255,256,257,1000,4096}, 3 random keys each; crypto_shorthash == crypto_shorthash_siphash24 | [x] |
| blake2-41 | crypto_shorthash_siphash24 | all-zero and all-0xff keys × inlen ∈ {0,7,8,9,64} | [x] |
| blake2-42 | crypto_shorthash_siphashx24 | every inlen 0..=80 plus {100,127,128,129,255,256,257,1000,4096}, 3 random keys each (16-byte output, second finalisation round) | [x] |
| blake2-43 | crypto_shorthash_siphashx24 | all-zero and all-0xff keys × inlen ∈ {0,7,8,9,64} | [x] |
| blake2-44 | crypto_verify_16_bytes, crypto_verify_32_bytes, crypto_verify_64_bytes | getters (16/32/64) | [x] |
| blake2-45 | crypto_verify_16 | 20 random equal pairs, aliased pointers, every one of 16×8 single-bit differences in both argument orders, 50 random pairs, all-zero/all-0xff degenerate pairs | [x] |
| blake2-46 | crypto_verify_32 | 20 random equal pairs, aliased pointers, every one of 32×8 single-bit differences in both argument orders, 50 random pairs, degenerate pairs | [x] |
| blake2-47 | crypto_verify_64 | 20 random equal pairs, aliased pointers, every one of 64×8 single-bit differences in both argument orders, 50 random pairs, degenerate pairs | [x] |

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

| ed25519low-1 | _sodium_fe25519_frombytes | 180 x 32B: 160 random, all-0x00, all-0xff, p-1/p/p+1/p+2, high bit set+clear (bit 255 is ignored by frombytes); full 10-limb int32 output compared | [x] |
| ed25519low-2 | _sodium_fe25519_tobytes | applied to frombytes output AND to fe25519_invert output (partially-reduced limbs); output buffer canary-prefilled | [x] |
| ed25519low-3 | _sodium_fe25519_invert | 180 field elements incl. 0 (degenerate: invert(0)=0), 1, p-1; plus out==z aliasing; full 10-limb output compared | [x] |
| ed25519low-4 | _sodium_ge25519_frombytes | valid canonical point encodings a*B (64 random a + specials), rc=0, full ge25519_p3 (40 x int32 = 160B) compared | [x] |
| ed25519low-5 | _sodium_ge25519_frombytes | sign bit set (s[31]>>7 == 1) on otherwise valid encodings -> fe25519_cmov(negx) branch | [x] |
| ed25519low-6 | _sodium_ge25519_frombytes | identity (y=1), order-2 (y=p-1), order-4 (y=0, both signs); plus libsodium's small-order blocklist byte patterns e0eb7a../5f9c95bc.. (both signs) which are NOT decodable in this build (rc=-1) | [x] |
| ed25519low-7 | _sodium_ge25519_frombytes | non-canonical encodings: y=p, y=p+1, y=2^255-1, y=p-1/p/p+1 with bit 255 set, d9ff..ff | [x] |
| ed25519low-8 | _sodium_ge25519_frombytes | y with neither root (has_m_root=0 && has_p_root=0) -> rc=-1; struct left partially written (X,Y,Z written, T not) and compared against a shared canary | [x] |
| ed25519low-9 | _sodium_ge25519_frombytes | has_p_root only (uv is a non-square) -> x *= sqrt(-1) cmov branch; hit by ~half of the 160 random 32B inputs | [x] |
| ed25519low-10 | _sodium_ge25519_frombytes_negate_vartime | same 6 input shapes as rows 4-9 (valid / sign bit / torsion / non-canonical / no-root rc=-1 / sqrt(-1) branch); rc + full p3 compared | [x] |
| ed25519low-11 | _sodium_ge25519_is_canonical | s[0] swept 0x00..0xff with s[1..30]=0xff and s[31]=0x7f and =0xff; one middle byte != 0xff; 160 random; all specials | [x] |
| ed25519low-12 | _sodium_ge25519_p3_tobytes | full p3 corpus: a*B, torsion, off-subgroup (a*B+T), 16 synthetic p3 (each coordinate a reduced field element), all-zero p3 (Z=0 -> invert(0)) | [x] |
| ed25519low-13 | _sodium_ge25519_tobytes (ge25519_p2) | 12 p2 from double_scalarmult_vartime, 16 synthetic p2, all-zero p2; output canary-prefilled | [x] |
| ed25519low-14 | _sodium_ge25519_p1p1_to_p2 | 133 ge25519_p1p1: 128 random (all four coordinates reduced field elements), all-zero, one-hot per coordinate; full 30-limb p2 compared | [x] |
| ed25519low-15 | _sodium_ge25519_p1p1_to_p3 | same 133 p1p1 inputs; full 40-limb p3 compared | [x] |
| ed25519low-16 | _sodium_ge25519_p2_to_p3 | the 133 p2 produced by row 14 plus 16 real projective points from double_scalarmult_vartime | [x] |
| ed25519low-17 | _sodium_ge25519_p3_add | full p3 corpus x {self, 2 deterministic partners} + 128 random pairs; includes P+P, P+identity, torsion, off-subgroup, synthetic, all-zero | [x] |
| ed25519low-18 | _sodium_ge25519_p3_sub | same shapes incl. P-P (= identity) | [x] |
| ed25519low-19 | _sodium_ge25519_clear_cofactor | in-place on the full p3 corpus (p3_dbl + 2x p2_dbl + p1p1_to_p2/p3 chain) | [x] |
| ed25519low-20 | _sodium_ge25519_scalarmult_base | scalars: 0, 1, 2, L-1, L, L+1, 2^252, 2^252-1, 2^253-1, 0x55.., 0xaa.., 0x7f.., 0xff.., 12 single-bit (bits 0,1,7,8,127,128,250..255), 128 random; full p3 compared | [x] |
| ed25519low-21 | _sodium_ge25519_scalarmult_base | a[31] > 127 (documented precondition violated): e[63] reaches 16 so ge25519_cmov8_base selects the neutral precomp; ~half of the 96 unmasked random scalars | [x] |
| ed25519low-22 | _sodium_ge25519_scalarmult | 40 scalars x p3 corpus (valid, identity, torsion, off-subgroup, synthetic, all-zero), strided sample; exercises ge25519_p3_to_cached / add_cached / cmov8_cached / p2_dbl | [x] |
| ed25519low-23 | _sodium_ge25519_double_scalarmult_vartime | 64 random (a,b) x random A from the p3 corpus; full ge25519_p2 compared | [x] |
| ed25519low-24 | _sodium_ge25519_double_scalarmult_vartime | 8x8 edge scalar matrix: 0 (empty slide -> ge25519_p2_0 result), 1, 0xff.. (slide carry-propagation + `cmp < -15` break), 0x55.., 0xaa.., 0x7f.., 0x80.., L | [x] |
| ed25519low-25 | _sodium_ge25519_is_on_curve | on-curve p3 (decoded points) and off-curve p3 (synthetic / partially-written); both return values 0 and 1 asserted to occur | [x] |
| ed25519low-26 | _sodium_ge25519_is_on_main_subgroup | in-subgroup (a*B), torsion, a*B+T off-subgroup, synthetic, all-zero; exercises ge25519_mul_l / p3_dbladd / p3p3_dbl / p3_neg; both 0 and 1 observed | [x] |
| ed25519low-27 | _sodium_ge25519_has_small_order | X=0, Y=0, Z=0, y*sqrt(-1)-x=0, y*sqrt(-1)+x=0, and none-of-the-above; both 0 and non-zero observed | [x] |
| ed25519low-28 | _sodium_ge25519_from_uniform | 190 r values (28 specials + 160 random + all-0x00 + all-0xff); covers x_sign = (r[31]>>7) both ways and fe25519_notsquare(gx1) both ways | [x] |
| ed25519low-29 | _sodium_ge25519_from_hash | 167 h[64]: all-0x00, all-0xff, all-0x80, 160 random, and the 4 combinations of h[31]/h[63] top bit set/clear driving fe25519_reduce64's 19/722 correction | [x] |
| ed25519low-30 | _sodium_sc25519_reduce | in-place on s[64]: 0, 1, 2, L-1, L, L+1, 2^252, all-0xff (32B and 64B), all-0x80, top-byte-only, 200 random; ALL 64 bytes compared (the C writes only s[0..32]) | [x] |
| ed25519low-31 | _sodium_sc25519_mul | 213 special x special combinations + 200 fully random pairs + 32 s==a aliasing cases | [x] |
| ed25519low-32 | _sodium_sc25519_muladd | 213 special triples + 200 fully random triples | [x] |
| ed25519low-33 | _sodium_sc25519_invert | 0 (-> 0), 1, 2, L-1, L, L+1, 2^252, 0xff.., 200 random, and a 256-step walk of s[0] with s[1..] = L | [x] |
| ed25519low-34 | _sodium_sc25519_is_canonical | s = L, L +/- 1 in every byte position, s[0] swept 0x00..0xff with s[1..]=L, all specials, 200 random; both 0 and 1 observed | [x] |
| ed25519low-35 | _sodium_ristretto255_from_hash | 164 h[64]: all-0x00, all-0xff, all-0x80, all-0x7f, 160 random; exercises ristretto255_elligator twice + ge25519_p3_add + ristretto255_p3_tobytes | [x] |
| ed25519low-36 | _sodium_ristretto255_frombytes | valid canonical ristretto encodings (from from_hash) and the identity (all-zero) -> rc=0, full p3 compared | [x] |
| ed25519low-37 | _sodium_ristretto255_frombytes | non-canonical: bit 255 set, s[0] odd, s >= p (p, p+1, p+2, p+3), 2^255-1 -> rc=-1, struct untouched (canary preserved) | [x] |
| ed25519low-38 | _sodium_ristretto255_frombytes | canonical but not a valid element (ristretto255_sqrt_ratio_m1 non-square, or isnegative(T), or Y=0) -> rc=-1 with the struct fully written | [x] |
| ed25519low-39 | _sodium_ristretto255_p3_tobytes | ristretto-decoded points, a*B, torsion, off-subgroup, 16 synthetic p3, all-zero p3; covers rotate=0 and rotate=1 (isnegative(T*z_inv)) | [x] |
| ed25519low-40 | crypto_core_ed25519_is_valid_point | valid subgroup point / non-canonical encoding / off-curve / small-order (identity, y=0 order-4, y=p-1 order-2) / canonical non-small-order point outside the prime-order subgroup (a*B + T, 9 cases); both 0 and 1 observed | [x] |
| ed25519low-41 | crypto_core_ed25519_add | (valid,valid), (invalid,valid), (valid,invalid), (invalid,invalid), P+P, and ~450 corpus pairs; rc + full 32B output (canary-prefilled) compared | [x] |
| ed25519low-42 | crypto_core_ed25519_sub | same four validity combinations + P-P + corpus pairs | [x] |
| ed25519low-43 | crypto_core_ed25519_random | RNG-driven: 16 calls per library; bytes not comparable, so each result is asserted valid by BOTH libraries' is_valid_point (cross-validated) | [x] |
| ed25519low-44 | crypto_core_ed25519_scalar_random | RNG-driven: 32 calls per library; asserted canonical, non-zero, and top 3 bits of byte 31 clear (the `&= 0x1f` + rejection loop) | [x] |
| ed25519low-45 | crypto_core_ed25519_scalar_reduce | 64B inputs: all specials (0, 1, L-1, L, L+1, 2^252, all-0xff, all-0x80, high-word only) + 200 random | [x] |
| ed25519low-46 | crypto_core_ed25519_scalar_invert | s=0 (rc=-1, output still written) and s!=0 (rc=0); 213 scalars | [x] |
| ed25519low-47 | crypto_core_ed25519_scalar_negate | 213 scalars incl. 0, 1, L-1, L, L+1, 0xff.. (sodium_sub of a 64B borrow chain + sc25519_reduce) | [x] |
| ed25519low-48 | crypto_core_ed25519_scalar_complement | same 213 scalars (t_[0]++ variant of negate) | [x] |
| ed25519low-49 | crypto_core_ed25519_scalar_add | 213 special pairs + 200 random pairs (sodium_add 32B carry + scalar_reduce) | [x] |
| ed25519low-50 | crypto_core_ed25519_scalar_sub | 213 special pairs + 200 random pairs (negate then add) | [x] |
| ed25519low-51 | crypto_core_ed25519_scalar_mul | 213 special pairs + 200 random pairs | [x] |
| ed25519low-52 | crypto_core_ed25519_scalar_is_canonical | 213 scalars incl. L-1/L/L+1; both 0 and 1 observed | [x] |
| ed25519low-53 | crypto_core_ed25519_from_string_nu | hash_alg=SHA256(1) and SHA512(2) x ctx_len in {0,1,52,255,256,300} x msg_len in {0,1,3,63,64,65,1000} (84 combos); n=1 point, big-endian byte reversal of a 48-byte h2c output | [x] |
| ed25519low-54 | crypto_core_ed25519_from_string | same 84 combos; n=2 -> two ge25519_from_hash points then crypto_core_ed25519_add | [x] |
| ed25519low-55 | crypto_core_ed25519_scalar_from_string | same 84 combos; 48-byte h2c output reversed then scalar_reduce | [x] |
| ed25519low-56 | crypto_core_ed25519_from_string_nu/_from_string/_scalar_from_string, crypto_core_ristretto255_from_string/_scalar_from_string | out-of-range hash_alg in {0, 3, -1, 7, INT_MAX} -> rc=-1, output buffer untouched | [x] |
| ed25519low-57 | all five *_from_string entry points | ctx=NULL and msg=NULL with ctx_len=msg_len=0 (only the output pointer is `nonnull` in the C prototype) | [x] |
| ed25519low-58 | crypto_core_ed25519_bytes/_uniformbytes/_hashbytes/_scalarbytes/_nonreducedscalarbytes, crypto_core_ristretto255_bytes/_hashbytes/_scalarbytes/_nonreducedscalarbytes | constant size getters; value compared C vs Rust and against the header macros (32/32/64/32/64, 32/64/32/64) | [x] |
| ed25519low-59 | crypto_core_ristretto255_is_valid_point | valid from_hash outputs, identity (all-zero), non-canonical (bit 255 set / s[0] flipped odd / s>=p), 200 random, ed25519 torsion encodings; both 0 and 1 observed | [x] |
| ed25519low-60 | crypto_core_ristretto255_add, crypto_core_ristretto255_sub | (valid,valid), (valid,invalid), (invalid,valid), (invalid,invalid), P+P, P-P; ~600 corpus pairs; rc + full output compared | [x] |
| ed25519low-61 | crypto_core_ristretto255_from_hash | 66 h[64] incl. all-0x00/all-0xff; always returns 0 | [x] |
| ed25519low-62 | crypto_core_ristretto255_random, crypto_core_ristretto255_scalar_random | RNG-driven; results cross-validated with both libraries' is_valid_point / scalar_is_canonical instead of byte comparison | [x] |
| ed25519low-63 | crypto_core_ristretto255_scalar_reduce/_invert/_negate/_complement/_add/_sub/_mul/_is_canonical | thin wrappers over the ed25519 scalar API; 133 special + 120 random 32B scalars, 133 special + 120 random 64B for reduce | [x] |
| ed25519low-64 | crypto_core_ristretto255_from_string, crypto_core_ristretto255_scalar_from_string | 84 (alg, ctx_len, msg_len) combos + invalid alg + NULL ctx/msg | [x] |
| ed25519low-65 | cross-layer identities | scalarmult_base(a) == scalarmult(a, B) inside each library, and crypto_core_ed25519_add == p3_add o frombytes composed with p3_tobytes, for 32 random a (a[31] &= 0x7f) | [x] |

| h2c-1 | _sodium_core_h2c_string_to_hash | hash_alg=CORE_H2C_SHA256(1), ctx_len<=0xff, h_len grid {0,1,2,15,16,31,32,33,47,48,49,63,64,65,95,96,97,127,128,129,159,160,191,192,193,223,224,254,255} × msg_len {0,1,37} (loop skipped / partial memcpy / exact multiple of 32 / +1) | [x] |
| h2c-2 | _sodium_core_h2c_string_to_hash | hash_alg=CORE_H2C_SHA512(2), ctx_len<=0xff, same h_len grid (loop step 64: exact multiples, ±1, tail memcpy of h_len-i<64) | [x] |
| h2c-3 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, ctx_len grid {0,1,2,16,31,32,33,63,64,65,127,128,129,253,254,255} (short-DST path, no pre-hash) × h_len {0,1,32,48,64,96,255} | [x] |
| h2c-4 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, ctx_len grid {256,257,258,300,511,512,513,1000,4096} → "H2C-OVERSIZE-DST-" pre-hash branch (ctx:=u0, ctx_len:=32) incl. the u0-aliasing quirk where the main hash overwrites u0 before the per-block loop re-reads it as DST | [x] |
| h2c-5 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, ctx_len<=0xff grid, h_len {0,1,32,48,64,96,255} | [x] |
| h2c-6 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, ctx_len>0xff → oversize pre-hash branch (ctx:=u0, ctx_len:=64) + same u0 aliasing quirk | [x] |
| h2c-7 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, msg_len grid {0,1,2,55,56,63,64,65,111,112,127,128,129,191,192,255,256,1000,4096,10000} (message absorbed after the 64-byte zero block ⇒ straddles SHA-256 block boundaries) | [x] |
| h2c-8 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, same msg_len grid (message absorbed after the 128-byte zero block) | [x] |
| h2c-9 | _sodium_core_h2c_string_to_hash | h=NULL with h_len=0 (memcpy loop never entered), both hash ids | [x] |
| h2c-10 | _sodium_core_h2c_string_to_hash | ctx=NULL, ctx_len=0 (update() early-returns on inlen==0), h_len {0,1,32,64}, both hash ids | [x] |
| h2c-11 | _sodium_core_h2c_string_to_hash | msg=NULL, msg_len=0, h_len {0,1,32,64}, both hash ids | [x] |
| h2c-12 | _sodium_core_h2c_string_to_hash | h=NULL, ctx=NULL, msg=NULL, all lengths 0, both hash ids | [x] |
| h2c-13 | _sodium_core_h2c_string_to_hash | non-NULL but empty ctx and msg, h_len=64, both hash ids | [x] |
| h2c-14 | _sodium_core_h2c_string_to_hash | 400 random (seed 0xF00DBABE) combinations: hash_alg∈{1,2}, h_len∈[0,255], ctx_len∈[0,599] (spans the 0xff pre-hash boundary), msg_len∈[0,399] | [x] |
| h2c-15 | crypto_core_ed25519_from_string | _string_to_points(n=2) ⇒ h_len=96; hash_alg∈{1,2} × ctx_len {0,1,16,63,64,65,254,255,256,257,512,1000} × msg_len {0,1,63,64,65,127,128,129,1000} | [x] |
| h2c-16 | crypto_core_ed25519_from_string | ctx=NULL/msg=NULL/both NULL with zero lengths, both hash ids | [x] |
| h2c-17 | crypto_core_ed25519_from_string | 60 random (seeded) ctx_len∈[0,399] × msg_len∈[0,299] × hash_alg∈{1,2} | [x] |
| h2c-18 | crypto_core_ed25519_from_string_nu | _string_to_points(n=1) ⇒ h_len=48; same ctx_len/msg_len/hash_alg grid + NULL cases + 60 random | [x] |
| h2c-19 | crypto_core_ed25519_scalar_from_string | h_len=HASH_SC_L=48, 48-byte big-endian reversal into a 64-byte zero-padded buffer then sc25519_reduce; same full grid + NULL cases + 60 random | [x] |
| h2c-20 | crypto_core_ristretto255_from_string | _string_to_element ⇒ h_len=crypto_core_ristretto255_HASHBYTES=64; same full grid + NULL cases + 60 random | [x] |
| h2c-21 | crypto_core_ristretto255_scalar_from_string | pure delegation to crypto_core_ed25519_scalar_from_string; same full grid + NULL cases + 60 random | [x] |
| h2c-22 | crypto_core_ristretto255_scalar_from_string, crypto_core_ed25519_scalar_from_string | equivalence cross-check: both libs must produce byte-identical output for the two entry points | [x] |
| h2c-23 | crypto_core_ed25519_from_string, crypto_core_ed25519_from_string_nu, _sodium_core_h2c_string_to_hash, _sodium_ge25519_from_hash, crypto_core_ed25519_add | layer-composition cross-check: from_string == add(from_hash(rev(h[0..48])), from_hash(rev(h[48..96]))) and _nu == from_hash(rev(h48)), for short and oversize (300/400-byte) ctx and both hash ids | [x] |
| h2c-24 | crypto_core_ristretto255_from_string, _sodium_core_h2c_string_to_hash, _sodium_ristretto255_from_hash | layer-composition cross-check: from_string == ristretto255_from_hash(h2c(64)), short + oversize ctx, both hash ids | [x] |
| h2c-25 | _sodium_ge25519_from_uniform | 10 edge 32-byte inputs (0, 1, 2^256-1, p-1, p, p+1, bit255 only, bit253 only (x_sign source), top-3-bits set, 0x55.. with s[31]=0x7f) + 60 random | [x] |
| h2c-26 | _sodium_ge25519_from_uniform | s == r (fully aliased in/out buffer; legal because the C does memcpy(s,r,32) first), all of the above inputs, plus equality with the non-aliased result | [x] |
| h2c-27 | _sodium_ge25519_from_hash | fe25519_reduce64 path: 15 edge 64-byte inputs (all-zero, all-0xff, lo=1, hi=1, p-1/p/p+1 in both halves, h[31]=h[63]∈{0x20,0x40,0x80,0xe0,0xff} to exercise the ((x>>5)^optblocker)>>2 carry terms *19 / *722) + 60 random | [x] |
| h2c-28 | _sodium_ristretto255_from_hash | same 15 edge 64-byte inputs + 60 random (two ristretto255_elligator calls + ge25519_p3_add + ristretto255_p3_tobytes) | [x] |
| h2c-29 | crypto_core_ristretto255_from_hash | public wrapper: same inputs, return value always 0, output equals the raw _sodium_ristretto255_from_hash in both libs | [x] |
| h2c-30 | crypto_core_ristretto255_from_hash | p aliasing the first 32 bytes of the 64-byte input h (both read r0/r1 before writing s) | [x] |
| h2c-31 | _sodium_core_h2c_string_to_hash | h_len=255 (the largest value allowed by assert(h_len <= 0xff)), both hash ids: 8 SHA-256 blocks with a 31-byte tail / 4 SHA-512 blocks with a 63-byte tail | [x] |

| hash-1 | crypto_hash_bytes, crypto_hash_primitive | constant getters, no input; value + string content compared | [x] |
| hash-2 | crypto_hash | in=NULL, inlen=0 | [x] |
| hash-3 | crypto_hash | inlen in {0,1,2,3,7,8,9,15,16,31,32,55..57,63..65,71..73,111..113,127..129,135..137,143,144,167..169,191,192,200,255,256,271,272,335..337,1000,4096} x 3 random msgs; also asserted identical to crypto_hash_sha512 | [x] |
| hash-4 | crypto_hash_sha256_bytes, crypto_hash_sha256_statebytes | constant getters (32 / 104) | [x] |
| hash-5 | crypto_hash_sha256 | one-shot, full SIZES list x 3 random messages each | [x] |
| hash-6 | crypto_hash_sha256 | in=NULL, inlen=0 (update() early-return path) | [x] |
| hash-7 | crypto_hash_sha256_init, _update, _final | 1 chunk = whole message; full 104-byte state compared after init/update/final | [x] |
| hash-8 | crypto_hash_sha256_update | inlen==0 chunk interleaved before/after data chunks (early return, count untouched) | [x] |
| hash-9 | crypto_hash_sha256_update | inlen < 64-r: buffer-only path, no transform | [x] |
| hash-10 | crypto_hash_sha256_update | inlen == 64-r exactly: one transform, `while (inlen>=64)` not entered | [x] |
| hash-11 | crypto_hash_sha256_update | inlen > 64-r: fill+transform, multi-block while loop, `inlen &= 63` tail copy | [x] |
| hash-12 | crypto_hash_sha256_init/_update/_final | 2,3,4,5 random chunks; plus [1,n-1], [n-1,1], [64,n-64], [64,0,n-64], [128,n-128], byte-at-a-time for n<=64 | [x] |
| hash-13 | crypto_hash_sha256_final | SHA256_Pad with r < 56 (single final transform) | [x] |
| hash-14 | crypto_hash_sha256_final | SHA256_Pad with r >= 56 (extra transform + memset(buf,0,56)) — n in {56..63} mod 64 | [x] |
| hash-15 | crypto_hash_sha256_final | state fully zeroized by sodium_memzero after final | [x] |
| hash-16 | crypto_hash_sha256 + streaming | 300003-byte seeded-RNG input, one-shot and random 1..9000-byte chunks | [x] |
| hash-17 | crypto_hash_sha512_bytes, crypto_hash_sha512_statebytes | constant getters (64 / 208) | [x] |
| hash-18 | crypto_hash_sha512 | one-shot, full SIZES list x 3 random messages each | [x] |
| hash-19 | crypto_hash_sha512 | in=NULL, inlen=0 | [x] |
| hash-20 | crypto_hash_sha512_init, _update, _final | 1 chunk; full 208-byte state compared after init/update/final | [x] |
| hash-21 | crypto_hash_sha512_update | inlen==0 chunk (early return) | [x] |
| hash-22 | crypto_hash_sha512_update | inlen < 128-r: buffer-only path | [x] |
| hash-23 | crypto_hash_sha512_update | inlen == 128-r exactly: one transform, no while loop | [x] |
| hash-24 | crypto_hash_sha512_update | inlen > 128-r: multi-block while loop + `inlen &= 127` tail | [x] |
| hash-25 | crypto_hash_sha512_init/_update/_final | 2..5 random chunks + [128,n-128], [128,0,n-128], [256,n-256], [1,n-1], [n-1,1] | [x] |
| hash-26 | crypto_hash_sha512_update | count[1] low-word accumulation / count[0] carry (128-bit bit counter) exercised via 300KB streaming | [x] |
| hash-27 | crypto_hash_sha512_final | SHA512_Pad with r < 112 (single final transform) | [x] |
| hash-28 | crypto_hash_sha512_final | SHA512_Pad with r >= 112 (extra transform + memset(buf,0,112)) — n in {112..127} mod 128 | [x] |
| hash-29 | crypto_hash_sha512_final | state fully zeroized after final | [x] |
| hash-30 | crypto_hash_sha512 + streaming | 300003-byte input, one-shot and random chunking | [x] |
| hash-31 | crypto_hash_sha3256_bytes, crypto_hash_sha3256_statebytes | constant getters (32 / 256) | [x] |
| hash-32 | crypto_hash_sha3512_bytes, crypto_hash_sha3512_statebytes | constant getters (64 / 256) | [x] |
| hash-33 | crypto_hash_sha3256 | one-shot, full SIZES list x 3 random messages; plus FIPS-202 KAT for "" | [x] |
| hash-34 | crypto_hash_sha3512 | one-shot, full SIZES list x 3 random messages; plus FIPS-202 KAT for "" | [x] |
| hash-35 | crypto_hash_sha3256, crypto_hash_sha3512 | in=NULL, inlen=0 (empty input) | [x] |
| hash-36 | crypto_hash_sha3256_init/_update/_final | rate=136: 1 chunk; state compared after every call | [x] |
| hash-37 | crypto_hash_sha3512_init/_update/_final | rate=72: 1 chunk; state compared after every call | [x] |
| hash-38 | crypto_hash_sha3*_update | inlen==0 chunks interleaved (offset/rate untouched) | [x] |
| hash-39 | crypto_hash_sha3*_update | `offset != 0 && inlen > 0` partial-block XOR path with chunk_size > inlen (clamped) | [x] |
| hash-40 | crypto_hash_sha3*_update | `offset == rate && inlen > 0` -> permute then offset=0 (split exactly on rate: [136,n-136] / [72,n-72]) | [x] |
| hash-41 | crypto_hash_sha3*_update | full-rate `while (inlen-consumed >= rate)` loop with and without trailing bytes (offset left == rate) | [x] |
| hash-42 | crypto_hash_sha3*_update | 2..5 random chunks, [1,n-1], [n-1,1], byte-at-a-time for n<=64 | [x] |
| hash-43 | crypto_hash_sha3*_final | offset == rate at final -> permute, offset=0, then normal padding | [x] |
| hash-44 | crypto_hash_sha3*_final | offset == rate-1 -> single-byte padding `0x06 ^ 0x80` (n = 71/135 mod rate) | [x] |
| hash-45 | crypto_hash_sha3*_final | normal padding: 0x06 at offset, 0x80 at rate-1 | [x] |
| hash-46 | crypto_hash_sha3*_update / _final | phase == FINALIZED on entry (see errors table) | [x] |
| hash-47 | crypto_hash_sha3256, crypto_hash_sha3512 | 280007-byte seeded-RNG input, one-shot vs random 1..5000-byte chunks | [x] |
| hash-48 | crypto_core_keccak1600_statebytes | constant getter (224) | [x] |
| hash-49 | crypto_core_keccak1600_init | canary-filled 224-byte state: only bytes 0..200 zeroed, 200..224 untouched | [x] |
| hash-50 | crypto_core_keccak1600_permute_24 | all-zero state, applied 1..4 times consecutively (round-constant order) | [x] |
| hash-51 | crypto_core_keccak1600_permute_12 | all-zero state, applied 1..4 times consecutively (constants 12..23) | [x] |
| hash-52 | crypto_core_keccak1600_permute_24 / _permute_12 | all-0xFF state | [x] |
| hash-53 | crypto_core_keccak1600_xor_bytes | 200 random iterations x 6 steps: random offset 0..199, random length 0..200-offset — covers the unaligned head loop, the 8-byte body loop and the tail loop | [x] |
| hash-54 | crypto_core_keccak1600_xor_bytes | length == 0 at offsets {0,1,7,8,9,199} (no-op) | [x] |
| hash-55 | crypto_core_keccak1600_extract_bytes | random offset/length, output canary-guarded, also checked to equal the raw state slice | [x] |
| hash-56 | crypto_core_keccak1600_extract_bytes | length == 0 (no write) | [x] |
| hash-57 | crypto_core_keccak1600_* mixed | random xor / extract / permute_24 / permute_12 sequences on a random 224-byte state | [x] |
| hash-58 | _sodium_keccak1600_ref_init | zeroes exactly KECCAK1600_STATEBYTES (200) of a canary buffer | [x] |
| hash-59 | _sodium_keccak1600_ref_xor_bytes | 150 iterations, offsets covering all 8 alignment classes x multiples of 8 | [x] |
| hash-60 | _sodium_keccak1600_ref_extract_bytes | random offset/length with output guard | [x] |
| hash-61 | _sodium_keccak1600_ref_permute_24, _permute_12 | 150 random 200-byte states, both applied in sequence | [x] |
| hash-62 | crypto_xof_shake128_blockbytes/_statebytes/_domain_standard | constant getters (168 / 256 / 0x1F) | [x] |
| hash-63 | crypto_xof_shake256_blockbytes/_statebytes/_domain_standard | constant getters (136 / 256 / 0x1F) | [x] |
| hash-64 | crypto_xof_turboshake128_blockbytes/_statebytes/_domain_standard | constant getters (168 / 256 / 0x1F) | [x] |
| hash-65 | crypto_xof_turboshake256_blockbytes/_statebytes/_domain_standard | constant getters (136 / 256 / 0x1F) | [x] |
| hash-66 | crypto_xof_shake128 / shake256 / turboshake128 / turboshake256 | one-shot: inlen over the full SIZES list x outlen in {0,1,2,7,8,31,32,63,64,71,72,73,135,136,137,167,168,169,200,271,272,273,335,336,337,504,1000} | [x] |
| hash-67 | crypto_xof_* | one-shot with in=NULL, inlen=0; plus SHAKE128/256("",32) KATs | [x] |
| hash-68 | crypto_xof_* | one-shot with outlen == 0 (out buffer untouched) | [x] |
| hash-69 | _sodium_shake128_ref / _sodium_shake256_ref / _sodium_turboshake128_ref / _sodium_turboshake256_ref | internal one-shot over the same inlen x outlen grid; also asserted equal to the public one-shot | [x] |
| hash-70 | crypto_xof_*_init + _update + _squeeze | 1 update, 1 squeeze; 256-byte state compared after every call | [x] |
| hash-71 | crypto_xof_*_update | inlen==0 chunks; `offset == RATE && inlen > 0` permute path (split exactly on the rate) | [x] |
| hash-72 | crypto_xof_*_update | partial-block path with clamped chunk_size, full-rate while loop with/without tail | [x] |
| hash-73 | crypto_xof_*_update | 2..5 random chunks + [rate, n-rate], [rate,0,n-rate], [2*rate,...], [1,n-1], [n-1,1] | [x] |
| hash-74 | crypto_xof_*_squeeze | one-shot squeeze of the whole output | [x] |
| hash-75 | crypto_xof_*_squeeze | multi-chunk squeeze: halves, [rate, outlen-rate], [1, outlen-1], doubling sizes 1,3,7,15,..., and 0-length squeezes at the start/end — all compared against the one-shot | [x] |
| hash-76 | crypto_xof_*_squeeze | `offset == RATE && outlen > 0` permute path; `offset != 0` partial-block extract; full-rate extract loop | [x] |
| hash-77 | crypto_xof_*_squeeze | repeated squeezes past several rate blocks (4 x rate+3 bytes after the first) | [x] |
| hash-78 | crypto_xof_*_init vs _init_with_domain(0x1F) | both must produce identical state and identical output | [x] |
| hash-79 | crypto_xof_*_init_with_domain | domain in {0x00,0x01,0x02,0x06,0x07,0x0B,0x1F,0x7F,0x80,0x81,0xA5,0xFE,0xFF} x inlen in {0,1,rate-2,rate-1,rate,rate+1,2rate-1,2rate,2rate+1} x outlen in {0,1,32,rate,rate+1,2rate+5} | [x] |
| hash-80 | crypto_xof_*_squeeze (finalize) | offset == RATE-1 special case: padding collapses to `domain ^ 0x80` (inlen ≡ rate-1 mod rate) | [x] |
| hash-81 | crypto_xof_*_squeeze (finalize) | offset == RATE at finalize -> permute then offset=0 then normal padding | [x] |
| hash-82 | crypto_xof_*_update after _squeeze | absorb/squeeze interleaving: update returns -1, permutes (24 rounds for shake, 12 for turboshake), resets, then squeeze re-finalizes | [x] |
| hash-83 | _sodium_*_ref_init / _ref_init_with_domain / _ref_update / _ref_squeeze | internal streaming API: init (both flavours), two updates, two squeezes; state compared after each call; result equal to the public one-shot | [x] |
| hash-84 | _sodium_*_ref_update after _ref_squeeze | internal absorb/squeeze interleaving with a non-standard domain (0x0B), returns -1 | [x] |
| hash-85 | crypto_xof_* | 260011-byte seeded-RNG input, 40009-byte output: one-shot vs random-chunk absorb (1..7000) + random-chunk squeeze (1..1000) | [x] |

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

| pwhash-1 | crypto_pwhash_alg_argon2i13, crypto_pwhash_alg_argon2id13, crypto_pwhash_alg_default, crypto_pwhash_bytes_min/max, crypto_pwhash_passwd_min/max, crypto_pwhash_saltbytes, crypto_pwhash_strbytes, crypto_pwhash_strprefix, crypto_pwhash_primitive, crypto_pwhash_opslimit_min/max/interactive/moderate/sensitive, crypto_pwhash_memlimit_min/max/interactive/moderate/sensitive | all 21 generic getters, value-checked against the header macros | [x] |
| pwhash-2 | crypto_pwhash_argon2i_alg_argon2i13, _bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/moderate/sensitive, _memlimit_min/max/interactive/moderate/sensitive | all 18 argon2i getters | [x] |
| pwhash-3 | crypto_pwhash_argon2id_alg_argon2id13, _bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/moderate/sensitive, _memlimit_min/max/interactive/moderate/sensitive | all 18 argon2id getters | [x] |
| pwhash-4 | crypto_pwhash_scryptsalsa208sha256_bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/sensitive, _memlimit_min/max/interactive/sensitive | all 15 scrypt getters (BYTES_MAX = min(SIZE_MAX,0x1fffffffe0), PASSWD_MAX = SIZE_MAX) | [x] |
| pwhash-5 | _crypto_pwhash_argon2_pick_best_implementation | no SIMD compiled -> always selects argon2_fill_segment_ref, returns 0 | [x] |
| pwhash-6 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2I13 (1), opslimit=3, memlimit=8192, outlen=16 | [x] |
| pwhash-7 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2ID13 (2), opslimit=1, memlimit=8192, outlen=16 | [x] |
| pwhash-8 | crypto_pwhash | out-of-range enum alg = 0, 3, -1, 999, INT_MIN, INT_MAX -> -1/EINVAL, full out buffer compared | [x] |
| pwhash-9 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen = 0, 1, 63, 64, 65, 200 x 3 random cases each, random salt per case | [x] |
| pwhash-10 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen = 15 (MIN-1, rejected), 16 (MIN), 17, 31, 32, 64, 200; canary-filled out buffer, full-buffer compare | [x] |
| pwhash-11 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit = 0, MIN-1, MIN, MIN+1, MIN+2, 2^32, UINT64_MAX | [x] |
| pwhash-12 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit = 0, 1, 8191, 8192, 8193 (not 1024-aligned), 9215, 16384, 65536, MEMLIMIT_MAX+1, SIZE_MAX | [x] |
| pwhash-13 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | alg argument = own alg (ok) / other family's alg / 0 / 3 / -1 / 999 (all rejected) | [x] |
| pwhash-14 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | out == passwd (same pointer) -> EINVAL | [x] |
| pwhash-15 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen = 0, 1, 32, 200 x 3 cases, opslimit=MIN, memlimit=8192, deterministic randombytes -> byte-exact 128-byte out buffer | [x] |
| pwhash-16 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | out-of-range opslimit/memlimit/passwdlen (7 rejection combinations), full out buffer compared | [x] |
| pwhash-17 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify, crypto_pwhash_str_verify | correct password (0), wrong password (-1/EINVAL), passwdlen=2^32 (EFBIG) | [x] |
| pwhash-18 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify, crypto_pwhash_str_verify | 21 malformed hash strings x 3 verifiers: valid-other-alg, corrupted b64 hash char, corrupted salt char, corrupted `m=` digit, empty, missing leading `$`, prefix only, truncated (half), missing hash field, over-long b64, over-short b64, trailing garbage, wrong version, leading-zero decimal, decimal > 2^32, `p=0`, `$7$` (scrypt) prefix | [x] |
| pwhash-19 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash, crypto_pwhash_str_needs_rehash | matching params -> 0, different opslimit -> 1, different memlimit -> 1 | [x] |
| pwhash-20 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash, crypto_pwhash_str_needs_rehash | 21 malformed strings x 4 (opslimit, memlimit) combos incl. opslimit>UINT32_MAX and memlimit=SIZE_MAX; plus a 128-char string (>= crypto_pwhash_STRBYTES) | [x] |
| pwhash-21 | crypto_pwhash_str | == crypto_pwhash_argon2id_str; byte-exact under deterministic RNG, cross-verified with crypto_pwhash_str_verify | [x] |
| pwhash-22 | crypto_pwhash_str_alg | alg = 1 and 2 (byte-exact out), plus opslimit/memlimit rejection paths for both algs | [x] |
| pwhash-23 | _sodium_argon2i_hash_raw, _sodium_argon2id_hash_raw | 9 (t_cost, m_cost, parallelism) combos x hashlen 16/17/31/32/64/65/100, random pwd (1..40) and salt (8..32) | [x] |
| pwhash-24 | _sodium_argon2i_hash_encoded, _sodium_argon2id_hash_encoded | same 9 x 7 grid, 256-byte encoded buffer, canary-compared in full | [x] |
| pwhash-25 | _sodium_argon2i_hash_raw, _sodium_argon2id_hash_raw | rejected parameter combos with exact ARGON2_* codes: t_cost=0, m_cost=7, m_cost=0, lanes=0, m_cost<8*lanes (2 shapes), lanes=0x1000000 | [x] |
| pwhash-26 | _sodium_argon2i_hash_encoded | hashlen = 0/15 (OUTPUT_TOO_SHORT), 16 (ok), 2^32 and SIZE_MAX (OUTPUT_TOO_LONG) | [x] |
| pwhash-27 | _sodium_argon2i_hash_encoded | pwdlen = 2^32 / SIZE_MAX (PWD_TOO_LONG); saltlen = 0/1/7 (SALT_TOO_SHORT), 2^32 / SIZE_MAX (SALT_TOO_LONG) | [x] |
| pwhash-28 | _sodium_argon2i_hash_encoded | pwd = NULL with pwdlen = 0 (ok) and 1 (PWD_PTR_MISMATCH); salt = NULL with saltlen 0 / 8 | [x] |
| pwhash-29 | _sodium_argon2id_hash_encoded | encodedlen too small (1, 5, 12, 13, 20, 26, 27) -> ARGON2_ENCODING_FAIL | [x] |
| pwhash-30 | _sodium_argon2_hash | type = 1, 2 (ok) and 0, 3, -1, 999 (out-of-range enum -> ARGON2_INCORRECT_TYPE); hash AND encoded requested simultaneously | [x] |
| pwhash-31 | _sodium_argon2_hash | hash = NULL && encoded = NULL (no output requested); encoded != NULL with encodedlen = 0 (encoding skipped) | [x] |
| pwhash-32 | _sodium_argon2_verify, _sodium_argon2i_verify, _sodium_argon2id_verify | 2 types x 3 random (t_cost, m_cost, pwd, salt) cases: correct password, wrong password (VERIFY_MISMATCH), wrong type, out-of-range type (0/3/-1/999), empty encoded string | [x] |
| pwhash-33 | _sodium_argon2_validate_inputs | 30-row matrix: NULL context, out=NULL, outlen 0/1/15/16/UINT32_MAX, pwd NULL x len, salt NULL x len, saltlen 0/1/7/8, secret NULL/set x len, ad NULL/set x len, lanes 0/0xFFFFFF/0x1000000, m_cost 0/1/7/8/15/16/UINT32_MAX, m_cost<8*lanes, t_cost 0/UINT32_MAX, threads 0/0xFFFFFF/0x1000000; context must not be mutated | [x] |
| pwhash-34 | _sodium_argon2_ctx | 6 (t_cost, m_cost, lanes) combos x 2 random cases x outlen 16/32/64/80 x type 1/2, secret and ad both non-NULL | [x] |
| pwhash-35 | _sodium_argon2_ctx | out-of-range type = 0, 3, -1, 999, INT_MIN -> ARGON2_INCORRECT_TYPE; validation failures propagated unchanged; NULL context | [x] |
| pwhash-36 | _sodium_argon2_initialize, _sodium_argon2_fill_memory_blocks, _sodium_argon2_finalize | 5 (t_cost, m_cost, lanes, type) instances driven directly; the 2 first blocks per lane compared after initialize, the WHOLE memory region (memory_blocks x 1024 B) compared after every pass, out compared after finalize, region/pseudo_rands freed | [x] |
| pwhash-37 | _sodium_argon2_fill_segment_ref | same 5 instances, every (pass, slice, lane) segment driven by hand; region compared after each complete pass (exercises starting_index=2, prev_offset wraparound, data-independent vs data-dependent addressing for Argon2i/Argon2id) | [x] |
| pwhash-38 | _sodium_argon2_initialize, _sodium_argon2_fill_memory_blocks, _sodium_argon2_finalize, _sodium_argon2_fill_segment_ref | NULL-pointer / lanes==0 early-return paths | [x] |
| pwhash-39 | _sodium_argon2_encode_string | 8 random (saltlen 8..32, outlen 16..65) x m_cost/t_cost/lanes from {8,9,100,65536,UINT32_MAX} x {1,2,7,UINT32_MAX,3} x {1..5}, type 1/2/0/3/-1, dst_len = 0/1/5/11/12/13/header_len-1/header_len/need/need+1/512 | [x] |
| pwhash-40 | _sodium_argon2_encode_string | invalid ctx (outlen=0, lanes=0, out=NULL) -> validation code returned after the prefix has been written | [x] |
| pwhash-41 | _sodium_argon2_decode_string | 41 input strings x type 1/2/0/3/-1/999 x 4 (max saltlen, max outlen) shapes incl. (0,0); all out-params and both scratch buffers compared | [x] |
| pwhash-42 | _sodium_blake2b_long | outlen = 0,1,2,15,16,31,32,63,64 (= BYTES_MAX, single-shot path), 65,66,95,96,97,127,128,129,200,1024,1025 (multi-block extension path) x inlen = 0,1,4,63,64,72,128,1024; canary buffer compared in full; in=NULL with inlen=0 | [x] |
| pwhash-43 | crypto_pwhash_scryptsalsa208sha256_ll | 11 (N, r, p) combos {N=2..1024, r=1..8, p=1..3} x buflen = 0,1,16,31,32,33,64,100 x 2 random (passwd, salt) cases | [x] |
| pwhash-44 | _sodium_escrypt_kdf_nosse | same grid via a caller-managed escrypt_local_t, called twice per region so the "region already large enough" reuse path is taken; output cross-checked against crypto_pwhash_scryptsalsa208sha256_ll | [x] |
| pwhash-45 | crypto_pwhash_scryptsalsa208sha256_ll | rejected params with exact errno: N=0/1/3/5/6/1000/0xFFFFFFFF (EINVAL), N=2^32/UINT64_MAX (EFBIG), r=0, p=0, r=0&&p=0 (EINVAL), r*p=2^30 and r=p=0xFFFFFFFF (EFBIG), buflen=2^37 (EFBIG), N=2^31 & r=2^27 (ENOMEM) | [x] |
| pwhash-46 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_alloc_region failure (N=2^30, r=2^26 -> ~2^63 bytes) -> -1 | [x] |
| pwhash-47 | _sodium_escrypt_PBKDF2_SHA256 | c = 0, 1, 2, 3, 10 x dkLen = 0,1,2,31,32,33,63,64,65,100,128 x 4 random (passwd, salt) cases; canary buffer compared in full | [x] |
| pwhash-48 | _sodium_escrypt_alloc_region, _sodium_escrypt_free_region | size = 0,1,63,64,65,4096,2^20: returns 64-byte-aligned `aligned` within [base, base+63], records size, frees and re-zeroes the region | [x] |
| pwhash-49 | _sodium_escrypt_alloc_region | size = SIZE_MAX and SIZE_MAX-62 (size+63 overflows) -> NULL / size 0 / ENOMEM; size = 2^62 (malloc failure) | [x] |
| pwhash-50 | _sodium_escrypt_init_local, _sodium_escrypt_free_local | init_local zeroes a dirty region and returns 0; free_local/free_region on a zeroed region are no-ops returning 0 | [x] |
| pwhash-51 | _sodium_escrypt_gensalt_r | srclen = 0,1,2,3,4,31,32,33,48 x 10 (N_log2, r, p) combos incl. N_log2 = 0/63/64/255 and r*p >= 2^30 and r=p=0 x buflen = 0/1/need-1/need/need+1/128; full 256-byte buffer compared | [x] |
| pwhash-52 | _sodium_escrypt_parse_setting | 5 gensalt_r-produced settings (N_log2 = 0,1,10,14,63) + 11 malformed: empty, `$`, `$7`, `$7$`, `$8$...`, missing `$`, invalid N_log2/r/p characters, salt with `$hash` suffix, truncated r field; returned offset and all three out-params compared | [x] |
| pwhash-53 | _sodium_escrypt_r | 5 (N_log2, r, p) settings x passwdlen 0/1/32 x buflen 102 (exact `need`)/103/200, deterministic RNG -> full 256-byte buffer byte-exact | [x] |
| pwhash-54 | _sodium_escrypt_r | buflen = 0/1/50/101 (< need) -> NULL; invalid settings -> NULL; buf = NULL -> NULL with no randombytes consumed | [x] |
| pwhash-55 | crypto_pwhash_scryptsalsa208sha256 | 7 (opslimit, memlimit) combos incl. (0,0) and (1,0) which exercise both pickparams branches x outlen 16/17/32/64/100 x 3 random passwd cases | [x] |
| pwhash-56 | crypto_pwhash_scryptsalsa208sha256 | outlen = 0/1/15 (< BYTES_MIN) -> EINVAL; out == passwd -> EINVAL | [x] |
| pwhash-57 | crypto_pwhash_scryptsalsa208sha256_str | 4 (opslimit, memlimit) combos x passwdlen 0/1/32 x 3 cases, deterministic RNG -> byte-exact 102-byte string, `$7$` prefix, NUL at index 101 | [x] |
| pwhash-58 | crypto_pwhash_scryptsalsa208sha256_str_verify | correct password (0), wrong password (-1) | [x] |
| pwhash-59 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | matching params -> 0; 3 other (opslimit, memlimit) combos | [x] |
| pwhash-60 | crypto_pwhash_scryptsalsa208sha256_str_verify, _str_needs_rehash | 6 malformed strings: empty, short, 101 garbage chars, right length + `$7$` + invalid base64, 102 bytes with NO NUL terminator (sodium_strnlen boundary), 102 chars + NUL | [x] |

| sign-1 | crypto_scalarmult_bytes, crypto_scalarmult_scalarbytes, crypto_scalarmult_primitive | value + string content ("curve25519") vs C | [x] |
| sign-2 | crypto_scalarmult_curve25519_bytes, crypto_scalarmult_curve25519_scalarbytes | constant getters (32/32) | [x] |
| sign-3 | crypto_scalarmult_ed25519_bytes, crypto_scalarmult_ed25519_scalarbytes | constant getters (32/32) | [x] |
| sign-4 | crypto_scalarmult_ristretto255_bytes, crypto_scalarmult_ristretto255_scalarbytes | constant getters (32/32) | [x] |
| sign-5 | crypto_sign_bytes, _seedbytes, _publickeybytes, _secretkeybytes, _messagebytes_max, _statebytes, _primitive | generic dispatcher getters (64/32/32/64/SIZE_MAX-64/208/"ed25519") | [x] |
| sign-6 | crypto_sign_ed25519_bytes, _seedbytes, _publickeybytes, _secretkeybytes, _messagebytes_max, crypto_sign_ed25519ph_statebytes | ed25519 getters; statebytes == crypto_sign_statebytes | [x] |
| sign-7 | crypto_scalarmult_curve25519_ref10_implementation (exported data object) | `both_data!`, both function pointers (`mult`, `mult_base`) invoked directly: 24 random scalars, 24 random scalar/point pairs | [x] |
| sign-8 | crypto_scalarmult_curve25519_ref10_implementation.mult | all 7 blocklist entries + their bit-255-set variants through the raw function pointer | [x] |
| sign-9 | _crypto_scalarmult_curve25519_pick_best_implementation | called 3x on both libs (HAVE_AVX_ASM undefined ⇒ always ref10); curve25519 mult + blocklist rejection re-verified afterwards | [x] |
| sign-10 | crypto_scalarmult_curve25519_base, crypto_scalarmult_base | 40 scalars: all-zero, all-0xff, all-0x01, L, 2L, 7L, low-3-bits-only, bit-255-only, pre-clamped, 30 random; clamp-invariance (n vs clamp(n)) | [x] |
| sign-11 | crypto_scalarmult_curve25519_base | output aliases the scalar (`q == n`, the C uses `t = q` as scratch), 8 random scalars, full canary buffer compared | [x] |
| sign-12 | crypto_scalarmult_curve25519, crypto_scalarmult | 30 random scalar × random point pairs; dispatcher output compared against the curve25519 entry point | [x] |
| sign-13 | crypto_scalarmult_curve25519 | X25519 ECDH: 16 pairs, `a*base(b) == b*base(a)` | [x] |
| sign-14 | crypto_scalarmult_curve25519 | non-canonical point encodings p+2 … p+18 (byte 31 = 0x7f) accepted and equal to the canonical encoding of 2 … 18 | [x] |
| sign-15 | crypto_scalarmult_curve25519 | bit 255 of the point set vs cleared (masked away by fe25519_frombytes), 8 random points | [x] |
| sign-16 | crypto_scalarmult_curve25519 | scalar clamping: raw n vs clamp(n) = (n[0]&248, n[31]&127\|64), 12 random pairs | [x] |
| sign-17 | crypto_scalarmult_curve25519 | output aliases the scalar (`q == n`) and the point (`q == p`), 8 pairs each | [x] |
| sign-18 | crypto_scalarmult_curve25519 vs implementation->mult | 270 candidate points (blocklist ± deltas, p…p+20, 100 random) comparing raw `mult` result and wrapper return, searching for the all-zero-output rejection | [x] |
| sign-19 | crypto_scalarmult_ed25519_base | 38 scalars: zero, 0xff…, 0x01…, k·L (k=1..7), 1, bit-255-only, L\|2^255, 25 random | [x] |
| sign-20 | crypto_scalarmult_ed25519_base_noclamp | same 38 scalars; k·L and scalars that mask to 0 give the identity ⇒ -1 with identity bytes written | [x] |
| sign-21 | crypto_scalarmult_ed25519_base vs _base_noclamp | clamp equivalence: base(n) == base_noclamp(n[0]&248, n[31]\|64 then &127), 12 random | [x] |
| sign-22 | crypto_scalarmult_ed25519_base, _base_noclamp | output aliases the scalar (`q == n`), 6 random scalars × 2 variants | [x] |
| sign-23 | crypto_scalarmult_ed25519 | 71 points (12 valid main-subgroup, 12 small-order encodings, 7 non-canonical y≥p, 40 random) × 6 scalars (0, 0xff…, L, 3L, random, 1) | [x] |
| sign-24 | crypto_scalarmult_ed25519_noclamp | same 71×6 matrix; k·L on a main-subgroup point ⇒ identity ⇒ -1 | [x] |
| sign-25 | crypto_scalarmult_ed25519 vs _noclamp | clamp equivalence on 6 valid points; `q == n` and `q == p` aliasing | [x] |
| sign-26 | crypto_scalarmult_ristretto255_base | 36 scalars: zero, 0xff…, k·L (k=1..7), bit-255-only, 1, 25 random; identity result ⇒ -1 with all-zero output | [x] |
| sign-27 | crypto_scalarmult_ristretto255 | 66 points (12 valid, all-zero identity encoding, all-0xff, 12 ed25519 small-order encodings, 40 random) × 5 scalars (0, 0xff…, L, 2L, random) | [x] |
| sign-28 | crypto_scalarmult_ristretto255, _base | output aliases the scalar (`q == n`) and the point (`q == p`), 5 each | [x] |
| sign-29 | crypto_sign_ed25519_seed_keypair, crypto_sign_seed_keypair | 28 seeds (zero, 0xff…, L, 25 random); deterministic ⇒ pk and sk bytes compared; sk[0..32]==seed, sk[32..64]==pk | [x] |
| sign-30 | crypto_sign_ed25519_keypair, crypto_sign_keypair | RNG-driven ⇒ return code only; per-library self-consistency (sk[32..]==pk) | [x] |
| sign-31 | crypto_sign_ed25519_sk_to_pk, crypto_sign_ed25519_sk_to_seed | 28 secret keys, canary-padded output, result matched against the keypair | [x] |
| sign-32 | crypto_sign_ed25519, crypto_sign | mlen ∈ {0,1,31,32,33,64,127,128,1000} × 2 keys, smlen_p non-NULL | [x] |
| sign-33 | crypto_sign_ed25519 | same message set with `smlen_p == NULL` (output must be identical) | [x] |
| sign-34 | crypto_sign_ed25519 | in-place signing: `m == sm + crypto_sign_BYTES` (memmove overlap), all message lengths | [x] |
| sign-35 | crypto_sign_ed25519_open, crypto_sign_open | valid signed message, `m != NULL, mlen_p != NULL`, all message lengths | [x] |
| sign-36 | crypto_sign_ed25519_open | `m == NULL, mlen_p != NULL` | [x] |
| sign-37 | crypto_sign_ed25519_open | `m != NULL, mlen_p == NULL` | [x] |
| sign-38 | crypto_sign_ed25519_open | `m == NULL, mlen_p == NULL` | [x] |
| sign-39 | crypto_sign_ed25519_open | in-place open (`m == sm`, memmove overlap) | [x] |
| sign-40 | crypto_sign_ed25519_open | every one of the 64 signature bytes flipped ⇒ -1, `*mlen_p = 0`, `m` zeroed over exactly mlen bytes (canary intact) | [x] |
| sign-41 | crypto_sign_ed25519_open | message body tampered at offset 0, mlen/2, mlen-1 | [x] |
| sign-42 | crypto_sign_ed25519_open | signature verified against a different public key | [x] |
| sign-43 | crypto_sign_ed25519_detached, crypto_sign_detached | mlen ∈ {0,1,31,32,33,64,127,128,1000}, siglen_p non-NULL (⇒ 64) | [x] |
| sign-44 | crypto_sign_ed25519_detached | same set with `siglen_p == NULL` | [x] |
| sign-45 | _crypto_sign_ed25519_detached | prehashed ∈ {0, 1, 2, -1} (C int, any non-zero is "true") × all message lengths; ph=0 must equal the public wrapper | [x] |
| sign-46 | _crypto_sign_ed25519_verify_detached | prehashed ∈ {0, 1, 2, -1} round-trip against the matching signature | [x] |
| sign-47 | crypto_sign_ed25519_verify_detached, crypto_sign_verify_detached | valid signature, all message lengths | [x] |
| sign-48 | crypto_sign_ed25519_verify_detached | 64 signature bytes × 3 bit masks (0x01/0x40/0x80) flipped ⇒ -1 | [x] |
| sign-49 | crypto_sign_ed25519_verify_detached | message tampered at 3 offsets; message length truncated by 1 | [x] |
| sign-50 | crypto_sign_ed25519_verify_detached | mlen = 0 with `m == NULL` | [x] |
| sign-51 | crypto_sign_ed25519ph_init, _update, _final_create | mlen ∈ {0,1,31,32,33,64,127,128,1000} × 2 passes, random chunk splits; FULL 208-byte state buffer compared after init and after every update (crypto_sign_ed25519ph_state is padding-free: uint64[8] + uint64[2] + uint8[128]) | [x] |
| sign-52 | crypto_sign_ed25519ph_final_create | `siglen_p == NULL` vs non-NULL on identical states | [x] |
| sign-53 | crypto_sign_ed25519ph_update | zero-length updates prepended and appended to the chunk list (pass 1) | [x] |
| sign-54 | crypto_sign_ed25519ph_final_verify | matching signature ⇒ 0; each of the 64 signature bytes flipped ⇒ -1; wrong public key ⇒ -1; state buffer compared each time | [x] |
| sign-55 | crypto_sign_init, crypto_sign_update, crypto_sign_final_create, crypto_sign_final_verify | generic dispatcher, single one-shot update; signature must equal the chunked ed25519ph one | [x] |
| sign-56 | _crypto_sign_ed25519_ref10_hinit | prehashed ∈ {0, 1, 2, -1, INT_MIN, INT_MAX}: full sha512 state compared, then finalized (ph=0 ⇒ SHA-512(""), ph!=0 ⇒ DOM2PREFIX absorbed); all non-zero values equivalent | [x] |
| sign-57 | crypto_sign_ed25519_pk_to_curve25519 | 20 valid ed25519 public keys; result == crypto_scalarmult_curve25519_base(sk_to_curve25519(sk)) | [x] |
| sign-58 | crypto_sign_ed25519_pk_to_curve25519 | 79 invalid keys: 12 small-order encodings, 7 non-canonical y≥p, 60 random (off-curve / off-main-subgroup) | [x] |
| sign-59 | crypto_sign_ed25519_sk_to_curve25519 | 20 valid secret keys + 20 random + all-zero + all-0xff (never fails) | [x] |
| sign-60 | crypto_sign_ed25519_open, crypto_sign_open | smlen = 0 … 63 (short), with mlen_p non-NULL and NULL, m non-NULL and NULL | [x] |
| sign-61 | crypto_sign_ed25519_detached / _verify_detached (cross-library) | keys from each library's own RNG, signed by C and verified by Rust and vice versa, mlen ∈ {0,1,33,128} | [x] |

| sodium-1 | sodium_memcmp | len=0 (incl. NULL/NULL), 1,2,7,8,15,16,17,31,32,33,64,100; equal, same-pointer, and differing at every byte position × delta {1,0x0f,0x80,0xff} | [x] |
| sodium-2 | sodium_compare | len=1 exhaustive (all 65536 byte pairs), checked against a reference little-endian bignum compare; result in {-1,0,1} | [x] |
| sodium-3 | sodium_compare | len=2 exhaustive over a 16-value boundary grid per byte (65536 pairs) + 4000 random pairs | [x] |
| sodium-4 | sodium_compare | len=0 (NULL/NULL), 3, 8, 16, 32; random pairs, pairs sharing a random high-order common prefix, and equal buffers | [x] |
| sodium-5 | sodium_is_zero | len=0 (NULL), 1,2,8,16,31,32,33,64; all-zero, single non-zero byte {1,0x80,0xff} at every position, random | [x] |
| sodium-6 | sodium_increment | nlen=0,1,2,3,7,8,9,11,12,13,16,23,24,25,32,64 (incl. the lengths the `HAVE_AMD64_ASM` variant special-cases); all-zero, all-0xff, 0xff-prefix of every length, random, plus 600 successive increments | [x] |
| sodium-7 | sodium_add | len=0,1,2,3,8,12,16,24,32,64,65; zero+zero, 0xff+0xff, full carry chains, 0xff-prefix chains, 30 random pairs, and `a == b` aliasing | [x] |
| sodium-8 | sodium_sub | len=0,1,2,3,8,12,16,24,32,64,65 (incl. 64, the `HAVE_AMD64_ASM` case); full borrow chains, 0-1 underflow, 30 random pairs, and `a == b` aliasing | [x] |
| sodium-9 | sodium_memzero | len=0,1,2,7,8,15,16,31,32,64,1000 × start offset 0,1,3 inside a larger buffer; `(NULL, 0)` | [x] |
| sodium-10 | sodium_stackzero | len=0,1,64,4096,100000 (empty body in this build: neither HAVE_C_VARARRAYS nor HAVE_ALLOCA) | [x] |
| sodium-11 | sodium_pad | blocksize=1,2,3,15,16,17,32,64 (power-of-two `&` branch and non-power-of-two `%` branch) × unpadded_buflen 0..2*blocksize+1 × max_buflen {sufficient, n+1, n, 0, n+blocksize}; whole buffer + canary compared | [x] |
| sodium-12 | sodium_pad | padded_buflen_p = NULL vs non-NULL | [x] |
| sodium-13 | sodium_unpad | round-trip of every sodium_pad result with the correct blocksize, and with blocksize {1, bs+1, bs-1, 0} | [x] |
| sodium-14 | sodium_unpad | padding corrupted at every position in the last block (0x80 barrier flipped); return value and *unpadded_buflen_p compared | [x] |
| sodium-15 | sodium_unpad | blocksize=1,2,16,17,64 × 200 fully random buffers each, padded_buflen bs..4*bs | [x] |
| sodium-16 | sodium_malloc | size=0,1,2,15,16,17,63,64,4095,4096,100000; buffer non-NULL, full contents compared (must be 0xdb), then written and re-read | [x] |
| sodium-17 | sodium_allocarray | (count,size) = (0,0),(0,32),(1,0),(1,32),(7,13),(100,64); contents compared | [x] |
| sodium-18 | sodium_free | free of every sodium_malloc/sodium_allocarray result, plus sodium_free(NULL) | [x] |
| sodium-19 | sodium_mlock, sodium_munlock | on every allocation size above and on (NULL, 0); return value + errno compared, and munlock's zeroing of the buffer verified | [x] |
| sodium-20 | sodium_mprotect_noaccess, sodium_mprotect_readonly, sodium_mprotect_readwrite | on a live sodium_malloc pointer and on NULL; return value + errno compared | [x] |
| sodium-21 | _sodium_alloc_init | called 3× (refills the canary via randombytes_buf); return value compared | [x] |
| sodium-22 | sodium_bin2hex | bin_len 0..64 × {all-zero, all-0xff, counting, all 256 byte values in the last position, 10 random} × hex_maxlen = 2n+{1,2,8}; return pointer, full buffer + canary, and only 2n+1 bytes written | [x] |
| sodium-23 | sodium_hex2bin | 35 hand-written inputs (valid, uppercase/lowercase/mixed, separators, non-hex chars at every class boundary `@ / : \` G z`, embedded NUL, odd length) × hex_len {n, n-1} × ignore {NULL, ": \n", "", "xyz"} × bin_maxlen {0,1,2,3,n/2,n+4} × bin_len {NULL, ptr} × hex_end {NULL, ptr} | [x] |
| sodium-24 | sodium_hex2bin | 3000 random strings over `0-9a-fA-F: \n@Gz/` × random ignore/bin_maxlen/out-pointer combinations; ret, errno, bin buffer, *bin_len and hex_end offset all compared | [x] |
| sodium-25 | sodium_hex2bin | round-trip of sodium_bin2hex output for n = 0..48 with exact-fit bin_maxlen | [x] |
| sodium-26 | sodium_base64_encoded_len | variant ORIGINAL / ORIGINAL_NO_PADDING / URLSAFE / URLSAFE_NO_PADDING × bin_len 0..300 and 1000, 2^20, 2^40+1, SIZE_MAX/8, 3*((SIZE_MAX-5)/4) | [x] |
| sodium-27 | sodium_bin2base64 | all 4 variants × bin_len 0..64 (all three `bin_len % 3` remainders) × {all-zero, all-0xff, patterned, 8 random} × b64_maxlen = encoded_len + {0,1,5,32} (exercises the trailing-NUL fill loop); full buffer + canary, alphabet and padding-length invariants | [x] |
| sodium-28 | sodium_base642bin | all 4 variants × round-trip of sodium_bin2base64 output for n=0..48 × bin_maxlen {n, n+1, n-1, 0} × ignore {NULL, " \n", "", "="} × bin_len {NULL,ptr} × b64_end {NULL,ptr} | [x] |
| sodium-29 | sodium_base642bin | 45 hand-written inputs (valid, 1/2/3-char tails, wrong padding, over-padding, urlsafe chars in the standard alphabet and vice versa, invalid chars, embedded NUL, high-bit bytes) × all 4 variants × b64_len {n, n-1} × ignore × bin_maxlen {0,1,2,3,8} × out-pointer combinations | [x] |
| sodium-30 | sodium_base642bin | 6000 random strings over `ABCZaz09+/-_=! \n\0` × random variant/ignore/bin_maxlen/out-pointer; ret, errno, bin buffer, *bin_len, b64_end offset compared | [x] |
| sodium-31 | sodium_ip2bin | 66 hand-written IPv4/IPv6/zone-id inputs (valid, malformed, truncated, embedded NUL, `::`-forms, embedded IPv4, uppercase hex) × ip_len_ {n, n+1, n-1, 0, 3} | [x] |
| sodium-32 | sodium_ip2bin | 20000 random strings over `0-9a-fA-F.:%_-gz ` (length 0..23); return value and the 16-byte output + canary compared | [x] |
| sodium-33 | sodium_bin2ip | IPv4-mapped inputs, 12 near-miss mapped prefixes, all single-non-zero-word patterns, every zero-run start×length, equal-length zero-run tie-breaks, 1000 random/sparse patterns × ip_maxlen 0,1,2,3,4,5,8,16,40,46,64; NULL-ness, return pointer, full buffer + canary | [x] |
| sodium-34 | sodium_bin2ip + sodium_ip2bin | every bin2ip output re-parsed by ip2bin and required to reproduce the original 16 bytes | [x] |
| sodium-35 | sodium_init | called 3 more times after the harness' first call; must return 1 (already-initialized path) | [x] |
| sodium-36 | sodium_crit_enter, sodium_crit_leave | 3 balanced enter/leave pairs plus one unbalanced leave (no-op versions: no HAVE_PTHREAD / HAVE_ATOMIC_OPS / _WIN32) | [x] |
| sodium-37 | sodium_set_misuse_handler | handler = NULL, handler = fn, handler = NULL again | [x] |
| sodium-38 | sodium_set_misuse_handler + sodium_misuse | handler installed in a forked child, then a misuse triggered: the handler is called (child exits 42) before abort() | [x] |
| sodium-39 | sodium_runtime_has_neon/armcrypto/sse2/sse3/ssse3/sse41/avx/avx2/avx512f/pclmul/aesni/rdrand | all 12 queried before and after re-running detection | [x] |
| sodium-40 | _sodium_runtime_get_cpu_features | called 3× (idempotent); return value compared, and the has_* answers re-checked afterwards | [x] |
| sodium-41 | sodium_version_string, sodium_library_version_major, sodium_library_version_minor, sodium_library_minimal | all four compared | [x] |
| sodium-42 | randombytes_set_implementation, randombytes_implementation_name, randombytes_random, randombytes_buf, randombytes, randombytes_stir, randombytes_uniform, randombytes_seedbytes, randombytes_close | a deterministic test implementation is installed in both libraries (independent per-library counters) and a 700-entry transcript is compared entry-by-entry: 64 `random()` draws, `uniform()` × 14 upper bounds × 8 draws, `buf`/`randombytes` at sizes 0,1,2,15,16,17,63,64,100 (full buffer + canary), `stir`, `close` | [x] |
| sodium-43 | randombytes_uniform | implementation with `uniform == NULL`: exercises the `(1+~ub) % ub` rejection-sampling fallback deterministically for upper_bound 0,1,2,3,5,16,17,255,256,1000,2^31,2^31+1,0xfffffffe,0xffffffff | [x] |
| sodium-44 | randombytes_uniform | implementation with `uniform != NULL`: exercises the delegation branch for the same 14 upper bounds | [x] |
| sodium-45 | randombytes_stir, randombytes_close | implementation with `stir == NULL` and `close == NULL`: both must be no-ops (stir) / return 0 (close) | [x] |
| sodium-46 | randombytes_close, randombytes_implementation_name | implementation pointer set to NULL: close() returns 0, and the next call re-installs the default (`sysrandom`) implementation via randombytes_init_if_needed() | [x] |
| sodium-47 | randombytes_buf_deterministic, randombytes_seedbytes | 40 seeds (all-zero, all-0xff, 38 random) × size 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000; **bytes** compared plus canary | [x] |
| sodium-48 | randombytes_sysrandom_implementation (exported data object) | every function pointer called directly: implementation_name ("sysrandom"), uniform == NULL, close() before/after stir(), stir(), random() ×512, buf() at 1,16,31,32,33,256,257,1000 (crosses the 256-byte getrandom chunk boundary), close() ×2 more | [x] |
| sodium-49 | randombytes_internal_implementation (exported data object) | same sequence; implementation_name ("internal"), uniform == NULL, close() before stir returns -1 in both, after stir returns 0 | [x] |
| sodium-50 | randombytes_set_implementation + public API | both exported implementations installed and driven through randombytes_implementation_name/stir/close/seedbytes/uniform/buf/randombytes, including size 0 (buffer must be untouched) | [x] |
| sodium-51 | crypto_ipcrypt_bytes, _keybytes, _nd_keybytes, _nd_tweakbytes, _nd_inputbytes, _nd_outputbytes, _ndx_keybytes, _ndx_tweakbytes, _ndx_inputbytes, _ndx_outputbytes, _pfx_keybytes, _pfx_bytes | all 12 constants compared (and pinned to 16/16/16/8/16/24/32/16/16/32/32/16) | [x] |
| sodium-52 | crypto_ipcrypt_encrypt, crypto_ipcrypt_decrypt | 26 keys (all-zero, all-0xff, 24 random) × 46 inputs (zero, 0xff, ::1, 5 IPv4-mapped, 12 near-miss mapped prefixes, 24 random); output bytes + canary, decrypt round-trip, and `out == in` aliasing | [x] |
| sodium-53 | crypto_ipcrypt_nd_encrypt, crypto_ipcrypt_nd_decrypt | 18 × 16-byte keys × 10 × 8-byte tweaks (incl. all-zero, all-0xff) × 20 inputs; 24-byte output + canary, tweak prefix, round-trip | [x] |
| sodium-54 | crypto_ipcrypt_ndx_encrypt, crypto_ipcrypt_ndx_decrypt | 20 × 32-byte keys × 8 × 16-byte tweaks × 16 inputs; 32-byte output + canary, tweak prefix, round-trip | [x] |
| sodium-55 | crypto_ipcrypt_ndx_encrypt/decrypt, crypto_ipcrypt_pfx_encrypt/decrypt | keys whose two 16-byte halves are identical (all-zero, all-0xff, all-0x5a, counting): the two key schedules coincide, so `d == 0` and the `k[i] ^ 0x5a` re-derivation branch is taken | [x] |
| sodium-56 | crypto_ipcrypt_pfx_encrypt, crypto_ipcrypt_pfx_decrypt | 20 × 32-byte keys × 46 inputs covering both `prefix_start = 0` (generic) and `prefix_start = 96` (IPv4-mapped) loops; output bytes + canary, mapped-prefix preservation, round-trip | [x] |
| sodium-57 | crypto_ipcrypt_keygen, crypto_ipcrypt_nd_keygen, crypto_ipcrypt_ndx_keygen, crypto_ipcrypt_pfx_keygen | with the deterministic test RNG installed in both libraries: output **bytes** compared (16/16/32/32) + canary; then repeated with the real RNG to confirm the written length | [x] |
| sodium-58 | ipcrypt_soft_implementation (exported data object) | all 8 function pointers (encrypt, decrypt, nd_encrypt, nd_decrypt, ndx_encrypt, ndx_decrypt, pfx_encrypt, pfx_decrypt) called directly with 8 random key sets × 10 inputs; outputs + canary compared and round-tripped | [x] |
| sodium-59 | _crypto_ipcrypt_pick_best_implementation | called 3× (always selects the soft backend in this build); return value compared, and crypto_ipcrypt_encrypt re-verified afterwards | [x] |
| sodium-60 | sodium_ip2bin + crypto_ipcrypt_pfx_encrypt/decrypt + sodium_bin2ip | end-to-end: 10 IPv4/IPv6/zone-id/mapped strings × 8 random 32-byte keys, parse → encrypt → format → decrypt | [x] |

| stream-1 | crypto_core_salsa20 | c=NULL → built-in sigma constants; 44 in/k cases (all-zero, all-0xff, mixed, 40 random) | [x] |
| stream-2 | crypto_core_salsa20 | c!=NULL → constants loaded from c[0..16]; same 44 in/k cases | [x] |
| stream-3 | crypto_core_salsa20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-4 | crypto_core_salsa2012 | c=NULL, 12 rounds, 44 in/k cases | [x] |
| stream-5 | crypto_core_salsa2012 | c!=NULL, 12 rounds, 44 in/k cases | [x] |
| stream-6 | crypto_core_salsa2012_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-7 | crypto_core_salsa208 | c=NULL, 8 rounds, 44 in/k cases | [x] |
| stream-8 | crypto_core_salsa208 | c!=NULL, 8 rounds, 44 in/k cases | [x] |
| stream-9 | crypto_core_salsa208_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-10 | crypto_core_hsalsa20 | c=NULL → built-in constants; 44 in/k cases; 32-byte output | [x] |
| stream-11 | crypto_core_hsalsa20 | c!=NULL → constants from c[0..16]; 44 in/k cases | [x] |
| stream-12 | crypto_core_hsalsa20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 32 / 16 / 32 / 16 | [x] |
| stream-13 | crypto_core_hchacha20 | c=NULL → built-in constants; 44 in/k cases; 32-byte output | [x] |
| stream-14 | crypto_core_hchacha20 | c!=NULL → constants from c[0..16]; 44 in/k cases | [x] |
| stream-15 | crypto_core_hchacha20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 32 / 16 / 32 / 16 | [x] |
| stream-16 | crypto_stream_salsa20 | clen=0 → early `return 0`, output buffer untouched | [x] |
| stream-17 | crypto_stream_salsa20 | clen ∈ {64,128,192,256,320,384,512,1024} (whole-block loop writes the core output straight into `c`) | [x] |
| stream-18 | crypto_stream_salsa20 | clen non-multiple of 64 ∈ {1,2,3,31,32,33,63,65,66,100,127,129,130,…,1025} + 20 random ≤1500 (trailing partial block copied out of `block[]`) | [x] |
| stream-19 | crypto_stream_salsa20_xor | out-of-place, 53 lengths (all of stream-17/18 plus 0) | [x] |
| stream-20 | crypto_stream_salsa20_xor | in-place (c == m), 53 lengths, result compared against the out-of-place result | [x] |
| stream-21 | crypto_stream_salsa20_xor_ic | ic=0 (identical to `_xor`), 53 lengths | [x] |
| stream-22 | crypto_stream_salsa20_xor_ic | ic ∈ {1,2,7,0xdeadbeef12345678}, 53 lengths | [x] |
| stream-23 | crypto_stream_salsa20_xor_ic | ic ∈ {2^32-2, 2^32-1, 2^32, 2^32+1} — carry out of the low 4 counter bytes of the 8-byte LE counter | [x] |
| stream-24 | crypto_stream_salsa20_xor_ic | ic ∈ {2^64-2, 2^64-1} — full 64-bit counter overflow (all 8 counter bytes wrap to 0 mid-message) | [x] |
| stream-25 | crypto_stream_salsa20_xor_ic | in-place (c == m) for every (length, ic) pair above | [x] |
| stream-26 | crypto_stream_salsa20_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SODIUM_SIZE_MAX (== SIZE_MAX) | [x] |
| stream-27 | crypto_stream_salsa20_ref_implementation → `.stream` fn-ptr | exported implementation struct; called directly through both libs, 53 lengths | [x] |
| stream-28 | crypto_stream_salsa20_ref_implementation → `.stream_xor_ic` fn-ptr (`stream_ref_xor_ic`) | 53 lengths × 11 ic values, out-of-place + in-place | [x] |
| stream-29 | _crypto_stream_salsa20_pick_best_implementation | returns 0 (ref impl always chosen, no SIMD in this build); dispatchers re-verified afterwards | [x] |
| stream-30 | crypto_stream_salsa2012 | clen=0, multiples of 64, non-multiples of 64, 53 lengths | [x] |
| stream-31 | crypto_stream_salsa2012_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-32 | crypto_stream_salsa2012_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-33 | crypto_stream_salsa208 | clen=0, multiples of 64, non-multiples of 64, 53 lengths | [x] |
| stream-34 | crypto_stream_salsa208_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-35 | crypto_stream_salsa208_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-36 | crypto_stream_xsalsa20 | 24-byte nonce → hsalsa20 subkey + salsa20 keystream on n+16; 53 lengths | [x] |
| stream-37 | crypto_stream_xsalsa20_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-38 | crypto_stream_xsalsa20_xor_ic | 53 lengths × 11 ic values incl. 2^32 and 2^64 rollover, out-of-place + in-place | [x] |
| stream-39 | crypto_stream_xsalsa20_keybytes, _noncebytes, _messagebytes_max | 32 / 24 / SIZE_MAX | [x] |
| stream-40 | crypto_stream | generic dispatcher → xsalsa20; 53 lengths, 24-byte nonce | [x] |
| stream-41 | crypto_stream_xor | generic dispatcher → xsalsa20_xor; out-of-place + in-place, 53 lengths | [x] |
| stream-42 | crypto_stream_keybytes, _noncebytes, _messagebytes_max, _primitive | 32 / 24 / SIZE_MAX / "xsalsa20" | [x] |
| stream-43 | crypto_stream_chacha20 | clen=0 → early `return 0` | [x] |
| stream-44 | crypto_stream_chacha20 | clen < 64 (partial-block path: `memset(tmp)`, `ctarget` redirection) — 1,2,3,31,32,33,63 | [x] |
| stream-45 | crypto_stream_chacha20 | clen == 64 exactly (`bytes <= 64` taken, `bytes < 64` not taken) | [x] |
| stream-46 | crypto_stream_chacha20 | clen > 64 and a multiple of 64 (128,192,256,320,384,512,1024) | [x] |
| stream-47 | crypto_stream_chacha20 | clen > 64 and not a multiple of 64 (65,66,100,127,129,…,1025 + 20 random) — full-block loop followed by the `tmp[]` tail | [x] |
| stream-48 | crypto_stream_chacha20_xor | out-of-place + in-place, 53 lengths, 8-byte nonce | [x] |
| stream-49 | crypto_stream_chacha20_xor_ic | ic ∈ {0,1,2,7,0xdeadbeef12345678} split into low/high 32-bit counter words, 53 lengths | [x] |
| stream-50 | crypto_stream_chacha20_xor_ic | ic ∈ {2^32-2, 2^32-1} with mlen > 64 — `j12` wraps to 0 and increments `j13` (high counter word) | [x] |
| stream-51 | crypto_stream_chacha20_xor_ic | ic ∈ {2^64-2, 2^64-1} — both counter words wrap; also ic = 2^32 / 2^32+1 (high word non-zero from the start) | [x] |
| stream-52 | crypto_stream_chacha20_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-53 | crypto_stream_chacha20_ietf | 12-byte nonce, 32-bit counter word (`chacha_ietf_ivsetup`); 53 lengths incl. 0, <64, ==64, >64 | [x] |
| stream-54 | crypto_stream_chacha20_ietf_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-55 | crypto_stream_chacha20_ietf_ext | internal-but-exported keystream entry point, 53 lengths | [x] |
| stream-56 | crypto_stream_chacha20_ietf_ext_xor_ic | 32-bit ic ∈ {0,1,2,0x7fffffff,0x80000000}, 53 lengths, out-of-place + in-place | [x] |
| stream-57 | crypto_stream_chacha20_ietf_ext_xor_ic | ic ∈ {2^32-2, 2^32-1} with mlen > 64 — counter overflows *into the IV word* (`j13`), the behaviour the `_ext` variant exists for | [x] |
| stream-58 | crypto_stream_chacha20_ietf_xor_ic | ic at the exact largest accepted value 2^32 − ceil(mlen/64) for mlen ∈ {0,1,63,64,65,127,128,129,192,256,1000,1024,1025} | [x] |
| stream-59 | crypto_stream_chacha20_ietf_xor_ic | ic = max−1, 0 and 1 for the same mlen set (inside the accepted range) | [x] |
| stream-60 | crypto_stream_chacha20_ietf_keybytes, _ietf_noncebytes, _ietf_messagebytes_max | 32 / 12 / min(SIZE_MAX, 64·2^32) = 274877906944 | [x] |
| stream-61 | crypto_stream_chacha20_ref_implementation → `.stream`, `.stream_ietf_ext`, `.stream_xor_ic`, `.stream_ietf_ext_xor_ic` | exported implementation struct; every fn-ptr called directly through both libs over the full length × ic matrix | [x] |
| stream-62 | _crypto_stream_chacha20_pick_best_implementation | returns 0 (ref impl always chosen); dispatchers re-verified afterwards | [x] |
| stream-63 | crypto_stream_xchacha20 | 24-byte nonce → hchacha20 subkey + chacha20 on n+16; 53 lengths | [x] |
| stream-64 | crypto_stream_xchacha20_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-65 | crypto_stream_xchacha20_xor_ic | 53 lengths × 11 ic values incl. 2^32 and 2^64 rollover, out-of-place + in-place | [x] |
| stream-66 | crypto_stream_xchacha20_keybytes, _noncebytes, _messagebytes_max | 32 / 24 / SIZE_MAX | [x] |
| stream-67 | crypto_stream_keygen, crypto_stream_salsa20_keygen, crypto_stream_salsa2012_keygen, crypto_stream_salsa208_keygen, crypto_stream_xsalsa20_keygen, crypto_stream_chacha20_keygen, crypto_stream_chacha20_ietf_keygen, crypto_stream_xchacha20_keygen | a deterministic `randombytes_implementation` is installed in BOTH libraries via randombytes_set_implementation, so the 32 written bytes are compared byte-for-byte; canary proves exactly KEYBYTES bytes are written | [x] |
| stream-68 | all `*_xor` / `*_xor_ic` / keystream entry points | output buffer padded with a 32-byte 0x5A canary that is compared too, so any over/under-write is caught | [x] |


## Summary

* total configuration rows: **686**
* rows passing across randomized inputs (fixed seed): **686**
* rows still open: **0**

All rows are exercised through the `.so` exports of both libraries with
`libloading`; nothing is called directly. Test files: `tests/{aead1,aead2,blake2,
box,ed25519low,gaps,h2c,hash,mac,pwhash,sign,smoke,sodium,stream}.rs`
(238 test functions, `cargo test --release`).
