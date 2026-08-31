## Area 1 — sodium core + randombytes

Configuration surface: valid-input combinations that the compiled C actually branches on.
Same build assumptions as `errors_1.md` (x86-64 Linux, no `HAVE_*`, `-std=c99`, asserts live, `HAVE_LINUX_COMPATIBLE_GETRANDOM` and `BLOCK_ON_DEV_RANDOM` active, `page_size == DEFAULT_PAGE_SIZE == 0x10000`, `CANARY_SIZE == 16`, `GARBAGE_VALUE == 0xdb`, `TLS` empty).

### version.h / runtime.h / core.h

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.1 | `sodium_version_string` | any call | [x] |
| 1.2 | `sodium_library_version_major` / `sodium_library_version_minor` | any call → `SODIUM_LIBRARY_VERSION_MAJOR == 30`, `MINOR == 0` | [x] |
| 1.3 | `sodium_library_minimal` | `SODIUM_LIBRARY_MINIMAL` **undefined** in this build → the `#else` arm → `0` | [x] |
| 1.4 | `sodium_init` | very first call in the process (`initialized == 0`) → runs all `_pick_best_implementation` hooks + `randombytes_stir()` + `_sodium_alloc_init()` → `0` | [x] |
| 1.5 | `sodium_init` | second and subsequent calls (`initialized != 0`) → early-out → `1` | [x] |
| 1.6 | `sodium_init` | called *after* a lazily-initialising randombytes call (e.g. `randombytes_buf` first) — `randombytes_stir()` runs a second time on an already-stirred implementation | [x] |
| 1.7 | `sodium_crit_enter` / `sodium_crit_leave` | no-op `#else` implementations: enter-then-leave, leave without enter, nested enter, leave twice — all `0`, no `locked` bookkeeping | [x] |
| 1.8 | `sodium_set_misuse_handler` | `handler == NULL` (clears the handler) → a later `sodium_misuse()` goes straight to `abort()` | [x] |
| 1.9 | `sodium_set_misuse_handler` | `handler != NULL` → a later `sodium_misuse()` calls the handler first, then `abort()` (handler cannot prevent the abort by returning) | [x] |
| 1.10 | `sodium_set_misuse_handler` | called twice — second handler replaces the first | [x] |
| 1.11 | `_sodium_runtime_get_cpu_features` | first call vs. repeated calls; `_cpu_features.initialized` set to 1 either way; return value always `-1` | [x] |
| 1.12 | `sodium_runtime_has_*` (all 12) | queried before `sodium_init()` (static zero state) vs. after `sodium_init()` (still all zero, since both probes bail out early) — must be identical | [x] |

### utils.c — memzero / stackzero

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.13 | `sodium_memzero` | `len == 0` → the volatile `while (i < len)` loop body never executes; `pnt` is never dereferenced (a dangling/NULL `pnt` with `len == 0` is safe) | [x] |
| 1.14 | `sodium_memzero` | `len == 1` | [x] |
| 1.15 | `sodium_memzero` | `len` not a multiple of the word size (e.g. 7, 17, 31) — the compiled path is a byte-at-a-time volatile loop, so no alignment special-casing | [x] |
| 1.16 | `sodium_memzero` | `len` large (e.g. 4096) and/or a sub-slice of a larger buffer — bytes outside `[pnt, pnt+len)` must be untouched | [x] |
| 1.17 | `sodium_stackzero` | any `len` including 0 — compiled body is **empty** (no `HAVE_C_VARARRAYS`, no `HAVE_ALLOCA`); observable behaviour is "does nothing" | [x] |

### utils.c — constant-time compare / is_zero

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.18 | `sodium_memcmp` | `len == 0` → `d` stays 0 → `0` (equal), pointers never dereferenced | [x] |
| 1.19 | `sodium_memcmp` | `len == 1`, equal | [x] |
| 1.20 | `sodium_memcmp` | `b1_ == b2_` (same pointer), `len > 0` → `0` | [x] |
| 1.21 | `sodium_memcmp` | equal buffers at the crypto-relevant sizes 16 / 32 / 64 | [x] |
| 1.22 | `sodium_memcmp` | differ only in byte 0 / only in the last byte / in every byte → `-1` in all cases | [x] |
| 1.23 | `sodium_memcmp` | `b1` all-`0x00` vs `b2` all-`0xff` → `-1` (exercises the `(d - 1) >> 8` sentinel with `d == 0xff`) | [x] |
| 1.24 | `sodium_compare` | `len == 0` → `gt == 0`, `eq == 1` → `0` | [x] |
| 1.25 | `sodium_compare` | equal buffers, `len` 1 / 8 / 32 → `0` | [x] |
| 1.26 | `sodium_compare` | differ only in the **highest** index (most significant, little-endian) — `b1 < b2` → `-1`; swapped → `1` | [x] |
| 1.27 | `sodium_compare` | differ only in index 0 (least significant) — proves little-endian ordering, not `memcmp` ordering | [x] |
| 1.28 | `sodium_compare` | `b1` all-`0x00` vs `b2` all-`0xff` → `-1`; and `0xff…` vs `0x00…` → `1` | [x] |
| 1.29 | `sodium_compare` | `len` 8 / 12 / 24 / 32 / 64 — the `HAVE_AMD64_ASM` fast paths are **not** compiled, so all lengths take the identical portable loop; verify no length is special | [x] |
| 1.30 | `sodium_is_zero` | `nlen == 0` → `1` (vacuously zero) | [x] |
| 1.31 | `sodium_is_zero` | all-zero buffer, `nlen` 1 / 16 / 32 → `1` | [x] |
| 1.32 | `sodium_is_zero` | single non-zero byte at index 0 / at the last index / `0x01` only → `0` | [x] |
| 1.33 | `sodium_is_zero` | all-`0xff` → `0` | [x] |

### utils.c — increment / add / sub (portable loop only; the `HAVE_AMD64_ASM` 8/12/24/64-byte paths are NOT compiled)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.34 | `sodium_increment` | `nlen == 0` → no-op, `n` never dereferenced | [x] |
| 1.35 | `sodium_increment` | `nlen == 1`, `n == {0x00}` → `{0x01}` | [x] |
| 1.36 | `sodium_increment` | `nlen == 1`, `n == {0xff}` → `{0x00}` (wrap, carry dropped) | [x] |
| 1.37 | `sodium_increment` | carry across a single byte boundary: `{0xff, 0x00}` → `{0x00, 0x01}` | [x] |
| 1.38 | `sodium_increment` | `nlen == 8` / `12` / `24` (the ex-asm sizes) with `n` all-`0xff` → all-`0x00`; and `n` all-`0x00` → `{0x01, 0, …}` | [x] |
| 1.39 | `sodium_increment` | `nlen == 32` (nonce/counter size), full carry chain from all-`0xff` | [x] |
| 1.40 | `sodium_add` | `len == 0` → no-op | [x] |
| 1.41 | `sodium_add` | `len == 1`, no carry (`0x01 + 0x01`) and with wrap (`0xff + 0x01` → `0x00`) | [x] |
| 1.42 | `sodium_add` | `len == 8` / `12` / `24` / `32` / `64` (ex-asm sizes) — `a` all-`0xff` plus `b == {0x01, 0, …}` → all-zero | [x] |
| 1.43 | `sodium_add` | full carry chain: `a` all-`0xff`, `b` all-`0xff` | [x] |
| 1.44 | `sodium_add` | `a == b` (aliasing the same buffer) — doubling | [x] |
| 1.45 | `sodium_sub` | `len == 0` → no-op | [x] |
| 1.46 | `sodium_sub` | `len == 1`, no borrow (`0x02 - 0x01`) and with borrow (`0x00 - 0x01` → `0xff`) | [x] |
| 1.47 | `sodium_sub` | `len == 64` (the ex-asm size) — `a` all-zero minus `b == {0x01, 0, …}` → all-`0xff` (borrow propagates the whole length) | [x] |
| 1.48 | `sodium_sub` | `a == b` bytewise → all-zero result; and `a` and `b` the **same pointer** | [x] |
| 1.49 | `sodium_sub` | `len` 8 / 24 / 32 (not the asm size) — must be identical to `len == 64` semantics | [x] |

### codecs.c — hex

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.50 | `sodium_bin2hex` | `bin_len == 0`, `hex_maxlen == 1` → writes only the terminating NUL, returns `hex` | [x] |
| 1.51 | `sodium_bin2hex` | `hex_maxlen == 2 * bin_len + 1` exactly (minimum accepted size) | [x] |
| 1.52 | `sodium_bin2hex` | `hex_maxlen > 2 * bin_len + 1` — only `2*bin_len + 1` bytes are written, the tail of `hex` must be untouched | [x] |
| 1.53 | `sodium_bin2hex` | bin bytes covering both nibble branches: `0x00`, `0x09`, `0x0a`, `0x0f`, `0x10`, `0xa0`, `0xff` → lowercase `0-9a-f` output, low nibble first in the pair | [x] |
| 1.54 | `sodium_hex2bin` | `ignore == NULL`, `bin_len != NULL`, `hex_end != NULL`, even-length all-valid lowercase hex, `bin_maxlen == hex_len / 2` exactly | [x] |
| 1.55 | `sodium_hex2bin` | `bin_len == NULL` **and** `hex_end == NULL` (both out-params omitted) with fully-consumed valid input → `0` | [x] |
| 1.56 | `sodium_hex2bin` | `bin_len != NULL`, `hex_end == NULL` — strict mode: all of `hex` must be consumed | [x] |
| 1.57 | `sodium_hex2bin` | `bin_len == NULL`, `hex_end != NULL` — the end pointer is set but no length is reported | [x] |
| 1.58 | `sodium_hex2bin` | `hex_len == 0` → `0`, `*bin_len == 0`, `*hex_end == hex` | [x] |
| 1.59 | `sodium_hex2bin` | uppercase hex (`"AB"`), lowercase (`"ab"`), and mixed (`"aB"`) — the `c & ~32U` branch | [x] |
| 1.60 | `sodium_hex2bin` | `bin_maxlen == 0` with `hex_len == 0` → `0`; `bin` never written | [x] |
| 1.61 | `sodium_hex2bin` | `ignore = ":"` with separators **between** byte pairs (`"aa:bb:cc"`) — skipped only while `state == 0` | [x] |
| 1.62 | `sodium_hex2bin` | `ignore = ":"` with a separator **inside** a byte pair (`"a:abb"`) — `state != 0` so the ignore branch is *not* taken; the char terminates the scan | [x] |
| 1.63 | `sodium_hex2bin` | `ignore = " \n"` with leading ignorable chars (`"  aabb"`) — skipped at the very start | [x] |
| 1.64 | `sodium_hex2bin` | `ignore` non-NULL with **trailing** ignorable chars (`"aabb  "`) and `hex_end != NULL` → `0` with `*hex_end` pointing **at** the trailing chars (unlike `sodium_base642bin`, `hex2bin` does *not* skip trailing ignore chars) | [x] |
| 1.65 | `sodium_hex2bin` | `ignore` containing a character that is itself a hex digit (e.g. `ignore = "a"`) — the hex-digit branch wins, the ignore set is only consulted for non-hex chars | [x] |
| 1.66 | `sodium_hex2bin` | `ignore = ""` (empty, non-NULL) — behaves like `ignore == NULL` because `strchr("", c)` is NULL for any non-NUL `c` | [x] |
| 1.67 | `sodium_hex2bin` | valid hex followed by a non-hex char with `hex_end != NULL` (`"aabbZZ"`) → `0`, `*bin_len == 2`, `*hex_end` at the `'Z'` | [x] |
| 1.68 | `sodium_hex2bin` | embedded NUL inside `hex` with `hex_len` spanning it — the NUL is not a hex digit, so it terminates the scan (and `strchr(ignore, 0)` would match the ignore string's own terminator when `ignore != NULL` — worth pinning) | [x] |

### codecs.c — base64 (4 variants × input shape)

`sodium_base64_VARIANT_ORIGINAL = 1`, `ORIGINAL_NO_PADDING = 3`, `URLSAFE = 5`, `URLSAFE_NO_PADDING = 7`.
`VARIANT_NO_PADDING_MASK = 0x2`, `VARIANT_URLSAFE_MASK = 0x4`. Valid variants are exactly `{1,3,5,7}`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.69 | `sodium_base64_encoded_len` / `sodium_base64_ENCODED_LEN` (macro) | VARIANT_ORIGINAL, `bin_len == 0` → `1`; and `bin_len % 3 == 0, > 0` → `4*(n/3) + 1` | [x] |
| 1.70 | `sodium_base64_encoded_len` / macro | VARIANT_ORIGINAL, `bin_len % 3 == 1` → `4*(n/3) + 4 + 1` | [x] |
| 1.71 | `sodium_base64_encoded_len` / macro | VARIANT_ORIGINAL, `bin_len % 3 == 2` → `4*(n/3) + 4 + 1` | [x] |
| 1.72 | `sodium_base64_encoded_len` / macro | VARIANT_ORIGINAL_NO_PADDING, `bin_len % 3 == 0 / 1 / 2` → `+0 / +2 / +3` before the `+1` | [x] |
| 1.73 | `sodium_base64_encoded_len` / macro | VARIANT_URLSAFE — identical lengths to ORIGINAL (URLSAFE bit does not affect length) | [x] |
| 1.74 | `sodium_base64_encoded_len` / macro | VARIANT_URLSAFE_NO_PADDING — identical lengths to ORIGINAL_NO_PADDING | [x] |
| 1.75 | `sodium_base64_encoded_len` vs. the `sodium_base64_ENCODED_LEN` macro | same `(bin_len, variant)` pairs must agree; the macro additionally clamps to `SIZE_MAX` where the function aborts | [x] |
| 1.76 | `sodium_bin2base64` | VARIANT_ORIGINAL, `bin_len == 0`, `b64_maxlen == 1` → empty string | [x] |
| 1.77 | `sodium_bin2base64` | VARIANT_ORIGINAL, `bin_len % 3 == 0` (3, 6, 300) — no padding needed even in a padded variant | [x] |
| 1.78 | `sodium_bin2base64` | VARIANT_ORIGINAL, `bin_len % 3 == 1` → 2 trailing `'='` | [x] |
| 1.79 | `sodium_bin2base64` | VARIANT_ORIGINAL, `bin_len % 3 == 2` → 1 trailing `'='` | [x] |
| 1.80 | `sodium_bin2base64` | VARIANT_ORIGINAL_NO_PADDING, `bin_len % 3 == 0` (identical output to 1.77) | [x] |
| 1.81 | `sodium_bin2base64` | VARIANT_ORIGINAL_NO_PADDING, `bin_len % 3 == 1` → `b64_len += 2 + (1 >> 1) == 2` chars, no `'='` | [x] |
| 1.82 | `sodium_bin2base64` | VARIANT_ORIGINAL_NO_PADDING, `bin_len % 3 == 2` → `b64_len += 2 + (2 >> 1) == 3` chars, no `'='` | [x] |
| 1.83 | `sodium_bin2base64` | VARIANT_URLSAFE, `bin_len % 3 == 0 / 1 / 2` — `b64_byte_to_urlsafe_char`, `'-'`/`'_'` for 62/63, padded | [x] |
| 1.84 | `sodium_bin2base64` | VARIANT_URLSAFE_NO_PADDING, `bin_len % 3 == 0 / 1 / 2` | [x] |
| 1.85 | `sodium_bin2base64` | bin bytes chosen so 6-bit indices `62` and `63` occur (e.g. `{0xfb, 0xef, 0xbe}`) — verifies `'+'`/`'/'` for ORIGINAL vs `'-'`/`'_'` for URLSAFE | [x] |
| 1.86 | `sodium_bin2base64` | bin bytes covering index ranges `<26` (`A-Z`), `26..51` (`a-z`), `52..61` (`0-9`) — the three `LT`/`GE` branches of `b64_byte_to_char` | [x] |
| 1.87 | `sodium_bin2base64` | `b64_maxlen == b64_len + 1` exactly (minimum accepted) → single NUL written | [x] |
| 1.88 | `sodium_bin2base64` | `b64_maxlen > b64_len + 1` — the `do { b64[b64_pos++] = 0; } while (b64_pos < b64_maxlen)` loop **zero-fills the entire remainder of `b64_maxlen`**, not just one NUL byte | [x] |
| 1.89 | `sodium_base642bin` | VARIANT_ORIGINAL, `b64_len % 4 == 0` with no `'='` (whole-block input), `ignore == NULL`, `bin_len != NULL`, `b64_end != NULL` | [x] |
| 1.90 | `sodium_base642bin` | VARIANT_ORIGINAL, input ending in one `'='` (`acc_len == 2` → 1 pad char consumed by `_sodium_base642bin_skip_padding`) | [x] |
| 1.91 | `sodium_base642bin` | VARIANT_ORIGINAL, input ending in two `'=='` (`acc_len == 4` → 2 pad chars consumed) | [x] |
| 1.92 | `sodium_base642bin` | VARIANT_ORIGINAL, `b64_len == 0` → `0`, `*bin_len == 0`, `*b64_end == b64` | [x] |
| 1.93 | `sodium_base642bin` | VARIANT_ORIGINAL_NO_PADDING with 2 trailing chars (`acc_len == 4`) — `skip_padding` is skipped entirely thanks to `VARIANT_NO_PADDING_MASK` | [x] |
| 1.94 | `sodium_base642bin` | VARIANT_ORIGINAL_NO_PADDING with 3 trailing chars (`acc_len == 2`) | [x] |
| 1.95 | `sodium_base642bin` | VARIANT_ORIGINAL_NO_PADDING with `b64_len % 4 == 0` (identical to the padded case minus padding) | [x] |
| 1.96 | `sodium_base642bin` | VARIANT_ORIGINAL_NO_PADDING fed input that *does* contain `'='`: `'='` is not a valid b64 char → loop breaks; with `b64_end != NULL` → `0` and `*b64_end` at the `'='`; with `b64_end == NULL` → `-1`/`EINVAL` | [x] |
| 1.97 | `sodium_base642bin` | VARIANT_URLSAFE with `'-'` and `'_'` in the input (`b64_urlsafe_char_to_byte`), padded, all three `acc_len` residues | [x] |
| 1.98 | `sodium_base642bin` | VARIANT_URLSAFE_NO_PADDING with `'-'`/`'_'`, all three residues | [x] |
| 1.99 | `sodium_base642bin` | cross-alphabet rejection as a *valid* stop: URLSAFE variant given `'+'`/`'/'`, and ORIGINAL variant given `'-'`/`'_'` — with `b64_end != NULL` the decode stops cleanly at that char and returns `0` | [x] |
| 1.100 | `sodium_base642bin` | `'A'` in the input — `b64_char_to_byte` returns `0` for `'A'` and relies on `(EQ(x,0) & (EQ(c,'A') ^ 0xFF))` to distinguish it from an invalid char; input `"AAAA"` must decode to three `0x00` bytes | [x] |
| 1.101 | `sodium_base642bin` | `ignore = " \n\r"` with ignorable chars interleaved between data chars (PEM-style wrapped input) | [x] |
| 1.102 | `sodium_base642bin` | `ignore` non-NULL with ignorable chars **inside** the padding run (e.g. `"QQ=\n="`) — `_sodium_base642bin_skip_padding` consults `ignore` too | [x] |
| 1.103 | `sodium_base642bin` | `ignore` non-NULL with **trailing** ignorable chars after the data/padding and `ret == 0` — the final `while (b64_pos < b64_len && strchr(ignore, …))` loop consumes them, so `*b64_end == b64 + b64_len` and `b64_end == NULL` also succeeds | [x] |
| 1.104 | `sodium_base642bin` | `ignore = ""` (empty, non-NULL) — the `ignore != NULL` branches are entered but `strchr` never matches; trailing-skip loop terminates immediately | [x] |
| 1.105 | `sodium_base642bin` | `bin_len == NULL` and `b64_end == NULL` (both omitted, strict mode) with fully-consumed padded input | [x] |
| 1.106 | `sodium_base642bin` | `bin_len != NULL`, `b64_end == NULL` — strict length check on the input | [x] |
| 1.107 | `sodium_base642bin` | `bin_len == NULL`, `b64_end != NULL` | [x] |
| 1.108 | `sodium_base642bin` | `bin_maxlen` exactly the decoded size (minimum accepted) vs. `bin_maxlen` larger (tail of `bin` must be untouched) | [x] |
| 1.109 | `sodium_bin2base64` → `sodium_base642bin` | round-trip for each of the 4 variants × each `bin_len % 3` residue × `bin_len == 0` | [x] |
| 1.110 | `sodium_bin2hex` → `sodium_hex2bin` | round-trip for `bin_len` 0 / 1 / 32 | [x] |

### codecs.c — ip2bin / bin2ip

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.111 | `sodium_ip2bin` | IPv4 dotted quad `"1.2.3.4"` → IPv4-mapped form: `bin[0..9] == 0`, `bin[10] == bin[11] == 0xff`, `bin[12..15] == {1,2,3,4}` | [x] |
| 1.112 | `sodium_ip2bin` | IPv4 boundary values `"0.0.0.0"` and `"255.255.255.255"` | [x] |
| 1.113 | `sodium_ip2bin` | IPv4 with 1-, 2-, and 3-digit octets and leading zeros (`"01.002.3.40"`) — accepted since `digits <= 3` and `val <= 255` | [x] |
| 1.114 | `sodium_ip2bin` | `ip_len_` **larger** than `strlen(ip)` — the scan loop stops at the first NUL, so the excess is ignored | [x] |
| 1.115 | `sodium_ip2bin` | `ip_len_` **smaller** than `strlen(ip)` — the address is truncated at `ip_len_` (e.g. `"1.2.3.45"` with `ip_len_ == 7` → `1.2.3.4`); non-NUL-terminated input is legal | [x] |
| 1.116 | `sodium_ip2bin` | IPv6 with all 8 explicit groups `"1:2:3:4:5:6:7:8"` (no `colonp`, `tp == endp` exactly) | [x] |
| 1.117 | `sodium_ip2bin` | IPv6 with `"::"` in the middle `"1:2::7:8"` — `colonp` set, `memmove`/`memset` zero-fill path | [x] |
| 1.118 | `sodium_ip2bin` | IPv6 with a leading `"::"` (`"::1"`) — the `*p == ':'` prologue branch | [x] |
| 1.119 | `sodium_ip2bin` | IPv6 with a trailing `"::"` (`"1::"`) — `saw_xdigit == 0` at loop exit, `colonp` set | [x] |
| 1.120 | `sodium_ip2bin` | IPv6 `"::"` alone → all-zero `bin` (unspecified address) | [x] |
| 1.121 | `sodium_ip2bin` | IPv6 with embedded IPv4 after `"::ffff:"` (`"::ffff:1.2.3.4"`) — the `ch == '.'` branch calling `parse_ipv4(curtok, end, tp)`; must equal the result of `sodium_ip2bin("1.2.3.4")` | [x] |
| 1.122 | `sodium_ip2bin` | IPv6 with embedded IPv4 and a non-`ffff` prefix (`"64:ff9b::1.2.3.4"`) | [x] |
| 1.123 | `sodium_ip2bin` | IPv6 with 8 explicit groups where the last is an embedded IPv4 (`"1:2:3:4:5:6:1.2.3.4"`) — `tp + 4 == endp` exactly | [x] |
| 1.124 | `sodium_ip2bin` | IPv6 groups of 1, 2, 3, and 4 hex digits (`"a:bc:def:0123::"`), uppercase (`"FE80::1"`) and lowercase, mixed case | [x] |
| 1.125 | `sodium_ip2bin` | IPv6 with a zone id: `"fe80::1%eth0"`, `"fe80::1%1"`, `"fe80::1%a-b_c.d"` — zone chars `[0-9a-zA-Z._-]`; `end` is moved back to the `'%'` so the zone is parsed but discarded | [x] |
| 1.126 | `sodium_ip2bin` | `bin` fully overwritten on success and untouched on failure — verify the 16 output bytes for both the IPv4 (`memset` 10 + `0xff 0xff` + 4-byte copy) and IPv6 (`memcpy` of 16) paths | [x] |
| 1.127 | `sodium_bin2ip` | `bin` = IPv4-mapped prefix + `{1,2,3,4}` → `"1.2.3.4"`, with `ip_maxlen == len + 1` exactly and with `ip_maxlen` larger | [x] |
| 1.128 | `sodium_bin2ip` | `bin` = IPv4-mapped prefix + `{0,0,0,0}` → `"0.0.0.0"` (matches the prefix, so the IPv4 branch wins even though the whole address is "small") | [x] |
| 1.129 | `sodium_bin2ip` | `bin` = IPv4-mapped prefix + `{255,255,255,255}` → `"255.255.255.255"` (longest IPv4 output, 15 chars) | [x] |
| 1.130 | `sodium_bin2ip` | `bin[10..11] == {0xff, 0xff}` but some byte in `bin[0..9]` non-zero → `memcmp` fails → IPv6 formatting path | [x] |
| 1.131 | `sodium_bin2ip` | `bin` all-zero → `best_len == 8` → `"::"` | [x] |
| 1.132 | `sodium_bin2ip` | exactly **one** zero group (`best_len == 1 < 2`) → `best_start` forced to `-1`, so the group is printed as `"0"` and **not** compressed | [x] |
| 1.133 | `sodium_bin2ip` | exactly **two** consecutive zero groups (`best_len == 2`, the compression threshold) → `"::"` | [x] |
| 1.134 | `sodium_bin2ip` | two zero runs of **equal** length — the `cur_len > best_len` strict comparison keeps the **first** run | [x] |
| 1.135 | `sodium_bin2ip` | two zero runs of **different** lengths — the longer one is compressed regardless of position | [x] |
| 1.136 | `sodium_bin2ip` | zero run at the **start** (`best_start == 0`) → leading `"::"`; the `i != 0` guard suppresses an extra `':'` | [x] |
| 1.137 | `sodium_bin2ip` | zero run in the **middle** → `"1:2::7:8"`; the `i != best_start + best_len` guard suppresses the duplicate `':'` after `"::"` | [x] |
| 1.138 | `sodium_bin2ip` | zero run at the **end** (`best_start + best_len == 8`) → trailing `"::"` | [x] |
| 1.139 | `sodium_bin2ip` | `bin` all-`0xff` → `"ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff"` (39 chars); `ip_maxlen == 40` exactly and `ip_maxlen == 41` | [x] |
| 1.140 | `sodium_bin2ip` | `ip_maxlen == 3` (the smallest accepted value) with `bin` all-zero → `"::"` fits exactly | [x] |
| 1.141 | `sodium_bin2ip` | groups needing 1 / 2 / 3 / 4 hex digits — `ip_write_num(base 16)` emits **no leading zeros** (`"1:20:300:4000:…"`) | [x] |
| 1.142 | `sodium_bin2ip` | octet values 1 / 2 / 3 digits in the IPv4 path — `ip_write_num(base 10)` | [x] |
| 1.143 | `sodium_ip2bin` → `sodium_bin2ip` | round-trip for every shape in 1.111–1.125 (note: the IPv4-mapped `bin` round-trips back to dotted-quad text, **not** to `"::ffff:1.2.3.4"`) | [x] |
| 1.144 | `sodium_bin2ip` | IPv4-mapped branch: `memcpy(ip, buf, len + 1U)` reads `buf[len]` which is **uninitialised stack**, then immediately overwrites `ip[len] = 0` — the extra byte is guaranteed in range by the `len >= ip_maxlen` check, but the copied value is indeterminate; pin the observable result (`ip[len] == 0`) | [x] |

### utils.c — pad / unpad

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.145 | `sodium_pad` | `blocksize == 1` → `xpadlen == 0`, always appends exactly one `0x80` byte | [x] |
| 1.146 | `sodium_pad` | `blocksize` a power of two (e.g. 16, 64) → the `(blocksize & (blocksize-1)) == 0` fast path using `& (blocksize - 1)` | [x] |
| 1.147 | `sodium_pad` | `blocksize` **not** a power of two (e.g. 3, 13, 100) → the `%` path; must agree with the fast path on power-of-two sizes | [x] |
| 1.148 | `sodium_pad` | `unpadded_buflen == 0` → `xpadlen == blocksize - 1`, `xpadded_len == blocksize - 1`, `*padded_buflen_p == blocksize` | [x] |
| 1.149 | `sodium_pad` | `unpadded_buflen % blocksize == 0` (and > 0) → a **full extra block** of padding: `*padded_buflen_p == unpadded_buflen + blocksize` | [x] |
| 1.150 | `sodium_pad` | `unpadded_buflen % blocksize == blocksize - 1` → `xpadlen == 0`, only the `0x80` barrier is appended | [x] |
| 1.151 | `sodium_pad` | `unpadded_buflen % blocksize` in the middle of the range (e.g. 1 with blocksize 16) → `blocksize - 1 - r` zero bytes then `0x80`… (barrier at `tail - xpadlen`, zeros above it) | [x] |
| 1.152 | `sodium_pad` | `padded_buflen_p == NULL` — padding is still written, only the length report is skipped | [x] |
| 1.153 | `sodium_pad` | `padded_buflen_p != NULL` — `*padded_buflen_p == xpadded_len + 1` | [x] |
| 1.154 | `sodium_pad` | `max_buflen == xpadded_len + 1` exactly (minimum accepted) vs. `max_buflen` much larger — bytes beyond `xpadded_len` must be untouched | [x] |
| 1.155 | `sodium_pad` | pre-existing garbage in `buf[unpadded_buflen .. xpadded_len]` — the constant-time loop must overwrite it (`& mask` clears above the barrier, `0x80 & barrier_mask` sets the barrier) | [x] |
| 1.156 | `sodium_unpad` | `unpadded_buflen_p` (always dereferenced on the non-early-return paths); `blocksize == 1`, `padded_buflen == 1`, buffer `{0x80}` → `*unpadded_buflen_p == 0`, ret `0` | [x] |
| 1.157 | `sodium_unpad` | `padded_buflen == blocksize` exactly (minimum accepted) | [x] |
| 1.158 | `sodium_unpad` | `padded_buflen > blocksize` — only the last `blocksize` bytes are scanned | [x] |
| 1.159 | `sodium_unpad` | barrier at the very last byte (`pad_len == 0`) vs. barrier `blocksize - 1` bytes back (`pad_len == blocksize - 1`, maximum) | [x] |
| 1.160 | `sodium_unpad` | tail containing **multiple** `0x80` bytes — the `(acc - 1U) & (pad_len - 1U) & ((c ^ 0x80) - 1U)` gating means only the first `0x80` found scanning backwards from the tail (with all-zero bytes after it) counts | [x] |
| 1.161 | `sodium_unpad` | a valid buffer unpadded with a **larger** `blocksize` than it was padded with — still finds the barrier (as long as `padded_buflen >= blocksize`) | [x] |
| 1.162 | `sodium_unpad` | a valid buffer unpadded with a **smaller** `blocksize` — the barrier may fall outside the scan window → ret `-1` | [x] |
| 1.163 | `sodium_pad` → `sodium_unpad` | round-trip for `blocksize` ∈ {1, 3, 16, 64} × `unpadded_buflen` ∈ {0, 1, blocksize-1, blocksize, blocksize+1, 2*blocksize} | [x] |

### utils.c — mlock / malloc / mprotect (all degraded in this build)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.164 | `sodium_mlock` | any `(addr, len)` including `len == 0` → `-1`/`ENOSYS`; the buffer contents are **unchanged** (no `MADV_DONTDUMP` call either) | [x] |
| 1.165 | `sodium_munlock` | any `(addr, len)` → `-1`/`ENOSYS`, **but `[addr, addr+len)` has been zeroed**; `len == 0` → no write | [x] |
| 1.166 | `sodium_mprotect_noaccess` / `_readonly` / `_readwrite` | any pointer (`sodium_malloc` result, plain `malloc` result, stack address) → `-1`/`ENOSYS`; the region remains fully readable **and writable** afterwards (no real protection is installed) | [x] |
| 1.167 | `sodium_mprotect_*` | called in sequence (noaccess → readonly → readwrite) on the same pointer — all `-1`, no state kept | [x] |
| 1.168 | `sodium_malloc` | `size == 0` → `malloc(1)`, returns non-NULL; `memset(ptr, 0xdb, 0)` writes nothing; the single byte is uninitialised | [x] |
| 1.169 | `sodium_malloc` | `size > 0` → exactly `size` bytes of `GARBAGE_VALUE == 0xdb` (note: `utils.h` documents `0xd0`; the code writes `0xdb`) | [x] |
| 1.170 | `sodium_malloc` | `size` 1 / 16 / 32 / `page_size` (`0x10000`) / `page_size + 1` — plain `malloc`, so **no** guard pages, **no** 16-byte canary, **no** `mlock`, **no** stored `unprotected_size`, and the returned pointer has ordinary `malloc` alignment | [x] |
| 1.171 | `sodium_malloc` | one-byte-past-the-end read/write does **not** fault in this build (the `HAVE_ALIGNED_MALLOC` guard-page layout described in `utils.h` is absent) | [x] |
| 1.172 | `sodium_allocarray` | `count == 0` (any `size`) — the overflow guard is skipped, `sodium_malloc(0)` → `malloc(1)` | [x] |
| 1.173 | `sodium_allocarray` | `size == 0`, `count > 0` — `0 >= SIZE_MAX/count` is false → `sodium_malloc(0)` → `malloc(1)` | [x] |
| 1.174 | `sodium_allocarray` | `count, size` both > 0 and well within range → `count * size` bytes of `0xdb` | [x] |
| 1.175 | `sodium_allocarray` | boundary: `size == SIZE_MAX / count - 1` (accepted, then `malloc` almost certainly fails → `NULL`) vs. `size == SIZE_MAX / count` (rejected → `ENOMEM`) | [x] |
| 1.176 | `sodium_free` | `ptr == NULL` → no-op | [x] |
| 1.177 | `sodium_free` | `ptr` from `sodium_malloc` / `sodium_allocarray` → plain `free`; **no canary verification and no zeroing of the user region** in this build | [x] |
| 1.178 | `_sodium_alloc_init` | called directly and via `sodium_init` — returns `0`, draws 16 bytes into the (unused) module-static `canary`; calling it twice re-draws the canary | [x] |

### randombytes.c

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.179 | `randombytes_seedbytes` | any call → `randombytes_SEEDBYTES == 32` | [x] |
| 1.180 | `randombytes_implementation_name` | **default** implementation (no `set_implementation` call) → `"sysrandom"`; this call itself triggers `randombytes_init_if_needed()` + `randombytes_stir()` | [x] |
| 1.181 | `randombytes_set_implementation(&randombytes_sysrandom_implementation)` | explicit install of the default → `"sysrandom"`, `uniform == NULL`, `stir`/`close` non-NULL | [x] |
| 1.182 | `randombytes_set_implementation(&randombytes_internal_implementation)` | the ChaCha20-based implementation → `"internal"`, `uniform == NULL`; also reachable via the `randombytes_salsa20_implementation` compatibility alias | [x] |
| 1.183 | `randombytes_set_implementation(NULL)` | the next randombytes call sees `implementation == NULL` and `randombytes_init_if_needed()` **reinstalls** `&randombytes_sysrandom_implementation` and re-stirs | [x] |
| 1.184 | `randombytes_set_implementation` | a custom implementation with `stir == NULL` → `randombytes_stir()` (including the one inside `init_if_needed`) is a silent no-op | [x] |
| 1.185 | `randombytes_set_implementation` | a custom implementation with `uniform == NULL` → `randombytes_uniform` uses the built-in rejection sampler over `implementation->random()` | [x] |
| 1.186 | `randombytes_set_implementation` | a custom implementation with `uniform != NULL` → `randombytes_uniform` delegates **before** the `upper_bound < 2` guard, so the callback receives `0` and `1` verbatim | [x] |
| 1.187 | `randombytes_set_implementation` | a custom implementation with `close == NULL` → `randombytes_close()` returns `0` | [x] |
| 1.188 | `randombytes_set_implementation` | a custom implementation with `close != NULL` → `randombytes_close()` returns the callback's value verbatim | [x] |
| 1.189 | `randombytes_close` | called **before** any other randombytes call (`implementation == NULL`) → `0`, and lazy init is *not* triggered | [x] |
| 1.190 | `randombytes_close` | called twice in a row on the sysrandom implementation | [x] |
| 1.191 | `randombytes_close` then `randombytes_buf` | sysrandom with getrandom available: `close()` returns `0` but leaves `stream.initialized == 1`, so the following `buf` works without re-init | [x] |
| 1.192 | `randombytes_buf` | `size == 0` → `implementation->buf` is **not** invoked; `buf` untouched (a NULL `buf` with `size == 0` is safe) | [x] |
| 1.193 | `randombytes_buf` | `size == 1` | [x] |
| 1.194 | `randombytes_buf` | `size` around the 256-byte getrandom chunk boundary: 255, 256, 257, 512, 1000 (exercises `randombytes_linux_getrandom`'s chunk loop and its `chunk_size` shrink on the final partial chunk) | [x] |
| 1.195 | `randombytes_buf` | first call in the process (triggers `init_if_needed` → `stir` → `randombytes_sysrandom_init` → 16-byte getrandom probe) vs. subsequent calls | [~] OS-dependent output; contract-verified only |
| 1.196 | `randombytes_random` | default (sysrandom): each call pulls exactly 4 bytes through `randombytes_sysrandom_buf` | [x] |
| 1.197 | `randombytes_random` | internal implementation: pops 4 bytes from the `16 * 32 == 512`-byte `rnd32` pool; the pool holds `512 - 32 == 480` usable bytes, so it refills (and re-keys) every 120 calls — exercise the refill boundary | [~] OS-dependent output; contract-verified only |
| 1.198 | `randombytes_uniform` | `upper_bound == 0` and `== 1` → `0` (built-in path, no randomness drawn) | [x] |
| 1.199 | `randombytes_uniform` | `upper_bound == 2` → `min = (1 + ~2) % 2 == 0`, single draw always accepted | [x] |
| 1.200 | `randombytes_uniform` | `upper_bound` a power of two (256, `1 << 31`) → `min == 0`, no rejection loop | [x] |
| 1.201 | `randombytes_uniform` | `upper_bound == 0x80000001` (`2^31 + 1`, the documented worst case, `min` just under 2^31) → the rejection loop runs ~2 iterations on average | [x] |
| 1.202 | `randombytes_uniform` | `upper_bound == UINT32_MAX` and `== 3` (non-power-of-two, small `min`) | [x] |
| 1.203 | `randombytes_buf_deterministic` | `size == 0` → ChaCha20-IETF with zero length; `buf` untouched | [x] |
| 1.204 | `randombytes_buf_deterministic` | `size` 1 / 63 / 64 / 65 / 1024 — ChaCha20-IETF block-boundary shapes with the fixed nonce `"LibsodiumDRG"` (12 bytes) and `seed` as the 32-byte key | [x] |
| 1.205 | `randombytes_buf_deterministic` | same `seed` twice → identical output; two different seeds → different output; all-zero seed and all-`0xff` seed | [x] |
| 1.206 | `randombytes_buf_deterministic` | called with **no** implementation installed / with a custom implementation installed — the output must be identical because this function never touches `implementation` and never triggers lazy init | [x] |
| 1.207 | `randombytes` (NaCl alias) | `buf_len == 0` (no-op) and `buf_len > 0` → must be equivalent to `randombytes_buf(buf, (size_t) buf_len)` | [x] |
| 1.208 | `randombytes_stir` | called explicitly before any other call (triggers lazy init, then the implementation's own `stir`) vs. called again afterwards (re-seeds) | [x] |

### randombytes_sysrandom.c (the default implementation)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.209 | `randombytes_sysrandom_stir` | `stream.initialized == 0` → runs `randombytes_sysrandom_init` and sets `initialized = 1`; second call is a no-op | [x] |
| 1.210 | `randombytes_sysrandom_init` | normal Linux ≥ 3.17: the 16-byte `getrandom` probe succeeds → `getrandom_available = 1`, `random_data_source_fd` stays `-1`, `errno` restored to its pre-call value | [~] OS-dependent output; contract-verified only |
| 1.211 | `randombytes_sysrandom_init` | `getrandom` unavailable (probe fails, e.g. `ENOSYS`): `getrandom_available = 0`, then `BLOCK_ON_DEV_RANDOM` polls `/dev/random` for readiness and `/dev/urandom` is opened with `FD_CLOEXEC` | [~] OS-dependent output; contract-verified only |
| 1.212 | `randombytes_sysrandom_random_dev_open` | `/dev/urandom` opens and is a character device → returned immediately (first entry of `devices[]` since `USE_BLOCKING_RANDOM` is undefined); fallback to `/dev/random` only if `/dev/urandom` is unusable | [~] OS-dependent output; contract-verified only |
| 1.213 | `randombytes_sysrandom_buf` | `getrandom_available != 0` path with `size` < 256, == 256, > 256 | [x] |
| 1.214 | `randombytes_sysrandom_buf` | `getrandom_available == 0` path → `safe_read` from the device fd, including a short-read retry loop | [~] OS-dependent output; contract-verified only |
| 1.215 | `randombytes_sysrandom` (the `random` member) | always `size == 4` (`sizeof(uint32_t)`) through `randombytes_sysrandom_buf` | [x] |
| 1.216 | `randombytes_sysrandom_close` | `getrandom_available != 0`, `fd == -1` → `0` (the `getrandom` override), state left initialised | [x] |
| 1.217 | `randombytes_sysrandom_close` | `getrandom_available == 0`, `fd != -1`, `close()` succeeds → `fd = -1`, `initialized = 0`, `0`; a following `randombytes_buf` re-runs `stir` | [~] OS-dependent output; contract-verified only |
| 1.218 | `randombytes_sysrandom_implementation` (struct) | field shape: `implementation_name`, `random`, `stir`, `buf`, `close` all non-NULL; `uniform == NULL` | [x] |

### randombytes_internal_random.c (only reachable via `randombytes_set_implementation`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1.219 | `randombytes_internal_implementation` (struct) | field shape: `uniform == NULL`, all other members non-NULL; the `randombytes_salsa20_implementation` macro alias resolves to the same object | [x] |
| 1.220 | `randombytes_internal_random_stir` | first call: `global.initialized == 0` → `randombytes_internal_random_init()` (which sets `rdrand_available = sodium_runtime_has_rdrand() == 0` and probes `getrandom`), then keys the ChaCha20 stream from `getrandom`; `nonce = sodium_hrtime()` (gettimeofday microseconds); `rnd32` zeroed, `rnd32_outleft = 0` | [~] OS-dependent output; contract-verified only |
| 1.221 | `randombytes_internal_random_stir` | second and later calls: `global.initialized != 0` → init is skipped, but the nonce is re-read and the key re-drawn | [~] OS-dependent output; contract-verified only |
| 1.222 | `randombytes_internal_random_stir_if_needed` | `stream.initialized == 0` → stirs; `!= 0` → returns immediately. `HAVE_GETPID` is off, so there is **no** `getpid()` fork check | [~] OS-dependent output; contract-verified only |
| 1.223 | `randombytes_internal_random_buf` | `size == 0` (reachable only by calling `impl->buf` directly; `randombytes_buf` filters it) / `size == 1` / `size` spanning several ChaCha20 blocks; after each call the key is re-keyed with `crypto_stream_chacha20_xor` and `nonce++` | [x] |
| 1.224 | `randombytes_internal_random_buf` | successive calls produce distinct output (nonce advances, key rotates); the `size` bytes are XORed into `stream.key[0..7]` before re-keying | [~] OS-dependent output; contract-verified only |
| 1.225 | `randombytes_internal_random` | `rnd32_outleft == 0` (cold, forces a pool refill + re-key) vs. `rnd32_outleft > 0` (warm pop of 4 bytes, popped slot zeroed) | [x] |
| 1.226 | `randombytes_internal_random_xorhwrand` | `HAVE_RDRAND` off → compiled body is **empty**; `global.rdrand_available` is always `0` and is never consulted | [x] |
| 1.227 | `randombytes_internal_random_close` | `getrandom_available != 0` → `0`, plus `sodium_memzero(&stream, sizeof stream)` so `initialized` returns to 0 and the next call re-stirs (new nonce, new key) | [x] |
| 1.228 | `randombytes_internal_random_close` | `getrandom_available == 0` → `-1`, but the stream is still zeroed (so a re-stir still happens) | [~] OS-dependent output; contract-verified only |
| 1.229 | `randombytes_set_implementation(&randombytes_internal_implementation)` then `randombytes_random` / `randombytes_buf` / `randombytes_uniform` / `randombytes_stir` / `randombytes_close` / `randombytes_implementation_name` | full public-API sweep against the non-default implementation | [x] |
| 1.230 | `randombytes_set_implementation` back and forth | internal → sysrandom → internal: each `set_implementation` leaves the previous implementation's state intact (sysrandom's `stream.initialized`, internal's `global.initialized`), and `init_if_needed` only stirs when `implementation == NULL` | [x] |
| 1.231 | internal implementation, multi-threaded | `TLS` expands to **nothing** under `-std=c99` (`__STDC_VERSION__ == 199901L`), so `static TLS InternalRandom stream` is a shared global, not thread-local — concurrent `randombytes_random()` calls race on `rnd32_outleft` | [~] OS-dependent output; contract-verified only |
