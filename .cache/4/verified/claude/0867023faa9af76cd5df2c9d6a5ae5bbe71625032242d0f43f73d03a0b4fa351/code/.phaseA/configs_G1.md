| `sodium_bin2hex` | `bin_len=0`, `hex_maxlen=1` (minimum legal): writes only the terminator | result is the empty string, returns `hex`; codecs.c:26-38 |
| `sodium_bin2hex` | `bin_len=1`, `bin={0x00}`, `hex_maxlen=3` -> `"00"` (exercises the `c < 10` digit branch of the `87U + c + (((c-10U)>>8) & ~38U)` expression) | codecs.c:29-33 |
| `sodium_bin2hex` | `bin_len=1`, `bin={0x0a}`, `hex_maxlen=3` -> `"0a"` (low nibble >= 10 -> letter branch) | codecs.c:29-33 |
| `sodium_bin2hex` | `bin_len=1`, `bin={0xff}` -> `"ff"`; `bin={0xa0}` -> `"a0"` (both nibbles >= 10) | output is ALWAYS lowercase; codecs.c:29-33 |
| `sodium_bin2hex` | `bin_len=2`, `hex_maxlen=5` (exact fit) vs `hex_maxlen=64` (oversized) | with an oversized buffer only `bin_len*2+1` bytes are written; the rest of `hex` is left UNTOUCHED (contrast with `sodium_bin2base64`, which zero-fills); codecs.c:36 |
| `sodium_bin2hex` | `bin_len=32` (typical key), `hex_maxlen=65`; and `bin_len=1024`, `hex_maxlen=2049` | linear loop, no size-specific branches; codecs.c:26-35 |
| `sodium_hex2bin` | `hex_len=0`, `bin_maxlen=0`, `ignore=NULL`, `bin_len!=NULL`, `hex_end=NULL` | returns `0`, `*bin_len=0`; the empty input is valid; codecs.c:57, 92-100 |
| `sodium_hex2bin` | even lowercase input `hex="00ff"`, `bin_maxlen=2`, `ignore=NULL`, `hex_end=NULL`, `bin_len!=NULL` | returns `0`, 2 bytes; codecs.c:57-83 |
| `sodium_hex2bin` | UPPERCASE input `hex="00FF"` (the `(c & ~32U) - 55U` alpha path) | accepted, same bytes as lowercase; codecs.c:61-62 |
| `sodium_hex2bin` | MIXED case `hex="AbCd"` | accepted -> `{0xab,0xcd}`; codecs.c:61-62 |
| `sodium_hex2bin` | `ignore=":"`, `hex="00:11:22"`, `hex_len=8`, `bin_maxlen=3`, `hex_end=NULL` | returns `0`, 3 bytes; separators consumed only at byte boundaries; codecs.c:64-67 |
| `sodium_hex2bin` | `ignore=": \n"`, `hex="00 11\n22"` (multiple distinct ignore chars) | returns `0`, 3 bytes; codecs.c:64 |
| `sodium_hex2bin` | `ignore` containing a character that IS a hex digit, e.g. `ignore="a"`, `hex="aa"` | the ignore test is only reached for NON-hex characters, so `'a'` is decoded, never skipped -> `{0xaa}`; codecs.c:63-69 |
| `sodium_hex2bin` | `ignore=""` (non-NULL empty string), `hex="0011"` | behaves like `ignore=NULL` for all non-NUL chars, BUT `strchr("", '\0')` is non-NULL so embedded NUL bytes are skipped; codecs.c:64 |
| `sodium_hex2bin` | embedded NUL: `hex={'0','0',0,'1','1'}`, `hex_len=5`, `ignore=":"` | the NUL is treated as an ignorable char (`strchr` matches the terminator) -> decodes 2 bytes, returns `0`; codecs.c:64 |
| `sodium_hex2bin` | same embedded NUL with `ignore=NULL`, `hex_end!=NULL` | loop breaks at the NUL, returns `0`, `*bin_len=1`, `*hex_end=&hex[2]`; codecs.c:63-69, 92-93 |
| `sodium_hex2bin` | `hex_end != NULL` with trailing garbage: `hex="00zz"`, `hex_len=4` | returns `0`, `*bin_len=1`, `*hex_end=&hex[2]`; garbage is tolerated when `hex_end` is supplied; codecs.c:92-93 |
| `sodium_hex2bin` | `hex_end != NULL` and full consumption: `hex="0011"` | returns `0`, `*hex_end == hex + hex_len`; codecs.c:92-93 |
| `sodium_hex2bin` | `bin_len = NULL` (caller does not want the length), `hex_end=NULL`, valid even input | returns `0`, length not reported; codecs.c:98-100 |
| `sodium_hex2bin` | `bin_maxlen` much larger than needed (e.g. 64) with `hex_len=4` | only 2 bytes written, the rest of `bin` untouched; codecs.c:79 |
| `sodium_hex2bin` | `hex_len` smaller than the actual string: `hex="00112233"`, `hex_len=4` | decodes only 2 bytes; the length argument, not a NUL, bounds the scan; codecs.c:57 |
| `sodium_hex2bin` | large input: 2048 hex chars, `bin_maxlen=1024` | returns `0`, 1024 bytes; codecs.c:57-83 |
| `sodium_base64_encoded_len` | `variant=1 (ORIGINAL)` x `bin_len=0,1,2,3,4,5,6` -> `1,5,5,5,9,9,9` (includes the trailing NUL) | macro `sodium_base64_ENCODED_LEN`; utils.h:79-88, codecs.c:181 |
| `sodium_base64_encoded_len` | `variant=3 (ORIGINAL_NO_PADDING)` x `bin_len=0,1,2,3,4,5,6` -> `1,3,4,5,7,8,9` | the `(VARIANT & 2)` term removes the padding allowance; utils.h:79-88 |
| `sodium_base64_encoded_len` | `variant=5 (URLSAFE)` x `bin_len=1,2,3` -> `5,5,5` (identical to ORIGINAL: the URLSAFE bit does not affect length) | utils.h:79-88 |
| `sodium_base64_encoded_len` | `variant=7 (URLSAFE_NO_PADDING)` x `bin_len=1,2,3` -> `3,4,5` | utils.h:79-88 |
| `sodium_base64_encoded_len` | large valid value: `bin_len=1000000, variant=1` and `variant=3` | pure arithmetic, no allocation; codecs.c:173-182 |
| `sodium_base64_encoded_len` | exactly one below the misuse threshold: `bin_len=0xBFFFFFFFFFFFFFFC` | returns a huge size_t without aborting (threshold is `bin_len/3 > (SIZE_MAX-5)/4`); codecs.c:178 |
| `sodium_bin2base64` | `variant=1`, `bin_len=0`, `b64_maxlen=1` | empty string; `b64_len=0`, one NUL written; codecs.c:198-247 |
| `sodium_bin2base64` | `variant=1`, `bin_len=1` (`bin_len % 3 == 1`), `b64_maxlen=5` | 2 data chars + 2 `=`, e.g. `bin={0x00}` -> `"AA=="`; codecs.c:204-206, 240-242 |
| `sodium_bin2base64` | `variant=1`, `bin_len=2` (`% 3 == 2`), `b64_maxlen=5` | 3 data chars + 1 `=`; codecs.c:204-206, 240-242 |
| `sodium_bin2base64` | `variant=1`, `bin_len=3` (`% 3 == 0`), `b64_maxlen=5` | 4 data chars, no padding; codecs.c:198-203 |
| `sodium_bin2base64` | `variant=1`, `bin_len=4` and `bin_len=5` (nibbles=1, remainder 1 then 2), `b64_maxlen=9` | `b64_len=8` in both cases; codecs.c:198-209 |
| `sodium_bin2base64` | `variant=3 (ORIGINAL_NO_PADDING)` x `bin_len=1` -> `b64_len=2`; `bin_len=2` -> `b64_len=3` (`2 + (remainder >> 1)`) | codecs.c:207-209 |
| `sodium_bin2base64` | `variant=5 (URLSAFE)`, `bin={0xff,0xff,0xff}` -> `"____"` vs `variant=1` -> `"////"` | the URLSAFE branch swaps `+`/`/` for `-`/`_`; codecs.c:214-225 vs 226-238 |
| `sodium_bin2base64` | `variant=5`, `bin={0xfb,0xef,0xbe}` (all four 6-bit groups == 62) -> `"----"`; `variant=1` -> `"++++"` | codecs.c:120-146 |
| `sodium_bin2base64` | `variant=7 (URLSAFE_NO_PADDING)`, `bin_len=1` and `bin_len=2` | urlsafe alphabet AND no `=`; codecs.c:207-209, 214-225 |
| `sodium_bin2base64` | all 4 variants x `bin_len=0,1,2,3,31,32,33,1024,1025,1026` (covers every `bin_len % 3` at small and large sizes) | the encoder is a single 6-bit accumulator loop; codecs.c:214-238 |
| `sodium_bin2base64` | `b64_maxlen == encoded_len` (exact fit) e.g. `bin_len=3, variant=1, b64_maxlen=5` | one NUL written at index 4; codecs.c:243-245 |
| `sodium_bin2base64` | `b64_maxlen` MUCH larger than needed, e.g. `bin_len=3, variant=1, b64_maxlen=64` | the `do { b64[b64_pos++] = 0; } while (b64_pos < b64_maxlen)` loop ZERO-FILLS the whole remaining buffer up to `b64_maxlen` (60 NUL bytes here) — a key observable difference from `sodium_bin2hex`; codecs.c:243-245 |
| `sodium_bin2base64` | byte values covering the whole alphabet: `bin` = 0x00..0x2F (48 bytes -> 64 chars, all 64 symbols) for each variant | exercises every branch of `b64_byte_to_char` / `b64_byte_to_urlsafe_char`; codecs.c:117-146 |
| `sodium_base642bin` | `variant=1`, `b64="AAAA"`, `b64_len=4`, `bin_maxlen=3`, `ignore=NULL`, `bin_len!=NULL`, `b64_end=NULL` | returns `0`, `*bin_len=3`; codecs.c:292-342 |
| `sodium_base642bin` | `variant=1`, `b64="AA=="` (2 data chars leave `acc_len=4`, so `padding_len = acc_len/2 = 2` pad chars are required), `bin_maxlen=1` | returns `0`, `*bin_len=1`; codecs.c:319-325, 250-273 |
| `sodium_base642bin` | `variant=1`, `b64="AAA="` (1 pad char, `acc_len=2` -> `padding_len=1`), `bin_maxlen=2` | returns `0`, `*bin_len=2`; codecs.c:321-325 |
| `sodium_base642bin` | `variant=3 (NO_PADDING)`, `b64="AA"` / `"AAA"` / `"AAAA"` | returns `0` with 1/2/3 bytes; the `_sodium_base642bin_skip_padding` call is skipped entirely; codecs.c:321-322 |
| `sodium_base642bin` | `variant=3`, padded input `b64="AA=="` with `b64_end != NULL` | returns `0`, `*bin_len=1`, `*b64_end=&b64[2]` (points AT the first `=`); with `b64_end=NULL` the same input returns `-1`/EINVAL; codecs.c:333-337 |
| `sodium_base642bin` | `variant=5 (URLSAFE)`, `b64="____"` -> 3 bytes of `0xff`; `variant=7`, `b64="__"` | codecs.c:294-298, 148-159 |
| `sodium_base642bin` | `variant=5`, `b64="----"` (62 chars) | urlsafe `-` accepted; codecs.c:148-159 |
| `sodium_base642bin` | `ignore=" \n"`, `variant=1`, `b64="AAAA AAAA\n"`, `b64_end=NULL` | leading/interior/trailing ignored chars are consumed; the trailing-ignore sweep at :328-331 only runs when `ret == 0`; codecs.c:300-303, 328-331 |
| `sodium_base642bin` | `ignore=" "`, `variant=1`, `b64="AA == "` (spaces inside the padding run) | `_sodium_base642bin_skip_padding` also honours `ignore`; returns `0`; codecs.c:264-270 |
| `sodium_base642bin` | `ignore=" "` with trailing spaces after the padding: `b64="AA== "` | the post-success sweep consumes them so `b64_pos == b64_len` and `b64_end=NULL` still returns `0`; codecs.c:328-331 |
| `sodium_base642bin` | `b64_len=0`, `bin_maxlen=0`, any valid variant, `b64_end=NULL`, `bin_len!=NULL` | returns `0`, `*bin_len=0`; codecs.c:292, 319-325 |
| `sodium_base642bin` | `bin_len = NULL` and/or `b64_end = NULL` vs both non-NULL, on the same valid input | both out-params are independently optional; codecs.c:333-341 |
| `sodium_base642bin` | `bin_maxlen` exactly the decoded size vs much larger (e.g. 64) | no over-write; only `*bin_len` bytes touched; codecs.c:315 |
| `sodium_base642bin` | embedded NUL with `ignore != NULL`: `b64={'A','A','A','A',0,'A','A','A','A'}`, `b64_len=9`, `variant=3` | `strchr(ignore, 0)` matches the terminator, so the NUL is skipped and all 8 chars decode; codecs.c:300 |
| `sodium_base642bin` | large input: 4096 base64 chars, `variant=3`, `bin_maxlen=3072` | returns `0`; codecs.c:292-318 |
| `sodium_base642bin` | round-trip: for each of the 4 variants and each `bin_len` in `0..8`, `sodium_bin2base64` then `sodium_base642bin` | must recover the original bytes exactly; codecs.c:184-343 |
| `sodium_ip2bin` | IPv4 dotted-quad `"0.0.0.0"`, `"1.2.3.4"`, `"255.255.255.255"`, `ip_len_=strlen` | returns `0`; `bin[0..9]=0`, `bin[10]=bin[11]=0xff`, `bin[12..15]` = the octets (IPv4-mapped form); codecs.c:517-525 |
| `sodium_ip2bin` | IPv4 with leading zeros `"01.02.03.04"` and `"001.002.003.004"` (<= 3 digits, value <= 255) | accepted, equals `1.2.3.4`; codecs.c:370-375 |
| `sodium_ip2bin` | IPv4 `"0.0.0.255"` / `"255.0.0.0"` (boundary octet values) | accepted; codecs.c:372 |
| `sodium_ip2bin` | IPv6 fully expanded `"0001:0002:0003:0004:0005:0006:0007:0008"` | returns `0`, big-endian group order; codecs.c:415-464 |
| `sodium_ip2bin` | IPv6 with leading zeros stripped `"1:2:3:4:5:6:7:8"` | same bin as the expanded form; codecs.c:449-456 |
| `sodium_ip2bin` | IPv6 `"::"` (all zeros) | returns `0`, 16 zero bytes; the `colonp` memmove/memset path with `n == 0`; codecs.c:408-414, 465-474 |
| `sodium_ip2bin` | IPv6 `"::1"` (compression at the start) | returns `0`, `bin[15]=1`; codecs.c:408-414, 465-474 |
| `sodium_ip2bin` | IPv6 `"1::"` (compression at the end) | returns `0`, `bin[1]=1`; codecs.c:418-439, 465-474 |
| `sodium_ip2bin` | IPv6 `"1::8"` and `"1:2::7:8"` (compression in the middle) | returns `0`; codecs.c:465-474 |
| `sodium_ip2bin` | IPv6 UPPERCASE `"FE80::1"` and mixed `"Fe80::AbCd"` (`ip_hex_digit` folds case via bitwise-or with `32U`) | accepted; codecs.c:345-355 |
| `sodium_ip2bin` | IPv6 exactly 4 hex digits per group `"ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"` (`xdigits` hits the limit of 4 but not >) | accepted -> 16x 0xff; codecs.c:450 |
| `sodium_ip2bin` | IPv6 with zone/scope id: `"fe80::1%eth0"`, `"fe80::1%1"`, `"fe80::1%en0.5"`, `"fe80::1%A_b-c"` | returns `0`; the zone is VALIDATED then DISCARDED (`end = zone`), so the resulting bin is identical to `"fe80::1"`; codecs.c:497-509 |
| `sodium_ip2bin` | IPv6 with embedded IPv4 `"::1.2.3.4"`, `"::ffff:1.2.3.4"`, `"64:ff9b::1.2.3.4"` | returns `0`; `"::ffff:1.2.3.4"` produces the SAME bin as `sodium_ip2bin("1.2.3.4")`; codecs.c:441-447 |
| `sodium_ip2bin` | `ip_len_` shorter than the string: `ip="1.2.3.4extra"`, `ip_len_=7` | returns `0` for `1.2.3.4`; the scan is bounded by `ip + ip_len_`; codecs.c:487, 494-496 |
| `sodium_ip2bin` | `ip_len_` larger than the NUL-terminated string, e.g. `ip="1.2.3.4"`, `ip_len_=64` | returns `0`; the scan stops at the first NUL; codecs.c:494-496 |
| `sodium_ip2bin` | `ip_len_` cutting a valid prefix out of a longer address: `ip="1:2:3:4:5:6:7:8"`, `ip_len_=3` (`"1:2"`) | rejected (too few groups) — demonstrates that `ip_len_` is authoritative; codecs.c:494-496, 475 |
| `sodium_bin2ip` | IPv4-mapped bin (`bin[0..9]=0, bin[10]=bin[11]=0xff`) with `bin[12..15]={1,2,3,4}`, `ip_maxlen=16` | returns `ip` = `"1.2.3.4"` (decimal, base 10, no leading zeros); codecs.c:564-578 |
| `sodium_bin2ip` | IPv4-mapped `"0.0.0.0"` (`bin[12..15]=0`) and `"255.255.255.255"` (len 15) with `ip_maxlen=16` | exact-fit boundary of the v4 branch; codecs.c:571-574 |
| `sodium_bin2ip` | IPv4-mapped bin with `bin[12..15]={0,0,0,0}` — note that `::ffff:0:0` matches the 12-byte prefix | rendered as `"0.0.0.0"`, NOT as an IPv6 literal; codecs.c:564 |
| `sodium_bin2ip` | all-zero bin, `ip_maxlen=3` (minimum that passes `ip_maxlen <= 2`) | returns `"::"` (len 2); codecs.c:561-563, 601-615 |
| `sodium_bin2ip` | bin = `::1`, `ip_maxlen=4` | returns `"::1"`; codecs.c:604-615 |
| `sodium_bin2ip` | bin with a SINGLE zero group, e.g. `1:0:2:3:4:5:6:7` | `best_len < 2` so NO `::` compression -> `"1:0:2:3:4:5:6:7"`; codecs.c:601-603 |
| `sodium_bin2ip` | bin with two zero runs of different lengths, e.g. `1:0:0:2:0:0:0:3` | the LONGEST run wins -> `"1:0:0:2::3"`; codecs.c:589-600 |
| `sodium_bin2ip` | bin with two zero runs of EQUAL length, e.g. `1:0:0:2:0:0:3:4` | the strict `>` comparison keeps the FIRST run -> `"1::2:0:0:3:4"`; codecs.c:589-600 |
| `sodium_bin2ip` | zero run at the very end, e.g. `1:2:3:4:5:6:0:0` | the trailing-run fixup after the loop applies -> `"1:2:3:4:5:6::"`; codecs.c:597-600, 611 |
| `sodium_bin2ip` | zero run at the very start, e.g. `0:0:1:2:3:4:5:6` | `"::1:2:3:4:5:6"`; codecs.c:604-610 |
| `sodium_bin2ip` | bin with high nibbles set, e.g. all 0xff | `"ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"` (len 39) — lowercase hex, no leading zeros, needs `ip_maxlen >= 40`; codecs.c:531-547, 614 |
| `sodium_bin2ip` | bin group values 0x0001 / 0x0010 / 0x0100 / 0x1000 | leading zeros are suppressed by `ip_write_num` (`"1"`, `"10"`, `"100"`, `"1000"`); codecs.c:531-547 |
| `sodium_bin2ip` | `ip_maxlen` exactly `len + 1` vs much larger (e.g. 46) | only `len + 1` bytes are written; the v4 branch does `memcpy(ip, buf, len+1)` then `ip[len]=0`, the v6 branch does `memcpy(ip, buf, len)` then `ip[len]=0`; codecs.c:575-576, 620-621 |
| `sodium_ip2bin` + `sodium_bin2ip` | round-trip over the whole matrix above (v4 dotted-quad, v4-mapped, full v6, compressed v6, `::`, zone-suffixed) | `bin2ip(ip2bin(x))` is the CANONICAL form, not necessarily `x` (zone dropped, zeros compressed, lowercase); codecs.c:483-624 |
| `sodium_pad` | `blocksize=1` x `unpadded_buflen=0,1,5,64`; `max_buflen = unpadded_buflen + 1` | `xpadlen=0`, `*padded_buflen_p = unpadded_buflen + 1`, a single `0x80` appended; utils.c:758-782 |
| `sodium_pad` | `blocksize=2` x `unpadded_buflen=0,1,2,3` -> padded 2,2,4,4 | power-of-two mask path; utils.c:759-760 |
| `sodium_pad` | `blocksize=16` x `unpadded_buflen=0,1,15,16,17,32` -> padded 16,16,16,32,32,48 | a whole extra block is added when `unpadded_buflen % blocksize == 0`; utils.c:758-770 |
| `sodium_pad` | `blocksize=64` x `unpadded_buflen=0,1,63,64,65` -> padded 64,64,64,128,128 | utils.c:758-770 |
| `sodium_pad` | non-power-of-two `blocksize=3` x `unpadded_buflen=0,1,2,3,4` (takes the `%` branch instead of the mask branch) | utils.c:761-762 |
| `sodium_pad` | non-power-of-two `blocksize=17` x `unpadded_buflen=0,16,17,18` | utils.c:761-762 |
| `sodium_pad` | `padded_buflen_p = NULL` with an otherwise valid call | allowed — the pointer IS NULL-checked, so padding is applied and the length simply is not reported; utils.c:772-774 |
| `sodium_pad` | `max_buflen` exactly `xpadded_len + 1` (the minimum accepted) vs much larger | only `blocksize` bytes at `buf[xpadded_len - blocksize + 1 .. xpadded_len]` are touched; utils.c:768-781 |
| `sodium_pad` | `unpadded_buflen=0`, `blocksize=16`, `max_buflen=16`: the loop writes over `buf[0..15]` | the constant-time loop rewrites the entire trailing block, so `buf` must be at least `xpadded_len + 1` bytes; utils.c:771, 776-781 |
| `sodium_unpad` | `blocksize=1` x `padded_buflen=1,2,64` with the last byte `0x80` | returns `0`, `*unpadded_buflen_p = padded_buflen - 1`; utils.c:800-812 |
| `sodium_unpad` | `blocksize=16`, `padded_buflen=16`, `buf` = 15 data bytes + `0x80` (`pad_len = 0`) | returns `0`, `*unpadded_buflen_p = 15`; utils.c:802-810 |
| `sodium_unpad` | `blocksize=16`, `padded_buflen=16`, `buf` = `0x80` followed by 15 zeros (`pad_len = 15`, maximum) | returns `0`, `*unpadded_buflen_p = 0`; utils.c:802-810 |
| `sodium_unpad` | `padded_buflen == blocksize` exactly (the boundary of the `padded_buflen < blocksize` test) | accepted; utils.c:797 |
| `sodium_unpad` | `padded_buflen` much larger than `blocksize` (e.g. 1024 and 16) | only the last `blocksize` bytes are inspected; utils.c:802-809 |
| `sodium_unpad` | `blocksize` DIFFERENT from the one used by `sodium_pad`, e.g. pad with 16 then unpad with 64 on a 32-byte buffer | scans a larger window; succeeds only if the `0x80` is within the last `blocksize` bytes and everything after it is zero; utils.c:802-812 |
| `sodium_pad` + `sodium_unpad` | round-trip matrix: `blocksize` in {1,2,3,16,17,64} x `unpadded_buflen` in {0,1,blocksize-1,blocksize,blocksize+1,1000} | must recover the original length exactly; utils.c:744-813 |
| `sodium_memcmp` | `len` in {0,1,8,16,32,33}; equal buffers; differing at the first byte; differing at the last byte; differing in every byte | `len=0` -> `0`; constant-time accumulate-then-collapse, no length special cases (`HAVE_WEAK_SYMBOLS` is undefined so the volatile-pointer variant is compiled); utils.c:186-208 |
| `sodium_memcmp` | all-zero vs all-zero, and all-0xff vs all-0xff, `len=32` | returns `0`; utils.c:204-207 |
| `sodium_compare` | `len=0` (both pointers may be anything) | returns `0`; the `while (i != 0U)` loop never runs; utils.c:246-254 |
| `sodium_compare` | `len=1`: `{0}` vs `{1}` -> `-1`; `{1}` vs `{0}` -> `1`; `{7}` vs `{7}` -> `0` | utils.c:247-254 |
| `sodium_compare` | LITTLE-ENDIAN ordering check, `len=2`: `b1={0xff,0x00}` vs `b2={0x00,0x01}` -> `-1` (index `len-1` is the most significant byte) | the scan runs from `len-1` downwards; utils.c:246-253 |
| `sodium_compare` | `len=32`: all-zero vs all-0xff -> `-1`; all-0xff vs all-zero -> `1`; equal -> `0` | utils.c:247-254 |
| `sodium_compare` | `len` in {8,16,32,33} with the difference in the least-significant byte only (index 0) | exercises the `eq` mask propagation across the full length; utils.c:251-252 |
| `sodium_is_zero` | `nlen=0` -> `1`; all-zero `nlen=1,8,16,32,33` -> `1`; single `1` at index 0, middle, and `nlen-1` -> `0` | utils.c:257-267 |
| `sodium_increment` | `nlen=0` | no-op (loop body never runs); utils.c:309-313 |
| `sodium_increment` | `nlen=1`: `{0x00}` -> `{0x01}`; `{0xfe}` -> `{0xff}`; `{0xff}` -> `{0x00}` (wrap) | utils.c:309-313 |
| `sodium_increment` | `nlen=8`, all 0xff -> all 0x00 (full carry chain and wrap-around) | `HAVE_AMD64_ASM` is UNDEFINED, so the `nlen == 8/12/24` asm fast paths are dead and the generic loop is used for every length; utils.c:275-313 |
| `sodium_increment` | `nlen=8,12,24` (the lengths that would take asm paths in an asm build), values `{0xff,0x00,...}` -> `{0x00,0x01,...}` | must match the generic little-endian increment; utils.c:275-313 |
| `sodium_increment` | `nlen=16,32,33` with `{0xff x nlen}` and `{0xff,0xff,0x00,...}` | partial-carry propagation stops at the first non-0xff byte; utils.c:309-313 |
| `sodium_add` | `len=0` -> no-op; `len=1`: `a={0xff}, b={0x01}` -> `{0x00}` | utils.c:358-362 |
| `sodium_add` | `len=8`: `a` all 0xff, `b={1,0,...,0}` -> all 0x00 (carry through every byte) | utils.c:358-362 |
| `sodium_add` | `len=8`: `a = b =` all 0xff -> `{0xfe,0xff,...,0xff}` | utils.c:358-362 |
| `sodium_add` | `len=8,12,24` (would-be asm sizes) and `len=16,32,33` | generic loop only (`HAVE_AMD64_ASM` undefined); utils.c:322-362 |
| `sodium_add` | aliasing: `a == b`, `len=32`, all bytes 0x80 (doubling in place) | utils.c:358-362 |
| `sodium_sub` | `len=0` -> no-op; `len=1`: `a={0x00}, b={0x01}` -> `{0xff}` (borrow wraps) | utils.c:400-404 |
| `sodium_sub` | `len=8`: `a` all 0x00, `b={1,0,...}` -> all 0xff (borrow through every byte) | utils.c:400-404 |
| `sodium_sub` | `len=8`: `a = b` -> all 0x00 | utils.c:400-404 |
| `sodium_sub` | `len=64` (the would-be asm size) and `len=16,32,33` | generic loop only (`HAVE_AMD64_ASM` undefined); utils.c:371-404 |
| `sodium_memzero` | `len=0` (with any `pnt`, including NULL); `len=1,32,4096` | the portable `volatile unsigned char` loop is the live branch (all of `_WIN32`, `HAVE_MEMSET_S`, `HAVE_EXPLICIT_BZERO`, `HAVE_MEMSET_EXPLICIT`, `HAVE_EXPLICIT_MEMSET`, `HAVE_WEAK_SYMBOLS` are undefined); utils.c:148-156 |
| `sodium_stackzero` | `len=0`, `len=512`, `len=SIZE_MAX` | ALL are no-ops: with `HAVE_C_VARARRAYS` and `HAVE_ALLOCA` both undefined the function body compiles away entirely; utils.c:159-168 |
| `sodium_malloc` | `size=0` | non-NULL (`malloc(1)` via the `size > 0 ? size : 1` fallback), and `memset(ptr, 0xdb, 0)` writes nothing; utils.c:590-594, 639-650 |
| `sodium_malloc` | `size=1,16,32,4096,65536` | non-NULL, EVERY byte pre-filled with `GARBAGE_VALUE = 0xdb` (note: the header comment claims 0xd0; the code uses 0xdb); utils.c:69, 647 |
| `sodium_malloc` | NO guard page / canary / page rounding in this build (`HAVE_ALIGNED_MALLOC` undefined) | the returned pointer is a plain `malloc` pointer; reading or writing one byte past the region does NOT fault, and `sodium_free` performs no canary check; utils.c:589-594, 662-667 |
| `sodium_allocarray` | `count=0, size=0` and `count=0, size=1000` | the overflow check is skipped when `count == 0`, so both return a non-NULL 1-byte allocation; utils.c:655-659 |
| `sodium_allocarray` | `count=4, size=8` -> 32 bytes; `count=1, size=1` -> 1 byte; `count=1000, size=32` | non-NULL, all bytes 0xdb; utils.c:655-659 |
| `sodium_allocarray` | just below the overflow threshold: `count=2, size=SIZE_MAX/2 - 1` | passes the `size >= SIZE_MAX/count` test and then fails in `malloc` -> NULL; utils.c:655-659 |
| `sodium_free` | `sodium_free(NULL)`; `sodium_free(sodium_malloc(0))`; `sodium_free(sodium_malloc(n))`; `sodium_free(sodium_allocarray(c,s))` | all are no-ops / plain `free`; double-free is UB; utils.c:662-667 |
| `sodium_mlock` / `sodium_munlock` | any `addr`/`len`, before or after `sodium_init()`, on stack, heap or `sodium_malloc` memory; `len=0` | always `-1` with `errno=ENOSYS`; `sodium_munlock` additionally ZEROES the buffer before failing; utils.c:432-463 |
| `sodium_mprotect_noaccess` / `_readonly` / `_readwrite` | full lifecycle: `p = sodium_malloc(32)`, then noaccess, readonly, readwrite, then `sodium_free(p)`; also on a non-sodium pointer | every call returns `-1` with `errno=ENOSYS` and the memory remains fully readable/writable throughout; utils.c:701-742 |
| `sodium_init` | FIRST call in the process | returns `0`; runs `_sodium_runtime_get_cpu_features()` (which returns -1 and is ignored), `randombytes_stir()`, `_sodium_alloc_init()` (fills the 16-byte canary via `randombytes_buf`) and all `_pick_best_implementation()` hooks; core.c:27-56 |
| `sodium_init` | SECOND and subsequent calls | returns `1` (idempotent, `initialized != 0` short-circuit); the crit-section stubs always succeed so `-1` is unreachable; core.c:33-38 |
| `sodium_init` | calling a G1 API (e.g. `sodium_bin2hex`, `randombytes_buf`) WITHOUT `sodium_init()` first | works: none of the G1 entry points require initialization; `randombytes_*` self-initializes via `randombytes_init_if_needed`; randombytes.c:130-146 |
| `sodium_set_misuse_handler` | install a handler, then trigger a misuse (e.g. `sodium_bin2hex(h, 0, b, 0)`) | handler runs first, then `abort()` still fires unless the handler `longjmp`s / `_exit`s; returns `0`; core.c:191-219 |
| `sodium_set_misuse_handler` | `sodium_set_misuse_handler(NULL)` (clear), then trigger a misuse | returns `0`; misuse goes straight to `abort()`; core.c:199, 204 |
| `sodium_runtime_has_neon` / `_armcrypto` / `_sse2` / `_sse3` / `_ssse3` / `_sse41` / `_avx` / `_avx2` / `_avx512f` / `_pclmul` / `_aesni` / `_rdrand` | called BEFORE `sodium_init()` and AFTER `sodium_init()`, on any x86-64 host | ALL return `0` in both cases: `_cpuid()` takes the `#else` stub that zero-fills `cpu_info`, so `_sodium_runtime_intel_cpu_features` bails at `cpu_info[0] == 0`; runtime.c:195-198, 207-210, 330-400 |
| `_sodium_runtime_get_cpu_features` | called directly, repeatedly | returns `-1` every time; sets `_cpu_features.initialized = 1`; runtime.c:318-328 |
| `sodium_version_string` | any time, before or after `sodium_init()` | returns the static string `"1.0.23"`; version.c:4-8, version.h:7 |
| `sodium_library_version_major` / `sodium_library_version_minor` | any time | `30` and `0`; version.c:10-20, version.h:9-10 |
| `sodium_library_minimal` | any time (`SODIUM_LIBRARY_MINIMAL` is NOT defined) | returns `0`; version.c:22-30 |
| `randombytes_seedbytes` | any time | returns `32` (`randombytes_SEEDBYTES`); randombytes.c:229-233 |
| `randombytes_implementation_name` | default state (no `randombytes_set_implementation` call) | returns `"sysrandom"` (`RANDOMBYTES_DEFAULT_IMPLEMENTATION = &randombytes_sysrandom_implementation`, since neither `RANDOMBYTES_CUSTOM_IMPLEMENTATION` nor `__EMSCRIPTEN__` is defined); randombytes.c:29-35, 155-160 |
| `randombytes_implementation_name` | after `randombytes_set_implementation(&randombytes_internal_implementation)` | returns `"internal"`; randombytes_internal_random.c:650-663 |
| `randombytes_implementation_name` | after `randombytes_set_implementation(&randombytes_sysrandom_implementation)` (explicitly re-installing the default) | returns `"sysrandom"`; randombytes_sysrandom.c:385-398 |
| `randombytes_set_implementation` | `randombytes_set_implementation(NULL)` then any `randombytes_*` call | returns `0`, but leaves `implementation == NULL`, so `randombytes_init_if_needed()` silently RE-INSTALLS the default and re-stirs; randombytes.c:133-153 |
| `randombytes_set_implementation` | custom impl with `stir == NULL` | `randombytes_stir()` becomes a no-op (member IS NULL-checked); `randombytes_init_if_needed` still works; randombytes.c:169-176 |
| `randombytes_set_implementation` | custom impl with `close == NULL` | `randombytes_close()` returns `0` (member IS NULL-checked); randombytes.c:235-242 |
| `randombytes_set_implementation` | custom impl with `uniform == NULL` (as both built-in impls have) | `randombytes_uniform` uses the generic modulo-rejection algorithm over `implementation->random()`; randombytes.c:185-198 |
| `randombytes_set_implementation` | custom impl with `uniform != NULL` | `randombytes_uniform(ub)` delegates unconditionally — even for `ub = 0` or `1`, so the `< 2 -> 0` shortcut is BYPASSED; randombytes.c:185-187 |
| `randombytes_uniform` | `upper_bound = 0` and `upper_bound = 1`, with the default impl (`uniform == NULL`) | returns `0` without consuming any randomness; randombytes.c:188-190 |
| `randombytes_uniform` | `upper_bound = 2, 3, 10, 256, 0x7fffffff` | `min = (1U + ~upper_bound) % upper_bound`, rejection loop, result `< upper_bound`; randombytes.c:191-198 |
| `randombytes_uniform` | `upper_bound = 0x80000001` (2^31+1, the documented worst case, `min = 0x7fffffff`) | ~2 draws on average; result `< upper_bound`; randombytes.c:191-198 |
| `randombytes_uniform` | `upper_bound = 0x40000000` (a power of two, `min = 0`) and `upper_bound = 0xffffffff` | powers of two never reject; randombytes.c:191-194 |
| `randombytes_random` | default impl; and with a custom deterministic `random` callback | returns the raw 32-bit value from the impl, no post-processing; randombytes.c:162-167 |
| `randombytes_buf` | `size = 0` (with any `buf`, even NULL) | `implementation->buf` is NOT called at all; randombytes.c:204-207 |
| `randombytes_buf` | `size = 1, 32, 256, 257, 4096` with the default sysrandom impl | the getrandom path chunks at 256 bytes per syscall (`chunk_size = 256U`); randombytes_sysrandom.c:242-260 |
| `randombytes_buf_deterministic` | `size` in {0, 1, 63, 64, 65, 1000} with a fixed seed | ChaCha20-IETF keystream with the fixed 12-byte nonce `"LibsodiumDRG"` and the seed as the key; output for smaller `size` is a strict PREFIX of the output for larger `size`; randombytes.c:210-227 |
| `randombytes_buf_deterministic` | `size = 0` | writes nothing, no misuse (the `size > 0x4000000000ULL` guard passes); randombytes.c:219-226 |
| `randombytes_buf_deterministic` | the same `size` with different seeds: all-zero seed, all-0xff seed, `seed[i] = i`, and two seeds differing in one bit | different keystreams; identical seed+size must reproduce byte-for-byte; randombytes.c:225-226 |
| `randombytes_buf_deterministic` | independent of `implementation` — call it before/after `randombytes_set_implementation(...)` and before/after `sodium_init()` | it never touches `implementation` and never calls `randombytes_init_if_needed()`; randombytes.c:210-227 |
| `randombytes` (NaCl API) | `buf_len = 0` (no impl call) and `buf_len = 32` | thin wrapper over `randombytes_buf`; randombytes.c:244-249 |
| `randombytes_stir` | first call (installs the default impl and stirs) vs repeated calls | with sysrandom, the second `stir` is a no-op because `stream.initialized != 0`; randombytes.c:169-176, randombytes_sysrandom.c:296-303 |
| `randombytes_close` | default sysrandom impl AFTER at least one `randombytes_*` call (so `getrandom_available == 1`) | returns `0`; `stream.initialized` is NOT reset on the getrandom path (the `fd != -1` branch is not taken), so subsequent calls still work; randombytes_sysrandom.c:313-337 |
| `randombytes_close` | called twice in a row after initialization | both calls return `0`; randombytes_sysrandom.c:325-329 |
| `randombytes_close` | internal impl after at least one use (`global.getrandom_available == 1`) | returns `0` AND `sodium_memzero(&stream, ...)` clears the TLS state, so the next call re-stirs (`stream.initialized == 0`); note `global.initialized` stays `1` in this build; randombytes_internal_random.c:515-546 |
| `randombytes_internal_implementation.buf` | `size = 0` via `randombytes_buf` (not called) vs via the exported struct directly (chacha20 with length 0, no assert) | randombytes_internal_random.c:588-612 |
| `randombytes_internal_implementation.random` | repeated calls exhausting the 16-block pool (`rnd32_outleft` reaches 0 after `(16*32 - 32)/4 = 120` draws) then refilling | the refill path re-keys via `xorkey` and increments the nonce; randombytes_internal_random.c:620-648 |
| `randombytes_*` | default (sysrandom) on Linux x86-64 takes the `HAVE_LINUX_COMPATIBLE_GETRANDOM` path via `syscall(SYS_getrandom, ...)`, NOT `/dev/urandom` | `__linux__` and `SYS_getrandom`/`__NR_getrandom` are platform macros, not `HAVE_*`, so this branch IS live; `BLOCK_ON_DEV_RANDOM` is also live but only used by the `/dev/random` fallback; randombytes_sysrandom.c:23-47, 227-261 |
