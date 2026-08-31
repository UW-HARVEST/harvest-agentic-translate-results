## Area 7 — scalarmult / sign / box / kx / kdf / kem

Files covered: as listed in `errors_7.md` (same file set, read in full).

### Configuration axes extracted from the source

| axis | values |
|------|--------|
| curve25519 scalarmult entry point | `crypto_scalarmult` / `crypto_scalarmult_curve25519` (full, 2-arg point) vs `crypto_scalarmult_base` / `crypto_scalarmult_curve25519_base` |
| curve25519 scalar shape | arbitrary 32 bytes (always clamped: `t[0] &= 248; t[31] &= 127; t[31] |= 64`), all-zero, all-`0xff`, `L`, pre-clamped |
| curve25519 point shape | basepoint `09 00 … 00`, random valid point, non-canonical-but-not-blocklisted (`>= p`), blocklisted small-order (7 encodings) |
| ed25519 scalarmult variant | `crypto_scalarmult_ed25519` (clamped, point) / `_noclamp` (point) / `_base` (clamped) / `_base_noclamp` |
| ristretto255 scalarmult variant | `crypto_scalarmult_ristretto255` (point) / `_base`; never clamped, only `t[31] &= 127` |
| sign API shape | attached one-shot (`crypto_sign` / `_open`) vs detached one-shot (`crypto_sign_detached` / `_verify_detached`) vs multipart prehashed (`crypto_sign_init` / `_update` / `_final_create` / `_final_verify`) |
| sign namespace | generic `crypto_sign_*` vs explicit `crypto_sign_ed25519*` / `crypto_sign_ed25519ph_*` (the generic ones are pure aliases) |
| multipart chunking | 0 / 1 / 2 / many `_update` calls; chunk boundaries at 0, 1, 63, 64, 65, 127, 128, 129 bytes |
| keypair source | `_seed_keypair(seed)` (deterministic) vs `_keypair()` (`randombytes_buf`) |
| key conversions | `crypto_sign_ed25519_sk_to_seed`, `_sk_to_pk`, `_pk_to_curve25519`, `_sk_to_curve25519` |
| message length | 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 8192 (spans SHA-512 block boundaries at 128 and the 112-byte padding cliff) |
| `siglen_p` / `smlen_p` / `mlen_p` / `m` | NULL vs non-NULL (each is explicitly NULL-checked) |
| box AEAD primitive | `curve25519xsalsa20poly1305` (default, generic `crypto_box_*`) vs `curve25519xchacha20poly1305` |
| box API shape | `_easy` / `_open_easy`; `_detached` / `_open_detached`; `_beforenm` + `_easy_afternm` / `_open_easy_afternm` / `_detached_afternm` / `_open_detached_afternm`; NaCl padded `crypto_box` / `_open` / `_afternm` / `_open_afternm` (xsalsa only); `_seal` / `_seal_open` |
| kx role | `crypto_kx_client_session_keys` vs `crypto_kx_server_session_keys` (loop order is swapped) |
| kx output pointers | both non-NULL, `rx == NULL`, `tx == NULL` |
| kdf primitive | blake2b (`crypto_kdf_*`) vs hkdf-sha256 vs hkdf-sha512 |
| kdf blake2b axes | `subkey_len` ∈ {16 (MIN), 17, 31, 32, 33, 63, 64 (MAX)}; `subkey_id` ∈ {0, 1, 2, 2^32-1, 2^32, 2^63, 2^64-1}; 8-byte `ctx` |
| hkdf axes | `extract` one-shot vs `extract_init` + N × `extract_update` + `extract_final`; `salt_len` ∈ {0, 1, 32, 64, 128, 129}; `ikm_len` ∈ {0, 1, 32, 64, 1000}; `out_len` ∈ {0 (MIN), 1, 31, 32, 33, 63, 64, 65, 8160 / 16320 (MAX)}; `ctx_len` ∈ {0, 1, 8, 64} |
| kem primitive | `mlkem768` vs `xwing` vs generic `crypto_kem_*` (→ xwing) |
| kem API shape | `_keypair` / `_seed_keypair`; `_enc` (randomised) / `_enc_deterministic`; `_dec` |
| dispatch | `_crypto_scalarmult_curve25519_pick_best_implementation` — only one value on this build (ref10); sandy2x not selected |

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 7.1 | `crypto_scalarmult_curve25519_base`, `crypto_scalarmult_base` | RFC 7748 vector: `n = a5 46 e3 6b f0 52 7c 9d 3b 16 15 4b 82 46 5e dd 62 14 4c 0a c1 fc 5a 18 50 6a 22 44 ba 44 9a c4` → `q = 8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a` | [x] |
| 7.2 | `crypto_scalarmult_curve25519_base` | `n` = 32 zero bytes (clamps to scalar `2^254`) → succeeds, `0` | [x] |
| 7.3 | `crypto_scalarmult_curve25519_base` | `n` = 32 `0xff` bytes (clamps to `2^254 + …`), and `n = L` little-endian → succeeds, `0` | [x] |
| 7.4 | `crypto_scalarmult_curve25519_base` | `n` already clamped (bit 255 clear, bit 254 set, low 3 bits clear) — clamping must be idempotent | [x] |
| 7.5 | `crypto_scalarmult_curve25519_base` | in-place aliasing: `q == n` (the impl copies `n` into `t = q` first, so this is the *intended* usage) | [x] |
| 7.6 | `crypto_scalarmult_curve25519`, `crypto_scalarmult` | RFC 7748 X25519 vector 1: `n = a546…49ac4`, `p = e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c` → `q = c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552` | [x] |
| 7.7 | `crypto_scalarmult_curve25519`, `crypto_scalarmult` | `p` = basepoint `09 00 … 00` with a random clamped `n`; cross-check equality with `crypto_scalarmult_curve25519_base(q, n)` — the two code paths are structurally different (Montgomery ladder vs `ge25519_scalarmult_base` + `edwards_to_montgomery`) and must agree | [x] |
| 7.8 | `crypto_scalarmult_curve25519` | `p` non-canonical but **not** blocklisted, e.g. `p` = `ef ff … ff 7f` (`= p+2`) or any value in `[p+2, 2^255)` → accepted, `fe25519_frombytes` reduces mod `p`; returns `0` | [x] |
| 7.9 | `crypto_scalarmult_curve25519` | `p` with bit 255 set (`p[31] |= 0x80`) on an otherwise valid point → accepted; `fe25519_frombytes` masks bit 255 | [x] |
| 7.10 | `crypto_scalarmult_curve25519` | DH agreement round trip: `base(pkA, skA)`, `base(pkB, skB)`, then `mult(s1, skA, pkB)` == `mult(s2, skB, pkA)`; over 100 random keypairs | [x] |
| 7.11 | `crypto_scalarmult_curve25519` | `n` = 32 zero bytes with a valid `p` (clamping ⇒ effective scalar `2^254`, so this **succeeds** with `0`, unlike ed25519/ristretto) | [x] |
| 7.12 | `_crypto_scalarmult_curve25519_pick_best_implementation` | called (e.g. via `sodium_init`) then re-run 7.6 — on this build (no `HAVE_AVX_ASM`) the selected implementation is always ref10, so results must be unchanged | [x] |
| 7.13 | `crypto_scalarmult_ed25519_base` | random 32-byte `n` → 32-byte compressed Edwards point; verify `ge25519_is_canonical` holds and the point is on the main subgroup | [x] |
| 7.14 | `crypto_scalarmult_ed25519_base` vs `crypto_scalarmult_ed25519_base_noclamp` | the same `n` through both: with clamped `n` (bit 254 set, low 3 bits clear, bit 255 clear) the two must agree; with any other `n` they must differ | [x] |
| 7.15 | `crypto_scalarmult_ed25519_base_noclamp` | `n = 1` (`01 00 … 00`) → must equal the ed25519 basepoint encoding `5866666666666666666666666666666666666666666666666666666666666666` | [x] |
| 7.16 | `crypto_scalarmult_ed25519_base_noclamp` | `n = 2, 3, 8` and `n = L - 1` (→ `-B`); check additive homomorphism against `crypto_core_ed25519_add` where available | [x] |
| 7.17 | `crypto_scalarmult_ed25519_base_noclamp` | `n` with bit 255 set — must give the same result as `n` with bit 255 cleared (`t[31] &= 127` is applied on all four ed25519 paths) | [x] |
| 7.18 | `crypto_scalarmult_ed25519` | `p` = output of `crypto_scalarmult_ed25519_base(n1)`, `n = n2` → equals `crypto_scalarmult_ed25519_base_noclamp(clamp(n2) · clamp(n1) mod L)`; commutativity check `mult(n1, base(n2)) == mult(n2, base(n1))` for clamped scalars | [x] |
| 7.19 | `crypto_scalarmult_ed25519_noclamp` | `p` = basepoint encoding, `n` random with bit 255 clear → must equal `crypto_scalarmult_ed25519_base_noclamp(n)` | [x] |
| 7.20 | `crypto_scalarmult_ed25519` / `_noclamp` | in-place aliasing `q == n` (documented pattern: `unsigned char *t = q`) and `q == p` (the point is decoded into `P` *before* `t = q` is written, so this is safe) | [x] |
| 7.21 | `crypto_scalarmult_ed25519_base` / `_base_noclamp` | in-place aliasing `q == n` | [x] |
| 7.22 | `crypto_scalarmult_ristretto255_base` | `n = 1` → the canonical ristretto255 basepoint `e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76`; also `n = 2, 3, 15` against the published ristretto255 multiples-of-basepoint vectors | [x] |
| 7.23 | `crypto_scalarmult_ristretto255_base` | random `n`; then feed the result as `p` into `crypto_scalarmult_ristretto255` with a second scalar and check DH commutativity | [x] |
| 7.24 | `crypto_scalarmult_ristretto255` | `p` = a valid ristretto255 encoding, `n` with bit 255 set vs cleared → identical results (`t[31] &= 127`) | [x] |
| 7.25 | `crypto_scalarmult_ristretto255` | `n = 1` with any valid `p` → `q == p` (identity map; verifies canonical re-encoding) | [x] |
| 7.26 | `crypto_scalarmult_ristretto255` / `_base` | in-place aliasing `q == n`; and `q == p` for the point variant | [x] |
| 7.27 | `crypto_scalarmult_ristretto255` vs `crypto_scalarmult_ed25519_noclamp` | same `n`, `p` chosen so both decode — results must **differ** (different encodings/cofactor handling); documents that the two are not interchangeable | [x] |
| 7.28 | `crypto_scalarmult_bytes`, `_scalarbytes`, `_primitive`, `crypto_scalarmult_curve25519_bytes/_scalarbytes`, `crypto_scalarmult_ed25519_bytes/_scalarbytes`, `crypto_scalarmult_ristretto255_bytes/_scalarbytes` | all 10 accessors → `32` × 9 and `"curve25519"` | [x] |
| 7.29 | `crypto_sign_ed25519_seed_keypair` | RFC 8032 test 1 seed `9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60` → `pk = d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`, `sk = seed ‖ pk` | [x] |
| 7.30 | `crypto_sign_ed25519_seed_keypair`, `crypto_sign_seed_keypair` | seed = 32 zero bytes; seed = 32 `0xff` bytes; the generic alias must produce byte-identical output to the ed25519 form | [x] |
| 7.31 | `crypto_sign_ed25519_keypair`, `crypto_sign_keypair` | randomised: verify `sk[0..31]` is the seed, `sk[32..63] == pk`, and `seed_keypair(sk[0..31])` reproduces both | [x] |
| 7.32 | `crypto_sign_ed25519_sk_to_seed`, `_sk_to_pk` | round trip after `_keypair`: `sk_to_seed(sk) == sk[0..31]`, `sk_to_pk(sk) == pk`; also with overlapping buffers (both use `memmove`) | [x] |
| 7.33 | `crypto_sign_ed25519_detached`, `crypto_sign_detached` | RFC 8032 test 1: empty message, seed as 7.29 → `sig = e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b`; `siglen_p` non-NULL must receive `64` | [x] |
| 7.34 | `crypto_sign_ed25519_detached` | RFC 8032 tests 2 (1-byte `72`), 3 (2-byte `af82`), and 1024 (the 1023-byte message) → published signatures | [x] |
| 7.35 | `crypto_sign_ed25519_detached` | `siglen_p == NULL` → still succeeds, `sig` written, no store | [x] |
| 7.36 | `crypto_sign_ed25519_detached` | message lengths 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 8192 — verifies the SHA-512 block/padding boundaries inside the two `hinit`+`update`+`final` passes | [x] |
| 7.37 | `crypto_sign_ed25519`, `crypto_sign` (attached) | `sm` buffer of `mlen + 64`; message lengths as 7.36; `*smlen_p == mlen + 64`; `sm[64…] == m`; `sm[0..63]` equals the detached signature over the same message | [x] |
| 7.38 | `crypto_sign_ed25519` | `smlen_p == NULL` → succeeds; and `m == sm + 64` (in-place signing — the `memmove` at `sign.c:111` is a no-op then) | [x] |
| 7.39 | `crypto_sign_ed25519` | `mlen == 0` → `sm` is exactly 64 bytes, `*smlen_p == 64` | [x] |
| 7.40 | `crypto_sign_ed25519_open`, `crypto_sign_open` | round trip against 7.37 for every message length; `*mlen_p == smlen - 64`, `m == original` | [x] |
| 7.41 | `crypto_sign_ed25519_open` | `m == NULL` (verify-only mode) with `mlen_p` non-NULL → `0`, `*mlen_p` set, nothing written | [x] |
| 7.42 | `crypto_sign_ed25519_open` | `mlen_p == NULL` with `m` non-NULL → `0`, message copied | [x] |
| 7.43 | `crypto_sign_ed25519_open` | `m == sm` (fully in-place open — the `memmove` at `open.c:95` shifts down by 64) | [x] |
| 7.44 | `crypto_sign_ed25519_open` | `smlen == 64` exactly (empty signed message) → `0`, `*mlen_p == 0` | [x] |
| 7.45 | `crypto_sign_ed25519_verify_detached`, `crypto_sign_verify_detached` | valid `sig`/`m`/`pk` for every message length of 7.36 → `0`; the generic alias must agree bit-for-bit with the ed25519 form | [x] |
| 7.46 | `crypto_sign_ed25519_verify_detached` — strict-vs-compat axis | `sig` with `(sig[63] & 240) == 0` (short-circuits the `sc25519_is_canonical` call) vs `(sig[63] & 240) != 0` with a canonical `S` (takes the canonicality call and passes) — both must return `0`. Documents that the build has no `ED25519_COMPAT`, so `sig[63] & 224` is *not* the guard | [x] |
| 7.47 | `crypto_sign_ed25519_verify_detached` | `pk` of order `8L` (valid point, non-small-order, off the main subgroup) with a signature that verifies cofactored → **accepted** (`0`), unlike `pk_to_curve25519` | [x] |
| 7.48 | `_crypto_sign_ed25519_verify_detached` cofactored acceptance | `sig` with a torsion component added to `R` such that `check` is small-order but not the identity → `0` | [x] |
| 7.49 | `crypto_sign_init`, `crypto_sign_update`, `crypto_sign_final_create` / `crypto_sign_ed25519ph_init/_update/_final_create` | **0 update calls** (prehash = SHA-512 of the empty string) → sign, then `crypto_sign_final_verify` with a fresh `init`+0 updates → `0` | [x] |
| 7.50 | multipart sign | **1 update call** with lengths 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024 | [x] |
| 7.51 | multipart sign | **many update calls**: split a 1024-byte message as 1+1+…, as 127+1+128+…, as 64×16, and as one 1023-byte + one 1-byte chunk — all must yield the identical signature (streaming invariance) | [x] |
| 7.52 | multipart sign | `crypto_sign_final_create` with `siglen_p == NULL` and with non-NULL (`== 64`) | [x] |
| 7.53 | multipart sign vs one-shot | `crypto_sign_ed25519ph_final_create` over message `M` must **not** equal `crypto_sign_ed25519_detached` over `M` (`DOM2PREFIX` domain separation) — and `final_verify` must accept only its own | [x] |
| 7.54 | `crypto_sign_final_verify`, `crypto_sign_ed25519ph_final_verify` | round trip for 0/1/many-chunk configurations of 7.49–7.51; also with a state chunked differently on the verify side than on the sign side (must still verify — only the concatenation matters) | [x] |
| 7.55 | `crypto_sign_statebytes`, `crypto_sign_ed25519ph_statebytes` | both → `sizeof(crypto_hash_sha512_state)` = `208` on LP64; a heap-allocated state of exactly that size must work | [x] |
| 7.56 | `crypto_sign_ed25519_pk_to_curve25519` | after `crypto_sign_ed25519_seed_keypair(pk, sk, seed)`: `pk_to_curve25519(cpk, pk)` must equal `crypto_scalarmult_curve25519_base(cpk2, sk_to_curve25519(csk, sk))` — i.e. the two conversions must be mutually consistent | [x] |
| 7.57 | `crypto_sign_ed25519_sk_to_curve25519` | `csk = SHA-512(sk[0..31])[0..31]` clamped; verify against the value implied by `crypto_sign_ed25519_seed_keypair`'s internal clamp; input `sk` = 64 zero bytes and 64 `0xff` bytes | [x] |
| 7.58 | `crypto_sign_ed25519_pk_to_curve25519` + `crypto_sign_ed25519_sk_to_curve25519` + `crypto_box_beforenm` | full cross-protocol bridge: two ed25519 keypairs → converted curve25519 keys → `crypto_box_easy`/`_open_easy` round trip | [x] |
| 7.59 | `crypto_sign_bytes`, `_seedbytes`, `_publickeybytes`, `_secretkeybytes`, `_messagebytes_max`, `_primitive`, `_statebytes` and the seven `crypto_sign_ed25519_*` twins | → `64`, `32`, `32`, `64`, `2^64-65`, `"ed25519"`, `208` | [x] |
| 7.60 | `crypto_box_keypair`, `crypto_box_curve25519xsalsa20poly1305_keypair` | randomised; verify `pk == crypto_scalarmult_curve25519_base(sk)` | [x] |
| 7.61 | `crypto_box_seed_keypair`, `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | deterministic: `sk = SHA-512(seed)[0..31]` (**unclamped** in `sk`, clamped only inside `_base`), `pk = base(sk)`; seed = 32 zero bytes, 32 `0xff` bytes, and a fixed vector | [x] |
| 7.62 | `crypto_box_easy` / `crypto_box_open_easy` | round trip, `mlen` ∈ {0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024}; `c` buffer `mlen + 16`; MAC is at `c[0..15]`, body at `c[16…]` | [x] |
| 7.63 | `crypto_box_easy` / `_open_easy` | in-place: `c == m` is **not** possible for `_easy` (output is prefixed) but `m == c` for `_open_easy` with the shift, plus `c + 16 == m` for `_easy` (documented in-place pattern) | [x] |
| 7.64 | `crypto_box_detached` / `crypto_box_open_detached` | round trip with a separate 16-byte `mac` buffer; `mlen` ∈ {0, 1, 16, 63, 64, 65, 1024}; check `c ‖ mac` layout equals the `_easy` output reordered (`mac ‖ c`) | [x] |
| 7.65 | `crypto_box_beforenm` + `crypto_box_easy_afternm` / `crypto_box_open_easy_afternm` | precomputed key path; `k` is 32 bytes; must produce identical ciphertext to `crypto_box_easy` with the same `pk`/`sk`/`n`/`m` | [x] |
| 7.66 | `crypto_box_beforenm` + `crypto_box_detached_afternm` / `crypto_box_open_detached_afternm` | same, detached form; verify `beforenm(k, pkB, skA) == beforenm(k, pkA, skB)` (symmetry via HSalsa20 of the DH secret) | [x] |
| 7.67 | `crypto_box_beforenm` + `crypto_box_afternm` / `crypto_box_open_afternm` (NaCl padded) | `m` with `crypto_box_ZEROBYTES` = 32 leading zero bytes, `c` with `crypto_box_BOXZEROBYTES` = 16 leading zero bytes; `mlen` ∈ {32, 33, 48, 64, 1056} | [x] |
| 7.68 | `crypto_box` / `crypto_box_open`, `crypto_box_curve25519xsalsa20poly1305` / `_open` (NaCl padded) | full round trip in the padded convention; verify `c[0..15] == 0` on output and `m[0..31] == 0` after open | [x] |
| 7.69 | `crypto_box_seal` / `crypto_box_seal_open` | round trip, `mlen` ∈ {0, 1, 16, 32, 63, 64, 65, 1024}; `c` buffer `mlen + 48`; `c[0..31]` is a fresh ephemeral pk each call (two seals of the same message must differ) | [x] |
| 7.70 | `crypto_box_seal_open` | `clen == 48` exactly (empty sealed message) → `0`, nothing written to `m` | [x] |
| 7.71 | `crypto_box_seal_open` | anonymous-sender property: the recipient can open without knowing the sender; and `crypto_box_seal_open` with `pk` derived from `sk` via `crypto_scalarmult_base` (the two must be consistent or it fails) | [x] |
| 7.72 | `crypto_box_curve25519xchacha20poly1305_keypair` / `_seed_keypair` | identical key derivation to the xsalsa variant (both are `SHA-512(seed)[0..31]` then `_base`) — outputs must be byte-identical across the two primitives for the same seed | [x] |
| 7.73 | `crypto_box_curve25519xchacha20poly1305_beforenm` | must **differ** from `crypto_box_curve25519xsalsa20poly1305_beforenm` for the same `pk`/`sk` (HChaCha20 vs HSalsa20 of the same DH secret) | [x] |
| 7.74 | `crypto_box_curve25519xchacha20poly1305_easy` / `_open_easy` | round trip, `mlen` ∈ {0, 1, 15, 16, 17, 63, 64, 65, 127, 128, 129, 1024}; ciphertext must differ from the xsalsa variant | [x] |
| 7.75 | `crypto_box_curve25519xchacha20poly1305_detached` / `_open_detached` | round trip with a separate `mac`; `mlen` ∈ {0, 1, 64, 1024} | [x] |
| 7.76 | `crypto_box_curve25519xchacha20poly1305_beforenm` + `_easy_afternm` / `_open_easy_afternm` | precomputed path must match the non-`afternm` form | [x] |
| 7.77 | `crypto_box_curve25519xchacha20poly1305_beforenm` + `_detached_afternm` / `_open_detached_afternm` | precomputed detached path | [x] |
| 7.78 | `crypto_box_curve25519xchacha20poly1305_seal` / `_seal_open` | round trip, `mlen` ∈ {0, 1, 64, 1024}; `SEALBYTES` = 48; nonce = `BLAKE2b-24(epk ‖ pk)` — must differ from the xsalsa seal for the same inputs | [x] |
| 7.79 | xchacha **absent** APIs | confirm there is no `crypto_box_curve25519xchacha20poly1305()` / `_open()` / `_afternm()` / `_open_afternm()` / `_zerobytes()` / `_boxzerobytes()` — the NaCl padded convention exists only for xsalsa. A port must not invent them | [x] |
| 7.80 | `crypto_box_seedbytes`, `_publickeybytes`, `_secretkeybytes`, `_beforenmbytes`, `_noncebytes`, `_zerobytes`, `_boxzerobytes`, `_macbytes`, `_messagebytes_max`, `_sealbytes`, `_primitive` and the xsalsa/xchacha twins | → `32,32,32,32,24,32,16,16,2^64-17,48,"curve25519xsalsa20poly1305"`; xchacha has no `_zerobytes`/`_boxzerobytes` | [x] |
| 7.81 | `crypto_kx_keypair` | randomised; verify `pk == crypto_scalarmult_base(sk)` | [x] |
| 7.82 | `crypto_kx_seed_keypair` | deterministic: `sk = BLAKE2b-32(seed)` (no key, no salt), `pk = crypto_scalarmult_base(sk)`; seed = 32 zero bytes, 32 `0xff` bytes, fixed vector | [x] |
| 7.83 | `crypto_kx_client_session_keys` + `crypto_kx_server_session_keys` | full handshake with both `rx` and `tx` non-NULL: `client_rx == server_tx` and `client_tx == server_rx`; the shared hash is `BLAKE2b-64(q ‖ client_pk ‖ server_pk)` split as `keys[0..31]` / `keys[32..63]` | [x] |
| 7.84 | `crypto_kx_client_session_keys` | `rx == NULL`, `tx` non-NULL → `0`; the surviving buffer holds `keys[32..63]` (the tx key) because of the byte-interleaved aliased writes | [x] |
| 7.85 | `crypto_kx_client_session_keys` | `tx == NULL`, `rx` non-NULL → `0`; the surviving buffer **also** holds `keys[32..63]`, i.e. the *tx* key, not the rx key (aliasing footgun to replicate exactly) | [x] |
| 7.86 | `crypto_kx_server_session_keys` | `rx == NULL` and separately `tx == NULL` → `0`; the surviving buffer holds `keys[32..63]` = the server's **rx** key (loop order is reversed relative to the client) | [x] |
| 7.87 | `crypto_kx_client_session_keys` / `_server_session_keys` | `rx == tx` (caller deliberately aliases two non-NULL equal pointers) — same interleaving as 7.84–7.86 | [x] |
| 7.88 | `crypto_kx_client_session_keys` | client and server keys swapped (client calls the server function and vice versa) → session keys must **not** match, documenting the role asymmetry | [x] |
| 7.89 | `crypto_kx_publickeybytes`, `_secretkeybytes`, `_seedbytes`, `_sessionkeybytes`, `_primitive` | → `32, 32, 32, 32, "x25519blake2b"` | [x] |
| 7.90 | `crypto_kdf_derive_from_key`, `crypto_kdf_blake2b_derive_from_key` | `subkey_len = 16` (`BYTES_MIN`), `subkey_id = 0`, `ctx = "context1"` (8 bytes), fixed 32-byte key → deterministic vector; the generic alias must be byte-identical | [x] |
| 7.91 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len` = 16, 17, 31, 32, 33, 63, 64 (`BYTES_MAX`) with everything else fixed → 7 distinct subkeys; a shorter subkey must **not** be a prefix of a longer one (BLAKE2b `outlen` is in the parameter block) | [x] |
| 7.92 | `crypto_kdf_blake2b_derive_from_key` | `subkey_id` = 0, 1, 2, `0xffffffff`, `0x100000000`, `0x8000000000000000`, `0xffffffffffffffff` — stored `STORE64_LE` into `salt[0..7]` with `salt[8..15] = 0` | [x] |
| 7.93 | `crypto_kdf_blake2b_derive_from_key` | `ctx` = 8 zero bytes; `ctx` = `"12345678"`; `ctx` = 8 `0xff` bytes — zero-padded into the 16-byte BLAKE2b *personal* field | [x] |
| 7.94 | `crypto_kdf_blake2b_derive_from_key` | `key` = 32 zero bytes and 32 `0xff` bytes (`keylen` is always `crypto_kdf_blake2b_KEYBYTES` = 32) | [x] |
| 7.95 | `crypto_kdf_keygen` | 32 bytes of `randombytes_buf`; verify successive calls differ | [x] |
| 7.96 | `crypto_kdf_bytes_min`, `_bytes_max`, `_contextbytes`, `_keybytes`, `_primitive` and the four `crypto_kdf_blake2b_*` twins | → `16, 64, 8, 32, "blake2b"` | [x] |
| 7.97 | `crypto_kdf_hkdf_sha256_extract` | RFC 5869 test case 1: `salt = 000102030405060708090a0b0c` (13 B), `ikm = 0b×22` → `prk = 077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5` | [x] |
| 7.98 | `crypto_kdf_hkdf_sha256_expand` | RFC 5869 test 1 continued: `ctx = f0f1f2f3f4f5f6f7f8f9`, `out_len = 42` → `3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865` | [x] |
| 7.99 | `crypto_kdf_hkdf_sha256_extract` | `salt_len` = 0 (RFC 5869 test 3, empty salt), 1, 32, 64 (= HMAC block size), 65, 128, 129 — the `> blocksize` case forces the HMAC key-hashing path | [x] |
| 7.100 | `crypto_kdf_hkdf_sha256_extract` | `ikm_len` = 0, 1, 22, 32, 64, 80 (RFC 5869 test 3 has `ikm_len = 22`, test 2 has 80) | [x] |
| 7.101 | `crypto_kdf_hkdf_sha256_extract_init` / `_extract_update` / `_extract_final` | **0 updates** (empty ikm) → must equal `crypto_kdf_hkdf_sha256_extract(prk, salt, salt_len, NULL/ptr, 0)` | [x] |
| 7.102 | hkdf-sha256 streaming extract | **1 update**; then **many updates** splitting the same ikm as 1+1+…, 31+1+32, 32×N, 63+1 — all must produce the identical `prk` | [x] |
| 7.103 | `crypto_kdf_hkdf_sha256_extract_final` | state is `sodium_memzero`'d on return — a second `_extract_final` on the same state is a misuse; document that the state is single-shot | [x] |
| 7.104 | `crypto_kdf_hkdf_sha256_expand` | `out_len` = 0 (`BYTES_MIN`, legal, no writes), 1, 31, 32, 33, 63, 64, 65, 96, 8160 (`BYTES_MAX`) — exercises both the full-block loop and the `left = out_len & 31` tail | [x] |
| 7.105 | `crypto_kdf_hkdf_sha256_expand` | `ctx_len` = 0, 1, 8, 10, 64; and `ctx` = NULL with `ctx_len = 0` | [x] |
| 7.106 | `crypto_kdf_hkdf_sha256_expand` | counter progression: `out_len = 8160` exhausts the counter to `0xff`; check the last 32-byte block matches an independent HMAC computation with `counter = 255` | [x] |
| 7.107 | `crypto_kdf_hkdf_sha256_keygen` | 32 bytes random; then `_expand` with that prk | [x] |
| 7.108 | `crypto_kdf_hkdf_sha512_extract` / `_expand` | mirror of 7.97–7.106 with `KEYBYTES = 64`, `BYTES_MAX = 16320`, block size 128, `left = out_len & 63`; `out_len` ∈ {0, 1, 63, 64, 65, 127, 128, 129, 16320} | [x] |
| 7.109 | `crypto_kdf_hkdf_sha512_extract_init/_update/_final` | 0 / 1 / many updates; `salt_len` ∈ {0, 1, 64, 128, 129} (128 = HMAC-SHA512 block size) | [x] |
| 7.110 | `crypto_kdf_hkdf_sha512_keygen` | 64 bytes random | [x] |
| 7.111 | `crypto_kdf_hkdf_sha256_*` vs `crypto_kdf_hkdf_sha512_*` | same salt/ikm/ctx through both → outputs must differ; documents that the two are separate namespaces with different `KEYBYTES` | [x] |
| 7.112 | `crypto_kdf_hkdf_sha256_keybytes`, `_bytes_min`, `_bytes_max`, `_statebytes` and the sha512 twins | → `32, 0, 8160, sizeof(state)` and `64, 0, 16320, sizeof(state)` | [x] |
| 7.113 | `crypto_kem_mlkem768_seed_keypair` | fixed 64-byte seed (`d = seed[0..31]`, `z = seed[32..63]`) → deterministic `pk` (1184 B) and `sk` (2400 B); verify `sk[1152..2335] == pk`, `sk[2336..2367] == SHA3-256(pk)`, `sk[2368..2399] == seed[32..63]`. Use the FIPS 203 / ML-KEM-768 KAT vectors | [x] |
| 7.114 | `crypto_kem_mlkem768_keypair` | randomised; verify the same structural invariants as 7.113 | [x] |
| 7.115 | `crypto_kem_mlkem768_enc_deterministic` | fixed `pk` from 7.113 plus a fixed 32-byte `seed` (= the message `m`) → deterministic `ct` (1088 B) and `ss` (32 B); ML-KEM-768 KAT | [x] |
| 7.116 | `crypto_kem_mlkem768_enc` + `_dec` | randomised round trip × 100: `ss_enc == ss_dec`, `0` from both | [x] |
| 7.117 | `crypto_kem_mlkem768_enc_deterministic` + `_dec` | deterministic round trip; then `_dec` with a single bit flipped anywhere in `ct` → still `0` but `ss` differs (implicit rejection, `SHAKE256(z ‖ ct)`) — the derived value must be reproducible | [x] |
| 7.118 | `crypto_kem_mlkem768_enc*` | `pk` = 1184 zero bytes (canonical! all coefficients 0) → **succeeds**; `pk` with `publicseed` (`pk[1152..1183]`) varied → different `ct` | [x] |
| 7.119 | `crypto_kem_mlkem768_*bytes` accessors | → `1184, 2400, 1088, 32, 64` | [x] |
| 7.120 | `crypto_kem_xwing_seed_keypair` | fixed 32-byte seed → deterministic `pk` (1216 B = 1184 ML-KEM ‖ 32 X25519) and `sk` (exactly the 32-byte seed); verify `expand_decaps_key` layout: `SHAKE256(seed, 96)` → `[0..63]` = ML-KEM seed, `[64..95]` = X25519 scalar; `pk[1184..1215] == crypto_scalarmult_curve25519_base(sk_x25519)`. X-Wing draft test vectors | [x] |
| 7.121 | `crypto_kem_xwing_keypair` | randomised; `sk` is 32 bytes; re-deriving via `_seed_keypair(sk)` must reproduce `pk` exactly | [x] |
| 7.122 | `crypto_kem_xwing_enc_deterministic` | fixed `pk` + fixed **64-byte** seed (`seed[0..31]` = ML-KEM message, `seed[32..63]` = ephemeral X25519 scalar) → deterministic `ct` (1120 B = 1088 ML-KEM ‖ 32 X25519) and `ss` (32 B). `ss = SHA3-256(ss_mlkem ‖ ss_x25519 ‖ ct_x25519 ‖ pk_x25519 ‖ 5c2e2f2f5e5c)` | [x] |
| 7.123 | `crypto_kem_xwing_enc` + `_dec` | randomised round trip × 100: `ss_enc == ss_dec`, `0` from both | [x] |
| 7.124 | `crypto_kem_xwing_enc_deterministic` + `_dec` | deterministic round trip against the X-Wing vectors; then `_dec` with a bit flipped in `ct[0..1087]` (ML-KEM half) → `0` with a different `ss` | [x] |
| 7.125 | `crypto_kem_xwing_dec` | `sk` = 32 zero bytes (legal seed) and 32 `0xff` bytes; full round trip for each | [x] |
| 7.126 | `crypto_kem_xwing_*bytes` accessors | → `1216, 32, 1120, 32, 32` (note `SECRETKEYBYTES == SEEDBYTES == 32`) | [x] |
| 7.127 | `crypto_kem_seed_keypair`, `crypto_kem_keypair`, `crypto_kem_enc`, `crypto_kem_dec` (generic dispatch) | must be byte-identical to the `crypto_kem_xwing_*` equivalents for the same inputs; `crypto_kem_primitive() == "xwing"`; `crypto_kem_*bytes()` must equal the xwing values | [x] |
| 7.128 | xwing vs mlkem768 | same-length comparison: `crypto_kem_xwing_CIPHERTEXTBYTES` (1120) = mlkem `CIPHERTEXTBYTES` (1088) + 32; `PUBLICKEYBYTES` 1216 = 1184 + 32 — verify the concatenation offsets used by `crypto_kem_xwing_enc`/`_dec` (`ct + 1088`, `pk + 1184`) | [x] |
| 7.129 | cross-area consistency | `crypto_kx` shared secret vs `crypto_box_beforenm`: both start from `crypto_scalarmult`/`crypto_scalarmult_curve25519` on the same keypairs but apply BLAKE2b vs HSalsa20 — verify they differ and that each is stable | [x] |
| 7.130 | build-configuration invariant | no `HAVE_*` macro is defined by the CMake build, therefore: sandy2x is never selected (`scalarmult_curve25519.c:54-58` removed), `fe25519_sub_lazy` is `fe25519_sub` (`x25519_ref10.c:88-91`), `ED25519_COMPAT` is off so `open.c:34-42` (strict `sc25519_is_canonical` + `ge25519_is_canonical`) is live and `open.c:31-33` is dead, `ED25519_NONDETERMINISTIC` is off so signing is fully deterministic (`sign.c:66`, not `:64`), and `cmov`'s `HAVE_INLINE_ASM` barrier (`kem_mlkem768_ref.c:696-698`) is absent. Every row above must be evaluated under exactly this configuration | [x] |
