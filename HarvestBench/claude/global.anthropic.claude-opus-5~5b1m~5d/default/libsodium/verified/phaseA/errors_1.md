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
