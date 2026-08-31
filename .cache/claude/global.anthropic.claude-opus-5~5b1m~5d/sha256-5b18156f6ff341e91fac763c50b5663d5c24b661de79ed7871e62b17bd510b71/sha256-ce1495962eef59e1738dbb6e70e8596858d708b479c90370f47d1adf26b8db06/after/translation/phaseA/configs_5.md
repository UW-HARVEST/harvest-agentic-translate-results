## Area 5 — crypto_stream

### Axes extracted from the source

- **Primitive** (7): `salsa20` (8-byte nonce), `salsa2012` (8-byte nonce), `salsa208` (8-byte nonce, deprecated), `xsalsa20` (24-byte nonce), `chacha20` "original" (8-byte nonce, 64-bit `ic`), `chacha20_ietf` (12-byte nonce, 32-bit `ic`), `xchacha20` (24-byte nonce, 64-bit `ic`).
- **Form** (3): keystream generator `crypto_stream_*(c, clen, n, k)`; XOR form `crypto_stream_*_xor(c, m, mlen, n, k)`; initial-counter form `crypto_stream_*_xor_ic(c, m, mlen, n, ic, k)`. Note `salsa2012` and `salsa208` have **no** `_xor_ic`. `chacha20` additionally exposes the internal-but-exported `_ietf_ext` and `_ietf_ext_xor_ic` (declared in `include/sodium/private/chacha20_ietf_ext.h`).
- **Initial counter `ic`**: `0`; `1`; small (`2, 3, 7`); values that roll the block counter over mid-message.
- **Message/keystream length** — the sweep **L** = `{0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 511, 512}` (16 values). This crosses the 64-byte block boundary in both directions and exercises both sides of the ref implementations' bulk/partial split: `while (clen >= 64) { ... }` then `if (clen) { ... }` in `salsa20_ref.c` / `stream_salsa2012_ref.c` / `stream_salsa208_ref.c`, and the `if (bytes < 64) { tmp path }` / `if (bytes <= 64) { finish }` structure in `chacha20_ref.c:112-220`.
- **Key/nonce shape**: all-`0x00`, all-`0xff`, RFC/DJB test-vector values, pseudorandom.
- **Buffer aliasing**: `c != m` (out-of-place) vs `c == m` (in-place). `chacha20_ref.c` `stream_ref` relies on in-place operation internally (`memset(c,0,clen); chacha20_encrypt_bytes(&ctx, c, c, clen)`).
- **`*_keygen`** (7 + generic + `chacha20_ietf_keygen`).
- **Accessors**: `*_keybytes`, `*_noncebytes`, `*_messagebytes_max` for all primitives + `crypto_stream_primitive`.
- **Implementation selection**: with the CMake build defining no `HAVE_*` macros, `_crypto_stream_salsa20_pick_best_implementation` and `_crypto_stream_chacha20_pick_best_implementation` always land on the `*_ref_implementation` — so the ref path is the *only* configuration reachable and there is no dispatch axis to sweep.

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 5.1 | `crypto_stream_keybytes`, `crypto_stream_noncebytes`, `crypto_stream_messagebytes_max`, `crypto_stream_primitive` | no inputs; assert exact values `32`, `24`, `SODIUM_SIZE_MAX` (= `0xFFFFFFFFFFFFFFFF` on LP64), and `"xsalsa20"` | [x] |
| 5.2 | `crypto_stream` (generic wrapper → `crypto_stream_xsalsa20`) | `k` = all-`0x00`, `n` (24 B) = all-`0x00`; `clen` over the full sweep **L** | [x] |
| 5.3 | `crypto_stream` | `k` = all-`0xff`, `n` = all-`0xff`; `clen` over **L** | [x] |
| 5.4 | `crypto_stream` | pseudorandom `k`, `n`; `clen` over **L** | [x] |
| 5.5 | `crypto_stream_xor` (generic → `crypto_stream_xsalsa20_xor`) | out-of-place (`c != m`), pseudorandom `m`; `mlen` over **L** | [x] |
| 5.6 | `crypto_stream_xor` | in-place (`c == m`), pseudorandom `m`; `mlen` over **L** | [x] |
| 5.7 | `crypto_stream_xor` | round-trip: XOR twice with the same `(n, k)` must restore `m`; `mlen` over **L** | [x] |
| 5.8 | `crypto_stream_xor` vs `crypto_stream` | equivalence: `m` = all-zero → `crypto_stream_xor` output must byte-equal `crypto_stream` output for the same `(n, k)`; `mlen` over **L** | [x] |
| 5.9 | `crypto_stream` / `crypto_stream_xor` vs `crypto_stream_xsalsa20` / `crypto_stream_xsalsa20_xor` | equivalence: generic wrapper output must byte-equal the xsalsa20-specific entry point for identical arguments; `mlen` over **L** | [x] |
| 5.10 | `crypto_stream_keygen` | 32-byte output buffer; check length written, non-constant across calls, surrounding bytes untouched | [x] |
| 5.11 | `crypto_stream_salsa20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.12 | `crypto_stream_salsa20` | `k`/`n` = all-`0x00`; `clen` over **L** (bulk loop `clen >= 64` writes directly into `c`, tail via `block[64]`) | [x] |
| 5.13 | `crypto_stream_salsa20` | `k`/`n` = all-`0xff`, and `n` = `0x0102030405060708`; `clen` over **L** | [x] |
| 5.14 | `crypto_stream_salsa20` | DJB/libsodium salsa20 test-vector `k`/`n`; `clen ∈ {64, 512}` | [x] |
| 5.15 | `crypto_stream_salsa20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.16 | `crypto_stream_salsa20_xor` | in-place (`c == m`); `mlen` over **L** | [x] |
| 5.17 | `crypto_stream_salsa20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_salsa20_xor` for the same inputs; `mlen` over **L** | [x] |
| 5.18 | `crypto_stream_salsa20_xor_ic` | `ic = 1`; must equal the tail of a `crypto_stream_salsa20_xor` run over a 64-byte-prefixed message; `mlen` over **L** | [x] |
| 5.19 | `crypto_stream_salsa20_xor_ic` | `ic ∈ {2, 3, 7}` (small); `mlen` over **L** | [x] |
| 5.20 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFF` (32-bit boundary — carry propagates from `in[11]` into `in[12]`); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.21 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFF` (64-bit block counter rolls `2^64-1 → 0` mid-message; carry out of `in[15]` is dropped silently); `mlen ∈ {65, 128, 129, 192}` | [x] |
| 5.22 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFE` (two wraps across a 3-block message); `mlen ∈ {129, 192, 193}` | [x] |
| 5.23 | `crypto_stream_salsa20_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFFFFFFFFFF}`; early `if (!mlen) return 0;` — output buffer must be left untouched | [x] |
| 5.24 | `crypto_stream_salsa20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.25 | `crypto_stream_salsa2012_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.26 | `crypto_stream_salsa2012` | `k`/`n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.27 | `crypto_stream_salsa2012_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.28 | `crypto_stream_salsa2012_xor` | in-place (`c == m`); `mlen` over **L** | [x] |
| 5.29 | `crypto_stream_salsa2012_xor` | round-trip (XOR twice) restores `m`; and `m` = all-zero must equal `crypto_stream_salsa2012`; `mlen` over **L** | [x] |
| 5.30 | `crypto_stream_salsa2012` / `_xor` | no `_xor_ic` entry point exists → counter always starts at `in[8..15] = 0`; verify multi-block counter increment via `clen = 512` (8 blocks) and that output differs from salsa20 for the same `(n, k)` | [x] |
| 5.31 | `crypto_stream_salsa2012_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.32 | `crypto_stream_salsa208_keybytes`, `_noncebytes`, `_messagebytes_max` (all `__attribute__((deprecated))`) | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.33 | `crypto_stream_salsa208` (deprecated) | `k`/`n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.34 | `crypto_stream_salsa208_xor` (deprecated) | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.35 | `crypto_stream_salsa208_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_salsa208`; `mlen` over **L** | [x] |
| 5.36 | `crypto_stream_salsa208` / `_xor` | no `_xor_ic`; counter starts at 0; `clen = 512` covers 8 counter increments; output must differ from salsa2012 and salsa20 for the same `(n, k)` | [x] |
| 5.37 | `crypto_stream_salsa208_keygen` (deprecated) | 32-byte output; length + non-constancy | [x] |
| 5.38 | `crypto_stream_xsalsa20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`24`**, `SODIUM_SIZE_MAX` | [x] |
| 5.39 | `crypto_stream_xsalsa20` | 24-byte `n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.40 | `crypto_stream_xsalsa20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.41 | `crypto_stream_xsalsa20_xor` | in-place (`c == m`); round-trip; `mlen` over **L** | [x] |
| 5.42 | `crypto_stream_xsalsa20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_xsalsa20_xor` (which is defined as `_xor_ic(..., 0ULL, ...)`); `mlen` over **L** | [x] |
| 5.43 | `crypto_stream_xsalsa20_xor_ic` | `ic = 1` and small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.44 | `crypto_stream_xsalsa20_xor_ic` | `ic = 0xFFFFFFFF` (32-bit boundary) and `ic = 0xFFFFFFFFFFFFFFFF` (64-bit rollover mid-message); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.45 | `crypto_stream_xsalsa20` / `_xor_ic` vs `crypto_stream_salsa20*` | equivalence: `crypto_stream_xsalsa20(c, clen, n, k)` must equal `crypto_stream_salsa20(c, clen, n + 16, hsalsa20(n, k))`; same for `_xor_ic`; `clen ∈ {0, 64, 65, 512}` | [x] |
| 5.46 | `crypto_stream_xsalsa20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.47 | `crypto_stream_chacha20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`8`**, `SODIUM_SIZE_MAX` | [x] |
| 5.48 | `crypto_stream_chacha20` (original, 8-byte nonce) | `k`/`n` = all-`0x00`; `clen` over **L**. Note the impl does `memset(c, 0, clen)` then encrypts in place. | [x] |
| 5.49 | `crypto_stream_chacha20` | `k`/`n` = all-`0xff`, and DJB chacha20 test-vector `k`/`n`; `clen` over **L** | [x] |
| 5.50 | `crypto_stream_chacha20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** (exercises `bytes < 64` `tmp[64]` zero-pad path for non-multiples, and the `bytes == 64` direct-exit path) | [x] |
| 5.51 | `crypto_stream_chacha20_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_chacha20`; `mlen` over **L** | [x] |
| 5.52 | `crypto_stream_chacha20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_chacha20_xor`; `mlen` over **L** | [x] |
| 5.53 | `crypto_stream_chacha20_xor_ic` | `ic = 1`; must equal the second-block-onward keystream; `mlen` over **L** | [x] |
| 5.54 | `crypto_stream_chacha20_xor_ic` | small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.55 | `crypto_stream_chacha20_xor_ic` | `ic = 0xFFFFFFFF` — 32-bit counter word `j12` wraps to 0 and carries into `j13` (which is the counter **high** word for the original nonce layout, so this is a correct 64-bit increment); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.56 | `crypto_stream_chacha20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFF` — full 64-bit counter rolls over to 0 mid-message, silently (no check); `mlen ∈ {65, 128, 129, 192}` | [x] |
| 5.57 | `crypto_stream_chacha20_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFFFFFFFFFF}`; early return, output untouched | [x] |
| 5.58 | `crypto_stream_chacha20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.59 | `crypto_stream_chacha20_ietf_keybytes`, `_ietf_noncebytes`, `_ietf_messagebytes_max` | no inputs; assert `32`, **`12`**, **`274877906944`** (`= 64 * 2^32 = 2^38`) — distinct from the non-ietf `messagebytes_max` | [x] |
| 5.60 | `crypto_stream_chacha20_ietf` (12-byte nonce) | `k`/`n` = all-`0x00`, all-`0xff`, RFC 7539 §2.4.2 vector; `clen` over **L** | [x] |
| 5.61 | `crypto_stream_chacha20_ietf_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.62 | `crypto_stream_chacha20_ietf_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_chacha20_ietf`; `mlen` over **L** | [x] |
| 5.63 | `crypto_stream_chacha20_ietf_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_chacha20_ietf_xor`; `mlen` over **L** | [x] |
| 5.64 | `crypto_stream_chacha20_ietf_xor_ic` | `ic = 1` (RFC 7539 §2.4.2 uses counter 1); `mlen` over **L** | [x] |
| 5.65 | `crypto_stream_chacha20_ietf_xor_ic` | small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.66 | `crypto_stream_chacha20_ietf_xor_ic` | **exact accepted boundary** `ic = 4294967296 - ceil(mlen/64)`: `(mlen=1, ic=0xFFFFFFFF)`, `(mlen=63, ic=0xFFFFFFFF)`, `(mlen=64, ic=0xFFFFFFFF)`, `(mlen=65, ic=0xFFFFFFFE)`, `(mlen=128, ic=0xFFFFFFFE)`, `(mlen=129, ic=0xFFFFFFFD)`, `(mlen=512, ic=0xFFFFFFF8)` — all must succeed with `ic + ceil(mlen/64) == 2^32` exactly, i.e. the counter reaches `0xFFFFFFFF` on the final block and never wraps | [x] |
| 5.67 | `crypto_stream_chacha20_ietf_xor_ic` | **one past the boundary** `ic = 4294967297 - ceil(mlen/64)`: `(mlen=65, ic=0xFFFFFFFF)`, `(mlen=128, ic=0xFFFFFFFF)`, `(mlen=129, ic=0xFFFFFFFE)`, `(mlen=512, ic=0xFFFFFFF9)` — each must hit `sodium_misuse()` | [x] |
| 5.68 | `crypto_stream_chacha20_ietf_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFF}` — guard limit is `2^32`, never fires; then early `return 0`, output untouched | [x] |
| 5.69 | `crypto_stream_chacha20_ietf_ext` (private-but-exported, `private/chacha20_ietf_ext.h`) | `clen` over **L**; must byte-equal `crypto_stream_chacha20_ietf` for all `clen <= 2^38` | [x] |
| 5.70 | `crypto_stream_chacha20_ietf_ext_xor_ic` | `ic ∈ {0, 1, 2, 3, 7}`; `mlen` over **L**; must byte-equal `crypto_stream_chacha20_ietf_xor_ic` wherever the latter's guard permits | [x] |
| 5.71 | `crypto_stream_chacha20_ietf_ext_xor_ic` | **32-bit counter rollover into the IV** — `ic = 0xFFFFFFFF` with `mlen ∈ {65, 128, 129, 192}`: `j12` wraps `0xFFFFFFFF → 0` and the carry increments `j13`, which under `chacha_ietf_ivsetup` is **nonce word 0**. No guard on this entry point (unlike 5.67). Verify the resulting keystream equals nonce-incremented, counter-0 output. | [x] |
| 5.72 | `crypto_stream_chacha20_ietf_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.73 | `crypto_stream_chacha20_ietf*` vs `crypto_stream_chacha20*` | cross-variant separation: for the same key, the ietf (12-byte nonce, `input[12]`=counter, `input[13..15]`=nonce) and original (8-byte nonce, `input[12..13]`=counter, `input[14..15]`=nonce) layouts must produce different keystreams; `clen ∈ {64, 128}` | [x] |
| 5.74 | `crypto_stream_chacha20_IETF_KEYBYTES` / `_IETF_NONCEBYTES` / `_IETF_MESSAGEBYTES_MAX` legacy aliases (header only) | assert each alias equals its lowercase counterpart | [x] |
| 5.75 | `crypto_stream_xchacha20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`24`**, `SODIUM_SIZE_MAX` | [x] |
| 5.76 | `crypto_stream_xchacha20` | 24-byte `n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.77 | `crypto_stream_xchacha20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.78 | `crypto_stream_xchacha20_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_xchacha20`; `mlen` over **L** | [x] |
| 5.79 | `crypto_stream_xchacha20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_xchacha20_xor` (defined as `_xor_ic(..., 0U, ...)`); `mlen` over **L** | [x] |
| 5.80 | `crypto_stream_xchacha20_xor_ic` | `ic = 1` and small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.81 | `crypto_stream_xchacha20_xor_ic` | `ic` is `uint64_t` and forwards to the **original** chacha20 path, so the IETF 32-bit guard does **not** apply: `ic = 0xFFFFFFFF` (32→64 carry) and `ic = 0xFFFFFFFFFFFFFFFF` (silent 64-bit rollover); `mlen ∈ {64, 65, 128, 129, 192}` — all must succeed with no misuse | [x] |
| 5.82 | `crypto_stream_xchacha20*` vs `crypto_stream_chacha20*` | equivalence: `crypto_stream_xchacha20(c, clen, n, k)` must equal `crypto_stream_chacha20(c, clen, n + 16, hchacha20(n, k))`; same for `_xor_ic`; `clen ∈ {0, 64, 65, 512}` | [x] |
| 5.83 | `crypto_stream_xchacha20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.84 | all `crypto_stream_*` keystream forms | `clen = 0` for every one of the 7 primitives + generic: `if (!clen) return 0;` — output buffer must be entirely untouched (verify with a poisoned buffer) | [x] |
| 5.85 | all `crypto_stream_*_xor{,_ic}` forms | `mlen = 0` for every primitive + generic: `if (!mlen) return 0;` — output buffer untouched | [x] |
| 5.86 | every `int`-returning entry point in area 5 | return value must be `0` for all of the above; there is no `-1` path (only `sodium_misuse()` → `abort()`) | [x] |
| 5.87 | length-sweep exactness (all primitives, all forms) | for each `mlen ∈ L`, assert byte `c[mlen]` and beyond are untouched — pins the bulk/partial split (`clen >= 64` loop vs `if (clen)` tail; `bytes < 64` `tmp[64]` path vs `bytes <= 64` exit) against over-writes | [x] |
| 5.88 | length-sweep prefix consistency (all primitives, all forms) | output for length `n1` must be a prefix of output for length `n2 > n1` with identical `(n, k, ic)`; sweep all adjacent pairs in **L** | [x] |
| 5.89 | `_crypto_stream_salsa20_pick_best_implementation`, `_crypto_stream_chacha20_pick_best_implementation` | no `HAVE_*` macros → both unconditionally select `*_ref_implementation` and `return 0`; calling either before/after any of the above must not change any output | [x] |
