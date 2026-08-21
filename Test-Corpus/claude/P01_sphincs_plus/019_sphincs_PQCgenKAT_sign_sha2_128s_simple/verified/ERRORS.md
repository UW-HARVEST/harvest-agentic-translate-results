# ERRORS.md — error-surface table

Derived mechanically from the C sources by grepping for every `return -1`,
`return NULL`, `return RNG_*`, `assert`, `abort()`, explicit range check,
null check and min/max constant:

```
grep -rn -e 'return -1' -e 'return NULL' -e 'assert' -e 'RNG_BAD' -e 'return RNG' \
        -e 'abort()' -e 'handleErrors' -e '== NULL' -e '!= NULL' -e 'memset(m, 0' \
        c_src/app/src c_src/lib
```

Findings (complete):

* `c_src/app/src/sign.c:179-181` — `crypto_sign_verify`: `if (siglen != SPX_BYTES) return -1;`
* `c_src/app/src/sign.c:235-237` — `crypto_sign_verify`: `if (memcmp(root, pub_root, SPX_N)) return -1;`
* `c_src/app/src/sign.c:269-272` — `crypto_sign_open`: `if (smlen < SPX_BYTES) { memset(m,0,smlen); *mlen = 0; return -1; }`
* `c_src/app/src/sign.c:277-281` — `crypto_sign_open`: verification failure → `memset(m,0,smlen); *mlen = 0; return -1;`
* `c_src/app/src/rng.c:32-33` — `seedexpander_init`: `if (maxlen >= 0x100000000) return RNG_BAD_MAXLEN;` (`-1`)
* `c_src/app/src/rng.c:66-67` — `seedexpander`: `if (x == NULL) return RNG_BAD_OUTBUF;` (`-2`)
* `c_src/app/src/rng.c:68-69` — `seedexpander`: `if (xlen >= ctx->length_remaining) return RNG_BAD_REQ_LEN;` (`-3`)
* `c_src/app/src/rng.c:143` — `randombytes_init`: `if (personalization_string)` — NULL is a *valid* input meaning "no personalization".
* `c_src/app/src/rng.c:205` — `AES256_CTR_DRBG_Update`: `if (provided_data != NULL)` — NULL is a *valid* input meaning "no provided data".
* `c_src/app/src/rng.c:106-130` — `AES256_ECB`/`handleErrors` → `abort()` only on OpenSSL allocation/EVP failure (unreachable for a well-formed 32-byte key / 16-byte block; not an input-driven rejection).
* All other `#error` directives are *compile-time* parameter sanity checks
  (`SPX_D` must divide `SPX_FULL_HEIGHT`, `SPX_TREE_BITS > 64`,
  `SPX_BLAKE256_OUTPUT_BYTES < SPX_N`, `SPX_WOTS_W` ∉ {16,256}, …). They are not
  runtime rejections and are reflected in `src/params.rs` by construction (all 48
  valid combinations compile; no other combination exists).
* There are **no** `assert()`s and **no** `return NULL`s anywhere in the library.
* `SPX_thash`, `SPX_prf_addr`, `SPX_treehash`, `SPX_compute_root`,
  `SPX_wots_*`, `SPX_fors_*`, `SPX_merkle_*`, `SPX_set_*_addr`,
  `SPX_ull_to_bytes`, `SPX_u32_to_bytes`, `SPX_bytes_to_ull`,
  `crypto_sign_signature`, `crypto_sign_seed_keypair`, `crypto_sign_keypair`,
  `crypto_sign` and all backend primitives (`blake*`, `sha*`, `shake256*`,
  `haraka*`) have **no** input validation at all: they return `void`, or always
  return `0`. Their "error surface" is therefore *the absence of a check*, which
  must also be reproduced (the Rust must not add a check and must not
  panic/abort where the C silently proceeds). Those rows are included below and
  are just as important as the explicit rejections.

## The table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `crypto_sign_verify` | `siglen == 0` | returns `-1`, no output written | `err_verify_siglen_zero` | [x] |
| E2 | `crypto_sign_verify` | `siglen == SPX_BYTES - 1` (one below valid) | returns `-1` | `err_verify_siglen_minus1` | [x] |
| E3 | `crypto_sign_verify` | `siglen == SPX_BYTES + 1` (one above valid) | returns `-1` | `err_verify_siglen_plus1` | [x] |
| E4 | `crypto_sign_verify` | `siglen == u64::MAX`-ish / huge (`usize::MAX`) | returns `-1` (checked before any read) | `err_verify_siglen_huge` | [x] |
| E5 | `crypto_sign_verify` | `siglen == SPX_BYTES`, signature bit-flipped (any of the R / FORS / WOTS / auth-path regions) | returns `-1` (root mismatch) | `err_verify_corrupt_sig` | [x] |
| E6 | `crypto_sign_verify` | `siglen == SPX_BYTES`, message bit-flipped (**every** byte position is tried) | returns `-1` for the message bytes the C actually hashes. **C QUIRK (faithfully reproduced):** for the `blake` backend `lib/blake/src/hash_blake.c` calls `blakeX_update(&S, m, mlen)`, but `blake256_update`/`blake512_update` take their length in **bits** (`(datalen >> 3) & 0x3F` bytes are copied). So only about `mlen/8` message bytes reach the digest and the C **accepts** a flip beyond that prefix. The other three backends pass proper byte lengths and reject every position. The test asserts C == Rust at every position (the real contract) plus the absolute `-1` at message byte 0, and additionally requires *all* positions to be rejected for the non-BLAKE backends. | `err_verify_corrupt_msg` | [x] |
| E7 | `crypto_sign_verify` | `siglen == SPX_BYTES`, wrong public key (root byte flipped) | returns `-1` | `err_verify_wrong_pk` | [x] |
| E8 | `crypto_sign_verify` | `siglen == SPX_BYTES`, `pk` root correct but `pub_seed` flipped | returns `-1` | `err_verify_wrong_pubseed` | [x] |
| E9 | `crypto_sign_verify` | all-zero signature + all-zero pk | returns `-1` (overwhelmingly), byte-identical between C and Rust | `err_verify_all_zero` | [x] |
| E10 | `crypto_sign_verify` | valid signature, `mlen == 0` | returns `0` (a *valid* edge case, must not be rejected) | `err_verify_empty_msg_ok` | [x] |
| E11 | `crypto_sign_open` | `smlen == 0` | returns `-1`; `*mlen = 0`; `memset(m, 0, 0)` (no write) | `err_open_smlen_zero` | [x] |
| E12 | `crypto_sign_open` | `0 < smlen < SPX_BYTES` (e.g. `SPX_BYTES-1`, `1`, `SPX_N`) | returns `-1`; `*mlen = 0`; first `smlen` bytes of `m` zeroed | `err_open_smlen_short` | [x] |
| E13 | `crypto_sign_open` | `smlen == SPX_BYTES` (empty message) with a valid signature | returns `0`; `*mlen = 0` | `err_open_smlen_exact` | [x] |
| E14 | `crypto_sign_open` | `smlen >= SPX_BYTES` but signature corrupt (flip at offset 0, `SPX_N`, `SPX_BYTES-1`, `SPX_BYTES`, and the last byte) | returns `-1`; `*mlen = 0`; **all `smlen` bytes** of `m` zeroed (note: `memset(m,0,smlen)`, i.e. more than `*mlen`). The absolute `-1` is asserted for offsets `<= SPX_BYTES`; a flip in the message *tail* is subject to the E6 BLAKE quirk, so only the differential assertion applies there. | `err_open_corrupt` | [x] |
| E15 | `crypto_sign_open` | `smlen` one larger than what was signed (extra trailing byte) | returns `-1` (message digest differs) | `err_open_smlen_extra` | [x] |
| E16 | `seedexpander_init` | `maxlen == 0x100000000` (exactly the bound) | returns `RNG_BAD_MAXLEN` (`-1`), `ctx` untouched | `err_seedexpander_init_maxlen_bound` | [x] |
| E17 | `seedexpander_init` | `maxlen > 0x100000000` (e.g. `0xFFFFFFFFFFFFFFFF`) | returns `-1` | `err_seedexpander_init_maxlen_huge` | [x] |
| E18 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (one below the bound) | returns `RNG_SUCCESS` (`0`) and initialises `ctx` (`ctr[8..12] = BE(maxlen)`, `ctr[12..16] = 0`, `buffer_pos = 16`, `buffer = 0`) | `err_seedexpander_init_maxlen_ok` | [x] |
| E19 | `seedexpander_init` | `maxlen == 0` | returns `0`, `length_remaining = 0` (so every later `seedexpander` call fails with `-3`) | `err_seedexpander_init_maxlen_zero` | [x] |
| E20 | `seedexpander` | `x == NULL` (any `xlen`, including 0) | returns `RNG_BAD_OUTBUF` (`-2`) **before** touching `ctx` | `err_seedexpander_null_out` | [x] |
| E21 | `seedexpander` | `xlen == ctx->length_remaining` (exactly, `>=` is the check) | returns `RNG_BAD_REQ_LEN` (`-3`), `ctx` untouched | `err_seedexpander_xlen_eq_remaining` | [x] |
| E22 | `seedexpander` | `xlen > ctx->length_remaining` | returns `-3` | `err_seedexpander_xlen_gt_remaining` | [x] |
| E23 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | returns `RNG_SUCCESS` (`0`), nothing written, `ctx` unchanged (loop never entered) | `err_seedexpander_xlen_zero` | [x] |
| E24 | `seedexpander` | `xlen == length_remaining - 1` (largest accepted) | returns `0`, produces the full stream | `err_seedexpander_xlen_max_ok` | [x] |
| E25 | `seedexpander` | `ctx->buffer_pos` forced to `> 16` (e.g. `17`, `0xFFFF...`) — `16 - buffer_pos` underflows in `unsigned long` | C computes a huge `avail`, takes the "buffer has what we need" branch and `memcpy`s out of bounds; Rust must wrap identically (`wrapping_sub`) and take the same branch / return `RNG_SUCCESS` | `err_seedexpander_buffer_pos_overflow` | [x] |
| E26 | `randombytes_init` | `personalization_string == NULL` | valid: seeds from `entropy_input` only | `err_randombytes_init_null_ps` | [x] |
| E27 | `randombytes` | `xlen == 0` | returns `RNG_SUCCESS`; **still** runs `AES256_CTR_DRBG_Update(NULL, …)` and `reseed_counter++` (the `while` is not entered but the tail always runs) | `err_randombytes_xlen_zero` | [x] |
| E28 | `randombytes` | `xlen` not a multiple of 16 (e.g. 1, 15, 17, 31) | returns `0`; only `xlen` bytes written, the remainder of the last AES block discarded | `err_randombytes_partial_block` | [x] |
| E29 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` | valid: no XOR step | `err_drbg_update_null_pd` | [x] |
| E30 | `AES256_CTR_DRBG_Update` | `V` all `0xff` (carry ripples through all 16 bytes → wraps to all-zero) | identical `Key`/`V` output | `err_drbg_update_v_all_ff` | [x] |
| E31 | `randombytes` | `DRBG_ctx.V` all `0xff` (carry propagation across the whole counter) | identical output bytes and identical resulting `DRBG_ctx` | `err_randombytes_v_all_ff` | [x] |
| E32 | `SPX_ull_to_bytes` | `outlen == 0` | writes nothing (loop from `-1`) — must not panic | `err_ull_to_bytes_outlen_zero` | [x] |
| E33 | `SPX_ull_to_bytes` | `outlen > 8` (e.g. 16) | zero-pads the high bytes (`in >> 8` shifts out to 0) | `err_ull_to_bytes_outlen_gt8` | [x] |
| E34 | `SPX_bytes_to_ull` | `inlen == 0` | returns `0` | `err_bytes_to_ull_inlen_zero` | [x] |
| E35 | `SPX_bytes_to_ull` | `inlen > 8` (e.g. 12) — `8*(inlen-1-i)` is a shift `>= 64`, UB in C, in practice `x86-64 shl` masks the count to 6 bits | C and Rust must produce the **same** value | `err_bytes_to_ull_inlen_gt8` | [x] |
| E36 | `SPX_set_type` | out-of-range "enum" value (`type` has no valid `SPX_ADDR_TYPE_*` variant: 7, 255, 256, 0x0000_01FF / 0xFFFF_FF01, `u32::MAX`) | only the low byte is stored (`(unsigned char)type`); no validation | `err_set_type_out_of_range` | [x] |
| E37 | `SPX_set_layer_addr` / `SPX_set_chain_addr` / `SPX_set_hash_addr` / `SPX_set_tree_height` | value `> 255` | truncated to the low byte, no validation | `err_addr_setters_truncate` | [x] |
| E38 | `SPX_thash` | `inblocks == 0` | no rejection: hashes `pub_seed‖addr` (`simple`) with a zero-length data part; `haraka` takes the `else` branch (`inblocks != 1`) with a 32-byte input | `err_thash_inblocks_zero` | [x] |
| E39 | `SPX_thash` | `inblocks == 1` vs `> 1` (the branch to `thash_512` for `SPX_SHA512`/`SPX_BLAKE512`, and the `haraka` F-function branch) | different primitive selected; both must match | `cfg_thash_inblocks` (Phase B) | [x] |
| E40 | `SPX_thash` | very large `inblocks` (`SPX_WOTS_LEN`, `SPX_FORS_TREES`, 255) | no rejection; must match | `err_thash_inblocks_large` | [x] |
| E41 | `SPX_treehash` | `tree_height == 0` | `1 << 0 == 1` leaf, one `gen_leaf` call, root = that leaf, no `thash` | `err_treehash_height_zero` | [x] |
| E42 | `SPX_compute_root` | `tree_height == 1` (`for (i = 0; i < tree_height - 1; …)` runs zero times) | one `thash`; must match | `err_compute_root_height_one` | [x] |
| E43 | `SPX_compute_root` | `tree_height == 0` → `tree_height - 1` underflows to `0xFFFFFFFF` in `uint32_t` | C loops ~2^32 times reading past `auth_path` — **not exercised** (would run for hours / segfault in both). Documented as a shared UB; the Rust reproduces the same `u32` wrapping expression so the divergence is only in wall-clock, not semantics. | *(documented, not executed)* | [x] |
| E44 | `SPX_wots_gen_leafx1` | `info->wots_sign_leaf == leaf_idx` but `info->wots_sig == NULL` | C writes through the NULL pointer → crash. Rust does the same. **Not exercised** (crash in both). The benign form used by the benchmark macro sets `wots_sign_leaf = ~0u`, which never matches, and *that* is tested. | *(documented, not executed)* | [x] |
| E45 | `SPX_fors_sign` / `SPX_fors_pk_from_sig` | `m` shorter than `SPX_FORS_MSG_BYTES` | no check; reads `SPX_FORS_MSG_BYTES` bytes. Tested with a full-length buffer of adversarial values (all `0x00`, all `0xFF`) that drive `message_to_indices` to its extreme indices | `err_fors_extreme_indices` | [x] |
| E46 | `SPX_chain_lengths` | message bytes all `0x00` / all `0xFF` (checksum at its minimum / maximum, exercising the `csum` shift and `base_w` tail) | identical `SPX_WOTS_LEN` lengths | `err_chain_lengths_extremes` | [x] |
| E47 | `crypto_sign_signature` | `mlen == 0` | no rejection, produces a valid signature; `*siglen = SPX_BYTES` | `err_sign_mlen_zero` | [x] |
| E48 | `crypto_sign` / `crypto_sign_open` | `mlen == 0` round trip | `*smlen = SPX_BYTES`; `crypto_sign_open` returns `0` with `*mlen = 0` | `err_sign_open_mlen_zero` | [x] |
| E49 | `blake256_update` / `blake512_update` | `datalen == 0` (bit length) | `S->buflen = 0`, no compression | `err_blake_update_zero` | [x] |
| E50 | `blake*_mgf1` / `SPX_mgf1_256` / `SPX_mgf1_512` | `outlen == 0` | writes nothing (neither loop body nor tail runs) | `err_mgf1_outlen_zero` | [x] |
| E51 | `blake*_mgf1` / `SPX_mgf1_*` | `outlen` exactly one block / one block + 1 / one less than a block (the `(i+1)*OUT <= outlen` boundary and the tail `outlen > i*OUT`) | identical bytes | `err_mgf1_outlen_boundaries` | [x] |
| E52 | `shake256_inc_squeeze` / `haraka_S_inc_squeeze` | `outlen == 0` | writes nothing | `err_squeeze_outlen_zero` | [x] |
| E53 | `sha256_inc_finalize` / `sha512_inc_finalize` | `inlen` at the padding boundaries (55/56/57 and 111/112/113, plus 0) | identical digests (the `if (inlen < 56)` / `< 112` extra-block branch) | `err_sha_inc_finalize_padding` | [x] |
| E54 | `AES256_ECB` | key/ctr aliasing the output buffer is *not* handled by C (`EVP_EncryptUpdate` with overlapping buffers) | not exercised; non-aliased use only | *(documented, not executed)* | [x] |
| E55 | `crypto_sign_open` | `m == NULL` together with `smlen == 0` (the `memset(m, 0, smlen)` writes nothing, so the NULL is never dereferenced) | returns `-1`, `*mlen = 0`, no write through `m` | `err_open_null_out_smlen_zero` | [x] |
| E56 | `SPX_ull_to_bytes`, `SPX_bytes_to_ull`, `SPX_blake256_mgf1` / `SPX_mgf1_256` / `shake256` | NULL output/input pointer combined with a zero length (every one of these C loops does zero iterations, so the NULL is never dereferenced) | no write, no read, no crash; `bytes_to_ull` returns `0` | `err_null_out_zero_len` | [x] |
| E57 | `randombytes_urandom` (`app/src/randombytes.c`) | `xlen == 0`; `xlen > 1048576` (the C chunking bound); repeated calls (the C keeps its `static int fd` open) | fills exactly `xlen` bytes and returns; a byte-for-byte differential test is impossible for a `/dev/urandom` source, so the observable properties are checked instead | `tests/urandom.rs` | [x] |

Legend: rows marked *(documented, not executed)* are C paths whose trigger is a
crash or an effectively infinite loop in the C itself; the Rust reproduces the
same expression/UB, and executing them would only crash the harness. Every other
row has an executed differential test in
`tests/differential.rs` (module `errors`).

## Additional finding: `hash_message` / `gen_message_random` bit-vs-byte length (BLAKE backend)

While building the E6 test the C was found to reject only the first `~mlen/8`
message bytes for `HASH_BACKEND=blake`:

```c
/* lib/blake/src/hash_blake.c */
blakeX_update(&S, R,  SPX_N);          /* SPX_N  passed as a *bit* count */
blakeX_update(&S, pk, SPX_PK_BYTES);   /* likewise */
blakeX_update(&S, m,  mlen);           /* likewise */
```

```c
/* lib/blake/src/blake256.c */
void blake256_update(blakestate256 *S, const u8 *data, u64 datalen) {
  ...
  memcpy((void *)(S->buf + left), (void *)data, (datalen >> 3) & 0x3F);
```

`blake256_update`/`blake512_update` document `datalen` as a **bit** length (the
reference BLAKE API), and the SPHINCS+ glue passes **byte** counts. This is a
real bug in this C reference, not in the test; per the rules the Rust must — and
does — reproduce it verbatim (`src/backends/blake/hash_blake.rs` passes
`SPX_N as u64` / `SPX_PK_BYTES as u64` / `mlen` unchanged). It is covered
byte-for-byte by `cfg_gen_message_random` and `cfg_hash_message` over 63
different `mlen` values, and by the KAT transcript comparison in `./kat_all.sh`.
Two consequences worth recording:

* the BLAKE parameter sets are *not* EUF-CMA secure as built (a message tail can
  be changed freely) — but that is the C's behaviour and is out of scope here;
* the same applies to `gen_message_random`, whose `blakeX_final(&S, R)` also
  writes the **full** 32-/64-byte BLAKE digest into `R`, not just `SPX_N` bytes.
  The differential tests allocate an over-sized `R` and compare all of it, so
  this over-write is verified to be identical too.
