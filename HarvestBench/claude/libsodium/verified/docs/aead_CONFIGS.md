# AEAD + SECRETBOX + SECRETSTREAM — configuration coverage

Each row is a meaningful configuration / input-shape combination exercised by
`tests/aead.rs`. All are driven with many randomized inputs (deterministic
seed) across message lengths `{0,1,15,16,17,31,32,33,63,64,65,100,127,128,129,
255,256,1000,4096,8192}` and AAD lengths `{0,1,7,16,33,64,200}` (0 uses a NULL
AAD pointer). Every row asserts C/Rust return-code parity, output byte-equality,
and encrypt->decrypt roundtrip on BOTH libraries; `[x]` = its test passes.

| # | entry point(s) | configuration (options + shape) | done |
|---|----------------|----------------------------------|------|
| 1 | `crypto_aead_chacha20poly1305_encrypt` / `_decrypt` | combined, 8-byte nonce, varied mlen+adlen, roundtrip + cross-decrypt | [x] |
| 2 | `crypto_aead_chacha20poly1305_encrypt_detached` / `_decrypt_detached` | detached mac, mac==combined-tail check | [x] |
| 3 | `crypto_aead_chacha20poly1305_ietf_encrypt` / `_decrypt` | combined, 12-byte IETF nonce | [x] |
| 4 | `crypto_aead_chacha20poly1305_ietf_*_detached` | detached IETF | [x] |
| 5 | `crypto_aead_xchacha20poly1305_ietf_encrypt` / `_decrypt` | combined, 24-byte extended nonce | [x] |
| 6 | `crypto_aead_xchacha20poly1305_ietf_*_detached` | detached | [x] |
| 7 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | combined, 16-byte key, 16-byte nonce, 32-byte tag | [x] |
| 8 | `crypto_aead_aegis128l_*_detached` | detached | [x] |
| 9 | `crypto_aead_aegis256_encrypt` / `_decrypt` | combined, 32-byte key, 32-byte nonce, 32-byte tag | [x] |
| 10 | `crypto_aead_aegis256_*_detached` | detached | [x] |
| 11 | `crypto_aead_aes256gcm_is_available` | availability flag parity C vs Rust | [x] |
| 12 | `crypto_aead_aes256gcm_encrypt` / `_decrypt` (+detached) | combined + detached (only when is_available; else skipped) | [x] |
| 13 | `crypto_aead_aes256gcm_beforenm` + `_encrypt_afternm` / `_decrypt_afternm` | precomputed key expansion, afternm roundtrip (when available) | [x] |
| 14 | `crypto_secretbox_easy` / `_open_easy` | default (xsalsa20poly1305), mac||ct layout, varied mlen | [x] |
| 15 | `crypto_secretbox_detached` / `_open_detached` | detached mac, equals easy-prefix check | [x] |
| 16 | `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy` | xchacha variant, easy | [x] |
| 17 | `crypto_secretbox_xchacha20poly1305_detached` / `_open_detached` | xchacha variant, detached | [x] |
| 18 | `crypto_secretbox` / `crypto_secretbox_open` | NaCl-style 32-byte zero-padded API, varied body sizes | [x] |
| 19 | `crypto_secretbox_xsalsa20poly1305` / `_open` | NaCl-style padded xsalsa20poly1305 | [x] |
| 20 | `crypto_secretstream_..._init_push` / `_push` / `_init_pull` / `_pull` | full stream: TAG_MESSAGE + TAG_PUSH + TAG_FINAL, varied mlen/adlen, 1–10 messages, C-encrypt/Rust+C-decrypt | [x] |
| 21 | `crypto_secretstream_..._push` (Rust side) -> C `_pull` | Rust encrypt path decrypted by C (both encrypt directions covered) | [x] |
| 22 | `crypto_secretstream_..._push` with `TAG_REKEY` + explicit `_rekey()` | rekey semantics parity across C and Rust decrypt | [x] |
| 23 | `crypto_secretstream_..._tag_{message,push,rekey,final}` | tag constant getters parity | [x] |
| 24 | size/introspection getters (`*_keybytes`, `_npubbytes`, `_nsecbytes`, `_abytes`, `_macbytes`, `_noncebytes`, `_zerobytes`, `_boxzerobytes`, `_statebytes`, `_headerbytes`, `_messagebytes_max`) + `crypto_secretbox_primitive` | constant parity across all families | [x] |
