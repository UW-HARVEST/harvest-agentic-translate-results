| sodium-1 | sodium_memcmp | len=0 (incl. NULL/NULL), 1,2,7,8,15,16,17,31,32,33,64,100; equal, same-pointer, and differing at every byte position × delta {1,0x0f,0x80,0xff} | [x] |
| sodium-2 | sodium_compare | len=1 exhaustive (all 65536 byte pairs), checked against a reference little-endian bignum compare; result in {-1,0,1} | [x] |
| sodium-3 | sodium_compare | len=2 exhaustive over a 16-value boundary grid per byte (65536 pairs) + 4000 random pairs | [x] |
| sodium-4 | sodium_compare | len=0 (NULL/NULL), 3, 8, 16, 32; random pairs, pairs sharing a random high-order common prefix, and equal buffers | [x] |
| sodium-5 | sodium_is_zero | len=0 (NULL), 1,2,8,16,31,32,33,64; all-zero, single non-zero byte {1,0x80,0xff} at every position, random | [x] |
| sodium-6 | sodium_increment | nlen=0,1,2,3,7,8,9,11,12,13,16,23,24,25,32,64 (incl. the lengths the `HAVE_AMD64_ASM` variant special-cases); all-zero, all-0xff, 0xff-prefix of every length, random, plus 600 successive increments | [x] |
| sodium-7 | sodium_add | len=0,1,2,3,8,12,16,24,32,64,65; zero+zero, 0xff+0xff, full carry chains, 0xff-prefix chains, 30 random pairs, and `a == b` aliasing | [x] |
| sodium-8 | sodium_sub | len=0,1,2,3,8,12,16,24,32,64,65 (incl. 64, the `HAVE_AMD64_ASM` case); full borrow chains, 0-1 underflow, 30 random pairs, and `a == b` aliasing | [x] |
| sodium-9 | sodium_memzero | len=0,1,2,7,8,15,16,31,32,64,1000 × start offset 0,1,3 inside a larger buffer; `(NULL, 0)` | [x] |
| sodium-10 | sodium_stackzero | len=0,1,64,4096,100000 (empty body in this build: neither HAVE_C_VARARRAYS nor HAVE_ALLOCA) | [x] |
| sodium-11 | sodium_pad | blocksize=1,2,3,15,16,17,32,64 (power-of-two `&` branch and non-power-of-two `%` branch) × unpadded_buflen 0..2*blocksize+1 × max_buflen {sufficient, n+1, n, 0, n+blocksize}; whole buffer + canary compared | [x] |
| sodium-12 | sodium_pad | padded_buflen_p = NULL vs non-NULL | [x] |
| sodium-13 | sodium_unpad | round-trip of every sodium_pad result with the correct blocksize, and with blocksize {1, bs+1, bs-1, 0} | [x] |
| sodium-14 | sodium_unpad | padding corrupted at every position in the last block (0x80 barrier flipped); return value and *unpadded_buflen_p compared | [x] |
| sodium-15 | sodium_unpad | blocksize=1,2,16,17,64 × 200 fully random buffers each, padded_buflen bs..4*bs | [x] |
| sodium-16 | sodium_malloc | size=0,1,2,15,16,17,63,64,4095,4096,100000; buffer non-NULL, full contents compared (must be 0xdb), then written and re-read | [x] |
| sodium-17 | sodium_allocarray | (count,size) = (0,0),(0,32),(1,0),(1,32),(7,13),(100,64); contents compared | [x] |
| sodium-18 | sodium_free | free of every sodium_malloc/sodium_allocarray result, plus sodium_free(NULL) | [x] |
| sodium-19 | sodium_mlock, sodium_munlock | on every allocation size above and on (NULL, 0); return value + errno compared, and munlock's zeroing of the buffer verified | [x] |
| sodium-20 | sodium_mprotect_noaccess, sodium_mprotect_readonly, sodium_mprotect_readwrite | on a live sodium_malloc pointer and on NULL; return value + errno compared | [x] |
| sodium-21 | _sodium_alloc_init | called 3× (refills the canary via randombytes_buf); return value compared | [x] |
| sodium-22 | sodium_bin2hex | bin_len 0..64 × {all-zero, all-0xff, counting, all 256 byte values in the last position, 10 random} × hex_maxlen = 2n+{1,2,8}; return pointer, full buffer + canary, and only 2n+1 bytes written | [x] |
| sodium-23 | sodium_hex2bin | 35 hand-written inputs (valid, uppercase/lowercase/mixed, separators, non-hex chars at every class boundary `@ / : \` G z`, embedded NUL, odd length) × hex_len {n, n-1} × ignore {NULL, ": \n", "", "xyz"} × bin_maxlen {0,1,2,3,n/2,n+4} × bin_len {NULL, ptr} × hex_end {NULL, ptr} | [x] |
| sodium-24 | sodium_hex2bin | 3000 random strings over `0-9a-fA-F: \n@Gz/` × random ignore/bin_maxlen/out-pointer combinations; ret, errno, bin buffer, *bin_len and hex_end offset all compared | [x] |
| sodium-25 | sodium_hex2bin | round-trip of sodium_bin2hex output for n = 0..48 with exact-fit bin_maxlen | [x] |
| sodium-26 | sodium_base64_encoded_len | variant ORIGINAL / ORIGINAL_NO_PADDING / URLSAFE / URLSAFE_NO_PADDING × bin_len 0..300 and 1000, 2^20, 2^40+1, SIZE_MAX/8, 3*((SIZE_MAX-5)/4) | [x] |
| sodium-27 | sodium_bin2base64 | all 4 variants × bin_len 0..64 (all three `bin_len % 3` remainders) × {all-zero, all-0xff, patterned, 8 random} × b64_maxlen = encoded_len + {0,1,5,32} (exercises the trailing-NUL fill loop); full buffer + canary, alphabet and padding-length invariants | [x] |
| sodium-28 | sodium_base642bin | all 4 variants × round-trip of sodium_bin2base64 output for n=0..48 × bin_maxlen {n, n+1, n-1, 0} × ignore {NULL, " \n", "", "="} × bin_len {NULL,ptr} × b64_end {NULL,ptr} | [x] |
| sodium-29 | sodium_base642bin | 45 hand-written inputs (valid, 1/2/3-char tails, wrong padding, over-padding, urlsafe chars in the standard alphabet and vice versa, invalid chars, embedded NUL, high-bit bytes) × all 4 variants × b64_len {n, n-1} × ignore × bin_maxlen {0,1,2,3,8} × out-pointer combinations | [x] |
| sodium-30 | sodium_base642bin | 6000 random strings over `ABCZaz09+/-_=! \n\0` × random variant/ignore/bin_maxlen/out-pointer; ret, errno, bin buffer, *bin_len, b64_end offset compared | [x] |
| sodium-31 | sodium_ip2bin | 66 hand-written IPv4/IPv6/zone-id inputs (valid, malformed, truncated, embedded NUL, `::`-forms, embedded IPv4, uppercase hex) × ip_len_ {n, n+1, n-1, 0, 3} | [x] |
| sodium-32 | sodium_ip2bin | 20000 random strings over `0-9a-fA-F.:%_-gz ` (length 0..23); return value and the 16-byte output + canary compared | [x] |
| sodium-33 | sodium_bin2ip | IPv4-mapped inputs, 12 near-miss mapped prefixes, all single-non-zero-word patterns, every zero-run start×length, equal-length zero-run tie-breaks, 1000 random/sparse patterns × ip_maxlen 0,1,2,3,4,5,8,16,40,46,64; NULL-ness, return pointer, full buffer + canary | [x] |
| sodium-34 | sodium_bin2ip + sodium_ip2bin | every bin2ip output re-parsed by ip2bin and required to reproduce the original 16 bytes | [x] |
| sodium-35 | sodium_init | called 3 more times after the harness' first call; must return 1 (already-initialized path) | [x] |
| sodium-36 | sodium_crit_enter, sodium_crit_leave | 3 balanced enter/leave pairs plus one unbalanced leave (no-op versions: no HAVE_PTHREAD / HAVE_ATOMIC_OPS / _WIN32) | [x] |
| sodium-37 | sodium_set_misuse_handler | handler = NULL, handler = fn, handler = NULL again | [x] |
| sodium-38 | sodium_set_misuse_handler + sodium_misuse | handler installed in a forked child, then a misuse triggered: the handler is called (child exits 42) before abort() | [x] |
| sodium-39 | sodium_runtime_has_neon/armcrypto/sse2/sse3/ssse3/sse41/avx/avx2/avx512f/pclmul/aesni/rdrand | all 12 queried before and after re-running detection | [x] |
| sodium-40 | _sodium_runtime_get_cpu_features | called 3× (idempotent); return value compared, and the has_* answers re-checked afterwards | [x] |
| sodium-41 | sodium_version_string, sodium_library_version_major, sodium_library_version_minor, sodium_library_minimal | all four compared | [x] |
| sodium-42 | randombytes_set_implementation, randombytes_implementation_name, randombytes_random, randombytes_buf, randombytes, randombytes_stir, randombytes_uniform, randombytes_seedbytes, randombytes_close | a deterministic test implementation is installed in both libraries (independent per-library counters) and a 700-entry transcript is compared entry-by-entry: 64 `random()` draws, `uniform()` × 14 upper bounds × 8 draws, `buf`/`randombytes` at sizes 0,1,2,15,16,17,63,64,100 (full buffer + canary), `stir`, `close` | [x] |
| sodium-43 | randombytes_uniform | implementation with `uniform == NULL`: exercises the `(1+~ub) % ub` rejection-sampling fallback deterministically for upper_bound 0,1,2,3,5,16,17,255,256,1000,2^31,2^31+1,0xfffffffe,0xffffffff | [x] |
| sodium-44 | randombytes_uniform | implementation with `uniform != NULL`: exercises the delegation branch for the same 14 upper bounds | [x] |
| sodium-45 | randombytes_stir, randombytes_close | implementation with `stir == NULL` and `close == NULL`: both must be no-ops (stir) / return 0 (close) | [x] |
| sodium-46 | randombytes_close, randombytes_implementation_name | implementation pointer set to NULL: close() returns 0, and the next call re-installs the default (`sysrandom`) implementation via randombytes_init_if_needed() | [x] |
| sodium-47 | randombytes_buf_deterministic, randombytes_seedbytes | 40 seeds (all-zero, all-0xff, 38 random) × size 0,1,15,16,17,31,32,33,63,64,65,127,128,129,1000; **bytes** compared plus canary | [x] |
| sodium-48 | randombytes_sysrandom_implementation (exported data object) | every function pointer called directly: implementation_name ("sysrandom"), uniform == NULL, close() before/after stir(), stir(), random() ×512, buf() at 1,16,31,32,33,256,257,1000 (crosses the 256-byte getrandom chunk boundary), close() ×2 more | [x] |
| sodium-49 | randombytes_internal_implementation (exported data object) | same sequence; implementation_name ("internal"), uniform == NULL, close() before stir returns -1 in both, after stir returns 0 | [x] |
| sodium-50 | randombytes_set_implementation + public API | both exported implementations installed and driven through randombytes_implementation_name/stir/close/seedbytes/uniform/buf/randombytes, including size 0 (buffer must be untouched) | [x] |
| sodium-51 | crypto_ipcrypt_bytes, _keybytes, _nd_keybytes, _nd_tweakbytes, _nd_inputbytes, _nd_outputbytes, _ndx_keybytes, _ndx_tweakbytes, _ndx_inputbytes, _ndx_outputbytes, _pfx_keybytes, _pfx_bytes | all 12 constants compared (and pinned to 16/16/16/8/16/24/32/16/16/32/32/16) | [x] |
| sodium-52 | crypto_ipcrypt_encrypt, crypto_ipcrypt_decrypt | 26 keys (all-zero, all-0xff, 24 random) × 46 inputs (zero, 0xff, ::1, 5 IPv4-mapped, 12 near-miss mapped prefixes, 24 random); output bytes + canary, decrypt round-trip, and `out == in` aliasing | [x] |
| sodium-53 | crypto_ipcrypt_nd_encrypt, crypto_ipcrypt_nd_decrypt | 18 × 16-byte keys × 10 × 8-byte tweaks (incl. all-zero, all-0xff) × 20 inputs; 24-byte output + canary, tweak prefix, round-trip | [x] |
| sodium-54 | crypto_ipcrypt_ndx_encrypt, crypto_ipcrypt_ndx_decrypt | 20 × 32-byte keys × 8 × 16-byte tweaks × 16 inputs; 32-byte output + canary, tweak prefix, round-trip | [x] |
| sodium-55 | crypto_ipcrypt_ndx_encrypt/decrypt, crypto_ipcrypt_pfx_encrypt/decrypt | keys whose two 16-byte halves are identical (all-zero, all-0xff, all-0x5a, counting): the two key schedules coincide, so `d == 0` and the `k[i] ^ 0x5a` re-derivation branch is taken | [x] |
| sodium-56 | crypto_ipcrypt_pfx_encrypt, crypto_ipcrypt_pfx_decrypt | 20 × 32-byte keys × 46 inputs covering both `prefix_start = 0` (generic) and `prefix_start = 96` (IPv4-mapped) loops; output bytes + canary, mapped-prefix preservation, round-trip | [x] |
| sodium-57 | crypto_ipcrypt_keygen, crypto_ipcrypt_nd_keygen, crypto_ipcrypt_ndx_keygen, crypto_ipcrypt_pfx_keygen | with the deterministic test RNG installed in both libraries: output **bytes** compared (16/16/32/32) + canary; then repeated with the real RNG to confirm the written length | [x] |
| sodium-58 | ipcrypt_soft_implementation (exported data object) | all 8 function pointers (encrypt, decrypt, nd_encrypt, nd_decrypt, ndx_encrypt, ndx_decrypt, pfx_encrypt, pfx_decrypt) called directly with 8 random key sets × 10 inputs; outputs + canary compared and round-tripped | [x] |
| sodium-59 | _crypto_ipcrypt_pick_best_implementation | called 3× (always selects the soft backend in this build); return value compared, and crypto_ipcrypt_encrypt re-verified afterwards | [x] |
| sodium-60 | sodium_ip2bin + crypto_ipcrypt_pfx_encrypt/decrypt + sodium_bin2ip | end-to-end: 10 IPv4/IPv6/zone-id/mapped strings × 8 random 32-byte keys, parse → encrypt → format → decrypt | [x] |
