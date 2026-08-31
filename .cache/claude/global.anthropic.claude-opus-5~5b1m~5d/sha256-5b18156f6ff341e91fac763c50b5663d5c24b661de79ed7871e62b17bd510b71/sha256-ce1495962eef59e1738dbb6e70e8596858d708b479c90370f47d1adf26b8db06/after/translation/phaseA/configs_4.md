## Area 4 — crypto_auth + crypto_onetimeauth

Configuration axes taken directly from the source:

- **Primitive**: `hmacsha256` (SHA-256, block 64, tag 32, `crypto_verify_32`), `hmacsha512` (SHA-512, block 128, tag 64, `crypto_verify_64`), `hmacsha512256` (SHA-512 internally, tag truncated to 32, `crypto_verify_32`), `poly1305` (block 16, tag 16, `crypto_verify_16`).
- **Entry style**: one-shot `*_auth(out, in, inlen, k)` vs. streaming `*_init` / `*_update` / `*_final`. The one-shot HMAC functions are literally `init(&state, k, KEYBYTES); update(...); final(...)`, so streaming with a 32-byte key must be bit-identical.
- **Key length** (HMAC only — `*_init` takes an explicit `keylen`): `keylen < BLOCKBYTES`, `keylen == BLOCKBYTES` (must **not** hash), `keylen > BLOCKBYTES` (hashed to 32 / 64 bytes). `poly1305` has no keylen parameter — always exactly 32 bytes.
- **Message length / update splitting**: pad and block boundaries of the underlying hash, plus multi-`update` splits that straddle those boundaries and the poly1305 16-byte `leftover` buffer.
- **Wrapper level**: generic `crypto_auth*` / `crypto_onetimeauth*` vs. primitive-specific entry points, plus the `*bytes` / `*keybytes` / `*statebytes` / `*primitive` accessors.
- **Build config**: no `HAVE_*` macros are defined, so poly1305 uses `donna` with `poly1305_donna32.h` (32-bit limbs); `sse2/` is not compiled, `crypto_verify_n` and `sodium_memcmp` take their portable branches.

### CONFIGURATION SURFACE

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 4.1 | `crypto_auth` | generic one-shot wrapper; 32-byte key; must be byte-identical to `crypto_auth_hmacsha512256` for every message length in {0,1,55,56,63,64,65,111,112,127,128,129} | [x] |
| 4.2 | `crypto_auth_verify` | generic verify; good 32-byte tag ⇒ `0`; tag with one flipped bit ⇒ `-1`; identical results to `crypto_auth_hmacsha512256_verify` | [x] |
| 4.3 | `crypto_auth_keygen` | fills exactly `crypto_auth_KEYBYTES` = 32 bytes from `randombytes_buf`; two successive calls differ; no bytes written past index 31 | [x] |
| 4.4 | `crypto_auth_primitive` | returns the literal `"hmacsha512256"` (`crypto_auth_PRIMITIVE`) | [x] |
| 4.5 | `crypto_auth_bytes` / `crypto_auth_keybytes` | return 32 / 32, matching the macros `crypto_auth_BYTES` = `crypto_auth_hmacsha512256_BYTES` and `crypto_auth_KEYBYTES` = `crypto_auth_hmacsha512256_KEYBYTES` | [x] |
| 4.6 | `crypto_auth.h` surface shape | the generic `crypto_auth` API deliberately exposes **no** state type, **no** `crypto_auth_statebytes`, and **no** init/update/final — streaming is only reachable through the primitive-specific names. Port must not invent a generic streaming API | [x] |
| 4.7 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 0` (empty message) | [x] |
| 4.8 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.9 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 55` (inner hash: last block has exactly 9 bytes for pad+length after the 64-byte ipad block) | [x] |
| 4.10 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 56` (pad spills into an extra SHA-256 block) | [x] |
| 4.11 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.12 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 64` (exactly one SHA-256 block after the ipad block) | [x] |
| 4.13 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.14 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 111` | [x] |
| 4.15 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 112` | [x] |
| 4.16 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.17 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 128` (two full blocks) | [x] |
| 4.18 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.19 | `crypto_auth_hmacsha256_init` + `_update` + `_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.7 | [x] |
| 4.20 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.8 | [x] |
| 4.21 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.9 | [x] |
| 4.22 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.10 | [x] |
| 4.23 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.11 | [x] |
| 4.24 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.12 | [x] |
| 4.25 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.13 | [x] |
| 4.26 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.14 | [x] |
| 4.27 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.15 | [x] |
| 4.28 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.16 | [x] |
| 4.29 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.17 | [x] |
| 4.30 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.18 | [x] |
| 4.31 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(0, 64)` — a zero-length first `update` must be a no-op | [x] |
| 4.32 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(1, 63)` on a 64-byte message (straddles the SHA-256 block boundary) | [x] |
| 4.33 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(63, 1)` on a 64-byte message (second update exactly completes the block) | [x] |
| 4.34 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(64, 1)` on a 65-byte message (first update ends exactly on a block) | [x] |
| 4.35 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(32, 32)` on a 64-byte message (neither part is block-aligned) | [x] |
| 4.36 | `crypto_auth_hmacsha256_update` ×129 | 129 successive 1-byte updates; must equal 4.18 | [x] |
| 4.37 | `crypto_auth_hmacsha256_update` ×3 | three-way split `(40, 40, 32)` on a 112-byte message; also `(56, 56)`; both must equal 4.15 | [x] |
| 4.38 | `crypto_auth_hmacsha256_init` | `keylen = 0` with a non-NULL `key` pointer — XOR loops iterate zero times, ipad/opad stay `0x36`/`0x5c` | [x] |
| 4.39 | `crypto_auth_hmacsha256_init` | `keylen = 0` with `key == NULL` — permitted (inner `if (keylen > 0)` false, no `sodium_misuse`); must equal 4.38 | [x] |
| 4.40 | `crypto_auth_hmacsha256_init` | `keylen = 1` (shorter than block) | [x] |
| 4.41 | `crypto_auth_hmacsha256_init` | `keylen = 31` (just under the canonical 32) | [x] |
| 4.42 | `crypto_auth_hmacsha256_init` | `keylen = 32` = `crypto_auth_hmacsha256_KEYBYTES`, the value the one-shot uses | [x] |
| 4.43 | `crypto_auth_hmacsha256_init` | `keylen = 63` (one below the block size) | [x] |
| 4.44 | `crypto_auth_hmacsha256_init` | `keylen = 64` == BLOCKBYTES — boundary: `keylen > 64` is false, so the key is **not** hashed and fills `pad` exactly | [x] |
| 4.45 | `crypto_auth_hmacsha256_init` | `keylen = 65` > BLOCKBYTES — key replaced by `SHA-256(key)`, `keylen` forced to 32; must equal `_init` with that 32-byte hash | [x] |
| 4.46 | `crypto_auth_hmacsha256_init` | `keylen = 128` > BLOCKBYTES (exactly two blocks of key material to hash) | [x] |
| 4.47 | `crypto_auth_hmacsha256_init` | `keylen = 1000` (multi-block key hashing, non-block-aligned); also `keylen` larger than any internal buffer to confirm no stack overflow of `pad[64]` | [x] |
| 4.48 | `crypto_auth_hmacsha256_verify` | good tag from 4.7–4.18 ⇒ `0`, for every message length in the set | [x] |
| 4.49 | `crypto_auth_hmacsha256_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.50 | `crypto_auth_hmacsha256_verify` | corrupted tag: flip bit 7 of byte 31 (last byte — catches short-compare bugs) ⇒ `-1` | [x] |
| 4.51 | `crypto_auth_hmacsha256_verify` | all-zero tag and fully random tag ⇒ `-1`; also correct tag verified against a different message and against a different key ⇒ `-1` | [x] |
| 4.52 | `crypto_auth_hmacsha256_keygen` | fills exactly 32 bytes; output usable as key for 4.42; successive calls differ | [x] |
| 4.53 | `crypto_auth_hmacsha256_bytes` / `_keybytes` / `_statebytes` | return 32 / 32 / `sizeof(crypto_auth_hmacsha256_state)` (= two `crypto_hash_sha256_state`s: `ictx` then `octx`) | [x] |
| 4.54 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 0` | [x] |
| 4.55 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.56 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 55` | [x] |
| 4.57 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 56` | [x] |
| 4.58 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.59 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 64` | [x] |
| 4.60 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.61 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 111` (SHA-512 last block has exactly 17 bytes for pad+length) | [x] |
| 4.62 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 112` (pad spills into an extra 128-byte block) | [x] |
| 4.63 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.64 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 128` (exactly one SHA-512 block after the ipad block) | [x] |
| 4.65 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.66 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.54 | [x] |
| 4.67 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.55 | [x] |
| 4.68 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.56 | [x] |
| 4.69 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.57 | [x] |
| 4.70 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.58 | [x] |
| 4.71 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.59 | [x] |
| 4.72 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.60 | [x] |
| 4.73 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.61 | [x] |
| 4.74 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.62 | [x] |
| 4.75 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.63 | [x] |
| 4.76 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.64 | [x] |
| 4.77 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.65 | [x] |
| 4.78 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(0, 128)` — zero-length first update is a no-op | [x] |
| 4.79 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(1, 127)` on a 128-byte message (straddles the SHA-512 block boundary) | [x] |
| 4.80 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(127, 1)` on a 128-byte message (second update completes the block) | [x] |
| 4.81 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(128, 1)` on a 129-byte message (first update ends exactly on a block) | [x] |
| 4.82 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(64, 64)` on a 128-byte message | [x] |
| 4.83 | `crypto_auth_hmacsha512_update` ×129 | 129 successive 1-byte updates; must equal 4.65 | [x] |
| 4.84 | `crypto_auth_hmacsha512_update` ×3 | three-way split `(40, 40, 32)` on a 112-byte message; must equal 4.62 | [x] |
| 4.85 | `crypto_auth_hmacsha512_init` | `keylen = 0` with a non-NULL `key` | [x] |
| 4.86 | `crypto_auth_hmacsha512_init` | `keylen = 0` with `key == NULL` — permitted, no `sodium_misuse`; must equal 4.85 | [x] |
| 4.87 | `crypto_auth_hmacsha512_init` | `keylen = 1` (shorter than the 128-byte block) | [x] |
| 4.88 | `crypto_auth_hmacsha512_init` | `keylen = 32` = `crypto_auth_hmacsha512_KEYBYTES` (what the one-shot passes) | [x] |
| 4.89 | `crypto_auth_hmacsha512_init` | `keylen = 64` (shorter than the block; equals the *tag* size, not the block size — must not trigger hashing) | [x] |
| 4.90 | `crypto_auth_hmacsha512_init` | `keylen = 127` (one below the block size) | [x] |
| 4.91 | `crypto_auth_hmacsha512_init` | `keylen = 128` == BLOCKBYTES — boundary: `keylen > 128` false, key **not** hashed, fills `pad[128]` exactly | [x] |
| 4.92 | `crypto_auth_hmacsha512_init` | `keylen = 129` > BLOCKBYTES — key replaced by `SHA-512(key)`, `keylen` forced to 64; must equal `_init` with that 64-byte hash | [x] |
| 4.93 | `crypto_auth_hmacsha512_init` | `keylen = 256` > BLOCKBYTES (exactly two blocks of key material) | [x] |
| 4.94 | `crypto_auth_hmacsha512_init` | `keylen = 1000` (multi-block, non-aligned key hashing); confirms `pad[128]`/`khash[64]` are never overrun | [x] |
| 4.95 | `crypto_auth_hmacsha512_verify` | good 64-byte tag ⇒ `0`, for every message length in the set | [x] |
| 4.96 | `crypto_auth_hmacsha512_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.97 | `crypto_auth_hmacsha512_verify` | corrupted tag: flip bit 7 of byte 63 (last byte of the 64-byte compare) ⇒ `-1` | [x] |
| 4.98 | `crypto_auth_hmacsha512_verify` | all-zero tag, random tag, right tag/wrong message, right tag/wrong key ⇒ `-1` | [x] |
| 4.99 | `crypto_auth_hmacsha512_keygen` | fills exactly `crypto_auth_hmacsha512_KEYBYTES` = 32 bytes (note: 32, not 64) | [x] |
| 4.100 | `crypto_auth_hmacsha512_bytes` / `_keybytes` / `_statebytes` | return 64 / 32 / `sizeof(crypto_auth_hmacsha512_state)` (two `crypto_hash_sha512_state`s) | [x] |
| 4.101 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 0` | [x] |
| 4.102 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.103 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 55` | [x] |
| 4.104 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 56` | [x] |
| 4.105 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.106 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 64` | [x] |
| 4.107 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.108 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 111` | [x] |
| 4.109 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 112` | [x] |
| 4.110 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.111 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 128` | [x] |
| 4.112 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.113 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.101 | [x] |
| 4.114 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.102 | [x] |
| 4.115 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.103 | [x] |
| 4.116 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.104 | [x] |
| 4.117 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.105 | [x] |
| 4.118 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.106 | [x] |
| 4.119 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.107 | [x] |
| 4.120 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.108 | [x] |
| 4.121 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.109 | [x] |
| 4.122 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.110 | [x] |
| 4.123 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.111 | [x] |
| 4.124 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.112 | [x] |
| 4.125 | `crypto_auth_hmacsha512256_update` ×2 | multi-update splits `(0, n)` and `(1, n-1)` on a 129-byte message | [x] |
| 4.126 | `crypto_auth_hmacsha512256_update` ×2 | multi-update split `(127, 1)` and `(128, 1)` straddling the 128-byte SHA-512 block boundary | [x] |
| 4.127 | `crypto_auth_hmacsha512256_update` ×2 | multi-update split `(64, 64)` on a 128-byte message | [x] |
| 4.128 | `crypto_auth_hmacsha512256_update` ×129 | 129 successive 1-byte updates; must equal 4.112 | [x] |
| 4.129 | `crypto_auth_hmacsha512256_init` | `keylen` shorter than BLOCKBYTES: each of {0, 1, 32, 64, 127} (block size is 128, inherited from `crypto_auth_hmacsha512_init`) | [x] |
| 4.130 | `crypto_auth_hmacsha512256_init` | `keylen = 128` == BLOCKBYTES boundary — key not hashed | [x] |
| 4.131 | `crypto_auth_hmacsha512256_init` | `keylen` > BLOCKBYTES: each of {129, 256, 1000} — key replaced by `SHA-512(key)`, `keylen` forced to 64 | [x] |
| 4.132 | `crypto_auth_hmacsha512256_init` | `keylen = 0` with `key == NULL` — permitted (header declares `__attribute__((nonnull))` on all args, but the code path itself does not misuse) | [x] |
| 4.133 | `crypto_auth_hmacsha512256_final` vs `crypto_auth_hmacsha512_final` | truncation semantics: the 32-byte output must equal the **first 32 bytes** of the 64-byte hmacsha512 tag for the same key/message; bytes 32..63 are discarded and `out0` is zeroed | [x] |
| 4.134 | `crypto_auth_hmacsha512256_state` / `crypto_auth_hmacsha512_state` | state-type aliasing: `crypto_auth_hmacsha512256_state` is a `typedef` of `crypto_auth_hmacsha512_state`; interop config — `_hmacsha512256_init` then `_hmacsha512_update` then `_hmacsha512_final` yields the 64-byte tag whose 32-byte prefix matches `_hmacsha512256_final` | [x] |
| 4.135 | `crypto_auth_hmacsha512256_verify` | good 32-byte tag ⇒ `0`, for every message length in the set | [x] |
| 4.136 | `crypto_auth_hmacsha512256_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.137 | `crypto_auth_hmacsha512256_verify` | corrupted tag: flip bit 7 of byte 31 ⇒ `-1` | [x] |
| 4.138 | `crypto_auth_hmacsha512256_verify` | truncation-confusion config: pass bytes 32..63 of the untruncated hmacsha512 tag ⇒ `-1`; also all-zero and random tags ⇒ `-1` | [x] |
| 4.139 | `crypto_auth_hmacsha512256_keygen` | fills exactly 32 bytes = `crypto_auth_hmacsha512256_KEYBYTES` | [x] |
| 4.140 | `crypto_auth_hmacsha512256_bytes` / `_keybytes` / `_statebytes` | return 32 / 32 / `sizeof(crypto_auth_hmacsha512256_state)`, which must equal `crypto_auth_hmacsha512_statebytes()` | [x] |
| 4.141 | `crypto_auth` vs `crypto_auth_hmacsha512256` | cross-level equivalence for the whole message-length set and for both verify outcomes; `crypto_auth_primitive()` string agrees with the delegate actually called | [x] |
| 4.142 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 0` (no blocks, no leftover — pure `poly1305_finish` on an empty accumulator) | [x] |
| 4.143 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 1` (single partial block ⇒ leftover path in `poly1305_finish`) | [x] |
| 4.144 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 15` (one byte short of a block) | [x] |
| 4.145 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 16` (exactly one `poly1305_block_size` block, no leftover) | [x] |
| 4.146 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 17` (one block + 1 leftover byte) | [x] |
| 4.147 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 31` | [x] |
| 4.148 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 32` (two full blocks — exercises `bytes & ~(16-1)` with two blocks) | [x] |
| 4.149 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 33` | [x] |
| 4.150 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 0`; must equal 4.142 | [x] |
| 4.151 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 1`; must equal 4.143 | [x] |
| 4.152 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 15`; must equal 4.144 | [x] |
| 4.153 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 16`; must equal 4.145 | [x] |
| 4.154 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 17`; must equal 4.146 | [x] |
| 4.155 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 31`; must equal 4.147 | [x] |
| 4.156 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 32`; must equal 4.148 | [x] |
| 4.157 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 33`; must equal 4.149 | [x] |
| 4.158 | `crypto_onetimeauth_poly1305_update` ×2 | split `(1, 15)` on a 16-byte message — second update fills the leftover buffer to exactly `poly1305_block_size`, so `poly1305_blocks` runs and `leftover` resets to 0 | [x] |
| 4.159 | `crypto_onetimeauth_poly1305_update` ×2 | split `(15, 1)` on a 16-byte message — `want = 16 - 15 = 1`, block completed with a single byte | [x] |
| 4.160 | `crypto_onetimeauth_poly1305_update` ×2 | split `(8, 8)` on a 16-byte message — both halves are pure leftover accumulation until the block completes | [x] |
| 4.161 | `crypto_onetimeauth_poly1305_update` ×2 | split `(15, 2)` on a 17-byte message — leftover completes the block **and** 1 byte is re-stored as new leftover (`st->leftover` was reset to 0 first, so the store must start at index 0) | [x] |
| 4.162 | `crypto_onetimeauth_poly1305_update` ×2 | split `(16, 1)` on a 17-byte message — first update takes the full-block path with `leftover == 0` and stores nothing | [x] |
| 4.163 | `crypto_onetimeauth_poly1305_update` ×2 | split `(17, 16)` on a 33-byte message — second update starts with `leftover == 1`, fills 15, flushes a block, then stores 1 leftover | [x] |
| 4.164 | `crypto_onetimeauth_poly1305_update` ×33 | 33 successive 1-byte updates; must equal 4.149; exercises `want > bytes ⇒ want = bytes` and the early `return` when `leftover < 16` | [x] |
| 4.165 | `crypto_onetimeauth_poly1305_update` | zero-length update with `leftover == 0` (immediately after `_init`) — must be a complete no-op; repeat it several times | [x] |
| 4.166 | `crypto_onetimeauth_poly1305_update` | zero-length update with `leftover > 0` — `want = 16 - leftover` but `want > bytes` forces `want = 0`, so the loop does nothing and the `leftover < 16` early `return` fires; state must be unchanged | [x] |
| 4.167 | `crypto_onetimeauth_poly1305_update` ×2 | leftover-exactly-completes-block, no remainder: `update(5)` then `update(11)` — flush one block, `bytes` becomes 0, neither the full-block nor the store branch runs | [x] |
| 4.168 | `crypto_onetimeauth_poly1305_update` ×2 | all three branches in one call: `update(5)` then `update(40)` — fills 11 (flush), 16 full-block bytes, 13 stored as new leftover | [x] |
| 4.169 | `crypto_onetimeauth_poly1305_update` ×2 | leftover + full blocks with empty remainder: `update(5)` then `update(27)` — fills 11 (flush), 16 full-block bytes, nothing stored | [x] |
| 4.170 | `crypto_onetimeauth_poly1305` / `_update` | long messages (e.g. 1024, 2048, 4096 bytes and a non-aligned 1000) both as one shot and split at odd offsets — exercises the multi-block `poly1305_blocks` loop and `bytes & ~15` masking | [x] |
| 4.171 | `crypto_onetimeauth_poly1305` / `_init` | key-shape configs: all-zero 32-byte key; all-`0xff` key (maximal `r` before clamping — `r` masks `0x3ffffff/0x3ffff03/0x3ffc0ff/0x3f03fff/0x00fffff`); key whose bytes 16..31 (`pad`) are all `0xff` (final addition carries); RFC 8439 test key | [x] |
| 4.172 | `crypto_onetimeauth_poly1305_verify` | good 16-byte tag ⇒ `0`, for every message length in {0,1,15,16,17,31,32,33} | [x] |
| 4.173 | `crypto_onetimeauth_poly1305_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.174 | `crypto_onetimeauth_poly1305_verify` | corrupted tag: flip bit 7 of byte 15 (last byte of the 16-byte compare) ⇒ `-1` | [x] |
| 4.175 | `crypto_onetimeauth_poly1305_verify` | all-zero tag, random tag, correct tag with wrong message, correct tag with wrong key ⇒ `-1`. Note this path uses **only** `crypto_verify_16` (no `sodium_memcmp`, no pointer-aliasing term, unlike the HMAC verifies) | [x] |
| 4.176 | `crypto_onetimeauth_poly1305_keygen` / `crypto_onetimeauth_keygen` | each fills exactly `crypto_onetimeauth_poly1305_KEYBYTES` = 32 bytes; successive calls differ | [x] |
| 4.177 | `crypto_onetimeauth_poly1305_bytes` / `_keybytes` / `_statebytes` | return 16 / 32 / `sizeof(crypto_onetimeauth_poly1305_state)` = 256 (the opaque `unsigned char opaque[256]`, `CRYPTO_ALIGN(16)`); must be `>= sizeof(poly1305_state_internal_t)` per the `COMPILER_ASSERT` in `_donna_init` | [x] |
| 4.178 | `crypto_onetimeauth` / `crypto_onetimeauth_verify` | generic one-shot wrappers must be byte-identical to `crypto_onetimeauth_poly1305` / `_poly1305_verify` for all lengths in the set and for good/corrupt tags | [x] |
| 4.179 | `crypto_onetimeauth_init/_update/_final` | generic streaming wrappers: `crypto_onetimeauth_state` is a `typedef` of `crypto_onetimeauth_poly1305_state`, and each wrapper is a cast-and-delegate — cross-mixing (generic `_init` + primitive `_update` + generic `_final`) must produce the same tag; `crypto_onetimeauth_statebytes()` == `crypto_onetimeauth_poly1305_statebytes()` == 256 | [x] |
| 4.180 | `crypto_onetimeauth_primitive` / `crypto_onetimeauth_bytes` / `_keybytes` | return `"poly1305"` / 16 / 32, matching `crypto_onetimeauth_PRIMITIVE`, `crypto_onetimeauth_BYTES`, `crypto_onetimeauth_KEYBYTES` | [x] |
| 4.181 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | build config: with no `HAVE_TI_MODE` / `HAVE_EMMINTRIN_H` the `sse2` block is not compiled, so the function unconditionally re-installs `crypto_onetimeauth_poly1305_donna_implementation` and returns `0`. Calling it before, between and after other calls must not change any tag; the static `implementation` pointer already defaults to donna | [x] |
| 4.182 | donna backend selection | `poly1305_donna.c` includes `poly1305_donna32.h` (no `HAVE_TI_MODE`), i.e. 32-bit 26-bit-limb arithmetic with `poly1305_state_internal_t { r[5], h[5], pad[4], leftover, buffer[16], final }` and `poly1305_block_size == 16`; `CRYPTO_ALIGN(64)` on the one-shot's local state. All vectors above must match the 64-bit implementation's outputs, so the config is behaviourally invisible but must be the one ported | [x] |
