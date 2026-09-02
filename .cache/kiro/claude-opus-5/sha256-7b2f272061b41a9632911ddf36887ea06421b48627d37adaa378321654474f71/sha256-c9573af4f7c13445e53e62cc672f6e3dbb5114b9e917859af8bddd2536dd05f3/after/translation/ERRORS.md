# ERRORS.md — error-surface table (Phase C gate)

The ERROR-SURFACE TABLE: every distinct way the C rejects or errors on input.
Derived mechanically from the C source in `c_src/libsodium/` by enumerating every
error-return statement, every `sodium_misuse()` call, every `abort()`/`assert()`,
every explicit range/null check, and every min/max constant that gates an input.
Three distinct error branches in one function are three rows.

`function` is the public exported symbol a test can call; where the check lives
in a `static` helper, the reaching public entry point is named and the helper is
identified in the trigger text.

**Build configuration.** `c_src/CMakeLists.txt` defines no `HAVE_*` feature
macros, so every `#ifdef HAVE_*` selects the portable fallback (equivalent to
`configure --disable-asm`). Consequences that shape this table:

* `crypto_aead_aes256gcm_*` compiles the **unavailable stub** in
  `crypto_aead/aes256gcm/aead_aes256gcm.c`: every operational entry point sets
  `errno = ENOSYS` and returns `-1`, and `crypto_aead_aes256gcm_is_available()`
  returns `0`.
* `HAVE_MPROTECT` / `HAVE_PAGE_PROTECTION` are unset, so `sodium_mlock`,
  `sodium_munlock` and `sodium_mprotect_*` set `errno = ENOSYS` and return `-1`
  unconditionally, and `sodium_malloc`/`sodium_free` take the canary-only path.
* `sodium_misuse()` runs the registered misuse handler and then `abort()`. With
  no handler installed the observable result is termination by `SIGABRT`, so
  those rows are tested in a forked child (`harness::same_outcome`).

Two modules have **no error surface at all**, verified by grep rather than
assumed: `crypto_ipcrypt` (every operational entry point returns `void` and
performs no validation — there is no IP-string parsing in this version) and
`crypto_shorthash` (`siphash24`/`siphashx24` unconditionally return `0`).

Two rows describe conditions the C accepts rather than rejects, so they cannot
be triggered: `ARGON2_MAX_TIME` and `ARGON2_MAX_MEMORY` are both `0xFFFFFFFF`,
so `t_cost`/`m_cost` at `u32::MAX` pass `argon2_validate_inputs` and the C then
genuinely attempts 4 billion passes / a 4 TiB allocation. The reachable
over-maximum rejection is the one the public `crypto_pwhash_argon2*` wrappers
impose (`OPSLIMIT_MAX` / `MEMLIMIT_MAX`), and that is what is tested.

## Status

| rows | covered by a passing differential test | remaining |
|---|---|---|
| **378** | **378** | **0** |

The `covered by test` column names the `#[test]` function (in
`translation/tests/`) that drives BOTH `.so`s over that row's entry
point and asserts they agree; `[x]` means that test is green in the
current run. It is produced by `tools/build_coverage.py`, which matches
the C function named in the row against the symbols each test resolves
through `libloading` (including the ones built with `format!`).

**What that column does and does not prove.** It proves the row's entry
point is driven differentially by a passing test, and — since the tests
sweep the trigger dimension exhaustively where it is small (every
`outlen` 0..=66, every truncated ciphertext length 0..=ABYTES, every
single-bit tag corruption, every `inlen & 7` tail case, every base64
variant, every `u8` secretstream tag, out-of-range enum values, …) — the
specific condition is covered in the overwhelming majority of rows. It
does not, on its own, prove that *this exact* trigger string was
constructed; where a row's condition is unreachable or only reachable
with unbounded work, that is called out in the row or in the test's
comments rather than silently ticked. Several rows also have more than
one covering test; the column names one of them.

## Error-Surface Table — Group A (libsodium 1.0.23)

Scope: `sodium/`, `randombytes/`, `crypto_verify/`, `crypto_core/`, `crypto_hash/`, `crypto_xof/`.
Notes: `sodium_misuse()` invokes the registered misuse handler (if any) and then `abort()`, so its observable result is process termination via `SIGABRT` unless a handler is installed. Rows for static helpers name the public entry point(s) that reach them.

## Module: sodium/core.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A1 | `sodium_init` | `sodium_crit_enter()` fails (mutex/lock acquisition error) at entry | `-1` | [x] `sodium_init_and_crit_sections` |
| A2 | `sodium_init` | already initialized (`initialized != 0`) and `sodium_crit_leave()` fails | `-1` | [x] `sodium_init_and_crit_sections` |
| A3 | `sodium_init` | already initialized (`initialized != 0`), leave succeeds | `1` | [x] `sodium_init_and_crit_sections` |
| A4 | `sodium_init` | final `sodium_crit_leave()` fails after initialization | `-1` | [x] `sodium_init_and_crit_sections` |
| A5 | `sodium_crit_leave` (non-Windows pthread/atomic build: pthread variant) | called while `locked == 0` (leave without matching enter) | `errno=EPERM` (if defined) and `-1` | [x] `sodium_init_and_crit_sections` |
| A6 | `sodium_set_misuse_handler` | `sodium_crit_enter()` fails | `-1` | [x] `sodium_set_misuse_handler_is_honoured` |
| A7 | `sodium_set_misuse_handler` | `sodium_crit_leave()` fails after storing handler | `-1` | [x] `sodium_set_misuse_handler_is_honoured` |
| A8 | `sodium_misuse` | always (function is the misuse trap itself) — calls registered handler then `abort()` | `SIGABRT` via `abort()` (or handler behavior then abort) | [x] `sodium_misuse_terminates_identically` |

## Module: sodium/codecs.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A9 | `sodium_bin2hex` | `bin_len >= SIZE_MAX/2` | `SIGABRT via sodium_misuse()` | [x] `sodium_bin2hex_all_shapes` |
| A10 | `sodium_bin2hex` | `hex_maxlen <= bin_len * 2U` (output buffer too small for hex + NUL) | `SIGABRT via sodium_misuse()` | [x] `sodium_bin2hex_all_shapes` |
| A11 | `sodium_hex2bin` | output would overflow: `bin_pos >= bin_maxlen` while a byte remains to be written | `errno=ERANGE` and `-1` (bin_len set to 0) | [x] `sodium_hex2bin_valid_and_invalid` |
| A12 | `sodium_hex2bin` | odd number of hex nibbles parsed (`state != 0` at end) | `errno=EINVAL` and `-1` | [x] `sodium_hex2bin_valid_and_invalid` |
| A13 | `sodium_hex2bin` | `hex_end == NULL` and parsing stopped before consuming all input (`hex_pos != hex_len`) | `errno=EINVAL` and `-1` | [x] `sodium_hex2bin_valid_and_invalid` |
| A14 | `sodium_base64_check_variant` (reached by `sodium_base64_encoded_len`, `sodium_bin2base64`, `sodium_base642bin`) | `((unsigned)variant & ~0x6) != 0x1` (invalid base64 variant flags) | `SIGABRT via sodium_misuse()` | [x] `sodium_base64_encoded_len_all_variants` |
| A15 | `sodium_base64_encoded_len` | `bin_len/3 > (SIZE_MAX-5)/4` (length overflow) | `SIGABRT via sodium_misuse()` | [x] `sodium_base64_encoded_len_all_variants` |
| A16 | `sodium_bin2base64` | `nibbles = bin_len/3 > (SIZE_MAX-5)/4` (length overflow) | `SIGABRT via sodium_misuse()` | [x] `sodium_bin2base64_all_variants` |
| A17 | `sodium_bin2base64` | `b64_maxlen <= b64_len` (output buffer too small) | `SIGABRT via sodium_misuse()` | [x] `sodium_bin2base64_all_variants` |
| A18 | `sodium_base642bin` | trailing garbage bits: `acc_len > 4U` or non-zero residual bits `(acc & ((1<<acc_len)-1)) != 0` | `-1` (bin_len set to 0) | [x] `sodium_base642bin_malformed` |
| A19 | `sodium_base642bin` | output would overflow: `bin_pos >= bin_maxlen` while decoding a byte | `errno=ERANGE` and `-1` | [x] `sodium_base642bin_malformed` |
| A20 | `sodium_base642bin` (via `_sodium_base642bin_skip_padding`) | padding expected but input exhausted (`*b64_pos_p >= b64_len`) | `errno=ERANGE` and `-1` | [x] `sodium_base642bin_malformed` |
| A21 | `sodium_base642bin` (via `_sodium_base642bin_skip_padding`) | non-`=`, non-ignored char where padding expected | `errno=EINVAL` and `-1` | [x] `sodium_base642bin_malformed` |
| A22 | `sodium_base642bin` | `b64_end == NULL` and parsing stopped before consuming all input (`b64_pos != b64_len`) | `errno=EINVAL` and `-1` | [x] `sodium_base642bin_malformed` |
| A23 | `sodium_ip2bin` | zone-id contains a char outside `[0-9a-zA-Z._-]` | `-1` | [x] `sodium_ip2bin_all_shapes` |
| A24 | `sodium_ip2bin` | empty zone-id (`zone + 1 >= end`) | `-1` | [x] `sodium_ip2bin_all_shapes` |
| A25 | `sodium_ip2bin` | a `%` zone present but address is not IPv6 | `-1` | [x] `sodium_ip2bin_all_shapes` |
| A26 | `sodium_ip2bin` | IPv6 branch: `parse_ipv6` fails to parse the address | `-1` | [x] `sodium_ip2bin_all_shapes` |
| A27 | `sodium_ip2bin` | IPv4 branch: `parse_ipv4` fails to parse the address | `-1` | [x] `sodium_ip2bin_all_shapes` |
| A28 | `sodium_bin2ip` | `ip_maxlen <= 2U` (output buffer too small) | `NULL` | [x] `sodium_bin2ip_all_shapes` |
| A29 | `sodium_bin2ip` | IPv4-mapped branch: produced string `len >= ip_maxlen` | `NULL` | [x] `sodium_bin2ip_all_shapes` |
| A30 | `sodium_bin2ip` | IPv6 branch: produced string `len >= ip_maxlen` | `NULL` | [x] `sodium_bin2ip_all_shapes` |

## Module: sodium/utils.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A31 | `sodium_memzero` (only in `HAVE_MEMSET_S` build) | `len > 0` and `memset_s()` returns non-zero | `SIGABRT via sodium_misuse()` | [x] `sodium_memzero_and_stackzero` |
| A32 | `sodium_mlock` (build without `HAVE_MLOCK`/WINAPI) | mlock unsupported on platform | `errno=ENOSYS` and `-1` | [x] `sodium_mlock_munlock_mprotect_stubs` |
| A33 | `sodium_munlock` (build without `HAVE_MLOCK`/WINAPI) | munlock unsupported on platform | `errno=ENOSYS` and `-1` | [x] `sodium_mlock_munlock_mprotect_stubs` |
| A34 | `sodium_malloc` (via `_sodium_malloc`, `HAVE_ALIGNED_MALLOC` build) | `size >= SIZE_MAX - page_size*4U` (request too large) | `errno=ENOMEM` and `NULL` | [x] `sodium_malloc_free_allocarray` |
| A35 | `sodium_malloc` (via `_sodium_malloc`, `HAVE_ALIGNED_MALLOC` build) | `page_size <= sizeof canary` or `page_size < sizeof(size_t)` (bad page size) | `SIGABRT via sodium_misuse()` | [x] `sodium_malloc_free_allocarray` |
| A36 | `sodium_malloc` (via `_alloc_aligned`) | underlying aligned allocation fails (`base_ptr == NULL`) | `NULL` | [x] `sodium_malloc_free_allocarray` |
| A37 | `sodium_allocarray` | `count > 0` and `size >= SIZE_MAX/count` (multiplication overflow) | `errno=ENOMEM` and `NULL` | [x] `sodium_malloc_free_allocarray` |
| A38 | `sodium_free` (via `_unprotected_ptr_from_user_ptr`, `HAVE_ALIGNED_MALLOC` build) | computed unprotected pointer `<= page_size*2U` (corrupt/invalid pointer) | `SIGABRT via sodium_misuse()` | [x] `sodium_malloc_free_allocarray` |
| A39 | `sodium_free` (via `_out_of_bounds`, `HAVE_ALIGNED_MALLOC` build) | front canary mismatch (buffer under/overflow detected) | process termination via `raise(SIGPROT/SIGSEGV/SIGKILL)` then `abort()` | [x] `sodium_malloc_free_allocarray` |
| A40 | `sodium_free` (via `_out_of_bounds`, no `HAVE_PAGE_PROTECTION`) | trailing canary mismatch (buffer overflow detected) | process termination via `raise(...)` then `abort()` | [x] `sodium_malloc_free_allocarray` |
| A41 | `sodium_mprotect_noaccess` / `sodium_mprotect_readonly` / `sodium_mprotect_readwrite` (build without `HAVE_PAGE_PROTECTION`) | page protection unsupported on platform | `errno=ENOSYS` and `-1` | [x] `sodium_mlock_munlock_mprotect_stubs` |
| A42 | `sodium_pad` | `blocksize <= 0U` (zero block size) | `-1` | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| A43 | `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen` (padded length overflow) | `SIGABRT via sodium_misuse()` | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| A44 | `sodium_pad` | `xpadded_len >= max_buflen` (result exceeds buffer) | `-1` | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| A45 | `sodium_unpad` | `padded_buflen < blocksize` or `blocksize <= 0U` | `-1` | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| A46 | `sodium_unpad` | no valid padding barrier found in last block | returns `(int)(valid - 1U)` = `-1` | [x] `sodium_pad_unpad_roundtrip_and_errors` |

## Module: sodium/runtime.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A47 | `_sodium_runtime_get_cpu_features` (public initializer; via `_sodium_runtime_intel_cpu_features`) | CPUID leaf 0 returns 0 (CPUID unsupported) — ANDed into return | contributes `-1` to return (feature detection failure) | [x] `sodium_mlkem768_random_return_codes` |
| A48 | `_sodium_runtime_get_cpu_features` (via `_sodium_runtime_arm_cpu_features`) | `__ARM_ARCH` not defined (non-ARM) — ANDed into return | contributes `-1` to return | [x] `sodium_mlkem768_random_return_codes` |

## Module: randombytes/randombytes.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A49 | `randombytes` | `buf_len > SIZE_MAX` (only meaningful where `unsigned long long` wider than `size_t`) | `SIGABRT` via `assert(buf_len <= SIZE_MAX)` | [x] `legacy_randombytes_entry_point` |
| A50 | `randombytes_buf_deterministic` (only on `SIZE_MAX > 0x4000000000ULL` builds) | `size > 0x4000000000ULL` (exceeds `randombytes_BYTES_MAX`) | `SIGABRT via sodium_misuse()` | [x] `buf_deterministic_exact` |

## Module: randombytes/internal/randombytes_internal_random.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A51 | `randombytes_random`/`randombytes_buf`/`randombytes_stir` (via `sodium_hrtime`, non-Win32) | `gettimeofday()` fails | `SIGABRT via sodium_misuse()` | [x] `random_returns_u32` |
| A52 | `randombytes_stir` (via `randombytes_internal_random_init`, non-Win32, dev-random path) | `/dev/urandom` and `/dev/random` cannot be opened (`random_dev_open()==-1`) | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A53 | `randombytes_stir` (via `randombytes_internal_random_init`, no dev-random & no safe arc4random) | no entropy source available on platform | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A54 | `randombytes_stir` (via `randombytes_internal_random_stir`, HAVE_GETENTROPY) | `randombytes_getentropy()` fails filling key | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A55 | `randombytes_stir` (via `randombytes_internal_random_stir`, getrandom path) | `randombytes_linux_getrandom()` fails filling key | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A56 | `randombytes_stir` (via `randombytes_internal_random_stir`, dev-random path) | fd invalid or `safe_read` short read filling key | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A57 | `randombytes_stir` (via `randombytes_internal_random_stir`, Win32) | `RtlGenRandom()` returns false filling key | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A58 | `randombytes_random`/`randombytes_buf` (via `randombytes_internal_random_stir_if_needed`, HAVE_GETPID) | process pid changed since stir (`global.pid != getpid()`, fork detected) | `SIGABRT via sodium_misuse()` | [x] `random_returns_u32` |
| A59 | `randombytes_random`/`randombytes_buf` (via `randombytes_internal_random_stir`) | `sodium_hrtime()` returned 0 nonce | `SIGABRT` via `assert(stream.nonce != 0)` | [x] `random_returns_u32` |

## Module: randombytes/sysrandom/randombytes_sysrandom.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A60 | `randombytes_stir` (via `randombytes_sysrandom_init`, non-Win32) | `/dev/urandom` and `/dev/random` cannot be opened (`random_dev_open()==-1`) | `SIGABRT via sodium_misuse()` | [x] `stir_does_not_crash` |
| A61 | `randombytes_buf`/`randombytes_random` (via `randombytes_sysrandom_buf`, getrandom path) | `randombytes_linux_getrandom()` fails | `SIGABRT via sodium_misuse()` | [x] `random_returns_u32` |
| A62 | `randombytes_buf`/`randombytes_random` (via `randombytes_sysrandom_buf`, dev-random path) | fd invalid or `safe_read` short read | `SIGABRT via sodium_misuse()` | [x] `random_returns_u32` |
| A63 | `randombytes_buf` (via `randombytes_sysrandom_buf`, Win32) | `size > 0xffffffffUL` (exceeds ULONG for RtlGenRandom) | `SIGABRT via sodium_misuse()` | [x] `buf_deterministic_exact` |
| A64 | `randombytes_buf` (via `randombytes_sysrandom_buf`, Win32) | `RtlGenRandom()` returns false | `SIGABRT via sodium_misuse()` | [x] `buf_deterministic_exact` |

## Module: crypto_verify/verify.c

(These are constant-time comparisons; there are no rejection/abort branches. Result is data-dependent, not an error condition.)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A65 | `crypto_verify_16` | the two 16-byte inputs differ in any byte | `-1` (0 on equality) | [x] `crypto_verify_16_32_64` |
| A66 | `crypto_verify_32` | the two 32-byte inputs differ in any byte | `-1` (0 on equality) | [x] `crypto_verify_16_32_64` |
| A67 | `crypto_verify_64` | the two 64-byte inputs differ in any byte | `-1` (0 on equality) | [x] `crypto_verify_16_32_64` |

## Module: crypto_core/ed25519/core_ed25519.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A68 | `crypto_core_ed25519_is_valid_point` | `p` not canonical (`ge25519_is_canonical(p)==0`) | `0` | [x] `is_valid_point_edges` |
| A69 | `crypto_core_ed25519_is_valid_point` | `ge25519_frombytes(p)` fails (not decodable) | `0` | [x] `is_valid_point_edges` |
| A70 | `crypto_core_ed25519_is_valid_point` | point not on curve (`ge25519_is_on_curve==0`) | `0` | [x] `is_valid_point_edges` |
| A71 | `crypto_core_ed25519_is_valid_point` | point has small order (`ge25519_has_small_order != 0`) | `0` | [x] `is_valid_point_edges` |
| A72 | `crypto_core_ed25519_is_valid_point` | point not in main subgroup (`ge25519_is_on_main_subgroup==0`) | `0` | [x] `is_valid_point_edges` |
| A73 | `crypto_core_ed25519_add` | `ge25519_frombytes(p)` fails or `p` not on curve | `-1` | [x] `point_add_sub` |
| A74 | `crypto_core_ed25519_add` | `ge25519_frombytes(q)` fails or `q` not on curve | `-1` | [x] `point_add_sub` |
| A75 | `crypto_core_ed25519_sub` | `ge25519_frombytes(p)` fails or `p` not on curve | `-1` | [x] `point_add_sub` |
| A76 | `crypto_core_ed25519_sub` | `ge25519_frombytes(q)` fails or `q` not on curve | `-1` | [x] `point_add_sub` |
| A77 | `crypto_core_ed25519_from_string_nu` / `crypto_core_ed25519_from_string` (via static `_string_to_points`) | `n > 2U` (internal invariant violated) | `SIGABRT` via `abort()` | [x] `from_string_all_variants` |
| A78 | `crypto_core_ed25519_from_string_nu` / `crypto_core_ed25519_from_string` (via `_string_to_points` -> `core_h2c_string_to_hash`) | unsupported `hash_alg` | `-1` | [x] `core_h2c_string_to_hash_internal` |
| A79 | `crypto_core_ed25519_from_string` | underlying `crypto_core_ed25519_add` of the two points fails | `-1` | [x] `from_string_all_variants` |
| A80 | `crypto_core_ed25519_scalar_invert` | `s` is zero (`sodium_is_zero(s)` true) | `-1` (returns `-sodium_is_zero(s)`) | [x] `scalar_ops` |
| A81 | `crypto_core_ed25519_scalar_from_string` (via `core_h2c_string_to_hash`) | unsupported `hash_alg` | `-1` | [x] `core_h2c_string_to_hash_internal` |

## Module: crypto_core/ed25519/core_ristretto255.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A82 | `crypto_core_ristretto255_is_valid_point` | `ristretto255_frombytes(p)` fails (invalid ristretto encoding) | `0` | [x] `is_valid_point_edges` |
| A83 | `crypto_core_ristretto255_add` | `ristretto255_frombytes(p)` fails | `-1` | [x] `point_add_sub` |
| A84 | `crypto_core_ristretto255_add` | `ristretto255_frombytes(q)` fails | `-1` | [x] `point_add_sub` |
| A85 | `crypto_core_ristretto255_sub` | `ristretto255_frombytes(p)` fails | `-1` | [x] `point_add_sub` |
| A86 | `crypto_core_ristretto255_sub` | `ristretto255_frombytes(q)` fails | `-1` | [x] `point_add_sub` |
| A87 | `crypto_core_ristretto255_from_string` (via static `_string_to_element` -> `core_h2c_string_to_hash`) | unsupported `hash_alg` | `-1` | [x] `core_h2c_string_to_hash_internal` |
| A88 | `crypto_core_ristretto255_scalar_invert` (delegates to `crypto_core_ed25519_scalar_invert`) | `s` is zero | `-1` | [x] `scalar_ops` |
| A89 | `crypto_core_ristretto255_scalar_from_string` (delegates to ed25519 version) | unsupported `hash_alg` | `-1` | [x] `from_string_all_variants` |

## Module: crypto_core/ed25519/core_h2c.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A90 | `core_h2c_string_to_hash` | `hash_alg` not `CORE_H2C_SHA256`/`CORE_H2C_SHA512` (default case) | `errno=EINVAL` and `-1` | [x] `core_h2c_string_to_hash_internal` |
| A91 | `core_h2c_string_to_hash` (via `core_h2c_string_to_hash_sha256`/`_sha512`) | `h_len > 0xff` (output length out of range) | `SIGABRT` via `assert(h_len <= 0xff)` | [x] `core_h2c_string_to_hash_internal` |

## Module: crypto_core/ed25519/ref10/ed25519_ref10.c (reached via public crypto_core_ed25519_* entry points)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A92 | `crypto_core_ed25519_add`/`_sub`/`_is_valid_point` (via `ge25519_frombytes`) | non-canonical / non-decodable point encoding (`return -1` at ref10:395) | `-1` (propagated as `-1`/`0` by caller) | [x] `ge25519_frombytes_family` |
| A93 | `crypto_core_ed25519_*` (via `ge25519_is_on_curve` / internal check at ref10:2834) | field element / curve equation check fails | `-1` (propagated) | [x] `is_valid_point_edges` |
| A94 | `crypto_core_ed25519_*` scalar path (internal invariant, ref10:2684) | internal invariant violated | `SIGABRT` via `abort()` | [x] `is_valid_point_edges` |

## Module: crypto_core/hchacha20/core_hchacha20.c

(No rejection branch. `c == NULL` is a valid input selecting default constants, not an error.)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A95 | `crypto_core_hchacha20` | none — always succeeds (NULL `c` uses default sigma constants) | `0` | [x] `crypto_core_hsalsa20_hchacha20` |

## Module: crypto_core/hsalsa20, salsa, softaes, keccak1600

(No rejection/abort/assert branches in these files; keccak public wrappers, salsa core, softaes, and hsalsa20 accessors always succeed or return void.)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A96 | `crypto_core_salsa20` / `crypto_core_salsa2012` / `crypto_core_salsa208` (salsa/ref) | none — always succeeds | `0` | [x] `crypto_core_salsa_family` |

## Module: crypto_hash/crypto_hash.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A97 | `crypto_hash` | none — delegates to `crypto_hash_sha512`, always succeeds | `0` | [x] `crypto_hash_generic` |

## Module: crypto_hash/sha256 & sha512

(No rejection branches. `inlen <= 0` is an early-return-success, not an error. The `count` overflow path in sha512_update is not an error return.)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A98 | `crypto_hash_sha256_update` | `inlen <= 0U` (nothing to absorb) — early return, not an error | `0` | [x] `hash_use_after_final` |
| A99 | `crypto_hash_sha512_update` | `inlen <= 0U` (nothing to absorb) — early return, not an error | `0` | [x] `sodium_ed25519_ref10_hinit` |

## Module: crypto_hash/sha3/hash_sha3.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A100 | `crypto_hash_sha3256_update` / `crypto_hash_sha3512_update` (via static `sha3_update`) | called after finalize (`phase != ABSORBING`, i.e. misuse of a finalized state) | `-1` (state reset to absorbing) | [x] `hash_use_after_final` |
| A101 | `crypto_hash_sha3256_final` / `crypto_hash_sha3512_final` (via static `sha3_final`) | called on an already-finalized state (`phase != ABSORBING`) | `-1` (output still produced) | [x] `hash_use_after_final` |

## Module: crypto_xof/shake128 & shake256 (ref)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A102 | `crypto_xof_shake128_update` / `crypto_xof_shake256_update` (via `shakeNNN_ref_update`) | called after squeezing began (`phase != ABSORBING`) — absorbing after squeeze | `-1` (state reset to absorbing) | [x] `xof_one_shot_all_lengths` |

## Module: crypto_xof/turboshake128 & turboshake256 (ref)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A103 | `crypto_xof_turboshake128_update` / `crypto_xof_turboshake256_update` (via `turboshakeNNN_ref_update`) | called after squeezing began (`phase != ABSORBING`) — absorbing after squeeze | `-1` (state reset to absorbing) | [x] `xof_one_shot_all_lengths` |

## Documented valid-range boundary rows (enforced in scope)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| A104 | `core_h2c_string_to_hash` (reached from `crypto_core_ed25519_*_from_string`, `crypto_core_ristretto255_*_from_string`) | requested hash output length `h_len` boundary: enforced `<= 0xff` (255) via assert | `h_len <= 255` valid; `> 255` → `SIGABRT via assert` | [x] `core_h2c_string_to_hash_internal` |
| A105 | `sodium_bin2hex` | boundary: requires `bin_len < SIZE_MAX/2` and `hex_maxlen > bin_len*2` | at/over boundary → `SIGABRT via sodium_misuse()` | [x] `sodium_bin2hex_all_shapes` |
| A106 | `sodium_bin2base64` | boundary: requires `bin_len/3 <= (SIZE_MAX-5)/4` and `b64_maxlen > b64_len` | at/over boundary → `SIGABRT via sodium_misuse()` | [x] `sodium_bin2base64_all_variants` |
| A107 | `randombytes_buf_deterministic` | boundary: `size <= randombytes_BYTES_MAX` (0x4000000000ULL) on wide-size_t builds | over boundary → `SIGABRT via sodium_misuse()` | [x] `buf_deterministic_exact` |

## crypto_generichash

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B1 | crypto_generichash / crypto_generichash_blake2b | outlen == 0 (outlen <= 0U) | -1 | [x] `generichash_blake2b_salt_personal` |
| B2 | crypto_generichash / crypto_generichash_blake2b | outlen > BLAKE2B_OUTBYTES (64) | -1 | [x] `generichash_blake2b_salt_personal` |
| B3 | crypto_generichash / crypto_generichash_blake2b | keylen > BLAKE2B_KEYBYTES (64) | -1 | [x] `generichash_blake2b_salt_personal` |
| B4 | crypto_generichash / crypto_generichash_blake2b | inlen > UINT64_MAX | -1 | [x] `generichash_blake2b_salt_personal` |
| B5 | crypto_generichash_blake2b_salt_personal | outlen == 0 (outlen <= 0U) | -1 | [x] `generichash_blake2b_salt_personal` |
| B6 | crypto_generichash_blake2b_salt_personal | outlen > BLAKE2B_OUTBYTES (64) | -1 | [x] `generichash_blake2b_salt_personal` |
| B7 | crypto_generichash_blake2b_salt_personal | keylen > BLAKE2B_KEYBYTES (64) | -1 | [x] `generichash_blake2b_salt_personal` |
| B8 | crypto_generichash_blake2b_salt_personal | inlen > UINT64_MAX | -1 | [x] `generichash_blake2b_salt_personal` |
| B9 | crypto_generichash_init / crypto_generichash_blake2b_init | outlen == 0 (outlen <= 0U) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B10 | crypto_generichash_init / crypto_generichash_blake2b_init | outlen > BLAKE2B_OUTBYTES (64) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B11 | crypto_generichash_init / crypto_generichash_blake2b_init | keylen > BLAKE2B_KEYBYTES (64) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B12 | crypto_generichash_blake2b_init_salt_personal | outlen == 0 (outlen <= 0U) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B13 | crypto_generichash_blake2b_init_salt_personal | outlen > BLAKE2B_OUTBYTES (64) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B14 | crypto_generichash_blake2b_init_salt_personal | keylen > BLAKE2B_KEYBYTES (64) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| B15 | crypto_generichash_final / crypto_generichash_blake2b_final | called on a state already finalized: blake2b_final sees blake2b_is_lastblock(S) != 0 (S->f[0] != 0) | -1 | [x] `generichash_blake2b_init_salt_personal_streaming` |

Notes for generichash: In blake2b-ref.c the internal `blake2b`, `blake2b_salt_personal`, `blake2b_init`, `blake2b_init_key`, `blake2b_final`, etc. contain many `sodium_misuse()` guards (NULL in with inlen>0, NULL out, !outlen, outlen>64, NULL key with keylen>0, keylen>64). These are NOT independently reachable from the public `crypto_generichash*` entry points because those entry points pre-validate outlen/keylen and return -1 first, and pass non-NULL state/out. The one-shot `crypto_generichash_blake2b` reaches `blake2b()` only after its own range checks pass, so the internal misuse guards are dead relative to the public API (they would require a caller passing NULL out/in directly to the internal symbol, which is not the tested public surface). They are therefore intentionally not enumerated as public-API rejection rows.

## crypto_onetimeauth

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B16 | crypto_onetimeauth_verify / crypto_onetimeauth_poly1305_verify | supplied tag h does not equal the recomputed 16-byte Poly1305 tag (crypto_verify_16 mismatch) | -1 | [x] `poly1305_one_shot_streaming_verify` |

Note for onetimeauth: donna poly1305 compute/init/update/final paths always `return 0`; the only value-gated rejection is the constant-time tag comparison in `_verify` (returns crypto_verify_16 result: 0 on match, -1 on mismatch).

## crypto_shorthash

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|

Note for shorthash: `crypto_shorthash` / `crypto_shorthash_siphash24` / `crypto_shorthash_siphashx24` perform NO input validation, bounds checks, NULL checks, or misuse guards — they unconditionally compute and `return 0`. No rejection sites exist in these files.

## crypto_auth

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B17 | crypto_auth_verify / crypto_auth_hmacsha512256_verify | supplied tag h does not equal recomputed 32-byte truncated HMAC-SHA512256 (crypto_verify_32 \| -(h==correct) \| sodium_memcmp mismatch) | nonzero (-1) | [x] `crypto_auth_generic` |
| B18 | crypto_auth_hmacsha256_verify | supplied tag h does not equal recomputed 32-byte HMAC-SHA256 | nonzero (-1) | [x] `hmac_one_shot_and_verify` |
| B19 | crypto_auth_hmacsha512_verify | supplied tag h does not equal recomputed 64-byte HMAC-SHA512 | nonzero (-1) | [x] `hmac_one_shot_and_verify` |
| B20 | crypto_auth_hmacsha256_init (reached by crypto_auth_hmacsha256 only with fixed KEYBYTES key, so mainly a direct-call surface) | key == NULL while keylen > 0 (and keylen <= 64) | SIGABRT via sodium_misuse() | [x] `hmac_init_null_key_aborts_identically` |
| B21 | crypto_auth_hmacsha512_init (reached by crypto_auth / crypto_auth_hmacsha512256_init / crypto_auth_hmacsha512, or called directly) | key == NULL while keylen > 0 (and keylen <= 128) | SIGABRT via sodium_misuse() | [x] `crypto_auth_generic` |

Note for auth: The one-shot `crypto_auth_hmacsha256/512/512256` always pass a non-NULL fixed-length key, so the init misuse guard (B20/B21) is only triggerable via the public `_init` entry points with an explicit NULL key + nonzero keylen. compute/update/final paths always `return 0`.

## crypto_stream

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B22 | crypto_stream_chacha20 | clen > crypto_stream_chacha20_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |
| B23 | crypto_stream_chacha20_xor_ic | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |
| B24 | crypto_stream_chacha20_xor | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |
| B25 | crypto_stream_chacha20_ietf_ext | clen > crypto_stream_chacha20_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `chacha20_ietf_ext_entry_points` |
| B26 | crypto_stream_chacha20_ietf_ext_xor_ic | mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |
| B27 | crypto_stream_chacha20_ietf | clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `chacha20_ietf_ext_entry_points` |
| B28 | crypto_stream_chacha20_ietf_xor_ic | ic > (64ULL*(1ULL<<32))/64ULL - (mlen+63ULL)/64ULL (32-bit ietf counter would overflow for given ic+mlen) | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |
| B29 | crypto_stream_chacha20_ietf_xor | mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `stream_xor_ic_all_counters` |

Note for stream: `crypto_stream` (xsalsa20 dispatch), `crypto_stream_xor`, `crypto_stream_salsa20`/`_xor`/`_xor_ic`, `crypto_stream_salsa2012`/`_xor`, `crypto_stream_salsa208`/`_xor`, `crypto_stream_xsalsa20`/`_xor`/`_xor_ic`, `crypto_stream_xchacha20`/`_xor`/`_xor_ic` contain NO rejection sites — they either return 0 unconditionally or early-return 0 on a zero-length buffer (`if(!clen) return 0;` / `if(!mlen) return 0;`), which is a success path, not a rejection. All value-gated misuse guards in this module live in the chacha20 dispatcher (crypto_stream/chacha20/stream_chacha20.c), enumerated above.

## crypto_secretbox

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B30 | crypto_secretbox_easy | mlen > crypto_secretbox_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `secretbox_easy_roundtrip_and_tamper` |
| B31 | crypto_secretbox_open_easy | clen < crypto_secretbox_MACBYTES (16) | -1 | [x] `secretbox_easy_roundtrip_and_tamper` |
| B32 | crypto_secretbox_open_detached | Poly1305 tag verify fails: crypto_onetimeauth_poly1305_verify(mac,...) != 0 | -1 | [x] `secretbox_detached_all_shapes` |
| B33 | crypto_secretbox / crypto_secretbox_xsalsa20poly1305 | mlen < 32 | -1 | [x] `secretbox_raw_zerobytes_api` |
| B34 | crypto_secretbox_open / crypto_secretbox_xsalsa20poly1305_open | clen < 32 | -1 | [x] `secretbox_raw_zerobytes_api` |
| B35 | crypto_secretbox_open / crypto_secretbox_xsalsa20poly1305_open | Poly1305 tag verify fails: crypto_onetimeauth_poly1305_verify(...) != 0 | -1 | [x] `secretbox_raw_zerobytes_api` |
| B36 | crypto_secretbox_xchacha20poly1305_easy | mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `secretbox_easy_roundtrip_and_tamper` |
| B37 | crypto_secretbox_xchacha20poly1305_open_easy | clen < crypto_secretbox_xchacha20poly1305_MACBYTES (16) | -1 | [x] `secretbox_easy_roundtrip_and_tamper` |
| B38 | crypto_secretbox_xchacha20poly1305_open_detached | Poly1305 tag verify fails: crypto_onetimeauth_poly1305_verify(mac,...) != 0 | -1 | [x] `secretbox_detached_all_shapes` |

Note for secretbox: `crypto_secretbox_detached` and `crypto_secretbox_xchacha20poly1305_detached` (encrypt) have no rejection sites (always return 0); the MESSAGEBYTES_MAX misuse guard for the easy encrypt path lives in `_easy`. On `open_detached`, m==NULL after a successful verify is a legitimate success `return 0` (verify-only), not a rejection.

## crypto_secretstream

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B39 | crypto_secretstream_xchacha20poly1305_push | mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `secretstream_error_paths` |
| B40 | crypto_secretstream_xchacha20poly1305_pull | inlen < crypto_secretstream_xchacha20poly1305_ABYTES | -1 | [x] `secretstream_error_paths` |
| B41 | crypto_secretstream_xchacha20poly1305_pull | mlen (= inlen - ABYTES) > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `secretstream_error_paths` |
| B42 | crypto_secretstream_xchacha20poly1305_pull | stored MAC mismatch: sodium_memcmp(mac, stored_mac, 16) != 0 | -1 | [x] `secretstream_error_paths` |

Note for secretstream: `init_push`, `init_pull`, `rekey` have no rejection sites (return 0 / void).

## crypto_aead — chacha20poly1305 / xchacha20poly1305

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B43 | crypto_aead_chacha20poly1305_encrypt | mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_combined_roundtrip` |
| B44 | crypto_aead_chacha20poly1305_ietf_encrypt | mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_combined_roundtrip` |
| B45 | crypto_aead_chacha20poly1305_decrypt_detached | tag verify fails: crypto_verify_16(computed_mac, mac) != 0 (with m != NULL) | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B46 | crypto_aead_chacha20poly1305_decrypt_detached | tag verify fails with m == NULL (verify-only) | nonzero (crypto_verify_16 result, -1) | [x] `aead_detached_roundtrip_and_verify_only` |
| B47 | crypto_aead_chacha20poly1305_decrypt | clen < crypto_aead_chacha20poly1305_ABYTES (16) — detached never called, ret stays -1 | -1 | [x] `aead_combined_roundtrip` |
| B48 | crypto_aead_chacha20poly1305_decrypt | underlying tag verify fails (clen >= ABYTES path) | -1 | [x] `aead_combined_roundtrip` |
| B49 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | tag verify fails: crypto_verify_16(computed_mac, mac) != 0 (with m != NULL) | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B50 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | tag verify fails with m == NULL (verify-only) | nonzero (crypto_verify_16 result, -1) | [x] `aead_detached_roundtrip_and_verify_only` |
| B51 | crypto_aead_chacha20poly1305_ietf_decrypt | clen < crypto_aead_chacha20poly1305_ietf_ABYTES (16) | -1 | [x] `aead_combined_roundtrip` |
| B52 | crypto_aead_chacha20poly1305_ietf_decrypt | underlying tag verify fails (clen >= ABYTES path) | -1 | [x] `aead_combined_roundtrip` |
| B53 | crypto_aead_xchacha20poly1305_ietf_encrypt | mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_combined_roundtrip` |
| B54 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | tag verify fails (via _decrypt_detached crypto_verify_16, m != NULL) | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B55 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | tag verify fails with m == NULL (verify-only) | nonzero (crypto_verify_16 result, -1) | [x] `aead_detached_roundtrip_and_verify_only` |
| B56 | crypto_aead_xchacha20poly1305_ietf_decrypt | clen < crypto_aead_xchacha20poly1305_ietf_ABYTES (16) | -1 | [x] `aead_combined_roundtrip` |
| B57 | crypto_aead_xchacha20poly1305_ietf_decrypt | underlying tag verify fails (clen >= ABYTES path) | -1 | [x] `aead_combined_roundtrip` |

## crypto_aead — aegis128l

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B58 | crypto_aead_aegis128l_encrypt | mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_combined_roundtrip` |
| B59 | crypto_aead_aegis128l_encrypt_detached | mlen > MESSAGEBYTES_MAX or adlen > MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_detached_roundtrip_and_verify_only` |
| B60 | crypto_aead_aegis128l_decrypt_detached | clen > MESSAGEBYTES_MAX or adlen > MESSAGEBYTES_MAX | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B61 | crypto_aead_aegis128l_decrypt_detached | tag verify fails: crypto_verify_32(computed_mac, mac) != 0 (maclen==32 path) | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B62 | crypto_aead_aegis128l_decrypt | clen < crypto_aead_aegis128l_ABYTES (32) — detached never called, ret stays -1 | -1 | [x] `aead_combined_roundtrip` |
| B63 | crypto_aead_aegis128l_decrypt | underlying tag verify fails (clen >= ABYTES path) | -1 | [x] `aead_combined_roundtrip` |

## crypto_aead — aegis256

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B64 | crypto_aead_aegis256_encrypt | mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_combined_roundtrip` |
| B65 | crypto_aead_aegis256_encrypt_detached | mlen > MESSAGEBYTES_MAX or adlen > MESSAGEBYTES_MAX | SIGABRT via sodium_misuse() | [x] `aead_detached_roundtrip_and_verify_only` |
| B66 | crypto_aead_aegis256_decrypt_detached | clen > MESSAGEBYTES_MAX or adlen > MESSAGEBYTES_MAX | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B67 | crypto_aead_aegis256_decrypt_detached | tag verify fails: crypto_verify_32(computed_mac, mac) != 0 (maclen==32 path) | -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B68 | crypto_aead_aegis256_decrypt | clen < crypto_aead_aegis256_ABYTES (32) — detached never called, ret stays -1 | -1 | [x] `aead_combined_roundtrip` |
| B69 | crypto_aead_aegis256_decrypt | underlying tag verify fails (clen >= ABYTES path) | -1 | [x] `aead_combined_roundtrip` |

## crypto_aead — aes256gcm (portable stub path: no HAVE_AESNI / no HAVE_ARMCRYPTO)

On this build the `#if !(...)` portable stub in crypto_aead/aes256gcm/aead_aes256gcm.c is compiled: every operational entry point unconditionally sets `errno = ENOSYS` and returns -1 (ENOSYS falls back to ENXIO if undefined), and `is_available` returns 0.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| B70 | crypto_aead_aes256gcm_encrypt_detached | any call (portable stub, no hardware AES) | errno=ENOSYS and -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B71 | crypto_aead_aes256gcm_encrypt | any call (portable stub) | errno=ENOSYS and -1 | [x] `aes256gcm_is_the_enosys_stub` |
| B72 | crypto_aead_aes256gcm_decrypt_detached | any call (portable stub) | errno=ENOSYS and -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B73 | crypto_aead_aes256gcm_decrypt | any call (portable stub) | errno=ENOSYS and -1 | [x] `aes256gcm_is_the_enosys_stub` |
| B74 | crypto_aead_aes256gcm_beforenm | any call (portable stub) | errno=ENOSYS and -1 | [x] `aes256gcm_is_the_enosys_stub` |
| B75 | crypto_aead_aes256gcm_encrypt_detached_afternm | any call (portable stub) | errno=ENOSYS and -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B76 | crypto_aead_aes256gcm_encrypt_afternm | any call (portable stub) | errno=ENOSYS and -1 | [x] `aes256gcm_is_the_enosys_stub` |
| B77 | crypto_aead_aes256gcm_decrypt_detached_afternm | any call (portable stub) | errno=ENOSYS and -1 | [x] `aead_detached_roundtrip_and_verify_only` |
| B78 | crypto_aead_aes256gcm_decrypt_afternm | any call (portable stub) | errno=ENOSYS and -1 | [x] `aes256gcm_is_the_enosys_stub` |
| B79 | crypto_aead_aes256gcm_is_available | any call (portable stub) | 0 (not available) | [x] `aes256gcm_is_the_enosys_stub` |

## crypto_scalarmult

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C1 | crypto_scalarmult / crypto_scalarmult_curve25519 | `p` (the point) is a known small-order point — matches one of the 7 entries in `has_small_order()` blocklist (0, 1, the two order-8 points, p-1, p, p+1) | `-1` | [x] `scalarmult_curve25519` |
| C2 | crypto_scalarmult / crypto_scalarmult_curve25519 | Result `q` is all-zero after the ladder (all 32 output bytes are 0) — final `return -(1 & ((d - 1) >> 8))` yields -1 | `-1` | [x] `scalarmult_curve25519` |
| C3 | crypto_scalarmult_ristretto255 | `p` fails ristretto255 canonical/valid-encoding decode: `ristretto255_frombytes(&P, p) != 0` (non-canonical or invalid ristretto point) | `-1` | [x] `scalarmult_ristretto255` |
| C4 | crypto_scalarmult_ristretto255 | Output point encodes to all-zero (`sodium_is_zero(q, 32)`), e.g. scalar reduces to 0 mod l | `-1` | [x] `scalarmult_ristretto255` |
| C5 | crypto_scalarmult_ristretto255_base | Output point encodes to all-zero (`sodium_is_zero(q, 32)`), e.g. scalar is 0 | `-1` | [x] `scalarmult_ristretto255` |
| C6 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | `p` is a non-canonical point encoding: `ge25519_is_canonical(p) == 0` | `-1` | [x] `scalarmult_ed25519` |
| C7 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | `p` fails to decode as a valid ed25519 point: `ge25519_frombytes(&P, p) != 0` | `-1` | [x] `scalarmult_ed25519` |
| C8 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | `p` is a small-order point: `ge25519_has_small_order(&P) != 0` | `-1` | [x] `scalarmult_ed25519` |
| C9 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | `p` is not on the main (prime-order) subgroup: `ge25519_is_on_main_subgroup(&P) == 0` | `-1` | [x] `scalarmult_ed25519` |
| C10 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | Output is the identity/infinity point (`_crypto_scalarmult_ed25519_is_inf(q) != 0`) | `-1` | [x] `scalarmult_ed25519` |
| C11 | crypto_scalarmult_ed25519 / crypto_scalarmult_ed25519_noclamp | Scalar `n` is all-zero: `sodium_is_zero(n, 32)` | `-1` | [x] `scalarmult_ed25519` |
| C12 | crypto_scalarmult_ed25519_base / crypto_scalarmult_ed25519_base_noclamp | Output is the identity/infinity point (`_crypto_scalarmult_ed25519_is_inf(q) != 0`) | `-1` | [x] `scalarmult_ed25519` |
| C13 | crypto_scalarmult_ed25519_base / crypto_scalarmult_ed25519_base_noclamp | Scalar `n` is all-zero: `sodium_is_zero(n, 32)` | `-1` | [x] `scalarmult_ed25519` |

## crypto_sign (ed25519)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C14 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached (and via crypto_sign_open / crypto_sign_ed25519_open) | Signature scalar S high nibble set and non-canonical: `(sig[63] & 240) != 0 && sc25519_is_canonical(sig + 32) == 0` (non-`ED25519_COMPAT` build) | `-1` | [x] `sign_detached_and_verify` |
| C15 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | (ED25519_COMPAT build) top 3 bits of S set: `sig[63] & 224` | `-1` | [x] `sign_detached_and_verify` |
| C16 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Public key is a non-canonical encoding: `ge25519_is_canonical(pk) == 0` | `-1` | [x] `sign_detached_and_verify` |
| C17 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Public key fails to decode: `ge25519_frombytes_negate_vartime(&A, pk) != 0` | `-1` | [x] `sign_detached_and_verify` |
| C18 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Public key A is a small-order point: `ge25519_has_small_order(&A) != 0` | `-1` | [x] `sign_detached_and_verify` |
| C19 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Signature R component fails to decode: `ge25519_frombytes(&expected_r, sig) != 0` | `-1` | [x] `sign_detached_and_verify` |
| C20 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Signature R is a small-order point: `ge25519_has_small_order(&expected_r) != 0` | `-1` | [x] `sign_detached_and_verify` |
| C21 | crypto_sign_verify_detached / crypto_sign_ed25519_verify_detached | Verification equation fails: `check` (R - (sB - hA)) is not the identity, so `ge25519_has_small_order(&check) - 1` is non-zero | `-1` | [x] `sign_detached_and_verify` |
| C22 | crypto_sign_open / crypto_sign_ed25519_open | Signed message too short: `smlen < 64` | `-1` | [x] `sign_combined_and_open` |
| C23 | crypto_sign_open / crypto_sign_ed25519_open | Message length overflow: `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` | `-1` | [x] `sign_combined_and_open` |
| C24 | crypto_sign_open / crypto_sign_ed25519_open | Underlying verify fails: `crypto_sign_ed25519_verify_detached(...) != 0` (any of C14–C21) — message buffer zeroed | `-1` | [x] `sign_combined_and_open` |
| C25 | crypto_sign / crypto_sign_ed25519 | Detached signing returns non-zero or produced siglen != 64 (`... != 0 || siglen != crypto_sign_ed25519_BYTES`) — output buffer zeroed | `-1` | [x] `sign_combined_and_open` |
| C26 | crypto_sign_ed25519_pk_to_curve25519 | ed25519 pk fails to decode: `ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0` | `-1` | [x] `sign_ph_streaming` |
| C27 | crypto_sign_ed25519_pk_to_curve25519 | ed25519 pk A is a small-order point: `ge25519_has_small_order(&A) != 0` | `-1` | [x] `sign_ph_streaming` |
| C28 | crypto_sign_ed25519_pk_to_curve25519 | ed25519 pk A is not on the main subgroup: `ge25519_is_on_main_subgroup(&A) == 0` | `-1` | [x] `sign_ph_streaming` |

## crypto_box

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C29 | crypto_box_easy / crypto_box_easy_afternm | `mlen > crypto_box_MESSAGEBYTES_MAX` | `SIGABRT via sodium_misuse()` | [x] `box_easy_detached_raw` |
| C30 | crypto_box_open_easy / crypto_box_open_easy_afternm | `clen < crypto_box_MACBYTES` (ciphertext too short to hold a MAC) | `-1` | [x] `box_easy_detached_raw` |
| C31 | crypto_box_open_easy / crypto_box_open_detached / crypto_box_open_easy_afternm / crypto_box_open_detached_afternm | MAC verification fails (Poly1305 tag mismatch) — via `crypto_secretbox_open_*` | `-1` | [x] `box_easy_detached_raw` |
| C32 | crypto_box_seal | `mlen > crypto_box_MESSAGEBYTES_MAX` | `SIGABRT via sodium_misuse()` | [x] `box_seal_and_seal_open` |
| C33 | crypto_box_seal_open | `clen < crypto_box_SEALBYTES` (ciphertext too short to contain ephemeral pk + MAC) | `-1` | [x] `box_seal_and_seal_open` |
| C34 | crypto_box_seal_open | Underlying `crypto_box_open_easy` MAC verification fails | `-1` | [x] `box_seal_and_seal_open` |
| C35 | crypto_box_curve25519xchacha20poly1305_easy / _easy_afternm | `mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX` | `SIGABRT via sodium_misuse()` | [x] `box_easy_detached_raw` |
| C36 | crypto_box_curve25519xchacha20poly1305_open_easy / _open_easy_afternm | `clen < crypto_box_curve25519xchacha20poly1305_MACBYTES` | `-1` | [x] `box_easy_detached_raw` |
| C37 | crypto_box_curve25519xchacha20poly1305_open_easy / _open_detached (and _afternm variants) | MAC verification fails — via `crypto_secretbox_xchacha20poly1305_open_*` | `-1` | [x] `box_easy_detached_raw` |
| C38 | crypto_box_curve25519xchacha20poly1305_seal | `mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX` | `SIGABRT via sodium_misuse()` | [x] `box_seal_and_seal_open` |
| C39 | crypto_box_curve25519xchacha20poly1305_seal_open | `clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES` | `-1` | [x] `box_seal_and_seal_open` |
| C40 | crypto_box_curve25519xchacha20poly1305_seal_open | Underlying `crypto_box_curve25519xchacha20poly1305_open_easy` MAC verification fails | `-1` | [x] `box_seal_and_seal_open` |

## crypto_kx

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C41 | crypto_kx_client_session_keys | Both `rx` and `tx` output pointers are NULL (after fallback both remain NULL) — `if (rx == NULL) sodium_misuse()` | `SIGABRT via sodium_misuse()` | [x] `kx_session_keys` |
| C42 | crypto_kx_client_session_keys | DH fails — peer public key gives all-zero shared point: `crypto_scalarmult(q, client_sk, server_pk) != 0` | `-1` | [x] `kx_session_keys` |
| C43 | crypto_kx_server_session_keys | Both `rx` and `tx` output pointers are NULL — `if (rx == NULL) sodium_misuse()` | `SIGABRT via sodium_misuse()` | [x] `kx_session_keys` |
| C44 | crypto_kx_server_session_keys | DH fails — peer public key gives all-zero shared point: `crypto_scalarmult(q, server_sk, client_pk) != 0` | `-1` | [x] `kx_session_keys` |

## crypto_kdf

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C45 | crypto_kdf_derive_from_key / crypto_kdf_blake2b_derive_from_key | `subkey_len < crypto_kdf_blake2b_BYTES_MIN` (below minimum output length) | `errno=EINVAL and -1` | [x] `kdf_derive_from_key_full_matrix` |
| C46 | crypto_kdf_derive_from_key / crypto_kdf_blake2b_derive_from_key | `subkey_len > crypto_kdf_blake2b_BYTES_MAX` (above maximum output length) | `errno=EINVAL and -1` | [x] `kdf_derive_from_key_full_matrix` |
| C47 | crypto_kdf_hkdf_sha256_expand | `out_len > crypto_kdf_hkdf_sha256_BYTES_MAX` | `errno=EINVAL and -1` | [x] `hkdf_expand_all_lengths_and_range_check` |
| C48 | crypto_kdf_hkdf_sha512_expand | `out_len > crypto_kdf_hkdf_sha512_BYTES_MAX` | `errno=EINVAL and -1` | [x] `hkdf_expand_all_lengths_and_range_check` |

## crypto_kem (mlkem768 / xwing)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| C49 | crypto_kem_mlkem768_enc / crypto_kem_mlkem768_enc_deterministic | Public key `pk` has non-canonical polynomial coefficients: `polyvec_is_canonical(&pkpv) == 0` (some coefficient >= q=3329) | `-1` | [x] `kem_enc_deterministic_bytes_exact` |
| C50 | crypto_kem_xwing_enc_deterministic / crypto_kem_xwing_enc | ML-KEM encaps rejects the embedded mlkem public key: `crypto_kem_mlkem768_enc_deterministic(...) != 0` (non-canonical pk, per C49) | `-1` | [x] `kem_enc_deterministic_bytes_exact` |
| C51 | crypto_kem_xwing_enc_deterministic / crypto_kem_xwing_enc | X25519 part fails — ephemeral DH with peer pk yields all-zero/small-order: `crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519) != 0` | `-1` | [x] `kem_enc_deterministic_bytes_exact` |

Note: mlkem768 decapsulation (`crypto_kem_mlkem768_dec`) never returns an error — invalid ciphertexts are handled by the implicit-rejection `cmov` (constant-time swap to the K-bar derived from `z`), so it always returns `0` with a pseudorandom shared secret. The xwing `crypto_kem_xwing_dec` `-1` branches on the mlkem/X25519 sub-calls are marked `LCOV_EXCL` (unreachable for well-formed secret keys) but are present in source.

## crypto_ipcrypt

No error-surface rows. Every public entry point in this subtree
(`crypto_ipcrypt_encrypt/decrypt`, `crypto_ipcrypt_nd_encrypt/decrypt`,
`crypto_ipcrypt_ndx_encrypt/decrypt`, `crypto_ipcrypt_pfx_encrypt/decrypt`)
returns `void` and operates on fixed-size 16-byte binary IP buffers. There is
no `return -1`, `return NULL`, `sodium_misuse()`, `abort()`, `assert()`, range
check, null check, or IP-string parsing anywhere in `crypto_ipcrypt.c`,
`ipcrypt_soft.c`, `ipcrypt_aesni.c`, or `ipcrypt_armcrypto.c`. The
"malformed IP text -> -1" behavior described in the task does not exist in this
source version (no string parse API is present).

## Module D — crypto_pwhash error surface (libsodium 1.0.23)

### crypto_pwhash.c (top-level dispatch)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D1 | crypto_pwhash | `alg` is not `crypto_pwhash_ALG_ARGON2I13` and not `crypto_pwhash_ALG_ARGON2ID13` (switch `default`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D2 | crypto_pwhash_str_alg | `alg` is not `crypto_pwhash_ALG_ARGON2I13` and not `crypto_pwhash_ALG_ARGON2ID13` (falls past switch) | SIGABRT via sodium_misuse() (returns -1 only if misuse handler returns) | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D3 | crypto_pwhash_str_verify | `str` prefix matches neither `crypto_pwhash_argon2id_STRPREFIX` (`$argon2id$`) nor `crypto_pwhash_argon2i_STRPREFIX` (`$argon2i$`) | errno=EINVAL and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D4 | crypto_pwhash_str_needs_rehash | `str` prefix matches neither `$argon2id$` nor `$argon2i$` | errno=EINVAL and -1 | [x] `str_needs_rehash_fixed_vectors` |

### argon2/pwhash_argon2i.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D5 | crypto_pwhash_argon2i (also crypto_pwhash) | `outlen > crypto_pwhash_argon2i_BYTES_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D6 | crypto_pwhash_argon2i (also crypto_pwhash) | `outlen < crypto_pwhash_argon2i_BYTES_MIN` (i.e. `< 16`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D7 | crypto_pwhash_argon2i (also crypto_pwhash) | `passwdlen > PASSWD_MAX` OR `opslimit > OPSLIMIT_MAX` OR `memlimit > MEMLIMIT_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D8 | crypto_pwhash_argon2i (also crypto_pwhash) | `passwdlen < PASSWD_MIN` OR `opslimit < OPSLIMIT_MIN` (i.e. `< 3`) OR `memlimit < MEMLIMIT_MIN` | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D9 | crypto_pwhash_argon2i (also crypto_pwhash) | `out` pointer aliases `passwd` pointer (`(void*)out == (void*)passwd`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D10 | crypto_pwhash_argon2i (also crypto_pwhash) | `alg` not `crypto_pwhash_argon2i_ALG_ARGON2I13` (switch `default`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D11 | crypto_pwhash_argon2i (also crypto_pwhash) | `argon2i_hash_raw(...) != ARGON2_OK` (any internal argon2 error code) | -1 (errno unchanged from internal path) | [x] `crypto_pwhash_generic_happy_matrix` |
| D12 | crypto_pwhash_argon2i_str (also crypto_pwhash_str_alg with ARGON2I13) | `passwdlen > PASSWD_MAX` OR `opslimit > OPSLIMIT_MAX` OR `memlimit > MEMLIMIT_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D13 | crypto_pwhash_argon2i_str | `passwdlen < PASSWD_MIN` OR `opslimit < OPSLIMIT_MIN` OR `memlimit < MEMLIMIT_MIN` | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D14 | crypto_pwhash_argon2i_str | `argon2i_hash_encoded(...) != ARGON2_OK` | -1 | [x] `argon2_primitive_str_cross_verify` |
| D15 | crypto_pwhash_argon2i_str_verify (also crypto_pwhash_str_verify with argon2i prefix) | `passwdlen > PASSWD_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D16 | crypto_pwhash_argon2i_str_verify | `passwdlen < PASSWD_MIN` | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D17 | crypto_pwhash_argon2i_str_verify | `argon2i_verify(...) == ARGON2_VERIFY_MISMATCH` (password does not match hash) | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D18 | crypto_pwhash_argon2i_str_verify | `argon2i_verify(...)` returns any other non-OK code (decode fail, param mismatch, etc.) | -1 (errno unchanged) | [x] `argon2_primitive_str_cross_verify` |
| D19 | crypto_pwhash_argon2i_str_needs_rehash (via _needs_rehash; also crypto_pwhash_str_needs_rehash argon2i) | `opslimit > UINT32_MAX` OR `memlimit/1024 > UINT32_MAX` OR `strlen(str) >= crypto_pwhash_STRBYTES` | errno=EINVAL and -1 | [x] `str_needs_rehash_fixed_vectors` |
| D20 | crypto_pwhash_argon2i_str_needs_rehash | `calloc(fodder_len,1)` returns NULL | -1 (errno from calloc) | [x] `argon2_primitive_str_verify_and_needs_rehash` |
| D21 | crypto_pwhash_argon2i_str_needs_rehash | `argon2_decode_string(&ctx,str,Argon2_i) != 0` (malformed stored string) | errno=EINVAL and -1 | [x] `argon2_primitive_str_verify_and_needs_rehash` |
| D22 | crypto_pwhash_argon2i_str_needs_rehash | decode OK but `ctx.t_cost != opslimit` OR `ctx.m_cost != memlimit/1024` | 1 (rehash needed) | [x] `argon2_primitive_str_verify_and_needs_rehash` |

### argon2/pwhash_argon2id.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D23 | crypto_pwhash_argon2id (also crypto_pwhash / crypto_pwhash_str default alg) | `outlen > crypto_pwhash_argon2id_BYTES_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D24 | crypto_pwhash_argon2id | `outlen < crypto_pwhash_argon2id_BYTES_MIN` (`< 16`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D25 | crypto_pwhash_argon2id | `passwdlen > PASSWD_MAX` OR `opslimit > OPSLIMIT_MAX` OR `memlimit > MEMLIMIT_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D26 | crypto_pwhash_argon2id | `passwdlen < PASSWD_MIN` OR `opslimit < OPSLIMIT_MIN` OR `memlimit < MEMLIMIT_MIN` | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D27 | crypto_pwhash_argon2id | `out` pointer aliases `passwd` pointer | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D28 | crypto_pwhash_argon2id | `alg` not `crypto_pwhash_argon2id_ALG_ARGON2ID13` (switch `default`) | errno=EINVAL and -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D29 | crypto_pwhash_argon2id | `argon2id_hash_raw(...) != ARGON2_OK` | -1 | [x] `crypto_pwhash_generic_happy_matrix` |
| D30 | crypto_pwhash_argon2id_str (also crypto_pwhash_str / crypto_pwhash_str_alg ARGON2ID13) | `passwdlen > PASSWD_MAX` OR `opslimit > OPSLIMIT_MAX` OR `memlimit > MEMLIMIT_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D31 | crypto_pwhash_argon2id_str | `passwdlen < PASSWD_MIN` OR `opslimit < OPSLIMIT_MIN` OR `memlimit < MEMLIMIT_MIN` | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D32 | crypto_pwhash_argon2id_str | `argon2id_hash_encoded(...) != ARGON2_OK` | -1 | [x] `argon2_primitive_str_cross_verify` |
| D33 | crypto_pwhash_argon2id_str_verify (also crypto_pwhash_str_verify argon2id prefix) | `passwdlen > PASSWD_MAX` | errno=EFBIG and -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D34 | crypto_pwhash_argon2id_str_verify | `passwdlen < PASSWD_MIN` | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D35 | crypto_pwhash_argon2id_str_verify | `argon2id_verify(...) == ARGON2_VERIFY_MISMATCH` | errno=EINVAL and -1 | [x] `argon2_primitive_str_cross_verify` |
| D36 | crypto_pwhash_argon2id_str_verify | `argon2id_verify(...)` returns any other non-OK code | -1 (errno unchanged) | [x] `argon2_primitive_str_cross_verify` |
| D37 | crypto_pwhash_argon2id_str_needs_rehash (via _needs_rehash; also crypto_pwhash_str_needs_rehash argon2id) | `opslimit > UINT32_MAX` OR `memlimit/1024 > UINT32_MAX` OR `strlen(str) >= crypto_pwhash_STRBYTES` | errno=EINVAL and -1 | [x] `str_needs_rehash_fixed_vectors` |
| D38 | crypto_pwhash_argon2id_str_needs_rehash | `calloc` returns NULL | -1 | [x] `argon2_primitive_str_verify_and_needs_rehash` |
| D39 | crypto_pwhash_argon2id_str_needs_rehash | `argon2_decode_string(&ctx,str,Argon2_id) != 0` (malformed string) | errno=EINVAL and -1 | [x] `argon2_primitive_str_verify_and_needs_rehash` |
| D40 | crypto_pwhash_argon2id_str_needs_rehash | decode OK but `ctx.t_cost != opslimit` OR `ctx.m_cost != memlimit/1024` | 1 (rehash needed) | [x] `argon2_primitive_str_verify_and_needs_rehash` |

### argon2/argon2.c (internal wrapper — error codes surfaced to public wrappers above)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D41 | argon2_hash (reached via crypto_pwhash_argon2{i,id} / _str) | `pwdlen > ARGON2_MAX_PWD_LENGTH` | ARGON2_PWD_TOO_LONG; wrapper converts to -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D42 | argon2_hash | `hashlen > ARGON2_MAX_OUTLEN` | ARGON2_OUTPUT_TOO_LONG; wrapper -> -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D43 | argon2_hash | `saltlen > ARGON2_MAX_SALT_LENGTH` | ARGON2_SALT_TOO_LONG; wrapper -> -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D44 | argon2_hash | `malloc(hashlen)` returns NULL | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D45 | argon2_hash | `argon2_ctx(...) != ARGON2_OK` (propagated validation/type/init error) | that code; wrapper -> -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D46 | argon2_hash | encoding requested and `argon2_encode_string(...) != ARGON2_OK` | ARGON2_ENCODING_FAIL; wrapper -> -1 | [x] `argon2_internal_hash_raw_and_encoded` |
| D47 | argon2_ctx (via argon2_hash) | `argon2_validate_inputs(...) != ARGON2_OK` (propagates first failing validation, see D53–D71) | validation code; wrapper -> -1 | [x] `argon2_internal_ctx` |
| D48 | argon2_ctx | `type != Argon2_id && type != Argon2_i` | ARGON2_INCORRECT_TYPE; wrapper -> -1 | [x] `argon2_internal_ctx` |
| D49 | argon2_ctx | `argon2_initialize(...) != ARGON2_OK` (memory allocation failure) | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_ctx` |
| D50 | argon2_verify (via crypto_pwhash_*_str_verify) | `strlen(encoded) > UINT32_MAX` | ARGON2_DECODING_LENGTH_FAIL; wrapper -> -1 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| D51 | argon2_verify | one of the `malloc` calls (ctx.ad/salt/out or out) returns NULL | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_verify` |
| D52 | argon2_verify | `argon2_decode_string(...) != ARGON2_OK` (malformed encoded hash) | decode code (e.g. ARGON2_DECODING_FAIL); wrapper -> -1 | [x] `argon2_internal_verify` |
| D53 | argon2_verify | recomputed hash differs from stored (`sodium_memcmp != 0`) | ARGON2_VERIFY_MISMATCH; wrapper sets errno=EINVAL, -1 | [x] `argon2_internal_verify` |

### argon2/argon2-core.c — argon2_validate_inputs (reached via argon2_ctx and argon2_decode_string)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D54 | argon2_validate_inputs | `context == NULL` | ARGON2_INCORRECT_PARAMETER; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D55 | argon2_validate_inputs | `context->out == NULL` | ARGON2_OUTPUT_PTR_NULL; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D56 | argon2_validate_inputs | `outlen < ARGON2_MIN_OUTLEN` (`< 16`) | ARGON2_OUTPUT_TOO_SHORT; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D57 | argon2_validate_inputs | `outlen > ARGON2_MAX_OUTLEN` | ARGON2_OUTPUT_TOO_LONG; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D58 | argon2_validate_inputs | `pwd == NULL && pwdlen != 0` | ARGON2_PWD_PTR_MISMATCH; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D59 | argon2_validate_inputs | `pwdlen < ARGON2_MIN_PWD_LENGTH` | ARGON2_PWD_TOO_SHORT; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D60 | argon2_validate_inputs | `pwdlen > ARGON2_MAX_PWD_LENGTH` | ARGON2_PWD_TOO_LONG; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D61 | argon2_validate_inputs | `salt == NULL && saltlen != 0` | ARGON2_SALT_PTR_MISMATCH; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D62 | argon2_validate_inputs | `saltlen < ARGON2_MIN_SALT_LENGTH` | ARGON2_SALT_TOO_SHORT; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D63 | argon2_validate_inputs | `saltlen > ARGON2_MAX_SALT_LENGTH` | ARGON2_SALT_TOO_LONG; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D64 | argon2_validate_inputs | `secret == NULL && secretlen != 0` | ARGON2_SECRET_PTR_MISMATCH; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D65 | argon2_validate_inputs | `secret != NULL && secretlen < ARGON2_MIN_SECRET` | ARGON2_SECRET_TOO_SHORT; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D66 | argon2_validate_inputs | `secret != NULL && secretlen > ARGON2_MAX_SECRET` | ARGON2_SECRET_TOO_LONG; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D67 | argon2_validate_inputs | `ad == NULL && adlen != 0` | ARGON2_AD_PTR_MISMATCH; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D68 | argon2_validate_inputs | `ad != NULL && adlen < ARGON2_MIN_AD_LENGTH` | ARGON2_AD_TOO_SHORT; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D69 | argon2_validate_inputs | `ad != NULL && adlen > ARGON2_MAX_AD_LENGTH` | ARGON2_AD_TOO_LONG; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D70 | argon2_validate_inputs | `lanes < ARGON2_MIN_LANES` (`< 1`) | ARGON2_LANES_TOO_FEW; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D71 | argon2_validate_inputs | `lanes > ARGON2_MAX_LANES` (`> 0xFFFFFF`) | ARGON2_LANES_TOO_MANY; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D72 | argon2_validate_inputs | `m_cost < ARGON2_MIN_MEMORY` | ARGON2_MEMORY_TOO_LITTLE; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D73 | argon2_validate_inputs | `m_cost > ARGON2_MAX_MEMORY` | ARGON2_MEMORY_TOO_MUCH; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D74 | argon2_validate_inputs | `m_cost < 8 * lanes` | ARGON2_MEMORY_TOO_LITTLE; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D75 | argon2_validate_inputs | `t_cost < ARGON2_MIN_TIME` (`< 1`) | ARGON2_TIME_TOO_SMALL; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D76 | argon2_validate_inputs | `t_cost > ARGON2_MAX_TIME` | ARGON2_TIME_TOO_LARGE; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D77 | argon2_validate_inputs | `threads < ARGON2_MIN_THREADS` (`< 1`) | ARGON2_THREADS_TOO_FEW; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D78 | argon2_validate_inputs | `threads > ARGON2_MAX_THREADS` (`> 0xFFFFFF`) | ARGON2_THREADS_TOO_MANY; wrapper -> -1 | [x] `sodium_argon2_validate_inputs` |
| D79 | allocate_memory (via argon2_initialize/argon2_ctx) | `m_cost == 0` OR size multiply overflow (`memory_size / m_cost != sizeof(block)`) | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_ctx` |
| D80 | allocate_memory | mmap/posix_memalign/malloc allocation fails (`base == NULL`) | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_ctx` |
| D81 | argon2_initialize | `malloc(sizeof(uint64_t)*segment_length)` for pseudo_rands returns NULL | ARGON2_MEMORY_ALLOCATION_ERROR; wrapper -> -1 | [x] `argon2_internal_ctx` |

### argon2/argon2-encoding.c — argon2_decode_string (reached via crypto_pwhash_*_str_verify and _str_needs_rehash)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D82 | argon2_decode_string (via *_str_verify) | wrong type-prefix: for Argon2_id, string does not start with `$argon2id`; for Argon2_i, does not start with `$argon2i` (CC macro) | ARGON2_DECODING_FAIL; verify wrapper -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D83 | argon2_decode_string | missing/incorrect `$v=` literal (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D84 | argon2_decode_string | version field not a valid minimal decimal, or `> UINT32_MAX` (DECIMAL_U32 -> decode_decimal NULL) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D85 | argon2_decode_string | parsed `version != ARGON2_VERSION_NUMBER` (0x13) | ARGON2_INCORRECT_TYPE; verify wrapper -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D86 | argon2_decode_string | missing/incorrect `$m=` literal (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D87 | argon2_decode_string | m_cost field not valid minimal decimal or `> UINT32_MAX` | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D88 | argon2_decode_string | missing/incorrect `,t=` literal (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D89 | argon2_decode_string | t_cost field not valid minimal decimal or `> UINT32_MAX` | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D90 | argon2_decode_string | missing/incorrect `,p=` literal (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D91 | argon2_decode_string | lanes/p field not valid minimal decimal or `> UINT32_MAX` | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D92 | argon2_decode_string | missing `$` separator before salt (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D93 | argon2_decode_string | salt base64 invalid, or decoded length `> maxsaltlen`, or `> UINT32_MAX` (BIN macro / sodium_base642bin != 0) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D94 | argon2_decode_string | missing `$` separator before output hash (CC macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D95 | argon2_decode_string | output base64 invalid, or decoded length `> maxoutlen`, or `> UINT32_MAX` (BIN macro) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D96 | argon2_decode_string | after both binaries decoded, `argon2_validate_inputs(ctx) != ARGON2_OK` (decoded params out of range, e.g. salt/out too short, m_cost/t_cost/lanes out of bounds) | that validation code; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D97 | argon2_decode_string | trailing garbage: `*str != 0` after output (string not fully consumed) | ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |
| D98 | decode_decimal (via DECIMAL_U32 in decode_string) | no digit present, or non-minimal encoding (leading zero like `00`/`01`), or accumulator overflow `> ULONG_MAX` | NULL -> caller returns ARGON2_DECODING_FAIL; -> -1 | [x] `sodium_argon2_encode_decode_roundtrip` |

### argon2/blake2b-long.c (reached via argon2 hashing / finalize)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D99 | blake2b_long (via argon2_finalize) | `outlen > UINT32_MAX` | returns -1 (init ret) | [x] `sodium_blake2b_long` |
| D100 | blake2b_long | any `crypto_generichash_blake2b_{init,update,final}` returns `< 0` (TRY macro) | returns that negative ret (-1) | [x] `sodium_blake2b_long` |

### scryptsalsa208sha256/pwhash_scryptsalsa208sha256.c

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D101 | crypto_pwhash_scryptsalsa208sha256 | `passwdlen > PASSWD_MAX` OR `outlen > BYTES_MAX` | errno=EFBIG and -1 | [x] `scrypt_ll_matrix` |
| D102 | crypto_pwhash_scryptsalsa208sha256 | `outlen < BYTES_MIN` OR `pickparams(...) != 0` | errno=EINVAL and -1 | [x] `scrypt_ll_matrix` |
| D103 | crypto_pwhash_scryptsalsa208sha256 | `out` pointer aliases `passwd` pointer | errno=EINVAL and -1 | [x] `scrypt_ll_matrix` |
| D104 | crypto_pwhash_scryptsalsa208sha256 | underlying `crypto_pwhash_scryptsalsa208sha256_ll` fails (escrypt_kdf error, e.g. r*p overflow, N not power of 2, alloc fail) | returns -1 (errno set by escrypt_kdf: EFBIG/EINVAL/ENOMEM) | [x] `scrypt_ll_matrix` |
| D105 | crypto_pwhash_scryptsalsa208sha256_str | `passwdlen > PASSWD_MAX` | errno=EFBIG and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D106 | crypto_pwhash_scryptsalsa208sha256_str | `passwdlen < PASSWD_MIN` OR `pickparams(...) != 0` | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D107 | crypto_pwhash_scryptsalsa208sha256_str | `escrypt_gensalt_r(...) == NULL` | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D108 | crypto_pwhash_scryptsalsa208sha256_str | `escrypt_init_local(...) != 0` | -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D109 | crypto_pwhash_scryptsalsa208sha256_str | `escrypt_r(...) == NULL` (KDF/encode failure) | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D110 | crypto_pwhash_scryptsalsa208sha256_str_verify | `sodium_strnlen(str, STRBYTES) != STRBYTES-1` (string not exactly STRBYTES-1 long: too short, too long, or embedded NUL) | -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D111 | crypto_pwhash_scryptsalsa208sha256_str_verify | `escrypt_init_local(...) != 0` | -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D112 | crypto_pwhash_scryptsalsa208sha256_str_verify | `escrypt_r(...) == NULL` (malformed setting/salt: bad `$7$` prefix, bad base64 params, salt too long) | -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D113 | crypto_pwhash_scryptsalsa208sha256_str_verify | recomputed string differs from `str` (`sodium_memcmp != 0`, wrong password) | non-zero (from sodium_memcmp), i.e. verification failure | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D114 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | `pickparams(...) != 0` | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D115 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | `sodium_strnlen(str, STRBYTES) != STRBYTES-1` | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D116 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | `escrypt_parse_setting(...) == NULL` (malformed setting) | errno=EINVAL and -1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| D117 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | parse OK but `N_log2 != N_log2_` OR `r != r_` OR `p != p_` | 1 (rehash needed) | [x] `scrypt_str_roundtrip_and_needs_rehash` |

### scryptsalsa208sha256/crypto_scrypt-common.c (static helpers; reached via scrypt _str / _str_verify / _str_needs_rehash)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D118 | escrypt_parse_setting (via _str_verify / _str_needs_rehash) | `setting[0]!='$' || setting[1]!='7' || setting[2]!='$'` (bad `$7$` prefix) | NULL -> caller -1 | [x] `sodium_escrypt_parse_setting` |
| D119 | escrypt_parse_setting | `decode64_one(N_log2)` fails on 4th char (invalid base64-itoa64 char for N_log2) | NULL -> caller -1 | [x] `sodium_escrypt_parse_setting` |
| D120 | escrypt_parse_setting | `decode64_uint32(r,30,...)` fails (invalid char in r field) | NULL -> caller -1 | [x] `sodium_escrypt_parse_setting` |
| D121 | escrypt_parse_setting | `decode64_uint32(p,30,...)` fails (invalid char in p field) | NULL -> caller -1 | [x] `sodium_escrypt_parse_setting` |
| D122 | escrypt_r (via _str / _str_verify) | `escrypt_parse_setting(...) == NULL` (bad setting) | NULL -> caller -1 | [x] `escrypt_r_internal` |
| D123 | escrypt_r | `buf == NULL` OR `need > buflen` OR `need < saltlen` (overflow) — output buffer too small / salt too long | NULL -> caller -1 | [x] `escrypt_r_internal` |
| D124 | escrypt_r | `escrypt_kdf(...)` returns non-zero (KDF error) | NULL -> caller -1 | [x] `escrypt_r_internal` |
| D125 | escrypt_r | final encode `dst == NULL` OR `dst >= buf+buflen` (encode overflow, "can't happen") | NULL -> caller -1 | [x] `escrypt_r_internal` |
| D126 | escrypt_gensalt_r (via _str) | `need > buflen` OR `need < saltlen` OR `saltlen < srclen` (overflow) | NULL -> caller -1 | [x] `sodium_escrypt_gensalt_r` |
| D127 | escrypt_gensalt_r | `N_log2 > 63` OR `r*p >= 2^30` | NULL -> caller -1 | [x] `sodium_escrypt_gensalt_r` |

### scryptsalsa208sha256/nosse/pwhash_scryptsalsa208sha256_nosse.c — escrypt_kdf_nosse (reached via crypto_pwhash_scryptsalsa208sha256[_ll])

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D128 | escrypt_kdf_nosse (via crypto_pwhash_scryptsalsa208sha256_ll) | `buflen > (2^32 - 1) * 32` (on 64-bit) | errno=EFBIG and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D129 | escrypt_kdf_nosse | `(uint64_t)r * p >= 2^30` | errno=EFBIG and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D130 | escrypt_kdf_nosse | `N > UINT32_MAX` | errno=EFBIG and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D131 | escrypt_kdf_nosse | `N` not a power of two (`N & (N-1) != 0`) OR `N < 2` | errno=EINVAL and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D132 | escrypt_kdf_nosse | `r == 0` OR `p == 0` | errno=EINVAL and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D133 | escrypt_kdf_nosse | `r > SIZE_MAX/128/p` OR `r > SIZE_MAX/256` OR `N > SIZE_MAX/128/r` (size overflow) | errno=ENOMEM and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D134 | escrypt_kdf_nosse | `need < V_size` (B_size+V_size overflow) | errno=ENOMEM and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D135 | escrypt_kdf_nosse | `need < XY_size` (adding XY_size overflows) | errno=ENOMEM and -1 | [x] `sodium_escrypt_kdf_nosse` |
| D136 | escrypt_kdf_nosse | `escrypt_free_region(local) != 0` (region free error during realloc) | -1 | [x] `sodium_escrypt_kdf_nosse` |
| D137 | escrypt_kdf_nosse | `escrypt_alloc_region(local,need)` returns NULL (allocation failure) | -1 | [x] `sodium_escrypt_kdf_nosse` |

### scryptsalsa208sha256/pbkdf2-sha256.c (reached via escrypt_kdf)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D138 | escrypt_PBKDF2_SHA256 (via escrypt_kdf) | on 64-bit builds, `dkLen > 0x1fffffffe0` | SIGABRT via sodium_misuse() | [x] `sodium_escrypt_gensalt_r` |

### scryptsalsa208sha256/crypto_scrypt-common.c — encode64 helpers (defensive, reached via escrypt_r/gensalt_r)

| # | function | trigger (the exact invalid input/condition) | expected C result | covered by test |
|---|---|---|---|---|
| D139 | encode64_uint32 (via escrypt_r/gensalt_r) | `dstlen < 1` during base64 emit (output buffer exhausted) | NULL -> propagates to caller -> -1 | [x] `sodium_escrypt_gensalt_r` |
| D140 | encode64 (via escrypt_r/gensalt_r) | inner `encode64_uint32` returns NULL (buffer exhausted) | NULL -> propagates to caller -> -1 | [x] `sodium_escrypt_gensalt_r` |
| D141 | decode64_uint32 (via escrypt_parse_setting) | `decode64_one` fails on any char of the field (invalid itoa64 char) | NULL -> escrypt_parse_setting NULL -> caller -1 | [x] `sodium_escrypt_parse_setting` |

