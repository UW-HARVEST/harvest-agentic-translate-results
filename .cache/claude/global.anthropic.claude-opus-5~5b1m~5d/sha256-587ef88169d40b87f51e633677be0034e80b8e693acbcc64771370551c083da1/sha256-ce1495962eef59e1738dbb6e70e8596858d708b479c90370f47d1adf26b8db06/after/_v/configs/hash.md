| hash-1 | crypto_hash_bytes, crypto_hash_primitive | constant getters, no input; value + string content compared | [x] |
| hash-2 | crypto_hash | in=NULL, inlen=0 | [x] |
| hash-3 | crypto_hash | inlen in {0,1,2,3,7,8,9,15,16,31,32,55..57,63..65,71..73,111..113,127..129,135..137,143,144,167..169,191,192,200,255,256,271,272,335..337,1000,4096} x 3 random msgs; also asserted identical to crypto_hash_sha512 | [x] |
| hash-4 | crypto_hash_sha256_bytes, crypto_hash_sha256_statebytes | constant getters (32 / 104) | [x] |
| hash-5 | crypto_hash_sha256 | one-shot, full SIZES list x 3 random messages each | [x] |
| hash-6 | crypto_hash_sha256 | in=NULL, inlen=0 (update() early-return path) | [x] |
| hash-7 | crypto_hash_sha256_init, _update, _final | 1 chunk = whole message; full 104-byte state compared after init/update/final | [x] |
| hash-8 | crypto_hash_sha256_update | inlen==0 chunk interleaved before/after data chunks (early return, count untouched) | [x] |
| hash-9 | crypto_hash_sha256_update | inlen < 64-r: buffer-only path, no transform | [x] |
| hash-10 | crypto_hash_sha256_update | inlen == 64-r exactly: one transform, `while (inlen>=64)` not entered | [x] |
| hash-11 | crypto_hash_sha256_update | inlen > 64-r: fill+transform, multi-block while loop, `inlen &= 63` tail copy | [x] |
| hash-12 | crypto_hash_sha256_init/_update/_final | 2,3,4,5 random chunks; plus [1,n-1], [n-1,1], [64,n-64], [64,0,n-64], [128,n-128], byte-at-a-time for n<=64 | [x] |
| hash-13 | crypto_hash_sha256_final | SHA256_Pad with r < 56 (single final transform) | [x] |
| hash-14 | crypto_hash_sha256_final | SHA256_Pad with r >= 56 (extra transform + memset(buf,0,56)) — n in {56..63} mod 64 | [x] |
| hash-15 | crypto_hash_sha256_final | state fully zeroized by sodium_memzero after final | [x] |
| hash-16 | crypto_hash_sha256 + streaming | 300003-byte seeded-RNG input, one-shot and random 1..9000-byte chunks | [x] |
| hash-17 | crypto_hash_sha512_bytes, crypto_hash_sha512_statebytes | constant getters (64 / 208) | [x] |
| hash-18 | crypto_hash_sha512 | one-shot, full SIZES list x 3 random messages each | [x] |
| hash-19 | crypto_hash_sha512 | in=NULL, inlen=0 | [x] |
| hash-20 | crypto_hash_sha512_init, _update, _final | 1 chunk; full 208-byte state compared after init/update/final | [x] |
| hash-21 | crypto_hash_sha512_update | inlen==0 chunk (early return) | [x] |
| hash-22 | crypto_hash_sha512_update | inlen < 128-r: buffer-only path | [x] |
| hash-23 | crypto_hash_sha512_update | inlen == 128-r exactly: one transform, no while loop | [x] |
| hash-24 | crypto_hash_sha512_update | inlen > 128-r: multi-block while loop + `inlen &= 127` tail | [x] |
| hash-25 | crypto_hash_sha512_init/_update/_final | 2..5 random chunks + [128,n-128], [128,0,n-128], [256,n-256], [1,n-1], [n-1,1] | [x] |
| hash-26 | crypto_hash_sha512_update | count[1] low-word accumulation / count[0] carry (128-bit bit counter) exercised via 300KB streaming | [x] |
| hash-27 | crypto_hash_sha512_final | SHA512_Pad with r < 112 (single final transform) | [x] |
| hash-28 | crypto_hash_sha512_final | SHA512_Pad with r >= 112 (extra transform + memset(buf,0,112)) — n in {112..127} mod 128 | [x] |
| hash-29 | crypto_hash_sha512_final | state fully zeroized after final | [x] |
| hash-30 | crypto_hash_sha512 + streaming | 300003-byte input, one-shot and random chunking | [x] |
| hash-31 | crypto_hash_sha3256_bytes, crypto_hash_sha3256_statebytes | constant getters (32 / 256) | [x] |
| hash-32 | crypto_hash_sha3512_bytes, crypto_hash_sha3512_statebytes | constant getters (64 / 256) | [x] |
| hash-33 | crypto_hash_sha3256 | one-shot, full SIZES list x 3 random messages; plus FIPS-202 KAT for "" | [x] |
| hash-34 | crypto_hash_sha3512 | one-shot, full SIZES list x 3 random messages; plus FIPS-202 KAT for "" | [x] |
| hash-35 | crypto_hash_sha3256, crypto_hash_sha3512 | in=NULL, inlen=0 (empty input) | [x] |
| hash-36 | crypto_hash_sha3256_init/_update/_final | rate=136: 1 chunk; state compared after every call | [x] |
| hash-37 | crypto_hash_sha3512_init/_update/_final | rate=72: 1 chunk; state compared after every call | [x] |
| hash-38 | crypto_hash_sha3*_update | inlen==0 chunks interleaved (offset/rate untouched) | [x] |
| hash-39 | crypto_hash_sha3*_update | `offset != 0 && inlen > 0` partial-block XOR path with chunk_size > inlen (clamped) | [x] |
| hash-40 | crypto_hash_sha3*_update | `offset == rate && inlen > 0` -> permute then offset=0 (split exactly on rate: [136,n-136] / [72,n-72]) | [x] |
| hash-41 | crypto_hash_sha3*_update | full-rate `while (inlen-consumed >= rate)` loop with and without trailing bytes (offset left == rate) | [x] |
| hash-42 | crypto_hash_sha3*_update | 2..5 random chunks, [1,n-1], [n-1,1], byte-at-a-time for n<=64 | [x] |
| hash-43 | crypto_hash_sha3*_final | offset == rate at final -> permute, offset=0, then normal padding | [x] |
| hash-44 | crypto_hash_sha3*_final | offset == rate-1 -> single-byte padding `0x06 ^ 0x80` (n = 71/135 mod rate) | [x] |
| hash-45 | crypto_hash_sha3*_final | normal padding: 0x06 at offset, 0x80 at rate-1 | [x] |
| hash-46 | crypto_hash_sha3*_update / _final | phase == FINALIZED on entry (see errors table) | [x] |
| hash-47 | crypto_hash_sha3256, crypto_hash_sha3512 | 280007-byte seeded-RNG input, one-shot vs random 1..5000-byte chunks | [x] |
| hash-48 | crypto_core_keccak1600_statebytes | constant getter (224) | [x] |
| hash-49 | crypto_core_keccak1600_init | canary-filled 224-byte state: only bytes 0..200 zeroed, 200..224 untouched | [x] |
| hash-50 | crypto_core_keccak1600_permute_24 | all-zero state, applied 1..4 times consecutively (round-constant order) | [x] |
| hash-51 | crypto_core_keccak1600_permute_12 | all-zero state, applied 1..4 times consecutively (constants 12..23) | [x] |
| hash-52 | crypto_core_keccak1600_permute_24 / _permute_12 | all-0xFF state | [x] |
| hash-53 | crypto_core_keccak1600_xor_bytes | 200 random iterations x 6 steps: random offset 0..199, random length 0..200-offset — covers the unaligned head loop, the 8-byte body loop and the tail loop | [x] |
| hash-54 | crypto_core_keccak1600_xor_bytes | length == 0 at offsets {0,1,7,8,9,199} (no-op) | [x] |
| hash-55 | crypto_core_keccak1600_extract_bytes | random offset/length, output canary-guarded, also checked to equal the raw state slice | [x] |
| hash-56 | crypto_core_keccak1600_extract_bytes | length == 0 (no write) | [x] |
| hash-57 | crypto_core_keccak1600_* mixed | random xor / extract / permute_24 / permute_12 sequences on a random 224-byte state | [x] |
| hash-58 | _sodium_keccak1600_ref_init | zeroes exactly KECCAK1600_STATEBYTES (200) of a canary buffer | [x] |
| hash-59 | _sodium_keccak1600_ref_xor_bytes | 150 iterations, offsets covering all 8 alignment classes x multiples of 8 | [x] |
| hash-60 | _sodium_keccak1600_ref_extract_bytes | random offset/length with output guard | [x] |
| hash-61 | _sodium_keccak1600_ref_permute_24, _permute_12 | 150 random 200-byte states, both applied in sequence | [x] |
| hash-62 | crypto_xof_shake128_blockbytes/_statebytes/_domain_standard | constant getters (168 / 256 / 0x1F) | [x] |
| hash-63 | crypto_xof_shake256_blockbytes/_statebytes/_domain_standard | constant getters (136 / 256 / 0x1F) | [x] |
| hash-64 | crypto_xof_turboshake128_blockbytes/_statebytes/_domain_standard | constant getters (168 / 256 / 0x1F) | [x] |
| hash-65 | crypto_xof_turboshake256_blockbytes/_statebytes/_domain_standard | constant getters (136 / 256 / 0x1F) | [x] |
| hash-66 | crypto_xof_shake128 / shake256 / turboshake128 / turboshake256 | one-shot: inlen over the full SIZES list x outlen in {0,1,2,7,8,31,32,63,64,71,72,73,135,136,137,167,168,169,200,271,272,273,335,336,337,504,1000} | [x] |
| hash-67 | crypto_xof_* | one-shot with in=NULL, inlen=0; plus SHAKE128/256("",32) KATs | [x] |
| hash-68 | crypto_xof_* | one-shot with outlen == 0 (out buffer untouched) | [x] |
| hash-69 | _sodium_shake128_ref / _sodium_shake256_ref / _sodium_turboshake128_ref / _sodium_turboshake256_ref | internal one-shot over the same inlen x outlen grid; also asserted equal to the public one-shot | [x] |
| hash-70 | crypto_xof_*_init + _update + _squeeze | 1 update, 1 squeeze; 256-byte state compared after every call | [x] |
| hash-71 | crypto_xof_*_update | inlen==0 chunks; `offset == RATE && inlen > 0` permute path (split exactly on the rate) | [x] |
| hash-72 | crypto_xof_*_update | partial-block path with clamped chunk_size, full-rate while loop with/without tail | [x] |
| hash-73 | crypto_xof_*_update | 2..5 random chunks + [rate, n-rate], [rate,0,n-rate], [2*rate,...], [1,n-1], [n-1,1] | [x] |
| hash-74 | crypto_xof_*_squeeze | one-shot squeeze of the whole output | [x] |
| hash-75 | crypto_xof_*_squeeze | multi-chunk squeeze: halves, [rate, outlen-rate], [1, outlen-1], doubling sizes 1,3,7,15,..., and 0-length squeezes at the start/end — all compared against the one-shot | [x] |
| hash-76 | crypto_xof_*_squeeze | `offset == RATE && outlen > 0` permute path; `offset != 0` partial-block extract; full-rate extract loop | [x] |
| hash-77 | crypto_xof_*_squeeze | repeated squeezes past several rate blocks (4 x rate+3 bytes after the first) | [x] |
| hash-78 | crypto_xof_*_init vs _init_with_domain(0x1F) | both must produce identical state and identical output | [x] |
| hash-79 | crypto_xof_*_init_with_domain | domain in {0x00,0x01,0x02,0x06,0x07,0x0B,0x1F,0x7F,0x80,0x81,0xA5,0xFE,0xFF} x inlen in {0,1,rate-2,rate-1,rate,rate+1,2rate-1,2rate,2rate+1} x outlen in {0,1,32,rate,rate+1,2rate+5} | [x] |
| hash-80 | crypto_xof_*_squeeze (finalize) | offset == RATE-1 special case: padding collapses to `domain ^ 0x80` (inlen ≡ rate-1 mod rate) | [x] |
| hash-81 | crypto_xof_*_squeeze (finalize) | offset == RATE at finalize -> permute then offset=0 then normal padding | [x] |
| hash-82 | crypto_xof_*_update after _squeeze | absorb/squeeze interleaving: update returns -1, permutes (24 rounds for shake, 12 for turboshake), resets, then squeeze re-finalizes | [x] |
| hash-83 | _sodium_*_ref_init / _ref_init_with_domain / _ref_update / _ref_squeeze | internal streaming API: init (both flavours), two updates, two squeezes; state compared after each call; result equal to the public one-shot | [x] |
| hash-84 | _sodium_*_ref_update after _ref_squeeze | internal absorb/squeeze interleaving with a non-standard domain (0x0B), returns -1 | [x] |
| hash-85 | crypto_xof_* | 260011-byte seeded-RNG input, 40009-byte output: one-shot vs random-chunk absorb (1..7000) + random-chunk squeeze (1..1000) | [x] |
