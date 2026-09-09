# CONFIGS.md — CONFIGURATION-SURFACE TABLE

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
the C code actually branches on: every runtime option/mode/flag the public API
can set, every input SHAPE the code special-cases, and the FULL set of public
entry points — including the lowest-level ones, not just the convenience
wrappers.

Build assumptions are the same as `ERRORS.md` (no `HAVE_*` macros → portable
fallbacks; `crypto_aead_aes256gcm_*` are ENOSYS stubs; `fe_25_5`; scalar
`crypto_verify`; poly1305 donna32; soft-AES ipcrypt; SIMD `runtime_has_*` = 0).

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_EF01`, xorshift64* PRNG in
`tests/common/mod.rs`), driving BOTH the C `.so` and the Rust `.so` through
`libloading` and comparing all output buffers plus the return value
byte-for-byte.

Legend for the checkbox column: `[x]` = passes across randomized inputs.

---

## Group 1 — `sodium/` utils, codecs, randombytes, verify, shorthash, onetimeauth, ipcrypt

Constants: `crypto_verify_{16,32,64}_BYTES = 16/32/64`;
`crypto_shorthash_siphash24` BYTES=8, KEYBYTES=16;
`crypto_shorthash_siphashx24` BYTES=16, KEYBYTES=16;
`crypto_onetimeauth_poly1305` BYTES=16, KEYBYTES=32, statebytes=256;
`randombytes_SEEDBYTES=32`;
base64 variants: `ORIGINAL=1`, `ORIGINAL_NO_PADDING=3`, `URLSAFE=5`,
`URLSAFE_NO_PADDING=7` (`NO_PADDING` bit = 0x2, `URLSAFE` bit = 0x4);
ipcrypt: BYTES=16, KEYBYTES=16, ND_KEY=16/ND_TWEAK=8/ND_OUT=24,
NDX_KEY=32/NDX_TWEAK=16/NDX_OUT=32, PFX_KEY=32/PFX_BYTES=16.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| 1 | `sodium_bin2hex` | `bin_len` 0..64, `hex_maxlen = 2*bin_len+1` | [x] |
| 2 | `sodium_hex2bin` | `ignore=NULL`, `hex_end=NULL`, even-length exact hex | [x] |
| 3 | `sodium_hex2bin` | `ignore=": "` non-NULL — separators skipped only when `state==0` | [x] |
| 4 | `sodium_hex2bin` | `hex_end != NULL` — partial parse, stops at first bad char | [x] |
| 5 | `sodium_hex2bin` | `bin_len != NULL` vs NULL; `bin_maxlen` exactly sufficient vs generous | [x] |
| 6 | `sodium_bin2base64` | variant 1 (`ORIGINAL`): `+/` alphabet, `=` padding; `bin_len % 3` = 0/1/2 | [x] |
| 7 | `sodium_bin2base64` | variant 3 (`ORIGINAL_NO_PADDING`): `+/`, no padding | [x] |
| 8 | `sodium_bin2base64` | variant 5 (`URLSAFE`): `-_` alphabet, `=` padding | [x] |
| 9 | `sodium_bin2base64` | variant 7 (`URLSAFE_NO_PADDING`): `-_`, no padding | [x] |
| 10 | `sodium_base642bin` | variant 1: std table + `=` padding consumed by skip_padding | [x] |
| 11 | `sodium_base642bin` | variant 3: skip_padding NOT invoked | [x] |
| 12 | `sodium_base642bin` | variant 5: urlsafe table + padding | [x] |
| 13 | `sodium_base642bin` | variant 7: urlsafe table, no padding | [x] |
| 14 | `sodium_base642bin` | `ignore != NULL` — whitespace mid-stream, in padding, trailing | [x] |
| 15 | `sodium_base642bin` | `b64_end != NULL` (partial ok) vs NULL (full-length required); `bin_len` NULL vs set | [x] |
| 16 | `sodium_base64_encoded_len` | all 4 variants × `bin_len` 0..64 | [x] |
| 17 | `sodium_memcmp` | `len` 0/1/15/16/17/32/64/random; equal vs differing at each position | [x] |
| 18 | `sodium_compare` | `len` 0/1/2/8/16/32/64; little-endian lexicographic; −1/0/1 | [x] |
| 19 | `sodium_is_zero` | `nlen` 0/1/16/32/64; all-zero vs one nonzero byte at each index | [x] |
| 20 | `sodium_increment` | `nlen` 1/2/7/8/9/12/16/24/32 (asm fast paths OFF); all-`0xff` carry-out | [x] |
| 21 | `sodium_add` | `len` 1/2/7/8/9/12/16/24/32; full carry propagation | [x] |
| 22 | `sodium_sub` | `len` 1/2/8/16/24/32/64; full borrow propagation | [x] |
| 23 | `sodium_pad` | power-of-two `blocksize` (1,2,4,8,16,64) → mask path | [x] |
| 24 | `sodium_pad` | non-power-of-two `blocksize` (3,5,7,17,100) → modulo path | [x] |
| 25 | `sodium_pad` | `unpadded_buflen % blocksize == 0` (full extra block); `padded_buflen_p` NULL vs set | [x] |
| 26 | `sodium_unpad` | round-trip of every row 23–25 shape; `unpadded_buflen_p` NULL vs set | [x] |
| 27 | `sodium_ip2bin` | IPv4 dotted quad → IPv4-mapped (`bin[10..12]=0xff`, `bin[12..16]`=octets) | [x] |
| 28 | `sodium_ip2bin` | full 8-group IPv6 | [x] |
| 29 | `sodium_ip2bin` | IPv6 with `::` compression at start/middle/end | [x] |
| 30 | `sodium_ip2bin` | IPv6 with embedded IPv4 tail (`::ffff:1.2.3.4`) | [x] |
| 31 | `sodium_ip2bin` | IPv6 with `%zone` suffix (stripped before parse) | [x] |
| 32 | `sodium_bin2ip` | bin matching IPv4-mapped prefix → dotted-quad text | [x] |
| 33 | `sodium_bin2ip` | IPv6 with longest zero-run ≥ 2 → `::` compression | [x] |
| 34 | `sodium_bin2ip` | IPv6 with no zero-run ≥ 2 → full 8-group text | [x] |
| 35 | `crypto_verify_16` / `_32` / `_64` | equal, and differing at byte 0 / middle / last | [x] |
| 36 | `crypto_shorthash` / `_siphash24` | 8-byte output, 16-byte key, `inlen` 0/1/7/8/15/16/63/64/random | [x] |
| 37 | `crypto_shorthash_siphashx24` | 16-byte output (extra `v1 ^= 0xdd`), same `inlen` sweep | [x] |
| 38 | `crypto_onetimeauth` / `_poly1305` one-shot | `inlen` 0/1/15/16/17/31/32/64/1000/random | [x] |
| 39 | `crypto_onetimeauth_init` + `update`×0 + `final` | empty stream | [x] |
| 40 | `crypto_onetimeauth_init` + `update`×1 + `final` | single chunk | [x] |
| 41 | `crypto_onetimeauth_init` + `update`×N + `final` | N random chunk sizes incl. sub-16 leftovers, block-aligned splits | [x] |
| 42 | `crypto_onetimeauth_poly1305_init/update/final` | same as 39–41 via the primitive-named symbols | [x] |
| 43 | `crypto_onetimeauth_verify` / `_poly1305_verify` | correct tag (→0) | [x] |
| 44 | `crypto_onetimeauth_statebytes`, `_bytes`, `_keybytes`, `_primitive` | constant getters | [x] |
| 45 | `crypto_onetimeauth_keygen` / `_poly1305_keygen` | output length only (nondeterministic) | [x] |
| 46 | `crypto_ipcrypt_encrypt` / `_decrypt` (deterministic) | key 16, in 16, out 16; round-trip; random keys/inputs | [x] |
| 47 | `crypto_ipcrypt_nd_encrypt` / `_nd_decrypt` | key 16, tweak 8, in 16 → out 24; `out[0..8]==tweak`; round-trip | [x] |
| 48 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | key 32, tweak 16, in 16 → out 32 (XEX); degenerate-key rekey guard | [x] |
| 49 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | key 32, in 16; IPv4-mapped input → `prefix_start=96` branch | [x] |
| 50 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | key 32, in 16; non-IPv4-mapped → `prefix_start=0` (all 128 bits) branch | [x] |
| 51 | `crypto_ipcrypt_*_keygen` (4 fns) | output length only | [x] |
| 52 | `randombytes_buf_deterministic` | seed 32 B, `size` 0/1/63/64/65/1000/4096 — deterministic, must match C exactly | [x] |
| 53 | `randombytes_uniform` | `upper_bound` 0/1/2/3/255/256/65537/`UINT32_MAX` — range/bias behaviour | [x] |
| 54 | `randombytes_seedbytes`, `_implementation_name`, `_set_implementation`, `_stir`, `_close` | lifecycle/config getters | [x] |
| 55 | `sodium_version_string`, `_library_version_major`, `_minor`, `_library_minimal` | constant getters | [x] |
| 56 | `sodium_runtime_has_*` (12 fns) | all report 0 in this build | [x] |
| 57 | `sodium_malloc` / `sodium_allocarray` / `sodium_free` | fallback path: alloc, `0xdb` prefill, free | [x] |
| 58 | `sodium_memzero`, `sodium_stackzero` | length 0/1/64/random | [x] |
| 59 | `sodium_init` | idempotent re-entry | [x] |

## Group 2 — `crypto_aead/`, `crypto_secretbox/`, `crypto_secretstream/`, `crypto_box/`, `crypto_stream/`

The standard length sweep for this group is
`{0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 1000, 4096}`
plus random lengths, and the standard AD sweep is `{NULL/0, empty non-NULL, 1, 15, 16, 17, 31, 32, 64, random}`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| 60 | `crypto_aead_chacha20poly1305_encrypt` | NPUB=8; full length sweep × AD sweep; `clen_p` NULL vs set | [x] |
| 61 | `crypto_aead_chacha20poly1305_encrypt_detached` | mac written separately; `maclen_p` NULL vs set; `nsec` NULL | [x] |
| 62 | `crypto_aead_chacha20poly1305_decrypt` | `clen == 16` (empty msg) and full sweep; `mlen_p` NULL vs set | [x] |
| 63 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m != NULL` vs `m == NULL` (verify-only mode) | [x] |
| 64 | `crypto_aead_chacha20poly1305_ietf_encrypt` / `_detached` | NPUB=12; AD zero-padded to 16; 32-bit counter; length sweep | [x] |
| 65 | `crypto_aead_chacha20poly1305_ietf_decrypt` / `_detached` | length sweep; `m` NULL vs non-NULL | [x] |
| 66 | `crypto_aead_xchacha20poly1305_ietf_encrypt` / `_detached` | NPUB=24 → HChaCha20 subkey + `00000000‖npub[16..24]`; length sweep | [x] |
| 67 | `crypto_aead_xchacha20poly1305_ietf_decrypt` / `_detached` | length sweep; `m` NULL vs non-NULL | [x] |
| 68 | `crypto_aead_aegis128l_encrypt` / `_encrypt_detached` | K=16, NPUB=16, ABYTES=32, RATE=32; AD absorbed 2×RATE/RATE/partial | [x] |
| 69 | `crypto_aead_aegis128l_decrypt` / `_decrypt_detached` | `clen == 32` and sweep; partial final block via `declast`; `m` NULL | [x] |
| 70 | `crypto_aead_aegis256_encrypt` / `_encrypt_detached` | K=32, NPUB=32, ABYTES=32, RATE=16; AD/block/partial shapes | [x] |
| 71 | `crypto_aead_aegis256_decrypt` / `_decrypt_detached` | `clen == 32` and sweep; `m` NULL | [x] |
| 72 | `crypto_aead_aes256gcm_*` (9 fns) + `_is_available` | ENOSYS stubs — input shape irrelevant, both must return −1 / 0 | [x] |
| 73 | `crypto_secretbox_easy` | length sweep; mac at `c[0..16]` | [x] |
| 74 | `crypto_secretbox_detached` | `block0` first-64 B path (`mlen0 = min(mlen,32)`), then chunked `xor_ic` with `ic=1` | [x] |
| 75 | `crypto_secretbox_open_easy` | `clen == 16` (empty msg) and sweep | [x] |
| 76 | `crypto_secretbox_open_detached` | `m != NULL` vs `m == NULL` (verify-only) | [x] |
| 77 | `crypto_secretbox_xsalsa20poly1305` (padded low-level) | `mlen ≥ 32` with first 32 B zero-padded; `c[0..16]` must be 0 | [x] |
| 78 | `crypto_secretbox_xsalsa20poly1305_open` (padded low-level) | `clen ≥ 32`; `m[0..32]` must be 0 | [x] |
| 79 | `crypto_secretbox_xchacha20poly1305_easy` / `_detached` | HChaCha20 subkey from `n[0..16]`, stream nonce `n+16`; length sweep | [x] |
| 80 | `crypto_secretbox_xchacha20poly1305_open_easy` / `_open_detached` | length sweep; `m` NULL vs non-NULL | [x] |
| 81 | `crypto_secretstream_xchacha20poly1305_init_push` | 24 B header, HChaCha20 key, counter = 1, INONCE = header[16..24] | [x] |
| 82 | `crypto_secretstream_xchacha20poly1305_init_pull` | 24 B header consumed, same derivation | [x] |
| 83 | `..._push` `TAG_MESSAGE` (0x00) | AD absent / empty / non-empty; length sweep; no rekey | [x] |
| 84 | `..._push` `TAG_PUSH` (0x01) | REKEY bit clear → no auto rekey | [x] |
| 85 | `..._push` `TAG_REKEY` (0x02) | REKEY bit set → auto rekey (INONCE ^= mac, counter reset) | [x] |
| 86 | `..._push` `TAG_FINAL` (0x03 = PUSH\|REKEY) | REKEY bit set → auto rekey | [x] |
| 87 | `..._rekey` (explicit) | manual rekey mid-stream, then continue push/pull | [x] |
| 88 | `..._pull` | each of the 4 tags; `inlen == 17` (empty msg) and sweep; `tag_p`/`mlen_p` NULL vs set | [x] |
| 89 | `..._keygen`, `_statebytes`, `_abytes`, `_headerbytes`, `_keybytes`, `_messagebytes_max`, `_tag_*` | getters | [x] |
| 90 | multi-message stream | 1/2/10/50 messages with mixed tags, incl. a REKEY in the middle, full push→pull round trip | [x] |
| 91 | `crypto_box_keypair` / `crypto_box_seed_keypair` | seeded (deterministic, must match C) vs random (length only) | [x] |
| 92 | `crypto_box_easy` / `crypto_box_open_easy` | length sweep | [x] |
| 93 | `crypto_box_detached` / `crypto_box_open_detached` | mac separate; length sweep | [x] |
| 94 | `crypto_box_beforenm` + `crypto_box_easy_afternm` / `_open_easy_afternm` | precomputed key reuse | [x] |
| 95 | `crypto_box_detached_afternm` / `_open_detached_afternm` | precomputed + detached mac | [x] |
| 96 | `crypto_box_seal` / `crypto_box_seal_open` | ephemeral keypair, nonce = BLAKE2b(epk‖pk), SEALBYTES=48 | [x] |
| 97 | `crypto_box_curve25519xsalsa20poly1305` / `_open` (padded low-level) | `mlen/clen ≥ 32` zero-padded form | [x] |
| 98 | `crypto_box_curve25519xsalsa20poly1305_afternm` / `_open_afternm` | padded + precomputed | [x] |
| 99 | `crypto_box_curve25519xsalsa20poly1305_beforenm` / `_keypair` / `_seed_keypair` | low-level primitive names | [x] |
| 100 | `crypto_box_curve25519xchacha20poly1305_easy` / `_open_easy` (+`_afternm`) | HChaCha20 variant, length sweep | [x] |
| 101 | `crypto_box_curve25519xchacha20poly1305_detached` / `_open_detached` (+`_afternm`) | detached mac | [x] |
| 102 | `crypto_box_curve25519xchacha20poly1305_seal` / `_seal_open` | SEALBYTES=48 | [x] |
| 103 | `crypto_box_curve25519xchacha20poly1305_beforenm` / `_keypair` / `_seed_keypair` | low-level | [x] |
| 104 | `crypto_stream` / `_xor` (xsalsa20) | NONCEBYTES=24; keystream vs XOR; length sweep | [x] |
| 105 | `crypto_stream_salsa20` (keystream) | NONCEBYTES=8; length sweep incl. 0 early-return and 64-B boundaries | [x] |
| 106 | `crypto_stream_salsa20_xor` | `ic = 0` implicit; length sweep | [x] |
| 107 | `crypto_stream_salsa20_xor_ic` | explicit 64-bit `ic` = 0/1/2/`0xffffffff`/`0x100000000`/`UINT64_MAX-1`; length sweep | [x] |
| 108 | `crypto_stream_salsa2012` / `_xor` | 12 rounds; length sweep | [x] |
| 109 | `crypto_stream_salsa208` / `_xor` | 8 rounds; length sweep | [x] |
| 110 | `crypto_stream_xsalsa20` / `_xor` / `_xor_ic` | HSalsa20 subkey + `n+16`; 64-bit `ic` sweep | [x] |
| 111 | `crypto_stream_chacha20` / `_xor` / `_xor_ic` | NONCEBYTES=8; 64-bit `ic` (input[12],[13]); length sweep | [x] |
| 112 | `crypto_stream_chacha20_ietf` / `_ietf_xor` / `_ietf_xor_ic` | NONCEBYTES=12; 32-bit `ic`; `ic` at 0/1/`2^32-1-ceil(mlen/64)` (max legal) | [x] |
| 113 | `crypto_stream_chacha20_ietf_ext` / `_ietf_ext_xor_ic` | ietf layout, 32-bit `ic`, NO overflow check (used by xchacha/secretstream) | [x] |
| 114 | `crypto_stream_xchacha20` / `_xor` / `_xor_ic` | HChaCha20 subkey + `n+16`; 64-bit `ic` sweep | [x] |
| 115 | all `crypto_stream_*_keygen`, `*_keybytes`, `*_noncebytes`, `*_messagebytes_max`, `*_primitive` | getters | [x] |

## Group 3 — `crypto_sign/`, `crypto_scalarmult/`, `crypto_core/ed25519`, `crypto_kx/`, `crypto_kem/`

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| 116 | `crypto_scalarmult_curve25519` | random 32-B scalar (clamped internally) × random valid point | [x] |
| 117 | `crypto_scalarmult_curve25519_base` | random 32-B scalar, fixed basepoint | [x] |
| 118 | `crypto_scalarmult` / `_base` | generic aliases | [x] |
| 119 | `crypto_scalarmult_ed25519` | clamp=1 (`t[0]&=248; t[31]|=64; t[31]&=127`) × main-subgroup point | [x] |
| 120 | `crypto_scalarmult_ed25519_noclamp` | clamp=0 (`t[31]&=127` only) × main-subgroup point | [x] |
| 121 | `crypto_scalarmult_ed25519_base` | clamp=1, fixed basepoint | [x] |
| 122 | `crypto_scalarmult_ed25519_base_noclamp` | clamp=0, fixed basepoint | [x] |
| 123 | `crypto_scalarmult_ristretto255` | 32-B scalar (`t[31]&=127`) × valid ristretto point | [x] |
| 124 | `crypto_scalarmult_ristretto255_base` | 32-B scalar, fixed basepoint | [x] |
| 125 | `crypto_core_ed25519_is_valid_point` | valid main-subgroup points → 1 | [x] |
| 126 | `crypto_core_ed25519_add` / `_sub` | two valid on-curve points | [x] |
| 127 | `crypto_core_ed25519_random` (uses `ge25519_from_uniform` internally) | `UNIFORMBYTES=32`; `crypto_core_ed25519_from_uniform` / `_from_hash` are NOT exported by this build, so they are covered only through `_random` and `_from_string*` | [x] |
| 128 | `crypto_core_ristretto255_from_hash` | `HASHBYTES=64` random input, plus all-zero and all-`0xff` | [x] |
| 129 | `crypto_core_ed25519_scalar_random` | length only (nondeterministic) | [x] |
| 130 | `crypto_core_ed25519_scalar_invert` | nonzero 32-B scalar; verify `s * s⁻¹ ≡ 1` | [x] |
| 131 | `crypto_core_ed25519_scalar_negate` / `_complement` | 32-B scalar (void, never fails) | [x] |
| 132 | `crypto_core_ed25519_scalar_add` / `_sub` / `_mul` | two 32-B scalars (void) | [x] |
| 133 | `crypto_core_ed25519_scalar_reduce` | `NONREDUCEDSCALARBYTES=64` input → 32-B output | [x] |
| 134 | `crypto_core_ed25519_scalar_is_canonical` | canonical, L, L−1, L+1, all-`0xff` | [x] |
| 135 | `crypto_core_ristretto255_is_valid_point` | valid ristretto encodings → 1 | [x] |
| 136 | `crypto_core_ristretto255_add` / `_sub` | two valid ristretto points | [x] |
| 137 | `crypto_core_ristretto255_from_hash` | `HASHBYTES=64` random input (always 0) | [x] |
| 138 | `crypto_core_ristretto255_scalar_*` (random/invert/negate/complement/add/sub/mul/reduce/is_canonical) | same shapes as rows 129–134 | [x] |
| 139 | all `crypto_core_*_bytes/_scalarbytes/_uniformbytes/_hashbytes/_nonreducedscalarbytes` | getters | [x] |
| 140 | `crypto_sign_keypair` | random (length only) | [x] |
| 141 | `crypto_sign_seed_keypair` / `_ed25519_seed_keypair` | `SEEDBYTES=32` → deterministic pk(32)/sk(64) | [x] |
| 142 | `crypto_sign` (combined) | `mlen` 0/1/32/64/65/1000/random; `smlen_p` NULL vs set | [x] |
| 143 | `crypto_sign_detached` / `_ed25519_detached` | prehashed=0; `mlen` sweep; `siglen_p` NULL vs set | [x] |
| 144 | `crypto_sign_open` / `_ed25519_open` | valid `sm`; `m` NULL vs set; `mlen_p` NULL vs set | [x] |
| 145 | `crypto_sign_verify_detached` / `_ed25519_verify_detached` | valid sig, `mlen` sweep → 0 | [x] |
| 146 | `crypto_sign_ed25519ph_init` + `update`×0 + `final_create`/`final_verify` | empty prehashed message | [x] |
| 147 | `crypto_sign_ed25519ph_init` + `update`×1 + `final_create`/`final_verify` | single chunk | [x] |
| 148 | `crypto_sign_ed25519ph_init` + `update`×N + `final_create`/`final_verify` | N random chunks (SHA-512 block-boundary splits) | [x] |
| 149 | `crypto_sign_ed25519_sk_to_seed` / `_sk_to_pk` | 64-B sk → 32-B seed / 32-B pk | [x] |
| 150 | `crypto_sign_ed25519_pk_to_curve25519` | valid main-subgroup ed25519 pk → curve25519 pk | [x] |
| 151 | `crypto_sign_ed25519_sk_to_curve25519` | 64-B ed25519 sk → clamped curve25519 sk | [x] |
| 152 | `crypto_sign_ed25519_bytes/_seedbytes/_publickeybytes/_secretkeybytes/_messagebytes_max/_statebytes`, `crypto_sign_primitive` | getters | [x] |
| 153 | `crypto_kx_keypair` | random (length only) | [x] |
| 154 | `crypto_kx_seed_keypair` | `SEEDBYTES=32` → deterministic sk = generichash(seed), pk = base(sk) | [x] |
| 155 | `crypto_kx_client_session_keys` | both `rx` and `tx` set: rx = H[0..32], tx = H[32..64] | [x] |
| 156 | `crypto_kx_client_session_keys` | `rx = NULL` (→ rx := tx) and `tx = NULL` (→ tx := rx) fallbacks | [x] |
| 157 | `crypto_kx_server_session_keys` | both set: tx = H[0..32], rx = H[32..64] (mirrored vs client) | [x] |
| 158 | `crypto_kx_server_session_keys` | `rx = NULL` / `tx = NULL` fallbacks | [x] |
| 159 | client+server pair | full handshake — client rx == server tx and client tx == server rx | [x] |
| 160 | `crypto_kem_mlkem768_seed_keypair` | `SEEDBYTES=64` → deterministic pk 1184 / sk 2400 | [x] |
| 161 | `crypto_kem_mlkem768_keypair` | random (length only) | [x] |
| 162 | `crypto_kem_mlkem768_enc_deterministic` | canonical pk 1184 + 32-B coins → ct 1088, ss 32 | [x] |
| 163 | `crypto_kem_mlkem768_enc` | random coins (length only) + `dec` round-trip | [x] |
| 164 | `crypto_kem_mlkem768_dec` | valid ct 1088 + sk 2400 → ss 32 (matches enc) | [x] |
| 165 | `crypto_kem_mlkem768_dec` | *corrupted* ct → implicit rejection, still returns 0, C and Rust must give the SAME derived ss | [x] |
| 166 | `crypto_kem_xwing_seed_keypair` / `crypto_kem_seed_keypair` | `SEEDBYTES=32` → SHAKE256 96-B expand → pk 1216, sk 32 | [x] |
| 167 | `crypto_kem_xwing_keypair` / `crypto_kem_keypair` | random (length only) | [x] |
| 168 | `crypto_kem_xwing_enc_deterministic` | pk 1216 + 64-B seed (32 mlkem coins ‖ 32 x25519 eph sk) → ct 1120, ss 32 | [x] |
| 169 | `crypto_kem_xwing_enc` / `crypto_kem_enc` | random (round-trip via dec) | [x] |
| 170 | `crypto_kem_xwing_dec` / `crypto_kem_dec` | valid ct 1120 + sk 32 → ss 32; and corrupted ct → same derived ss in both | [x] |
| 171 | all `crypto_kem_*_publickeybytes/_secretkeybytes/_ciphertextbytes/_bytes/_seedbytes/_primitive` | getters | [x] |

## Group 4 — `crypto_pwhash/`, `crypto_generichash/`, `crypto_auth/`, `crypto_kdf/`, `crypto_hash/`, `crypto_xof/`, `crypto_core/{salsa,hsalsa20,hchacha20}`

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|---|
| 172 | `crypto_pwhash` | `alg = ARGON2I13 (1)`, `outlen=16`, `opslimit=3`, `memlimit=8192` | [x] |
| 173 | `crypto_pwhash` | `alg = ARGON2ID13 (2)`, `outlen=16`, `opslimit=1`, `memlimit=8192` | [x] |
| 174 | `crypto_pwhash_argon2i` | `outlen` 16/32/64, `opslimit` 3/4, `memlimit` 8192/16384; `passwdlen` 0/1/16/64 | [x] |
| 175 | `crypto_pwhash_argon2id` | `outlen` 16/32/64, `opslimit` 1/2/3, `memlimit` 8192/16384; `passwdlen` 0/1/16/64 | [x] |
| 176 | `crypto_pwhash_argon2i_str` + `_str_verify` + `_str_needs_rehash` | `$argon2i$v=19$m=8,t=3,p=1$…`; verify correct/incorrect password; rehash same/different params | [x] |
| 177 | `crypto_pwhash_argon2id_str` + `_str_verify` + `_str_needs_rehash` | `$argon2id$v=19$…`; same three checks | [x] |
| 178 | `crypto_pwhash_str` / `_str_alg` | default = argon2id; `_str_alg` with alg 1 and 2 | [x] |
| 179 | `crypto_pwhash_str_verify` / `_str_needs_rehash` | dispatch on `$argon2i$` vs `$argon2id$` prefix | [x] |
| 180 | `crypto_pwhash_scryptsalsa208sha256` | `outlen` 16/32/64; `opslimit`/`memlimit` in both `pickparams` branches (`opslimit < memlimit/32` and `≥`) | [x] |
| 181 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N`=2/4/16/1024, `r`=1/2/8, `p`=1/2/4; `buflen` 1/16/32/64/100 | [x] |
| 182 | `crypto_pwhash_scryptsalsa208sha256_str` + `_str_verify` + `_str_needs_rehash` | `$7$` format; verify correct/incorrect; rehash same/different | [x] |
| 183 | all `crypto_pwhash_*` getters (`_alg_argon2i13`, `_alg_argon2id13`, `_alg_default`, `_bytes_min/max`, `_passwd_min/max`, `_opslimit_*`, `_memlimit_*`, `_strbytes`, `_strprefix`, `_saltbytes`, `_primitive`) | constant getters | [x] |
| 184 | `crypto_generichash` / `_blake2b` one-shot | `key = NULL` (unkeyed), `inlen` 0/1/64/127/128/129/1000 | [x] |
| 185 | `crypto_generichash` / `_blake2b` one-shot | `keylen = 0` with non-NULL key (also unkeyed path) | [x] |
| 186 | `crypto_generichash` / `_blake2b` one-shot | `keylen` = 1 / 16 (`KEYBYTES_MIN`) / 32 (`KEYBYTES`) / 64 (`KEYBYTES_MAX`) | [x] |
| 187 | `crypto_generichash` / `_blake2b` one-shot | `outlen` = 1 / 16 (`BYTES_MIN`) / 32 (`BYTES`) / 64 (`BYTES_MAX`) | [x] |
| 188 | `crypto_generichash_blake2b_salt_personal` | salt 16 + personal 16 set (all-zero and random) × keyed and unkeyed | [x] |
| 189 | `crypto_generichash_init` + `update`×0 + `final` | empty message, each `outlen` from row 187 | [x] |
| 190 | `crypto_generichash_init` + `update`×1 + `final` | single chunk | [x] |
| 191 | `crypto_generichash_init` + `update`×N + `final` | N random chunks incl. 128-B block-boundary splits | [x] |
| 192 | `crypto_generichash_blake2b_init_salt_personal` + `update`×N + `final` | streaming with salt+personal, keyed and unkeyed | [x] |
| 193 | `crypto_generichash_keygen` / `_blake2b_keygen`, `_statebytes`, `_bytes*`, `_keybytes*`, `_saltbytes`, `_personalbytes`, `_primitive` | getters | [x] |
| 194 | `crypto_hash_sha256` one-shot | `inlen` 0/1/55/56/63/64/65/119/120/1000/random (SHA-256 padding boundaries) | [x] |
| 195 | `crypto_hash_sha256_init/update/final` | `update`×0, ×1, ×N with block-boundary splits | [x] |
| 196 | `crypto_hash_sha512` one-shot | `inlen` 0/1/111/112/127/128/129/239/240/1000/random | [x] |
| 197 | `crypto_hash_sha512_init/update/final` | `update`×0, ×1, ×N | [x] |
| 198 | `crypto_hash` / `_primitive` / `_bytes` / `_statebytes` | generic = sha512 | [x] |
| 199 | `crypto_hash_sha3_256` one-shot | rate 136; `inlen` 0/1/135/136/137/271/272/1000/random | [x] |
| 200 | `crypto_hash_sha3_256_init/update/final` | `update`×0, ×1, ×N; rate-boundary splits | [x] |
| 201 | `crypto_hash_sha3_512` one-shot | rate 72; `inlen` 0/1/71/72/73/143/144/1000/random | [x] |
| 202 | `crypto_hash_sha3_512_init/update/final` | `update`×0, ×1, ×N | [x] |
| 203 | `crypto_xof_shake128` one-shot | rate 168; `outlen` 0/1/32/167/168/169/335/336/1000; `inlen` sweep | [x] |
| 204 | `crypto_xof_shake128_init` + `update`×N + `squeeze`×1 | single large squeeze | [x] |
| 205 | `crypto_xof_shake128_init` + `update`×N + `squeeze`×M | incremental squeeze; concatenation must equal the single-squeeze output | [x] |
| 206 | `crypto_xof_shake128_init_with_domain` | custom domain byte vs default `0x1F` | [x] |
| 207 | `crypto_xof_shake256` (+`_init`, `_init_with_domain`, `_update`, `_squeeze`) | rate 136; same shapes as 203–206 | [x] |
| 208 | `crypto_xof_turboshake128` (+ streaming, `_init_with_domain`) | rate 168, 12-round permutation; same shapes | [x] |
| 209 | `crypto_xof_turboshake256` (+ streaming, `_init_with_domain`) | rate 136, 12-round permutation; same shapes | [x] |
| 210 | all `crypto_xof_*_rate/_statebytes/_domain*` and `crypto_hash_sha3_*_bytes/_statebytes` | getters | [x] |
| 211 | `crypto_auth_hmacsha256` one-shot + `_verify` | `keylen` = 32 (`KEYBYTES`); `inlen` sweep | [x] |
| 212 | `crypto_auth_hmacsha256_init` | `keylen` 0 / 1 / 32 / 63 / 64 (= block) / 65 / 128 (> block → prehashed) | [x] |
| 213 | `crypto_auth_hmacsha256_init/update/final` | `update`×0, ×1, ×N | [x] |
| 214 | `crypto_auth_hmacsha512` one-shot + `_verify` + streaming | block size 128; `keylen` 0/1/64/127/128/129/256 | [x] |
| 215 | `crypto_auth_hmacsha512256` one-shot + `_verify` + streaming | truncated-to-32 output; same key sweep | [x] |
| 216 | `crypto_auth` / `_verify` | generic = hmacsha512256 | [x] |
| 217 | all `crypto_auth_*_keygen`, `_bytes`, `_keybytes`, `_statebytes`, `_primitive` | getters | [x] |
| 218 | `crypto_kdf_blake2b_derive_from_key` / `crypto_kdf_derive_from_key` | `subkey_len` = 16 (`BYTES_MIN`) / 32 / 64 (`BYTES_MAX`) | [x] |
| 219 | `crypto_kdf_blake2b_derive_from_key` | `subkey_id` = 0 / 1 / `UINT64_MAX` (→ LE salt[0..8]); `ctx` 8 B varied | [x] |
| 220 | `crypto_kdf_hkdf_sha256_extract` (one-shot) | `salt` len 0/1/32/64; `ikm` len 0/1/32/64 → prk 32 | [x] |
| 221 | `crypto_kdf_hkdf_sha256_extract_init/update/final` | `update`×0, ×1, ×N; equals one-shot | [x] |
| 222 | `crypto_kdf_hkdf_sha256_expand` | `out_len` 0/1/32/33/64/8160 (`BYTES_MAX`); `ctx` len 0/1/32 | [x] |
| 223 | `crypto_kdf_hkdf_sha512_extract` (one-shot + streaming) | prk 64; same shapes as 220–221 | [x] |
| 224 | `crypto_kdf_hkdf_sha512_expand` | `out_len` 0/1/64/65/128/16320 (`BYTES_MAX`) | [x] |
| 225 | `crypto_kdf_hkdf_sha256_keygen` / `sha512_keygen`, `crypto_kdf_keygen`, and all `_bytes*/_keybytes/_contextbytes/_statebytes/_primitive` getters | getters | [x] |
| 226 | `crypto_core_salsa20` | `c != NULL` (constants loaded from `c`) — 20 rounds | [x] |
| 227 | `crypto_core_salsa20` | `c == NULL` (default `sigma`) | [x] |
| 228 | `crypto_core_salsa2012` | `c != NULL` and `c == NULL` — 12 rounds | [x] |
| 229 | `crypto_core_salsa208` | `c != NULL` and `c == NULL` — 8 rounds | [x] |
| 230 | `crypto_core_hsalsa20` | `c != NULL` and `c == NULL` — 32-B output | [x] |
| 231 | `crypto_core_hchacha20` | `c != NULL` and `c == NULL` — 32-B output | [x] |
| 232 | all `crypto_core_*_outputbytes/_inputbytes/_keybytes/_constbytes` | getters | [x] |

---

## Coverage — which test covers which rows

Every row is driven through the `.so` exports of BOTH libraries and every output
buffer plus return value is compared byte-for-byte, across many randomized
inputs per row (fixed seed `0x5EED_1234_ABCD_EF01`).

| rows | test file / function |
|---|---|
| 1–5 (hex) | `b1_sodium_core::row1_bin2hex`, `rows2_5_hex2bin` |
| 6–16 (base64, all 4 variants) | `b1_sodium_core::rows6_9_bin2base64`, `rows10_15_base642bin`, `row16_base64_encoded_len` |
| 17–22 (memcmp/compare/is_zero/increment/add/sub) | `b1_sodium_core::row17_sodium_memcmp` … `row22_sodium_sub` |
| 23–26 (pad/unpad) | `b1_sodium_core::rows23_26_pad_unpad` |
| 27–34 (ip2bin/bin2ip) | `b1_sodium_core::rows27_34_ip2bin_bin2ip` |
| 35 (crypto_verify_16/32/64) | `b1_sodium_core::row35_crypto_verify` |
| 36–37 (siphash24 / siphashx24) | `b1_sodium_core::rows36_37_shorthash` |
| 38–45 (poly1305, one-shot + streaming + verify) | `b1_sodium_core::rows38_45_onetimeauth` |
| 46–51 (ipcrypt det / nd / ndx / pfx) | `b1_sodium_core::rows46_51_ipcrypt` |
| 52–59 (randombytes, getters, malloc) | `b1_sodium_core::row52_randombytes_buf_deterministic`, `row53_randombytes_uniform`, `rows54_59_getters_and_misc` |
| 60–72 (AEAD, all 6 primitives) | `b2_aead_box::rows60_63_*` … `row72_aes256gcm_enosys_stubs` |
| 73–80 (secretbox easy/detached, both primitives) | `b2_aead_box::rows73_76_secretbox`, `rows79_80_secretbox_xchacha20poly1305` |
| 77–78 (padded low-level secretbox) | `b2_aead_box::rows77_78_secretbox_padded_lowlevel` |
| 81–90 (secretstream, all 4 tags + rekey) | `b2_aead_box::rows81_90_secretstream` |
| 91–103 (box, all 3 primitives + seal + afternm) | `b2_aead_box::rows91_99_crypto_box`, `rows100_103_box_curve25519xchacha20poly1305` |
| 104–115 (all 8 stream primitives + ic) | `b2_stream::*` |
| 116–124 (scalarmult: curve25519 / ed25519 ±clamp / ristretto255) | `b3_sign_kem::rows116_118_*`, `rows119_122_*`, `rows123_124_*` |
| 125–139 (core ed25519 + ristretto255, incl. every scalar op) | `b3_sign_kem::rows125_134_core_ed25519`, `rows135_139_core_ristretto255` |
| 140–152 (sign: combined/detached/open/verify + ed25519ph + conversions) | `b3_sign_kem::rows140_152_sign` |
| 153–159 (kx, incl. rx/tx NULL fallbacks + handshake) | `b3_sign_kem::rows153_159_kx` |
| 160–171 (kem mlkem768 + xwing + generic) | `b3_sign_kem::rows160_165_kem_mlkem768`, `rows166_171_kem_xwing` |
| 172–183 (pwhash argon2i/id + scrypt + strings) | `b4_hash_kdf_pwhash::rows172_179_pwhash_argon2`, `rows176_179_pwhash_str`, `rows180_182_scrypt`, `row183_pwhash_getters` |
| 184–193 (generichash / blake2b, keyed + salt/personal + streaming) | `b4_hash_kdf_pwhash::rows184_193_generichash` |
| 194–202 (sha256/sha512/sha3-256/sha3-512) | `b4_hash_kdf_pwhash::rows194_198_sha256_sha512`, `rows199_202_sha3` |
| 203–210 (shake128/256, turboshake128/256, multi-squeeze, custom domains) | `b4_hash_kdf_pwhash::rows203_207_shake`, `rows208_210_turboshake` |
| 211–217 (hmac ×3 + generic crypto_auth) | `b4_hash_kdf_pwhash::rows211_217_hmac` |
| 218–225 (kdf blake2b + hkdf sha256/sha512) | `b4_hash_kdf_pwhash::rows218_219_kdf_blake2b`, `rows220_225_hkdf` |
| 226–232 (core salsa20/2012/208, hsalsa20, hchacha20; `c` NULL and non-NULL) | `b4_hash_kdf_pwhash::rows226_232_core_salsa_hsalsa_hchacha` |
| additional | `c3_boundaries::phase_d_all_getters_agree` compares **356** zero-argument constant getters across the whole exported surface; `c3_boundaries::secretstream_out_of_range_tags` covers all 256 tag byte values |

**Every row passes across its randomized inputs. The Phase B gate is satisfied.**

## Notes / corrections found while testing

- There is **no** `crypto_secretbox_xchacha20poly1305` / `_open` padded
  (NaCl-style) entry point in this build. The padded form exists only as
  `crypto_secretbox_xsalsa20poly1305` / `_open` and the generic
  `crypto_secretbox` / `crypto_secretbox_open`. Rows 77–78 test those.
- `crypto_core_ed25519_from_uniform` and `crypto_core_ed25519_from_hash` are not
  exported by this build (by either `.so`). The `ge25519_from_uniform` /
  `_from_hash` code paths are reached through `crypto_core_ed25519_random` and
  `crypto_core_ed25519_from_string{,_nu}` instead.
- `crypto_kem_enc_deterministic` does not exist on the generic `crypto_kem`
  façade (only on `crypto_kem_mlkem768_*` and `crypto_kem_xwing_*`).
- `crypto_hash` (generic) exposes only the one-shot entry point — there is no
  `crypto_hash_init/update/final/statebytes`.
- Functions whose output is nondeterministic (`*_keypair`, `*_keygen`,
  `crypto_box_seal`, `crypto_pwhash_*_str`, `crypto_secretstream_*_init_push`,
  `crypto_core_*_random`, `randombytes_*`) cannot be compared byte-for-byte
  directly. They are verified by **cross-consumption** instead: the C output is
  fed to the Rust implementation and vice-versa, and the deterministic
  counterpart (`*_seed_keypair`, `*_enc_deterministic`, `init_pull` on the
  C-generated header) is compared byte-for-byte.
