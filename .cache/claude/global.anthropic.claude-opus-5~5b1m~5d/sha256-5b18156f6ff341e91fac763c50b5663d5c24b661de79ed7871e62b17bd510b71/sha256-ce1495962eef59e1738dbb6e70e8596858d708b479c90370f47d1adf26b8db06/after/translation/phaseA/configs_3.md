## Area 3 — hashes / xof / generichash / shorthash

Scope: `crypto_hash/{crypto_hash.c, sha256/**, sha512/**, sha3/hash_sha3.c}`, `crypto_xof/**`,
`crypto_generichash/{crypto_generichash.c, blake2b/generichash_blake2.c, blake2b/ref/**}`,
`crypto_shorthash/**` + public headers.

Build assumption: no `HAVE_*` macros are defined, so every `#ifdef HAVE_*` takes the portable fallback
(`SHA256_Transform` scalar path, `blake2b_compress_ref`, non-`HAVE_TI_MODE` counter increment).

Block / rate sizes that drive the boundary axes:

| primitive | block or rate | source |
|---|---|---|
| SHA-256 | 64 B block | `hash_sha256_cp.c` (`& 0x3f`, pad threshold 56) |
| SHA-512 | 128 B block | `hash_sha512_cp.c` (`& 0x7f`, pad threshold 112) |
| SHA3-256 | 136 B rate | `SHA3_256_RATE` |
| SHA3-512 | 72 B rate | `SHA3_512_RATE` |
| SHAKE128 / TurboSHAKE128 | 168 B rate | `SHAKE128_RATE` / `TURBOSHAKE128_RATE` |
| SHAKE256 / TurboSHAKE256 | 136 B rate | `SHAKE256_RATE` / `TURBOSHAKE256_RATE` |
| BLAKE2b | 128 B block, 256 B lazy buffer (`buf[2*128]`) | `blake2.h` / `blake2b_update` |

**Length set L** (used by many rows below) = `{0, 1, 63, 64, 65, 127, 128, 129, 135, 136, 137, 143, 144, 255, 256}`.
It straddles every block/rate boundary in the area: 63/64/65 (SHA-256), 71/72/73 → covered by 143/144 mod 72,
127/128/129 (SHA-512, BLAKE2b), 135/136/137 (SHA3-256, SHAKE256, TurboSHAKE256), 143/144 (2×72 for SHA3-512),
255/256 (2× SHA-512 block, BLAKE2b full buffer).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 3.1 | `crypto_hash` | one-shot, `inlen` over all of L; must equal `crypto_hash_sha512` byte-for-byte (32→64 B out is 64) | [x] |
| 3.2 | `crypto_hash_bytes`, `crypto_hash_primitive` | no input; expect `64`, `"sha512"` | [x] |
| 3.3 | `crypto_hash_sha256` | one-shot, `inlen` over all of L | [x] |
| 3.4 | `crypto_hash_sha256_{init,update,final}` | streaming, single `update` of each `inlen` in L | [x] |
| 3.5 | `crypto_hash_sha256_*` | streaming, `inlen = 256` fed as 256 separate 1-byte `update` calls (exercises `r` walking 0..63 and the `inlen < 64 - r` lazy branch on every offset) | [x] |
| 3.6 | `crypto_hash_sha256_*` | streaming, two updates `(a, b)` for every pair with `a + b` ∈ L and `a` ∈ {0,1,31,32,33,63,64,65}: covers `r != 0` entry, the `for (i = 0; i < 64 - r; i++)` fill-and-transform, and the `inlen &= 63` tail | [x] |
| 3.7 | `crypto_hash_sha256_*` | streaming, `update` with `inlen == 0` interleaved between non-empty updates (must be a no-op: `count` unchanged) | [x] |
| 3.8 | `crypto_hash_sha256_*` | streaming with `inlen == 64 - r` exactly (update ends exactly on a block boundary → `inlen &= 63` yields 0, `buf` left untouched) | [x] |
| 3.9 | `crypto_hash_sha256_*` | `SHA256_Pad` short branch: total length ≡ `r < 56` (mod 64), e.g. 0, 1, 55 | [x] |
| 3.10 | `crypto_hash_sha256_*` | `SHA256_Pad` long branch: total length ≡ `r >= 56` (mod 64), e.g. 56, 57, 63, 120, 127 (two-block padding) | [x] |
| 3.11 | `crypto_hash_sha256_statebytes` | no input; equals `sizeof(crypto_hash_sha256_state)` | [x] |
| 3.12 | `crypto_hash_sha256` vs streaming | equivalence: one-shot == init/update×n/final for every split in 3.6 | [x] |
| 3.13 | `crypto_hash_sha512` | one-shot, `inlen` over all of L | [x] |
| 3.14 | `crypto_hash_sha512_{init,update,final}` | streaming, single `update` of each `inlen` in L | [x] |
| 3.15 | `crypto_hash_sha512_*` | streaming, `inlen = 256` fed as 256 separate 1-byte `update` calls (`r` walks 0..127) | [x] |
| 3.16 | `crypto_hash_sha512_*` | streaming, two updates `(a, b)` with `a + b` ∈ L and `a` ∈ {0,1,63,64,65,127,128,129}: covers `r != 0`, the `128 - r` fill, the `while (inlen >= 128)` bulk loop, and `inlen &= 127` | [x] |
| 3.17 | `crypto_hash_sha512_*` | `update` with `inlen == 0` interleaved (must not advance `count[0]`/`count[1]`) | [x] |
| 3.18 | `crypto_hash_sha512_*` | `SHA512_Pad` short branch: total ≡ `r < 112` (mod 128), e.g. 0, 1, 111 | [x] |
| 3.19 | `crypto_hash_sha512_*` | `SHA512_Pad` long branch: total ≡ `r >= 112` (mod 128), e.g. 112, 113, 127, 240, 255 | [x] |
| 3.20 | `crypto_hash_sha512_*` | length whose bit count exercises `bitlen[0] = inlen >> 61` ≠ 0 and the `count[1]` carry (conceptual / streaming-accumulated) | [x] |
| 3.21 | `crypto_hash_sha512_statebytes` | no input | [x] |
| 3.22 | `crypto_hash_sha512` vs streaming | equivalence across every split in 3.16 | [x] |
| 3.23 | `crypto_hash_sha3256` | one-shot, `inlen` over all of L plus `{71, 72, 73, 167, 168, 271, 272, 273}` (rate 136 and 2×136) | [x] |
| 3.24 | `crypto_hash_sha3256_{init,update,final}` | streaming, single `update` of each `inlen` above | [x] |
| 3.25 | `crypto_hash_sha3256_*` | streaming, 1-byte updates ×272 (drives `state->offset` 0..136 twice, incl. `offset == rate` re-permute at the head of `sha3_update`) | [x] |
| 3.26 | `crypto_hash_sha3256_*` | streaming, two updates `(a, b)` with `a` ∈ {1, 135, 136, 137} and `a + b` ∈ {136, 137, 272, 273}: hits the `offset != 0 && inlen > 0` partial-chunk arm, the `offset == rate && consumed < inlen` mid-permute, the `while (inlen - consumed >= rate)` bulk arm, and the trailing `consumed < inlen` arm | [x] |
| 3.27 | `crypto_hash_sha3256_*` | streaming, `update(inlen = 0)` first and between updates (must not permute; `offset` unchanged) | [x] |
| 3.28 | `crypto_hash_sha3256_*` | total input ≡ `rate - 1` (135) mod 136 → `sha3_final` fused pad byte `0x06 ^ 0x80 == 0x86` | [x] |
| 3.29 | `crypto_hash_sha3256_*` | total input ≡ 0 mod 136 with `offset == rate` at `final` (e.g. exactly 136 absorbed as one update) → `final` extra `permute_24` then pad at `offset == 0` | [x] |
| 3.30 | `crypto_hash_sha3256` one-shot vs streaming | equivalence for every split in 3.26 | [x] |
| 3.31 | `crypto_hash_sha3256_bytes`, `_statebytes` | no input; `32`, `256` | [x] |
| 3.32 | `crypto_hash_sha3512` | one-shot, `inlen` over all of L plus `{71, 72, 73, 143, 144, 145, 215, 216, 217}` (rate 72, 2×72, 3×72) | [x] |
| 3.33 | `crypto_hash_sha3512_{init,update,final}` | streaming, single `update` of each `inlen` above | [x] |
| 3.34 | `crypto_hash_sha3512_*` | streaming, 1-byte updates ×145 (`offset` 0..72 twice) | [x] |
| 3.35 | `crypto_hash_sha3512_*` | streaming, two updates `(a, b)` with `a` ∈ {1, 71, 72, 73} and `a + b` ∈ {72, 73, 144, 145} — same four `sha3_update` arms as 3.26 but at rate 72 | [x] |
| 3.36 | `crypto_hash_sha3512_*` | total ≡ 71 mod 72 → fused pad `0x86` | [x] |
| 3.37 | `crypto_hash_sha3512_*` | total ≡ 0 mod 72 with `offset == rate` at `final` | [x] |
| 3.38 | `crypto_hash_sha3512` one-shot vs streaming | equivalence for every split in 3.35 | [x] |
| 3.39 | `crypto_hash_sha3512_bytes`, `_statebytes` | no input; `64`, `256` | [x] |
| 3.40 | SHA-3 digest-size matrix | same message run through both `crypto_hash_sha3256` (32 B) and `crypto_hash_sha3512` (64 B) — confirms `outlen`/`rate` are carried in the state, not hard-coded in `sha3_final` | [x] |
| 3.41 | `crypto_xof_shake128` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {167, 168, 169, 335, 336, 337}, `outlen` ∈ {0, 1, 32, 167, 168, 169, 335, 336, 337, 512} | [x] |
| 3.42 | `crypto_xof_shake128_{init,update,squeeze}` | streaming with one `update` and one `squeeze`, over the same grid as 3.41 | [x] |
| 3.43 | `crypto_xof_shake128_*` | multiple absorb calls: `update` ×n with sizes `(1,1,…)`, `(167,1)`, `(168,1)`, `(1,167)`, `(100,68,168)` before a single `squeeze` — must equal the concatenated one-shot | [x] |
| 3.44 | `crypto_xof_shake128_*` | chunked squeeze: total 512 B extracted as 1-byte calls ×512; as `(1, 167)`, `(167, 1)`, `(168, 168, 176)`, `(169, 343)`; must equal a single 512-B squeeze (drives `offset == RATE` re-permute, the `offset != 0` partial arm, the `while (outlen - extracted >= RATE)` bulk arm and the trailing arm) | [x] |
| 3.45 | `crypto_xof_shake128_*` | `squeeze(outlen = 0)` before / between real squeezes (must be a no-op, must not permute) | [x] |
| 3.46 | `crypto_xof_shake128_*` | absorb total ≡ `RATE - 1` (167) mod 168 → `shake128_finalize` fused pad `domain ^ 0x80` | [x] |
| 3.47 | `crypto_xof_shake128_*` | absorb total ≡ 0 mod 168 with `offset == RATE` at first squeeze → extra `permute_24` inside `shake128_finalize` | [x] |
| 3.48 | `crypto_xof_shake128_init_with_domain` | `domain = crypto_xof_shake128_DOMAIN_STANDARD` (0x1F) — must match plain `_init` exactly | [x] |
| 3.49 | `crypto_xof_shake128_init_with_domain` | `domain` ∈ {0x00, 0x01, 0x02, 0x06, 0x07, 0x0B, 0x1F, 0x7F, 0x80, 0xFF} — all accepted, no range check; each must give a distinct stream | [x] |
| 3.50 | `crypto_xof_shake128_init_with_domain` | `domain = 0x06` with absorb length ≡ 0 mod 168 and squeeze 32 → cross-check that the SHA3 domain byte under a SHAKE rate is reachable (no validation) | [x] |
| 3.51 | `crypto_xof_shake128_blockbytes`, `_statebytes`, `_domain_standard` | no input; `168`, `256`, `0x1F` | [x] |
| 3.52 | `crypto_xof_shake128` one-shot vs streaming | equivalence for every (absorb split, squeeze split) pair from 3.43 × 3.44 | [x] |
| 3.53 | `crypto_xof_shake256` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {271, 272, 273}, `outlen` ∈ {0, 1, 32, 64, 135, 136, 137, 271, 272, 273, 512} | [x] |
| 3.54 | `crypto_xof_shake256_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.53 grid | [x] |
| 3.55 | `crypto_xof_shake256_*` | multiple absorb calls: `(1,1,…)`, `(135,1)`, `(136,1)`, `(1,135)`, `(100,36,136)` | [x] |
| 3.56 | `crypto_xof_shake256_*` | chunked squeeze: 512 B as 1-byte ×512; as `(1,135)`, `(135,1)`, `(136,136,240)`, `(137,375)` | [x] |
| 3.57 | `crypto_xof_shake256_*` | absorb total ≡ 135 mod 136 → fused pad byte | [x] |
| 3.58 | `crypto_xof_shake256_*` | absorb total ≡ 0 mod 136 with `offset == RATE` at first squeeze | [x] |
| 3.59 | `crypto_xof_shake256_init_with_domain` | `domain` ∈ {0x00, 0x01, 0x06, 0x1F, 0x7F, 0x80, 0xFF} | [x] |
| 3.60 | `crypto_xof_shake256_blockbytes`, `_statebytes`, `_domain_standard` | no input; `136`, `256`, `0x1F` | [x] |
| 3.61 | `crypto_xof_shake256` one-shot vs streaming | equivalence for 3.55 × 3.56 | [x] |
| 3.62 | SHAKE128 vs SHAKE256 | same message + same `outlen` through both — different rate ⇒ different output; confirms rate is not shared state | [x] |
| 3.63 | `crypto_xof_turboshake128` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {167, 168, 169, 335, 336, 337}, `outlen` ∈ {0, 1, 32, 167, 168, 169, 336, 512} — uses `permute_12`, not `permute_24` | [x] |
| 3.64 | `crypto_xof_turboshake128_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.63 grid | [x] |
| 3.65 | `crypto_xof_turboshake128_*` | multiple absorb calls: `(1,1,…)`, `(167,1)`, `(168,1)`, `(1,167)`, `(100,68,168)` | [x] |
| 3.66 | `crypto_xof_turboshake128_*` | chunked squeeze: 512 B as 1-byte ×512; `(1,167)`, `(167,1)`, `(168,168,176)`, `(169,343)` | [x] |
| 3.67 | `crypto_xof_turboshake128_*` | absorb total ≡ 167 mod 168 → fused pad `domain ^ 0x80` | [x] |
| 3.68 | `crypto_xof_turboshake128_*` | absorb total ≡ 0 mod 168 with `offset == RATE` at first squeeze → extra `permute_12` | [x] |
| 3.69 | `crypto_xof_turboshake128_init_with_domain` | `domain = crypto_xof_turboshake128_DOMAIN_STANDARD` (0x1F); must equal plain `_init` | [x] |
| 3.70 | `crypto_xof_turboshake128_init_with_domain` | **domain-byte sweep** `domain` ∈ {0x00, 0x01, 0x02, 0x03, 0x06, 0x07, 0x0A, 0x1F, 0x30, 0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF} — the spec-legal range is 0x01..0x7F but the C code range-checks **nothing**, so 0x00 / 0x80 / 0xFF must be accepted and produce well-defined output | [x] |
| 3.71 | `crypto_xof_turboshake128_init_with_domain` | every `domain` from 3.70 combined with absorb length ≡ 167 mod 168, so the domain byte goes through the fused `domain ^ 0x80` path (0x00→0x80, 0x80→0x00, 0xFF→0x7F) | [x] |
| 3.72 | `crypto_xof_turboshake128_blockbytes`, `_statebytes`, `_domain_standard` | no input; `168`, `256`, `0x1F` | [x] |
| 3.73 | `crypto_xof_turboshake128` one-shot vs streaming | equivalence for 3.65 × 3.66 | [x] |
| 3.74 | `crypto_xof_turboshake256` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {271, 272, 273}, `outlen` ∈ {0, 1, 32, 64, 135, 136, 137, 272, 512} | [x] |
| 3.75 | `crypto_xof_turboshake256_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.74 grid | [x] |
| 3.76 | `crypto_xof_turboshake256_*` | multiple absorb calls: `(1,1,…)`, `(135,1)`, `(136,1)`, `(1,135)`, `(100,36,136)` | [x] |
| 3.77 | `crypto_xof_turboshake256_*` | chunked squeeze: 512 B as 1-byte ×512; `(1,135)`, `(135,1)`, `(136,136,240)`, `(137,375)` | [x] |
| 3.78 | `crypto_xof_turboshake256_*` | absorb total ≡ 135 mod 136 → fused pad; and ≡ 0 mod 136 with `offset == RATE` at first squeeze | [x] |
| 3.79 | `crypto_xof_turboshake256_init_with_domain` | domain-byte sweep as in 3.70 (0x00..0xFF representatives), no range check | [x] |
| 3.80 | `crypto_xof_turboshake256_blockbytes`, `_statebytes`, `_domain_standard` | no input; `136`, `256`, `0x1F` | [x] |
| 3.81 | `crypto_xof_turboshake256` one-shot vs streaming | equivalence for 3.76 × 3.77 | [x] |
| 3.82 | TurboSHAKE vs SHAKE at equal rate | SHAKE128 (24 rounds) vs TurboSHAKE128 (12 rounds) at the same rate 168 and same `domain = 0x1F` — outputs must differ; likewise SHAKE256 vs TurboSHAKE256 at rate 136 | [x] |
| 3.83 | XOF long-stream continuity | for each of the 4 XOFs: `squeeze(N)` in one call vs `N` accumulated over ⌈N/1⌉…⌈N/RATE⌉-sized calls for `N` = 4096, which crosses ~24 blocks | [x] |
| 3.84 | `crypto_generichash_blake2b` (low-level one-shot) | unkeyed (`key = NULL`, `keylen = 0`), `outlen` ∈ {1, 2, 15, **16 = BYTES_MIN**, 17, 31, **32 = BYTES**, 33, 63, **64 = BYTES_MAX**}, `inlen` ∈ L ∪ {257} | [x] |
| 3.85 | `crypto_generichash_blake2b` | unkeyed, `inlen = 0` with `in = NULL` and separately with `in != NULL` — both legal, same digest | [x] |
| 3.86 | `crypto_generichash_blake2b` | keyed, `keylen` ∈ {1, 15, **16 = KEYBYTES_MIN**, 17, 31, **32 = KEYBYTES**, 33, 63, **64 = KEYBYTES_MAX**} × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 127, 128, 129} (keyed init pre-absorbs a 128-B zero-padded key block, so `buflen` starts at 128) | [x] |
| 3.87 | `crypto_generichash_blake2b` | `key != NULL` with `keylen = 0` → silently unkeyed; must equal the `key = NULL, keylen = 0` digest | [x] |
| 3.88 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt != NULL` + `personal != NULL` (16 B each), `outlen` ∈ {1, 16, 32, 64}, `inlen` ∈ L | [x] |
| 3.89 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt = NULL` + `personal = NULL` → must equal plain `crypto_generichash_blake2b` (both fields zeroed) | [x] |
| 3.90 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt != NULL` + `personal = NULL`; and `salt = NULL` + `personal != NULL` (the two mixed arms of `blake2b_init_salt_personal`) | [x] |
| 3.91 | `crypto_generichash_blake2b_salt_personal` | keyed, all four salt/personal NULL-combinations × `keylen` ∈ {1, 16, 32, 64} → routes through `blake2b_init_key_salt_personal` | [x] |
| 3.92 | `crypto_generichash_blake2b_salt_personal` | all-zero 16-B salt/personal buffers vs `NULL` → must be identical | [x] |
| 3.93 | `crypto_generichash_blake2b_salt_personal` | distinct salts with identical personal (and vice versa) → distinct digests; confirms both 16-B fields land at param offsets 32 and 48 | [x] |
| 3.94 | `crypto_generichash_blake2b_init` + `_update` + `_final` | unkeyed streaming, `outlen` ∈ {1, 16, 32, 64}, single `update` of each `inlen` ∈ L ∪ {257} | [x] |
| 3.95 | `crypto_generichash_blake2b_init/_update/_final` | unkeyed, `inlen = 256` fed as 256 1-byte `update` calls (walks `buflen` 0..256 and the lazy `inlen <= fill` arm at every offset) | [x] |
| 3.96 | `crypto_generichash_blake2b_init/_update/_final` | unkeyed, two updates `(a, b)` with `a` ∈ {0, 1, 127, 128, 129, 255, 256} and `a + b` ∈ {128, 129, 256, 257, 384}: hits `inlen > fill` (compress + 128-B left-shift) and `inlen <= fill` (lazy buffer) | [x] |
| 3.97 | `crypto_generichash_blake2b_init/_update/_final` | `update(inlen = 0)` first, between, and last (must be a no-op) | [x] |
| 3.98 | `crypto_generichash_blake2b_init/_update/_final` | keyed streaming: `keylen` ∈ {1, 16, 32, 64} × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 128, 129, 256} — note keyed init already leaves `buflen == 128`, so the first user byte lands at `buf[128]` | [x] |
| 3.99 | `crypto_generichash_blake2b_init` | `key = NULL` with `keylen` ∈ {1, 16, 32, 64} → the `key == NULL \|\| keylen <= 0` guard routes to **unkeyed** `blake2b_init`; must match the unkeyed digest (deliberate divergence from the one-shot, which aborts) | [x] |
| 3.100 | `crypto_generichash_blake2b_init` | `key != NULL` with `keylen = 0` → unkeyed | [x] |
| 3.101 | `crypto_generichash_blake2b_init_salt_personal` + `_update` + `_final` | unkeyed streaming × all four salt/personal NULL-combinations × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 128, 129, 256} | [x] |
| 3.102 | `crypto_generichash_blake2b_init_salt_personal` + `_update` + `_final` | keyed streaming (`key != NULL, keylen > 0`) × all four salt/personal NULL-combinations × `keylen` ∈ {1, 16, 32, 64} | [x] |
| 3.103 | `crypto_generichash_blake2b_init_salt_personal` | `key = NULL, keylen > 0` → routed to `blake2b_init_salt_personal` (unkeyed), same asymmetry as 3.99 | [x] |
| 3.104 | one-shot vs streaming equivalence (blake2b) | for every (key, salt, personal, outlen) combination in 3.84–3.93, `crypto_generichash_blake2b{,_salt_personal}` must equal `_init{,_salt_personal}` / `_update`×n / `_final` for every split in 3.96 | [x] |
| 3.105 | `blake2b_init_key` vs `blake2b_init` (via public wrappers) | `keylen = 64` (full key block, no zero padding) vs `keylen = 1` (127 zero-pad bytes) vs unkeyed — three structurally different first blocks | [x] |
| 3.106 | `blake2b_init_param` path coverage | exercised indirectly by all four init variants: `digest_length` = each valid `outlen`, `key_length` ∈ {0, 1, 16, 32, 64}, `fanout = 1`, `depth = 1`, `leaf_length = 0`, `node_offset = 0`, `node_depth = 0`, `inner_length = 0`, `reserved[14]` zero, `salt`/`personal` set or zero — i.e. every field of the 64-B param block that any public entry point can vary | [x] |
| 3.107 | `blake2b_state.last_node` | always `0` in this build (`blake2b_init0` zeroes it; nothing sets it) → `blake2b_set_lastblock` never calls `blake2b_set_lastnode`, `f[1]` stays `0`. Configuration to pin: the field exists in the state layout and `statebytes` accounting but is behaviourally inert | [x] |
| 3.108 | `crypto_generichash` (generic wrapper, one-shot) | must be byte-identical to `crypto_generichash_blake2b` across the whole 3.84–3.87 matrix: unkeyed / keyed, `outlen` ∈ {1, 15, 16, 32, 64}, `keylen` ∈ {0, 1, 16, 32, 64}, `inlen` ∈ L | [x] |
| 3.109 | `crypto_generichash_init` + `_update` + `_final` (generic wrappers) | must be byte-identical to the `_blake2b_` streaming path across 3.94–3.100; also verify the wrapper's `(state, key, keylen, outlen)` argument order is preserved | [x] |
| 3.110 | `crypto_generichash_final` | `outlen` **equal** to the `_init` `outlen` (the intended use) for each of {1, 16, 32, 64} | [x] |
| 3.111 | `crypto_generichash_final` | `outlen` **less than** the `_init` `outlen` (e.g. init 64 / final 32, init 32 / final 16, init 32 / final 1) — silently allowed, yields a prefix of the init-64 digest, **not** the init-32 digest; must be reproduced | [x] |
| 3.112 | `crypto_generichash_final` | `outlen` **greater than** the `_init` `outlen` but ≤ 64 (e.g. init 16 / final 64) — silently allowed | [x] |
| 3.113 | `crypto_generichash_state` / `crypto_generichash_blake2b_state` | `statebytes()` = `(sizeof(state) + 63) & ~63`; state is `unsigned char opaque[384]` with `CRYPTO_ALIGN(64)`; `crypto_generichash_state` is a typedef of the blake2b state, so a state initialized via `crypto_generichash_init` must be finalizable via `crypto_generichash_blake2b_final` and vice versa | [x] |
| 3.114 | `crypto_generichash_keygen`, `crypto_generichash_blake2b_keygen` | fills 32 (`KEYBYTES`) random bytes; verify length only | [x] |
| 3.115 | generichash size accessors | `crypto_generichash_{bytes_min,bytes_max,bytes,keybytes_min,keybytes_max,keybytes,primitive,statebytes}` = `16, 64, 32, 16, 64, 32, "blake2b", 384`; `crypto_generichash_blake2b_{…,saltbytes,personalbytes}` adds `16, 16` | [x] |
| 3.116 | `blake2b_compress_ref` selection | with no `HAVE_*` macros, `blake2b_compress` is `blake2b_compress_ref` at file scope and `blake2b_pick_best_implementation()` re-selects it; every blake2b row above must be checked against the ref compress only | [x] |
| 3.117 | `crypto_shorthash_siphash24` | `inlen` = 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 (every `inlen & 7` residue and the 0/1/2 full-word cases) with a fixed 16-B key | [x] |
| 3.118 | `crypto_shorthash_siphash24` | `inlen` = 17, 23, 24, 31, 32, 63, 64, 127, 128, 255, 256 (larger inputs, all 8 tail residues at multiple word counts) | [x] |
| 3.119 | `crypto_shorthash_siphash24` | `inlen = 0` with `in = NULL` — the `inlen ? in + inlen - (inlen % 8) : in` ternary means `end == in`, no dereference | [x] |
| 3.120 | `crypto_shorthash_siphash24` | `inlen = 255` and `inlen = 256` — the length byte is `((uint64_t) inlen) << 56`, so `inlen` aliases mod 256; both must be computed as C does | [x] |
| 3.121 | `crypto_shorthash_siphash24` | key variations: all-zero 16-B key, all-`0xFF` key, the RFC vector key `00 01 … 0f`, and a random key — over `inlen` 0..16 | [x] |
| 3.122 | `crypto_shorthash_siphashx24` | `inlen` = 0..16 inclusive with a fixed 16-B key; output is 16 B (`siphashx24_BYTES`) | [x] |
| 3.123 | `crypto_shorthash_siphashx24` | `inlen` = 17, 23, 24, 31, 32, 63, 64, 127, 128, 255, 256 | [x] |
| 3.124 | `crypto_shorthash_siphashx24` | key variations as in 3.121; verify the first 8 output bytes differ from `siphash24` (different `v1` init `…646f83`, `v2 ^= 0xee` vs `0xff`) | [x] |
| 3.125 | `crypto_shorthash_siphashx24` | second-half derivation: bytes 8..15 come from `v1 ^= 0xdd` + 4 extra SIPROUNDs after bytes 0..7 are stored — check both halves independently | [x] |
| 3.126 | `crypto_shorthash` (generic wrapper) | must be byte-identical to `crypto_shorthash_siphash24` over `inlen` 0..16 and 3.118's larger set | [x] |
| 3.127 | shorthash size accessors | `crypto_shorthash_{bytes,keybytes,primitive}` = `8, 16, "siphash24"`; `crypto_shorthash_siphash24_{bytes,keybytes}` = `8, 16`; `crypto_shorthash_siphashx24_{bytes,keybytes}` = `16, 16` | [x] |
| 3.128 | area-wide input-content axis | for every row above, run at least: all-zero input, all-`0xFF` input, and an incrementing `i & 0xff` pattern — the `LOAD*_BE`/`LOAD*_LE` and `STORE*` helpers are endian-sensitive and a byte-pattern input catches transposition bugs a constant input cannot | [x] (tests/a3_crosscut.rs) |
| 3.129 | area-wide state-reuse axis | `init` → `update` → `final` → `init` → `update` → `final` on the same state object for sha256, sha512, sha3-256, sha3-512, each XOF (`init` after `squeeze`) and blake2b (`_init` after `_final`) — re-init must fully reset (sha2 relies on the `memzero` in `final` plus a fresh `init`; sha3/XOF reset `phase`/`offset`; blake2b's `init0` clears `f`) | [x] (tests/a3_crosscut.rs) |
| 3.130 | area-wide overlapping / aliased buffers | `out` overlapping `in` for the one-shot entry points (`crypto_hash_sha256`, `crypto_hash_sha512`, `crypto_hash_sha3*`, `crypto_xof_*`, `crypto_generichash*`, `crypto_shorthash*`) — C writes `out` only after consuming `in` in all of these, so the aliased case is defined; the port must not regress it | [x] (tests/a3_crosscut.rs) |
| 3.131 | area-wide primitive-vs-generic consistency | `crypto_hash` ≡ `crypto_hash_sha512`; `crypto_generichash*` ≡ `crypto_generichash_blake2b*`; `crypto_shorthash` ≡ `crypto_shorthash_siphash24`; check for at least one input from each of the three content patterns in 3.128 | [x] (tests/a3_crosscut.rs) |
