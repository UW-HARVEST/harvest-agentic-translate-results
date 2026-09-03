# ERRORS.md — error-surface table

Every row below was derived by grepping `c_src` for each distinct way the C
code rejects, clamps, truncates or short-circuits on its input:

```
grep -rn "return -1\|return NULL\|assert\|RETURN_ERROR\|RNG_BAD\|#error\|abort()\|exit(" \
     --include=*.c --include=*.h c_src/
```

plus every explicit `if (...) return`, every null check, every min/max
constant, and every implicit narrowing cast on a public parameter.

Nothing here is invented: the "expected C result" column is what the C source
literally does. `SPX_BYTES`, `SPX_N`, `SPX_WOTS_W` etc. are the
configuration-dependent constants dumped from the C preprocessor by
`harness/dump_params.c`.

Tests live in `tests/errors.rs` (`err_*` / `ok_*`) with a few rows covered by
`tests/configs.rs` / `tests/backends.rs` (`cfg*`). Run everything with
`./run_tests.sh`; each row's `test` column names the `#[test]` that constructs
the condition, calls **both** `.so`s and asserts they return the **same**
code/sentinel and the same output-buffer side effects.

## A. Explicit error returns (`return -1` / `RNG_BAD_*`)

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `crypto_sign_verify` | `siglen != SPX_BYTES` (`sign.c:180`) — tested at `0`, `1`, `2`, `SPX_BYTES-2`, `SPX_BYTES-1`, `SPX_BYTES+1`, `SPX_BYTES+2`, `2*SPX_BYTES`, `usize::MAX` | `-1` | `err_verify_wrong_siglen` |
| 2 | `crypto_sign_verify` | `memcmp(root, pub_root, SPX_N) != 0` (`sign.c:236`) — correct length but a flipped byte anywhere in the FORS signature / WOTS signatures / auth paths, an all-zero signature, a random signature, or a flipped public-key byte | `-1` | `err_verify_bad_signature` |
| 3 | `crypto_sign_open` | `smlen < SPX_BYTES` (`sign.c:272`) — tested at `0`, `1`, `2`, `SPX_BYTES/2`, `SPX_BYTES-2`, `SPX_BYTES-1` | `-1`, **and** `memset(m, 0, smlen)`, **and** `*mlen = 0` | `err_open_short_smlen` |
| 4 | `crypto_sign_open` | inner `crypto_sign_verify` fails (`sign.c:280`) — `smlen >= SPX_BYTES` with a corrupted signature region, an all-zero `sm`, or a random `sm` | `-1`, **and** `memset(m, 0, smlen)` (the *full* `smlen`, not `smlen-SPX_BYTES`), **and** `*mlen = 0` | `err_open_bad_signature` |
| 5 | `seedexpander_init` | `maxlen >= 0x100000000` (`rng.c:33`) — tested at `0x1_0000_0000`, `0x1_0000_0001`, `0x2_0000_0000`, `0x8000_0000_0000_0000`, `u64::MAX` | `RNG_BAD_MAXLEN` = `-1`; `ctx` left **untouched** (the early return precedes every write — asserted with a `0x5A` pre-fill) | `err_seedexpander_init_maxlen` |
| 6 | `seedexpander` | `x == NULL` (`rng.c:67`) — the only explicit null check on an output pointer. The check *precedes* the length check, so `NULL` wins even when `xlen` is also invalid; tested at `xlen` ∈ {0, 1, 16, 4096, `u64::MAX`} | `RNG_BAD_OUTBUF` = `-2`; `ctx` unchanged | `err_seedexpander_null_outbuf` |
| 7 | `seedexpander` | `xlen >= ctx->length_remaining` (`rng.c:69`) — note `>=`, so requesting *exactly* the remaining length also fails; tested at `xlen` = `maxlen`, `maxlen+1`, `maxlen+1000`, `u64::MAX` for `maxlen` ∈ {0, 1, 16, 100} | `RNG_BAD_REQ_LEN` = `-3`; `ctx` unchanged and **no output bytes written** | `err_seedexpander_req_len` |
| 8 | `AES256_ECB` → `handleErrors` | OpenSSL `EVP_*` failure (`rng.c:109`) → `ERR_print_errors_fp` + `abort()` | **unreachable**: AES-256-ECB with a 32-byte key over one 16-byte block cannot fail. The Rust translation uses the `aes` crate and has no equivalent failure mode. Not testable without fault injection. | — (documented) |

## B. Success sentinels (so "same error" is never confused with "both failed")

| # | function | condition | expected C result | test |
|---|----------|-----------|-------------------|------|
| 9 | `seedexpander_init` | `maxlen <= 0xFFFFFFFF` (boundary `0xFFFF_FFFE`, `0xFFFF_FFFF` = largest accepted) | `RNG_SUCCESS` = `0`, full 72-byte `AES_XOF_struct` written | `err_seedexpander_init_maxlen` |
| 10 | `seedexpander` | `xlen < ctx->length_remaining`; 64 chained draws | `0`, `xlen` bytes of XOF output, `ctx` advanced | `ok_seedexpander_stream`, `cfg55_seedexpander_stream` |
| 11 | `randombytes` (`rng.c`) | any `xlen`, **including `0`** | always `RNG_SUCCESS` = `0`; nothing written for `xlen == 0`, but `Key`/`V` are still re-derived and `reseed_counter` still incremented — asserted by requiring the state to differ from the freshly seeded state | `err_randombytes_zero_len`, `cfg54_randombytes_stream` |
| 12 | `crypto_sign_seed_keypair` | any seed (all-zero, all-`0xFF`, counting, random) | always `0` | `ok_sign_verify_open_sentinels`, `cfg28_seed_keypair` |
| 13 | `crypto_sign_keypair` | DRBG seeded identically in both libraries | always `0` | `ok_keypair_from_drbg`, `cfg29_keypair_from_drbg` |
| 14 | `crypto_sign_signature` | any message | always `0`, `*siglen = SPX_BYTES` | `ok_sign_verify_open_sentinels`, `cfg30_signature_and_verify` |
| 15 | `crypto_sign` | any message | always `0`, `*smlen = SPX_BYTES + mlen` | `ok_sign_verify_open_sentinels`, `cfg31_sign_and_open` |
| 16 | `crypto_sign_verify` | valid signature, correct `siglen` | `0` | `err_verify_wrong_siglen`, `cfg30_signature_and_verify` |
| 17 | `crypto_sign_open` | valid `sm` | `0`, `*mlen = smlen - SPX_BYTES`, message moved to `m` | `ok_sign_verify_open_sentinels`, `cfg31_sign_and_open` |
| 18 | `blake256` / `blake512` | any input incl. `inlen == 0` | always `0` (the return is unconditional) | `ok_blake_return_zero`, `cfg34_37_blake_one_shot` |

## C. Silent truncation of out-of-range values (the "C enum accepts any int" class)

`app/include/address.h` documents `set_type`'s argument as one of
`SPX_ADDR_TYPE_WOTS = 0 … SPX_ADDR_TYPE_FORSPRF = 6`, but the parameter is a
plain `uint32_t` and `address.c` narrows it with `(unsigned char)`. Any `u32` is
therefore a legal input that the C handles by truncation, and the Rust must
truncate identically. Same for every other single-byte address field. Every row
here also asserts that the other 31 address bytes are untouched.

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 19 | `SPX_set_type` | values with no valid variant: `7`, `8`, `9`, `100`, `254`, `255`, `256`, `257`, `259`, `262`, `511`, `512`, `0xFFFF`, `0xFFFF_FF00`, `0xFFFF_FF06`, `0xFFFF_FFFF`, plus 512 random `u32`, over 3 base addresses | `addr[SPX_OFFSET_TYPE] = (u8)type`; all other 31 bytes unchanged | `err_set_type_out_of_range_enum`, `cfg04_set_type_all_variants_and_beyond` |
| 20 | `SPX_set_layer_addr` | `layer >= 256` (valid range `0..SPX_D`), incl. `SPX_D`, `255`, `256`, `0xFFFF_FFFF`, 256 random | `addr[SPX_OFFSET_LAYER] = (u8)layer` | `err_addr_byte_field_truncation`, `cfg02_set_layer_addr` |
| 21 | `SPX_set_chain_addr` | `chain >= 256` (valid range `0..SPX_WOTS_LEN`) | `addr[SPX_OFFSET_CHAIN_ADDR] = (u8)chain` | `err_addr_byte_field_truncation`, `cfg05_other_addr_setters` |
| 22 | `SPX_set_hash_addr` | `hash >= 256` (valid range `0..SPX_WOTS_W`) | `addr[SPX_OFFSET_HASH_ADDR] = (u8)hash` | `err_addr_byte_field_truncation`, `cfg05_other_addr_setters` |
| 23 | `SPX_set_tree_height` | `tree_height >= 256` (valid range `0..=SPX_TREE_HEIGHT`) | `addr[SPX_OFFSET_TREE_HGT] = (u8)tree_height` | `err_addr_byte_field_truncation`, `cfg05_other_addr_setters` |
| 24 | `SPX_set_tree_index` | any `u32` incl. `0xFFFF_FFFF` | 4 bytes **big-endian** at `SPX_OFFSET_TREE_INDEX` (asserted against `to_be_bytes`) | `err_addr_byte_field_truncation`, `cfg05_other_addr_setters` |
| 25 | `SPX_set_keypair_addr` | any `u32` | 4 bytes big-endian at `SPX_OFFSET_KP_ADDR` | `err_addr_byte_field_truncation`, `cfg05_other_addr_setters` |
| 26 | `SPX_set_tree_addr` | any `u64` incl. `u64::MAX`, and `2^(TREE_HEIGHT*(D-1))` (one past the documented tree count) | 8 bytes big-endian at `SPX_OFFSET_TREE` | `err_addr_byte_field_truncation`, `cfg03_set_tree_addr` |
| 27 | `SPX_ull_to_bytes` | `outlen == 0` — `for (i = (signed int)outlen - 1; i >= 0; i--)` runs zero times | nothing written (asserted with a `0xA5` guard fill) | `err_ull_to_bytes_outlen_edges`, `cfg07_ull_to_bytes` |
| 28 | `SPX_ull_to_bytes` | `outlen > 8` (`9`, `12`, `16`, `32`, `64`) — more bytes than a `u64` holds | the leading `outlen-8` bytes become `0` once `in` has been shifted out; nothing written past `outlen` | `err_ull_to_bytes_outlen_edges` |
| 29 | `SPX_bytes_to_ull` | `inlen == 0` | returns `0` | `err_bytes_to_ull_inlen_edges`, `cfg09_bytes_to_ull` |
| 30 | `SPX_bytes_to_ull` | `inlen == 8` (largest well-defined: the shift `8*(inlen-1-i)` peaks at 56); all-`0xFF` returns `u64::MAX` | big-endian decode of 8 bytes | `err_bytes_to_ull_inlen_edges` |
| 31 | `SPX_bytes_to_ull` | `inlen > 8` (`9`…`16`) — `((u64)in[i]) << (8*(inlen-1-i))` has a shift count `>= 64`, which is **undefined behaviour in C** | Excluded from byte-equality: there is no defined C behaviour to match. The test only calls both and requires neither to abort or read past the (32-byte) input. | `err_bytes_to_ull_inlen_edges` |

## D. Zero / empty / boundary lengths on the hash and tree primitives

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 32 | `SPX_thash` | `inblocks == 0` | `SPX_VLA(..., SPX_N + SPX_ADDR_BYTES + 0)`, i.e. a hash of just `pub_seed‖addr` (`simple`) or the MGF1-masked equivalent (`robust`); for `haraka` the `inblocks == 1` test fails so it takes the `haraka_S` sponge branch | `err_thash_inblocks_edges`, `cfg12_thash_all_inblocks` |
| 33 | `SPX_thash` | `inblocks == 1` vs `== 2` — the branch point in **all four** backends (`if (inblocks > 1) thash_512(...)` for blake/sha2 when `X512=1`; `if (inblocks == 1)` F-function vs sponge for haraka) | distinct code paths, both must match | `cfg12_thash_all_inblocks` |
| 34 | `SPX_thash` | the `inblocks` values the library actually uses — `1`, `2`, `SPX_WOTS_LEN`, `SPX_FORS_TREES` — plus `3`, `4`, `16`, `64` | must match at every value | `cfg12_thash_all_inblocks` |
| 35 | `SPX_gen_message_random` / `SPX_hash_message` | `mlen == 0`; for `sha2` this takes the "cannot fill an entire block" branch | both hash zero message bytes | `cfg13_gen_message_random`, `cfg14_hash_message` |
| 36 | `SPX_gen_message_random` (sha2) | `SPX_N + mlen == SPX_SHAX_BLOCK_BYTES` exactly — the `<` boundary at `hash_sha2.c:91`; ±2 around it for both 64- and 128-byte blocks | switches from `inc_finalize` to `inc_blocks` + `inc_finalize` | `cfg13_gen_message_random` |
| 37 | `SPX_hash_message` (sha2) | `SPX_N + SPX_PK_BYTES + mlen == k*SPX_SHAX_BLOCK_BYTES` exactly — the boundary at `hash_sha2.c:154`; ±2 around it | switches branch | `cfg14_hash_message` |
| 38 | `blake256_update` / `blake512_update` | `datalen` is in **bits**, not bytes; `datalen == 0`, repeated | `buflen` stays `0`, nothing compressed | `err_blake_update_zero` |
| 39 | `blake256_final` / `blake512_final` | the three-way branch at `blake256.c:346` — `buflen == 440` bits (one `0x81` padding byte), `buflen < 440` (one compression, `nullt = 1` when `buflen == 0`), `buflen > 440` (two compressions); driven via `*_update` with 0/1/54/55/56/63/64/… byte prefixes and the 110/111/112 equivalents for BLAKE-512 | three distinct padding paths; digest **and** final state compared | `err_blake_final_padding_branches`, `cfg35_37_blake_incremental` |
| 40 | `SPX_blake256_mgf1`, `SPX_blake512_mgf1`, `SPX_mgf1_256`, `SPX_mgf1_512` | `outlen == 0` (nothing written); `outlen == OUTPUT_BYTES` (exact multiple, tail branch skipped); `outlen == OUTPUT_BYTES ± 1` (tail branch taken); `inlen == 0` | nothing / whole blocks / partial tail; nothing past `outlen` | `err_mgf1_outlen_edges`, `cfg38_blake_mgf1`, `cfg42_sha_mgf1` |
| 41 | `shake256` / `SPX_haraka_S` / `blake*` / `sha*` | `outlen == 0` and `inlen == 0` in every combination | nothing written / empty absorb | `err_squeeze_zero_len`, `cfg44_shake256_one_shot`, `cfg49_haraka_s_one_shot` |
| 42 | `sha256_inc_blocks` / `sha512_inc_blocks` | `inblocks == 0`, repeated | state byte-identical to the post-`inc_init` state | `err_sha_inc_zero_blocks` |
| 43 | `sha256_inc_finalize` / `sha512_inc_finalize` | `inlen == 0`; and every `inlen` in `block-19 … block+19` and `2*block … 2*block+19`, covering the "padding needs an extra block" boundary | distinct padding paths | `err_sha_inc_finalize_padding`, `cfg41_sha_incremental` |
| 44 | `randombytes_init` | `personalization_string == NULL` — the second explicit null check (`rng.c:150`) | XOR step skipped, `seed_material = entropy_input`; cross-checked against an all-zero personalization string, which must give the identical state | `err_randombytes_init_null_pers`, `cfg53_randombytes_init` |
| 45 | `randombytes_init` | `personalization_string != NULL`, incl. all-`0xFF` | all 48 bytes XORed; both input buffers left untouched | `ok_randombytes_init_with_pers`, `cfg53_randombytes_init` |
| 46 | `SPX_compute_root` | `tree_height == 0` — `for (i = 0; i < tree_height - 1; i++)` underflows to `0xFFFFFFFF` | **Excluded**: the loop would read ≈4 G × `SPX_N` bytes past `auth_path` in **both** implementations (out-of-bounds read; no defined C behaviour to match). Unreachable from any public entry point — every shipped parameter set has `SPX_TREE_HEIGHT >= 3` and `SPX_FORS_HEIGHT >= 6`. | — (documented) |
| 47 | `SPX_compute_root` | `tree_height == 1` (smallest safe value) and `2`, `3`, `SPX_FORS_HEIGHT`, `SPX_TREE_HEIGHT`; `leaf_idx` even vs odd (the parity branch at `utils.c:57` / `:74`); `idx_offset` ∈ {0, 1, 2, `0xFFFF_FFFE`, random} | root **and** the final `addr` state compared | `cfg15_compute_root_shapes` |
| 48 | `SPX_treehash` / `SPX_wots_treehashx1` / `SPX_fors_treehashx1` | `leaf_idx == (uint32_t)~0` — the "don't generate an auth path" sentinel `merkle_gen_root` passes; no leaf ever matches, so the auth-path `memcpy`s never fire | root still computed, `auth_path` left at its guard fill | `cfg16_treehash_shapes`, `cfg17_wots_treehashx1`, `cfg26_merkle_sign` |
| 49 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` (`rng.c:186`) vs non-`NULL`; `V` = zeros / all-`0xFF` (full 16-byte carry) / `0xFF` in the low or high bytes only / random | the increment-`V` carry loop must propagate identically; `Key` and `V` compared, `provided_data` left untouched | `cfg52_drbg_update` |

## A note on one C quirk the tests had to accommodate

`lib/blake/src/hash_blake.c` calls

```c
blakeX_update(&S, R, SPX_N);
blakeX_update(&S, pk, SPX_PK_BYTES);
blakeX_update(&S, m, mlen);
```

but `blake256_update` / `blake512_update` take their length argument in **bits**
(`blake256(out, in, inlen)` itself calls `blake256_update(&S, in, inlen*8)`).
So under `HASH_BACKEND=blake` only the first `SPX_N/8` bytes of `R`, the first
`SPX_PK_BYTES/8` bytes of `pk` and the first `mlen/8` bytes of the message are
absorbed into the message hash.

Likewise `gen_message_random` for the BLAKE backend ends with
`blakeX_final(&S, R)`, which writes the **full** digest (32 bytes for BLAKE-256,
64 for BLAKE-512) into `R`, not `SPX_N` bytes — unlike sha2/shake/haraka, which
write exactly `SPX_N`. `sign.c` is unaffected because it passes `sig`, which is
`SPX_BYTES` long.

Both are properties of the C, so both are properties the Rust must reproduce —
and it does. The consequences for the tests:

* `cfg13_gen_message_random` sizes its output buffer for the largest possible
  write and compares the whole buffer, so the two implementations must agree on
  *how many* bytes they touch, not just on the first `SPX_N`.
* `err_verify_bad_signature` / `err_open_bad_signature` only assert
  "must be rejected" for corruptions at offsets `>= SPX_N` (the FORS signature,
  the WOTS signatures and the authentication paths, which always feed the
  recomputed root). For bytes inside `R` and inside the message, the C's own
  answer is the ground truth and the test asserts only that C and Rust agree —
  because for BLAKE many of those bytes genuinely do not affect verification.
  Each test additionally asserts a minimum number of actual rejections so it
  cannot pass vacuously.

## Gate

- [x] Rows 1–7, 9–30, 32–45, 47–49 each have a passing differential test.
- [x] Rows 8, 31, 46 are documented as untestable, with the reason
      (unreachable OpenSSL failure; C undefined behaviour; out-of-bounds read in
      both implementations).
- [x] All rows pass in **all 48** feature combinations (`./run_tests.sh`:
      `combos passed: 48   failed: 0`).
