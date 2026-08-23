# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

The mirror of `ERRORS.md` for **valid** inputs. Axes were derived mechanically
from the C source: every runtime option/mode/flag the public API can set, every
input SHAPE the code special-cases, and the FULL set of public entry points —
including the lowest-level ones, not just the one-shot convenience wrappers.

One row per meaningful COMBINATION the C treats differently. Each row is driven
with MANY randomized inputs (fixed seed `0x5EED_1234`, ≥64 iterations/row unless
noted) through BOTH `.so` files and compared byte-for-byte.

## Build configuration

`Cargo.toml` has **no `[features]` section**, so the crate has exactly ONE
configuration. `c_src/CMakeLists.txt` defines **no `HAVE_*` macros**, so every
`#ifdef HAVE_*` in the C selects the portable fallback (equivalent to
`configure --disable-asm`). Consequences that the table below relies on:

| axis | value in this build |
|---|---|
| AEGIS128L / AEGIS256 impl | `aegis*_soft.c` (softaes tables) |
| AES256GCM | **not available** — `aead_aes256gcm.c` ENOSYS stubs, `_is_available()==0` |
| poly1305 impl | `donna`, `poly1305_block_size == 16`, `poly1305_donna32.h` unless HAVE_TI_MODE |
| blake2b compress | `blake2b_compress_ref` |
| chacha20 / salsa20 | `*_ref` |
| curve25519 | `ref10` |
| argon2 fill_segment | `argon2_fill_segment_ref` |
| scrypt | `escrypt_kdf_nosse` |
| ipcrypt | `ipcrypt_soft` |
| keccak1600 | `keccak1600_ref` |
| `HAVE_PAGE_PROTECTION` | unset ⇒ `sodium_mprotect_*` return -1/ENOSYS, `sodium_malloc` is plain malloc |
| `ED25519_COMPAT` / `ED25519_NONDETERMINISTIC` | **not** defined |

**Determinism harness.** Anything that consumes randomness is made
differentially testable by injecting a counter-based
`struct randombytes_implementation` via `randombytes_set_implementation()`
(field order: `implementation_name`, `random`, `stir`, `uniform`, `buf`,
`close`). Rows marked **[RNG]** rely on this.

---

## 1. `sodium/codecs.c` — hex

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `sodium_bin2hex` | `bin_len` ∈ {0,1,2,3,16,64,255}; `hex_maxlen` exactly `2*bin_len+1` (minimum legal) | [ ] |
| 2 | `sodium_bin2hex` | same `bin_len` set; `hex_maxlen` oversized — assert bytes past `2*bin_len` are untouched | [ ] |
| 3 | `sodium_hex2bin` | lowercase / UPPERCASE / MiXeD hex, `ignore=NULL`, `hex_end=NULL`, exact `bin_maxlen` | [ ] |
| 4 | `sodium_hex2bin` | `ignore=":"`, separators leading / trailing / between bytes (all legal) | [ ] |
| 5 | `sodium_hex2bin` | `ignore=" \n"`, multi-line hex dump shape | [ ] |
| 6 | `sodium_hex2bin` | `ignore=""` (ignores nothing except the NUL quirk) | [ ] |
| 7 | `sodium_hex2bin` | `hex_end != NULL` × every input of rows 3–6 — assert `*hex_end` offset matches | [ ] |
| 8 | `sodium_hex2bin` | `bin_maxlen` oversized; `hex_len` ∈ {0,1,2,3, even, odd} | [ ] |
| 9 | round-trip `bin2hex` → `hex2bin` | `bin_len` 0..64, random bytes | [ ] |

## 2. `sodium/codecs.c` — base64 (full 4-variant cross-product)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 10 | `sodium_base64_encoded_len` + `sodium_base64_ENCODED_LEN` | variant {1,3,5,7} × `bin_len` 0..40 — assert the exact table (`bin_len` mod 3 = 0,1,2) | [ ] |
| 11 | `sodium_bin2base64` | variant **1** (ORIGINAL) × `bin_len` {0,1,2,3,4,5,6,7,8,32,33,64,255}; `b64_maxlen` = exact `ENCODED_LEN` | [ ] |
| 12 | `sodium_bin2base64` | variant **3** (ORIGINAL_NO_PADDING) × same `bin_len` set | [ ] |
| 13 | `sodium_bin2base64` | variant **5** (URLSAFE) × same `bin_len` set — alphabet `-_` | [ ] |
| 14 | `sodium_bin2base64` | variant **7** (URLSAFE_NO_PADDING) × same `bin_len` set | [ ] |
| 15 | `sodium_bin2base64` | all 4 variants, `b64_maxlen` OVERSIZED — compare the WHOLE buffer (C zero-fills `b64[b64_len..b64_maxlen)`) | [ ] |
| 16 | `sodium_base642bin` | variant {1,3,5,7} × `b64_len` mod 4 ∈ {0,2,3} × `bin_maxlen` exact | [ ] |
| 17 | `sodium_base642bin` | all 4 variants, `bin_maxlen` oversized, `b64_end=NULL`, `bin_len=NULL` | [ ] |
| 18 | `sodium_base642bin` | all 4 variants, `b64_end != NULL` — assert consumed offset | [ ] |
| 19 | `sodium_base642bin` | `ignore=" \n"` / `" \n\r"` with leading / interior / trailing whitespace | [ ] |
| 20 | `sodium_base642bin` | `"A"`,`"AA"`,`"AAAA"`,`"A==="` per variant (`'A'` is the only char mapping to 0) | [ ] |
| 21 | round-trip `bin2base64` → `base642bin` | all 4 variants × `bin_len` 0..64, `bin_maxlen` exact and oversized | [ ] |

## 3. `sodium/codecs.c` — IP

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 22 | `sodium_ip2bin` | IPv4 dotted quad, incl. leading zeros `"01.2.3.4"` → IPv4-mapped `00*10,ff,ff,octets` | [ ] |
| 23 | `sodium_ip2bin` | full 8-group IPv6, `"::"`, `"::1"`, collapsed `"2001:db8::1"` | [ ] |
| 24 | `sodium_ip2bin` | IPv4-mapped text `"::ffff:1.2.3.4"`; 6-group + embedded IPv4 `"1:2:3:4:5:6:1.2.3.4"` | [ ] |
| 25 | `sodium_ip2bin` | zone id `"fe80::1%eth0"` (zone parsed and DISCARDED) | [ ] |
| 26 | `sodium_ip2bin` | `ip_len_` longer than the NUL-terminated string (stops at NUL); `ip_len_` shorter (prefix parse) | [ ] |
| 27 | `sodium_bin2ip` | `ip_maxlen` 3..46; IPv4-mapped branch vs IPv6 branch | [ ] |
| 28 | `sodium_bin2ip` | longest-zero-run collapsing: run must be ≥2; ties keep the FIRST longest run | [ ] |
| 29 | round-trip `ip2bin` → `bin2ip` | canonicalisation is NOT identity (`"0:0:...:0"`→`"::"`, `"1:2:3:4:5:6:1.2.3.4"`→`"1:2:3:4:5:6:102:304"`) | [ ] |

## 4. `sodium/utils.c`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 30 | `sodium_pad` | `blocksize` ∈ {1,2,16,256} (power-of-two `&` path) × `unpadded_buflen` ∈ {0,1,bs-1,bs,bs+1,2bs} | [ ] |
| 31 | `sodium_pad` | `blocksize` ∈ {17,255} (non-power-of-two `%` path) × same shapes | [ ] |
| 32 | `sodium_pad` | `max_buflen` exactly `*padded_buflen_p` (accepted); `padded_buflen_p == NULL` (accepted) | [ ] |
| 33 | `sodium_unpad` | round-trip of rows 30–31; `padded_buflen` NOT a multiple of `blocksize` (e.g. 17/16) is legal | [ ] |
| 34 | `sodium_unpad` | barrier position 0..blocksize-1 from the end; lone `{0x80}` with `blocksize=1` → unpadded 0 | [ ] |
| 35 | `sodium_memcmp` | `len` ∈ {0,1,2,8,16,24,32,64}; equal, differ in byte 0, differ in byte len-1, aliased `b1==b2` | [ ] |
| 36 | `sodium_compare` | same `len` set; LITTLE-ENDIAN ordering (compares from index `len-1` down) | [ ] |
| 37 | `sodium_is_zero` | same `len` set; all-zero, all-0xff, single nonzero byte at each position | [ ] |
| 38 | `sodium_increment` | `len` ∈ {0,1,2,**8**,**12**,16,**24**,32,64} (8/12/24 are the AMD64-asm fast paths) × all-0xff wrap, partial carry | [ ] |
| 39 | `sodium_add` | same `len` set; carry out of the top byte; `a + b` random pairs | [ ] |
| 40 | `sodium_sub` | same `len` set incl. **64** (asm fast path); `0 - 1` borrow chain | [ ] |
| 41 | `sodium_malloc` / `sodium_free` | `size` ∈ {0,1,16,4095,4096,65535,65536,65537}; assert GARBAGE prefill and read/write of the whole region | [ ] |
| 42 | `sodium_allocarray` | `(count,size)` ∈ {(0,n),(n,0),(1,1),(1024,16)} | [ ] |
| 43 | `sodium_mprotect_*` lifecycle | malloc → write → readonly → noaccess → readwrite → free (all -1/ENOSYS in this build) | [ ] |
| 44 | `sodium_memzero` / `sodium_stackzero` | `len` ∈ {0,1,64,4096} | [ ] |

## 5. `sodium/core.c`, `runtime.c`, `version.c`, `randombytes/`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 45 | `sodium_init` | first call → 0, subsequent → 1; `sodium_runtime_has_*` (12 fns) before vs after init | [ ] |
| 46 | `sodium_version_string` / `_library_version_major` / `_minor` / `_library_minimal` | — (`"1.0.23"`, 30, 0, 0) | [ ] |
| 47 | `randombytes_buf_deterministic` | seed ∈ {all-0x00, all-0xff, 0x00..0x1f, ASCII, random} × `size` ∈ {0,1,31,32,33,63,64,65,1000,65536} — ChaCha20-IETF keystream, nonce `"LibsodiumDRG"` | [ ] |
| 48 | `randombytes_seedbytes` / `randombytes_implementation_name` | default impl (`"sysrandom"`) | [ ] |
| 49 | `randombytes_set_implementation` + `randombytes_random` / `_buf` | **[RNG]** injected counter impl, `uniform=NULL` | [ ] |
| 50 | `randombytes_uniform` | **[RNG]** injected impl, `uniform=NULL` ⇒ exercises libsodium's own rejection sampler; `upper_bound` ∈ {0,1,2,3,7,0x7fffffff,0x80000001,0xffffffff} | [ ] |
| 51 | `randombytes_uniform` | **[RNG]** injected impl WITH `uniform != NULL` ⇒ delegation path, incl. bound 0/1 | [ ] |
| 52 | `randombytes_stir` / `_close` | idempotency; `close` with `impl->close == NULL` | [ ] |
| 53 | `randombytes` (NaCl compat) + `randombytes_internal_implementation` | name `"internal"`, `uniform=NULL` | [ ] |

## 6. `crypto_verify` / `crypto_shorthash`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 54 | `crypto_verify_16` / `_32` / `_64` | equal buffers; differ at each byte position; all-0x00 vs all-0xff | [ ] |
| 55 | `crypto_shorthash_siphash24` | `inlen` ∈ {0,1,7,8,9,15,16,17,63,64,65,1000}; key all-0x00 / all-0xff / random | [ ] |
| 56 | `crypto_shorthash_siphashx24` | same shapes (16-byte output) | [ ] |
| 57 | `crypto_shorthash` (dispatch) + `_bytes`/`_keybytes`/`_primitive` | delegates to siphash24 | [ ] |

## 7. `crypto_hash` — sha256 / sha512 / sha3

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 58 | `crypto_hash_sha256` one-shot | `mlen` ∈ {0,1,55,**56**,63,**64**,65,111,112,119,120,127,128,191,192,1000} (pad boundary `r==56`) | [ ] |
| 59 | `crypto_hash_sha256_init/_update/_final` | single update; must equal row 58 | [ ] |
| 60 | `crypto_hash_sha256_*` | multi-update chunking: `inlen < 64-r` small path, exact-fill, `while (inlen >= 64)` loop, `inlen &= 63` tail; `inlen==0` no-op | [ ] |
| 61 | `crypto_hash_sha512` one-shot | `mlen` ∈ {0,1,111,**112**,119,120,127,**128**,129,239,240,255,256,1000} (pad boundary `r==112`) | [ ] |
| 62 | `crypto_hash_sha512_init/_update/_final` | single and multi-update chunking as row 60 (block 128) | [ ] |
| 63 | `crypto_hash` / `_bytes` / `_primitive` | dispatch to sha512 (`"sha512"`) | [ ] |
| 64 | `crypto_hash_sha3256` one-shot | rate **136**, `mlen` ∈ {0,1,**135**,**136**,137,143,144,271,272,1000} | [ ] |
| 65 | `crypto_hash_sha3512` one-shot | rate **72**, `mlen` ∈ {0,1,**71**,**72**,73,143,144,1000} | [ ] |
| 66 | `crypto_hash_sha3256_init/_update/_final` | `offset != 0` partial-fill branch; `while (inlen-consumed >= rate)` loop leaving `offset == rate`; trailing remainder | [ ] |
| 67 | `crypto_hash_sha3512_init/_update/_final` | same three branches at rate 72 | [ ] |

## 8. `crypto_xof` — shake128 / shake256 / turboshake128 / turboshake256

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 68 | `crypto_xof_shake128` one-shot | rate **168**; `outlen` ∈ {0,1,167,168,169,336,337,504,1000} × `inlen` ∈ {0,1,167,168,169,337} | [ ] |
| 69 | `crypto_xof_shake256` one-shot | rate **136**; `outlen`/`inlen` at {0,1,135,136,137,272,273,1000} | [ ] |
| 70 | `crypto_xof_turboshake128` one-shot | rate **168**, **12-round** permute; same shape set | [ ] |
| 71 | `crypto_xof_turboshake256` one-shot | rate **136**, **12-round** permute; same shape set | [ ] |
| 72 | `_init` / `_update` / `_squeeze` (all 4) | multi-squeeze: `squeeze(rate)`×2 == `squeeze(2*rate)`; `squeeze(1)`×N == `squeeze(N)` (resume mid-block) | [ ] |
| 73 | `_init` / `_update` / `_squeeze` (all 4) | absorb chunking straddling a rate boundary; multi-update splits | [ ] |
| 74 | `_init_with_domain` (all 4) | domain byte ∈ {0x00,0x01,0x06,0x07,0x1F,0x7F,**0x80**,0xFF} — no validation; 0x80 collides with the pad bit | [ ] |
| 75 | `_init_with_domain` (all 4) | padding-branch axis: `offset == rate-1` (single combined `domain^0x80`) vs `offset < rate-1` vs `offset == rate` | [ ] |
| 76 | `_blockbytes` / `_statebytes` / `_domain_standard` (all 4) | 168/136, 256, 0x1F | [ ] |

## 9. `crypto_core` — salsa / hsalsa20 / hchacha20 / keccak1600

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 77 | `crypto_core_salsa20` | `c == NULL` (sigma) vs explicit 16-byte constants; key/in all-0x00, all-0xff, random | [ ] |
| 78 | `crypto_core_salsa2012` (12 rounds) | same axes | [ ] |
| 79 | `crypto_core_salsa208` (8 rounds) | same axes | [ ] |
| 80 | `crypto_core_hsalsa20` | `c == NULL` vs explicit; out 32, in 16, k 32 | [ ] |
| 81 | `crypto_core_hchacha20` | `c == NULL` vs explicit; out 32, in 16, k 32 | [ ] |
| 82 | `crypto_core_keccak1600_init/_xor_bytes/_extract_bytes` | offset/length combos within 200 bytes; `_permute_24` vs `_permute_12` | [ ] |

## 10. `crypto_stream` — every cipher and every `_ic` entry point

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 83 | `crypto_stream_chacha20` (keystream) | 8-byte nonce; `clen` ∈ {0,1,31,32,63,**64**,65,127,128,129,191,192,255,256,512,513,1024,4096} | [ ] |
| 84 | `crypto_stream_chacha20_xor` | same lengths; in-place `c==m` and disjoint | [ ] |
| 85 | `crypto_stream_chacha20_xor_ic` | `ic` (u64) ∈ {0,1,**2^32-1**,**2^32**,2^32+1, 2^64-1} — exercises the `j12` wrap into `j13` | [ ] |
| 86 | `crypto_stream_chacha20_ietf` | 12-byte nonce; same length set | [ ] |
| 87 | `crypto_stream_chacha20_ietf_xor` | same lengths | [ ] |
| 88 | `crypto_stream_chacha20_ietf_xor_ic` | `ic` (u32) ∈ {0,1,2^32-2,2^32-1} and the guard boundary `ic == 2^32-ceil(mlen/64)` (last legal) | [ ] |
| 89 | `crypto_stream_chacha20_ietf_ext` / `_ext_xor_ic` | extended-counter low-level entry points; same lengths | [ ] |
| 90 | `crypto_stream_salsa20` / `_xor` | 8-byte nonce; same length set | [ ] |
| 91 | `crypto_stream_salsa20_xor_ic` | `ic` (u64) ∈ {0,1,2^32-1,2^32,2^64-2,2^64-1} — manual 8-byte carry chain, no wrap detection | [ ] |
| 92 | `crypto_stream_salsa2012` / `_xor` | 12 rounds; NO `_xor_ic` exists; same lengths | [ ] |
| 93 | `crypto_stream_salsa208` / `_xor` | 8 rounds; NO `_xor_ic`; same lengths | [ ] |
| 94 | `crypto_stream_xsalsa20` / `_xor` / `_xor_ic` | 24-byte nonce (HSalsa20 of `n[0..16)`); `ic` passed through | [ ] |
| 95 | `crypto_stream_xchacha20` / `_xor` / `_xor_ic` | 24-byte nonce (HChacha20 of `n[0..16)`) | [ ] |
| 96 | `crypto_stream` / `_xor` (dispatch) + `_primitive` | delegates to xsalsa20 (`"xsalsa20"`) | [ ] |
| 97 | all of 83–96 | key all-0x00 / all-0xff / random × nonce all-0x00 / all-0xff / random | [ ] |

## 11. `crypto_onetimeauth` / `crypto_auth`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 98 | `crypto_onetimeauth_poly1305` one-shot | `inlen` ∈ {0,1,15,**16**,17,31,**32**,33,63,64,1000} | [ ] |
| 99 | `crypto_onetimeauth_poly1305_init/_update/_final` | leftover-buffer axis (block_size **16** for donna): (a) `leftover==0 && bytes<16`; (b) top-up only, early return; (c) top-up exactly fills; (d) `bytes>=16` bulk; (e) nonzero remainder stored back | [ ] |
| 100 | `..._update` chunkings | 1+15, 15+1, 16 in one call, 16 in two calls, 17, 31+1, 32, 33, and a 0-length update | [ ] |
| 101 | `..._final` | `leftover == 0` vs `leftover` ∈ 1..15 (0x01 pad byte appended) | [ ] |
| 102 | `crypto_onetimeauth` / `_verify` (dispatch) + `_statebytes`/`_bytes`/`_keybytes`/`_primitive` | `"poly1305"` | [ ] |
| 103 | `crypto_auth_hmacsha256_init` | `keylen` ∈ {0 (key=NULL), 1, 32, **64** (== block), **65** (> block ⇒ key PRE-HASHED with `state->ictx`), 128} | [ ] |
| 104 | `crypto_auth_hmacsha256_*` | `_init/_update×N/_final` streaming, `inlen` incl. 0; vs one-shot | [ ] |
| 105 | `crypto_auth_hmacsha512_init` | `keylen` ∈ {0, 1, 32, **128** (== block), **129** (> block ⇒ pre-hashed), 200} | [ ] |
| 106 | `crypto_auth_hmacsha512_*` | streaming vs one-shot; 64-byte output | [ ] |
| 107 | `crypto_auth_hmacsha512256_*` | `_init/_update/_final`; state is a `hmacsha512_state`, `_final` truncates 64→32 | [ ] |
| 108 | `crypto_auth` / `_verify` (dispatch) + `_bytes`/`_keybytes`/`_primitive` | `"hmacsha512256"` | [ ] |
| 109 | all `*_verify` | correct tag; tag differing at each byte position | [ ] |
| 110 | all `*_keygen` | **[RNG]** injected impl | [ ] |

## 12. `crypto_generichash` / blake2b

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 111 | `crypto_generichash_blake2b` one-shot | `outlen` ∈ {**1**,**15**,16,32,63,64} × `keylen` = 0 (unkeyed path) | [ ] |
| 112 | `crypto_generichash_blake2b` one-shot | `outlen` ∈ {1,16,32,64} × `keylen` ∈ {**1**,**15**,16,32,64} (keyed path absorbs one zero-padded 128-byte block first) | [ ] |
| 113 | `crypto_generichash_blake2b` | `key == NULL` with `keylen > 0` ⇒ wrapper takes the UNKEYED path (no misuse) | [ ] |
| 114 | `crypto_generichash_blake2b_salt_personal` | 16-byte salt × 16-byte personal; `salt==NULL` and/or `personal==NULL` (zero-filled); distinct salt/personal must change the digest | [ ] |
| 115 | `crypto_generichash_blake2b_init/_update/_final` | unkeyed; `inlen` ∈ {0,1,**127**,**128**,129,**255**,**256**,257,383,384} — branch is `inlen > 256-buflen` | [ ] |
| 116 | `crypto_generichash_blake2b_init/_update/_final` | keyed (initial `buflen == 128`) × same `inlen` set | [ ] |
| 117 | `..._update` multi-chunk | 1+1, 63+65, 127+1, 128+128, 1×256, 256×1, and a final chunk landing exactly on `buflen == 128` (`_final`'s extra-compress fires only when `buflen > 128`) | [ ] |
| 118 | `crypto_generichash_blake2b_init_salt_personal` + `_update` + `_final` | salt/personal × keyed/unkeyed × chunking | [ ] |
| 119 | `crypto_generichash_blake2b_final` | `outlen` equal to init's (normal) and NOT equal (silently allowed, truncated digest) | [ ] |
| 120 | `crypto_generichash` / `_init` / `_update` / `_final` (dispatch) + `_statebytes` (384) / `_bytes*` / `_keybytes*` / `_primitive` | `"blake2b"` | [ ] |
| 121 | `crypto_generichash_keygen` / `_blake2b_keygen` | **[RNG]** | [ ] |

## 13. `crypto_scalarmult` — curve25519 / ed25519 / ristretto255

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 122 | `crypto_scalarmult_curve25519_base` | random 32-byte scalars; all-0x00, all-0xff, clamped forms (no rejection branch exists) | [ ] |
| 123 | `crypto_scalarmult_curve25519` | valid pk from `_base` × random scalars; ECDH agreement property | [ ] |
| 124 | `crypto_scalarmult` / `_base` (dispatch) + `_bytes`/`_scalarbytes`/`_primitive` | `"curve25519"` | [ ] |
| 125 | `crypto_scalarmult_ed25519` (**clamp**) | valid main-subgroup point × scalars {1,2,8,L-1,random} | [ ] |
| 126 | `crypto_scalarmult_ed25519_noclamp` | same point set × scalars {1,2,8,L-1,random}; clamp vs noclamp must DIFFER | [ ] |
| 127 | `crypto_scalarmult_ed25519_base` (clamp) | scalars {1,2,8,L-1,random} | [ ] |
| 128 | `crypto_scalarmult_ed25519_base_noclamp` | same scalar set | [ ] |
| 129 | `crypto_scalarmult_ristretto255` | valid canonical ristretto encodings × random scalars (always masks `t[31] &= 127`, NO clamping) | [ ] |
| 130 | `crypto_scalarmult_ristretto255_base` | random scalars | [ ] |

## 14. `crypto_core_ed25519` / `crypto_core_ristretto255`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 131 | `crypto_core_ed25519_is_valid_point` | generator B; identity; the 8 small-order points; main-subgroup point; torsion point; non-canonical y (p, p+1, 2^255-1); sign bit 0 and 1 | [ ] |
| 132 | `crypto_core_ed25519_add` / `_sub` | valid × valid; and the NON-canonical / small-order / torsion points that these ACCEPT (only `frombytes`+`is_on_curve` gate them) | [ ] |
| 133 | `crypto_core_ed25519_from_uniform` | 32-byte inputs: random, all-0x00, all-0xff; `x_sign` = bit 7 of `r[31]` | [ ] |
| 134 | `crypto_core_ed25519_from_hash` | 64-byte inputs: random, all-0x00, all-0xff (`fe25519_reduce64` folds dropped bits ×19 / ×722) | [ ] |
| 135 | `crypto_core_ed25519_random` | **[RNG]** | [ ] |
| 136 | `crypto_core_ed25519_scalar_random` | **[RNG]** — do/while until canonical and nonzero, `r[31] &= 0x1f` each iteration | [ ] |
| 137 | `crypto_core_ed25519_scalar_reduce` | 64-byte non-reduced: 0, 1, L, 2L, 2^512-1, random | [ ] |
| 138 | `crypto_core_ed25519_scalar_negate` / `_complement` | scalars 0, 1, L-1, random (build a 64-byte `t_ = L<<256 [+1]` then `sodium_sub` then reduce) | [ ] |
| 139 | `crypto_core_ed25519_scalar_add` / `_sub` / `_mul` | random canonical AND non-canonical scalars (`_mul` does NOT check canonicality) | [ ] |
| 140 | `crypto_core_ed25519_scalar_invert` | random nonzero scalars; assert `invert(s)*s == 1 mod L`, `invert(1)==1` | [ ] |
| 141 | `crypto_core_ed25519_scalar_is_canonical` | 0, 1, L-1, L, L+1, 2^252, 2^255-1, 2^256-1 | [ ] |
| 142 | `crypto_core_ed25519_from_string` (2 points, `h_len` 96) | `hash_alg` **1** (SHA-256) and **2** (SHA-512) × `ctx_len` ∈ {0, 1, 255, **256** (> 0xff ⇒ `H2C-OVERSIZE-DST-` rehash path)} × `msg_len` ∈ {0,1,64,128,1000} | [ ] |
| 143 | `crypto_core_ed25519_from_string_nu` (1 point, `h_len` 48) | same cross-product | [ ] |
| 144 | `crypto_core_ed25519_scalar_from_string` | same cross-product | [ ] |
| 145 | `crypto_core_ristretto255_is_valid_point` | identity (32 zero bytes); valid canonical even encodings; `s` odd; `s ≥ p`; bit 255 set; non-square | [ ] |
| 146 | `crypto_core_ristretto255_add` / `_sub` | valid × valid; group-law properties | [ ] |
| 147 | `crypto_core_ristretto255_from_hash` | 64-byte inputs: random, all-0x00, all-0xff (no rejection) | [ ] |
| 148 | `crypto_core_ristretto255_random` / `_scalar_random` | **[RNG]** | [ ] |
| 149 | `crypto_core_ristretto255_scalar_*` (`negate`,`complement`,`add`,`sub`,`mul`,`reduce`,`invert`,`is_canonical`) | same scalar sets as rows 137–141 | [ ] |
| 150 | `crypto_core_ristretto255_from_string` / `_from_string_nu` / `_scalar_from_string` | `hash_alg` 1 and 2 × `ctx_len` {0,1,255,256} × `msg_len` {0,1,64,1000} | [ ] |
| 151 | all `crypto_core_*_bytes` / `_uniformbytes` / `_hashbytes` / `_scalarbytes` / `_nonreducedscalarbytes` | constant getters | [ ] |

## 15. `crypto_sign` / ed25519

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 152 | `crypto_sign_ed25519_seed_keypair` | seeds all-0x00, all-0xff, RFC 8032 vectors, random; `sk = seed \|\| pk` | [ ] |
| 153 | `crypto_sign_ed25519_keypair` | **[RNG]** | [ ] |
| 154 | `crypto_sign_ed25519_detached` + `_verify_detached` | `mlen` ∈ {0,1,31,32,63,**64**,65,127,128,1000} (sha512 absorbs `sig[0..32)+pk` = 64 bytes first) | [ ] |
| 155 | `crypto_sign_ed25519` + `_open` (combined, `sm = sig\|\|m`) | same `mlen` set; `m == NULL` on open | [ ] |
| 156 | `crypto_sign` / `_open` / `_detached` / `_verify_detached` (dispatch) + `_bytes`/`_seedbytes`/`_publickeybytes`/`_secretkeybytes`/`_messagebytes_max`/`_primitive` | `"ed25519"` | [ ] |
| 157 | `crypto_sign_ed25519ph_init` / `_update` / `_final_create` / `_final_verify` | update chunking {0,1,127,128,129} × {1 update, N updates, ZERO updates}; DOM2PREFIX (34 bytes) prepended | [ ] |
| 158 | `crypto_sign_init` / `_update` / `_final_create` / `_final_verify` (dispatch) + `_statebytes` | delegates to ed25519ph | [ ] |
| 159 | `crypto_sign_ed25519_sk_to_seed` / `_sk_to_pk` | valid keypairs (`memmove` of `sk[0..32)` / `sk[32..64)`) | [ ] |
| 160 | `crypto_sign_ed25519_sk_to_curve25519` | valid keypairs (hash `sk[0..32)` then clamp) | [ ] |
| 161 | `crypto_sign_ed25519_pk_to_curve25519` | valid pk; and a NON-CANONICAL pk (ACCEPTED — no `is_canonical` call) | [ ] |
| 162 | malleability axis | `S` canonical (accept); `S ≥ L` with `sig[63]&0xF0 == 0` (**ACCEPTED** — guard needs the high nibble nonzero); `S ≥ L` with `sig[63]&0xF0 != 0` (reject) | [ ] |
| 163 | cofactored-verification axis | a signature differing by a torsion component still verifies (`check` need only be small-order) | [ ] |

## 16. `crypto_aead` — chacha20poly1305 family

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 164 | `crypto_aead_chacha20poly1305_encrypt` / `_decrypt` (8-byte nonce, **pre-IETF** poly1305 layout: NO 16-byte padding, lengths interleaved) | `mlen` ∈ {0,1,63,**64**,65,1000} × `adlen` ∈ {0,1,16,17,64} | [ ] |
| 165 | `crypto_aead_chacha20poly1305_encrypt_detached` / `_decrypt_detached` | same cross-product; separate MAC buffer | [ ] |
| 166 | `crypto_aead_chacha20poly1305_*` | `mlen` = **131072** (`STREAM_POLY1305_CHUNK`) and **131073** (chunk+1, exercises the 64-bit `ic` carry; decrypt does a SINGLE `xor_ic`) | [ ] |
| 167 | `crypto_aead_chacha20poly1305_ietf_encrypt` / `_decrypt` (12-byte nonce, **RFC 8439** layout with `_pad0`) | `mlen` ∈ {0,1,**15**,**16**,17,63,64,65,1000} × `adlen` ∈ {0,1,**15**,**16**,17} (pad is 0 exactly at %16==0) | [ ] |
| 168 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` / `_decrypt_detached` | same cross-product | [ ] |
| 169 | `crypto_aead_chacha20poly1305_ietf_*` | `mlen` = 131072 / 131073 (u32 `ic`) | [ ] |
| 170 | `crypto_aead_xchacha20poly1305_ietf_encrypt` / `_decrypt` (24-byte nonce, HChacha20 sub-key, `npub2 = 4 zero bytes \|\| npub[16..24)`) | `mlen` × `adlen` as row 167 | [ ] |
| 171 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` / `_decrypt_detached` | same cross-product | [ ] |
| 172 | all rows 164–171 | `ad == NULL` with `adlen == 0`; `nsec != NULL`; in-place `c == m`; `*clen_p`/`*maclen_p`/`*mlen_p` NULL | [ ] |
| 173 | `*_decrypt_detached` with `m == NULL` | verify-only mode (distinct code path) for all 4 variants | [ ] |
| 174 | `*_keygen` (all 4) + `_keybytes`/`_nsecbytes`/`_npubbytes`/`_abytes`/`_messagebytes_max` | **[RNG]** + constant getters, incl. the uppercase `_IETF_*` aliases | [ ] |

## 17. `crypto_aead` — AEGIS (soft impl) and AES256GCM (unavailable)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 175 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | RATE **32**; `mlen` ∈ {0,1,31,**32**,33,**64**,65,1000} × `adlen` ∈ {0,1,31,**32**,33,63,**64**,65,127,128} (absorb vs absorb2 `2*RATE` fast path) | [ ] |
| 176 | `crypto_aead_aegis128l_encrypt_detached` / `_decrypt_detached` | same cross-product; `m == NULL` scratch-decrypt path | [ ] |
| 177 | `crypto_aead_aegis256_encrypt` / `_decrypt` | RATE **16**; `mlen` ∈ {0,1,15,**16**,17,**32**,33,1000} × `adlen` ∈ {0,1,15,**16**,17,31,**32**,33} | [ ] |
| 178 | `crypto_aead_aegis256_encrypt_detached` / `_decrypt_detached` | same cross-product; `m == NULL` path | [ ] |
| 179 | `_crypto_aead_aegis128l_pick_best_implementation` / `_aegis256_...` | selects the `*_soft` impl in this build | [ ] |
| 180 | AEGIS `*_keygen` + all constant getters | `_keybytes` 16/32, `_npubbytes` 16/32, `_abytes` 32, `_nsecbytes` 0 | [ ] |
| 181 | `crypto_aead_aes256gcm_is_available` + all 9 stubs | **0** / -1+ENOSYS for every entry point (see ERRORS 106–115); `_statebytes` still returns a 16-aligned size | [ ] |

## 18. `crypto_secretbox` (both primitives, incl. the deprecated NaCl API)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 182 | `crypto_secretbox_easy` / `_open_easy` | `mlen` ∈ {0,1,31,**32**,33,64,1000} (block0 holds `min(mlen,32)` bytes after 32 zeros) | [ ] |
| 183 | `crypto_secretbox_detached` / `_open_detached` | same `mlen` set, DISJOINT buffers | [ ] |
| 184 | `crypto_secretbox_detached` / `_open_detached` | **overlap axis**: in-place `c == m`; partially overlapping FORWARD; partially overlapping BACKWARD (explicit `uintptr_t` detection + `memmove`) — 4 distinct paths | [ ] |
| 185 | `crypto_secretbox_detached` | `mlen` = 131072 / 131073 (`STREAM_POLY1305_CHUNK`, 64-bit `ic`) | [ ] |
| 186 | `crypto_secretbox` / `_open` (deprecated NaCl, requires 32 leading zeros in `m`, writes 16 leading zeros in `c`) | `mlen` ∈ {32,33,64,96,1000} | [ ] |
| 187 | `crypto_secretbox_xsalsa20poly1305` / `_open` (primitive level) | same as row 186 | [ ] |
| 188 | `crypto_secretbox_open_detached` with `m == NULL` | verify-only mode | [ ] |
| 189 | `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy` | `mlen` set of row 182; `_detached` calls `chacha20_xor` over exactly `mlen0+32`, then ONE `xor_ic(1)`; poly1305 over the full ct (no chunking) | [ ] |
| 190 | `crypto_secretbox_xchacha20poly1305_detached` / `_open_detached` | disjoint + all 3 overlap shapes; `m == NULL` | [ ] |
| 191 | `crypto_secretbox*_keygen` + `_keybytes`/`_noncebytes`/`_macbytes`/`_zerobytes`/`_boxzerobytes`/`_messagebytes_max`/`_primitive` | **[RNG]** + constants (`"xsalsa20poly1305"`) | [ ] |

## 19. `crypto_secretstream/xchacha20poly1305`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 192 | `_init_push` / `_push` / `_init_pull` / `_pull` | TAG_MESSAGE (0x00) only; `mlen` ∈ {0 (out = 17 bytes),1,15,16,**48** (makes `(0x10-64+mlen)&0xf == 0`),63,64,65} × `adlen` ∈ {0,1,15,16,17} | [ ] |
| 193 | same | TAG_PUSH (0x01) — bit 0x02 clear ⇒ NO rekey | [ ] |
| 194 | same | TAG_REKEY (0x02) — triggers an automatic rekey | [ ] |
| 195 | same | TAG_FINAL (0x03 = PUSH\|REKEY) — also rekeys | [ ] |
| 196 | same | ARBITRARY tag bytes {0x04, 0x42, 0x7f, 0xff}: accepted; any with bit 0x02 set rekeys | [ ] |
| 197 | `_rekey` | explicit symmetric rekey mid-stream on both sides | [ ] |
| 198 | multi-message sequence | 8-message stream mixing all 4 tags; MAC-chaining (`STATE_INONCE ^= tag[0..8)`) makes ordering part of the state | [ ] |
| 199 | poly1305-layout quirk | the ciphertext pad is `(0x10 - 64 + mlen) & 0xf` (the source documents this as a deviation from the intended `(0x10 - (64+mlen)) & 0xf`) — wire-format visible | [ ] |
| 200 | `_pull` out-params | `mlen_p == NULL` and/or `tag_p == NULL` | [ ] |
| 201 | `_keygen` + `_statebytes`/`_abytes`(17)/`_headerbytes`(24)/`_keybytes`/`_messagebytes_max`/`_tag_message`/`_tag_push`/`_tag_rekey`/`_tag_final` | **[RNG]** + constants | [ ] |

## 20. `crypto_box` (both primitives — every API shape)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 202 | `crypto_box_keypair` / `_seed_keypair` | **[RNG]** vs deterministic (`sk = SHA-512(seed)[0..32)`) | [ ] |
| 203 | `crypto_box_easy` / `_open_easy` | `mlen` ∈ {0,1,31,32,33,64,1000} | [ ] |
| 204 | `crypto_box_detached` / `_open_detached` | same `mlen` set; separate MAC | [ ] |
| 205 | `crypto_box_beforenm` + `_easy_afternm` / `_open_easy_afternm` | precomputed `k` reused; `hsalsa20(scalarmult(sk,pk), zero[16])` | [ ] |
| 206 | `crypto_box_beforenm` + `_detached_afternm` / `_open_detached_afternm` | aliases of `crypto_secretbox_detached` ⇒ inherits the 4-way overlap axis and `mlen0=min(mlen,32)` | [ ] |
| 207 | `crypto_box_afternm` / `_open_afternm` (deprecated padded) | `mlen ≥ 32` | [ ] |
| 208 | `crypto_box` / `_open` (deprecated NaCl padded) | `mlen ≥ 32` | [ ] |
| 209 | `crypto_box_curve25519xsalsa20poly1305` / `_open` (primitive level) | `mlen ≥ 32` | [ ] |
| 210 | `crypto_box_seal` / `_seal_open` | anonymous sender; ephemeral pk prepended; nonce = BLAKE2b-24(`epk\|\|pk`); `mlen` ∈ {0,1,64,1000}, `clen` = mlen+48 | [ ] |
| 211 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` / `_keypair` | deterministic + **[RNG]** | [ ] |
| 212 | `crypto_box_curve25519xchacha20poly1305_easy` / `_open_easy` / `_detached` / `_open_detached` | `mlen` set of row 203; `_beforenm` uses **HChacha20** (not hsalsa20) | [ ] |
| 213 | `crypto_box_curve25519xchacha20poly1305_beforenm` + `_easy_afternm` / `_open_easy_afternm` / `_detached_afternm` / `_open_detached_afternm` | NOTE: this primitive has NO plain `_afternm`/`_open_afternm` and no padded NaCl API | [ ] |
| 214 | `crypto_box_curve25519xchacha20poly1305_seal` / `_seal_open` | `mlen` ∈ {0,1,64,1000} | [ ] |
| 215 | ECDH agreement | A→B and B→A produce the same `beforenm` key, for both primitives | [ ] |
| 216 | all constant getters | `_seedbytes`/`_publickeybytes`/`_secretkeybytes`/`_beforenmbytes`/`_noncebytes`/`_macbytes`/`_zerobytes`/`_boxzerobytes`/`_sealbytes`(48)/`_messagebytes_max`/`_primitive` | [ ] |

## 21. `crypto_kdf`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 217 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len` ∈ {**16**,17,32,63,**64**} × `subkey_id` ∈ {0,1,0xFF,2^56,**u64::MAX**} × ctx all-zero / ASCII / embedded NULs | [ ] |
| 218 | `crypto_kdf_derive_from_key` (dispatch) + `_keygen` + `_bytes_min`/`_bytes_max`/`_contextbytes`/`_keybytes`/`_primitive` | `"blake2b"`, 16/64/8/32 | [ ] |
| 219 | `crypto_kdf_hkdf_sha256_extract` (one-shot) | `salt_len` ∈ {0,1,32,**64** (== block),65} × `ikm_len` ∈ {0,1,32,64,65,200} | [ ] |
| 220 | `crypto_kdf_hkdf_sha256_extract_init` / `_extract_update`×N / `_extract_final` | streaming; must equal row 219; `_extract_final` zeroes the state | [ ] |
| 221 | `crypto_kdf_hkdf_sha256_expand` | `out_len` ∈ {**0**,1,31,**32**,33,64,**8160** (=255 blocks, counter reaches 255)} × `ctx_len` ∈ {0,1,32} | [ ] |
| 222 | `crypto_kdf_hkdf_sha512_extract` (one-shot) | `salt_len` ∈ {0,1,64,**128**,129} × `ikm_len` ∈ {0,1,64,128,129,300} | [ ] |
| 223 | `crypto_kdf_hkdf_sha512_extract_init` / `_extract_update`×N / `_extract_final` | streaming; must equal row 222 | [ ] |
| 224 | `crypto_kdf_hkdf_sha512_expand` | `out_len` ∈ {0,1,63,**64**,65,128,**16320**} × `ctx_len` ∈ {0,1,64} | [ ] |
| 225 | `crypto_kdf_hkdf_sha256_keygen` / `_sha512_keygen` + `_statebytes`/`_bytes_min`/`_bytes_max`/`_keybytes` | **[RNG]** + constants (8160 / 16320) | [ ] |

## 22. `crypto_kem` — mlkem768 / xwing

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 226 | `crypto_kem_mlkem768_seed_keypair` | deterministic 64-byte seeds: all-0x00, all-0xff, counter, random×32 (`seed[32] = K = 3`; `seed[32..64)` copied verbatim as `z`) | [ ] |
| 227 | `crypto_kem_mlkem768_keypair` | **[RNG]** | [ ] |
| 228 | `crypto_kem_mlkem768_enc_deterministic` | valid pk × 32-byte seeds (deterministic KAT path); pk canonicality validated | [ ] |
| 229 | `crypto_kem_mlkem768_enc` | **[RNG]** | [ ] |
| 230 | `crypto_kem_mlkem768_dec` | VALID ct (`fail_mask=0` ⇒ true shared secret) — assert `ss_enc == ss_dec` | [ ] |
| 231 | `crypto_kem_mlkem768_dec` | TAMPERED ct at each of several byte positions (`fail_mask=0xFF` ⇒ implicit rejection `SHAKE256(z‖ct)`); still returns 0 | [ ] |
| 232 | rejection-sampling axis | many seeds so that `rej_uniform`'s `while (ctr < 256)` refill arm fires (seed-dependent extra squeeze); `transposed=0` (keypair) vs `1` (enc) | [ ] |
| 233 | `crypto_kem_xwing_seed_keypair` | deterministic 32-byte seeds; `expand_decaps_key`: `SHAKE256(seed,32)`→96 bytes | [ ] |
| 234 | `crypto_kem_xwing_keypair` | **[RNG]** | [ ] |
| 235 | `crypto_kem_xwing_enc_deterministic` | valid pk × 64-byte seeds (`[0..32)` = ML-KEM m, `[32..64)` = X25519 scalar) | [ ] |
| 236 | `crypto_kem_xwing_enc` / `_dec` | full round-trip; combiner `SHA3-256(ss_ml‖ss_x‖ct_x‖pk_x‖label)`, label `{5c,2e,2f,2f,5e,5c}` | [ ] |
| 237 | `crypto_kem_keypair` / `_seed_keypair` / `_enc` / `_dec` (dispatch) + `_publickeybytes`(1216)/`_secretkeybytes`(32)/`_ciphertextbytes`(1120)/`_sharedsecretbytes`(32)/`_seedbytes`(32)/`_primitive` | `"xwing"` | [ ] |
| 238 | `crypto_kem_mlkem768_*bytes` | 1184 / 2400 / 1088 / 32 / 64 | [ ] |

## 23. `crypto_kx`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 239 | `crypto_kx_seed_keypair` | deterministic seeds (`sk = BLAKE2b-32(seed)`) then X25519 base | [ ] |
| 240 | `crypto_kx_keypair` | **[RNG]** | [ ] |
| 241 | `crypto_kx_client_session_keys` + `_server_session_keys` | full handshake; assert `client.tx == server.rx` and `client.rx == server.tx` | [ ] |
| 242 | `crypto_kx_client_session_keys` | `rx == NULL` (tx-only) and `tx == NULL` (rx-only) — VALID shapes, args aliased | [ ] |
| 243 | `crypto_kx_server_session_keys` | `rx == NULL` and `tx == NULL` | [ ] |
| 244 | constant getters | `_publickeybytes`/`_secretkeybytes`/`_seedbytes`/`_sessionkeybytes` (all 32) / `_primitive` (`"x25519blake2b"`) | [ ] |

## 24. `crypto_pwhash` — argon2i / argon2id

Kept small (`opslimit`/`memlimit` at MIN) so each row runs fast.

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 245 | `crypto_pwhash` with `alg = ALG_ARGON2I13` (1) | `opslimit` = **3** (MIN) × `memlimit` = **8192** (MIN) × `outlen` ∈ {16,32,64} × `passwdlen` ∈ {0,1,16,64} | [ ] |
| 246 | `crypto_pwhash` with `alg = ALG_ARGON2ID13` (2) | `opslimit` = **1** (MIN) × `memlimit` = 8192 × same outlen/passwdlen sets (argon2id is data-INdependent only while `pass==0 && slice<2`) | [ ] |
| 247 | `crypto_pwhash_argon2i` (primitive level) | `t_cost` ∈ {3,4} × `m_cost` ∈ {8192,16384,32768} — `m_cost` is silently rounded DOWN to a multiple of `4*lanes` | [ ] |
| 248 | `crypto_pwhash_argon2id` (primitive level) | `t_cost` ∈ {**1** (single pass), **3** (multi-pass ⇒ data-dependent from pass 1 on)} × `m_cost` ∈ {8192,16384} | [ ] |
| 249 | `crypto_pwhash` | `outlen` = **65** and **128** ⇒ `blake2b_long` CHUNKED mode (`outlen > 64`: first 32 bytes, then a 32-byte loop, then a short final block) | [ ] |
| 250 | `crypto_pwhash_argon2i_str` / `_str_verify` round-trip | `opslimit` 3, `memlimit` 8192; correct and wrong password | [ ] |
| 251 | `crypto_pwhash_argon2id_str` / `_str_verify` round-trip | `opslimit` 1, `memlimit` 8192; correct and wrong password | [ ] |
| 252 | `crypto_pwhash_str` / `_str_alg` / `_str_verify` (dispatch) | `alg` 1 and 2; `STRPREFIX` `"$argon2id$"` | [ ] |
| 253 | `crypto_pwhash_str_needs_rehash` / `_argon2i_str_needs_rehash` / `_argon2id_str_needs_rehash` | matching params → 0; differing `opslimit` → 1; differing `memlimit` → 1 | [ ] |
| 254 | encode/decode round-trip (via `_str` → `_str_verify`) | type i vs id × 1-digit vs 10-digit `m`/`t`/`p` decimals × base64 `ORIGINAL_NO_PADDING` salt/hash | [ ] |
| 255 | all `crypto_pwhash*` constant getters | `_alg_argon2i13`(1)/`_alg_argon2id13`(2)/`_alg_default`(2)/`_bytes_min`(16)/`_bytes_max`/`_passwd_min`/`_passwd_max`/`_saltbytes`(16)/`_strbytes`(128)/`_strprefix`/`_opslimit_*`/`_memlimit_*`/`_primitive` for the dispatch + both primitives | [ ] |

## 25. `crypto_pwhash_scryptsalsa208sha256`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 256 | `crypto_pwhash_scryptsalsa208sha256_ll` (**lowest-level entry point**) | `N` ∈ {2,4,16,1024} × `r` ∈ {1,8} × `p` = **1** × `buflen` ∈ {**32** (exact PBKDF2 block), 33, 64, 80} × `saltlen` ∈ {0,32} (`_ll` does NOT enforce SALTBYTES) | [ ] |
| 257 | `crypto_pwhash_scryptsalsa208sha256_ll` | `p` ∈ {2,3,4} (multi-block `for (i=0;i<p;i++) smix(...)` path) × `N` ∈ {2,16} × `r` ∈ {1,8} | [ ] |
| 258 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit` = **32768** (MIN) × `memlimit` = **16777216** (MIN) — `pickparams` branch A (`opslimit < memlimit/32` ⇒ p=1) × `outlen` ∈ {16,32,64} | [ ] |
| 259 | `crypto_pwhash_scryptsalsa208sha256` | `pickparams` branch B (`opslimit >= memlimit/32` ⇒ p may be > 1): `opslimit` 1048576 × `memlimit` 16777216 | [ ] |
| 260 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit` BELOW MIN (e.g. 1) — silently CLAMPED UP to 32768, NOT rejected | [ ] |
| 261 | `crypto_pwhash_scryptsalsa208sha256_str` / `_str_verify` | round-trip; setting = `"$7$"` + 1 char N_log2 + 5 r + 5 p + 43 salt = 57 chars, total 101+NUL; correct and wrong password | [ ] |
| 262 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | matching → 0; differing N/r/p → 1 | [ ] |
| 263 | `escrypt_parse_setting` / `escrypt_gensalt_r` (via `_str`) | itoa64 alphabet `"./0-9A-Za-z"`; salt terminated by `strrchr(salt,'$')` vs end-of-string | [ ] |
| 264 | constant getters | `_bytes_min`(16)/`_bytes_max`/`_passwd_min`/`_passwd_max`/`_saltbytes`(32)/`_strbytes`(102)/`_strprefix`(`"$7$"`)/`_opslimit_min`(32768)/`_opslimit_max`/`_memlimit_min`(16777216)/`_memlimit_max`/`_opslimit_interactive`/`_memlimit_interactive`/`_opslimit_sensitive`/`_memlimit_sensitive` (**no MODERATE preset**) | [ ] |

## 26. `crypto_ipcrypt`

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 265 | `crypto_ipcrypt_encrypt` / `_decrypt` (deterministic, 16B in/out, key 16) | key all-0x00 / all-0xff / random × IP all-0x00 / all-0xff / IPv4-mapped / native IPv6 | [ ] |
| 266 | `crypto_ipcrypt_nd_encrypt` / `_nd_decrypt` (in 16, tweak **8**, out **24** = tweak\|\|ct) | tweak all-0x00 / all-0xff / counter × same key/IP sets; `_nd_decrypt` reads the tweak from `in[0..8)` | [ ] |
| 267 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` (in 16, tweak **16**, out **32**, key **32**) | key with DISTINCT halves × same tweak/IP sets | [ ] |
| 268 | `crypto_ipcrypt_ndx_*` | key with **IDENTICAL** 16-byte halves ⇒ triggers the `k[i]^0x5a` re-derivation of the 2nd schedule | [ ] |
| 269 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` (prefix-preserving, in/out 16, key 32) | IPv4-mapped input (`bin[10..12)==0xffff` ⇒ starts at prefix bit **96**, forces `out[10..12)=0xffff`) vs native IPv6 (starts at bit **0**) | [ ] |
| 270 | `crypto_ipcrypt_pfx_*` | identical key halves (same `^0x5a` re-derivation); prefix-preservation property: two IPs sharing an n-bit prefix ⇒ ciphertexts share an n-bit prefix | [ ] |
| 271 | all 4 `*_keygen` | **[RNG]** | [ ] |
| 272 | `_crypto_ipcrypt_pick_best_implementation` + all 12 constant getters | soft impl; 16,16,16,8,16,24,32,16,16,32,32,16 | [ ] |
| 273 | integration | `sodium_ip2bin("192.0.2.1")` / `("2001:db8::1")` → ipcrypt → `sodium_bin2ip` round-trip | [ ] |

## 27. Cross-module integration

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 274 | sign → box → secretbox pipeline | `crypto_sign_ed25519_sk_to_curve25519` + `_pk_to_curve25519` feeding `crypto_box_easy`, then `crypto_secretbox_easy` with the derived key | [ ] |
| 275 | kx → secretstream pipeline | `crypto_kx_*_session_keys` → `crypto_secretstream_*_init_push` → 8 pushes → pulls | [ ] |
| 276 | kem → kdf → aead pipeline | `crypto_kem_xwing_enc` → `crypto_kdf_hkdf_sha256_extract`/`_expand` → `crypto_aead_xchacha20poly1305_ietf_encrypt` | [ ] |
| 277 | pwhash → secretbox pipeline | `crypto_pwhash` (argon2id, MIN params) → `crypto_secretbox_easy` | [ ] |
| 278 | `sodium_init` ordering | every row above run BOTH before and after `sodium_init()` (the `*_pick_best_implementation` static initializers are already valid pre-init) | [ ] |
