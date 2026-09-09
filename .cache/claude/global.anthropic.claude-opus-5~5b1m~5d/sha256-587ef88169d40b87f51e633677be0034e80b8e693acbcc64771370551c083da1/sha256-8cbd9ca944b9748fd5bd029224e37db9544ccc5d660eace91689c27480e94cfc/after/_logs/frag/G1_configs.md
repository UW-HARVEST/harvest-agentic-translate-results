| G1 | sodium_init | first call in the process (initialized == 0): full CPU-feature probe + randombytes_stir + alloc init + impl picking | [ ] |
| G1 | sodium_init | second and subsequent calls (initialized != 0): idempotency path returning 1 | [ ] |
| G1 | sodium_init, sodium_set_misuse_handler | sodium_set_misuse_handler(NULL) then sodium_init(), handler stays NULL | [ ] |
| G1 | sodium_set_misuse_handler | set a non-NULL handler, then replace it with a second non-NULL handler, then reset to NULL | [ ] |
| G1 | sodium_crit_enter, sodium_crit_leave | balanced enter/leave pair (locked 0 -> 1 -> 0) | [ ] |
| G1 | sodium_version_string | no arguments; must equal "1.0.23" | [ ] |
| G1 | sodium_library_version_major | no arguments; must equal 30 | [ ] |
| G1 | sodium_library_version_minor | no arguments; must equal 0 | [ ] |
| G1 | sodium_library_minimal | non-minimal build (SODIUM_LIBRARY_MINIMAL undefined) -> 0 | [ ] |
| G1 | sodium_library_minimal | minimal build (SODIUM_LIBRARY_MINIMAL defined) -> 1 | [ ] |
| G1 | sodium_runtime_has_neon | after sodium_init() on x86-64 (expect 0); getter is a pure struct read, must be stable across repeated calls | [ ] |
| G1 | sodium_runtime_has_armcrypto | after sodium_init(); 0 unless has_neon and ARM crypto extensions detected | [ ] |
| G1 | sodium_runtime_has_sse2 | after sodium_init(); HAVE_EMMINTRIN_H build vs not (compile-time forced 0) | [ ] |
| G1 | sodium_runtime_has_sse3 | after sodium_init(); HAVE_PMMINTRIN_H build vs not | [ ] |
| G1 | sodium_runtime_has_ssse3 | after sodium_init(); HAVE_TMMINTRIN_H build vs not | [ ] |
| G1 | sodium_runtime_has_sse41 | after sodium_init(); HAVE_SMMINTRIN_H build vs not | [ ] |
| G1 | sodium_runtime_has_avx | after sodium_init(); requires AVX+XSAVE+OSXSAVE in CPUID and XCR0 SSE\|AVX bits | [ ] |
| G1 | sodium_runtime_has_avx2 | after sodium_init(); only probed (CPUID leaf 7 EBX bit 5) when has_avx == 1 | [ ] |
| G1 | sodium_runtime_has_avx512f | after sodium_init(); only probed when has_avx2 == 1, plus XCR0 OPMASK\|ZMM_HI256\|HI16_ZMM | [ ] |
| G1 | sodium_runtime_has_pclmul | after sodium_init(); HAVE_WMMINTRIN_H build vs not | [ ] |
| G1 | sodium_runtime_has_aesni | after sodium_init(); HAVE_WMMINTRIN_H build vs not | [ ] |
| G1 | sodium_runtime_has_rdrand | after sodium_init(); HAVE_RDRAND build vs not | [ ] |
| G1 | all sodium_runtime_has_* | called BEFORE sodium_init() (_cpu_features zeroed, initialized == 0) -> all return 0 | [ ] |
| G1 | crypto_verify_16_bytes / _32_bytes / _64_bytes | constant getters; must return 16 / 32 / 64 | [ ] |
| G1 | crypto_verify_16 | x == y (all 16 bytes equal), including all-zero and all-0xff buffers -> 0 | [ ] |
| G1 | crypto_verify_16 | x and y differ at exactly one byte position i, for each i in 0..15, differing by a single bit -> -1 | [ ] |
| G1 | crypto_verify_32 | x == y over all 32 bytes -> 0 | [ ] |
| G1 | crypto_verify_32 | x and y differ at exactly one byte position i, for each i in 0..31 (crosses the 16-byte SSE2 block boundary) -> -1 | [ ] |
| G1 | crypto_verify_64 | x == y over all 64 bytes -> 0 | [ ] |
| G1 | crypto_verify_64 | x and y differ at exactly one byte position i, for each i in 0..63 (exercises all 4 SSE2 blocks) -> -1 | [ ] |
| G1 | crypto_verify_16/32/64 | build with HAVE_EMMINTRIN_H && __SSE2__ (movemask path) vs the portable uint16 optblocker path | [ ] |
| G1 | sodium_memzero | len == 0 (must not touch the buffer) | [ ] |
| G1 | sodium_memzero | len == 1, len == 16, len == page-sized buffer of non-zero bytes -> all bytes zeroed | [ ] |
| G1 | sodium_stackzero | len == 0 and len == small value (e.g. 512); observable only as a no-crash/no-return-value call | [ ] |
| G1 | sodium_memcmp | len == 0 (vacuously equal) -> 0 | [ ] |
| G1 | sodium_memcmp | len == 1 / 16 / 32 / 64, buffers identical -> 0 | [ ] |
| G1 | sodium_memcmp | same pointer passed as both b1_ and b2_ -> 0 | [ ] |
| G1 | sodium_compare | len == 0 -> 0 (eq stays 1) | [ ] |
| G1 | sodium_compare | len == 1, b1 == b2 -> 0; b1 < b2 -> -1; b1 > b2 -> 1 | [ ] |
| G1 | sodium_compare | len == 8/16/32, difference only in the most significant (last) byte -> decided by that byte | [ ] |
| G1 | sodium_compare | len == 8/16/32, difference only in the least significant (byte 0), all higher bytes equal -> decided by byte 0 | [ ] |
| G1 | sodium_compare | little-endian ordering: b1 = {0xff,0x00}, b2 = {0x00,0x01} -> b1 < b2 (-1) | [ ] |
| G1 | sodium_is_zero | nlen == 0 -> 1 | [ ] |
| G1 | sodium_is_zero | nlen == 1/16/32, all bytes zero -> 1 | [ ] |
| G1 | sodium_is_zero | nlen == 32, single non-zero byte at each position i in 0..31 -> 0 | [ ] |
| G1 | sodium_increment | nlen == 0 (no-op) | [ ] |
| G1 | sodium_increment | nlen == 1, value 0x00 -> 0x01 (no carry) | [ ] |
| G1 | sodium_increment | nlen == 1, value 0xff -> 0x00 (carry dropped, wrap) | [ ] |
| G1 | sodium_increment | nlen == 8 (HAVE_AMD64_ASM `incq` fast path vs portable loop), value 0x00..00 and value 0xff..ff | [ ] |
| G1 | sodium_increment | nlen == 12 (HAVE_AMD64_ASM adcq+adcl fast path), carry propagating from byte 7 into byte 8 | [ ] |
| G1 | sodium_increment | nlen == 24 (HAVE_AMD64_ASM 3x adcq fast path), carry propagating from byte 15 into byte 16 | [ ] |
| G1 | sodium_increment | nlen == 16/32 (no asm fast path, portable loop), carry cascading across every byte (all 0xff) | [ ] |
| G1 | sodium_add | len == 0 (no-op) | [ ] |
| G1 | sodium_add | len == 1, no carry (0x01 + 0x01) and with wrap (0xff + 0x01 -> 0x00) | [ ] |
| G1 | sodium_add | len == 8 (AMD64 asm `addq` path vs portable), operands producing a carry out of byte 7 | [ ] |
| G1 | sodium_add | len == 12 (AMD64 asm addq+adcl path), carry crossing the 8-byte boundary | [ ] |
| G1 | sodium_add | len == 24 (AMD64 asm addq+2x adcq path), carry crossing both 8-byte boundaries | [ ] |
| G1 | sodium_add | len == 16/32/64 (portable loop), full carry chain (all-0xff + 1) | [ ] |
| G1 | sodium_add | a == b (aliased pointers), len == 32 (doubling) | [ ] |
| G1 | sodium_sub | len == 0 (no-op) | [ ] |
| G1 | sodium_sub | len == 1, no borrow (0x02 - 0x01) and with borrow (0x00 - 0x01 -> 0xff) | [ ] |
| G1 | sodium_sub | len == 64 (AMD64 asm 8x sbbq fast path vs portable loop), borrow chain across all 8 quadwords | [ ] |
| G1 | sodium_sub | len == 8/12/16/24/32 (portable loop only, no asm path), borrow crossing byte boundaries | [ ] |
| G1 | sodium_sub | a == b (aliased pointers), len == 32 -> all zero | [ ] |
| G1 | sodium_mlock, sodium_munlock | freshly mmap'ed/page-aligned buffer, len == page size (success path); then munlock the same region | [ ] |
| G1 | sodium_mlock, sodium_munlock | non page-aligned address inside a heap allocation, small len | [ ] |
| G1 | sodium_malloc | size == 0 (still returns a usable pointer; 0-byte user region, guard pages installed) | [ ] |
| G1 | sodium_malloc | size == 1 (user region filled with GARBAGE_VALUE 0xdb) | [ ] |
| G1 | sodium_malloc | size == 16 (== sizeof canary) | [ ] |
| G1 | sodium_malloc | size exactly one page (page_round is exact, no padding before canary) | [ ] |
| G1 | sodium_malloc | size not a multiple of the page size (padding inserted before the canary; returned ptr unaligned) | [ ] |
| G1 | sodium_malloc | size spanning multiple pages (e.g. 3*page_size + 1) | [ ] |
| G1 | sodium_malloc, sodium_free | allocate, write the whole user region, free (canary intact) | [ ] |
| G1 | sodium_free | ptr == NULL -> silent no-op | [ ] |
| G1 | sodium_allocarray | count == 0, size == 0 -> sodium_malloc(0) | [ ] |
| G1 | sodium_allocarray | count == 0, size == SIZE_MAX (overflow check skipped because count == 0) | [ ] |
| G1 | sodium_allocarray | count == 1, size == 32 (normal) | [ ] |
| G1 | sodium_allocarray | count == 32, size == 32 (product well below SIZE_MAX) | [ ] |
| G1 | sodium_mprotect_readonly, sodium_mprotect_readwrite | sodium_malloc'ed region: readwrite -> write -> readonly -> read -> readwrite -> write -> free | [ ] |
| G1 | sodium_mprotect_noaccess, sodium_mprotect_readwrite | sodium_malloc'ed region: noaccess then readwrite then free (round-trip restores writability) | [ ] |
| G1 | sodium_mprotect_readonly, sodium_free | region left read-only at free() time (sodium_free must re-mprotect readwrite first) | [ ] |
| G1 | sodium_mprotect_noaccess | called twice in a row on the same region (idempotent) | [ ] |
| G1 | sodium_bin2hex | bin_len == 0, hex_maxlen == 1 -> empty NUL-terminated string | [ ] |
| G1 | sodium_bin2hex | bin_len == 1, hex_maxlen == 3 (minimum valid), byte with both nibbles < 10 (e.g. 0x09) | [ ] |
| G1 | sodium_bin2hex | bin_len == 1, byte with both nibbles >= 10 (e.g. 0xff) -> lowercase 'a'-'f' branch of the constant-time digit | [ ] |
| G1 | sodium_bin2hex | bin_len == 1, mixed nibbles (0x0a, 0xa0, 0x10) -> exercises high/low nibble branches independently | [ ] |
| G1 | sodium_bin2hex | bin_len == 32, hex_maxlen > 65 (extra slack; only hex[bin_len*2] is NUL-terminated, rest untouched) | [ ] |
| G1 | sodium_hex2bin | hex_len == 0, ignore == NULL, hex_end == NULL, bin_len != NULL -> 0, *bin_len == 0 | [ ] |
| G1 | sodium_hex2bin | even-length all-lowercase hex, ignore == NULL, hex_end == NULL, bin_maxlen exactly hex_len/2 | [ ] |
| G1 | sodium_hex2bin | even-length all-uppercase hex (exercises the `c & ~32U` alpha branch) | [ ] |
| G1 | sodium_hex2bin | mixed-case hex plus digits (all three character classes) | [ ] |
| G1 | sodium_hex2bin | bin_maxlen strictly greater than needed -> 0, *bin_len == hex_len/2 | [ ] |
| G1 | sodium_hex2bin | ignore == ": " with separators between every byte pair ("de:ad:be:ef"), hex_end == NULL | [ ] |
| G1 | sodium_hex2bin | ignore != NULL with ignored chars leading and trailing the string | [ ] |
| G1 | sodium_hex2bin | ignore != NULL, ignore chars only skipped when state == 0 (between complete bytes, never mid-byte) | [ ] |
| G1 | sodium_hex2bin | hex_end != NULL and trailing non-hex garbage -> returns 0 with *hex_end pointing at the garbage | [ ] |
| G1 | sodium_hex2bin | bin_len == NULL (caller does not want the length) with an otherwise valid string | [ ] |
| G1 | sodium_hex2bin | hex_end != NULL and bin_len != NULL and ignore != NULL, all three set simultaneously | [ ] |
| G1 | sodium_base64_encoded_len | variant ORIGINAL (1), bin_len % 3 == 0 / 1 / 2, plus bin_len == 0 | [ ] |
| G1 | sodium_base64_encoded_len | variant ORIGINAL_NO_PADDING (3), bin_len % 3 == 0 / 1 / 2 | [ ] |
| G1 | sodium_base64_encoded_len | variant URLSAFE (5), bin_len % 3 == 0 / 1 / 2 | [ ] |
| G1 | sodium_base64_encoded_len | variant URLSAFE_NO_PADDING (7), bin_len % 3 == 0 / 1 / 2 | [ ] |
| G1 | sodium_base64_encoded_len, sodium_base64_ENCODED_LEN | function result must equal the macro for the same (bin_len, variant) across all 4 variants | [ ] |
| G1 | sodium_bin2base64 | ORIGINAL (1), bin_len == 0, b64_maxlen == 1 -> empty string | [ ] |
| G1 | sodium_bin2base64 | ORIGINAL (1), bin_len % 3 == 0 (e.g. 3, 6, 30) -> no '=' padding emitted | [ ] |
| G1 | sodium_bin2base64 | ORIGINAL (1), bin_len % 3 == 1 (e.g. 1, 4) -> 2 chars + "==" | [ ] |
| G1 | sodium_bin2base64 | ORIGINAL (1), bin_len % 3 == 2 (e.g. 2, 5) -> 3 chars + "=" | [ ] |
| G1 | sodium_bin2base64 | ORIGINAL_NO_PADDING (3), bin_len % 3 == 0 / 1 / 2 -> b64_len = nibbles*4 + 2 + (rem>>1), no '=' | [ ] |
| G1 | sodium_bin2base64 | URLSAFE (5), bin_len % 3 == 0 / 1 / 2 -> '-'/'_' alphabet with '=' padding | [ ] |
| G1 | sodium_bin2base64 | URLSAFE_NO_PADDING (7), bin_len % 3 == 0 / 1 / 2 -> '-'/'_' alphabet, no padding | [ ] |
| G1 | sodium_bin2base64 | b64_maxlen == b64_len + 1 exactly (minimum valid) for each variant | [ ] |
| G1 | sodium_bin2base64 | b64_maxlen > b64_len + 1 -> the whole remaining buffer is NUL-filled by the do/while loop | [ ] |
| G1 | sodium_bin2base64 | input bytes chosen so all 64 alphabet symbols are produced (indices 0..63), separately for standard ('+','/') and urlsafe ('-','_') | [ ] |
| G1 | sodium_base642bin | ORIGINAL (1), b64_len == 0, ignore == NULL, b64_end == NULL -> 0, *bin_len == 0 | [ ] |
| G1 | sodium_base642bin | ORIGINAL (1), b64 digit count % 4 == 0 (no padding needed), acc_len == 0 | [ ] |
| G1 | sodium_base642bin | ORIGINAL (1), 2 leftover chars + "==" (acc_len == 4, skip_padding called with 2) | [ ] |
| G1 | sodium_base642bin | ORIGINAL (1), 3 leftover chars + "=" (acc_len == 2, skip_padding called with 1) | [ ] |
| G1 | sodium_base642bin | ORIGINAL_NO_PADDING (3), digit count % 4 == 0 / 2 / 3, no '=' present, skip_padding not called | [ ] |
| G1 | sodium_base642bin | URLSAFE (5), '-' and '_' digits, with correct '=' padding for %4 == 2 and %4 == 3 | [ ] |
| G1 | sodium_base642bin | URLSAFE_NO_PADDING (7), '-'/'_' digits, digit count % 4 == 0 / 2 / 3 | [ ] |
| G1 | sodium_base642bin | ignore == " \n\r" with whitespace between digits, inside the padding region, and after the padding (trailing-ignore skip loop) | [ ] |
| G1 | sodium_base642bin | ignore == NULL, exact-length input, b64_end == NULL, bin_len == NULL | [ ] |
| G1 | sodium_base642bin | b64_end != NULL, trailing non-base64 data -> returns 0 with *b64_end at the first unconsumed char | [ ] |
| G1 | sodium_base642bin | bin_maxlen exactly the decoded length vs strictly larger | [ ] |
| G1 | sodium_base642bin | 'A' input digit (value 0) — special-cased by the `EQ(x,0) & (EQ(c,'A') ^ 0xFF)` sentinel in b64_char_to_byte / b64_urlsafe_char_to_byte | [ ] |
| G1 | sodium_bin2base64 + sodium_base642bin | round-trip for each of the 4 variants x bin_len 0..8 (covers every %3 residue and every %4 output residue) | [ ] |
| G1 | sodium_pad | blocksize a power of two (e.g. 16) — `blocksize & (blocksize-1) == 0` masking branch | [ ] |
| G1 | sodium_pad | blocksize not a power of two (e.g. 3, 10, 17) — modulo branch | [ ] |
| G1 | sodium_pad | blocksize == 1 (xpadlen always 0, one 0x80 barrier byte appended) | [ ] |
| G1 | sodium_pad | unpadded_buflen == 0, blocksize == 16 -> padded length 16 | [ ] |
| G1 | sodium_pad | unpadded_buflen % blocksize == 0 and non-zero -> a full extra block of padding is added | [ ] |
| G1 | sodium_pad | unpadded_buflen % blocksize == blocksize - 1 -> exactly 1 byte (the 0x80 barrier) added | [ ] |
| G1 | sodium_pad | unpadded_buflen % blocksize in the middle (e.g. 5 mod 16) -> barrier + zero bytes | [ ] |
| G1 | sodium_pad | max_buflen exactly xpadded_len + 1 (minimum acceptable buffer) | [ ] |
| G1 | sodium_pad | max_buflen much larger than needed (bytes past the padded region untouched) | [ ] |
| G1 | sodium_pad | padded_buflen_p == NULL (caller discards the length) with an otherwise valid call | [ ] |
| G1 | sodium_unpad | blocksize == 1, padded_buflen == 1, buf[0] == 0x80 -> *unpadded_buflen_p == 0 | [ ] |
| G1 | sodium_unpad | padded_buflen == blocksize exactly (minimum accepted) | [ ] |
| G1 | sodium_unpad | padding of exactly 1 byte (last byte 0x80) | [ ] |
| G1 | sodium_unpad | padding of blocksize bytes (0x80 followed by blocksize-1 zeros) — maximum pad_len | [ ] |
| G1 | sodium_unpad | mid-range pad_len (barrier several bytes before the end) | [ ] |
| G1 | sodium_unpad | data byte immediately before the barrier equal to 0x80 (must not be mistaken for the barrier — first barrier from the end wins) | [ ] |
| G1 | sodium_pad + sodium_unpad | round-trip over blocksize in {1,2,3,16,17,64} x unpadded_buflen in {0,1,blocksize-1,blocksize,blocksize+1} | [ ] |
| G1 | sodium_ip2bin | valid IPv4 dotted quad, e.g. "0.0.0.0", "255.255.255.255", "1.2.3.4" -> IPv4-mapped 16 bytes (00*10, ff, ff, quad) | [ ] |
| G1 | sodium_ip2bin | IPv4 with 1-, 2-, and 3-digit octets in the same address | [ ] |
| G1 | sodium_ip2bin | ip_len_ longer than the NUL-terminated string (scan stops at the embedded '\0') | [ ] |
| G1 | sodium_ip2bin | ip_len_ shorter than the string (explicit length bounds the parse) | [ ] |
| G1 | sodium_ip2bin | full 8-group IPv6, e.g. "1:2:3:4:5:6:7:8" | [ ] |
| G1 | sodium_ip2bin | IPv6 with leading "::" ("::1", "::") | [ ] |
| G1 | sodium_ip2bin | IPv6 with trailing "::" ("1::") | [ ] |
| G1 | sodium_ip2bin | IPv6 with a middle "::" ("1:2::7:8") | [ ] |
| G1 | sodium_ip2bin | IPv6 with an embedded IPv4 tail ("::ffff:1.2.3.4", "64:ff9b::1.2.3.4") | [ ] |
| G1 | sodium_ip2bin | IPv6 groups with 1, 2, 3 and 4 hex digits, upper- and lower-case (ip_hex_digit `\|32U` branch) | [ ] |
| G1 | sodium_ip2bin | IPv6 with a valid '%'-zone suffix ("fe80::1%eth0", zone chars covering digits/letters/'-'/'_'/'.') | [ ] |
| G1 | sodium_bin2ip | IPv4-mapped input (bin[0..11] == ipv4_mapped_prefix) -> dotted-quad output; ip_maxlen exactly len+1 and larger | [ ] |
| G1 | sodium_bin2ip | all-zero bin (::) -> "::", best_start == 0, best_len == 8 | [ ] |
| G1 | sodium_bin2ip | bin with a single zero word (best_len == 1 -> best_start reset to -1, no "::" compression) | [ ] |
| G1 | sodium_bin2ip | bin with exactly two consecutive zero words (best_len == 2, compression applied) | [ ] |
| G1 | sodium_bin2ip | bin with two zero runs of different lengths -> longest run compressed, first one wins on a tie | [ ] |
| G1 | sodium_bin2ip | zero run at the start / in the middle / at the end of the 8 words | [ ] |
| G1 | sodium_bin2ip | no zero words at all (8 full groups, longest possible IPv6 output, 39 chars) | [ ] |
| G1 | sodium_bin2ip | ip_maxlen == 3 (smallest value passing the `<= 2U` check) with bin == all zeros ("::") | [ ] |
| G1 | sodium_ip2bin + sodium_bin2ip | round-trip: IPv4 literals, IPv6 literals with and without "::", IPv4-mapped forms | [ ] |
