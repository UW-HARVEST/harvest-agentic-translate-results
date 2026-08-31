## Area 2 — crypto_verify + crypto_core

Files in scope (libsodium 1.0.23):

- `c_src/libsodium/crypto_verify/verify.c`
- `c_src/libsodium/crypto_core/salsa/ref/core_salsa_ref.c`
- `c_src/libsodium/crypto_core/hsalsa20/core_hsalsa20.c`, `crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c`
- `c_src/libsodium/crypto_core/hchacha20/core_hchacha20.c`
- `c_src/libsodium/crypto_core/keccak1600/keccak1600.c`, `keccak1600/ref/keccak1600_ref.c`
- `c_src/libsodium/crypto_core/softaes/softaes.c`
- `c_src/libsodium/crypto_core/ed25519/core_ed25519.c`, `core_ristretto255.c`, `core_h2c.c`, `ref10/ed25519_ref10.c`
- headers: `include/sodium/crypto_verify_{16,32,64}.h`, `crypto_core_salsa{20,2012,208}.h`, `crypto_core_hsalsa20.h`, `crypto_core_hchacha20.h`, `crypto_core_keccak1600.h`, `crypto_core_ed25519.h`, `crypto_core_ristretto255.h`

Build assumption: the CMake build defines **no** `HAVE_*` macros, so `HAVE_EMMINTRIN_H`/`__SSE2__`, `HAVE_INLINE_ASM`, `HAVE_TI_MODE`, `__ARM_FEATURE_SHA3` are all absent. Consequences relevant to this table: `crypto_verify_n` takes the constant-time byte-loop fallback; `equal()`/`negative()` in `ed25519_ref10.c` take the arithmetic fallback; field arithmetic is `fe_25_5` (10x25.5-bit limbs); `keccak1600_*` binds to `keccak1600_ref_*`; `softaes` takes the `#else` (non-`FAVOR_PERFORMANCE`) branch with `SOFTAES_STRIDE == 16`. `MINIMAL` is also not defined, so `crypto_core_salsa2012` / `crypto_core_salsa208` exist.

**Total-function note (no rejection branch at all):** `crypto_verify_{16,32,64}_bytes`, all `crypto_core_salsa*_{output,input,key,const}bytes`, `crypto_core_hsalsa20_*bytes`, `crypto_core_hchacha20_*bytes`, `crypto_core_keccak1600_statebytes`, `crypto_core_ed25519_{bytes,uniformbytes,hashbytes,scalarbytes,nonreducedscalarbytes}`, `crypto_core_ristretto255_{bytes,hashbytes,scalarbytes,nonreducedscalarbytes}`, `crypto_core_salsa20/2012/208` (always `return 0`), `crypto_core_hsalsa20`, `crypto_core_hchacha20` (always `return 0`), `crypto_core_keccak1600_{init,xor_bytes,extract_bytes,permute_24,permute_12}` (`void`), all `softaes_*` (`void`/`SoftAesBlock`, no status), `crypto_core_ristretto255_from_hash` (always `return 0`), `crypto_core_ed25519_random`, `crypto_core_ristretto255_random`, `crypto_core_ed25519_scalar_{negate,complement,add,sub,mul,reduce}` and their `ristretto255_*` wrappers (`void`). These are listed here once and do not occupy rows below.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 2.1 | `crypto_verify_16` (`verify.c:89`) | `x` and `y` differ in at least one of bytes 0..15 (fallback path: `d = OR of x[i]^y[i]` is nonzero) | `-1` | verified |
| 2.2 | `crypto_verify_32` (`verify.c:95`) | `x` and `y` differ in at least one of bytes 0..31 | `-1` | verified |
| 2.3 | `crypto_verify_64` (`verify.c:101`) | `x` and `y` differ in at least one of bytes 0..63 | `-1` | verified |
| 2.4 | `ge25519_is_canonical` (`ed25519_ref10.c:1156`) | encoding is non-canonical: `s[0] >= 0xed` AND `s[1..30] == 0xff` AND `(s[31] & 0x7f) == 0x7f` (i.e. `y >= 2^255-19`) | `0` (rejects) | verified |
| 2.5 | `ge25519_frombytes` (`ed25519_ref10.c:326`) | `y` (from `s`) admits no `x`: neither `vx^2-u == 0` nor `vx^2+u == 0`, i.e. `has_m_root == 0 && has_p_root == 0`; return value is `(has_m_root \| has_p_root) - 1` | `-1` | verified |
| 2.6 | `ge25519_frombytes_negate_vartime` (`ed25519_ref10.c:364`) | `fe25519_iszero(vx^2-u) == 0` and `fe25519_iszero(vx^2+u) == 0` (no square root for the given `y`) | `-1` | verified |
| 2.7 | `ge25519_is_on_curve` (`ed25519_ref10.c:1118`) | `(Y^2-X^2)Z^2 - (d*X^2*Y^2 + Z^4) != 0` (coords do not satisfy the twisted Edwards equation) | `0` (rejects) | verified |
| 2.8 | `ge25519_has_small_order` (`ed25519_ref10.c:1173`) | any of `X == 0`, `Y == 0`, `Z == 0`, `Y*sqrt(-1) - X == 0`, `Y*sqrt(-1) + X == 0` — the 8 points of order dividing 8, including the identity `(0,1)` | non-zero (`1`); caller treats as "reject" | verified |
| 2.9 | `ge25519_is_on_main_subgroup` (`ed25519_ref10.c:1143`) | `L*P != identity`, i.e. `fe25519_iszero(pl.X) & fe25519_iszero(pl.Y - pl.Z) == 0` | `0` (rejects) | verified |
| 2.10 | `fe25519_sqrt` (`ed25519_ref10.c:207`, static) | `x2` is not a quadratic residue mod `2^255-19`: `x^2 - x2 != 0`; return is `fe25519_iszero(check) - 1` | `-1` | unreachable-from-public-API |
| 2.11 | `sc25519_is_canonical` (`ed25519_ref10.c:2574`) | 32-byte scalar `s >= L` where `L = 2^252+27742317777372353535851937790883648493` (borrow chain leaves `c == 0`) | `0` (rejects) | verified |
| 2.12 | `ristretto255_is_canonical` (`ed25519_ref10.c:2802`, static) | any of: `s >= 2^255-19` (`c & d` set), bit 255 of `s[31]` set (`e` set), or `s[0]` odd (`s[0] & 1`) — expression `1 - (((c & d) \| e \| s[0]) & 1)` | `0` (rejects) | verified |
| 2.13 | `ristretto255_frombytes` (`ed25519_ref10.c:2821`) | `ristretto255_is_canonical(s) == 0` (see 2.12) — early return before any field work | `-1` | verified |
| 2.14 | `ristretto255_frombytes` | `ristretto255_sqrt_ratio_m1(inv_sqrt, 1, v*u2^2)` returns 0, i.e. `1/(v*u2^2)` is not a square; contributes `(1 - notsquare)` to `return -(...)` | `-1` | verified |
| 2.15 | `ristretto255_frombytes` | decoded `T = X*Y` is "negative" (`fe25519_isnegative(h->T) != 0`) | `-1` | verified |
| 2.16 | `ristretto255_frombytes` | decoded `Y == 0` (`fe25519_iszero(h->Y) != 0`) | `-1` | verified |
| 2.17 | `ristretto255_sqrt_ratio_m1` (`ed25519_ref10.c:2766`, static) | neither `vx^2-u == 0` nor `vx^2+u == 0` (`has_m_root \| has_p_root == 0`); `x` is still set to `abs(x*sqrt(-1))` | `0` ("was not a square"); caller (2.14) turns this into `-1` | verified |
| 2.18 | `crypto_core_ed25519_is_valid_point` (`core_ed25519.c:14`) | `ge25519_is_canonical(p) == 0` — non-canonical 32-byte encoding (see 2.4) | `0` | verified |
| 2.19 | `crypto_core_ed25519_is_valid_point` | `ge25519_frombytes(&p_p3, p) != 0` — `y` has no matching `x` (see 2.5) | `0` | verified |
| 2.20 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_curve(&p_p3) == 0` (see 2.7) | `0` | verified |
| 2.21 | `crypto_core_ed25519_is_valid_point` | `ge25519_has_small_order(&p_p3) != 0` — small-order point or the identity `01 00 ... 00` (see 2.8) | `0` | verified |
| 2.22 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_main_subgroup(&p_p3) == 0` — on-curve point of order `8L`/`2L`/`4L` not in the prime-order subgroup (see 2.9) | `0` | verified |
| 2.23 | `crypto_core_ed25519_add` (`core_ed25519.c:29`) | `ge25519_frombytes(&p_p3, p) != 0` — first operand `p` decodes to no curve point | `-1` | verified |
| 2.24 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(&p_p3) == 0` for first operand | `-1` | verified |
| 2.25 | `crypto_core_ed25519_add` | `ge25519_frombytes(&q_p3, q) != 0` — second operand `q` decodes to no curve point | `-1` | verified |
| 2.26 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(&q_p3) == 0` for second operand | `-1` | verified |
| 2.27 | `crypto_core_ed25519_sub` (`core_ed25519.c:45`) | `ge25519_frombytes(&p_p3, p) != 0` | `-1` | verified |
| 2.28 | `crypto_core_ed25519_sub` | `ge25519_is_on_curve(&p_p3) == 0` | `-1` | verified |
| 2.29 | `crypto_core_ed25519_sub` | `ge25519_frombytes(&q_p3, q) != 0` | `-1` | verified |
| 2.30 | `crypto_core_ed25519_sub` | `ge25519_is_on_curve(&q_p3) == 0` | `-1` | verified |
| 2.31 | `_string_to_points` (`core_ed25519.c:63`, static) | `n > 2U` | aborts (`abort()`, `core_ed25519.c:73`); unreachable from the public API — callers pass only `n = 1` or `n = 2` | unreachable-from-public-API |
| 2.32 | `_string_to_points` | `core_h2c_string_to_hash(...) != 0`, i.e. `hash_alg` is neither `1` (`CORE_H2C_SHA256`) nor `2` (`CORE_H2C_SHA512`) | `-1` | verified |
| 2.33 | `crypto_core_ed25519_from_string_nu` (`core_ed25519.c:92`) | `hash_alg` not in `{crypto_core_ed25519_H2CSHA256 (1), crypto_core_ed25519_H2CSHA512 (2)}` (propagated from 2.32) | `-1`, `errno == EINVAL` | verified |
| 2.34 | `crypto_core_ed25519_from_string` (`core_ed25519.c:101`) | `hash_alg` not in `{1, 2}` — `_string_to_points(px, 2, ...) != 0` | `-1`, `errno == EINVAL` | verified |
| 2.35 | `crypto_core_ed25519_from_string` | tail call `crypto_core_ed25519_add(p, &px[0], &px[32])` fails (would require `ge25519_from_hash` to emit a non-decodable encoding) | `-1`; not reachable in practice — `ge25519_from_hash` always emits a valid on-curve point | unreachable-from-public-API |
| 2.36 | `crypto_core_ed25519_scalar_invert` (`core_ed25519.c:135`) | `s` is the all-zero 32-byte scalar: `- sodium_is_zero(s, 32)` | `-1`; note `sc25519_invert` still ran and `recip` was written (all-zero output, since `0^(L-2) mod L == 0`) | verified |
| 2.37 | `crypto_core_ed25519_scalar_from_string` (`core_ed25519.c:240`) | `hash_alg` not in `{1, 2}` — `core_h2c_string_to_hash(h_be, 48, ...) != 0` | `-1`, `errno == EINVAL` | verified |
| 2.38 | `crypto_core_ed25519_scalar_is_canonical` (`core_ed25519.c:232`) | `s >= L` (delegates to `sc25519_is_canonical`, see 2.11) | `0` | verified |
| 2.39 | `crypto_core_ed25519_scalar_random` (`core_ed25519.c:125`) | drawn `r` (after `r[31] &= 0x1f`) is non-canonical (`sc25519_is_canonical(r) == 0`) or all-zero (`sodium_is_zero(r, 32)`) | no error return (`void`); the `do { ... } while` re-draws from `randombytes_buf` until accepted | verified |
| 2.40 | `ge25519_elligator2` (`ed25519_ref10.c:2653`, static) | `ge25519_xmont_to_ymont(y, x) != 0`, i.e. the recovered `x^3+Ax^2+x` is a non-square after the `notsquare` correction | aborts (`abort()`, `ed25519_ref10.c:2684`); mathematically unreachable (`LCOV_EXCL_LINE`) | unreachable-from-public-API |
| 2.41 | `core_h2c_string_to_hash` (`core_h2c.c:120`) | `hash_alg` matches neither `CORE_H2C_SHA256 (1)` nor `CORE_H2C_SHA512 (2)` — `default:` arm | sets `errno = EINVAL`, returns `-1` | verified |
| 2.42 | `core_h2c_string_to_hash_sha256` (`core_h2c.c:14`, static) | `h_len > 0xff` | aborts (`assert(h_len <= 0xff)`, `core_h2c.c:26`; no-op if `NDEBUG`); unreachable from the public API — callers pass `h_len` in `{48, 64, 96}` | verified |
| 2.43 | `core_h2c_string_to_hash_sha512` (`core_h2c.c:70`, static) | `h_len > 0xff` | aborts (`assert(h_len <= 0xff)`, `core_h2c.c:82`; no-op if `NDEBUG`); unreachable from the public API | verified |
| 2.44 | `crypto_core_ristretto255_is_valid_point` (`core_ristretto255.c:16`) | `ristretto255_frombytes(&p_p3, p) != 0` for any of the four reasons 2.13–2.16 (non-canonical / `s[31]` high bit set / `s[0]` odd / `s >= p` / non-square / `T` negative / `Y == 0`) | `0` | verified |
| 2.45 | `crypto_core_ristretto255_add` (`core_ristretto255.c:27`) | `ristretto255_frombytes(&p_p3, p) != 0` — first operand not a valid ristretto255 encoding | `-1` | verified |
| 2.46 | `crypto_core_ristretto255_add` | `ristretto255_frombytes(&q_p3, q) != 0` — second operand not a valid ristretto255 encoding | `-1` | verified |
| 2.47 | `crypto_core_ristretto255_sub` (`core_ristretto255.c:43`) | `ristretto255_frombytes(&p_p3, p) != 0` | `-1` | verified |
| 2.48 | `crypto_core_ristretto255_sub` | `ristretto255_frombytes(&q_p3, q) != 0` | `-1` | verified |
| 2.49 | `_string_to_element` (`core_ristretto255.c:67`, static) | `core_h2c_string_to_hash(h, 64, ...) != 0`, i.e. `hash_alg` not in `{1, 2}` | `-1` (`LCOV_EXCL_LINE`) | verified |
| 2.50 | `crypto_core_ristretto255_from_string` (`core_ristretto255.c:84`) | `hash_alg` not in `{crypto_core_ristretto255_H2CSHA256 (1), crypto_core_ristretto255_H2CSHA512 (2)}` | `-1`, `errno == EINVAL` | verified |
| 2.51 | `crypto_core_ristretto255_scalar_invert` (`core_ristretto255.c:108`) | `s` is the all-zero 32-byte scalar (delegates to `crypto_core_ed25519_scalar_invert`, see 2.36) | `-1` | verified |
| 2.52 | `crypto_core_ristretto255_scalar_is_canonical` (`core_ristretto255.c:157`) | `s >= L` (calls `sc25519_is_canonical` directly, see 2.11) | `0` | verified |

### Phase-C status notes

- **`verified`** (48 rows) — a differential test in `translation/tests/a2_*.rs` actually drove
  the branch on both `.so` files and compared the outcome. Rows whose trigger lives in a
  `static` helper were driven either through the exported internal symbol
  (`_sodium_ge25519_*`, `_sodium_sc25519_*`, `_sodium_ristretto255_*`,
  `_sodium_core_h2c_string_to_hash`) or through the public wrapper that propagates the
  status (2.32 via `crypto_core_ed25519_from_string{,_nu}`, 2.49 via
  `crypto_core_ristretto255_from_string`).
- **`unreachable-from-public-API`** (4 rows):
  - **2.10** `fe25519_sqrt` — `static`, and its *only* caller is `ge25519_xmont_to_ymont`,
    whose non-zero return makes `ge25519_elligator2` `abort()` (row 2.40). So the `-1`
    return can only be produced along an unreachable path.
  - **2.31** `_string_to_points(n > 2)` — `static`; the only two call sites pass the literal
    `1` and `2`.
  - **2.35** `crypto_core_ed25519_from_string`'s tail `crypto_core_ed25519_add` failure —
    `ge25519_from_hash` always emits a canonical, on-curve, cofactor-cleared encoding, so
    the `add` can never fail. `tests/a2_gaps.rs` re-derives `ge25519_from_hash` byte for byte
    and confirms the output always decodes.
  - **2.40** `ge25519_elligator2`'s `abort()` — the curve equation guarantees
    `x^3+Ax^2+x` is a square after the `notsquare` correction. `tests/a2_gaps.rs` asserts
    exactly this (`assert!(ok, ...)` inside its `elligator2` replica) over every input it feeds.
- Rows **2.42 / 2.43** are marked `verified` rather than `unreachable-from-public-API`
  because `core_h2c_string_to_hash` *is* exported as `_sodium_core_h2c_string_to_hash`, and
  `tests/a2_gaps.rs::core_h2c_h_len_assert_is_live` calls it with `h_len > 0xff` and checks
  that both libraries die on a fatal signal (and that `h_len <= 0xff` does *not* abort), which
  also proves `NDEBUG` is absent from this build. They remain unreachable from `sodium.h`.

### Rejection-surface remarks worth carrying into the Rust port

- **`crypto_core_ed25519_add`/`_sub` are deliberately weaker than `_is_valid_point`.** They only require `ge25519_frombytes` + `ge25519_is_on_curve`; they do **not** call `ge25519_is_canonical`, `ge25519_has_small_order`, or `ge25519_is_on_main_subgroup`. So the identity, all 8 small-order points, cofactor points, and non-canonical encodings that still decode are all *accepted* and return `0`.
- **`crypto_core_ed25519_scalar_invert` writes `recip` before deciding the return value** (rows 2.36/2.51). The out-buffer is always fully written, even on the `-1` path.
- **`ge25519_frombytes` is constant-time and sign-blinded** (`optblocker_u8`), while `ge25519_frombytes_negate_vartime` short-circuits — the two have the same accept/reject set but different control flow.
- **`ristretto255_frombytes` folds four independent rejections into one `-1`** via `- ((1 - notsquare) | isnegative(T) | iszero(Y))` plus the early canonical check; there is no way for a caller to distinguish them.
- **`errno`**: only `core_h2c_string_to_hash`'s `default:` arm sets `errno` (`EINVAL`). Every other `-1` in this area leaves `errno` untouched.
- **`assert`**: only in `core_h2c.c` (rows 2.42/2.43), compiled out under `NDEBUG`. `abort()` appears at `core_ed25519.c:73` and `ed25519_ref10.c:2684`, both unreachable from the public API. **No `sodium_misuse()` call exists anywhere in this area.**
- **No `return NULL`** anywhere in this area — every function returns `int`, `size_t`, `void`, or `SoftAesBlock`.
