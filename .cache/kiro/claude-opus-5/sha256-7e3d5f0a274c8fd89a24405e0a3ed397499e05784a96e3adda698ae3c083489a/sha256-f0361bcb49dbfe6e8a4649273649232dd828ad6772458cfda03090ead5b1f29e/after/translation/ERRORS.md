# ERRORS.md — ERROR-SURFACE TABLE

Derived mechanically from the C source in `c_src/libsodium/` by grepping every
`return -1`, `return NULL`, `sodium_misuse()`, `errno = …`, `ARGON2_*` error
enum, `assert`, explicit range/bounds `if`, and every `*_MIN` / `*_MAX`
constant. One row per DISTINCT rejection branch.

## Build configuration assumed by this table

The CMake build defines **no `HAVE_*` macros**, so every `#ifdef HAVE_*`
selects the portable `#else` fallback. Consequences that change the error
surface:

- `crypto_aead_aes256gcm_*` compile to **ENOSYS stubs** (`errno = ENOSYS;
  return -1`), and `crypto_aead_aes256gcm_is_available()` returns `0`.
- `sodium_mlock` / `sodium_munlock` / `sodium_mprotect_*` are **always**
  `errno = ENOSYS; return -1`.
- `sodium_malloc` is plain `malloc` + `memset 0xdb` — no guard pages, no
  canary, so the `SIZE_MAX - page_size*4` and canary `sodium_misuse()` rows
  are **unreachable**.
- `sodium_crit_enter` / `sodium_crit_leave` are no-op stubs returning `0`, so
  every `return -1 /* LCOV_EXCL */` guarded by them is **unreachable**.
- `fe_25_5` (not `fe_51`) is used (`HAVE_TI_MODE` undefined). No behavioural
  difference in outputs.
- `crypto_verify_*` scalar, poly1305 donna32, ipcrypt soft-AES.
- `sodium_runtime_has_{sse2,sse3,ssse3,sse41,avx,avx2,avx512f,pclmul,aesni}`
  all report `0`.

## `sodium_misuse()` semantics

`sodium_misuse()` calls the user handler installed by
`sodium_set_misuse_handler()` (if any) and then calls `abort()`
**unconditionally**. Differential tests for misuse rows therefore run the call
in a `fork()`ed child and compare the child's termination status (SIGABRT)
between C and Rust.

## Legend

- **R** = reachable in this build; **U** = present in source but unreachable
  in this build (documented for completeness, not tested).

---

## Group 1 — `sodium/` core, utils, codecs, randombytes, verify, shorthash, onetimeauth, ipcrypt

| # | function | trigger (exact invalid input/condition) | expected C result | R/U |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `sodium_mlock` | any call (portable fallback) | `errno=ENOSYS`, `-1` | R |
| 2 | `sodium_munlock` | any call (zeroes buffer first) | `errno=ENOSYS`, `-1` | R |
| 3 | `sodium_mprotect_noaccess` | any call | `errno=ENOSYS`, `-1` | R |
| 4 | `sodium_mprotect_readonly` | any call | `errno=ENOSYS`, `-1` | R |
| 5 | `sodium_mprotect_readwrite` | any call | `errno=ENOSYS`, `-1` | R |
| 6 | `sodium_allocarray` | `count>0 && size >= SIZE_MAX/count` | `errno=ENOMEM`, `NULL` | R |
| 7 | `_sodium_malloc` (ALIGNED) | `size >= SIZE_MAX - page_size*4` | `errno=ENOMEM`, `NULL` | U |
| 8 | `_sodium_malloc` (ALIGNED) | `page_size <= sizeof canary` | `sodium_misuse()` | U |
| 9 | `_unprotected_ptr_from_user_ptr` | `unprotected_ptr_u <= page_size*2` | `sodium_misuse()` | U |
| 10 | `sodium_pad` | `blocksize == 0` | `-1` | R |
| 11 | `sodium_pad` | `xpadded_len >= max_buflen` | `-1` | R |
| 12 | `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen` | `sodium_misuse()` | R |
| 13 | `sodium_unpad` | `blocksize == 0` | `-1` | R |
| 14 | `sodium_unpad` | `padded_buflen < blocksize` | `-1` | R |
| 15 | `sodium_unpad` | no `0x80` barrier found in last block (`valid==0`) | `-1` | R |
| 16 | `sodium_bin2hex` | `bin_len >= SIZE_MAX/2` | `sodium_misuse()` | U (needs huge len) |
| 17 | `sodium_bin2hex` | `hex_maxlen <= bin_len*2` | `sodium_misuse()` | R |
| 18 | `sodium_hex2bin` | decoded byte would exceed `bin_maxlen` | `errno=ERANGE`, `-1` | R |
| 19 | `sodium_hex2bin` | odd number of hex nibbles (`state != 0`) | `errno=EINVAL`, `-1` | R |
| 20 | `sodium_hex2bin` | `hex_end==NULL` and trailing non-hex junk remains | `errno=EINVAL`, `-1` | R |
| 21 | `sodium_base64_check_variant` (all base64 fns) | `(variant & ~0x6) != 0x1` — i.e. any variant not in {1,3,5,7} | `sodium_misuse()` | R |
| 22 | `sodium_base64_encoded_len` | `bin_len/3 > (SIZE_MAX-5)/4` | `sodium_misuse()` | U (needs huge len) |
| 23 | `sodium_bin2base64` | `nibbles > (SIZE_MAX-5)/4` | `sodium_misuse()` | U (needs huge len) |
| 24 | `sodium_bin2base64` | `b64_maxlen <= b64_len` | `sodium_misuse()` | R |
| 25 | `sodium_base642bin` | decoded byte would exceed `bin_maxlen` | `errno=ERANGE`, `-1` | R |
| 26 | `sodium_base642bin` | dangling bits: `acc_len>4 \|\| (acc & ((1<<acc_len)-1)) != 0` | `-1` | R |
| 27 | `_sodium_base642bin_skip_padding` | input exhausted while `=` still expected | `errno=ERANGE`, `-1` | R |
| 28 | `_sodium_base642bin_skip_padding` | non-`=`, non-ignored char inside padding | `errno=EINVAL`, `-1` | R |
| 29 | `sodium_base642bin` | `b64_end==NULL` and trailing junk remains | `errno=EINVAL`, `-1` | R |
| 30 | `sodium_ip2bin` | zone id contains char outside `[0-9a-zA-Z._-]` | `-1` | R |
| 31 | `sodium_ip2bin` | empty zone id (`zone+1 >= end`) | `-1` | R |
| 32 | `sodium_ip2bin` | zone present but address is not IPv6 | `-1` | R |
| 33 | `sodium_ip2bin` | IPv6 text fails `parse_ipv6` | `-1` | R |
| 34 | `sodium_ip2bin` | IPv4 text fails `parse_ipv4` | `-1` | R |
| 35 | `sodium_bin2ip` | `ip_maxlen <= 2` | `NULL` | R |
| 36 | `sodium_bin2ip` | IPv4-mapped output length `>= ip_maxlen` | `NULL` | R |
| 37 | `sodium_bin2ip` | IPv6 output length `>= ip_maxlen` | `NULL` | R |
| 38 | `sodium_init` | `sodium_crit_enter/leave != 0` | `-1` | U |
| 39 | `sodium_crit_leave` | `locked == 0` | `errno=EPERM`, `-1` | U |
| 40 | `sodium_set_misuse_handler` | `sodium_crit_enter/leave != 0` | `-1` | U |
| 41 | `sodium_misuse` | unconditional | handler() then `abort()` | R |
| 42 | `randombytes_buf_deterministic` | `size > 0x4000000000` | `sodium_misuse()` | R (huge; fork-tested only conceptually) |
| 43 | `randombytes_sysrandom_buf` | `/dev/urandom` open fails or short read | `sodium_misuse()` | R (env-dependent, not tested) |
| 44 | `randombytes_sysrandom_close` | fd not open / `close()` fails | `-1` | R |
| 45 | `crypto_onetimeauth_verify`, `crypto_onetimeauth_poly1305_verify` | recomputed tag ≠ `h` | `-1` | R |
| 46 | `crypto_verify_16` | any byte of the 16 differs | `-1` | R |
| 47 | `crypto_verify_32` | any byte of the 32 differs | `-1` | R |
| 48 | `crypto_verify_64` | any byte of the 64 differs | `-1` | R |
| 49 | `sodium_memcmp` | any byte of `len` differs | `-1` | R |

## Group 2 — `crypto_aead/`, `crypto_secretbox/`, `crypto_secretstream/`, `crypto_box/`, `crypto_stream/`

Constants: `chacha20poly1305` NPUB=8, ABYTES=16, K=32; `_ietf` NPUB=12;
`xchacha20poly1305_ietf` NPUB=24, ABYTES=16; `aegis128l` K=16, NPUB=16,
ABYTES=32; `aegis256` K=32, NPUB=32, ABYTES=32; secretbox/box MACBYTES=16,
NONCEBYTES=24, ZEROBYTES=32, BOXZEROBYTES=16; box PK=SK=BEFORENM=32,
SEALBYTES=48; secretstream ABYTES=17, HEADERBYTES=24, K=32,
TAG_MESSAGE=0x00, TAG_PUSH=0x01, TAG_REKEY=0x02, TAG_FINAL=0x03.

| # | function | trigger (exact invalid input/condition) | expected C result | R/U |
|---|----------|------------------------------------------|-------------------|-----|
| 50 | `crypto_aead_chacha20poly1305_encrypt` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 51 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `mlen > ietf_MESSAGEBYTES_MAX` (2^38) | `sodium_misuse()` | R |
| 52 | `crypto_aead_chacha20poly1305_decrypt` | `clen < 16` | `-1` (detached never called) | R |
| 53 | `crypto_aead_chacha20poly1305_decrypt_detached` | MAC mismatch, `m != NULL` | `memset(m,0,mlen)`; `-1` | R |
| 54 | `crypto_aead_chacha20poly1305_decrypt_detached` | MAC mismatch, `m == NULL` | `-1` (no memset) | R |
| 55 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen < 16` | `-1` | R |
| 56 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | MAC mismatch, `m != NULL` | `memset(m,0,mlen)`; `-1` | R |
| 57 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | MAC mismatch, `m == NULL` | `-1` | R |
| 58 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 59 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen < 16` | `-1` | R |
| 60 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | MAC mismatch, `m != NULL` | `memset(m,0,mlen)`; `-1` | R |
| 61 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | MAC mismatch, `m == NULL` | `-1` | R |
| 62 | `crypto_aead_aegis128l_encrypt` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 63 | `crypto_aead_aegis128l_encrypt_detached` | `mlen > MAX \|\| adlen > MAX` | `sodium_misuse()` (after `*maclen_p=32`) | R |
| 64 | `crypto_aead_aegis128l_decrypt` | `clen < 32` | `-1` | R |
| 65 | `crypto_aead_aegis128l_decrypt_detached` | `clen > MAX \|\| adlen > MAX` | `-1` | R |
| 66 | `crypto_aead_aegis128l_decrypt_detached` | tag verify fails | `memset(m,0,mlen)` if `m`; `-1` | R |
| 67 | `crypto_aead_aegis256_encrypt` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 68 | `crypto_aead_aegis256_encrypt_detached` | `mlen > MAX \|\| adlen > MAX` | `sodium_misuse()` | R |
| 69 | `crypto_aead_aegis256_decrypt` | `clen < 32` | `-1` | R |
| 70 | `crypto_aead_aegis256_decrypt_detached` | `clen > MAX \|\| adlen > MAX` | `-1` | R |
| 71 | `crypto_aead_aegis256_decrypt_detached` | tag verify fails | `memset(m,0,mlen)` if `m`; `-1` | R |
| 72 | `crypto_aead_aes256gcm_encrypt` | any call | `errno=ENOSYS`, `-1` | R |
| 73 | `crypto_aead_aes256gcm_encrypt_detached` | any call | `errno=ENOSYS`, `-1` | R |
| 74 | `crypto_aead_aes256gcm_decrypt` | any call | `errno=ENOSYS`, `-1` | R |
| 75 | `crypto_aead_aes256gcm_decrypt_detached` | any call | `errno=ENOSYS`, `-1` | R |
| 76 | `crypto_aead_aes256gcm_beforenm` | any call | `errno=ENOSYS`, `-1` | R |
| 77 | `crypto_aead_aes256gcm_encrypt_afternm` | any call | `errno=ENOSYS`, `-1` | R |
| 78 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | any call | `errno=ENOSYS`, `-1` | R |
| 79 | `crypto_aead_aes256gcm_decrypt_afternm` | any call | `errno=ENOSYS`, `-1` | R |
| 80 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | any call | `errno=ENOSYS`, `-1` | R |
| 81 | `crypto_aead_aes256gcm_is_available` | any call | `0` | R |
| 82 | `crypto_secretbox_easy` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 83 | `crypto_secretbox_open_easy` | `clen < 16` | `-1` | R |
| 84 | `crypto_secretbox_open_detached` | poly1305 verify fails | `-1` | R |
| 85 | `crypto_secretbox_xsalsa20poly1305` (padded) | `mlen < 32` | `-1` | R |
| 86 | `crypto_secretbox_xsalsa20poly1305_open` (padded) | `clen < 32` | `-1` | R |
| 87 | `crypto_secretbox_xsalsa20poly1305_open` | poly1305 verify fails | `-1` | R |
| 88 | `crypto_secretbox_xchacha20poly1305_easy` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 89 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen < 16` | `-1` | R |
| 90 | `crypto_secretbox_xchacha20poly1305_open_detached` | poly1305 verify fails | `-1` | R |
| 91 | `crypto_secretstream_xchacha20poly1305_push` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 92 | `crypto_secretstream_xchacha20poly1305_pull` | `inlen < 17` | `-1` | R |
| 93 | `crypto_secretstream_xchacha20poly1305_pull` | `inlen-17 > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 94 | `crypto_secretstream_xchacha20poly1305_pull` | `sodium_memcmp(mac, stored, 16) != 0` | `sodium_memzero(mac)`; `-1` | R |
| 95 | `crypto_box_seal` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 96 | `crypto_box_seal_open` | `clen < 48` | `-1` | R |
| 97 | `crypto_box_easy` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 98 | `crypto_box_easy_afternm` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 99 | `crypto_box_detached` | `crypto_box_beforenm != 0` (small-order/zero peer pk) | `-1` | R |
| 100 | `crypto_box_open_detached` | `crypto_box_beforenm != 0` | `-1` | R |
| 101 | `crypto_box_open_easy` | `clen < 16` | `-1` | R |
| 102 | `crypto_box_open_easy_afternm` | `clen < 16` | `-1` | R |
| 103 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | `crypto_scalarmult_curve25519 != 0` | `-1` | R |
| 104 | `crypto_box_curve25519xsalsa20poly1305` | `beforenm != 0` | `-1` | R |
| 105 | `crypto_box_curve25519xsalsa20poly1305_open` | `beforenm != 0` | `-1` | R |
| 106 | `crypto_box_curve25519xsalsa20poly1305_afternm` | `mlen < 32` (delegates to padded secretbox) | `-1` | R |
| 107 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | `clen < 32` or verify fail | `-1` | R |
| 108 | `crypto_box_curve25519xchacha20poly1305_beforenm` | `scalarmult != 0` | `-1` | R |
| 109 | `crypto_box_curve25519xchacha20poly1305_easy` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 110 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 111 | `crypto_box_curve25519xchacha20poly1305_detached` | `beforenm != 0` | `-1` | R |
| 112 | `crypto_box_curve25519xchacha20poly1305_open_detached` | `beforenm != 0` | `-1` | R |
| 113 | `crypto_box_curve25519xchacha20poly1305_open_easy` | `clen < 16` | `-1` | R |
| 114 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | `clen < 16` | `-1` | R |
| 115 | `crypto_box_curve25519xchacha20poly1305_seal` | `mlen > MESSAGEBYTES_MAX` | `sodium_misuse()` | R |
| 116 | `crypto_box_curve25519xchacha20poly1305_seal_open` | `clen < 48` | `-1` | R |
| 117 | `crypto_stream_chacha20`, `_xor`, `_xor_ic` | `len > MESSAGEBYTES_MAX` | `sodium_misuse()` | U (needs SIZE_MAX) |
| 118 | `crypto_stream_chacha20_ietf`, `_ietf_xor` | `len > 2^38` | `sodium_misuse()` | R |
| 119 | `crypto_stream_chacha20_ietf_xor_ic` | `ic > 2^32 - ceil(mlen/64)` (32-bit counter overflow) | `sodium_misuse()` | R |

## Group 3 — `crypto_sign/`, `crypto_scalarmult/`, `crypto_core/ed25519`, `crypto_kx/`, `crypto_kem/`

Constants: curve25519/ed25519/ristretto255 `BYTES=SCALARBYTES=32`,
`UNIFORMBYTES=32`, `HASHBYTES=64`, `NONREDUCEDSCALARBYTES=64`;
`crypto_sign_ed25519` `BYTES=64`, `SEEDBYTES=32`, `PUBLICKEYBYTES=32`,
`SECRETKEYBYTES=64`; `crypto_kx_*=32`; `mlkem768` PK=1184, SK=2400, CT=1088,
SS=32, SEED=64; `xwing` (= default `crypto_kem`) PK=1216, SK=32, CT=1120,
SS=32, SEED=32.

| # | function | trigger (exact invalid input/condition) | expected C result | R/U |
|---|----------|------------------------------------------|-------------------|-----|
| 120 | `crypto_scalarmult_curve25519` | `p` has small order (7-entry blocklist, incl. all-zero) | `-1` | R |
| 121 | `crypto_scalarmult_curve25519` | output `q` all-zero after ladder | `-1` | R |
| 122 | `crypto_scalarmult_ed25519` | `ge25519_is_canonical(p) == 0` | `-1` | R |
| 123 | `crypto_scalarmult_ed25519` | `ge25519_frombytes(p) != 0` | `-1` | R |
| 124 | `crypto_scalarmult_ed25519` | `ge25519_has_small_order(p) != 0` | `-1` | R |
| 125 | `crypto_scalarmult_ed25519` | `ge25519_is_on_main_subgroup(p) == 0` | `-1` | R |
| 126 | `crypto_scalarmult_ed25519` | result is identity, or `n` all-zero | `-1` | R |
| 127 | `crypto_scalarmult_ed25519_noclamp` | same four point gates as 122–125 | `-1` | R |
| 128 | `crypto_scalarmult_ed25519_noclamp` | result identity / `n` all-zero | `-1` | R |
| 129 | `crypto_scalarmult_ed25519_base` | result identity / `n` all-zero | `-1` | R |
| 130 | `crypto_scalarmult_ed25519_base_noclamp` | result identity / `n` all-zero | `-1` | R |
| 131 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes(p) != 0` | `-1` | R |
| 132 | `crypto_scalarmult_ristretto255` | output `q` all-zero | `-1` | R |
| 133 | `crypto_scalarmult_ristretto255_base` | output `q` all-zero (`n` ≡ 0 mod L) | `-1` | R |
| 134 | `crypto_core_ed25519_is_valid_point` | non-canonical / decode fail / off curve / small order / not on main subgroup | `0` (**not** `-1`) | R |
| 135 | `crypto_core_ed25519_add` | `p` or `q` fails frombytes or `is_on_curve` | `-1` | R |
| 136 | `crypto_core_ed25519_sub` | `p` or `q` fails frombytes or `is_on_curve` | `-1` | R |
| 137 | `crypto_core_ed25519_scalar_invert` | `s` all-zero | `-1` | R |
| 138 | `crypto_core_ed25519_from_string`/`_nu` | bad `hash_alg` | `errno=EINVAL`, `-1` | R |
| 138a | `crypto_core_ed25519_from_uniform` / `_from_hash` | n/a — **not exported** by this build (neither C nor Rust `.so`), so out of scope | — | U |
| 139 | `crypto_core_ed25519_scalar_from_string` | bad `hash_alg` | `errno=EINVAL`, `-1` | R |
| 140 | `core_h2c_string_to_hash` | `hash_alg` not SHA256/SHA512 | `errno=EINVAL`, `-1` | R |
| 141 | `crypto_core_ristretto255_is_valid_point` | `ristretto255_frombytes != 0` | `0` | R |
| 142 | `crypto_core_ristretto255_add` | `p` or `q` fails frombytes | `-1` | R |
| 143 | `crypto_core_ristretto255_sub` | `p` or `q` fails frombytes | `-1` | R |
| 144 | `crypto_core_ristretto255_scalar_invert` | `s` all-zero | `-1` | R |
| 145 | `crypto_core_ristretto255_scalar_from_string` | bad `hash_alg` | `errno=EINVAL`, `-1` | R |
| 146 | `crypto_sign_ed25519_verify_detached` | `(sig[63] & 240) != 0 && sc25519_is_canonical(sig+32) == 0` (non-canonical S) | `-1` | R |
| 147 | `crypto_sign_ed25519_verify_detached` | `ge25519_is_canonical(pk) == 0` (non-canonical A) | `-1` | R |
| 148 | `crypto_sign_ed25519_verify_detached` | `frombytes_negate_vartime(pk) != 0` or `has_small_order(A) != 0` | `-1` | R |
| 149 | `crypto_sign_ed25519_verify_detached` | `ge25519_frombytes(sig) != 0` or `has_small_order(expected_r) != 0` | `-1` | R |
| 150 | `crypto_sign_ed25519_verify_detached` | signature does not match message | `-1` | R |
| 151 | `crypto_sign_ed25519_open` | `smlen < 64` | `-1`, `*mlen_p = 0` | R |
| 152 | `crypto_sign_ed25519_open` | `smlen - 64 > MESSAGEBYTES_MAX` | `-1` | U |
| 153 | `crypto_sign_ed25519_open` | `verify_detached != 0` | zero `m`; `-1` | R |
| 154 | `crypto_sign_ed25519_pk_to_curve25519` | frombytes_negate fail / small order / not on main subgroup | `-1` | R |
| 155 | `crypto_sign_ed25519ph_final_verify` | any of rows 146–150 (prehashed=1) | `-1` | R |
| 156 | `crypto_kx_client_session_keys` | `rx == NULL && tx == NULL` | `sodium_misuse()` | R |
| 157 | `crypto_kx_client_session_keys` | `crypto_scalarmult != 0` (small-order/zero peer pk) | `-1` | R |
| 158 | `crypto_kx_server_session_keys` | `rx == NULL && tx == NULL` | `sodium_misuse()` | R |
| 159 | `crypto_kx_server_session_keys` | `crypto_scalarmult != 0` | `-1` | R |
| 160 | `crypto_kem_mlkem768_enc` | `pk` not canonical (any coeff ≥ Q=3329) | `-1` | R |
| 161 | `crypto_kem_mlkem768_enc_deterministic` | `pk` not canonical | `-1` | R |
| 162 | `crypto_kem_mlkem768_dec` | never rejects (constant-time implicit rejection) | always `0` | R |
| 163 | `crypto_kem_xwing_enc` / `_enc_deterministic` | inner `mlkem768_enc_deterministic != 0` | `-1` | R |
| 164 | `crypto_kem_xwing_enc_deterministic` | `crypto_scalarmult_curve25519` on x25519 half fails | `-1` | R |
| 165 | `crypto_kem_xwing_dec` | inner `mlkem768_dec != 0` | `-1` | U |
| 166 | `crypto_kem_xwing_dec` | `crypto_scalarmult_curve25519 != 0` | `-1` | U |

## Group 4 — `crypto_pwhash/`, `crypto_generichash/`, `crypto_auth/`, `crypto_kdf/`, `crypto_hash/`, `crypto_xof/`, `crypto_core/{salsa,hsalsa20,hchacha20,keccak1600}`

Constants: argon2i `OPSLIMIT_MIN=3U`, argon2id `OPSLIMIT_MIN=1U` (asymmetric),
both `MEMLIMIT_MIN=8192U`, `BYTES_MIN=16U`, `BYTES_MAX=4294967295U`,
`STRBYTES=128`, `ARGON2_VERSION_NUMBER=19`;
scrypt `BYTES_MIN=16U`, `BYTES_MAX=0x1fffffffe0`, `STRBYTES=102` (strnlen
must equal 101), `SALTBYTES=32`;
blake2b `OUTBYTES=64`, `KEYBYTES=64` (max), `SALTBYTES=16`, `PERSONALBYTES=16`;
`crypto_kdf_blake2b_BYTES_MIN=16`, `BYTES_MAX=64`;
`hkdf_sha256_BYTES_MAX=0xff*32=8160`, `hkdf_sha512_BYTES_MAX=0xff*64=16320`.

| # | function | trigger (exact invalid input/condition) | expected C result | R/U |
|---|----------|------------------------------------------|-------------------|-----|
| 167 | `crypto_pwhash` | `alg` ∉ {1,2} (incl. 0, 3, −1, 999) | `errno=EINVAL`, `-1` | R |
| 168 | `crypto_pwhash_str_alg` | `alg` ∉ {1,2} | `sodium_misuse()` | R |
| 169 | `crypto_pwhash_str_verify` | `str` matches neither `$argon2id$` nor `$argon2i$` | `errno=EINVAL`, `-1` | R |
| 170 | `crypto_pwhash_str_needs_rehash` | `str` matches neither prefix | `errno=EINVAL`, `-1` | R |
| 171 | `crypto_pwhash_argon2i` / `argon2id` | `outlen > BYTES_MAX` | `errno=EFBIG`, `-1` | U (huge) |
| 172 | `crypto_pwhash_argon2i` / `argon2id` | `outlen < 16` | `errno=EINVAL`, `-1` | R |
| 173 | `crypto_pwhash_argon2i` / `argon2id` | `opslimit > OPSLIMIT_MAX` or `memlimit > MEMLIMIT_MAX` or `passwdlen > PASSWD_MAX` | `errno=EFBIG`, `-1` | R |
| 174 | `crypto_pwhash_argon2i` | `opslimit < 3` | `errno=EINVAL`, `-1` | R |
| 175 | `crypto_pwhash_argon2id` | `opslimit < 1` (i.e. 0) | `errno=EINVAL`, `-1` | R |
| 176 | `crypto_pwhash_argon2i` / `argon2id` | `memlimit < 8192` | `errno=EINVAL`, `-1` | R |
| 177 | `crypto_pwhash_argon2i` / `argon2id` | `out == passwd` (aliasing) | `errno=EINVAL`, `-1` | R |
| 178 | `crypto_pwhash_argon2i` | `alg != ALG_ARGON2I13` | `errno=EINVAL`, `-1` | R |
| 179 | `crypto_pwhash_argon2id` | `alg != ALG_ARGON2ID13` | `errno=EINVAL`, `-1` | R |
| 180 | `crypto_pwhash_argon2i_str` / `argon2id_str` | `opslimit`/`memlimit` below MIN | `errno=EINVAL`, `-1` | R |
| 181 | `crypto_pwhash_argon2i_str_verify` / `argon2id_str_verify` | hash mismatch (`ARGON2_VERIFY_MISMATCH` = −35) | `errno=EINVAL`, `-1` | R |
| 182 | `crypto_pwhash_*_str_verify` | malformed encoded string (`ARGON2_DECODING_FAIL` = −32) | `-1` | R |
| 183 | `crypto_pwhash_*_str_verify` | wrong argon2 type in string (`ARGON2_INCORRECT_TYPE` = −26) | `-1` | R |
| 184 | `crypto_pwhash_*_str_verify` | `version != 19` in string | `-1` | R |
| 185 | `crypto_pwhash_*_str_needs_rehash` | `strlen(str) >= 128` | `errno=EINVAL`, `-1` | R |
| 186 | `crypto_pwhash_*_str_needs_rehash` | `argon2_decode_string != 0` | `errno=EINVAL`, `-1` | R |
| 187 | `crypto_pwhash_*_str_needs_rehash` | `t_cost != opslimit` or `m_cost != memlimit/1024` | `1` | R |
| 188 | `argon2_validate_inputs` (via `argon2_hash`) | `saltlen < 8` (`ARGON2_SALT_TOO_SHORT` = −6) | `-1` at the sodium layer | R |
| 189 | `argon2_validate_inputs` | `m_cost < 8` (`ARGON2_MEMORY_TOO_LITTLE` = −14) | `-1` | R (via memlimit<8192 first) |
| 190 | `argon2_validate_inputs` | `t_cost < 1` (`ARGON2_TIME_TOO_SMALL` = −12) | `-1` | R |
| 191 | `argon2_validate_inputs` | `outlen < 16` (`ARGON2_OUTPUT_TOO_SHORT` = −2) | `-1` | R |
| 192 | `crypto_pwhash_scryptsalsa208sha256` | `outlen < 16` | `errno=EINVAL`, `-1` | R |
| 193 | `crypto_pwhash_scryptsalsa208sha256` | `outlen > 0x1fffffffe0` | `errno=EFBIG`, `-1` | U (huge) |
| 194 | `crypto_pwhash_scryptsalsa208sha256` | `pickparams` fails (opslimit/memlimit unusable) | `errno=EINVAL`, `-1` | R |
| 195 | `crypto_pwhash_scryptsalsa208sha256` | `out == passwd` | `errno=EINVAL`, `-1` | R |
| 196 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `strnlen(str,102) != 101` | `-1` | R |
| 197 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | hash mismatch | `-1` | R |
| 198 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `strnlen(str,102) != 101` | `errno=EINVAL`, `-1` | R |
| 199 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | setting does not start `$7$` / bad decode64 | `errno=EINVAL`, `-1` | R |
| 200 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | stored N/r/p differ from requested | `1` | R |
| 201 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N` not a power of 2, or `N < 2` | `errno=EINVAL`, `-1` | R |
| 202 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r == 0` or `p == 0` | `errno=EINVAL`, `-1` | R |
| 203 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r*p >= 2^30` | `errno=EFBIG`, `-1` | R |
| 204 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > UINT32_MAX` | `errno=EFBIG`, `-1` | R |
| 205 | `crypto_pwhash_scryptsalsa208sha256_ll` | `buflen > (2^32-1)*32` | `errno=EFBIG`, `-1` | U (huge) |
| 206 | `crypto_pwhash_scryptsalsa208sha256_ll` | allocation-size overflow | `errno=ENOMEM`, `-1` | R |
| 207 | `crypto_generichash` / `_blake2b` | `outlen == 0` | `-1` | R |
| 208 | `crypto_generichash` / `_blake2b` | `outlen > 64` | `-1` | R |
| 209 | `crypto_generichash` / `_blake2b` | `keylen > 64` | `-1` | R |
| 210 | `crypto_generichash_blake2b_salt_personal` | `outlen == 0` / `> 64` / `keylen > 64` | `-1` | R |
| 211 | `crypto_generichash_init` / `_blake2b_init` | `outlen == 0` / `> 64` / `keylen > 64` | `-1` | R |
| 212 | `crypto_generichash_blake2b_init_salt_personal` | same bounds | `-1` | R |
| 213 | `crypto_auth_hmacsha256_init` | `key == NULL && keylen > 0` | `sodium_misuse()` | R |
| 214 | `crypto_auth_hmacsha512_init` | `key == NULL && keylen > 0` | `sodium_misuse()` | R |
| 215 | `crypto_auth_hmacsha256_verify` | tag mismatch | `-1` | R |
| 216 | `crypto_auth_hmacsha512_verify` | tag mismatch | `-1` | R |
| 217 | `crypto_auth_hmacsha512256_verify` / `crypto_auth_verify` | tag mismatch | `-1` | R |
| 218 | `crypto_kdf_blake2b_derive_from_key` / `crypto_kdf_derive_from_key` | `subkey_len < 16` | `errno=EINVAL`, `-1` | R |
| 219 | `crypto_kdf_blake2b_derive_from_key` / `crypto_kdf_derive_from_key` | `subkey_len > 64` | `errno=EINVAL`, `-1` | R |
| 220 | `crypto_kdf_hkdf_sha256_expand` | `out_len > 8160` | `errno=EINVAL`, `-1` | R |
| 221 | `crypto_kdf_hkdf_sha512_expand` | `out_len > 16320` | `errno=EINVAL`, `-1` | R |
| 222 | `crypto_hash_sha3_256_update` / `sha3_512_update` | called after `final` (phase ≠ ABSORBING) | permute+reset, `-1` | R |
| 223 | `crypto_hash_sha3_256_final` / `sha3_512_final` | called twice (phase ≠ ABSORBING) | `-1` | R |
| 224 | `crypto_xof_shake128_update` / `shake256_update` | called after first `squeeze` | permute+reset, `-1` | R |
| 225 | `crypto_xof_turboshake128_update` / `turboshake256_update` | called after first `squeeze` | permute+reset, `-1` | R |

---

## Additional generic FFI-boundary boundaries (covered even though not table rows)

| trigger | applies to |
|---|---|
| out-of-range `int` enum passed across FFI (`alg` = 0/3/−1/999/`INT_MIN`/`INT_MAX`) | `crypto_pwhash`, `crypto_pwhash_str_alg` |
| out-of-range base64 `variant` (0,2,4,6,8,−1,`INT_MAX`) | all `sodium_*base64*` |
| out-of-range `hash_alg` | `crypto_core_ed25519_from_string`, `_scalar_from_string`, ristretto255 equivalents |
| out-of-range secretstream `tag` byte (4..255) | `crypto_secretstream_xchacha20poly1305_push` |
| `len == 0` on every length-taking function | streams, hashes, pad/unpad, compare/add/sub/increment/is_zero |
| `NULL` optional out-params (`clen_p`, `mlen_p`, `maclen_p`, `siglen_p`, `tag_p`, `bin_len`, `hex_end`, `b64_end`, `padded_buflen_p`, `unpadded_buflen_p`, `rx`, `tx`, `m`, `c`) | all functions documenting them as optional |
| one step past every documented range (`BYTES_MIN-1`, `BYTES_MAX+1`, `OPSLIMIT_MIN-1`, `MEMLIMIT_MIN-1`, `blocksize`/`N`/`r`/`p` boundaries) | pwhash, kdf, generichash, xof, pad |

---

## Coverage — which test covers which rows

All tests load BOTH `.so` files through `libloading` and call only exported
symbols. For every row the assertion is that C and Rust return the **same**
error code / sentinel **and** set the **same `errno`** (via the `cmp2!` macro),
and that every output buffer is byte-identical, including on the failure path.

For rows where the C `abort()`s (`sodium_misuse()`), or where the C dereferences
out of bounds *before* validating (see the notes below), the call is made in a
`fork()`ed child and the assertion is that both children terminate identically
(same signal / same exit status).

| rows | test |
|---|---|
| 1–5 (ENOSYS mlock/mprotect) | `c1_errors_core::rows1_5_mlock_mprotect_enosys` |
| 6 (allocarray overflow) | `c1_errors_core::row6_allocarray_overflow` |
| 10, 11, 13, 14, 15 (pad/unpad) | `c1_errors_core::rows10_11_13_15_pad_unpad_errors` |
| 12 (pad size overflow → abort) | `c4_misuse::row12_sodium_pad_size_overflow_aborts` |
| 17 (bin2hex too small → abort) | `c1_errors_core::row17_bin2hex_output_too_small_aborts` |
| 18–20 (hex2bin) | `c1_errors_core::rows18_20_hex2bin_errors` |
| 21 (bad base64 variant → abort) | `c1_errors_core::row21_base64_bad_variant_aborts` |
| 24 (bin2base64 too small → abort) | `c1_errors_core::row24_bin2base64_output_too_small_aborts` |
| 25–29 (base642bin) | `c1_errors_core::rows25_29_base642bin_errors` |
| 30–37 (ip2bin / bin2ip) | `c1_errors_core::rows30_37_ip_errors` |
| 41 (sodium_misuse + handler) | `c4_misuse::row41_sodium_misuse_and_handler` |
| 42 (randombytes oversize → abort) | `c4_misuse::row42_randombytes_buf_deterministic_oversize_aborts` |
| 44 (randombytes_close) | `c1_errors_core::row44_randombytes_close` |
| 45–49 (verify / memcmp / poly1305) | `c1_errors_core::rows45_49_verify_mismatches` |
| 50–61 (chacha/xchacha AEAD) | `c2_errors_crypto::aead_chacha20poly1305_errors`, `aead_chacha20poly1305_ietf_errors`, `aead_xchacha20poly1305_ietf_errors` |
| 62–71 (aegis AEAD) | `c2_errors_crypto::aead_aegis_errors` |
| 72–81 (aes256gcm ENOSYS) | `b2_aead_box::row72_aes256gcm_enosys_stubs` |
| 82–84, 88–90 (secretbox) | `c2_errors_crypto::secretbox_errors` |
| 85–87 (padded secretbox) | `c2_errors_crypto::secretbox_padded_lowlevel_errors` |
| 91–94 (secretstream) | `c2_errors_crypto::secretstream_errors` |
| 95–116 (box, all 3 primitives) | `c2_errors_crypto::box_errors`, `box_curve25519xsalsa20poly1305_errors`, `box_curve25519xchacha20poly1305_errors` |
| 118, 119 (chacha20_ietf length / ic) | `c2_errors_crypto::rows118_119_stream_errors` |
| 120, 121 (curve25519 scalarmult) | `c2_errors_crypto::rows120_121_scalarmult_curve25519_errors` |
| 122–133 (ed25519 / ristretto255 scalarmult) | `c2_errors_crypto::rows122_130_scalarmult_ed25519_ristretto_errors` |
| 134–145 (core ed25519 / ristretto255) | `c2_errors_crypto::rows134_145_core_errors` |
| 146–155 (sign, incl. ed25519ph) | `c2_errors_crypto::rows146_155_sign_errors` |
| 156–159 (kx) | `c2_errors_crypto::rows156_159_kx_errors` |
| 160–166 (kem mlkem768 / xwing) | `c2_errors_crypto::rows160_166_kem_errors` |
| 167, 171–179 (pwhash argon2) | `c1_errors_core::rows167_179_pwhash_argon2_errors` |
| 168 (str_alg bad alg → abort) | `c1_errors_core::row168_pwhash_str_alg_bad_alg_aborts` |
| 169, 170, 180–187 (pwhash strings) | `c1_errors_core::rows169_170_181_187_pwhash_str_errors` |
| 192–206 (scrypt) | `c1_errors_core::rows192_206_scrypt_errors` |
| 207–212 (generichash bounds) | `c1_errors_core::rows207_212_generichash_errors` |
| 213, 214 (hmac init NULL key → abort) | `c1_errors_core::rows213_214_hmac_init_null_key_aborts` |
| 215–217 (hmac verify) | `c1_errors_core::rows215_217_hmac_verify_mismatch` |
| 218–221 (kdf / hkdf bounds) | `c1_errors_core::rows218_221_kdf_errors` |
| 222, 223 (sha3 phase) | `c1_errors_core::rows222_223_sha3_phase_errors` |
| 224, 225 (xof phase) | `c1_errors_core::rows224_225_xof_phase_errors` |
| generic FFI boundaries | `c3_boundaries::*` (out-of-range enums, out-of-range secretstream tags 4..=255, one-step-past every documented range, zero lengths, NULL optional out-params) |

**Every reachable row above has a passing differential test. The Phase C gate is
satisfied.**

## Rows deliberately NOT tested, and why

| rows | reason |
|---|---|
| 7, 8, 9 | Guard-page / canary `sodium_malloc` paths. This build compiles the portable fallback (plain `malloc` + `memset 0xdb`), so these branches do not exist in the binary. |
| 16, 22, 23 | Require `bin_len >= SIZE_MAX/2` (or `> (SIZE_MAX-5)/4·3`), i.e. an input length no allocation can back. Not constructible. |
| 32, 38, 39, 40 | Guarded by `sodium_crit_enter/leave`, which are no-op stubs returning `0` in this build, so the `-1` branch is dead code. |
| 43 | Depends on `/dev/urandom` being unopenable. Not constructible without breaking the test host. |
| 117 | Requires `len > SIZE_MAX` for `crypto_stream_chacha20`, which is unrepresentable in `unsigned long long`. |
| 152 | `smlen - 64 > MESSAGEBYTES_MAX` is unreachable: `MESSAGEBYTES_MAX` is `SIZE_MAX - 64`, so no `smlen` satisfies it. |
| 165, 166 | Marked `LCOV_EXCL` in the C: `crypto_kem_mlkem768_dec` never fails and the x25519 half of a derived xwing secret key is never small-order. The surrounding code paths ARE exercised with corrupted and fully random ciphertexts and secret keys, and C and Rust agree on every derived shared secret. |
| 188–191 | `argon2_validate_inputs` error codes (`ARGON2_SALT_TOO_SHORT`, `ARGON2_MEMORY_TOO_LITTLE`, `ARGON2_TIME_TOO_SMALL`, `ARGON2_OUTPUT_TOO_SHORT`). These are **shadowed** by the stricter checks in the `crypto_pwhash_argon2*` wrappers (fixed 16-byte salt, `MEMLIMIT_MIN` 8192, `OPSLIMIT_MIN` 1/3, `BYTES_MIN` 16), so no public-API input reaches them. They are reachable only through `argon2_ctx`, which is not exported. |
| 193, 205 | `outlen`/`buflen` beyond `BYTES_MAX`. Covered, but only through a forked-child termination comparison — see the note below. |

## Rows where the C reads/writes out of bounds BEFORE validating

These are genuine C behaviours that must be replicated, so the differential
assertion is "both processes terminate the same way", checked via `fork()`:

- `crypto_pwhash_argon2i` / `_argon2id` / `crypto_pwhash` begin with
  `memset(out, 0, outlen)` **before** the `outlen > BYTES_MAX` check, so a huge
  `outlen` writes past the caller's buffer.
- `crypto_pwhash_scryptsalsa208sha256` does the same `memset(out, 0, outlen)`
  first, and its `PASSWD_MAX` is `SODIUM_SIZE_MAX`, so `passwdlen == u64::MAX`
  is *accepted* and the password is read out of bounds.
- `crypto_pwhash_scryptsalsa208sha256_str` inherits the same `passwdlen`
  behaviour.

## Functions whose C implementation requires a non-NULL pointer the header does not mark

Discovered while writing these tests; recorded so the tests do not assert on
undefined behaviour:

- `sodium_unpad` dereferences `unpadded_buflen_p` unconditionally — `NULL` is UB
  in the C, so it is not a testable configuration.
- `crypto_secretstream_xchacha20poly1305_pull` passes `m` straight to
  `crypto_stream_chacha20_ietf_xor_ic`, so `m == NULL` with `mlen > 0` is UB —
  unlike the AEAD `*_decrypt_detached` functions, which explicitly support
  `m == NULL` as a verify-only mode.
