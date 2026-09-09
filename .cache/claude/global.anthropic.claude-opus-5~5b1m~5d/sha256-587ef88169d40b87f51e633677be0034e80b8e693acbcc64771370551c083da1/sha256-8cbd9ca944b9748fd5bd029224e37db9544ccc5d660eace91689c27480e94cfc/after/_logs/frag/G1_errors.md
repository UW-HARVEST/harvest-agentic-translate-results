| G1 | sodium_memzero | HAVE_MEMSET_S build: len > 0 and memset_s(pnt,len,0,len) returns non-zero (utils.c:131) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_memcmp | b1 and b2 differ in at least one of the first `len` bytes | -1 |
| G1 | sodium_compare | b1 < b2 interpreted little-endian over `len` bytes | -1 |
| G1 | sodium_compare | b1 > b2 interpreted little-endian over `len` bytes | 1 |
| G1 | sodium_is_zero | any of the first `nlen` bytes is non-zero | 0 (i.e. "not zero") |
| G1 | _sodium_alloc_init | HAVE_ALIGNED_MALLOC build and detected page_size < CANARY_SIZE (16) or page_size < sizeof(size_t) (utils.c:423) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_mlock | build without HAVE_MLOCK and without WINAPI_DESKTOP (utils.c:443) | errno = ENOSYS, returns -1 |
| G1 | sodium_mlock | mlock(addr,len) fails at OS level (len exceeds RLIMIT_MEMLOCK, unmapped addr, len==0 on some OSes) | -1 with errno from mlock (ENOMEM/EPERM/EINVAL) |
| G1 | sodium_mlock | WINAPI_DESKTOP build and VirtualLock() returns 0 | -1 |
| G1 | sodium_munlock | build without HAVE_MLOCK and without WINAPI_DESKTOP (utils.c:461) | errno = ENOSYS, returns -1 |
| G1 | sodium_munlock | munlock(addr,len) fails at OS level (region not locked / not mapped) | -1 with errno from munlock |
| G1 | sodium_munlock | WINAPI_DESKTOP build and VirtualUnlock() returns 0 | -1 |
| G1 | _mprotect_noaccess | build without HAVE_MPROTECT and without WINAPI_DESKTOP (utils.c:474) | errno = ENOSYS, returns -1 |
| G1 | _mprotect_noaccess | mprotect(ptr,size,PROT_NONE) fails (non page-aligned ptr / unmapped range) | -1 with errno from mprotect (EINVAL/ENOMEM/EACCES) |
| G1 | _mprotect_readonly | build without HAVE_MPROTECT and without WINAPI_DESKTOP (utils.c:488) | errno = ENOSYS, returns -1 |
| G1 | _mprotect_readonly | mprotect(ptr,size,PROT_READ) fails at OS level | -1 with errno from mprotect |
| G1 | _mprotect_readwrite | build without HAVE_MPROTECT and without WINAPI_DESKTOP (utils.c:502) | errno = ENOSYS, returns -1 |
| G1 | _mprotect_readwrite | mprotect(ptr,size,PROT_READ\|PROT_WRITE) fails at OS level | -1 with errno from mprotect |
| G1 | _out_of_bounds | reached from sodium_free() on canary corruption | raise(SIGPROT/SIGSEGV/SIGKILL) then abort(); process dies, noreturn |
| G1 | _unprotected_ptr_from_user_ptr | ptr not obtained from sodium_malloc(): (((uintptr_t)ptr - 16) & ~(page_size-1)) <= page_size*2 (utils.c:581) | sodium_misuse -> abort()/SIGABRT |
| G1 | _sodium_malloc (HAVE_ALIGNED_MALLOC) | size >= SIZE_MAX - page_size*4 (utils.c:607) | errno = ENOMEM, returns NULL |
| G1 | _sodium_malloc (HAVE_ALIGNED_MALLOC) | page_size <= sizeof canary (16) or page_size < sizeof(size_t) at call time (utils.c:611) | sodium_misuse -> abort()/SIGABRT |
| G1 | _sodium_malloc (HAVE_ALIGNED_MALLOC) | _alloc_aligned(total_size) fails: mmap returns MAP_FAILED / posix_memalign != 0 / VirtualAlloc NULL (utils.c:617) | NULL (errno from mmap, typically ENOMEM) |
| G1 | _sodium_malloc (HAVE_ALIGNED_MALLOC) | assert(_unprotected_ptr_from_user_ptr(user_ptr) == unprotected_ptr) fails (utils.c:633, NDEBUG unset) | assert -> abort()/SIGABRT |
| G1 | _sodium_malloc (!HAVE_ALIGNED_MALLOC) | malloc(size ? size : 1) returns NULL (heap exhausted) | NULL |
| G1 | sodium_malloc | _sodium_malloc() returned NULL for any of the above reasons (utils.c:644) | NULL |
| G1 | sodium_allocarray | count > 0 and size >= SIZE_MAX / count (multiplication overflow) (utils.c:655) | errno = ENOMEM, returns NULL |
| G1 | sodium_allocarray | count*size passes the overflow check but sodium_malloc() still fails | NULL |
| G1 | sodium_free | leading 16-byte canary at ptr-16 does not match the process canary (buffer underflow) (utils.c:687) | _out_of_bounds() -> SIGSEGV/abort() |
| G1 | sodium_free | !HAVE_PAGE_PROTECTION build: trailing canary at unprotected_ptr+unprotected_size mismatched (buffer overflow) (utils.c:691) | _out_of_bounds() -> SIGSEGV/abort() |
| G1 | sodium_free | ptr not returned by sodium_malloc()/sodium_allocarray() (reaches _unprotected_ptr_from_user_ptr) | sodium_misuse -> abort(), or undefined/SIGSEGV |
| G1 | _sodium_mprotect (!HAVE_PAGE_PROTECTION) | any call to sodium_mprotect_noaccess/readonly/readwrite on a build without page protection (utils.c:707) | errno = ENOSYS, returns -1 |
| G1 | sodium_mprotect_noaccess | ptr not from sodium_malloc() (invalid unprotected_ptr) | sodium_misuse -> abort() (or -1 from mprotect) |
| G1 | sodium_mprotect_readonly | ptr not from sodium_malloc() (invalid unprotected_ptr) | sodium_misuse -> abort() (or -1 from mprotect) |
| G1 | sodium_mprotect_readwrite | ptr not from sodium_malloc() (invalid unprotected_ptr) | sodium_misuse -> abort() (or -1 from mprotect) |
| G1 | sodium_pad | blocksize == 0 (utils.c:755) | -1 |
| G1 | sodium_pad | SIZE_MAX - unpadded_buflen <= xpadlen, i.e. unpadded_buflen near SIZE_MAX so padded length overflows (utils.c:764) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_pad | xpadded_len >= max_buflen, i.e. max_buflen <= unpadded_buflen + (blocksize-1 - unpadded_buflen%blocksize) — buffer too small for padding + barrier byte (utils.c:768) | -1 |
| G1 | sodium_unpad | padded_buflen < blocksize (utils.c:797) | -1 |
| G1 | sodium_unpad | blocksize == 0 (utils.c:797) | -1 |
| G1 | sodium_unpad | no 0x80 barrier byte found scanning the last `blocksize` bytes backwards, or non-zero bytes after the barrier (invalid ISO/IEC 7816-4 padding) -> valid == 0 (utils.c:812) | -1 (and *unpadded_buflen_p still written) |
| G1 | sodium_bin2hex | bin_len >= SIZE_MAX / 2 (codecs.c:23) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_bin2hex | hex_maxlen <= bin_len * 2 (no room for 2 chars/byte + NUL) (codecs.c:23) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_hex2bin | more hex digit pairs present than bin_maxlen bytes: bin_pos >= bin_maxlen with a further valid digit (codecs.c:71) | errno = ERANGE, returns -1, *bin_len = 0 |
| G1 | sodium_hex2bin | odd number of hex digits consumed (state != 0 at end of scan) (codecs.c:84) | errno = EINVAL, returns -1, *bin_len = 0, *hex_end points at the last digit |
| G1 | sodium_hex2bin | hex_end == NULL and scan stopped before hex_len (non-hex char not in `ignore`, or `ignore` char appearing mid-byte i.e. state != 0) (codecs.c:94) | errno = EINVAL, returns -1 |
| G1 | sodium_base64_check_variant | (((unsigned)variant) & ~0x6U) != 0x1U — any variant other than 1/3/5/7 (e.g. 0, 2, 4, 6, 8, -1, 9) (codecs.c:168) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_base64_encoded_len | invalid variant (not 1/3/5/7) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_base64_encoded_len | bin_len / 3 > (SIZE_MAX - 5) / 4 (codecs.c:178) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_bin2base64 | invalid variant (not 1/3/5/7) (codecs.c:197) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_bin2base64 | bin_len/3 > (SIZE_MAX - 5)/4 (codecs.c:199) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_bin2base64 | b64_maxlen <= computed b64_len (output buffer too small, no room for NUL) (codecs.c:211) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_bin2base64 | assert(b64_pos <= b64_len) violated (codecs.c:239, defensive) | assert -> abort()/SIGABRT |
| G1 | sodium_base642bin | invalid variant (not 1/3/5/7) (codecs.c:290) | sodium_misuse -> abort()/SIGABRT |
| G1 | sodium_base642bin | decoded output would exceed bin_maxlen: bin_pos >= bin_maxlen with a further full byte available (codecs.c:310) | errno = ERANGE, returns -1, *bin_len = 0 |
| G1 | sodium_base642bin | acc_len > 4 after the scan, i.e. exactly 1 leftover base64 char in the final group (b64 digit count % 4 == 1) (codecs.c:319) | -1, *bin_len = 0 (errno untouched) |
| G1 | sodium_base642bin | non-canonical encoding: leftover low bits non-zero, (acc & ((1<<acc_len)-1)) != 0 (e.g. "QQ==" -> ok but "QR==" style trailing bits set) (codecs.c:319) | -1, *bin_len = 0 |
| G1 | _sodium_base642bin_skip_padding | PADDED variant (ORIGINAL/URLSAFE) and input ends before all required '=' chars are seen: *b64_pos_p >= b64_len (codecs.c:258) | errno = ERANGE, returns -1 -> sodium_base642bin returns -1, *bin_len = 0 |
| G1 | _sodium_base642bin_skip_padding | PADDED variant and a character in the padding region is neither '=' nor a member of `ignore` (codecs.c:266) | errno = EINVAL, returns -1 -> sodium_base642bin returns -1, *bin_len = 0 |
| G1 | sodium_base642bin | b64_end == NULL and b64_pos != b64_len after decoding (trailing garbage / non-ignored char) (codecs.c:335) | errno = EINVAL, returns -1 |
| G1 | sodium_ip2bin | '%'-zone present and a zone character is not [0-9a-zA-Z._-] (codecs.c:500) | -1 |
| G1 | sodium_ip2bin | '%' present but zone is empty (zone + 1 >= end) (codecs.c:505) | -1 |
| G1 | sodium_ip2bin | '%'-zone present but address contains no ':' (zone on an IPv4 literal) (codecs.c:511) | -1 |
| G1 | sodium_ip2bin | address contains ':' but parse_ipv6() fails (codecs.c:515) | -1 |
| G1 | sodium_ip2bin | no ':' and parse_ipv4() fails (codecs.c:517) | -1 |
| G1 | parse_ipv4 | src >= end — empty string, or ip_len_ == 0, or ip[0] == '\0' (codecs.c:363) | 0 -> sodium_ip2bin returns -1 |
| G1 | parse_ipv4 | an octet has more than 3 digits or its value exceeds 255 (e.g. "256.0.0.1", "0000.0.0.1") (codecs.c:372) | 0 -> -1 |
| G1 | parse_ipv4 | an octet has zero digits (e.g. "1..2.3", ".1.2.3", "1.2.3.") (codecs.c:376) | 0 -> -1 |
| G1 | parse_ipv4 | missing '.' separator after octet i<3, or input ends early (e.g. "1.2.3") (codecs.c:382) | 0 -> -1 |
| G1 | parse_ipv4 | trailing characters after the 4th octet, p != end (e.g. "1.2.3.4.5", "1.2.3.4x") (codecs.c:387) | 0 -> -1 |
| G1 | parse_ipv6 | src >= end — empty string (codecs.c:405) | 0 -> sodium_ip2bin returns -1 |
| G1 | parse_ipv6 | leading single ':' not followed by another ':' (e.g. ":1::", ":") (codecs.c:409) | 0 -> -1 |
| G1 | parse_ipv6 | a second "::" / extra empty group when colonp already set (e.g. "1::2::3") (codecs.c:420) | 0 -> -1 |
| G1 | parse_ipv6 | more than 8 hex groups: tp + 2 > endp at a ':' separator (codecs.c:427) | 0 -> -1 |
| G1 | parse_ipv6 | trailing single ':' at end of input, p >= end after consuming ':' (e.g. "1:2:") (codecs.c:436) | 0 -> -1 |
| G1 | parse_ipv6 | embedded IPv4 tail with no room (tp + 4 > endp) or invalid IPv4 part (e.g. "1:2:3:4:5:6:7:1.2.3.4", "::1.2.3") (codecs.c:442) | 0 -> -1 |
| G1 | parse_ipv6 | invalid hex digit, or more than 4 hex digits in a group (e.g. "::12345", "::g") (codecs.c:450) | 0 -> -1 |
| G1 | parse_ipv6 | final hex group would overflow the 16 bytes (tp + 2 > endp) (codecs.c:459) | 0 -> -1 |
| G1 | parse_ipv6 | "::" present but address already fills all 16 bytes (tp == endp), e.g. "1:2:3:4:5:6:7:8::" (codecs.c:468) | 0 -> -1 |
| G1 | parse_ipv6 | no "::" and fewer than 8 groups, tp != endp (e.g. "1:2:3") (codecs.c:475) | 0 -> -1 |
| G1 | sodium_bin2ip | ip_maxlen <= 2 (codecs.c:561) | NULL |
| G1 | sodium_bin2ip | IPv4-mapped input (bin[0..11] == 00..00ffff) and formatted length >= ip_maxlen (codecs.c:572) | NULL |
| G1 | sodium_bin2ip | IPv6 input and formatted length >= ip_maxlen (codecs.c:617) | NULL |
| G1 | sodium_init | sodium_crit_enter() fails (pthread_mutex_lock error / _sodium_crit_init failure on Win32) (core.c:30) | -1 |
| G1 | sodium_init | already initialized and sodium_crit_leave() fails (core.c:34) | -1 |
| G1 | sodium_init | first-time init succeeded but sodium_crit_leave() fails (core.c:52) | -1 |
| G1 | sodium_crit_enter | assert(locked == 0) — re-entrant/nested sodium_crit_enter() (core.c:91 / core.c:122) | assert -> abort()/SIGABRT |
| G1 | sodium_crit_leave | called while locked == 0 (leave without a matching enter) (core.c:100 / core.c:131) | errno = EPERM, returns -1 |
| G1 | sodium_crit_leave | pthread build: pthread_mutex_unlock() returns non-zero (core.c:139) | non-zero errno value from pthread_mutex_unlock |
| G1 | _sodium_crit_init (Win32) | InterlockedCompareExchange returns a status other than 0 or 2, or InterlockedExchange race (core.c:76, core.c:80) | -1 -> sodium_crit_enter returns -1 |
| G1 | sodium_misuse | invoked from any misuse site; optional handler (set via sodium_set_misuse_handler) is called first if it returns | abort()/SIGABRT unconditionally (noreturn) |
| G1 | sodium_set_misuse_handler | sodium_crit_enter() fails (core.c:211) | -1 |
| G1 | sodium_set_misuse_handler | sodium_crit_leave() fails after storing the handler (core.c:215) | -1 |
| G1 | _sodium_runtime_arm_cpu_features | built for a non-ARM target (__ARM_ARCH undefined) (runtime.c:67) | -1 (has_neon = has_armcrypto = 0) |
| G1 | _sodium_runtime_intel_cpu_features | CPUID leaf 0 returns EAX == 0 (no/unsupported CPUID, non-x86 target) (runtime.c:208) | -1 (all x86 feature flags left 0) |
| G1 | _sodium_runtime_get_cpu_features | both the ARM and the Intel probe return -1 (neither ARM nor x86) | -1 (but _cpu_features.initialized still set to 1) |
| G1 | crypto_verify_16 | x and y differ in at least one of the 16 bytes | -1 |
| G1 | crypto_verify_32 | x and y differ in at least one of the 32 bytes | -1 |
| G1 | crypto_verify_64 | x and y differ in at least one of the 64 bytes | -1 |
