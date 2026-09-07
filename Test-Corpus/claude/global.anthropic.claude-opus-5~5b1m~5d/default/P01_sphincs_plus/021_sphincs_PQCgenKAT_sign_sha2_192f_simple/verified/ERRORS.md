# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically from the C source. Every rejection path was located with:

```bash
grep -rn 'return -1\|return RNG_\|assert\|abort\|== NULL\|!= NULL\|memcmp\|#error' \
  c_src/app/src/*.c c_src/lib/*/src/*.c
```

plus a read of every `if` guard in the returned hits. The complete set of
*runtime* rejection sites in the whole library is:

| C site | statement |
|---|---|
| `app/src/rng.c:32-33` | `if (maxlen >= 0x100000000) return RNG_BAD_MAXLEN;` |
| `app/src/rng.c:66-67` | `if (x == NULL) return RNG_BAD_OUTBUF;` |
| `app/src/rng.c:68-69` | `if (xlen >= ctx->length_remaining) return RNG_BAD_REQ_LEN;` |
| `app/src/rng.c:106-110` | `handleErrors()` → `abort()` (only on an OpenSSL EVP failure; unreachable for well-formed 32-byte keys) |
| `app/src/rng.c:205` | `if (provided_data != NULL)` — behavioural branch, not an error return |
| `app/src/sign.c:179-181` | `crypto_sign_verify`: `if (siglen != SPX_BYTES) return -1;` |
| `app/src/sign.c:235-237` | `crypto_sign_verify`: `if (memcmp(root, pub_root, SPX_N)) return -1;` |
| `app/src/sign.c:271-276` | `crypto_sign_open`: `if (smlen < SPX_BYTES) { memset(m,0,smlen); *mlen = 0; return -1; }` |
| `app/src/sign.c:279-284` | `crypto_sign_open`: verify failed → `memset(m,0,smlen); *mlen = 0; return -1;` |

There are **no** `assert()` calls, no `RETURN_ERROR` macro, and no `return NULL`
anywhere in the library. `#error` directives exist only in the params/backend
headers (`SPX_WOTS_W` not in {16,256}; `SPX_D` not dividing `SPX_FULL_HEIGHT`;
`SPX_N > 32` with BLAKE-256/SHA-256; `SPX_WOTS_LEN2` not precomputed) — those are
compile-time, cannot be reached at runtime, and all 48 shipped configurations
satisfy them, so they get no runtime row.

Return-code constants (`app/include/rng.h`):
`RNG_SUCCESS = 0`, `RNG_BAD_MAXLEN = -1`, `RNG_BAD_OUTBUF = -2`,
`RNG_BAD_REQ_LEN = -3`.

## Error-surface table

Every row has a differential test in `tests/errors.rs`
(`err_<n>_...`). "expected C result" is the value the C `.so` actually
returns, established by calling it — not inferred from docs.

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `seedexpander_init` | `maxlen == 0x100000000` (first value `>= 2^32`) | returns `-1` (`RNG_BAD_MAXLEN`); `*ctx` left **untouched** | [x] |
| 2 | `seedexpander_init` | `maxlen == 0xFFFFFFFFFFFFFFFF` (far above `2^32`) | returns `-1`; `*ctx` untouched | [x] |
| 3 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted, one below the range check) | returns `0` (`RNG_SUCCESS`); `ctx->ctr[8..12] = FF FF FF FF`, `buffer_pos = 16` | [x] |
| 4 | `seedexpander_init` | `maxlen == 0` (zero length) | returns `0`; `length_remaining = 0`, `ctr[8..12] = 00 00 00 00` | [x] |
| 5 | `seedexpander` | `x == NULL` (null output pointer), `xlen` arbitrary | returns `-2` (`RNG_BAD_OUTBUF`); `ctx` untouched (checked *before* `length_remaining`) | [x] |
| 6 | `seedexpander` | `x == NULL` **and** `xlen >= ctx->length_remaining` — both faults at once; the null check is first | returns `-2`, **not** `-3` | [x] |
| 7 | `seedexpander` | `xlen == ctx->length_remaining` (equal, so the `>=` fires) | returns `-3` (`RNG_BAD_REQ_LEN`); `ctx->length_remaining` **not** decremented | [x] |
| 8 | `seedexpander` | `xlen > ctx->length_remaining` | returns `-3`; `ctx` untouched | [x] |
| 9 | `seedexpander` | `xlen == ctx->length_remaining - 1` (largest accepted) | returns `0`; output + `ctx` state must match byte-for-byte | [x] |
| 10 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | returns `0`; `while(xlen>0)` never runs, so `ctx` unchanged except `length_remaining -= 0` | [x] |
| 11 | `seedexpander` | `xlen == 0` with `length_remaining == 0` → `0 >= 0` | returns `-3` | [x] |
| 12 | `seedexpander` | called on a ctx whose `length_remaining` was exhausted by an earlier successful call | returns `-3` | [x] |
| 13 | `crypto_sign_verify` | `siglen == SPX_BYTES - 1` (one below valid) | returns `-1` | [x] |
| 14 | `crypto_sign_verify` | `siglen == SPX_BYTES + 1` (one above valid) | returns `-1` | [x] |
| 15 | `crypto_sign_verify` | `siglen == 0` | returns `-1` | [x] |
| 16 | `crypto_sign_verify` | `siglen == SIZE_MAX` (oversized length) | returns `-1` | [x] |
| 17 | `crypto_sign_verify` | correct `siglen`, but one bit flipped in the `R` prefix (`sig[0..N]`) → wrong digest → root mismatch | returns `-1` | [x] |
| 18 | `crypto_sign_verify` | correct `siglen`, one bit flipped inside the FORS part of the signature | returns `-1` | [x] |
| 19 | `crypto_sign_verify` | correct `siglen`, one bit flipped inside a WOTS part | returns `-1` | [x] |
| 20 | `crypto_sign_verify` | correct `siglen`, one bit flipped inside a Merkle auth-path part | returns `-1` | [x] |
| 21 | `crypto_sign_verify` | valid signature verified against a **different** public key | returns `-1` | [x] |
| 22 | `crypto_sign_verify` | valid signature, `mlen` changed by +/-1 (message truncated or extended) | returns `-1` (the length itself feeds the digest) | [x] |
| 23 | `crypto_sign_verify` | valid signature, one message byte flipped, at every offset | **position-dependent.** Verified against the C: a flip in the first bytes gives `-1`, but flips at many later offsets are *accepted* (`0`). Cause: `blake256_update`'s guard `if (left && (((datalen >> 3) & 0x3F) >= fill))` masks the byte count with `0x3F`, so a whole-multiple-of-64 update never merges with the buffered `R || PK`, and the buffered prefix is what `blake*_final` pads -- large parts of the message never change the digest. This is a real weakness of *this* C code; the row asserts C == Rust for every offset (plus `-1` for offset 0), **not** that a flip is always rejected. | [x] |
| 24 | `crypto_sign_verify` | valid signature over `mlen == 0`, verified with `mlen == 0` | returns `0` (empty message is legal) | [x] |
| 25 | `crypto_sign_verify` | all-zero signature buffer of the correct `siglen` | returns `-1` | [x] |
| 26 | `crypto_sign_open` | `smlen == SPX_BYTES - 1` (one below the minimum) | returns `-1`, writes `*mlen = 0`, and zeroes exactly `smlen` bytes of `m` | [x] |
| 27 | `crypto_sign_open` | `smlen == 0` | returns `-1`, `*mlen = 0`, zeroes 0 bytes of `m` | [x] |
| 28 | `crypto_sign_open` | `smlen == SPX_BYTES` exactly (boundary; zero-length message) | returns `0`, `*mlen = 0` | [x] |
| 29 | `crypto_sign_open` | `smlen >= SPX_BYTES` but the signature part is corrupted → inner `crypto_sign_verify` fails | returns `-1`, `*mlen = 0`, and zeroes `smlen` bytes of `m` (note: **`smlen`**, not `smlen - SPX_BYTES`) | [x] |
| 30 | `crypto_sign_open` | valid `sm`, but verified under a different `pk` | returns `-1`, `*mlen = 0`, `m[0..smlen]` zeroed | [x] |
| 31 | `crypto_sign_open` | `smlen` one *greater* / one *smaller* than the signed length | one *smaller* -> `-1`. One *greater*: the C's answer depends on the backend for the same reason as row 23 (for blake/128f it *accepts*). The row asserts C == Rust for both, and `-1` for the one-byte-short case. | [x] |
| 32 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` — takes the `if (provided_data != NULL)` false branch, so `temp` is **not** XORed in | `Key`/`V` updated from the raw AES output only | [x] |
| 33 | `AES256_CTR_DRBG_Update` | `provided_data != NULL` (48 bytes) | `Key`/`V` updated from `temp ^ provided_data` | [x] |
| 34 | `randombytes_init` | `personalization_string == NULL` | `seed_material = entropy_input` unmodified; DRBG seeded from it; `reseed_counter = 1` | [x] |
| 35 | `randombytes_init` | `personalization_string != NULL` (48 bytes) | `seed_material = entropy_input ^ ps`; `reseed_counter = 1` | [x] |
| 36 | `randombytes` | `xlen == 0` | returns `0` (`RNG_SUCCESS`); the `while` body never runs but the trailing `AES256_CTR_DRBG_Update(NULL,..)` + `reseed_counter++` still execute, so the DRBG state **does** advance | [x] |
| 37 | `randombytes` | `xlen == 16` (exactly one AES block: `xlen > 15` is true, so the 16-byte `memcpy` path) | returns `0`; state advanced once | [x] |
| 38 | `randombytes` | `xlen == 15` (`xlen > 15` false → partial-block `memcpy` path) | returns `0`; only 15 bytes written, 16th byte of `x` untouched | [x] |
| 39 | `randombytes` | `xlen == 17` (one full block + a 1-byte tail) | returns `0` | [x] |
| 40 | `randombytes` | called after `V` has been driven to all-`0xFF` so the increment loop wraps every byte to `0x00` | returns `0`; identical carry-propagation output | [x] |
| 41 | `randombytes` / `randombytes_init` | never calling `randombytes_init` first — the zero-initialised `DRBG_ctx` global is used | returns `0`; output is AES-256 under the all-zero key, identical in both | [x] |
| 42 | `SPX_set_type` | `type` value with no `SPX_ADDR_TYPE_*` variant, e.g. `7`, `8`, `255`, `256`, `0xFFFFFFFF` (`uint32_t` / C enums accept any int) | no validation at all: `((unsigned char *)addr)[SPX_OFFSET_TYPE] = (unsigned char)type` — the value is **truncated to 8 bits** and written to that one byte; every other byte of `addr` is left alone | [x] |
| 43 | `set_layer_addr` | `layer > 255` (e.g. `0x1FF`, `0xFFFFFFFF`) | truncated to one byte at `SPX_OFFSET_LAYER`; no rejection | [x] |
| 44 | `set_tree_height` / `set_chain_addr` | value `> 255` | truncated to one byte; no rejection | [x] |
| 45 | `bytes_to_ull` | `inlen == 0` | returns `0` | [x] |
| 46 | `bytes_to_ull` | `inlen == 8` (full width) and `inlen > 8` (`9`, `16` — shifts past the width of `unsigned long long`) | no bounds check; the C shift-accumulate wraps, and Rust must wrap identically | [x] |
| 47 | `ull_to_bytes` | `outlen == 0` | writes nothing | [x] |
| 48 | `ull_to_bytes` | `outlen > 8` (e.g. `16`) — writes more bytes than the source has | high bytes come out `0` after the `in >>= 8` drains; no rejection | [x] |
| 49 | `compute_root` | `tree_height == 0` (empty auth path) | root = leaf after the single final `thash`; no rejection | [x] |
| 50 | `compute_root` | `leaf_idx` larger than `2^tree_height` (out-of-range index) | no check; the `leaf_idx & 1` / `leaf_idx >>= 1` loop simply consumes the extra high bits | [x] |
| 51 | `treehash` | `tree_height == 0` | single leaf, empty auth path; no rejection | [x] |
| 52 | `thash` | `inblocks == 0` | no rejection; hashes only the (pub_seed ‖ addr) prefix | [x] |
| 53 | `blake256_update` / `blake512_update` | `inlen == 0` (bit length 0) | no-op on the state; no rejection | [x] |
| 54 | `blake256_final` | called twice on the same state (`nullt` already set) | second call takes the `S->nullt` branch; both must agree | [x] |
| 55 | `sha256_inc_blocks` / `sha512_inc_blocks` | `inblocks == 0` | no-op; no rejection | [x] |
| 56 | `sha256_inc_finalize` / `sha512_inc_finalize` | `inlen == 0` (finalize with an empty tail) | pads a lone block; no rejection | [x] |
| 57 | `shake256_inc_absorb` | `inlen == 0` | no-op; no rejection | [x] |
| 58 | `shake256_inc_squeeze` | `outlen == 0` | writes nothing; no rejection | [x] |
| 59 | `shake256` / `sha256` / `blake256` / `haraka_S` | `inlen == 0` (empty input) | hash of the empty string; no rejection | [x] |
| 60 | `mgf1_256` / `mgf1_512` / `blake256_mgf1` / `blake512_mgf1` | `outlen == 0`, and `outlen` not a multiple of the block size (partial trailing block) | no rejection; the partial-block `memcpy` tail must match | [x] |
| 61 | `haraka_S_inc_squeeze` | `outlen == 0`, and `outlen` spanning several sponge blocks | no rejection | [x] |
| 62 | `chain_lengths` | message byte values `0x00` and `0xFF` (extremes of the base-`w` decomposition) plus the `SPX_WOTS_LEN2` checksum overflow path | no rejection; every `lengths[i] < SPX_WOTS_W` | [x] |
| 63 | `crypto_sign_seed_keypair` | seed of exactly `CRYPTO_SEEDBYTES = 3*SPX_N` (the only accepted size; no length parameter exists, so there is no check) | returns `0`; `pk`/`sk` fully determined by the seed | [x] |
| 64 | `crypto_sign` | `mlen == 0` | returns `0`; `*smlen == SPX_BYTES` | [x] |

## Phase C result

All **64** rows above have a passing differential test in `tests/errors.rs`, for
**all 48** feature combinations (44 error tests per config for the blake
backend, 43/42 for sha2/haraka/shake depending on how many backend-specific
`*_edges` tests apply). Every test asserts the C and the Rust return the *same*
code / sentinel and leave the *same* observable state (context images, output
buffers including marker tails, and the exported `DRBG_ctx` global) -- never
merely "both failed".

Generic boundaries covered on top of the table: null output pointers
(rows 5, 6, 32, 34), zero lengths (rows 4, 10, 11, 15, 27, 36, 45, 47, 52-61),
oversized lengths (rows 2, 8, 16), one-past-range values (rows 1, 3, 9, 13, 14,
26, 31, 46, 48) and out-of-range enum values crossing FFI (rows 42-44: `set_type`
with `7, 8, 255, 256, 0xFFFFFFFF` -- no valid `SPX_ADDR_TYPE_*` variant -- plus
the other one-byte setters above 255).

### Two rows whose original expectation was wrong

Rows 23 and 31 were first written expecting `-1`. Running them against the C
showed it *accepts* many of those inputs. The cause is in the C, and the Rust
reproduces it exactly, so the rows were rewritten to state the real C behaviour
(see the row text). Recorded here because it is exactly the sort of thing a
"looks-reasonable" expectation would have papered over: the assertion was
changed to match the C, **not** the C to match the assertion.
