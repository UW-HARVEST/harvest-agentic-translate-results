# ERRORS.md — error-surface table (Phase A / Phase C)

One row per distinct rejection branch in the C source, derived mechanically
by grepping every `return -1`, `return NULL`, error enum, `sodium_misuse()`,
`assert`, explicit range/null check and min/max constant in `c_src/libsodium`.
The `status` column records the Phase-C outcome for that row.

Build under test: x86-64 Linux, **no `HAVE_*` macros** (see c_src/CMakeLists.txt),
so every `#ifdef HAVE_*` selects the portable fallback.

## Row counts

| area | rows |
|------|------|
| 1 | 132 |
| 2 | 52 |
| 3 | 110 |
| 4 | 24 |
| 5 | 29 |
| 6 | 84 |
| 7 | 129 |
| 8 | 214 |
| **total** | **774** |


## Area 1 — sodium core + randombytes

Build assumed: x86-64 Linux, CMake defines **no** `HAVE_*` macros, `-std=c99`, **no `-DNDEBUG`** (so `assert()` is live).
Derived macros that *are* active: `__linux__`, `__x86_64__`, `HAVE_LINUX_COMPATIBLE_GETRANDOM` (derived from `__linux__` + `SYS_getrandom`/`__NR_getrandom` via `syscall()`), `BLOCK_ON_DEV_RANDOM`, `DEFAULT_PAGE_SIZE 0x10000`, `ENOSYS`.
Derived macros that are **NOT** active: `HAVE_ALIGNED_MALLOC`, `HAVE_PAGE_PROTECTION`, `HAVE_MPROTECT`, `HAVE_MLOCK`, `HAVE_MMAP`/`MAP_ANON`, `HAVE_POSIX_MEMALIGN`, `HAVE_WEAK_SYMBOLS`, `HAVE_AMD64_ASM`, `HAVE_CPUID`, `HAVE_RDRAND`, `HAVE_GETPID`, `HAVE_GETENTROPY`, `HAVE_PTHREAD`, `HAVE_ATOMIC_OPS`, `HAVE_C_VARARRAYS`/`HAVE_ALLOCA`, `WINAPI_DESKTOP`, `NONEXISTENT_DEV_RANDOM`, `HAVE_SAFE_ARC4RANDOM`, `TLS` (empty → `stream` in randombytes_internal_random.c is a plain global, **not** thread-local).

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 1.1 | `sodium_crit_enter` | any call — `#else` no-op branch is compiled (no `_WIN32`, no `HAVE_PTHREAD`, no `HAVE_ATOMIC_OPS`) | never fails: always `0`; the `assert(locked == 0)` and `errno=EPERM` paths are **not compiled** | verified |
| 1.2 | `sodium_crit_leave` | any call, including leave-without-enter | never fails: always `0` (the `locked == 0` → `-1`/`EPERM` branch is not compiled) | verified |
| 1.3 | `sodium_init` | `sodium_crit_enter() != 0` (entry lock failure) | `-1`; **unreachable** in this build (1.1) | not-compiled-in-this-build |
| 1.4 | `sodium_init` | called a 2nd or later time (`initialized != 0`) | `1` (distinct non-error sentinel, not `0`) | verified |
| 1.5 | `sodium_init` | `sodium_crit_leave() != 0` on the already-initialized path | `-1`; **unreachable** (1.2) | not-compiled-in-this-build |
| 1.6 | `sodium_init` | `sodium_crit_leave() != 0` after first-time init | `-1`; **unreachable** (1.2) | not-compiled-in-this-build |
| 1.7 | `sodium_misuse` | reached from any misuse site | aborts (sodium_misuse) — calls the installed `_misuse_handler` first (if non-NULL), then unconditional `abort()`; `noreturn` | verified |
| 1.8 | `sodium_set_misuse_handler` | `sodium_crit_enter() != 0` | `-1`; **unreachable** (1.1) | not-compiled-in-this-build |
| 1.9 | `sodium_set_misuse_handler` | `sodium_crit_leave() != 0` after storing the handler | `-1`; **unreachable** (1.2) | not-compiled-in-this-build |
| 1.10 | `_sodium_runtime_arm_cpu_features` | `__ARM_ARCH` undefined (x86-64) → unconditional `#ifndef __ARM_ARCH return -1` | always `-1`, after setting `has_neon = has_armcrypto = 0` | verified-indirectly (static fn; observed through its public callers) |
| 1.11 | `_sodium_runtime_intel_cpu_features` | `HAVE_CPUID` undefined → `_cpuid()` zero-fills `cpu_info`, so `cpu_info[0] == 0U` | always `-1`, taken **before** any `has_*` field is assigned (all stay statically zero) | verified-indirectly (static fn; observed through its public callers) |
| 1.12 | `_sodium_runtime_get_cpu_features` | both sub-probes return `-1`; `ret = -1 & -1 & -1` | always `-1` (return value is ignored by `sodium_init`); `_cpu_features.initialized = 1` is still set | verified |
| 1.13 | `sodium_runtime_has_neon` / `_armcrypto` / `_sse2` / `_sse3` / `_ssse3` / `_sse41` / `_avx` / `_avx2` / `_avx512f` / `_pclmul` / `_aesni` / `_rdrand` | any call, before or after `sodium_init()` | all 12 always return `0` ("feature absent") in this build | verified |
| 1.14 | `sodium_memzero` | — | never fails; only the `HAVE_MEMSET_S` branch could `sodium_misuse()` and it is **not compiled**; the compiled branch is the `#else` volatile byte loop | verified |
| 1.15 | `sodium_stackzero` | — | never fails; both `HAVE_C_VARARRAYS` and `HAVE_ALLOCA` are off → **empty body, complete no-op** | verified |
| 1.16 | `sodium_memcmp` | `b1 != b2` in any byte within `len` | `-1` (0 on equality). No `errno`. `len == 0` → `0` | verified |
| 1.17 | `sodium_compare` | `b1 < b2` interpreted little-endian (highest differing index decides) | `-1`. No `errno` | verified |
| 1.18 | `sodium_compare` | `b1 > b2` little-endian | `1`. No `errno` (`len == 0` → `0`) | verified |
| 1.19 | `sodium_is_zero` | any byte in `n[0..nlen)` is non-zero | `0` (returns `1` when all-zero or `nlen == 0`) | verified |
| 1.20 | `_sodium_alloc_init` | — | never fails; the `page_size < CANARY_SIZE` `sodium_misuse()` is inside `#ifdef HAVE_ALIGNED_MALLOC`, **not compiled**. Always returns `0` after `randombytes_buf(canary, 16)` | verified |
| 1.21 | `sodium_mlock` | **any** call (`HAVE_MLOCK` and `WINAPI_DESKTOP` both off → `#else` branch) | `-1`, `errno=ENOSYS` — unconditional | verified |
| 1.22 | `sodium_munlock` | **any** call | `-1`, `errno=ENOSYS` — unconditional, but **only after `sodium_memzero(addr, len)` has already destroyed the buffer** | verified |
| 1.23 | `_sodium_mprotect` | `!HAVE_PAGE_PROTECTION` branch is compiled → unconditional | `-1`, `errno=ENOSYS` (the `cb` is never invoked, `ptr` is never dereferenced) | verified-indirectly (static fn; observed through its public callers) |
| 1.24 | `sodium_mprotect_noaccess` | **any** pointer, including a non-`sodium_malloc` pointer | `-1`, `errno=ENOSYS`; no protection is applied | verified |
| 1.25 | `sodium_mprotect_readonly` | **any** pointer | `-1`, `errno=ENOSYS`; no protection is applied | verified |
| 1.26 | `sodium_mprotect_readwrite` | **any** pointer | `-1`, `errno=ENOSYS`; no protection is applied | verified |
| 1.27 | `_mprotect_noaccess` / `_mprotect_readonly` / `_mprotect_readwrite` | each has an unconditional `errno=ENOSYS; return -1;` (`#else` branch) | `-1`, `errno=ENOSYS`; **dead code** in this build — `_sodium_mprotect` (1.23) never calls `cb`, and `_sodium_malloc`/`sodium_free` do not call them either | not-compiled-in-this-build |
| 1.28 | `_sodium_malloc` | `malloc(size ? size : 1)` returns `NULL` (host OOM) | `NULL`, `errno=ENOMEM` (set by libc `malloc`). The `size >= SIZE_MAX - page_size*4` / `page_size <= sizeof canary` `sodium_misuse()` checks live in the `HAVE_ALIGNED_MALLOC` variant and are **not compiled** | host-OOM-not-forceable |
| 1.29 | `sodium_malloc` | `_sodium_malloc(size) == NULL` | `NULL` (errno as set by libc `malloc`); the `memset(ptr, 0xdb, size)` is skipped | host-OOM-not-forceable |
| 1.30 | `sodium_allocarray` | `count > 0 && size >= SIZE_MAX / count` (multiplication overflow guard) | `NULL`, `errno=ENOMEM` | verified |
| 1.31 | `sodium_allocarray` | guard passes but `sodium_malloc(count * size)` returns `NULL` | `NULL` (errno from libc `malloc`) | host-OOM-not-forceable |
| 1.32 | `sodium_free` | `!HAVE_ALIGNED_MALLOC` branch is compiled → plain `free(ptr)` | never fails, never aborts. **No canary check, no `_out_of_bounds()`, no `sodium_munlock`, no `_unprotected_ptr_from_user_ptr` `sodium_misuse()`** in this build; `sodium_free(NULL)` is a libc no-op; a foreign pointer is libc UB, not a sodium error | verified |
| 1.33 | `_out_of_bounds` / `_unprotected_ptr_from_user_ptr` / `_page_round` / `_alloc_aligned` / `_free_aligned` | inside `#ifdef HAVE_ALIGNED_MALLOC` | **not compiled** — the `abort()` on canary corruption and the `unprotected_ptr_u <= page_size*2` `sodium_misuse()` do not exist in this build | not-compiled-in-this-build |
| 1.34 | `sodium_pad` | `blocksize == 0` (`blocksize <= 0U` with `size_t`) | `-1`; `errno` untouched; `*padded_buflen_p` not written | verified |
| 1.35 | `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen` (i.e. `unpadded_buflen` within `blocksize-1` of `SIZE_MAX`) | aborts (sodium_misuse) | verified |
| 1.36 | `sodium_pad` | `xpadded_len >= max_buflen`, i.e. `unpadded_buflen + xpadlen + 1 > max_buflen` — output buffer too small | `-1`; `errno` untouched; `*padded_buflen_p` **not** written (the write happens after this check) | verified |
| 1.37 | `sodium_unpad` | `blocksize == 0` | `-1`; `errno` untouched; `*unpadded_buflen_p` not written | verified |
| 1.38 | `sodium_unpad` | `padded_buflen < blocksize` (including `padded_buflen == 0`) | `-1`; `errno` untouched; `*unpadded_buflen_p` not written | verified |
| 1.39 | `sodium_unpad` | no `0x80` barrier byte found in the last `blocksize` bytes (`valid == 0`) — e.g. all-zero tail, or all-`0xff` tail | `-1` via `(int)(valid - 1U)`; `errno` untouched; **`*unpadded_buflen_p` IS still written** (`padded_buflen - 1 - pad_len`, garbage value) before returning | verified |
| 1.40 | `sodium_bin2hex` | `bin_len >= SIZE_MAX / 2` | aborts (sodium_misuse) | verified |
| 1.41 | `sodium_bin2hex` | `hex_maxlen <= bin_len * 2U` — no room for the `2*bin_len` digits plus the NUL | aborts (sodium_misuse) | verified |
| 1.42 | `sodium_hex2bin` | `bin_pos >= bin_maxlen` while a further hex digit pair is pending — output buffer too small | `-1`, `errno=ERANGE`; loop breaks; `*bin_len = 0` (forced); `*hex_end` = position of the digit that did not fit | verified |
| 1.43 | `sodium_hex2bin` | odd number of hex digits consumed (`state != 0` at loop exit), e.g. `hex="abc"` | `-1`, `errno=EINVAL`; `hex_pos--` (backs up onto the dangling digit); `*bin_len = 0` | verified |
| 1.44 | `sodium_hex2bin` | `hex_end == NULL` **and** `hex_pos != hex_len` — a non-hex, non-ignored character (or embedded NUL) stopped the scan and the caller did not ask for an end pointer | `-1`, `errno=EINVAL` (overwrites any prior errno on this path) | verified |
| 1.45 | `sodium_base64_check_variant` (via `sodium_base64_encoded_len`) | `(((unsigned)variant) & ~0x6U) != 0x1U` — i.e. `variant` not in `{1,3,5,7}` (`0`, `2`, `4`, `6`, `8`, `9`, `-1`, …) | aborts (sodium_misuse) | verified |
| 1.46 | `sodium_base64_encoded_len` | `bin_len / 3 > (SIZE_MAX - 5) / 4` | aborts (sodium_misuse) | verified |
| 1.47 | `sodium_base64_check_variant` (via `sodium_bin2base64`) | `variant` not in `{1,3,5,7}` | aborts (sodium_misuse) | verified |
| 1.48 | `sodium_bin2base64` | `bin_len / 3 > (SIZE_MAX - 5) / 4` | aborts (sodium_misuse) | verified |
| 1.49 | `sodium_bin2base64` | `b64_maxlen <= b64_len` — output buffer has no room for the encoding plus a NUL | aborts (sodium_misuse) | verified |
| 1.50 | `sodium_bin2base64` | `assert(b64_pos <= b64_len)` | live assert (no `NDEBUG`); mathematically always true → never fires | compile-time-only (cannot fire at runtime) |
| 1.51 | `sodium_base64_check_variant` (via `sodium_base642bin`) | `variant` not in `{1,3,5,7}` | aborts (sodium_misuse) | verified |
| 1.52 | `sodium_base642bin` | `bin_pos >= bin_maxlen` when a decoded byte is ready — output buffer too small | `-1`, `errno=ERANGE`; loop breaks; `*bin_len = 0`; trailing-ignore skipping is **not** performed | verified |
| 1.53 | `sodium_base642bin` | `acc_len > 4U` at loop exit — a run of base64 chars whose length ≡ 1 mod 4 (e.g. `"A"`, `"AAAAA"`) | `-1`; **`errno` is NOT set** by libsodium on this path; `*bin_len = 0` | verified |
| 1.54 | `sodium_base642bin` | `(acc & ((1U << acc_len) - 1U)) != 0U` — non-canonical encoding: the unused low bits of the last base64 char are non-zero (e.g. `"AB"` where `acc_len==4`, `"ABC"` where `acc_len==2`) | `-1`; **`errno` is NOT set**; `*bin_len = 0` | verified |
| 1.55 | `_sodium_base642bin_skip_padding` (via padded variants `1`/`5`) | `*b64_pos_p >= b64_len` — input ended before the required `acc_len/2` `'='` characters were consumed (e.g. `"QQ"` under `VARIANT_ORIGINAL`) | `-1`, `errno=ERANGE`; propagated by `sodium_base642bin`, which then sets `*bin_len = 0` | verified |
| 1.56 | `_sodium_base642bin_skip_padding` (via padded variants) | a character in the padding region that is neither `'='` nor (when `ignore != NULL`) a member of `ignore` — e.g. `"QQ=x"` | `-1`, `errno=EINVAL`; propagated; `*bin_len = 0` | verified |
| 1.57 | `sodium_base642bin` | `b64_end == NULL` **and** `b64_pos != b64_len` — leftover input after decoding (invalid char with `ignore == NULL`, or excess padding) | `-1`, `errno=EINVAL` | verified |
| 1.58 | `sodium_ip2bin` | a zone-id character after `'%'` outside `[0-9a-zA-Z._-]` (e.g. `"fe80::1%et h0"`, `"fe80::1%*"`) | `-1` | verified |
| 1.59 | `sodium_ip2bin` | `'%'` is the last character (`zone + 1 >= end`) — empty zone id, e.g. `"fe80::1%"` | `-1` | verified |
| 1.60 | `sodium_ip2bin` | `zone != NULL && !is_ipv6` — a `'%'` present but no `':'` in the address part, e.g. `"1.2.3.4%eth0"` | `-1` | verified |
| 1.61 | `sodium_ip2bin` | `is_ipv6` (contains `':'`) and `parse_ipv6()` returns `0` | `-1` (`bin[16]` may have been partially written? no — `parse_ipv6` only `memcpy`s to `out` on success) | verified |
| 1.62 | `sodium_ip2bin` | `!is_ipv6` and `parse_ipv4()` returns `0` | `-1`; `bin` untouched | verified |
| 1.63 | `parse_ipv4` (via `sodium_ip2bin`, and via `parse_ipv6` for embedded IPv4) | `src >= end` — zero-length address (`ip_len_ == 0`, or `ip[0] == '\0'`) | `0` → caller `-1` | verified |
| 1.64 | `parse_ipv4` | an octet with more than 3 digits (`++digits > 3`), e.g. `"1.2.3.0004"` | `0` → `-1` | verified |
| 1.65 | `parse_ipv4` | an octet whose running value exceeds 255 (`val > 255U`), e.g. `"1.2.3.256"`, `"999.1.1.1"` | `0` → `-1` | verified |
| 1.66 | `parse_ipv4` | `digits == 0` at an octet position — missing octet, e.g. `"1..2.3"`, `"1.2.3."`, `".1.2.3"`, `"a.b.c.d"` | `0` → `-1` | verified |
| 1.67 | `parse_ipv4` | for `i < 3`: `p >= end` (input exhausted) or `*p != '.'` — missing/wrong separator, e.g. `"1.2.3"`, `"1.2,3.4"` | `0` → `-1` | verified |
| 1.68 | `parse_ipv4` | `p != end` after the 4th octet — trailing garbage, e.g. `"1.2.3.4.5"`, `"1.2.3.4x"` | `0` (via `return p == end`) → `-1` | verified |
| 1.69 | `parse_ipv6` (via `sodium_ip2bin`) | `src >= end` — empty address part (cannot normally be reached since `is_ipv6` requires a `':'`) | `0` → `-1` | verified |
| 1.70 | `parse_ipv6` | leading single `':'` not followed by another `':'` — e.g. `":1:2:3:4:5:6:7"`, `":"` | `0` → `-1` | verified |
| 1.71 | `parse_ipv6` | a second `"::"` (`colonp != NULL` when an empty group is seen again) — e.g. `"1::2::3"` | `0` → `-1` | verified |
| 1.72 | `parse_ipv6` | `tp + 2 > endp` when flushing a group at a `':'` — more than 8 groups, e.g. `"1:2:3:4:5:6:7:8:9"` | `0` → `-1` | verified |
| 1.73 | `parse_ipv6` | `p >= end` immediately after consuming a `':'` that followed a group — trailing single colon, e.g. `"1:2:3:4:5:6:7:"` | `0` → `-1` | verified |
| 1.74 | `parse_ipv6` | `'.'` seen but `tp + 4 > endp` — no room for the embedded IPv4, e.g. `"1:2:3:4:5:6:7:1.2.3.4"` | `0` → `-1` | verified |
| 1.75 | `parse_ipv6` | `'.'` seen and `parse_ipv4(curtok, end, tp) == 0` — malformed embedded IPv4, e.g. `"::ffff:1.2.3"`, `"::ffff:1.2.3.999"` | `0` → `-1` | verified |
| 1.76 | `parse_ipv6` | `ip_hex_digit(ch) < 0` — character that is not `[0-9a-fA-F:.]`, e.g. `"1:2:g::"`, `"fe80::1 "` (trailing space) | `0` → `-1` | verified |
| 1.77 | `parse_ipv6` | `xdigits >= 4` — a group with more than 4 hex digits, e.g. `"12345::1"` | `0` → `-1` | verified |
| 1.78 | `parse_ipv6` | `tp + 2 > endp` when flushing the final group (`saw_xdigit`) — 9th group at the tail | `0` → `-1` | verified |
| 1.79 | `parse_ipv6` | `colonp != NULL && tp == endp` — `"::"` present but the explicit groups already fill 16 bytes, e.g. `"1:2:3:4:5:6:7::"`, `"::1:2:3:4:5:6:7:8"` | `0` → `-1` | verified |
| 1.80 | `parse_ipv6` | `tp != endp` at the end with no `"::"` — fewer than 8 groups, e.g. `"1:2:3"` | `0` → `-1` | verified |
| 1.81 | `sodium_bin2ip` | `ip_maxlen <= 2U` (0, 1, or 2) | `NULL`; `ip` untouched; `errno` untouched | verified |
| 1.82 | `sodium_bin2ip` | IPv4-mapped input (`bin[0..11] == {0×10, 0xff, 0xff}`) and `len >= ip_maxlen`, where `len` is the dotted-quad length (7…15) — e.g. `ip_maxlen == 8` for `"255.255.255.255"` | `NULL`; `errno` untouched | verified |
| 1.83 | `sodium_bin2ip` | IPv6 formatting path and `len >= ip_maxlen` (`len` up to 39, or up to 45 with an embedded-IPv4-looking tail) — e.g. `ip_maxlen == 10` for all-`0xff` | `NULL`; `errno` untouched | verified |
| 1.84 | `randombytes_set_implementation` | any argument, including `NULL` (the `nonnull` attribute is advisory only) | always `0` — **never rejects**. Storing `NULL` makes the next call fall back to `randombytes_init_if_needed()`, which reinstalls `&randombytes_sysrandom_implementation`; storing a struct with `NULL` required members (`implementation_name`/`random`/`buf`) yields a NULL-function-pointer call (SIGSEGV), not an error return | verified |
| 1.85 | `randombytes_uniform` | `upper_bound < 2` (i.e. `0` or `1`), with `implementation->uniform == NULL` | returns `0` without consuming any randomness (no error signalled). If `implementation->uniform != NULL`, this guard is **bypassed** and the callback sees `0`/`1` verbatim | verified |
| 1.86 | `randombytes_buf_deterministic` | `size > 0x4000000000ULL` (256 GiB); the `#if SIZE_MAX > 0x4000000000ULL` guard **is** compiled on x86-64 | aborts (sodium_misuse) | verified |
| 1.87 | `randombytes_buf_deterministic` | `COMPILER_ASSERT(randombytes_SEEDBYTES == crypto_stream_chacha20_ietf_KEYBYTES)` and `COMPILER_ASSERT(randombytes_BYTES_MAX <= 0x4000000000ULL)` | compile-time only (32 == 32; `randombytes_BYTES_MAX == 0xffffffff`); cannot fail at runtime | compile-time-only (cannot fire at runtime) |
| 1.88 | `randombytes_close` | `implementation == NULL` (nothing ever initialised it) **or** `implementation->close == NULL` | returns `0` (reports success without doing anything, and **without** triggering lazy init) | verified |
| 1.89 | `randombytes_close` | `implementation->close` present and itself fails | returns the callback's value verbatim (`-1` for both bundled implementations under the conditions in 1.107 / 1.125) | verified |
| 1.90 | `randombytes` (NaCl alias) | `assert(buf_len <= SIZE_MAX)` | live assert but a tautology on x86-64 (`unsigned long long` and `size_t` are both 64-bit) → never fires | compile-time-only (cannot fire at runtime) |
| 1.91 | `randombytes_buf` | `size == 0` | `implementation->buf` is **not** called; silent no-op (so an all-zero `size` never reaches the getrandom asserts) | verified |
| 1.92 | `randombytes_stir` | `implementation->stir == NULL` | silent no-op, no error | verified |
| 1.93 | `safe_read` (sysrandom) | `assert(size > (size_t) 0U)` — reached only if `impl->buf` is invoked with `size == 0` through a path bypassing `randombytes_buf` | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.94 | `safe_read` (sysrandom) | `assert(size <= SSIZE_MAX)`, where `SSIZE_MAX` is the libc value on Linux | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.95 | `safe_read` (sysrandom) | `read()` fails with an errno other than `EINTR`/`EAGAIN` (e.g. `EIO`, `EBADF`) | returns the negative `readnb` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.96 | `safe_read` (sysrandom) | `read()` returns `0` (EOF on the device) | `break`s and returns a **short** byte count (`< size`) — the caller treats this as failure | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.97 | `randombytes_block_on_dev_random` (sysrandom) | `open("/dev/random", O_RDONLY) == -1` | returns `0` — **treated as success**, so a missing `/dev/random` does not block device opening | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.98 | `randombytes_block_on_dev_random` (sysrandom) | `poll()` returns a value other than `1` after `EINTR`/`EAGAIN` retries | closes the fd, sets `errno=EIO`, returns `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.99 | `randombytes_block_on_dev_random` (sysrandom) | `close(fd)` fails | returns `close()`'s `-1` (errno from `close`) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.100 | `randombytes_sysrandom_random_dev_open` | `randombytes_block_on_dev_random() != 0` (1.98/1.99) | `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.101 | `randombytes_sysrandom_random_dev_open` | neither `/dev/urandom` nor `/dev/random` yields a usable fd (`open` fails for a reason other than `EINTR`, or `fstat` fails, or `!S_ISCHR(st.st_mode)`) | `errno=EIO`, `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.102 | `_randombytes_linux_getrandom` (sysrandom) | `assert(size <= 256U)` — caller passed a chunk larger than 256 | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.103 | `_randombytes_linux_getrandom` (sysrandom) | `getrandom()` (via `syscall(SYS_getrandom, …)`) returns anything other than exactly `(int) size` after `EINTR`/`EAGAIN` retries — including `-ENOSYS` on kernels < 3.17 | `(readnb == (int) size) - 1` → `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.104 | `randombytes_linux_getrandom` (sysrandom) | `assert(chunk_size > (size_t) 0U)` when called with `size == 0` (the `do/while` runs at least once) | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.105 | `randombytes_linux_getrandom` (sysrandom) | any 256-byte chunk fails (1.103) | `-1` (partial output already written to `buf`) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.106 | `randombytes_sysrandom_init` | the 16-byte getrandom probe failed (`getrandom_available = 0`) **and** `randombytes_sysrandom_random_dev_open() == -1` (1.100/1.101) | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.107 | `randombytes_sysrandom_close` | `random_data_source_fd == -1` or `close()` fails, **and** `getrandom_available == 0` | `-1`. Note the normal Linux case (`getrandom_available != 0`) forces `ret = 0` while leaving `stream.initialized == 1`, i.e. `close()` succeeds without deinitialising anything | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.108 | `randombytes_sysrandom_buf` | `getrandom_available != 0` and `randombytes_linux_getrandom(buf, size)` fails (1.105) | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.109 | `randombytes_sysrandom_buf` | `getrandom_available == 0` and (`random_data_source_fd == -1` **or** `safe_read(...) != (ssize_t) size`) | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.110 | `randombytes_sysrandom_buf` | `assert(size <= ULLONG_MAX)` | inside `#if SIZE_MAX > ULLONG_MAX` → **not compiled** on x86-64 | not-compiled-in-this-build |
| 1.111 | `sodium_hrtime` (internal impl) | `gettimeofday(&tv, NULL) != 0` | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.112 | `_randombytes_linux_getrandom` (internal impl) | `assert(size <= 256U)` | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.113 | `_randombytes_linux_getrandom` (internal impl) | `getrandom()` returns `!= (int) size` after retries | `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.114 | `randombytes_linux_getrandom` (internal impl) | `assert(chunk_size > (size_t) 0U)` when `size == 0` | aborts (assert) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.115 | `randombytes_linux_getrandom` (internal impl) | any chunk fails | `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.116 | `randombytes_block_on_dev_random` (internal impl) | `open("/dev/random")` fails | `0` (treated as success) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.117 | `randombytes_block_on_dev_random` (internal impl) | `poll()` returns `!= 1` | `errno=EIO`, `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.118 | `randombytes_block_on_dev_random` (internal impl) | `close(fd)` fails | `close()`'s `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.119 | `randombytes_internal_random_random_dev_open` | `randombytes_block_on_dev_random() != 0` | `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.120 | `randombytes_internal_random_random_dev_open` | neither `/dev/urandom` nor `/dev/random` usable (`fstat` fails or `!(S_ISNAM(mode) \|\| S_ISCHR(mode))`; `S_ISNAM(X)` is `#define`d to `0` here) | `errno=EIO`, `-1` | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.121 | `safe_read` (internal impl) | `assert(size > 0)`; `assert(size <= SSIZE_MAX)`; `read()` error → negative return; `read()` EOF → short count | aborts (assert) / negative / short. **Dead in this build**: the only `safe_read` call site in `randombytes_internal_random_stir` sits in an `#elif !defined(NONEXISTENT_DEV_RANDOM)` arm that is shadowed by the `HAVE_LINUX_COMPATIBLE_GETRANDOM` arm | not-compiled-in-this-build |
| 1.122 | `randombytes_internal_random_init` | `assert((global.getentropy_available \| global.getrandom_available) == 0)` after a failed getrandom probe | live assert; holds by construction → never fires | compile-time-only (cannot fire at runtime) |
| 1.123 | `randombytes_internal_random_init` | getrandom probe failed **and** `randombytes_internal_random_random_dev_open() == -1` | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.124 | `randombytes_internal_random_stir` | `assert(stream.nonce != (uint64_t) 0U)` — would need `gettimeofday` to return exactly epoch 0 | live assert; effectively never fires | compile-time-only (cannot fire at runtime) |
| 1.125 | `randombytes_internal_random_stir` | `global.getrandom_available != 0` and `randombytes_linux_getrandom(stream.key, 32)` fails | aborts (sodium_misuse) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.126 | `randombytes_internal_random_stir` | `global.getrandom_available == 0` (probe failed, fd was opened instead) | **no error and no key material**: with `HAVE_LINUX_COMPATIBLE_GETRANDOM` selected, the `safe_read`-into-`stream.key` arm is an `#elif` and is not compiled, so `stream.key` is left all-zero and `stream.initialized = 1` is still set. Silent weak-key path | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.127 | `randombytes_internal_random_stir_if_needed` | `global.pid != getpid()` (fork detection) → `sodium_misuse()` | inside `#ifdef HAVE_GETPID` → **not compiled**; there is no fork detection in this build | not-compiled-in-this-build |
| 1.128 | `randombytes_internal_random_buf` | `assert(ret == 0)` on `crypto_stream_chacha20` | live assert; `crypto_stream_chacha20` always returns `0` → never fires | compile-time-only (cannot fire at runtime) |
| 1.129 | `randombytes_internal_random` | `assert(ret == 0)` on `crypto_stream_chacha20`; plus two `COMPILER_ASSERT`s on `sizeof stream.rnd32` | compile-time / never fires | compile-time-only (cannot fire at runtime) |
| 1.130 | `randombytes_internal_random_close` | `global.getrandom_available == 0` | `-1`. The `close(global.random_data_source_fd)` arm is an `#elif` and is **not compiled**, so any opened `/dev/urandom` fd is leaked; `sodium_memzero(&stream, sizeof stream)` still runs (forcing a re-stir on the next call) | OS-level-not-forceable (device/syscall failure cannot be induced) |
| 1.131 | `randombytes_internal_random_buf` | `assert(size <= ULLONG_MAX)` | inside `#if SIZE_MAX > ULLONG_MAX` → **not compiled** on x86-64 | not-compiled-in-this-build |
| 1.132 | `sodium_version_string` / `sodium_library_version_major` / `sodium_library_version_minor` / `sodium_library_minimal` / `randombytes_seedbytes` / `randombytes_implementation_name` / `randombytes_random` / `randombytes_increment`-family (`sodium_increment`/`sodium_add`/`sodium_sub`) | — | **no rejection branches at all**: total functions with no error return, no assert, no `sodium_misuse` (`sodium_increment`/`add`/`sub`'s `HAVE_AMD64_ASM` length special-cases are not compiled, so `len == 0` is a plain no-op) | verified |

## Area 2 — crypto_verify + crypto_core

Files in scope (libsodium 1.0.23):

- `c_src/libsodium/crypto_verify/verify.c`
- `c_src/libsodium/crypto_core/salsa/ref/core_salsa_ref.c`
- `c_src/libsodium/crypto_core/hsalsa20/core_hsalsa20.c`, `crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c`
- `c_src/libsodium/crypto_core/hchacha20/core_hchacha20.c`
- `c_src/libsodium/crypto_core/keccak1600/keccak1600.c`, `keccak1600/ref/keccak1600_ref.c`
- `c_src/libsodium/crypto_core/softaes/softaes.c`
- `c_src/libsodium/crypto_core/ed25519/core_ed25519.c`, `core_ristretto255.c`, `core_h2c.c`, `ref10/ed25519_ref10.c`
- headers: `include/sodium/crypto_verify_{16,32,64}.h`, `crypto_core_salsa{20,2012,208}.h`, `crypto_core_hsalsa20.h`, `crypto_core_hchacha20.h`, `crypto_core_keccak1600.h`, `crypto_core_ed25519.h`, `crypto_core_ristretto255.h`

Build assumption: the CMake build defines **no** `HAVE_*` macros, so `HAVE_EMMINTRIN_H`/`__SSE2__`, `HAVE_INLINE_ASM`, `HAVE_TI_MODE`, `__ARM_FEATURE_SHA3` are all absent. Consequences relevant to this table: `crypto_verify_n` takes the constant-time byte-loop fallback; `equal()`/`negative()` in `ed25519_ref10.c` take the arithmetic fallback; field arithmetic is `fe_25_5` (10x25.5-bit limbs); `keccak1600_*` binds to `keccak1600_ref_*`; `softaes` takes the `#else` (non-`FAVOR_PERFORMANCE`) branch with `SOFTAES_STRIDE == 16`. `MINIMAL` is also not defined, so `crypto_core_salsa2012` / `crypto_core_salsa208` exist.

**Total-function note (no rejection branch at all):** `crypto_verify_{16,32,64}_bytes`, all `crypto_core_salsa*_{output,input,key,const}bytes`, `crypto_core_hsalsa20_*bytes`, `crypto_core_hchacha20_*bytes`, `crypto_core_keccak1600_statebytes`, `crypto_core_ed25519_{bytes,uniformbytes,hashbytes,scalarbytes,nonreducedscalarbytes}`, `crypto_core_ristretto255_{bytes,hashbytes,scalarbytes,nonreducedscalarbytes}`, `crypto_core_salsa20/2012/208` (always `return 0`), `crypto_core_hsalsa20`, `crypto_core_hchacha20` (always `return 0`), `crypto_core_keccak1600_{init,xor_bytes,extract_bytes,permute_24,permute_12}` (`void`), all `softaes_*` (`void`/`SoftAesBlock`, no status), `crypto_core_ristretto255_from_hash` (always `return 0`), `crypto_core_ed25519_random`, `crypto_core_ristretto255_random`, `crypto_core_ed25519_scalar_{negate,complement,add,sub,mul,reduce}` and their `ristretto255_*` wrappers (`void`). These are listed here once and do not occupy rows below.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 2.1 | `crypto_verify_16` (`verify.c:89`) | `x` and `y` differ in at least one of bytes 0..15 (fallback path: `d = OR of x[i]^y[i]` is nonzero) | `-1` | verified |
| 2.2 | `crypto_verify_32` (`verify.c:95`) | `x` and `y` differ in at least one of bytes 0..31 | `-1` | verified |
| 2.3 | `crypto_verify_64` (`verify.c:101`) | `x` and `y` differ in at least one of bytes 0..63 | `-1` | verified |
| 2.4 | `ge25519_is_canonical` (`ed25519_ref10.c:1156`) | encoding is non-canonical: `s[0] >= 0xed` AND `s[1..30] == 0xff` AND `(s[31] & 0x7f) == 0x7f` (i.e. `y >= 2^255-19`) | `0` (rejects) | verified |
| 2.5 | `ge25519_frombytes` (`ed25519_ref10.c:326`) | `y` (from `s`) admits no `x`: neither `vx^2-u == 0` nor `vx^2+u == 0`, i.e. `has_m_root == 0 && has_p_root == 0`; return value is `(has_m_root \| has_p_root) - 1` | `-1` | verified |
| 2.6 | `ge25519_frombytes_negate_vartime` (`ed25519_ref10.c:364`) | `fe25519_iszero(vx^2-u) == 0` and `fe25519_iszero(vx^2+u) == 0` (no square root for the given `y`) | `-1` | verified |
| 2.7 | `ge25519_is_on_curve` (`ed25519_ref10.c:1118`) | `(Y^2-X^2)Z^2 - (d*X^2*Y^2 + Z^4) != 0` (coords do not satisfy the twisted Edwards equation) | `0` (rejects) | verified |
| 2.8 | `ge25519_has_small_order` (`ed25519_ref10.c:1173`) | any of `X == 0`, `Y == 0`, `Z == 0`, `Y*sqrt(-1) - X == 0`, `Y*sqrt(-1) + X == 0` — the 8 points of order dividing 8, including the identity `(0,1)` | non-zero (`1`); caller treats as "reject" | verified |
| 2.9 | `ge25519_is_on_main_subgroup` (`ed25519_ref10.c:1143`) | `L*P != identity`, i.e. `fe25519_iszero(pl.X) & fe25519_iszero(pl.Y - pl.Z) == 0` | `0` (rejects) | verified |
| 2.10 | `fe25519_sqrt` (`ed25519_ref10.c:207`, static) | `x2` is not a quadratic residue mod `2^255-19`: `x^2 - x2 != 0`; return is `fe25519_iszero(check) - 1` | `-1` | unreachable-from-public-API |
| 2.11 | `sc25519_is_canonical` (`ed25519_ref10.c:2574`) | 32-byte scalar `s >= L` where `L = 2^252+27742317777372353535851937790883648493` (borrow chain leaves `c == 0`) | `0` (rejects) | verified |
| 2.12 | `ristretto255_is_canonical` (`ed25519_ref10.c:2802`, static) | any of: `s >= 2^255-19` (`c & d` set), bit 255 of `s[31]` set (`e` set), or `s[0]` odd (`s[0] & 1`) — expression `1 - (((c & d) \| e \| s[0]) & 1)` | `0` (rejects) | verified |
| 2.13 | `ristretto255_frombytes` (`ed25519_ref10.c:2821`) | `ristretto255_is_canonical(s) == 0` (see 2.12) — early return before any field work | `-1` | verified |
| 2.14 | `ristretto255_frombytes` | `ristretto255_sqrt_ratio_m1(inv_sqrt, 1, v*u2^2)` returns 0, i.e. `1/(v*u2^2)` is not a square; contributes `(1 - notsquare)` to `return -(...)` | `-1` | verified |
| 2.15 | `ristretto255_frombytes` | decoded `T = X*Y` is "negative" (`fe25519_isnegative(h->T) != 0`) | `-1` | verified |
| 2.16 | `ristretto255_frombytes` | decoded `Y == 0` (`fe25519_iszero(h->Y) != 0`) | `-1` | verified |
| 2.17 | `ristretto255_sqrt_ratio_m1` (`ed25519_ref10.c:2766`, static) | neither `vx^2-u == 0` nor `vx^2+u == 0` (`has_m_root \| has_p_root == 0`); `x` is still set to `abs(x*sqrt(-1))` | `0` ("was not a square"); caller (2.14) turns this into `-1` | verified |
| 2.18 | `crypto_core_ed25519_is_valid_point` (`core_ed25519.c:14`) | `ge25519_is_canonical(p) == 0` — non-canonical 32-byte encoding (see 2.4) | `0` | verified |
| 2.19 | `crypto_core_ed25519_is_valid_point` | `ge25519_frombytes(&p_p3, p) != 0` — `y` has no matching `x` (see 2.5) | `0` | verified |
| 2.20 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_curve(&p_p3) == 0` (see 2.7) | `0` | verified |
| 2.21 | `crypto_core_ed25519_is_valid_point` | `ge25519_has_small_order(&p_p3) != 0` — small-order point or the identity `01 00 ... 00` (see 2.8) | `0` | verified |
| 2.22 | `crypto_core_ed25519_is_valid_point` | `ge25519_is_on_main_subgroup(&p_p3) == 0` — on-curve point of order `8L`/`2L`/`4L` not in the prime-order subgroup (see 2.9) | `0` | verified |
| 2.23 | `crypto_core_ed25519_add` (`core_ed25519.c:29`) | `ge25519_frombytes(&p_p3, p) != 0` — first operand `p` decodes to no curve point | `-1` | verified |
| 2.24 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(&p_p3) == 0` for first operand | `-1` | verified |
| 2.25 | `crypto_core_ed25519_add` | `ge25519_frombytes(&q_p3, q) != 0` — second operand `q` decodes to no curve point | `-1` | verified |
| 2.26 | `crypto_core_ed25519_add` | `ge25519_is_on_curve(&q_p3) == 0` for second operand | `-1` | verified |
| 2.27 | `crypto_core_ed25519_sub` (`core_ed25519.c:45`) | `ge25519_frombytes(&p_p3, p) != 0` | `-1` | verified |
| 2.28 | `crypto_core_ed25519_sub` | `ge25519_is_on_curve(&p_p3) == 0` | `-1` | verified |
| 2.29 | `crypto_core_ed25519_sub` | `ge25519_frombytes(&q_p3, q) != 0` | `-1` | verified |
| 2.30 | `crypto_core_ed25519_sub` | `ge25519_is_on_curve(&q_p3) == 0` | `-1` | verified |
| 2.31 | `_string_to_points` (`core_ed25519.c:63`, static) | `n > 2U` | aborts (`abort()`, `core_ed25519.c:73`); unreachable from the public API — callers pass only `n = 1` or `n = 2` | unreachable-from-public-API |
| 2.32 | `_string_to_points` | `core_h2c_string_to_hash(...) != 0`, i.e. `hash_alg` is neither `1` (`CORE_H2C_SHA256`) nor `2` (`CORE_H2C_SHA512`) | `-1` | verified |
| 2.33 | `crypto_core_ed25519_from_string_nu` (`core_ed25519.c:92`) | `hash_alg` not in `{crypto_core_ed25519_H2CSHA256 (1), crypto_core_ed25519_H2CSHA512 (2)}` (propagated from 2.32) | `-1`, `errno == EINVAL` | verified |
| 2.34 | `crypto_core_ed25519_from_string` (`core_ed25519.c:101`) | `hash_alg` not in `{1, 2}` — `_string_to_points(px, 2, ...) != 0` | `-1`, `errno == EINVAL` | verified |
| 2.35 | `crypto_core_ed25519_from_string` | tail call `crypto_core_ed25519_add(p, &px[0], &px[32])` fails (would require `ge25519_from_hash` to emit a non-decodable encoding) | `-1`; not reachable in practice — `ge25519_from_hash` always emits a valid on-curve point | unreachable-from-public-API |
| 2.36 | `crypto_core_ed25519_scalar_invert` (`core_ed25519.c:135`) | `s` is the all-zero 32-byte scalar: `- sodium_is_zero(s, 32)` | `-1`; note `sc25519_invert` still ran and `recip` was written (all-zero output, since `0^(L-2) mod L == 0`) | verified |
| 2.37 | `crypto_core_ed25519_scalar_from_string` (`core_ed25519.c:240`) | `hash_alg` not in `{1, 2}` — `core_h2c_string_to_hash(h_be, 48, ...) != 0` | `-1`, `errno == EINVAL` | verified |
| 2.38 | `crypto_core_ed25519_scalar_is_canonical` (`core_ed25519.c:232`) | `s >= L` (delegates to `sc25519_is_canonical`, see 2.11) | `0` | verified |
| 2.39 | `crypto_core_ed25519_scalar_random` (`core_ed25519.c:125`) | drawn `r` (after `r[31] &= 0x1f`) is non-canonical (`sc25519_is_canonical(r) == 0`) or all-zero (`sodium_is_zero(r, 32)`) | no error return (`void`); the `do { ... } while` re-draws from `randombytes_buf` until accepted | verified |
| 2.40 | `ge25519_elligator2` (`ed25519_ref10.c:2653`, static) | `ge25519_xmont_to_ymont(y, x) != 0`, i.e. the recovered `x^3+Ax^2+x` is a non-square after the `notsquare` correction | aborts (`abort()`, `ed25519_ref10.c:2684`); mathematically unreachable (`LCOV_EXCL_LINE`) | unreachable-from-public-API |
| 2.41 | `core_h2c_string_to_hash` (`core_h2c.c:120`) | `hash_alg` matches neither `CORE_H2C_SHA256 (1)` nor `CORE_H2C_SHA512 (2)` — `default:` arm | sets `errno = EINVAL`, returns `-1` | verified |
| 2.42 | `core_h2c_string_to_hash_sha256` (`core_h2c.c:14`, static) | `h_len > 0xff` | aborts (`assert(h_len <= 0xff)`, `core_h2c.c:26`; no-op if `NDEBUG`); unreachable from the public API — callers pass `h_len` in `{48, 64, 96}` | verified |
| 2.43 | `core_h2c_string_to_hash_sha512` (`core_h2c.c:70`, static) | `h_len > 0xff` | aborts (`assert(h_len <= 0xff)`, `core_h2c.c:82`; no-op if `NDEBUG`); unreachable from the public API | verified |
| 2.44 | `crypto_core_ristretto255_is_valid_point` (`core_ristretto255.c:16`) | `ristretto255_frombytes(&p_p3, p) != 0` for any of the four reasons 2.13–2.16 (non-canonical / `s[31]` high bit set / `s[0]` odd / `s >= p` / non-square / `T` negative / `Y == 0`) | `0` | verified |
| 2.45 | `crypto_core_ristretto255_add` (`core_ristretto255.c:27`) | `ristretto255_frombytes(&p_p3, p) != 0` — first operand not a valid ristretto255 encoding | `-1` | verified |
| 2.46 | `crypto_core_ristretto255_add` | `ristretto255_frombytes(&q_p3, q) != 0` — second operand not a valid ristretto255 encoding | `-1` | verified |
| 2.47 | `crypto_core_ristretto255_sub` (`core_ristretto255.c:43`) | `ristretto255_frombytes(&p_p3, p) != 0` | `-1` | verified |
| 2.48 | `crypto_core_ristretto255_sub` | `ristretto255_frombytes(&q_p3, q) != 0` | `-1` | verified |
| 2.49 | `_string_to_element` (`core_ristretto255.c:67`, static) | `core_h2c_string_to_hash(h, 64, ...) != 0`, i.e. `hash_alg` not in `{1, 2}` | `-1` (`LCOV_EXCL_LINE`) | verified |
| 2.50 | `crypto_core_ristretto255_from_string` (`core_ristretto255.c:84`) | `hash_alg` not in `{crypto_core_ristretto255_H2CSHA256 (1), crypto_core_ristretto255_H2CSHA512 (2)}` | `-1`, `errno == EINVAL` | verified |
| 2.51 | `crypto_core_ristretto255_scalar_invert` (`core_ristretto255.c:108`) | `s` is the all-zero 32-byte scalar (delegates to `crypto_core_ed25519_scalar_invert`, see 2.36) | `-1` | verified |
| 2.52 | `crypto_core_ristretto255_scalar_is_canonical` (`core_ristretto255.c:157`) | `s >= L` (calls `sc25519_is_canonical` directly, see 2.11) | `0` | verified |

### Phase-C status notes

- **`verified`** (48 rows) — a differential test in `translation/tests/a2_*.rs` actually drove
  the branch on both `.so` files and compared the outcome. Rows whose trigger lives in a
  `static` helper were driven either through the exported internal symbol
  (`_sodium_ge25519_*`, `_sodium_sc25519_*`, `_sodium_ristretto255_*`,
  `_sodium_core_h2c_string_to_hash`) or through the public wrapper that propagates the
  status (2.32 via `crypto_core_ed25519_from_string{,_nu}`, 2.49 via
  `crypto_core_ristretto255_from_string`).
- **`unreachable-from-public-API`** (4 rows):
  - **2.10** `fe25519_sqrt` — `static`, and its *only* caller is `ge25519_xmont_to_ymont`,
    whose non-zero return makes `ge25519_elligator2` `abort()` (row 2.40). So the `-1`
    return can only be produced along an unreachable path.
  - **2.31** `_string_to_points(n > 2)` — `static`; the only two call sites pass the literal
    `1` and `2`.
  - **2.35** `crypto_core_ed25519_from_string`'s tail `crypto_core_ed25519_add` failure —
    `ge25519_from_hash` always emits a canonical, on-curve, cofactor-cleared encoding, so
    the `add` can never fail. `tests/a2_gaps.rs` re-derives `ge25519_from_hash` byte for byte
    and confirms the output always decodes.
  - **2.40** `ge25519_elligator2`'s `abort()` — the curve equation guarantees
    `x^3+Ax^2+x` is a square after the `notsquare` correction. `tests/a2_gaps.rs` asserts
    exactly this (`assert!(ok, ...)` inside its `elligator2` replica) over every input it feeds.
- Rows **2.42 / 2.43** are marked `verified` rather than `unreachable-from-public-API`
  because `core_h2c_string_to_hash` *is* exported as `_sodium_core_h2c_string_to_hash`, and
  `tests/a2_gaps.rs::core_h2c_h_len_assert_is_live` calls it with `h_len > 0xff` and checks
  that both libraries die on a fatal signal (and that `h_len <= 0xff` does *not* abort), which
  also proves `NDEBUG` is absent from this build. They remain unreachable from `sodium.h`.

### Rejection-surface remarks worth carrying into the Rust port

- **`crypto_core_ed25519_add`/`_sub` are deliberately weaker than `_is_valid_point`.** They only require `ge25519_frombytes` + `ge25519_is_on_curve`; they do **not** call `ge25519_is_canonical`, `ge25519_has_small_order`, or `ge25519_is_on_main_subgroup`. So the identity, all 8 small-order points, cofactor points, and non-canonical encodings that still decode are all *accepted* and return `0`.
- **`crypto_core_ed25519_scalar_invert` writes `recip` before deciding the return value** (rows 2.36/2.51). The out-buffer is always fully written, even on the `-1` path.
- **`ge25519_frombytes` is constant-time and sign-blinded** (`optblocker_u8`), while `ge25519_frombytes_negate_vartime` short-circuits — the two have the same accept/reject set but different control flow.
- **`ristretto255_frombytes` folds four independent rejections into one `-1`** via `- ((1 - notsquare) | isnegative(T) | iszero(Y))` plus the early canonical check; there is no way for a caller to distinguish them.
- **`errno`**: only `core_h2c_string_to_hash`'s `default:` arm sets `errno` (`EINVAL`). Every other `-1` in this area leaves `errno` untouched.
- **`assert`**: only in `core_h2c.c` (rows 2.42/2.43), compiled out under `NDEBUG`. `abort()` appears at `core_ed25519.c:73` and `ed25519_ref10.c:2684`, both unreachable from the public API. **No `sodium_misuse()` call exists anywhere in this area.**
- **No `return NULL`** anywhere in this area — every function returns `int`, `size_t`, `void`, or `SoftAesBlock`.

## Area 3 — hashes / xof / generichash / shorthash

Scope: `crypto_hash/{crypto_hash.c, sha256/**, sha512/**, sha3/hash_sha3.c}`,
`crypto_xof/**` (shake128, shake256, turboshake128, turboshake256 + `ref/`),
`crypto_generichash/{crypto_generichash.c, blake2b/generichash_blake2.c, blake2b/ref/**}`,
`crypto_shorthash/**`, and the matching public headers.

Build assumption: the CMake build defines **no** `HAVE_*` macros, so:
* `hash_sha256_cp.c` takes the `#else` (portable `SHA256_Transform`) branch — `HAVE_SHA256_ARMCRYPTO` is off.
* `blake2b-ref.c` uses `blake2b_compress = blake2b_compress_ref` and the `#else` (64-bit pair) branch of
  `blake2b_increment_counter` (`HAVE_TI_MODE` off).
* `blake2b_pick_best_implementation()` unconditionally selects `blake2b_compress_ref`.

Constants referenced below: `crypto_hash_sha256_BYTES`=32, `crypto_hash_sha512_BYTES`=64,
`crypto_hash_sha3256_BYTES`=32, `crypto_hash_sha3512_BYTES`=64, `SHA3_256_RATE`=136, `SHA3_512_RATE`=72,
`SHAKE128_RATE`=`TURBOSHAKE128_RATE`=168, `SHAKE256_RATE`=`TURBOSHAKE256_RATE`=136,
`crypto_xof_*_DOMAIN_STANDARD`=0x1F, `SHA3_DOMAIN`=0x06,
`BLAKE2B_OUTBYTES`=`BLAKE2B_KEYBYTES`=64, `BLAKE2B_BLOCKBYTES`=128, `BLAKE2B_SALTBYTES`=`BLAKE2B_PERSONALBYTES`=16,
`crypto_generichash_blake2b_BYTES_MIN`=16, `_BYTES`=32, `_BYTES_MAX`=64,
`_KEYBYTES_MIN`=16, `_KEYBYTES`=32, `_KEYBYTES_MAX`=64,
`crypto_shorthash_siphash24_BYTES`=8/`KEYBYTES`=16, `crypto_shorthash_siphashx24_BYTES`=16/`KEYBYTES`=16.

`sodium_misuse()` (sodium/core.c:192) calls the registered misuse handler if any and then `abort()` — i.e. it
never returns. Rows marked "abort" are process-terminating, not `-1`-returning.

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 3.1 | `crypto_hash` (crypto_hash.c:11) | none — no validation at all; tail-calls `crypto_hash_sha512` | always `0`; infallible | verified |
| 3.2 | `crypto_hash_bytes` / `crypto_hash_primitive` | none | `64` / `"sha512"`; infallible | verified |
| 3.3 | `crypto_hash_sha256_init` (hash_sha256_cp.c:336) | none — no validation; sets `count=0`, copies IV | always `0`; infallible | verified |
| 3.4 | `crypto_hash_sha256_update` (hash_sha256_cp.c:350) | `inlen == 0` → early `return 0` **before** any state mutation (`state->count` NOT advanced, buffer untouched) | `0`; must be a true no-op | verified |
| 3.5 | `crypto_hash_sha256_update` | any `inlen > 0` (incl. values that overflow `count` past 2^64 bits) — no range check; `state->count += inlen<<3` wraps silently | always `0`; no error, no saturation | verified |
| 3.6 | `crypto_hash_sha256_final` (hash_sha256_cp.c:392) | none — no validation; writes exactly 32 bytes, then `sodium_memzero(state)` | always `0`; infallible | verified |
| 3.7 | `crypto_hash_sha256_final` called a 2nd time on the same state | state was zeroed by the 1st `final` → `count=0`, `state[8]=0`, so the 2nd call pads/compresses an all-zero IV | `0` (**no error**), but digest is the non-standard "compress(zero IV, padded empty)" value; state reuse is silently accepted | verified |
| 3.8 | `crypto_hash_sha256_update` after `final` | same as 3.7 — no phase tracking exists; update resumes from the zeroed state | `0`; silently wrong digest, no error | verified |
| 3.9 | `crypto_hash_sha256` (one-shot, hash_sha256_cp.c:405) | ignores the return of init/update/final; unconditional `return 0` | always `0`; infallible | verified |
| 3.10 | `crypto_hash_sha512_init` (hash_sha512_cp.c:196) | none — `count[0]=count[1]=0`, copies IV | always `0`; infallible | verified |
| 3.11 | `crypto_hash_sha512_update` (hash_sha512_cp.c:211) | `inlen == 0` → early `return 0` before touching `count`/`buf` | `0`; true no-op | verified |
| 3.12 | `crypto_hash_sha512_update` | `inlen >= 2^61` → `bitlen[0] = inlen >> 61` is nonzero and folded into `count[0]`; 128-bit counter carry at `(state->count[1] += bitlen[1]) < bitlen[1]` (LCOV_EXCL region) — no check, no error | always `0`; carry must be reproduced exactly | verified (128-bit carry driven through the header-declared state); `inlen >= 2^61` unreachable-from-public-API |
| 3.13 | `crypto_hash_sha512_final` (hash_sha512_cp.c:261) | none; writes exactly 64 bytes then `sodium_memzero(state)` | always `0`; infallible | verified |
| 3.14 | `crypto_hash_sha512_final` called twice / `update` after `final` | state zeroed by 1st `final`; no phase flag exists | `0` (**no error**), non-standard digest — same silent-reuse semantics as 3.7/3.8 | verified |
| 3.15 | `crypto_hash_sha512` (one-shot) | discards sub-call returns; unconditional `return 0` | always `0`; infallible | verified |
| 3.16 | `crypto_hash_sha256_statebytes` / `crypto_hash_sha512_statebytes` | none | `sizeof(state)`; infallible | verified |
| 3.17 | `crypto_hash_sha3256_init` / `crypto_hash_sha3512_init` (hash_sha3.c:132/178) | `COMPILER_ASSERT(sizeof(crypto_hash_sha3{256,512}_state) >= sizeof(sha3_state_internal))` — **compile-time** static assert (opaque state is 256 B, `CRYPTO_ALIGN(16)`); no runtime check | compile-time only; runtime always `0` — **status: verified** (runtime `0` and the 256-byte state size are pinned on both sides; the `COMPILER_ASSERT` itself is compile-time → unreachable-from-public-API) |
| 3.18 | `sha3_update` (hash_sha3.c:36) via `crypto_hash_sha3{256,512}_update` | `state->phase != SHA3_PHASE_ABSORBING`, i.e. **absorb after finalize** (`update` called after `final`) → extra `permute_24`, `phase := ABSORBING`, `offset := 0`, `ret = -1` | returns **`-1`**, yet the state IS mutated and the new data IS absorbed (the function continues and returns `-1` only as a flag) — **status: verified** |
| 3.19 | `sha3_update` | any `inlen` (0 included) with `phase == ABSORBING` — no length range check; `inlen` is truncated `unsigned long long` → `size_t` at hash_sha3.c:143/189 | `0` — **status: verified** |
| 3.20 | `sha3_final` (hash_sha3.c:85) via `crypto_hash_sha3{256,512}_final` | `state->phase != SHA3_PHASE_ABSORBING`, i.e. **`final` called a second time (squeeze-after-squeeze)** → skips padding, does one bare `permute_24`, `ret = -1` | returns **`-1`**, but still writes `state->outlen` (32 resp. 64) bytes to `out` and re-sets `phase = FINALIZED`, `offset = 0` — **status: verified** |
| 3.21 | `sha3_final` | `state->offset == state->rate` (buffer exactly full) → extra `permute_24`, `offset = 0` before padding — boundary path, not an error | `0` — **status: verified** |
| 3.22 | `sha3_final` | `state->offset == state->rate - 1` → single fused pad byte `SHA3_DOMAIN ^ 0x80` = `0x86`; else two separate XORs (`0x06` at `offset`, `0x80` at `rate-1`) — boundary path, not an error | `0` — **status: verified** |
| 3.23 | `crypto_hash_sha3256` / `crypto_hash_sha3512` (one-shot) | discards init/update/final returns; unconditional `return 0`; `sodium_memzero(&state)` at the end | always `0`; infallible even if the inner calls would have returned `-1` — **status: verified** |
| 3.24 | `crypto_hash_sha3{256,512}_bytes` / `_statebytes` | none | `32`/`64`, `sizeof(state)`=256; infallible — **status: verified** |
| 3.25 | `shake128_ref_init_with_domain` (shake128_ref.c:10) and the shake256 / turboshake128 / turboshake256 twins | **NO range check on `domain`** — every byte value `0x00`..`0xFF` is stored verbatim into `state->domain` | always `0`. In particular `domain = 0x00` and `domain = 0x80` are accepted even though the TurboSHAKE spec restricts the separation byte to `0x01..0x7F`; the port must NOT add validation — **status: verified** (all 256 domain bytes swept through both `_sodium_*_ref_init_with_domain` and the public wrapper; every value stored verbatim, no validation added) |
| 3.26 | `crypto_xof_shake128_init_with_domain` / `_shake256_` / `_turboshake128_` / `_turboshake256_` | as 3.25 — the public wrapper adds only `COMPILER_ASSERT(sizeof(public_state) >= sizeof(internal_state))` (compile-time; public state is 256 B `CRYPTO_ALIGN(16)`) | always `0` — **status: verified** (the `COMPILER_ASSERT` is compile-time → unreachable-from-public-API; the runtime `0` for every domain byte is pinned) |
| 3.27 | `shake128_ref_update` (shake128_ref.c:27) + shake256 / turboshake128 / turboshake256 twins | `state->phase != *_PHASE_ABSORBING`, i.e. **absorb after squeeze** → extra `permute_24` (`permute_12` for turboshake), `phase := ABSORBING`, `offset := 0`, `ret = -1` | returns **`-1`**; the new input is still absorbed into the re-keyed state — **status: verified** |
| 3.28 | `crypto_xof_*_update` public wrappers | pass-through of 3.27; `inlen` truncated `unsigned long long` → `size_t`; no length check | `0` normally, `-1` on absorb-after-squeeze — **status: verified** |
| 3.29 | `shake128_ref_squeeze` (shake128_ref.c:106) + all 3 twins | **no error path whatsoever**: `outlen == 0` is a no-op, `outlen` unbounded, squeeze-after-squeeze is legal and simply continues the stream | always `0` — **status: verified** |
| 3.30 | `crypto_xof_*_squeeze` public wrappers | as 3.29 | always `0` — **status: verified** |
| 3.31 | `shake128_ref` / `shake256_ref` / `turboshake128_ref` / `turboshake256_ref` (one-shot) | discards init/update/squeeze returns; unconditional `return 0`; note the internal state is **not** zeroed on exit (unlike sha3 one-shot) | always `0` — **status: verified** |
| 3.32 | `crypto_xof_shake128` / `_shake256` / `_turboshake128` / `_turboshake256` (public one-shot) | `COMPILER_ASSERT` only; `inlen` truncated to `size_t`; no check on `outlen` | always `0` — **status: verified** |
| 3.33 | `crypto_xof_*_blockbytes` / `_statebytes` / `_domain_standard` | none | `168`/`136`, `256`, `0x1F`; infallible — **status: verified** |
| 3.34 | `crypto_generichash_blake2b` (generichash_blake2b.c:16) | `outlen <= 0U` (i.e. `outlen == 0`) | **`-1`** | verified |
| 3.35 | `crypto_generichash_blake2b` | `outlen > BLAKE2B_OUTBYTES` (65, 256, `SIZE_MAX`, …) | **`-1`** | verified |
| 3.36 | `crypto_generichash_blake2b` | `keylen > BLAKE2B_KEYBYTES` (65, 256, …) | **`-1`** | verified |
| 3.37 | `crypto_generichash_blake2b` | `inlen > UINT64_MAX` — **dead branch** where `unsigned long long` is 64-bit; can never fire | unreachable; keep the check as a no-op | unreachable-from-public-API (dead branch: `unsigned long long` is already 64-bit) |
| 3.38 | `crypto_generichash_blake2b` | `0 < outlen < crypto_generichash_blake2b_BYTES_MIN` (1..15) — **`BYTES_MIN` is NOT enforced** | **`0`** (accepted); the port must not reject 1..15 | verified |
| 3.39 | `crypto_generichash_blake2b` | `0 < keylen < crypto_generichash_blake2b_KEYBYTES_MIN` (1..15) — **`KEYBYTES_MIN` is NOT enforced** | **`0`** (accepted) | verified |
| 3.40 | `crypto_generichash_blake2b` | `assert(outlen <= UINT8_MAX)` / `assert(keylen <= UINT8_MAX)` (lines 20–21) — unreachable, already excluded by 3.35/3.36; compiled out under `NDEBUG` | never fires | unreachable-from-public-API (assert pre-empted by the `-1` range check) |
| 3.41 | `crypto_generichash_blake2b` | `key == NULL` **and** `keylen == 0` (unkeyed) → `blake2b(...)` with `keylen==0` → `blake2b_init` | `0` | verified |
| 3.42 | `crypto_generichash_blake2b` | `key == NULL` **and** `keylen > 0` → passes the wrapper's checks, then `blake2b()` hits `NULL == key && keylen > 0` | **`sodium_misuse()` → abort** (not `-1`); `nonnull` is not on `key` | verified |
| 3.43 | `crypto_generichash_blake2b` | `key != NULL` **and** `keylen == 0` → `blake2b()` takes the `else` branch → `blake2b_init` (key ignored, unkeyed digest) | `0`; key pointer silently ignored | verified |
| 3.44 | `crypto_generichash_blake2b` | `in == NULL` **and** `inlen > 0` → `blake2b()` line 341 | **`sodium_misuse()` → abort** | verified |
| 3.45 | `crypto_generichash_blake2b` | `in == NULL` **and** `inlen == 0` → allowed (`blake2b_update(S, NULL, 0)` loop body never runs) | `0` | verified |
| 3.46 | `crypto_generichash_blake2b` | `out == NULL` → `blake2b()` line 344 (header marks `out` `nonnull(1)`, so this is UB-ish but the runtime check exists) | **`sodium_misuse()` → abort** | verified |
| 3.47 | `crypto_generichash_blake2b_salt_personal` (generichash_blake2b.c:33) | `outlen == 0` \| `outlen > 64` \| `keylen > 64` \| `inlen > UINT64_MAX` — identical 4-way check to 3.34–3.37 | **`-1`** | verified |
| 3.48 | `crypto_generichash_blake2b_salt_personal` | `salt == NULL` and/or `personal == NULL` → `blake2b_salt_personal` → `blake2b_init{,_key}_salt_personal` `memset`s that field to zero | `0`; NULL means "all-zero salt/personal", **not** an error | verified |
| 3.49 | `crypto_generichash_blake2b_salt_personal` | `key == NULL && keylen > 0`, or `in == NULL && inlen > 0`, or `out == NULL` → `blake2b_salt_personal` lines 379/382/388 | **`sodium_misuse()` → abort** | verified |
| 3.50 | `crypto_generichash_blake2b_init` (generichash_blake2b.c:46) | `outlen <= 0U` | **`-1`** | verified |
| 3.51 | `crypto_generichash_blake2b_init` | `outlen > BLAKE2B_OUTBYTES` (65 …) | **`-1`** | verified |
| 3.52 | `crypto_generichash_blake2b_init` | `keylen > BLAKE2B_KEYBYTES` (65 …) | **`-1`** | verified |
| 3.53 | `crypto_generichash_blake2b_init` | note there is **no** `inlen`-style check and no `BYTES_MIN`/`KEYBYTES_MIN` check: `outlen` 1..15 and `keylen` 1..15 are accepted | `0` | verified |
| 3.54 | `crypto_generichash_blake2b_init` | `COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state)` — compile-time (`crypto_generichash_blake2b_state` = 384 B `CRYPTO_ALIGN(64)`) | compile-time only | unreachable-from-public-API (compile-time assert; the 384-byte state size is checked at runtime via `statebytes()`) |
| 3.55 | `crypto_generichash_blake2b_init` | `key == NULL` (any `keylen`, incl. `keylen > 0` after the ≤64 check) **or** `keylen <= 0U` → `blake2b_init(state, outlen)` — key is ignored | `0`. Note the asymmetry vs 3.42: here `key==NULL && 0<keylen<=64` is silently treated as **unkeyed**, no abort | verified |
| 3.56 | `crypto_generichash_blake2b_init` | `key != NULL && keylen > 0` → `blake2b_init_key(...)`; the `!= 0 → return -1` arm is `LCOV_EXCL_LINE` because `blake2b_init_key` either returns `0` or aborts | `0` (never `-1` from this arm) | verified |
| 3.57 | `crypto_generichash_blake2b_init_salt_personal` (generichash_blake2b.c:69) | `outlen == 0` \| `outlen > 64` \| `keylen > 64` | **`-1`** | verified |
| 3.58 | `crypto_generichash_blake2b_init_salt_personal` | `key == NULL` **or** `keylen == 0` → `blake2b_init_salt_personal(state, outlen, salt, personal)` (unkeyed + salt/personal) | `0` | verified |
| 3.59 | `crypto_generichash_blake2b_init_salt_personal` | `key != NULL && keylen > 0` → `blake2b_init_key_salt_personal(...)` | `0` | verified |
| 3.60 | `crypto_generichash_blake2b_init_salt_personal` | `salt == NULL` and/or `personal == NULL` → the corresponding 16-byte param field is `memset` to 0 | `0`; NULL is legal | verified |
| 3.61 | `crypto_generichash_blake2b_init_salt_personal` | **missing** `COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state)` (present in `_init` at line 56 but absent here) — behavioural note only | n/a | unreachable-from-public-API (source-level note only, no runtime surface) |
| 3.62 | `crypto_generichash_blake2b_update` (generichash_blake2b.c:95) | no validation at all — no `outlen`, no NULL, no phase check; `inlen` cast to `uint64_t` | always `0` | verified |
| 3.63 | `crypto_generichash_blake2b_update` after `final` | `blake2b_update` has **no** `is_lastblock` guard → it keeps buffering into the finalized state (`f[0] == -1`) and mutating `S->buf`/`S->buflen` | **`0` — no error**. The subsequent `final` will return `-1` (3.69), so update-after-final is only detectable at the next `final` | verified |
| 3.64 | `crypto_generichash_blake2b_update` | `in == NULL && inlen == 0` → `while (inlen > 0)` never entered | `0` | verified |
| 3.65 | `crypto_generichash_blake2b_update` | `in == NULL && inlen > 0` → `memcpy` from NULL; **no runtime check** (header only has `nonnull(1)` on `state`) | UB — no defined C result; must be made unrepresentable in the port | unreachable-from-public-API (UB: `memcpy` from NULL — no defined C result to compare against) |
| 3.66 | `crypto_generichash_blake2b_final` (generichash_blake2b.c:104) | `assert(outlen <= UINT8_MAX)` — this assert is **NOT** paired with an early `return -1`. With asserts enabled: abort. Under `NDEBUG`: `outlen` is truncated by `(uint8_t) outlen`, so e.g. `outlen == 256` becomes `0` → `blake2b_final` sees `!outlen` → `sodium_misuse()` | abort (assert) / abort (`sodium_misuse` after truncation) — **never `-1`** | verified — **divergence found and fixed**: the port lacked `assert(outlen <= UINT8_MAX)`, so `outlen == 257` truncated to 1 and returned `0` where C aborts |
| 3.67 | `blake2b_final` (blake2b-ref.c:292) | `!outlen` (`outlen == 0`) | **`sodium_misuse()` → abort** | verified |
| 3.68 | `blake2b_final` | `outlen > BLAKE2B_OUTBYTES` (65..255 survive the `uint8_t` cast in 3.66) | **`sodium_misuse()` → abort** | verified |
| 3.69 | `blake2b_final` | `blake2b_is_lastblock(S)` i.e. `S->f[0] != 0` — **`final` called a second time on the same state** | **`-1`** (the only `-1` in blake2b-ref.c); `out` is not written | verified |
| 3.70 | `crypto_generichash_blake2b_final` with `outlen` ≠ the `outlen` given to `_init` | **NO check exists**: `blake2b_final` validates only `1 <= outlen <= 64` and then `memcpy(out, buffer, outlen)` from the 64-byte digest buffer | **`0`** — silently emits `outlen` bytes of a digest whose `digest_length` param was the *init* value. Digest-length mismatch is NOT an error in C | verified |
| 3.71 | `blake2b_final` | `assert(S->buflen <= BLAKE2B_BLOCKBYTES)` (line 306) after the `S->buflen > 128` compress-and-shift — an invariant assert; unreachable from any public path | never fires | unreachable-from-public-API (invariant assert; `buflen <= 128` always holds after the compress-and-shift) |
| 3.72 | `blake2b_final` | `COMPILER_ASSERT(sizeof buffer == 64U)` | compile-time only | unreachable-from-public-API (compile-time assert) |
| 3.73 | `blake2b_final` | `out == NULL` → `memcpy(out, ...)`; header has `nonnull` on `crypto_generichash_blake2b_final`, no runtime check | UB — no defined result | unreachable-from-public-API (UB: `memcpy` to NULL) |
| 3.74 | `blake2b_init` (blake2b-ref.c:126) | `!outlen \|\| outlen > BLAKE2B_OUTBYTES` | **`sodium_misuse()` → abort** (LCOV_EXCL: unreachable via the public wrappers, which pre-filter with `-1`) | verified (driven directly through the exported `_sodium_blake2b_init`) |
| 3.75 | `blake2b_init_salt_personal` (blake2b-ref.c:148) | `!outlen \|\| outlen > BLAKE2B_OUTBYTES` | **`sodium_misuse()` → abort** (unreachable via public wrappers) | verified (driven directly through `_sodium_blake2b_init_salt_personal`) |
| 3.76 | `blake2b_init_key` (blake2b-ref.c:179) | `!outlen \|\| outlen > BLAKE2B_OUTBYTES` | **`sodium_misuse()` → abort** | verified (driven directly through `_sodium_blake2b_init_key`) |
| 3.77 | `blake2b_init_key` | `!key` (key `NULL`) — reachable only if a caller passes `key==NULL, keylen>0`; the public `_init` routes that to `blake2b_init` instead (3.55), so unreachable from libsodium's own API | **`sodium_misuse()` → abort** | verified (driven directly through `_sodium_blake2b_init_key`) |
| 3.78 | `blake2b_init_key` | `!keylen` (`keylen == 0`) | **`sodium_misuse()` → abort** (unreachable: wrappers route `keylen==0` to `blake2b_init`) | verified (driven directly through `_sodium_blake2b_init_key`) |
| 3.79 | `blake2b_init_key` | `keylen > BLAKE2B_KEYBYTES` | **`sodium_misuse()` → abort** (unreachable: wrappers return `-1` first) | verified (driven directly through `_sodium_blake2b_init_key`) |
| 3.80 | `blake2b_init_key` | `blake2b_init_param(S, P) < 0` (line 202) | **`sodium_misuse()` → abort**; unreachable because `blake2b_init_param` always returns `0` | unreachable-from-public-API (`blake2b_init_param` always returns `0`) |
| 3.81 | `blake2b_init_key` | valid path side effect: absorbs one **zero-padded 128-byte block** containing the key (`blake2b_update(S, block, 128)`), then `sodium_memzero(block)`. Consequence: `S->buflen == 128` immediately after keyed init | `0` | verified |
| 3.82 | `blake2b_init_key_salt_personal` (blake2b-ref.c:216) | `!outlen \|\| outlen > 64` | **`sodium_misuse()` → abort** | verified (driven directly through `_sodium_blake2b_init_key_salt_personal`) |
| 3.83 | `blake2b_init_key_salt_personal` | `!key \|\| !keylen \|\| keylen > BLAKE2B_KEYBYTES` | **`sodium_misuse()` → abort** | verified (driven directly through `_sodium_blake2b_init_key_salt_personal`) |
| 3.84 | `blake2b_init_key_salt_personal` | `blake2b_init_param(S, P) < 0` (line 248) | **`sodium_misuse()` → abort**; unreachable | unreachable-from-public-API (`blake2b_init_param` always returns `0`) |
| 3.85 | `blake2b_init_param` (blake2b-ref.c:109) | `COMPILER_ASSERT(sizeof *P == 64)` — compile-time (relies on `#pragma pack(push,1)` around `blake2b_param`) | compile-time only | unreachable-from-public-API (compile-time assert) |
| 3.86 | `blake2b_init_param` | **no runtime error path**: `blake2b_init0` writes the IV and zeroes `t`,`f`,`buf`,`buflen`,`last_node` via one `memset` over `offsetof(last_node)+sizeof(last_node) - offsetof(t)`, then XORs the 64-byte param block into `h[0..7]` as 8 LE `uint64`s | always `0`; never `-1` | verified |
| 3.87 | `blake2b_init_param` | `P->digest_length == 0` or `> 64`, or `P->key_length > 64`, or nonzero `fanout`/`depth`/`node_offset`/… — **no validation** of the param block at this level | always `0`; arbitrary params are XOR-ed in verbatim | verified |
| 3.88 | `blake2b_state.last_node` | `blake2b_set_lastblock` (line 52) calls `blake2b_set_lastnode` (`S->f[1] = -1`) **only if** `S->last_node != 0`. `last_node` is zeroed by `blake2b_init0` and is **never set by any public or private entry point in this tree** (`blake2b_set_lastnode` is wholly inside `LCOV_EXCL_START/STOP`) | dead code — `f[1]` always stays `0` in this build; port may model `last_node` as always-false but must keep the field for state layout | verified (`f[1]` and `last_node` observed to stay `0` through init/update/final) |
| 3.89 | `blake2b_update` (blake2b-ref.c:263) | `inlen == 0` → `while (inlen > 0)` skipped | always `0` | verified |
| 3.90 | `blake2b_update` | boundary `inlen > fill` where `fill = 2*128 - S->buflen`: fills to 256, compresses the first 128 bytes, shifts the 2nd half down, `buflen -= 128`. The "lazy" `else` branch (`inlen <= fill`) never compresses — so `buflen` can legitimately be `128 < buflen <= 256` on entry to `final` | always `0`; no error, but this lazy-buffering shape is behaviourally load-bearing | verified |
| 3.91 | `blake2b_increment_counter` | `#else` (no `HAVE_TI_MODE`) branch: `S->t[0] += inc; S->t[1] += (S->t[0] < inc);` — 128-bit counter, silent wrap past 2^128 | no error | verified |
| 3.92 | `blake2b_compress_ref` (blake2b-compress-ref.c:31) | no validation, no error path; 12 rounds, `S->h[i] ^= v[i] ^ v[i+8]` | always `0` | verified |
| 3.93 | `blake2b_pick_best_implementation` / `_crypto_generichash_blake2b_pick_best_implementation` | with no `HAVE_*` macros all SIMD arms are compiled out → `blake2b_compress = blake2b_compress_ref` | always `0`; infallible | verified |
| 3.94 | `crypto_generichash` (crypto_generichash.c:54) | pure delegation to `crypto_generichash_blake2b` → inherits 3.34–3.46 exactly (`-1` for outlen 0 / >64 / keylen >64; abort for NULL-with-length) | as 3.34–3.46 | verified |
| 3.95 | `crypto_generichash_init` (crypto_generichash.c:62) | pure delegation to `crypto_generichash_blake2b_init` (note the C-side reordering: wrapper signature is `(state, key, keylen, outlen)`, same as callee) → inherits 3.50–3.56 | as 3.50–3.56 | verified |
| 3.96 | `crypto_generichash_update` (crypto_generichash.c:71) | delegation to `crypto_generichash_blake2b_update` → inherits 3.62–3.65 | always `0` | verified |
| 3.97 | `crypto_generichash_final` (crypto_generichash.c:80) | delegation to `crypto_generichash_blake2b_final` → inherits 3.66–3.70: `-1` only on double-final, abort on `outlen==0`/`>64`, and **`0` on outlen≠init-outlen** | as 3.66–3.70 | verified |
| 3.98 | `crypto_generichash_statebytes` / `crypto_generichash_blake2b_statebytes` | none; `(sizeof(state) + 63) & ~63` — i.e. 384 rounded to 384 | infallible | verified |
| 3.99 | `crypto_generichash_keygen` / `crypto_generichash_blake2b_keygen` | none; `randombytes_buf(k, 32)`, `void` return, cannot fail | no error surface | verified |
| 3.100 | `crypto_generichash_bytes*` / `keybytes*` / `saltbytes` / `personalbytes` / `primitive` accessors | none | constants above; infallible | verified |
| 3.101 | `crypto_shorthash_siphash24` (shorthash_siphash24_ref.c:6) | **no validation of anything**: `k` is dereferenced as two `LOAD64_LE` (16 bytes) with no NULL/length check; `inlen` unbounded | always `0`; infallible | verified |
| 3.102 | `crypto_shorthash_siphash24` | `inlen == 0` → `end = in` (the `inlen ? … : in` ternary avoids computing `in + inlen - …` on a possibly-NULL/one-past pointer), `left = 0`, `switch(left)` → `case 0: break` | `0`; writes 8 bytes = SipHash-2-4 of the empty message | verified |
| 3.103 | `crypto_shorthash_siphash24` | `inlen & 7` ∈ {1..7} → the fall-through `switch` reads `in[0..left-1]` past `end`; `b`'s top byte is `((uint64_t) inlen) << 56` (so `inlen mod 256` is what enters the tag) | `0`; note `inlen >= 256` silently aliases in the length byte — no error | verified |
| 3.104 | `crypto_shorthash_siphashx24` (shorthash_siphashx24_ref.c:6) | same absence of checks; differs only in `v1` init (`…646f83`), `v2 ^= 0xee`, and the extra `v1 ^= 0xdd` + 4 SIPROUNDs producing bytes 8..15 | always `0`; infallible; writes 16 bytes | verified |
| 3.105 | `crypto_shorthash` (crypto_shorthash.c:24) | pure delegation to `crypto_shorthash_siphash24` | always `0` | verified |
| 3.106 | `crypto_shorthash_keygen` | `randombytes_buf(k, 16)`, `void` | no error surface | verified |
| 3.107 | `crypto_shorthash_bytes` / `_keybytes` / `_primitive` / `siphash24_bytes` / `siphash24_keybytes` / `siphashx24_bytes` / `siphashx24_keybytes` | none | `8`,`16`,`"siphash24"`,`8`,`16`,`16`,`16`; infallible | verified |
| 3.108 | area-wide: `__attribute__((nonnull))` on the public headers | `crypto_hash*`/`crypto_xof_*`/`crypto_generichash*`/`crypto_shorthash*` mark `out` (`nonnull(1)`) and `state` as non-null; there is **no runtime enforcement** for the sha2/sha3/xof/shorthash families (only blake2b's one-shots check `out`/`key`/`in` at runtime) | UB in C; must be encoded as non-optional references in the port | undefined-behaviour-not-tested (`nonnull` params are UB, not a defined rejection; the defined `in == NULL && inlen == 0` case IS verified) |
| 3.109 | area-wide: `unsigned long long inlen` → `size_t` narrowing | `crypto_hash_sha3*_update`, `crypto_xof_*` (one-shot and `_update`) cast `(size_t) inlen`; on a 32-bit `size_t` this silently truncates. No check | no error reported; truncation | unreachable-from-public-API (`size_t` is 64-bit on this target, so the narrowing cast cannot truncate) |
| 3.110 | area-wide: no `sodium_init()` requirement | none of these functions checks initialization; `blake2b_pick_best_implementation` is only a dispatch setup and the ref path is the default | infallible w.r.t. init state | verified (tests/a3_crosscut.rs exercises every entry point; all are infallible w.r.t. init state) |

## Area 4 — crypto_auth + crypto_onetimeauth

Files analysed (READ-ONLY):
- `c_src/libsodium/crypto_auth/crypto_auth.c`
- `c_src/libsodium/crypto_auth/hmacsha256/auth_hmacsha256.c`
- `c_src/libsodium/crypto_auth/hmacsha512/auth_hmacsha512.c`
- `c_src/libsodium/crypto_auth/hmacsha512256/auth_hmacsha512256.c`
- `c_src/libsodium/crypto_onetimeauth/crypto_onetimeauth.c`
- `c_src/libsodium/crypto_onetimeauth/poly1305/onetimeauth_poly1305.c`
- `c_src/libsodium/crypto_onetimeauth/poly1305/donna/poly1305_donna.c` (`sse2/` is **not** selected: no `HAVE_TI_MODE`/`HAVE_EMMINTRIN_H`, so `poly1305_donna32.h` 32-bit limbs are used and `crypto_onetimeauth_poly1305_donna_implementation` is the only implementation ever installed)
- headers: `include/sodium/crypto_auth.h`, `crypto_auth_hmacsha256.h`, `crypto_auth_hmacsha512.h`, `crypto_auth_hmacsha512256.h`, `crypto_onetimeauth.h`, `crypto_onetimeauth_poly1305.h`

Return-value primitives used by this area (both take the portable, non-SSE2 branch in this build):
- `crypto_verify_16/32/64` (`crypto_verify/verify.c`) — constant-time, returns `0` iff all bytes equal, else `-1`.
- `sodium_memcmp` (`sodium/utils.c`) — constant-time, returns `0` iff equal, else `-1`.
- `sodium_misuse()` (`sodium/core.c`) — calls the installed misuse handler if any, then `abort()`; it **never returns**.

Every HMAC `*_verify` in this area returns the bitwise OR of three terms:
`crypto_verify_N(h, correct) | (-(h == correct)) | sodium_memcmp(correct, h, N)`
so any single failing term forces `-1`. The poly1305 verify uses only `crypto_verify_16`.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 4.1 | `crypto_auth_hmacsha256_verify` | at least one bit of `h[0..31]` differs from `HMAC-SHA-256(k, in)`; first OR-term `crypto_verify_32(h, correct)` yields `-1` | returns `-1` (constant time; no output written) | verified |
| 4.2 | `crypto_auth_hmacsha256_verify` | third OR-term `sodium_memcmp(correct, h, 32)` yields `-1` on the same mismatch — redundant second constant-time compare, must also be modelled so the result is `-1` even if `crypto_verify_32` were bypassed | returns `-1` | verified |
| 4.3 | `crypto_auth_hmacsha256_verify` | aliasing guard `-(h == correct)`: caller-supplied `h` pointer equal to the internal stack buffer `correct`. Unreachable from outside (the buffer is a fresh local), i.e. the term is always `0` in practice, but it is a *forced-rejection* branch: if it ever held, result is `-1` even for a matching tag | returns `-1` (dead branch; preserved semantics: pointer identity ⇒ reject) | unreachable-from-public-API |
| 4.4 | `crypto_auth_hmacsha512_verify` | at least one bit of `h[0..63]` differs from `HMAC-SHA-512(k, in)`; `crypto_verify_64(h, correct)` yields `-1` (64-byte compare, not 32) | returns `-1` | verified |
| 4.5 | `crypto_auth_hmacsha512_verify` | `sodium_memcmp(correct, h, 64)` yields `-1` (redundant second compare over the full 64 bytes) | returns `-1` | verified |
| 4.6 | `crypto_auth_hmacsha512_verify` | aliasing guard `-(h == correct)` holds | returns `-1` (dead branch) | unreachable-from-public-API |
| 4.7 | `crypto_auth_hmacsha512256_verify` | at least one bit of `h[0..31]` differs from the **first 32 bytes** of `HMAC-SHA-512(k, in)`; `crypto_verify_32(h, correct)` yields `-1`. Notably: passing bytes 32..63 of the untruncated SHA-512 tag also rejects | returns `-1` | verified |
| 4.8 | `crypto_auth_hmacsha512256_verify` | `sodium_memcmp(correct, h, 32)` yields `-1` | returns `-1` | verified |
| 4.9 | `crypto_auth_hmacsha512256_verify` | aliasing guard `-(h == correct)` holds | returns `-1` (dead branch) | unreachable-from-public-API |
| 4.10 | `crypto_auth_verify` (generic wrapper, `crypto_auth.c`) | tag mismatch — unconditionally delegates to `crypto_auth_hmacsha512256_verify`, so all of 4.7/4.8/4.9 propagate verbatim | returns `-1` | verified |
| 4.11 | `crypto_onetimeauth_poly1305_donna_verify` (reached via `crypto_onetimeauth_poly1305_verify` → `implementation->onetimeauth_verify`) | at least one bit of `h[0..15]` differs from `Poly1305(k, in)`; `crypto_verify_16(h, correct)` yields `-1`. **No** `sodium_memcmp` and **no** aliasing guard here — single OR-term only | returns `-1` | verified |
| 4.12 | `crypto_onetimeauth_verify` (generic wrapper, `crypto_onetimeauth.c`) | tag mismatch — unconditionally delegates to `crypto_onetimeauth_poly1305_verify`, which dispatches through the function-pointer table to 4.11 | returns `-1` | verified |
| 4.13 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `0 < keylen <= 64` (i.e. the `keylen > 64` branch is not taken, so the `else if (key == NULL)` arm runs and `keylen > 0`) → `sodium_misuse()` | never returns: misuse handler (if installed via `sodium_set_misuse_handler`) then `abort()`. Not an `int` error code | verified |
| 4.14 | `crypto_auth_hmacsha512_init` | `key == NULL` **and** `0 < keylen <= 128` (block size is 128 here, not 64) → `sodium_misuse()` | never returns: handler then `abort()` | verified |
| 4.15 | `crypto_auth_hmacsha512256_init` | same condition as 4.14 — the function is a pure cast-and-delegate to `crypto_auth_hmacsha512_init`, so `key == NULL && 0 < keylen <= 128` aborts | never returns: handler then `abort()` | verified |
| 4.16 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `keylen > 64`: the `keylen > 64` branch wins, so **no** `sodium_misuse()` fires; control reaches `crypto_hash_sha256_update(&state->ictx, NULL, keylen)` | **not a checked rejection** — undefined behaviour / NULL deref in C. Rust port must not treat this as a defined `-1`; document as unreachable/`debug_assert` | undefined-behaviour-not-tested |
| 4.17 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | `key == NULL` **and** `keylen > 128`: `keylen > 128` branch wins, no misuse, `crypto_hash_sha512_update(..., NULL, keylen)` | **not a checked rejection** — undefined behaviour, same treatment as 4.16 | undefined-behaviour-not-tested |
| 4.18 | `crypto_auth_hmacsha256_init` | `key == NULL` **and** `keylen == 0`: `else if (key == NULL)` taken but inner `if (keylen > 0)` is false → **no** misuse. Both XOR loops iterate zero times, so the HMAC is computed with an all-zero key | returns `0` (explicitly *not* an error — a rejection branch that is deliberately *not* taken) | verified |
| 4.19 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | `key == NULL` **and** `keylen == 0` → no misuse, all-zero 128-byte key material | returns `0` (not an error) | verified |
| 4.20 | `crypto_auth_hmacsha256_init` | key-length branch `keylen > 64` (non-error): `key` is replaced by `SHA-256(key)` and `keylen` is forced to `32`. `keylen == 64` exactly does **not** hash. Boundary must be `>` not `>=` | returns `0`; tag equals the tag for the 32-byte hashed key | verified |
| 4.21 | `crypto_auth_hmacsha512_init` / `crypto_auth_hmacsha512256_init` | key-length branch `keylen > 128` (non-error): `key` replaced by `SHA-512(key)`, `keylen` forced to `64`. `keylen == 128` exactly does **not** hash | returns `0`; tag equals the tag for the 64-byte hashed key | verified |
| 4.22 | `crypto_onetimeauth_poly1305_donna_init` | `COMPILER_ASSERT(sizeof(crypto_onetimeauth_poly1305_state) >= sizeof(poly1305_state_internal_t))` — expands to `(void) sizeof(char[(X) ? 1 : -1])`; a static/compile-time assertion, never a runtime rejection. Opaque state is 256 bytes vs. the donna32 internal struct | compile-time failure only; at runtime returns `0` | verified |
| 4.23 | all non-`verify` entry points in this area (`crypto_auth`, `crypto_auth_hmacsha256/512/512256`, all `_init`/`_update`/`_final`, `crypto_onetimeauth*`, `crypto_onetimeauth_poly1305*`, `_crypto_onetimeauth_poly1305_pick_best_implementation`) | no reachable rejection: they perform no length checks, no NULL checks (only `__attribute__((nonnull))` hints in the headers), and unconditionally `return 0`. `inlen` is `unsigned long long` and is never validated | always returns `0`; the only ways to fail are 4.13–4.15 (abort) and the `verify` paths | verified |
| 4.24 | `crypto_auth_keygen`, `crypto_auth_hmacsha256_keygen`, `crypto_auth_hmacsha512_keygen`, `crypto_auth_hmacsha512256_keygen`, `crypto_onetimeauth_keygen`, `crypto_onetimeauth_poly1305_keygen` | `void` return; only failure mode is inside `randombytes_buf` (out of scope for this area — it aborts on RNG failure rather than returning a code) | no error code exists; cannot return `-1` | verified |

## Area 5 — crypto_stream

Files covered (read in full):

- `c_src/libsodium/crypto_stream/crypto_stream.c`
- `c_src/libsodium/crypto_stream/chacha20/stream_chacha20.c`, `chacha20/stream_chacha20.h`, `chacha20/ref/chacha20_ref.c`
- `c_src/libsodium/crypto_stream/salsa20/stream_salsa20.c`, `salsa20/ref/salsa20_ref.c`
- `c_src/libsodium/crypto_stream/salsa2012/stream_salsa2012.c`, `salsa2012/ref/stream_salsa2012_ref.c`
- `c_src/libsodium/crypto_stream/salsa208/stream_salsa208.c`, `salsa208/ref/stream_salsa208_ref.c`
- `c_src/libsodium/crypto_stream/xsalsa20/stream_xsalsa20.c`
- `c_src/libsodium/crypto_stream/xchacha20/stream_xchacha20.c`
- headers `include/sodium/crypto_stream{,_chacha20,_salsa20,_salsa2012,_salsa208,_xsalsa20,_xchacha20}.h`

### Constants resolved (LP64, CMake build with no `HAVE_*` macros)

| symbol | value |
|--------|-------|
| `SODIUM_SIZE_MAX` = `SODIUM_MIN(UINT64_MAX, SIZE_MAX)` | `0xFFFF_FFFF_FFFF_FFFF` (= `UINT64_MAX`, since `SIZE_MAX == UINT64_MAX` on LP64) |
| `crypto_stream_salsa20/salsa2012/salsa208/xsalsa20/chacha20/xchacha20_MESSAGEBYTES_MAX` | `SODIUM_SIZE_MAX` = `2^64 - 1` |
| `crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX` = `SODIUM_MIN(SODIUM_SIZE_MAX, 64ULL * (1ULL << 32))` | `274877906944` = `2^38` = `0x40_0000_0000` |
| `crypto_stream_KEYBYTES` / `NONCEBYTES` / `PRIMITIVE` | `32` / `24` / `"xsalsa20"` (xsalsa20 aliases) |
| nonce sizes | salsa20/salsa2012/salsa208/chacha20 = 8; chacha20_ietf = 12; xsalsa20/xchacha20 = 24 |

### Global observations about the error surface

- **There is not a single `return -1` anywhere in area 5.** Every `int`-returning function in these files returns `0` unconditionally on the paths that return at all. The only non-zero-return exit is `sodium_misuse()`, which does not return (it calls the user misuse handler if installed, then `abort()` — see `c_src/libsodium/sodium/core.c:192-206`).
- **There is no runtime `assert()` in area 5.** The only asserts are `COMPILER_ASSERT(X)` = `(void) sizeof(char[(X) ? 1 : -1])`, i.e. compile-time-only (rows 5.20–5.22).
- **All nine `sodium_misuse()` sites are in `chacha20/stream_chacha20.c`.** salsa20, salsa2012, salsa208, xsalsa20 and xchacha20 have **zero** length/counter validation of their own.
- All the length-guarded functions take `unsigned long long mlen`/`clen`. Because `crypto_stream_chacha20_MESSAGEBYTES_MAX == UINT64_MAX` on LP64, the six `> crypto_stream_chacha20_MESSAGEBYTES_MAX` comparisons are **provably false / dead branches on this build**. Only the two `ietf_MESSAGEBYTES_MAX` (`2^38`) checks and the one 32-bit-counter check are reachable.
- Every public entry point is annotated `__attribute__((nonnull))` / `nonnull(1,4,5)` / `nonnull(1,4,6)`. NULL pointers are **not** checked at runtime — passing NULL is UB, not an error return (row 5.23).

### ERROR-SURFACE table

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 5.1 | `crypto_stream_chacha20` (`stream_chacha20.c:64`) | `clen > crypto_stream_chacha20_MESSAGEBYTES_MAX` i.e. `clen > UINT64_MAX` | `sodium_misuse()` → misuse handler then `abort()`. **Unreachable on LP64**: `clen` is `unsigned long long` so the predicate is always false. Dead branch; must be preserved semantically but is untriggerable. | dead-branch-on-this-build |
| 5.2 | `crypto_stream_chacha20_xor_ic` (`:74`) | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (`UINT64_MAX`) | `sodium_misuse()` → `abort()`. Unreachable on LP64 (same reason as 5.1). | dead-branch-on-this-build |
| 5.3 | `crypto_stream_chacha20_xor` (`:86`) | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (`UINT64_MAX`) | `sodium_misuse()` → `abort()`. Unreachable on LP64. | dead-branch-on-this-build |
| 5.4 | `crypto_stream_chacha20_ietf_ext` (`:97`) | `clen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (`UINT64_MAX`). **Note: the `_ext` variant deliberately checks the NON-ietf max, so it does NOT enforce the `2^38` IETF limit.** | `sodium_misuse()` → `abort()`. Unreachable on LP64 — meaning `_ietf_ext` accepts `clen > 2^38` and silently lets the 32-bit block counter overflow into nonce word 0 (see 5.16). | dead-branch-on-this-build |
| 5.5 | `crypto_stream_chacha20_ietf_ext_xor_ic` (`:107`) | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (`UINT64_MAX`); again NOT the ietf max | `sodium_misuse()` → `abort()`. Unreachable on LP64 → `_ext_xor_ic` performs no effective validation at all; `ic` is completely unchecked here. | dead-branch-on-this-build |
| 5.6 | `crypto_stream_chacha20_ietf_ext_xor` (static, `:119`; reached only from `crypto_stream_chacha20_ietf_xor`) | `mlen > crypto_stream_chacha20_MESSAGEBYTES_MAX` (`UINT64_MAX`) | `sodium_misuse()` → `abort()`. Unreachable on LP64, and additionally shadowed by the caller's stricter check (5.10). | dead-branch-on-this-build |
| 5.7 | `crypto_stream_chacha20_ietf` (`:130`) | `clen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX`, i.e. `clen > 274877906944` (`clen >= 274877906945`) | `sodium_misuse()` → misuse handler then `abort()`. **Reachable** (needs a 256 GiB output buffer). `clen == 274877906944` exactly is accepted. | checked-via-guard-only |
| 5.8 | `crypto_stream_chacha20_ietf_xor_ic` (`:140`) | 32-bit block-counter overflow guard: `(unsigned long long) ic > (64ULL * (1ULL << 32)) / 64ULL - (mlen + 63ULL) / 64ULL`, i.e. `ic > 4294967296 - ceil(mlen / 64)` | `sodium_misuse()` → misuse handler then `abort()`. **Reachable and cheap to trigger.** Concrete boundaries: `mlen ∈ [1,64]` → limit `4294967295` → never fires (`ic` is `uint32_t`). `mlen ∈ [65,128]` → limit `4294967294` → `ic == 0xFFFFFFFF` fires. `mlen ∈ [129,192]` → limit `4294967293` → `ic >= 0xFFFFFFFE` fires. `mlen == 2^38` → limit `0` → any `ic >= 1` fires, `ic == 0` accepted. `mlen == 0` → limit `4294967296` → never fires. | verified |
| 5.9 | `crypto_stream_chacha20_ietf_xor_ic` — **guard-underflow hole** (`:145-148`) | `mlen > 2^38` (i.e. `ceil(mlen/64) > 4294967296`, so `mlen >= 274877906945`). The RHS `4294967296 - ceil(mlen/64)` is computed in `unsigned long long` and **underflows** to a value near `2^64`. | **No `sodium_misuse()`, no error.** `ic` (max `0xFFFFFFFF`) is never `>` the wrapped RHS, so the guard silently passes; the call proceeds into `crypto_stream_chacha20_ietf_ext_xor_ic` whose own check (5.5) is dead. Net effect: **silent** 32-bit block-counter overflow that carries into nonce word 0 (`j13`), producing keystream reuse. Note `_ietf_xor_ic` never checks `mlen` against `ietf_MESSAGEBYTES_MAX` at all, unlike `_ietf` (5.7) and `_ietf_xor` (5.10). Semantics to preserve exactly. | checked-via-guard-only |
| 5.10 | `crypto_stream_chacha20_ietf_xor` (`:153`) | `mlen > crypto_stream_chacha20_ietf_MESSAGEBYTES_MAX`, i.e. `mlen >= 274877906945` | `sodium_misuse()` → `abort()`. **Reachable.** `mlen == 274877906944` exactly is accepted (with `ic == 0` this is exactly `2^32` blocks, the last legal one). | checked-via-guard-only |
| 5.11 | `crypto_stream_xchacha20` (`stream_xchacha20.c:29`) | inherits 5.1 via `crypto_stream_chacha20(c, clen, n+16, k2)` | `sodium_misuse()` if `clen > UINT64_MAX` → unreachable on LP64. xchacha20 itself adds **no** check; `crypto_stream_xchacha20_MESSAGEBYTES_MAX` is `SODIUM_SIZE_MAX` and is never compared against. | dead-branch-on-this-build |
| 5.12 | `crypto_stream_xchacha20_xor_ic` (`:45`) / `crypto_stream_xchacha20_xor` (`:57`) | inherits 5.2 via `crypto_stream_chacha20_xor_ic`. `ic` is `uint64_t` and reaches the **original** (8-byte-nonce, 64-bit-counter) chacha20 path, so the IETF `2^38`/32-bit constraints do **not** apply. | `sodium_misuse()` only if `mlen > UINT64_MAX` → unreachable. Otherwise always returns `0`. | dead-branch-on-this-build |
| 5.13 | `crypto_stream_chacha20_xor_ic` / `_xor` — 64-bit counter wraparound (`chacha20_ref.c:182-187`) | `ic + ceil(mlen/64) > 2^64`, e.g. `ic = 0xFFFFFFFFFFFFFFFF` with `mlen > 64` | **Silent wraparound, no error, returns 0.** `j12 = PLUSONE(j12)`; if `j12` became 0 then `j13 = PLUSONE(j13)`. Both are counter words for the original (8-byte-nonce) layout, so the 64-bit counter wraps `2^64 - 1 → 0` and keystream repeats. No check anywhere. | verified |
| 5.14 | `crypto_stream_chacha20_xor_ic` — 32→64 bit counter carry (`chacha20_ref.c:182-187`) | `ic` crosses a multiple of `2^32` mid-message, e.g. `ic = 0xFFFFFFFF, mlen >= 65` | **No error, returns 0.** Carry propagates from `j12` (`input[12]`) into `j13` (`input[13]`), which for the *original* nonce layout is the counter high word — correct 64-bit increment, not corruption. Contrast with 5.16. | verified |
| 5.15 | `crypto_stream_chacha20_ietf_ext` / `_ietf_ext_xor_ic` with `clen`/`mlen > 2^38` | 32-bit counter exhausted (`chacha20_ref.c:182-187`, ietf IV layout `chacha20_ref.c:72-78`) | **Silent, returns 0.** `j12` wraps `0xFFFFFFFF → 0` and the carry increments `j13`, which for `chacha_ietf_ivsetup` is **nonce word 0** (`LOAD32_LE(iv + 0)`). This is the documented `_ext` behaviour ("The ietf_ext variant allows the internal counter to overflow into the IV", `include/sodium/private/chacha20_ietf_ext.h`). No error signalled. | checked-via-guard-only |
| 5.16 | `crypto_stream_chacha20_ietf_ext_xor_ic` with large `ic` | `ic` near `0xFFFFFFFF` combined with `mlen >= 65` (no guard on this entry point at all — 5.5 is dead) | **Silent counter overflow into nonce word 0, returns 0.** This is the only way to reach the overflow with a small message; `crypto_stream_chacha20_ietf_xor_ic` would have rejected it via 5.8. | verified |
| 5.17 | `crypto_stream_salsa20_xor_ic` (`salsa20_ref.c:63`) and `crypto_stream_xsalsa20_xor_ic` (`stream_xsalsa20.c:22`) — 64-bit counter wraparound | `ic + ceil(mlen/64) > 2^64`, e.g. `ic = 0xFFFFFFFFFFFFFFFF, mlen = 128` | **Silent wraparound, no error, returns 0.** The counter lives in `in[8..15]` as a 64-bit LE value; the carry loop `u = 1; for (i = 8; i < 16; i++) { u += in[i]; in[i] = u; u >>= 8; }` drops the final carry out of `in[15]`, so `2^64 - 1 → 0`. **There is no counter or length check anywhere in the salsa20 family.** | verified |
| 5.18 | `crypto_stream_salsa20`, `_xor`, `_xor_ic`; `crypto_stream_salsa2012`, `_xor`; `crypto_stream_salsa208`, `_xor`; `crypto_stream_xsalsa20`, `_xor`, `_xor_ic`; `crypto_stream`, `crypto_stream_xor` | any `clen`/`mlen` including values `> *_MESSAGEBYTES_MAX` | **No validation at all.** These functions have no `sodium_misuse()`, no `return -1`, no assert. They always `return 0` (`crypto_stream_xsalsa20*` returns `ret` propagated from `crypto_stream_salsa20*`, which is structurally always `0`). The declared `*_MESSAGEBYTES_MAX` is advisory only. | verified |
| 5.19 | `crypto_stream_salsa2012_xor` / `crypto_stream_salsa208_xor` (and their keystream forms) — counter wraparound | `ceil(mlen/64) > 2^64` (unreachable in practice: needs `mlen > 2^70`) | **Silent, returns 0.** These have no `_xor_ic` entry point, so the counter always starts at 0 (`in[8..15] = 0`) and wraparound is only reachable via an absurd `mlen`. No check. | unreachable-from-public-API |
| 5.20 | `stream_ref` / `stream_ietf_ext_ref` (`chacha20_ref.c:232`, `:251`) | `COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES == 256 / 8)` | **Compile-time only** (`(void) sizeof(char[(X) ? 1 : -1])`). Holds (32 == 32). No runtime effect; no runtime error surface. | dead-branch-on-this-build |
| 5.21 | `crypto_stream_xchacha20` (`stream_xchacha20.c:35`) | `COMPILER_ASSERT(crypto_stream_chacha20_KEYBYTES <= sizeof k2)` (32 <= 32) | Compile-time only; holds. No runtime effect. | dead-branch-on-this-build |
| 5.22 | `crypto_stream_xchacha20` (`stream_xchacha20.c:36-38`) | `COMPILER_ASSERT(crypto_stream_chacha20_NONCEBYTES == crypto_stream_xchacha20_NONCEBYTES - crypto_core_hchacha20_INPUTBYTES)` (8 == 24 - 16) | Compile-time only; holds. No runtime effect. | dead-branch-on-this-build |
| 5.23 | every public entry point in area 5 | NULL `c`, `n`, or `k` (or NULL `m` for `_xor*`) — declared `__attribute__((nonnull))`, `nonnull(1,4,5)`, `nonnull(1,4,6)` | **No runtime check; undefined behaviour** (segfault in practice, or elided by the optimiser). This is a *contract*, not an error return. Exception: `mlen == 0` short-circuits before dereferencing `m`/`c` in every `_xor*` implementation, so `(NULL, 0)` happens to be benign — but is still formally UB per `nonnull`. | verified |
| 5.24 | `crypto_stream_keybytes`, `crypto_stream_noncebytes`, `crypto_stream_messagebytes_max`, `crypto_stream_primitive` (`crypto_stream.c:6,12,18,24`) | — | **Cannot fail.** Constant returns `32`, `24`, `SODIUM_SIZE_MAX` (`2^64-1`), and the string literal `"xsalsa20"` respectively. `crypto_stream_primitive` returns a pointer to static storage; never NULL. | verified |
| 5.25 | `crypto_stream_{salsa20,salsa2012,salsa208,xsalsa20,chacha20,chacha20_ietf,xchacha20}_{keybytes,noncebytes,messagebytes_max}` | — | **Cannot fail.** Constant returns. Note the narrowing: all return `size_t`, and `crypto_stream_chacha20_messagebytes_max()` returns `SODIUM_SIZE_MAX` while `crypto_stream_chacha20_ietf_messagebytes_max()` returns `274877906944`. The `salsa208` accessors are `__attribute__((deprecated))` but functionally identical. | verified |
| 5.26 | `crypto_stream_keygen` and all `crypto_stream_*_keygen` | — | **Cannot fail; `void` return.** Delegates to `randombytes_buf(k, 32)`, which aborts internally on RNG failure rather than reporting an error. No error surface in area 5 itself. | verified |
| 5.27 | `_crypto_stream_salsa20_pick_best_implementation` (`stream_salsa20.c:87`), `_crypto_stream_chacha20_pick_best_implementation` (`stream_chacha20.c:176`) | — | **Cannot fail; always `return 0`.** With no `HAVE_*` macros defined, every `#if` branch is removed and the functions unconditionally select `crypto_stream_salsa20_ref_implementation` / `crypto_stream_chacha20_ref_implementation`. | verified |
| 5.28 | `chacha20_encrypt_bytes` early exit (`chacha20_ref.c:92-94`) | `bytes == 0` | Returns immediately without touching `c` or the counter words. Not an error; a no-op guard. Combined with the `if (!clen) return 0;` / `if (!mlen) return 0;` guards in `stream_ref`, `stream_ietf_ext_ref`, `stream_ref_xor_ic`, `stream_ietf_ext_ref_xor_ic`, `salsa20_ref.c` `stream_ref`/`stream_ref_xor_ic`, `stream_salsa2012_ref.c`, `stream_salsa208_ref.c` → **zero-length input is always success with no writes**, for every primitive. | verified |
| 5.29 | `chacha20_encrypt_bytes` partial-block path (`chacha20_ref.c:113-121`, `206-211`) | `bytes % 64 != 0` on the final block | Not an error, but a behavioural cliff worth pinning: the tail is copied into a zero-filled 64-byte `tmp[64]`, encrypted, and only the first `bytes` bytes copied back to `ctarget`. Never over-writes `c` past `bytes`. `bytes == 64` exactly takes the direct (non-`tmp`) path (`bytes < 64` is false, `bytes <= 64` is true) and returns. | verified |

## Area 6 — crypto_aead / secretbox / secretstream

Scope: `c_src/libsodium/crypto_aead/{aegis128l,aegis256,aes256gcm,chacha20poly1305,xchacha20poly1305}`,
`c_src/libsodium/crypto_secretbox/**`, `c_src/libsodium/crypto_secretstream/xchacha20poly1305/**`
and the matching public headers in `c_src/libsodium/include/sodium/`.

### Build-configuration facts that drive this table

* The CMake build defines **no** `HAVE_*` macros. Therefore:
  * `aegis128l`/`aegis256` `implementation` stays `&aegis128l_soft_implementation` /
    `&aegis256_soft_implementation` (the `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H`
    blocks in `_crypto_aead_aegis*_pick_best_implementation()` are compiled out), so the portable
    `aegis*_soft.c` path is the only one reachable. Behaviour (return codes) is identical either way.
  * The whole `#if !((HAVE_ARMCRYPTO && NATIVE_LITTLE_ENDIAN) || (HAVE_TMMINTRIN_H && HAVE_WMMINTRIN_H))`
    block at `crypto_aead/aes256gcm/aead_aes256gcm.c:50-157` **is** compiled, i.e. the **stub**
    family is what links. `crypto_aead_aes256gcm_is_available()` returns **0**, and *every* other
    aes256gcm entry point (`_encrypt`, `_encrypt_detached`, `_decrypt`, `_decrypt_detached`,
    `_beforenm`, `_encrypt_afternm`, `_encrypt_detached_afternm`, `_decrypt_afternm`,
    `_decrypt_detached_afternm`) unconditionally sets `errno = ENOSYS` (aliased to `ENXIO` if
    `ENOSYS` is undefined) and returns **-1**, *without touching* `*clen_p` / `*mlen_p` /
    `*maclen_p` and without reading any input buffer. Rows 6.30–6.39 below.
  * `crypto_aead_aes256gcm_keybytes/nsecbytes/npubbytes/abytes/statebytes/messagebytes_max/keygen`
    live *outside* that `#if`, so they still work normally even though the cipher is unavailable.
* `sodium_misuse()` (`sodium/core.c:192`) calls the registered misuse handler if any and then
  `abort()`s. It **never returns**. Every `mlen > *_MESSAGEBYTES_MAX` overflow guard in this area
  (except the aegis `*_decrypt_detached` ones, which `return -1`) goes through `sodium_misuse()`,
  i.e. the "expected C result" is *process abort*, not an error code. These branches are marked
  `abort()` below and are unreachable on 64-bit hosts for realistically-sized inputs (they are
  all tagged `LCOV_EXCL_LINE` upstream).
* Relevant constants: `aegis128l` KEY 16 / NPUB 16 / **ABYTES 32** / NSEC 0;
  `aegis256` KEY 32 / NPUB 32 / **ABYTES 32** / NSEC 0;
  `aes256gcm` KEY 32 / NPUB 12 / ABYTES 16 / NSEC 0;
  `chacha20poly1305` KEY 32 / **NPUB 8** / ABYTES 16 / NSEC 0;
  `chacha20poly1305_ietf` KEY 32 / **NPUB 12** / ABYTES 16 / NSEC 0;
  `xchacha20poly1305_ietf` KEY 32 / **NPUB 24** / ABYTES 16 / NSEC 0;
  `secretbox` KEY 32 / NONCE 24 / MAC 16 / BOXZERO 16 / **ZERO 32**;
  `secretstream_xchacha20poly1305` KEY 32 / HEADER 24 / **ABYTES 17** (= 1 + 16),
  `TAG_MESSAGE 0x00`, `TAG_PUSH 0x01`, `TAG_REKEY 0x02`, `TAG_FINAL 0x03`.
* `MESSAGEBYTES_MAX`: `aegis128l`/`aegis256` = `MIN(SIZE_MAX-32, 2^61-1)`;
  `aes256gcm` = `MIN(SIZE_MAX-16, 16*(2^32-2))`; `chacha20poly1305` = `SIZE_MAX-16`;
  `chacha20poly1305_ietf` = `MIN(SIZE_MAX-16, 64*(2^32-1))`;
  `xchacha20poly1305_ietf` = `SIZE_MAX-16`;
  `secretbox_xsalsa20poly1305` / `secretbox_xchacha20poly1305` = `stream_MESSAGEBYTES_MAX - 16`;
  `secretstream` = `MIN(SIZE_MAX-17, 64*(2^32-2))`.
* `nsec` is `NSECBYTES == 0` for every AEAD here. Every implementation does `(void) nsec;`
  (`aead_aegis128l.c:115,136`, `aead_aegis256.c:115,135`, `aead_chacha20poly1305.c:38,122,212,293`,
  `aead_xchacha20poly1305.c:40,108`) — i.e. `nsec` is *always ignored*, `NULL` and non-`NULL` are
  indistinguishable, and it is never written on the decrypt side. There is no rejection branch for it.
* The combined `*_encrypt`/`*_decrypt` wrappers are the only place `clen_p`/`mlen_p` are written;
  they are all `if (ptr != NULL)`-guarded, so a `NULL` out-length pointer is **legal** and simply
  suppresses the store. Same for `maclen_p` in `*_encrypt_detached` and `outlen_p`/`mlen_p`/`tag_p`
  in secretstream. No rejection branch — but the *value* written on failure is load-bearing (0).

### ERROR-SURFACE table

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 6.1 | `crypto_aead_aegis128l_encrypt` | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:69`) | `sodium_misuse()` → misuse handler then `abort()`; never returns | verified |
| 6.2 | `crypto_aead_aegis128l_encrypt_detached` | `mlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:119`) | `sodium_misuse()` → `abort()`. NB `*maclen_p = 32` has *already* been stored at line 117 before the check | verified |
| 6.3 | `crypto_aead_aegis128l_encrypt_detached` | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:120`) | `sodium_misuse()` → `abort()` | verified |
| 6.4 | `crypto_aead_aegis128l_decrypt` | `clen < 32` (`clen < crypto_aead_aegis128l_ABYTES`, guard at `aead_aegis128l.c:92` not taken) — includes `clen == 0` and `clen == 31` | returns `-1`; if `mlen_p != NULL` then `*mlen_p = 0`; `m` untouched; detached path never entered | verified |
| 6.5 | `crypto_aead_aegis128l_decrypt` | `clen >= 32` but tag (last 32 bytes) does not verify — flipped ciphertext bit, flipped tag bit, wrong `k`, wrong `npub`, wrong/absent `ad` | returns `-1` (propagated from `_decrypt_detached`); if `mlen_p != NULL` then `*mlen_p = 0`; `m[0 .. clen-32)` is zeroed by `aegis128l_soft.c:249` | verified |
| 6.6 | `crypto_aead_aegis128l_decrypt_detached` | `clen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:137`) | returns `-1` immediately (does **not** abort — differs from the encrypt side); `m` untouched | verified |
| 6.7 | `crypto_aead_aegis128l_decrypt_detached` | `adlen > crypto_aead_aegis128l_MESSAGEBYTES_MAX` (`aead_aegis128l.c:138`) | returns `-1` immediately; `m` untouched | verified |
| 6.8 | `crypto_aead_aegis128l_decrypt_detached` | `m != NULL`, `crypto_verify_32(computed_mac, mac) != 0` (`aegis128l_soft.c:244,247`) | returns `-1`; `memset(m, 0, clen)` — plaintext buffer wiped, no partial plaintext leak | verified |
| 6.9 | `crypto_aead_aegis128l_decrypt_detached` | `m == NULL` (verify-only mode) and tag mismatch (`aegis128l_soft.c:225-229,248`) | returns `-1`; nothing written anywhere (the `memset` is skipped) | verified |
| 6.10 | `aegis128l_mac` (internal, via `encrypt_detached`/`decrypt_detached`) | `maclen` neither 16 nor 32 (`aegis128l_common.h:62-64`) | `memset(mac, 0, maclen)` then `-1`. **Unreachable from the public API** — `maclen` is hard-wired to `crypto_aead_aegis128l_ABYTES == 32` at `aead_aegis128l.c:113,134`. Documented for completeness only | unreachable-from-public-API |
| 6.11 | `crypto_aead_aegis256_encrypt` | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:69`) | `sodium_misuse()` → `abort()` | verified |
| 6.12 | `crypto_aead_aegis256_encrypt_detached` | `mlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:119`) | `sodium_misuse()` → `abort()`; `*maclen_p = 32` already stored at line 117 | verified |
| 6.13 | `crypto_aead_aegis256_encrypt_detached` | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:120`) | `sodium_misuse()` → `abort()` | verified |
| 6.14 | `crypto_aead_aegis256_decrypt` | `clen < 32` (`aead_aegis256.c:92`) | returns `-1`; `*mlen_p = 0` if `mlen_p != NULL`; `m` untouched | verified |
| 6.15 | `crypto_aead_aegis256_decrypt` | `clen >= 32` but tag mismatch (bit-flip in `c`, in the trailing tag, wrong `k`/`npub`/`ad`) | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-32)` zeroed (`aegis256_soft.c` / `aegis256_common.h:232`) | verified |
| 6.16 | `crypto_aead_aegis256_decrypt_detached` | `clen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:136`) | returns `-1` immediately | verified |
| 6.17 | `crypto_aead_aegis256_decrypt_detached` | `adlen > crypto_aead_aegis256_MESSAGEBYTES_MAX` (`aead_aegis256.c:137`) | returns `-1` immediately | verified |
| 6.18 | `crypto_aead_aegis256_decrypt_detached` | `m != NULL` and `crypto_verify_32(computed_mac, mac) != 0` (`aegis256_common.h:227,230`) | returns `-1`; `memset(m, 0, clen)` | verified |
| 6.19 | `crypto_aead_aegis256_decrypt_detached` | `m == NULL` and tag mismatch (`aegis256_common.h:208-211,231`) | returns `-1`; nothing written | verified |
| 6.20 | `aegis256_mac` (internal) | `maclen` neither 16 nor 32 (`aegis256_common.h:62-64`) | `memset(mac, 0, maclen)`, `-1`. Unreachable from public API (`maclen` fixed to 32) | unreachable-from-public-API |
| 6.21 | `crypto_aead_chacha20poly1305_encrypt` | `mlen > crypto_aead_chacha20poly1305_MESSAGEBYTES_MAX` (= `SIZE_MAX - 16`) (`aead_chacha20poly1305.c:89`) | `sodium_misuse()` → `abort()` | verified |
| 6.22 | `crypto_aead_chacha20poly1305_decrypt` | `clen < 16` (`clen < crypto_aead_chacha20poly1305_ABYTES`, `aead_chacha20poly1305.c:259`) — incl. `clen == 0`, `clen == 15` | returns `-1`; `*mlen_p = 0` if non-NULL; `m` untouched | verified |
| 6.23 | `crypto_aead_chacha20poly1305_decrypt` | `clen >= 16`, tag mismatch | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_chacha20poly1305.c:236`) | verified |
| 6.24 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m != NULL`, `crypto_verify_16(computed_mac, mac) != 0` (`aead_chacha20poly1305.c:230,235-238`) | returns `-1`; `memset(m, 0, clen)`; the ChaCha20 keystream XOR at line 240 is **not** executed | verified |
| 6.25 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m == NULL` and tag mismatch (`aead_chacha20poly1305.c:232-234`) | returns the raw `crypto_verify_16` result = `-1`; nothing written | verified |
| 6.26 | `crypto_aead_chacha20poly1305_ietf_encrypt` | `mlen > crypto_aead_chacha20poly1305_ietf_MESSAGEBYTES_MAX` (= `MIN(SIZE_MAX-16, 64*(2^32-1))`) (`aead_chacha20poly1305.c:177`) | `sodium_misuse()` → `abort()` | verified |
| 6.27 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen < 16` (`aead_chacha20poly1305.c:344`) | returns `-1`; `*mlen_p = 0`; `m` untouched | verified |
| 6.28 | `crypto_aead_chacha20poly1305_ietf_decrypt` | `clen >= 16`, tag mismatch | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_chacha20poly1305.c:321`) | verified |
| 6.29 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m != NULL`, `crypto_verify_16` fails (`aead_chacha20poly1305.c:315,320-323`) | returns `-1`; `memset(m, 0, clen)` | verified |
| 6.29a | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m == NULL` and tag mismatch (`aead_chacha20poly1305.c:317-319`) | returns `-1`; nothing written | verified |
| 6.30 | `crypto_aead_aes256gcm_is_available` | always, in this build (no `HAVE_TMMINTRIN_H`/`HAVE_WMMINTRIN_H`/`HAVE_ARMCRYPTO`) (`aead_aes256gcm.c:151-155`) | returns `0` — the cipher is permanently unavailable | verified |
| 6.31 | `crypto_aead_aes256gcm_encrypt` | any call, even with fully valid key/nonce/message (`aead_aes256gcm.c:69-76`) | `errno = ENOSYS`; returns `-1`. `*clen_p` is **not** written (differs from every other AEAD, which zeroes it) | verified |
| 6.32 | `crypto_aead_aes256gcm_encrypt_detached` | any call (`aead_aes256gcm.c:57-66`) | `errno = ENOSYS`; returns `-1`; `*maclen_p` not written; `c`/`mac` untouched | verified |
| 6.33 | `crypto_aead_aes256gcm_decrypt` | any call — valid ciphertext, `clen < 16`, `clen == 0`, tampered tag: all identical (`aead_aes256gcm.c:89-97`) | `errno = ENOSYS`; returns `-1`; `*mlen_p` **not** written | verified |
| 6.34 | `crypto_aead_aes256gcm_decrypt_detached` | any call (`aead_aes256gcm.c:78-87`) | `errno = ENOSYS`; returns `-1`; `m` untouched (not even zeroed) | verified |
| 6.35 | `crypto_aead_aes256gcm_beforenm` | any call, even with a valid 32-byte key and a properly aligned `crypto_aead_aes256gcm_state` (`aead_aes256gcm.c:99-104`) | `errno = ENOSYS`; returns `-1`; `st_` left **uninitialised** | verified |
| 6.36 | `crypto_aead_aes256gcm_encrypt_afternm` | any call (with or without a preceding successful `_beforenm`, which can never succeed) (`aead_aes256gcm.c:118-127`) | `errno = ENOSYS`; returns `-1`; `*clen_p` not written | verified |
| 6.37 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | any call (`aead_aes256gcm.c:106-116`) | `errno = ENOSYS`; returns `-1`; `*maclen_p` not written | verified |
| 6.38 | `crypto_aead_aes256gcm_decrypt_afternm` | any call (`aead_aes256gcm.c:140-149`) | `errno = ENOSYS`; returns `-1`; `*mlen_p` not written | verified |
| 6.39 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | any call (`aead_aes256gcm.c:129-138`) | `errno = ENOSYS`; returns `-1`; `m` untouched | verified |
| 6.40 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | `mlen > crypto_aead_xchacha20poly1305_ietf_MESSAGEBYTES_MAX` (= `SIZE_MAX - 16`) (`aead_xchacha20poly1305.c:185`) | `sodium_misuse()` → `abort()` | verified |
| 6.41 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen < 16` (`aead_xchacha20poly1305.c:237`) — incl. `clen == 0`, `clen == 15` | returns `-1`; `*mlen_p = 0` if non-NULL; `m` untouched | verified |
| 6.42 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | `clen >= 16`, tag mismatch (bit-flip anywhere in `c`, wrong `k`/`npub`/`ad`) | returns `-1`; `*mlen_p = 0`; `m[0 .. clen-16)` zeroed (`aead_xchacha20poly1305.c:136`) | verified |
| 6.43 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m != NULL`, `crypto_verify_16(computed_mac, mac) != 0` (`aead_xchacha20poly1305.c:130,135-138`) | returns `-1`; `memset(m, 0, clen)`; keystream XOR at line 140 skipped | verified |
| 6.44 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m == NULL` and tag mismatch (`aead_xchacha20poly1305.c:132-134`) | returns `-1`; nothing written | verified |
| 6.45 | `crypto_aead_*_encrypt_detached` (chacha20poly1305, chacha20poly1305_ietf, xchacha20poly1305_ietf) | **no** length guard exists in these three (`aead_chacha20poly1305.c:23`, `:107`, `aead_xchacha20poly1305.c:146`): they have neither `mlen > MESSAGEBYTES_MAX` nor `adlen` checks | always returns `0`. Contrast with aegis128l/aegis256 `_encrypt_detached` (rows 6.2/6.3/6.12/6.13), which *do* guard and abort. A translation must not add a rejection here | verified |
| 6.46 | `crypto_aead_*_encrypt_detached` with `maclen_p == NULL` | all six families | **legal**, not an error: the `if (maclen_p != NULL)` guard simply skips the store (`aead_aegis128l.c:116`, `aead_aegis256.c:116`, `aead_chacha20poly1305.c:69,157`, `aead_xchacha20poly1305.c:84`). Return `0` (or the aes256gcm `-1` of row 6.32) | verified |
| 6.47 | `crypto_aead_*_encrypt` with `clen_p == NULL` / `crypto_aead_*_decrypt` with `mlen_p == NULL` | all six families | **legal**, not an error: guarded stores; the return value alone conveys success/failure. Notably a `clen < ABYTES` rejection is then observable *only* via the `-1` return | verified |
| 6.48 | `crypto_aead_*_encrypt` / `_encrypt_detached` with `nsec != NULL` | all six families (`NSECBYTES == 0`) | ignored via `(void) nsec;` — identical result to `nsec == NULL`; **no rejection branch** | verified |
| 6.49 | `crypto_aead_*_decrypt` / `_decrypt_detached` with `nsec != NULL` (out-param) | all six families | ignored via `(void) nsec;`; the buffer is never written, even on success; **no rejection branch** | verified |
| 6.50 | `crypto_secretbox_easy` | `mlen > crypto_secretbox_MESSAGEBYTES_MAX` (`crypto_secretbox_easy.c:97`) | `sodium_misuse()` → `abort()` | verified |
| 6.51 | `crypto_secretbox_detached` | none — there is no length or validity check at all (`crypto_secretbox_easy.c:19-90`) | always returns `0`. `mlen == 0` is accepted (produces a MAC over the empty string) | verified |
| 6.52 | `crypto_secretbox_open_easy` | `clen < crypto_secretbox_MACBYTES` (= 16) (`crypto_secretbox_easy.c:170-172`) — incl. `clen == 0`, `clen == 15` | returns `-1` before any crypto; `m` untouched | verified |
| 6.53 | `crypto_secretbox_open_easy` | `clen >= 16` but MAC (leading 16 bytes of `c`) does not verify — flipped MAC bit, flipped ciphertext bit, wrong `k`, wrong `n` | returns `-1` (from `crypto_secretbox_open_detached`); `m` untouched (**not** zeroed, unlike the AEADs) | verified |
| 6.54 | `crypto_secretbox_open_detached` | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` (`crypto_secretbox_easy.c:127-130`) | `sodium_memzero(subkey)`, returns `-1`; `m` untouched; the salsa20 decryption is never run | verified |
| 6.55 | `crypto_secretbox_open_detached` with `m == NULL` | MAC verifies (`crypto_secretbox_easy.c:131-134`) | returns `0` — verify-only mode, **not** an error. With a bad MAC it returns `-1` via row 6.54 | verified |
| 6.56 | `crypto_secretbox_xchacha20poly1305_easy` | `mlen > crypto_secretbox_xchacha20poly1305_MESSAGEBYTES_MAX` (`secretbox_xchacha20poly1305.c:89`) | `sodium_misuse()` → `abort()` | verified |
| 6.57 | `crypto_secretbox_xchacha20poly1305_detached` | none — no checks (`secretbox_xchacha20poly1305.c:19-80`) | always returns `0` | verified |
| 6.58 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen < crypto_secretbox_xchacha20poly1305_MACBYTES` (= 16) (`secretbox_xchacha20poly1305.c:164-166`) | returns `-1` before any crypto | verified |
| 6.59 | `crypto_secretbox_xchacha20poly1305_open_easy` | `clen >= 16` but MAC mismatch | returns `-1` (from `_open_detached`); `m` untouched | verified |
| 6.60 | `crypto_secretbox_xchacha20poly1305_open_detached` | `crypto_onetimeauth_poly1305_verify(mac, c, clen, block0) != 0` (`secretbox_xchacha20poly1305.c:120-123`) | `sodium_memzero(subkey)`, returns `-1`; `m` untouched | verified |
| 6.61 | `crypto_secretbox_xchacha20poly1305_open_detached` with `m == NULL` | MAC verifies (`secretbox_xchacha20poly1305.c:124-127`) | returns `0` — verify-only mode, not an error | verified |
| 6.62 | `crypto_secretbox` → `crypto_secretbox_xsalsa20poly1305` | `mlen < 32` (`mlen < crypto_secretbox_ZEROBYTES`) (`secretbox_xsalsa20poly1305.c:15-17`) — the NaCl-style API requires the caller to prepend 32 zero bytes, so `mlen` counts padding+plaintext; `mlen ∈ {0,1,16,31}` all rejected | returns `-1`; `c` untouched | verified |
| 6.63 | `crypto_secretbox` → `crypto_secretbox_xsalsa20poly1305` | `mlen >= 32` but `m[0..31]` are **not** all zero (no explicit check exists; `secretbox_xsalsa20poly1305.c:18-19` XORs the keystream over `m[0..31]` and derives the Poly1305 key from `c[0..31]`) | returns `0` — **silently accepted**. The produced box is unopenable: `crypto_secretbox_open` derives `subkey` from the raw keystream and will fail MAC verification (row 6.65). This is a latent correctness hazard, not a rejection | verified |
| 6.64 | `crypto_secretbox_open` → `crypto_secretbox_xsalsa20poly1305_open` | `clen < 32` (`clen < crypto_secretbox_ZEROBYTES`) (`secretbox_xsalsa20poly1305.c:35-37`) — `clen ∈ {0,1,16,17,31}` all rejected. NB the leading 16 bytes of `c` must be zero padding (`BOXZEROBYTES`) with the MAC at `c+16` | returns `-1`; `m` untouched | verified |
| 6.65 | `crypto_secretbox_open` → `crypto_secretbox_xsalsa20poly1305_open` | `clen >= 32` but `crypto_onetimeauth_poly1305_verify(c+16, c+32, clen-32, subkey) != 0` — flipped MAC/ciphertext bit, wrong `k`/`n`, or the caller failed to zero `c[0..15]` | returns `-1`; `m` untouched (**not** zeroed) | verified |
| 6.66 | `crypto_secretbox_open` | `clen >= 32`, MAC verifies, but `c[0..15]` is non-zero garbage | the padding bytes are *not* validated; they are decrypted and then `m[0..31]` is force-zeroed (`secretbox_xsalsa20poly1305.c:45-47`). Returns `0`. In practice a non-zero `c[0..15]` changes nothing that the MAC covers, so this is reachable and must round-trip identically | verified |
| 6.67 | `crypto_secretbox_xchacha20poly1305` (NaCl-style, zero-padded) | — | **does not exist**: the xchacha20poly1305 secretbox family only provides `_easy`/`_open_easy`/`_detached`/`_open_detached` (`secretbox_xchacha20poly1305.c`, `crypto_secretbox_xchacha20poly1305.h`). Any translation must not expose a NaCl-style variant here | verified |
| 6.68 | `crypto_secretstream_xchacha20poly1305_init_push` | none — no validation of `state`, `out` or `k` (`secretstream_xchacha20poly1305.c:42-65`) | always returns `0`; `out` filled with 24 random header bytes | verified |
| 6.69 | `crypto_secretstream_xchacha20poly1305_init_pull` | none — the 24-byte header is **not** validated in any way (it is fed straight into `crypto_core_hchacha20`); an all-zero header, a truncated/garbage header, or a header from a different session are all accepted (`secretstream_xchacha20poly1305.c:67-80`) | always returns `0`. The mismatch only surfaces later as a `_pull` MAC failure (row 6.75) | verified |
| 6.70 | `crypto_secretstream_xchacha20poly1305_push` | `mlen > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` (= `MIN(SIZE_MAX-17, 64*(2^32-2))`) (`secretstream_xchacha20poly1305.c:128-130`) | `sodium_misuse()` → `abort()`. NB `*outlen_p = 0` has already been stored at lines 123-125 | verified |
| 6.71 | `crypto_secretstream_xchacha20poly1305_push` | any other input, incl. `mlen == 0`, `ad == NULL`/`adlen == 0`, an out-of-range `tag` byte (e.g. `0x04`..`0xff`) — the `tag` value is **never validated** | returns `0`; `*outlen_p = 17 + mlen`. A `tag` with bit `0x02` (`TAG_REKEY`) set — which includes `TAG_FINAL == 0x03` and any bogus tag with that bit — triggers an implicit `_rekey()` (line 168-172) | verified |
| 6.72 | `crypto_secretstream_xchacha20poly1305_push` | 32-bit counter wraps to zero after `sodium_increment` (`secretstream_xchacha20poly1305.c:169-170`) — i.e. `2^32 - 1` messages pushed since the last rekey | returns `0` but performs an implicit `_rekey()`. Not an error; the pull side wraps identically so the session stays in sync | verified |
| 6.73 | `crypto_secretstream_xchacha20poly1305_push` with `outlen_p == NULL` | — | **legal**: both stores are `if (outlen_p != NULL)`-guarded (lines 123, 173). Returns `0` | verified |
| 6.74 | `crypto_secretstream_xchacha20poly1305_pull` | `inlen < crypto_secretstream_xchacha20poly1305_ABYTES` (= 17) (`secretstream_xchacha20poly1305.c:201-203`) — incl. `inlen == 0`, `1`, `16` | returns `-1`; `*mlen_p = 0` and `*tag_p = 0xff` were already stored at lines 195-200; `m` untouched; state **unchanged** (no nonce/counter advance) | verified |
| 6.75 | `crypto_secretstream_xchacha20poly1305_pull` | `sodium_memcmp(mac, stored_mac, 16) != 0` (`secretstream_xchacha20poly1305.c:239-242`) — tampered `in[0]` tag byte, tampered ciphertext, tampered trailing MAC, wrong/absent `ad`, wrong key, header mismatch from row 6.69, or a stream replayed/reordered out of sequence | `sodium_memzero(mac)`, returns `-1`; `*mlen_p` stays `0`, `*tag_p` stays `0xff`; `m` untouched (**not** zeroed); state **unchanged**, so the session is not advanced and a correct frame can still be pulled afterwards | verified |
| 6.76 | `crypto_secretstream_xchacha20poly1305_pull` | `mlen = inlen - 17 > crypto_secretstream_xchacha20poly1305_MESSAGEBYTES_MAX` (`secretstream_xchacha20poly1305.c:205-207`) | `sodium_misuse()` → `abort()` | verified |
| 6.77 | `crypto_secretstream_xchacha20poly1305_pull` after a `TAG_FINAL` frame | the C code does **not** latch a "finished" flag; `_pull` will happily be called again. Because `TAG_FINAL` (`0x03`) has the `TAG_REKEY` bit set, `_pull` rekeyed the state, so the next frame's MAC will not match | returns `-1` via row 6.75. There is no dedicated "stream already ended" error code | verified |
| 6.78 | `crypto_secretstream_xchacha20poly1305_pull` with `mlen_p == NULL` and/or `tag_p == NULL` | — | **legal**: all four stores are NULL-guarded (lines 195-200, 255-260). On the `inlen < 17` and MAC-mismatch paths the caller then only sees `-1` | verified |
| 6.79 | `crypto_secretstream_xchacha20poly1305_pull` with `m == NULL` | `mlen > 0` — unlike the AEADs, `_pull` has **no** `m == NULL` verify-only branch; line 245 unconditionally calls `crypto_stream_chacha20_ietf_xor_ic(m, c, mlen, ...)` | **undefined behaviour** (NULL deref) in C. Not a rejection row: a translation must either forbid this at the type level or document it. Only safe when `mlen == 0` | undefined-behaviour-not-tested |
| 6.80 | `crypto_secretstream_xchacha20poly1305_rekey` | none — `void` return, no validation (`secretstream_xchacha20poly1305.c:82-108`) | cannot fail; derives a new `state->k` + inonce and resets the counter to 1. An explicit `_rekey()` on only one side of the session desynchronises it, and every subsequent `_pull` then fails with `-1` (row 6.75) | verified |
| 6.81 | `crypto_secretstream_xchacha20poly1305_pull` with an `ad` that differs from the pushed `ad` (incl. `NULL`/0 vs non-empty of the same content) | MAC covers `ad` and `adlen` (`secretstream_xchacha20poly1305.c:212-214, 230-231`) | returns `-1` via row 6.75 | verified |
| 6.82 | all `*_keygen` (`crypto_aead_aegis128l_keygen`, `_aegis256_keygen`, `_aes256gcm_keygen`, `_chacha20poly1305_keygen`, `_chacha20poly1305_ietf_keygen`, `_xchacha20poly1305_ietf_keygen`, `crypto_secretbox_keygen`, `crypto_secretbox_xsalsa20poly1305_keygen`, `crypto_secretstream_xchacha20poly1305_keygen`) | none | `void` return; cannot fail. `crypto_aead_aes256gcm_keygen` still works despite row 6.30. NB there is **no** `crypto_secretbox_xchacha20poly1305_keygen` in `secretbox_xchacha20poly1305.c` | verified |
| 6.83 | all `*_keybytes`/`_nsecbytes`/`_npubbytes`/`_abytes`/`_messagebytes_max`/`_macbytes`/`_noncebytes`/`_zerobytes`/`_boxzerobytes`/`_statebytes`/`_headerbytes`/`_primitive`/`_tag_*` getters | none | pure constant returns; cannot fail. `crypto_aead_aes256gcm_statebytes()` = `(sizeof(crypto_aead_aes256gcm_state) + 15) & ~15`; `crypto_secretbox_primitive()` = `"xsalsa20poly1305"` | verified |

## Area 7 — scalarmult / sign / box / kx / kdf / kem

Files covered (read in full):

- `c_src/libsodium/crypto_scalarmult/crypto_scalarmult.c`
- `c_src/libsodium/crypto_scalarmult/curve25519/scalarmult_curve25519.c`, `curve25519/ref10/x25519_ref10.c`
- `c_src/libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`
- `c_src/libsodium/crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`
- `c_src/libsodium/crypto_sign/crypto_sign.c`, `ed25519/sign_ed25519.c`, `ed25519/ref10/{keypair.c,sign.c,open.c}`
- `c_src/libsodium/crypto_box/{crypto_box.c,crypto_box_easy.c,crypto_box_seal.c}`,
  `curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c`,
  `curve25519xchacha20poly1305/{box_curve25519xchacha20poly1305.c,box_seal_curve25519xchacha20poly1305.c}`
- `c_src/libsodium/crypto_kx/crypto_kx.c`
- `c_src/libsodium/crypto_kdf/crypto_kdf.c`, `blake2b/kdf_blake2b.c`, `hkdf/kdf_hkdf_sha256.c`, `hkdf/kdf_hkdf_sha512.c`
- `c_src/libsodium/crypto_kem/crypto_kem.c`, `mlkem768/kem_mlkem768.c`, `mlkem768/ref/kem_mlkem768_ref.c`, `xwing/kem_xwing.c`
- headers `include/sodium/crypto_scalarmult{,_curve25519,_ed25519,_ristretto255}.h`, `crypto_sign{,_ed25519}.h`,
  `crypto_box{,_curve25519xsalsa20poly1305,_curve25519xchacha20poly1305}.h`, `crypto_kx.h`,
  `crypto_kdf{,_blake2b,_hkdf_sha256,_hkdf_sha512}.h`, `crypto_kem{,_mlkem768,_xwing}.h`

### Constants resolved (LP64, CMake build with no `HAVE_*` macros)

| symbol | value |
|--------|-------|
| `crypto_scalarmult_curve25519_BYTES` / `_SCALARBYTES` | `32` / `32` |
| `crypto_scalarmult_BYTES` / `_SCALARBYTES` / `_PRIMITIVE` | `32` / `32` / `"curve25519"` (aliases curve25519) |
| `crypto_scalarmult_ed25519_BYTES` / `_SCALARBYTES` | `32` / `32` |
| `crypto_scalarmult_ristretto255_BYTES` / `_SCALARBYTES` | `32` / `32` |
| `crypto_sign_ed25519_BYTES` / `_SEEDBYTES` / `_PUBLICKEYBYTES` / `_SECRETKEYBYTES` | `64` / `32` / `32` / `64` |
| `crypto_sign_ed25519_MESSAGEBYTES_MAX` = `SODIUM_SIZE_MAX - 64` | `0xFFFF_FFFF_FFFF_FFBF` = `2^64 - 65` |
| `crypto_sign_BYTES/SEEDBYTES/PUBLICKEYBYTES/SECRETKEYBYTES/PRIMITIVE` | `64` / `32` / `32` / `64` / `"ed25519"` (aliases ed25519) |
| `crypto_box_*_SEEDBYTES` / `_PUBLICKEYBYTES` / `_SECRETKEYBYTES` / `_BEFORENMBYTES` | `32` / `32` / `32` / `32` (both xsalsa and xchacha) |
| `crypto_box_*_NONCEBYTES` / `_MACBYTES` | `24` / `16` (both) |
| `crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES` / `_ZEROBYTES` | `16` / `16 + 16 = 32` (NaCl-style only; xchacha has neither) |
| `crypto_box_SEALBYTES` / `crypto_box_curve25519xchacha20poly1305_SEALBYTES` | `32 + 16 = 48` / `32 + 16 = 48` |
| `crypto_box_*_MESSAGEBYTES_MAX` | `crypto_secretbox_*_MESSAGEBYTES_MAX` = `SODIUM_SIZE_MAX - 16` = `2^64 - 17` |
| `crypto_box_PRIMITIVE` | `"curve25519xsalsa20poly1305"` |
| `crypto_kx_PUBLICKEYBYTES` / `_SECRETKEYBYTES` / `_SEEDBYTES` / `_SESSIONKEYBYTES` / `_PRIMITIVE` | `32` / `32` / `32` / `32` / `"x25519blake2b"` |
| `crypto_kdf_blake2b_BYTES_MIN` / `_BYTES_MAX` / `_CONTEXTBYTES` / `_KEYBYTES` | `16` / `64` / `8` / `32` |
| `crypto_kdf_BYTES_MIN` / `_BYTES_MAX` / `_CONTEXTBYTES` / `_KEYBYTES` / `_PRIMITIVE` | `16` / `64` / `8` / `32` / `"blake2b"` (aliases blake2b) |
| `crypto_kdf_hkdf_sha256_KEYBYTES` / `_BYTES_MIN` / `_BYTES_MAX` | `32` / `0` / `0xff * 32 = 8160` |
| `crypto_kdf_hkdf_sha512_KEYBYTES` / `_BYTES_MIN` / `_BYTES_MAX` | `64` / `0` / `0xff * 64 = 16320` |
| `crypto_kem_mlkem768_PUBLICKEYBYTES` / `_SECRETKEYBYTES` / `_CIPHERTEXTBYTES` / `_SHAREDSECRETBYTES` / `_SEEDBYTES` | `1184` / `2400` / `1088` / `32` / `64` |
| `crypto_kem_xwing_PUBLICKEYBYTES` / `_SECRETKEYBYTES` / `_CIPHERTEXTBYTES` / `_SHAREDSECRETBYTES` / `_SEEDBYTES` | `1216` / `32` / `1120` / `32` / `32` |
| `crypto_kem_*` generic / `_PRIMITIVE` | aliases xwing / `"xwing"` |
| mlkem768 internals | `Q=3329`, `N=256`, `K=3`, `POLYBYTES=384`, `POLYVECBYTES=1152`, `POLYVECCOMPRESSEDBYTES_DU=960`, `POLYCOMPRESSEDBYTES_DV=128` |

### Global observations about the error surface

- **`-1` is the only error code in area 7.** No function here sets a distinguishing errno except the two kdf bound checks (`kdf_blake2b.c:45`, `kdf_hkdf_sha256.c:66`, `kdf_hkdf_sha512.c:66` set `errno = EINVAL` before `return -1`). Everything else returns bare `-1` / `0`.
- **`sodium_misuse()` sites (abort, not an error return):** `crypto_box_easy_afternm`, `crypto_box_easy`, `crypto_box_seal` (`crypto_box_easy.c:45,57`, `crypto_box_seal.c:34`), the two xchacha equivalents plus `..._seal` (`box_curve25519xchacha20poly1305.c:94,106`, `box_seal_curve25519xchacha20poly1305.c:40`), and `crypto_kx_{client,server}_session_keys` when **both** `rx` and `tx` are NULL (`crypto_kx.c:52,93`).
- **Output-buffer aliasing on failure is load-bearing.** In `scalarmult_ed25519_ref10.c` and `scalarmult_ristretto255_ref10.c` the clamped scalar is staged in `q` itself (`unsigned char *t = q;`), and in `x25519_ref10.c:176` `crypto_scalarmult_curve25519_ref10_base` does the same. So on the *late* rejection paths (`_is_inf` / `sodium_is_zero(q,…)`) the caller's `q` has already been fully overwritten with the result; on the *early* rejection paths (`ge25519_is_canonical` / `ristretto255_frombytes` / `has_small_order(p)`) `q` is untouched. A Rust port must reproduce which is which.
- **`crypto_scalarmult_curve25519` has two independent guards**: the ref10 `has_small_order(p)` blocklist (`x25519_ref10.c:106`) *and* the post-hoc all-zero-output check `return -(1 & ((d - 1) >> 8));` (`scalarmult_curve25519.c:24-27`). Note `d` is `volatile unsigned char`; with `d == 0`, `d - 1` promotes to `int` `-1`, `-1 >> 8 == -1`, `1 & -1 == 1`, so the return is `-1`. With `d != 0` the expression is `-(1 & 0) == 0`.
- **`crypto_scalarmult_curve25519_base` bypasses the wrapper entirely** — it calls `crypto_scalarmult_curve25519_ref10_implementation.mult_base` directly (`scalarmult_curve25519.c:33-35`), so there is **no** all-zero-output check on the `_base` path, and `crypto_scalarmult_curve25519_ref10_base` unconditionally `return 0`. `_base` can never fail.
- **ed25519 signature verification is cofactored**: the last line of `_crypto_sign_ed25519_verify_detached` is `return ge25519_has_small_order(&check) - 1;` — success (`0`) requires `check = R - (sB - hA)` to be a *small-order* point, not strictly the identity. This accepts signatures off by any torsion component.
- **mlkem768 decapsulation never fails.** `mlkem768_ref_dec` always `return 0` and uses *implicit rejection*: on re-encryption mismatch it constant-time-swaps in `SHAKE256(z || ct)`. There is no ciphertext validity check whatsoever. Hence the `if (crypto_kem_mlkem768_dec(...) != 0)` branch in `crypto_kem_xwing_dec` (`kem_xwing.c:188`) is dead (it is marked `LCOV_EXCL`).
- **mlkem768 `_enc`/`_enc_deterministic` is the only kem function with a real validity check**: `polyvec_is_canonical(&pkpv) == 0` → `-1` (`kem_mlkem768_ref.c:745-747`), i.e. any of the 768 unpacked 12-bit coefficients of `pk[0..1151]` being `>= 3329`.
- **Every public entry point is `__attribute__((nonnull))`.** NULL pointers are UB, never an error return — with the single, deliberate exception of `rx`/`tx` in `crypto_kx_{client,server}_session_keys`, which are explicitly NULL-checked and mutually substituted.
- **No `assert()` anywhere in area 7.** The only asserts are `COMPILER_ASSERT` (compile-time): `x25519_ref10.c:56` (blocklist has 7 entries), `crypto_box_easy.c:29`, `box_curve25519xchacha20poly1305.c:74-75`, `crypto_box_seal.c:60`, `box_seal_curve25519xchacha20poly1305.c:68-69`, `crypto_kx.c:26-27,57,98`.
- **`has_small_order` ignores the top bit of `p`**: the final byte is compared as `(s[31] & 0x7f)` (`x25519_ref10.c:63`), so setting bit 255 on a blocklisted encoding does **not** evade the guard.

### ERROR-SURFACE table

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|-------|
| 7.1 | `crypto_scalarmult_curve25519` (`scalarmult_curve25519.c:15`) → `crypto_scalarmult_curve25519_ref10` (`x25519_ref10.c:106`) | `p` = 32 zero bytes (blocklist entry 0, "0", order 4) | `-1`. `has_small_order(p) != 0` → `mult` returns `-1` → wrapper returns `-1` at `:22`. `q` **not** written (early reject before the ladder). | verified |
| 7.2 | same | `p` = `01 00 … 00` (blocklist entry 1, "1", order 1) | `-1`, `q` untouched. | verified |
| 7.3 | same | `p` = `e0 eb 7a 7c 3b 41 b8 ae 16 56 e3 fa f1 9f c4 6a da 09 8d eb 9c 32 b1 fd 86 62 05 16 5f 49 b8 00` (order 8) | `-1`, `q` untouched. | verified |
| 7.4 | same | `p` = `5f 9c 95 bc a3 50 8c 24 b1 d0 b1 55 9c 83 ef 5b 04 44 5c c4 58 1c 8e 86 d8 22 4e dd d0 9f 11 57` (order 8) | `-1`, `q` untouched. | verified |
| 7.5 | same | `p` = `ec ff … ff 7f` (`p-1`, order 2) | `-1`, `q` untouched. | verified |
| 7.6 | same | `p` = `ed ff … ff 7f` (`= p`, i.e. the non-canonical encoding of 0, order 4) | `-1`, `q` untouched. | verified |
| 7.7 | same | `p` = `ee ff … ff 7f` (`= p+1`, non-canonical encoding of 1, order 1) | `-1`, `q` untouched. | verified |
| 7.8 | same | any of 7.1–7.7 with bit 255 set (`p[31] |= 0x80`) | `-1` still. The comparison masks `s[31] & 0x7f` (`x25519_ref10.c:63`), so the high bit cannot be used to bypass the blocklist. | verified |
| 7.9 | `crypto_scalarmult_curve25519` — post-hoc all-zero-output guard (`scalarmult_curve25519.c:24-27`) | `implementation->mult` returns `0` but the 32 output bytes are all zero (`d == 0`) | `-1` via `-(1 & ((d - 1) >> 8))`. **Unreachable with ref10** because every low-order input is already rejected by 7.1–7.8 — but it is a distinct branch that must be preserved (it is the defence-in-depth guard for a hypothetical optimised backend). `q` **has** been written (all zeros) when this fires. | unreachable-from-public-API |
| 7.10 | `crypto_scalarmult` (`crypto_scalarmult.c:17`) | any of 7.1–7.9 | `-1`. Thin alias for `crypto_scalarmult_curve25519`; adds nothing. | verified |
| 7.11 | `crypto_scalarmult_curve25519_base` (`scalarmult_curve25519.c:31`) | **none** — including `n` = 32 zero bytes, `n` = `ff…ff`, `n` = `L` | Always `0`. `crypto_scalarmult_curve25519_ref10_base` clamps (`t[0] &= 248; t[31] &= 127; t[31] |= 64`) into `t = q` and unconditionally `return 0` (`x25519_ref10.c:191`). There is **no** all-zero check and **no** small-order check on this path (the wrapper is bypassed). `n = 0` yields the valid X25519 pk for scalar `2^254`. | verified |
| 7.12 | `crypto_scalarmult_base` (`crypto_scalarmult.c:11`) | **none** | Always `0`. Alias for 7.11. | verified |
| 7.13 | `crypto_scalarmult_ed25519` / `_noclamp` (`scalarmult_ed25519_ref10.c:39`) | `p` is a non-canonical point encoding: `p[31] & 0x7f == 0x7f`, `p[1..30] == 0xff`, `p[0] >= 0xed` (i.e. `y >= 2^255 - 19`) | `-1` (`ge25519_is_canonical(p) == 0`). Early reject: `q` untouched. | verified |
| 7.14 | same | `p` decodes to a `y` for which `x^2 = (y^2-1)/(dy^2+1)` is not a square → `ge25519_frombytes(&P, p) != 0` (e.g. `p = 02 00 … 00`) | `-1`. Early reject, `q` untouched. | verified |
| 7.15 | same | `p` = `01 00 … 00` (Edwards identity) → `ge25519_has_small_order(&P) != 0` | `-1`. Early reject, `q` untouched. | verified |
| 7.16 | same | `p` is any of the other 7 torsion encodings (`00 00 … 00` order 4; `ec ff … 7f` order 2; the two order-4 and four order-8 points) → `ge25519_has_small_order` | `-1`. Note `ge25519_has_small_order` tests `X==0 ∨ Y==0 ∨ Z==0 ∨ Y·√-1 == X ∨ Y·√-1 == -X` (`ed25519_ref10.c:1173-1189`). Early reject, `q` untouched. | verified |
| 7.17 | same | `p` is a valid, canonical, non-small-order point that is **not** in the prime-order subgroup (order `2L`, `4L` or `8L`; obtain by adding a torsion point to a legitimate pk) → `ge25519_is_on_main_subgroup(&P) == 0` | `-1`. Early reject, `q` untouched. | verified |
| 7.18 | `crypto_scalarmult_ed25519` (clamped, `:53`) | `n` = 32 zero bytes, valid `p` | `-1` via `sodium_is_zero(n, 32)`. Note clamping makes the *effective* scalar `2^254`, so `_is_inf(q)` is false and it is the `sodium_is_zero(n,…)` disjunct that fires. **Late reject: `q` already holds the (valid, non-identity) result bytes.** | verified |
| 7.19 | `crypto_scalarmult_ed25519_noclamp` (`:53`) | `n` = 32 zero bytes, valid `p` | `-1`. Both disjuncts fire: `q` is the identity so `_crypto_scalarmult_ed25519_is_inf(q) != 0`, and `sodium_is_zero(n,32)`. **Late reject: `q` holds `01 00 … 00`.** | verified |
| 7.20 | `crypto_scalarmult_ed25519_noclamp` | `n` = `L` little-endian (`ed d3 f5 5c 1a 63 12 58 d6 9c f7 a2 de f9 de 14 00 … 00 10`), valid `p` | `-1` via `_is_inf(q)` only (`sodium_is_zero(n)` is false). **Late reject: `q` = `01 00 … 00`.** Same for any `n ≡ 0 (mod L)` whose bit 255 is clear. | verified |
| 7.21 | `crypto_scalarmult_ed25519_noclamp` | `n = L + 2^255` (bit 255 set) | `-1`. `t[31] &= 127` (`:49`) clears bit 255 unconditionally on **both** the clamped and noclamp paths, so this reduces to 7.20. Bit 255 of `n` is never honoured. | verified |
| 7.22 | `crypto_scalarmult_ed25519_base` (`_crypto_scalarmult_ed25519_base`, `:91`) | `n` = 32 zero bytes | `-1` via `sodium_is_zero(n, 32)` (clamping means `_is_inf` is false). **Late reject: `q` holds the valid point for scalar `2^254`.** | verified |
| 7.23 | `crypto_scalarmult_ed25519_base_noclamp` (`:91`) | `n` = 32 zero bytes | `-1`; both `_is_inf(q)` and `sodium_is_zero(n,32)` fire. `q` = `01 00 … 00`. | verified |
| 7.24 | `crypto_scalarmult_ed25519_base_noclamp` | `n = L` (or any nonzero `n ≡ 0 mod L`, bit 255 cleared) | `-1` via `_is_inf(q)` only. `q` = `01 00 … 00`. | verified |
| 7.25 | all four `crypto_scalarmult_ed25519*` | **shared side-effect contract**, not a trigger | On the *early* rejects (7.13–7.17) `q` is untouched; on the *late* rejects (7.18–7.24) `q` has been fully written with `ge25519_p3_tobytes` output, and additionally `q` was used as scratch for the clamped scalar `t` before that. `_is_inf` tests `s[0]^0x01 | s[1..30] | (s[31] & 0x7f) == 0`, i.e. it ignores bit 255 of `q`. | verified |
| 7.26 | `crypto_scalarmult_ristretto255` (`scalarmult_ristretto255_ref10.c:18`) | `p` is not a canonical ristretto255 encoding (`ristretto255_is_canonical(s) == 0`): value `>= 2^255-19`, or bit 255 set, or an odd/negative representative | `-1` from `ristretto255_frombytes`. Early reject: `q` untouched. | verified |
| 7.27 | same | `p` is canonical bytes but `v·u2²` is a non-square (`notsquare == 1`) — the encoding is not on the ristretto255 image | `-1` (`ed25519_ref10.c:2864-2866`, `-((1-notsquare) | isnegative(T) | iszero(Y))`). Early reject, `q` untouched. | verified |
| 7.28 | same | `p` decodes with `fe25519_isnegative(h->T)` or `fe25519_iszero(h->Y)` | `-1`. Early reject, `q` untouched. | verified |
| 7.29 | `crypto_scalarmult_ristretto255` (`:27`) | `p` = 32 zero bytes (the ristretto255 identity — this **is** a valid, accepted encoding), any `n` | `-1` via `sodium_is_zero(q, 32)`: `n·identity = identity` which ristretto-encodes as 32 zero bytes. **Late reject: `q` = 32 zero bytes.** | verified |
| 7.30 | `crypto_scalarmult_ristretto255` | `n` = 32 zero bytes, valid non-identity `p` | `-1` via `sodium_is_zero(q, 32)`. There is **no clamping** on the ristretto path (only `t[31] &= 127`), so `n = 0` really is scalar 0. **Late reject: `q` = zeros.** | verified |
| 7.31 | `crypto_scalarmult_ristretto255` | `n = L` little-endian (or any nonzero `n ≡ 0 mod L` with bit 255 clear), valid `p` | `-1` via `sodium_is_zero(q, 32)`. **Late reject: `q` = zeros.** | verified |
| 7.32 | `crypto_scalarmult_ristretto255_base` (`:47`) | `n` = 32 zero bytes | `-1` via `sodium_is_zero(q, 32)`. **Late reject: `q` = zeros.** | verified |
| 7.33 | `crypto_scalarmult_ristretto255_base` | `n = L` (bit 255 clear) | `-1` via `sodium_is_zero(q, 32)`. **Late reject: `q` = zeros.** Also true for `n = L + 2^255` since `t[31] &= 127`. | verified |
| 7.34 | `crypto_sign_ed25519_open` / `crypto_sign_open` (`open.c:81`) | `smlen < 64` (any of `0 … 63`) | `-1`; `*mlen_p = 0` if `mlen_p != NULL`. `m` is **not** touched (the `memset(m, 0, mlen)` is on the other `goto badsig` path). | verified |
| 7.35 | `crypto_sign_ed25519_open` (`open.c:81`) | `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` = `2^64 - 65` | `-1`, `*mlen_p = 0`. **Unreachable on LP64**: `smlen` is `unsigned long long`, so `smlen ≤ 2^64-1` and `smlen-64 ≤ 2^64-65`; the predicate is never true. Dead branch, preserve semantically. | unreachable-from-public-API |
| 7.36 | `crypto_sign_ed25519_open` (`open.c:85-90`) | `smlen >= 64` but any bit of the message part `sm[64…]` is flipped | `-1`; **`memset(m, 0, mlen)` is executed first** (so the caller's `m` is zeroed over `smlen-64` bytes) and then `*mlen_p = 0`. Both side effects are load-bearing. | verified |
| 7.37 | same | any bit of `sm[0..63]` (the signature) flipped | `-1`, `m` zeroed over `mlen`, `*mlen_p = 0`. | verified |
| 7.38 | same | correct `sm` but `pk` from a different keypair | `-1`, `m` zeroed, `*mlen_p = 0`. | verified |
| 7.39 | `crypto_sign_ed25519_verify_detached` / `_crypto_sign_ed25519_verify_detached` (`open.c:35-38`) | `(sig[63] & 240) != 0` **and** `sc25519_is_canonical(sig + 32) == 0`, i.e. `S >= L` (e.g. take a valid sig and add `L` to `S`) | `-1`. Note this is a **conjunction**: the cheap `sig[63] & 240` test gates the constant-time canonicality test. | verified |
| 7.40 | same | `S == L` exactly (`sig[32..63]` = `ed d3 f5 5c 1a 63 12 58 d6 9c f7 a2 de f9 de 14 00 … 00 10`) | `-1`. `sig[63] == 0x10` so `0x10 & 240 != 0`, and for `s == L` every `((s[i]-L[i])>>8) & n` term is `0`, leaving `c == 0`, so `sc25519_is_canonical` returns `(c != 0) == 0` → the `== 0` guard fires → rejected. `S == L-1` is accepted. | verified |
| 7.41 | same | `S` with `(sig[63] & 240) == 0`, i.e. `S < 2^252` | The canonicality test is **skipped** (short-circuit). Since `L > 2^252`, every such `S` is automatically canonical, so this is not a hole — but a port must reproduce the short-circuit exactly, because it is the difference between the strict and the `ED25519_COMPAT` variants. | verified |
| 7.42 | same (`open.c:39-41`) | `pk` non-canonical: `pk[31] & 0x7f == 0x7f`, `pk[1..30] == 0xff`, `pk[0] >= 0xed` | `-1` (`ge25519_is_canonical(pk) == 0`). With `ED25519_COMPAT` defined this check would not exist — but the CMake build defines no `HAVE_*`/`ED25519_COMPAT`, so the strict branch (`open.c:34-42`) is the live one. | verified |
| 7.43 | same (`open.c:43`) | `pk` does not decode: `ge25519_frombytes_negate_vartime(&A, pk) != 0` (non-square `x²`) | `-1`. | verified |
| 7.44 | same (`open.c:44`) | `pk` is one of the 8 small-order encodings → `ge25519_has_small_order(&A) != 0` | `-1`. **Note: no `is_on_main_subgroup` check here** — a non-small-order pk of order `8L` is accepted by verification (unlike `pk_to_curve25519`, 7.55). | verified |
| 7.45 | same (`open.c:47`) | `R` = `sig[0..31]` does not decode to a curve point → `ge25519_frombytes(&expected_r, sig) != 0` | `-1`. **Note: `R` is NOT canonicality-checked** — only `pk` is. A non-canonical-but-decodable `R` passes this test. | verified |
| 7.46 | same (`open.c:48`) | `R` is a small-order point → `ge25519_has_small_order(&expected_r) != 0` (e.g. `R = 01 00 … 00`) | `-1`. | verified |
| 7.47 | same (`open.c:62`) | the cofactored equation fails: `check = R - (h·A + S·B)` is **not** a small-order point | `-1`, computed as `ge25519_has_small_order(&check) - 1`. Success (`0`) is returned when `check` **is** small-order, i.e. verification accepts `R ≡ S·B + h·A (mod torsion)`. `h = SHA-512(R ‖ pk ‖ m) mod L` after `sc25519_reduce`. | verified |
| 7.48 | same | correct `sig`/`pk` but a different `mlen` passed than was signed (e.g. sign 32 bytes, verify with `mlen = 31` or `33`) | `-1` via 7.47 (different `h`). Note `mlen == 0` with a genuine empty-message signature succeeds. | verified |
| 7.49 | `crypto_sign_ed25519ph_final_verify` / `crypto_sign_final_verify` (`sign_ed25519.c:88`, `prehashed = 1`) | the signature was produced by `crypto_sign_detached` / `crypto_sign` (`prehashed = 0`) | `-1` via 7.47. `_crypto_sign_ed25519_ref10_hinit` prepends the 34-byte `DOM2PREFIX` (`"SigEd25519 no Ed25519 collisions" ‖ 0x01 0x00`) only when `prehashed`, so the two domains never cross-verify. | verified |
| 7.50 | `crypto_sign_ed25519_verify_detached` (`prehashed = 0`) | the signature was produced by `crypto_sign_ed25519ph_final_create` | `-1` via 7.47 (mirror of 7.49). | verified |
| 7.51 | `crypto_sign_final_verify` after `crypto_sign_init` with **zero** `crypto_sign_update` calls | verifying a signature that was created over non-empty prehash input (or vice versa) | `-1`. The prehash is `SHA-512("")` for the zero-update case; a signature over any other prehash fails 7.47. Zero updates is otherwise perfectly legal and round-trips against a matching zero-update `final_create`. | verified |
| 7.52 | `crypto_sign_ed25519` (`sign.c:113-121`) | internal failure: `crypto_sign_ed25519_detached(...) != 0` **or** `siglen != 64` | `-1`; `*smlen_p = 0` if non-NULL, and `memset(sm, 0, mlen + 64)`. **Unreachable**: `_crypto_sign_ed25519_detached` has no failure path (it always sets `*siglen_p = 64` and `return 0`). Marked `LCOV_EXCL_START/STOP`. Must still be structurally present. | unreachable-from-public-API |
| 7.53 | `crypto_sign_ed25519_pk_to_curve25519` (`keypair.c:53`) | `ed25519_pk` does not decode: `ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0` | `-1`; `curve25519_pk` untouched. | verified |
| 7.54 | same (`keypair.c:54`) | `ed25519_pk` has small order (any of the 8 torsion encodings, including `01 00 … 00` and `00 00 … 00`) | `-1`; `curve25519_pk` untouched. This also covers the `y == 1` case which would otherwise make `1 - y == 0` and `fe25519_invert` divide by zero. | verified |
| 7.55 | same (`keypair.c:55`) | `ed25519_pk` is a valid non-small-order point **not** on the main subgroup (`ge25519_is_on_main_subgroup(&A) == 0`) | `-1`. This is **stricter than signature verification** (7.44), which omits the subgroup test. | verified |
| 7.56 | same | `ed25519_pk` non-canonical but decodable (e.g. `y` in `[2^255-19, 2^255)` reduced) | **Accepted, returns `0`** — `pk_to_curve25519` has **no** `ge25519_is_canonical` check, unlike `verify_detached` (7.42). A distinct asymmetry to preserve. | unreachable-from-public-API |
| 7.57 | `crypto_sign_ed25519_sk_to_curve25519` (`keypair.c:71`), `crypto_sign_ed25519_sk_to_seed` (`sign_ed25519.c:45`), `crypto_sign_ed25519_sk_to_pk` (`:53`) | any input, including a structurally invalid 64-byte `sk` | Always `0`. Pure `memmove`/`memcpy` + SHA-512 + clamp; **no validation at all** (they do not check that `sk[32..63]` matches `sk[0..31]`'s derived pk). | verified |
| 7.58 | `crypto_sign_ed25519_seed_keypair` / `crypto_sign_ed25519_keypair` (`keypair.c:13,33`), `crypto_sign_ed25519ph_init` / `_update` / `_final_create` (`sign_ed25519.c:61,68,75`), `crypto_sign_ed25519_detached` (`sign.c:97`) | — | **Cannot fail; always `0`.** `_update` returns `crypto_hash_sha512_update`'s value which is structurally `0`. `_keypair` returns `seed_keypair`'s value which is `0`. | verified |
| 7.59 | `crypto_box_beforenm` → `crypto_box_curve25519xsalsa20poly1305_beforenm` (`box_curve25519xsalsa20poly1305.c:42`) | `pk` = any of the 7 blocklisted small-order encodings (notably 32 zero bytes) — `crypto_scalarmult_curve25519(s, sk, pk) != 0` | `-1`; `k` **untouched**. | verified |
| 7.60 | `crypto_box_easy` (`crypto_box_easy.c:52`) → `crypto_box_detached` → `crypto_box_beforenm` | `pk` small-order as in 7.59 | `-1` (from `crypto_box_detached`'s `:30-32`). `c` untouched. | verified |
| 7.61 | `crypto_box_detached` (`crypto_box_easy.c:21`) | `pk` small-order | `-1`; `c` and `mac` untouched. | verified |
| 7.62 | `crypto_box` / `crypto_box_curve25519xsalsa20poly1305` (`box_curve25519xsalsa20poly1305.c:81`) | `pk` small-order | `-1`; `c` untouched. | verified |
| 7.63 | `crypto_box_open` / `crypto_box_open_easy` / `crypto_box_open_detached` / `crypto_box_curve25519xsalsa20poly1305_open` | `pk` small-order (`beforenm` fails) | `-1`. For `_open_easy`/`_open_detached` this happens **after** the `clen` check but **before** any MAC work. | verified |
| 7.64 | `crypto_box_open_easy` (`crypto_box_easy.c:109-111`) | `clen < crypto_box_MACBYTES` = `16` (i.e. `clen ∈ 0…15`) | `-1`, checked **before** `crypto_box_beforenm`, so no scalarmult is performed. `m` untouched. | verified |
| 7.65 | `crypto_box_open_easy_afternm` (`crypto_box_easy.c:96-98`) | `clen < 16` | `-1`. `m` untouched. `clen == 16` is legal (empty message, MAC only). | verified |
| 7.66 | `crypto_box_open_easy` / `_open_easy_afternm` | `clen >= 16` but the MAC `c[0..15]` or the ciphertext `c[16…]` has been tampered with, or the nonce/key is wrong | `-1` propagated from `crypto_secretbox_open_detached`. Per secretbox semantics `m` is zeroed over `clen-16` bytes on MAC failure. | verified |
| 7.67 | `crypto_box_open_detached` (`crypto_box_easy.c:74`) | tampered `mac`, `c`, wrong `n`, or mismatched `pk`/`sk` pair | `-1` from `crypto_secretbox_open_detached`; `m` zeroed over `clen`. | verified |
| 7.68 | `crypto_box_open_detached_afternm` (`crypto_box_easy.c:64`) | tampered `mac`/`c`, wrong `n`/`k` | `-1`; `m` zeroed over `clen`. No length guard here at all — `clen == 0` is valid. | verified |
| 7.69 | `crypto_box_open` / `crypto_box_open_afternm` → `crypto_secretbox_xsalsa20poly1305_open` (NaCl padded API) | `clen < crypto_box_BOXZEROBYTES` = `16` | `-1` (the guard lives in `crypto_secretbox_xsalsa20poly1305_open`, area 6). NaCl API contract: `c[0..15]` must be zero padding and `m[0..31]` is zeroed output. | verified |
| 7.70 | `crypto_box_open` / `crypto_box_open_afternm` | `clen >= 16` but the Poly1305 tag at `c[16..31]` does not match | `-1`. | verified |
| 7.71 | `crypto_box_open*` (all variants) | correct ciphertext but the wrong 24-byte nonce `n` | `-1` (MAC failure). | verified |
| 7.72 | `crypto_box_seal_open` (`crypto_box_seal.c:55-57`) | `clen < crypto_box_SEALBYTES` = `48` (i.e. `clen ∈ 0…47`) | `-1`, before any hashing or scalarmult. `m` untouched. `clen == 48` is legal (empty sealed message). | verified |
| 7.73 | `crypto_box_seal_open` | `clen >= 48` but `pk` is not the recipient public key matching `sk` | `-1`. Two independent reasons: the derived nonce `BLAKE2b-24(c[0..31] ‖ pk)` differs, and the DH shared secret differs. | verified |
| 7.74 | `crypto_box_seal_open` | the embedded ephemeral pk `c[0..31]` is tampered with | `-1` (nonce and shared secret both change). | verified |
| 7.75 | `crypto_box_seal_open` | `c[32…]` (MAC or ciphertext body) tampered with | `-1` from `crypto_box_open_easy`. | verified |
| 7.76 | `crypto_box_seal_open` | `c[0..31]` is a small-order encoding (e.g. 32 zero bytes) → `crypto_box_open_easy`'s `beforenm` fails | `-1` (7.59 path, since the ephemeral pk is passed as `pk` to `crypto_box_open_easy`). | verified |
| 7.77 | `crypto_box_seal` (`crypto_box_seal.c:25`) | `pk` is a small-order encoding | `-1` propagated from `crypto_box_easy` → `crypto_box_detached` → `beforenm`. Side effect: **`memcpy(c, epk, 32)` runs anyway** (`:42`), so `c[0..31]` is written with the ephemeral pk even on failure. | verified |
| 7.78 | `crypto_box_seal` (`crypto_box_seal.c:36-38`) | `crypto_box_keypair(epk, esk) != 0` | `-1`. **Unreachable**: `crypto_box_curve25519xsalsa20poly1305_keypair` returns `crypto_scalarmult_curve25519_base` which is always `0` (7.11). Marked `LCOV_EXCL_LINE`. | unreachable-from-public-API |
| 7.79 | `crypto_box_easy` (`:56`), `crypto_box_easy_afternm` (`:44`), `crypto_box_seal` (`:33`) | `mlen > crypto_box_MESSAGEBYTES_MAX` = `2^64 - 17` (i.e. `mlen >= 2^64 - 16`) | `sodium_misuse()` → misuse handler then `abort()`. **Reachable in principle only with an absurd `mlen`** (`mlen ∈ {2^64-16 … 2^64-1}`); marked `LCOV_EXCL_LINE`. Note this is an abort, not `-1`. | verified |
| 7.80 | `crypto_box_seed_keypair` / `crypto_box_keypair` (`crypto_box.c:65,72` → `box_curve25519xsalsa20poly1305.c:12,26`), `crypto_box_afternm`, `crypto_box_detached_afternm`, `crypto_box_curve25519xsalsa20poly1305_afternm` | — | **Cannot fail; always `0`.** `seed_keypair`/`keypair` return `crypto_scalarmult_curve25519_base` (7.11). Encryption `afternm` variants return `crypto_secretbox_*` which is `0`. | verified |
| 7.81 | `crypto_box_curve25519xchacha20poly1305_beforenm` (`box_curve25519xchacha20poly1305.c:48`) | `pk` = any of the 7 blocklisted small-order encodings | `-1`; `k` untouched. Identical guard to 7.59, differing only in the `crypto_core_hchacha20` post-processing. | verified |
| 7.82 | `crypto_box_curve25519xchacha20poly1305_easy` (`:101`) / `_detached` (`:66`) | `pk` small-order | `-1`. | verified |
| 7.83 | `crypto_box_curve25519xchacha20poly1305_open_easy` (`:159-161`) | `clen < crypto_box_curve25519xchacha20poly1305_MACBYTES` = `16` | `-1`, before `beforenm`. | verified |
| 7.84 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` (`:146-148`) | `clen < 16` | `-1`. | verified |
| 7.85 | `crypto_box_curve25519xchacha20poly1305_open_detached` (`:123`) / `_open_detached_afternm` (`:114`) | tampered `mac`/`c`, wrong `n`/`k`, or (for `_open_detached`) small-order `pk` | `-1` from `crypto_secretbox_xchacha20poly1305_open_detached` (or `beforenm`). No `clen` guard on either — `clen == 0` is legal. | verified |
| 7.86 | `crypto_box_curve25519xchacha20poly1305_open_easy` / `_open_detached` | `pk` small-order → `beforenm` fails | `-1` (after the `clen` guard for `_open_easy`). | verified |
| 7.87 | `crypto_box_curve25519xchacha20poly1305_seal_open` (`box_seal_curve25519xchacha20poly1305.c:63-65`) | `clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES` = `48` | `-1`, `m` untouched. | verified |
| 7.88 | `crypto_box_curve25519xchacha20poly1305_seal_open` | wrong recipient `pk`, tampered `c[0..31]`, or tampered `c[32…]` | `-1`. Same three-way trigger set as 7.73–7.75. | verified |
| 7.89 | `crypto_box_curve25519xchacha20poly1305_seal` (`:39-41,42-44`) | `mlen > MESSAGEBYTES_MAX` → `sodium_misuse()`; or `..._keypair(epk, esk) != 0` → `-1` (unreachable, `LCOV_EXCL_LINE`) | `abort()` / `-1`. As in 7.77, `memcpy(c, epk, 32)` (`:49`) runs even when `..._easy` returns `-1` for a small-order `pk`. | verified / unreachable-from-public-API |
| 7.90 | `crypto_box_curve25519xchacha20poly1305_easy` (`:105`) / `_easy_afternm` (`:93`) | `mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX` = `2^64 - 17` | `sodium_misuse()` → `abort()`. `LCOV_EXCL_LINE`. | verified |
| 7.91 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` / `_keypair` / `_detached_afternm` | — | **Cannot fail; always `0`.** Note the xchacha subdirectory provides **no** NaCl-style padded `crypto_box_curve25519xchacha20poly1305()`/`_open()` and **no** `_afternm`/`_open_afternm`; only easy/detached/seal. | verified |
| 7.92 | `crypto_kx_client_session_keys` (`crypto_kx.c:54-56`) | `server_pk` is one of the 7 blocklisted small-order encodings (notably 32 zero bytes) → `crypto_scalarmult(q, client_sk, server_pk) != 0` | `-1`. **`rx` and `tx` are left untouched** (the write loop is after the guard) and `q` is a local. | verified |
| 7.93 | `crypto_kx_server_session_keys` (`crypto_kx.c:95-97`) | `client_pk` small-order → `crypto_scalarmult(q, server_sk, client_pk) != 0` | `-1`; `rx`/`tx` untouched. | verified |
| 7.94 | `crypto_kx_client_session_keys` / `_server_session_keys` (`:45-53`, `:86-94`) | **both** `rx == NULL` and `tx == NULL` | `sodium_misuse()` → misuse handler then `abort()`. Reached because `rx = tx` (still NULL) then `tx = rx` (still NULL) then `if (rx == NULL)`. `LCOV_EXCL_LINE` but genuinely reachable from user code. | verified |
| 7.95 | `crypto_kx_client_session_keys` | `rx == NULL`, `tx != NULL` (valid usage: "I only want the tx key") | `0`. `rx` is retargeted to `tx`, so the loop writes `rx[i] = keys[i]` then `tx[i] = keys[i+32]` **into the same buffer, byte by byte**. Net result: the caller's `tx` buffer ends up holding `keys[32..63]` (the real tx key). Correct, but the intermediate write of `keys[0..31]` is observable through the aliasing and must be reproduced (or at least the final state must match). | verified |
| 7.96 | `crypto_kx_client_session_keys` | `tx == NULL`, `rx != NULL` | `0`. `tx` is retargeted to `rx`; same byte-interleaved aliasing → the caller's `rx` buffer ends up holding `keys[32..63]`, i.e. the **tx** key, **not** the rx key. This is a genuine footgun in the C and must be replicated exactly. | verified |
| 7.97 | `crypto_kx_server_session_keys` | `rx == NULL` or `tx == NULL` (one of them) | `0`. Server loop order is reversed: `tx[i] = keys[i]` then `rx[i] = keys[i+32]`, so the surviving buffer holds `keys[32..63]` = the server's **rx** key. | verified |
| 7.98 | `crypto_kx_seed_keypair` (`crypto_kx.c:13`) / `crypto_kx_keypair` (`:23`) | any 32-byte `seed`, including all zeros | **Cannot fail; always `0`.** Returns `crypto_scalarmult_base(pk, sk)` which is unconditionally `0` (7.12). `sk = BLAKE2b-32(seed)` for `_seed_keypair`. | verified |
| 7.99 | `crypto_kdf_blake2b_derive_from_key` (`kdf_blake2b.c:43-47`) | `subkey_len < crypto_kdf_blake2b_BYTES_MIN` = `16`, i.e. `subkey_len ∈ {0, 1, …, 15}` | `errno = EINVAL; return -1`. `subkey` untouched. **Note the bound check happens *after* `ctx_padded` and `salt` are built** (`:39-42`), but those are locals, so there is no observable side effect. | verified |
| 7.100 | same | `subkey_len > crypto_kdf_blake2b_BYTES_MAX` = `64`, i.e. `subkey_len >= 65` (up to `SIZE_MAX`) | `errno = EINVAL; return -1`. `subkey` untouched. | verified |
| 7.101 | `crypto_kdf_derive_from_key` (`crypto_kdf.c:36`) | `subkey_len ∉ [16, 64]` | `errno = EINVAL; return -1`. Thin alias for 7.99/7.100. | verified |
| 7.102 | `crypto_kdf_blake2b_derive_from_key` / `crypto_kdf_derive_from_key` | `subkey_len ∈ [16, 64]`, any `subkey_id`, any 8-byte `ctx`, any 32-byte `key` | `0`. The tail return is `crypto_generichash_blake2b_salt_personal(...)` which itself validates `outlen ∈ [16,64]` and `keylen ≤ 64` — both already satisfied, so it cannot fail from here. `ctx` is zero-padded to 16 bytes as the BLAKE2b *personal*; `subkey_id` is `STORE64_LE`'d into the low 8 bytes of the 16-byte *salt* with the upper 8 zeroed. | verified |
| 7.103 | `crypto_kdf_hkdf_sha256_expand` (`kdf_hkdf_sha256.c:65-68`) | `out_len > crypto_kdf_hkdf_sha256_BYTES_MAX` = `0xff * 32` = `8160`, i.e. `out_len >= 8161` | `errno = EINVAL; return -1`. `out` untouched. `out_len == 8160` is accepted (counter reaches `0xff`). | verified |
| 7.104 | `crypto_kdf_hkdf_sha512_expand` (`kdf_hkdf_sha512.c:65-68`) | `out_len > crypto_kdf_hkdf_sha512_BYTES_MAX` = `0xff * 64` = `16320`, i.e. `out_len >= 16321` | `errno = EINVAL; return -1`. `out` untouched. `out_len == 16320` accepted. | verified |
| 7.105 | `crypto_kdf_hkdf_sha256_expand` / `_sha512_expand` | `out_len == 0` (`BYTES_MIN` is `0`, so this is **legal**) | `0`; nothing is written (the main loop condition `0 + 32 <= 0` is false and `left = 0 & 31 == 0`). Note `crypto_kdf_hkdf_*_BYTES_MIN` is never actually compared against anywhere — there is **no** lower-bound check in hkdf, unlike blake2b. | verified |
| 7.106 | `crypto_kdf_hkdf_sha256_expand` / `_sha512_expand` — `left` masking | `out_len` not a multiple of `32` / `64` | `0`. `left = out_len & (BYTES - 1)` relies on `BYTES` being a power of two (`32`, `64`); the tail block is computed into `tmp` and only `left` bytes copied. A port must not write past `out_len`. | verified |
| 7.107 | `crypto_kdf_hkdf_sha256_extract` / `_extract_init` / `_extract_update` / `_extract_final` (and the sha512 twins) | any `salt_len` (including `0` and `> block size`), any `ikm_len` including `0`, any number of `_extract_update` calls including zero | **Cannot fail; always `0`.** `crypto_auth_hmacsha{256,512}_init` returns `0` unconditionally; `_final` returns `void` and the wrapper hard-codes `return 0`. `_extract_final` additionally `sodium_memzero(state, sizeof *state)` — the state is destroyed and must not be reused. | verified |
| 7.108 | `crypto_kdf_keygen` (`crypto_kdf.c:46`), `crypto_kdf_hkdf_sha256_keygen`, `crypto_kdf_hkdf_sha512_keygen` | — | **Cannot fail; `void` return.** Pure `randombytes_buf` of 32 / 32 / 64 bytes. | verified |
| 7.109 | `crypto_kdf_hkdf_*_statebytes` / `_keybytes` / `_bytes_min` / `_bytes_max`, `crypto_kdf_bytes_min/max`, `_contextbytes`, `_keybytes`, `_primitive` | — | **Cannot fail.** Constant returns as in the constants table. | verified |
| 7.110 | `crypto_kem_mlkem768_enc_deterministic` → `mlkem768_ref_enc_deterministic` (`kem_mlkem768_ref.c:744-747`) | `pk` whose first `1152` bytes unpack to any 12-bit coefficient `>= 3329` — e.g. set `pk[0] = 0x01, pk[1] = 0x0d` giving coefficient `0xd01 = 3329` | `-1` (`polyvec_is_canonical(&pkpv) == 0`). `ct` and `ss` untouched. This is the **only** validity check in the whole kem area. Note the trailing 32-byte `publicseed` (`pk[1152..1183]`) is **not** validated at all. | verified |
| 7.111 | `crypto_kem_mlkem768_enc` → `mlkem768_ref_enc` (`:764`) | same non-canonical `pk` as 7.110 | `-1` propagated. `seed` is zeroed before returning. | verified |
| 7.112 | `crypto_kem_mlkem768_enc*` | `pk` = 1184 zero bytes | **`0` (accepted!)** — all coefficients are `0 < 3329`, so `polyvec_is_canonical` passes. Encapsulation against the all-zero pk succeeds and produces a well-formed `ct`/`ss`. Not an error; a boundary that must not accidentally become a rejection. | verified |
| 7.113 | `crypto_kem_mlkem768_dec` → `mlkem768_ref_dec` (`:777-816`) | `ct` with any bit flipped (or wholly random, or all-zero) | **`0`, never `-1`.** Implicit rejection: `fail = sodium_memcmp(ct, cmp, 1088)` (`0` on match, `-1` on mismatch), `fail_mask = ((unsigned) fail) >> 31`, then `cmov(kr, k_bar, 32, fail_mask)` swaps in `k_bar = SHAKE256(z ‖ ct)[0..31]`. `ss` is a deterministic pseudorandom value that differs from the encapsulated secret. **There is no ciphertext-length or ciphertext-validity check.** | verified |
| 7.114 | `crypto_kem_mlkem768_dec` | `sk` from a different keypair than the one `ct` was produced against | `0`, `ss` mismatched (implicit rejection path). | verified |
| 7.115 | `crypto_kem_mlkem768_dec` | `sk` structurally corrupted (e.g. the embedded `hpk` at `sk[2336..2367]` altered, or the `z` at `sk[2368..2399]` altered) | `0`, `ss` mismatched. No consistency check between `sk`'s embedded `pk`, `H(pk)` and the polyvec. | verified |
| 7.116 | `crypto_kem_mlkem768_keypair` / `_seed_keypair` (`kem_mlkem768.c:35,41` → `kem_mlkem768_ref.c:706,724`) | any 64-byte `seed`, including all zeros | **Cannot fail; always `0`.** `seed[0..31]` → `indseed = seed[0..31] ‖ 0x03` for `indcpa_keypair`; `seed[32..63]` → the implicit-rejection value `z` at `sk[2368..2399]`. | verified |
| 7.117 | `crypto_kem_xwing_enc_deterministic` (`kem_xwing.c:134-136`) | `pk[0..1183]` (the ML-KEM part) is non-canonical per 7.110 | `-1` from `crypto_kem_mlkem768_enc_deterministic`. `ct`/`ss` untouched. Marked `LCOV_EXCL_LINE`. | verified |
| 7.118 | `crypto_kem_xwing_enc_deterministic` (`kem_xwing.c:140-143`) | `pk[1184..1215]` (the X25519 part) is a blocklisted small-order encoding — notably 32 zero bytes | `-1` from `crypto_scalarmult_curve25519(ss_x25519, sk_e_x25519, pk_x25519)`; `ss_mlkem` is zeroed first. **Side effect: `ct` has *not* yet been written (the `memcpy`s are at `:145-146`), but `crypto_scalarmult_curve25519_base(ct_x25519, sk_e_x25519)` at `:138` already ran into a local.** Marked `LCOV_EXCL_LINE`, but reachable with an attacker-supplied pk. | verified |
| 7.119 | `crypto_kem_xwing_enc` (`kem_xwing.c:157`) | either trigger of 7.117 / 7.118 | `-1` propagated; the 64-byte random `seed` is zeroed on both paths. `LCOV_EXCL_LINE`. | verified |
| 7.120 | `crypto_kem_xwing_dec` (`kem_xwing.c:188-192`) | `crypto_kem_mlkem768_dec(...) != 0` | `-1` with `sk_mlkem` / `sk_x25519` zeroed. **Dead branch** — `mlkem768_ref_dec` always returns `0` (7.113). `LCOV_EXCL_START/STOP`. | unreachable-from-public-API |
| 7.121 | `crypto_kem_xwing_dec` (`kem_xwing.c:194-199`) | `ct[1088..1119]` (the X25519 ciphertext half) is a blocklisted small-order encoding — e.g. an attacker sends `ct` with 32 zero bytes there | `-1`; `ss_mlkem`, `sk_mlkem`, `sk_x25519` zeroed, `ss` untouched. This is the **only reachable failure of xwing decapsulation** and is inside the `LCOV_EXCL` region, so it is untested upstream. | verified |
| 7.122 | `crypto_kem_xwing_dec` | `ct[0..1087]` (the ML-KEM half) tampered with, while `ct[1088..1119]` remains a valid non-small-order X25519 point | **`0`**, with `ss` differing from the encapsulated secret (implicit rejection inside ML-KEM propagated through `combiner`). No error is signalled. | verified |
| 7.123 | `crypto_kem_xwing_dec` | `sk` (32-byte seed) from a different keypair | `0`, `ss` mismatched (both the ML-KEM implicit-rejection path and a different `ss_x25519`). | verified |
| 7.124 | `crypto_kem_xwing_seed_keypair` (`kem_xwing.c:86`) / `_keypair` (`:107`) | any 32-byte `seed`, including all zeros | **Cannot fail; always `0`.** `sk` is *just the 32-byte seed* (`crypto_kem_xwing_SECRETKEYBYTES == 32`); everything else is re-derived by `expand_decaps_key` on each `dec`. `pk = pk_mlkem(1184) ‖ pk_x25519(32)`. Return value of `crypto_kem_mlkem768_seed_keypair` inside `expand_decaps_key` is **discarded** (`:30`) — harmless since it is always `0`. | verified |
| 7.125 | `crypto_kem_seed_keypair` / `_keypair` / `_enc` / `_dec` (`crypto_kem.c:40,47,53,59`) | any trigger of 7.117–7.123 | Identical results — these are thin aliases for the xwing functions. `crypto_kem_primitive()` returns `"xwing"`. | verified |
| 7.126 | seed / key length contract mismatches (documentation-level, not runtime-checked) | `crypto_kem_xwing_SEEDBYTES == 32` (used by `_seed_keypair`) but `crypto_kem_xwing_enc_deterministic` takes `seed[64]` (`seed[0..31]` → ML-KEM `m`, `seed[32..63]` → the ephemeral X25519 scalar). Symmetrically `crypto_kem_mlkem768_SEEDBYTES == 64` for `_seed_keypair` but `crypto_kem_mlkem768_enc_deterministic` takes `seed[32]`. | **No runtime check.** Passing a 32-byte buffer to `crypto_kem_xwing_enc_deterministic` (or a 64-byte one where 32 is expected) is an out-of-bounds read / silent misuse, not an error return. Rust port should encode these as distinct fixed-size array types. | undefined-behaviour-not-tested |
| 7.127 | `crypto_scalarmult_primitive`, `_bytes`, `_scalarbytes`; `crypto_scalarmult_curve25519_bytes/_scalarbytes`; `crypto_scalarmult_ed25519_bytes/_scalarbytes`; `crypto_scalarmult_ristretto255_bytes/_scalarbytes`; `crypto_sign_statebytes/_bytes/_seedbytes/_publickeybytes/_secretkeybytes/_messagebytes_max/_primitive`; `crypto_sign_ed25519ph_statebytes` and the `crypto_sign_ed25519_*bytes` family; the whole `crypto_box_*bytes` / `crypto_box_*_*bytes` family; `crypto_kx_*bytes` / `_primitive`; `crypto_kem_*bytes` / `_primitive` | — | **Cannot fail.** Constant returns; `*_primitive()` returns a pointer to a string literal in static storage (never NULL). `crypto_sign_statebytes()` = `sizeof(crypto_sign_state)` = `sizeof(crypto_sign_ed25519ph_state)` = `sizeof(crypto_hash_sha512_state)` = `208` on LP64. | verified |
| 7.128 | `_crypto_scalarmult_curve25519_pick_best_implementation` (`scalarmult_curve25519.c:50`) | — | **Cannot fail; always `0`.** With no `HAVE_AVX_ASM` defined, the sandy2x branch is preprocessed away and `implementation` is unconditionally set to `crypto_scalarmult_curve25519_ref10_implementation`. Nothing in area 7 ever depends on runtime dispatch on this build. | verified |
| 7.129 | every public entry point in area 7 | NULL for any pointer parameter declared `__attribute__((nonnull))` — e.g. `q`/`n`/`p` in `crypto_scalarmult*`, `sig`/`m`/`pk` in `crypto_sign_verify_detached`, `c`/`n`/`pk`/`sk` in `crypto_box*`, `key`/`ctx` in `crypto_kdf*`, `pk`/`sk`/`ct`/`ss` in `crypto_kem*` | **No runtime check; undefined behaviour** (segfault in practice, or elided by the optimiser). This is a contract, not an error return. The **only** NULL parameters with defined behaviour in area 7 are: `siglen_p`/`smlen_p`/`mlen_p` in the sign API (checked `!= NULL` before writing), `m` in `crypto_sign_open` (checked), and `rx`/`tx` in `crypto_kx_*_session_keys` (rows 7.94–7.97). | undefined-behaviour-not-tested |

## Area 8 — crypto_pwhash + crypto_ipcrypt

Files covered: `crypto_pwhash/crypto_pwhash.c`; `crypto_pwhash/argon2/{argon2.c, argon2-core.c,
argon2-encoding.c, argon2-fill-block-ref.c, blake2b-long.c, pwhash_argon2i.c, pwhash_argon2id.c}`;
`crypto_pwhash/scryptsalsa208sha256/{crypto_scrypt-common.c, pbkdf2-sha256.c,
pwhash_scryptsalsa208sha256.c, scrypt_platform.c, nosse/pwhash_scryptsalsa208sha256_nosse.c}`;
`crypto_ipcrypt/{crypto_ipcrypt.c, ipcrypt_soft.c}`; headers `crypto_pwhash.h`,
`crypto_pwhash_argon2i.h`, `crypto_pwhash_argon2id.h`, `crypto_pwhash_scryptsalsa208sha256.h`,
`crypto_ipcrypt.h`, `argon2.h`, `argon2-core.h`, `argon2-encoding.h`, `crypto_scrypt.h`.

Numeric constants assumed below are those of a 64-bit Linux build (`SIZE_MAX = 2^64-1`,
`HAVE_MMAP`, no `HAVE_*INTRIN_H`/`HAVE_ARMCRYPTO`, so `argon2_fill_segment_ref`,
`escrypt_kdf_nosse` and `ipcrypt_soft_implementation` are the selected implementations):

| symbol | value |
|---|---|
| `crypto_pwhash_BYTES_MIN` / `_MAX` | 16 / 4294967295 |
| `crypto_pwhash_PASSWD_MIN` / `_MAX` | 0 / 4294967295 |
| `crypto_pwhash_SALTBYTES` / `_STRBYTES` | 16 / 128 |
| `crypto_pwhash_argon2i_OPSLIMIT_MIN` / `argon2id_OPSLIMIT_MIN` | 3 / 1 |
| `crypto_pwhash_argon2*_OPSLIMIT_MAX` | 4294967295 |
| `crypto_pwhash_argon2*_MEMLIMIT_MIN` / `_MAX` | 8192 / 4398046510080 |
| `crypto_pwhash_argon2i_STRPREFIX` / `argon2id_STRPREFIX` | `"$argon2i$"` / `"$argon2id$"` |
| `ARGON2_MIN_OUTLEN` / `MAX_OUTLEN` | 16 / 0xFFFFFFFF |
| `ARGON2_MIN_SALT_LENGTH` / `MAX_SALT_LENGTH` | 8 / 0xFFFFFFFF |
| `ARGON2_MIN_MEMORY` / `MAX_MEMORY` | 8 / 0xFFFFFFFF |
| `ARGON2_MIN_LANES` / `MAX_LANES` / `MIN_THREADS` / `MAX_THREADS` | 1 / 0xFFFFFF / 1 / 0xFFFFFF |
| `ARGON2_MIN_TIME` / `MAX_TIME` | 1 / 0xFFFFFFFF |
| `ARGON2_VERSION_NUMBER` | 0x13 (decimal 19) |
| `crypto_pwhash_scryptsalsa208sha256_BYTES_MIN` / `_MAX` | 16 / 0x1fffffffe0 (137438953440) |
| `..._scryptsalsa208sha256_SALTBYTES` / `_STRBYTES` | 32 / 102 (string body is exactly 101 chars) |
| `..._scryptsalsa208sha256_OPSLIMIT_MIN` / `MEMLIMIT_MIN` | 32768 / 16777216 (**not enforced**, see 8.118) |
| `crypto_ipcrypt_{BYTES,KEYBYTES}` | 16 / 16 |
| `crypto_ipcrypt_ND_{KEYBYTES,TWEAKBYTES,INPUTBYTES,OUTPUTBYTES}` | 16 / 8 / 16 / 24 |
| `crypto_ipcrypt_NDX_{KEYBYTES,TWEAKBYTES,INPUTBYTES,OUTPUTBYTES}` | 32 / 16 / 16 / 32 |
| `crypto_ipcrypt_PFX_{KEYBYTES,BYTES}` | 32 / 16 |

`argon2_error_codes` values used below: `ARGON2_OK`=0, `OUTPUT_PTR_NULL`=-1, `OUTPUT_TOO_SHORT`=-2,
`OUTPUT_TOO_LONG`=-3, `PWD_TOO_SHORT`=-4, `PWD_TOO_LONG`=-5, `SALT_TOO_SHORT`=-6, `SALT_TOO_LONG`=-7,
`AD_TOO_SHORT`=-8, `AD_TOO_LONG`=-9, `SECRET_TOO_SHORT`=-10, `SECRET_TOO_LONG`=-11,
`TIME_TOO_SMALL`=-12, `TIME_TOO_LARGE`=-13, `MEMORY_TOO_LITTLE`=-14, `MEMORY_TOO_MUCH`=-15,
`LANES_TOO_FEW`=-16, `LANES_TOO_MANY`=-17, `PWD_PTR_MISMATCH`=-18, `SALT_PTR_MISMATCH`=-19,
`SECRET_PTR_MISMATCH`=-20, `AD_PTR_MISMATCH`=-21, `MEMORY_ALLOCATION_ERROR`=-22,
`FREE_MEMORY_CBK_NULL`=-23, `ALLOCATE_MEMORY_CBK_NULL`=-24, `INCORRECT_PARAMETER`=-25,
`INCORRECT_TYPE`=-26, `OUT_PTR_MISMATCH`=-27, `THREADS_TOO_FEW`=-28, `THREADS_TOO_MANY`=-29,
`MISSING_ARGS`=-30, `ENCODING_FAIL`=-31, `DECODING_FAIL`=-32, `THREAD_FAIL`=-33,
`DECODING_LENGTH_FAIL`=-34, `VERIFY_MISMATCH`=-35.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 8.1 | `crypto_pwhash` | `alg` is not `crypto_pwhash_ALG_ARGON2I13`(1) nor `ALG_ARGON2ID13`(2): e.g. `alg = 0` | `-1`, `errno = EINVAL`; `out` untouched (dispatch happens before any memset) | verified |
| 8.2 | `crypto_pwhash` | `alg = 3` (above the last valid id) | `-1`, `errno = EINVAL` | verified |
| 8.3 | `crypto_pwhash` | `alg = -1` | `-1`, `errno = EINVAL` | verified |
| 8.4 | `crypto_pwhash_str_alg` | `alg` not in {1,2}, e.g. `alg = 0` | `sodium_misuse()` → prints a message and `abort()`s; the function never returns (the trailing `return -1` is unreachable) | verified |
| 8.5 | `crypto_pwhash_str_verify` | `str` starts with neither `"$argon2id$"` nor `"$argon2i$"`, e.g. `"$7$..."` or `"$argon2d$v=19$..."` or `""` | `-1`, `errno = EINVAL` (no argon2 work done) | verified |
| 8.6 | `crypto_pwhash_str_needs_rehash` | same prefix condition as 8.5 | `-1`, `errno = EINVAL` | verified |
| 8.7 | `crypto_pwhash_argon2i` | `outlen > crypto_pwhash_argon2i_BYTES_MAX` (4294967295), e.g. `outlen = 4294967296` | `-1`, `errno = EFBIG` (note: `memset(out, 0, outlen)` already ran, i.e. the caller's buffer is zeroed / UB if shorter) | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >4 GiB caller buffer) |
| 8.8 | `crypto_pwhash_argon2i` | `outlen < crypto_pwhash_argon2i_BYTES_MIN` (16): `outlen = 0`, `1`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.9 | `crypto_pwhash_argon2i` | `passwdlen > crypto_pwhash_argon2i_PASSWD_MAX` (4294967295) | `-1`, `errno = EFBIG` | verified |
| 8.10 | `crypto_pwhash_argon2i` | `opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX` (4294967295), e.g. `4294967296` | `-1`, `errno = EFBIG` | verified |
| 8.11 | `crypto_pwhash_argon2i` | `memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX` (4398046510080) | `-1`, `errno = EFBIG` | verified |
| 8.12 | `crypto_pwhash_argon2i` | `opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN` (3): `opslimit = 0`, `1`, `2` | `-1`, `errno = EINVAL` | verified |
| 8.13 | `crypto_pwhash_argon2i` | `memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN` (8192): `memlimit = 0`, `1024`, `8191` | `-1`, `errno = EINVAL` | verified |
| 8.14 | `crypto_pwhash_argon2i` | `passwdlen < crypto_pwhash_argon2i_PASSWD_MIN` (0) | unreachable (`PASSWD_MIN == 0`); documented dead branch, would be `-1`/`EINVAL` | unreachable-from-public-API (PASSWD_MIN == 0) |
| 8.15 | `crypto_pwhash_argon2i` | `(const void *) out == (const void *) passwd` (output aliases password) | `-1`, `errno = EINVAL` | verified |
| 8.16 | `crypto_pwhash_argon2i` | `alg != crypto_pwhash_argon2i_ALG_ARGON2I13` (1) — e.g. `alg = 2` (`ARGON2ID13`) passed to the argon2i entry point | `-1`, `errno = EINVAL` (switch `default`) | verified |
| 8.17 | `crypto_pwhash_argon2i` | inner `argon2i_hash_raw() != ARGON2_OK` (only reachable via memory-allocation failure) | `-1`, `errno` left as set by the allocator | unreachable-from-public-API (inner allocation failure) |
| 8.18 | `crypto_pwhash_argon2id` | `outlen > crypto_pwhash_argon2id_BYTES_MAX` (4294967295) | `-1`, `errno = EFBIG` | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >4 GiB caller buffer) |
| 8.19 | `crypto_pwhash_argon2id` | `outlen < 16`: `0`, `1`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.20 | `crypto_pwhash_argon2id` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.21 | `crypto_pwhash_argon2id` | `opslimit > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.22 | `crypto_pwhash_argon2id` | `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.23 | `crypto_pwhash_argon2id` | `opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN` (1), i.e. `opslimit = 0` | `-1`, `errno = EINVAL` | verified |
| 8.24 | `crypto_pwhash_argon2id` | `memlimit < 8192`: `0`, `8191` | `-1`, `errno = EINVAL` | verified |
| 8.25 | `crypto_pwhash_argon2id` | `out == passwd` | `-1`, `errno = EINVAL` | verified |
| 8.26 | `crypto_pwhash_argon2id` | `alg != 2` — e.g. `alg = 1` (`ARGON2I13`) passed to the argon2id entry point, or `alg = 0` | `-1`, `errno = EINVAL` | verified |
| 8.27 | `crypto_pwhash_argon2id` | inner `argon2id_hash_raw() != ARGON2_OK` | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.28 | `crypto_pwhash_argon2i_str` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` (`out` already fully zeroed) | verified |
| 8.29 | `crypto_pwhash_argon2i_str` | `opslimit > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.30 | `crypto_pwhash_argon2i_str` | `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.31 | `crypto_pwhash_argon2i_str` | `opslimit < 3` (`0`,`1`,`2`) | `-1`, `errno = EINVAL` | verified |
| 8.32 | `crypto_pwhash_argon2i_str` | `memlimit < 8192` | `-1`, `errno = EINVAL` | verified |
| 8.33 | `crypto_pwhash_argon2i_str` | `argon2i_hash_encoded() != ARGON2_OK` (encoding buffer too small / allocation failure) | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.34 | `crypto_pwhash_argon2id_str` | `passwdlen > 4294967295` / `opslimit > 4294967295` / `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.35 | `crypto_pwhash_argon2id_str` | `opslimit < 1` (i.e. `0`) | `-1`, `errno = EINVAL` | verified |
| 8.36 | `crypto_pwhash_argon2id_str` | `memlimit < 8192` | `-1`, `errno = EINVAL` | verified |
| 8.37 | `crypto_pwhash_argon2id_str` | `argon2id_hash_encoded() != ARGON2_OK` | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.38 | `crypto_pwhash_argon2i_str_verify` | `passwdlen > crypto_pwhash_argon2i_PASSWD_MAX` (4294967295) | `-1`, `errno = EFBIG` | verified |
| 8.39 | `crypto_pwhash_argon2i_str_verify` | `passwdlen < PASSWD_MIN` (0) | unreachable dead branch (would be `-1`/`EINVAL`) | unreachable-from-public-API (PASSWD_MIN == 0) |
| 8.40 | `crypto_pwhash_argon2i_str_verify` | correct string, wrong password → `argon2i_verify` returns `ARGON2_VERIFY_MISMATCH` (-35) | `-1`, `errno = EINVAL` | verified |
| 8.41 | `crypto_pwhash_argon2i_str_verify` | malformed `str` (any `argon2_decode_string` failure, see 8.79–8.100) | `-1`, `errno` **not** set by this function (only `VERIFY_MISMATCH` sets `EINVAL`) | verified |
| 8.42 | `crypto_pwhash_argon2i_str_verify` | `str` is an argon2**id** string, e.g. `"$argon2id$v=19$m=8,t=1,p=1$<salt>$<hash>"` — `CC("$argon2i")` matches but the next `CC("$v=")` sees `"d$v="` | `-1` (inner `ARGON2_DECODING_FAIL` = -32) | verified |
| 8.43 | `crypto_pwhash_argon2i_str_verify` | `str = ""` (empty) | `-1` (inner `ARGON2_DECODING_FAIL`) | verified |
| 8.44 | `crypto_pwhash_argon2id_str_verify` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` (LCOV-excluded branch) | verified |
| 8.45 | `crypto_pwhash_argon2id_str_verify` | wrong password (`ARGON2_VERIFY_MISMATCH`) | `-1`, `errno = EINVAL` | verified |
| 8.46 | `crypto_pwhash_argon2id_str_verify` | malformed `str` / wrong prefix (`"$argon2i$v=19$..."` fails `CC("$argon2id")`) | `-1` (inner `ARGON2_DECODING_FAIL`) | verified |
| 8.47 | `_needs_rehash` (via `crypto_pwhash_argon2i_str_needs_rehash` / `crypto_pwhash_argon2id_str_needs_rehash`) | `opslimit > UINT32_MAX`, e.g. `4294967296` | `-1`, `errno = EINVAL` | verified |
| 8.48 | `_needs_rehash` | `memlimit / 1024U > UINT32_MAX`, i.e. `memlimit > 4398046511104` | `-1`, `errno = EINVAL` | verified |
| 8.49 | `_needs_rehash` | `strlen(str) >= crypto_pwhash_STRBYTES` (128) | `-1`, `errno = EINVAL` | verified |
| 8.50 | `_needs_rehash` | `calloc(strlen(str), 1)` returns NULL (OOM) | `-1` (errno from `calloc`) | unreachable-from-public-API (calloc() failure) |
| 8.51 | `_needs_rehash` | `argon2_decode_string()` fails (malformed string, wrong type, bad version, bad base64, trailing garbage, salt < 8 bytes, hash < 16 bytes, …) | `-1`, `errno = EINVAL` | verified |
| 8.52 | `_needs_rehash` | valid string but `ctx.t_cost != (uint32_t) opslimit` **or** `ctx.m_cost != (uint32_t) (memlimit/1024)` | `1` (non-zero, non-error “needs rehash”); note `p`/lanes and the argon2 *type* are **not** compared | verified |
| 8.53 | `argon2_ctx` | `argon2_validate_inputs(context) != ARGON2_OK` | that validation code is returned verbatim (see 8.58–8.78) | verified |
| 8.54 | `argon2_ctx` | `type` is neither `Argon2_id`(2) nor `Argon2_i`(1), e.g. `type = 0` or `3` (Argon2_d is not compiled in) | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.55 | `argon2_ctx` | `argon2_initialize()` fails | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` (or `ARGON2_INCORRECT_PARAMETER` = `-25` if instance/context NULL) | unreachable-from-public-API (allocation failure) |
| 8.56 | `argon2_ctx` | `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` (from `argon2_validate_inputs`) | verified |
| 8.57 | `argon2_validate_inputs` | `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` | verified |
| 8.58 | `argon2_validate_inputs` | `context->out == NULL` | `ARGON2_OUTPUT_PTR_NULL` = `-1` | verified |
| 8.59 | `argon2_validate_inputs` | `context->outlen < ARGON2_MIN_OUTLEN` (16): `0`, `1`, `15` | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.60 | `argon2_validate_inputs` | `context->outlen > ARGON2_MAX_OUTLEN` (0xFFFFFFFF) | `ARGON2_OUTPUT_TOO_LONG` = `-3`; **unreachable through `argon2_context`** because `outlen` is `uint32_t` (reachable only via `argon2_hash`, row 8.72) | unreachable-from-public-API (outlen is uint32_t (reachable via argon2_hash, row 8.84)) |
| 8.61 | `argon2_validate_inputs` | `context->pwd == NULL && context->pwdlen != 0` | `ARGON2_PWD_PTR_MISMATCH` = `-18` | verified |
| 8.62 | `argon2_validate_inputs` | `context->pwdlen < ARGON2_MIN_PWD_LENGTH` (0) | `ARGON2_PWD_TOO_SHORT` = `-4`; unreachable (min is 0 and the field is unsigned) | unreachable-from-public-API (ARGON2_MIN_PWD_LENGTH == 0) |
| 8.63 | `argon2_validate_inputs` | `context->pwdlen > ARGON2_MAX_PWD_LENGTH` (0xFFFFFFFF) | `ARGON2_PWD_TOO_LONG` = `-5`; unreachable via the `uint32_t` field (reachable via `argon2_hash`, row 8.71) | unreachable-from-public-API (pwdlen is uint32_t (reachable via argon2_hash, row 8.83)) |
| 8.64 | `argon2_validate_inputs` | `context->salt == NULL && context->saltlen != 0` | `ARGON2_SALT_PTR_MISMATCH` = `-19` | verified |
| 8.65 | `argon2_validate_inputs` | `context->saltlen < ARGON2_MIN_SALT_LENGTH` (8): `0` (with `salt == NULL`), `1`, `7` | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.66 | `argon2_validate_inputs` | `context->saltlen > ARGON2_MAX_SALT_LENGTH` (0xFFFFFFFF) | `ARGON2_SALT_TOO_LONG` = `-7`; unreachable via the `uint32_t` field (reachable via `argon2_hash`, row 8.73) | unreachable-from-public-API (saltlen is uint32_t (reachable via argon2_hash, row 8.85)) |
| 8.67 | `argon2_validate_inputs` | `context->secret == NULL && context->secretlen != 0` | `ARGON2_SECRET_PTR_MISMATCH` = `-20` | verified |
| 8.68 | `argon2_validate_inputs` | `secret != NULL && secretlen < ARGON2_MIN_SECRET` (0) | `ARGON2_SECRET_TOO_SHORT` = `-10`; unreachable (min is 0) | unreachable-from-public-API (ARGON2_MIN_SECRET == 0) |
| 8.69 | `argon2_validate_inputs` | `secret != NULL && secretlen > ARGON2_MAX_SECRET` (0xFFFFFFFF) | `ARGON2_SECRET_TOO_LONG` = `-11`; unreachable (`uint32_t` field) | unreachable-from-public-API (secretlen is uint32_t) |
| 8.70 | `argon2_validate_inputs` | `context->ad == NULL && context->adlen != 0` | `ARGON2_AD_PTR_MISMATCH` = `-21` | verified |
| 8.71 | `argon2_validate_inputs` | `ad != NULL && adlen < ARGON2_MIN_AD_LENGTH` (0) | `ARGON2_AD_TOO_SHORT` = `-8`; unreachable (min is 0) | unreachable-from-public-API (ARGON2_MIN_AD_LENGTH == 0) |
| 8.72 | `argon2_validate_inputs` | `ad != NULL && adlen > ARGON2_MAX_AD_LENGTH` (0xFFFFFFFF) | `ARGON2_AD_TOO_LONG` = `-9`; unreachable (`uint32_t` field) | unreachable-from-public-API (adlen is uint32_t) |
| 8.73 | `argon2_validate_inputs` | `context->lanes < ARGON2_MIN_LANES` (1), i.e. `lanes = 0` | `ARGON2_LANES_TOO_FEW` = `-16` | verified |
| 8.74 | `argon2_validate_inputs` | `context->lanes > ARGON2_MAX_LANES` (0xFFFFFF), e.g. `lanes = 0x1000000` | `ARGON2_LANES_TOO_MANY` = `-17` | verified |
| 8.75 | `argon2_validate_inputs` | `context->m_cost < ARGON2_MIN_MEMORY` (8): `m_cost = 0..7` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.76 | `argon2_validate_inputs` | `context->m_cost > ARGON2_MAX_MEMORY` (0xFFFFFFFF on this build) | `ARGON2_MEMORY_TOO_MUCH` = `-15`; unreachable because `m_cost` is `uint32_t` and `ARGON2_MAX_MEMORY == UINT32_MAX` here (reachable on platforms where `ARGON2_MAX_MEMORY_BITS < 32`, e.g. 32-bit `void *` → max 2^21) | unreachable-from-public-API (ARGON2_MAX_MEMORY == UINT32_MAX on this build) |
| 8.77 | `argon2_validate_inputs` | second memory check: `m_cost < 8 * lanes` with `m_cost >= 8`, e.g. `lanes = 4, m_cost = 31` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` (distinct branch from 8.75) | verified |
| 8.78 | `argon2_validate_inputs` | `context->t_cost < ARGON2_MIN_TIME` (1), i.e. `t_cost = 0` | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.79 | `argon2_validate_inputs` | `context->t_cost > ARGON2_MAX_TIME` (0xFFFFFFFF) | `ARGON2_TIME_TOO_LARGE` = `-13`; unreachable (`uint32_t` field) | unreachable-from-public-API (t_cost is uint32_t) |
| 8.80 | `argon2_validate_inputs` | `context->threads < ARGON2_MIN_THREADS` (1), i.e. `threads = 0` (with `lanes >= 1`) | `ARGON2_THREADS_TOO_FEW` = `-28` | verified |
| 8.81 | `argon2_validate_inputs` | `context->threads > ARGON2_MAX_THREADS` (0xFFFFFF) | `ARGON2_THREADS_TOO_MANY` = `-29` | verified |
| 8.82 | `argon2_validate_inputs` | (never produced) `FREE_MEMORY_CBK_NULL` -23, `ALLOCATE_MEMORY_CBK_NULL` -24, `OUT_PTR_MISMATCH` -27, `MISSING_ARGS` -30, `THREAD_FAIL` -33 | dead enum values in this libsodium fork: no code path returns them | unreachable-from-public-API (dead enum values; no code path returns them) |
| 8.83 | `argon2_hash` | `pwdlen > ARGON2_MAX_PWD_LENGTH` (0xFFFFFFFF) — reachable because `pwdlen` is `size_t` | `ARGON2_PWD_TOO_LONG` = `-5` (checked *after* `randombytes_buf(hash, hashlen)` has already overwritten the caller's `hash` buffer) | verified |
| 8.84 | `argon2_hash` | `hashlen > ARGON2_MAX_OUTLEN` (0xFFFFFFFF) | `ARGON2_OUTPUT_TOO_LONG` = `-3` | verified |
| 8.85 | `argon2_hash` | `saltlen > ARGON2_MAX_SALT_LENGTH` (0xFFFFFFFF) | `ARGON2_SALT_TOO_LONG` = `-7` | verified |
| 8.86 | `argon2_hash` | `malloc(hashlen)` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.87 | `argon2_hash` | any `argon2_ctx` failure (e.g. `saltlen = 4` → `-6`; `m_cost = 4` → `-14`; `t_cost = 0` → `-12`; `parallelism = 0` → `-16`; `hashlen = 8` → `-2`) | that code is returned verbatim; `out` scratch buffer is zeroed and freed | verified |
| 8.88 | `argon2_hash` | `encoded != NULL && encodedlen != 0` and `argon2_encode_string()` fails (buffer too small) | `ARGON2_ENCODING_FAIL` = `-31`; both `out` and `encoded` are zeroed | verified |
| 8.89 | `argon2i_hash_encoded` / `argon2id_hash_encoded` | `encodedlen` smaller than the required encoded length, e.g. `encodedlen = 10` with `hashlen = 32` | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.90 | `argon2i_hash_raw` / `argon2id_hash_raw` | `hashlen < 16` (e.g. 8) | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.91 | `argon2i_hash_raw` / `argon2id_hash_raw` | `saltlen < 8` (e.g. 4) | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.92 | `argon2i_hash_raw` / `argon2id_hash_raw` | `parallelism = 0` | `ARGON2_LANES_TOO_FEW` = `-16` | verified |
| 8.93 | `argon2i_hash_raw` / `argon2id_hash_raw` | `t_cost = 0` | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.94 | `argon2i_hash_raw` / `argon2id_hash_raw` | `m_cost < 8` or `m_cost < 8 * parallelism` (e.g. `m_cost = 8, parallelism = 2`) | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.95 | `argon2_verify` (`argon2i_verify` / `argon2id_verify`) | `strlen(encoded) > UINT32_MAX` | `ARGON2_DECODING_LENGTH_FAIL` = `-34` | unreachable-from-public-API (strlen() cannot exceed UINT32_MAX here) |
| 8.96 | `argon2_verify` | any of the three `malloc(strlen(encoded))` (ad/salt/out) or the fourth `malloc(ctx.outlen)` returns NULL — including `encoded = ""` on implementations where `malloc(0)` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.97 | `argon2_verify` | `argon2_decode_string()` != OK | that decode code is returned verbatim (`-32`, `-26`, or a validation code) | verified |
| 8.98 | `argon2_verify` | decode OK, re-hash OK, `sodium_memcmp(out, ctx.out, ctx.outlen) != 0` (wrong password) | `ARGON2_VERIFY_MISMATCH` = `-35` | verified |
| 8.99 | `argon2_verify` | decode OK but the recomputation `argon2_hash(...)` fails (allocation failure) | that code is returned; **no** mismatch conversion (the `ret == ARGON2_OK` guard) | unreachable-from-public-API (allocation failure) |
| 8.100 | `argon2_verify` | `type` not `Argon2_i`/`Argon2_id` (reaches `argon2_decode_string`’s `else`) | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.101 | `argon2_initialize` | `instance == NULL` or `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` | verified |
| 8.102 | `argon2_initialize` | `malloc(sizeof(uint64_t) * segment_length)` for `pseudo_rands` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.103 | `allocate_memory` (static, via `argon2_initialize`) | `region == NULL` | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (region is never NULL at the call site) |
| 8.104 | `allocate_memory` | `m_cost == 0`, or `sizeof(block) * m_cost` overflows (`memory_size / m_cost != sizeof(block)`) | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (m_cost >= 8 is already validated) |
| 8.105 | `allocate_memory` | `malloc(sizeof(block_region))` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.106 | `allocate_memory` | `mmap()` fails (`MAP_FAILED`) — e.g. m_cost near `ARGON2_MAX_MEMORY` (4 TiB) | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22`; `*region` freed and set to NULL | unreachable-from-public-API (would mmap(MAP_POPULATE) 4 TiB - not safely testable) |
| 8.107 | `blake2b_long` | `outlen > UINT32_MAX` | `-1` (goto fail with `ret` still `-1`); unreachable from argon2 | verified |
| 8.108 | `blake2b_long` | `outlen == 0` (or any `outlen` rejected by `crypto_generichash_blake2b_init`, which requires `1 <= outlen <= 64` for the short path) | `-1` (the value returned by the failing `crypto_generichash_blake2b_*` call) | verified |
| 8.109 | `argon2_finalize` | `blake2b_long()` fails | **return value is ignored**: `argon2_finalize` is `void`, so `context->out` is left unmodified and `argon2_ctx` still returns `ARGON2_OK`. Silent-failure path (unreachable in practice since `outlen >= 16`) | unreachable-from-public-API (outlen >= 16 is already validated) |
| 8.110 | `argon2_fill_memory_blocks` | `instance == NULL` or `instance->lanes == 0` | returns early (`void`); no error signalled | verified |
| 8.111 | `argon2_fill_segment_ref` | `instance == NULL` | returns early (`void`) | verified |
| 8.112 | `argon2_decode_string` | `type` neither `Argon2_i` nor `Argon2_id` | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.113 | `argon2_decode_string` | wrong type prefix: `"$argon2id..."` decoded as `Argon2_i` succeeds at `CC("$argon2i")` but then fails `CC("$v=")`; `"$argon2i..."` decoded as `Argon2_id` fails `CC("$argon2id")` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.114 | `argon2_decode_string` | prefix garbage: `""`, `"argon2i$v=19$..."` (no leading `$`), `"$argon2d$v=19$..."`, `"$ARGON2I$..."` (case-sensitive), `"$argon"` (truncated) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.115 | `argon2_decode_string` | missing `$v=`: `"$argon2id$m=8,t=1,p=1$..."` (libsodium requires the version field; the `CC_opt` optional-prefix macro is defined but unused) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.116 | `argon2_decode_string` | `v=` value is not a minimal decimal: `"$argon2id$v=$..."` (no digit), `"$argon2id$v=019$..."` (leading zero), `"$argon2id$v=+19$..."`, `"$argon2id$v=1a9$..."` (stops at `a`, later `CC("$m=")` fails) | `ARGON2_DECODING_FAIL` = `-32` (via `decode_decimal`/`DECIMAL_U32` returning NULL, or the following `CC`) | verified |
| 8.117 | `argon2_decode_string` | `v=` value `> UINT32_MAX`, e.g. `"v=4294967296"`; or so long it overflows `unsigned long` | `ARGON2_DECODING_FAIL` = `-32` (`DECIMAL_U32` rejects `dec_x > UINT32_MAX`; `decode_decimal` rejects `acc > ULONG_MAX/10`) | verified |
| 8.118 | `argon2_decode_string` | `version != ARGON2_VERSION_NUMBER` (0x13 = 19): `"v=16"`, `"v=0"`, `"v=20"` | `ARGON2_INCORRECT_TYPE` = `-26` (note: *not* `DECODING_FAIL`) | verified |
| 8.119 | `argon2_decode_string` | missing `$m=` after the version, e.g. `"$argon2id$v=19$t=1,p=1$..."` or `"$argon2id$v=19"` (truncated) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.120 | `argon2_decode_string` | bad `m=` value: `"m="` (empty), `"m=08"` (leading zero), `"m=4294967296"` (> UINT32_MAX), `"m=-8"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.121 | `argon2_decode_string` | missing `,t=`: `"$argon2id$v=19$m=8$..."` or `"$argon2id$v=19$m=8;t=1..."` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.122 | `argon2_decode_string` | bad `t=` value: `"t="`, `"t=01"`, `"t=4294967296"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.123 | `argon2_decode_string` | missing `,p=`: `"$argon2id$v=19$m=8,t=1$..."` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.124 | `argon2_decode_string` | bad `p=` value: `"p="`, `"p=01"`, `"p=4294967296"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.125 | `argon2_decode_string` | the three `if (ctx->m_cost / t_cost / lanes > UINT32_MAX)` guards after each `DECIMAL_U32` | `ARGON2_INCORRECT_TYPE` = `-26`; dead code (the values are already `uint32_t`) | verified |
| 8.126 | `argon2_decode_string` | missing `$` before the salt: `"$argon2id$v=19$m=8,t=1,p=1<salt>$<hash>"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.127 | `argon2_decode_string` | salt Base64 decodes to more than `maxsaltlen` bytes (`sodium_base642bin` → `ERANGE`) — i.e. salt longer than the caller's buffer (`ctx->saltlen` on entry) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.128 | `argon2_decode_string` | salt Base64 has invalid trailing bits (`acc_len > 4` or non-zero low bits), e.g. `"...$c29tZQ=="` (padding is rejected: `ORIGINAL_NO_PADDING` variant) or a 1-char group | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.129 | `argon2_decode_string` | salt field empty (`"$argon2id$v=19$m=8,t=1,p=1$$<hash>"`) → `saltlen = 0` and `salt != NULL` | `ARGON2_SALT_TOO_SHORT` = `-6` from the `argon2_validate_inputs` call at the end | verified |
| 8.130 | `argon2_decode_string` | salt shorter than 8 bytes after decoding, e.g. base64 of 4 bytes | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.131 | `argon2_decode_string` | missing `$` between salt and hash, e.g. `"...$<salt><hash>"` (the salt Base64 consumes both, then `CC("$")` sees NUL) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.132 | `argon2_decode_string` | hash Base64 exceeds `maxoutlen` (`ctx->outlen` on entry) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.133 | `argon2_decode_string` | hash Base64 invalid / truncated group, e.g. `"...$Zg"` decodes to 1 byte | `ARGON2_OUTPUT_TOO_SHORT` = `-2` (validation) — or `-32` if the bit-padding check fails first | verified |
| 8.134 | `argon2_decode_string` | hash field empty (`"...$<salt>$"`) → `outlen = 0` | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.135 | `argon2_decode_string` | `p=0` in the string (lanes 0) | `ARGON2_LANES_TOO_FEW` = `-16` (from the final `argon2_validate_inputs`; note `threads` is set from `lanes`, so `-28` is not reached first) | verified |
| 8.136 | `argon2_decode_string` | `t=0` in the string | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.137 | `argon2_decode_string` | `m=0`..`m=7` in the string | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.138 | `argon2_decode_string` | `m` valid but `m < 8 * p`, e.g. `"m=8,t=1,p=2"` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.139 | `argon2_decode_string` | trailing garbage after the hash: `*str != 0`, e.g. `"...$<hash>$"`, `"...$<hash>x"`, `"...$<hash>\n"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.140 | `argon2_decode_string` | via `argon2_verify` with `ctx.pwd == NULL, pwdlen == 0` — decoding leaves `ctx->pwd` NULL; if a caller sets `pwd = NULL, pwdlen != 0` | `ARGON2_PWD_PTR_MISMATCH` = `-18` from the final validation | verified |
| 8.141 | `decode_decimal` (static) | no digit at all at the current position | `NULL` → caller yields `ARGON2_DECODING_FAIL` | verified |
| 8.142 | `decode_decimal` | non-minimal encoding: first char `'0'` and more than one digit consumed (`"00"`, `"007"`, `"019"`) — note bare `"0"` **is** accepted | `NULL` → `ARGON2_DECODING_FAIL` | verified |
| 8.143 | `decode_decimal` | value overflows `unsigned long`: `acc > ULONG_MAX/10` before the multiply, or `c > ULONG_MAX - acc` after | `NULL` → `ARGON2_DECODING_FAIL` | verified |
| 8.144 | `argon2_encode_string` | `type` not `Argon2_i`/`Argon2_id` | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.145 | `argon2_encode_string` | `dst_len` too small for the `"$argon2id$v="` / `"$argon2i$v="` prefix (`pp_len >= dst_len` in `SS`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.146 | `argon2_encode_string` | `argon2_validate_inputs(ctx) != ARGON2_OK` (checked **after** the prefix has already been written into `dst`) | that validation code (e.g. `-6` for a short salt, `-2` for a short out); `dst` already contains a partial `"$argon2id$v="` string | verified |
| 8.147 | `argon2_encode_string` | `dst_len` runs out at any later `SS`/`SX` (`"$m="`, m_cost digits, `",t="`, `",p="`, `"$"`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.148 | `argon2_encode_string` | `sodium_bin2base64` returns NULL because `dst_len` is too small for the salt or the output (`SB`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.149 | `crypto_pwhash_scryptsalsa208sha256` | `outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX` (0x1fffffffe0) | `-1`, `errno = EFBIG` (LCOV-excluded); `memset(out, 0, outlen)` already ran | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >137 GB caller buffer) |
| 8.150 | `crypto_pwhash_scryptsalsa208sha256` | `passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX` (`SODIUM_SIZE_MAX`) | `-1`, `errno = EFBIG`; unreachable on 64-bit (`PASSWD_MAX == SIZE_MAX`) | unreachable-from-public-API (PASSWD_MAX == SIZE_MAX on 64-bit) |
| 8.151 | `crypto_pwhash_scryptsalsa208sha256` | `outlen < 16`: `0`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.152 | `crypto_pwhash_scryptsalsa208sha256` | `pickparams() != 0` | unreachable — `pickparams` always returns `0`; documented dead branch (`-1`/`EINVAL`) | unreachable-from-public-API (pickparams() always returns 0) |
| 8.153 | `crypto_pwhash_scryptsalsa208sha256` | `(const void *) out == (const void *) passwd` | `-1`, `errno = EINVAL` | verified |
| 8.154 | `crypto_pwhash_scryptsalsa208sha256` | **no** validation of `opslimit`/`memlimit` against `OPSLIMIT_MIN`(32768)/`MEMLIMIT_MIN`(16777216): `opslimit = 0` is silently clamped to 32768 by `pickparams`, `memlimit = 0` yields `N=2, r=8, p=512` | returns `0` (success) — asymmetric with the argon2 entry points; **not** a rejection | verified |
| 8.155 | `crypto_pwhash_scryptsalsa208sha256` | inner `crypto_pwhash_scryptsalsa208sha256_ll` failure (see 8.169–8.181), e.g. giant `memlimit` making `r*p >= 2^30` | `-1` with `errno` from `escrypt_kdf_nosse` | unreachable-from-public-API (pickparams output can never make r*p >= 2^30; any other failure needs a >=64 GiB region) |
| 8.156 | `crypto_pwhash_scryptsalsa208sha256_str` | `passwdlen > PASSWD_MAX` (`SIZE_MAX`) | `-1`, `errno = EFBIG`; unreachable on 64-bit | unreachable-from-public-API (PASSWD_MAX == SIZE_MAX on 64-bit) |
| 8.157 | `crypto_pwhash_scryptsalsa208sha256_str` | `passwdlen < PASSWD_MIN` (0) or `pickparams() != 0` | `-1`, `errno = EINVAL`; both unreachable | unreachable-from-public-API (PASSWD_MIN == 0 and pickparams() always returns 0) |
| 8.158 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_gensalt_r(...) == NULL` | `-1`, `errno = EINVAL`; unreachable from `pickparams` output (`N_log2 <= 63`, `r*p <= 0x3FFFFFF8 < 2^30`) | unreachable-from-public-API (pickparams output always satisfies gensalt) |
| 8.159 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_init_local() != 0` | `-1`; unreachable (`escrypt_init_local` always returns 0) | unreachable-from-public-API (escrypt_init_local() always returns 0) |
| 8.160 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_r(...) == NULL` (KDF failure / allocation failure) | `-1`, `errno = EINVAL` | unreachable-from-public-API (pickparams output always yields a working KDF setting) |
| 8.161 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `sodium_strnlen(str, 102) != 101`: `str` shorter than 101 chars (`""`, a truncated `$7$…`), or 102+ chars / not NUL-terminated within 102 | `-1` (errno untouched) | verified |
| 8.162 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | 101-char `str` whose setting is malformed → `escrypt_r` returns NULL: prefix not `"$7$"`, `N_log2` char outside itoa64 (`"./0-9A-Za-z"`), a non-itoa64 char in the 5-char `r` or `p` fields, or `need > buflen` | `-1` | verified |
| 8.163 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | wrong password (well-formed string, `sodium_memcmp(wanted, str, 102) != 0`) | `-1` (the value of `sodium_memcmp`) | verified |
| 8.164 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `escrypt_init_local() != 0` | `-1`; unreachable | unreachable-from-public-API (escrypt_init_local() always returns 0) |
| 8.165 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `pickparams() != 0` | `-1`, `errno = EINVAL`; unreachable | unreachable-from-public-API (pickparams() always returns 0) |
| 8.166 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `sodium_strnlen(str, 102) != 101` (too short / too long) | `-1`, `errno = EINVAL` | verified |
| 8.167 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `escrypt_parse_setting(str, …) == NULL` (bad `$7$` prefix or bad itoa64 chars) | `-1`, `errno = EINVAL` | verified |
| 8.168 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | valid string but `N_log2`, `r` or `p` differ from `pickparams(opslimit, memlimit)` | `1` (non-zero, non-error) | verified |
| 8.169 | `crypto_pwhash_scryptsalsa208sha256_ll` / `escrypt_kdf_nosse` | `buflen > ((2^32)-1)*32` = 137438953440 (only compiled when `SIZE_MAX > UINT32_MAX`) | `-1`, `errno = EFBIG` | verified |
| 8.170 | `crypto_pwhash_scryptsalsa208sha256_ll` | `(uint64_t) r * p >= 2^30`, e.g. `r = 1, p = 1073741824` or `r = 32768, p = 32768` | `-1`, `errno = EFBIG` | verified |
| 8.171 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > UINT32_MAX`, e.g. `N = 2^32` | `-1`, `errno = EFBIG` | verified |
| 8.172 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N` not a power of two: `3`, `1000`, `1023` | `-1`, `errno = EINVAL` | verified |
| 8.173 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N < 2`: `N = 0` or `N = 1` (note `N = 0` also passes the power-of-two test, so the `N < 2` clause is the one that fires) | `-1`, `errno = EINVAL` | verified |
| 8.174 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r == 0` | `-1`, `errno = EINVAL` | verified |
| 8.175 | `crypto_pwhash_scryptsalsa208sha256_ll` | `p == 0` | `-1`, `errno = EINVAL` | verified |
| 8.176 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r > SIZE_MAX / 128 / p` | `-1`, `errno = ENOMEM` | unreachable-from-public-API (r*p < 2^30 is checked first, so r > SIZE_MAX/128/p is impossible on 64-bit) |
| 8.177 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r > SIZE_MAX / 256` (only compiled when `SIZE_MAX/256 <= UINT32_MAX`, i.e. 32-bit) | `-1`, `errno = ENOMEM` | unreachable-from-public-API (only compiled when SIZE_MAX/256 <= UINT32_MAX (32-bit)) |
| 8.178 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > SIZE_MAX / 128 / r` | `-1`, `errno = ENOMEM` | verified |
| 8.179 | `crypto_pwhash_scryptsalsa208sha256_ll` | `need = B_size + V_size` wraps (`need < V_size`) | `-1`, `errno = ENOMEM` | verified |
| 8.180 | `crypto_pwhash_scryptsalsa208sha256_ll` | `need += XY_size` wraps (`need < XY_size`) | `-1`, `errno = ENOMEM` | verified |
| 8.181 | `crypto_pwhash_scryptsalsa208sha256_ll` | `escrypt_free_region()` fails (munmap error) or `escrypt_alloc_region()` returns NULL (OOM for `128*r*(N+p) + 256*r + 64` bytes) | `-1` | unreachable-from-public-API (would need a >=256 GiB mmap(MAP_POPULATE) - not safely testable) |
| 8.182 | `escrypt_parse_setting` | `setting[0] != '$' \|\| setting[1] != '7' \|\| setting[2] != '$'` — e.g. `"$6$…"`, `"7$…"`, `""` | `NULL` | verified |
| 8.183 | `escrypt_parse_setting` | `setting[3]` (the `N_log2` char) is not in `"./0123456789A-Za-z"`, e.g. `'$'`, `'-'`, `'*'`, NUL | `NULL` (and `*N_log2_p` set to 0) | verified |
| 8.184 | `escrypt_parse_setting` | any of the 5 chars of the 30-bit `r` field is not in itoa64 (includes a string that ends early, since NUL is not in itoa64) | `NULL` (`*r_p = 0`) | verified |
| 8.185 | `escrypt_parse_setting` | any of the 5 chars of the 30-bit `p` field is not in itoa64 | `NULL` (`*p_p = 0`) | verified |
| 8.186 | `escrypt_gensalt_r` | `need = 14 + BYTES2CHARS(srclen) + 1 > buflen`, e.g. `srclen = 32` (`saltlen = 43`, `need = 58`) with `buflen = 57` | `NULL` | verified |
| 8.187 | `escrypt_gensalt_r` | `need < saltlen` (size wrap) | `NULL`; unreachable | unreachable-from-public-API (size wrap is impossible) |
| 8.188 | `escrypt_gensalt_r` | `saltlen < srclen`, i.e. `BYTES2CHARS(srclen) < srclen` | `NULL`; unreachable (`(8b+5)/6 >= b`) | unreachable-from-public-API ((8b+5)/6 >= b always holds) |
| 8.189 | `escrypt_gensalt_r` | `N_log2 > 63` (would index past `itoa64`) | `NULL` | verified |
| 8.190 | `escrypt_gensalt_r` | `(uint64_t) r * p >= 2^30` | `NULL` | verified |
| 8.191 | `escrypt_gensalt_r` | `encode64_uint32`/`encode64` runs out of `dstlen`, or `dst >= buf + buflen` | `NULL` (“can't happen” after the `need > buflen` check) | unreachable-from-public-API ("can't happen" after the need > buflen check) |
| 8.192 | `escrypt_r` | `escrypt_parse_setting(setting, …) == NULL` (see 8.182–8.185) | `NULL` (note: `randombytes_buf(buf, buflen)` has already scrambled the caller's output buffer) | verified |
| 8.193 | `escrypt_r` | `buf == NULL` | `NULL` | verified |
| 8.194 | `escrypt_r` | `need = prefixlen + saltlen + 1 + 43 + 1 > buflen` — e.g. a `$7$` setting with an over-long salt field, or `buflen` < 102 | `NULL` | verified |
| 8.195 | `escrypt_r` | `need < saltlen` (size wrap) | `NULL` | unreachable-from-public-API (size wrap is impossible) |
| 8.196 | `escrypt_r` | `escrypt_kdf(...) != 0` — any of 8.169–8.181, e.g. a setting encoding `r = 0`/`p = 0` or `N_log2 = 0` (→ `N = 1 < 2`) or `N_log2 = 63` (→ `N > UINT32_MAX`) | `NULL` | verified |
| 8.197 | `escrypt_r` | final `encode64` returns NULL or `dst >= buf + buflen` | `NULL` (“can't happen”) | unreachable-from-public-API ("can't happen" after the need > buflen check) |
| 8.198 | `escrypt_alloc_region` | `mmap()` fails (`MAP_FAILED`) | `NULL` returned, `region->base = NULL`, `region->size = 0` | verified |
| 8.199 | `escrypt_alloc_region` | non-mmap fallback: `size + 63 < size` (wrap) or `malloc(size + 63) == NULL` | `NULL`, `errno = ENOMEM` | unreachable-from-public-API (the non-mmap fallback is not compiled (HAVE_MMAP)) |
| 8.200 | `escrypt_free_region` | `munmap(region->base, region->size)` fails | `-1` (propagates to `-1` from `_ll` / `escrypt_free_local`) | unreachable-from-public-API (munmap() failure) |
| 8.201 | `escrypt_PBKDF2_SHA256` | `dkLen > 0x1fffffffe0` (only compiled when `SIZE_MAX > 0x1fffffffe0`) | `sodium_misuse()` → `abort()`; the function is `void` and cannot report an error otherwise | verified |
| 8.202 | `crypto_ipcrypt_encrypt` / `_decrypt` | — | **no rejection branch exists**: the function is `void`, performs a fixed 16-byte AES-128 ECB block operation, and validates nothing. Buffers shorter than `crypto_ipcrypt_BYTES` (16) / keys shorter than `crypto_ipcrypt_KEYBYTES` (16) are out-of-bounds reads/writes (UB), not errors | verified |
| 8.203 | `crypto_ipcrypt_nd_encrypt` | — | `void`, no validation. Reads exactly 16 bytes of `in`, 8 of `t`, 16 of `k`; writes exactly 24 bytes of `out` (`t` copied to `out[0..8)`, ciphertext to `out[8..24)`). Short `out` (e.g. 16 bytes) is a buffer overflow, not a reported error | verified |
| 8.204 | `crypto_ipcrypt_nd_decrypt` | — | `void`, no validation. Reads 24 bytes of `in` (tweak `in[0..8)`, ct `in[8..24)`), writes 16 bytes of `out`. A corrupted/forged input is decrypted to garbage: **there is no authentication tag and therefore no failure indication** | verified |
| 8.205 | `crypto_ipcrypt_ndx_encrypt` | — | `void`, no validation. 32-byte key (`k[0..16)` = data key, `k[16..32)` = tweak key), 16-byte tweak, 16-byte input, 32-byte output | verified |
| 8.206 | `crypto_ipcrypt_ndx_decrypt` | — | `void`, no validation; no integrity check, so a forged 32-byte input silently produces garbage | verified |
| 8.207 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | degenerate key where the two halves are identical (`k[0..16) == k[16..32)`, detected as `tkeys[5] XOR rkeys[5] == 0`) | **not an error**: the data key is silently replaced by `k[i] ^ 0x5a` and the operation proceeds. Must be modelled as a normal (not rejecting) path | verified |
| 8.208 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | — | `void`, no validation; 32-byte key, 16-byte in/out | verified |
| 8.209 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | degenerate key `k[0..16) == k[16..32)` (`k1keys[5] XOR k2keys[5] == 0`) | **not an error**: `k2` is re-derived as `k[i] ^ 0x5a`; operation proceeds | verified |
| 8.210 | `crypto_ipcrypt_keygen` / `_nd_keygen` / `_ndx_keygen` / `_pfx_keygen` | — | `void`, cannot fail (delegates to `randombytes_buf`) | verified |
| 8.211 | `_crypto_ipcrypt_pick_best_implementation` | — | always returns `0`; with no `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H` it always selects `ipcrypt_soft_implementation` | verified |
| 8.212 | `_crypto_pwhash_argon2_pick_best_implementation` | — | always returns `0`; with no SIMD macros it always selects `argon2_fill_segment_ref` | verified |
| 8.213 | (adjacent, `sodium/codecs.c` — **not** in area 8's files) `sodium_ip2bin` | libsodium 1.0.23's `crypto_ipcrypt` has **no** IP-string entry points; string↔16-byte conversion is done by `sodium_ip2bin` / `sodium_bin2ip`. Rejections: zone (`%…`) on a non-IPv6 address, empty zone, zone char outside `[0-9a-zA-Z._-]`, malformed IPv6 (bad `::`, >4 hex digits per group, wrong group count, embedded IPv4 not at the end), malformed IPv4 (>3 digits, octet > 255, missing/extra dots, trailing junk) | `-1` (0 on success). Listed here only so the “bad IP string” cases are accounted for; they belong to the utils/codecs area | verified |
| 8.214 | (adjacent, `sodium/codecs.c`) `sodium_bin2ip` | `ip_maxlen <= 2`, or the rendered address needs `>= ip_maxlen` bytes | `NULL` | verified |

**Row count: 214.**  162 rows are `verified` by
`tests/a8_argon2.rs` (8.1 – 8.52), `tests/a8_argon2_core.rs` (8.53 – 8.111 and 8.148),
`tests/a8_argon2_encoding.rs` (8.112 – 8.147), `tests/a8_scrypt.rs` (8.149 – 8.201) and
`tests/a8_ipcrypt.rs` (8.202 – 8.214).  The other 52 rows, marked
`unreachable-from-public-API` are real C branches that cannot fire on this platform (a
`uint32_t` field, a minimum of 0, `pickparams` never failing) or that would require an
allocation failure / a multi-terabyte buffer.

Corrections found while writing those tests (the C is authoritative):

* **8.48** — the truncating division makes the first rejected `memlimit`
  `4294967296 * 1024 = 4398046511104`, not "`> 4398046511104`"; `4398046511103` is accepted.
* **8.116** — `"v=1a9"` is not a `DECODING_FAIL`: `decode_decimal` stops at `'a'` with the
  value 1 and the `version != ARGON2_VERSION_NUMBER` check fires first, giving
  `ARGON2_INCORRECT_TYPE` (-26).
* **8.133** — `"...$YWJjZA=="` gives `ARGON2_OUTPUT_TOO_SHORT` (-2), not -32: `'='` is
  outside the `ORIGINAL_NO_PADDING` alphabet, so 4 bytes decode and the
  `argon2_validate_inputs` call runs *before* the trailing-NUL check.
* **8.148** — `sodium_bin2base64` never returns NULL for a short buffer; it calls
  `sodium_misuse()` and **aborts**.  So the `SB` NULL check is dead code and
  `argon2_encode_string` / `argon2_hash` abort instead of returning
  `ARGON2_ENCODING_FAIL` once `dst_len` is large enough to get past the last `SS`
  (`dst_len >= 27` for `$argon2i$v=19$m=8,t=1,p=1$`).  Verified with `eq_abort`.
* **8.139** — appending `'x'` to the hash is *not* trailing garbage (it is a valid Base64
  character and simply lengthens the digest); a character outside the alphabet is needed.
* **8.183/8.184/8.185** — `decode64_one` is `strchr(itoa64, c)`, which also matches the
  terminating NUL of `itoa64` and yields the out-of-range value 64.  A `$7$` setting that
  ends early therefore keeps reading past its own NUL, and a truncation landing exactly on
  the last character of the `r` or `p` field still parses.

### Cross-cutting notes for the Rust port

1. `errno` is part of the observable contract for every `crypto_pwhash*` and scrypt entry point:
   `EINVAL` for “below minimum / malformed / aliasing / bad alg”, `EFBIG` for “above maximum”,
   `ENOMEM` for scrypt sizing overflows. `argon2_*` and `escrypt_*` do not touch `errno`
   themselves except through `posix_memalign`/`malloc`.
2. Every `crypto_pwhash_argon2*` and scrypt entry point zeroes (`out`) or randomizes
   (`argon2_hash`'s `hash`, `escrypt_r`'s `buf`) the caller's output buffer **before** validating,
   so a rejected call still mutates the output.
3. Rows marked *unreachable* correspond to real C branches that cannot fire on this platform
   (field is `uint32_t`, min is 0, or `pickparams` never fails). They still need to exist in the
   port if the port exposes the same internal functions with wider integer types.
4. `crypto_pwhash_str_alg` with an unknown `alg` **aborts** (`sodium_misuse`) rather than returning
   `-1`; this is the only abort in the argon2 path. `escrypt_PBKDF2_SHA256` with `dkLen` above
   `0x1fffffffe0` is the only abort in the scrypt path.
5. All of `crypto_ipcrypt_*` is total (`void`, never fails). The only data-dependent branches are
   the degenerate-key fixups (8.207, 8.209) and `is_ipv4_mapped` in the PFX variants.
