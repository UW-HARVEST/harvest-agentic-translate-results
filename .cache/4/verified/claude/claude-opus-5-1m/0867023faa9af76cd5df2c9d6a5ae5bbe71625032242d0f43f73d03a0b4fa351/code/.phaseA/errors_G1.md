| `sodium_bin2hex` | `bin_len >= SIZE_MAX / 2`, e.g. `bin_len = 0x7FFFFFFFFFFFFFFF`, any `hex`/`hex_maxlen` (bin never dereferenced before the check) | `SIGABRT` (sodium_misuse -> abort) | codecs.c:23 |
| `sodium_bin2hex` | `bin_len = SIZE_MAX` (upper clause of same test) | `SIGABRT` (sodium_misuse) | codecs.c:23 |
| `sodium_bin2hex` | `hex_maxlen <= bin_len * 2U` with no room for the NUL, e.g. `bin_len=4, hex_maxlen=8` | `SIGABRT` (sodium_misuse) | codecs.c:23 |
| `sodium_bin2hex` | `bin_len=0, hex_maxlen=0` (0 <= 0 is true, so even the empty case needs 1 byte) | `SIGABRT` (sodium_misuse) | codecs.c:23 |
| `sodium_bin2hex` | `bin_len=1, hex_maxlen=2` | `SIGABRT` (sodium_misuse) | codecs.c:23 |
| `sodium_hex2bin` | output buffer full: `bin_maxlen=1, hex="0011", hex_len=4, ignore=NULL, hex_end!=NULL` | returns `-1`, `errno=ERANGE`, `*bin_len=0`, `*hex_end=&hex[2]` | codecs.c:71-74 |
| `sodium_hex2bin` | `bin_maxlen=0, hex="00", hex_len=2, hex_end!=NULL` (fails on the very first nibble pair) | returns `-1`, `errno=ERANGE`, `*bin_len=0`, `*hex_end=&hex[0]` | codecs.c:71-74 |
| `sodium_hex2bin` | same ERANGE case but `hex_end=NULL`: `bin_maxlen=1, hex="0011", hex_len=4` | returns `-1`, but `errno` is overwritten to `EINVAL` by the hex_end==NULL branch, `*bin_len=0` | codecs.c:71-74 then 94-96 |
| `sodium_hex2bin` | odd number of hex digits: `hex="abc", hex_len=3, bin_maxlen=2, hex_end!=NULL` | returns `-1`, `errno=EINVAL`, `*bin_len=0`, `hex_pos` decremented so `*hex_end=&hex[2]` | codecs.c:84-88 |
| `sodium_hex2bin` | single hex digit: `hex="a", hex_len=1, bin_maxlen=1` | returns `-1`, `errno=EINVAL`, `*bin_len=0`, `*hex_end=&hex[0]` | codecs.c:84-88 |
| `sodium_hex2bin` | ignore char lands mid-byte (state != 0): `ignore=":", hex="0:0", hex_len=3` (the `state==0U` guard blocks the skip) | returns `-1`, `errno=EINVAL`, `*bin_len=0`, `*hex_end=&hex[0]` | codecs.c:64 then 84-88 |
| `sodium_hex2bin` | trailing garbage with `hex_end=NULL`: `hex="00zz", hex_len=4, bin_maxlen=4, ignore=NULL` | returns `-1`, `errno=EINVAL`, but `*bin_len=1` and `bin[0]=0x00` (bin_pos is zeroed at :90 BEFORE this branch, so it is NOT reset) | codecs.c:94-96 |
| `sodium_hex2bin` | non-hex, non-ignored char with `hex_end=NULL`: `ignore=":", hex="00-11", hex_len=5` | returns `-1`, `errno=EINVAL`, `*bin_len=1` (not reset) | codecs.c:94-96 |
| `sodium_hex2bin` | `hex_len=1, hex=":"`, `ignore=":"`, `hex_end=NULL`: ignored char consumed, hex_pos==hex_len | returns `0`, `*bin_len=0` (NOT an error) | codecs.c:64-67, 94 |
| `sodium_base64_check_variant` | `variant` with `(((unsigned)variant) & ~0x6U) != 0x1U`; only 1, 3, 5, 7 are accepted | `SIGABRT` (sodium_misuse) | codecs.c:168-169 |
| `sodium_base64_encoded_len` | `variant=0` | `SIGABRT` (sodium_misuse via check_variant) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=2` | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=4` | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=6` | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=8` (bit0 clear and a bit outside the 0x6 mask set) | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=9` (`9 & ~6 == 9`) | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=11` or `variant=15` (`& ~6 == 9`) | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `variant=-1` (`0xFFFFFFFF & ~6 == 0xFFFFFFF9`) | `SIGABRT` (sodium_misuse) | codecs.c:176 -> 168 |
| `sodium_base64_encoded_len` | `bin_len / 3 > (SIZE_MAX - 5) / 4`, i.e. `bin_len >= 0xBFFFFFFFFFFFFFFD` (e.g. `bin_len=0xBFFFFFFFFFFFFFFD, variant=1`); no memory is touched so this is directly callable | `SIGABRT` (sodium_misuse). NOTE: the `(size_t)SIZE_MAX` result of the `sodium_base64_ENCODED_LEN` macro is therefore UNREACHABLE through this function | codecs.c:178-179 |
| `sodium_bin2base64` | invalid `variant` (0/2/4/6/8/9/11/15/-1), any other args | `SIGABRT` (sodium_misuse via check_variant) | codecs.c:197 -> 168 |
| `sodium_bin2base64` | `bin_len / 3 > (SIZE_MAX - 5) / 4`, e.g. `bin_len=0xBFFFFFFFFFFFFFFD` with any non-NULL `bin` (bin not dereferenced yet) | `SIGABRT` (sodium_misuse) | codecs.c:199-200 |
| `sodium_bin2base64` | output too small, `b64_maxlen <= b64_len`: `bin_len=3, variant=1, b64_maxlen=4` (needs 5) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `bin_len=0, b64_maxlen=0` (b64_len==0, `0 <= 0`) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `bin_len=1, variant=1 (ORIGINAL)`, `b64_maxlen=4` (padded length is 4, needs 5) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `bin_len=1, variant=3 (ORIGINAL_NO_PADDING)`, `b64_maxlen=2` (b64_len==2, needs 3) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `bin_len=2, variant=7 (URLSAFE_NO_PADDING)`, `b64_maxlen=3` (b64_len==3, needs 4) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `bin_len=32, variant=1`, `b64_maxlen=44` (b64_len==44, needs 45) | `SIGABRT` (sodium_misuse) | codecs.c:211-212 |
| `sodium_bin2base64` | `assert(b64_pos <= b64_len)` — active (no NDEBUG) but defensive; not reachable from any input | `SIGABRT` if it ever fired | codecs.c:239 |
| `sodium_base642bin` | invalid `variant` (0/2/4/6/8/9/11/15/-1) | `SIGABRT` (sodium_misuse via check_variant) | codecs.c:290 -> 168 |
| `sodium_base642bin` | output buffer full: `bin_maxlen=1, b64="AAAA", b64_len=4, variant=1, ignore=NULL, b64_end!=NULL` | returns `-1`, `errno=ERANGE`, `*bin_len=0`, `*b64_end=&b64[2]` | codecs.c:310-313 |
| `sodium_base642bin` | same with `b64_end=NULL` | returns `-1`, `errno` overwritten to `EINVAL` (b64_pos != b64_len), `*bin_len=0` | codecs.c:310-313 then 335-337 |
| `sodium_base642bin` | `bin_maxlen=0, b64="AA", b64_len=2, variant=3` | returns `-1`, `errno=ERANGE`, `*bin_len=0` | codecs.c:310-313 |
| `sodium_base642bin` | b64 length ≡ 1 (mod 4) so `acc_len > 4U`: `b64="A", b64_len=1, variant=3, bin_maxlen=1` | returns `-1`, **errno NOT set/unchanged**, `*bin_len=0` | codecs.c:319-320 |
| `sodium_base642bin` | b64 length ≡ 1 (mod 4), longer: `b64="AAAAA", b64_len=5, variant=3` | returns `-1`, errno unchanged, `*bin_len=0` | codecs.c:319-320 |
| `sodium_base642bin` | non-canonical encoding, leftover bits non-zero: `b64="AB", b64_len=2, variant=3` (acc_len=4, low nibble = 1) | returns `-1`, errno unchanged, `*bin_len=0` | codecs.c:319-320 |
| `sodium_base642bin` | non-canonical encoding, 3 chars: `b64="AAB", b64_len=3, variant=3` (acc_len=2, low 2 bits = 1) | returns `-1`, errno unchanged, `*bin_len=0` | codecs.c:319-320 |
| `sodium_base642bin` | non-canonical with padding: `b64="AAB=", variant=1` | returns `-1` (the acc check at :319 runs before the padding check), errno unchanged | codecs.c:319-320 |
| `sodium_base642bin` | padded variant, padding entirely missing: `b64="AAA", b64_len=3, variant=1` (needs 1 `=`) | returns `-1`, `errno=ERANGE`, `*bin_len=0` | codecs.c:258-260 |
| `sodium_base642bin` | padded variant, both `=` missing: `b64="AA", b64_len=2, variant=1` (needs 2 `=`) | returns `-1`, `errno=ERANGE`, `*bin_len=0` | codecs.c:258-260 |
| `sodium_base642bin` | padded variant, only one of two `=`: `b64="AA=", b64_len=3, variant=5` | returns `-1`, `errno=ERANGE`, `*bin_len=0` | codecs.c:258-260 |
| `sodium_base642bin` | padded variant, non-`=` non-ignored char in the padding position: `b64="AAA*", b64_len=4, variant=1, ignore=NULL` | returns `-1`, `errno=EINVAL`, `*bin_len=0` | codecs.c:266-268 |
| `sodium_base642bin` | `ignore="="` with a padded variant: `b64="AA==", variant=1` — the `=` are consumed as *ignored* chars by the main loop, so skip_padding runs off the end | returns `-1`, `errno=ERANGE`, `*bin_len=0` (i.e. putting `=` in `ignore` breaks padded decoding) | codecs.c:300-303 then 258-260 |
| `sodium_base642bin` | trailing garbage with `b64_end=NULL`: `b64="AAAA!", b64_len=5, variant=3, bin_maxlen=3, ignore=NULL` | returns `-1`, `errno=EINVAL`, but `*bin_len=3` and `bin[0..2]` written (bin_pos zeroed at :327 BEFORE this branch) | codecs.c:335-337 |
| `sodium_base642bin` | URLSAFE variant fed ORIGINAL alphabet: `variant=5, b64="++++", b64_len=4, b64_end=NULL` (`+` maps to 0xFF for urlsafe) | returns `-1`, `errno=EINVAL`, `*bin_len=0` (loop breaks at pos 0) | codecs.c:299-305, 335-337 |
| `sodium_base642bin` | ORIGINAL variant fed URLSAFE alphabet: `variant=1, b64="----", b64_len=4, b64_end=NULL` | returns `-1`, `errno=EINVAL` | codecs.c:299-305, 335-337 |
| `sodium_base642bin` | NO_PADDING variant given padded input with `b64_end=NULL`: `variant=3, b64="AA==", b64_len=4` (`=` is not a b64 char and the padding skip is not run) | returns `-1`, `errno=EINVAL`, `*bin_len=1` (not reset) | codecs.c:322, 335-337 |
| `sodium_ip2bin` | empty input: `ip_len_=0` (or `ip=""`), so `end==ip` | returns `-1` (parse_ipv4 rejects `src >= end`) | codecs.c:363-365 -> 517-519 |
| `sodium_ip2bin` | IPv4 octet > 255: `ip="256.1.1.1"` | returns `-1` | codecs.c:372-374 -> 518 |
| `sodium_ip2bin` | IPv4 octet with 4 digits: `ip="1234.1.1.1"` (`++digits > 3`) | returns `-1` | codecs.c:372-374 -> 518 |
| `sodium_ip2bin` | IPv4 empty octet: `ip="1..2.3"` or `ip=".1.2.3"` (`digits == 0`) | returns `-1` | codecs.c:376-378 -> 518 |
| `sodium_ip2bin` | IPv4 missing separator/short: `ip="1.2.3"` (`p >= end` where a `.` was required) | returns `-1` | codecs.c:381-385 -> 518 |
| `sodium_ip2bin` | IPv4 wrong separator: `ip="1.2.3-4"` | returns `-1` | codecs.c:381-385 -> 518 |
| `sodium_ip2bin` | IPv4 trailing garbage: `ip="1.2.3.4x"` or `ip="1.2.3.4."` (final `return p == end` fails) | returns `-1` | codecs.c:387 -> 518 |
| `sodium_ip2bin` | invalid zone-id character: `ip="fe80::1%eth0!"` (allowed set is `[0-9a-zA-Z]`, `-`, `_`, `.`) | returns `-1` | codecs.c:500-503 |
| `sodium_ip2bin` | empty zone id: `ip="fe80::1%"` (`zone + 1 >= end`) | returns `-1` | codecs.c:505-507 |
| `sodium_ip2bin` | zone id only: `ip="%"` | returns `-1` | codecs.c:505-507 |
| `sodium_ip2bin` | zone id on a non-IPv6 address: `ip="1.2.3.4%eth0"` | returns `-1` | codecs.c:511-513 |
| `sodium_ip2bin` | `ip="1.2.3.4%foo:bar"` (the `:` is after `%` so it is cut off and `is_ipv6` is false) | returns `-1` | codecs.c:511-513 |
| `sodium_ip2bin` | IPv6 single leading colon: `ip=":1::2"` or `ip=":"` (`++p >= end or *p != ':'`) | returns `-1` | codecs.c:408-411 -> 515 |
| `sodium_ip2bin` | IPv6 two `::` runs: `ip="1::2::3"` (`colonp != NULL`) | returns `-1` | codecs.c:419-422 -> 515 |
| `sodium_ip2bin` | IPv6 too many groups: `ip="1:2:3:4:5:6:7:8:9"` (`tp + 2 > endp`) | returns `-1` | codecs.c:427-429 -> 515 |
| `sodium_ip2bin` | IPv6 trailing single colon: `ip="1:2:"` (`p >= end` after `++p`) | returns `-1` | codecs.c:436-438 -> 515 |
| `sodium_ip2bin` | IPv6 group with 5 hex digits: `ip="12345::"` (`xdigits >= 4`) | returns `-1` | codecs.c:449-452 -> 515 |
| `sodium_ip2bin` | IPv6 invalid hex char: `ip="1:2:3:4:5:6:7:g"` (`ip_hex_digit` returns -1) | returns `-1` | codecs.c:449-452 -> 515 |
| `sodium_ip2bin` | IPv6 embedded IPv4 that does not fit: `ip="1:2:3:4:5:6:7:1.2.3.4"` (`tp + 4 > endp`) | returns `-1` | codecs.c:441-444 -> 515 |
| `sodium_ip2bin` | IPv6 embedded IPv4 malformed: `ip="::1.2.3"` | returns `-1` | codecs.c:441-444 -> 515 |
| `sodium_ip2bin` | IPv6 embedded IPv4 not at end: `ip="::1.2.3.4:5"` (parse_ipv4 is given the full `end`) | returns `-1` | codecs.c:441-444 -> 515 |
| `sodium_ip2bin` | `::` used when the address is already full: `ip="1:2:3:4:5:6:7:8::"` (`tp == endp` with colonp set) | returns `-1` | codecs.c:465-473 -> 515 |
| `sodium_ip2bin` | IPv6 too few groups and no `::`: `ip="1:2:3"` (`tp != endp`) | returns `-1` | codecs.c:475-477 -> 515 |
| `sodium_ip2bin` | IPv6 final group overflow: `ip="1:2:3:4:5:6:7:8:9999"`-style where `tp + 2 > endp` at the tail | returns `-1` | codecs.c:458-461 -> 515 |
| `sodium_bin2ip` | `ip_maxlen <= 2U`, e.g. `ip_maxlen = 0, 1, 2` with any `bin` | returns `NULL` (nothing written), errno untouched | codecs.c:561-563 |
| `sodium_bin2ip` | IPv4-mapped `bin` and `len >= ip_maxlen`: `bin = ::ffff:255.255.255.255` (text len 15), `ip_maxlen=15` | returns `NULL`, `ip` untouched | codecs.c:571-574 |
| `sodium_bin2ip` | IPv4-mapped `bin` and `ip_maxlen=7` with `bin = ::ffff:10.0.0.1` (text len 8) | returns `NULL` | codecs.c:571-574 |
| `sodium_bin2ip` | IPv6 `bin` and `len >= ip_maxlen`: `bin` = 16x0xff (text `ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff`, len 39), `ip_maxlen=39` | returns `NULL`, `ip` untouched | codecs.c:616-619 |
| `sodium_bin2ip` | IPv6 `bin` = `::1` (text len 3) with `ip_maxlen=3` | returns `NULL` | codecs.c:616-619 |
| `sodium_memcmp` | any `len >= 1` where the buffers differ in at least one byte, e.g. `len=1, b1={0}, b2={1}` | returns `-1` (equal -> `0`; `len=0` -> `0`) | utils.c:207 |
| `sodium_compare` | `b1 < b2` little-endian (most significant byte is index `len-1`), e.g. `len=2, b1={0xff,0x00}, b2={0x00,0x01}` | returns `-1` | utils.c:254 |
| `sodium_compare` | `b1 > b2`, e.g. `len=1, b1={1}, b2={0}` | returns `1` | utils.c:254 |
| `sodium_is_zero` | any byte non-zero, e.g. `nlen=32` with `n[31]=1` | returns `0` (all-zero or `nlen=0` -> `1`) | utils.c:266 |
| `sodium_mlock` | ALWAYS in this build (`HAVE_MLOCK` and `WINAPI_DESKTOP` both undefined) — any `addr`/`len` | returns `-1`, `errno=ENOSYS` (buffer untouched) | utils.c:443-444 |
| `sodium_munlock` | ALWAYS in this build — any `addr`/`len` | returns `-1`, `errno=ENOSYS`, **but the `len` bytes at `addr` have been zeroed first** by `sodium_memzero` | utils.c:451, 460-461 |
| `sodium_mprotect_noaccess` | ALWAYS in this build (`HAVE_PAGE_PROTECTION` undefined -> stub `_sodium_mprotect`) — any `ptr`, including a valid `sodium_malloc` pointer | returns `-1`, `errno=ENOSYS`, memory left readable/writable | utils.c:702-708 via 729 |
| `sodium_mprotect_readonly` | ALWAYS in this build — any `ptr` | returns `-1`, `errno=ENOSYS` | utils.c:702-708 via 735 |
| `sodium_mprotect_readwrite` | ALWAYS in this build — any `ptr` | returns `-1`, `errno=ENOSYS` | utils.c:702-708 via 741 |
| `_mprotect_noaccess` / `_mprotect_readonly` / `_mprotect_readwrite` | `errno=ENOSYS; return -1` fallbacks — DEAD CODE: `_sodium_mprotect` never invokes the callback in this build | n/a | utils.c:474-475, 488-489, 502-503 |
| `sodium_malloc` | underlying `malloc` fails: `size = SIZE_MAX` (this build has NO `size >= SIZE_MAX - page_size*4` pre-check, it goes straight to `malloc(size)`) | returns `NULL`, `errno=ENOMEM` (set by libc malloc); no memset performed | utils.c:593, 644-646 |
| `sodium_malloc` | `size = SIZE_MAX - 4096` (still an impossible malloc) | returns `NULL`, `errno=ENOMEM` | utils.c:593, 644-646 |
| `sodium_allocarray` | `count > 0 && size >= SIZE_MAX / count`: `count=2, size=SIZE_MAX/2` (note `>=`, not `>`) | returns `NULL`, `errno=ENOMEM`, no allocation attempted | utils.c:655-658 |
| `sodium_allocarray` | `count=1, size=SIZE_MAX` (`SIZE_MAX/1 == SIZE_MAX`, so `size >= that`) | returns `NULL`, `errno=ENOMEM` | utils.c:655-658 |
| `sodium_allocarray` | `count=SIZE_MAX, size=1` (`SIZE_MAX/SIZE_MAX == 1`, `1 >= 1`) | returns `NULL`, `errno=ENOMEM` | utils.c:655-658 |
| `sodium_allocarray` | `count=3, size=SIZE_MAX/3` | returns `NULL`, `errno=ENOMEM` | utils.c:655-658 |
| `sodium_allocarray` | passes the overflow check but the product is unallocatable: `count=2, size=SIZE_MAX/2 - 1` | returns `NULL` from `sodium_malloc`, `errno=ENOMEM` (from libc) | utils.c:659 -> 593 |
| `sodium_pad` | `blocksize == 0` (`blocksize <= 0U`), any other args | returns `-1`, `*padded_buflen_p` NOT written | utils.c:755-757 |
| `sodium_pad` | `SIZE_MAX - unpadded_buflen <= xpadlen`: `unpadded_buflen = SIZE_MAX, blocksize = 16` (xpadlen becomes 0) | `SIGABRT` (sodium_misuse); `buf` is never dereferenced before this point | utils.c:764-766 |
| `sodium_pad` | `unpadded_buflen = SIZE_MAX - 3, blocksize = 16` (xpadlen = 3, `3 <= 3`) | `SIGABRT` (sodium_misuse) | utils.c:764-766 |
| `sodium_pad` | `unpadded_buflen = SIZE_MAX, blocksize = 1` (xpadlen = 0, `0 <= 0`) | `SIGABRT` (sodium_misuse) | utils.c:764-766 |
| `sodium_pad` | `xpadded_len >= max_buflen`: `unpadded_buflen=10, blocksize=16, max_buflen=15` (xpadded_len = 15) | returns `-1`, `*padded_buflen_p` NOT written, `buf` untouched | utils.c:768-770 |
| `sodium_pad` | exact-fit-minus-one: `unpadded_buflen=16, blocksize=16, max_buflen=31` (xpadded_len = 31, a full extra block is required) | returns `-1` | utils.c:768-770 |
| `sodium_pad` | `max_buflen = 0` with any `unpadded_buflen`/`blocksize >= 1` | returns `-1` (`xpadded_len >= 0` always) | utils.c:768-770 |
| `sodium_pad` | `unpadded_buflen=0, blocksize=16, max_buflen=15` (xpadded_len = 15) | returns `-1` | utils.c:768-770 |
| `sodium_pad` | non-power-of-two blocksize: `unpadded_buflen=5, blocksize=3, max_buflen=6` (xpadlen = 2 - 5%3 = 0, xpadded_len = 5, padded = 6 -> OK) vs `max_buflen=5` | returns `-1` for `max_buflen=5` | utils.c:761-762, 768-770 |
| `sodium_unpad` | `blocksize == 0` (second clause of the test), any `padded_buflen` | returns `-1`, `*unpadded_buflen_p` NOT written | utils.c:797-799 |
| `sodium_unpad` | `padded_buflen < blocksize`: `padded_buflen=0, blocksize=16` | returns `-1`, `*unpadded_buflen_p` NOT written | utils.c:797-799 |
| `sodium_unpad` | `padded_buflen=15, blocksize=16` | returns `-1`, `*unpadded_buflen_p` NOT written | utils.c:797-799 |
| `sodium_unpad` | `padded_buflen=0, blocksize=1` | returns `-1` | utils.c:797-799 |
| `sodium_unpad` | invalid padding, no 0x80 barrier in the last block: `buf` = 16 zero bytes, `padded_buflen=16, blocksize=16` | returns `-1` **and still writes** `*unpadded_buflen_p = padded_buflen - 1 - pad_len = 15` (the write at :810 is unconditional) | utils.c:810, 812 |
| `sodium_unpad` | invalid padding, non-0x80 garbage after the barrier: `buf = {..., 0x80, 0x01}`, `blocksize=16` | returns `-1`, `*unpadded_buflen_p` written to `padded_buflen-1` | utils.c:810, 812 |
| `sodium_unpad` | invalid padding, last byte is 0xff: `buf` = 16 bytes ending in `0xff`, `blocksize=16` | returns `-1`, `*unpadded_buflen_p = 15` | utils.c:810, 812 |
| `sodium_unpad` | `unpadded_buflen_p = NULL` with an otherwise valid call (`padded_buflen >= blocksize`, `blocksize > 0`) — the code performs NO NULL check | `SIGSEGV` (unconditional store at :810) | utils.c:810 |
| `sodium_memzero` | no rejection path in this build; the `sodium_misuse()` at :132 is inside `#elif defined(HAVE_MEMSET_S)` -> DEAD CODE. `len=0` is a no-op (even with `pnt=NULL`) | always returns void | utils.c:128-157 |
| `sodium_stackzero` | no rejection path — with `HAVE_C_VARARRAYS` and `HAVE_ALLOCA` both undefined the function body is EMPTY, so any `len` (including `SIZE_MAX`) is a no-op | always returns void, nothing is zeroed | utils.c:159-168 |
| `sodium_free` | `ptr == NULL` | no-op (plain `free(NULL)`); the `if (ptr == NULL) return;` guard at :678 is in the DEAD `HAVE_ALIGNED_MALLOC` variant | utils.c:663-667 |
| `sodium_free` | pointer not obtained from `sodium_malloc`/`sodium_allocarray` | undefined behaviour in libc `free()`; NO canary check and NO `_out_of_bounds()` in this build | utils.c:663-667 |
| `_out_of_bounds` / canary check / guard pages | `abort()` on buffer-underflow/overflow detection — DEAD CODE: guarded by `#ifdef HAVE_ALIGNED_MALLOC`, which is undefined here | n/a (writing past a `sodium_malloc` region does NOT fault in this build) | utils.c:510-523, 687-694 |
| `_sodium_malloc` (aligned variant) | `size >= SIZE_MAX - page_size*4` -> `errno=ENOMEM; return NULL`; `page_size <= sizeof canary` -> `sodium_misuse()`; `_alloc_aligned` failure -> `NULL`; `assert(_unprotected_ptr_from_user_ptr(...) == ...)` | ALL DEAD CODE (`HAVE_ALIGNED_MALLOC` undefined) | utils.c:607-618, 633 |
| `_unprotected_ptr_from_user_ptr` | `unprotected_ptr_u <= page_size * 2U` -> `sodium_misuse()` — DEAD CODE | n/a | utils.c:581-583 |
| `_sodium_alloc_init` | `page_size < CANARY_SIZE or page_size < sizeof(size_t)` -> `sodium_misuse()` — DEAD CODE (inside `#ifdef HAVE_ALIGNED_MALLOC`); the live body is just `randombytes_buf(canary, 16); return 0;` | always returns `0` | utils.c:423-429 |
| `sodium_misuse` | reached from any of the misuse sites above | calls the misuse handler (if one was installed via `sodium_set_misuse_handler`) and then `abort()` UNCONDITIONALLY -> `SIGABRT`; a handler that simply returns does NOT prevent the abort | core.c:191-205 |
| `sodium_init` | `sodium_crit_enter()/leave()` failure -> `return -1` — DEAD CODE: with `_WIN32`, `HAVE_PTHREAD` and `HAVE_ATOMIC_OPS` all undefined the crit functions are the no-op stubs that always return `0` | never returns `-1` in this build | core.c:30-35, 52-54, 175-186 |
| `sodium_set_misuse_handler` | `return -1` paths — DEAD CODE (same reason) | always returns `0` | core.c:211-217 |
| `sodium_crit_leave` | `locked == 0` -> `errno=EPERM; return -1` — DEAD CODE (that copy is inside the `_WIN32`/`HAVE_PTHREAD` branches) | n/a | core.c:100-104, 131-135 |
| `sodium_crit_enter` | `assert(locked == 0)` — DEAD CODE (inside `_WIN32`/`HAVE_PTHREAD` branches) | n/a | core.c:91, 122 |
| `_sodium_runtime_arm_cpu_features` | `#ifndef __ARM_ARCH return -1;` — LIVE on x86-64, taken unconditionally | returns `-1` (after zeroing has_neon/has_armcrypto) | runtime.c:66-68 |
| `_sodium_runtime_intel_cpu_features` | `cpu_info[0] == 0U` — LIVE and always taken: `HAVE_CPUID` is undefined so `_cpuid()` just zero-fills `cpu_info` | returns `-1`; every `has_*` field stays `0` | runtime.c:195-198, 207-210 |
| `_sodium_runtime_get_cpu_features` | always (`-1 & -1 & -1`) | returns `-1`; all `sodium_runtime_has_*()` then return `0`. `sodium_init()` ignores this value | runtime.c:318-328 |
| `randombytes_buf_deterministic` | `size > 0x4000000000ULL` (the `#if SIZE_MAX > 0x4000000000ULL` block IS live on x86-64): `size = 0x4000000001ULL` with any `buf`/`seed` (checked before `buf` is written) | `SIGABRT` (sodium_misuse) | randombytes.c:219-224 |
| `randombytes` (NaCl API) | `assert(buf_len <= SIZE_MAX)` — cannot fail on x86-64 (`unsigned long long` and `size_t` are both 64-bit, `ULLONG_MAX == SIZE_MAX`) | never fires | randombytes.c:247 |
| `randombytes_implementation_name` | custom implementation installed with `implementation_name == NULL` (the code performs NO NULL check on this member) | `SIGSEGV` (call through NULL) | randombytes.c:158-159 |
| `randombytes_random` | custom implementation installed with `random == NULL` (no NULL check) | `SIGSEGV` | randombytes.c:165-166 |
| `randombytes_buf` | custom implementation installed with `buf == NULL` and `size > 0` (no NULL check) | `SIGSEGV`; with `size == 0` the callback is never invoked so it is safe | randombytes.c:204-207 |
| `randombytes_uniform` | custom implementation with `uniform == NULL` -> falls through to the generic rejection-sampling path; only `stir`, `uniform` and `close` are NULL-checked | no error | randombytes.c:185-187 |
| `randombytes_close` | `implementation == NULL` (no randombytes/sodium_init call has happened yet) | returns `0` without touching any RNG state | randombytes.c:238-241 |
| `randombytes_close` | `randombytes_set_implementation(&randombytes_sysrandom_implementation)` then `randombytes_close()` with no prior stir: `random_data_source_fd == -1` and `getrandom_available == 0` | returns `-1` | randombytes_sysrandom.c:316, 319-329, 336 |
| `randombytes_close` | `randombytes_set_implementation(&randombytes_internal_implementation)` then `randombytes_close()` with no prior stir: `global.getrandom_available == 0` | returns `-1` (and `stream` is zeroed) | randombytes_internal_random.c:516, 524-527, 542-544 |
| `randombytes_sysrandom_implementation.buf` | called directly with `size == 0` (the exported struct bypasses `randombytes_buf`'s `size > 0` guard): `randombytes_sysrandom_implementation.buf(b, 0)` -> `randombytes_linux_getrandom(b, 0)` -> `chunk_size = 0` | `SIGABRT` (`assert(chunk_size > 0U)`, active because NDEBUG is not defined) | randombytes_sysrandom.c:248-251 |
| `randombytes_sysrandom_init` | `/dev/urandom` and `/dev/random` both unopenable AND `getrandom(2)` unavailable | `SIGABRT` (sodium_misuse); on Linux with `SYS_getrandom` present the getrandom probe succeeds so this is unreachable | randombytes_sysrandom.c:281-284 |
| `randombytes_sysrandom_random_dev_open` | no usable device found after trying `/dev/urandom` then `/dev/random` | `errno=EIO`, returns `-1` (-> caller misuses) | randombytes_sysrandom.c:222-223 |
| `randombytes_block_on_dev_random` | `poll()` on `/dev/random` returns != 1 (BLOCK_ON_DEV_RANDOM is LIVE on Linux) | `errno=EIO`, returns `-1` -> `randombytes_sysrandom_random_dev_open` returns `-1` -> `sodium_misuse()` | randombytes_sysrandom.c:170-174, 194-196 |
| `randombytes_sysrandom_buf` | `getrandom_available != 0` and `randombytes_linux_getrandom()` fails (short/failed syscall) | `SIGABRT` (sodium_misuse) | randombytes_sysrandom.c:351-354 |
| `randombytes_sysrandom_buf` | `random_data_source_fd == -1` or `safe_read()` returns fewer than `size` bytes | `SIGABRT` (sodium_misuse) | randombytes_sysrandom.c:358-361 |
| `randombytes_sysrandom_buf` | the `size > 0xffffffffUL` and `!RtlGenRandom` misuse paths — DEAD CODE (`_WIN32` branch) | n/a | randombytes_sysrandom.c:364-369 |
| `safe_read` (sysrandom) | `assert(size > 0U)` and `assert(size <= SSIZE_MAX)` — active asserts; only reachable on the `/dev/urandom` fallback path | `SIGABRT` if violated | randombytes_sysrandom.c:134-135 |
| `_randombytes_linux_getrandom` (sysrandom) | `assert(size <= 256U)` — active; callers always chunk to 256 | `SIGABRT` if violated | randombytes_sysrandom.c:233 |
| `randombytes_internal_implementation.buf` / `.random` | `sodium_hrtime()`: `gettimeofday()` failure during stir | `SIGABRT` (sodium_misuse) | randombytes_internal_random.c:172-174 |
| `randombytes_internal_random_stir` | `assert(stream.nonce != 0)` — fires only if `gettimeofday` yields exactly 0 µs since the epoch | `SIGABRT` if violated | randombytes_internal_random.c:430 |
| `randombytes_internal_random_stir` | `getrandom_available != 0` and `randombytes_linux_getrandom(stream.key, 32)` fails | `SIGABRT` (sodium_misuse) | randombytes_internal_random.c:449-454 |
| `randombytes_internal_random_stir` | `random_data_source_fd == -1` or short `safe_read` of the 32-byte key (fallback path) | `SIGABRT` (sodium_misuse) | randombytes_internal_random.c:457-462 |
| `randombytes_internal_random_init` | `assert((getentropy_available or getrandom_available) == 0)` then device-open failure | `SIGABRT` (assert or sodium_misuse) | randombytes_internal_random.c:406-410 |
| `randombytes_internal_random_init` | trailing `sodium_misuse()` under `#ifndef HAVE_SAFE_ARC4RANDOM` — DEAD CODE (the `!NONEXISTENT_DEV_RANDOM` block above it always `return`s) | n/a | randombytes_internal_random.c:415-417 |
| `randombytes_internal_random_stir_if_needed` | `global.pid != getpid()` -> `sodium_misuse()` (fork detection) — DEAD CODE: `HAVE_GETPID` is undefined, so there is NO fork protection in this build | n/a | randombytes_internal_random.c:483-493 |
| `randombytes_internal_random_buf` | `assert(ret == 0)` on the chacha20 return; `assert(size <= ULLONG_MAX)` is not even compiled (`SIZE_MAX > ULLONG_MAX` is false) | `SIGABRT` if violated (unreachable) | randombytes_internal_random.c:596-604 |
| `randombytes_internal_random` | `assert(ret == 0)` on the chacha20 return | `SIGABRT` if violated (unreachable) | randombytes_internal_random.c:636 |
| `randombytes_internal_random.c` `_randombytes_getentropy` / `randombytes_getentropy` | `errno=ENOSYS; return -1` and `assert(size <= 256U)` — DEAD CODE (`HAVE_GETENTROPY` and `HAVE_COMMONCRYPTO_COMMONRANDOM_H` undefined) | n/a | randombytes_internal_random.c:194-240 |
| `randombytes_linux_getrandom` (internal) | called with `size == 0` -> `chunk_size = 0` -> `assert(chunk_size > 0U)`; reachable only internally (key is always 32 bytes) | `SIGABRT` if violated | randombytes_internal_random.c:263-267 |
