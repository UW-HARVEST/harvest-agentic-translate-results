| G3 | crypto_generichash | one-shot, unkeyed (`key=NULL, keylen=0`), `outlen=32` (BYTES), `inlen=0` (empty input, `in=NULL`) | [ ] |
| G3 | crypto_generichash | one-shot, unkeyed, `outlen=32`, `inlen=1` | [ ] |
| G3 | crypto_generichash | one-shot, unkeyed, `outlen=16` (BYTES_MIN) | [ ] |
| G3 | crypto_generichash | one-shot, unkeyed, `outlen=64` (BYTES_MAX) | [ ] |
| G3 | crypto_generichash | one-shot, unkeyed, `outlen` swept over every value 1..64 (incl. non-multiples 17, 23, 31, 33, 47, 63) | [ ] |
| G3 | crypto_generichash | one-shot, keyed, `keylen=32` (KEYBYTES), `outlen=32` | [ ] |
| G3 | crypto_generichash | one-shot, keyed, `keylen` swept over every value 1..64 (incl. 15, 16=KEYBYTES_MIN, 17, 31, 33, 63, 64=KEYBYTES_MAX), `outlen=32` | [ ] |
| G3 | crypto_generichash | one-shot, `key != NULL` but `keylen=0` -> silently takes the unkeyed path (must match unkeyed digest) | [ ] |
| G3 | crypto_generichash | one-shot, keyed, `inlen=0` (keyed hash of the empty message) | [ ] |
| G3 | crypto_generichash_blake2b | one-shot, same sweeps as crypto_generichash (must be bit-identical to the generic wrapper) | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, single `_update` with `inlen=0` | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, no `_update` call at all between init and final | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, input 256 bytes fed 1 byte per `_update` (256 calls) | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, `inlen=127` in one update (just under BLAKE2B_BLOCKBYTES=128) | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, `inlen=128` in one update (exactly one block; exercises the lazy "do not compress" path) | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, `inlen=129` in one update | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, `inlen=255` / `256` / `257` (2*BLOCKBYTES boundary, exercises the `inlen > fill` shift-buffer path) | [ ] |
| G3 | crypto_generichash_init/_update/_final | unkeyed, `outlen=32`, 300-byte input split as 127+1, 1+127, 128+128, 64+64+64+64, 100+200 | [ ] |
| G3 | crypto_generichash_init/_update/_final | keyed `keylen=32`, `outlen=32` — note `_init` pre-absorbs a full zero-padded 128-byte key block, so the block-boundary offsets shift by 128 | [ ] |
| G3 | crypto_generichash_init/_update/_final | keyed `keylen=16` and `keylen=64`, `outlen=64`, chunked update at 127/128/129 | [ ] |
| G3 | crypto_generichash_init/_update/_final | streaming result vs one-shot `crypto_generichash` result must be identical for each (keylen, outlen, split) triple | [ ] |
| G3 | crypto_generichash_blake2b_init/_update/_final | same streaming matrix as the generic wrapper | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, `salt=NULL, personal=NULL` (must equal plain `crypto_generichash_blake2b`) | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, `salt` = 16 non-zero bytes, `personal=NULL` | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, `salt=NULL`, `personal` = 16 non-zero bytes | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, both `salt` and `personal` non-NULL, 16 bytes each (SALTBYTES=PERSONALBYTES=16) | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, all-zero 16-byte `salt`+`personal` (must equal the NULL/NULL case) | [ ] |
| G3 | crypto_generichash_blake2b_salt_personal | one-shot, keyed (`keylen=32`) + salt + personal, `outlen` in {16,32,64} | [ ] |
| G3 | crypto_generichash_blake2b_init_salt_personal | init/update/final, `salt=NULL, personal=NULL`, unkeyed, `outlen=32` | [ ] |
| G3 | crypto_generichash_blake2b_init_salt_personal | init/update/final, non-NULL salt+personal, unkeyed, chunked update 127/128/129 | [ ] |
| G3 | crypto_generichash_blake2b_init_salt_personal | init/update/final, non-NULL salt+personal, keyed `keylen` in {16,32,64}, `outlen` in {16,32,64} | [ ] |
| G3 | crypto_generichash_blake2b_init_salt_personal + _final | streaming salt/personal result must match one-shot `_salt_personal` | [ ] |
| G3 | crypto_generichash_statebytes / crypto_generichash_blake2b_statebytes | returns `(sizeof(state) + 63) & ~63` — 64-byte-rounded value of the 384-byte opaque state (expect 384) | [ ] |
| G3 | crypto_generichash_bytes_min/_bytes_max/_bytes/_keybytes_min/_keybytes_max/_keybytes/_primitive | constant accessors: 16, 64, 32, 16, 64, 32, "blake2b" | [ ] |
| G3 | crypto_generichash_blake2b_bytes_min/_bytes_max/_bytes/_keybytes_min/_keybytes_max/_keybytes/_saltbytes/_personalbytes | 16, 64, 32, 16, 64, 32, 16, 16 | [ ] |
| G3 | crypto_generichash_keygen / crypto_generichash_blake2b_keygen | fills 32 bytes; verify subsequent keyed hash round-trips | [ ] |
| G3 | _crypto_generichash_blake2b_pick_best_implementation | switches the `blake2b_compress` fn pointer (ref / ssse3 / sse41 / avx2) — all variants must produce identical digests for the full outlen/keylen/split matrix | [ ] |
| G3 | crypto_hash_sha256 | one-shot, `inlen=0` (empty) | [ ] |
| G3 | crypto_hash_sha256 | one-shot, `inlen` swept 0..200 (covers padding boundaries 55, 56, 63, 64, 119, 120, 127, 128) | [ ] |
| G3 | crypto_hash_sha256 | one-shot, `inlen=1000` (multi-block, >> 64) | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | no update at all (digest of empty message) | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | `_update` with `inlen=0` (early-return path, `in=NULL` allowed) | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | 200-byte input fed 1 byte per `_update` | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | splits landing exactly on the 64-byte block boundary: 64+64, 63+1, 1+63, 55+9, 56+8 | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | split where the first chunk fills part of the buffer and the second spans several blocks: 10+190 | [ ] |
| G3 | crypto_hash_sha256_init/_update/_final | streaming result identical to one-shot for every split | [ ] |
| G3 | crypto_hash_sha256_bytes / crypto_hash_sha256_statebytes | 32; sizeof(crypto_hash_sha256_state) | [ ] |
| G3 | crypto_hash_sha512 | one-shot, `inlen=0` (empty) | [ ] |
| G3 | crypto_hash_sha512 | one-shot, `inlen` swept 0..300 (covers padding boundaries 111, 112, 127, 128, 239, 240, 255, 256) | [ ] |
| G3 | crypto_hash_sha512 | one-shot, `inlen=2000` (multi-block, >> 128) | [ ] |
| G3 | crypto_hash_sha512_init/_update/_final | no update at all; `_update` with `inlen=0` | [ ] |
| G3 | crypto_hash_sha512_init/_update/_final | 300-byte input fed 1 byte per `_update` | [ ] |
| G3 | crypto_hash_sha512_init/_update/_final | splits on the 128-byte block boundary: 128+128, 127+1, 1+127, 111+17, 112+16 | [ ] |
| G3 | crypto_hash_sha512_init/_update/_final | streaming result identical to one-shot for every split | [ ] |
| G3 | crypto_hash_sha512_bytes / crypto_hash_sha512_statebytes | 64; sizeof(crypto_hash_sha512_state) | [ ] |
| G3 | crypto_hash / crypto_hash_bytes / crypto_hash_primitive | default alias for sha512: 64, "sha512"; digest must equal `crypto_hash_sha512` | [ ] |
| G3 | crypto_hash_sha3256 | one-shot, `inlen=0` (empty) | [ ] |
| G3 | crypto_hash_sha3256 | one-shot, `inlen` swept 0..300 covering the rate-136 boundaries: 134, 135 (`offset == rate-1` single-byte pad special case), 136, 137, 271, 272, 273 | [ ] |
| G3 | crypto_hash_sha3256 | one-shot, `inlen=1000` (multi-permutation) | [ ] |
| G3 | crypto_hash_sha3256_init/_update/_final | no update; `_update` with `inlen=0` | [ ] |
| G3 | crypto_hash_sha3256_init/_update/_final | 1 byte per `_update` for a 300-byte message | [ ] |
| G3 | crypto_hash_sha3256_init/_update/_final | splits exercising the `offset != 0` partial-chunk path: 100+36, 135+1, 136+1, 1+135, 136+136, 50+150 | [ ] |
| G3 | crypto_hash_sha3256_init/_update/_final | split leaving `offset == rate` exactly at the last update (final's `offset == rate` pre-permute branch): total input a multiple of 136 | [ ] |
| G3 | crypto_hash_sha3256_bytes / _statebytes | 32; sizeof(crypto_hash_sha3256_state) = 256 | [ ] |
| G3 | crypto_hash_sha3512 | one-shot, `inlen=0` (empty) | [ ] |
| G3 | crypto_hash_sha3512 | one-shot, `inlen` swept 0..200 covering the rate-72 boundaries: 70, 71 (single-byte pad special case), 72, 73, 143, 144, 145 | [ ] |
| G3 | crypto_hash_sha3512 | one-shot, `inlen=1000` | [ ] |
| G3 | crypto_hash_sha3512_init/_update/_final | no update; `inlen=0` update; 1-byte-at-a-time; splits 71+1, 72+1, 1+71, 72+72, 30+42 | [ ] |
| G3 | crypto_hash_sha3512_bytes / _statebytes | 64; sizeof(crypto_hash_sha3512_state) = 256 | [ ] |
| G3 | crypto_hash_sha3256 / crypto_hash_sha3512 | streaming result identical to one-shot for every split (SHA3 domain byte 0x06) | [ ] |
| G3 | crypto_xof_shake128 | one-shot, `inlen=0`, `outlen=32` | [ ] |
| G3 | crypto_xof_shake128 | one-shot, `outlen` swept 1..512 covering the rate-168 boundaries 167, 168, 169, 335, 336, 337 | [ ] |
| G3 | crypto_xof_shake128 | one-shot, `inlen` swept 0..400 covering absorb boundaries 166, 167 (`offset == rate-1` pad case), 168, 169, 335, 336, 337 | [ ] |
| G3 | crypto_xof_shake128 | one-shot, `outlen=0` (accepted, writes nothing) | [ ] |
| G3 | crypto_xof_shake128_init + _update + _squeeze | init (standard domain 0x1F), single update, single squeeze `outlen=32`; must match one-shot | [ ] |
| G3 | crypto_xof_shake128_init + _squeeze | squeeze with no update at all (XOF of the empty message) | [ ] |
| G3 | crypto_xof_shake128_init + _update + _squeeze | 1 byte per `_update` for a 400-byte message | [ ] |
| G3 | crypto_xof_shake128_init + _update + _squeeze | update splits 167+1, 168+1, 1+167, 168+168, 100+68 | [ ] |
| G3 | crypto_xof_shake128_..._squeeze | multi-squeeze: 512 bytes squeezed 1 byte at a time; must equal one 512-byte squeeze | [ ] |
| G3 | crypto_xof_shake128_..._squeeze | multi-squeeze splits 167+1, 168+1, 1+167, 168+168, 100+412, 336+176 (exercises `offset == rate` re-permute and partial-chunk paths) | [ ] |
| G3 | crypto_xof_shake128_..._squeeze | squeeze `outlen=0` between two non-zero squeezes (must be a no-op) | [ ] |
| G3 | crypto_xof_shake128_init_with_domain | `domain = 0x1F` (standard) must equal `crypto_xof_shake128_init` | [ ] |
| G3 | crypto_xof_shake128_init_with_domain | `domain` swept over 0x01..0x7F, plus 0x06 (SHA3), 0x1F (SHAKE), 0x0B, 0x7F | [ ] |
| G3 | crypto_xof_shake128_init_with_domain | `domain` with input length such that `offset == rate-1` at finalize (`inlen % 168 == 167`) — hits the `domain ^ 0x80` single-byte pad branch | [ ] |
| G3 | crypto_xof_shake128_blockbytes / _statebytes / _domain_standard | 168; 256; 0x1F | [ ] |
| G3 | crypto_xof_shake256 | one-shot, `inlen=0`, `outlen=64`; `outlen` swept 1..512 covering rate-136 boundaries 135, 136, 137, 271, 272, 273 | [ ] |
| G3 | crypto_xof_shake256 | one-shot, `inlen` swept 0..400 covering 134, 135, 136, 137, 271, 272, 273 | [ ] |
| G3 | crypto_xof_shake256_init/_update/_squeeze | no-update squeeze; 1-byte updates; update splits 135+1, 136+1, 1+135, 136+136 | [ ] |
| G3 | crypto_xof_shake256_..._squeeze | multi-squeeze 1 byte at a time and splits 135+1, 136+1, 136+136, 272+240; must equal a single squeeze | [ ] |
| G3 | crypto_xof_shake256_init_with_domain | `domain` swept 0x01..0x7F; `inlen % 136 == 135` to hit the single-byte pad branch | [ ] |
| G3 | crypto_xof_shake256_blockbytes / _statebytes / _domain_standard | 136; 256; 0x1F | [ ] |
| G3 | crypto_xof_turboshake128 | one-shot (12-round permutation), `inlen=0`, `outlen=32`; `outlen` swept 1..512 across rate-168 boundaries 167, 168, 169, 336 | [ ] |
| G3 | crypto_xof_turboshake128 | one-shot, `inlen` swept 0..400 across 166, 167, 168, 169, 336, 337 | [ ] |
| G3 | crypto_xof_turboshake128_init/_update/_squeeze | no-update squeeze; 1-byte updates; update splits 167+1, 168+1, 1+167, 168+168 | [ ] |
| G3 | crypto_xof_turboshake128_..._squeeze | multi-squeeze 1 byte at a time and splits 167+1, 168+1, 168+168, 336+176 | [ ] |
| G3 | crypto_xof_turboshake128_init_with_domain | `domain` swept over the full documented TurboSHAKE separation range 0x01..0x7F (128 distinct configs) | [ ] |
| G3 | crypto_xof_turboshake128_init_with_domain | `domain` at each endpoint (0x01, 0x02, 0x1F, 0x7F) combined with `inlen % 168 == 167` (single-byte `domain ^ 0x80` pad branch) | [ ] |
| G3 | crypto_xof_turboshake128_blockbytes / _statebytes / _domain_standard | 168; 256; 0x1F | [ ] |
| G3 | crypto_xof_turboshake256 | one-shot, `inlen=0`, `outlen=64`; `outlen` swept 1..512 across rate-136 boundaries 135, 136, 137, 272 | [ ] |
| G3 | crypto_xof_turboshake256 | one-shot, `inlen` swept 0..400 across 134, 135, 136, 137, 272, 273 | [ ] |
| G3 | crypto_xof_turboshake256_init/_update/_squeeze | no-update squeeze; 1-byte updates; update splits 135+1, 136+1, 1+135, 136+136 | [ ] |
| G3 | crypto_xof_turboshake256_..._squeeze | multi-squeeze 1 byte at a time and splits 135+1, 136+1, 136+136, 272+240 | [ ] |
| G3 | crypto_xof_turboshake256_init_with_domain | `domain` swept 0x01..0x7F; endpoints combined with `inlen % 136 == 135` | [ ] |
| G3 | crypto_xof_turboshake256_blockbytes / _statebytes / _domain_standard | 136; 256; 0x1F | [ ] |
| G3 | crypto_core_keccak1600_init + _permute_24 + _extract_bytes | init to all-zero, permute 24 rounds, extract 200 bytes (raw permutation-of-zero test vector) | [ ] |
| G3 | crypto_core_keccak1600_init + _permute_12 + _extract_bytes | init, permute 12 rounds (TurboSHAKE variant), extract 200 bytes | [ ] |
| G3 | crypto_core_keccak1600_xor_bytes | `offset=0, length=200` (whole state); `offset=0, length=0`; `offset=199, length=1`; `offset=7, length=1` (unaligned single byte) | [ ] |
| G3 | crypto_core_keccak1600_xor_bytes | called twice at the same offset (XOR accumulates, must not overwrite) | [ ] |
| G3 | crypto_core_keccak1600_xor_bytes | unaligned span crossing a 64-bit lane boundary: `offset=5, length=11` | [ ] |
| G3 | crypto_core_keccak1600_extract_bytes | `offset=0, length=200`; `offset=0, length=0`; `offset=199, length=1`; `offset=3, length=13` (unaligned) | [ ] |
| G3 | crypto_core_keccak1600_permute_24 / _permute_12 | repeated application (e.g. 5x) on the same state — sequence must match reference | [ ] |
| G3 | crypto_core_keccak1600_statebytes | sizeof(crypto_core_keccak1600_state) (200-byte lane array + alignment) | [ ] |
| G3 | keccak1600_ref_* vs keccak1600_armsha3_* | the two `crypto_core_keccak1600` backends selected by `__ARM_FEATURE_SHA3` must produce identical permutation output | [ ] |
| G3 | crypto_auth_hmacsha256 | one-shot, 32-byte key (KEYBYTES), `inlen=0` | [ ] |
| G3 | crypto_auth_hmacsha256 | one-shot, 32-byte key, `inlen` swept 0..200 across the sha256 block boundaries 55, 56, 63, 64, 128 | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen=32`, single update; must match the one-shot MAC | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen=0` with `key=NULL` (all-zero HMAC key, accepted) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen` swept 1..64 (shorter than the 64-byte HMAC block; zero-padded ipad/opad path) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen=64` exactly (block-sized key, no hashing) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen=65` (first size that triggers the `keylen > 64` key-pre-hashing path: key := SHA-256(key), keylen := 32) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | `keylen` in {65, 100, 128, 200, 1000} (long-key hashing path, incl. keys longer than one sha256 block) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | long key K (>64) must give the same MAC as short key SHA-256(K) | [ ] |
| G3 | crypto_auth_hmacsha256_init/_update/_final | multi-chunk updates: 1 byte at a time; splits 63+1, 64+1, 1+63, 64+64 | [ ] |
| G3 | crypto_auth_hmacsha256_verify | matching MAC (returns 0) | [ ] |
| G3 | crypto_auth_hmacsha256_verify | mismatching MAC: flip bit 0 of byte 0; flip bit 7 of byte 31; all-zero `h`; correct MAC with wrong key; correct MAC with a 1-byte-different message | [ ] |
| G3 | crypto_auth_hmacsha256_keygen | fills 32 bytes; MAC + verify round-trip with the generated key | [ ] |
| G3 | crypto_auth_hmacsha256_bytes / _keybytes / _statebytes | 32; 32; sizeof(crypto_auth_hmacsha256_state) | [ ] |
| G3 | crypto_auth_hmacsha512 | one-shot, 32-byte key, `inlen=0`; `inlen` swept 0..300 across sha512 boundaries 111, 112, 127, 128, 256 | [ ] |
| G3 | crypto_auth_hmacsha512_init/_update/_final | `keylen=0` (NULL key); `keylen` swept 1..128; `keylen=128` exactly (block-sized) | [ ] |
| G3 | crypto_auth_hmacsha512_init/_update/_final | `keylen=129` (first size triggering the `keylen > 128` pre-hash: key := SHA-512(key), keylen := 64) | [ ] |
| G3 | crypto_auth_hmacsha512_init/_update/_final | `keylen` in {129, 200, 256, 1000} (long-key hashing path); long key K must match short key SHA-512(K) | [ ] |
| G3 | crypto_auth_hmacsha512_init/_update/_final | multi-chunk updates: 1 byte at a time; splits 127+1, 128+1, 1+127, 128+128 | [ ] |
| G3 | crypto_auth_hmacsha512_verify | matching MAC (0) and mismatching 64-byte MAC (bit flips at byte 0 and byte 63, all-zero `h`, wrong key, wrong message) | [ ] |
| G3 | crypto_auth_hmacsha512_keygen / _bytes / _keybytes / _statebytes | keygen 32 bytes; 64; 32; sizeof(state) | [ ] |
| G3 | crypto_auth_hmacsha512256 | one-shot, 32-byte key, `inlen=0`; `inlen` swept 0..300 across 111, 112, 127, 128 | [ ] |
| G3 | crypto_auth_hmacsha512256_init/_update/_final | `keylen=0`, 1..128, 128, 129, 200 (delegates to hmacsha512, so the >128 pre-hash path applies) | [ ] |
| G3 | crypto_auth_hmacsha512256_final | output is the first 32 bytes of the corresponding hmacsha512 MAC for the same key/message | [ ] |
| G3 | crypto_auth_hmacsha512256_init/_update/_final | multi-chunk updates: 1 byte at a time; splits 127+1, 128+1, 128+128 | [ ] |
| G3 | crypto_auth_hmacsha512256_verify | matching (0) and mismatching 32-byte MAC (bit flips at bytes 0 and 31, all-zero `h`, wrong key, wrong message) | [ ] |
| G3 | crypto_auth_hmacsha512256_keygen / _bytes / _keybytes / _statebytes | keygen 32 bytes; 32; 32; sizeof(state) | [ ] |
| G3 | crypto_auth / crypto_auth_verify | default primitive alias for hmacsha512256: MAC must equal `crypto_auth_hmacsha512256`; verify matching + mismatching | [ ] |
| G3 | crypto_auth_bytes / _keybytes / _primitive / crypto_auth_keygen | 32; 32; "hmacsha512256"; keygen 32 bytes | [ ] |
| G3 | crypto_onetimeauth_poly1305 | one-shot, 32-byte key, `inlen=0` (empty message) | [ ] |
| G3 | crypto_onetimeauth_poly1305 | one-shot, `inlen` swept 0..64 covering the 16-byte block boundaries 15, 16, 17, 31, 32, 33, 47, 48, 63, 64 | [ ] |
| G3 | crypto_onetimeauth_poly1305 | one-shot, `inlen=1024` (many full blocks) | [ ] |
| G3 | crypto_onetimeauth_poly1305 | one-shot, key with `r` clamped bits all set (0xff...) and all-zero key (edge accumulator values) | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | single update; must match the one-shot tag | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | no update at all (tag of the empty message) | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | `_update` with `inlen=0` (leftover unchanged) | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | 64-byte message fed 1 byte per `_update` (exercises the `leftover` fill/flush path 16 times) | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | splits straddling the 16-byte block: 15+1, 16+1, 1+15, 1+31, 8+8, 8+24, 16+16, 17+15, 31+33 | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | split where the second chunk exactly completes the leftover buffer (`want == block_size - leftover`), e.g. 5+11 | [ ] |
| G3 | crypto_onetimeauth_poly1305_init/_update/_final | split where `leftover` remains partially filled at `_final` (e.g. 20+5 -> 9 leftover bytes: partial-final-block pad path) | [ ] |
| G3 | crypto_onetimeauth_poly1305_verify | matching 16-byte tag (0) | [ ] |
| G3 | crypto_onetimeauth_poly1305_verify | mismatching tag: bit flip at byte 0; bit flip at byte 15; all-zero `h`; correct tag with wrong key; correct tag with a 1-byte-different message | [ ] |
| G3 | crypto_onetimeauth_poly1305_keygen | fills 32 bytes; tag + verify round-trip | [ ] |
| G3 | crypto_onetimeauth_poly1305_bytes / _keybytes / _statebytes | 16; 32; sizeof(crypto_onetimeauth_poly1305_state) | [ ] |
| G3 | crypto_onetimeauth_poly1305_donna_implementation (exported struct) | call each of the 5 function pointers (`onetimeauth`, `onetimeauth_verify`, `onetimeauth_init`, `onetimeauth_update`, `onetimeauth_final`) directly; results must match the dispatched public API | [ ] |
| G3 | _crypto_onetimeauth_poly1305_pick_best_implementation | donna vs sse2 backend selection: both must produce identical tags for the full inlen/split matrix | [ ] |
| G3 | crypto_onetimeauth / _verify / _init / _update / _final / _bytes / _keybytes / _statebytes / _primitive / _keygen | default alias for poly1305; must be bit-identical to the poly1305 entry points; "poly1305" | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len=16` (BYTES_MIN), `subkey_id=0`, 8-byte ctx, 32-byte key | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len=32`, `subkey_id=0` | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len=64` (BYTES_MAX), `subkey_id=0` | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_len` swept over every value 16..64 (incl. non-multiples 17, 23, 31, 33, 63) | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | `subkey_id` in {0, 1, 2, 255, 256, 0xFFFFFFFF, 0x100000000, UINT64_MAX} (stored little-endian in the blake2b salt) | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | ctx (CONTEXTBYTES=8) all-zero; all-0xFF; ASCII "context1"; ctx with embedded NUL bytes (not NUL-terminated semantics) | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | two different ctx values with the same key+subkey_id must give different subkeys (domain separation) | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | result must equal `crypto_generichash_blake2b_salt_personal(subkey, len, NULL, 0, key, 32, salt=LE64(id)||zeros16, personal=ctx||zeros8)` | [ ] |
| G3 | crypto_kdf_blake2b_derive_from_key | key all-zero and key all-0xFF, 32 bytes (KEYBYTES) | [ ] |
| G3 | crypto_kdf_derive_from_key (wrapper) | same `subkey_len` / `subkey_id` / ctx matrix; must be identical to the blake2b entry point | [ ] |
| G3 | crypto_kdf_bytes_min / _bytes_max / _contextbytes / _keybytes / _primitive / crypto_kdf_keygen | 16; 64; 8; 32; "blake2b"; keygen 32 bytes | [ ] |
| G3 | crypto_kdf_blake2b_bytes_min / _bytes_max / _contextbytes / _keybytes | 16; 64; 8; 32 | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract | `salt=NULL, salt_len=0`, `ikm_len=32` (RFC 5869 "no salt" case) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract | `salt_len` in {0, 1, 13, 32, 63, 64} (up to the 64-byte HMAC block) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract | `salt_len=65` and `salt_len=200` (long-salt HMAC key-pre-hash path) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract | `ikm_len` in {0, 1, 22, 32, 64, 80, 1000} | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract_init/_extract_update/_extract_final | streaming extract with a single `_extract_update`; must match the one-shot `_extract` | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract_init/_extract_update/_extract_final | no `_extract_update` at all (empty IKM) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract_init/_extract_update/_extract_final | IKM fed as multiple `_extract_update` calls: 1 byte at a time; splits 63+1, 64+1, 1+63, 64+64 | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract_final | writes exactly KEYBYTES=32 bytes and zeroizes the state; `_statebytes` == sizeof(crypto_auth_hmacsha256_state) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=1` (single partial block, `left` path only, `i == 0`) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=31` (one partial block just under BYTES=32) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=32` (exactly one full block, `left == 0`, single loop iteration) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=33` (one full block + 1-byte partial: exercises both loop and `left` branch with T(1) chaining) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=64`, `96`, `128` (multi-block expand, counter 1..N, exact multiples) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=100` (3 full blocks + 4-byte remainder) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=8160` (BYTES_MAX = 0xff*32, counter reaches 255 — maximum multi-block expand) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=8159` (BYTES_MAX-1: 254 full blocks + 31-byte partial) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `out_len=0` (BYTES_MIN; loop and `left` both skipped, writes nothing) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `ctx_len=0` (`ctx=NULL` allowed since `_update` with len 0 short-circuits) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | `ctx_len` in {0, 1, 8, 10, 32, 64, 100, 1000} with the same prk/out_len | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | ctx containing embedded NUL bytes and non-ASCII bytes (`ctx` is length-delimited, not a C string) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_expand | prk all-zero and all-0xFF, 32 bytes (KEYBYTES) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_extract + _expand | full RFC 5869 SHA-256 test-vector chain (extract then expand) | [ ] |
| G3 | crypto_kdf_hkdf_sha256_keygen | fills 32 bytes; expand round-trip with the generated prk | [ ] |
| G3 | crypto_kdf_hkdf_sha256_keybytes / _bytes_min / _bytes_max / _statebytes | 32; 0; 8160; sizeof(crypto_kdf_hkdf_sha256_state) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract | `salt=NULL, salt_len=0`; `salt_len` in {0, 1, 13, 64, 127, 128} (up to the 128-byte HMAC block) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract | `salt_len=129` and `salt_len=300` (long-salt HMAC key-pre-hash path) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract | `ikm_len` in {0, 1, 22, 64, 128, 160, 1000} | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract_init/_extract_update/_extract_final | streaming extract, single update; no update; 1 byte at a time; splits 127+1, 128+1, 1+127, 128+128 | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract_final | writes exactly KEYBYTES=64 bytes and zeroizes the state; `_statebytes` == sizeof(crypto_auth_hmacsha512_state) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_expand | `out_len` in {0, 1, 63, 64, 65, 128, 192, 200} (partial-only, exact-block, block+partial, multi-block) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_expand | `out_len=16320` (BYTES_MAX = 0xff*64, counter reaches 255) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_expand | `out_len=16319` (BYTES_MAX-1: 254 full blocks + 63-byte partial) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_expand | `ctx_len` in {0, 1, 8, 64, 128, 1000}; ctx with embedded NULs | [ ] |
| G3 | crypto_kdf_hkdf_sha512_expand | prk all-zero and all-0xFF, 64 bytes (KEYBYTES) | [ ] |
| G3 | crypto_kdf_hkdf_sha512_extract + _expand | full RFC 5869 SHA-512 chain | [ ] |
| G3 | crypto_kdf_hkdf_sha512_keygen / _keybytes / _bytes_min / _bytes_max | keygen 64 bytes; 64; 0; 16320 | [ ] |
| G3 | crypto_shorthash_siphash24 | 16-byte key, `inlen=0` (`switch (left)` case 0, `end == in`) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=1` (`switch (left)` case 1) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=2` (case 2) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=3` (case 3) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=4` (case 4) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=5` (case 5) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=6` (case 6) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=7` (case 7) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=8` (one full 64-bit word, `left == 0` with a non-empty compression loop) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen` swept over every value 0..64 (covers all 8 `switch (left)` cases at every word count; matches the upstream SipHash-2-4 64-vector suite) | [ ] |
| G3 | crypto_shorthash_siphash24 | `inlen=255` and `inlen=256` (length byte `inlen << 56` truncation behaviour) | [ ] |
| G3 | crypto_shorthash_siphash24 | key = 0x00..0x0f (canonical SipHash vector key); all-zero key; all-0xFF key | [ ] |
| G3 | crypto_shorthash_siphash24_bytes / _keybytes | 8; 16 | [ ] |
| G3 | crypto_shorthash_siphashx24 | 16-byte key, `inlen=0` (case 0), 128-bit output | [ ] |
| G3 | crypto_shorthash_siphashx24 | `inlen=1`..`inlen=7` (all seven `switch (left)` fall-through cases) | [ ] |
| G3 | crypto_shorthash_siphashx24 | `inlen=8` (`left == 0`, one compression round) | [ ] |
| G3 | crypto_shorthash_siphashx24 | `inlen` swept over every value 0..64 (all `switch (left)` cases; upstream SipHash-2-4-128 vector suite) | [ ] |
| G3 | crypto_shorthash_siphashx24 | key = 0x00..0x0f; all-zero key; all-0xFF key | [ ] |
| G3 | crypto_shorthash_siphashx24 | first 8 bytes of the 16-byte output must NOT be assumed equal to `crypto_shorthash_siphash24` (different finalization) | [ ] |
| G3 | crypto_shorthash_siphashx24_bytes / _keybytes | 16; 16 | [ ] |
| G3 | crypto_shorthash / crypto_shorthash_bytes / _keybytes / _primitive / crypto_shorthash_keygen | default alias for siphash24: output must equal `crypto_shorthash_siphash24`; 8; 16; "siphash24"; keygen 16 bytes | [ ] |
