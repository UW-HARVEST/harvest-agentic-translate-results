# AEAD + SECRETBOX + SECRETSTREAM — error / rejection paths

All rejections in the C ground-truth source for this family, and the result the
C code returns. Every row is exercised in `tests/aead.rs` (Phase C) and asserted
to match on BOTH the C and the Rust `.so`.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | `crypto_aead_*_encrypt` (all families) | `mlen > MESSAGEBYTES_MAX` | `-1` (guarded; not hit with in-range test sizes) |
| 2 | `crypto_aead_*_decrypt` (combined) | `clen < ABYTES` (truncated ciphertext) | `-1`, `*mlen_p = 0` |
| 3 | `crypto_aead_*_decrypt` / `_decrypt_detached` | tampered ciphertext byte -> Poly1305/AEGIS tag mismatch | `-1` (plaintext zeroed) |
| 4 | `crypto_aead_*_decrypt` / `_decrypt_detached` | tampered authentication tag/MAC | `-1` |
| 5 | `crypto_aead_*_decrypt` / `_decrypt_detached` | tampered AAD | `-1` |
| 6 | `crypto_aead_*_decrypt` / `_decrypt_detached` | wrong nonce (`npub`) | `-1` |
| 7 | `crypto_aead_*_decrypt` / `_decrypt_detached` | wrong key | `-1` |
| 8 | `crypto_aead_aegis128l/256_{encrypt,decrypt}` | `mlen`/`adlen`/`clen > MESSAGEBYTES_MAX` | `-1` (guarded) |
| 9 | `crypto_aead_aes256gcm_*` (one-shot) | called when `crypto_aead_aes256gcm_is_available() == 0` | `sodium_misuse()` -> `abort()` (NOT reachable on this host; body skipped, availability parity asserted) |
| 10 | `crypto_secretbox_easy` / `_detached` | `mlen > MESSAGEBYTES_MAX` | `-1` (guarded) |
| 11 | `crypto_secretbox_open_easy` | `clen < MACBYTES` (truncated) | `-1` |
| 12 | `crypto_secretbox_open_easy` / `_open_detached` | tampered ciphertext or MAC | `-1` |
| 13 | `crypto_secretbox_open_easy` / `_open_detached` | wrong nonce | `-1` |
| 14 | `crypto_secretbox_xsalsa20poly1305` (NaCl padded encrypt) | `mlen < 32` (`crypto_secretbox_ZEROBYTES`) | `-1` |
| 15 | `crypto_secretbox_xsalsa20poly1305_open` (NaCl padded open) | `clen < 32` | `-1` |
| 16 | `crypto_secretbox_xsalsa20poly1305_open` | Poly1305 verify fails (tampered / wrong nonce / wrong key) | `-1` |
| 17 | `crypto_secretbox` / `crypto_secretbox_open` | same as rows 14–16 (NaCl padded default = xsalsa20poly1305) | `-1` |
| 18 | `crypto_secretstream_xchacha20poly1305_pull` | `clen < ABYTES` (17) — truncated | `-1`, `*tag_p` set to `0xff` first |
| 19 | `crypto_secretstream_xchacha20poly1305_pull` | tampered ciphertext / MAC (`sodium_memcmp` fails) | `-1` |
| 20 | `crypto_secretstream_xchacha20poly1305_pull` | wrong key supplied to `init_pull` | `-1` (auth failure) |
| 21 | `crypto_secretstream_xchacha20poly1305_pull` | tampered header supplied to `init_pull` | `-1` (auth failure) |

Notes:
- For the AEAD one-shot combined `*_decrypt`, `clen < ABYTES` short-circuits and
  returns `-1` without calling the detached path (see
  `aead_chacha20poly1305.c`); `*mlen_p` is set to `0`.
- `*_decrypt_detached` with `m == NULL` returns the raw verify result rather than
  `-1`; not exercised (all tests pass a real output buffer).
- AES-256-GCM is hardware-gated. On the CI/build host used here
  `crypto_aead_aes256gcm_is_available()` returns `0` on BOTH libraries, so the
  crypto body is skipped to avoid the intentional `abort()`; only availability
  parity is asserted. The precomputed beforenm/afternm code path is covered when
  hardware AES is present.
