# ERRORS.md — error-surface table

Derived mechanically from the C sources in `c_src/` by grepping for every
`return -1`, `return RNG_*`, `return NULL`, `assert`, `abort`, explicit range
check, null check and min/max constant:

```
grep -rn "return -1\|return NULL\|RETURN_ERROR\|assert\|RNG_BAD\|return RNG\|abort()\|#error" \
     --include=*.c --include=*.h c_src/app c_src/lib
```

The library has **no** error enums and **no** runtime `assert`s.  The complete
set of runtime rejection points is: 4 in `app/src/sign.c`, 3 in `app/src/rng.c`
(plus one unreachable OpenSSL `abort()`).  All the remaining hits are `#error`
directives, i.e. *compile-time* parameter-sanity checks that can never be
observed at run time — they are listed separately at the bottom.

Every row below has a differential test in `tests/errors.rs` that constructs
the exact condition, calls **both** the C `.so` and the Rust `.so`, and asserts
the identical return value *and* identical output-buffer side effects.

## Runtime rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `crypto_sign_verify` (`sign.c:179-181`) | `siglen != SPX_BYTES` — checked *before* anything else, so it fires even for a perfectly valid signature. Sub-cases exercised: `siglen = 0`, `1`, `SPX_BYTES-1`, `SPX_BYTES+1`, `SPX_BYTES*2`, `SIZE_MAX` | returns `-1`; `sig`/`m`/`pk` untouched | `err01_verify_wrong_siglen` |
| 2 | `crypto_sign_verify` (`sign.c:235-237`) | `memcmp(root, pub_root, SPX_N) != 0` — recomputed hypertree root differs from the root in the public key. Reached by flipping one bit anywhere in `sig` (all `SPX_BYTES` regions: R, FORS, WOTS, auth path), in `m`, or in `pk` | returns `-1` | `err02_verify_root_mismatch` |
| 3 | `crypto_sign_open` (`sign.c:269-273`) | `smlen < SPX_BYTES` | returns `-1`, **and** `memset(m, 0, smlen)`, `*mlen = 0`. Note the memset length is `smlen`, *not* `smlen - SPX_BYTES` | `err03_open_smlen_too_short` |
| 4 | `crypto_sign_open` (`sign.c:277-281`) | inner `crypto_sign_verify(sm, SPX_BYTES, sm+SPX_BYTES, smlen-SPX_BYTES, pk)` returns non-zero (corrupted `sm` or wrong `pk`) | returns `-1`, **and** `memset(m, 0, smlen)` (the *whole* `smlen`, which is longer than the message), `*mlen = 0` | `err04_open_verify_fails` |
| 5 | `seedexpander_init` (`rng.c:32-33`) | `maxlen >= 0x100000000` | returns `RNG_BAD_MAXLEN` = `-1`; `ctx` left completely untouched | `err05_seedexpander_init_maxlen` |
| 6 | `seedexpander` (`rng.c:66-67`) | `x == NULL` — checked before the length check, so it wins even when `xlen` is also invalid | returns `RNG_BAD_OUTBUF` = `-2`; `ctx` untouched | `err06_seedexpander_null_outbuf` |
| 7 | `seedexpander` (`rng.c:68-69`) | `xlen >= ctx->length_remaining` (note: `>=`, so requesting *exactly* the remaining budget is already an error, and `xlen == 0` with `length_remaining == 0` also errors) | returns `RNG_BAD_REQ_LEN` = `-3`; `ctx` untouched | `err07_seedexpander_req_len` |
| 8 | `AES256_ECB` → `handleErrors` (`rng.c:106-110`, called from `rng.c:124/127/130`) | an OpenSSL `EVP_*` call fails | `ERR_print_errors_fp(stderr)` then `abort()` | *unreachable* — AES-256-ECB over one 16-byte block cannot fail for any input; documented, not tested (a test would have to break libcrypto) |

## Boundary / degenerate inputs that the C does **not** reject

These are not rejections, but they are the generic API boundaries the task
requires covering.  The C accepts them and produces a defined result, so the
Rust must produce the *same* result rather than panicking.

| # | function | input | C behaviour | test |
|---|----------|-------|-------------|------|
| B1 | `set_type` | `type` outside the documented `SPX_ADDR_TYPE_*` range `0..=6` — a C enum accepts any `int`. Values tested: `7`, `8`, `255`, `256`, `0x100`, `0xFFFFFFFF`, plus `-1` reinterpreted | truncates: `((unsigned char*)addr)[SPX_OFFSET_TYPE] = (unsigned char)type`, i.e. `type & 0xff`; no rejection | `err_b1_set_type_out_of_range` |
| B2 | `set_layer_addr`, `set_chain_addr`, `set_hash_addr`, `set_tree_height` | any `uint32_t`, incl. values far past the valid layer/chain/height range | all truncate to the low byte | `err_b2_single_byte_setters_truncate` |
| B3 | `set_keypair_addr`, `set_tree_index` | any `uint32_t` (`0`, `1`, `0x7FFFFFFF`, `0xFFFFFFFF`) | full 4 bytes written big-endian, no range check | `err_b3_u32_setters_full_range` |
| B4 | `set_tree_addr` | any `uint64_t` incl. `UINT64_MAX` (past the `2^(TREE_HEIGHT*(D-1))` valid range) | full 8 bytes written big-endian, no range check | `err_b4_set_tree_addr_full_range` |
| B5 | `ull_to_bytes` | `outlen = 0` | writes nothing (loop starts at `(signed)0 - 1 = -1`) | `err_b5_ull_to_bytes_zero_len` |
| B6 | `ull_to_bytes` | `outlen` larger than the 8 bytes of the input (`9`, `16`, `32`) | zero-extends on the left; `in` is shifted right past 0 | `err_b6_ull_to_bytes_oversized` |
| B7 | `bytes_to_ull` | `inlen = 0` | returns `0` | `err_b7_bytes_to_ull_zero_len` |
| B8 | `bytes_to_ull` | `inlen > 8` (`9`, `16`) — the shift `8*(inlen-1-i)` exceeds 63 and is UB in C | whatever the compiled C does; the Rust must match bit-for-bit | `err_b8_bytes_to_ull_oversized` |
| B9 | `thash` | `inblocks = 0` | hashes just `pub_seed || addr` (robust: an empty bitmask); no rejection | `err_b9_thash_zero_inblocks` |
| B10 | `blake256` / `blake512` / `sha256` / `shake256` / `haraka_S` | `inlen = 0` | defined: hash of the empty string | `err_b10_backend_hash_empty` |
| B11 | `blake256_mgf1` / `blake512_mgf1` / `mgf1_256` / `mgf1_512` | `outlen = 0`, and `outlen` not a multiple of the block size | `outlen = 0` writes nothing; partial trailing block is filled from a truncated hash | `err_b11_mgf1_boundary_outlen` |
| B12 | `crypto_sign` / `crypto_sign_open` | `mlen = 0` (empty message) | fully defined; `crypto_sign_open` returns `0` with `*mlen = 0` | `err_b12_empty_message_roundtrip` |
| B13 | `crypto_sign_open` | `smlen == SPX_BYTES` exactly (boundary of row 3: `<` not `<=`) | *not* rejected by the length check; proceeds to verify a zero-length message | `err_b13_open_smlen_exactly_spx_bytes` |
| B14 | `seedexpander` | `xlen == ctx->length_remaining - 1` (one step inside the valid range) | succeeds, returns `RNG_SUCCESS` | `err_b14_seedexpander_max_valid_len` |
| B15 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (one step below the row-5 limit) | succeeds, returns `RNG_SUCCESS` | `err_b15_seedexpander_init_max_valid` |
| B16 | `randombytes` (rng.c DRBG) | `xlen = 0` | returns `RNG_SUCCESS`, still runs `AES256_CTR_DRBG_Update` and bumps `reseed_counter` — i.e. it *does* mutate `DRBG_ctx` | `err_b16_randombytes_zero_len` |
| B17 | `randombytes_init` | `personalization_string == NULL` | skips the XOR loop entirely (the null check is the only guard) | `err_b17_randombytes_init_null_pers` |
| B18 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` | skips the XOR loop | `err_b18_drbg_update_null_provided_data` |
| B19 | `seedexpander` | `ctx->buffer_pos > 16` — `buffer_pos` is caller-owned state in a caller-allocated struct, so any `unsigned long` can arrive here | `16 - ctx->buffer_pos` wraps, the `xlen <= avail` branch is taken, and `memcpy(x, ctx->buffer + buffer_pos, xlen)` reads the *following struct fields*; returns `RNG_SUCCESS` and advances `buffer_pos`. No rejection | `err_extra_seedexpander_buffer_pos_out_of_range` |
| B20 | `AES256_CTR_DRBG_Update`, `randombytes` | `V` = all-`0xff`, and every prefix of the 16-byte carry cascade | the increment wraps the whole counter to zero; no rejection | `err_extra_drbg_carry_cascade` |
| B21 | `seedexpander` | `ctr[12..16]` = `0x00,0xff,0xff,0xff`, forcing the 4-byte counter carry on the first re-key | carries into `ctr[12]`; no rejection | `err_extra_seedexpander_ctr_carry` |
| B22 | `crypto_sign`, `crypto_sign_open` | `m` overlapping `sm` (the in-place `crypto_sign(sm, &l, sm+SPX_BYTES, mlen, sk)` / `crypto_sign_open(sm, &l, sm, smlen, pk)` idiom) | `sign.c:254` and `sign.c:284` use **`memmove`**, not `memcpy`, exactly so this works; the Rust `extern "C"` wrappers use `core::ptr::copy` for the same reason | `err_extra_inplace_overlapping_sign_open` |

## Crash-equivalent inputs (both implementations fault; not differentially testable in-process)

These are inputs on which the C dereferences a NULL pointer.  There is no return
code to compare, so no differential test can observe them without forking; they
are listed because the Rust must NOT "helpfully" succeed where the C faults —
an earlier revision of `wotsx1.rs` modelled `wots_sig`/`wots_steps` as
`Option<&mut [u8]>` / an inline array and silently skipped the access, producing
a *different answer* rather than the C's crash.  The current translation keeps
both as raw pointers and dereferences them unconditionally, exactly like the C.

| # | function | trigger | C result | Rust result |
|---|----------|---------|----------|-------------|
| C1 | `SPX_wots_gen_leafx1`, `SPX_wots_treehashx1`, `SPX_merkle_sign` | `info->wots_steps == NULL` (`wotsx1.c:41` loads it unconditionally) | SIGSEGV | SIGSEGV (`wotsx1.rs`: `*info.wots_steps.add(i)`) |
| C2 | `SPX_wots_gen_leafx1`, `SPX_wots_treehashx1` | `info->wots_sig == NULL` **and** `leaf_idx == info->wots_sign_leaf` (`wotsx1.c:58` memcpy runs) | SIGSEGV | SIGSEGV (`wotsx1.rs`: `copy_nonoverlapping(.., info.wots_sig.add(off), SPX_N)`). Note `merkle_gen_root` passes `wots_sign_leaf = ~0u`, so a NULL `wots_sig` is safe there — and *is* exercised by `cfg12_wots_gen_leafx1` |
| C3 | `SPX_treehash` | `gen_leaf == NULL` (`utils.c:121` calls through it) | SIGSEGV | SIGSEGV (`utils.rs` transmutes the `Option<fn>` back and calls it rather than panicking) |
| C4 | `SPX_compute_root` | `tree_height == 0`: `for (i = 0; i < tree_height - 1; i++)` underflows to `0xFFFFFFFF` iterations and walks off `auth_path` | reads `auth_path[0..SPX_N]`, mutates `addr`, hashes, eventually SIGSEGV | same (the wrapper sizes `auth_path` as `max(tree_height,1)*SPX_N` so the first read succeeds as in C instead of panicking immediately) |
| C5 | `SPX_treehash` | `leaf_idx` outside `0 .. 2^tree_height` makes the C write one node *past* `tree_height*SPX_N` | writes `auth_path[tree_height*SPX_N ..][..SPX_N]` | same — and `cfg15_treehash` allocates head-room and compares that over-run node too |

## Compile-time-only checks (`#error`) — not run-time reachable

| location | condition |
|---|---|
| `app/params/params-sphincs-*.h:~30` | `SPX_WOTS_W` not 16 or 256 |
| `app/params/params-sphincs-*.h:~42,~52` | `SPX_WOTS_LEN2` not precomputed for this `SPX_N` |
| `app/params/params-sphincs-*.h:~64` | `SPX_D` does not divide `SPX_FULL_HEIGHT` |
| `app/src/address.c:21-23` | `SPX_TREE_HEIGHT * (SPX_D-1) > 64` |
| `lib/*/src/hash_*.c` | `SPX_TREE_BITS > 64` |
| `lib/blake/include/blake.h:9-11` | `SPX_BLAKE256_OUTPUT_BYTES < SPX_N` |
| `lib/sha2/include/sha2.h:13-15` | `SPX_SHA256_OUTPUT_BYTES < SPX_N` |
| `lib/sha2/src/hash_sha2.c:78-80,138-140` | `SPX_N > SPX_SHAX_BLOCK_BYTES`; `SPX_SHAX_BLOCK_BYTES` not a power of 2 |

All of these hold for every one of the 48 supported configurations (verified:
all 48 CMake configurations compile), so none can be triggered at run time.
The Rust mirrors them as `const` assertions / `#[cfg]` selection in
`src/params.rs`.
