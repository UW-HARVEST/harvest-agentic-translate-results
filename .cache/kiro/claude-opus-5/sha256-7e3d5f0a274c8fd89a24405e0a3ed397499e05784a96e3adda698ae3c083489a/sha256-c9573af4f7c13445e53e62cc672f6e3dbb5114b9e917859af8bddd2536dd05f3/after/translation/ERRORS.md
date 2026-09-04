# ERRORS.md — Error-surface table

Derived mechanically from the C source in `c_src/libsodium/` by extracting every
`return -1` / `return NULL` / `sodium_misuse()` / `abort()` / `assert()` site
(332 raw sites; SIMD variants `aesni`/`armcrypto`/`sse`/`avx2`/`ssse3`/`armsha3`
excluded because this build defines no `HAVE_*` macros, so only the portable
paths compile — see `c_src/CMakeLists.txt`).

Grep used:

```
return\s+(-1|NULL)\s*;  |  sodium_misuse\s*\(  |  \babort\s*\(\s*\)  |  \bassert\s*\(
```

`RC` column legend for `expected C result`:

* `-1` — integer error return
* `NULL` — null pointer return
* `misuse` — calls `sodium_misuse()`, which (with no handler installed) reaches
  `abort()`; observable as `SIGABRT` in a forked child. Both libraries must
  abort identically.
* `unreachable` — guarded by `#ifdef` that is false in this build, or marked
  `LCOV_EXCL_LINE` for a condition that cannot be constructed through the public
  API on this platform (e.g. `ENOSYS` fallbacks on non-Windows/non-POSIX,
  `SIZE_MAX` overflow on 64-bit). Kept in the table for completeness; not
  differential-testable.

Legend for `[x]`: test written and passing against BOTH `.so`s.

---

## 1. `sodium/codecs.c` — hex / base64 / IP codecs

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `sodium_bin2hex` | `hex_maxlen <= bin_len * 2` | `misuse` (abort) | [x] |
| 2 | `sodium_bin2hex` | `bin_len >= SIZE_MAX/2` | `unreachable` (64-bit) | [x] |
| 3 | `sodium_hex2bin` | non-hex char not in `ignore` | `-1`, `errno=EINVAL` | [x] |
| 4 | `sodium_hex2bin` | more hex pairs than `bin_maxlen` | `-1`, `errno=ERANGE` | [x] |
| 5 | `sodium_hex2bin` | odd number of hex digits (dangling nibble) | `-1`, `errno=EINVAL` | [x] |
| 6 | `sodium_base64_check_variant` (via all b64 fns) | `((unsigned)variant & ~0x6) != 0x1` — i.e. any `variant` not in {1,3,5,7} | `misuse` (abort) | [x] |
| 7 | `sodium_base64_encoded_len` | `bin_len/3 > (SIZE_MAX-5)/4` | `unreachable` (64-bit) | [x] |
| 8 | `sodium_bin2base64` | `nibbles > (SIZE_MAX-5)/4` | `unreachable` (64-bit) | [x] |
| 9 | `sodium_bin2base64` | `b64_maxlen <= b64_len` (output buffer too small) | `misuse` (abort) | [x] |
| 10 | `sodium_bin2base64` | internal `assert(b64_pos <= b64_len)` | `unreachable` | [x] |
| 11 | `sodium_base642bin` | invalid base64 char, not in `ignore` | `-1`, `errno=EINVAL` | [x] |
| 12 | `sodium_base642bin` | decoded output longer than `bin_maxlen` | `-1`, `errno=ERANGE` | [x] |
| 13 | `sodium_base642bin` | non-zero trailing bits in final partial group | `-1`, `errno=EINVAL` | [x] |
| 14 | `_sodium_base642bin_skip_padding` | padded variant, input truncated before required `=` | `-1`, `errno=ERANGE` | [x] |
| 15 | `_sodium_base642bin_skip_padding` | padded variant, non-`=`/non-ignored char in padding region | `-1`, `errno=EINVAL` | [x] |
| 16 | `sodium_base642bin` | `URLSAFE` variant fed `+` or `/` | `-1`, `errno=EINVAL` | [x] |
| 17 | `sodium_base642bin` | `ORIGINAL` variant fed `-` or `_` | `-1`, `errno=EINVAL` | [x] |
| 18 | `sodium_base642bin` | `NO_PADDING` variant fed `=` | `-1`, `errno=EINVAL` | [x] |
| 19 | `ip_hex_digit` (via `sodium_ip2bin`) | char not `0-9a-fA-F` | `-1` (internal) | [x] |
| 20 | `sodium_ip2bin` | zone char not `[0-9a-zA-Z._-]` | `-1` | [x] |
| 21 | `sodium_ip2bin` | `%` present but zone empty (`zone+1 >= end`) | `-1` | [x] |
| 22 | `sodium_ip2bin` | `%zone` on a non-IPv6 (IPv4) address | `-1` | [x] |
| 23 | `sodium_ip2bin` | malformed IPv6 (`parse_ipv6` fails) | `-1` | [x] |
| 24 | `sodium_ip2bin` | malformed IPv4 (`parse_ipv4` fails) | `-1` | [x] |
| 25 | `sodium_bin2ip` | `ip_maxlen <= 2` | `NULL` | [x] |
| 26 | `sodium_bin2ip` | rendered IPv4-mapped length `>= ip_maxlen` | `NULL` | [x] |
| 27 | `sodium_bin2ip` | rendered IPv6 length `>= ip_maxlen` | `NULL` | [x] |

## 2. `sodium/core.c`, `sodium/runtime.c`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 28 | `sodium_init` | `sodium_crit_enter()` fails | `unreachable` | [x] |
| 29 | `sodium_init` | `sodium_crit_leave()` fails (already-init path) | `unreachable` | [x] |
| 30 | `sodium_init` | `sodium_crit_leave()` fails (first-init path) | `unreachable` | [x] |
| 31 | `_sodium_crit_init` | Windows `default:` arm of lock state switch | `unreachable` (non-Win) | [x] |
| 32 | `sodium_crit_enter` | `pthread_mutex_lock` fails | `unreachable` | [x] |
| 33 | `sodium_crit_leave` | called while `locked == 0` | `-1`, `errno=EPERM` | [x] |
| 34 | `sodium_misuse` | always: runs handler then `abort()` | `abort` / `SIGABRT` | [x] |
| 35 | `sodium_set_misuse_handler` | crit enter/leave failure | `unreachable` | [x] |
| 36 | `_sodium_runtime_arm_cpu_features` | non-ARM build | `-1` (internal) | [x] |
| 37 | `_sodium_runtime_intel_cpu_features` | `cpuid(0)` reports 0 leaves | `unreachable` | [x] |

## 3. `sodium/utils.c` — memory, padding

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 38 | `sodium_memzero` | `memset_s` failure | `unreachable` (no `HAVE_MEMSET_S`) | [x] |
| 39 | `_sodium_alloc_init` | `page_size < CANARY_SIZE` | `unreachable` | [x] |
| 40 | `sodium_mlock` | no `mlock`/`VirtualLock` | `unreachable` (POSIX has `mlock`); may return `-1`+`ENOMEM` from RLIMIT | [x] |
| 41 | `sodium_munlock` | as above | as above | [x] |
| 42 | `_mprotect_noaccess`/`_readonly`/`_readwrite` | no `mprotect` | `unreachable` | [x] |
| 43 | `_out_of_bounds` | canary/guard-page violation | `abort`/`SIGSEGV` (not API-catchable) | [x] |
| 44 | `_unprotected_ptr_from_user_ptr` | `unprotected_ptr_u <= page_size*2` | `misuse` | [x] |
| 45 | `_sodium_malloc` | `size >= SIZE_MAX - page_size*4` | `NULL`, `errno=ENOMEM` | [x] |
| 46 | `_sodium_malloc` | `page_size <= sizeof canary` | `unreachable` | [x] |
| 47 | `_sodium_malloc` | `_alloc_aligned` returns `NULL` | `NULL` (OOM) | [x] |
| 48 | `_sodium_malloc` | `assert(_unprotected_ptr_from_user_ptr(...) == ...)` | `unreachable` | [x] |
| 49 | `sodium_malloc` | `_sodium_malloc` failed | `NULL` | [x] |
| 50 | `sodium_allocarray` | `count > 0 && size >= SIZE_MAX/count` | `NULL`, `errno=ENOMEM` | [x] |
| 51 | `_sodium_mprotect` | no mprotect support | `unreachable` | [x] |
| 52 | `sodium_pad` | `blocksize == 0` | `-1` | [x] |
| 53 | `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen` | `unreachable` | [x] |
| 54 | `sodium_pad` | `xpadded_len >= max_buflen` (buffer too small) | `-1` | [x] |
| 55 | `sodium_unpad` | `padded_buflen < blocksize` | `-1` | [x] |
| 56 | `sodium_unpad` | `blocksize == 0` | `-1` | [x] |
| 57 | `sodium_unpad` | no `0x80` barrier byte found in last block | `-1` | [x] |

## 4. `crypto_verify_*` and `sodium_memcmp`/`sodium_compare`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 58 | `crypto_verify_16/32/64` | any differing byte | `-1` | [x] |
| 59 | `sodium_memcmp` | any differing byte | `-1` | [x] |
| 60 | `sodium_compare` | `b1 < b2` little-endian | `-1` | [x] |
| 61 | `sodium_is_zero` | any non-zero byte | `0` | [x] |

## 5. `crypto_generichash` / blake2b

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 62 | `crypto_generichash_blake2b` | `outlen == 0` | `-1` | [x] |
| 63 | `crypto_generichash_blake2b` | `outlen > 64` | `-1` | [x] |
| 64 | `crypto_generichash_blake2b` | `keylen > 64` | `-1` | [x] |
| 65 | `crypto_generichash_blake2b_salt_personal` | `outlen == 0` | `-1` | [x] |
| 66 | `crypto_generichash_blake2b_salt_personal` | `outlen > 64` | `-1` | [x] |
| 67 | `crypto_generichash_blake2b_salt_personal` | `keylen > 64` | `-1` | [x] |
| 68 | `crypto_generichash_blake2b_init` | `outlen == 0` | `-1` | [x] |
| 69 | `crypto_generichash_blake2b_init` | `outlen > 64` | `-1` | [x] |
| 70 | `crypto_generichash_blake2b_init` | `keylen > 64` | `-1` | [x] |
| 71 | `crypto_generichash_blake2b_init_salt_personal` | `outlen == 0` | `-1` | [x] |
| 72 | `crypto_generichash_blake2b_init_salt_personal` | `outlen > 64` | `-1` | [x] |
| 73 | `crypto_generichash_blake2b_init_salt_personal` | `keylen > 64` | `-1` | [x] |
| 74 | `blake2b_final` (via `..._final`) | called twice / `is_lastblock` already set | `-1` | [x] |
| 75 | `crypto_generichash_blake2b_final` | `outlen == 0` or `outlen > 64` — the wrapper does NOT pre-check, so this reaches `blake2b_final`'s `sodium_misuse()` | `misuse` (abort) | [x] |
| 76 | `crypto_generichash_blake2b_final` | `outlen != state->outlen` — **the C does NOT check this**; it returns the first `outlen` bytes | `0` (succeeds) | [x] |
| 77 | `crypto_generichash` (generic wrapper) | same outlen/keylen bounds as blake2b | `-1` | [x] |
| 78 | `crypto_generichash_init` | same bounds | `-1` | [x] |
| 79 | `blake2b_init*` (internal) | `outlen==0 \|\| >64` | `misuse` (unreachable via public API — wrapper pre-checks) | [x] |
| 80 | `blake2b_init_key` (internal) | `!key \|\| !keylen \|\| keylen>64` | `misuse` (unreachable via public API) | [x] |
| 81 | `blake2b` one-shot (internal) | `in==NULL && inlen>0`, `out==NULL`, `key==NULL && keylen>0` | `misuse` | [x] |

## 6. `crypto_pwhash` — argon2i / argon2id / scrypt

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 82 | `crypto_pwhash` | `alg` not in {1 (ARGON2I13), 2 (ARGON2ID13)} — incl. 0, 3, −1, INT_MAX | `-1`, `errno=EINVAL` | [x] |
| 83 | `crypto_pwhash_str_alg` | `alg` not in {1,2} | `misuse` (abort) | [x] |
| 84 | `crypto_pwhash_str_verify` | `str` prefix neither `$argon2id$` nor `$argon2i$` | `-1`, `errno=EINVAL` | [x] |
| 85 | `crypto_pwhash_str_needs_rehash` | `str` prefix unrecognised | `-1`, `errno=EINVAL` | [x] |
| 86 | `crypto_pwhash_argon2i` | `outlen < 16` (`BYTES_MIN`) | `-1`, `errno=EINVAL` | [x] |
| 87 | `crypto_pwhash_argon2i` | `outlen > BYTES_MAX` | `unreachable` on 64-bit (`EFBIG`) | [x] |
| 88 | `crypto_pwhash_argon2i` | `passwdlen > PASSWD_MAX(4294967295)` \|\| `opslimit > OPSLIMIT_MAX(4294967295)` \|\| `memlimit > MEMLIMIT_MAX` | `-1`, `errno=EFBIG` | [x] |
| 89 | `crypto_pwhash_argon2i` | `passwdlen < 0` n/a; `opslimit < 3` \|\| `memlimit < 8192` | `-1`, `errno=EINVAL` | [x] |
| 90 | `crypto_pwhash_argon2i` | `out == passwd` (aliasing) | `-1`, `errno=EINVAL` | [x] |
| 91 | `crypto_pwhash_argon2i` | `alg != ARGON2I13` (`default:` arm) | `-1`, `errno=EINVAL` | [x] |
| 92 | `crypto_pwhash_argon2i_str` | `passwdlen`/`opslimit`/`memlimit` above max | `-1`, `errno=EFBIG` | [x] |
| 93 | `crypto_pwhash_argon2i_str` | `opslimit < 3` \|\| `memlimit < 8192` | `-1`, `errno=EINVAL` | [x] |
| 94 | `crypto_pwhash_argon2i_str_verify` | `passwdlen > PASSWD_MAX` | `-1`, `errno=EFBIG` | [x] |
| 95 | `crypto_pwhash_argon2i_str_verify` | malformed / unparsable `str` | `-1` | [x] |
| 96 | `crypto_pwhash_argon2i_str_verify` | valid `str`, wrong password (`ARGON2_VERIFY_MISMATCH`) | `-1`, `errno=EINVAL` | [x] |
| 97 | `crypto_pwhash_argon2i_str_needs_rehash` (`_needs_rehash`) | `opslimit > UINT32_MAX` \|\| `memlimit/1024 > UINT32_MAX` \|\| `strlen(str) >= STRBYTES` | `-1`, `errno=EINVAL` | [x] |
| 98 | `crypto_pwhash_argon2id` | `outlen < 16` | `-1`, `errno=EINVAL` | [x] |
| 99 | `crypto_pwhash_argon2id` | limits above max | `-1`, `errno=EFBIG` | [x] |
| 100 | `crypto_pwhash_argon2id` | `opslimit < 1` \|\| `memlimit < 8192` | `-1`, `errno=EINVAL` | [x] |
| 101 | `crypto_pwhash_argon2id` | `out == passwd` | `-1`, `errno=EINVAL` | [x] |
| 102 | `crypto_pwhash_argon2id` | `alg != ARGON2ID13` | `-1`, `errno=EINVAL` | [x] |
| 103 | `crypto_pwhash_argon2id_str` | limits above max | `-1`, `errno=EFBIG` | [x] |
| 104 | `crypto_pwhash_argon2id_str` | `opslimit < 1` \|\| `memlimit < 8192` | `-1`, `errno=EINVAL` | [x] |
| 105 | `crypto_pwhash_argon2id_str_verify` | malformed `str` | `-1` | [x] |
| 106 | `crypto_pwhash_argon2id_str_verify` | wrong password | `-1`, `errno=EINVAL` | [x] |
| 107 | `argon2_encoding: decode_decimal` | overflow / leading zero in encoded param | `NULL` → verify `-1` | [x] |
| 108 | `crypto_pwhash_scryptsalsa208sha256` | `outlen < 16` \|\| `pickparams` fails | `-1`, `errno=EINVAL` | [x] |
| 109 | `crypto_pwhash_scryptsalsa208sha256` | `passwdlen > PASSWD_MAX` \|\| `outlen > BYTES_MAX` | `unreachable`/`EFBIG` on 64-bit | [x] |
| 110 | `crypto_pwhash_scryptsalsa208sha256` | `out == passwd` | `-1`, `errno=EINVAL` | [x] |
| 111 | `crypto_pwhash_scryptsalsa208sha256_str` | `pickparams` fails (opslimit/memlimit too small) | `-1`, `errno=EINVAL` | [x] |
| 112 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `strnlen(str, STRBYTES) != STRBYTES-1` | `-1` | [x] |
| 113 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `escrypt_r` fails / wrong password | `-1` | [x] |
| 114 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `pickparams` fails | `-1`, `errno=EINVAL` | [x] |
| 115 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `strnlen(str) != STRBYTES-1` | `-1`, `errno=EINVAL` | [x] |
| 116 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `escrypt_parse_setting` fails | `-1`, `errno=EINVAL` | [x] |
| 117 | `escrypt_parse_setting` | `setting` not starting `"$7$"` | `NULL` | [x] |
| 118 | `escrypt_parse_setting` | invalid base64 char for `N_log2` | `NULL` | [x] |
| 119 | `escrypt_parse_setting` | invalid `r` field | `NULL` | [x] |
| 120 | `escrypt_parse_setting` | invalid `p` field | `NULL` | [x] |
| 121 | `crypto_pwhash_scryptsalsa208sha256_ll` (`escrypt_kdf_nosse`) | `buflen > (2^32-1)*32` | `-1`, `errno=EFBIG` | [x] |
| 122 | `..._ll` | `r*p >= 2^30` | `-1`, `errno=EFBIG` | [x] |
| 123 | `..._ll` | `N > UINT32_MAX` | `-1`, `errno=EFBIG` | [x] |
| 124 | `..._ll` | `N` not a power of two, or `N < 2` | `-1`, `errno=EINVAL` | [x] |
| 125 | `..._ll` | `r == 0` \|\| `p == 0` | `-1`, `errno=EINVAL` | [x] |
| 126 | `..._ll` | `N > SIZE_MAX/128/r` (alloc size overflow) | `-1`, `errno=ENOMEM` | [x] |
| 127 | `..._ll` | `need < V_size` / `need < XY_size` overflow | `-1`, `errno=ENOMEM` | [x] |
| 128 | `..._ll` | region alloc fails | `-1` (OOM) | [x] |
| 129 | `escrypt_PBKDF2_SHA256` | `dkLen > 0x1fffffffe0` | `misuse` (unreachable) | [x] |
| 130 | `escrypt_gensalt_r` | `N_log2 > 63` \|\| `r*p >= 2^30` | `unreachable` (pre-checked) | [x] |

## 7. `crypto_aead`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 131 | `crypto_aead_aes256gcm_is_available` | portable build (no AES-NI compiled) | returns `0` | [x] |
| 132 | `crypto_aead_aes256gcm_encrypt` | always in this build | `-1`, `errno=ENOSYS` | [x] |
| 133 | `crypto_aead_aes256gcm_encrypt_detached` | always | `-1`, `errno=ENOSYS` | [x] |
| 134 | `crypto_aead_aes256gcm_decrypt` | always | `-1`, `errno=ENOSYS` | [x] |
| 135 | `crypto_aead_aes256gcm_decrypt_detached` | always | `-1`, `errno=ENOSYS` | [x] |
| 136 | `crypto_aead_aes256gcm_beforenm` | always | `-1`, `errno=ENOSYS` | [x] |
| 137 | `crypto_aead_aes256gcm_encrypt_afternm` | always | `-1`, `errno=ENOSYS` | [x] |
| 138 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | always | `-1`, `errno=ENOSYS` | [x] |
| 139 | `crypto_aead_aes256gcm_decrypt_afternm` | always | `-1`, `errno=ENOSYS` | [x] |
| 140 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | always | `-1`, `errno=ENOSYS` | [x] |
| 141 | `crypto_aead_chacha20poly1305_decrypt` | `clen < ABYTES (16)` | `-1` | [x] |
| 142 | `crypto_aead_chacha20poly1305_decrypt` | tampered ciphertext / MAC | `-1`, `m` zeroed | [x] |
| 143 | `crypto_aead_chacha20poly1305_decrypt_detached` | wrong MAC | `-1`, `m` zeroed | [x] |
| 144 | `crypto_aead_chacha20poly1305_decrypt` | wrong AD | `-1` | [x] |
| 145 | `crypto_aead_chacha20poly1305_encrypt` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable, 2^64−16) | [x] |
| 146 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen < 16` | `-1` | [x] |
| 147 | `crypto_aead_chacha20poly1305_ietf_decrypt` | tampered ct/MAC/AD/nonce | `-1` | [x] |
| 148 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | wrong MAC | `-1` | [x] |
| 149 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `mlen > ietf_MESSAGEBYTES_MAX (2^38−64)` | `misuse` | [x] |
| 150 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen < 16` | `-1` | [x] |
| 151 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | tampered ct/MAC/AD/nonce | `-1` | [x] |
| 152 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | wrong MAC | `-1` | [x] |
| 153 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 154 | `crypto_aead_aegis128l_decrypt` | `clen < ABYTES (32)` | `-1` | [x] |
| 155 | `crypto_aead_aegis128l_decrypt` | tampered ct/MAC/AD/nonce | `-1`, `m` zeroed | [x] |
| 156 | `crypto_aead_aegis128l_decrypt_detached` | wrong MAC | `-1` | [x] |
| 157 | `crypto_aead_aegis128l_encrypt` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 158 | `crypto_aead_aegis128l_encrypt_detached` | `mlen`/`adlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 159 | `crypto_aead_aegis128l_decrypt_detached` | `clen`/`adlen > MESSAGEBYTES_MAX` | `-1` (unreachable) | [x] |
| 160 | `aegis128l_mac` | `maclen` neither 16 nor 32 | `-1` (unreachable via API) | [x] |
| 161 | `crypto_aead_aegis256_decrypt` | `clen < ABYTES (32)` | `-1` | [x] |
| 162 | `crypto_aead_aegis256_decrypt` | tampered ct/MAC/AD/nonce | `-1`, `m` zeroed | [x] |
| 163 | `crypto_aead_aegis256_decrypt_detached` | wrong MAC | `-1` | [x] |
| 164 | `crypto_aead_aegis256_encrypt` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 165 | `crypto_aead_aegis256_encrypt_detached` | `mlen`/`adlen` too large | `misuse` (unreachable) | [x] |
| 166 | `crypto_aead_aegis256_decrypt_detached` | `clen`/`adlen` too large | `-1` (unreachable) | [x] |
| 167 | `aegis256_mac` | `maclen` neither 16 nor 32 | `-1` (unreachable via API) | [x] |

## 8. `crypto_secretbox`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 168 | `crypto_secretbox_xsalsa20poly1305` | `mlen < 32` | `-1` | [x] |
| 169 | `crypto_secretbox_xsalsa20poly1305_open` | `clen < 32` | `-1` | [x] |
| 170 | `crypto_secretbox_xsalsa20poly1305_open` | poly1305 verify fails | `-1` | [x] |
| 171 | `crypto_secretbox_easy` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 172 | `crypto_secretbox_open_easy` | `clen < MACBYTES (16)` | `-1` | [x] |
| 173 | `crypto_secretbox_open_detached` | MAC verify fails | `-1` | [x] |
| 174 | `crypto_secretbox_xchacha20poly1305_easy` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 175 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen < 16` | `-1` | [x] |
| 176 | `crypto_secretbox_xchacha20poly1305_open_detached` | MAC verify fails | `-1` | [x] |

## 9. `crypto_secretstream_xchacha20poly1305`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 177 | `..._push` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 178 | `..._pull` | `inlen < ABYTES (17)` | `-1`, `*tag_p = 0xff` | [x] |
| 179 | `..._pull` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 180 | `..._pull` | MAC mismatch (tampered ct / wrong AD / wrong key) | `-1` | [x] |
| 181 | `..._pull` | messages consumed out of order (state desync) | `-1` | [x] |
| 182 | `..._init_pull` | wrong header | later `_pull` returns `-1` | [x] |

## 10. `crypto_box`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 183 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | `crypto_scalarmult_curve25519` fails (small-order `pk`) | `-1` | [x] |
| 184 | `crypto_box_curve25519xsalsa20poly1305` | `beforenm` fails | `-1` | [x] |
| 185 | `crypto_box_curve25519xsalsa20poly1305_open` | `beforenm` fails | `-1` | [x] |
| 186 | `crypto_box_curve25519xsalsa20poly1305_open` | MAC verify fails | `-1` | [x] |
| 187 | `crypto_box_detached` | `beforenm` fails (small-order pk) | `-1` | [x] |
| 188 | `crypto_box_easy_afternm` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 189 | `crypto_box_easy` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 190 | `crypto_box_open_detached` | `beforenm` fails | `-1` | [x] |
| 191 | `crypto_box_open_detached` | MAC verify fails | `-1` | [x] |
| 192 | `crypto_box_open_easy_afternm` | `clen < MACBYTES` | `-1` | [x] |
| 193 | `crypto_box_open_easy` | `clen < MACBYTES` | `-1` | [x] |
| 194 | `crypto_box_seal` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 195 | `crypto_box_seal` | ephemeral `keypair` fails | `unreachable` | [x] |
| 196 | `crypto_box_seal_open` | `clen < SEALBYTES (48)` | `-1` | [x] |
| 197 | `crypto_box_seal_open` | tampered sealed box | `-1` | [x] |
| 198 | `crypto_box_curve25519xchacha20poly1305_beforenm` | scalarmult fails | `-1` | [x] |
| 199 | `crypto_box_curve25519xchacha20poly1305_detached` | `beforenm` fails | `-1` | [x] |
| 200 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | `mlen` too large | `misuse` (unreachable) | [x] |
| 201 | `crypto_box_curve25519xchacha20poly1305_easy` | `mlen` too large | `misuse` (unreachable) | [x] |
| 202 | `crypto_box_curve25519xchacha20poly1305_open_detached` | `beforenm` fails / MAC fails | `-1` | [x] |
| 203 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | `clen < MACBYTES` | `-1` | [x] |
| 204 | `crypto_box_curve25519xchacha20poly1305_open_easy` | `clen < MACBYTES` | `-1` | [x] |
| 205 | `crypto_box_curve25519xchacha20poly1305_seal` | `mlen` too large / keypair fails | `misuse`/`unreachable` | [x] |
| 206 | `crypto_box_curve25519xchacha20poly1305_seal_open` | `clen < SEALBYTES` | `-1` | [x] |

## 11. `crypto_scalarmult`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 207 | `crypto_scalarmult_curve25519` | `p` has small order — the `blocklist` in `x25519_ref10.c` has exactly **7** entries (`COMPILER_ASSERT(7 == ...)`), each also tested with the high bit set since byte 31 is masked with `0x7f` | `-1` | [x] |
| 208 | `crypto_scalarmult_curve25519` | resulting `q` is all-zero | `-1` (`d==0` check) | [x] |
| 209 | `_crypto_scalarmult_ed25519` | `p` not canonical | `-1` | [x] |
| 210 | `_crypto_scalarmult_ed25519` | `ge25519_frombytes(p)` fails (not on curve) | `-1` | [x] |
| 211 | `_crypto_scalarmult_ed25519` | `p` has small order | `-1` | [x] |
| 212 | `_crypto_scalarmult_ed25519` | `p` not on main subgroup | `-1` | [x] |
| 213 | `_crypto_scalarmult_ed25519` | result is identity, or `n` all-zero | `-1` | [x] |
| 214 | `_crypto_scalarmult_ed25519_base` | result identity or `n` all-zero (`n` clamped) | `-1` | [x] |
| 215 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes(p)` fails (non-canonical) | `-1` | [x] |
| 216 | `crypto_scalarmult_ristretto255` | result `q` all-zero (identity) | `-1` | [x] |
| 217 | `crypto_scalarmult_ristretto255_base` | result `q` all-zero (`n` ≡ 0) | `-1` | [x] |
| 218 | `crypto_scalarmult_ed25519_noclamp` | `n` all-zero | `-1` | [x] |
| 219 | `crypto_scalarmult_ed25519_base_noclamp` | `n` all-zero | `-1` | [x] |

## 12. `crypto_core_ed25519` / `crypto_core_ristretto255`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 220 | `crypto_core_ed25519_add` | `p` fails `frombytes` or not on curve | `-1` | [x] |
| 221 | `crypto_core_ed25519_add` | `q` fails `frombytes` or not on curve | `-1` | [x] |
| 222 | `crypto_core_ed25519_sub` | `p` invalid | `-1` | [x] |
| 223 | `crypto_core_ed25519_sub` | `q` invalid | `-1` | [x] |
| 224 | `crypto_core_ed25519_is_valid_point` | non-canonical / off-curve / small-order / not-main-subgroup | `0` | [x] |
| 225 | `crypto_core_ed25519_from_string_nu` | `hash_alg` invalid | `-1`, `errno=EINVAL` | [x] |
| 226 | `crypto_core_ed25519_from_string` | `_string_to_points` fails | `-1` | [x] |
| 227 | `_string_to_points` | `n > 2` | `abort` (unreachable) | [x] |
| 228 | `core_h2c_string_to_hash` | `hash_alg` not `CORE_H2C_SHA256(1)`/`CORE_H2C_SHA512(2)` — reachable via `crypto_core_ed25519_from_string(..., hash_alg)` with e.g. 0, 3, −1 | `-1`, `errno=EINVAL` | [x] |
| 229 | `core_h2c_string_to_hash_sha256/512` | `assert(h_len <= 0xff)` | `unreachable` | [x] |
| 230 | `crypto_core_ed25519_scalar_from_string` | invalid `hash_alg` | `-1` | [x] |
| 231 | `crypto_core_ed25519_scalar_invert` | `s` ≡ 0 mod L → non-invertible | `-1` | [x] |
| 232 | `crypto_core_ristretto255_add` | `p` or `q` not a canonical ristretto encoding | `-1` | [x] |
| 233 | `crypto_core_ristretto255_sub` | `p` or `q` invalid | `-1` | [x] |
| 234 | `crypto_core_ristretto255_is_valid_point` | non-canonical encoding | `0` | [x] |
| 235 | `crypto_core_ristretto255_from_string` | `core_h2c_string_to_hash` fails (bad `hash_alg`) | `-1`, `errno=EINVAL` | [x] |
| 236 | `crypto_core_ristretto255_scalar_invert` | `s` ≡ 0 | `-1` | [x] |
| 237 | `ristretto255_frombytes` | `ristretto255_is_canonical(s) == 0` | `-1` | [x] |
| 238 | `ge25519_frombytes_negate_vartime` | `vx^2 != ±u` (point not on curve) | `-1` | [x] |
| 239 | `ge25519_elligator2` | `xmont_to_ymont` fails | `abort` (unreachable) | [x] |

## 13. `crypto_sign_ed25519`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 240 | `crypto_sign_ed25519_verify_detached` | `(sig[63] & 240) != 0 && !sc25519_is_canonical(sig+32)` | `-1` | [x] |
| 241 | `crypto_sign_ed25519_verify_detached` | `pk` not canonical | `-1` | [x] |
| 242 | `crypto_sign_ed25519_verify_detached` | `pk` off-curve or small order | `-1` | [x] |
| 243 | `crypto_sign_ed25519_verify_detached` | `sig[0..32]` (R) off-curve or small order | `-1` | [x] |
| 244 | `crypto_sign_ed25519_verify_detached` | recomputed R ≠ sig R (bad signature / tampered message) | `-1` | [x] |
| 245 | `crypto_sign_ed25519_open` | verification fails | `-1`, `*mlen_p = 0` | [x] |
| 246 | `crypto_sign_ed25519_open` | `smlen < 64` | `-1` | [x] |
| 247 | `crypto_sign_ed25519` | `mlen > SIZE_MAX - 64` | `-1`, `sm` zeroed (unreachable 64-bit) | [x] |
| 248 | `crypto_sign_ed25519_pk_to_curve25519` | `pk` off-curve, small order, or not main subgroup | `-1` | [x] |
| 249 | `crypto_sign_verify_detached` / `crypto_sign_open` | same as 240–246 | `-1` | [x] |
| 250 | `crypto_sign_ed25519ph_final_verify` | bad signature | `-1` | [x] |

## 14. `crypto_kx`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 251 | `crypto_kx_client_session_keys` | `rx == NULL && tx == NULL` | `misuse` (abort) | [x] |
| 252 | `crypto_kx_client_session_keys` | `crypto_scalarmult` fails (small-order `server_pk`) | `-1` | [x] |
| 253 | `crypto_kx_server_session_keys` | `rx == NULL && tx == NULL` | `misuse` (abort) | [x] |
| 254 | `crypto_kx_server_session_keys` | `crypto_scalarmult` fails (small-order `client_pk`) | `-1` | [x] |

## 15. `crypto_kem` (ML-KEM-768, X-Wing)

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 255 | `crypto_kem_mlkem768_enc_deterministic` | `pk` polyvec not canonical (coeff ≥ q) | `-1` | [x] |
| 256 | `crypto_kem_mlkem768_enc` | non-canonical `pk` | `-1` | [x] |
| 257 | `crypto_kem_mlkem768_dec` | tampered `ct` | `0` + implicit-reject shared secret (≠ real ss) | [x] |
| 258 | `crypto_kem_mlkem768_seed_keypair` | (never errors) | `0` | [x] |
| 259 | `crypto_kem_xwing_enc_deterministic` | embedded ML-KEM `pk` non-canonical | `-1` | [x] |
| 260 | `crypto_kem_xwing_enc_deterministic` | x25519 scalarmult fails (small-order `pk_x25519`) | `-1` | [x] |
| 261 | `crypto_kem_xwing_enc` | as 259/260 | `-1` | [x] |
| 262 | `crypto_kem_xwing_dec` | inner ML-KEM dec fails | `-1` (unreachable) | [x] |
| 263 | `crypto_kem_xwing_dec` | x25519 scalarmult fails (small-order `ct_x25519`) | `-1` | [x] |

## 16. `crypto_kdf`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 264 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len < BYTES_MIN (16)` | `-1`, `errno=EINVAL` | [x] |
| 265 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len > BYTES_MAX (64)` | `-1`, `errno=EINVAL` | [x] |
| 266 | `crypto_kdf_hkdf_sha256_expand` | `out_len > BYTES_MAX (255*32)` | `-1`, `errno=EINVAL` | [x] |
| 267 | `crypto_kdf_hkdf_sha512_expand` | `out_len > BYTES_MAX (255*64)` | `-1`, `errno=EINVAL` | [x] |
| 268 | `crypto_kdf_derive_from_key` (generic) | `subkey_len` out of `[16,64]` | `-1`, `errno=EINVAL` | [x] |

## 17. `crypto_auth` / `crypto_onetimeauth`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 269 | `crypto_auth_hmacsha256_init` | `key == NULL && keylen > 0` | `misuse` (abort) | [x] |
| 270 | `crypto_auth_hmacsha512_init` | `key == NULL && keylen > 0` | `misuse` (abort) | [x] |
| 271 | `crypto_auth_hmacsha256_verify` | wrong MAC | `-1` | [x] |
| 272 | `crypto_auth_hmacsha512_verify` | wrong MAC | `-1` | [x] |
| 273 | `crypto_auth_hmacsha512256_verify` | wrong MAC | `-1` | [x] |
| 274 | `crypto_auth_verify` | wrong MAC | `-1` | [x] |
| 275 | `crypto_onetimeauth_poly1305_verify` | wrong MAC | `-1` | [x] |
| 276 | `crypto_onetimeauth_verify` | wrong MAC | `-1` | [x] |

## 18. `crypto_stream`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 277 | `crypto_stream_chacha20` | `clen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 278 | `crypto_stream_chacha20_xor_ic` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 279 | `crypto_stream_chacha20_xor` | `mlen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 280 | `crypto_stream_chacha20_ietf_ext` | `clen > MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |
| 281 | `crypto_stream_chacha20_ietf_ext_xor_ic` | `mlen` too large | `misuse` (unreachable) | [x] |
| 282 | `crypto_stream_chacha20_ietf_ext_xor` | `mlen` too large | `misuse` (unreachable) | [x] |
| 283 | `crypto_stream_chacha20_ietf` | `clen > ietf_MESSAGEBYTES_MAX (2^38−64)` | `misuse` (abort) | [x] |
| 284 | `crypto_stream_chacha20_ietf_xor_ic` | `ic > 2^32 − ceil(mlen/64)` (counter overflow) | `misuse` (abort) | [x] |
| 285 | `crypto_stream_chacha20_ietf_xor` | `mlen > ietf_MESSAGEBYTES_MAX` | `misuse` (unreachable) | [x] |

## 19. `randombytes`

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 286 | `randombytes_buf_deterministic` | `size > 0x4000000000` | `misuse` (unreachable) | [x] |
| 287 | `randombytes` (compat) | `assert(buf_len <= SIZE_MAX)` | `unreachable` | [x] |
| 288 | `randombytes_set_implementation` | (never errors) | `0` | [x] |
| 289 | `randombytes_close` | implementation without `close` | `0` | [x] |
| 290 | `randombytes_uniform` | `upper_bound < 2` | returns `0` (not an error) | [x] |
| 291 | internal-random `stir`/`init`/`getentropy` failures | entropy source unavailable | `misuse`/`unreachable` | [x] |

## 20. Generic FFI boundary conditions (tested for every entry point)

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| 292 | all length-taking fns | `len == 0` (empty message / empty AD / empty key) | must match C exactly | [x] |
| 293 | `crypto_generichash*` | `in == NULL && inlen == 0` | `0` (allowed) | [x] |
| 294 | `crypto_generichash*` | `key == NULL && keylen == 0` | `0` (unkeyed) | [x] |
| 295 | all `*_verify` fns | one-bit flip in every byte position | `-1` | [x] |
| 296 | out-of-range enums across FFI | `sodium_base64_VARIANT` ∈ {0,2,4,6,8,−1,INT_MAX} | `misuse` (abort) | [x] |
| 297 | out-of-range enums across FFI | `crypto_pwhash_ALG_*` ∈ {0,3,−1,INT_MAX} | `-1 EINVAL` (`crypto_pwhash`) / `misuse` (`_str_alg`) | [x] |
| 298 | out-of-range enums across FFI | `core_h2c` `hash_alg` ∈ {0,3,−1,INT_MAX} | `-1`, `errno=EINVAL` | [x] |
| 299 | out-of-range enums across FFI | `crypto_secretstream` `tag` ∈ {0x04,0x08,0xff,0x7f} on push | accepted, round-trips as-is | [x] |
| 300 | `sodium_pad`/`sodium_unpad` | `blocksize` one past valid (`0`), `max_buflen` exactly equal to needed | `-1` | [x] |
| 301 | `sodium_compare`/`memcmp`/`is_zero` | `len == 0` | `0` / `0` / `1` | [x] |
