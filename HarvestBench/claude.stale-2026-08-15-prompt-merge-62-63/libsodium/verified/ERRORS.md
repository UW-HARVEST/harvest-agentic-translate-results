# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Mechanically derived from the C source under `c_src/libsodium/` by grepping every
`return -1`, `return NULL`, `sodium_misuse()`, `abort()`, `assert()`, `errno =`,
`ARGON2_*` error enum, explicit range check, null check and min/max constant.

One row per DISTINCT rejection branch. `[x]` = a Phase C differential test exists
and passes against BOTH the C `.so` and the Rust `.so`.

Legend for "expected C result":
* `-1` / `0` / `NULL` — the returned value
* `misuse` — `sodium_misuse()`: runs the handler set by
  `sodium_set_misuse_handler()` then `abort()`s. Tested by installing a handler
  that `longjmp`s out, so the *reachability* of the branch is asserted.
* `errno=E*` — errno is also set

---

## A. `sodium/codecs.c` — hex

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `sodium_bin2hex` | `bin_len >= SIZE_MAX / 2` | misuse | [ ] |
| 2 | `sodium_bin2hex` | `hex_maxlen <= bin_len * 2` (needs `2*bin_len+1`) | misuse | [ ] |
| 3 | `sodium_hex2bin` | odd number of hex digits consumed (`state != 0` at end), e.g. `"abc"` | -1, `errno=EINVAL`, `*bin_len=0`, `*hex_end=&hex[hex_pos-1]` | [ ] |
| 4 | `sodium_hex2bin` | non-hex char, `ignore==NULL`, `hex_end==NULL` | -1, `errno=EINVAL`, `*bin_len` = decoded prefix (NOT reset to 0) | [ ] |
| 5 | `sodium_hex2bin` | non-hex char, `ignore==NULL`, `hex_end!=NULL` | **0** (not an error), `*bin_len`=prefix, `*hex_end` at bad char | [ ] |
| 6 | `sodium_hex2bin` | char in `ignore` mid-byte (`state==1`), e.g. `ignore=":"`, `"a:bcd"` | -1, `errno=EINVAL`, `*bin_len=0` | [ ] |
| 7 | `sodium_hex2bin` | `bin_maxlen` exhausted, `hex_end!=NULL` | -1, `errno=ERANGE`, `*bin_len=0`, `*hex_end` at first unconsumed char | [ ] |
| 8 | `sodium_hex2bin` | `bin_maxlen` exhausted, `hex_end==NULL` | -1, `errno=EINVAL` (ERANGE overwritten), `*bin_len=0` | [ ] |
| 9 | `sodium_hex2bin` | `bin_maxlen==0` with ≥1 valid hex digit | -1, ERANGE if `hex_end!=NULL` else EINVAL | [ ] |
| 10 | `sodium_hex2bin` | embedded NUL in `hex` with `ignore!=NULL` (`strchr(ignore,0)` matches) → NUL is SKIPPED | 0, NUL treated as ignorable (quirk) | [ ] |

## B. `sodium/codecs.c` — base64

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 11 | `sodium_base64_check_variant` (via `_encoded_len`/`bin2base64`/`base642bin`) | `((unsigned)variant & ~0x6) != 0x1`, i.e. variant ∉ {1,3,5,7} — tested 0,2,4,6,8,9,99,0xFFFFFFFF | misuse | [ ] |
| 12 | `sodium_base64_encoded_len` | `bin_len/3 > (SIZE_MAX-5)/4` | misuse | [ ] |
| 13 | `sodium_bin2base64` | `bin_len/3 > (SIZE_MAX-5)/4` | misuse | [ ] |
| 14 | `sodium_bin2base64` | `b64_maxlen <= required b64_len` | misuse | [ ] |
| 15 | `sodium_base642bin` | invalid b64 char not in `ignore`, `b64_end==NULL`, not at `b64_len` | -1, `errno=EINVAL`, `*bin_len` may be NONZERO (partial decode kept) | [ ] |
| 16 | `sodium_base642bin` | invalid char where padding expected (padded variant, `skip_padding` sees non-`=`) | -1, `errno=EINVAL`, `*bin_len=0` | [ ] |
| 17 | `sodium_base642bin` | invalid char, `b64_end!=NULL`, no padding obligation | **0**, `*b64_end` at bad char, `*bin_len`=prefix | [ ] |
| 18 | `sodium_base642bin` | one leftover b64 char (`consumed % 4 == 1` ⇒ `acc_len==6`), e.g. `"A"` | -1, **errno UNCHANGED** (no assignment on this branch), `*bin_len=0` | [ ] |
| 19 | `sodium_base642bin` | nonzero trailing bits, e.g. `"AC"` | -1, **errno UNCHANGED**, `*bin_len=0` | [ ] |
| 20 | `sodium_base642bin` | `bin_maxlen` exceeded, `b64_end!=NULL` | -1, `errno=ERANGE`, `*bin_len=0` | [ ] |
| 21 | `sodium_base642bin` | `bin_maxlen` exceeded, `b64_end==NULL` | -1, `errno=EINVAL` (ERANGE overwritten), `*bin_len=0` | [ ] |
| 22 | `sodium_base642bin` | `bin_maxlen==0` with decodable data | -1, ERANGE or EINVAL as above | [ ] |
| 23 | `sodium_base642bin` | PADDED variant (1,5), padding absent/truncated: `"/w"`, `"/wE"` | -1, `errno=ERANGE` (from `skip_padding` overrun — NOT EINVAL) | [ ] |
| 24 | `sodium_base642bin` | NO_PADDING variant (3,7) fed padded `"/w=="`, `b64_end==NULL` | -1, `errno=EINVAL`, `*bin_len` NONZERO | [ ] |
| 25 | `sodium_base642bin` | NO_PADDING variant fed padded input, `b64_end!=NULL` | **0**, `*b64_end` at first `=` | [ ] |
| 26 | `sodium_base642bin` | URLSAFE variant (5,7) fed `'+'` or `'/'` | invalid char → row 15/17 behaviour | [ ] |
| 27 | `sodium_base642bin` | ORIGINAL variant (1,3) fed `'-'` or `'_'` | invalid char → row 15/17/23 behaviour | [ ] |
| 28 | `sodium_base642bin` | `ignore` CONTAINS `'='` with a padded variant, `"/w=="`, `ignore="="` | -1, `errno=ERANGE`, `*bin_len=0` (quirk) | [ ] |
| 29 | `sodium_base642bin` | extra `'='` beyond required padding `"/w==="`, `b64_end==NULL` | -1, `errno=EINVAL` | [ ] |
| 30 | `sodium_base642bin` | `b64_end==NULL` and any unconsumed trailing byte | -1, `errno=EINVAL` | [ ] |

## C. `sodium/codecs.c` — IP parsing

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 31 | `sodium_ip2bin` | `'%'` zone with a char outside `[0-9a-zA-Z._-]`, e.g. `"fe80::1%bad!"` | -1 | [ ] |
| 32 | `sodium_ip2bin` | `'%'` with empty zone, `"fe80::1%"` | -1 | [ ] |
| 33 | `sodium_ip2bin` | `'%'` zone but address has no `':'`, `"1.2.3.4%eth0"` | -1 | [ ] |
| 34 | `sodium_ip2bin` | IPv4 octet > 255 (`"256.0.0.1"`) | -1 | [ ] |
| 35 | `sodium_ip2bin` | IPv4 with > 3 digits in an octet (`"1234.1.1.1"`) | -1 | [ ] |
| 36 | `sodium_ip2bin` | IPv4 missing octet (`"1.2.3"`) | -1 | [ ] |
| 37 | `sodium_ip2bin` | IPv4 extra octet (`"1.2.3.4.5"`) | -1 | [ ] |
| 38 | `sodium_ip2bin` | empty string `""` or `"."` | -1 | [ ] |
| 39 | `sodium_ip2bin` | IPv6 group with > 4 hex digits (`"12345::"`) | -1 | [ ] |
| 40 | `sodium_ip2bin` | IPv6 non-hex char (`"g::1"`) | -1 | [ ] |
| 41 | `sodium_ip2bin` | single leading `':'` not followed by `':'` (`":1"`) | -1 | [ ] |
| 42 | `sodium_ip2bin` | trailing single `':'` (`"1:"`) | -1 | [ ] |
| 43 | `sodium_ip2bin` | two `"::"` runs (`"2001:db8::1::2"`) | -1 | [ ] |
| 44 | `sodium_ip2bin` | more than 8 IPv6 groups (`"1:2:3:4:5:6:7:8:9"`) | -1 | [ ] |
| 45 | `sodium_ip2bin` | `"::"` when 16 bytes already filled | -1 | [ ] |
| 46 | `sodium_bin2ip` | `ip_maxlen <= 2` | NULL | [ ] |
| 47 | `sodium_bin2ip` | formatted length `>= ip_maxlen` | NULL | [ ] |

## D. `sodium/utils.c` — pad / unpad / compare / alloc

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 48 | `sodium_pad` | `blocksize == 0` | -1, `*padded_buflen_p` NOT written | [ ] |
| 49 | `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen` (arith overflow) | **misuse** (not -1) | [ ] |
| 50 | `sodium_pad` | `xpadded_len >= max_buflen` | -1, `*padded_buflen_p` NOT written | [ ] |
| 51 | `sodium_unpad` | `blocksize == 0` | -1, `*unpadded_buflen_p` NOT written | [ ] |
| 52 | `sodium_unpad` | `padded_buflen < blocksize` | -1, `*unpadded_buflen_p` NOT written | [ ] |
| 53 | `sodium_unpad` | no `0x80` barrier preceded only by `0x00`s in the last `blocksize` bytes (all-zero, all-0xff, `0x81` terminator, nonzero after barrier) | -1 **AND** `*unpadded_buflen_p = padded_buflen-1` IS written | [ ] |
| 54 | `sodium_malloc` | `size >= SIZE_MAX - page_size*4` (HAVE_ALIGNED_MALLOC) | NULL, `errno=ENOMEM` | [ ] |
| 55 | `sodium_allocarray` | `count > 0 && size >= SIZE_MAX / count` | NULL, `errno=ENOMEM` | [ ] |
| 56 | `sodium_free` | `ptr == NULL` | no-op | [ ] |
| 57 | `sodium_mprotect_noaccess` | build without HAVE_PAGE_PROTECTION | -1, `errno=ENOSYS` | [ ] |
| 58 | `sodium_mprotect_readonly` | build without HAVE_PAGE_PROTECTION | -1, `errno=ENOSYS` | [ ] |
| 59 | `sodium_mprotect_readwrite` | build without HAVE_PAGE_PROTECTION | -1, `errno=ENOSYS` | [ ] |
| 60 | `sodium_mlock` | no HAVE_MLOCK, or `mlock()` fails (RLIMIT_MEMLOCK/EPERM) | -1 | [ ] |
| 61 | `sodium_munlock` | as `sodium_mlock`; zeroes region first | -1 | [ ] |
| 62 | `sodium_memcmp` | any of `len` bytes differ | -1 (0 if equal; `len==0` → 0) | [ ] |
| 63 | `sodium_compare` | little-endian `b1 < b2` / `>` | -1 / 1 (0 if equal; `len==0` → 0) | [ ] |
| 64 | `sodium_is_zero` | any nonzero byte | 0 (1 if all zero; `nlen==0` → 1) | [ ] |

## E. `sodium/core.c`, `runtime.c`, `version.c`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 65 | `sodium_init` | any call after the first successful one | 1 | [ ] |
| 66 | `sodium_init` | `sodium_crit_enter()`/`_leave()` failure | -1 | [ ] |
| 67 | `sodium_crit_leave` | called while `locked == 0` | -1, `errno=EPERM` | [ ] |
| 68 | `sodium_misuse` | always | handler (if set) then unconditional `abort()`; `noreturn` — a handler that RETURNS still aborts | [ ] |
| 69 | `sodium_set_misuse_handler` | `handler == NULL` | **0** (valid — clears the handler) | [ ] |
| 70 | `sodium_runtime_has_*` (12 fns) | called before `sodium_init()` | 0 (static zero-init `CPUFeatures`) | [ ] |

## F. `randombytes/`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 71 | `randombytes_set_implementation` | any impl (never validated) | 0 | [ ] |
| 72 | `randombytes_set_implementation` | `NULL` (declared nonnull) | 0; impl becomes NULL ⇒ next call resets to `&randombytes_sysrandom_implementation` + stirs | [ ] |
| 73 | `randombytes_uniform` | `upper_bound == 0` | 0 (no error) | [ ] |
| 74 | `randombytes_uniform` | `upper_bound == 1` | 0 | [ ] |
| 75 | `randombytes_uniform` | `impl->uniform != NULL` | delegates entirely, incl. bound 0/1 | [ ] |
| 76 | `randombytes_buf_deterministic` | `size > 0x4000000000` (2^38) when `SIZE_MAX >` that | misuse | [ ] |
| 77 | `randombytes_buf` | `size == 0` | inits impl, writes nothing | [ ] |
| 78 | `randombytes_close` | `impl->close == NULL` | 0 | [ ] |
| 79 | `randombytes_close` | `/dev/urandom` path, second call (fd already -1) | -1 | [ ] |
| 80 | `randombytes_seedbytes` | — | 32 | [ ] |

## G. `crypto_verify/verify.c`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 81 | `crypto_verify_16` | any byte of `x[0..16)` differs from `y` | -1 (0 if equal) | [ ] |
| 82 | `crypto_verify_32` | any byte of `x[0..32)` differs | -1 | [ ] |
| 83 | `crypto_verify_64` | any byte of `x[0..64)` differs | -1 | [ ] |

## H. `crypto_aead/chacha20poly1305` + `xchacha20poly1305`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 84 | `crypto_aead_chacha20poly1305_encrypt` | `mlen > MESSAGEBYTES_MAX` (`SODIUM_SIZE_MAX-16`) | misuse | [ ] |
| 85 | `crypto_aead_chacha20poly1305_decrypt` | `clen < ABYTES` (16) | -1, `*mlen_p=0` | [ ] |
| 86 | `crypto_aead_chacha20poly1305_decrypt_detached` | `crypto_verify_16` mismatch, `m != NULL` | `memset(m,0,clen)`; -1 | [ ] |
| 87 | `crypto_aead_chacha20poly1305_decrypt_detached` | `crypto_verify_16` mismatch, `m == NULL` (verify-only) | -1, no zeroing | [ ] |
| 88 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `mlen > 64*(2^32-1)` = 274877906880 | misuse | [ ] |
| 89 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen < 16` | -1, `*mlen_p=0` | [ ] |
| 90 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | verify mismatch, `m != NULL` | `memset(m,0,clen)`; -1 | [ ] |
| 91 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | verify mismatch, `m == NULL` | -1 | [ ] |
| 92 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `mlen > SODIUM_SIZE_MAX-16` | misuse | [ ] |
| 93 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen < 16` | -1, `*mlen_p=0` | [ ] |
| 94 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | verify mismatch, `m != NULL` | `memset(m,0,clen)`; -1 | [ ] |
| 95 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | verify mismatch, `m == NULL` | -1 | [ ] |

## I. `crypto_aead/aegis128l` + `aegis256`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 96 | `crypto_aead_aegis128l_encrypt` | `mlen > MESSAGEBYTES_MAX` (`min(SIZE_MAX-32, 2^61-1)`) | misuse | [ ] |
| 97 | `crypto_aead_aegis128l_encrypt_detached` | `mlen >` or `adlen > MESSAGEBYTES_MAX` (checked AFTER `*maclen_p` written) | misuse, `*maclen_p` already mutated | [ ] |
| 98 | `crypto_aead_aegis128l_decrypt` | `clen < ABYTES` (32) | -1, `*mlen_p=0` | [ ] |
| 99 | `crypto_aead_aegis128l_decrypt_detached` | `clen >` or `adlen > MESSAGEBYTES_MAX` | -1 | [ ] |
| 100 | `crypto_aead_aegis128l_decrypt_detached` | `crypto_verify_32` mismatch (maclen 32) | `memset(m,0,mlen)` if `m!=NULL`; -1 | [ ] |
| 101 | `crypto_aead_aegis256_encrypt` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 102 | `crypto_aead_aegis256_encrypt_detached` | `mlen >` or `adlen > MESSAGEBYTES_MAX` (after `*maclen_p`) | misuse | [ ] |
| 103 | `crypto_aead_aegis256_decrypt` | `clen < 32` | -1, `*mlen_p=0` | [ ] |
| 104 | `crypto_aead_aegis256_decrypt_detached` | `clen >` or `adlen > MESSAGEBYTES_MAX` | -1 | [ ] |
| 105 | `crypto_aead_aegis256_decrypt_detached` | `crypto_verify_32` mismatch | `memset(m,0,mlen)`; -1 | [ ] |

## J. `crypto_aead/aes256gcm` — portable build: ENOSYS stubs

No `HAVE_TMMINTRIN_H`/`HAVE_WMMINTRIN_H`/`HAVE_ARMCRYPTO` ⇒ `aead_aes256gcm.c`
stubs are linked. **Every** entry point unconditionally fails.

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 106 | `crypto_aead_aes256gcm_is_available` | always | **0** | [ ] |
| 107 | `crypto_aead_aes256gcm_encrypt` | always | -1, `errno=ENOSYS` | [ ] |
| 108 | `crypto_aead_aes256gcm_encrypt_detached` | always | -1, `errno=ENOSYS` | [ ] |
| 109 | `crypto_aead_aes256gcm_decrypt` | always | -1, `errno=ENOSYS` | [ ] |
| 110 | `crypto_aead_aes256gcm_decrypt_detached` | always | -1, `errno=ENOSYS` | [ ] |
| 111 | `crypto_aead_aes256gcm_beforenm` | always | -1, `errno=ENOSYS` | [ ] |
| 112 | `crypto_aead_aes256gcm_encrypt_afternm` | always | -1, `errno=ENOSYS` | [ ] |
| 113 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | always | -1, `errno=ENOSYS` | [ ] |
| 114 | `crypto_aead_aes256gcm_decrypt_afternm` | always | -1, `errno=ENOSYS` | [ ] |
| 115 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | always | -1, `errno=ENOSYS` | [ ] |

## K. `crypto_secretbox`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 116 | `crypto_secretbox_easy` | `mlen > SODIUM_SIZE_MAX-16` | misuse | [ ] |
| 117 | `crypto_secretbox_open_easy` | `clen < MACBYTES` (16) | -1 | [ ] |
| 118 | `crypto_secretbox_open_detached` | poly1305 verify mismatch | zeroes subkey; -1 | [ ] |
| 119 | `crypto_secretbox_xsalsa20poly1305` (= `crypto_secretbox`) | `mlen < ZEROBYTES` (32) | -1 | [ ] |
| 120 | `crypto_secretbox_xsalsa20poly1305_open` (= `crypto_secretbox_open`) | `clen < 32` | -1 | [ ] |
| 121 | `crypto_secretbox_xsalsa20poly1305_open` | poly1305 verify mismatch | -1 | [ ] |
| 122 | `crypto_secretbox_xchacha20poly1305_easy` | `mlen > SODIUM_SIZE_MAX-16` | misuse | [ ] |
| 123 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen < 16` | -1 | [ ] |
| 124 | `crypto_secretbox_xchacha20poly1305_open_detached` | poly1305 verify mismatch | zeroes subkey; -1 | [ ] |

## L. `crypto_secretstream/xchacha20poly1305`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 125 | `..._push` | `mlen > MESSAGEBYTES_MAX` (`min(SIZE_MAX-17, 64*(2^32-2))` = 274877906816) | misuse, `*outlen_p` already 0 | [ ] |
| 126 | `..._pull` | `inlen < ABYTES` (17) | -1, `*mlen_p=0`, `*tag_p=0xff` | [ ] |
| 127 | `..._pull` | `inlen-17 > MESSAGEBYTES_MAX` | misuse | [ ] |
| 128 | `..._pull` | `sodium_memcmp(mac, stored_mac, 16) != 0` (forged) | -1, `*mlen_p` stays 0, `*tag_p` stays 0xff, **state NOT advanced** | [ ] |
| 129 | `..._init_pull` | any 24-byte header (NO validation) | **0** — a wrong header only shows up as a MAC failure on the first `_pull` | [ ] |
| 130 | `..._push` | any `tag` byte 0..255 (NO validation); only bit `0x02` triggers rekey | **0** | [ ] |

## M. `crypto_box` (both primitives)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 131 | `crypto_box_beforenm` / `..._curve25519xsalsa20poly1305_beforenm` | `crypto_scalarmult_curve25519` returns -1 (pk is a low-order point ⇒ all-zero shared secret) | -1 | [ ] |
| 132 | `crypto_box_detached` | `crypto_box_beforenm` fails | -1 | [ ] |
| 133 | `crypto_box_easy` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 134 | `crypto_box_easy_afternm` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 135 | `crypto_box_easy` | beforenm fails (low-order pk) | -1 | [ ] |
| 136 | `crypto_box_open_detached` | beforenm fails | -1 | [ ] |
| 137 | `crypto_box_open_easy` | `clen < MACBYTES` (16) | -1 | [ ] |
| 138 | `crypto_box_open_easy_afternm` | `clen < 16` | -1 | [ ] |
| 139 | `crypto_box_open_detached_afternm` | poly1305 mismatch | -1 | [ ] |
| 140 | `crypto_box_curve25519xsalsa20poly1305` (= `crypto_box`) | `mlen < 32` | -1 | [ ] |
| 141 | `crypto_box_curve25519xsalsa20poly1305_open` (= `crypto_box_open`) | `clen < 32` | -1 | [ ] |
| 142 | `crypto_box_seal` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 143 | `crypto_box_seal` | recipient pk low-order ⇒ `crypto_box_easy` fails | -1; `c[0..32)` STILL overwritten with epk | [ ] |
| 144 | `crypto_box_seal_open` | `clen < SEALBYTES` (48) | -1 | [ ] |
| 145 | `crypto_box_seal_open` | inner `crypto_box_open_easy` fails | -1 | [ ] |
| 146 | `crypto_box_curve25519xchacha20poly1305_beforenm` | scalarmult fails | -1 | [ ] |
| 147 | `crypto_box_curve25519xchacha20poly1305_easy` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 148 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 149 | `crypto_box_curve25519xchacha20poly1305_open_easy` | `clen < 16` | -1 | [ ] |
| 150 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | `clen < 16` | -1 | [ ] |
| 151 | `crypto_box_curve25519xchacha20poly1305_seal` | `mlen > MESSAGEBYTES_MAX` | misuse | [ ] |
| 152 | `crypto_box_curve25519xchacha20poly1305_seal_open` | `clen < SEALBYTES` (48) | -1 | [ ] |
| 153 | `crypto_box_curve25519xchacha20poly1305_detached` / `_open_detached` | beforenm fails | -1 | [ ] |

## N. `crypto_auth` / `crypto_onetimeauth`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 154 | `crypto_auth_hmacsha256_init` | `key == NULL && 0 < keylen <= 64` | misuse | [ ] |
| 155 | `crypto_auth_hmacsha512_init` | `key == NULL && 0 < keylen <= 128` | misuse | [ ] |
| 156 | `crypto_auth_hmacsha512256_init` | `key == NULL && 0 < keylen <= 128` (delegates) | misuse | [ ] |
| 157 | `crypto_auth_hmacsha256_verify` | `crypto_verify_32` mismatch | -1 | [ ] |
| 158 | `crypto_auth_hmacsha512_verify` | `crypto_verify_64` mismatch | -1 | [ ] |
| 159 | `crypto_auth_hmacsha512256_verify` | `crypto_verify_32` mismatch | -1 | [ ] |
| 160 | `crypto_auth_verify` | delegates to hmacsha512256 | -1 | [ ] |
| 161 | `crypto_onetimeauth_poly1305_verify` / `crypto_onetimeauth_verify` | `crypto_verify_16` mismatch | -1 | [ ] |

## O. `crypto_sign/ed25519`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 162 | `crypto_sign_ed25519_verify_detached` | `(sig[63] & 240) != 0` **and** `sc25519_is_canonical(sig+32) == 0` (S ≥ L with high nibble set) | -1 | [ ] |
| 163 | `crypto_sign_ed25519_verify_detached` | `ge25519_is_canonical(pk) == 0` (y ≥ 2^255-19) | -1 | [ ] |
| 164 | `crypto_sign_ed25519_verify_detached` | `ge25519_frombytes_negate_vartime(&A, pk) != 0` (pk y has no curve point) | -1 | [ ] |
| 165 | `crypto_sign_ed25519_verify_detached` | `ge25519_has_small_order(&A) != 0` (pk is one of the 8 small-order points) | -1 | [ ] |
| 166 | `crypto_sign_ed25519_verify_detached` | `ge25519_frombytes(&expected_r, sig) != 0` (R not a curve point) | -1 | [ ] |
| 167 | `crypto_sign_ed25519_verify_detached` | `ge25519_has_small_order(&expected_r) != 0` (R small order) | -1 | [ ] |
| 168 | `crypto_sign_ed25519_verify_detached` | FINAL: `check = R - (S*B - h*A)` is NOT small order. Return is `ge25519_has_small_order(&check) - 1` — **COFACTORED** verification; `crypto_verify_32` is included but never called | -1 | [ ] |
| 169 | `crypto_sign_verify_detached` | thin wrapper — all of 162–168 | -1 | [ ] |
| 170 | `crypto_sign_ed25519_open` | `smlen < 64` | -1, `*mlen_p=0`, `m` untouched | [ ] |
| 171 | `crypto_sign_ed25519_open` | `smlen - 64 > MESSAGEBYTES_MAX` | -1, `*mlen_p=0` | [ ] |
| 172 | `crypto_sign_ed25519_open` | inner verify fails | -1, `memset(m,0,mlen)` if `m!=NULL`, `*mlen_p=0` | [ ] |
| 173 | `crypto_sign_open` | thin wrapper | -1 | [ ] |
| 174 | `crypto_sign_ed25519_pk_to_curve25519` | `ge25519_frombytes_negate_vartime` fails | -1 | [ ] |
| 175 | `crypto_sign_ed25519_pk_to_curve25519` | `ge25519_has_small_order(&A) != 0` | -1 | [ ] |
| 176 | `crypto_sign_ed25519_pk_to_curve25519` | `ge25519_is_on_main_subgroup(&A) == 0` (torsion component) | -1 | [ ] |
| 177 | `crypto_sign_ed25519_pk_to_curve25519` | **NO** `ge25519_is_canonical` call ⇒ non-canonical y encodings are ACCEPTED | **0** | [ ] |
| 178 | `crypto_sign_ed25519ph_final_verify` / `crypto_sign_final_verify` | all of 162–168 on the 64-byte prehash with `prehashed=1` | -1 | [ ] |
| 179 | `crypto_sign_ed25519` | `siglen != 64` (LCOV_EXCL, unreachable) | -1, `*smlen_p=0`, `memset(sm,0,mlen+64)` | [ ] |

## P. `crypto_scalarmult`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 180 | `crypto_scalarmult_curve25519` | `has_small_order(p)`: p (bit 255 masked) is one of 7 blocklisted encodings (0, 1, order-8 pts, p-1, p, p+1) | -1 | [ ] |
| 181 | `crypto_scalarmult_curve25519` | all 32 output bytes zero: `-(1 & ((d-1) >> 8))` | -1 | [ ] |
| 182 | `crypto_scalarmult` | thin wrapper | -1 | [ ] |
| 183 | `crypto_scalarmult_curve25519_base` | **NO** rejection branch at all (no small-order, no zero-output check) | **0** always | [ ] |
| 184 | `crypto_scalarmult_ed25519` / `_noclamp` | `ge25519_is_canonical(p) == 0` | -1 | [ ] |
| 185 | `crypto_scalarmult_ed25519` / `_noclamp` | `ge25519_frombytes(&P, p) != 0` | -1 | [ ] |
| 186 | `crypto_scalarmult_ed25519` / `_noclamp` | `ge25519_has_small_order(&P) != 0` | -1 | [ ] |
| 187 | `crypto_scalarmult_ed25519` / `_noclamp` | `ge25519_is_on_main_subgroup(&P) == 0` | -1 | [ ] |
| 188 | `crypto_scalarmult_ed25519` / `_noclamp` | `_is_inf(q)` — result encodes the identity | -1 | [ ] |
| 189 | `crypto_scalarmult_ed25519` / `_noclamp` | `sodium_is_zero(n, 32)` — checked AFTER the multiply, on the ORIGINAL (unclamped) n | -1 | [ ] |
| 190 | `crypto_scalarmult_ed25519_base` / `_base_noclamp` | `_is_inf(q)` | -1 | [ ] |
| 191 | `crypto_scalarmult_ed25519_base` / `_base_noclamp` | `sodium_is_zero(n, 32)` (fires even though clamping makes the point non-identity) | -1 | [ ] |
| 192 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes` fails: `is_canonical(s)==0` (s ≥ p, or bit 255 set, or s odd) | -1 | [ ] |
| 193 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes` fails: `1/(v*u2^2)` not a square | -1 | [ ] |
| 194 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes` fails: `fe25519_isnegative(h->T)` | -1 | [ ] |
| 195 | `crypto_scalarmult_ristretto255` | `ristretto255_frombytes` fails: `fe25519_iszero(h->Y)` | -1 | [ ] |
| 196 | `crypto_scalarmult_ristretto255` | `sodium_is_zero(q, 32)` (result is the ristretto identity, e.g. n ≡ 0 mod L) | -1 | [ ] |
| 197 | `crypto_scalarmult_ristretto255_base` | `sodium_is_zero(q, 32)` (n ≡ 0 mod L after `t[31] &= 127`) | -1 | [ ] |

## Q. `crypto_core/ed25519` + `ristretto255` + `h2c`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 198 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_canonical(p) == 0` | **0** | [ ] |
| 199 | `crypto_core_ed25519_is_valid_point` | `ge25519_frombytes` fails | 0 | [ ] |
| 200 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_curve == 0` | 0 | [ ] |
| 201 | `crypto_core_ed25519_is_valid_point` | `ge25519_has_small_order != 0` | 0 | [ ] |
| 202 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_main_subgroup == 0` | 0 | [ ] |
| 203 | `crypto_core_ed25519_add` | `ge25519_frombytes(p)` fails | -1 | [ ] |
| 204 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(p) == 0` | -1 | [ ] |
| 205 | `crypto_core_ed25519_add` | `ge25519_frombytes(q)` fails | -1 | [ ] |
| 206 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(q) == 0` | -1 | [ ] |
| 207 | `crypto_core_ed25519_add` / `_sub` | **NO** is_canonical / small-order / main-subgroup checks ⇒ those points are ACCEPTED | **0** | [ ] |
| 208 | `crypto_core_ed25519_sub` | `frombytes(p)` / `is_on_curve(p)` / `frombytes(q)` / `is_on_curve(q)` fail (4 branches) | -1 | [ ] |
| 209 | `crypto_core_ed25519_from_string` / `_from_string_nu` / `_scalar_from_string` | `hash_alg` ∉ {1,2} — **out-of-range enum across FFI** | -1, `errno=EINVAL` | [ ] |
| 210 | `crypto_core_ed25519_scalar_invert` | `sodium_is_zero(s, 32)`; `recip` IS still written by `sc25519_invert` before the check | -1 | [ ] |
| 211 | `crypto_core_ed25519_scalar_is_canonical` | `s >= L` | **0** (1 if canonical) | [ ] |
| 212 | `crypto_core_ristretto255_is_valid_point` | `ristretto255_frombytes` fails (4 sub-branches of 192–195) | **0** | [ ] |
| 213 | `crypto_core_ristretto255_add` | `frombytes(p)` fails | -1 | [ ] |
| 214 | `crypto_core_ristretto255_add` | `frombytes(q)` fails | -1 | [ ] |
| 215 | `crypto_core_ristretto255_sub` | `frombytes(p)` fails | -1 | [ ] |
| 216 | `crypto_core_ristretto255_sub` | `frombytes(q)` fails | -1 | [ ] |
| 217 | `crypto_core_ristretto255_scalar_invert` | `sodium_is_zero(s, 32)` | -1 | [ ] |
| 218 | `crypto_core_ristretto255_from_string` / `_from_string_nu` / `_scalar_from_string` | `hash_alg` ∉ {1,2} | -1, `errno=EINVAL` | [ ] |
| 219 | `crypto_core_ristretto255_scalar_is_canonical` | `s >= L` | 0 | [ ] |
| 220 | `crypto_core_ristretto255_from_hash` | any 64-byte input (NO rejection) | **0** | [ ] |
| 221 | `core_h2c_string_to_hash` | `hash_alg` default case | -1, `errno=EINVAL` | [ ] |

## R. `crypto_generichash/blake2b`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 222 | `crypto_generichash_blake2b` | `outlen == 0` | -1 | [ ] |
| 223 | `crypto_generichash_blake2b` | `outlen > 64` | -1 | [ ] |
| 224 | `crypto_generichash_blake2b` | `keylen > 64` | -1 | [ ] |
| 225 | `crypto_generichash_blake2b` | `outlen` 1..15 (< `BYTES_MIN`=16) and `keylen` 1..15 (< `KEYBYTES_MIN`=16) are **ACCEPTED** — the `_MIN` constants are NOT enforced anywhere | **0** | [ ] |
| 226 | `crypto_generichash_blake2b_salt_personal` | `outlen==0` \| `outlen>64` \| `keylen>64` | -1 | [ ] |
| 227 | `crypto_generichash_blake2b_init` | `outlen == 0` | -1 | [ ] |
| 228 | `crypto_generichash_blake2b_init` | `outlen > 64` | -1 | [ ] |
| 229 | `crypto_generichash_blake2b_init` | `keylen > 64` | -1 | [ ] |
| 230 | `crypto_generichash_blake2b_init` | `key == NULL` OR `keylen == 0` ⇒ unkeyed path (NOT an error) | **0** | [ ] |
| 231 | `crypto_generichash_blake2b_init_salt_personal` | `outlen==0` \| `outlen>64` \| `keylen>64` | -1 | [ ] |
| 232 | `crypto_generichash_blake2b_final` | `(uint8_t)outlen == 0` or `> 64` reaches `blake2b_final`'s guard | **misuse** (NOT -1) | [ ] |
| 233 | `crypto_generichash_blake2b_final` | `blake2b_is_lastblock(S)` — `_final` already called on this state | -1 | [ ] |
| 234 | `crypto_generichash_blake2b_final` | **NO** check that final's `outlen` matches init's ⇒ mismatch silently returns 0 with a truncated digest | **0** | [ ] |
| 235 | `crypto_generichash` / `_init` / `_update` / `_final` | thin wrappers — identical branches | -1 / misuse | [ ] |
| 236 | `crypto_generichash_statebytes` | — | 384 | [ ] |

## S. `crypto_hash` (sha256 / sha512 / sha3) + `crypto_xof`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 237 | `crypto_hash_sha256_update` / `sha512_update` | `inlen == 0` ⇒ early return, no state change | 0 | [ ] |
| 238 | `crypto_hash_sha256_final` / `sha512_final` | zeroizes state ⇒ calling `_final` twice silently restarts from an all-zero state | 0 | [ ] |
| 239 | `crypto_hash_sha3256_update` / `sha3512_update` | `state->phase != ABSORBING` (update after final) — the input IS still absorbed after permute+reset | **-1** | [ ] |
| 240 | `crypto_hash_sha3256_final` / `sha3512_final` | `state->phase != ABSORBING` (final twice) — output IS still written | **-1** | [ ] |
| 241 | `crypto_hash_sha3256` / `sha3512` one-shot | discards `_update`/`_final` return values | 0 | [ ] |
| 242 | `crypto_xof_shake128_update` (and shake256 / turboshake128 / turboshake256) | `state->phase != ABSORBING` (update after squeeze) | **-1** | [ ] |
| 243 | `crypto_xof_*_init_with_domain` | **ANY** domain byte 0x00..0xFF accepted — there is NO 0x01..0x7F range check anywhere in `crypto_xof` | **0** | [ ] |
| 244 | `crypto_xof_*_squeeze` | `outlen == 0` still finalizes if phase == ABSORBING | 0 | [ ] |

## T. `crypto_stream`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 245 | `crypto_stream_chacha20` | `clen > MESSAGEBYTES_MAX` (`SODIUM_SIZE_MAX`) | misuse | [ ] |
| 246 | `crypto_stream_chacha20_xor_ic` | `mlen > SODIUM_SIZE_MAX` | misuse | [ ] |
| 247 | `crypto_stream_chacha20_xor` | `mlen > SODIUM_SIZE_MAX` | misuse | [ ] |
| 248 | `crypto_stream_chacha20_ietf` | `clen > ietf_MESSAGEBYTES_MAX` (`min(SODIUM_SIZE_MAX, 64*2^32)` = 274877906944) | misuse | [ ] |
| 249 | `crypto_stream_chacha20_ietf_xor_ic` | `ic > 2^32 - ceil(mlen/64)` (32-bit block counter would wrap) | misuse | [ ] |
| 250 | `crypto_stream_chacha20_ietf_xor_ic` | QUIRK: no separate `mlen` check; for `mlen > 64*2^32` the unsigned subtraction underflows so the guard **NEVER fires** | **0** | [ ] |
| 251 | `crypto_stream_chacha20_ietf_xor` | `mlen > ietf_MESSAGEBYTES_MAX` | misuse | [ ] |
| 252 | `crypto_stream_chacha20_ietf_ext` / `_ext_xor_ic` | `clen`/`mlen > SODIUM_SIZE_MAX` (the NON-ietf bound) | misuse | [ ] |
| 253 | `crypto_stream_salsa20` / `_xor` / `_xor_ic` | **NO bounds check at all** (`MESSAGEBYTES_MAX` never tested) | **0** | [ ] |
| 254 | `crypto_stream_salsa2012` / `salsa208` / `_xor` | **NO bounds check**; `len == 0` ⇒ early return | 0 | [ ] |
| 255 | `crypto_stream_xsalsa20*` | no own check; delegates to salsa20 (never fails) | 0 | [ ] |
| 256 | `crypto_stream_xchacha20*` | delegates to chacha20 ⇒ aborts if `len > SODIUM_SIZE_MAX` | 0 / misuse | [ ] |
| 257 | all `stream_ref*` | `clen`/`mlen == 0` ⇒ early return, nothing written | 0 | [ ] |

## U. `crypto_pwhash/argon2` — `argon2_validate_inputs` (argon2-core.c)

Numeric values are the `Argon2_ErrorCodes` enum from `crypto_pwhash/argon2/argon2.h`.

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 258 | `argon2_validate_inputs` | `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = -25 | [ ] |
| 259 | `argon2_validate_inputs` | `context->out == NULL` | `ARGON2_OUTPUT_PTR_NULL` = -1 | [ ] |
| 260 | `argon2_validate_inputs` | `outlen < ARGON2_MIN_OUTLEN` (16) | `ARGON2_OUTPUT_TOO_SHORT` = -2 | [ ] |
| 261 | `argon2_validate_inputs` | `outlen > ARGON2_MAX_OUTLEN` (0xFFFFFFFF) — unreachable, field is u32 | `ARGON2_OUTPUT_TOO_LONG` = -3 | [ ] |
| 262 | `argon2_validate_inputs` | `pwd == NULL && pwdlen != 0` | `ARGON2_PWD_PTR_MISMATCH` = -18 | [ ] |
| 263 | `argon2_validate_inputs` | `pwdlen > ARGON2_MAX_PWD_LENGTH` — unreachable | `ARGON2_PWD_TOO_LONG` = -5 | [ ] |
| 264 | `argon2_validate_inputs` | `salt == NULL && saltlen != 0` | `ARGON2_SALT_PTR_MISMATCH` = -19 | [ ] |
| 265 | `argon2_validate_inputs` | `saltlen < ARGON2_MIN_SALT_LENGTH` (8), e.g. 0 or 7 | `ARGON2_SALT_TOO_SHORT` = -6 | [ ] |
| 266 | `argon2_validate_inputs` | `saltlen > ARGON2_MAX_SALT_LENGTH` — unreachable | `ARGON2_SALT_TOO_LONG` = -7 | [ ] |
| 267 | `argon2_validate_inputs` | `secret == NULL && secretlen != 0` | `ARGON2_SECRET_PTR_MISMATCH` = -20 | [ ] |
| 268 | `argon2_validate_inputs` | `secret != NULL && secretlen > ARGON2_MAX_SECRET` — unreachable | `ARGON2_SECRET_TOO_LONG` = -11 | [ ] |
| 269 | `argon2_validate_inputs` | `ad == NULL && adlen != 0` | `ARGON2_AD_PTR_MISMATCH` = -21 | [ ] |
| 270 | `argon2_validate_inputs` | `ad != NULL && adlen > ARGON2_MAX_AD_LENGTH` — unreachable | `ARGON2_AD_TOO_LONG` = -9 | [ ] |
| 271 | `argon2_validate_inputs` | `lanes < ARGON2_MIN_LANES` (1), i.e. `lanes == 0` | `ARGON2_LANES_TOO_FEW` = -16 | [ ] |
| 272 | `argon2_validate_inputs` | `lanes > ARGON2_MAX_LANES` (0xFFFFFF = 16777215) | `ARGON2_LANES_TOO_MANY` = -17 | [ ] |
| 273 | `argon2_validate_inputs` | `m_cost < ARGON2_MIN_MEMORY` (`2*ARGON2_SYNC_POINTS` = 8) | `ARGON2_MEMORY_TOO_LITTLE` = -14 | [ ] |
| 274 | `argon2_validate_inputs` | `m_cost > ARGON2_MAX_MEMORY` (4294967295) | `ARGON2_MEMORY_TOO_MUCH` = -15 | [ ] |
| 275 | `argon2_validate_inputs` | `m_cost < 8 * lanes` (e.g. lanes=4, m_cost=16) | `ARGON2_MEMORY_TOO_LITTLE` = -14 | [ ] |
| 276 | `argon2_validate_inputs` | `t_cost < ARGON2_MIN_TIME` (1), i.e. `t_cost == 0` | `ARGON2_TIME_TOO_SMALL` = -12 | [ ] |
| 277 | `argon2_validate_inputs` | `t_cost > ARGON2_MAX_TIME` — unreachable | `ARGON2_TIME_TOO_LARGE` = -13 | [ ] |
| 278 | `argon2_validate_inputs` | `threads < ARGON2_MIN_THREADS` (1) | `ARGON2_THREADS_TOO_FEW` = -28 | [ ] |
| 279 | `argon2_validate_inputs` | `threads > ARGON2_MAX_THREADS` (16777215) | `ARGON2_THREADS_TOO_MANY` = -29 | [ ] |
| 280 | `argon2_ctx` | `type` ∉ {`Argon2_i`=1, `Argon2_id`=2} | `ARGON2_INCORRECT_TYPE` = -26 | [ ] |
| 281 | `argon2_initialize` | `instance == NULL \|\| context == NULL` | -25 | [ ] |
| 282 | `allocate_memory` | `m_cost == 0` or `sizeof(block)*m_cost` overflows | `ARGON2_MEMORY_ALLOCATION_ERROR` = -22 | [ ] |
| 283 | `allocate_memory` | `malloc` / `mmap` / `posix_memalign` failure | -22 | [ ] |
| 284 | `argon2_hash` | `pwdlen > ARGON2_MAX_PWD_LENGTH` | -5 | [ ] |
| 285 | `argon2_hash` | `hashlen > ARGON2_MAX_OUTLEN` | -3 | [ ] |
| 286 | `argon2_hash` | `saltlen > ARGON2_MAX_SALT_LENGTH` | -7 | [ ] |
| 287 | `argon2_hash` | `argon2_encode_string` fails (encoded buffer too small) | `ARGON2_ENCODING_FAIL` = -31 | [ ] |
| 288 | `argon2_verify` | `strlen(encoded) > UINT32_MAX` | `ARGON2_DECODING_LENGTH_FAIL` = -34 | [ ] |
| 289 | `argon2_verify` | decode succeeds, recompute succeeds, but `sodium_memcmp` differs (wrong password) | `ARGON2_VERIFY_MISMATCH` = -35 | [ ] |

## V. `crypto_pwhash/argon2/argon2-encoding.c` — decode / encode

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 290 | `argon2_decode_string` | `type` ∉ {1,2} | -26 | [ ] |
| 291 | `argon2_decode_string` | str does not start with `"$argon2id"` / `"$argon2i"` for the requested type (incl. wrong-variant string) | `ARGON2_DECODING_FAIL` = -32 | [ ] |
| 292 | `argon2_decode_string` | missing `"$v="` | -32 | [ ] |
| 293 | `argon2_decode_string` | version field not a minimal decimal / > UINT32_MAX | -32 | [ ] |
| 294 | `argon2_decode_string` | `version != ARGON2_VERSION_NUMBER` (0x13 = 19), e.g. `"$v=16"` | **-26** (`ARGON2_INCORRECT_TYPE`) | [ ] |
| 295 | `argon2_decode_string` | missing `"$m="` | -32 | [ ] |
| 296 | `argon2_decode_string` | `m=` not a minimal decimal / > UINT32_MAX | -32 | [ ] |
| 297 | `argon2_decode_string` | missing `",t="` | -32 | [ ] |
| 298 | `argon2_decode_string` | `t=` not a minimal decimal / > UINT32_MAX | -32 | [ ] |
| 299 | `argon2_decode_string` | missing `",p="` | -32 | [ ] |
| 300 | `argon2_decode_string` | `p=` not a minimal decimal / > UINT32_MAX | -32 | [ ] |
| 301 | `argon2_decode_string` | missing `"$"` before the salt | -32 | [ ] |
| 302 | `argon2_decode_string` | salt not valid base64 `ORIGINAL_NO_PADDING`, or decoded salt longer than `ctx->saltlen` | -32 | [ ] |
| 303 | `argon2_decode_string` | missing `"$"` before the hash | -32 | [ ] |
| 304 | `argon2_decode_string` | hash not valid base64, or decoded hash longer than `ctx->outlen` | -32 | [ ] |
| 305 | `argon2_decode_string` | `argon2_validate_inputs` on the decoded ctx fails (saltlen<8 → -6; outlen<16 → -2; m_cost<8 or <8*p → -14; t=0 → -12; p=0 → -16; p>16777215 → -17) | that `ARGON2_*` code | [ ] |
| 306 | `argon2_decode_string` | trailing garbage after the hash (`*str != 0`) | -32 | [ ] |
| 307 | `decode_decimal` | no digit at the current position | NULL ⇒ -32 | [ ] |
| 308 | `decode_decimal` | non-minimal encoding: leading `'0'` followed by more digits (e.g. `"m=065536"`) | NULL ⇒ -32 | [ ] |
| 309 | `decode_decimal` | numeric overflow (`acc > ULONG_MAX/10`, or `c > ULONG_MAX-acc`) | NULL ⇒ -32 | [ ] |
| 310 | `argon2_encode_string` | `type` ∉ {1,2} (switch default) | -31 | [ ] |
| 311 | `argon2_encode_string` | `argon2_validate_inputs(ctx)` fails | that code | [ ] |
| 312 | `argon2_encode_string` | `SS()`: fixed segment does not fit in remaining `dst_len` | -31 | [ ] |
| 313 | `argon2_encode_string` | `SB()`: `sodium_bin2base64` NULL because `dst_len` too small for the b64 salt or out | -31 | [ ] |
| 314 | `blake2b_long` | `outlen > UINT32_MAX` | -1 | [ ] |
| 315 | `blake2b_long` | any inner `crypto_generichash_blake2b_*` returns < 0 (e.g. `outlen == 0`) | -1 | [ ] |

## W. `crypto_pwhash` public API (argon2i / argon2id / dispatch)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 316 | `crypto_pwhash_argon2i` | `outlen > BYTES_MAX` (4294967295) | -1, `errno=EFBIG` | [ ] |
| 317 | `crypto_pwhash_argon2i` | `outlen < BYTES_MIN` (16) | -1, `errno=EINVAL` | [ ] |
| 318 | `crypto_pwhash_argon2i` | `passwdlen > PASSWD_MAX` (4294967295) | -1, `errno=EFBIG` | [ ] |
| 319 | `crypto_pwhash_argon2i` | `opslimit > OPSLIMIT_MAX` (4294967295) | -1, `errno=EFBIG` | [ ] |
| 320 | `crypto_pwhash_argon2i` | `memlimit > MEMLIMIT_MAX` (4398046510080) | -1, `errno=EFBIG` | [ ] |
| 321 | `crypto_pwhash_argon2i` | `opslimit < OPSLIMIT_MIN` (**3**), e.g. 1 or 2 | -1, `errno=EINVAL` | [ ] |
| 322 | `crypto_pwhash_argon2i` | `memlimit < MEMLIMIT_MIN` (8192) | -1, `errno=EINVAL` | [ ] |
| 323 | `crypto_pwhash_argon2i` | `(const void*)out == (const void*)passwd` (aliasing) | -1, `errno=EINVAL` | [ ] |
| 324 | `crypto_pwhash_argon2i` | `alg != ALG_ARGON2I13` (1) — incl. `alg=2` | -1, `errno=EINVAL` | [ ] |
| 325 | `crypto_pwhash_argon2i` | inner `argon2i_hash_raw` != OK | -1 (errno untouched) | [ ] |
| 326 | `crypto_pwhash_argon2id` | `outlen > BYTES_MAX` | -1, `errno=EFBIG` | [ ] |
| 327 | `crypto_pwhash_argon2id` | `outlen < BYTES_MIN` (16) | -1, `errno=EINVAL` | [ ] |
| 328 | `crypto_pwhash_argon2id` | `passwdlen`/`opslimit`/`memlimit` above max | -1, `errno=EFBIG` | [ ] |
| 329 | `crypto_pwhash_argon2id` | `opslimit < OPSLIMIT_MIN` (**1**), i.e. 0 | -1, `errno=EINVAL` | [ ] |
| 330 | `crypto_pwhash_argon2id` | `memlimit < 8192` | -1, `errno=EINVAL` | [ ] |
| 331 | `crypto_pwhash_argon2id` | `out == passwd` | -1, `errno=EINVAL` | [ ] |
| 332 | `crypto_pwhash_argon2id` | `alg != ALG_ARGON2ID13` (2) — incl. `alg=1` | -1, `errno=EINVAL` | [ ] |
| 333 | `crypto_pwhash_argon2i_str` / `argon2id_str` | `passwdlen`/`opslimit`/`memlimit` above max | -1, `errno=EFBIG` | [ ] |
| 334 | `crypto_pwhash_argon2i_str` | `opslimit < 3` or `memlimit < 8192` | -1, `errno=EINVAL` | [ ] |
| 335 | `crypto_pwhash_argon2id_str` | `opslimit < 1` or `memlimit < 8192` | -1, `errno=EINVAL` | [ ] |
| 336 | `crypto_pwhash_argon2i_str_verify` / `argon2id_str_verify` | `passwdlen > PASSWD_MAX` | -1, `errno=EFBIG` | [ ] |
| 337 | `crypto_pwhash_argon2i_str_verify` / `argon2id_str_verify` | inner verify == `ARGON2_VERIFY_MISMATCH` (wrong password) | -1, `errno=EINVAL` | [ ] |
| 338 | `crypto_pwhash_argon2i_str_verify` / `argon2id_str_verify` | inner verify returns any other non-OK (malformed / non-NUL-terminated / wrong prefix / wrong version / bad base64) | -1, **errno unchanged** | [ ] |
| 339 | `_needs_rehash` (static) | `opslimit > UINT32_MAX` | -1, `errno=EINVAL` | [ ] |
| 340 | `_needs_rehash` | `memlimit/1024 > UINT32_MAX` | -1, `errno=EINVAL` | [ ] |
| 341 | `_needs_rehash` | `strlen(str) >= crypto_pwhash_STRBYTES` (128) — also catches non-terminated | -1, `errno=EINVAL` | [ ] |
| 342 | `_needs_rehash` | `argon2_decode_string` fails (bad string / wrong type) | -1, `errno=EINVAL` | [ ] |
| 343 | `_needs_rehash` | decoded `t_cost != opslimit` OR `m_cost != memlimit/1024` | **1** (rehash needed — NOT an error) | [ ] |
| 344 | `crypto_pwhash` | `alg` ∉ {1,2} (e.g. 0, 3, -1) — **out-of-range enum across FFI** | -1, `errno=EINVAL` | [ ] |
| 345 | `crypto_pwhash_str_alg` | `alg` ∉ {1,2} | **misuse** (`return -1` unreachable) | [ ] |
| 346 | `crypto_pwhash_str_verify` | str begins with neither `"$argon2id$"` nor `"$argon2i$"` | -1, `errno=EINVAL` | [ ] |
| 347 | `crypto_pwhash_str_needs_rehash` | str begins with neither prefix | -1, `errno=EINVAL` | [ ] |

## X. `crypto_pwhash/scryptsalsa208sha256`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 348 | `crypto_pwhash_scryptsalsa208sha256` | `passwdlen > PASSWD_MAX` or `outlen > BYTES_MAX` (137438953440) | -1, `errno=EFBIG` | [ ] |
| 349 | `crypto_pwhash_scryptsalsa208sha256` | `outlen < BYTES_MIN` (16) | -1, `errno=EINVAL` | [ ] |
| 350 | `crypto_pwhash_scryptsalsa208sha256` | `(const void*)out == (const void*)passwd` | -1, `errno=EINVAL` | [ ] |
| 351 | `crypto_pwhash_scryptsalsa208sha256` | inner `_ll` fails | -1 (errno EFBIG/EINVAL/ENOMEM from `_ll`) | [ ] |
| 352 | `crypto_pwhash_scryptsalsa208sha256_str` | `passwdlen > PASSWD_MAX` | -1, `errno=EFBIG` | [ ] |
| 353 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_gensalt_r` / `escrypt_r` returns NULL | -1, `errno=EINVAL` | [ ] |
| 354 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `sodium_strnlen(str,102) != 101` (too short/long/not NUL-terminated) | -1 | [ ] |
| 355 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `escrypt_r` NULL (setting not `"$7$"` / bad itoa64 / bad length) | -1 | [ ] |
| 356 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `sodium_memcmp(wanted, str, 102) != 0` (wrong password) | -1 | [ ] |
| 357 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `sodium_strnlen(str,102) != 101` | -1, `errno=EINVAL` | [ ] |
| 358 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `escrypt_parse_setting` NULL | -1, `errno=EINVAL` | [ ] |
| 359 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | decoded `N_log2/r/p` differ from requested | **1** (not an error) | [ ] |
| 360 | `crypto_pwhash_scryptsalsa208sha256_ll` | `buflen > ((1<<32)-1)*32` = 137438953440 | -1, `errno=EFBIG` | [ ] |
| 361 | `crypto_pwhash_scryptsalsa208sha256_ll` | `(u64)r * (u64)p >= 2^30` (1073741824) | -1, `errno=EFBIG` | [ ] |
| 362 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > UINT32_MAX` | -1, `errno=EFBIG` | [ ] |
| 363 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N` not a power of two (`N & (N-1)`), e.g. 3, 1000 | -1, `errno=EINVAL` | [ ] |
| 364 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N < 2` (0 or 1) | -1, `errno=EINVAL` | [ ] |
| 365 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r == 0` or `p == 0` | -1, `errno=EINVAL` | [ ] |
| 366 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r > SIZE_MAX/128/p` or `N > SIZE_MAX/128/r` | -1, `errno=ENOMEM` | [ ] |
| 367 | `crypto_pwhash_scryptsalsa208sha256_ll` | `B_size + V_size` or `need + XY_size` wraps | -1, `errno=ENOMEM` | [ ] |
| 368 | `escrypt_parse_setting` | `setting[0..3) != "$7$"` | NULL | [ ] |
| 369 | `escrypt_parse_setting` | `N_log2` char, or any of the 5 `r` / 5 `p` chars, not in itoa64 `"./0-9A-Za-z"` | NULL | [ ] |
| 370 | `escrypt_gensalt_r` | `N_log2 > 63` | NULL | [ ] |
| 371 | `escrypt_gensalt_r` | `(u64)r*(u64)p >= 2^30` | NULL | [ ] |
| 372 | `escrypt_gensalt_r` | `need > buflen` or `need < saltlen` or `saltlen < srclen` | NULL | [ ] |
| 373 | `escrypt_r` | `buf == NULL`, or `need > buflen`, or `need < saltlen` | NULL | [ ] |
| 374 | `escrypt_PBKDF2_SHA256` | `dkLen > 0x1fffffffe0` | misuse (function is `void`) | [ ] |

## Y. `crypto_kdf`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 375 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len < BYTES_MIN` (16), e.g. 0 or 15 | -1, `errno=EINVAL` | [ ] |
| 376 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len > BYTES_MAX` (64), e.g. 65 | -1, `errno=EINVAL` | [ ] |
| 377 | `crypto_kdf_derive_from_key` | `subkey_len < 16` or `> 64` (delegates) | -1, `errno=EINVAL` | [ ] |
| 378 | `crypto_kdf_hkdf_sha256_expand` | `out_len > BYTES_MAX` (`0xff*32` = 8160) | -1, `errno=EINVAL` | [ ] |
| 379 | `crypto_kdf_hkdf_sha512_expand` | `out_len > BYTES_MAX` (`0xff*64` = 16320) | -1, `errno=EINVAL` | [ ] |

## Z. `crypto_kem` / `crypto_kx`

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 380 | `crypto_kem_mlkem768_enc_deterministic` | `polyvec_frombytes(pk)` yields a coefficient ≥ `MLKEM768_Q` (3329) ⇒ `polyvec_is_canonical == 0`. Only the first 1152 bytes are checked; the trailing 32-byte seed is never validated | -1 | [ ] |
| 381 | `crypto_kem_mlkem768_enc` | inner `_enc_deterministic` fails (non-canonical pk) | -1 | [ ] |
| 382 | `crypto_kem_mlkem768_dec` | **NO** rejection branch — tampered ct handled by constant-time implicit rejection (`cmov` of `shake256(z‖ct)`) | **0** always | [ ] |
| 383 | `crypto_kem_xwing_enc_deterministic` | inner `crypto_kem_mlkem768_enc_deterministic` fails (non-canonical ML-KEM part of pk) | -1 | [ ] |
| 384 | `crypto_kem_xwing_enc_deterministic` | `crypto_scalarmult_curve25519(ss, seed+32, pk+1184)` fails (low-order X25519 part of pk) | -1 | [ ] |
| 385 | `crypto_kem_xwing_enc` | either sub-failure above | -1 | [ ] |
| 386 | `crypto_kem_xwing_dec` | `crypto_scalarmult_curve25519(ss, sk_x25519, ct+1088)` fails (low-order X25519 part of ct) | -1 | [ ] |
| 387 | `crypto_kem_enc` / `_dec` | thin dispatch to `crypto_kem_xwing_*`; compile-time primitive `"xwing"`, so there is **NO** unknown-primitive error path | propagates 0 / -1 | [ ] |
| 388 | `crypto_kx_client_session_keys` | `rx == NULL && tx == NULL` (BOTH output buffers NULL) | misuse | [ ] |
| 389 | `crypto_kx_client_session_keys` | `crypto_scalarmult(q, client_sk, server_pk)` fails (low-order `server_pk`) | -1 | [ ] |
| 390 | `crypto_kx_server_session_keys` | `rx == NULL && tx == NULL` | misuse | [ ] |
| 391 | `crypto_kx_server_session_keys` | `crypto_scalarmult(q, server_sk, client_pk)` fails (low-order `client_pk`) | -1 | [ ] |

## AA. `crypto_ipcrypt` / `crypto_shorthash` — no error paths

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| 392 | `crypto_ipcrypt_*` (encrypt/decrypt, nd_*, ndx_*, pfx_*, all 4 `_keygen`) | **NO ERROR PATHS EXIST** — all return `void`; no length args, no -1/NULL/misuse | `void` | [ ] |
| 393 | `crypto_ipcrypt_ndx_*` / `pfx_*` | degenerate key where both 16-byte halves are equal | **no error**: silently re-derives the 2nd schedule from `k[i]^0x5a` (deterministic — must be mirrored) | [ ] |
| 394 | `crypto_shorthash_siphash24` / `siphashx24` | any `inlen` incl. 0 (no rejection) | 0 | [ ] |

---

## Generic FFI-boundary boundaries (covered in addition to the table)

| # | condition | [ ] |
|---|-----------|-----|
| G1 | NULL pointers for every optional out-param (`*outlen_p`, `*mlen_p`, `*tag_p`, `*clen_p`, `*maclen_p`, `*hex_end`, `*b64_end`, `*bin_len`, `*padded_buflen_p`) | [ ] |
| G2 | NULL plaintext `m` in every `*_decrypt_detached` / `*_open_detached` (verify-only mode — a DISTINCT code path) | [ ] |
| G3 | Zero lengths for every length arg (`mlen`, `clen`, `adlen`, `inlen`, `outlen`, `keylen`, `len`) | [ ] |
| G4 | Out-of-range enums across FFI: `hash_alg` ∉ {1,2}; `crypto_pwhash` `alg` ∉ {1,2}; `sodium_base64_VARIANT_*` ∉ {1,3,5,7}; `crypto_xof` domain byte 0x00/0x80/0xFF | [ ] |
| G5 | One step past every documented range (`BYTES_MIN-1`, `BYTES_MAX+1`, `OPSLIMIT_MIN-1`, `MEMLIMIT_MIN-1`, `ABYTES-1`, `blocksize-1`) | [ ] |
| G6 | `ad == NULL` with `adlen == 0`; `nsec != NULL` (all `*_NSECBYTES == 0`) | [ ] |



