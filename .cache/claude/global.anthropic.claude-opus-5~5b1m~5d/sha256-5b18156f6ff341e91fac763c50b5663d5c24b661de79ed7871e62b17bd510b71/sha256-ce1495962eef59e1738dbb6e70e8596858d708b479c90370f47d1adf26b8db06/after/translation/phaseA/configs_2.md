## Area 2 — crypto_verify + crypto_core

Configuration surface = the *valid-input* option/shape combinations that the C code actually branches on (or that select a distinct code path / constant set). Rejection branches live in `errors_2.md`.

Build assumption: no `HAVE_*` macros are defined by the CMake build, so the following are fixed and are **not** configuration axes:

- `crypto_verify_n` → byte-loop fallback (`verify.c:63`), not the SSE2 `__m128i` variant; `HAVE_INLINE_ASM` optimization barrier absent, so the `optblocker_u16` trick is the only barrier.
- `ed25519_ref10.c` → `fe_25_5` field arithmetic (`10 x 25.5`-bit limbs) + `fe_25_5/base.h` / `fe_25_5/base2.h` precomputed tables; `equal()` / `negative()` take the arithmetic fallback with `optblocker_u8`.
- `keccak1600.c` → `keccak1600_ref_*` (no `__ARM_FEATURE_SHA3`).
- `softaes.c` → the `#else` (non-`FAVOR_PERFORMANCE`) branch, `SOFTAES_STRIDE == 16`, i.e. the on-the-fly 16-entry SBOX slice tables, not `_aes_lut[1024]`.
- `MINIMAL` is not defined → `crypto_core_salsa2012` and `crypto_core_salsa208` are present.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 2.1 | `crypto_verify_16` | `x == y`, both all-zero 16 bytes | [x] |
| 2.2 | `crypto_verify_16` | `x == y`, both random 16 bytes | [x] |
| 2.3 | `crypto_verify_16` | differ at byte `k = 0` only, single-bit flip (`x[0] ^ 0x01`) | [x] |
| 2.4 | `crypto_verify_16` | differ at byte `k = 15` only (last byte, exercises the full loop before divergence) | [x] |
| 2.5 | `crypto_verify_16` | differ at every byte (`y = ~x`) | [x] |
| 2.6 | `crypto_verify_32` | `x == y`, 32 bytes (both all-zero and random) | [x] |
| 2.7 | `crypto_verify_32` | differ at byte `k` for `k ∈ {0, 15, 16, 31}` (spans the 16-byte boundary that the SSE2 variant would chunk on) | [x] |
| 2.8 | `crypto_verify_64` | `x == y`, 64 bytes (both all-zero and random) | [x] |
| 2.9 | `crypto_verify_64` | differ at byte `k` for `k ∈ {0, 31, 32, 63}` | [x] |
| 2.10 | `crypto_verify_16_bytes` / `_32_bytes` / `_64_bytes` | constant getters; must return `16U` / `32U` / `64U` matching the `crypto_verify_*_BYTES` macros | [x] |
| 2.11 | `crypto_core_salsa20` | `rounds = 20`; `c == NULL` → built-in sigma constants `0x61707865, 0x3320646e, 0x79622d32, 0x6b206574`; `k` 32 bytes, `in` 16 bytes, `out` 64 bytes | [x] |
| 2.12 | `crypto_core_salsa20` | `rounds = 20`; `c != NULL` (16-byte custom constant, `LOAD32_LE` into `j0/j5/j10/j15`) | [x] |
| 2.13 | `crypto_core_salsa20` | all-zero `in` and `k`, `c == NULL` (canonical zero-key vector) | [x] |
| 2.14 | `crypto_core_salsa2012` | `rounds = 12`; `c == NULL` (sigma) | [x] |
| 2.15 | `crypto_core_salsa2012` | `rounds = 12`; `c != NULL` (custom 16-byte constant) | [x] |
| 2.16 | `crypto_core_salsa208` | `rounds = 8`; `c == NULL` (sigma) — note the whole `salsa208` API is `__attribute__((deprecated))` in the header | [x] |
| 2.17 | `crypto_core_salsa208` | `rounds = 8`; `c != NULL` (custom 16-byte constant) | [x] |
| 2.18 | `crypto_core_salsa20` vs `_salsa2012` vs `_salsa208` | identical `(in, k, c)` fed to all three; outputs must differ (only the `rounds` argument to the shared static `crypto_core_salsa` changes: 20 / 12 / 8, loop steps by 2) | [x] |
| 2.19 | `crypto_core_salsa20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` and the `salsa2012` / `salsa208` equivalents | getters; `64U / 16U / 32U / 16U` for each of the three families | [x] |
| 2.20 | `crypto_core_hsalsa20` | `c == NULL` → `U32C` sigma constants; `k` 32 bytes, `in` 16 bytes, `out` 32 bytes (`x0, x5, x10, x15, x6..x9`, no feed-forward addition) | [x] |
| 2.21 | `crypto_core_hsalsa20` | `c != NULL` (16-byte custom constant, `LOAD32_LE` branch at `core_hsalsa20_ref2.c:31`) | [x] |
| 2.22 | `crypto_core_hsalsa20` | all-zero `in` and `k`, `c == NULL` | [x] |
| 2.23 | `crypto_core_hsalsa20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` | getters in `core_hsalsa20.c`; `32U / 16U / 32U / 16U` | [x] |
| 2.24 | `crypto_core_hchacha20` | `c == NULL` → literal `0x61707865, 0x3320646e, 0x79622d32, 0x6b206574` into `x0..x3`; 10 double-rounds of `QUARTERROUND`; out = `x0..x3, x12..x15` | [x] |
| 2.25 | `crypto_core_hchacha20` | `c != NULL` (16-byte custom constant, `LOAD32_LE` branch at `core_hchacha20.c:29`) | [x] |
| 2.26 | `crypto_core_hchacha20` | all-zero `in` and `k`, `c == NULL` | [x] |
| 2.27 | `crypto_core_hchacha20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` | getters; `32U / 16U / 32U / 16U` | [x] |
| 2.28 | `crypto_core_keccak1600_statebytes` | must return `sizeof(crypto_core_keccak1600_state) == 224` (opaque `unsigned char[224]`, `CRYPTO_ALIGN(16)`, `#pragma pack(1)`), while `keccak1600_ref_init` only zeroes the first `KECCAK1600_STATEBYTES == 200` | [x] |
| 2.29 | `crypto_core_keccak1600_init` + `_permute_24` | all-zero state; 24 rounds using `keccak_round_constants[0..23]`; 23x `..._IOTA_PRE` + final `..._IOTA` | [x] |
| 2.30 | `crypto_core_keccak1600_init` + `_permute_12` | all-zero state; 12 rounds using `keccak_round_constants[12..23]` only (different round-constant window than 2.29) | [x] |
| 2.31 | `crypto_core_keccak1600_permute_24` | applied twice in a row (state carried across, exercising `LOAD64_LE` / `STORE64_LE` round-trip of a non-zero state) | [x] |
| 2.32 | `crypto_core_keccak1600_permute_24` | non-trivial state: `init` → `xor_bytes` a rate-sized block → `permute_24` | [x] |
| 2.33 | `crypto_core_keccak1600_permute_12` | non-trivial state: `init` → `xor_bytes` → `permute_12` | [x] |
| 2.34 | `crypto_core_keccak1600_xor_bytes` | `offset == 0`, `length` a multiple of 8 (e.g. 136 = SHA3-256 rate, 168 = SHA3-128 rate, 200 = full state) → skips the leading unaligned loop, only the 8-byte `LOAD64_LE`/`STORE64_LE` loop runs | [x] |
| 2.35 | `crypto_core_keccak1600_xor_bytes` | `offset % 8 != 0` (e.g. `offset = 3`) and `length` large enough to cross into the 8-byte loop and leave a `< 8` tail → all three `while` loops execute | [x] |
| 2.36 | `crypto_core_keccak1600_xor_bytes` | `offset == 0`, `0 < length < 8` → leading loop skipped (offset already aligned), 8-byte loop skipped, only the trailing byte loop runs | [x] |
| 2.37 | `crypto_core_keccak1600_xor_bytes` | `length == 0` → no-op for any `offset` (all three loop guards false) | [x] |
| 2.38 | `crypto_core_keccak1600_xor_bytes` | `offset + length == 200` exactly (writes up to the last state byte, never touching the 24 padding bytes of the 224-byte struct) | [x] |
| 2.39 | `crypto_core_keccak1600_extract_bytes` | `offset == 0`, `length == 200` (full state `memcpy`) | [x] |
| 2.40 | `crypto_core_keccak1600_extract_bytes` | `offset != 0` and partial `length` (e.g. `offset = 5`, `length = 32`) | [x] |
| 2.41 | `crypto_core_keccak1600_extract_bytes` | `length == 0` → zero-byte `memcpy`, output untouched | [x] |
| 2.42 | `softaes_expand_key128` (private `private/softaes.h`; reachable only from `aegis128l_soft.c`, `aegis256_soft.c`, `ipcrypt_soft.c` — not from `sodium.h`) | 16-byte key → `SoftAesBlock rkeys[11]`, `w[44]`, `RCON[1..10]`, `sub_word`/`rot_word` on every 4th word | [x] |
| 2.43 | `softaes_expand_key256` | 32-byte key → `SoftAesBlock rkeys[15]`, 60 words, `sub_word` on both the `i % 8 == 0` (with `rot_word` + `RCON`) and `i % 8 == 4` (no rotate) positions | [x] |
| 2.44 | `softaes_invert_key_schedule128` | called after `softaes_expand_key128`; `inv_mix_columns` applied to `rkeys[1..9]` (indices 0 and 10 left alone) | [x] |
| 2.45 | `softaes_invert_key_schedule256` | called after `softaes_expand_key256`; `inv_mix_columns` applied to `rkeys[1..13]` (indices 0 and 14 left alone) | [x] |
| 2.46 | `softaes_inv_mix_columns` | arbitrary block; four `inv_mix_column` calls using `gf_mul_0e/0b/0d/09` | [x] |
| 2.47 | `softaes_block_encrypt` + `softaes_block_encryptlast` | full AES-128 encryption: 9 x `block_encrypt` (SBOX slice tables + `mix_column`) then 1 x `block_encryptlast` (SBOX only, no MixColumns), with `softaes_block_load`/`_store`/`_xor` from the header | [x] |
| 2.48 | `softaes_block_decrypt` + `softaes_block_decryptlast` | full AES-128 decryption using the inverted key schedule (2.44): 9 x `block_decrypt` (`INV_SBOX` + `inv_mix_column`) then 1 x `block_decryptlast` | [x] |
| 2.49 | `softaes_block_encrypt`/`decrypt` round trip | AES-128: encrypt then decrypt an arbitrary block recovers the plaintext | [x] |
| 2.50 | `softaes_block_encrypt`/`decrypt` round trip | AES-256 (14 rounds, `rkeys[15]`) with `expand_key256` + `invert_key_schedule256` | [x] |
| 2.51 | `crypto_core_ed25519_scalar_random` | no inputs; must yield a scalar that is canonical (`< L`) and non-zero, with `r[31] & 0xe0 == 0` because of `r[31] &= 0x1f` before the acceptance test | [x] |
| 2.52 | `crypto_core_ed25519_scalar_invert` | `s = 1` → `recip = 1`; returns `0` | [x] |
| 2.53 | `crypto_core_ed25519_scalar_invert` | `s` a random canonical scalar (`0 < s < L`) → `s * recip mod L == 1`; returns `0` | [x] |
| 2.54 | `crypto_core_ed25519_scalar_invert` | `s = L - 1` (largest canonical scalar) → `recip = L - 1`; returns `0` | [x] |
| 2.55 | `crypto_core_ed25519_scalar_invert` | `s` **non-reduced** (32-byte value `>= L`, e.g. all-`0xff`) — accepted (no canonicity check in this function); `sc25519_invert` operates on `s mod L` implicitly through `sc25519_mul`; returns `0` | [x] |
| 2.56 | `crypto_core_ed25519_scalar_negate` | `s = 0` → `neg = 0` (`2^256*0 + L - 0` reduces to `0`) | [x] |
| 2.57 | `crypto_core_ed25519_scalar_negate` | `s = 1` → `neg = L - 1` | [x] |
| 2.58 | `crypto_core_ed25519_scalar_negate` | `s` random canonical → `s + neg mod L == 0`; uses the 64-byte `t_` with `L` placed at offset 32, `sodium_sub`, then `sc25519_reduce` | [x] |
| 2.59 | `crypto_core_ed25519_scalar_negate` | `s` non-canonical (`s >= L`, up to all-`0xff`) — accepted; `sodium_sub(t_, s_, 64)` may borrow past the `L` block | [x] |
| 2.60 | `crypto_core_ed25519_scalar_complement` | `s = 0` → `comp = 1` (`t_[0]++` before the subtraction) | [x] |
| 2.61 | `crypto_core_ed25519_scalar_complement` | `s = 1` → `comp = 0` | [x] |
| 2.62 | `crypto_core_ed25519_scalar_complement` | `s` random canonical → `s + comp mod L == 1` | [x] |
| 2.63 | `crypto_core_ed25519_scalar_add` | `x, y` both canonical with `x + y < L` → no wrap; `sodium_add` over 32 bytes then `crypto_core_ed25519_scalar_reduce` over the 64-byte buffer | [x] |
| 2.64 | `crypto_core_ed25519_scalar_add` | `x, y` chosen so `x + y >= L` (e.g. both `L - 1`) → reduction path exercised | [x] |
| 2.65 | `crypto_core_ed25519_scalar_add` | `y = 0` (identity) and `x = 0, y = 0` | [x] |
| 2.66 | `crypto_core_ed25519_scalar_add` | `x, y` non-canonical 32-byte values (`>= L`) — accepted; note `sodium_add(x_, y_, 32)` only carries within the first 32 bytes of the 64-byte buffer, so any 33rd-byte carry is dropped before `sc25519_reduce` | [x] |
| 2.67 | `crypto_core_ed25519_scalar_sub` | `x > y`, both canonical → plain difference (implemented as `negate(y)` then `add`) | [x] |
| 2.68 | `crypto_core_ed25519_scalar_sub` | `x < y` → wraps mod `L` | [x] |
| 2.69 | `crypto_core_ed25519_scalar_sub` | `x == y` → `0`; and `y = 0` → `x` | [x] |
| 2.70 | `crypto_core_ed25519_scalar_mul` | `x, y` random canonical → `sc25519_mul` (12 x 21-bit limb schoolbook + Barrett-style reduction with the `666643/470296/654183/997805/136657/683901` constants) | [x] |
| 2.71 | `crypto_core_ed25519_scalar_mul` | `y = 1` (identity) and `y = 0` (annihilator) | [x] |
| 2.72 | `crypto_core_ed25519_scalar_mul` | `x, y` non-canonical (`>= L`); note `sc25519_mul` reads `a11 = load_4(a+28) >> 7` **unmasked**, so bit 255 participates | [x] |
| 2.73 | `crypto_core_ed25519_scalar_reduce` | 64-byte input whose value is already `< L` → output equals the low 32 bytes unchanged | [x] |
| 2.74 | `crypto_core_ed25519_scalar_reduce` | 64-byte input = `L` exactly (little-endian, zero-padded) → output `0` | [x] |
| 2.75 | `crypto_core_ed25519_scalar_reduce` | 64-byte all-`0xff` (maximal non-reduced scalar) → full `sc25519_reduce` carry cascade; `crypto_core_ed25519_NONREDUCEDSCALARBYTES == 64` | [x] |
| 2.76 | `crypto_core_ed25519_scalar_is_canonical` | `s < L` (e.g. `L - 1`) → `1`; `s == L` → `0`; `s` all-`0xff` → `0`; `s = 0` → `1` | [x] |
| 2.77 | `crypto_core_ed25519_scalar_from_string` | `hash_alg = crypto_core_ed25519_H2CSHA256 (1)`; `h_len = HASH_SC_L = 48` → SHA-256 `expand_message_xmd` loop runs 2 iterations (32 + 16-byte truncated `memcpy`); result is the big-endian-to-little-endian-flipped digest reduced mod `L` | [x] |
| 2.78 | `crypto_core_ed25519_scalar_from_string` | `hash_alg = crypto_core_ed25519_H2CSHA512 (2)`; `h_len = 48` → SHA-512 loop runs 1 iteration with a truncated 48-of-64-byte `memcpy`; `empty_block` is 128 bytes instead of 64 | [x] |
| 2.79 | `crypto_core_ed25519_scalar_from_string` | `ctx_len = 0` (`ctx` may be `NULL`, only param 1 is `nonnull`) | [x] |
| 2.80 | `crypto_core_ed25519_scalar_from_string` | `ctx_len = 255` (`0xff`, the largest value taking the direct DST path) | [x] |
| 2.81 | `crypto_core_ed25519_scalar_from_string` | `ctx_len > 255` → `H2C-OVERSIZE-DST-` prefixed re-hash of the DST; `ctx` is replaced by `u0` and `ctx_len` becomes `HASH_BYTES` (32 for SHA-256, 64 for SHA-512) | [x] |
| 2.82 | `crypto_core_ed25519_scalar_from_string` | `msg_len = 0`, and `msg_len` larger than one hash block (e.g. 200 bytes) | [x] |
| 2.83 | `crypto_core_ed25519_is_valid_point` | canonical prime-order-subgroup point, e.g. the Ed25519 base point encoding → `1` (all five checks pass) | [x] |
| 2.84 | `crypto_core_ed25519_is_valid_point` | output of `crypto_core_ed25519_random` → `1` (`ge25519_from_uniform` ends with `ge25519_clear_cofactor`) | [x] |
| 2.85 | `crypto_core_ed25519_add` | two canonical main-subgroup points → `0`, `r` = valid encoding; `ge25519_p3_add` via `ge25519_p3_to_cached` + `ge25519_add_cached` + `ge25519_p1p1_to_p3` | [x] |
| 2.86 | `crypto_core_ed25519_add` | `q` = the identity encoding `01 00 ... 00` → `0`, `r == p` (accepted despite `has_small_order(identity) != 0`) | [x] |
| 2.87 | `crypto_core_ed25519_add` | `p` and `q` a point/negation pair (`q[31] ^= 0x80`) → `0`, `r` = identity encoding | [x] |
| 2.88 | `crypto_core_ed25519_add` | one operand a small-order point (order 2/4/8) → `0`; result leaves the prime-order subgroup | [x] |
| 2.89 | `crypto_core_ed25519_add` | one operand a non-canonical encoding that still decodes (`ge25519_frombytes` succeeds) → `0`; `_add` performs no canonicity check | [x] |
| 2.90 | `crypto_core_ed25519_sub` | two canonical main-subgroup points → `0`; `ge25519_p3_sub` = `ge25519_p3_neg` + `ge25519_p3_add` | [x] |
| 2.91 | `crypto_core_ed25519_sub` | `p == q` → `0`, `r` = identity encoding; and `q` = identity → `r == p` | [x] |
| 2.92 | `crypto_core_ed25519_sub` | small-order / non-canonical operands (mirrors 2.88, 2.89) → `0` | [x] |
| 2.93 | `crypto_core_ed25519_random` | no inputs; internally `randombytes_buf(h, crypto_core_ed25519_UNIFORMBYTES == 32)` then `ge25519_from_uniform` — output always passes `crypto_core_ed25519_is_valid_point` | [x] |
| 2.94 | `ge25519_from_uniform` (private `private/ed25519_ref10.h`; **not** exported in 1.0.23 — reachable only via `crypto_core_ed25519_random`) | `r[31]` bit 5 clear vs set → `x_sign = ((r[31] >> 5) ^ optblocker_u8) >> 2` selects whether `p3.X` is conditionally negated; `s[31] &= 0x7f` masks the input | [x] |
| 2.95 | `ge25519_from_uniform` / `ge25519_elligator2` | `r` such that `gx1 = x1^3 + A*x1^2 + x1` **is** a square (`fe25519_notsquare == 0`) vs **is not** a square (`== 1`, taking the `x = -x1-A` correction with the `ed25519_A` cmov) — both must be covered | [x] |
| 2.96 | `ge25519_mont_to_ed` (inside 2.94/2.95) | the `fe25519_iszero(x_plus_one_y_inv)` cmov path, i.e. `(x+1)*y == 0` → `yed` forced to `1` | [x] |
| 2.97 | `crypto_core_ed25519_from_string_nu` | **NU (non-uniform) variant**: `_string_to_points(p, n = 1, ...)`, `h_len = 1 * HASH_GE_L = 48`; `hash_alg = 1` (SHA-256) | [x] |
| 2.98 | `crypto_core_ed25519_from_string_nu` | NU variant with `hash_alg = 2` (SHA-512) | [x] |
| 2.99 | `crypto_core_ed25519_from_string` | **RO (random-oracle) variant** — this is the `_ro` analogue in 1.0.23 (there is no symbol literally named `..._from_string_ro`): `_string_to_points(px, n = 2, ...)` with `h_len = 2 * 48 = 96`, then `crypto_core_ed25519_add(p, &px[0], &px[32])`; `hash_alg = 1` (SHA-256) | [x] |
| 2.100 | `crypto_core_ed25519_from_string` | RO variant with `hash_alg = 2` (SHA-512); `h_len = 96` makes the SHA-512 expand loop run 2 iterations | [x] |
| 2.101 | `crypto_core_ed25519_from_string_nu` / `_from_string` | `ctx_len = 0` vs `ctx_len = 255` vs `ctx_len > 255` (oversize-DST re-hash), cross-producted with `hash_alg ∈ {1, 2}` | [x] |
| 2.102 | `crypto_core_ed25519_from_string_nu` / `_from_string` | `msg_len = 0` vs `msg_len` > one hash block; also confirm `_from_string_nu(p)` and `_from_string(p)` on the same `(ctx, msg, hash_alg)` produce **different** points | [x] |
| 2.103 | `ge25519_from_hash` (private; reached via 2.97–2.102) | `fe25519_reduce64` on the 64-byte big-endian-flipped digest: `h[31]` and `h[63]` bit-5 contributions (`* 19` and `* 722`) with both bits clear and both set; then `y_sign = notsquare ^ 1` cmov | [x] |
| 2.104 | `crypto_core_ristretto255_is_valid_point` | all-zero 32-byte input = the ristretto255 identity → `1` (canonical, `s[0]` even, `Y == 1`, `T == 0` non-negative) | [x] |
| 2.105 | `crypto_core_ristretto255_is_valid_point` | the ristretto255 basepoint encoding → `1` | [x] |
| 2.106 | `crypto_core_ristretto255_is_valid_point` | output of `crypto_core_ristretto255_random` / `_from_hash` → `1` | [x] |
| 2.107 | `crypto_core_ristretto255_add` | two valid encodings → `0`; `ristretto255_frombytes` x2 + `ge25519_p3_add` + `ristretto255_p3_tobytes` | [x] |
| 2.108 | `crypto_core_ristretto255_add` | `q` = identity (all-zero) → `0`, `r == p` | [x] |
| 2.109 | `crypto_core_ristretto255_sub` | two valid encodings → `0`; and `p == q` → `r` = all-zero identity encoding | [x] |
| 2.110 | `crypto_core_ristretto255_sub` | `q` = identity (all-zero) → `0`, `r == p` | [x] |
| 2.111 | `crypto_core_ristretto255_from_hash` | arbitrary 64-byte `r` → always `0`; `ristretto255_elligator` on `r[0..31]` and `r[32..63]` (each `fe25519_frombytes`, so bit 255 of each half is ignored) then `ge25519_p3_add` | [x] |
| 2.112 | `crypto_core_ristretto255_from_hash` | all-zero 64-byte input → `0`; exercises `ristretto255_elligator` with `t = 0` (`r = 0`, `wasnt_square` path) | [x] |
| 2.113 | `crypto_core_ristretto255_from_hash` | inputs chosen so `ristretto255_sqrt_ratio_m1(s, u, v)` returns 1 (`wasnt_square == 0`) and inputs where it returns 0 (`wasnt_square == 1`, taking the `s_prime = -abs(s*t)` and `c = r` cmovs) — both must be covered | [x] |
| 2.114 | `ristretto255_p3_tobytes` (via 2.107–2.112) | `rotate = fe25519_isnegative(T * z_inv)` both `0` and `1` (the `iy`/`ix`/`eden` cmov triple), plus the `fe25519_isnegative(x_z_inv)` conditional negation of `y_` | [x] |
| 2.115 | `crypto_core_ristretto255_random` | no inputs; `randombytes_buf(h, crypto_core_ristretto255_HASHBYTES == 64)` then `from_hash`; result always passes `_is_valid_point` | [x] |
| 2.116 | `crypto_core_ristretto255_from_string` | `hash_alg = crypto_core_ristretto255_H2CSHA256 (1)`; `h_len = crypto_core_ristretto255_HASHBYTES = 64` → SHA-256 expand loop runs 2 full 32-byte iterations | [x] |
| 2.117 | `crypto_core_ristretto255_from_string` | `hash_alg = crypto_core_ristretto255_H2CSHA512 (2)`; `h_len = 64` → SHA-512 expand loop runs exactly 1 iteration with a full 64-byte `memcpy` | [x] |
| 2.118 | `crypto_core_ristretto255_from_string` | `ctx_len = 0` / `255` / `> 255` (oversize DST), crossed with `msg_len = 0` and `msg_len` > one block | [x] |
| 2.119 | `crypto_core_ristretto255_scalar_random` | delegates verbatim to `crypto_core_ed25519_scalar_random` — canonical, non-zero, `r[31] <= 0x1f` | [x] |
| 2.120 | `crypto_core_ristretto255_scalar_invert` | `s = 1`, `s` random canonical, `s = L - 1`, `s` non-canonical — all → `0` (delegates to the ed25519 version) | [x] |
| 2.121 | `crypto_core_ristretto255_scalar_negate` / `_complement` | `s = 0`, `s = 1`, `s` random canonical, `s` non-canonical (delegates to the ed25519 versions) | [x] |
| 2.122 | `crypto_core_ristretto255_scalar_add` / `_sub` | reduced operands with and without wrap; `0` operands; non-canonical operands (delegates to the ed25519 versions) | [x] |
| 2.123 | `crypto_core_ristretto255_scalar_mul` | random canonical `x, y`; `y = 1`; `y = 0`; non-canonical operands — calls `sc25519_mul` **directly**, not through `crypto_core_ed25519_scalar_mul` | [x] |
| 2.124 | `crypto_core_ristretto255_scalar_reduce` | 64-byte input `< L`; `== L`; all-`0xff` (delegates to `crypto_core_ed25519_scalar_reduce`); `crypto_core_ristretto255_NONREDUCEDSCALARBYTES == 64` | [x] |
| 2.125 | `crypto_core_ristretto255_scalar_is_canonical` | `s < L` → `1`; `s == L` → `0`; `s = 0` → `1`; all-`0xff` → `0` — calls `sc25519_is_canonical` directly | [x] |
| 2.126 | `crypto_core_ristretto255_scalar_from_string` | `hash_alg ∈ {1, 2}` x `ctx_len ∈ {0, 255, > 255}` x `msg_len ∈ {0, large}`; `h_len = HASH_SC_L = 48` (delegates to `crypto_core_ed25519_scalar_from_string`) | [x] |
| 2.127 | `crypto_core_ristretto255_bytes` / `_hashbytes` / `_scalarbytes` / `_nonreducedscalarbytes` and `crypto_core_ed25519_bytes` / `_uniformbytes` / `_hashbytes` / `_scalarbytes` / `_nonreducedscalarbytes` | getters; ed25519: `32 / 32 / 64 / 32 / 64`; ristretto255: `32 / 64 / 32 / 64` | [x] |
| 2.128 | `ge25519_scalarmult` / `ge25519_scalarmult_base` / `ge25519_double_scalarmult_vartime` (private `private/ed25519_ref10.h`; not part of the public `crypto_core_*` surface but defined in `ed25519_ref10.c`) | scalars with `a[31] <= 127` (documented precondition); `a = 0`, `a = 1`, `a = L - 1`; `ge25519_cmov8` / `ge25519_cmov8_cached` / `ge25519_cmov8_base` digit values `e[i] ∈ [-8, 8]` including the `bnegative` branch; `slide_vartime` with the `cmp <= 15`, `cmp < -15` (break) and carry-propagation arms | [x] |

### Notes recorded while ticking the rows

- Rows **2.94** and **2.103** describe the sign/correction bit as "`r[31]` bit 5" because of
  the shape of the C expression `((r[31] >> 5) ^ optblocker_u8) >> 2`. That expression is
  `r[31] >> 7`, i.e. the branch is driven by **bit 7** (`0x80`), not bit 5; the `>> 5` /
  `>> 2` split only exists so that the `volatile` optimisation blocker can be XORed in.
  Both bits are covered by the tests, and `tests/a2_gaps.rs` classifies inputs by bit 7 and
  requires both classes to be non-empty.
- Row **2.96** (`ge25519_mont_to_ed`'s `fe25519_iszero(x_plus_one_y_inv)` cmov) is reachable
  from **exactly one** input: `r == 0` (either sign bit). `x1 == 0` needs `1 + 2r^2 == 0`,
  which has no solution because `-1/2` is a quadratic non-residue mod `2^255-19`; `y == 0`
  with `x != 0` needs `A^2-4` to be a square, which it is not; and `x == 0` after the
  correction needs `x1 == -A`, i.e. `r == 0`, where `-A` is a non-residue so the `notsquare`
  arm is taken. `tests/a2_gaps.rs::mont_to_ed_cmov_path_at_zero` pins this down and checks
  that `ge25519_from_uniform(0)` is the identity encoding.
- Rows **2.95**, **2.113** and **2.114** ask for both arms of a branch inside a `static`
  helper that is invisible from outside. They are ticked on the strength of
  `tests/a2_gaps.rs`, which reimplements `F_p` test-side, replicates
  `ge25519_elligator2` / `ristretto255_elligator` / `ristretto255_p3_tobytes` statement by
  statement, validates each replica end-to-end against the C library through exported entry
  points, and then asserts that both arms are actually taken by the inputs being fed to both
  `.so` files.
