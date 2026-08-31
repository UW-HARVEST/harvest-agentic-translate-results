## Area 6 — crypto_aead / secretbox / secretstream

Scope: `c_src/libsodium/crypto_aead/{aegis128l,aegis256,aes256gcm,chacha20poly1305,xchacha20poly1305}`,
`c_src/libsodium/crypto_secretbox/**`, `c_src/libsodium/crypto_secretstream/xchacha20poly1305/**`
and the matching public headers in `c_src/libsodium/include/sodium/`.

### Build-configuration facts that drive this table

* The CMake build defines **no** `HAVE_*` macros. Therefore:
  * `aegis128l`/`aegis256` `implementation` stays `&aegis128l_soft_implementation` /
    `&aegis256_soft_implementation` (the `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H`
    blocks in `_crypto_aead_aegis*_pick_best_implementation()` are compiled out), so the portable
    `aegis*_soft.c` path is the only one reachable. Behaviour (return codes) is identical either way.
  * The whole `#if !((HAVE_ARMCRYPTO && NATIVE_LITTLE_ENDIAN) || (HAVE_TMMINTRIN_H && HAVE_WMMINTRIN_H))`
    block at `crypto_aead/aes256gcm/aead_aes256gcm.c:50-157` **is** compiled, i.e. the **stub**
    family is what links. `crypto_aead_aes256gcm_is_available()` returns **0**, and *every* other
    aes256gcm entry point (`_encrypt`, `_encrypt_detached`, `_decrypt`, `_decrypt_detached`,
    `_beforenm`, `_encrypt_afternm`, `_encrypt_detached_afternm`, `_decrypt_afternm`,
    `_decrypt_detached_afternm`) unconditionally sets `errno = ENOSYS` (aliased to `ENXIO` if
    `ENOSYS` is undefined) and returns **-1**, *without touching* `*clen_p` / `*mlen_p` /
    `*maclen_p` and without reading any input buffer. Rows 6.30–6.39 below.
  * `crypto_aead_aes256gcm_keybytes/nsecbytes/npubbytes/abytes/statebytes/messagebytes_max/keygen`
    live *outside* that `#if`, so they still work normally even though the cipher is unavailable.
* `sodium_misuse()` (`sodium/core.c:192`) calls the registered misuse handler if any and then
  `abort()`s. It **never returns**. Every `mlen > *_MESSAGEBYTES_MAX` overflow guard in this area
  (except the aegis `*_decrypt_detached` ones, which `return -1`) goes through `sodium_misuse()`,
  i.e. the "expected C result" is *process abort*, not an error code. These branches are marked
  `abort()` below and are unreachable on 64-bit hosts for realistically-sized inputs (they are
  all tagged `LCOV_EXCL_LINE` upstream).
* Relevant constants: `aegis128l` KEY 16 / NPUB 16 / **ABYTES 32** / NSEC 0;
  `aegis256` KEY 32 / NPUB 32 / **ABYTES 32** / NSEC 0;
  `aes256gcm` KEY 32 / NPUB 12 / ABYTES 16 / NSEC 0;
  `chacha20poly1305` KEY 32 / **NPUB 8** / ABYTES 16 / NSEC 0;
  `chacha20poly1305_ietf` KEY 32 / **NPUB 12** / ABYTES 16 / NSEC 0;
  `xchacha20poly1305_ietf` KEY 32 / **NPUB 24** / ABYTES 16 / NSEC 0;
  `secretbox` KEY 32 / NONCE 24 / MAC 16 / BOXZERO 16 / **ZERO 32**;
  `secretstream_xchacha20poly1305` KEY 32 / HEADER 24 / **ABYTES 17** (= 1 + 16),
  `TAG_MESSAGE 0x00`, `TAG_PUSH 0x01`, `TAG_REKEY 0x02`, `TAG_FINAL 0x03`.
* `MESSAGEBYTES_MAX`: `aegis128l`/`aegis256` = `MIN(SIZE_MAX-32, 2^61-1)`;
  `aes256gcm` = `MIN(SIZE_MAX-16, 16*(2^32-2))`; `chacha20poly1305` = `SIZE_MAX-16`;
  `chacha20poly1305_ietf` = `MIN(SIZE_MAX-16, 64*(2^32-1))`;
  `xchacha20poly1305_ietf` = `SIZE_MAX-16`;
  `secretbox_xsalsa20poly1305` / `secretbox_xchacha20poly1305` = `stream_MESSAGEBYTES_MAX - 16`;
  `secretstream` = `MIN(SIZE_MAX-17, 64*(2^32-2))`.
* `nsec` is `NSECBYTES == 0` for every AEAD here. Every implementation does `(void) nsec;`
  (`aead_aegis128l.c:115,136`, `aead_aegis256.c:115,135`, `aead_chacha20poly1305.c:38,122,212,293`,
  `aead_xchacha20poly1305.c:40,108`) — i.e. `nsec` is *always ignored*, `NULL` and non-`NULL` are
  indistinguishable, and it is never written on the decrypt side. There is no rejection branch for it.
* The combined `*_encrypt`/`*_decrypt` wrappers are the only place `clen_p`/`mlen_p` are written;
  they are all `if (ptr != NULL)`-guarded, so a `NULL` out-length pointer is **legal** and simply
  suppresses the store. Same for `maclen_p` in `*_encrypt_detached` and `outlen_p`/`mlen_p`/`tag_p`
  in secretstream. No rejection branch — but the *value* written on failure is load-bearing (0).

### ERROR-SURFACE table

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 6.1 | `crypto_aead_aegis128l_encrypt` | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:69`) | `sodium_misuse()` → misuse handler then `abort()`; never returns | verified |
| 6.2 | `crypto_aead_aegis128l_encrypt_detached` | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:119`) | `sodium_misuse()` → `abort()`. NB `*maclen_p = 32` has *already* been stored at line 117 before the check | verified |
| 6.3 | `crypto_aead_aegis128l_encrypt_detached` | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:120`) | `sodium_misuse()` → `abort()` | verified |
| 6.4 | `crypto_aead_aegis128l_decrypt` | `clen < 32` (`clen < crypto_aead_aegis128l_ABYTES`, guard at `aead_aegis128l.c:92` not taken) — includes `clen == 0` and `clen == 31` | returns `-1`; if `mlen_p != NULL` then `*mlen_p = 0`; `m` untouched; detached path never entered | verified |
| 6.5 | `crypto_aead_aegis128l_decrypt` | `clen >= 32` but tag (last 32 bytes) does not verify — flipped ciphertext bit, flipped tag bit, wrong `k`, wrong `npub`, wrong/absent `ad` | returns `-1` (propagated from `_decrypt_detached`); if `mlen_p != NULL` then `*mlen_p = 0`; `m[0 .. clen-32)` is zeroed by `aegis128l_soft.c:249` | verified |
| 6.6 | `crypto_aead_aegis128l_decrypt_detached` | `clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:137`) | returns `-1` immediately (does **not** abort — differs from the encrypt side); `m` untouched | verified |
| 6.7 | `crypto_aead_aegis128l_decrypt_detached` | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:138`) | returns `-1` immediately; `m` untouched | verified |
| 6.8 | `crypto_aead_aegis128l_decrypt_detached` | `m != NULL`, `crypto_verify_32(computed_mac, mac) != 0` (`aegis128l_soft.c:244,247`) | returns `-1`; `memset(m, 0, clen)` — plaintext buffer wiped, no partial plaintext leak | verified |
| 6.9 | `crypto_aead_aegis128l_decrypt_detached` | `m == NULL` (verify-only mode) and tag mismatch (`aegis128l_soft.c:225-229,248`) | returns `-1`; nothing written anywhere (the `memset` is skipped) | verified |
| 6.10 | `aegis128l_mac` (internal, via `encrypt_detached`/`decrypt_detached`) | `maclen` neither 16 nor 32 (`aegis128l_common.h:62-64`) | `memset(mac, 0, maclen)` then `-1`. **Unreachable from the public API** — `maclen` is hard-wired to `crypto_aead_aegis128l_ABYTES == 32` at `aead_aegis128l.c:113,134`. Documented for completeness only | unreachable-from-public-API |
| 6.11 | `crypto_aead_aegis256_encrypt` | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:69`) | `sodium_misuse()` → `abort()` | verified |
| 6.12 | `crypto_aead_aegis256_encrypt_detached` | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:119`) | `sodium_misuse()` → `abort()`; `*maclen_p = 32` already stored at line 117 | verified |
| 6.13 | `crypto_aead_aegis256_encrypt_detached` | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:120`) | `sodium_misuse()` → `abort()` | verified |
| 6.14 | `crypto_aead_aegis256_decrypt` | `clen < 32` (`aead_aegis256.c:92`) | returns `-1`; `*mlen_p = 0` if `mlen_p != NULL`; `m` untouched | verified |
| 6.15 | `crypto_aead_aegis256_decrypt` | `clen >= 32` but tag mismatch (bit-flip in `c`, in the trailing tag, wrong `k`/`npub`/`ad`) | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-32)` zeroed (`aegis256_soft.c` / `aegis256_common.h:232`) | verified |
| 6.16 | `crypto_aead_aegis256_decrypt_detached` | `clen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:136`) | returns `-1` immediately | verified |
| 6.17 | `crypto_aead_aegis256_decrypt_detached` | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:137`) | returns `-1` immediately | verified |
| 6.18 | `crypto_aead_aegis256_decrypt_detached` | `m != NULL` and `crypto_verify_32(computed_mac, mac) != 0` (`aegis256_common.h:227,230`) | returns `-1`; `memset(m, 0, clen)` | verified |
| 6.19 | `crypto_aead_aegis256_decrypt_detached` | `m == NULL` and tag mismatch (`aegis256_common.h:208-211,231`) | returns `-1`; nothing written | verified |
| 6.20 | `aegis256_mac` (internal) | `maclen` neither 16 nor 32 (`aegis256_common.h:62-64`) | `memset(mac, 0, maclen)`, `-1`. Unreachable from public API (`maclen` fixed to 32) | unreachable-from-public-API |
| 6.21 | `crypto_aead_chacha20poly1305_encrypt` | `mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX` (= `SIZE_MAX - 16`) (`aead_chacha20poly1305.c:89`) | `sodium_misuse()` → `abort()` | verified |
| 6.22 | `crypto_aead_chacha20poly1305_decrypt` | `clen < 16` (`clen < crypto_aead_chacha20poly1305_ABYTES`, `aead_chacha20poly1305.c:259`) — incl. `clen == 0`, `clen == 15` | returns `-1`; `*mlen_p = 0` if non-NULL; `m` untouched | verified |
| 6.23 | `crypto_aead_chacha20poly1305_decrypt` | `clen >= 16`, tag mismatch | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_chacha20poly1305.c:236`) | verified |
| 6.24 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m != NULL`, `crypto_verify_16(computed_mac, mac) != 0` (`aead_chacha20poly1305.c:230,235-238`) | returns `-1`; `memset(m, 0, clen)`; the ChaCha20 keystream XOR at line 240 is **not** executed | verified |
| 6.25 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m == NULL` and tag mismatch (`aead_chacha20poly1305.c:232-234`) | returns the raw `crypto_verify_16` result = `-1`; nothing written | verified |
| 6.26 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX` (= `MIN(SIZE_MAX-16, 64*(2^32-1))`) (`aead_chacha20poly1305.c:177`) | `sodium_misuse()` → `abort()` | verified |
| 6.27 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen < 16` (`aead_chacha20poly1305.c:344`) | returns `-1`; `*mlen_p = 0`; `m` untouched | verified |
| 6.28 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen >= 16`, tag mismatch | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_chacha20poly1305.c:321`) | verified |
| 6.29 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m != NULL`, `crypto_verify_16` fails (`aead_chacha20poly1305.c:315,320-323`) | returns `-1`; `memset(m, 0, clen)` | verified |
| 6.29a | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m == NULL` and tag mismatch (`aead_chacha20poly1305.c:317-319`) | returns `-1`; nothing written | verified |
| 6.30 | `crypto_aead_aes256gcm_is_available` | always, in this build (no `HAVE_TMMINTRIN_H`/`HAVE_WMMINTRIN_H`/`HAVE_ARMCRYPTO`) (`aead_aes256gcm.c:151-155`) | returns `0` — the cipher is permanently unavailable | verified |
| 6.31 | `crypto_aead_aes256gcm_encrypt` | any call, even with fully valid key/nonce/message (`aead_aes256gcm.c:69-76`) | `errno = ENOSYS`; returns `-1`. `*clen_p` is **not** written (differs from every other AEAD, which zeroes it) | verified |
| 6.32 | `crypto_aead_aes256gcm_encrypt_detached` | any call (`aead_aes256gcm.c:57-66`) | `errno = ENOSYS`; returns `-1`; `*maclen_p` not written; `c`/`mac` untouched | verified |
| 6.33 | `crypto_aead_aes256gcm_decrypt` | any call — valid ciphertext, `clen < 16`, `clen == 0`, tampered tag: all identical (`aead_aes256gcm.c:89-97`) | `errno = ENOSYS`; returns `-1`; `*mlen_p` **not** written | verified |
| 6.34 | `crypto_aead_aes256gcm_decrypt_detached` | any call (`aead_aes256gcm.c:78-87`) | `errno = ENOSYS`; returns `-1`; `m` untouched (not even zeroed) | verified |
| 6.35 | `crypto_aead_aes256gcm_beforenm` | any call, even with a valid 32-byte key and a properly aligned `crypto_aead_aes256gcm_state` (`aead_aes256gcm.c:99-104`) | `errno = ENOSYS`; returns `-1`; `st_` left **uninitialised** | verified |
| 6.36 | `crypto_aead_aes256gcm_encrypt_afternm` | any call (with or without a preceding successful `_beforenm`, which can never succeed) (`aead_aes256gcm.c:118-127`) | `errno = ENOSYS`; returns `-1`; `*clen_p` not written | verified |
| 6.37 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | any call (`aead_aes256gcm.c:106-116`) | `errno = ENOSYS`; returns `-1`; `*maclen_p` not written | verified |
| 6.38 | `crypto_aead_aes256gcm_decrypt_afternm` | any call (`aead_aes256gcm.c:140-149`) | `errno = ENOSYS`; returns `-1`; `*mlen_p` not written | verified |
| 6.39 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | any call (`aead_aes256gcm.c:129-138`) | `errno = ENOSYS`; returns `-1`; `m` untouched | verified |
| 6.40 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX` (= `SIZE_MAX - 16`) (`aead_xchacha20poly1305.c:185`) | `sodium_misuse()` → `abort()` | verified |
| 6.41 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen < 16` (`aead_xchacha20poly1305.c:237`) — incl. `clen == 0`, `clen == 15` | returns `-1`; `*mlen_p = 0` if non-NULL; `m` untouched | verified |
| 6.42 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen >= 16`, tag mismatch (bit-flip anywhere in `c`, wrong `k`/`npub`/`ad`) | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_xchacha20poly1305.c:136`) | verified |
| 6.43 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m != NULL`, `crypto_verify_16(computed_mac, mac) != 0` (`aead_xchacha20poly1305.c:130,135-138`) | returns `-1`; `memset(m, 0, clen)`; keystream XOR at line 140 skipped | verified |
| 6.44 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m == NULL` and tag mismatch (`aead_xchacha20poly1305.c:132-134`) | returns `-1`; nothing written | verified |
| 6.45 | `crypto_aead_*_encrypt_detached` (chacha20poly1305, chacha20poly1305_ietf, xchacha20poly1305_ietf) | **no** length guard exists in these three (`aead_chacha20poly1305.c:23`, `:107`, `aead_xchacha20poly1305.c:146`): they have neither `mlen > MESSAGEBYTES_MAX` nor `adlen` checks | always returns `0`. Contrast with aegis128l/aegis256 `_encrypt_detached` (rows 6.2/6.3/6.12/6.13), which *do* guard and abort. A translation must not add a rejection here | verified |
| 6.46 | `crypto_aead_*_encrypt_detached` with `maclen_p == NULL` | all six families | **legal**, not an error: the `if (maclen_p != NULL)` guard simply skips the store (`aead_aegis128l.c:116`, `aead_aegis256.c:116`, `aead_chacha20poly1305.c:69,157`, `aead_xchacha20poly1305.c:84`). Return `0` (or the aes256gcm `-1` of row 6.32) | verified |
| 6.47 | `crypto_aead_*_encrypt` with `clen_p == NULL` / `crypto_aead_*_decrypt` with `mlen_p == NULL` | all six families | **legal**, not an error: guarded stores; the return value alone conveys success/failure. Notably a `clen < ABYTES` rejection is then observable *only* via the `-1` return | verified |
| 6.48 | `crypto_aead_*_encrypt` / `_encrypt_detached` with `nsec != NULL` | all six families (`NSECBYTES == 0`) | ignored via `(void) nsec;` — identical result to `nsec == NULL`; **no rejection branch** | verified |
| 6.49 | `crypto_aead_*_decrypt` / `_decrypt_detached` with `nsec != NULL` (out-param) | all six families | ignored via `(void) nsec;`; the buffer is never written, even on success; **no rejection branch** | verified |
| 6.50 | `crypto_secretbox_easy` | `mlen > crypto_secretbox_MESSAGEBYTES_MAX` (`crypto_secretbox_easy.c:97`) | `sodium_misuse()` → `abort()` | verified |
| 6.51 | `crypto_secretbox_detached` | none — there is no length or validity check at all (`crypto_secretbox_easy.c:19-90`) | always returns `0`. `mlen == 0` is accepted (produces a MAC over the empty string) | verified |
| 6.52 | `crypto_secretbox_open_easy` | `clen < crypto_secretbox_MACBYTES` (= 16) (`crypto_secretbox_easy.c:170-172`) — incl. `clen == 0`, `clen == 15` | returns `-1` before any crypto; `m` untouched | verified |
| 6.53 | `crypto_secretbox_open_easy` | `clen >= 16` but MAC (leading 16 bytes of `c`) does not verify — flipped MAC bit, flipped ciphertext bit, wrong `k`, wrong `n` | returns `-1` (from `crypto_secretbox_open_detached`); `m` untouched (**not** zeroed, unlike the AEADs) | verified |
| 6.54 | `crypto_secretbox_open_detached` | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` (`crypto_secretbox_easy.c:127-130`) | `sodium_memzero(subkey)`, returns `-1`; `m` untouched; the salsa20 decryption is never run | verified |
| 6.55 | `crypto_secretbox_open_detached` with `m == NULL` | MAC verifies (`crypto_secretbox_easy.c:131-134`) | returns `0` — verify-only mode, **not** an error. With a bad MAC it returns `-1` via row 6.54 | verified |
| 6.56 | `crypto_secretbox_xchacha20poly1305_easy` | `mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX` (`secretbox_xchacha20poly1305.c:89`) | `sodium_misuse()` → `abort()` | verified |
| 6.57 | `crypto_secretbox_xchacha20poly1305_detached` | none — no checks (`secretbox_xchacha20poly1305.c:19-80`) | always returns `0` | verified |
| 6.58 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen < crypto_secretbox_xchacha20poly1305_MACBYTES` (= 16) (`secretbox_xchacha20poly1305.c:164-166`) | returns `-1` before any crypto | verified |
| 6.59 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen >= 16` but MAC mismatch | returns `-1` (from `_open_detached`); `m` untouched | verified |
| 6.60 | `crypto_secretbox_xchacha20poly1305_open_detached` | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` (`secretbox_xchacha20poly1305.c:120-123`) | `sodium_memzero(subkey)`, returns `-1`; `m` untouched | verified |
| 6.61 | `crypto_secretbox_xchacha20poly1305_open_detached` with `m == NULL` | MAC verifies (`secretbox_xchacha20poly1305.c:124-127`) | returns `0` — verify-only mode, not an error | verified |
| 6.62 | `crypto_secretbox` → `crypto_secretbox_xsalsa20poly1305` | `mlen < 32` (`mlen < crypto_secretbox_ZEROBYTES`) (`secretbox_xsalsa20poly1305.c:15-17`) — the NaCl-style API requires the caller to prepend 32 zero bytes, so `mlen` counts padding+plaintext; `mlen ∈ {0,1,16,31}` all rejected | returns `-1`; `c` untouched | verified |
| 6.63 | `crypto_secretbox` → `crypto_secretbox_xsalsa20poly1305` | `mlen >= 32` but `m[0..31]` are **not** all zero (no explicit check exists; `secretbox_xsalsa20poly1305.c:18-19` XORs the keystream over `m[0..31]` and derives the Poly1305 key from `c[0..31]`) | returns `0` — **silently accepted**. The produced box is unopenable: `crypto_secretbox_open` derives `subkey` from the raw keystream and will fail MAC verification (row 6.65). This is a latent correctness hazard, not a rejection | verified |
| 6.64 | `crypto_secretbox_open` → `crypto_secretbox_xsalsa20poly1305_open` | `clen < 32` (`clen < crypto_secretbox_ZEROBYTES`) (`secretbox_xsalsa20poly1305.c:35-37`) — `clen ∈ {0,1,16,17,31}` all rejected. NB the leading 16 bytes of `c` must be zero padding (`BOXZEROBYTES`) with the MAC at `c+16` | returns `-1`; `m` untouched | verified |
| 6.65 | `crypto_secretbox_open` → `crypto_secretbox_xsalsa20poly1305_open` | `clen >= 32` but `crypto_onetimeauth_poly1305_verify(c+16, c+32, clen-32, subkey) != 0` — flipped MAC/ciphertext bit, wrong `k`/`n`, or the caller failed to zero `c[0..15]` | returns `-1`; `m` untouched (**not** zeroed) | verified |
| 6.66 | `crypto_secretbox_open` | `clen >= 32`, MAC verifies, but `c[0..15]` is non-zero garbage | the padding bytes are *not* validated; they are decrypted and then `m[0..31]` is force-zeroed (`secretbox_xsalsa20poly1305.c:45-47`). Returns `0`. In practice a non-zero `c[0..15]` changes nothing that the MAC covers, so this is reachable and must round-trip identically | verified |
| 6.67 | `crypto_secretbox_xchacha20poly1305` (NaCl-style, zero-padded) | — | **does not exist**: the xchacha20poly1305 secretbox family only provides `_easy`/`_open_easy`/`_detached`/`_open_detached` (`secretbox_xchacha20poly1305.c`, `crypto_secretbox_xchacha20poly1305.h`). Any translation must not expose a NaCl-style variant here | verified |
| 6.68 | `crypto_secretstream_xchacha20poly1305_init_push` | none — no validation of `state`, `out` or `k` (`secretstream_xchacha20poly1305.c:42-65`) | always returns `0`; `out` filled with 24 random header bytes | verified |
| 6.69 | `crypto_secretstream_xchacha20poly1305_init_pull` | none — the 24-byte header is **not** validated in any way (it is fed straight into `crypto_core_hchacha20`); an all-zero header, a truncated/garbage header, or a header from a different session are all accepted (`secretstream_xchacha20poly1305.c:67-80`) | always returns `0`. The mismatch only surfaces later as a `_pull` MAC failure (row 6.75) | verified |
| 6.70 | `crypto_secretstream_xchacha20poly1305_push` | `mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` (= `MIN(SIZE_MAX-17, 64*(2^32-2))`) (`secretstream_xchacha20poly1305.c:128-130`) | `sodium_misuse()` → `abort()`. NB `*outlen_p = 0` has already been stored at lines 123-125 | verified |
| 6.71 | `crypto_secretstream_xchacha20poly1305_push` | any other input, incl. `mlen == 0`, `ad == NULL`/`adlen == 0`, an out-of-range `tag` byte (e.g. `0x04`..`0xff`) — the `tag` value is **never validated** | returns `0`; `*outlen_p = 17 + mlen`. A `tag` with bit `0x02` (`TAG_REKEY`) set — which includes `TAG_FINAL == 0x03` and any bogus tag with that bit — triggers an implicit `_rekey()` (line 168-172) | verified |
| 6.72 | `crypto_secretstream_xchacha20poly1305_push` | 32-bit counter wraps to zero after `sodium_increment` (`secretstream_xchacha20poly1305.c:169-170`) — i.e. `2^32 - 1` messages pushed since the last rekey | returns `0` but performs an implicit `_rekey()`. Not an error; the pull side wraps identically so the session stays in sync | verified |
| 6.73 | `crypto_secretstream_xchacha20poly1305_push` with `outlen_p == NULL` | — | **legal**: both stores are `if (outlen_p != NULL)`-guarded (lines 123, 173). Returns `0` | verified |
| 6.74 | `crypto_secretstream_xchacha20poly1305_pull` | `inlen < crypto_secretstream_xchacha20poly1305_ABYTES` (= 17) (`secretstream_xchacha20poly1305.c:201-203`) — incl. `inlen == 0`, `1`, `16` | returns `-1`; `*mlen_p = 0` and `*tag_p = 0xff` were already stored at lines 195-200; `m` untouched; state **unchanged** (no nonce/counter advance) | verified |
| 6.75 | `crypto_secretstream_xchacha20poly1305_pull` | `sodium_memcmp(mac, stored_mac, 16) != 0` (`secretstream_xchacha20poly1305.c:239-242`) — tampered `in[0]` tag byte, tampered ciphertext, tampered trailing MAC, wrong/absent `ad`, wrong key, header mismatch from row 6.69, or a stream replayed/reordered out of sequence | `sodium_memzero(mac)`, returns `-1`; `*mlen_p` stays `0`, `*tag_p` stays `0xff`; `m` untouched (**not** zeroed); state **unchanged**, so the session is not advanced and a correct frame can still be pulled afterwards | verified |
| 6.76 | `crypto_secretstream_xchacha20poly1305_pull` | `mlen = inlen - 17 > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` (`secretstream_xchacha20poly1305.c:205-207`) | `sodium_misuse()` → `abort()` | verified |
| 6.77 | `crypto_secretstream_xchacha20poly1305_pull` after a `TAG_FINAL` frame | the C code does **not** latch a "finished" flag; `_pull` will happily be called again. Because `TAG_FINAL` (`0x03`) has the `TAG_REKEY` bit set, `_pull` rekeyed the state, so the next frame's MAC will not match | returns `-1` via row 6.75. There is no dedicated "stream already ended" error code | verified |
| 6.78 | `crypto_secretstream_xchacha20poly1305_pull` with `mlen_p == NULL` and/or `tag_p == NULL` | — | **legal**: all four stores are NULL-guarded (lines 195-200, 255-260). On the `inlen < 17` and MAC-mismatch paths the caller then only sees `-1` | verified |
| 6.79 | `crypto_secretstream_xchacha20poly1305_pull` with `m == NULL` | `mlen > 0` — unlike the AEADs, `_pull` has **no** `m == NULL` verify-only branch; line 245 unconditionally calls `crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, ...)` | **undefined behaviour** (NULL deref) in C. Not a rejection row: a translation must either forbid this at the type level or document it. Only safe when `mlen == 0` | undefined-behaviour-not-tested |
| 6.80 | `crypto_secretstream_xchacha20poly1305_rekey` | none — `void` return, no validation (`secretstream_xchacha20poly1305.c:82-108`) | cannot fail; derives a new `state->k` + inonce and resets the counter to 1. An explicit `_rekey()` on only one side of the session desynchronises it, and every subsequent `_pull` then fails with `-1` (row 6.75) | verified |
| 6.81 | `crypto_secretstream_xchacha20poly1305_pull` with an `ad` that differs from the pushed `ad` (incl. `NULL`/0 vs non-empty of the same content) | MAC covers `ad` and `adlen` (`secretstream_xchacha20poly1305.c:212-214, 230-231`) | returns `-1` via row 6.75 | verified |
| 6.82 | all `*_keygen` (`crypto_aead_aegis128l_keygen`, `_aegis256_keygen`, `_aes256gcm_keygen`, `_chacha20poly1305_keygen`, `_chacha20poly1305_ietf_keygen`, `_xchacha20poly1305_ietf_keygen`, `crypto_secretbox_keygen`, `crypto_secretbox_xsalsa20poly1305_keygen`, `crypto_secretstream_xchacha20poly1305_keygen`) | none | `void` return; cannot fail. `crypto_aead_aes256gcm_keygen` still works despite row 6.30. NB there is **no** `crypto_secretbox_xchacha20poly1305_keygen` in `secretbox_xchacha20poly1305.c` | verified |
| 6.83 | all `*_keybytes`/`_nsecbytes`/`_npubbytes`/`_abytes`/`_messagebytes_max`/`_macbytes`/`_noncebytes`/`_zerobytes`/`_boxzerobytes`/`_statebytes`/`_headerbytes`/`_primitive`/`_tag_*` getters | none | pure constant returns; cannot fail. `crypto_aead_aes256gcm_statebytes()` = `(sizeof(crypto_aead_aes256gcm_state) + 15) & ~15`; `crypto_secretbox_primitive()` = `"xsalsa20poly1305"` | verified |
