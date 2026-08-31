## Area 6 — crypto_aead / secretbox / secretstream

Scope: `c_src/libsodium/crypto_aead/{aegis128l,aegis256,aes256gcm,chacha20poly1305,xchacha20poly1305}`,
`c_src/libsodium/crypto_secretbox/**`, `c_src/libsodium/crypto_secretstream/xchacha20poly1305/**`
plus the matching public headers.

### Named sweeps used below

* **`MLEN`** = `{0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129}` — the mandatory
  short-message shape sweep. Every row that says "`mlen ∈ MLEN`" is one sub-case per element.
* **`ADLEN`** = `{ad = NULL & adlen = 0, ad = non-NULL & adlen = 0, 1, 15, 16, 17, 31, 32, 33}`
  — 9 sub-cases. The two `adlen == 0` variants are distinguished deliberately: `ad == NULL` with
  `adlen == 0` reaches `crypto_onetimeauth_poly1305_update(&state, NULL, 0)` /
  `memcpy(src, NULL, 0)`, which C technically leaves undefined but libsodium relies on.
* **`BIG_AEGIS128L`** = `{224, 255, 256, 257, 511, 512, 513, 1024, 4096}` — aegis128l `RATE == 32`
  (`aegis128l_common.h:1`) and the absorb loop consumes `RATE*2 == 64` at a time
  (`aegis128l_soft.c:172-182`), so multiples/off-by-ones of 32 and 64 exercise
  `absorb2` / `absorb` / the `% RATE` tail and `declast`.
* **`BIG_AEGIS256`** = `{112, 127, 128, 129, 255, 256, 257, 1024, 4096}` — aegis256 `RATE == 16`
  (`aegis256_common.h:1`), absorb2 consumes `2*RATE == 32`.
* **`BIG_CHACHA`** = `{4096, 65536, 131071, 131072, 131073, 262144, 262145}` — crosses the
  64-byte ChaCha20 block and the `STREAM_POLY1305_CHUNK == 131072` re-entry boundary
  (`aead_chacha20poly1305.c:20,52-61`), which is where the `ic` counter arithmetic
  (`ic += cl / 64U`) can go wrong.
* **`BIG_SECRETBOX`** = `{32, 33, 63, 64, 65, 4096, 131072, 131073, 262145}` — crosses the
  `64 - ZEROBYTES == 32` first-block special case (`crypto_secretbox_easy.c:50-52`) and the
  `STREAM_POLY1305_CHUNK` boundary (`crypto_secretbox_easy.c:71-82`).

### Build-configuration constants that fix some axes

* No `HAVE_*` macros ⇒ aegis128l/aegis256 always run the portable `*_soft.c` implementation;
  the aes256gcm **stub** family links, `crypto_aead_aes256gcm_is_available()` returns `0`, and
  all nine other aes256gcm entry points return `-1` with `errno = ENOSYS`. The aes256gcm rows
  below therefore have no positive/round-trip configurations at all — they are all
  "must return -1 regardless of shape" rows (cross-referenced to `errors_6.md` 6.30–6.39).
* `NSECBYTES == 0` for all six AEADs ⇒ **`nsec` is always `NULL`** in every row, and the
  implementations do `(void) nsec;`.
* ABYTES: aegis128l 32, aegis256 32, aes256gcm 16, chacha20poly1305 16,
  chacha20poly1305_ietf 16, xchacha20poly1305_ietf 16, secretbox MAC 16,
  secretstream 17 (`1 + 16`).
* NPUBBYTES: aegis128l 16, aegis256 32, aes256gcm 12, chacha20poly1305 **8** (original),
  chacha20poly1305_ietf **12**, xchacha20poly1305_ietf **24**, secretbox nonce 24,
  secretstream header 24.
* KEYBYTES: aegis128l **16**; everything else 32.

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| **aegis128l** (KEY 16, NPUB 16, ABYTES 32, RATE 32, portable soft impl) | | | |
| 6.1 | `crypto_aead_aegis128l_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | constant getters; assert `16 / 0 / 16 / 32 / MIN(SIZE_MAX-32, 2^61-1)` | [x] |
| 6.2 | `crypto_aead_aegis128l_keygen` | fill a 16-byte buffer; two successive calls differ; buffer fully written | [x] |
| 6.3 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | round trip, `ad = NULL`, `adlen = 0`, `nsec = NULL`, `clen_p != NULL`, `mlen_p != NULL`; `mlen ∈ MLEN`; assert `clen == mlen + 32`, `*mlen_p == mlen`, recovered `m` equal | [x] |
| 6.4 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | as 6.3 but `clen_p = NULL` on encrypt and `mlen_p = NULL` on decrypt; `mlen ∈ MLEN` | [x] |
| 6.5 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 32, 33, 64}`; ad byte-for-byte identical on both sides | [x] |
| 6.6 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | `mlen ∈ BIG_AEGIS128L`, `adlen ∈ {0, 64, 65, 128}` — exercises `aegis128l_absorb2` (64-byte stride), `absorb` (32-byte stride), the `adlen % 32` tail, and `aegis128l_declast` | [x] |
| 6.7 | `crypto_aead_aegis128l_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (assert `*maclen_p == 32`), separate 32-byte `mac` buffer; `mlen ∈ MLEN`, `adlen ∈ ADLEN`; detached output must equal `encrypt` output split at `mlen` | [x] |
| 6.8 | `crypto_aead_aegis128l_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` — assert identical ciphertext/mac to 6.7 | [x] |
| 6.9 | `crypto_aead_aegis128l_decrypt_detached` | `m = NULL` (verify-only); valid mac ⇒ `0`, tampered mac ⇒ `-1`; `mlen ∈ MLEN` — exercises the `else` branches at `aegis128l_soft.c:225-229, 234` | [x] |
| 6.10 | `crypto_aead_aegis128l_decrypt` / `_decrypt_detached` | `nsec` (out-param) `NULL` vs pointing at a poisoned 1-byte buffer; assert byte unmodified and result unchanged | [x] |
| 6.11 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | in-place: `c == m` and `m == c` aliasing for `mlen ∈ {0, 1, 32, 33, 64, 1024}` | [x] |
| 6.12 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | key/nonce corner shapes: all-zero `k`, all-`0xff` `k`, all-zero `npub`, all-`0xff` `npub`; `mlen ∈ {0, 32, 33}` | [x] |
| 6.13 | `crypto_aead_aegis128l_encrypt` | fixed KAT vectors (deterministic `k`, `npub`, `m`, `ad`) — the portable soft path must match the reference AEGIS-128L tag/ciphertext | [x] |
| **aegis256** (KEY 32, NPUB 32, ABYTES 32, RATE 16, portable soft impl) | | | |
| 6.14 | `crypto_aead_aegis256_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 32 / 32 / MIN(SIZE_MAX-32, 2^61-1)` | [x] |
| 6.15 | `crypto_aead_aegis256_keygen` | fill a 32-byte buffer; two calls differ | [x] |
| 6.16 | `crypto_aead_aegis256_encrypt` + `_decrypt` | round trip, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 32` | [x] |
| 6.17 | `crypto_aead_aegis256_encrypt` + `_decrypt` | as 6.16 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.18 | `crypto_aead_aegis256_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 16, 17, 32}` | [x] |
| 6.19 | `crypto_aead_aegis256_encrypt` + `_decrypt` | `mlen ∈ BIG_AEGIS256`, `adlen ∈ {0, 32, 33, 64}` — exercises `aegis256_absorb2` (32-byte stride), `absorb` (16-byte stride), `adlen % 16` tail, `aegis256_declast` | [x] |
| 6.20 | `crypto_aead_aegis256_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 32`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; must agree with the combined API | [x] |
| 6.21 | `crypto_aead_aegis256_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.22 | `crypto_aead_aegis256_decrypt_detached` | `m = NULL` verify-only; valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.23 | `crypto_aead_aegis256_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.24 | `crypto_aead_aegis256_encrypt` / `_decrypt` | in-place `c == m` for `mlen ∈ {0, 1, 16, 17, 32, 1024}` | [x] |
| 6.25 | `crypto_aead_aegis256_encrypt` / `_decrypt` | all-zero / all-`0xff` `k` and `npub`; `mlen ∈ {0, 16, 17}` | [x] |
| 6.26 | `crypto_aead_aegis256_encrypt` | fixed AEGIS-256 KAT vectors | [x] |
| **aes256gcm** — unavailable in this build (`is_available() == 0`; all ops `-1`/`ENOSYS`) | | | |
| 6.27 | `crypto_aead_aes256gcm_is_available` | no options; assert returns exactly `0` in the no-`HAVE_*` CMake configuration. Every other row in this block is conditioned on that | [x] |
| 6.28 | `crypto_aead_aes256gcm_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` / `_statebytes` | still functional despite unavailability; assert `32 / 0 / 12 / 16 / MIN(SIZE_MAX-16, 16*(2^32-2))` and `_statebytes() == (sizeof(state)+15) & ~15` (multiple of 16, non-zero) | [x] |
| 6.29 | `crypto_aead_aes256gcm_keygen` | still functional; fills 32 bytes; two calls differ | [x] |
| 6.30 | `crypto_aead_aes256gcm_encrypt` | `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `clen_p ∈ {NULL, non-NULL}`, `nsec = NULL`; **every** case ⇒ `-1`, `errno == ENOSYS`, `*clen_p` left at its pre-call poison value, `c` untouched | [x] |
| 6.31 | `crypto_aead_aes256gcm_encrypt_detached` | `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `maclen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`, `*maclen_p` poison preserved, `c`/`mac` untouched | [x] |
| 6.32 | `crypto_aead_aes256gcm_decrypt` | `clen ∈ {0, 1, 15, 16, 17, 48}` (both below and above ABYTES) × `adlen ∈ ADLEN` × `mlen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`, `*mlen_p` poison preserved | [x] |
| 6.33 | `crypto_aead_aes256gcm_decrypt_detached` | `clen ∈ MLEN` × `adlen ∈ ADLEN`, `m ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`; `m` not even zeroed | [x] |
| 6.34 | `crypto_aead_aes256gcm_beforenm` | 16-byte-aligned `crypto_aead_aes256gcm_state` (heap via `sodium_malloc` and stack via `CRYPTO_ALIGN(16)`), valid 32-byte `k`; ⇒ `-1`/`ENOSYS`, state left uninitialised | [x] |
| 6.35 | `crypto_aead_aes256gcm_encrypt_afternm` | state from a failed `_beforenm` (the only kind obtainable); `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `clen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.36 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | same state; `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `maclen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.37 | `crypto_aead_aes256gcm_decrypt_afternm` | same state; `clen ∈ {0, 15, 16, 17, 48}` × `adlen ∈ ADLEN` × `mlen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.38 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | same state; `clen ∈ MLEN` × `adlen ∈ ADLEN`, `m ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.39 | full aes256gcm "state API" sequence | `_beforenm` → `_encrypt_afternm` → `_decrypt_afternm` in order; assert the sequence never produces a successful round trip and each step independently reports `-1`/`ENOSYS` | [x] |
| **chacha20poly1305 "original"** (KEY 32, **NPUB 8**, ABYTES 16) | | | |
| 6.40 | `crypto_aead_chacha20poly1305_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 8 / 16 / SIZE_MAX-16` | [x] |
| 6.41 | `crypto_aead_chacha20poly1305_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.42 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | 8-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.43 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | as 6.42 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.44 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 16, 63, 64, 65}`. Note the **original** construction has *no* 16-byte zero-padding of `ad`/`c` in the MAC (`aead_chacha20poly1305.c:43-45, 63-64`), unlike the ietf variant — so unaligned `adlen` must produce a *different* tag from the ietf variant on the same inputs | [x] |
| 6.45 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 16, 17}` — crosses the 64-byte block and the `STREAM_POLY1305_CHUNK == 131072` chunk restart with a 64-bit `ic` counter | [x] |
| 6.46 | `crypto_aead_chacha20poly1305_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 16`, written *after* the crypto at `aead_chacha20poly1305.c:69-71`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; must match the combined API split at `mlen` | [x] |
| 6.47 | `crypto_aead_chacha20poly1305_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` — assert identical output to 6.46 | [x] |
| 6.48 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m = NULL` verify-only (`aead_chacha20poly1305.c:232-234`); valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.49 | `crypto_aead_chacha20poly1305_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.50 | `crypto_aead_chacha20poly1305_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.51 | `crypto_aead_chacha20poly1305_encrypt` | fixed RFC-style KAT with 8-byte nonce | [x] |
| **chacha20poly1305_ietf** (KEY 32, **NPUB 12**, ABYTES 16) | | | |
| 6.52 | `crypto_aead_chacha20poly1305_ietf_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 12 / 16 / MIN(SIZE_MAX-16, 64*(2^32-1))` | [x] |
| 6.53 | `crypto_aead_chacha20poly1305_ietf_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.54 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | 12-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.55 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | as 6.54 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.56 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ MLEN` — exercises both `_pad0` padding calls `(0x10 - adlen) & 0xf` and `(0x10 - mlen) & 0xf` (`aead_chacha20poly1305.c:128, 146`) at every residue class mod 16 | [x] |
| 6.57 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 15, 16, 17}` — 32-bit `ic` counter across the `STREAM_POLY1305_CHUNK` restart | [x] |
| 6.58 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 16`); `mlen ∈ MLEN`, `adlen ∈ ADLEN` | [x] |
| 6.59 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.60 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m = NULL` verify-only (`aead_chacha20poly1305.c:317-319`); `mlen ∈ MLEN` | [x] |
| 6.61 | `crypto_aead_chacha20poly1305_ietf_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.62 | `crypto_aead_chacha20poly1305_ietf_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.63 | `crypto_aead_chacha20poly1305_ietf_encrypt` | RFC 8439 KAT vectors (12-byte nonce) | [x] |
| 6.64 | `crypto_aead_chacha20poly1305_ietf_*` vs `crypto_aead_chacha20poly1305_*` | same key, same first-8-bytes-of-nonce: assert the two families produce **different** ciphertexts/tags (different nonce layout and different MAC framing) — guards against collapsing them in translation | [x] |
| **xchacha20poly1305_ietf** (KEY 32, **NPUB 24**, ABYTES 16) | | | |
| 6.65 | `crypto_aead_xchacha20poly1305_ietf_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 24 / 16 / SIZE_MAX-16`; also the `crypto_aead_xchacha20poly1305_IETF_*` uppercase aliases resolve identically | [x] |
| 6.66 | `crypto_aead_xchacha20poly1305_ietf_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.67 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | 24-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.68 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | as 6.67 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.69 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ MLEN` — every residue class mod 16 for both `_pad0` calls (`aead_xchacha20poly1305.c:46, 73`) | [x] |
| 6.70 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 16, 17}` — plus the `chunk` selection branch at `aead_xchacha20poly1305.c:56-58` (`mlen <= 64*(0xffffffff-1)` ⇒ chunked, else single pass); only the chunked side is reachable for realistic sizes but the branch must be preserved | [x] |
| 6.71 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == crypto_aead_chacha20poly1305_ietf_ABYTES == 16`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; internally goes through HChaCha20 subkey derivation + a 12-byte `npub2` with 4 leading zero bytes (`aead_xchacha20poly1305.c:158-163`) | [x] |
| 6.72 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.73 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m = NULL` verify-only (`aead_xchacha20poly1305.c:132-134`); valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.74 | `crypto_aead_xchacha20poly1305_ietf_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.75 | `crypto_aead_xchacha20poly1305_ietf_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.76 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | fixed KAT vectors (24-byte nonce), incl. all-zero `npub` and all-`0xff` `npub` | [x] |
| 6.77 | `crypto_aead_xchacha20poly1305_ietf_encrypt` cross-check | equal to `crypto_aead_chacha20poly1305_ietf_encrypt` under `k2 = hchacha20(npub[0..15], k)` and `npub2 = 0x00000000 || npub[16..23]` — confirms the XChaCha20 construction wiring | [x] |
| **secretbox — generic / xsalsa20poly1305 (KEY 32, NONCE 24, MAC 16, ZERO 32, BOXZERO 16)** | | | |
| 6.78 | `crypto_secretbox_keybytes` / `_noncebytes` / `_macbytes` / `_zerobytes` / `_boxzerobytes` / `_messagebytes_max` / `_primitive` | assert `32 / 24 / 16 / 32 / 16 / stream_MESSAGEBYTES_MAX-16` and `_primitive() == "xsalsa20poly1305"` | [x] |
| 6.79 | `crypto_secretbox_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.80 | `crypto_secretbox_easy` + `crypto_secretbox_open_easy` | round trip, out buffer `mlen + 16`; `mlen ∈ MLEN`; assert `c[0..16)` is the MAC and `open_easy` recovers `m` | [x] |
| 6.81 | `crypto_secretbox_easy` + `crypto_secretbox_open_easy` | `mlen ∈ BIG_SECRETBOX` — crosses the `mlen0 = min(mlen, 64 - 32) == 32` first-block special case (`crypto_secretbox_easy.c:49-63`) and the `STREAM_POLY1305_CHUNK` restart (`:71-82`) | [x] |
| 6.82 | `crypto_secretbox_detached` + `crypto_secretbox_open_detached` | separate 16-byte `mac` buffer; `mlen ∈ MLEN` ∪ `BIG_SECRETBOX`; assert `detached` output == `easy` output split at 16 | [x] |
| 6.83 | `crypto_secretbox_open_detached` | `m = NULL` verify-only (`crypto_secretbox_easy.c:131-134`); valid mac ⇒ `0`, tampered ⇒ `-1`; `clen ∈ MLEN` | [x] |
| 6.84 | `crypto_secretbox_easy` / `_open_easy` in-place | `c == m` and the documented `m = c + 16` / `c = m - 16` overlap patterns that trigger the `memmove` branches (`crypto_secretbox_easy.c:40-46` and `:145-151`); `mlen ∈ {0, 1, 31, 32, 33, 64, 4096}` | [x] |
| 6.85 | `crypto_secretbox_detached` / `_open_detached` | fully disjoint buffers (the `memmove` branches *not* taken) for the same `mlen` set as 6.84 — both sides of each overlap branch must be covered | [x] |
| 6.86 | `crypto_secretbox` (NaCl-style) + `crypto_secretbox_open` | `m` buffer with `m[0..31] = 0` zero padding, `mlen = 32 + plaintext_len` for `plaintext_len ∈ MLEN`; assert `c[0..15] == 0` (BOXZEROBYTES forced to zero at `secretbox_xsalsa20poly1305.c:20-22`), MAC at `c[16..31]`, and `crypto_secretbox_open` returns `0` with `m[0..31]` re-zeroed and plaintext at `m + 32` | [x] |
| 6.87 | `crypto_secretbox` + `crypto_secretbox_open` | large NaCl-style: `mlen = 32 + n` for `n ∈ {0, 1, 32, 63, 64, 65, 4096, 131073}` (the xsalsa20 path is a single `crypto_stream_xsalsa20_xor` with no chunking, unlike `_easy`) | [x] |
| 6.88 | `crypto_secretbox_xsalsa20poly1305` / `_open` | called directly (not via the `crypto_secretbox` wrapper at `crypto_secretbox.c:47-61`); assert byte-identical results to 6.86 | [x] |
| 6.89 | `crypto_secretbox_xsalsa20poly1305_keybytes` / `_noncebytes` / `_zerobytes` / `_boxzerobytes` / `_macbytes` / `_messagebytes_max` / `_keygen` | assert `32 / 24 / 32 / 16 / 16 / …` and that `crypto_secretbox_*` aliases resolve to the same values | [x] |
| 6.90 | `crypto_secretbox_easy` vs `crypto_secretbox` | same `k`, `n`, plaintext: assert `easy(c, m, len)` output equals `secretbox(c', 32-zero-padded m, 32+len)` shifted by 16 (`c == c' + 16`) — the two APIs are the same construction with different framing | [x] |
| 6.91 | `crypto_secretbox_easy` / `crypto_secretbox` | corner keys/nonces: all-zero `k`, all-`0xff` `k`, all-zero `n`, all-`0xff` `n`, `n` with a non-zero high half only (`n + 16` is the salsa20 nonce, `n[0..15]` the hsalsa20 input); `mlen ∈ {0, 1, 32, 33}` | [x] |
| 6.92 | `crypto_secretbox` / `crypto_secretbox_open` | NaCl KAT vectors (the classic libsodium/NaCl `secretbox` test vector) | [x] |
| **secretbox — xchacha20poly1305 primitive family (KEY 32, NONCE 24, MAC 16)** | | | |
| 6.93 | `crypto_secretbox_xchacha20poly1305_keybytes` / `_noncebytes` / `_macbytes` / `_messagebytes_max` | assert `32 / 24 / 16 / stream_xchacha20_MESSAGEBYTES_MAX - 16`. Note there is **no** `_zerobytes`/`_boxzerobytes`/`_keygen`/`_primitive` in this family | [x] |
| 6.94 | `crypto_secretbox_xchacha20poly1305_easy` + `_open_easy` | round trip, out buffer `mlen + 16`; `mlen ∈ MLEN` | [x] |
| 6.95 | `crypto_secretbox_xchacha20poly1305_easy` + `_open_easy` | `mlen ∈ BIG_SECRETBOX` — crosses the `mlen0 = min(mlen, 64-32) == 32` first-block case (`secretbox_xchacha20poly1305.c:51-72`). NB unlike the xsalsa20 variant this one does **not** chunk at 131072; it does a single `crypto_stream_chacha20_xor_ic` for the tail | [x] |
| 6.96 | `crypto_secretbox_xchacha20poly1305_detached` + `_open_detached` | separate `mac` buffer; `mlen ∈ MLEN` ∪ `BIG_SECRETBOX`; must equal `_easy` output split at 16 | [x] |
| 6.97 | `crypto_secretbox_xchacha20poly1305_open_detached` | `m = NULL` verify-only (`secretbox_xchacha20poly1305.c:124-127`); valid ⇒ `0`, tampered ⇒ `-1`; `clen ∈ MLEN` | [x] |
| 6.98 | `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy` in-place | `c == m` plus the `m = c + 16` overlap patterns that hit the `memmove` branches (`secretbox_xchacha20poly1305.c:42-48`, `:138-144`); `mlen ∈ {0, 1, 31, 32, 33, 64, 4096}` | [x] |
| 6.99 | `crypto_secretbox_xchacha20poly1305_detached` / `_open_detached` | fully disjoint buffers (memmove branches not taken), same `mlen` set as 6.98 | [x] |
| 6.100 | `crypto_secretbox_xchacha20poly1305_easy` | corner keys/nonces: all-zero / all-`0xff` `k`; all-zero / all-`0xff` `n`; note the split `n[0..15]` → hchacha20 input, `n + 16` → chacha20 nonce; `mlen ∈ {0, 1, 32, 33}` | [x] |
| 6.101 | `crypto_secretbox_xchacha20poly1305_easy` | fixed KAT vectors; assert output **differs** from `crypto_secretbox_easy` on identical `k`/`n`/`m` (different primitive) | [x] |
| 6.102 | `crypto_secretbox_xchacha20poly1305_easy` + `crypto_secretbox_open_easy` (mismatched families) | cross-family: encrypt with xchacha20poly1305, open with the xsalsa20poly1305 default; must fail with `-1` for `mlen ∈ {0, 1, 32, 33}` (and vice versa) | [x] |
| **secretstream_xchacha20poly1305** (KEY 32, HEADER 24, ABYTES 17, tags 0x00/0x01/0x02/0x03) | | | |
| 6.103 | `crypto_secretstream_xchacha20poly1305_keybytes` / `_headerbytes` / `_abytes` / `_statebytes` / `_messagebytes_max` | assert `32 / 24 / 17 / sizeof(state) / MIN(SIZE_MAX-17, 64*(2^32-2))` | [x] |
| 6.104 | `crypto_secretstream_xchacha20poly1305_tag_message` / `_tag_push` / `_tag_rekey` / `_tag_final` | assert `0x00 / 0x01 / 0x02 / 0x03` and that `TAG_FINAL == (TAG_PUSH \| TAG_REKEY)` | [x] |
| 6.105 | `crypto_secretstream_xchacha20poly1305_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.106 | `crypto_secretstream_xchacha20poly1305_init_push` | writes 24 random header bytes, resets the 4-byte counter to `{1,0,0,0}`, copies the 8-byte inonce, zeroes `state->_pad`; returns `0`; two inits with the same `k` give different headers | [x] |
| 6.107 | `_init_push` + `_init_pull` | pull side initialised from the pushed header with the same `k`; assert both states derive the same `state->k` (observable via a successful first `_pull`) | [x] |
| 6.108 | `_init_push` → `_push(TAG_MESSAGE)` → `_init_pull` → `_pull` | single-frame session, `ad = NULL`/`adlen = 0`, `outlen_p`/`mlen_p`/`tag_p` all non-NULL; `mlen ∈ MLEN`; assert `*outlen_p == mlen + 17`, `*mlen_p == mlen`, `*tag_p == TAG_MESSAGE` | [x] |
| 6.109 | same session as 6.108 | `outlen_p = NULL` on push and `mlen_p = NULL` / `tag_p = NULL` on pull (all four combinations of the two pull pointers); `mlen ∈ MLEN`; output bytes must be identical to 6.108 | [x] |
| 6.110 | multi-frame session, all `TAG_MESSAGE` | 1, 2, 3, 8, 64 frames, each with `mlen ∈ MLEN` (rotating); assert the inonce/counter chaining (`XOR_BUF(STATE_INONCE, mac, 8)` + `sodium_increment(counter, 4)` at `secretstream_xchacha20poly1305.c:164-167`) keeps push and pull in lockstep | [x] |
| 6.111 | session with `TAG_PUSH` (0x01) frames | `TAG_MESSAGE`, `TAG_PUSH`, `TAG_MESSAGE` sequence; `TAG_PUSH` does **not** have the `0x02` bit so it must **not** trigger an implicit rekey; assert `*tag_p == TAG_PUSH` and the stream continues | [x] |
| 6.112 | session with `TAG_REKEY` (0x02) frames | `TAG_MESSAGE`, `TAG_REKEY`, `TAG_MESSAGE` sequence; the `0x02` bit triggers the implicit `_rekey()` on **both** push and pull (`:168-172`, `:250-254`); assert the post-rekey frames still round trip and that the derived key changed | [x] |
| 6.113 | session with `TAG_FINAL` (0x03) | `TAG_MESSAGE` × n then `TAG_FINAL`; assert `*tag_p == TAG_FINAL` on the last pull and that `TAG_FINAL` also triggers the implicit rekey (it has the `0x02` bit) | [x] |
| 6.114 | full tag matrix in one session | ordered sequence `TAG_MESSAGE, TAG_PUSH, TAG_MESSAGE, TAG_REKEY, TAG_MESSAGE, TAG_PUSH, TAG_FINAL`, each frame with a different `mlen` drawn from `MLEN`; assert every `*tag_p` matches what was pushed | [x] |
| 6.115 | explicit `crypto_secretstream_xchacha20poly1305_rekey` on **both** sides | push `TAG_MESSAGE`, call `_rekey(push_state)` and `_rekey(pull_state)` at the same point in the sequence, push/pull another `TAG_MESSAGE`; assert the session stays in sync and the counter resets to 1 | [x] |
| 6.116 | explicit `_rekey` repeated | `_rekey` called 0, 1, 2, 5 times consecutively (symmetrically on both states) before the next frame; each count must still round trip | [x] |
| 6.117 | explicit `_rekey` interleaved with an implicit `TAG_REKEY` | `_push(TAG_REKEY)` followed by an explicit `_rekey` on both sides; assert both rekeys are applied in the same order on both sides | [x] |
| 6.118 | push/pull with `ad` present | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 15, 16, 17, 63, 64, 65}` — exercises the `(0x10 - adlen) & 0xf` padding at `:136-137` / `:213-214` at every residue mod 16 | [x] |
| 6.119 | push/pull with `ad` varying **per frame** | frame 0 with `ad = NULL`/0, frame 1 with a 17-byte `ad`, frame 2 with a 32-byte `ad`, frame 3 with `ad = non-NULL`/`adlen = 0`; each pull must supply the matching `ad` | [x] |
| 6.120 | push/pull with large messages | `mlen ∈ {4096, 65536, 131072, 131073, 262145}` — the secretstream path calls `crypto_stream_chacha20_ietf_xor_ic(..., ic = 2)` in a **single** pass (no chunking, `:147`, `:245`), unlike the AEAD path | [x] |
| 6.121 | push/pull message-length boundary around the quirky padding | `mlen ∈ {0, 15, 16, 17, 47, 48, 49, 63, 64, 65}` — the padding expression `(0x10 - (sizeof block) + mlen) & 0xf` at `:149-151` / `:226-228` is the documented off-by-`sizeof block` quirk (`sizeof block == 64`, so it reduces to `mlen & 0xf`); the translation must reproduce this bug exactly, and the `slen` length field is `64 + mlen`, not `mlen` (`:155`, `:232`) | [x] |
| 6.122 | `_push` with a `tag` value outside `{0x00, 0x01, 0x02, 0x03}` | `tag ∈ {0x04, 0x7f, 0x80, 0xfe, 0xff}` — never validated; assert `_push` returns `0`, `_pull` reports the same `*tag_p`, and that any tag with the `0x02` bit set (`0x06`, `0x7f`, `0xff`, …) triggers the implicit rekey on both sides | [x] |
| 6.123 | `_init_pull` with an arbitrary 24-byte header | all-zero header, all-`0xff` header, and a header from a *different* `_init_push`; assert `_init_pull` returns `0` regardless, and the mismatch only shows up as `-1` on the first `_pull` | [x] |
| 6.124 | `_push` / `_pull` in-place | `out == m` and `m == in` aliasing where the API permits it (`out` is `1 + mlen + 16` bytes, `in` is `mlen + 17`); `mlen ∈ {0, 1, 64, 65}` — note the code writes `out[0]` before the `xor_ic` into `out + 1`, so `in`-place pull needs `m == in` handling | [x] |
| 6.125 | `_push` with corner keys | all-zero `k`, all-`0xff` `k` into `_init_push`; `mlen ∈ {0, 1, 64}`; full round trip | [x] |
| 6.126 | `_push` determinism given a fixed header | drive `_init_pull` from a hard-coded header and hard-coded `k`, then `_push`-equivalent framing via a second `_init_pull`-seeded state (or a KAT of `{header, k, [(tag, ad, m)…]}` → concatenated ciphertext frames) so the byte-exact stream format is pinned, including the `state->_pad` zeroing at `:62`/`:77` | [x] |
| 6.127 | `_statebytes` vs actual state usage | allocate exactly `crypto_secretstream_xchacha20poly1305_statebytes()` bytes (heap, unaligned-by-1 offsets included) for the state and run a full session; assert no over-read/over-write and that `sizeof(crypto_secretstream_xchacha20poly1305_state)` is what `_statebytes()` reports | [x] |
| 6.128 | 32-bit counter wrap | force `STATE_COUNTER` near `0xffffffff` (either by direct state manipulation in a white-box test or by documenting it as unreachable) so that `sodium_is_zero(counter, 4)` at `:169-170` / `:251-252` fires and an implicit rekey happens on both sides without an explicit `TAG_REKEY` | [x] |
| 6.129 | cross-API check: secretstream vs `crypto_aead_xchacha20poly1305_ietf` | assert secretstream framing is **not** interchangeable with the AEAD (extra 1-byte tag, `ic` starting at 2, `slen = 64 + mlen`) — encrypting with one and decrypting with the other must fail | [x] |

### Coverage notes (Phase B/C, area 6)

Test files: `tests/a6_aead.rs`, `tests/a6_aes256gcm.rs`, `tests/a6_secretbox.rs`,
`tests/a6_secretstream.rs` (69 test functions, all green).

* Row 6.65: `crypto_aead_xchacha20poly1305_IETF_*` are **preprocessor macro aliases**
  (`crypto_aead_xchacha20poly1305.h:90-94`), not exported symbols, so they cannot be resolved
  through `dlsym`; the lowercase getters they alias are verified instead.
* Rows 6.13 / 6.26 / 6.51 / 6.76 / 6.92 / 6.126: implemented as *pinned* vectors — fully
  deterministic hard-coded `k` / `npub` (or header) / `m` / `ad` compared byte-for-byte between
  the two libraries — plus, for the constructions, an independent re-derivation from the
  already-verified low-level primitives (`crypto_core_hsalsa20` + `crypto_stream_salsa20_xor{,_ic}`
  + `crypto_onetimeauth_poly1305` for secretbox, `crypto_core_hchacha20` + `crypto_stream_chacha20`
  for the xchacha20poly1305 secretbox, `crypto_core_hchacha20` + `crypto_aead_chacha20poly1305_ietf`
  for xchacha20poly1305_ietf). Row 6.63 additionally pins the absolute RFC 8439 §2.8.2 tag.
* Row 6.128 (secretstream 32-bit counter wrap) is reached white-box, by writing
  `0xffffffff` into `STATE_COUNTER` on both sides before the next `_push`/`_pull`.
