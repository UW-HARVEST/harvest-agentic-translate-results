| sodium-E1 | sodium_mlock | no HAVE_MLOCK / WINAPI_DESKTOP in this build (unconditional) | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E2 | sodium_munlock | no HAVE_MLOCK / WINAPI_DESKTOP in this build (unconditional; buffer is still zeroed first) | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E3 | sodium_mprotect_noaccess | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E4 | sodium_mprotect_readonly | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E5 | sodium_mprotect_readwrite | `_sodium_mprotect`: HAVE_PAGE_PROTECTION not defined | returns -1, errno = ENOSYS (38) | [x] |
| sodium-E6 | _mprotect_noaccess, _mprotect_readonly, _mprotect_readwrite (static callbacks) | no HAVE_MPROTECT: each sets errno = ENOSYS and returns -1; in this build `_sodium_mprotect` never invokes the callback, so they are only reachable as function pointers | returns -1, errno = ENOSYS | [n/a] |
| sodium-E7 | sodium_malloc | `_sodium_malloc()` (i.e. `malloc()`) returned NULL | returns NULL | [x] |
| sodium-E8 | sodium_allocarray | `count > 0 && size >= SIZE_MAX / count` | returns NULL, errno = ENOMEM (12) | [x] |
| sodium-E9 | sodium_free | `ptr == NULL` (not compiled in this build: the non-HAVE_ALIGNED_MALLOC body is a bare `free(ptr)`, which tolerates NULL) | no-op | [x] |
| sodium-E10 | sodium_pad | `blocksize <= 0U` | returns -1, buffer and *padded_buflen_p untouched | [x] |
| sodium-E11 | sodium_pad | `(size_t) SIZE_MAX - unpadded_buflen <= xpadlen` (integer-overflow guard) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E12 | sodium_pad | `xpadded_len >= max_buflen` (output buffer too small) | returns -1, buffer and *padded_buflen_p untouched | [x] |
| sodium-E13 | sodium_unpad | `padded_buflen < blocksize` (incl. padded_buflen == 0) | returns -1, *unpadded_buflen_p NOT written | [x] |
| sodium-E14 | sodium_unpad | `blocksize <= 0U` | returns -1, *unpadded_buflen_p NOT written | [x] |
| sodium-E15 | sodium_unpad | no 0x80 barrier found in the last `blocksize` bytes (`valid == 0`) | returns -1, but *unpadded_buflen_p IS written (unconditional store before the return) | [x] |
| sodium-E16 | _sodium_alloc_init | `page_size < CANARY_SIZE \|\| page_size < sizeof(size_t)` → sodium_misuse() | not compiled (inside `#ifdef HAVE_ALIGNED_MALLOC`); the surviving body only calls randombytes_buf and returns 0 | [n/a] |
| sodium-E17 | sodium_bin2hex | `bin_len >= SIZE_MAX / 2` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E18 | sodium_bin2hex | `hex_maxlen <= bin_len * 2U` (output buffer too small, incl. hex_maxlen == 0) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E19 | sodium_hex2bin | `bin_pos >= bin_maxlen` (output buffer too small) | ret = -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E20 | sodium_hex2bin | odd number of hex digits consumed (`state != 0`) | ret = -1, errno = EINVAL (22), *bin_len = 0, *hex_end backed up by one character | [x] |
| sodium-E21 | sodium_hex2bin | non-hex character that is not in `ignore` (or `ignore == NULL`, or `state != 0`) | loop breaks; success unless hex_end == NULL (→ E22) | [x] |
| sodium-E22 | sodium_hex2bin | `hex_end == NULL && hex_pos != hex_len` (unconsumed input and no way to report it) | ret = -1, errno = EINVAL (22); *bin_len keeps the already-decoded count (it is zeroed *before* this check) | [x] |
| sodium-E23 | sodium_base64_check_variant (from sodium_base64_encoded_len) | `(((unsigned) variant) & ~0x6U) != 0x1U` — tested with variant −1, 0, 2, 4, 6, 8, 9, 99, INT_MAX | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E24 | sodium_base64_check_variant (from sodium_bin2base64) | same out-of-range variant check | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E25 | sodium_base64_check_variant (from sodium_base642bin) | same out-of-range variant check | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E26 | sodium_base64_encoded_len | `bin_len / 3 > (SIZE_MAX - 5) / 4` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E27 | sodium_base64_ENCODED_LEN macro | `(BIN_LEN)/3U > (SIZE_MAX-5)/4U` → `(size_t) SIZE_MAX` | unreachable through sodium_base64_encoded_len (E26 aborts first); no separate test | [n/a] |
| sodium-E28 | sodium_bin2base64 | `nibbles > (SIZE_MAX - 5) / 4` | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E29 | sodium_bin2base64 | `b64_maxlen <= b64_len` (output buffer too small, incl. b64_maxlen == 0) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E30 | sodium_bin2base64 | `assert(b64_pos <= b64_len)` (live: the reference build defines no NDEBUG) | abort — provably unreachable (b64_pos is always ≤ b64_len); Rust reproduces the abort-on-failure by inspection | [abort] |
| sodium-E31 | _sodium_base642bin_skip_padding | `*b64_pos_p >= b64_len` while padding characters are still required (truncated `=` padding) | returns -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E32 | _sodium_base642bin_skip_padding | character is not `'='` and (`ignore == NULL` or not in `ignore`) | returns -1, errno = EINVAL (22), *bin_len = 0 | [x] |
| sodium-E33 | sodium_base642bin | `bin_pos >= bin_maxlen` (output buffer too small) | ret = -1, errno = ERANGE (34), *bin_len = 0 | [x] |
| sodium-E34 | sodium_base642bin | `acc_len > 4U` (dangling 6-bit group) | ret = -1, errno left untouched, *bin_len = 0 | [x] |
| sodium-E35 | sodium_base642bin | `(acc & ((1U << acc_len) - 1U)) != 0U` (non-canonical encoding: leftover bits set) | ret = -1, errno left untouched, *bin_len = 0 | [x] |
| sodium-E36 | sodium_base642bin | invalid base64 character (`d == 0xFF`) not in `ignore` | loop breaks; success unless b64_end == NULL (→ E37) | [x] |
| sodium-E37 | sodium_base642bin | `b64_end == NULL && b64_pos != b64_len` | ret = -1, errno = EINVAL (22); *bin_len keeps the already-decoded count | [x] |
| sodium-E38 | ip_hex_digit | character is not `[0-9a-fA-F]` | returns -1 (drives parse_ipv6 → E48) | [x] |
| sodium-E39 | parse_ipv4 | `src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end` (empty input) | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E40 | parse_ipv4 | `++digits > 3` (more than 3 digits in an octet, e.g. "0000.1.1.1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E41 | parse_ipv4 | `val > 255U` (octet out of range, e.g. "256.1.1.1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E42 | parse_ipv4 | `digits == 0` (empty octet, e.g. "1..2.3", ".1.2.3") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E43 | parse_ipv4 | missing `'.'` separator: `i < 3 && (p >= end \|\| *p++ != '.')` (e.g. "1.2.3") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E44 | parse_ipv4 | `p != end` after 4 octets (trailing junk, e.g. "1.2.3.4.5", "1.2.3.4 ") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E45 | parse_ipv6 | `src == NULL \|\| end == NULL \|\| out == NULL \|\| src >= end` | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E46 | parse_ipv6 | leading single `':'` not followed by another `':'` (e.g. ":1") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E47 | parse_ipv6 | a second `"::"` run (`colonp != NULL` when `!saw_xdigit`), e.g. "1:::2", "::1::2" | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E48 | parse_ipv6 | `hv < 0` (non-hex character) or `xdigits >= 4` (group longer than 4 hex digits, e.g. "12345::") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E49 | parse_ipv6 | `tp + 2 > endp` when flushing a group (more than 8 groups, e.g. "1:2:3:4:5:6:7:8:9") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E50 | parse_ipv6 | trailing `':'` (`p >= end` right after a group separator, e.g. "1:") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E51 | parse_ipv6 | embedded IPv4: `tp + 4 > endp \|\| parse_ipv4(curtok, end, tp) == 0` (e.g. "::1.2.3", "1:2:3:4:5:6:7:1.2.3.4") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E52 | parse_ipv6 | final `tp + 2 > endp` for the last group | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E53 | parse_ipv6 | `colonp != NULL && tp == endp` (`"::"` present but the address is already full, e.g. "::1:2:3:4:5:6:7:8") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E54 | parse_ipv6 | `tp != endp` (fewer than 8 groups and no `"::"`, e.g. "1:2:3:4:5:6:7") | returns 0 → sodium_ip2bin returns -1 | [x] |
| sodium-E55 | sodium_ip2bin | zone-id contains a character outside `[0-9a-zA-Z._-]` (e.g. "fe80::1%!") | returns -1 | [x] |
| sodium-E56 | sodium_ip2bin | empty zone-id (`zone + 1 >= end`, e.g. "fe80::1%") | returns -1 | [x] |
| sodium-E57 | sodium_ip2bin | zone-id present but the address is not IPv6 (`zone != NULL && !is_ipv6`, e.g. "1.2.3.4%eth0", "%eth0") | returns -1 | [x] |
| sodium-E58 | sodium_ip2bin | `parse_ipv6()` returned 0 | returns -1 (bin untouched) | [x] |
| sodium-E59 | sodium_ip2bin | `parse_ipv4()` returned 0 | returns -1 (bin untouched) | [x] |
| sodium-E60 | sodium_bin2ip | `ip_maxlen <= 2U` | returns NULL, `ip` untouched | [x] |
| sodium-E61 | sodium_bin2ip | IPv4-mapped branch: `len >= ip_maxlen` (formatted dotted-quad does not fit) | returns NULL, `ip` untouched | [x] |
| sodium-E62 | sodium_bin2ip | IPv6 branch: `len >= ip_maxlen` (formatted address does not fit) | returns NULL, `ip` untouched | [x] |
| sodium-E63 | sodium_init | `sodium_crit_enter() != 0` | returns -1 — unreachable in this build (crit_enter is the no-op version that always returns 0) | [n/a] |
| sodium-E64 | sodium_init | `sodium_crit_leave() != 0` (both call sites) | returns -1 — unreachable in this build (crit_leave always returns 0) | [n/a] |
| sodium-E65 | sodium_init | `initialized != 0` (already initialized) | returns 1 | [x] |
| sodium-E66 | sodium_crit_leave | `locked == 0` → errno = EPERM, return -1 | not compiled (only in the _WIN32 / HAVE_PTHREAD variants); the compiled version unconditionally returns 0 | [n/a] |
| sodium-E67 | sodium_misuse | unconditional `abort()` after (optionally) calling the misuse handler | abort — verified in a forked child (SIGABRT), and the handler-is-called path verified by a child that exits 42 from the handler | [x] |
| sodium-E68 | sodium_set_misuse_handler | `sodium_crit_enter() != 0` / `sodium_crit_leave() != 0` | returns -1 — unreachable in this build | [n/a] |
| sodium-E69 | _sodium_runtime_arm_cpu_features | `#ifndef __ARM_ARCH` → unconditional `return -1` on x86-64 | returns -1 (has_neon/has_armcrypto left 0) | [x] |
| sodium-E70 | _sodium_runtime_intel_cpu_features | `cpu_info[0] == 0U` (no HAVE_CPUID, so `_cpuid` zeroes the array) | returns -1; every has_* field keeps its zero value | [x] |
| sodium-E71 | _sodium_runtime_get_cpu_features | `ret = -1 & arm & intel` | returns -1 | [x] |
| sodium-E72 | randombytes_uniform | `upper_bound < 2` (0 or 1), only when `implementation->uniform == NULL` | returns 0 | [x] |
| sodium-E73 | randombytes_buf | `size == 0` | `implementation->buf` is NOT called; buffer untouched | [x] |
| sodium-E74 | randombytes_buf_deterministic | `size > 0x4000000000ULL` (randombytes_BYTES_MAX) | `sodium_misuse()` → abort — verified in a forked child (SIGABRT) | [x] |
| sodium-E75 | randombytes_close | `implementation == NULL` or `implementation->close == NULL` | returns 0 | [x] |
| sodium-E76 | randombytes_stir | `implementation->stir == NULL` | no call (no-op) | [x] |
| sodium-E77 | randombytes | `assert(buf_len <= SIZE_MAX)` | abort — unreachable on this target (both types are 64-bit) | [abort] |
| sodium-E78 | safe_read (sysrandom + internal) | `assert(size > 0U)`, `assert(size <= SSIZE_MAX)` | abort — unreachable in this build (the getrandom path is always taken, so safe_read is never called) | [abort] |
| sodium-E79 | safe_read (sysrandom + internal) | `read()` returned < 0 → return readnb; returned 0 → break with a short count | short/negative result → caller calls sodium_misuse() | [n/a] |
| sodium-E80 | randombytes_block_on_dev_random | `open("/dev/random")` failed | returns 0 (treated as "don't block") | [n/a] |
| sodium-E81 | randombytes_block_on_dev_random | `poll()` returned != 1 | errno = EIO (5), returns -1 → dev_open returns -1 | [n/a] |
| sodium-E82 | randombytes_sysrandom_random_dev_open / randombytes_internal_random_random_dev_open | all devices in the list exhausted | errno = EIO (5), returns -1 | [n/a] |
| sodium-E83 | _randombytes_linux_getrandom (sysrandom + internal) | `assert(size <= 256U)` | abort — unreachable (the caller chunks to ≤ 256); Rust reproduces the check | [abort] |
| sodium-E84 | _randombytes_linux_getrandom (sysrandom + internal) | `readnb != (int) size` (short getrandom) | returns -1 | [n/a] |
| sodium-E85 | randombytes_linux_getrandom (sysrandom) | `assert(chunk_size > 0U)` — REACHABLE: `randombytes_sysrandom_implementation.buf(p, 0)` called directly (randombytes_buf() itself guards size > 0) | abort — verified in a forked child (SIGABRT in both libraries); **this was a real divergence, now fixed** (see report) | [x] |
| sodium-E86 | randombytes_linux_getrandom (internal) | `assert(chunk_size > 0U)` | abort — unreachable (only called with 16 and 32) | [abort] |
| sodium-E87 | randombytes_sysrandom_init | `randombytes_sysrandom_random_dev_open() == -1` | `sodium_misuse()` → abort — unreachable here (getrandom(2) succeeds on this kernel, so the function returns early) | [abort] |
| sodium-E88 | randombytes_sysrandom_close | `random_data_source_fd == -1` and `getrandom_available == 0` | returns -1 - unreachable once sodium_init() has stirred sysrandom (getrandom_available becomes 1 and never goes back to 0); the analogous internal-RNG path IS tested, see E96 | [n/a] |
| sodium-E89 | randombytes_sysrandom_buf | `randombytes_linux_getrandom() != 0` | `sodium_misuse()` → abort — not reachable with a working getrandom(2) | [abort] |
| sodium-E90 | randombytes_sysrandom_buf | `random_data_source_fd == -1 \|\| safe_read(...) != size` | `sodium_misuse()` → abort — not reachable (getrandom path returns first) | [abort] |
| sodium-E91 | sodium_hrtime (internal) | `gettimeofday() != 0` | `sodium_misuse()` → abort — not reachable | [abort] |
| sodium-E92 | randombytes_internal_random_init | `assert((getentropy_available \| getrandom_available) == 0)` | abort — unreachable (the getrandom branch returns before it) | [abort] |
| sodium-E93 | randombytes_internal_random_init | `randombytes_internal_random_random_dev_open() == -1` | `sodium_misuse()` → abort — unreachable (getrandom succeeds) | [abort] |
| sodium-E94 | randombytes_internal_random_stir | `assert(stream.nonce != 0)` | abort — unreachable (gettimeofday-derived microsecond clock is never 0) | [abort] |
| sodium-E95 | randombytes_internal_random_stir | `randombytes_linux_getrandom(stream.key, 32) != 0` | `sodium_misuse()` → abort — not reachable | [abort] |
| sodium-E96 | randombytes_internal_random_close | `getrandom_available == 0` (close before any stir) | returns -1; after a stir it returns 0 | [x] |
| sodium-E97 | randombytes_internal_random_buf / randombytes_internal_random | `assert(ret == 0)` on the crypto_stream_chacha20 result | abort — unreachable (chacha20 always returns 0) | [abort] |
| sodium-E98 | crypto_ipcrypt_* (all of `crypto_ipcrypt.c` and `ipcrypt_soft.c`) | no rejection sites at all: every entry point returns `void` or a fixed `size_t`, and there are no length/range checks, `assert`s or `sodium_misuse()` calls | n/a — nothing to reject; verified by exhaustive grep | [x] |
| sodium-E99 | _crypto_ipcrypt_pick_best_implementation | the HAVE_ARMCRYPTO and HAVE_AVXINTRIN_H+HAVE_WMMINTRIN_H early-return branches are not compiled | always returns 0 with the soft backend selected | [x] |
