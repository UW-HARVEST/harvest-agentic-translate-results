| h2c-1 | _sodium_core_h2c_string_to_hash | hash_alg=CORE_H2C_SHA256(1), ctx_len<=0xff, h_len grid {0,1,2,15,16,31,32,33,47,48,49,63,64,65,95,96,97,127,128,129,159,160,191,192,193,223,224,254,255} × msg_len {0,1,37} (loop skipped / partial memcpy / exact multiple of 32 / +1) | [x] |
| h2c-2 | _sodium_core_h2c_string_to_hash | hash_alg=CORE_H2C_SHA512(2), ctx_len<=0xff, same h_len grid (loop step 64: exact multiples, ±1, tail memcpy of h_len-i<64) | [x] |
| h2c-3 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, ctx_len grid {0,1,2,16,31,32,33,63,64,65,127,128,129,253,254,255} (short-DST path, no pre-hash) × h_len {0,1,32,48,64,96,255} | [x] |
| h2c-4 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, ctx_len grid {256,257,258,300,511,512,513,1000,4096} → "H2C-OVERSIZE-DST-" pre-hash branch (ctx:=u0, ctx_len:=32) incl. the u0-aliasing quirk where the main hash overwrites u0 before the per-block loop re-reads it as DST | [x] |
| h2c-5 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, ctx_len<=0xff grid, h_len {0,1,32,48,64,96,255} | [x] |
| h2c-6 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, ctx_len>0xff → oversize pre-hash branch (ctx:=u0, ctx_len:=64) + same u0 aliasing quirk | [x] |
| h2c-7 | _sodium_core_h2c_string_to_hash | hash_alg=SHA256, msg_len grid {0,1,2,55,56,63,64,65,111,112,127,128,129,191,192,255,256,1000,4096,10000} (message absorbed after the 64-byte zero block ⇒ straddles SHA-256 block boundaries) | [x] |
| h2c-8 | _sodium_core_h2c_string_to_hash | hash_alg=SHA512, same msg_len grid (message absorbed after the 128-byte zero block) | [x] |
| h2c-9 | _sodium_core_h2c_string_to_hash | h=NULL with h_len=0 (memcpy loop never entered), both hash ids | [x] |
| h2c-10 | _sodium_core_h2c_string_to_hash | ctx=NULL, ctx_len=0 (update() early-returns on inlen==0), h_len {0,1,32,64}, both hash ids | [x] |
| h2c-11 | _sodium_core_h2c_string_to_hash | msg=NULL, msg_len=0, h_len {0,1,32,64}, both hash ids | [x] |
| h2c-12 | _sodium_core_h2c_string_to_hash | h=NULL, ctx=NULL, msg=NULL, all lengths 0, both hash ids | [x] |
| h2c-13 | _sodium_core_h2c_string_to_hash | non-NULL but empty ctx and msg, h_len=64, both hash ids | [x] |
| h2c-14 | _sodium_core_h2c_string_to_hash | 400 random (seed 0xF00DBABE) combinations: hash_alg∈{1,2}, h_len∈[0,255], ctx_len∈[0,599] (spans the 0xff pre-hash boundary), msg_len∈[0,399] | [x] |
| h2c-15 | crypto_core_ed25519_from_string | _string_to_points(n=2) ⇒ h_len=96; hash_alg∈{1,2} × ctx_len {0,1,16,63,64,65,254,255,256,257,512,1000} × msg_len {0,1,63,64,65,127,128,129,1000} | [x] |
| h2c-16 | crypto_core_ed25519_from_string | ctx=NULL/msg=NULL/both NULL with zero lengths, both hash ids | [x] |
| h2c-17 | crypto_core_ed25519_from_string | 60 random (seeded) ctx_len∈[0,399] × msg_len∈[0,299] × hash_alg∈{1,2} | [x] |
| h2c-18 | crypto_core_ed25519_from_string_nu | _string_to_points(n=1) ⇒ h_len=48; same ctx_len/msg_len/hash_alg grid + NULL cases + 60 random | [x] |
| h2c-19 | crypto_core_ed25519_scalar_from_string | h_len=HASH_SC_L=48, 48-byte big-endian reversal into a 64-byte zero-padded buffer then sc25519_reduce; same full grid + NULL cases + 60 random | [x] |
| h2c-20 | crypto_core_ristretto255_from_string | _string_to_element ⇒ h_len=crypto_core_ristretto255_HASHBYTES=64; same full grid + NULL cases + 60 random | [x] |
| h2c-21 | crypto_core_ristretto255_scalar_from_string | pure delegation to crypto_core_ed25519_scalar_from_string; same full grid + NULL cases + 60 random | [x] |
| h2c-22 | crypto_core_ristretto255_scalar_from_string, crypto_core_ed25519_scalar_from_string | equivalence cross-check: both libs must produce byte-identical output for the two entry points | [x] |
| h2c-23 | crypto_core_ed25519_from_string, crypto_core_ed25519_from_string_nu, _sodium_core_h2c_string_to_hash, _sodium_ge25519_from_hash, crypto_core_ed25519_add | layer-composition cross-check: from_string == add(from_hash(rev(h[0..48])), from_hash(rev(h[48..96]))) and _nu == from_hash(rev(h48)), for short and oversize (300/400-byte) ctx and both hash ids | [x] |
| h2c-24 | crypto_core_ristretto255_from_string, _sodium_core_h2c_string_to_hash, _sodium_ristretto255_from_hash | layer-composition cross-check: from_string == ristretto255_from_hash(h2c(64)), short + oversize ctx, both hash ids | [x] |
| h2c-25 | _sodium_ge25519_from_uniform | 10 edge 32-byte inputs (0, 1, 2^256-1, p-1, p, p+1, bit255 only, bit253 only (x_sign source), top-3-bits set, 0x55.. with s[31]=0x7f) + 60 random | [x] |
| h2c-26 | _sodium_ge25519_from_uniform | s == r (fully aliased in/out buffer; legal because the C does memcpy(s,r,32) first), all of the above inputs, plus equality with the non-aliased result | [x] |
| h2c-27 | _sodium_ge25519_from_hash | fe25519_reduce64 path: 15 edge 64-byte inputs (all-zero, all-0xff, lo=1, hi=1, p-1/p/p+1 in both halves, h[31]=h[63]∈{0x20,0x40,0x80,0xe0,0xff} to exercise the ((x>>5)^optblocker)>>2 carry terms *19 / *722) + 60 random | [x] |
| h2c-28 | _sodium_ristretto255_from_hash | same 15 edge 64-byte inputs + 60 random (two ristretto255_elligator calls + ge25519_p3_add + ristretto255_p3_tobytes) | [x] |
| h2c-29 | crypto_core_ristretto255_from_hash | public wrapper: same inputs, return value always 0, output equals the raw _sodium_ristretto255_from_hash in both libs | [x] |
| h2c-30 | crypto_core_ristretto255_from_hash | p aliasing the first 32 bytes of the 64-byte input h (both read r0/r1 before writing s) | [x] |
| h2c-31 | _sodium_core_h2c_string_to_hash | h_len=255 (the largest value allowed by assert(h_len <= 0xff)), both hash ids: 8 SHA-256 blocks with a 31-byte tail / 4 SHA-512 blocks with a 63-byte tail | [x] |
