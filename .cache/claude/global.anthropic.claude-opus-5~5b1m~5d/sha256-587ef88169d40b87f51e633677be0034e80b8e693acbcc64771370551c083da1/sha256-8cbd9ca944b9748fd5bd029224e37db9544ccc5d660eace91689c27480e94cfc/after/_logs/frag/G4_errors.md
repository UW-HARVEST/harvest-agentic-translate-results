| G4 | crypto_aead_aegis128l_encrypt | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (= SODIUM_MIN(SIZE_MAX-32, 2^61-1)) | `sodium_misuse()` -> user handler then `abort()`; process terminates, never returns |
| G4 | crypto_aead_aegis128l_encrypt_detached | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (checked AFTER `*maclen_p` has already been set to 32) | `sodium_misuse()` -> `abort()`; `*maclen_p` left = 32 |
| G4 | crypto_aead_aegis128l_encrypt_detached | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (same `if`, second disjunct) | `sodium_misuse()` -> `abort()`; `*maclen_p` left = 32 |
| G4 | crypto_aead_aegis128l_decrypt | `clen < crypto_aead_aegis128l_ABYTES` (i.e. clen in 0..31): `if (clen >= ABYTES)` not taken, `ret` stays -1, detached never called | returns -1; `*mlen_p = 0` if `mlen_p != NULL` |
| G4 | crypto_aead_aegis128l_decrypt | `clen >= 32` but underlying tag check fails (ret != 0 from decrypt_detached) | returns -1; `*mlen_p = 0` if `mlen_p != NULL` |
| G4 | crypto_aead_aegis128l_decrypt_detached | `clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` | returns -1 (no misuse/abort; note asymmetry vs encrypt_detached which aborts) |
| G4 | crypto_aead_aegis128l_decrypt_detached | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (same `if`, second disjunct) | returns -1 |
| G4 | crypto_aead_aegis128l_decrypt_detached (aegis128l_soft `decrypt_detached`, aegis128l_common.h) | `crypto_verify_32(computed_mac, mac) != 0` (forged/corrupted 32-byte tag, wrong key, wrong npub, or wrong ad) with `m != NULL` | returns -1 (nonzero from crypto_verify_32); `memset(m, 0, clen)` wipes the plaintext buffer |
| G4 | crypto_aead_aegis128l_decrypt_detached (soft impl) | tag mismatch with `m == NULL` (verify-only mode) | returns -1; no output buffer touched |
| G4 | aegis128l_mac (static, aegis128l_common.h) | `maclen` neither 16 nor 32 (unreachable from public API: maclen is hard-wired to ABYTES=32) | `memset(mac, 0, maclen)` then return -1, which propagates as -1 from encrypt_detached/decrypt_detached |
| G4 | crypto_aead_aegis256_encrypt | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (= SODIUM_MIN(SIZE_MAX-32, 2^61-1)) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_aegis256_encrypt_detached | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (checked after `*maclen_p = 32`) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_aegis256_encrypt_detached | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_aegis256_decrypt | `clen < crypto_aead_aegis256_ABYTES` (clen in 0..31) | returns -1; `*mlen_p = 0` if `mlen_p != NULL` |
| G4 | crypto_aead_aegis256_decrypt | `clen >= 32` but detached tag verification fails | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_aegis256_decrypt_detached | `clen > crypto_aead_aegis256_MESSAGEBYTES_MAX` | returns -1 |
| G4 | crypto_aead_aegis256_decrypt_detached | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` | returns -1 |
| G4 | crypto_aead_aegis256_decrypt_detached (aegis256_soft, aegis256_common.h) | `crypto_verify_32(computed_mac, mac) != 0` with `m != NULL` | returns -1; `memset(m, 0, clen)` |
| G4 | crypto_aead_aegis256_decrypt_detached (soft impl) | tag mismatch with `m == NULL` | returns -1; no output written |
| G4 | aegis256_mac (static, aegis256_common.h) | `maclen` neither 16 nor 32 (unreachable: always ABYTES=32) | `memset(mac, 0, maclen)`, return -1 |
| G4 | crypto_aead_aes256gcm_is_available | this build defines none of HAVE_ARMCRYPTO+NATIVE_LITTLE_ENDIAN nor HAVE_TMMINTRIN_H+HAVE_WMMINTRIN_H, so the stub block in aead_aes256gcm.c is compiled | returns 0 (AES-NI/ARM-crypto AES256-GCM unavailable) — every aes256gcm operation below is a hard rejection regardless of input validity |
| G4 | crypto_aead_aes256gcm_encrypt | ANY input in this build (portable/soft path stub; no HW AES) | sets `errno = ENOSYS` (or ENXIO if ENOSYS undefined) and returns -1; does NOT sodium_misuse/abort; `*clen_p` NOT written |
| G4 | crypto_aead_aes256gcm_encrypt_detached | ANY input in this build | `errno = ENOSYS`; returns -1; `*maclen_p` NOT written |
| G4 | crypto_aead_aes256gcm_decrypt | ANY input in this build (including clen < ABYTES) | `errno = ENOSYS`; returns -1; `*mlen_p` NOT written |
| G4 | crypto_aead_aes256gcm_decrypt_detached | ANY input in this build | `errno = ENOSYS`; returns -1 |
| G4 | crypto_aead_aes256gcm_beforenm | ANY input in this build | `errno = ENOSYS`; returns -1; `st_` left uninitialized |
| G4 | crypto_aead_aes256gcm_encrypt_afternm | ANY input in this build | `errno = ENOSYS`; returns -1 |
| G4 | crypto_aead_aes256gcm_encrypt_detached_afternm | ANY input in this build | `errno = ENOSYS`; returns -1 |
| G4 | crypto_aead_aes256gcm_decrypt_afternm | ANY input in this build | `errno = ENOSYS`; returns -1 |
| G4 | crypto_aead_aes256gcm_decrypt_detached_afternm | ANY input in this build | `errno = ENOSYS`; returns -1 |
| G4 | crypto_aead_aes256gcm_encrypt_detached_afternm (aesni/armcrypto path, COMPILED OUT here) | `ad_len_ > SODIUM_SIZE_MAX \|\| m_len_ > SODIUM_SIZE_MAX` | `sodium_misuse()` -> `abort()`; would apply only on a HW-AES build |
| G4 | crypto_aead_aes256gcm_encrypt_detached_afternm (aesni/armcrypto, COMPILED OUT) | `required_blocks(ad_len,m_len) == 0`: `ad_len > SIZE_MAX-2*PARALLEL_BLOCKS*16` or `m_len > SIZE_MAX-2*PARALLEL_BLOCKS*16` or `ad_len < ad_blocks` or `m_len < m_blocks` or `m_blocks >= (1ULL<<32)-2` (32-bit GCM counter overflow guard) | `memset(mac,0xd0,16)`, `memset(c,0,m_len)`, return -1; `*maclen_p` left 0 |
| G4 | crypto_aead_aes256gcm_decrypt_detached_afternm (aesni/armcrypto, COMPILED OUT) | `ad_len_ > SODIUM_SIZE_MAX \|\| c_len_ > SODIUM_SIZE_MAX` | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_aes256gcm_decrypt_detached_afternm (aesni/armcrypto, COMPILED OUT) | `required_blocks(ad_len, c_len) == 0` (same 5 disjuncts incl. 32-bit block-counter overflow) | returns -1 |
| G4 | crypto_aead_aes256gcm_decrypt_detached_afternm (aesni/armcrypto, COMPILED OUT) | `crypto_verify_16(mac, computed_mac) != 0` with `m != NULL` | `memset(m, 0xd0, c_len)` (note: 0xd0 fill, not 0) and return -1 |
| G4 | crypto_aead_aes256gcm_verify_mac (static, aesni/armcrypto, COMPILED OUT; reached via decrypt_detached_afternm with `m == NULL`) | `required_blocks()==0`, or `crypto_verify_16` mismatch | returns -1 |
| G4 | crypto_aead_aes256gcm_decrypt_afternm (aesni/armcrypto, COMPILED OUT) | `clen < crypto_aead_aes256gcm_ABYTES` (0..15) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_chacha20poly1305_encrypt | `mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX` (= SIZE_MAX - 16) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_chacha20poly1305_encrypt_detached | no length/overflow/NULL check at all (adlen unbounded, mlen unbounded) | always returns 0; `*maclen_p = 16` if non-NULL — no rejection path exists |
| G4 | crypto_aead_chacha20poly1305_decrypt | `clen < crypto_aead_chacha20poly1305_ABYTES` (clen in 0..15) | returns -1; `*mlen_p = 0` if `mlen_p != NULL` |
| G4 | crypto_aead_chacha20poly1305_decrypt | detached call returns nonzero (tag mismatch) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_chacha20poly1305_decrypt_detached | `crypto_verify_16(computed_mac, mac) != 0` with `m != NULL` (bad tag / wrong k / wrong npub / wrong ad / truncated ad) | `memset(m, 0, clen)` then return -1; plaintext never produced |
| G4 | crypto_aead_chacha20poly1305_decrypt_detached | tag mismatch with `m == NULL` (verify-only; m is nullable per `nonnull(3,5,8,9)`) | returns the raw `crypto_verify_16` result (-1); no buffer written |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt | `mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX` (= SODIUM_MIN(SIZE_MAX-16, 64*(2^32-1))) — the 32-bit ChaCha20 block-counter cap | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_chacha20poly1305_ietf_encrypt_detached | no checks (mlen/adlen unbounded here; caller's wrapper does the capping) | always returns 0; `*maclen_p = 16` |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt | `clen < crypto_aead_chacha20poly1305_ietf_ABYTES` (0..15) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt | detached call fails (tag mismatch) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | `crypto_verify_16(computed_mac, mac) != 0` with `m != NULL` | `memset(m, 0, clen)`, return -1 |
| G4 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | tag mismatch with `m == NULL` | returns -1; no buffer written |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt | `mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX` (= SIZE_MAX - 16) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached | no checks (goes straight to hchacha20 + static `_encrypt_detached`) | always returns 0; `*maclen_p = 16` |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt | `clen < crypto_aead_xchacha20poly1305_ietf_ABYTES` (0..15) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt | detached call fails (tag mismatch) | returns -1; `*mlen_p = 0` |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached (static `_decrypt_detached`) | `crypto_verify_16(computed_mac, mac) != 0` with `m != NULL` | `memset(m, 0, clen)`, return -1 |
| G4 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | tag mismatch with `m == NULL` | returns -1; no buffer written |
| G4 | crypto_secretbox_easy | `mlen > crypto_secretbox_MESSAGEBYTES_MAX` (= SIZE_MAX - 16) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_secretbox_detached | no length check, no NULL check; only pointer-overlap memmove fixups | always returns 0 — no rejection path |
| G4 | crypto_secretbox_open_easy | `clen < crypto_secretbox_MACBYTES` (clen in 0..15) | returns -1 before touching m |
| G4 | crypto_secretbox_open_easy | `clen >= 16` but open_detached fails (tag mismatch) | returns -1 |
| G4 | crypto_secretbox_open_detached | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` (bad 16-byte mac / wrong k / wrong n) | `sodium_memzero(subkey)`, return -1; m NOT zeroed and NOT written (differs from AEAD behavior) |
| G4 | crypto_secretbox_open_detached | `m == NULL` with valid mac (verify-only mode) | returns 0 early after zeroing subkey; no plaintext written |
| G4 | crypto_secretbox_xchacha20poly1305_easy | `mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX` (= SIZE_MAX - 16) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_secretbox_xchacha20poly1305_detached | no checks | always returns 0 |
| G4 | crypto_secretbox_xchacha20poly1305_open_easy | `clen < crypto_secretbox_xchacha20poly1305_MACBYTES` (0..15) | returns -1 |
| G4 | crypto_secretbox_xchacha20poly1305_open_detached | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` | `sodium_memzero(subkey)`, return -1; m untouched |
| G4 | crypto_secretbox_xchacha20poly1305_open_detached | `m == NULL` with valid mac | returns 0 early; no plaintext written |
| G4 | crypto_secretbox / crypto_secretbox_xsalsa20poly1305 (NaCl zero-padded low-level API) | `mlen < 32` (i.e. `mlen < crypto_secretbox_ZEROBYTES`; the caller must supply 32 leading zero bytes) | returns -1 before any stream/auth work; c untouched |
| G4 | crypto_secretbox_open / crypto_secretbox_xsalsa20poly1305_open (NaCl low-level API) | `clen < 32` (`< ZEROBYTES`) | returns -1; m untouched |
| G4 | crypto_secretbox_open / crypto_secretbox_xsalsa20poly1305_open | `crypto_onetimeauth_poly1305_verify(c+16, c+32, clen-32, subkey) != 0` (bad mac at c[16..32), or c[0..16) not zero is NOT checked) | returns -1; m untouched, no ACQUIRE_FENCE/decrypt |
| G4 | crypto_secretstream_xchacha20poly1305_init_push | no failure condition (randombytes_buf + hchacha20) | always returns 0 |
| G4 | crypto_secretstream_xchacha20poly1305_init_pull | no failure condition; any 24-byte header accepted, no validation | always returns 0 |
| G4 | crypto_secretstream_xchacha20poly1305_push | `mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` (= SODIUM_MIN(SIZE_MAX-17, 64*(2^32-2))) — checked after `*outlen_p` set to 0 | `sodium_misuse()` -> `abort()`; `*outlen_p` left 0 |
| G4 | crypto_secretstream_xchacha20poly1305_push | out-of-range / arbitrary `tag` byte (e.g. 0x04..0xFF, or 0x03 = TAG_FINAL) | NO rejection: any tag byte is accepted verbatim and encrypted; only bit `TAG_REKEY` (0x02) is interpreted, forcing an implicit rekey. Returns 0 |
| G4 | crypto_secretstream_xchacha20poly1305_push | 32-bit message counter wraps: `sodium_increment(STATE_COUNTER,4)` makes `sodium_is_zero(STATE_COUNTER,4)` true (after 2^32-1 pushes) | NOT an error: implicit `crypto_secretstream_xchacha20poly1305_rekey(state)` and counter reset to 1; still returns 0 |
| G4 | crypto_secretstream_xchacha20poly1305_pull | `inlen < crypto_secretstream_xchacha20poly1305_ABYTES` (= 17; inlen in 0..16) | returns -1; `*mlen_p = 0`, `*tag_p = 0xff` if pointers non-NULL |
| G4 | crypto_secretstream_xchacha20poly1305_pull | `inlen - 17 > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` | `sodium_misuse()` -> `abort()` |
| G4 | crypto_secretstream_xchacha20poly1305_pull | `sodium_memcmp(mac, stored_mac, 16) != 0` (forged ciphertext, wrong ad, wrong key, out-of-order message, or replayed message) | `sodium_memzero(mac)`, return -1; m NOT written/zeroed; `*mlen_p` stays 0, `*tag_p` stays 0xff; state left un-advanced |
| G4 | crypto_secretstream_xchacha20poly1305_pull | `m == NULL` (not permitted: header declares `nonnull` on m; the code unconditionally calls `crypto_stream_chacha20_ietf_xor_ic(m, ...)`) | undefined behaviour / segfault — there is NO verify-only mode, unlike the AEAD decrypt_detached family |
| G4 | crypto_secretstream_xchacha20poly1305_rekey | none (void return) | cannot fail |
| G4 | crypto_stream_chacha20 | `clen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX) | `sodium_misuse()` -> `abort()` (unreachable when unsigned long long == size_t) |
| G4 | crypto_stream_chacha20_xor | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_stream_chacha20_xor_ic | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX). NOTE: 64-bit `ic` is NOT range-checked — the internal counter simply wraps j12/j13 | `sodium_misuse()` -> `abort()` on the length; no ic rejection exists |
| G4 | crypto_stream_chacha20_ietf | `clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` (= SODIUM_MIN(SIZE_MAX, 64*(2^32-1))) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_stream_chacha20_ietf_xor | `mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` | `sodium_misuse()` -> `abort()` |
| G4 | crypto_stream_chacha20_ietf_xor_ic | 32-bit counter overflow guard: `(unsigned long long) ic > (64ULL*(1ULL<<32))/64ULL - (mlen+63ULL)/64ULL`, i.e. `ic + ceil(mlen/64) > 2^32` (e.g. ic = 2^32-1 with mlen = 65) | `sodium_misuse()` -> `abort()`; this is the ONLY ic check in the module (note mlen itself is not separately capped here) |
| G4 | crypto_stream_chacha20_ietf_ext | `clen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX; the non-ietf cap) | `sodium_misuse()` -> `abort()` |
| G4 | crypto_stream_chacha20_ietf_ext_xor_ic | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX`. Deliberately has NO ic overflow guard — the 32-bit counter is allowed to carry into the IV | `sodium_misuse()` -> `abort()` on length only |
| G4 | crypto_stream_chacha20_ietf_ext_xor (static, internal) | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` | `sodium_misuse()` -> `abort()` |
| G4 | chacha20 ref `stream_ref` / `stream_ietf_ext_ref` / `stream_ref_xor_ic` / `stream_ietf_ext_ref_xor_ic` | `clen`/`mlen == 0` | early `return 0`, output buffer untouched (not an error, but a distinct short-circuit path) |
| G4 | crypto_stream_salsa20 | none — the wrapper does NO length check (no MESSAGEBYTES_MAX guard, unlike chacha20) | always returns 0 |
| G4 | crypto_stream_salsa20_xor / crypto_stream_salsa20_xor_ic | none — no length check, no 64-bit `ic` check (`ic` is written straight into `in[8..16]`, then byte-wise incremented and allowed to wrap) | always returns 0 |
| G4 | salsa20 ref `stream_ref` / `stream_ref_xor_ic` | `clen`/`mlen == 0` | early `return 0`, output untouched |
| G4 | crypto_stream_salsa2012 / crypto_stream_salsa2012_xor | no checks of any kind; there is no `_xor_ic` entry point for salsa2012 | always returns 0 (`clen==0`/`mlen==0` short-circuits to 0 first) |
| G4 | crypto_stream_salsa208 / crypto_stream_salsa208_xor | no checks of any kind; no `_xor_ic` entry point for salsa208 | always returns 0 (`clen==0`/`mlen==0` short-circuits to 0 first) |
| G4 | crypto_stream_xsalsa20 / _xor / _xor_ic | no checks; returns whatever `crypto_stream_salsa20*` returns (always 0). No ic guard | always returns 0 |
| G4 | crypto_stream_xchacha20 | no own check; delegates to `crypto_stream_chacha20`, so inherits `clen > SODIUM_SIZE_MAX` -> misuse | returns 0, or aborts via the inherited chacha20 misuse |
| G4 | crypto_stream_xchacha20_xor / crypto_stream_xchacha20_xor_ic | no own check; delegates to `crypto_stream_chacha20_xor_ic`, inheriting `mlen > SODIUM_SIZE_MAX` -> misuse. 64-bit `ic` unchecked | returns 0, or aborts via inherited chacha20 misuse |
| G4 | crypto_stream / crypto_stream_xor (default = xsalsa20) | no checks | always returns 0 |
| G4 | crypto_core_salsa20 / crypto_core_salsa2012 / crypto_core_salsa208 | no validation of any argument; `c == NULL` selects the "expand 32-byte k" sigma constants, `c != NULL` loads 16 bytes from c | always returns 0 — no rejection path |
| G4 | crypto_core_hsalsa20 | no validation; `c == NULL` -> sigma constants, else `LOAD32_LE(c+0..12)` | always returns 0 — no rejection path |
| G4 | crypto_core_hchacha20 | no validation; `c == NULL` -> sigma constants, else `LOAD32_LE(c+0..12)` | always returns 0 — no rejection path |
| G4 | all G4 `*_keygen`, `*_keybytes`, `*_noncebytes`, `*_npubbytes`, `*_nsecbytes`, `*_abytes`, `*_macbytes`, `*_zerobytes`, `*_boxzerobytes`, `*_headerbytes`, `*_statebytes`, `*_messagebytes_max`, `*_outputbytes`, `*_inputbytes`, `*_constbytes`, `*_primitive`, `*_tag_*` getters | no arguments validated; `nonnull` on keygen buffers is a compiler attribute only (passing NULL is UB, not a checked rejection) | never fail (void / constant return); no error path |
| G4 | (whole G4 surface) | NULL pointer passed for any `nonnull`-annotated parameter (c, m, n, k, npub, mac, state) | NO runtime NULL check exists anywhere in G4 — `__attribute__((nonnull(...)))` is compile-time only; result is undefined behaviour / segfault, not -1. The only intentionally nullable pointers are `nsec` (always ignored via `(void) nsec`, never dereferenced, no `abort()` on non-NULL), `clen_p`/`mlen_p`/`maclen_p`/`outlen_p`/`tag_p` (skipped when NULL), `ad` (only read when `adlen > 0`), the core `c` constant, and `m` in the AEAD/secretbox `*_open_detached`/`*_decrypt_detached` verify-only mode |
| G4 | (whole G4 surface) | no `sodium_is_zero` shared-key / weak-key rejection is present in G4 — that check lives in crypto_box/crypto_kx, not here | n/a: no such rejection path in these modules |
