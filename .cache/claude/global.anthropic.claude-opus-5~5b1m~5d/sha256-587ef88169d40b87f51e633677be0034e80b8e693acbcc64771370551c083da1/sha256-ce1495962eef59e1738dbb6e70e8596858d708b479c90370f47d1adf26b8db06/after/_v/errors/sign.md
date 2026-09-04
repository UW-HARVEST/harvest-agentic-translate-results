| sign-E1 | crypto_scalarmult_curve25519 | `implementation->mult(q,n,p) != 0` (marked LCOV_EXCL_LINE) — reached for the 7 blocklisted points | returns -1, q untouched | [x] |
| sign-E2 | crypto_scalarmult_curve25519 | all-zero result: `return -(1 & ((d - 1) >> 8))` where `d = OR of q[0..32]` | returns -1 | [unreachable] |
| sign-E3 | crypto_scalarmult_curve25519_ref10 (implementation->mult) | `has_small_order(p)`: p ∈ {0, 1, 3256…504, 3938…823, p-1, p, p+1} comparing 31 bytes plus `s[31] & 0x7f` (bit 255 ignored) | returns -1 | [x] |
| sign-E4 | crypto_scalarmult_ed25519, _noclamp | `ge25519_is_canonical(p) == 0` (y ≥ 2^255-19) | returns -1, q untouched | [x] |
| sign-E5 | crypto_scalarmult_ed25519, _noclamp | `ge25519_frombytes(&P, p) != 0` (point not on the curve) | returns -1, q untouched | [x] |
| sign-E6 | crypto_scalarmult_ed25519, _noclamp | `ge25519_has_small_order(&P) != 0` (order 1/2/4/8 points) | returns -1, q untouched | [x] |
| sign-E7 | crypto_scalarmult_ed25519, _noclamp | `ge25519_is_on_main_subgroup(&P) == 0` (canonical, on-curve, order 2L/4L/8L) | returns -1, q untouched | [x] |
| sign-E8 | crypto_scalarmult_ed25519_noclamp | `_crypto_scalarmult_ed25519_is_inf(q) != 0`: n ≡ 0 (mod L) after `n[31] &= 127` (n = k·L, k = 1..7) | returns -1, q = identity encoding | [x] |
| sign-E9 | crypto_scalarmult_ed25519 | `_crypto_scalarmult_ed25519_is_inf(q) != 0` with the clamped scalar (multiple of 8, bit 254 set, bit 255 clear) — needs 8L \| t, impossible for t < 2^255 | returns -1 | [unreachable] |
| sign-E10 | crypto_scalarmult_ed25519, _noclamp | `sodium_is_zero(n, 32)` — all-zero scalar, checked AFTER the scalar multiplication | returns -1, q holds the (identity) result | [x] |
| sign-E11 | crypto_scalarmult_ed25519_base_noclamp | `_crypto_scalarmult_ed25519_is_inf(q) != 0`: n = k·L (k = 1..7) or n masking to 0 (`n[31] = 0x80`) | returns -1, q = identity encoding | [x] |
| sign-E12 | crypto_scalarmult_ed25519_base | `_crypto_scalarmult_ed25519_is_inf(q) != 0` with the clamped scalar — needs 8L \| t, impossible for t < 2^255 | returns -1 | [unreachable] |
| sign-E13 | crypto_scalarmult_ed25519_base, _base_noclamp | `sodium_is_zero(n, 32)` — all-zero scalar | returns -1 | [x] |
| sign-E14 | crypto_scalarmult_ristretto255 | `ristretto255_frombytes(&P, p) != 0` (non-canonical / non-square / negative / not a valid ristretto encoding) | returns -1, q untouched | [x] |
| sign-E15 | crypto_scalarmult_ristretto255 | `sodium_is_zero(q, 32)` — result is the ristretto identity (n ≡ 0 mod L after `n[31] &= 127`, or p = identity encoding) | returns -1, q = all zeros | [x] |
| sign-E16 | crypto_scalarmult_ristretto255_base | `sodium_is_zero(q, 32)` — n = 0, n = k·L (k = 1..7), n masking to 0 | returns -1, q = all zeros | [x] |
| sign-E17 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_frombytes_negate_vartime(&A, pk) != 0` (pk not on the curve) | returns -1, output untouched | [x] |
| sign-E18 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_has_small_order(&A) != 0` | returns -1, output untouched | [x] |
| sign-E19 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_is_on_main_subgroup(&A) == 0` | returns -1, output untouched | [x] |
| sign-E20 | _crypto_sign_ed25519_verify_detached | `(sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0` — S ≥ L (S = L, L+1, 2L, 7L, 2^253-ish, all-0xff) | returns -1 | [x] |
| sign-E21 | _crypto_sign_ed25519_verify_detached | `ge25519_is_canonical(pk) == 0` (pk y ≥ 2^255-19, with and without bit 255 set) | returns -1 | [x] |
| sign-E22 | _crypto_sign_ed25519_verify_detached | `ge25519_frombytes_negate_vartime(&A, pk) != 0` (pk not on the curve) | returns -1 | [x] |
| sign-E23 | _crypto_sign_ed25519_verify_detached | `ge25519_has_small_order(&A) != 0` (small-order pk, incl. all-zero pk) | returns -1 | [x] |
| sign-E24 | _crypto_sign_ed25519_verify_detached | `ge25519_frombytes(&expected_r, sig) != 0` (R not on the curve) | returns -1 | [x] |
| sign-E25 | _crypto_sign_ed25519_verify_detached | `ge25519_has_small_order(&expected_r) != 0` (small-order R, incl. all-zero R) | returns -1 | [x] |
| sign-E26 | _crypto_sign_ed25519_verify_detached | final `return ge25519_has_small_order(&check) - 1` — equation does not hold (tampered sig/message/pk) | returns -1 (0 only when `check` has small order) | [x] |
| sign-E27 | crypto_sign_ed25519_verify_detached | ED25519_COMPAT variant `if (sig[63] & 224) return -1;` | not compiled (ED25519_COMPAT undefined) — verified by `tools/cpp.sh` | [n/a] |
| sign-E28 | crypto_sign_ed25519_open, crypto_sign_open | `smlen < 64` → `goto badsig` | returns -1, `*mlen_p = 0`, `m` untouched | [x] |
| sign-E29 | crypto_sign_ed25519_open | `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX - 64 = UINT64_MAX - 64 on LP64) → requires `smlen > UINT64_MAX` | returns -1 | [unreachable] |
| sign-E30 | crypto_sign_ed25519_open | `crypto_sign_ed25519_verify_detached(...) != 0` → `memset(m, 0, mlen)` then `goto badsig` | returns -1, m zeroed (exactly mlen bytes), `*mlen_p = 0` | [x] |
| sign-E31 | crypto_sign_ed25519_open | badsig path with `mlen_p == NULL` and/or `m == NULL` (the C explicitly NULL-checks both) | returns -1, no write | [x] |
| sign-E32 | crypto_sign_ed25519 | `crypto_sign_ed25519_detached(...) != 0 \|\| siglen != crypto_sign_ed25519_BYTES` (LCOV_EXCL_START/STOP) → `*smlen_p = 0`, `memset(sm, 0, mlen + 64)` | returns -1 | [unreachable] |
| sign-E33 | crypto_sign_ed25519, crypto_sign_ed25519_detached, _crypto_sign_ed25519_detached, crypto_sign_ed25519ph_final_create | `siglen_p == NULL` / `smlen_p == NULL` tolerated (explicit NULL check before the store) | returns 0, no store | [x] |
| sign-E34 | crypto_sign_ed25519_sk_to_seed, _sk_to_pk, _sk_to_curve25519, crypto_sign_ed25519ph_init, _update | no rejection sites at all (plain memmove / sha512 delegation) | always returns 0 | [x] |
| sign-E35 | crypto_scalarmult_curve25519_base, crypto_scalarmult_base, crypto_scalarmult_curve25519_ref10_base | no rejection site (the all-zero / small-order check is only in the two-argument entry point) | always returns 0, even for the all-zero scalar | [x] |
| sign-E36 | _crypto_scalarmult_curve25519_pick_best_implementation | no failure path (HAVE_AVX_ASM undefined) | always returns 0 | [x] |
