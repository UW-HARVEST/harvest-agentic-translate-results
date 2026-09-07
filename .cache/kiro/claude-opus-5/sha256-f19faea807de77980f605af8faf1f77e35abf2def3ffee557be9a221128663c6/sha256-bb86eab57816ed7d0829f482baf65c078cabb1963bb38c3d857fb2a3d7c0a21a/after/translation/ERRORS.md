# ERRORS.md — error-surface table

Derived mechanically from the C source.  The grep that produced the candidate
set (run inside `c_src/`):

```
grep -rn "return *-\|return *NULL\|assert\|abort\|RETURN_ERROR\|RNG_BAD\|exit(" \
     app/src lib/*/src app/include lib/*/include
```

Hits: `rng.c:33`, `rng.c:67`, `rng.c:69`, `rng.c:109`, `sign.c:180`,
`sign.c:236`, `sign.c:272`, `sign.c:280`, plus the `RNG_BAD_*` constants in
`rng.h:14-16`.  There are **no** `assert()`s, no error enums, and no
`RETURN_ERROR`-style macros anywhere in this tree.

Sentinel values (`app/include/rng.h`):
`RNG_SUCCESS = 0`, `RNG_BAD_MAXLEN = -1`, `RNG_BAD_OUTBUF = -2`,
`RNG_BAD_REQ_LEN = -3`.  `sign.c` uses a bare `-1` / `0`.

Rows 9–17 are the generic FFI boundaries the task requires even though the C
does not explicitly check them (they exercise "one step past the range",
out-of-range enum ints, and zero/oversized lengths).

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `crypto_sign_verify` | `siglen != SPX_BYTES` (`sign.c:179`) — e.g. `SPX_BYTES-1`, `SPX_BYTES+1`, `0` | returns `-1`, no output buffer written | [x] |
| 2 | `crypto_sign_verify` | signature/message/pk mismatch so that the recomputed root `memcmp(root, pub_root, SPX_N) != 0` (`sign.c:235`) — flip any bit of `sig`, `m`, or `pk` | returns `-1` | [x] |
| 3 | `crypto_sign_open` | `smlen < SPX_BYTES` (`sign.c:269`) — e.g. `0`, `1`, `SPX_BYTES-1` | `memset(m,0,smlen)`, `*mlen = 0`, returns `-1` | [x] |
| 4 | `crypto_sign_open` | `smlen >= SPX_BYTES` but inner `crypto_sign_verify` fails (`sign.c:277`) — corrupt any byte of `sm` or `pk` | `memset(m,0,smlen)` (**note: `smlen`, not `*mlen`**), `*mlen = 0`, returns `-1` | [x] |
| 5 | `seedexpander_init` | `maxlen >= 0x100000000` (`rng.c:32`) | returns `RNG_BAD_MAXLEN` (`-1`); `ctx` left untouched | [x] |
| 6 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted value, one below the check) | returns `0`; `ctx->length_remaining = 0xFFFFFFFF`, `ctr[8..12] = FF FF FF FF`, `ctr[12..16] = 0`, `buffer_pos = 16` | [x] |
| 7 | `seedexpander` | `x == NULL` (`rng.c:66`) | returns `RNG_BAD_OUTBUF` (`-2`); `ctx` unmodified | [x] |
| 8 | `seedexpander` | `xlen >= ctx->length_remaining` (`rng.c:68`) — **note `>=`, so requesting exactly the remaining length is rejected** | returns `RNG_BAD_REQ_LEN` (`-3`); `ctx` unmodified | [x] |
| 9 | `seedexpander` | `xlen == ctx->length_remaining - 1` (largest accepted) | returns `0`, fills `xlen` bytes, `length_remaining` decremented | [x] |
| 10 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | returns `0` immediately (the `while (xlen > 0)` loop body never runs), nothing written | [x] |
| 11 | `seedexpander_init` + `seedexpander` | `maxlen == 0` then any `xlen` (`xlen >= 0 == length_remaining`) | every `seedexpander` call returns `RNG_BAD_REQ_LEN` | [x] |
| 12 | `randombytes` (rng.c / DRBG) | `xlen == 0` | returns `RNG_SUCCESS` (`0`); loop skipped but `AES256_CTR_DRBG_Update(NULL,…)` still runs and `reseed_counter` still increments — i.e. **state advances even for a zero-length request** | [x] |
| 13 | `randombytes_init` | `personalization_string == NULL` (`rng.c:141`) | the XOR loop is skipped; only `entropy_input` seeds the DRBG | [x] |
| 14 | `randombytes_init` | `personalization_string != NULL` | all 48 bytes XORed into `seed_material` | [x] |
| 15 | `set_type` / `SPX_set_type` | `type` outside `{0..6}` (`SPX_ADDR_TYPE_*`); C enums/`uint32_t` accept any `int` — e.g. `7`, `255`, `256`, `0xFFFFFFFF` | no validation: writes `(unsigned char)type` to `addr[SPX_OFFSET_TYPE]`, i.e. **truncates mod 256** | [x] |
| 16 | `set_layer_addr`, `set_chain_addr`, `set_hash_addr`, `set_tree_height` | argument `> 255` (e.g. `256`, `0xFFFFFFFF`) | no validation: `(unsigned char)` truncation mod 256 | [x] |
| 17 | `ull_to_bytes` | `outlen == 0` | the `for (i = outlen-1; i >= 0; …)` loop with `(signed int)outlen-1 == -1` never executes → nothing written (no out-of-bounds write) | [x] |
| 18 | `bytes_to_ull` | `inlen == 0` | returns `0` | [x] |
| 19 | `bytes_to_ull` | `inlen > 8` (e.g. `9`, `16`) — shift `8*(inlen-1-i)` exceeds 63 | C UB in principle; in the built `.so` the high bytes are folded by the x86 shift semantics.  The Rust must reproduce whatever the compiled C does, byte for byte. | [x] |
| 20 | `crypto_sign` / `crypto_sign_signature` | `mlen == 0` | succeeds; `*siglen = SPX_BYTES`, `*smlen = SPX_BYTES` | [x] |
| 21 | `crypto_sign_open` | `smlen == SPX_BYTES` exactly (empty message) | `*mlen = 0`; returns `0` for a valid signature over the empty message | [x] |
| 22 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` (`rng.c:196`) | the 48-byte XOR is skipped; `Key`/`V` come straight from the three AES-ECB blocks | [x] |

Notes on things that are **not** rows:

* `rng.c:109` `abort()` inside `handleErrors()` is only reachable if OpenSSL's
  EVP layer fails to allocate/initialise AES-256-ECB.  Not triggerable from the
  public API with valid arguments, and the Rust port uses the `aes` crate, which
  has no corresponding failure mode.  Excluded deliberately.
* `crypto_sign_seed_keypair`, `crypto_sign_keypair`, `crypto_sign_signature`,
  `crypto_sign` have **no** error branches at all — they unconditionally
  `return 0`.  Covered as configuration rows in `CONFIGS.md`, not here.
* `set_tree_addr` / `set_keypair_addr` / `set_tree_index` write full 8- resp.
  4-byte big-endian fields, so there is no truncation boundary to test beyond
  the full range (covered by randomized inputs in Phase B).

## Where each row is tested

`translation/tests/err_paths.rs` (18 tests covering the 22 rows):

| rows | test |
|---|---|
| 1 | `row01_verify_wrong_siglen` |
| 2 | `row02_verify_root_mismatch` |
| 3 | `row03_open_smlen_too_small` |
| 4 | `row04_open_bad_signature` |
| 5 | `row05_seedexpander_init_maxlen_too_large` |
| 6 | `row06_seedexpander_init_maxlen_boundary` |
| 7 | `row07_seedexpander_null_output` |
| 8, 9 | `row08_row09_seedexpander_req_len` |
| 10 | `row10_seedexpander_zero_len` |
| 11 | `row11_seedexpander_maxlen_zero` |
| 12, 13, 14 | `row12_row13_row14_randombytes` (`rand_drbg`) / `diff_f_urandom.rs` (`rand_urandom`) |
| 15, 16 | `row15_row16_out_of_range_address_fields`, `row15_out_of_range_type_through_prf_and_thash` |
| 17 | `row17_ull_to_bytes_zero_outlen` |
| 18, 19 | `row18_row19_bytes_to_ull_degenerate` |
| 20 | `row20_zero_length_message` |
| 21 | `row21_open_empty_message` |
| 22 | `row22_drbg_update_null_provided_data` |

Result: **22/22 rows pass in all 96 feature combinations**
(4 backends × 2 THASH × 6 SECPAR × {DRBG, urandom}).

## A C quirk found while testing row 2

`row02` initially failed the "a corrupted message must be rejected" expectation
— **and both implementations agreed**.  The cause is in the C:

* `lib/blake/src/blake256.c`'s `blake256_update(S, data, datalen)` takes
  `datalen` in **bits** (`blake256()` itself calls it as
  `blake256_update(&S, in, inlen*8)`).
* `lib/blake/src/hash_blake.c`'s `hash_message()` and `gen_message_random()`
  call `blakeX_update(&S, m, mlen)` with `mlen` in **bytes**.

So under the blake backend only `mlen / 8` message bytes (and 2 of `R`'s
`SPX_N` bytes, and 4 of `pk`'s `SPX_PK_BYTES`) reach the digest.  This is a real
weakness of the C, not a translation defect, and the Rust reproduces it exactly.
The test now demands rejection only for the bytes the C actually observes, and
asserts C/Rust agreement for the bytes it does not.  `sha2`, `shake` and
`haraka` read the whole message, and the test enforces that for them.
