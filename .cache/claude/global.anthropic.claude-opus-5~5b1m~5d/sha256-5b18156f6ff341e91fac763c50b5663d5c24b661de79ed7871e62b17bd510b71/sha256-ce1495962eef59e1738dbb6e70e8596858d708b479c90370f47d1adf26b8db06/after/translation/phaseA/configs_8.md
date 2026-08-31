## Area 8 — crypto_pwhash + crypto_ipcrypt

Configuration surface of **valid** (non-rejecting) inputs for
`crypto_pwhash/crypto_pwhash.c`, `crypto_pwhash/argon2/*`,
`crypto_pwhash/scryptsalsa208sha256/*` (incl. `nosse/`), and
`crypto_ipcrypt/{crypto_ipcrypt.c, ipcrypt_soft.c}`.

Axes extracted from the source:

* **alg**: `crypto_pwhash_ALG_ARGON2I13` (1) vs `crypto_pwhash_ALG_ARGON2ID13` (2);
  `crypto_pwhash_ALG_DEFAULT` is an alias of `ARGON2ID13`. Internally `argon2_type` ∈
  {`Argon2_i`=1, `Argon2_id`=2} (Argon2_d is not compiled in).
* **entry-point layer**: high-level `crypto_pwhash*` / `crypto_pwhash_argon2i*` /
  `crypto_pwhash_argon2id*` (which hard-wire `lanes = threads = 1`,
  `saltlen = 16`, `STR_HASHBYTES = 32`) **and** the low-level `argon2_ctx`, `argon2_hash`,
  `argon2i_hash_raw`, `argon2id_hash_raw`, `argon2i_hash_encoded`, `argon2id_hash_encoded`,
  `argon2_verify`, `argon2i_verify`, `argon2id_verify`, `argon2_encode_string`,
  `argon2_decode_string` (which expose lanes/threads/secret/ad/flags).
* **lanes / threads**: `lanes ∈ {1, 2, 4, …}`, `threads` independent of `lanes`
  (`argon2_fill_memory_blocks` is single-threaded regardless — `threads` only affects validation,
  never the output).
* **m_cost**: at `8 * lanes` (the effective minimum), just above it, and moderate values;
  `segment_length = m_cost / (lanes * 4)` and `m_cost` is then re-rounded down to
  `segment_length * lanes * 4`, so several distinct `m_cost` values collapse to identical work.
* **t_cost**: 1 (single pass, `fill_block`), 2 and 3 (extra passes take the
  `fill_block_with_xor` + `pass != 0` branch of `index_alpha`).
* **outlen**: `ARGON2_MIN_OUTLEN` = 16, typical 24/32, `64` (= `blake2b_BYTES_MAX`, last value on
  the short `blake2b_long` path), `65`/`128`/`1024` (long path with the 32-byte-per-iteration loop).
* **pwd / salt / secret / ad**: present vs absent (`NULL` + len 0) and at their minimum lengths
  (`pwd` min 0, `salt` min 8, `secret` min 0, `ad` min 0).
* **flags**: `ARGON2_DEFAULT_FLAGS`, `ARGON2_FLAG_CLEAR_PASSWORD`, `ARGON2_FLAG_CLEAR_SECRET`, both.
* **scrypt**: `N ∈ {2, 16, 512, 1024, 16384}`, `r ∈ {1, 8}`, `p ∈ {1, 2, 512}`; the two branches of
  `pickparams` (`opslimit < memlimit/32` vs not); the `$7$` setting round trip.
* **ipcrypt**: deterministic / ND / NDX / PFX, encrypt and decrypt, IPv4-mapped vs pure IPv6
  16-byte inputs, distinct vs identical key halves.

**Speed note:** every argon2 row below uses `m_cost` ≤ 64 KiB (`memlimit` ≤ 65536) and
`t_cost` ≤ 3 unless the row explicitly says otherwise, and every scrypt row uses `N ≤ 16384, r ≤ 8,
p ≤ 2` unless stated, so the whole table runs in well under a second per row. Rows marked
**(SLOW — optional)** use the documented INTERACTIVE/MODERATE/SENSITIVE presets and should be run at
most once each, or skipped in fast CI.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 8.1 | `crypto_pwhash_alg_argon2i13`, `crypto_pwhash_alg_argon2id13`, `crypto_pwhash_alg_default` | no input; must return 1, 2, 2 respectively (`ALG_DEFAULT == ALG_ARGON2ID13`) | [x] |
| 8.2 | `crypto_pwhash_bytes_min`/`_max`, `_passwd_min`/`_max`, `_saltbytes`, `_strbytes`, `_strprefix`, `_primitive` | no input; 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2id$"` / `"argon2id,argon2i"` | [x] |
| 8.3 | `crypto_pwhash_opslimit_min`/`_max`/`_interactive`/`_moderate`/`_sensitive`, `crypto_pwhash_memlimit_min`/`_max`/`_interactive`/`_moderate`/`_sensitive` | no input; 1 / 4294967295 / 2 / 3 / 4 and 8192 / 4398046510080 / 67108864 / 268435456 / 1073741824 (all alias the argon2id values) | [x] |
| 8.4 | `crypto_pwhash_argon2i_*` constant getters (alg, bytes_min/max, passwd_min/max, saltbytes, strbytes, strprefix, opslimit_min/max/interactive/moderate/sensitive, memlimit_min/max/interactive/moderate/sensitive) | no input; 1 / 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2i$"` / 3,4294967295,4,6,8 / 8192,4398046510080,33554432,134217728,536870912 | [x] |
| 8.5 | `crypto_pwhash_argon2id_*` constant getters (same list) | no input; 2 / 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2id$"` / 1,4294967295,2,3,4 / 8192,4398046510080,67108864,268435456,1073741824 | [x] |
| 8.6 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3` (`argon2i_OPSLIMIT_MIN`), `memlimit = 8192` (`MEMLIMIT_MIN` → `m_cost = 8`), `outlen = 16` (`BYTES_MIN`), `passwd = "test"` (len 4), 16-byte salt; fast | [x] |
| 8.7 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 8192`, `outlen = 32` | [x] |
| 8.8 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 4`, `memlimit = 16384` (`m_cost = 16`), `outlen = 64` (last short `blake2b_long` size) | [x] |
| 8.9 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 65536` (`m_cost = 64`), `outlen = 65` (first long `blake2b_long` size) | [x] |
| 8.10 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 1` (`argon2id_OPSLIMIT_MIN`), `memlimit = 8192`, `outlen = 16` | [x] |
| 8.11 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 2`, `memlimit = 8192`, `outlen = 32` (two passes: pass 1 is fully data-dependent) | [x] |
| 8.12 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 3`, `memlimit = 32768` (`m_cost = 32`), `outlen = 32` | [x] |
| 8.13 | `crypto_pwhash` | `alg = ALG_DEFAULT`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16`; output must be byte-identical to row 8.10 (`ALG_DEFAULT == ARGON2ID13`) | [x] |
| 8.14 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `outlen = 128` and `outlen = 1024` (multi-iteration long `blake2b_long` path), `opslimit = 1`, `memlimit = 8192` | [x] |
| 8.15 | `crypto_pwhash` | `passwdlen = 0` with a non-NULL `passwd` pointer (`PASSWD_MIN` is 0), `alg = ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16` | [x] |
| 8.16 | `crypto_pwhash` | `passwdlen = 1`; then a long password (e.g. 256 bytes, and one > 128 bytes containing NUL and 0xFF bytes — password is binary, not a C string) | [x] |
| 8.17 | `crypto_pwhash` | `memlimit` not a multiple of 1024: `memlimit = 8192 + 512` and `9215` both truncate to `m_cost = 8` → output identical to row 8.10 | [x] |
| 8.18 | `crypto_pwhash` | salt values: all-zero 16 bytes, all-0xFF 16 bytes, random; salt length is fixed at `crypto_pwhash_SALTBYTES` = 16 by the wrapper | [x] |
| 8.19 | `crypto_pwhash` **(SLOW — optional)** | `alg = ALG_ARGON2ID13`, `opslimit = crypto_pwhash_OPSLIMIT_INTERACTIVE` (2), `memlimit = crypto_pwhash_MEMLIMIT_INTERACTIVE` (67108864 → `m_cost = 65536`), `outlen = 32` — the documented interactive preset; ~64 MiB allocation | [x] |
| 8.20 | `crypto_pwhash` **(SLOW — optional)** | `alg = ALG_ARGON2I13`, `opslimit = crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE` (4), `memlimit = 33554432` (`m_cost = 32768`), `outlen = 32` | [x] |
| 8.21 | `crypto_pwhash_argon2i` (direct) | `alg = crypto_pwhash_argon2i_ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 8192`, `outlen = 16`; must equal `crypto_pwhash` with `ALG_ARGON2I13` (row 8.6 shape) | [x] |
| 8.22 | `crypto_pwhash_argon2id` (direct) | `alg = crypto_pwhash_argon2id_ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16`; must equal row 8.10 | [x] |
| 8.23 | `crypto_pwhash_str` | `passwd = "password"`, `opslimit = crypto_pwhash_OPSLIMIT_MIN` (1), `memlimit = crypto_pwhash_MEMLIMIT_MIN` (8192) — deliberately minimal so the test is fast. Output: 128-byte buffer, `"$argon2id$v=19$m=8,t=1,p=1$"` + 22 base64 chars + `"$"` + 43 base64 chars + NUL, all remaining bytes 0 | [x] |
| 8.24 | `crypto_pwhash_str` + `crypto_pwhash_str_verify` | round trip at (`opslimit = 1`, `memlimit = 8192`) with the same password → 0; each call produces a *different* string (random 16-byte salt) yet both verify | [x] |
| 8.25 | `crypto_pwhash_str` + `crypto_pwhash_str_verify` | `passwdlen = 0` round trip (empty password is legal) | [x] |
| 8.26 | `crypto_pwhash_str_alg` | `alg = ALG_ARGON2I13`, `opslimit = 3` (`argon2i` min), `memlimit = 8192` → string starts with `"$argon2i$v=19$m=8,t=3,p=1$"`; verify with `crypto_pwhash_str_verify` (prefix dispatch) → 0 | [x] |
| 8.27 | `crypto_pwhash_str_alg` | `alg = ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192` → string starts with `"$argon2id$v=19$m=8,t=1,p=1$"`; identical to `crypto_pwhash_str` behaviour | [x] |
| 8.28 | `crypto_pwhash_str_alg` **(SLOW — optional)** | `alg = ALG_ARGON2ID13`, `opslimit = OPSLIMIT_INTERACTIVE` (2), `memlimit = MEMLIMIT_INTERACTIVE` (67108864) → `"...m=65536,t=2,p=1..."` | [x] |
| 8.29 | `crypto_pwhash_argon2i_str` / `crypto_pwhash_argon2i_str_verify` | direct argon2i round trip at (3, 8192); the produced string must also be accepted by the generic `crypto_pwhash_str_verify` | [x] |
| 8.30 | `crypto_pwhash_argon2id_str` / `crypto_pwhash_argon2id_str_verify` | direct argon2id round trip at (1, 8192) | [x] |
| 8.31 | `crypto_pwhash_str_needs_rehash` | argon2id string produced at (1, 8192), queried with the same `(opslimit = 1, memlimit = 8192)` → `0` | [x] |
| 8.32 | `crypto_pwhash_str_needs_rehash` | same string, queried with a different `opslimit` (2) → `1`; and with a different `memlimit` (16384) → `1` | [x] |
| 8.33 | `crypto_pwhash_str_needs_rehash` | same string, `memlimit = 8192 + 1023` (truncating division by 1024 → `m_cost = 8`) → `0`; documents the truncation semantics | [x] |
| 8.34 | `crypto_pwhash_str_needs_rehash` | argon2**i** string (prefix dispatch to `crypto_pwhash_argon2i_str_needs_rehash`) produced at (3, 8192), queried with (3, 8192) → `0`, with (4, 8192) → `1` | [x] |
| 8.35 | `crypto_pwhash_str_needs_rehash` | hand-written `"$argon2id$v=19$m=8,t=1,p=2$<22b64>$<43b64>"` queried with (1, 8192) → `0` even though `p` differs: **`lanes`/`p` and the type are not compared** — quirk to preserve | [x] |
| 8.36 | `crypto_pwhash_argon2i_str_needs_rehash`, `crypto_pwhash_argon2id_str_needs_rehash` | boundary: `strlen(str) == 127` (max accepted, `< crypto_pwhash_STRBYTES`) with an otherwise valid string | [x] |
| 8.37 | `argon2_ctx` | `type = Argon2_i`, minimal legal context: `out`/`outlen = 16`, `pwd = NULL, pwdlen = 0`, `salt` 8 bytes / `saltlen = 8` (`ARGON2_MIN_SALT_LENGTH`), `secret = NULL, secretlen = 0`, `ad = NULL, adlen = 0`, `t_cost = 1`, `m_cost = 8` (= `8*lanes`), `lanes = 1`, `threads = 1`, `flags = ARGON2_DEFAULT_FLAGS` → `ARGON2_OK` | [x] |
| 8.38 | `argon2_ctx` | same as 8.37 but `type = Argon2_id` → different digest, `ARGON2_OK` | [x] |
| 8.39 | `argon2_ctx` | `lanes = 2`, `threads = 2`, `m_cost = 16` (= `8*lanes`, the minimum for 2 lanes), `t_cost = 1`, `outlen = 32`, both types | [x] |
| 8.40 | `argon2_ctx` | `lanes = 4`, `threads = 4`, `m_cost = 32` (= `8*lanes`), `t_cost = 1`, `outlen = 32`, both types (exercises the multi-lane XOR in `argon2_finalize`) | [x] |
| 8.41 | `argon2_ctx` | `lanes = 2`, `threads = 1` (threads < lanes is legal); output must be identical to `lanes = 2, threads = 2` — `threads` never affects the digest | [x] |
| 8.42 | `argon2_ctx` | `lanes = 1`, `threads = 4` (threads > lanes is legal, `threads <= ARGON2_MAX_THREADS`); output identical to `threads = 1` | [x] |
| 8.43 | `argon2_ctx` | `lanes = ARGON2_MAX_LANES` boundary check only via validation-adjacent config: `lanes = 8`, `threads = 8`, `m_cost = 64` | [x] |
| 8.44 | `argon2_ctx` | m_cost just above the minimum with `lanes = 1`: `m_cost = 9, 10, 11` all round down to `segment_length = 2` → identical digests to `m_cost = 8`; `m_cost = 12` gives `segment_length = 3` (different digest) | [x] |
| 8.45 | `argon2_ctx` | m_cost rounding with `lanes = 2`: `m_cost = 16..23` → `segment_length = 2` (identical digests); `m_cost = 24` → `segment_length = 3` | [x] |
| 8.46 | `argon2_ctx` | moderate m_cost: `m_cost = 512` (`lanes = 1` → `segment_length = 128 == ARGON2_ADDRESSES_IN_BLOCK`) and `m_cost = 1024` (`segment_length = 256`, forces a second address block in `generate_addresses`) | [x] |
| 8.47 | `argon2_ctx` | `t_cost = 1` (single pass; `fill_block` only) | [x] |
| 8.48 | `argon2_ctx` | `t_cost = 2` (second pass takes `fill_block_with_xor` and the `pass != 0` `index_alpha` branch) | [x] |
| 8.49 | `argon2_ctx` | `t_cost = 3` | [x] |
| 8.50 | `argon2_ctx` | `type = Argon2_id, t_cost = 1`: slices 0–1 data-independent (`generate_addresses`), slices 2–3 data-dependent; `type = Argon2_id, t_cost = 2`: pass 1 fully data-dependent | [x] |
| 8.51 | `argon2_ctx` | `type = Argon2_i, t_cost = 2`: **all** passes/slices data-independent | [x] |
| 8.52 | `argon2_ctx` | `outlen = 16` (MIN), `24`, `32`, `48`, `64` (short `blake2b_long` path); `outlen = 65`, `96`, `128`, `1024` (long path) — all with `t_cost = 1, m_cost = 8, lanes = 1` | [x] |
| 8.53 | `argon2_ctx` | `saltlen = 8` (min), `16` (libsodium's `SALTBYTES`), `32`, `64`; salt all-zero and random | [x] |
| 8.54 | `argon2_ctx` | `pwd` absent (`NULL`, `pwdlen = 0`) vs present with `pwdlen = 0` (non-NULL pointer) vs `pwdlen = 1` vs `pwdlen = 64` — the first two must give the same digest (only `pwdlen` is hashed) | [x] |
| 8.55 | `argon2_ctx` | `secret` absent (`NULL, 0`) vs present with `secretlen = 0` (non-NULL, min is 0) vs `secretlen = 8`, `16`, `32` — keyed argon2; digest differs from the unkeyed case for `secretlen > 0` | [x] |
| 8.56 | `argon2_ctx` | `ad` absent (`NULL, 0`) vs present with `adlen = 0` vs `adlen = 8`, `16`, `64` | [x] |
| 8.57 | `argon2_ctx` | both `secret` (16 bytes) and `ad` (16 bytes) present, `type = Argon2_id`, `t_cost = 2`, `m_cost = 16`, `lanes = 2` | [x] |
| 8.58 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_PASSWORD` with `pwd` non-NULL, `pwdlen = 16`: after the call `pwd` is all-zero and `context->pwdlen == 0`; digest identical to the `ARGON2_DEFAULT_FLAGS` run | [x] |
| 8.59 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_SECRET` with `secret` non-NULL, `secretlen = 16`: after the call `secret` is zeroed and `context->secretlen == 0` | [x] |
| 8.60 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_PASSWORD \| ARGON2_FLAG_CLEAR_SECRET` (both cleared) | [x] |
| 8.61 | `argon2_hash` | `hash != NULL, encoded == NULL, encodedlen = 0` (raw-only mode), `t_cost = 1, m_cost = 8, parallelism = 1, hashlen = 16, saltlen = 8`; identical to `argon2i_hash_raw` | [x] |
| 8.62 | `argon2_hash` | `hash == NULL, encoded != NULL, encodedlen = 128` (encoded-only mode); identical to `argon2i_hash_encoded` | [x] |
| 8.63 | `argon2_hash` | **both** `hash != NULL` and `encoded != NULL, encodedlen = 128` — raw digest and encoded string written in one call and must agree | [x] |
| 8.64 | `argon2_hash` | `encoded != NULL` but `encodedlen = 0` → the `if (encoded && encodedlen)` guard skips encoding; returns `ARGON2_OK` with `encoded` untouched | [x] |
| 8.65 | `argon2_hash` | `hash == NULL` and `encoded == NULL` (both outputs suppressed) → still returns `ARGON2_OK` after doing the full KDF | [x] |
| 8.66 | `argon2_hash` | `parallelism = 2` with `m_cost = 16`, and `parallelism = 4` with `m_cost = 32` — note `argon2_hash` sets `lanes = threads = parallelism` | [x] |
| 8.67 | `argon2i_hash_raw` | `t_cost = 1, m_cost = 8, parallelism = 1, pwdlen = 0..32, saltlen = 8/16, hashlen = 16/32/64` | [x] |
| 8.68 | `argon2i_hash_raw` | `t_cost = 2, m_cost = 32, parallelism = 4` (`m_cost == 8*parallelism` boundary), `hashlen = 32` | [x] |
| 8.69 | `argon2id_hash_raw` | same matrix as 8.67 and 8.68 with `Argon2_id`; digests must differ from the argon2i ones for identical parameters | [x] |
| 8.70 | `argon2i_hash_encoded` | `t_cost = 1, m_cost = 8, parallelism = 1, saltlen = 8, hashlen = 16, encodedlen = 128` → `"$argon2i$v=19$m=8,t=1,p=1$<11 b64>$<22 b64>"`; also `encodedlen` exactly equal to `strlen(result)+1` | [x] |
| 8.71 | `argon2id_hash_encoded` | `t_cost = 1, m_cost = 8, parallelism = 1, saltlen = 16, hashlen = 32, encodedlen = 128` → `"$argon2id$v=19$m=8,t=1,p=1$<22 b64>$<43 b64>"` | [x] |
| 8.72 | `argon2id_hash_encoded` | `parallelism = 2, m_cost = 16, t_cost = 2, saltlen = 16, hashlen = 32` → `"...m=16,t=2,p=2..."` | [x] |
| 8.73 | `argon2i_verify` / `argon2id_verify` | round trip against the strings from 8.70–8.72 with the correct password → `ARGON2_OK`; `pwdlen = 0` case included | [x] |
| 8.74 | `argon2_verify` (generic) | `type = Argon2_i` and `type = Argon2_id` explicitly, against a matching encoded string, `hashlen` 16 and 64 variants | [x] |
| 8.75 | `argon2_verify` | encoded string with `p=2`: verification recomputes with `ctx.threads = ctx.lanes = 2` (`argon2_decode_string` copies `lanes` into `threads`) → `ARGON2_OK` | [x] |
| 8.76 | `argon2_encode_string` | `type = Argon2_i`, `m_cost = 8, t_cost = 1, lanes = 1, saltlen = 8, outlen = 16`, `dst_len = 128` → exact expected string; `argon2_decode_string` of it returns all four parameters unchanged | [x] |
| 8.77 | `argon2_encode_string` | `type = Argon2_id`, `m_cost = 65536, t_cost = 3, lanes = 1, saltlen = 16, outlen = 32`, `dst_len = 128` | [x] |
| 8.78 | `argon2_encode_string` | `type = Argon2_id`, `m_cost = 4294967295, t_cost = 4294967295, lanes = 16777215` (max-width decimal fields, 10+10+8 digits), `saltlen = 16`, `outlen = 32`, `dst_len = 128` → still fits (≈118 bytes); round-trips through `argon2_decode_string` | [x] |
| 8.79 | `argon2_encode_string` | `dst_len` exactly `strlen(expected) + 1` (tightest accepting size) for both types | [x] |
| 8.80 | `argon2_encode_string` | salt/out byte patterns that exercise the non-URL-safe Base64 alphabet: bytes producing `'+'` and `'/'` characters, and no `'='` padding (`sodium_base64_VARIANT_ORIGINAL_NO_PADDING`) | [x] |
| 8.81 | `argon2_encode_string` / `argon2_decode_string` | `saltlen % 3` ∈ {0, 1, 2}: `saltlen = 9` (12 b64 chars, no leftover bits), `saltlen = 16` (22 chars, 2 leftover bits), `saltlen = 8` (11 chars, 4 leftover bits) — all must round trip | [x] |
| 8.82 | `argon2_decode_string` | `"$argon2i$v=19$m=8,t=1,p=1$<salt b64>$<hash b64>"` with `ctx.saltlen`/`ctx.outlen` set to buffer capacities ≥ the encoded sizes → `ARGON2_OK`, and `ctx.threads == ctx.lanes` afterwards | [x] |
| 8.83 | `argon2_decode_string` | `maxsaltlen` and `maxoutlen` set *exactly* to the decoded sizes (8 and 16) — tightest accepting capacity | [x] |
| 8.84 | `argon2_decode_string` | `"$argon2id$v=19$m=65536,t=2,p=4$…"` (multi-digit m, p > 1); and `"m=8,t=1,p=1"` with a bare `0`-free minimal decimal for each field | [x] |
| 8.85 | `argon2_decode_string` | version field is mandatory and must be exactly `v=19`; confirm `"$argon2id$v=19$…"` succeeds (the `CC_opt` optional-version macro is dead code in this fork) | [x] |
| 8.86 | `argon2_decode_string` → `argon2_encode_string` | decode then re-encode a canonical string and compare byte-for-byte (canonical-form round trip), both types | [x] |
| 8.87 | `blake2b_long` | `outlen = 16, 32, 64` (short path, `crypto_generichash_blake2b_init` directly) with `inlen = ARGON2_BLOCK_SIZE` (1024) | [x] |
| 8.88 | `blake2b_long` | `outlen = 65` (first long-path size), `128`, `1024` (`ARGON2_BLOCK_SIZE`, as used by `argon2_fill_first_blocks`), `outlen` not a multiple of 32 (e.g. 100) to exercise the final partial `toproduce` block | [x] |
| 8.89 | `argon2_fill_segment_ref` | driven through `argon2_ctx`: `(pass = 0, slice = 0)` with `starting_index = 2`; `(pass = 0, slice > 0)`; `(pass > 0, any slice)`; `Argon2_id` with `slice < 2` vs `slice >= 2` — all four `data_independent_addressing`/`starting_index` combinations | [x] |
| 8.90 | `_crypto_pwhash_argon2_pick_best_implementation` | call it; returns 0 and (with no SIMD macros) leaves `fill_segment = argon2_fill_segment_ref`; digests before and after the call must be identical | [x] |
| 8.91 | `crypto_pwhash_scryptsalsa208sha256_*` constant getters | `bytes_min`/`_max` = 16 / 137438953440, `passwd_min`/`_max` = 0 / `SIZE_MAX`, `saltbytes` = 32, `strbytes` = 102, `strprefix` = `"$7$"`, `opslimit_min`/`_max`/`_interactive`/`_sensitive` = 32768 / 4294967295 / 524288 / 33554432, `memlimit_min`/`_max`/`_interactive`/`_sensitive` = 16777216 / 68719476736 / 16777216 / 1073741824 | [x] |
| 8.92 | `crypto_pwhash_scryptsalsa208sha256_ll` | smallest legal parameter set: `N = 2, r = 1, p = 1`, `passwdlen = 0`, `saltlen = 0`, `buflen = 16`; very fast | [x] |
| 8.93 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16, r = 1, p = 1`, `buflen = 32` (the classic scrypt test vector shape `N=16,r=1,p=1` with empty password and salt) | [x] |
| 8.94 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 1, p = 1`, `buflen = 64` | [x] |
| 8.95 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 8, p = 1`, `buflen = 64` (the params `pickparams` yields at OPSLIMIT_MIN/MEMLIMIT_MIN) | [x] |
| 8.96 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 8, p = 2` (`p > 1` → the `for (i = 0; i < p; i++) smix(...)` loop runs twice) | [x] |
| 8.97 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16384, r = 8, p = 1`, `buflen = 64` (the INTERACTIVE params); ~16 MiB, still fast | [x] |
| 8.98 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16, r = 8, p = 16` (`r*p = 128`, well under 2^30) — many-p shape | [x] |
| 8.99 | `crypto_pwhash_scryptsalsa208sha256_ll` | `buflen` variations at fixed `N = 16, r = 1, p = 1`: `1`, `16`, `31`, `32`, `33`, `64`, `100` (non-multiple of 32 exercises the partial `clen` copy in `escrypt_PBKDF2_SHA256`) | [x] |
| 8.100 | `crypto_pwhash_scryptsalsa208sha256_ll` | `saltlen` variations: `0`, `1`, `32` (`SALTBYTES`), `64`; `passwdlen` variations: `0`, `1`, `64`; binary password/salt containing NUL bytes | [x] |
| 8.101 | `crypto_pwhash_scryptsalsa208sha256_ll` | repeated calls on the same `escrypt_local_t`-free API path — verify the local region is allocated and freed each call (`escrypt_init_local` → `escrypt_kdf_nosse` → `escrypt_free_local`) and results are reproducible | [x] |
| 8.102 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = OPSLIMIT_MIN` (32768), `memlimit = MEMLIMIT_MIN` (16777216): `pickparams` takes the **first** branch (`opslimit < memlimit/32` → 32768 < 524288) → `r = 8, p = 1, N_log2 = 10` (`N = 1024`); `outlen = 32`, 32-byte salt. Fast | [x] |
| 8.103 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = OPSLIMIT_INTERACTIVE` (524288), `memlimit = MEMLIMIT_INTERACTIVE` (16777216): `opslimit < memlimit/32` is **false** (equal) → **second** branch → `N_log2 = 14` (`N = 16384`), `r = 8`, `maxrp = 8`, `p = 1` | [x] |
| 8.104 | `crypto_pwhash_scryptsalsa208sha256` | second branch with `p > 1`: `opslimit = 32768`, `memlimit = 524288` → `maxN = 512`, `N_log2 = 9` (`N = 512`), `maxrp = 16`, `p = 2` | [x] |
| 8.105 | `crypto_pwhash_scryptsalsa208sha256` | degenerate-but-legal: `memlimit = 0` → second branch, `maxN = 0`, `N_log2 = 1` (`N = 2`), `r = 8`, `p = 512`; returns 0 (no minimum is enforced — see errors row 8.154) | [x] |
| 8.106 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = 0` (clamped to 32768 inside `pickparams`) with `memlimit = 16777216` → identical output to row 8.102 | [x] |
| 8.107 | `crypto_pwhash_scryptsalsa208sha256` | `outlen = 16` (`BYTES_MIN`), `32`, `64`, `100`; `passwdlen = 0`; salt is always exactly `SALTBYTES` = 32 bytes | [x] |
| 8.108 | `crypto_pwhash_scryptsalsa208sha256` **(SLOW — optional)** | `opslimit = OPSLIMIT_SENSITIVE` (33554432), `memlimit = MEMLIMIT_SENSITIVE` (1073741824) — 1 GiB; run at most once | [x] |
| 8.109 | `crypto_pwhash_scryptsalsa208sha256_str` | `opslimit = 32768, memlimit = 16777216`, `passwd = "password"` → 102-byte buffer holding exactly 101 chars + NUL, starting with `"$7$"`; `escrypt_gensalt_r` uses a random 32-byte salt so every call differs | [x] |
| 8.110 | `crypto_pwhash_scryptsalsa208sha256_str` + `_str_verify` | round trip at (32768, 16777216) → `0`; also with `passwdlen = 0` | [x] |
| 8.111 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | string produced at (32768, 16777216) queried with the same pair → `0`; queried with (524288, 16777216) → `1` (different `N_log2`); queried with (32768, 524288) → `1` (different `N_log2` and `p`) | [x] |
| 8.112 | `escrypt_gensalt_r` + `escrypt_parse_setting` | round trip for `(N_log2, r, p)` = `(10, 8, 1)`, `(14, 8, 1)`, `(9, 8, 2)`, `(1, 1, 1)`, `(1, 8, 512)`, `(63, 1, 1)` (max `N_log2`), `(0, 1, 1)`; `src` = 32 bytes, `buflen = 58` (`= 14 + 43 + 1`) | [x] |
| 8.113 | `escrypt_gensalt_r` | `srclen` variations: `0` (`saltlen = 0`, `need = 15`), `1`, `16`, `32`; `buflen` exactly `need` (tightest accepting size) | [x] |
| 8.114 | `escrypt_gensalt_r` | `r * p` just under the limit: `r = 1, p = 1073741823` (`= 2^30 - 1`) → accepted by `gensalt` (rejected later by the KDF only if actually used) | [x] |
| 8.115 | `escrypt_parse_setting` | parse a real `crypto_pwhash_scryptsalsa208sha256_str` output; returns a pointer to the first salt char (`setting + 14`) and the correct `N_log2`, `r`, `p`; also parse a bare setting with no trailing `$hash` | [x] |
| 8.116 | `escrypt_r` | `setting` from `escrypt_gensalt_r(10, 8, 1, salt32)`, `buflen = 102` (`crypto_pwhash_scryptsalsa208sha256_STRBYTES`) → 101-char `$7$` string; result must equal `crypto_pwhash_scryptsalsa208sha256_str` with the same salt | [x] |
| 8.117 | `escrypt_r` | `setting` **with** a trailing `"$<hash>"` (i.e. an existing password string used as the setting, which is how `_str_verify` works): `strrchr(salt, '$')` bounds the salt, and the recomputed string must equal the input | [x] |
| 8.118 | `escrypt_r` | shorter salt in the setting (e.g. 16-byte salt → `saltlen = 22`): `need = 14 + 22 + 1 + 43 + 1 = 81 <= buflen` → accepted, output is 80 chars + NUL | [x] |
| 8.119 | `escrypt_init_local` / `escrypt_free_local` / `escrypt_alloc_region` / `escrypt_free_region` | init → alloc `size = 1024`, `65536`, `128*8*(1024+1)+256*8+64` → free → free again (idempotent after `init_region`); `region->aligned` is 64-byte aligned when the non-mmap path is used | [x] |
| 8.120 | `escrypt_PBKDF2_SHA256` | `c = 1` (the only value scrypt uses), `dkLen` ∈ {0, 1, 32, 33, 64, 100, 128}; `passwdlen`/`saltlen` ∈ {0, 1, 32}; known PBKDF2-HMAC-SHA256 vectors | [x] |
| 8.121 | `escrypt_PBKDF2_SHA256` | `c = 2` and `c = 4096` (exercises the inner U-chain loop that scrypt itself never uses) with `dkLen = 32` | [x] |
| 8.122 | `escrypt_kdf_nosse` (direct) | called directly with an `escrypt_local_t` reused across several calls with growing `need` (forces the `local->size < need` re-allocation branch) and shrinking `need` (region reused, no re-allocation) | [x] |
| 8.123 | `crypto_ipcrypt_bytes`, `_keybytes` | no input; 16, 16 | [x] |
| 8.124 | `crypto_ipcrypt_nd_keybytes`, `_nd_tweakbytes`, `_nd_inputbytes`, `_nd_outputbytes` | no input; 16, 8, 16, 24 (`OUTPUT == TWEAK + INPUT`) | [x] |
| 8.125 | `crypto_ipcrypt_ndx_keybytes`, `_ndx_tweakbytes`, `_ndx_inputbytes`, `_ndx_outputbytes` | no input; 32, 16, 16, 32 | [x] |
| 8.126 | `crypto_ipcrypt_pfx_keybytes`, `_pfx_bytes` | no input; 32, 16 | [x] |
| 8.127 | `crypto_ipcrypt_keygen`, `_nd_keygen`, `_ndx_keygen`, `_pfx_keygen` | fill 16/16/32/32 bytes; two successive calls differ (randomness), whole buffer written | [x] |
| 8.128 | `crypto_ipcrypt_encrypt` / `crypto_ipcrypt_decrypt` | 16-byte round trip with an all-zero key; input = all-zero 16 bytes; deterministic (same in/key → same out) | [x] |
| 8.129 | `crypto_ipcrypt_encrypt` / `_decrypt` | round trip with a random `crypto_ipcrypt_keygen` key; input = all-0xFF 16 bytes | [x] |
| 8.130 | `crypto_ipcrypt_encrypt` / `_decrypt` | IPv4-mapped input: `::ffff:192.0.2.1` = `00×10 ff ff c0 00 02 01`; note the deterministic variant is *not* format-preserving (output is an arbitrary 16-byte block) | [x] |
| 8.131 | `crypto_ipcrypt_encrypt` / `_decrypt` | IPv4-mapped edge addresses `::ffff:0.0.0.0` and `::ffff:255.255.255.255` | [x] |
| 8.132 | `crypto_ipcrypt_encrypt` / `_decrypt` | pure IPv6 inputs: `::` (all-zero), `::1`, `2001:db8::1`, `ffff:…:ffff` (all-0xFF) | [x] |
| 8.133 | `crypto_ipcrypt_encrypt` | fixed known-answer: it is plain AES-128 ECB on one block, so `crypto_ipcrypt_encrypt` with the FIPS-197 key/plaintext must give the FIPS-197 ciphertext; `crypto_ipcrypt_decrypt` the inverse | [x] |
| 8.134 | `crypto_ipcrypt_nd_encrypt` / `crypto_ipcrypt_nd_decrypt` | 16-byte input, 8-byte tweak (all-zero), 16-byte key (all-zero) → 24-byte output whose first 8 bytes equal the tweak; decrypt recovers the input | [x] |
| 8.135 | `crypto_ipcrypt_nd_encrypt` / `_nd_decrypt` | random 8-byte tweak, random key, IPv4-mapped and IPv6 inputs; two different tweaks over the same input give different ciphertext halves; the same tweak reproduces the same output (deterministic given the tweak) | [x] |
| 8.136 | `crypto_ipcrypt_nd_encrypt` | tweak edge values: all-zero, all-0xFF, and a tweak whose odd bytes are non-zero (`tweak_expand` packs `tweak[2i]` and `tweak[2i+1]` into the low 16 bits of each 32-bit word, so all 8 bytes matter) | [x] |
| 8.137 | `crypto_ipcrypt_nd_decrypt` | feed back a 24-byte buffer built by hand (`tweak ‖ ciphertext`) rather than by `nd_encrypt` — decryption depends only on `in[0..8)` as the tweak | [x] |
| 8.138 | `crypto_ipcrypt_ndx_encrypt` / `crypto_ipcrypt_ndx_decrypt` | 16-byte input, 16-byte tweak (all-zero), 32-byte key with **distinct** halves → 32-byte output whose first 16 bytes equal the tweak; decrypt recovers the input | [x] |
| 8.139 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | random 16-byte tweak, random `crypto_ipcrypt_ndx_keygen` key, IPv4-mapped and IPv6 inputs; different tweaks → different ciphertexts | [x] |
| 8.140 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | **degenerate key**: `k[0..16) == k[16..32)` (e.g. all-zero 32-byte key) → the `d == 0` fixup re-derives the data key as `k[i] ^ 0x5a`; encryption/decryption still round trip, and the result differs from a non-degenerate key | [x] |
| 8.141 | `crypto_ipcrypt_ndx_encrypt` | key halves differing in a single bit (non-degenerate, `d != 0`) → no fixup applied | [x] |
| 8.142 | `crypto_ipcrypt_pfx_encrypt` / `crypto_ipcrypt_pfx_decrypt` | IPv4-mapped input (`::ffff:192.0.2.1`) with a 32-byte key of distinct halves → output keeps the `00×10 ff ff` IPv4-mapped prefix (format-preserving: `prefix_start = 96`, `encrypted[10] = encrypted[11] = 0xff`), only the last 4 bytes are randomised; decrypt recovers the input | [x] |
| 8.143 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | pure IPv6 input (`2001:db8::1`) → `prefix_start = 0`, all 128 bits processed, `pfx_pad_prefix` uses the `padded_prefix[15] = 0x01` seed; round trip | [x] |
| 8.144 | `crypto_ipcrypt_pfx_encrypt` | prefix-preservation property: two IPv4-mapped addresses sharing a /24 (`::ffff:192.0.2.1`, `::ffff:192.0.2.99`) must produce ciphertexts sharing the same leading 24 bits of the 32-bit v4 part; two addresses differing in the first octet must not | [x] |
| 8.145 | `crypto_ipcrypt_pfx_encrypt` | prefix-preservation for IPv6: two addresses sharing a /64 produce ciphertexts sharing the first 64 bits | [x] |
| 8.146 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | **degenerate key** `k[0..16) == k[16..32)` → `k2` re-derived as `k[i] ^ 0x5a`; round trip still holds | [x] |
| 8.147 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | edge inputs: `::` (all-zero), all-0xFF, `::ffff:0.0.0.0`, `::ffff:255.255.255.255` | [x] |
| 8.148 | `_crypto_ipcrypt_pick_best_implementation` | call it; returns 0 and (with no `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H`) keeps `ipcrypt_soft_implementation`; all outputs identical before and after the call | [x] |
| 8.149 | `ipcrypt_soft_implementation` struct | all eight function pointers (`encrypt`, `decrypt`, `nd_encrypt`, `nd_decrypt`, `ndx_encrypt`, `ndx_decrypt`, `pfx_encrypt`, `pfx_decrypt`) are non-NULL and reachable through the `crypto_ipcrypt_*` wrappers | [x] |
| 8.150 | (adjacent, `sodium/codecs.c`) `sodium_ip2bin` / `sodium_bin2ip` | used to build/verify the 16-byte ipcrypt inputs from IP **strings**: `"192.0.2.1"` → IPv4-mapped 16 bytes, `"2001:db8::1"` → IPv6 16 bytes, `"::ffff:192.0.2.1"` → same bytes as `"192.0.2.1"`; `sodium_bin2ip` renders an IPv4-mapped block back in dotted-quad form. **libsodium 1.0.23 has no `crypto_ipcrypt_*_str` entry points** — this row records where the string forms actually live | [x] |

**Row count: 150.** All 150 rows are covered by
`tests/a8_argon2.rs` (8.1 – 8.36), `tests/a8_argon2_core.rs` (8.37 – 8.75, 8.87 – 8.90),
`tests/a8_argon2_encoding.rs` (8.76 – 8.86), `tests/a8_scrypt.rs` (8.91 – 8.122) and
`tests/a8_ipcrypt.rs` (8.123 – 8.150).

Corrections found while writing those tests (the C is authoritative):

* Row 8.35's literal example `"$argon2id$v=19$m=8,t=1,p=2$…"` is **rejected** by
  `argon2_decode_string`'s final `argon2_validate_inputs` (`m_cost < 8 * lanes` →
  `ARGON2_MEMORY_TOO_LITTLE`), so `crypto_pwhash_str_needs_rehash` returns `-1`, not `0`.
  The "`p`/lanes are not compared" quirk is real and is pinned with `m=16,t=1,p=2` instead.
* Rows 8.44/8.45 are only true of the *amount of work*: `argon2_initial_hash` hashes the
  caller's raw `m_cost`, so `m_cost = 8, 9, 10, 11` produce four **different** digests even
  though they all round down to `segment_length = 2`.  The same applies to row 8.17's
  `memlimit = 9216`.

### Notes on axis interactions worth encoding as test-matrix invariants

1. `threads` never influences the digest (`argon2_fill_memory_blocks` ignores it); `lanes` does.
   Rows 8.41/8.42 pin this.
2. `m_cost` is rounded down to `segment_length * lanes * ARGON2_SYNC_POINTS`, so ranges of `m_cost`
   values are observationally equal (rows 8.44/8.45). The high-level API's `memlimit / 1024U`
   truncation adds a second, independent rounding (rows 8.17/8.33).
3. The high-level API fixes `lanes = threads = 1` and `saltlen = crypto_pwhash_SALTBYTES` (16), so
   the lanes/salt axes are only reachable through `argon2_ctx`/`argon2_hash`/`argon2*_hash_*`.
4. `crypto_pwhash_str*` always uses `STR_HASHBYTES = 32` and `p = 1`; the encoded-string axis for
   other `outlen`/`p` values is only reachable via `argon2*_hash_encoded` / `argon2_encode_string`.
5. scrypt's `pickparams` is the only place `opslimit`/`memlimit` are interpreted; both of its
   branches, and the `p > 1` and `N = 2` corners, are covered by rows 8.102–8.106.
6. ipcrypt ND/NDX are deterministic *given the tweak* and carry the tweak in the output
   (`out[0..8)` / `out[0..16)`); PFX is deterministic and prefix-preserving; the deterministic
   variant is raw AES-128 on one block.
