# CONFIGS.md — configuration-surface table (Phase A / Phase B)

One row per meaningful combination of runtime options and input shapes that
the C source actually branches on, derived mechanically from the public
headers and the `if`/`switch`/`#ifdef` branches in `c_src/libsodium`.
`[x]` = differentially verified across randomized inputs;
`[~]` = contract-verified only (output is OS-dependent and cannot be compared byte for byte).

Build under test: x86-64 Linux, **no `HAVE_*` macros**, so every
`#ifdef HAVE_*` selects the portable fallback.

## Row counts

| area | rows | checked |
|------|------|---------|
| 1 | 231 | 218 |
| 2 | 128 | 128 |
| 3 | 131 | 131 |
| 4 | 182 | 182 |
| 5 | 89 | 89 |
| 6 | 129 | 129 |
| 7 | 130 | 130 |
| 8 | 150 | 150 |
| **total** | **1170** | **1157** |


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

## Area 2 — crypto_verify + crypto_core

Configuration surface = the *valid-input* option/shape combinations that the C code actually branches on (or that select a distinct code path / constant set). Rejection branches live in `errors_2.md`.

Build assumption: no `HAVE_*` macros are defined by the CMake build, so the following are fixed and are **not** configuration axes:

- `crypto_verify_n` → byte-loop fallback (`verify.c:63`), not the SSE2 `__m128i` variant; `HAVE_INLINE_ASM` optimization barrier absent, so the `optblocker_u16` trick is the only barrier.
- `ed25519_ref10.c` → `fe_25_5` field arithmetic (`10 x 25.5`-bit limbs) + `fe_25_5/base.h` / `fe_25_5/base2.h` precomputed tables; `equal()` / `negative()` take the arithmetic fallback with `optblocker_u8`.
- `keccak1600.c` → `keccak1600_ref_*` (no `__ARM_FEATURE_SHA3`).
- `softaes.c` → the `#else` (non-`FAVOR_PERFORMANCE`) branch, `SOFTAES_STRIDE == 16`, i.e. the on-the-fly 16-entry SBOX slice tables, not `_aes_lut[1024]`.
- `MINIMAL` is not defined → `crypto_core_salsa2012` and `crypto_core_salsa208` are present.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 2.1 | `crypto_verify_16` | `x == y`, both all-zero 16 bytes | [x] |
| 2.2 | `crypto_verify_16` | `x == y`, both random 16 bytes | [x] |
| 2.3 | `crypto_verify_16` | differ at byte `k = 0` only, single-bit flip (`x[0] ^ 0x01`) | [x] |
| 2.4 | `crypto_verify_16` | differ at byte `k = 15` only (last byte, exercises the full loop before divergence) | [x] |
| 2.5 | `crypto_verify_16` | differ at every byte (`y = ~x`) | [x] |
| 2.6 | `crypto_verify_32` | `x == y`, 32 bytes (both all-zero and random) | [x] |
| 2.7 | `crypto_verify_32` | differ at byte `k` for `k ∈ {0, 15, 16, 31}` (spans the 16-byte boundary that the SSE2 variant would chunk on) | [x] |
| 2.8 | `crypto_verify_64` | `x == y`, 64 bytes (both all-zero and random) | [x] |
| 2.9 | `crypto_verify_64` | differ at byte `k` for `k ∈ {0, 31, 32, 63}` | [x] |
| 2.10 | `crypto_verify_16_bytes` / `_32_bytes` / `_64_bytes` | constant getters; must return `16U` / `32U` / `64U` matching the `crypto_verify_*_BYTES` macros | [x] |
| 2.11 | `crypto_core_salsa20` | `rounds = 20`; `c == NULL` → built-in sigma constants `0x61707865, 0x3320646e, 0x79622d32, 0x6b206574`; `k` 32 bytes, `in` 16 bytes, `out` 64 bytes | [x] |
| 2.12 | `crypto_core_salsa20` | `rounds = 20`; `c != NULL` (16-byte custom constant, `LOAD32_LE` into `j0/j5/j10/j15`) | [x] |
| 2.13 | `crypto_core_salsa20` | all-zero `in` and `k`, `c == NULL` (canonical zero-key vector) | [x] |
| 2.14 | `crypto_core_salsa2012` | `rounds = 12`; `c == NULL` (sigma) | [x] |
| 2.15 | `crypto_core_salsa2012` | `rounds = 12`; `c != NULL` (custom 16-byte constant) | [x] |
| 2.16 | `crypto_core_salsa208` | `rounds = 8`; `c == NULL` (sigma) — note the whole `salsa208` API is `__attribute__((deprecated))` in the header | [x] |
| 2.17 | `crypto_core_salsa208` | `rounds = 8`; `c != NULL` (custom 16-byte constant) | [x] |
| 2.18 | `crypto_core_salsa20` vs `_salsa2012` vs `_salsa208` | identical `(in, k, c)` fed to all three; outputs must differ (only the `rounds` argument to the shared static `crypto_core_salsa` changes: 20 / 12 / 8, loop steps by 2) | [x] |
| 2.19 | `crypto_core_salsa20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` and the `salsa2012` / `salsa208` equivalents | getters; `64U / 16U / 32U / 16U` for each of the three families | [x] |
| 2.20 | `crypto_core_hsalsa20` | `c == NULL` → `U32C` sigma constants; `k` 32 bytes, `in` 16 bytes, `out` 32 bytes (`x0, x5, x10, x15, x6..x9`, no feed-forward addition) | [x] |
| 2.21 | `crypto_core_hsalsa20` | `c != NULL` (16-byte custom constant, `LOAD32_LE` branch at `core_hsalsa20_ref2.c:31`) | [x] |
| 2.22 | `crypto_core_hsalsa20` | all-zero `in` and `k`, `c == NULL` | [x] |
| 2.23 | `crypto_core_hsalsa20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` | getters in `core_hsalsa20.c`; `32U / 16U / 32U / 16U` | [x] |
| 2.24 | `crypto_core_hchacha20` | `c == NULL` → literal `0x61707865, 0x3320646e, 0x79622d32, 0x6b206574` into `x0..x3`; 10 double-rounds of `QUARTERROUND`; out = `x0..x3, x12..x15` | [x] |
| 2.25 | `crypto_core_hchacha20` | `c != NULL` (16-byte custom constant, `LOAD32_LE` branch at `core_hchacha20.c:29`) | [x] |
| 2.26 | `crypto_core_hchacha20` | all-zero `in` and `k`, `c == NULL` | [x] |
| 2.27 | `crypto_core_hchacha20_outputbytes` / `_inputbytes` / `_keybytes` / `_constbytes` | getters; `32U / 16U / 32U / 16U` | [x] |
| 2.28 | `crypto_core_keccak1600_statebytes` | must return `sizeof(crypto_core_keccak1600_state) == 224` (opaque `unsigned char[224]`, `CRYPTO_ALIGN(16)`, `#pragma pack(1)`), while `keccak1600_ref_init` only zeroes the first `KECCAK1600_STATEBYTES == 200` | [x] |
| 2.29 | `crypto_core_keccak1600_init` + `_permute_24` | all-zero state; 24 rounds using `keccak_round_constants[0..23]`; 23x `..._IOTA_PRE` + final `..._IOTA` | [x] |
| 2.30 | `crypto_core_keccak1600_init` + `_permute_12` | all-zero state; 12 rounds using `keccak_round_constants[12..23]` only (different round-constant window than 2.29) | [x] |
| 2.31 | `crypto_core_keccak1600_permute_24` | applied twice in a row (state carried across, exercising `LOAD64_LE` / `STORE64_LE` round-trip of a non-zero state) | [x] |
| 2.32 | `crypto_core_keccak1600_permute_24` | non-trivial state: `init` → `xor_bytes` a rate-sized block → `permute_24` | [x] |
| 2.33 | `crypto_core_keccak1600_permute_12` | non-trivial state: `init` → `xor_bytes` → `permute_12` | [x] |
| 2.34 | `crypto_core_keccak1600_xor_bytes` | `offset == 0`, `length` a multiple of 8 (e.g. 136 = SHA3-256 rate, 168 = SHA3-128 rate, 200 = full state) → skips the leading unaligned loop, only the 8-byte `LOAD64_LE`/`STORE64_LE` loop runs | [x] |
| 2.35 | `crypto_core_keccak1600_xor_bytes` | `offset % 8 != 0` (e.g. `offset = 3`) and `length` large enough to cross into the 8-byte loop and leave a `< 8` tail → all three `while` loops execute | [x] |
| 2.36 | `crypto_core_keccak1600_xor_bytes` | `offset == 0`, `0 < length < 8` → leading loop skipped (offset already aligned), 8-byte loop skipped, only the trailing byte loop runs | [x] |
| 2.37 | `crypto_core_keccak1600_xor_bytes` | `length == 0` → no-op for any `offset` (all three loop guards false) | [x] |
| 2.38 | `crypto_core_keccak1600_xor_bytes` | `offset + length == 200` exactly (writes up to the last state byte, never touching the 24 padding bytes of the 224-byte struct) | [x] |
| 2.39 | `crypto_core_keccak1600_extract_bytes` | `offset == 0`, `length == 200` (full state `memcpy`) | [x] |
| 2.40 | `crypto_core_keccak1600_extract_bytes` | `offset != 0` and partial `length` (e.g. `offset = 5`, `length = 32`) | [x] |
| 2.41 | `crypto_core_keccak1600_extract_bytes` | `length == 0` → zero-byte `memcpy`, output untouched | [x] |
| 2.42 | `softaes_expand_key128` (private `private/softaes.h`; reachable only from `aegis128l_soft.c`, `aegis256_soft.c`, `ipcrypt_soft.c` — not from `sodium.h`) | 16-byte key → `SoftAesBlock rkeys[11]`, `w[44]`, `RCON[1..10]`, `sub_word`/`rot_word` on every 4th word | [x] |
| 2.43 | `softaes_expand_key256` | 32-byte key → `SoftAesBlock rkeys[15]`, 60 words, `sub_word` on both the `i % 8 == 0` (with `rot_word` + `RCON`) and `i % 8 == 4` (no rotate) positions | [x] |
| 2.44 | `softaes_invert_key_schedule128` | called after `softaes_expand_key128`; `inv_mix_columns` applied to `rkeys[1..9]` (indices 0 and 10 left alone) | [x] |
| 2.45 | `softaes_invert_key_schedule256` | called after `softaes_expand_key256`; `inv_mix_columns` applied to `rkeys[1..13]` (indices 0 and 14 left alone) | [x] |
| 2.46 | `softaes_inv_mix_columns` | arbitrary block; four `inv_mix_column` calls using `gf_mul_0e/0b/0d/09` | [x] |
| 2.47 | `softaes_block_encrypt` + `softaes_block_encryptlast` | full AES-128 encryption: 9 x `block_encrypt` (SBOX slice tables + `mix_column`) then 1 x `block_encryptlast` (SBOX only, no MixColumns), with `softaes_block_load`/`_store`/`_xor` from the header | [x] |
| 2.48 | `softaes_block_decrypt` + `softaes_block_decryptlast` | full AES-128 decryption using the inverted key schedule (2.44): 9 x `block_decrypt` (`INV_SBOX` + `inv_mix_column`) then 1 x `block_decryptlast` | [x] |
| 2.49 | `softaes_block_encrypt`/`decrypt` round trip | AES-128: encrypt then decrypt an arbitrary block recovers the plaintext | [x] |
| 2.50 | `softaes_block_encrypt`/`decrypt` round trip | AES-256 (14 rounds, `rkeys[15]`) with `expand_key256` + `invert_key_schedule256` | [x] |
| 2.51 | `crypto_core_ed25519_scalar_random` | no inputs; must yield a scalar that is canonical (`< L`) and non-zero, with `r[31] & 0xe0 == 0` because of `r[31] &= 0x1f` before the acceptance test | [x] |
| 2.52 | `crypto_core_ed25519_scalar_invert` | `s = 1` → `recip = 1`; returns `0` | [x] |
| 2.53 | `crypto_core_ed25519_scalar_invert` | `s` a random canonical scalar (`0 < s < L`) → `s * recip mod L == 1`; returns `0` | [x] |
| 2.54 | `crypto_core_ed25519_scalar_invert` | `s = L - 1` (largest canonical scalar) → `recip = L - 1`; returns `0` | [x] |
| 2.55 | `crypto_core_ed25519_scalar_invert` | `s` **non-reduced** (32-byte value `>= L`, e.g. all-`0xff`) — accepted (no canonicity check in this function); `sc25519_invert` operates on `s mod L` implicitly through `sc25519_mul`; returns `0` | [x] |
| 2.56 | `crypto_core_ed25519_scalar_negate` | `s = 0` → `neg = 0` (`2^256*0 + L - 0` reduces to `0`) | [x] |
| 2.57 | `crypto_core_ed25519_scalar_negate` | `s = 1` → `neg = L - 1` | [x] |
| 2.58 | `crypto_core_ed25519_scalar_negate` | `s` random canonical → `s + neg mod L == 0`; uses the 64-byte `t_` with `L` placed at offset 32, `sodium_sub`, then `sc25519_reduce` | [x] |
| 2.59 | `crypto_core_ed25519_scalar_negate` | `s` non-canonical (`s >= L`, up to all-`0xff`) — accepted; `sodium_sub(t_, s_, 64)` may borrow past the `L` block | [x] |
| 2.60 | `crypto_core_ed25519_scalar_complement` | `s = 0` → `comp = 1` (`t_[0]++` before the subtraction) | [x] |
| 2.61 | `crypto_core_ed25519_scalar_complement` | `s = 1` → `comp = 0` | [x] |
| 2.62 | `crypto_core_ed25519_scalar_complement` | `s` random canonical → `s + comp mod L == 1` | [x] |
| 2.63 | `crypto_core_ed25519_scalar_add` | `x, y` both canonical with `x + y < L` → no wrap; `sodium_add` over 32 bytes then `crypto_core_ed25519_scalar_reduce` over the 64-byte buffer | [x] |
| 2.64 | `crypto_core_ed25519_scalar_add` | `x, y` chosen so `x + y >= L` (e.g. both `L - 1`) → reduction path exercised | [x] |
| 2.65 | `crypto_core_ed25519_scalar_add` | `y = 0` (identity) and `x = 0, y = 0` | [x] |
| 2.66 | `crypto_core_ed25519_scalar_add` | `x, y` non-canonical 32-byte values (`>= L`) — accepted; note `sodium_add(x_, y_, 32)` only carries within the first 32 bytes of the 64-byte buffer, so any 33rd-byte carry is dropped before `sc25519_reduce` | [x] |
| 2.67 | `crypto_core_ed25519_scalar_sub` | `x > y`, both canonical → plain difference (implemented as `negate(y)` then `add`) | [x] |
| 2.68 | `crypto_core_ed25519_scalar_sub` | `x < y` → wraps mod `L` | [x] |
| 2.69 | `crypto_core_ed25519_scalar_sub` | `x == y` → `0`; and `y = 0` → `x` | [x] |
| 2.70 | `crypto_core_ed25519_scalar_mul` | `x, y` random canonical → `sc25519_mul` (12 x 21-bit limb schoolbook + Barrett-style reduction with the `666643/470296/654183/997805/136657/683901` constants) | [x] |
| 2.71 | `crypto_core_ed25519_scalar_mul` | `y = 1` (identity) and `y = 0` (annihilator) | [x] |
| 2.72 | `crypto_core_ed25519_scalar_mul` | `x, y` non-canonical (`>= L`); note `sc25519_mul` reads `a11 = load_4(a+28) >> 7` **unmasked**, so bit 255 participates | [x] |
| 2.73 | `crypto_core_ed25519_scalar_reduce` | 64-byte input whose value is already `< L` → output equals the low 32 bytes unchanged | [x] |
| 2.74 | `crypto_core_ed25519_scalar_reduce` | 64-byte input = `L` exactly (little-endian, zero-padded) → output `0` | [x] |
| 2.75 | `crypto_core_ed25519_scalar_reduce` | 64-byte all-`0xff` (maximal non-reduced scalar) → full `sc25519_reduce` carry cascade; `crypto_core_ed25519_NONREDUCEDSCALARBYTES == 64` | [x] |
| 2.76 | `crypto_core_ed25519_scalar_is_canonical` | `s < L` (e.g. `L - 1`) → `1`; `s == L` → `0`; `s` all-`0xff` → `0`; `s = 0` → `1` | [x] |
| 2.77 | `crypto_core_ed25519_scalar_from_string` | `hash_alg = crypto_core_ed25519_H2CSHA256 (1)`; `h_len = HASH_SC_L = 48` → SHA-256 `expand_message_xmd` loop runs 2 iterations (32 + 16-byte truncated `memcpy`); result is the big-endian-to-little-endian-flipped digest reduced mod `L` | [x] |
| 2.78 | `crypto_core_ed25519_scalar_from_string` | `hash_alg = crypto_core_ed25519_H2CSHA512 (2)`; `h_len = 48` → SHA-512 loop runs 1 iteration with a truncated 48-of-64-byte `memcpy`; `empty_block` is 128 bytes instead of 64 | [x] |
| 2.79 | `crypto_core_ed25519_scalar_from_string` | `ctx_len = 0` (`ctx` may be `NULL`, only param 1 is `nonnull`) | [x] |
| 2.80 | `crypto_core_ed25519_scalar_from_string` | `ctx_len = 255` (`0xff`, the largest value taking the direct DST path) | [x] |
| 2.81 | `crypto_core_ed25519_scalar_from_string` | `ctx_len > 255` → `H2C-OVERSIZE-DST-` prefixed re-hash of the DST; `ctx` is replaced by `u0` and `ctx_len` becomes `HASH_BYTES` (32 for SHA-256, 64 for SHA-512) | [x] |
| 2.82 | `crypto_core_ed25519_scalar_from_string` | `msg_len = 0`, and `msg_len` larger than one hash block (e.g. 200 bytes) | [x] |
| 2.83 | `crypto_core_ed25519_is_valid_point` | canonical prime-order-subgroup point, e.g. the Ed25519 base point encoding → `1` (all five checks pass) | [x] |
| 2.84 | `crypto_core_ed25519_is_valid_point` | output of `crypto_core_ed25519_random` → `1` (`ge25519_from_uniform` ends with `ge25519_clear_cofactor`) | [x] |
| 2.85 | `crypto_core_ed25519_add` | two canonical main-subgroup points → `0`, `r` = valid encoding; `ge25519_p3_add` via `ge25519_p3_to_cached` + `ge25519_add_cached` + `ge25519_p1p1_to_p3` | [x] |
| 2.86 | `crypto_core_ed25519_add` | `q` = the identity encoding `01 00 ... 00` → `0`, `r == p` (accepted despite `has_small_order(identity) != 0`) | [x] |
| 2.87 | `crypto_core_ed25519_add` | `p` and `q` a point/negation pair (`q[31] ^= 0x80`) → `0`, `r` = identity encoding | [x] |
| 2.88 | `crypto_core_ed25519_add` | one operand a small-order point (order 2/4/8) → `0`; result leaves the prime-order subgroup | [x] |
| 2.89 | `crypto_core_ed25519_add` | one operand a non-canonical encoding that still decodes (`ge25519_frombytes` succeeds) → `0`; `_add` performs no canonicity check | [x] |
| 2.90 | `crypto_core_ed25519_sub` | two canonical main-subgroup points → `0`; `ge25519_p3_sub` = `ge25519_p3_neg` + `ge25519_p3_add` | [x] |
| 2.91 | `crypto_core_ed25519_sub` | `p == q` → `0`, `r` = identity encoding; and `q` = identity → `r == p` | [x] |
| 2.92 | `crypto_core_ed25519_sub` | small-order / non-canonical operands (mirrors 2.88, 2.89) → `0` | [x] |
| 2.93 | `crypto_core_ed25519_random` | no inputs; internally `randombytes_buf(h, crypto_core_ed25519_UNIFORMBYTES == 32)` then `ge25519_from_uniform` — output always passes `crypto_core_ed25519_is_valid_point` | [x] |
| 2.94 | `ge25519_from_uniform` (private `private/ed25519_ref10.h`; **not** exported in 1.0.23 — reachable only via `crypto_core_ed25519_random`) | `r[31]` bit 5 clear vs set → `x_sign = ((r[31] >> 5) ^ optblocker_u8) >> 2` selects whether `p3.X` is conditionally negated; `s[31] &= 0x7f` masks the input | [x] |
| 2.95 | `ge25519_from_uniform` / `ge25519_elligator2` | `r` such that `gx1 = x1^3 + A*x1^2 + x1` **is** a square (`fe25519_notsquare == 0`) vs **is not** a square (`== 1`, taking the `x = -x1-A` correction with the `ed25519_A` cmov) — both must be covered | [x] |
| 2.96 | `ge25519_mont_to_ed` (inside 2.94/2.95) | the `fe25519_iszero(x_plus_one_y_inv)` cmov path, i.e. `(x+1)*y == 0` → `yed` forced to `1` | [x] |
| 2.97 | `crypto_core_ed25519_from_string_nu` | **NU (non-uniform) variant**: `_string_to_points(p, n = 1, ...)`, `h_len = 1 * HASH_GE_L = 48`; `hash_alg = 1` (SHA-256) | [x] |
| 2.98 | `crypto_core_ed25519_from_string_nu` | NU variant with `hash_alg = 2` (SHA-512) | [x] |
| 2.99 | `crypto_core_ed25519_from_string` | **RO (random-oracle) variant** — this is the `_ro` analogue in 1.0.23 (there is no symbol literally named `..._from_string_ro`): `_string_to_points(px, n = 2, ...)` with `h_len = 2 * 48 = 96`, then `crypto_core_ed25519_add(p, &px[0], &px[32])`; `hash_alg = 1` (SHA-256) | [x] |
| 2.100 | `crypto_core_ed25519_from_string` | RO variant with `hash_alg = 2` (SHA-512); `h_len = 96` makes the SHA-512 expand loop run 2 iterations | [x] |
| 2.101 | `crypto_core_ed25519_from_string_nu` / `_from_string` | `ctx_len = 0` vs `ctx_len = 255` vs `ctx_len > 255` (oversize-DST re-hash), cross-producted with `hash_alg ∈ {1, 2}` | [x] |
| 2.102 | `crypto_core_ed25519_from_string_nu` / `_from_string` | `msg_len = 0` vs `msg_len` > one hash block; also confirm `_from_string_nu(p)` and `_from_string(p)` on the same `(ctx, msg, hash_alg)` produce **different** points | [x] |
| 2.103 | `ge25519_from_hash` (private; reached via 2.97–2.102) | `fe25519_reduce64` on the 64-byte big-endian-flipped digest: `h[31]` and `h[63]` bit-5 contributions (`* 19` and `* 722`) with both bits clear and both set; then `y_sign = notsquare ^ 1` cmov | [x] |
| 2.104 | `crypto_core_ristretto255_is_valid_point` | all-zero 32-byte input = the ristretto255 identity → `1` (canonical, `s[0]` even, `Y == 1`, `T == 0` non-negative) | [x] |
| 2.105 | `crypto_core_ristretto255_is_valid_point` | the ristretto255 basepoint encoding → `1` | [x] |
| 2.106 | `crypto_core_ristretto255_is_valid_point` | output of `crypto_core_ristretto255_random` / `_from_hash` → `1` | [x] |
| 2.107 | `crypto_core_ristretto255_add` | two valid encodings → `0`; `ristretto255_frombytes` x2 + `ge25519_p3_add` + `ristretto255_p3_tobytes` | [x] |
| 2.108 | `crypto_core_ristretto255_add` | `q` = identity (all-zero) → `0`, `r == p` | [x] |
| 2.109 | `crypto_core_ristretto255_sub` | two valid encodings → `0`; and `p == q` → `r` = all-zero identity encoding | [x] |
| 2.110 | `crypto_core_ristretto255_sub` | `q` = identity (all-zero) → `0`, `r == p` | [x] |
| 2.111 | `crypto_core_ristretto255_from_hash` | arbitrary 64-byte `r` → always `0`; `ristretto255_elligator` on `r[0..31]` and `r[32..63]` (each `fe25519_frombytes`, so bit 255 of each half is ignored) then `ge25519_p3_add` | [x] |
| 2.112 | `crypto_core_ristretto255_from_hash` | all-zero 64-byte input → `0`; exercises `ristretto255_elligator` with `t = 0` (`r = 0`, `wasnt_square` path) | [x] |
| 2.113 | `crypto_core_ristretto255_from_hash` | inputs chosen so `ristretto255_sqrt_ratio_m1(s, u, v)` returns 1 (`wasnt_square == 0`) and inputs where it returns 0 (`wasnt_square == 1`, taking the `s_prime = -abs(s*t)` and `c = r` cmovs) — both must be covered | [x] |
| 2.114 | `ristretto255_p3_tobytes` (via 2.107–2.112) | `rotate = fe25519_isnegative(T * z_inv)` both `0` and `1` (the `iy`/`ix`/`eden` cmov triple), plus the `fe25519_isnegative(x_z_inv)` conditional negation of `y_` | [x] |
| 2.115 | `crypto_core_ristretto255_random` | no inputs; `randombytes_buf(h, crypto_core_ristretto255_HASHBYTES == 64)` then `from_hash`; result always passes `_is_valid_point` | [x] |
| 2.116 | `crypto_core_ristretto255_from_string` | `hash_alg = crypto_core_ristretto255_H2CSHA256 (1)`; `h_len = crypto_core_ristretto255_HASHBYTES = 64` → SHA-256 expand loop runs 2 full 32-byte iterations | [x] |
| 2.117 | `crypto_core_ristretto255_from_string` | `hash_alg = crypto_core_ristretto255_H2CSHA512 (2)`; `h_len = 64` → SHA-512 expand loop runs exactly 1 iteration with a full 64-byte `memcpy` | [x] |
| 2.118 | `crypto_core_ristretto255_from_string` | `ctx_len = 0` / `255` / `> 255` (oversize DST), crossed with `msg_len = 0` and `msg_len` > one block | [x] |
| 2.119 | `crypto_core_ristretto255_scalar_random` | delegates verbatim to `crypto_core_ed25519_scalar_random` — canonical, non-zero, `r[31] <= 0x1f` | [x] |
| 2.120 | `crypto_core_ristretto255_scalar_invert` | `s = 1`, `s` random canonical, `s = L - 1`, `s` non-canonical — all → `0` (delegates to the ed25519 version) | [x] |
| 2.121 | `crypto_core_ristretto255_scalar_negate` / `_complement` | `s = 0`, `s = 1`, `s` random canonical, `s` non-canonical (delegates to the ed25519 versions) | [x] |
| 2.122 | `crypto_core_ristretto255_scalar_add` / `_sub` | reduced operands with and without wrap; `0` operands; non-canonical operands (delegates to the ed25519 versions) | [x] |
| 2.123 | `crypto_core_ristretto255_scalar_mul` | random canonical `x, y`; `y = 1`; `y = 0`; non-canonical operands — calls `sc25519_mul` **directly**, not through `crypto_core_ed25519_scalar_mul` | [x] |
| 2.124 | `crypto_core_ristretto255_scalar_reduce` | 64-byte input `< L`; `== L`; all-`0xff` (delegates to `crypto_core_ed25519_scalar_reduce`); `crypto_core_ristretto255_NONREDUCEDSCALARBYTES == 64` | [x] |
| 2.125 | `crypto_core_ristretto255_scalar_is_canonical` | `s < L` → `1`; `s == L` → `0`; `s = 0` → `1`; all-`0xff` → `0` — calls `sc25519_is_canonical` directly | [x] |
| 2.126 | `crypto_core_ristretto255_scalar_from_string` | `hash_alg ∈ {1, 2}` x `ctx_len ∈ {0, 255, > 255}` x `msg_len ∈ {0, large}`; `h_len = HASH_SC_L = 48` (delegates to `crypto_core_ed25519_scalar_from_string`) | [x] |
| 2.127 | `crypto_core_ristretto255_bytes` / `_hashbytes` / `_scalarbytes` / `_nonreducedscalarbytes` and `crypto_core_ed25519_bytes` / `_uniformbytes` / `_hashbytes` / `_scalarbytes` / `_nonreducedscalarbytes` | getters; ed25519: `32 / 32 / 64 / 32 / 64`; ristretto255: `32 / 64 / 32 / 64` | [x] |
| 2.128 | `ge25519_scalarmult` / `ge25519_scalarmult_base` / `ge25519_double_scalarmult_vartime` (private `private/ed25519_ref10.h`; not part of the public `crypto_core_*` surface but defined in `ed25519_ref10.c`) | scalars with `a[31] <= 127` (documented precondition); `a = 0`, `a = 1`, `a = L - 1`; `ge25519_cmov8` / `ge25519_cmov8_cached` / `ge25519_cmov8_base` digit values `e[i] ∈ [-8, 8]` including the `bnegative` branch; `slide_vartime` with the `cmp <= 15`, `cmp < -15` (break) and carry-propagation arms | [x] |

### Notes recorded while ticking the rows

- Rows **2.94** and **2.103** describe the sign/correction bit as "`r[31]` bit 5" because of
  the shape of the C expression `((r[31] >> 5) ^ optblocker_u8) >> 2`. That expression is
  `r[31] >> 7`, i.e. the branch is driven by **bit 7** (`0x80`), not bit 5; the `>> 5` /
  `>> 2` split only exists so that the `volatile` optimisation blocker can be XORed in.
  Both bits are covered by the tests, and `tests/a2_gaps.rs` classifies inputs by bit 7 and
  requires both classes to be non-empty.
- Row **2.96** (`ge25519_mont_to_ed`'s `fe25519_iszero(x_plus_one_y_inv)` cmov) is reachable
  from **exactly one** input: `r == 0` (either sign bit). `x1 == 0` needs `1 + 2r^2 == 0`,
  which has no solution because `-1/2` is a quadratic non-residue mod `2^255-19`; `y == 0`
  with `x != 0` needs `A^2-4` to be a square, which it is not; and `x == 0` after the
  correction needs `x1 == -A`, i.e. `r == 0`, where `-A` is a non-residue so the `notsquare`
  arm is taken. `tests/a2_gaps.rs::mont_to_ed_cmov_path_at_zero` pins this down and checks
  that `ge25519_from_uniform(0)` is the identity encoding.
- Rows **2.95**, **2.113** and **2.114** ask for both arms of a branch inside a `static`
  helper that is invisible from outside. They are ticked on the strength of
  `tests/a2_gaps.rs`, which reimplements `F_p` test-side, replicates
  `ge25519_elligator2` / `ristretto255_elligator` / `ristretto255_p3_tobytes` statement by
  statement, validates each replica end-to-end against the C library through exported entry
  points, and then asserts that both arms are actually taken by the inputs being fed to both
  `.so` files.

## Area 3 — hashes / xof / generichash / shorthash

Scope: `crypto_hash/{crypto_hash.c, sha256/**, sha512/**, sha3/hash_sha3.c}`, `crypto_xof/**`,
`crypto_generichash/{crypto_generichash.c, blake2b/generichash_blake2.c, blake2b/ref/**}`,
`crypto_shorthash/**` + public headers.

Build assumption: no `HAVE_*` macros are defined, so every `#ifdef HAVE_*` takes the portable fallback
(`SHA256_Transform` scalar path, `blake2b_compress_ref`, non-`HAVE_TI_MODE` counter increment).

Block / rate sizes that drive the boundary axes:

| primitive | block or rate | source |
|---|---|---|
| SHA-256 | 64 B block | `hash_sha256_cp.c` (`& 0x3f`, pad threshold 56) |
| SHA-512 | 128 B block | `hash_sha512_cp.c` (`& 0x7f`, pad threshold 112) |
| SHA3-256 | 136 B rate | `SHA3_256_RATE` |
| SHA3-512 | 72 B rate | `SHA3_512_RATE` |
| SHAKE128 / TurboSHAKE128 | 168 B rate | `SHAKE128_RATE` / `TURBOSHAKE128_RATE` |
| SHAKE256 / TurboSHAKE256 | 136 B rate | `SHAKE256_RATE` / `TURBOSHAKE256_RATE` |
| BLAKE2b | 128 B block, 256 B lazy buffer (`buf[2*128]`) | `blake2.h` / `blake2b_update` |

**Length set L** (used by many rows below) = `{0, 1, 63, 64, 65, 127, 128, 129, 135, 136, 137, 143, 144, 255, 256}`.
It straddles every block/rate boundary in the area: 63/64/65 (SHA-256), 71/72/73 → covered by 143/144 mod 72,
127/128/129 (SHA-512, BLAKE2b), 135/136/137 (SHA3-256, SHAKE256, TurboSHAKE256), 143/144 (2×72 for SHA3-512),
255/256 (2× SHA-512 block, BLAKE2b full buffer).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 3.1 | `crypto_hash` | one-shot, `inlen` over all of L; must equal `crypto_hash_sha512` byte-for-byte (32→64 B out is 64) | [x] |
| 3.2 | `crypto_hash_bytes`, `crypto_hash_primitive` | no input; expect `64`, `"sha512"` | [x] |
| 3.3 | `crypto_hash_sha256` | one-shot, `inlen` over all of L | [x] |
| 3.4 | `crypto_hash_sha256_{init,update,final}` | streaming, single `update` of each `inlen` in L | [x] |
| 3.5 | `crypto_hash_sha256_*` | streaming, `inlen = 256` fed as 256 separate 1-byte `update` calls (exercises `r` walking 0..63 and the `inlen < 64 - r` lazy branch on every offset) | [x] |
| 3.6 | `crypto_hash_sha256_*` | streaming, two updates `(a, b)` for every pair with `a + b` ∈ L and `a` ∈ {0,1,31,32,33,63,64,65}: covers `r != 0` entry, the `for (i = 0; i < 64 - r; i++)` fill-and-transform, and the `inlen &= 63` tail | [x] |
| 3.7 | `crypto_hash_sha256_*` | streaming, `update` with `inlen == 0` interleaved between non-empty updates (must be a no-op: `count` unchanged) | [x] |
| 3.8 | `crypto_hash_sha256_*` | streaming with `inlen == 64 - r` exactly (update ends exactly on a block boundary → `inlen &= 63` yields 0, `buf` left untouched) | [x] |
| 3.9 | `crypto_hash_sha256_*` | `SHA256_Pad` short branch: total length ≡ `r < 56` (mod 64), e.g. 0, 1, 55 | [x] |
| 3.10 | `crypto_hash_sha256_*` | `SHA256_Pad` long branch: total length ≡ `r >= 56` (mod 64), e.g. 56, 57, 63, 120, 127 (two-block padding) | [x] |
| 3.11 | `crypto_hash_sha256_statebytes` | no input; equals `sizeof(crypto_hash_sha256_state)` | [x] |
| 3.12 | `crypto_hash_sha256` vs streaming | equivalence: one-shot == init/update×n/final for every split in 3.6 | [x] |
| 3.13 | `crypto_hash_sha512` | one-shot, `inlen` over all of L | [x] |
| 3.14 | `crypto_hash_sha512_{init,update,final}` | streaming, single `update` of each `inlen` in L | [x] |
| 3.15 | `crypto_hash_sha512_*` | streaming, `inlen = 256` fed as 256 separate 1-byte `update` calls (`r` walks 0..127) | [x] |
| 3.16 | `crypto_hash_sha512_*` | streaming, two updates `(a, b)` with `a + b` ∈ L and `a` ∈ {0,1,63,64,65,127,128,129}: covers `r != 0`, the `128 - r` fill, the `while (inlen >= 128)` bulk loop, and `inlen &= 127` | [x] |
| 3.17 | `crypto_hash_sha512_*` | `update` with `inlen == 0` interleaved (must not advance `count[0]`/`count[1]`) | [x] |
| 3.18 | `crypto_hash_sha512_*` | `SHA512_Pad` short branch: total ≡ `r < 112` (mod 128), e.g. 0, 1, 111 | [x] |
| 3.19 | `crypto_hash_sha512_*` | `SHA512_Pad` long branch: total ≡ `r >= 112` (mod 128), e.g. 112, 113, 127, 240, 255 | [x] |
| 3.20 | `crypto_hash_sha512_*` | length whose bit count exercises `bitlen[0] = inlen >> 61` ≠ 0 and the `count[1]` carry (conceptual / streaming-accumulated) | [x] |
| 3.21 | `crypto_hash_sha512_statebytes` | no input | [x] |
| 3.22 | `crypto_hash_sha512` vs streaming | equivalence across every split in 3.16 | [x] |
| 3.23 | `crypto_hash_sha3256` | one-shot, `inlen` over all of L plus `{71, 72, 73, 167, 168, 271, 272, 273}` (rate 136 and 2×136) | [x] |
| 3.24 | `crypto_hash_sha3256_{init,update,final}` | streaming, single `update` of each `inlen` above | [x] |
| 3.25 | `crypto_hash_sha3256_*` | streaming, 1-byte updates ×272 (drives `state->offset` 0..136 twice, incl. `offset == rate` re-permute at the head of `sha3_update`) | [x] |
| 3.26 | `crypto_hash_sha3256_*` | streaming, two updates `(a, b)` with `a` ∈ {1, 135, 136, 137} and `a + b` ∈ {136, 137, 272, 273}: hits the `offset != 0 && inlen > 0` partial-chunk arm, the `offset == rate && consumed < inlen` mid-permute, the `while (inlen - consumed >= rate)` bulk arm, and the trailing `consumed < inlen` arm | [x] |
| 3.27 | `crypto_hash_sha3256_*` | streaming, `update(inlen = 0)` first and between updates (must not permute; `offset` unchanged) | [x] |
| 3.28 | `crypto_hash_sha3256_*` | total input ≡ `rate - 1` (135) mod 136 → `sha3_final` fused pad byte `0x06 ^ 0x80 == 0x86` | [x] |
| 3.29 | `crypto_hash_sha3256_*` | total input ≡ 0 mod 136 with `offset == rate` at `final` (e.g. exactly 136 absorbed as one update) → `final` extra `permute_24` then pad at `offset == 0` | [x] |
| 3.30 | `crypto_hash_sha3256` one-shot vs streaming | equivalence for every split in 3.26 | [x] |
| 3.31 | `crypto_hash_sha3256_bytes`, `_statebytes` | no input; `32`, `256` | [x] |
| 3.32 | `crypto_hash_sha3512` | one-shot, `inlen` over all of L plus `{71, 72, 73, 143, 144, 145, 215, 216, 217}` (rate 72, 2×72, 3×72) | [x] |
| 3.33 | `crypto_hash_sha3512_{init,update,final}` | streaming, single `update` of each `inlen` above | [x] |
| 3.34 | `crypto_hash_sha3512_*` | streaming, 1-byte updates ×145 (`offset` 0..72 twice) | [x] |
| 3.35 | `crypto_hash_sha3512_*` | streaming, two updates `(a, b)` with `a` ∈ {1, 71, 72, 73} and `a + b` ∈ {72, 73, 144, 145} — same four `sha3_update` arms as 3.26 but at rate 72 | [x] |
| 3.36 | `crypto_hash_sha3512_*` | total ≡ 71 mod 72 → fused pad `0x86` | [x] |
| 3.37 | `crypto_hash_sha3512_*` | total ≡ 0 mod 72 with `offset == rate` at `final` | [x] |
| 3.38 | `crypto_hash_sha3512` one-shot vs streaming | equivalence for every split in 3.35 | [x] |
| 3.39 | `crypto_hash_sha3512_bytes`, `_statebytes` | no input; `64`, `256` | [x] |
| 3.40 | SHA-3 digest-size matrix | same message run through both `crypto_hash_sha3256` (32 B) and `crypto_hash_sha3512` (64 B) — confirms `outlen`/`rate` are carried in the state, not hard-coded in `sha3_final` | [x] |
| 3.41 | `crypto_xof_shake128` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {167, 168, 169, 335, 336, 337}, `outlen` ∈ {0, 1, 32, 167, 168, 169, 335, 336, 337, 512} | [x] |
| 3.42 | `crypto_xof_shake128_{init,update,squeeze}` | streaming with one `update` and one `squeeze`, over the same grid as 3.41 | [x] |
| 3.43 | `crypto_xof_shake128_*` | multiple absorb calls: `update` ×n with sizes `(1,1,…)`, `(167,1)`, `(168,1)`, `(1,167)`, `(100,68,168)` before a single `squeeze` — must equal the concatenated one-shot | [x] |
| 3.44 | `crypto_xof_shake128_*` | chunked squeeze: total 512 B extracted as 1-byte calls ×512; as `(1, 167)`, `(167, 1)`, `(168, 168, 176)`, `(169, 343)`; must equal a single 512-B squeeze (drives `offset == RATE` re-permute, the `offset != 0` partial arm, the `while (outlen - extracted >= RATE)` bulk arm and the trailing arm) | [x] |
| 3.45 | `crypto_xof_shake128_*` | `squeeze(outlen = 0)` before / between real squeezes (must be a no-op, must not permute) | [x] |
| 3.46 | `crypto_xof_shake128_*` | absorb total ≡ `RATE - 1` (167) mod 168 → `shake128_finalize` fused pad `domain ^ 0x80` | [x] |
| 3.47 | `crypto_xof_shake128_*` | absorb total ≡ 0 mod 168 with `offset == RATE` at first squeeze → extra `permute_24` inside `shake128_finalize` | [x] |
| 3.48 | `crypto_xof_shake128_init_with_domain` | `domain = crypto_xof_shake128_DOMAIN_STANDARD` (0x1F) — must match plain `_init` exactly | [x] |
| 3.49 | `crypto_xof_shake128_init_with_domain` | `domain` ∈ {0x00, 0x01, 0x02, 0x06, 0x07, 0x0B, 0x1F, 0x7F, 0x80, 0xFF} — all accepted, no range check; each must give a distinct stream | [x] |
| 3.50 | `crypto_xof_shake128_init_with_domain` | `domain = 0x06` with absorb length ≡ 0 mod 168 and squeeze 32 → cross-check that the SHA3 domain byte under a SHAKE rate is reachable (no validation) | [x] |
| 3.51 | `crypto_xof_shake128_blockbytes`, `_statebytes`, `_domain_standard` | no input; `168`, `256`, `0x1F` | [x] |
| 3.52 | `crypto_xof_shake128` one-shot vs streaming | equivalence for every (absorb split, squeeze split) pair from 3.43 × 3.44 | [x] |
| 3.53 | `crypto_xof_shake256` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {271, 272, 273}, `outlen` ∈ {0, 1, 32, 64, 135, 136, 137, 271, 272, 273, 512} | [x] |
| 3.54 | `crypto_xof_shake256_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.53 grid | [x] |
| 3.55 | `crypto_xof_shake256_*` | multiple absorb calls: `(1,1,…)`, `(135,1)`, `(136,1)`, `(1,135)`, `(100,36,136)` | [x] |
| 3.56 | `crypto_xof_shake256_*` | chunked squeeze: 512 B as 1-byte ×512; as `(1,135)`, `(135,1)`, `(136,136,240)`, `(137,375)` | [x] |
| 3.57 | `crypto_xof_shake256_*` | absorb total ≡ 135 mod 136 → fused pad byte | [x] |
| 3.58 | `crypto_xof_shake256_*` | absorb total ≡ 0 mod 136 with `offset == RATE` at first squeeze | [x] |
| 3.59 | `crypto_xof_shake256_init_with_domain` | `domain` ∈ {0x00, 0x01, 0x06, 0x1F, 0x7F, 0x80, 0xFF} | [x] |
| 3.60 | `crypto_xof_shake256_blockbytes`, `_statebytes`, `_domain_standard` | no input; `136`, `256`, `0x1F` | [x] |
| 3.61 | `crypto_xof_shake256` one-shot vs streaming | equivalence for 3.55 × 3.56 | [x] |
| 3.62 | SHAKE128 vs SHAKE256 | same message + same `outlen` through both — different rate ⇒ different output; confirms rate is not shared state | [x] |
| 3.63 | `crypto_xof_turboshake128` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {167, 168, 169, 335, 336, 337}, `outlen` ∈ {0, 1, 32, 167, 168, 169, 336, 512} — uses `permute_12`, not `permute_24` | [x] |
| 3.64 | `crypto_xof_turboshake128_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.63 grid | [x] |
| 3.65 | `crypto_xof_turboshake128_*` | multiple absorb calls: `(1,1,…)`, `(167,1)`, `(168,1)`, `(1,167)`, `(100,68,168)` | [x] |
| 3.66 | `crypto_xof_turboshake128_*` | chunked squeeze: 512 B as 1-byte ×512; `(1,167)`, `(167,1)`, `(168,168,176)`, `(169,343)` | [x] |
| 3.67 | `crypto_xof_turboshake128_*` | absorb total ≡ 167 mod 168 → fused pad `domain ^ 0x80` | [x] |
| 3.68 | `crypto_xof_turboshake128_*` | absorb total ≡ 0 mod 168 with `offset == RATE` at first squeeze → extra `permute_12` | [x] |
| 3.69 | `crypto_xof_turboshake128_init_with_domain` | `domain = crypto_xof_turboshake128_DOMAIN_STANDARD` (0x1F); must equal plain `_init` | [x] |
| 3.70 | `crypto_xof_turboshake128_init_with_domain` | **domain-byte sweep** `domain` ∈ {0x00, 0x01, 0x02, 0x03, 0x06, 0x07, 0x0A, 0x1F, 0x30, 0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF} — the spec-legal range is 0x01..0x7F but the C code range-checks **nothing**, so 0x00 / 0x80 / 0xFF must be accepted and produce well-defined output | [x] |
| 3.71 | `crypto_xof_turboshake128_init_with_domain` | every `domain` from 3.70 combined with absorb length ≡ 167 mod 168, so the domain byte goes through the fused `domain ^ 0x80` path (0x00→0x80, 0x80→0x00, 0xFF→0x7F) | [x] |
| 3.72 | `crypto_xof_turboshake128_blockbytes`, `_statebytes`, `_domain_standard` | no input; `168`, `256`, `0x1F` | [x] |
| 3.73 | `crypto_xof_turboshake128` one-shot vs streaming | equivalence for 3.65 × 3.66 | [x] |
| 3.74 | `crypto_xof_turboshake256` | one-shot, `(inlen, outlen)` grid: `inlen` ∈ L ∪ {271, 272, 273}, `outlen` ∈ {0, 1, 32, 64, 135, 136, 137, 272, 512} | [x] |
| 3.75 | `crypto_xof_turboshake256_{init,update,squeeze}` | streaming, single update + single squeeze over the 3.74 grid | [x] |
| 3.76 | `crypto_xof_turboshake256_*` | multiple absorb calls: `(1,1,…)`, `(135,1)`, `(136,1)`, `(1,135)`, `(100,36,136)` | [x] |
| 3.77 | `crypto_xof_turboshake256_*` | chunked squeeze: 512 B as 1-byte ×512; `(1,135)`, `(135,1)`, `(136,136,240)`, `(137,375)` | [x] |
| 3.78 | `crypto_xof_turboshake256_*` | absorb total ≡ 135 mod 136 → fused pad; and ≡ 0 mod 136 with `offset == RATE` at first squeeze | [x] |
| 3.79 | `crypto_xof_turboshake256_init_with_domain` | domain-byte sweep as in 3.70 (0x00..0xFF representatives), no range check | [x] |
| 3.80 | `crypto_xof_turboshake256_blockbytes`, `_statebytes`, `_domain_standard` | no input; `136`, `256`, `0x1F` | [x] |
| 3.81 | `crypto_xof_turboshake256` one-shot vs streaming | equivalence for 3.76 × 3.77 | [x] |
| 3.82 | TurboSHAKE vs SHAKE at equal rate | SHAKE128 (24 rounds) vs TurboSHAKE128 (12 rounds) at the same rate 168 and same `domain = 0x1F` — outputs must differ; likewise SHAKE256 vs TurboSHAKE256 at rate 136 | [x] |
| 3.83 | XOF long-stream continuity | for each of the 4 XOFs: `squeeze(N)` in one call vs `N` accumulated over ⌈N/1⌉…⌈N/RATE⌉-sized calls for `N` = 4096, which crosses ~24 blocks | [x] |
| 3.84 | `crypto_generichash_blake2b` (low-level one-shot) | unkeyed (`key = NULL`, `keylen = 0`), `outlen` ∈ {1, 2, 15, **16 = BYTES_MIN**, 17, 31, **32 = BYTES**, 33, 63, **64 = BYTES_MAX**}, `inlen` ∈ L ∪ {257} | [x] |
| 3.85 | `crypto_generichash_blake2b` | unkeyed, `inlen = 0` with `in = NULL` and separately with `in != NULL` — both legal, same digest | [x] |
| 3.86 | `crypto_generichash_blake2b` | keyed, `keylen` ∈ {1, 15, **16 = KEYBYTES_MIN**, 17, 31, **32 = KEYBYTES**, 33, 63, **64 = KEYBYTES_MAX**} × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 127, 128, 129} (keyed init pre-absorbs a 128-B zero-padded key block, so `buflen` starts at 128) | [x] |
| 3.87 | `crypto_generichash_blake2b` | `key != NULL` with `keylen = 0` → silently unkeyed; must equal the `key = NULL, keylen = 0` digest | [x] |
| 3.88 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt != NULL` + `personal != NULL` (16 B each), `outlen` ∈ {1, 16, 32, 64}, `inlen` ∈ L | [x] |
| 3.89 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt = NULL` + `personal = NULL` → must equal plain `crypto_generichash_blake2b` (both fields zeroed) | [x] |
| 3.90 | `crypto_generichash_blake2b_salt_personal` | unkeyed, `salt != NULL` + `personal = NULL`; and `salt = NULL` + `personal != NULL` (the two mixed arms of `blake2b_init_salt_personal`) | [x] |
| 3.91 | `crypto_generichash_blake2b_salt_personal` | keyed, all four salt/personal NULL-combinations × `keylen` ∈ {1, 16, 32, 64} → routes through `blake2b_init_key_salt_personal` | [x] |
| 3.92 | `crypto_generichash_blake2b_salt_personal` | all-zero 16-B salt/personal buffers vs `NULL` → must be identical | [x] |
| 3.93 | `crypto_generichash_blake2b_salt_personal` | distinct salts with identical personal (and vice versa) → distinct digests; confirms both 16-B fields land at param offsets 32 and 48 | [x] |
| 3.94 | `crypto_generichash_blake2b_init` + `_update` + `_final` | unkeyed streaming, `outlen` ∈ {1, 16, 32, 64}, single `update` of each `inlen` ∈ L ∪ {257} | [x] |
| 3.95 | `crypto_generichash_blake2b_init/_update/_final` | unkeyed, `inlen = 256` fed as 256 1-byte `update` calls (walks `buflen` 0..256 and the lazy `inlen <= fill` arm at every offset) | [x] |
| 3.96 | `crypto_generichash_blake2b_init/_update/_final` | unkeyed, two updates `(a, b)` with `a` ∈ {0, 1, 127, 128, 129, 255, 256} and `a + b` ∈ {128, 129, 256, 257, 384}: hits `inlen > fill` (compress + 128-B left-shift) and `inlen <= fill` (lazy buffer) | [x] |
| 3.97 | `crypto_generichash_blake2b_init/_update/_final` | `update(inlen = 0)` first, between, and last (must be a no-op) | [x] |
| 3.98 | `crypto_generichash_blake2b_init/_update/_final` | keyed streaming: `keylen` ∈ {1, 16, 32, 64} × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 128, 129, 256} — note keyed init already leaves `buflen == 128`, so the first user byte lands at `buf[128]` | [x] |
| 3.99 | `crypto_generichash_blake2b_init` | `key = NULL` with `keylen` ∈ {1, 16, 32, 64} → the `key == NULL \|\| keylen <= 0` guard routes to **unkeyed** `blake2b_init`; must match the unkeyed digest (deliberate divergence from the one-shot, which aborts) | [x] |
| 3.100 | `crypto_generichash_blake2b_init` | `key != NULL` with `keylen = 0` → unkeyed | [x] |
| 3.101 | `crypto_generichash_blake2b_init_salt_personal` + `_update` + `_final` | unkeyed streaming × all four salt/personal NULL-combinations × `outlen` ∈ {1, 16, 32, 64} × `inlen` ∈ {0, 1, 128, 129, 256} | [x] |
| 3.102 | `crypto_generichash_blake2b_init_salt_personal` + `_update` + `_final` | keyed streaming (`key != NULL, keylen > 0`) × all four salt/personal NULL-combinations × `keylen` ∈ {1, 16, 32, 64} | [x] |
| 3.103 | `crypto_generichash_blake2b_init_salt_personal` | `key = NULL, keylen > 0` → routed to `blake2b_init_salt_personal` (unkeyed), same asymmetry as 3.99 | [x] |
| 3.104 | one-shot vs streaming equivalence (blake2b) | for every (key, salt, personal, outlen) combination in 3.84–3.93, `crypto_generichash_blake2b{,_salt_personal}` must equal `_init{,_salt_personal}` / `_update`×n / `_final` for every split in 3.96 | [x] |
| 3.105 | `blake2b_init_key` vs `blake2b_init` (via public wrappers) | `keylen = 64` (full key block, no zero padding) vs `keylen = 1` (127 zero-pad bytes) vs unkeyed — three structurally different first blocks | [x] |
| 3.106 | `blake2b_init_param` path coverage | exercised indirectly by all four init variants: `digest_length` = each valid `outlen`, `key_length` ∈ {0, 1, 16, 32, 64}, `fanout = 1`, `depth = 1`, `leaf_length = 0`, `node_offset = 0`, `node_depth = 0`, `inner_length = 0`, `reserved[14]` zero, `salt`/`personal` set or zero — i.e. every field of the 64-B param block that any public entry point can vary | [x] |
| 3.107 | `blake2b_state.last_node` | always `0` in this build (`blake2b_init0` zeroes it; nothing sets it) → `blake2b_set_lastblock` never calls `blake2b_set_lastnode`, `f[1]` stays `0`. Configuration to pin: the field exists in the state layout and `statebytes` accounting but is behaviourally inert | [x] |
| 3.108 | `crypto_generichash` (generic wrapper, one-shot) | must be byte-identical to `crypto_generichash_blake2b` across the whole 3.84–3.87 matrix: unkeyed / keyed, `outlen` ∈ {1, 15, 16, 32, 64}, `keylen` ∈ {0, 1, 16, 32, 64}, `inlen` ∈ L | [x] |
| 3.109 | `crypto_generichash_init` + `_update` + `_final` (generic wrappers) | must be byte-identical to the `_blake2b_` streaming path across 3.94–3.100; also verify the wrapper's `(state, key, keylen, outlen)` argument order is preserved | [x] |
| 3.110 | `crypto_generichash_final` | `outlen` **equal** to the `_init` `outlen` (the intended use) for each of {1, 16, 32, 64} | [x] |
| 3.111 | `crypto_generichash_final` | `outlen` **less than** the `_init` `outlen` (e.g. init 64 / final 32, init 32 / final 16, init 32 / final 1) — silently allowed, yields a prefix of the init-64 digest, **not** the init-32 digest; must be reproduced | [x] |
| 3.112 | `crypto_generichash_final` | `outlen` **greater than** the `_init` `outlen` but ≤ 64 (e.g. init 16 / final 64) — silently allowed | [x] |
| 3.113 | `crypto_generichash_state` / `crypto_generichash_blake2b_state` | `statebytes()` = `(sizeof(state) + 63) & ~63`; state is `unsigned char opaque[384]` with `CRYPTO_ALIGN(64)`; `crypto_generichash_state` is a typedef of the blake2b state, so a state initialized via `crypto_generichash_init` must be finalizable via `crypto_generichash_blake2b_final` and vice versa | [x] |
| 3.114 | `crypto_generichash_keygen`, `crypto_generichash_blake2b_keygen` | fills 32 (`KEYBYTES`) random bytes; verify length only | [x] |
| 3.115 | generichash size accessors | `crypto_generichash_{bytes_min,bytes_max,bytes,keybytes_min,keybytes_max,keybytes,primitive,statebytes}` = `16, 64, 32, 16, 64, 32, "blake2b", 384`; `crypto_generichash_blake2b_{…,saltbytes,personalbytes}` adds `16, 16` | [x] |
| 3.116 | `blake2b_compress_ref` selection | with no `HAVE_*` macros, `blake2b_compress` is `blake2b_compress_ref` at file scope and `blake2b_pick_best_implementation()` re-selects it; every blake2b row above must be checked against the ref compress only | [x] |
| 3.117 | `crypto_shorthash_siphash24` | `inlen` = 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 (every `inlen & 7` residue and the 0/1/2 full-word cases) with a fixed 16-B key | [x] |
| 3.118 | `crypto_shorthash_siphash24` | `inlen` = 17, 23, 24, 31, 32, 63, 64, 127, 128, 255, 256 (larger inputs, all 8 tail residues at multiple word counts) | [x] |
| 3.119 | `crypto_shorthash_siphash24` | `inlen = 0` with `in = NULL` — the `inlen ? in + inlen - (inlen % 8) : in` ternary means `end == in`, no dereference | [x] |
| 3.120 | `crypto_shorthash_siphash24` | `inlen = 255` and `inlen = 256` — the length byte is `((uint64_t) inlen) << 56`, so `inlen` aliases mod 256; both must be computed as C does | [x] |
| 3.121 | `crypto_shorthash_siphash24` | key variations: all-zero 16-B key, all-`0xFF` key, the RFC vector key `00 01 … 0f`, and a random key — over `inlen` 0..16 | [x] |
| 3.122 | `crypto_shorthash_siphashx24` | `inlen` = 0..16 inclusive with a fixed 16-B key; output is 16 B (`siphashx24_BYTES`) | [x] |
| 3.123 | `crypto_shorthash_siphashx24` | `inlen` = 17, 23, 24, 31, 32, 63, 64, 127, 128, 255, 256 | [x] |
| 3.124 | `crypto_shorthash_siphashx24` | key variations as in 3.121; verify the first 8 output bytes differ from `siphash24` (different `v1` init `…646f83`, `v2 ^= 0xee` vs `0xff`) | [x] |
| 3.125 | `crypto_shorthash_siphashx24` | second-half derivation: bytes 8..15 come from `v1 ^= 0xdd` + 4 extra SIPROUNDs after bytes 0..7 are stored — check both halves independently | [x] |
| 3.126 | `crypto_shorthash` (generic wrapper) | must be byte-identical to `crypto_shorthash_siphash24` over `inlen` 0..16 and 3.118's larger set | [x] |
| 3.127 | shorthash size accessors | `crypto_shorthash_{bytes,keybytes,primitive}` = `8, 16, "siphash24"`; `crypto_shorthash_siphash24_{bytes,keybytes}` = `8, 16`; `crypto_shorthash_siphashx24_{bytes,keybytes}` = `16, 16` | [x] |
| 3.128 | area-wide input-content axis | for every row above, run at least: all-zero input, all-`0xFF` input, and an incrementing `i & 0xff` pattern — the `LOAD*_BE`/`LOAD*_LE` and `STORE*` helpers are endian-sensitive and a byte-pattern input catches transposition bugs a constant input cannot | [x] (tests/a3_crosscut.rs) |
| 3.129 | area-wide state-reuse axis | `init` → `update` → `final` → `init` → `update` → `final` on the same state object for sha256, sha512, sha3-256, sha3-512, each XOF (`init` after `squeeze`) and blake2b (`_init` after `_final`) — re-init must fully reset (sha2 relies on the `memzero` in `final` plus a fresh `init`; sha3/XOF reset `phase`/`offset`; blake2b's `init0` clears `f`) | [x] (tests/a3_crosscut.rs) |
| 3.130 | area-wide overlapping / aliased buffers | `out` overlapping `in` for the one-shot entry points (`crypto_hash_sha256`, `crypto_hash_sha512`, `crypto_hash_sha3*`, `crypto_xof_*`, `crypto_generichash*`, `crypto_shorthash*`) — C writes `out` only after consuming `in` in all of these, so the aliased case is defined; the port must not regress it | [x] (tests/a3_crosscut.rs) |
| 3.131 | area-wide primitive-vs-generic consistency | `crypto_hash` ≡ `crypto_hash_sha512`; `crypto_generichash*` ≡ `crypto_generichash_blake2b*`; `crypto_shorthash` ≡ `crypto_shorthash_siphash24`; check for at least one input from each of the three content patterns in 3.128 | [x] (tests/a3_crosscut.rs) |

## Area 4 — crypto_auth + crypto_onetimeauth

Configuration axes taken directly from the source:

- **Primitive**: `hmacsha256` (SHA-256, block 64, tag 32, `crypto_verify_32`), `hmacsha512` (SHA-512, block 128, tag 64, `crypto_verify_64`), `hmacsha512256` (SHA-512 internally, tag truncated to 32, `crypto_verify_32`), `poly1305` (block 16, tag 16, `crypto_verify_16`).
- **Entry style**: one-shot `*_auth(out, in, inlen, k)` vs. streaming `*_init` / `*_update` / `*_final`. The one-shot HMAC functions are literally `init(&state, k, KEYBYTES); update(...); final(...)`, so streaming with a 32-byte key must be bit-identical.
- **Key length** (HMAC only — `*_init` takes an explicit `keylen`): `keylen < BLOCKBYTES`, `keylen == BLOCKBYTES` (must **not** hash), `keylen > BLOCKBYTES` (hashed to 32 / 64 bytes). `poly1305` has no keylen parameter — always exactly 32 bytes.
- **Message length / update splitting**: pad and block boundaries of the underlying hash, plus multi-`update` splits that straddle those boundaries and the poly1305 16-byte `leftover` buffer.
- **Wrapper level**: generic `crypto_auth*` / `crypto_onetimeauth*` vs. primitive-specific entry points, plus the `*bytes` / `*keybytes` / `*statebytes` / `*primitive` accessors.
- **Build config**: no `HAVE_*` macros are defined, so poly1305 uses `donna` with `poly1305_donna32.h` (32-bit limbs); `sse2/` is not compiled, `crypto_verify_n` and `sodium_memcmp` take their portable branches.

### CONFIGURATION SURFACE

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 4.1 | `crypto_auth` | generic one-shot wrapper; 32-byte key; must be byte-identical to `crypto_auth_hmacsha512256` for every message length in {0,1,55,56,63,64,65,111,112,127,128,129} | [x] |
| 4.2 | `crypto_auth_verify` | generic verify; good 32-byte tag ⇒ `0`; tag with one flipped bit ⇒ `-1`; identical results to `crypto_auth_hmacsha512256_verify` | [x] |
| 4.3 | `crypto_auth_keygen` | fills exactly `crypto_auth_KEYBYTES` = 32 bytes from `randombytes_buf`; two successive calls differ; no bytes written past index 31 | [x] |
| 4.4 | `crypto_auth_primitive` | returns the literal `"hmacsha512256"` (`crypto_auth_PRIMITIVE`) | [x] |
| 4.5 | `crypto_auth_bytes` / `crypto_auth_keybytes` | return 32 / 32, matching the macros `crypto_auth_BYTES` = `crypto_auth_hmacsha512256_BYTES` and `crypto_auth_KEYBYTES` = `crypto_auth_hmacsha512256_KEYBYTES` | [x] |
| 4.6 | `crypto_auth.h` surface shape | the generic `crypto_auth` API deliberately exposes **no** state type, **no** `crypto_auth_statebytes`, and **no** init/update/final — streaming is only reachable through the primitive-specific names. Port must not invent a generic streaming API | [x] |
| 4.7 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 0` (empty message) | [x] |
| 4.8 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.9 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 55` (inner hash: last block has exactly 9 bytes for pad+length after the 64-byte ipad block) | [x] |
| 4.10 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 56` (pad spills into an extra SHA-256 block) | [x] |
| 4.11 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.12 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 64` (exactly one SHA-256 block after the ipad block) | [x] |
| 4.13 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.14 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 111` | [x] |
| 4.15 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 112` | [x] |
| 4.16 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.17 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 128` (two full blocks) | [x] |
| 4.18 | `crypto_auth_hmacsha256` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.19 | `crypto_auth_hmacsha256_init` + `_update` + `_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.7 | [x] |
| 4.20 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.8 | [x] |
| 4.21 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.9 | [x] |
| 4.22 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.10 | [x] |
| 4.23 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.11 | [x] |
| 4.24 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.12 | [x] |
| 4.25 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.13 | [x] |
| 4.26 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.14 | [x] |
| 4.27 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.15 | [x] |
| 4.28 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.16 | [x] |
| 4.29 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.17 | [x] |
| 4.30 | `crypto_auth_hmacsha256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.18 | [x] |
| 4.31 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(0, 64)` — a zero-length first `update` must be a no-op | [x] |
| 4.32 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(1, 63)` on a 64-byte message (straddles the SHA-256 block boundary) | [x] |
| 4.33 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(63, 1)` on a 64-byte message (second update exactly completes the block) | [x] |
| 4.34 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(64, 1)` on a 65-byte message (first update ends exactly on a block) | [x] |
| 4.35 | `crypto_auth_hmacsha256_update` ×2 | multi-update split `(32, 32)` on a 64-byte message (neither part is block-aligned) | [x] |
| 4.36 | `crypto_auth_hmacsha256_update` ×129 | 129 successive 1-byte updates; must equal 4.18 | [x] |
| 4.37 | `crypto_auth_hmacsha256_update` ×3 | three-way split `(40, 40, 32)` on a 112-byte message; also `(56, 56)`; both must equal 4.15 | [x] |
| 4.38 | `crypto_auth_hmacsha256_init` | `keylen = 0` with a non-NULL `key` pointer — XOR loops iterate zero times, ipad/opad stay `0x36`/`0x5c` | [x] |
| 4.39 | `crypto_auth_hmacsha256_init` | `keylen = 0` with `key == NULL` — permitted (inner `if (keylen > 0)` false, no `sodium_misuse`); must equal 4.38 | [x] |
| 4.40 | `crypto_auth_hmacsha256_init` | `keylen = 1` (shorter than block) | [x] |
| 4.41 | `crypto_auth_hmacsha256_init` | `keylen = 31` (just under the canonical 32) | [x] |
| 4.42 | `crypto_auth_hmacsha256_init` | `keylen = 32` = `crypto_auth_hmacsha256_KEYBYTES`, the value the one-shot uses | [x] |
| 4.43 | `crypto_auth_hmacsha256_init` | `keylen = 63` (one below the block size) | [x] |
| 4.44 | `crypto_auth_hmacsha256_init` | `keylen = 64` == BLOCKBYTES — boundary: `keylen > 64` is false, so the key is **not** hashed and fills `pad` exactly | [x] |
| 4.45 | `crypto_auth_hmacsha256_init` | `keylen = 65` > BLOCKBYTES — key replaced by `SHA-256(key)`, `keylen` forced to 32; must equal `_init` with that 32-byte hash | [x] |
| 4.46 | `crypto_auth_hmacsha256_init` | `keylen = 128` > BLOCKBYTES (exactly two blocks of key material to hash) | [x] |
| 4.47 | `crypto_auth_hmacsha256_init` | `keylen = 1000` (multi-block key hashing, non-block-aligned); also `keylen` larger than any internal buffer to confirm no stack overflow of `pad[64]` | [x] |
| 4.48 | `crypto_auth_hmacsha256_verify` | good tag from 4.7–4.18 ⇒ `0`, for every message length in the set | [x] |
| 4.49 | `crypto_auth_hmacsha256_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.50 | `crypto_auth_hmacsha256_verify` | corrupted tag: flip bit 7 of byte 31 (last byte — catches short-compare bugs) ⇒ `-1` | [x] |
| 4.51 | `crypto_auth_hmacsha256_verify` | all-zero tag and fully random tag ⇒ `-1`; also correct tag verified against a different message and against a different key ⇒ `-1` | [x] |
| 4.52 | `crypto_auth_hmacsha256_keygen` | fills exactly 32 bytes; output usable as key for 4.42; successive calls differ | [x] |
| 4.53 | `crypto_auth_hmacsha256_bytes` / `_keybytes` / `_statebytes` | return 32 / 32 / `sizeof(crypto_auth_hmacsha256_state)` (= two `crypto_hash_sha256_state`s: `ictx` then `octx`) | [x] |
| 4.54 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 0` | [x] |
| 4.55 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.56 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 55` | [x] |
| 4.57 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 56` | [x] |
| 4.58 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.59 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 64` | [x] |
| 4.60 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.61 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 111` (SHA-512 last block has exactly 17 bytes for pad+length) | [x] |
| 4.62 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 112` (pad spills into an extra 128-byte block) | [x] |
| 4.63 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.64 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 128` (exactly one SHA-512 block after the ipad block) | [x] |
| 4.65 | `crypto_auth_hmacsha512` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.66 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.54 | [x] |
| 4.67 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.55 | [x] |
| 4.68 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.56 | [x] |
| 4.69 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.57 | [x] |
| 4.70 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.58 | [x] |
| 4.71 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.59 | [x] |
| 4.72 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.60 | [x] |
| 4.73 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.61 | [x] |
| 4.74 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.62 | [x] |
| 4.75 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.63 | [x] |
| 4.76 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.64 | [x] |
| 4.77 | `crypto_auth_hmacsha512_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.65 | [x] |
| 4.78 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(0, 128)` — zero-length first update is a no-op | [x] |
| 4.79 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(1, 127)` on a 128-byte message (straddles the SHA-512 block boundary) | [x] |
| 4.80 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(127, 1)` on a 128-byte message (second update completes the block) | [x] |
| 4.81 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(128, 1)` on a 129-byte message (first update ends exactly on a block) | [x] |
| 4.82 | `crypto_auth_hmacsha512_update` ×2 | multi-update split `(64, 64)` on a 128-byte message | [x] |
| 4.83 | `crypto_auth_hmacsha512_update` ×129 | 129 successive 1-byte updates; must equal 4.65 | [x] |
| 4.84 | `crypto_auth_hmacsha512_update` ×3 | three-way split `(40, 40, 32)` on a 112-byte message; must equal 4.62 | [x] |
| 4.85 | `crypto_auth_hmacsha512_init` | `keylen = 0` with a non-NULL `key` | [x] |
| 4.86 | `crypto_auth_hmacsha512_init` | `keylen = 0` with `key == NULL` — permitted, no `sodium_misuse`; must equal 4.85 | [x] |
| 4.87 | `crypto_auth_hmacsha512_init` | `keylen = 1` (shorter than the 128-byte block) | [x] |
| 4.88 | `crypto_auth_hmacsha512_init` | `keylen = 32` = `crypto_auth_hmacsha512_KEYBYTES` (what the one-shot passes) | [x] |
| 4.89 | `crypto_auth_hmacsha512_init` | `keylen = 64` (shorter than the block; equals the *tag* size, not the block size — must not trigger hashing) | [x] |
| 4.90 | `crypto_auth_hmacsha512_init` | `keylen = 127` (one below the block size) | [x] |
| 4.91 | `crypto_auth_hmacsha512_init` | `keylen = 128` == BLOCKBYTES — boundary: `keylen > 128` false, key **not** hashed, fills `pad[128]` exactly | [x] |
| 4.92 | `crypto_auth_hmacsha512_init` | `keylen = 129` > BLOCKBYTES — key replaced by `SHA-512(key)`, `keylen` forced to 64; must equal `_init` with that 64-byte hash | [x] |
| 4.93 | `crypto_auth_hmacsha512_init` | `keylen = 256` > BLOCKBYTES (exactly two blocks of key material) | [x] |
| 4.94 | `crypto_auth_hmacsha512_init` | `keylen = 1000` (multi-block, non-aligned key hashing); confirms `pad[128]`/`khash[64]` are never overrun | [x] |
| 4.95 | `crypto_auth_hmacsha512_verify` | good 64-byte tag ⇒ `0`, for every message length in the set | [x] |
| 4.96 | `crypto_auth_hmacsha512_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.97 | `crypto_auth_hmacsha512_verify` | corrupted tag: flip bit 7 of byte 63 (last byte of the 64-byte compare) ⇒ `-1` | [x] |
| 4.98 | `crypto_auth_hmacsha512_verify` | all-zero tag, random tag, right tag/wrong message, right tag/wrong key ⇒ `-1` | [x] |
| 4.99 | `crypto_auth_hmacsha512_keygen` | fills exactly `crypto_auth_hmacsha512_KEYBYTES` = 32 bytes (note: 32, not 64) | [x] |
| 4.100 | `crypto_auth_hmacsha512_bytes` / `_keybytes` / `_statebytes` | return 64 / 32 / `sizeof(crypto_auth_hmacsha512_state)` (two `crypto_hash_sha512_state`s) | [x] |
| 4.101 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 0` | [x] |
| 4.102 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 1` | [x] |
| 4.103 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 55` | [x] |
| 4.104 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 56` | [x] |
| 4.105 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 63` | [x] |
| 4.106 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 64` | [x] |
| 4.107 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 65` | [x] |
| 4.108 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 111` | [x] |
| 4.109 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 112` | [x] |
| 4.110 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 127` | [x] |
| 4.111 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 128` | [x] |
| 4.112 | `crypto_auth_hmacsha512256` | one-shot; 32-byte key; `inlen = 129` | [x] |
| 4.113 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 0`; must equal 4.101 | [x] |
| 4.114 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 1`; must equal 4.102 | [x] |
| 4.115 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 55`; must equal 4.103 | [x] |
| 4.116 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 56`; must equal 4.104 | [x] |
| 4.117 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 63`; must equal 4.105 | [x] |
| 4.118 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 64`; must equal 4.106 | [x] |
| 4.119 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 65`; must equal 4.107 | [x] |
| 4.120 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 111`; must equal 4.108 | [x] |
| 4.121 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 112`; must equal 4.109 | [x] |
| 4.122 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 127`; must equal 4.110 | [x] |
| 4.123 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 128`; must equal 4.111 | [x] |
| 4.124 | `crypto_auth_hmacsha512256_init/_update/_final` | streaming, `keylen = 32`, single `update`, `inlen = 129`; must equal 4.112 | [x] |
| 4.125 | `crypto_auth_hmacsha512256_update` ×2 | multi-update splits `(0, n)` and `(1, n-1)` on a 129-byte message | [x] |
| 4.126 | `crypto_auth_hmacsha512256_update` ×2 | multi-update split `(127, 1)` and `(128, 1)` straddling the 128-byte SHA-512 block boundary | [x] |
| 4.127 | `crypto_auth_hmacsha512256_update` ×2 | multi-update split `(64, 64)` on a 128-byte message | [x] |
| 4.128 | `crypto_auth_hmacsha512256_update` ×129 | 129 successive 1-byte updates; must equal 4.112 | [x] |
| 4.129 | `crypto_auth_hmacsha512256_init` | `keylen` shorter than BLOCKBYTES: each of {0, 1, 32, 64, 127} (block size is 128, inherited from `crypto_auth_hmacsha512_init`) | [x] |
| 4.130 | `crypto_auth_hmacsha512256_init` | `keylen = 128` == BLOCKBYTES boundary — key not hashed | [x] |
| 4.131 | `crypto_auth_hmacsha512256_init` | `keylen` > BLOCKBYTES: each of {129, 256, 1000} — key replaced by `SHA-512(key)`, `keylen` forced to 64 | [x] |
| 4.132 | `crypto_auth_hmacsha512256_init` | `keylen = 0` with `key == NULL` — permitted (header declares `__attribute__((nonnull))` on all args, but the code path itself does not misuse) | [x] |
| 4.133 | `crypto_auth_hmacsha512256_final` vs `crypto_auth_hmacsha512_final` | truncation semantics: the 32-byte output must equal the **first 32 bytes** of the 64-byte hmacsha512 tag for the same key/message; bytes 32..63 are discarded and `out0` is zeroed | [x] |
| 4.134 | `crypto_auth_hmacsha512256_state` / `crypto_auth_hmacsha512_state` | state-type aliasing: `crypto_auth_hmacsha512256_state` is a `typedef` of `crypto_auth_hmacsha512_state`; interop config — `_hmacsha512256_init` then `_hmacsha512_update` then `_hmacsha512_final` yields the 64-byte tag whose 32-byte prefix matches `_hmacsha512256_final` | [x] |
| 4.135 | `crypto_auth_hmacsha512256_verify` | good 32-byte tag ⇒ `0`, for every message length in the set | [x] |
| 4.136 | `crypto_auth_hmacsha512256_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.137 | `crypto_auth_hmacsha512256_verify` | corrupted tag: flip bit 7 of byte 31 ⇒ `-1` | [x] |
| 4.138 | `crypto_auth_hmacsha512256_verify` | truncation-confusion config: pass bytes 32..63 of the untruncated hmacsha512 tag ⇒ `-1`; also all-zero and random tags ⇒ `-1` | [x] |
| 4.139 | `crypto_auth_hmacsha512256_keygen` | fills exactly 32 bytes = `crypto_auth_hmacsha512256_KEYBYTES` | [x] |
| 4.140 | `crypto_auth_hmacsha512256_bytes` / `_keybytes` / `_statebytes` | return 32 / 32 / `sizeof(crypto_auth_hmacsha512256_state)`, which must equal `crypto_auth_hmacsha512_statebytes()` | [x] |
| 4.141 | `crypto_auth` vs `crypto_auth_hmacsha512256` | cross-level equivalence for the whole message-length set and for both verify outcomes; `crypto_auth_primitive()` string agrees with the delegate actually called | [x] |
| 4.142 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 0` (no blocks, no leftover — pure `poly1305_finish` on an empty accumulator) | [x] |
| 4.143 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 1` (single partial block ⇒ leftover path in `poly1305_finish`) | [x] |
| 4.144 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 15` (one byte short of a block) | [x] |
| 4.145 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 16` (exactly one `poly1305_block_size` block, no leftover) | [x] |
| 4.146 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 17` (one block + 1 leftover byte) | [x] |
| 4.147 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 31` | [x] |
| 4.148 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 32` (two full blocks — exercises `bytes & ~(16-1)` with two blocks) | [x] |
| 4.149 | `crypto_onetimeauth_poly1305` | one-shot; 32-byte key; `inlen = 33` | [x] |
| 4.150 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 0`; must equal 4.142 | [x] |
| 4.151 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 1`; must equal 4.143 | [x] |
| 4.152 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 15`; must equal 4.144 | [x] |
| 4.153 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 16`; must equal 4.145 | [x] |
| 4.154 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 17`; must equal 4.146 | [x] |
| 4.155 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 31`; must equal 4.147 | [x] |
| 4.156 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 32`; must equal 4.148 | [x] |
| 4.157 | `crypto_onetimeauth_poly1305_init/_update/_final` | streaming, single `update`, `inlen = 33`; must equal 4.149 | [x] |
| 4.158 | `crypto_onetimeauth_poly1305_update` ×2 | split `(1, 15)` on a 16-byte message — second update fills the leftover buffer to exactly `poly1305_block_size`, so `poly1305_blocks` runs and `leftover` resets to 0 | [x] |
| 4.159 | `crypto_onetimeauth_poly1305_update` ×2 | split `(15, 1)` on a 16-byte message — `want = 16 - 15 = 1`, block completed with a single byte | [x] |
| 4.160 | `crypto_onetimeauth_poly1305_update` ×2 | split `(8, 8)` on a 16-byte message — both halves are pure leftover accumulation until the block completes | [x] |
| 4.161 | `crypto_onetimeauth_poly1305_update` ×2 | split `(15, 2)` on a 17-byte message — leftover completes the block **and** 1 byte is re-stored as new leftover (`st->leftover` was reset to 0 first, so the store must start at index 0) | [x] |
| 4.162 | `crypto_onetimeauth_poly1305_update` ×2 | split `(16, 1)` on a 17-byte message — first update takes the full-block path with `leftover == 0` and stores nothing | [x] |
| 4.163 | `crypto_onetimeauth_poly1305_update` ×2 | split `(17, 16)` on a 33-byte message — second update starts with `leftover == 1`, fills 15, flushes a block, then stores 1 leftover | [x] |
| 4.164 | `crypto_onetimeauth_poly1305_update` ×33 | 33 successive 1-byte updates; must equal 4.149; exercises `want > bytes ⇒ want = bytes` and the early `return` when `leftover < 16` | [x] |
| 4.165 | `crypto_onetimeauth_poly1305_update` | zero-length update with `leftover == 0` (immediately after `_init`) — must be a complete no-op; repeat it several times | [x] |
| 4.166 | `crypto_onetimeauth_poly1305_update` | zero-length update with `leftover > 0` — `want = 16 - leftover` but `want > bytes` forces `want = 0`, so the loop does nothing and the `leftover < 16` early `return` fires; state must be unchanged | [x] |
| 4.167 | `crypto_onetimeauth_poly1305_update` ×2 | leftover-exactly-completes-block, no remainder: `update(5)` then `update(11)` — flush one block, `bytes` becomes 0, neither the full-block nor the store branch runs | [x] |
| 4.168 | `crypto_onetimeauth_poly1305_update` ×2 | all three branches in one call: `update(5)` then `update(40)` — fills 11 (flush), 16 full-block bytes, 13 stored as new leftover | [x] |
| 4.169 | `crypto_onetimeauth_poly1305_update` ×2 | leftover + full blocks with empty remainder: `update(5)` then `update(27)` — fills 11 (flush), 16 full-block bytes, nothing stored | [x] |
| 4.170 | `crypto_onetimeauth_poly1305` / `_update` | long messages (e.g. 1024, 2048, 4096 bytes and a non-aligned 1000) both as one shot and split at odd offsets — exercises the multi-block `poly1305_blocks` loop and `bytes & ~15` masking | [x] |
| 4.171 | `crypto_onetimeauth_poly1305` / `_init` | key-shape configs: all-zero 32-byte key; all-`0xff` key (maximal `r` before clamping — `r` masks `0x3ffffff/0x3ffff03/0x3ffc0ff/0x3f03fff/0x00fffff`); key whose bytes 16..31 (`pad`) are all `0xff` (final addition carries); RFC 8439 test key | [x] |
| 4.172 | `crypto_onetimeauth_poly1305_verify` | good 16-byte tag ⇒ `0`, for every message length in {0,1,15,16,17,31,32,33} | [x] |
| 4.173 | `crypto_onetimeauth_poly1305_verify` | corrupted tag: flip bit 0 of byte 0 ⇒ `-1` | [x] |
| 4.174 | `crypto_onetimeauth_poly1305_verify` | corrupted tag: flip bit 7 of byte 15 (last byte of the 16-byte compare) ⇒ `-1` | [x] |
| 4.175 | `crypto_onetimeauth_poly1305_verify` | all-zero tag, random tag, correct tag with wrong message, correct tag with wrong key ⇒ `-1`. Note this path uses **only** `crypto_verify_16` (no `sodium_memcmp`, no pointer-aliasing term, unlike the HMAC verifies) | [x] |
| 4.176 | `crypto_onetimeauth_poly1305_keygen` / `crypto_onetimeauth_keygen` | each fills exactly `crypto_onetimeauth_poly1305_KEYBYTES` = 32 bytes; successive calls differ | [x] |
| 4.177 | `crypto_onetimeauth_poly1305_bytes` / `_keybytes` / `_statebytes` | return 16 / 32 / `sizeof(crypto_onetimeauth_poly1305_state)` = 256 (the opaque `unsigned char opaque[256]`, `CRYPTO_ALIGN(16)`); must be `>= sizeof(poly1305_state_internal_t)` per the `COMPILER_ASSERT` in `_donna_init` | [x] |
| 4.178 | `crypto_onetimeauth` / `crypto_onetimeauth_verify` | generic one-shot wrappers must be byte-identical to `crypto_onetimeauth_poly1305` / `_poly1305_verify` for all lengths in the set and for good/corrupt tags | [x] |
| 4.179 | `crypto_onetimeauth_init/_update/_final` | generic streaming wrappers: `crypto_onetimeauth_state` is a `typedef` of `crypto_onetimeauth_poly1305_state`, and each wrapper is a cast-and-delegate — cross-mixing (generic `_init` + primitive `_update` + generic `_final`) must produce the same tag; `crypto_onetimeauth_statebytes()` == `crypto_onetimeauth_poly1305_statebytes()` == 256 | [x] |
| 4.180 | `crypto_onetimeauth_primitive` / `crypto_onetimeauth_bytes` / `_keybytes` | return `"poly1305"` / 16 / 32, matching `crypto_onetimeauth_PRIMITIVE`, `crypto_onetimeauth_BYTES`, `crypto_onetimeauth_KEYBYTES` | [x] |
| 4.181 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | build config: with no `HAVE_TI_MODE` / `HAVE_EMMINTRIN_H` the `sse2` block is not compiled, so the function unconditionally re-installs `crypto_onetimeauth_poly1305_donna_implementation` and returns `0`. Calling it before, between and after other calls must not change any tag; the static `implementation` pointer already defaults to donna | [x] |
| 4.182 | donna backend selection | `poly1305_donna.c` includes `poly1305_donna32.h` (no `HAVE_TI_MODE`), i.e. 32-bit 26-bit-limb arithmetic with `poly1305_state_internal_t { r[5], h[5], pad[4], leftover, buffer[16], final }` and `poly1305_block_size == 16`; `CRYPTO_ALIGN(64)` on the one-shot's local state. All vectors above must match the 64-bit implementation's outputs, so the config is behaviourally invisible but must be the one ported | [x] |

## Area 5 — crypto_stream

### Axes extracted from the source

- **Primitive** (7): `salsa20` (8-byte nonce), `salsa2012` (8-byte nonce), `salsa208` (8-byte nonce, deprecated), `xsalsa20` (24-byte nonce), `chacha20` "original" (8-byte nonce, 64-bit `ic`), `chacha20_ietf` (12-byte nonce, 32-bit `ic`), `xchacha20` (24-byte nonce, 64-bit `ic`).
- **Form** (3): keystream generator `crypto_stream_*(c, clen, n, k)`; XOR form `crypto_stream_*_xor(c, m, mlen, n, k)`; initial-counter form `crypto_stream_*_xor_ic(c, m, mlen, n, ic, k)`. Note `salsa2012` and `salsa208` have **no** `_xor_ic`. `chacha20` additionally exposes the internal-but-exported `_ietf_ext` and `_ietf_ext_xor_ic` (declared in `include/sodium/private/chacha20_ietf_ext.h`).
- **Initial counter `ic`**: `0`; `1`; small (`2, 3, 7`); values that roll the block counter over mid-message.
- **Message/keystream length** — the sweep **L** = `{0, 1, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 511, 512}` (16 values). This crosses the 64-byte block boundary in both directions and exercises both sides of the ref implementations' bulk/partial split: `while (clen >= 64) { ... }` then `if (clen) { ... }` in `salsa20_ref.c` / `stream_salsa2012_ref.c` / `stream_salsa208_ref.c`, and the `if (bytes < 64) { tmp path }` / `if (bytes <= 64) { finish }` structure in `chacha20_ref.c:112-220`.
- **Key/nonce shape**: all-`0x00`, all-`0xff`, RFC/DJB test-vector values, pseudorandom.
- **Buffer aliasing**: `c != m` (out-of-place) vs `c == m` (in-place). `chacha20_ref.c` `stream_ref` relies on in-place operation internally (`memset(c,0,clen); chacha20_encrypt_bytes(&ctx, c, c, clen)`).
- **`*_keygen`** (7 + generic + `chacha20_ietf_keygen`).
- **Accessors**: `*_keybytes`, `*_noncebytes`, `*_messagebytes_max` for all primitives + `crypto_stream_primitive`.
- **Implementation selection**: with the CMake build defining no `HAVE_*` macros, `_crypto_stream_salsa20_pick_best_implementation` and `_crypto_stream_chacha20_pick_best_implementation` always land on the `*_ref_implementation` — so the ref path is the *only* configuration reachable and there is no dispatch axis to sweep.

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 5.1 | `crypto_stream_keybytes`, `crypto_stream_noncebytes`, `crypto_stream_messagebytes_max`, `crypto_stream_primitive` | no inputs; assert exact values `32`, `24`, `SODIUM_SIZE_MAX` (= `0xFFFFFFFFFFFFFFFF` on LP64), and `"xsalsa20"` | [x] |
| 5.2 | `crypto_stream` (generic wrapper → `crypto_stream_xsalsa20`) | `k` = all-`0x00`, `n` (24 B) = all-`0x00`; `clen` over the full sweep **L** | [x] |
| 5.3 | `crypto_stream` | `k` = all-`0xff`, `n` = all-`0xff`; `clen` over **L** | [x] |
| 5.4 | `crypto_stream` | pseudorandom `k`, `n`; `clen` over **L** | [x] |
| 5.5 | `crypto_stream_xor` (generic → `crypto_stream_xsalsa20_xor`) | out-of-place (`c != m`), pseudorandom `m`; `mlen` over **L** | [x] |
| 5.6 | `crypto_stream_xor` | in-place (`c == m`), pseudorandom `m`; `mlen` over **L** | [x] |
| 5.7 | `crypto_stream_xor` | round-trip: XOR twice with the same `(n, k)` must restore `m`; `mlen` over **L** | [x] |
| 5.8 | `crypto_stream_xor` vs `crypto_stream` | equivalence: `m` = all-zero → `crypto_stream_xor` output must byte-equal `crypto_stream` output for the same `(n, k)`; `mlen` over **L** | [x] |
| 5.9 | `crypto_stream` / `crypto_stream_xor` vs `crypto_stream_xsalsa20` / `crypto_stream_xsalsa20_xor` | equivalence: generic wrapper output must byte-equal the xsalsa20-specific entry point for identical arguments; `mlen` over **L** | [x] |
| 5.10 | `crypto_stream_keygen` | 32-byte output buffer; check length written, non-constant across calls, surrounding bytes untouched | [x] |
| 5.11 | `crypto_stream_salsa20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.12 | `crypto_stream_salsa20` | `k`/`n` = all-`0x00`; `clen` over **L** (bulk loop `clen >= 64` writes directly into `c`, tail via `block[64]`) | [x] |
| 5.13 | `crypto_stream_salsa20` | `k`/`n` = all-`0xff`, and `n` = `0x0102030405060708`; `clen` over **L** | [x] |
| 5.14 | `crypto_stream_salsa20` | DJB/libsodium salsa20 test-vector `k`/`n`; `clen ∈ {64, 512}` | [x] |
| 5.15 | `crypto_stream_salsa20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.16 | `crypto_stream_salsa20_xor` | in-place (`c == m`); `mlen` over **L** | [x] |
| 5.17 | `crypto_stream_salsa20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_salsa20_xor` for the same inputs; `mlen` over **L** | [x] |
| 5.18 | `crypto_stream_salsa20_xor_ic` | `ic = 1`; must equal the tail of a `crypto_stream_salsa20_xor` run over a 64-byte-prefixed message; `mlen` over **L** | [x] |
| 5.19 | `crypto_stream_salsa20_xor_ic` | `ic ∈ {2, 3, 7}` (small); `mlen` over **L** | [x] |
| 5.20 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFF` (32-bit boundary — carry propagates from `in[11]` into `in[12]`); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.21 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFF` (64-bit block counter rolls `2^64-1 → 0` mid-message; carry out of `in[15]` is dropped silently); `mlen ∈ {65, 128, 129, 192}` | [x] |
| 5.22 | `crypto_stream_salsa20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFE` (two wraps across a 3-block message); `mlen ∈ {129, 192, 193}` | [x] |
| 5.23 | `crypto_stream_salsa20_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFFFFFFFFFF}`; early `if (!mlen) return 0;` — output buffer must be left untouched | [x] |
| 5.24 | `crypto_stream_salsa20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.25 | `crypto_stream_salsa2012_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.26 | `crypto_stream_salsa2012` | `k`/`n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.27 | `crypto_stream_salsa2012_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.28 | `crypto_stream_salsa2012_xor` | in-place (`c == m`); `mlen` over **L** | [x] |
| 5.29 | `crypto_stream_salsa2012_xor` | round-trip (XOR twice) restores `m`; and `m` = all-zero must equal `crypto_stream_salsa2012`; `mlen` over **L** | [x] |
| 5.30 | `crypto_stream_salsa2012` / `_xor` | no `_xor_ic` entry point exists → counter always starts at `in[8..15] = 0`; verify multi-block counter increment via `clen = 512` (8 blocks) and that output differs from salsa20 for the same `(n, k)` | [x] |
| 5.31 | `crypto_stream_salsa2012_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.32 | `crypto_stream_salsa208_keybytes`, `_noncebytes`, `_messagebytes_max` (all `__attribute__((deprecated))`) | no inputs; assert `32`, `8`, `SODIUM_SIZE_MAX` | [x] |
| 5.33 | `crypto_stream_salsa208` (deprecated) | `k`/`n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.34 | `crypto_stream_salsa208_xor` (deprecated) | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.35 | `crypto_stream_salsa208_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_salsa208`; `mlen` over **L** | [x] |
| 5.36 | `crypto_stream_salsa208` / `_xor` | no `_xor_ic`; counter starts at 0; `clen = 512` covers 8 counter increments; output must differ from salsa2012 and salsa20 for the same `(n, k)` | [x] |
| 5.37 | `crypto_stream_salsa208_keygen` (deprecated) | 32-byte output; length + non-constancy | [x] |
| 5.38 | `crypto_stream_xsalsa20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`24`**, `SODIUM_SIZE_MAX` | [x] |
| 5.39 | `crypto_stream_xsalsa20` | 24-byte `n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.40 | `crypto_stream_xsalsa20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.41 | `crypto_stream_xsalsa20_xor` | in-place (`c == m`); round-trip; `mlen` over **L** | [x] |
| 5.42 | `crypto_stream_xsalsa20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_xsalsa20_xor` (which is defined as `_xor_ic(..., 0ULL, ...)`); `mlen` over **L** | [x] |
| 5.43 | `crypto_stream_xsalsa20_xor_ic` | `ic = 1` and small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.44 | `crypto_stream_xsalsa20_xor_ic` | `ic = 0xFFFFFFFF` (32-bit boundary) and `ic = 0xFFFFFFFFFFFFFFFF` (64-bit rollover mid-message); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.45 | `crypto_stream_xsalsa20` / `_xor_ic` vs `crypto_stream_salsa20*` | equivalence: `crypto_stream_xsalsa20(c, clen, n, k)` must equal `crypto_stream_salsa20(c, clen, n + 16, hsalsa20(n, k))`; same for `_xor_ic`; `clen ∈ {0, 64, 65, 512}` | [x] |
| 5.46 | `crypto_stream_xsalsa20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.47 | `crypto_stream_chacha20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`8`**, `SODIUM_SIZE_MAX` | [x] |
| 5.48 | `crypto_stream_chacha20` (original, 8-byte nonce) | `k`/`n` = all-`0x00`; `clen` over **L**. Note the impl does `memset(c, 0, clen)` then encrypts in place. | [x] |
| 5.49 | `crypto_stream_chacha20` | `k`/`n` = all-`0xff`, and DJB chacha20 test-vector `k`/`n`; `clen` over **L** | [x] |
| 5.50 | `crypto_stream_chacha20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** (exercises `bytes < 64` `tmp[64]` zero-pad path for non-multiples, and the `bytes == 64` direct-exit path) | [x] |
| 5.51 | `crypto_stream_chacha20_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_chacha20`; `mlen` over **L** | [x] |
| 5.52 | `crypto_stream_chacha20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_chacha20_xor`; `mlen` over **L** | [x] |
| 5.53 | `crypto_stream_chacha20_xor_ic` | `ic = 1`; must equal the second-block-onward keystream; `mlen` over **L** | [x] |
| 5.54 | `crypto_stream_chacha20_xor_ic` | small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.55 | `crypto_stream_chacha20_xor_ic` | `ic = 0xFFFFFFFF` — 32-bit counter word `j12` wraps to 0 and carries into `j13` (which is the counter **high** word for the original nonce layout, so this is a correct 64-bit increment); `mlen ∈ {64, 65, 128, 129, 192}` | [x] |
| 5.56 | `crypto_stream_chacha20_xor_ic` | `ic = 0xFFFFFFFFFFFFFFFF` — full 64-bit counter rolls over to 0 mid-message, silently (no check); `mlen ∈ {65, 128, 129, 192}` | [x] |
| 5.57 | `crypto_stream_chacha20_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFFFFFFFFFF}`; early return, output untouched | [x] |
| 5.58 | `crypto_stream_chacha20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.59 | `crypto_stream_chacha20_ietf_keybytes`, `_ietf_noncebytes`, `_ietf_messagebytes_max` | no inputs; assert `32`, **`12`**, **`274877906944`** (`= 64 * 2^32 = 2^38`) — distinct from the non-ietf `messagebytes_max` | [x] |
| 5.60 | `crypto_stream_chacha20_ietf` (12-byte nonce) | `k`/`n` = all-`0x00`, all-`0xff`, RFC 7539 §2.4.2 vector; `clen` over **L** | [x] |
| 5.61 | `crypto_stream_chacha20_ietf_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.62 | `crypto_stream_chacha20_ietf_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_chacha20_ietf`; `mlen` over **L** | [x] |
| 5.63 | `crypto_stream_chacha20_ietf_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_chacha20_ietf_xor`; `mlen` over **L** | [x] |
| 5.64 | `crypto_stream_chacha20_ietf_xor_ic` | `ic = 1` (RFC 7539 §2.4.2 uses counter 1); `mlen` over **L** | [x] |
| 5.65 | `crypto_stream_chacha20_ietf_xor_ic` | small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.66 | `crypto_stream_chacha20_ietf_xor_ic` | **exact accepted boundary** `ic = 4294967296 - ceil(mlen/64)`: `(mlen=1, ic=0xFFFFFFFF)`, `(mlen=63, ic=0xFFFFFFFF)`, `(mlen=64, ic=0xFFFFFFFF)`, `(mlen=65, ic=0xFFFFFFFE)`, `(mlen=128, ic=0xFFFFFFFE)`, `(mlen=129, ic=0xFFFFFFFD)`, `(mlen=512, ic=0xFFFFFFF8)` — all must succeed with `ic + ceil(mlen/64) == 2^32` exactly, i.e. the counter reaches `0xFFFFFFFF` on the final block and never wraps | [x] |
| 5.67 | `crypto_stream_chacha20_ietf_xor_ic` | **one past the boundary** `ic = 4294967297 - ceil(mlen/64)`: `(mlen=65, ic=0xFFFFFFFF)`, `(mlen=128, ic=0xFFFFFFFF)`, `(mlen=129, ic=0xFFFFFFFE)`, `(mlen=512, ic=0xFFFFFFF9)` — each must hit `sodium_misuse()` | [x] |
| 5.68 | `crypto_stream_chacha20_ietf_xor_ic` | `mlen = 0` with `ic ∈ {0, 1, 0xFFFFFFFF}` — guard limit is `2^32`, never fires; then early `return 0`, output untouched | [x] |
| 5.69 | `crypto_stream_chacha20_ietf_ext` (private-but-exported, `private/chacha20_ietf_ext.h`) | `clen` over **L**; must byte-equal `crypto_stream_chacha20_ietf` for all `clen <= 2^38` | [x] |
| 5.70 | `crypto_stream_chacha20_ietf_ext_xor_ic` | `ic ∈ {0, 1, 2, 3, 7}`; `mlen` over **L**; must byte-equal `crypto_stream_chacha20_ietf_xor_ic` wherever the latter's guard permits | [x] |
| 5.71 | `crypto_stream_chacha20_ietf_ext_xor_ic` | **32-bit counter rollover into the IV** — `ic = 0xFFFFFFFF` with `mlen ∈ {65, 128, 129, 192}`: `j12` wraps `0xFFFFFFFF → 0` and the carry increments `j13`, which under `chacha_ietf_ivsetup` is **nonce word 0**. No guard on this entry point (unlike 5.67). Verify the resulting keystream equals nonce-incremented, counter-0 output. | [x] |
| 5.72 | `crypto_stream_chacha20_ietf_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.73 | `crypto_stream_chacha20_ietf*` vs `crypto_stream_chacha20*` | cross-variant separation: for the same key, the ietf (12-byte nonce, `input[12]`=counter, `input[13..15]`=nonce) and original (8-byte nonce, `input[12..13]`=counter, `input[14..15]`=nonce) layouts must produce different keystreams; `clen ∈ {64, 128}` | [x] |
| 5.74 | `crypto_stream_chacha20_IETF_KEYBYTES` / `_IETF_NONCEBYTES` / `_IETF_MESSAGEBYTES_MAX` legacy aliases (header only) | assert each alias equals its lowercase counterpart | [x] |
| 5.75 | `crypto_stream_xchacha20_keybytes`, `_noncebytes`, `_messagebytes_max` | no inputs; assert `32`, **`24`**, `SODIUM_SIZE_MAX` | [x] |
| 5.76 | `crypto_stream_xchacha20` | 24-byte `n` = all-`0x00`, all-`0xff`, pseudorandom; `clen` over **L** | [x] |
| 5.77 | `crypto_stream_xchacha20_xor` | out-of-place, pseudorandom `m`; `mlen` over **L** | [x] |
| 5.78 | `crypto_stream_xchacha20_xor` | in-place (`c == m`); round-trip; `m` = all-zero equals `crypto_stream_xchacha20`; `mlen` over **L** | [x] |
| 5.79 | `crypto_stream_xchacha20_xor_ic` | `ic = 0`; must byte-equal `crypto_stream_xchacha20_xor` (defined as `_xor_ic(..., 0U, ...)`); `mlen` over **L** | [x] |
| 5.80 | `crypto_stream_xchacha20_xor_ic` | `ic = 1` and small `ic ∈ {2, 3, 7}`; `mlen` over **L** | [x] |
| 5.81 | `crypto_stream_xchacha20_xor_ic` | `ic` is `uint64_t` and forwards to the **original** chacha20 path, so the IETF 32-bit guard does **not** apply: `ic = 0xFFFFFFFF` (32→64 carry) and `ic = 0xFFFFFFFFFFFFFFFF` (silent 64-bit rollover); `mlen ∈ {64, 65, 128, 129, 192}` — all must succeed with no misuse | [x] |
| 5.82 | `crypto_stream_xchacha20*` vs `crypto_stream_chacha20*` | equivalence: `crypto_stream_xchacha20(c, clen, n, k)` must equal `crypto_stream_chacha20(c, clen, n + 16, hchacha20(n, k))`; same for `_xor_ic`; `clen ∈ {0, 64, 65, 512}` | [x] |
| 5.83 | `crypto_stream_xchacha20_keygen` | 32-byte output; length + non-constancy | [x] |
| 5.84 | all `crypto_stream_*` keystream forms | `clen = 0` for every one of the 7 primitives + generic: `if (!clen) return 0;` — output buffer must be entirely untouched (verify with a poisoned buffer) | [x] |
| 5.85 | all `crypto_stream_*_xor{,_ic}` forms | `mlen = 0` for every primitive + generic: `if (!mlen) return 0;` — output buffer untouched | [x] |
| 5.86 | every `int`-returning entry point in area 5 | return value must be `0` for all of the above; there is no `-1` path (only `sodium_misuse()` → `abort()`) | [x] |
| 5.87 | length-sweep exactness (all primitives, all forms) | for each `mlen ∈ L`, assert byte `c[mlen]` and beyond are untouched — pins the bulk/partial split (`clen >= 64` loop vs `if (clen)` tail; `bytes < 64` `tmp[64]` path vs `bytes <= 64` exit) against over-writes | [x] |
| 5.88 | length-sweep prefix consistency (all primitives, all forms) | output for length `n1` must be a prefix of output for length `n2 > n1` with identical `(n, k, ic)`; sweep all adjacent pairs in **L** | [x] |
| 5.89 | `_crypto_stream_salsa20_pick_best_implementation`, `_crypto_stream_chacha20_pick_best_implementation` | no `HAVE_*` macros → both unconditionally select `*_ref_implementation` and `return 0`; calling either before/after any of the above must not change any output | [x] |

## Area 6 — crypto_aead / secretbox / secretstream

Scope: `c_src/libsodium/crypto_aead/{aegis128l,aegis256,aes256gcm,chacha20poly1305,xchacha20poly1305}`,
`c_src/libsodium/crypto_secretbox/**`, `c_src/libsodium/crypto_secretstream/xchacha20poly1305/**`
plus the matching public headers.

### Named sweeps used below

* **`MLEN`** = `{0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129}` — the mandatory
  short-message shape sweep. Every row that says "`mlen ∈ MLEN`" is one sub-case per element.
* **`ADLEN`** = `{ad = NULL & adlen = 0, ad = non-NULL & adlen = 0, 1, 15, 16, 17, 31, 32, 33}`
  — 9 sub-cases. The two `adlen == 0` variants are distinguished deliberately: `ad == NULL` with
  `adlen == 0` reaches `crypto_onetimeauth_poly1305_update(&state, NULL, 0)` /
  `memcpy(src, NULL, 0)`, which C technically leaves undefined but libsodium relies on.
* **`BIG_AEGIS128L`** = `{224, 255, 256, 257, 511, 512, 513, 1024, 4096}` — aegis128l `RATE == 32`
  (`aegis128l_common.h:1`) and the absorb loop consumes `RATE*2 == 64` at a time
  (`aegis128l_soft.c:172-182`), so multiples/off-by-ones of 32 and 64 exercise
  `absorb2` / `absorb` / the `% RATE` tail and `declast`.
* **`BIG_AEGIS256`** = `{112, 127, 128, 129, 255, 256, 257, 1024, 4096}` — aegis256 `RATE == 16`
  (`aegis256_common.h:1`), absorb2 consumes `2*RATE == 32`.
* **`BIG_CHACHA`** = `{4096, 65536, 131071, 131072, 131073, 262144, 262145}` — crosses the
  64-byte ChaCha20 block and the `STREAM_POLY1305_CHUNK == 131072` re-entry boundary
  (`aead_chacha20poly1305.c:20,52-61`), which is where the `ic` counter arithmetic
  (`ic += cl / 64U`) can go wrong.
* **`BIG_SECRETBOX`** = `{32, 33, 63, 64, 65, 4096, 131072, 131073, 262145}` — crosses the
  `64 - ZEROBYTES == 32` first-block special case (`crypto_secretbox_easy.c:50-52`) and the
  `STREAM_POLY1305_CHUNK` boundary (`crypto_secretbox_easy.c:71-82`).

### Build-configuration constants that fix some axes

* No `HAVE_*` macros ⇒ aegis128l/aegis256 always run the portable `*_soft.c` implementation;
  the aes256gcm **stub** family links, `crypto_aead_aes256gcm_is_available()` returns `0`, and
  all nine other aes256gcm entry points return `-1` with `errno = ENOSYS`. The aes256gcm rows
  below therefore have no positive/round-trip configurations at all — they are all
  "must return -1 regardless of shape" rows (cross-referenced to `errors_6.md` 6.30–6.39).
* `NSECBYTES == 0` for all six AEADs ⇒ **`nsec` is always `NULL`** in every row, and the
  implementations do `(void) nsec;`.
* ABYTES: aegis128l 32, aegis256 32, aes256gcm 16, chacha20poly1305 16,
  chacha20poly1305_ietf 16, xchacha20poly1305_ietf 16, secretbox MAC 16,
  secretstream 17 (`1 + 16`).
* NPUBBYTES: aegis128l 16, aegis256 32, aes256gcm 12, chacha20poly1305 **8** (original),
  chacha20poly1305_ietf **12**, xchacha20poly1305_ietf **24**, secretbox nonce 24,
  secretstream header 24.
* KEYBYTES: aegis128l **16**; everything else 32.

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| **aegis128l** (KEY 16, NPUB 16, ABYTES 32, RATE 32, portable soft impl) | | | |
| 6.1 | `crypto_aead_aegis128l_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | constant getters; assert `16 / 0 / 16 / 32 / MIN(SIZE_MAX-32, 2^61-1)` | [x] |
| 6.2 | `crypto_aead_aegis128l_keygen` | fill a 16-byte buffer; two successive calls differ; buffer fully written | [x] |
| 6.3 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | round trip, `ad = NULL`, `adlen = 0`, `nsec = NULL`, `clen_p != NULL`, `mlen_p != NULL`; `mlen ∈ MLEN`; assert `clen == mlen + 32`, `*mlen_p == mlen`, recovered `m` equal | [x] |
| 6.4 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | as 6.3 but `clen_p = NULL` on encrypt and `mlen_p = NULL` on decrypt; `mlen ∈ MLEN` | [x] |
| 6.5 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 32, 33, 64}`; ad byte-for-byte identical on both sides | [x] |
| 6.6 | `crypto_aead_aegis128l_encrypt` + `_decrypt` | `mlen ∈ BIG_AEGIS128L`, `adlen ∈ {0, 64, 65, 128}` — exercises `aegis128l_absorb2` (64-byte stride), `absorb` (32-byte stride), the `adlen % 32` tail, and `aegis128l_declast` | [x] |
| 6.7 | `crypto_aead_aegis128l_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (assert `*maclen_p == 32`), separate 32-byte `mac` buffer; `mlen ∈ MLEN`, `adlen ∈ ADLEN`; detached output must equal `encrypt` output split at `mlen` | [x] |
| 6.8 | `crypto_aead_aegis128l_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` — assert identical ciphertext/mac to 6.7 | [x] |
| 6.9 | `crypto_aead_aegis128l_decrypt_detached` | `m = NULL` (verify-only); valid mac ⇒ `0`, tampered mac ⇒ `-1`; `mlen ∈ MLEN` — exercises the `else` branches at `aegis128l_soft.c:225-229, 234` | [x] |
| 6.10 | `crypto_aead_aegis128l_decrypt` / `_decrypt_detached` | `nsec` (out-param) `NULL` vs pointing at a poisoned 1-byte buffer; assert byte unmodified and result unchanged | [x] |
| 6.11 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | in-place: `c == m` and `m == c` aliasing for `mlen ∈ {0, 1, 32, 33, 64, 1024}` | [x] |
| 6.12 | `crypto_aead_aegis128l_encrypt` / `_decrypt` | key/nonce corner shapes: all-zero `k`, all-`0xff` `k`, all-zero `npub`, all-`0xff` `npub`; `mlen ∈ {0, 32, 33}` | [x] |
| 6.13 | `crypto_aead_aegis128l_encrypt` | fixed KAT vectors (deterministic `k`, `npub`, `m`, `ad`) — the portable soft path must match the reference AEGIS-128L tag/ciphertext | [x] |
| **aegis256** (KEY 32, NPUB 32, ABYTES 32, RATE 16, portable soft impl) | | | |
| 6.14 | `crypto_aead_aegis256_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 32 / 32 / MIN(SIZE_MAX-32, 2^61-1)` | [x] |
| 6.15 | `crypto_aead_aegis256_keygen` | fill a 32-byte buffer; two calls differ | [x] |
| 6.16 | `crypto_aead_aegis256_encrypt` + `_decrypt` | round trip, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 32` | [x] |
| 6.17 | `crypto_aead_aegis256_encrypt` + `_decrypt` | as 6.16 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.18 | `crypto_aead_aegis256_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 16, 17, 32}` | [x] |
| 6.19 | `crypto_aead_aegis256_encrypt` + `_decrypt` | `mlen ∈ BIG_AEGIS256`, `adlen ∈ {0, 32, 33, 64}` — exercises `aegis256_absorb2` (32-byte stride), `absorb` (16-byte stride), `adlen % 16` tail, `aegis256_declast` | [x] |
| 6.20 | `crypto_aead_aegis256_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 32`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; must agree with the combined API | [x] |
| 6.21 | `crypto_aead_aegis256_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.22 | `crypto_aead_aegis256_decrypt_detached` | `m = NULL` verify-only; valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.23 | `crypto_aead_aegis256_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.24 | `crypto_aead_aegis256_encrypt` / `_decrypt` | in-place `c == m` for `mlen ∈ {0, 1, 16, 17, 32, 1024}` | [x] |
| 6.25 | `crypto_aead_aegis256_encrypt` / `_decrypt` | all-zero / all-`0xff` `k` and `npub`; `mlen ∈ {0, 16, 17}` | [x] |
| 6.26 | `crypto_aead_aegis256_encrypt` | fixed AEGIS-256 KAT vectors | [x] |
| **aes256gcm** — unavailable in this build (`is_available() == 0`; all ops `-1`/`ENOSYS`) | | | |
| 6.27 | `crypto_aead_aes256gcm_is_available` | no options; assert returns exactly `0` in the no-`HAVE_*` CMake configuration. Every other row in this block is conditioned on that | [x] |
| 6.28 | `crypto_aead_aes256gcm_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` / `_statebytes` | still functional despite unavailability; assert `32 / 0 / 12 / 16 / MIN(SIZE_MAX-16, 16*(2^32-2))` and `_statebytes() == (sizeof(state)+15) & ~15` (multiple of 16, non-zero) | [x] |
| 6.29 | `crypto_aead_aes256gcm_keygen` | still functional; fills 32 bytes; two calls differ | [x] |
| 6.30 | `crypto_aead_aes256gcm_encrypt` | `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `clen_p ∈ {NULL, non-NULL}`, `nsec = NULL`; **every** case ⇒ `-1`, `errno == ENOSYS`, `*clen_p` left at its pre-call poison value, `c` untouched | [x] |
| 6.31 | `crypto_aead_aes256gcm_encrypt_detached` | `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `maclen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`, `*maclen_p` poison preserved, `c`/`mac` untouched | [x] |
| 6.32 | `crypto_aead_aes256gcm_decrypt` | `clen ∈ {0, 1, 15, 16, 17, 48}` (both below and above ABYTES) × `adlen ∈ ADLEN` × `mlen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`, `*mlen_p` poison preserved | [x] |
| 6.33 | `crypto_aead_aes256gcm_decrypt_detached` | `clen ∈ MLEN` × `adlen ∈ ADLEN`, `m ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS`; `m` not even zeroed | [x] |
| 6.34 | `crypto_aead_aes256gcm_beforenm` | 16-byte-aligned `crypto_aead_aes256gcm_state` (heap via `sodium_malloc` and stack via `CRYPTO_ALIGN(16)`), valid 32-byte `k`; ⇒ `-1`/`ENOSYS`, state left uninitialised | [x] |
| 6.35 | `crypto_aead_aes256gcm_encrypt_afternm` | state from a failed `_beforenm` (the only kind obtainable); `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `clen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.36 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | same state; `mlen ∈ MLEN` × `adlen ∈ ADLEN` × `maclen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.37 | `crypto_aead_aes256gcm_decrypt_afternm` | same state; `clen ∈ {0, 15, 16, 17, 48}` × `adlen ∈ ADLEN` × `mlen_p ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.38 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | same state; `clen ∈ MLEN` × `adlen ∈ ADLEN`, `m ∈ {NULL, non-NULL}`; all ⇒ `-1`/`ENOSYS` | [x] |
| 6.39 | full aes256gcm "state API" sequence | `_beforenm` → `_encrypt_afternm` → `_decrypt_afternm` in order; assert the sequence never produces a successful round trip and each step independently reports `-1`/`ENOSYS` | [x] |
| **chacha20poly1305 "original"** (KEY 32, **NPUB 8**, ABYTES 16) | | | |
| 6.40 | `crypto_aead_chacha20poly1305_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 8 / 16 / SIZE_MAX-16` | [x] |
| 6.41 | `crypto_aead_chacha20poly1305_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.42 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | 8-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.43 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | as 6.42 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.44 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 16, 63, 64, 65}`. Note the **original** construction has *no* 16-byte zero-padding of `ad`/`c` in the MAC (`aead_chacha20poly1305.c:43-45, 63-64`), unlike the ietf variant — so unaligned `adlen` must produce a *different* tag from the ietf variant on the same inputs | [x] |
| 6.45 | `crypto_aead_chacha20poly1305_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 16, 17}` — crosses the 64-byte block and the `STREAM_POLY1305_CHUNK == 131072` chunk restart with a 64-bit `ic` counter | [x] |
| 6.46 | `crypto_aead_chacha20poly1305_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 16`, written *after* the crypto at `aead_chacha20poly1305.c:69-71`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; must match the combined API split at `mlen` | [x] |
| 6.47 | `crypto_aead_chacha20poly1305_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` — assert identical output to 6.46 | [x] |
| 6.48 | `crypto_aead_chacha20poly1305_decrypt_detached` | `m = NULL` verify-only (`aead_chacha20poly1305.c:232-234`); valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.49 | `crypto_aead_chacha20poly1305_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.50 | `crypto_aead_chacha20poly1305_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.51 | `crypto_aead_chacha20poly1305_encrypt` | fixed RFC-style KAT with 8-byte nonce | [x] |
| **chacha20poly1305_ietf** (KEY 32, **NPUB 12**, ABYTES 16) | | | |
| 6.52 | `crypto_aead_chacha20poly1305_ietf_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 12 / 16 / MIN(SIZE_MAX-16, 64*(2^32-1))` | [x] |
| 6.53 | `crypto_aead_chacha20poly1305_ietf_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.54 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | 12-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.55 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | as 6.54 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.56 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ MLEN` — exercises both `_pad0` padding calls `(0x10 - adlen) & 0xf` and `(0x10 - mlen) & 0xf` (`aead_chacha20poly1305.c:128, 146`) at every residue class mod 16 | [x] |
| 6.57 | `crypto_aead_chacha20poly1305_ietf_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 15, 16, 17}` — 32-bit `ic` counter across the `STREAM_POLY1305_CHUNK` restart | [x] |
| 6.58 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == 16`); `mlen ∈ MLEN`, `adlen ∈ ADLEN` | [x] |
| 6.59 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.60 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | `m = NULL` verify-only (`aead_chacha20poly1305.c:317-319`); `mlen ∈ MLEN` | [x] |
| 6.61 | `crypto_aead_chacha20poly1305_ietf_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.62 | `crypto_aead_chacha20poly1305_ietf_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.63 | `crypto_aead_chacha20poly1305_ietf_encrypt` | RFC 8439 KAT vectors (12-byte nonce) | [x] |
| 6.64 | `crypto_aead_chacha20poly1305_ietf_*` vs `crypto_aead_chacha20poly1305_*` | same key, same first-8-bytes-of-nonce: assert the two families produce **different** ciphertexts/tags (different nonce layout and different MAC framing) — guards against collapsing them in translation | [x] |
| **xchacha20poly1305_ietf** (KEY 32, **NPUB 24**, ABYTES 16) | | | |
| 6.65 | `crypto_aead_xchacha20poly1305_ietf_keybytes` / `_nsecbytes` / `_npubbytes` / `_abytes` / `_messagebytes_max` | assert `32 / 0 / 24 / 16 / SIZE_MAX-16`; also the `crypto_aead_xchacha20poly1305_IETF_*` uppercase aliases resolve identically | [x] |
| 6.66 | `crypto_aead_xchacha20poly1305_ietf_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.67 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | 24-byte `npub`, `ad = NULL`/`adlen = 0`, `nsec = NULL`, out-length pointers non-NULL; `mlen ∈ MLEN`; assert `clen == mlen + 16` | [x] |
| 6.68 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | as 6.67 with `clen_p = NULL` / `mlen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.69 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | `adlen ∈ ADLEN` × `mlen ∈ MLEN` — every residue class mod 16 for both `_pad0` calls (`aead_xchacha20poly1305.c:46, 73`) | [x] |
| 6.70 | `crypto_aead_xchacha20poly1305_ietf_encrypt` + `_decrypt` | `mlen ∈ BIG_CHACHA`, `adlen ∈ {0, 16, 17}` — plus the `chunk` selection branch at `aead_xchacha20poly1305.c:56-58` (`mlen <= 64*(0xffffffff-1)` ⇒ chunked, else single pass); only the chunked side is reachable for realistic sizes but the branch must be preserved | [x] |
| 6.71 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` + `_decrypt_detached` | `maclen_p != NULL` (`*maclen_p == crypto_aead_chacha20poly1305_ietf_ABYTES == 16`); `mlen ∈ MLEN`, `adlen ∈ ADLEN`; internally goes through HChaCha20 subkey derivation + a 12-byte `npub2` with 4 leading zero bytes (`aead_xchacha20poly1305.c:158-163`) | [x] |
| 6.72 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | `maclen_p = NULL`; `mlen ∈ MLEN` | [x] |
| 6.73 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | `m = NULL` verify-only (`aead_xchacha20poly1305.c:132-134`); valid ⇒ `0`, tampered ⇒ `-1`; `mlen ∈ MLEN` | [x] |
| 6.74 | `crypto_aead_xchacha20poly1305_ietf_decrypt` / `_decrypt_detached` | `nsec` out-param `NULL` vs poisoned buffer | [x] |
| 6.75 | `crypto_aead_xchacha20poly1305_ietf_encrypt` / `_decrypt` | in-place `c == m`; `mlen ∈ {0, 1, 64, 65, 131073}` | [x] |
| 6.76 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | fixed KAT vectors (24-byte nonce), incl. all-zero `npub` and all-`0xff` `npub` | [x] |
| 6.77 | `crypto_aead_xchacha20poly1305_ietf_encrypt` cross-check | equal to `crypto_aead_chacha20poly1305_ietf_encrypt` under `k2 = hchacha20(npub[0..15], k)` and `npub2 = 0x00000000 || npub[16..23]` — confirms the XChaCha20 construction wiring | [x] |
| **secretbox — generic / xsalsa20poly1305 (KEY 32, NONCE 24, MAC 16, ZERO 32, BOXZERO 16)** | | | |
| 6.78 | `crypto_secretbox_keybytes` / `_noncebytes` / `_macbytes` / `_zerobytes` / `_boxzerobytes` / `_messagebytes_max` / `_primitive` | assert `32 / 24 / 16 / 32 / 16 / stream_MESSAGEBYTES_MAX-16` and `_primitive() == "xsalsa20poly1305"` | [x] |
| 6.79 | `crypto_secretbox_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.80 | `crypto_secretbox_easy` + `crypto_secretbox_open_easy` | round trip, out buffer `mlen + 16`; `mlen ∈ MLEN`; assert `c[0..16)` is the MAC and `open_easy` recovers `m` | [x] |
| 6.81 | `crypto_secretbox_easy` + `crypto_secretbox_open_easy` | `mlen ∈ BIG_SECRETBOX` — crosses the `mlen0 = min(mlen, 64 - 32) == 32` first-block special case (`crypto_secretbox_easy.c:49-63`) and the `STREAM_POLY1305_CHUNK` restart (`:71-82`) | [x] |
| 6.82 | `crypto_secretbox_detached` + `crypto_secretbox_open_detached` | separate 16-byte `mac` buffer; `mlen ∈ MLEN` ∪ `BIG_SECRETBOX`; assert `detached` output == `easy` output split at 16 | [x] |
| 6.83 | `crypto_secretbox_open_detached` | `m = NULL` verify-only (`crypto_secretbox_easy.c:131-134`); valid mac ⇒ `0`, tampered ⇒ `-1`; `clen ∈ MLEN` | [x] |
| 6.84 | `crypto_secretbox_easy` / `_open_easy` in-place | `c == m` and the documented `m = c + 16` / `c = m - 16` overlap patterns that trigger the `memmove` branches (`crypto_secretbox_easy.c:40-46` and `:145-151`); `mlen ∈ {0, 1, 31, 32, 33, 64, 4096}` | [x] |
| 6.85 | `crypto_secretbox_detached` / `_open_detached` | fully disjoint buffers (the `memmove` branches *not* taken) for the same `mlen` set as 6.84 — both sides of each overlap branch must be covered | [x] |
| 6.86 | `crypto_secretbox` (NaCl-style) + `crypto_secretbox_open` | `m` buffer with `m[0..31] = 0` zero padding, `mlen = 32 + plaintext_len` for `plaintext_len ∈ MLEN`; assert `c[0..15] == 0` (BOXZEROBYTES forced to zero at `secretbox_xsalsa20poly1305.c:20-22`), MAC at `c[16..31]`, and `crypto_secretbox_open` returns `0` with `m[0..31]` re-zeroed and plaintext at `m + 32` | [x] |
| 6.87 | `crypto_secretbox` + `crypto_secretbox_open` | large NaCl-style: `mlen = 32 + n` for `n ∈ {0, 1, 32, 63, 64, 65, 4096, 131073}` (the xsalsa20 path is a single `crypto_stream_xsalsa20_xor` with no chunking, unlike `_easy`) | [x] |
| 6.88 | `crypto_secretbox_xsalsa20poly1305` / `_open` | called directly (not via the `crypto_secretbox` wrapper at `crypto_secretbox.c:47-61`); assert byte-identical results to 6.86 | [x] |
| 6.89 | `crypto_secretbox_xsalsa20poly1305_keybytes` / `_noncebytes` / `_zerobytes` / `_boxzerobytes` / `_macbytes` / `_messagebytes_max` / `_keygen` | assert `32 / 24 / 32 / 16 / 16 / …` and that `crypto_secretbox_*` aliases resolve to the same values | [x] |
| 6.90 | `crypto_secretbox_easy` vs `crypto_secretbox` | same `k`, `n`, plaintext: assert `easy(c, m, len)` output equals `secretbox(c', 32-zero-padded m, 32+len)` shifted by 16 (`c == c' + 16`) — the two APIs are the same construction with different framing | [x] |
| 6.91 | `crypto_secretbox_easy` / `crypto_secretbox` | corner keys/nonces: all-zero `k`, all-`0xff` `k`, all-zero `n`, all-`0xff` `n`, `n` with a non-zero high half only (`n + 16` is the salsa20 nonce, `n[0..15]` the hsalsa20 input); `mlen ∈ {0, 1, 32, 33}` | [x] |
| 6.92 | `crypto_secretbox` / `crypto_secretbox_open` | NaCl KAT vectors (the classic libsodium/NaCl `secretbox` test vector) | [x] |
| **secretbox — xchacha20poly1305 primitive family (KEY 32, NONCE 24, MAC 16)** | | | |
| 6.93 | `crypto_secretbox_xchacha20poly1305_keybytes` / `_noncebytes` / `_macbytes` / `_messagebytes_max` | assert `32 / 24 / 16 / stream_xchacha20_MESSAGEBYTES_MAX - 16`. Note there is **no** `_zerobytes`/`_boxzerobytes`/`_keygen`/`_primitive` in this family | [x] |
| 6.94 | `crypto_secretbox_xchacha20poly1305_easy` + `_open_easy` | round trip, out buffer `mlen + 16`; `mlen ∈ MLEN` | [x] |
| 6.95 | `crypto_secretbox_xchacha20poly1305_easy` + `_open_easy` | `mlen ∈ BIG_SECRETBOX` — crosses the `mlen0 = min(mlen, 64-32) == 32` first-block case (`secretbox_xchacha20poly1305.c:51-72`). NB unlike the xsalsa20 variant this one does **not** chunk at 131072; it does a single `crypto_stream_chacha20_xor_ic` for the tail | [x] |
| 6.96 | `crypto_secretbox_xchacha20poly1305_detached` + `_open_detached` | separate `mac` buffer; `mlen ∈ MLEN` ∪ `BIG_SECRETBOX`; must equal `_easy` output split at 16 | [x] |
| 6.97 | `crypto_secretbox_xchacha20poly1305_open_detached` | `m = NULL` verify-only (`secretbox_xchacha20poly1305.c:124-127`); valid ⇒ `0`, tampered ⇒ `-1`; `clen ∈ MLEN` | [x] |
| 6.98 | `crypto_secretbox_xchacha20poly1305_easy` / `_open_easy` in-place | `c == m` plus the `m = c + 16` overlap patterns that hit the `memmove` branches (`secretbox_xchacha20poly1305.c:42-48`, `:138-144`); `mlen ∈ {0, 1, 31, 32, 33, 64, 4096}` | [x] |
| 6.99 | `crypto_secretbox_xchacha20poly1305_detached` / `_open_detached` | fully disjoint buffers (memmove branches not taken), same `mlen` set as 6.98 | [x] |
| 6.100 | `crypto_secretbox_xchacha20poly1305_easy` | corner keys/nonces: all-zero / all-`0xff` `k`; all-zero / all-`0xff` `n`; note the split `n[0..15]` → hchacha20 input, `n + 16` → chacha20 nonce; `mlen ∈ {0, 1, 32, 33}` | [x] |
| 6.101 | `crypto_secretbox_xchacha20poly1305_easy` | fixed KAT vectors; assert output **differs** from `crypto_secretbox_easy` on identical `k`/`n`/`m` (different primitive) | [x] |
| 6.102 | `crypto_secretbox_xchacha20poly1305_easy` + `crypto_secretbox_open_easy` (mismatched families) | cross-family: encrypt with xchacha20poly1305, open with the xsalsa20poly1305 default; must fail with `-1` for `mlen ∈ {0, 1, 32, 33}` (and vice versa) | [x] |
| **secretstream_xchacha20poly1305** (KEY 32, HEADER 24, ABYTES 17, tags 0x00/0x01/0x02/0x03) | | | |
| 6.103 | `crypto_secretstream_xchacha20poly1305_keybytes` / `_headerbytes` / `_abytes` / `_statebytes` / `_messagebytes_max` | assert `32 / 24 / 17 / sizeof(state) / MIN(SIZE_MAX-17, 64*(2^32-2))` | [x] |
| 6.104 | `crypto_secretstream_xchacha20poly1305_tag_message` / `_tag_push` / `_tag_rekey` / `_tag_final` | assert `0x00 / 0x01 / 0x02 / 0x03` and that `TAG_FINAL == (TAG_PUSH \| TAG_REKEY)` | [x] |
| 6.105 | `crypto_secretstream_xchacha20poly1305_keygen` | fills 32 bytes; two calls differ | [x] |
| 6.106 | `crypto_secretstream_xchacha20poly1305_init_push` | writes 24 random header bytes, resets the 4-byte counter to `{1,0,0,0}`, copies the 8-byte inonce, zeroes `state->_pad`; returns `0`; two inits with the same `k` give different headers | [x] |
| 6.107 | `_init_push` + `_init_pull` | pull side initialised from the pushed header with the same `k`; assert both states derive the same `state->k` (observable via a successful first `_pull`) | [x] |
| 6.108 | `_init_push` → `_push(TAG_MESSAGE)` → `_init_pull` → `_pull` | single-frame session, `ad = NULL`/`adlen = 0`, `outlen_p`/`mlen_p`/`tag_p` all non-NULL; `mlen ∈ MLEN`; assert `*outlen_p == mlen + 17`, `*mlen_p == mlen`, `*tag_p == TAG_MESSAGE` | [x] |
| 6.109 | same session as 6.108 | `outlen_p = NULL` on push and `mlen_p = NULL` / `tag_p = NULL` on pull (all four combinations of the two pull pointers); `mlen ∈ MLEN`; output bytes must be identical to 6.108 | [x] |
| 6.110 | multi-frame session, all `TAG_MESSAGE` | 1, 2, 3, 8, 64 frames, each with `mlen ∈ MLEN` (rotating); assert the inonce/counter chaining (`XOR_BUF(STATE_INONCE, mac, 8)` + `sodium_increment(counter, 4)` at `secretstream_xchacha20poly1305.c:164-167`) keeps push and pull in lockstep | [x] |
| 6.111 | session with `TAG_PUSH` (0x01) frames | `TAG_MESSAGE`, `TAG_PUSH`, `TAG_MESSAGE` sequence; `TAG_PUSH` does **not** have the `0x02` bit so it must **not** trigger an implicit rekey; assert `*tag_p == TAG_PUSH` and the stream continues | [x] |
| 6.112 | session with `TAG_REKEY` (0x02) frames | `TAG_MESSAGE`, `TAG_REKEY`, `TAG_MESSAGE` sequence; the `0x02` bit triggers the implicit `_rekey()` on **both** push and pull (`:168-172`, `:250-254`); assert the post-rekey frames still round trip and that the derived key changed | [x] |
| 6.113 | session with `TAG_FINAL` (0x03) | `TAG_MESSAGE` × n then `TAG_FINAL`; assert `*tag_p == TAG_FINAL` on the last pull and that `TAG_FINAL` also triggers the implicit rekey (it has the `0x02` bit) | [x] |
| 6.114 | full tag matrix in one session | ordered sequence `TAG_MESSAGE, TAG_PUSH, TAG_MESSAGE, TAG_REKEY, TAG_MESSAGE, TAG_PUSH, TAG_FINAL`, each frame with a different `mlen` drawn from `MLEN`; assert every `*tag_p` matches what was pushed | [x] |
| 6.115 | explicit `crypto_secretstream_xchacha20poly1305_rekey` on **both** sides | push `TAG_MESSAGE`, call `_rekey(push_state)` and `_rekey(pull_state)` at the same point in the sequence, push/pull another `TAG_MESSAGE`; assert the session stays in sync and the counter resets to 1 | [x] |
| 6.116 | explicit `_rekey` repeated | `_rekey` called 0, 1, 2, 5 times consecutively (symmetrically on both states) before the next frame; each count must still round trip | [x] |
| 6.117 | explicit `_rekey` interleaved with an implicit `TAG_REKEY` | `_push(TAG_REKEY)` followed by an explicit `_rekey` on both sides; assert both rekeys are applied in the same order on both sides | [x] |
| 6.118 | push/pull with `ad` present | `adlen ∈ ADLEN` × `mlen ∈ {0, 1, 15, 16, 17, 63, 64, 65}` — exercises the `(0x10 - adlen) & 0xf` padding at `:136-137` / `:213-214` at every residue mod 16 | [x] |
| 6.119 | push/pull with `ad` varying **per frame** | frame 0 with `ad = NULL`/0, frame 1 with a 17-byte `ad`, frame 2 with a 32-byte `ad`, frame 3 with `ad = non-NULL`/`adlen = 0`; each pull must supply the matching `ad` | [x] |
| 6.120 | push/pull with large messages | `mlen ∈ {4096, 65536, 131072, 131073, 262145}` — the secretstream path calls `crypto_stream_chacha20_ietf_xor_ic(..., ic = 2)` in a **single** pass (no chunking, `:147`, `:245`), unlike the AEAD path | [x] |
| 6.121 | push/pull message-length boundary around the quirky padding | `mlen ∈ {0, 15, 16, 17, 47, 48, 49, 63, 64, 65}` — the padding expression `(0x10 - (sizeof block) + mlen) & 0xf` at `:149-151` / `:226-228` is the documented off-by-`sizeof block` quirk (`sizeof block == 64`, so it reduces to `mlen & 0xf`); the translation must reproduce this bug exactly, and the `slen` length field is `64 + mlen`, not `mlen` (`:155`, `:232`) | [x] |
| 6.122 | `_push` with a `tag` value outside `{0x00, 0x01, 0x02, 0x03}` | `tag ∈ {0x04, 0x7f, 0x80, 0xfe, 0xff}` — never validated; assert `_push` returns `0`, `_pull` reports the same `*tag_p`, and that any tag with the `0x02` bit set (`0x06`, `0x7f`, `0xff`, …) triggers the implicit rekey on both sides | [x] |
| 6.123 | `_init_pull` with an arbitrary 24-byte header | all-zero header, all-`0xff` header, and a header from a *different* `_init_push`; assert `_init_pull` returns `0` regardless, and the mismatch only shows up as `-1` on the first `_pull` | [x] |
| 6.124 | `_push` / `_pull` in-place | `out == m` and `m == in` aliasing where the API permits it (`out` is `1 + mlen + 16` bytes, `in` is `mlen + 17`); `mlen ∈ {0, 1, 64, 65}` — note the code writes `out[0]` before the `xor_ic` into `out + 1`, so `in`-place pull needs `m == in` handling | [x] |
| 6.125 | `_push` with corner keys | all-zero `k`, all-`0xff` `k` into `_init_push`; `mlen ∈ {0, 1, 64}`; full round trip | [x] |
| 6.126 | `_push` determinism given a fixed header | drive `_init_pull` from a hard-coded header and hard-coded `k`, then `_push`-equivalent framing via a second `_init_pull`-seeded state (or a KAT of `{header, k, [(tag, ad, m)…]}` → concatenated ciphertext frames) so the byte-exact stream format is pinned, including the `state->_pad` zeroing at `:62`/`:77` | [x] |
| 6.127 | `_statebytes` vs actual state usage | allocate exactly `crypto_secretstream_xchacha20poly1305_statebytes()` bytes (heap, unaligned-by-1 offsets included) for the state and run a full session; assert no over-read/over-write and that `sizeof(crypto_secretstream_xchacha20poly1305_state)` is what `_statebytes()` reports | [x] |
| 6.128 | 32-bit counter wrap | force `STATE_COUNTER` near `0xffffffff` (either by direct state manipulation in a white-box test or by documenting it as unreachable) so that `sodium_is_zero(counter, 4)` at `:169-170` / `:251-252` fires and an implicit rekey happens on both sides without an explicit `TAG_REKEY` | [x] |
| 6.129 | cross-API check: secretstream vs `crypto_aead_xchacha20poly1305_ietf` | assert secretstream framing is **not** interchangeable with the AEAD (extra 1-byte tag, `ic` starting at 2, `slen = 64 + mlen`) — encrypting with one and decrypting with the other must fail | [x] |

### Coverage notes (Phase B/C, area 6)

Test files: `tests/a6_aead.rs`, `tests/a6_aes256gcm.rs`, `tests/a6_secretbox.rs`,
`tests/a6_secretstream.rs` (69 test functions, all green).

* Row 6.65: `crypto_aead_xchacha20poly1305_IETF_*` are **preprocessor macro aliases**
  (`crypto_aead_xchacha20poly1305.h:90-94`), not exported symbols, so they cannot be resolved
  through `dlsym`; the lowercase getters they alias are verified instead.
* Rows 6.13 / 6.26 / 6.51 / 6.76 / 6.92 / 6.126: implemented as *pinned* vectors — fully
  deterministic hard-coded `k` / `npub` (or header) / `m` / `ad` compared byte-for-byte between
  the two libraries — plus, for the constructions, an independent re-derivation from the
  already-verified low-level primitives (`crypto_core_hsalsa20` + `crypto_stream_salsa20_xor{,_ic}`
  + `crypto_onetimeauth_poly1305` for secretbox, `crypto_core_hchacha20` + `crypto_stream_chacha20`
  for the xchacha20poly1305 secretbox, `crypto_core_hchacha20` + `crypto_aead_chacha20poly1305_ietf`
  for xchacha20poly1305_ietf). Row 6.63 additionally pins the absolute RFC 8439 §2.8.2 tag.
* Row 6.128 (secretstream 32-bit counter wrap) is reached white-box, by writing
  `0xffffffff` into `STATE_COUNTER` on both sides before the next `_push`/`_pull`.

## Area 7 — scalarmult / sign / box / kx / kdf / kem

Files covered: as listed in `errors_7.md` (same file set, read in full).

### Configuration axes extracted from the source

| axis | values |
|------|--------|
| curve25519 scalarmult entry point | `crypto_scalarmult` / `crypto_scalarmult_curve25519` (full, 2-arg point) vs `crypto_scalarmult_base` / `crypto_scalarmult_curve25519_base` |
| curve25519 scalar shape | arbitrary 32 bytes (always clamped: `t[0] &= 248; t[31] &= 127; t[31] |= 64`), all-zero, all-`0xff`, `L`, pre-clamped |
| curve25519 point shape | basepoint `09 00 … 00`, random valid point, non-canonical-but-not-blocklisted (`>= p`), blocklisted small-order (7 encodings) |
| ed25519 scalarmult variant | `crypto_scalarmult_ed25519` (clamped, point) / `_noclamp` (point) / `_base` (clamped) / `_base_noclamp` |
| ristretto255 scalarmult variant | `crypto_scalarmult_ristretto255` (point) / `_base`; never clamped, only `t[31] &= 127` |
| sign API shape | attached one-shot (`crypto_sign` / `_open`) vs detached one-shot (`crypto_sign_detached` / `_verify_detached`) vs multipart prehashed (`crypto_sign_init` / `_update` / `_final_create` / `_final_verify`) |
| sign namespace | generic `crypto_sign_*` vs explicit `crypto_sign_ed25519*` / `crypto_sign_ed25519ph_*` (the generic ones are pure aliases) |
| multipart chunking | 0 / 1 / 2 / many `_update` calls; chunk boundaries at 0, 1, 63, 64, 65, 127, 128, 129 bytes |
| keypair source | `_seed_keypair(seed)` (deterministic) vs `_keypair()` (`randombytes_buf`) |
| key conversions | `crypto_sign_ed25519_sk_to_seed`, `_sk_to_pk`, `_pk_to_curve25519`, `_sk_to_curve25519` |
| message length | 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 8192 (spans SHA-512 block boundaries at 128 and the 112-byte padding cliff) |
| `siglen_p` / `smlen_p` / `mlen_p` / `m` | NULL vs non-NULL (each is explicitly NULL-checked) |
| box AEAD primitive | `curve25519xsalsa20poly1305` (default, generic `crypto_box_*`) vs `curve25519xchacha20poly1305` |
| box API shape | `_easy` / `_open_easy`; `_detached` / `_open_detached`; `_beforenm` + `_easy_afternm` / `_open_easy_afternm` / `_detached_afternm` / `_open_detached_afternm`; NaCl padded `crypto_box` / `_open` / `_afternm` / `_open_afternm` (xsalsa only); `_seal` / `_seal_open` |
| kx role | `crypto_kx_client_session_keys` vs `crypto_kx_server_session_keys` (loop order is swapped) |
| kx output pointers | both non-NULL, `rx == NULL`, `tx == NULL` |
| kdf primitive | blake2b (`crypto_kdf_*`) vs hkdf-sha256 vs hkdf-sha512 |
| kdf blake2b axes | `subkey_len` ∈ {16 (MIN), 17, 31, 32, 33, 63, 64 (MAX)}; `subkey_id` ∈ {0, 1, 2, 2^32-1, 2^32, 2^63, 2^64-1}; 8-byte `ctx` |
| hkdf axes | `extract` one-shot vs `extract_init` + N × `extract_update` + `extract_final`; `salt_len` ∈ {0, 1, 32, 64, 128, 129}; `ikm_len` ∈ {0, 1, 32, 64, 1000}; `out_len` ∈ {0 (MIN), 1, 31, 32, 33, 63, 64, 65, 8160 / 16320 (MAX)}; `ctx_len` ∈ {0, 1, 8, 64} |
| kem primitive | `mlkem768` vs `xwing` vs generic `crypto_kem_*` (→ xwing) |
| kem API shape | `_keypair` / `_seed_keypair`; `_enc` (randomised) / `_enc_deterministic`; `_dec` |
| dispatch | `_crypto_scalarmult_curve25519_pick_best_implementation` — only one value on this build (ref10); sandy2x not selected |

### CONFIGURATION-SURFACE table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 7.1 | `crypto_scalarmult_curve25519_base`, `crypto_scalarmult_base` | RFC 7748 vector: `n = a5 46 e3 6b f0 52 7c 9d 3b 16 15 4b 82 46 5e dd 62 14 4c 0a c1 fc 5a 18 50 6a 22 44 ba 44 9a c4` → `q = 8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a` | [x] |
| 7.2 | `crypto_scalarmult_curve25519_base` | `n` = 32 zero bytes (clamps to scalar `2^254`) → succeeds, `0` | [x] |
| 7.3 | `crypto_scalarmult_curve25519_base` | `n` = 32 `0xff` bytes (clamps to `2^254 + …`), and `n = L` little-endian → succeeds, `0` | [x] |
| 7.4 | `crypto_scalarmult_curve25519_base` | `n` already clamped (bit 255 clear, bit 254 set, low 3 bits clear) — clamping must be idempotent | [x] |
| 7.5 | `crypto_scalarmult_curve25519_base` | in-place aliasing: `q == n` (the impl copies `n` into `t = q` first, so this is the *intended* usage) | [x] |
| 7.6 | `crypto_scalarmult_curve25519`, `crypto_scalarmult` | RFC 7748 X25519 vector 1: `n = a546…49ac4`, `p = e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c` → `q = c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552` | [x] |
| 7.7 | `crypto_scalarmult_curve25519`, `crypto_scalarmult` | `p` = basepoint `09 00 … 00` with a random clamped `n`; cross-check equality with `crypto_scalarmult_curve25519_base(q, n)` — the two code paths are structurally different (Montgomery ladder vs `ge25519_scalarmult_base` + `edwards_to_montgomery`) and must agree | [x] |
| 7.8 | `crypto_scalarmult_curve25519` | `p` non-canonical but **not** blocklisted, e.g. `p` = `ef ff … ff 7f` (`= p+2`) or any value in `[p+2, 2^255)` → accepted, `fe25519_frombytes` reduces mod `p`; returns `0` | [x] |
| 7.9 | `crypto_scalarmult_curve25519` | `p` with bit 255 set (`p[31] |= 0x80`) on an otherwise valid point → accepted; `fe25519_frombytes` masks bit 255 | [x] |
| 7.10 | `crypto_scalarmult_curve25519` | DH agreement round trip: `base(pkA, skA)`, `base(pkB, skB)`, then `mult(s1, skA, pkB)` == `mult(s2, skB, pkA)`; over 100 random keypairs | [x] |
| 7.11 | `crypto_scalarmult_curve25519` | `n` = 32 zero bytes with a valid `p` (clamping ⇒ effective scalar `2^254`, so this **succeeds** with `0`, unlike ed25519/ristretto) | [x] |
| 7.12 | `_crypto_scalarmult_curve25519_pick_best_implementation` | called (e.g. via `sodium_init`) then re-run 7.6 — on this build (no `HAVE_AVX_ASM`) the selected implementation is always ref10, so results must be unchanged | [x] |
| 7.13 | `crypto_scalarmult_ed25519_base` | random 32-byte `n` → 32-byte compressed Edwards point; verify `ge25519_is_canonical` holds and the point is on the main subgroup | [x] |
| 7.14 | `crypto_scalarmult_ed25519_base` vs `crypto_scalarmult_ed25519_base_noclamp` | the same `n` through both: with clamped `n` (bit 254 set, low 3 bits clear, bit 255 clear) the two must agree; with any other `n` they must differ | [x] |
| 7.15 | `crypto_scalarmult_ed25519_base_noclamp` | `n = 1` (`01 00 … 00`) → must equal the ed25519 basepoint encoding `5866666666666666666666666666666666666666666666666666666666666666` | [x] |
| 7.16 | `crypto_scalarmult_ed25519_base_noclamp` | `n = 2, 3, 8` and `n = L - 1` (→ `-B`); check additive homomorphism against `crypto_core_ed25519_add` where available | [x] |
| 7.17 | `crypto_scalarmult_ed25519_base_noclamp` | `n` with bit 255 set — must give the same result as `n` with bit 255 cleared (`t[31] &= 127` is applied on all four ed25519 paths) | [x] |
| 7.18 | `crypto_scalarmult_ed25519` | `p` = output of `crypto_scalarmult_ed25519_base(n1)`, `n = n2` → equals `crypto_scalarmult_ed25519_base_noclamp(clamp(n2) · clamp(n1) mod L)`; commutativity check `mult(n1, base(n2)) == mult(n2, base(n1))` for clamped scalars | [x] |
| 7.19 | `crypto_scalarmult_ed25519_noclamp` | `p` = basepoint encoding, `n` random with bit 255 clear → must equal `crypto_scalarmult_ed25519_base_noclamp(n)` | [x] |
| 7.20 | `crypto_scalarmult_ed25519` / `_noclamp` | in-place aliasing `q == n` (documented pattern: `unsigned char *t = q`) and `q == p` (the point is decoded into `P` *before* `t = q` is written, so this is safe) | [x] |
| 7.21 | `crypto_scalarmult_ed25519_base` / `_base_noclamp` | in-place aliasing `q == n` | [x] |
| 7.22 | `crypto_scalarmult_ristretto255_base` | `n = 1` → the canonical ristretto255 basepoint `e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76`; also `n = 2, 3, 15` against the published ristretto255 multiples-of-basepoint vectors | [x] |
| 7.23 | `crypto_scalarmult_ristretto255_base` | random `n`; then feed the result as `p` into `crypto_scalarmult_ristretto255` with a second scalar and check DH commutativity | [x] |
| 7.24 | `crypto_scalarmult_ristretto255` | `p` = a valid ristretto255 encoding, `n` with bit 255 set vs cleared → identical results (`t[31] &= 127`) | [x] |
| 7.25 | `crypto_scalarmult_ristretto255` | `n = 1` with any valid `p` → `q == p` (identity map; verifies canonical re-encoding) | [x] |
| 7.26 | `crypto_scalarmult_ristretto255` / `_base` | in-place aliasing `q == n`; and `q == p` for the point variant | [x] |
| 7.27 | `crypto_scalarmult_ristretto255` vs `crypto_scalarmult_ed25519_noclamp` | same `n`, `p` chosen so both decode — results must **differ** (different encodings/cofactor handling); documents that the two are not interchangeable | [x] |
| 7.28 | `crypto_scalarmult_bytes`, `_scalarbytes`, `_primitive`, `crypto_scalarmult_curve25519_bytes/_scalarbytes`, `crypto_scalarmult_ed25519_bytes/_scalarbytes`, `crypto_scalarmult_ristretto255_bytes/_scalarbytes` | all 10 accessors → `32` × 9 and `"curve25519"` | [x] |
| 7.29 | `crypto_sign_ed25519_seed_keypair` | RFC 8032 test 1 seed `9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60` → `pk = d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`, `sk = seed ‖ pk` | [x] |
| 7.30 | `crypto_sign_ed25519_seed_keypair`, `crypto_sign_seed_keypair` | seed = 32 zero bytes; seed = 32 `0xff` bytes; the generic alias must produce byte-identical output to the ed25519 form | [x] |
| 7.31 | `crypto_sign_ed25519_keypair`, `crypto_sign_keypair` | randomised: verify `sk[0..31]` is the seed, `sk[32..63] == pk`, and `seed_keypair(sk[0..31])` reproduces both | [x] |
| 7.32 | `crypto_sign_ed25519_sk_to_seed`, `_sk_to_pk` | round trip after `_keypair`: `sk_to_seed(sk) == sk[0..31]`, `sk_to_pk(sk) == pk`; also with overlapping buffers (both use `memmove`) | [x] |
| 7.33 | `crypto_sign_ed25519_detached`, `crypto_sign_detached` | RFC 8032 test 1: empty message, seed as 7.29 → `sig = e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b`; `siglen_p` non-NULL must receive `64` | [x] |
| 7.34 | `crypto_sign_ed25519_detached` | RFC 8032 tests 2 (1-byte `72`), 3 (2-byte `af82`), and 1024 (the 1023-byte message) → published signatures | [x] |
| 7.35 | `crypto_sign_ed25519_detached` | `siglen_p == NULL` → still succeeds, `sig` written, no store | [x] |
| 7.36 | `crypto_sign_ed25519_detached` | message lengths 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024, 8192 — verifies the SHA-512 block/padding boundaries inside the two `hinit`+`update`+`final` passes | [x] |
| 7.37 | `crypto_sign_ed25519`, `crypto_sign` (attached) | `sm` buffer of `mlen + 64`; message lengths as 7.36; `*smlen_p == mlen + 64`; `sm[64…] == m`; `sm[0..63]` equals the detached signature over the same message | [x] |
| 7.38 | `crypto_sign_ed25519` | `smlen_p == NULL` → succeeds; and `m == sm + 64` (in-place signing — the `memmove` at `sign.c:111` is a no-op then) | [x] |
| 7.39 | `crypto_sign_ed25519` | `mlen == 0` → `sm` is exactly 64 bytes, `*smlen_p == 64` | [x] |
| 7.40 | `crypto_sign_ed25519_open`, `crypto_sign_open` | round trip against 7.37 for every message length; `*mlen_p == smlen - 64`, `m == original` | [x] |
| 7.41 | `crypto_sign_ed25519_open` | `m == NULL` (verify-only mode) with `mlen_p` non-NULL → `0`, `*mlen_p` set, nothing written | [x] |
| 7.42 | `crypto_sign_ed25519_open` | `mlen_p == NULL` with `m` non-NULL → `0`, message copied | [x] |
| 7.43 | `crypto_sign_ed25519_open` | `m == sm` (fully in-place open — the `memmove` at `open.c:95` shifts down by 64) | [x] |
| 7.44 | `crypto_sign_ed25519_open` | `smlen == 64` exactly (empty signed message) → `0`, `*mlen_p == 0` | [x] |
| 7.45 | `crypto_sign_ed25519_verify_detached`, `crypto_sign_verify_detached` | valid `sig`/`m`/`pk` for every message length of 7.36 → `0`; the generic alias must agree bit-for-bit with the ed25519 form | [x] |
| 7.46 | `crypto_sign_ed25519_verify_detached` — strict-vs-compat axis | `sig` with `(sig[63] & 240) == 0` (short-circuits the `sc25519_is_canonical` call) vs `(sig[63] & 240) != 0` with a canonical `S` (takes the canonicality call and passes) — both must return `0`. Documents that the build has no `ED25519_COMPAT`, so `sig[63] & 224` is *not* the guard | [x] |
| 7.47 | `crypto_sign_ed25519_verify_detached` | `pk` of order `8L` (valid point, non-small-order, off the main subgroup) with a signature that verifies cofactored → **accepted** (`0`), unlike `pk_to_curve25519` | [x] |
| 7.48 | `_crypto_sign_ed25519_verify_detached` cofactored acceptance | `sig` with a torsion component added to `R` such that `check` is small-order but not the identity → `0` | [x] |
| 7.49 | `crypto_sign_init`, `crypto_sign_update`, `crypto_sign_final_create` / `crypto_sign_ed25519ph_init/_update/_final_create` | **0 update calls** (prehash = SHA-512 of the empty string) → sign, then `crypto_sign_final_verify` with a fresh `init`+0 updates → `0` | [x] |
| 7.50 | multipart sign | **1 update call** with lengths 0, 1, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024 | [x] |
| 7.51 | multipart sign | **many update calls**: split a 1024-byte message as 1+1+…, as 127+1+128+…, as 64×16, and as one 1023-byte + one 1-byte chunk — all must yield the identical signature (streaming invariance) | [x] |
| 7.52 | multipart sign | `crypto_sign_final_create` with `siglen_p == NULL` and with non-NULL (`== 64`) | [x] |
| 7.53 | multipart sign vs one-shot | `crypto_sign_ed25519ph_final_create` over message `M` must **not** equal `crypto_sign_ed25519_detached` over `M` (`DOM2PREFIX` domain separation) — and `final_verify` must accept only its own | [x] |
| 7.54 | `crypto_sign_final_verify`, `crypto_sign_ed25519ph_final_verify` | round trip for 0/1/many-chunk configurations of 7.49–7.51; also with a state chunked differently on the verify side than on the sign side (must still verify — only the concatenation matters) | [x] |
| 7.55 | `crypto_sign_statebytes`, `crypto_sign_ed25519ph_statebytes` | both → `sizeof(crypto_hash_sha512_state)` = `208` on LP64; a heap-allocated state of exactly that size must work | [x] |
| 7.56 | `crypto_sign_ed25519_pk_to_curve25519` | after `crypto_sign_ed25519_seed_keypair(pk, sk, seed)`: `pk_to_curve25519(cpk, pk)` must equal `crypto_scalarmult_curve25519_base(cpk2, sk_to_curve25519(csk, sk))` — i.e. the two conversions must be mutually consistent | [x] |
| 7.57 | `crypto_sign_ed25519_sk_to_curve25519` | `csk = SHA-512(sk[0..31])[0..31]` clamped; verify against the value implied by `crypto_sign_ed25519_seed_keypair`'s internal clamp; input `sk` = 64 zero bytes and 64 `0xff` bytes | [x] |
| 7.58 | `crypto_sign_ed25519_pk_to_curve25519` + `crypto_sign_ed25519_sk_to_curve25519` + `crypto_box_beforenm` | full cross-protocol bridge: two ed25519 keypairs → converted curve25519 keys → `crypto_box_easy`/`_open_easy` round trip | [x] |
| 7.59 | `crypto_sign_bytes`, `_seedbytes`, `_publickeybytes`, `_secretkeybytes`, `_messagebytes_max`, `_primitive`, `_statebytes` and the seven `crypto_sign_ed25519_*` twins | → `64`, `32`, `32`, `64`, `2^64-65`, `"ed25519"`, `208` | [x] |
| 7.60 | `crypto_box_keypair`, `crypto_box_curve25519xsalsa20poly1305_keypair` | randomised; verify `pk == crypto_scalarmult_curve25519_base(sk)` | [x] |
| 7.61 | `crypto_box_seed_keypair`, `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | deterministic: `sk = SHA-512(seed)[0..31]` (**unclamped** in `sk`, clamped only inside `_base`), `pk = base(sk)`; seed = 32 zero bytes, 32 `0xff` bytes, and a fixed vector | [x] |
| 7.62 | `crypto_box_easy` / `crypto_box_open_easy` | round trip, `mlen` ∈ {0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 1024}; `c` buffer `mlen + 16`; MAC is at `c[0..15]`, body at `c[16…]` | [x] |
| 7.63 | `crypto_box_easy` / `_open_easy` | in-place: `c == m` is **not** possible for `_easy` (output is prefixed) but `m == c` for `_open_easy` with the shift, plus `c + 16 == m` for `_easy` (documented in-place pattern) | [x] |
| 7.64 | `crypto_box_detached` / `crypto_box_open_detached` | round trip with a separate 16-byte `mac` buffer; `mlen` ∈ {0, 1, 16, 63, 64, 65, 1024}; check `c ‖ mac` layout equals the `_easy` output reordered (`mac ‖ c`) | [x] |
| 7.65 | `crypto_box_beforenm` + `crypto_box_easy_afternm` / `crypto_box_open_easy_afternm` | precomputed key path; `k` is 32 bytes; must produce identical ciphertext to `crypto_box_easy` with the same `pk`/`sk`/`n`/`m` | [x] |
| 7.66 | `crypto_box_beforenm` + `crypto_box_detached_afternm` / `crypto_box_open_detached_afternm` | same, detached form; verify `beforenm(k, pkB, skA) == beforenm(k, pkA, skB)` (symmetry via HSalsa20 of the DH secret) | [x] |
| 7.67 | `crypto_box_beforenm` + `crypto_box_afternm` / `crypto_box_open_afternm` (NaCl padded) | `m` with `crypto_box_ZEROBYTES` = 32 leading zero bytes, `c` with `crypto_box_BOXZEROBYTES` = 16 leading zero bytes; `mlen` ∈ {32, 33, 48, 64, 1056} | [x] |
| 7.68 | `crypto_box` / `crypto_box_open`, `crypto_box_curve25519xsalsa20poly1305` / `_open` (NaCl padded) | full round trip in the padded convention; verify `c[0..15] == 0` on output and `m[0..31] == 0` after open | [x] |
| 7.69 | `crypto_box_seal` / `crypto_box_seal_open` | round trip, `mlen` ∈ {0, 1, 16, 32, 63, 64, 65, 1024}; `c` buffer `mlen + 48`; `c[0..31]` is a fresh ephemeral pk each call (two seals of the same message must differ) | [x] |
| 7.70 | `crypto_box_seal_open` | `clen == 48` exactly (empty sealed message) → `0`, nothing written to `m` | [x] |
| 7.71 | `crypto_box_seal_open` | anonymous-sender property: the recipient can open without knowing the sender; and `crypto_box_seal_open` with `pk` derived from `sk` via `crypto_scalarmult_base` (the two must be consistent or it fails) | [x] |
| 7.72 | `crypto_box_curve25519xchacha20poly1305_keypair` / `_seed_keypair` | identical key derivation to the xsalsa variant (both are `SHA-512(seed)[0..31]` then `_base`) — outputs must be byte-identical across the two primitives for the same seed | [x] |
| 7.73 | `crypto_box_curve25519xchacha20poly1305_beforenm` | must **differ** from `crypto_box_curve25519xsalsa20poly1305_beforenm` for the same `pk`/`sk` (HChaCha20 vs HSalsa20 of the same DH secret) | [x] |
| 7.74 | `crypto_box_curve25519xchacha20poly1305_easy` / `_open_easy` | round trip, `mlen` ∈ {0, 1, 15, 16, 17, 63, 64, 65, 127, 128, 129, 1024}; ciphertext must differ from the xsalsa variant | [x] |
| 7.75 | `crypto_box_curve25519xchacha20poly1305_detached` / `_open_detached` | round trip with a separate `mac`; `mlen` ∈ {0, 1, 64, 1024} | [x] |
| 7.76 | `crypto_box_curve25519xchacha20poly1305_beforenm` + `_easy_afternm` / `_open_easy_afternm` | precomputed path must match the non-`afternm` form | [x] |
| 7.77 | `crypto_box_curve25519xchacha20poly1305_beforenm` + `_detached_afternm` / `_open_detached_afternm` | precomputed detached path | [x] |
| 7.78 | `crypto_box_curve25519xchacha20poly1305_seal` / `_seal_open` | round trip, `mlen` ∈ {0, 1, 64, 1024}; `SEALBYTES` = 48; nonce = `BLAKE2b-24(epk ‖ pk)` — must differ from the xsalsa seal for the same inputs | [x] |
| 7.79 | xchacha **absent** APIs | confirm there is no `crypto_box_curve25519xchacha20poly1305()` / `_open()` / `_afternm()` / `_open_afternm()` / `_zerobytes()` / `_boxzerobytes()` — the NaCl padded convention exists only for xsalsa. A port must not invent them | [x] |
| 7.80 | `crypto_box_seedbytes`, `_publickeybytes`, `_secretkeybytes`, `_beforenmbytes`, `_noncebytes`, `_zerobytes`, `_boxzerobytes`, `_macbytes`, `_messagebytes_max`, `_sealbytes`, `_primitive` and the xsalsa/xchacha twins | → `32,32,32,32,24,32,16,16,2^64-17,48,"curve25519xsalsa20poly1305"`; xchacha has no `_zerobytes`/`_boxzerobytes` | [x] |
| 7.81 | `crypto_kx_keypair` | randomised; verify `pk == crypto_scalarmult_base(sk)` | [x] |
| 7.82 | `crypto_kx_seed_keypair` | deterministic: `sk = BLAKE2b-32(seed)` (no key, no salt), `pk = crypto_scalarmult_base(sk)`; seed = 32 zero bytes, 32 `0xff` bytes, fixed vector | [x] |
| 7.83 | `crypto_kx_client_session_keys` + `crypto_kx_server_session_keys` | full handshake with both `rx` and `tx` non-NULL: `client_rx == server_tx` and `client_tx == server_rx`; the shared hash is `BLAKE2b-64(q ‖ client_pk ‖ server_pk)` split as `keys[0..31]` / `keys[32..63]` | [x] |
| 7.84 | `crypto_kx_client_session_keys` | `rx == NULL`, `tx` non-NULL → `0`; the surviving buffer holds `keys[32..63]` (the tx key) because of the byte-interleaved aliased writes | [x] |
| 7.85 | `crypto_kx_client_session_keys` | `tx == NULL`, `rx` non-NULL → `0`; the surviving buffer **also** holds `keys[32..63]`, i.e. the *tx* key, not the rx key (aliasing footgun to replicate exactly) | [x] |
| 7.86 | `crypto_kx_server_session_keys` | `rx == NULL` and separately `tx == NULL` → `0`; the surviving buffer holds `keys[32..63]` = the server's **rx** key (loop order is reversed relative to the client) | [x] |
| 7.87 | `crypto_kx_client_session_keys` / `_server_session_keys` | `rx == tx` (caller deliberately aliases two non-NULL equal pointers) — same interleaving as 7.84–7.86 | [x] |
| 7.88 | `crypto_kx_client_session_keys` | client and server keys swapped (client calls the server function and vice versa) → session keys must **not** match, documenting the role asymmetry | [x] |
| 7.89 | `crypto_kx_publickeybytes`, `_secretkeybytes`, `_seedbytes`, `_sessionkeybytes`, `_primitive` | → `32, 32, 32, 32, "x25519blake2b"` | [x] |
| 7.90 | `crypto_kdf_derive_from_key`, `crypto_kdf_blake2b_derive_from_key` | `subkey_len = 16` (`BYTES_MIN`), `subkey_id = 0`, `ctx = "context1"` (8 bytes), fixed 32-byte key → deterministic vector; the generic alias must be byte-identical | [x] |
| 7.91 | `crypto_kdf_blake2b_derive_from_key` | `subkey_len` = 16, 17, 31, 32, 33, 63, 64 (`BYTES_MAX`) with everything else fixed → 7 distinct subkeys; a shorter subkey must **not** be a prefix of a longer one (BLAKE2b `outlen` is in the parameter block) | [x] |
| 7.92 | `crypto_kdf_blake2b_derive_from_key` | `subkey_id` = 0, 1, 2, `0xffffffff`, `0x100000000`, `0x8000000000000000`, `0xffffffffffffffff` — stored `STORE64_LE` into `salt[0..7]` with `salt[8..15] = 0` | [x] |
| 7.93 | `crypto_kdf_blake2b_derive_from_key` | `ctx` = 8 zero bytes; `ctx` = `"12345678"`; `ctx` = 8 `0xff` bytes — zero-padded into the 16-byte BLAKE2b *personal* field | [x] |
| 7.94 | `crypto_kdf_blake2b_derive_from_key` | `key` = 32 zero bytes and 32 `0xff` bytes (`keylen` is always `crypto_kdf_blake2b_KEYBYTES` = 32) | [x] |
| 7.95 | `crypto_kdf_keygen` | 32 bytes of `randombytes_buf`; verify successive calls differ | [x] |
| 7.96 | `crypto_kdf_bytes_min`, `_bytes_max`, `_contextbytes`, `_keybytes`, `_primitive` and the four `crypto_kdf_blake2b_*` twins | → `16, 64, 8, 32, "blake2b"` | [x] |
| 7.97 | `crypto_kdf_hkdf_sha256_extract` | RFC 5869 test case 1: `salt = 000102030405060708090a0b0c` (13 B), `ikm = 0b×22` → `prk = 077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5` | [x] |
| 7.98 | `crypto_kdf_hkdf_sha256_expand` | RFC 5869 test 1 continued: `ctx = f0f1f2f3f4f5f6f7f8f9`, `out_len = 42` → `3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865` | [x] |
| 7.99 | `crypto_kdf_hkdf_sha256_extract` | `salt_len` = 0 (RFC 5869 test 3, empty salt), 1, 32, 64 (= HMAC block size), 65, 128, 129 — the `> blocksize` case forces the HMAC key-hashing path | [x] |
| 7.100 | `crypto_kdf_hkdf_sha256_extract` | `ikm_len` = 0, 1, 22, 32, 64, 80 (RFC 5869 test 3 has `ikm_len = 22`, test 2 has 80) | [x] |
| 7.101 | `crypto_kdf_hkdf_sha256_extract_init` / `_extract_update` / `_extract_final` | **0 updates** (empty ikm) → must equal `crypto_kdf_hkdf_sha256_extract(prk, salt, salt_len, NULL/ptr, 0)` | [x] |
| 7.102 | hkdf-sha256 streaming extract | **1 update**; then **many updates** splitting the same ikm as 1+1+…, 31+1+32, 32×N, 63+1 — all must produce the identical `prk` | [x] |
| 7.103 | `crypto_kdf_hkdf_sha256_extract_final` | state is `sodium_memzero`'d on return — a second `_extract_final` on the same state is a misuse; document that the state is single-shot | [x] |
| 7.104 | `crypto_kdf_hkdf_sha256_expand` | `out_len` = 0 (`BYTES_MIN`, legal, no writes), 1, 31, 32, 33, 63, 64, 65, 96, 8160 (`BYTES_MAX`) — exercises both the full-block loop and the `left = out_len & 31` tail | [x] |
| 7.105 | `crypto_kdf_hkdf_sha256_expand` | `ctx_len` = 0, 1, 8, 10, 64; and `ctx` = NULL with `ctx_len = 0` | [x] |
| 7.106 | `crypto_kdf_hkdf_sha256_expand` | counter progression: `out_len = 8160` exhausts the counter to `0xff`; check the last 32-byte block matches an independent HMAC computation with `counter = 255` | [x] |
| 7.107 | `crypto_kdf_hkdf_sha256_keygen` | 32 bytes random; then `_expand` with that prk | [x] |
| 7.108 | `crypto_kdf_hkdf_sha512_extract` / `_expand` | mirror of 7.97–7.106 with `KEYBYTES = 64`, `BYTES_MAX = 16320`, block size 128, `left = out_len & 63`; `out_len` ∈ {0, 1, 63, 64, 65, 127, 128, 129, 16320} | [x] |
| 7.109 | `crypto_kdf_hkdf_sha512_extract_init/_update/_final` | 0 / 1 / many updates; `salt_len` ∈ {0, 1, 64, 128, 129} (128 = HMAC-SHA512 block size) | [x] |
| 7.110 | `crypto_kdf_hkdf_sha512_keygen` | 64 bytes random | [x] |
| 7.111 | `crypto_kdf_hkdf_sha256_*` vs `crypto_kdf_hkdf_sha512_*` | same salt/ikm/ctx through both → outputs must differ; documents that the two are separate namespaces with different `KEYBYTES` | [x] |
| 7.112 | `crypto_kdf_hkdf_sha256_keybytes`, `_bytes_min`, `_bytes_max`, `_statebytes` and the sha512 twins | → `32, 0, 8160, sizeof(state)` and `64, 0, 16320, sizeof(state)` | [x] |
| 7.113 | `crypto_kem_mlkem768_seed_keypair` | fixed 64-byte seed (`d = seed[0..31]`, `z = seed[32..63]`) → deterministic `pk` (1184 B) and `sk` (2400 B); verify `sk[1152..2335] == pk`, `sk[2336..2367] == SHA3-256(pk)`, `sk[2368..2399] == seed[32..63]`. Use the FIPS 203 / ML-KEM-768 KAT vectors | [x] |
| 7.114 | `crypto_kem_mlkem768_keypair` | randomised; verify the same structural invariants as 7.113 | [x] |
| 7.115 | `crypto_kem_mlkem768_enc_deterministic` | fixed `pk` from 7.113 plus a fixed 32-byte `seed` (= the message `m`) → deterministic `ct` (1088 B) and `ss` (32 B); ML-KEM-768 KAT | [x] |
| 7.116 | `crypto_kem_mlkem768_enc` + `_dec` | randomised round trip × 100: `ss_enc == ss_dec`, `0` from both | [x] |
| 7.117 | `crypto_kem_mlkem768_enc_deterministic` + `_dec` | deterministic round trip; then `_dec` with a single bit flipped anywhere in `ct` → still `0` but `ss` differs (implicit rejection, `SHAKE256(z ‖ ct)`) — the derived value must be reproducible | [x] |
| 7.118 | `crypto_kem_mlkem768_enc*` | `pk` = 1184 zero bytes (canonical! all coefficients 0) → **succeeds**; `pk` with `publicseed` (`pk[1152..1183]`) varied → different `ct` | [x] |
| 7.119 | `crypto_kem_mlkem768_*bytes` accessors | → `1184, 2400, 1088, 32, 64` | [x] |
| 7.120 | `crypto_kem_xwing_seed_keypair` | fixed 32-byte seed → deterministic `pk` (1216 B = 1184 ML-KEM ‖ 32 X25519) and `sk` (exactly the 32-byte seed); verify `expand_decaps_key` layout: `SHAKE256(seed, 96)` → `[0..63]` = ML-KEM seed, `[64..95]` = X25519 scalar; `pk[1184..1215] == crypto_scalarmult_curve25519_base(sk_x25519)`. X-Wing draft test vectors | [x] |
| 7.121 | `crypto_kem_xwing_keypair` | randomised; `sk` is 32 bytes; re-deriving via `_seed_keypair(sk)` must reproduce `pk` exactly | [x] |
| 7.122 | `crypto_kem_xwing_enc_deterministic` | fixed `pk` + fixed **64-byte** seed (`seed[0..31]` = ML-KEM message, `seed[32..63]` = ephemeral X25519 scalar) → deterministic `ct` (1120 B = 1088 ML-KEM ‖ 32 X25519) and `ss` (32 B). `ss = SHA3-256(ss_mlkem ‖ ss_x25519 ‖ ct_x25519 ‖ pk_x25519 ‖ 5c2e2f2f5e5c)` | [x] |
| 7.123 | `crypto_kem_xwing_enc` + `_dec` | randomised round trip × 100: `ss_enc == ss_dec`, `0` from both | [x] |
| 7.124 | `crypto_kem_xwing_enc_deterministic` + `_dec` | deterministic round trip against the X-Wing vectors; then `_dec` with a bit flipped in `ct[0..1087]` (ML-KEM half) → `0` with a different `ss` | [x] |
| 7.125 | `crypto_kem_xwing_dec` | `sk` = 32 zero bytes (legal seed) and 32 `0xff` bytes; full round trip for each | [x] |
| 7.126 | `crypto_kem_xwing_*bytes` accessors | → `1216, 32, 1120, 32, 32` (note `SECRETKEYBYTES == SEEDBYTES == 32`) | [x] |
| 7.127 | `crypto_kem_seed_keypair`, `crypto_kem_keypair`, `crypto_kem_enc`, `crypto_kem_dec` (generic dispatch) | must be byte-identical to the `crypto_kem_xwing_*` equivalents for the same inputs; `crypto_kem_primitive() == "xwing"`; `crypto_kem_*bytes()` must equal the xwing values | [x] |
| 7.128 | xwing vs mlkem768 | same-length comparison: `crypto_kem_xwing_CIPHERTEXTBYTES` (1120) = mlkem `CIPHERTEXTBYTES` (1088) + 32; `PUBLICKEYBYTES` 1216 = 1184 + 32 — verify the concatenation offsets used by `crypto_kem_xwing_enc`/`_dec` (`ct + 1088`, `pk + 1184`) | [x] |
| 7.129 | cross-area consistency | `crypto_kx` shared secret vs `crypto_box_beforenm`: both start from `crypto_scalarmult`/`crypto_scalarmult_curve25519` on the same keypairs but apply BLAKE2b vs HSalsa20 — verify they differ and that each is stable | [x] |
| 7.130 | build-configuration invariant | no `HAVE_*` macro is defined by the CMake build, therefore: sandy2x is never selected (`scalarmult_curve25519.c:54-58` removed), `fe25519_sub_lazy` is `fe25519_sub` (`x25519_ref10.c:88-91`), `ED25519_COMPAT` is off so `open.c:34-42` (strict `sc25519_is_canonical` + `ge25519_is_canonical`) is live and `open.c:31-33` is dead, `ED25519_NONDETERMINISTIC` is off so signing is fully deterministic (`sign.c:66`, not `:64`), and `cmov`'s `HAVE_INLINE_ASM` barrier (`kem_mlkem768_ref.c:696-698`) is absent. Every row above must be evaluated under exactly this configuration | [x] |

## Area 8 — crypto_pwhash + crypto_ipcrypt

Configuration surface of **valid** (non-rejecting) inputs for
`crypto_pwhash/crypto_pwhash.c`, `crypto_pwhash/argon2/*`,
`crypto_pwhash/scryptsalsa208sha256/*` (incl. `nosse/`), and
`crypto_ipcrypt/{crypto_ipcrypt.c, ipcrypt_soft.c}`.

Axes extracted from the source:

* **alg**: `crypto_pwhash_ALG_ARGON2I13` (1) vs `crypto_pwhash_ALG_ARGON2ID13` (2);
  `crypto_pwhash_ALG_DEFAULT` is an alias of `ARGON2ID13`. Internally `argon2_type` ∈
  {`Argon2_i`=1, `Argon2_id`=2} (Argon2_d is not compiled in).
* **entry-point layer**: high-level `crypto_pwhash*` / `crypto_pwhash_argon2i*` /
  `crypto_pwhash_argon2id*` (which hard-wire `lanes = threads = 1`,
  `saltlen = 16`, `STR_HASHBYTES = 32`) **and** the low-level `argon2_ctx`, `argon2_hash`,
  `argon2i_hash_raw`, `argon2id_hash_raw`, `argon2i_hash_encoded`, `argon2id_hash_encoded`,
  `argon2_verify`, `argon2i_verify`, `argon2id_verify`, `argon2_encode_string`,
  `argon2_decode_string` (which expose lanes/threads/secret/ad/flags).
* **lanes / threads**: `lanes ∈ {1, 2, 4, …}`, `threads` independent of `lanes`
  (`argon2_fill_memory_blocks` is single-threaded regardless — `threads` only affects validation,
  never the output).
* **m_cost**: at `8 * lanes` (the effective minimum), just above it, and moderate values;
  `segment_length = m_cost / (lanes * 4)` and `m_cost` is then re-rounded down to
  `segment_length * lanes * 4`, so several distinct `m_cost` values collapse to identical work.
* **t_cost**: 1 (single pass, `fill_block`), 2 and 3 (extra passes take the
  `fill_block_with_xor` + `pass != 0` branch of `index_alpha`).
* **outlen**: `ARGON2_MIN_OUTLEN` = 16, typical 24/32, `64` (= `blake2b_BYTES_MAX`, last value on
  the short `blake2b_long` path), `65`/`128`/`1024` (long path with the 32-byte-per-iteration loop).
* **pwd / salt / secret / ad**: present vs absent (`NULL` + len 0) and at their minimum lengths
  (`pwd` min 0, `salt` min 8, `secret` min 0, `ad` min 0).
* **flags**: `ARGON2_DEFAULT_FLAGS`, `ARGON2_FLAG_CLEAR_PASSWORD`, `ARGON2_FLAG_CLEAR_SECRET`, both.
* **scrypt**: `N ∈ {2, 16, 512, 1024, 16384}`, `r ∈ {1, 8}`, `p ∈ {1, 2, 512}`; the two branches of
  `pickparams` (`opslimit < memlimit/32` vs not); the `$7$` setting round trip.
* **ipcrypt**: deterministic / ND / NDX / PFX, encrypt and decrypt, IPv4-mapped vs pure IPv6
  16-byte inputs, distinct vs identical key halves.

**Speed note:** every argon2 row below uses `m_cost` ≤ 64 KiB (`memlimit` ≤ 65536) and
`t_cost` ≤ 3 unless the row explicitly says otherwise, and every scrypt row uses `N ≤ 16384, r ≤ 8,
p ≤ 2` unless stated, so the whole table runs in well under a second per row. Rows marked
**(SLOW — optional)** use the documented INTERACTIVE/MODERATE/SENSITIVE presets and should be run at
most once each, or skipped in fast CI.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 8.1 | `crypto_pwhash_alg_argon2i13`, `crypto_pwhash_alg_argon2id13`, `crypto_pwhash_alg_default` | no input; must return 1, 2, 2 respectively (`ALG_DEFAULT == ALG_ARGON2ID13`) | [x] |
| 8.2 | `crypto_pwhash_bytes_min`/`_max`, `_passwd_min`/`_max`, `_saltbytes`, `_strbytes`, `_strprefix`, `_primitive` | no input; 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2id$"` / `"argon2id,argon2i"` | [x] |
| 8.3 | `crypto_pwhash_opslimit_min`/`_max`/`_interactive`/`_moderate`/`_sensitive`, `crypto_pwhash_memlimit_min`/`_max`/`_interactive`/`_moderate`/`_sensitive` | no input; 1 / 4294967295 / 2 / 3 / 4 and 8192 / 4398046510080 / 67108864 / 268435456 / 1073741824 (all alias the argon2id values) | [x] |
| 8.4 | `crypto_pwhash_argon2i_*` constant getters (alg, bytes_min/max, passwd_min/max, saltbytes, strbytes, strprefix, opslimit_min/max/interactive/moderate/sensitive, memlimit_min/max/interactive/moderate/sensitive) | no input; 1 / 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2i$"` / 3,4294967295,4,6,8 / 8192,4398046510080,33554432,134217728,536870912 | [x] |
| 8.5 | `crypto_pwhash_argon2id_*` constant getters (same list) | no input; 2 / 16 / 4294967295 / 0 / 4294967295 / 16 / 128 / `"$argon2id$"` / 1,4294967295,2,3,4 / 8192,4398046510080,67108864,268435456,1073741824 | [x] |
| 8.6 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3` (`argon2i_OPSLIMIT_MIN`), `memlimit = 8192` (`MEMLIMIT_MIN` → `m_cost = 8`), `outlen = 16` (`BYTES_MIN`), `passwd = "test"` (len 4), 16-byte salt; fast | [x] |
| 8.7 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 8192`, `outlen = 32` | [x] |
| 8.8 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 4`, `memlimit = 16384` (`m_cost = 16`), `outlen = 64` (last short `blake2b_long` size) | [x] |
| 8.9 | `crypto_pwhash` | `alg = ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 65536` (`m_cost = 64`), `outlen = 65` (first long `blake2b_long` size) | [x] |
| 8.10 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 1` (`argon2id_OPSLIMIT_MIN`), `memlimit = 8192`, `outlen = 16` | [x] |
| 8.11 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 2`, `memlimit = 8192`, `outlen = 32` (two passes: pass 1 is fully data-dependent) | [x] |
| 8.12 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `opslimit = 3`, `memlimit = 32768` (`m_cost = 32`), `outlen = 32` | [x] |
| 8.13 | `crypto_pwhash` | `alg = ALG_DEFAULT`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16`; output must be byte-identical to row 8.10 (`ALG_DEFAULT == ARGON2ID13`) | [x] |
| 8.14 | `crypto_pwhash` | `alg = ALG_ARGON2ID13`, `outlen = 128` and `outlen = 1024` (multi-iteration long `blake2b_long` path), `opslimit = 1`, `memlimit = 8192` | [x] |
| 8.15 | `crypto_pwhash` | `passwdlen = 0` with a non-NULL `passwd` pointer (`PASSWD_MIN` is 0), `alg = ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16` | [x] |
| 8.16 | `crypto_pwhash` | `passwdlen = 1`; then a long password (e.g. 256 bytes, and one > 128 bytes containing NUL and 0xFF bytes — password is binary, not a C string) | [x] |
| 8.17 | `crypto_pwhash` | `memlimit` not a multiple of 1024: `memlimit = 8192 + 512` and `9215` both truncate to `m_cost = 8` → output identical to row 8.10 | [x] |
| 8.18 | `crypto_pwhash` | salt values: all-zero 16 bytes, all-0xFF 16 bytes, random; salt length is fixed at `crypto_pwhash_SALTBYTES` = 16 by the wrapper | [x] |
| 8.19 | `crypto_pwhash` **(SLOW — optional)** | `alg = ALG_ARGON2ID13`, `opslimit = crypto_pwhash_OPSLIMIT_INTERACTIVE` (2), `memlimit = crypto_pwhash_MEMLIMIT_INTERACTIVE` (67108864 → `m_cost = 65536`), `outlen = 32` — the documented interactive preset; ~64 MiB allocation | [x] |
| 8.20 | `crypto_pwhash` **(SLOW — optional)** | `alg = ALG_ARGON2I13`, `opslimit = crypto_pwhash_argon2i_OPSLIMIT_INTERACTIVE` (4), `memlimit = 33554432` (`m_cost = 32768`), `outlen = 32` | [x] |
| 8.21 | `crypto_pwhash_argon2i` (direct) | `alg = crypto_pwhash_argon2i_ALG_ARGON2I13`, `opslimit = 3`, `memlimit = 8192`, `outlen = 16`; must equal `crypto_pwhash` with `ALG_ARGON2I13` (row 8.6 shape) | [x] |
| 8.22 | `crypto_pwhash_argon2id` (direct) | `alg = crypto_pwhash_argon2id_ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192`, `outlen = 16`; must equal row 8.10 | [x] |
| 8.23 | `crypto_pwhash_str` | `passwd = "password"`, `opslimit = crypto_pwhash_OPSLIMIT_MIN` (1), `memlimit = crypto_pwhash_MEMLIMIT_MIN` (8192) — deliberately minimal so the test is fast. Output: 128-byte buffer, `"$argon2id$v=19$m=8,t=1,p=1$"` + 22 base64 chars + `"$"` + 43 base64 chars + NUL, all remaining bytes 0 | [x] |
| 8.24 | `crypto_pwhash_str` + `crypto_pwhash_str_verify` | round trip at (`opslimit = 1`, `memlimit = 8192`) with the same password → 0; each call produces a *different* string (random 16-byte salt) yet both verify | [x] |
| 8.25 | `crypto_pwhash_str` + `crypto_pwhash_str_verify` | `passwdlen = 0` round trip (empty password is legal) | [x] |
| 8.26 | `crypto_pwhash_str_alg` | `alg = ALG_ARGON2I13`, `opslimit = 3` (`argon2i` min), `memlimit = 8192` → string starts with `"$argon2i$v=19$m=8,t=3,p=1$"`; verify with `crypto_pwhash_str_verify` (prefix dispatch) → 0 | [x] |
| 8.27 | `crypto_pwhash_str_alg` | `alg = ALG_ARGON2ID13`, `opslimit = 1`, `memlimit = 8192` → string starts with `"$argon2id$v=19$m=8,t=1,p=1$"`; identical to `crypto_pwhash_str` behaviour | [x] |
| 8.28 | `crypto_pwhash_str_alg` **(SLOW — optional)** | `alg = ALG_ARGON2ID13`, `opslimit = OPSLIMIT_INTERACTIVE` (2), `memlimit = MEMLIMIT_INTERACTIVE` (67108864) → `"...m=65536,t=2,p=1..."` | [x] |
| 8.29 | `crypto_pwhash_argon2i_str` / `crypto_pwhash_argon2i_str_verify` | direct argon2i round trip at (3, 8192); the produced string must also be accepted by the generic `crypto_pwhash_str_verify` | [x] |
| 8.30 | `crypto_pwhash_argon2id_str` / `crypto_pwhash_argon2id_str_verify` | direct argon2id round trip at (1, 8192) | [x] |
| 8.31 | `crypto_pwhash_str_needs_rehash` | argon2id string produced at (1, 8192), queried with the same `(opslimit = 1, memlimit = 8192)` → `0` | [x] |
| 8.32 | `crypto_pwhash_str_needs_rehash` | same string, queried with a different `opslimit` (2) → `1`; and with a different `memlimit` (16384) → `1` | [x] |
| 8.33 | `crypto_pwhash_str_needs_rehash` | same string, `memlimit = 8192 + 1023` (truncating division by 1024 → `m_cost = 8`) → `0`; documents the truncation semantics | [x] |
| 8.34 | `crypto_pwhash_str_needs_rehash` | argon2**i** string (prefix dispatch to `crypto_pwhash_argon2i_str_needs_rehash`) produced at (3, 8192), queried with (3, 8192) → `0`, with (4, 8192) → `1` | [x] |
| 8.35 | `crypto_pwhash_str_needs_rehash` | hand-written `"$argon2id$v=19$m=8,t=1,p=2$<22b64>$<43b64>"` queried with (1, 8192) → `0` even though `p` differs: **`lanes`/`p` and the type are not compared** — quirk to preserve | [x] |
| 8.36 | `crypto_pwhash_argon2i_str_needs_rehash`, `crypto_pwhash_argon2id_str_needs_rehash` | boundary: `strlen(str) == 127` (max accepted, `< crypto_pwhash_STRBYTES`) with an otherwise valid string | [x] |
| 8.37 | `argon2_ctx` | `type = Argon2_i`, minimal legal context: `out`/`outlen = 16`, `pwd = NULL, pwdlen = 0`, `salt` 8 bytes / `saltlen = 8` (`ARGON2_MIN_SALT_LENGTH`), `secret = NULL, secretlen = 0`, `ad = NULL, adlen = 0`, `t_cost = 1`, `m_cost = 8` (= `8*lanes`), `lanes = 1`, `threads = 1`, `flags = ARGON2_DEFAULT_FLAGS` → `ARGON2_OK` | [x] |
| 8.38 | `argon2_ctx` | same as 8.37 but `type = Argon2_id` → different digest, `ARGON2_OK` | [x] |
| 8.39 | `argon2_ctx` | `lanes = 2`, `threads = 2`, `m_cost = 16` (= `8*lanes`, the minimum for 2 lanes), `t_cost = 1`, `outlen = 32`, both types | [x] |
| 8.40 | `argon2_ctx` | `lanes = 4`, `threads = 4`, `m_cost = 32` (= `8*lanes`), `t_cost = 1`, `outlen = 32`, both types (exercises the multi-lane XOR in `argon2_finalize`) | [x] |
| 8.41 | `argon2_ctx` | `lanes = 2`, `threads = 1` (threads < lanes is legal); output must be identical to `lanes = 2, threads = 2` — `threads` never affects the digest | [x] |
| 8.42 | `argon2_ctx` | `lanes = 1`, `threads = 4` (threads > lanes is legal, `threads <= ARGON2_MAX_THREADS`); output identical to `threads = 1` | [x] |
| 8.43 | `argon2_ctx` | `lanes = ARGON2_MAX_LANES` boundary check only via validation-adjacent config: `lanes = 8`, `threads = 8`, `m_cost = 64` | [x] |
| 8.44 | `argon2_ctx` | m_cost just above the minimum with `lanes = 1`: `m_cost = 9, 10, 11` all round down to `segment_length = 2` → identical digests to `m_cost = 8`; `m_cost = 12` gives `segment_length = 3` (different digest) | [x] |
| 8.45 | `argon2_ctx` | m_cost rounding with `lanes = 2`: `m_cost = 16..23` → `segment_length = 2` (identical digests); `m_cost = 24` → `segment_length = 3` | [x] |
| 8.46 | `argon2_ctx` | moderate m_cost: `m_cost = 512` (`lanes = 1` → `segment_length = 128 == ARGON2_ADDRESSES_IN_BLOCK`) and `m_cost = 1024` (`segment_length = 256`, forces a second address block in `generate_addresses`) | [x] |
| 8.47 | `argon2_ctx` | `t_cost = 1` (single pass; `fill_block` only) | [x] |
| 8.48 | `argon2_ctx` | `t_cost = 2` (second pass takes `fill_block_with_xor` and the `pass != 0` `index_alpha` branch) | [x] |
| 8.49 | `argon2_ctx` | `t_cost = 3` | [x] |
| 8.50 | `argon2_ctx` | `type = Argon2_id, t_cost = 1`: slices 0–1 data-independent (`generate_addresses`), slices 2–3 data-dependent; `type = Argon2_id, t_cost = 2`: pass 1 fully data-dependent | [x] |
| 8.51 | `argon2_ctx` | `type = Argon2_i, t_cost = 2`: **all** passes/slices data-independent | [x] |
| 8.52 | `argon2_ctx` | `outlen = 16` (MIN), `24`, `32`, `48`, `64` (short `blake2b_long` path); `outlen = 65`, `96`, `128`, `1024` (long path) — all with `t_cost = 1, m_cost = 8, lanes = 1` | [x] |
| 8.53 | `argon2_ctx` | `saltlen = 8` (min), `16` (libsodium's `SALTBYTES`), `32`, `64`; salt all-zero and random | [x] |
| 8.54 | `argon2_ctx` | `pwd` absent (`NULL`, `pwdlen = 0`) vs present with `pwdlen = 0` (non-NULL pointer) vs `pwdlen = 1` vs `pwdlen = 64` — the first two must give the same digest (only `pwdlen` is hashed) | [x] |
| 8.55 | `argon2_ctx` | `secret` absent (`NULL, 0`) vs present with `secretlen = 0` (non-NULL, min is 0) vs `secretlen = 8`, `16`, `32` — keyed argon2; digest differs from the unkeyed case for `secretlen > 0` | [x] |
| 8.56 | `argon2_ctx` | `ad` absent (`NULL, 0`) vs present with `adlen = 0` vs `adlen = 8`, `16`, `64` | [x] |
| 8.57 | `argon2_ctx` | both `secret` (16 bytes) and `ad` (16 bytes) present, `type = Argon2_id`, `t_cost = 2`, `m_cost = 16`, `lanes = 2` | [x] |
| 8.58 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_PASSWORD` with `pwd` non-NULL, `pwdlen = 16`: after the call `pwd` is all-zero and `context->pwdlen == 0`; digest identical to the `ARGON2_DEFAULT_FLAGS` run | [x] |
| 8.59 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_SECRET` with `secret` non-NULL, `secretlen = 16`: after the call `secret` is zeroed and `context->secretlen == 0` | [x] |
| 8.60 | `argon2_ctx` | `flags = ARGON2_FLAG_CLEAR_PASSWORD \| ARGON2_FLAG_CLEAR_SECRET` (both cleared) | [x] |
| 8.61 | `argon2_hash` | `hash != NULL, encoded == NULL, encodedlen = 0` (raw-only mode), `t_cost = 1, m_cost = 8, parallelism = 1, hashlen = 16, saltlen = 8`; identical to `argon2i_hash_raw` | [x] |
| 8.62 | `argon2_hash` | `hash == NULL, encoded != NULL, encodedlen = 128` (encoded-only mode); identical to `argon2i_hash_encoded` | [x] |
| 8.63 | `argon2_hash` | **both** `hash != NULL` and `encoded != NULL, encodedlen = 128` — raw digest and encoded string written in one call and must agree | [x] |
| 8.64 | `argon2_hash` | `encoded != NULL` but `encodedlen = 0` → the `if (encoded && encodedlen)` guard skips encoding; returns `ARGON2_OK` with `encoded` untouched | [x] |
| 8.65 | `argon2_hash` | `hash == NULL` and `encoded == NULL` (both outputs suppressed) → still returns `ARGON2_OK` after doing the full KDF | [x] |
| 8.66 | `argon2_hash` | `parallelism = 2` with `m_cost = 16`, and `parallelism = 4` with `m_cost = 32` — note `argon2_hash` sets `lanes = threads = parallelism` | [x] |
| 8.67 | `argon2i_hash_raw` | `t_cost = 1, m_cost = 8, parallelism = 1, pwdlen = 0..32, saltlen = 8/16, hashlen = 16/32/64` | [x] |
| 8.68 | `argon2i_hash_raw` | `t_cost = 2, m_cost = 32, parallelism = 4` (`m_cost == 8*parallelism` boundary), `hashlen = 32` | [x] |
| 8.69 | `argon2id_hash_raw` | same matrix as 8.67 and 8.68 with `Argon2_id`; digests must differ from the argon2i ones for identical parameters | [x] |
| 8.70 | `argon2i_hash_encoded` | `t_cost = 1, m_cost = 8, parallelism = 1, saltlen = 8, hashlen = 16, encodedlen = 128` → `"$argon2i$v=19$m=8,t=1,p=1$<11 b64>$<22 b64>"`; also `encodedlen` exactly equal to `strlen(result)+1` | [x] |
| 8.71 | `argon2id_hash_encoded` | `t_cost = 1, m_cost = 8, parallelism = 1, saltlen = 16, hashlen = 32, encodedlen = 128` → `"$argon2id$v=19$m=8,t=1,p=1$<22 b64>$<43 b64>"` | [x] |
| 8.72 | `argon2id_hash_encoded` | `parallelism = 2, m_cost = 16, t_cost = 2, saltlen = 16, hashlen = 32` → `"...m=16,t=2,p=2..."` | [x] |
| 8.73 | `argon2i_verify` / `argon2id_verify` | round trip against the strings from 8.70–8.72 with the correct password → `ARGON2_OK`; `pwdlen = 0` case included | [x] |
| 8.74 | `argon2_verify` (generic) | `type = Argon2_i` and `type = Argon2_id` explicitly, against a matching encoded string, `hashlen` 16 and 64 variants | [x] |
| 8.75 | `argon2_verify` | encoded string with `p=2`: verification recomputes with `ctx.threads = ctx.lanes = 2` (`argon2_decode_string` copies `lanes` into `threads`) → `ARGON2_OK` | [x] |
| 8.76 | `argon2_encode_string` | `type = Argon2_i`, `m_cost = 8, t_cost = 1, lanes = 1, saltlen = 8, outlen = 16`, `dst_len = 128` → exact expected string; `argon2_decode_string` of it returns all four parameters unchanged | [x] |
| 8.77 | `argon2_encode_string` | `type = Argon2_id`, `m_cost = 65536, t_cost = 3, lanes = 1, saltlen = 16, outlen = 32`, `dst_len = 128` | [x] |
| 8.78 | `argon2_encode_string` | `type = Argon2_id`, `m_cost = 4294967295, t_cost = 4294967295, lanes = 16777215` (max-width decimal fields, 10+10+8 digits), `saltlen = 16`, `outlen = 32`, `dst_len = 128` → still fits (≈118 bytes); round-trips through `argon2_decode_string` | [x] |
| 8.79 | `argon2_encode_string` | `dst_len` exactly `strlen(expected) + 1` (tightest accepting size) for both types | [x] |
| 8.80 | `argon2_encode_string` | salt/out byte patterns that exercise the non-URL-safe Base64 alphabet: bytes producing `'+'` and `'/'` characters, and no `'='` padding (`sodium_base64_VARIANT_ORIGINAL_NO_PADDING`) | [x] |
| 8.81 | `argon2_encode_string` / `argon2_decode_string` | `saltlen % 3` ∈ {0, 1, 2}: `saltlen = 9` (12 b64 chars, no leftover bits), `saltlen = 16` (22 chars, 2 leftover bits), `saltlen = 8` (11 chars, 4 leftover bits) — all must round trip | [x] |
| 8.82 | `argon2_decode_string` | `"$argon2i$v=19$m=8,t=1,p=1$<salt b64>$<hash b64>"` with `ctx.saltlen`/`ctx.outlen` set to buffer capacities ≥ the encoded sizes → `ARGON2_OK`, and `ctx.threads == ctx.lanes` afterwards | [x] |
| 8.83 | `argon2_decode_string` | `maxsaltlen` and `maxoutlen` set *exactly* to the decoded sizes (8 and 16) — tightest accepting capacity | [x] |
| 8.84 | `argon2_decode_string` | `"$argon2id$v=19$m=65536,t=2,p=4$…"` (multi-digit m, p > 1); and `"m=8,t=1,p=1"` with a bare `0`-free minimal decimal for each field | [x] |
| 8.85 | `argon2_decode_string` | version field is mandatory and must be exactly `v=19`; confirm `"$argon2id$v=19$…"` succeeds (the `CC_opt` optional-version macro is dead code in this fork) | [x] |
| 8.86 | `argon2_decode_string` → `argon2_encode_string` | decode then re-encode a canonical string and compare byte-for-byte (canonical-form round trip), both types | [x] |
| 8.87 | `blake2b_long` | `outlen = 16, 32, 64` (short path, `crypto_generichash_blake2b_init` directly) with `inlen = ARGON2_BLOCK_SIZE` (1024) | [x] |
| 8.88 | `blake2b_long` | `outlen = 65` (first long-path size), `128`, `1024` (`ARGON2_BLOCK_SIZE`, as used by `argon2_fill_first_blocks`), `outlen` not a multiple of 32 (e.g. 100) to exercise the final partial `toproduce` block | [x] |
| 8.89 | `argon2_fill_segment_ref` | driven through `argon2_ctx`: `(pass = 0, slice = 0)` with `starting_index = 2`; `(pass = 0, slice > 0)`; `(pass > 0, any slice)`; `Argon2_id` with `slice < 2` vs `slice >= 2` — all four `data_independent_addressing`/`starting_index` combinations | [x] |
| 8.90 | `_crypto_pwhash_argon2_pick_best_implementation` | call it; returns 0 and (with no SIMD macros) leaves `fill_segment = argon2_fill_segment_ref`; digests before and after the call must be identical | [x] |
| 8.91 | `crypto_pwhash_scryptsalsa208sha256_*` constant getters | `bytes_min`/`_max` = 16 / 137438953440, `passwd_min`/`_max` = 0 / `SIZE_MAX`, `saltbytes` = 32, `strbytes` = 102, `strprefix` = `"$7$"`, `opslimit_min`/`_max`/`_interactive`/`_sensitive` = 32768 / 4294967295 / 524288 / 33554432, `memlimit_min`/`_max`/`_interactive`/`_sensitive` = 16777216 / 68719476736 / 16777216 / 1073741824 | [x] |
| 8.92 | `crypto_pwhash_scryptsalsa208sha256_ll` | smallest legal parameter set: `N = 2, r = 1, p = 1`, `passwdlen = 0`, `saltlen = 0`, `buflen = 16`; very fast | [x] |
| 8.93 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16, r = 1, p = 1`, `buflen = 32` (the classic scrypt test vector shape `N=16,r=1,p=1` with empty password and salt) | [x] |
| 8.94 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 1, p = 1`, `buflen = 64` | [x] |
| 8.95 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 8, p = 1`, `buflen = 64` (the params `pickparams` yields at OPSLIMIT_MIN/MEMLIMIT_MIN) | [x] |
| 8.96 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 1024, r = 8, p = 2` (`p > 1` → the `for (i = 0; i < p; i++) smix(...)` loop runs twice) | [x] |
| 8.97 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16384, r = 8, p = 1`, `buflen = 64` (the INTERACTIVE params); ~16 MiB, still fast | [x] |
| 8.98 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N = 16, r = 8, p = 16` (`r*p = 128`, well under 2^30) — many-p shape | [x] |
| 8.99 | `crypto_pwhash_scryptsalsa208sha256_ll` | `buflen` variations at fixed `N = 16, r = 1, p = 1`: `1`, `16`, `31`, `32`, `33`, `64`, `100` (non-multiple of 32 exercises the partial `clen` copy in `escrypt_PBKDF2_SHA256`) | [x] |
| 8.100 | `crypto_pwhash_scryptsalsa208sha256_ll` | `saltlen` variations: `0`, `1`, `32` (`SALTBYTES`), `64`; `passwdlen` variations: `0`, `1`, `64`; binary password/salt containing NUL bytes | [x] |
| 8.101 | `crypto_pwhash_scryptsalsa208sha256_ll` | repeated calls on the same `escrypt_local_t`-free API path — verify the local region is allocated and freed each call (`escrypt_init_local` → `escrypt_kdf_nosse` → `escrypt_free_local`) and results are reproducible | [x] |
| 8.102 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = OPSLIMIT_MIN` (32768), `memlimit = MEMLIMIT_MIN` (16777216): `pickparams` takes the **first** branch (`opslimit < memlimit/32` → 32768 < 524288) → `r = 8, p = 1, N_log2 = 10` (`N = 1024`); `outlen = 32`, 32-byte salt. Fast | [x] |
| 8.103 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = OPSLIMIT_INTERACTIVE` (524288), `memlimit = MEMLIMIT_INTERACTIVE` (16777216): `opslimit < memlimit/32` is **false** (equal) → **second** branch → `N_log2 = 14` (`N = 16384`), `r = 8`, `maxrp = 8`, `p = 1` | [x] |
| 8.104 | `crypto_pwhash_scryptsalsa208sha256` | second branch with `p > 1`: `opslimit = 32768`, `memlimit = 524288` → `maxN = 512`, `N_log2 = 9` (`N = 512`), `maxrp = 16`, `p = 2` | [x] |
| 8.105 | `crypto_pwhash_scryptsalsa208sha256` | degenerate-but-legal: `memlimit = 0` → second branch, `maxN = 0`, `N_log2 = 1` (`N = 2`), `r = 8`, `p = 512`; returns 0 (no minimum is enforced — see errors row 8.154) | [x] |
| 8.106 | `crypto_pwhash_scryptsalsa208sha256` | `opslimit = 0` (clamped to 32768 inside `pickparams`) with `memlimit = 16777216` → identical output to row 8.102 | [x] |
| 8.107 | `crypto_pwhash_scryptsalsa208sha256` | `outlen = 16` (`BYTES_MIN`), `32`, `64`, `100`; `passwdlen = 0`; salt is always exactly `SALTBYTES` = 32 bytes | [x] |
| 8.108 | `crypto_pwhash_scryptsalsa208sha256` **(SLOW — optional)** | `opslimit = OPSLIMIT_SENSITIVE` (33554432), `memlimit = MEMLIMIT_SENSITIVE` (1073741824) — 1 GiB; run at most once | [x] |
| 8.109 | `crypto_pwhash_scryptsalsa208sha256_str` | `opslimit = 32768, memlimit = 16777216`, `passwd = "password"` → 102-byte buffer holding exactly 101 chars + NUL, starting with `"$7$"`; `escrypt_gensalt_r` uses a random 32-byte salt so every call differs | [x] |
| 8.110 | `crypto_pwhash_scryptsalsa208sha256_str` + `_str_verify` | round trip at (32768, 16777216) → `0`; also with `passwdlen = 0` | [x] |
| 8.111 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | string produced at (32768, 16777216) queried with the same pair → `0`; queried with (524288, 16777216) → `1` (different `N_log2`); queried with (32768, 524288) → `1` (different `N_log2` and `p`) | [x] |
| 8.112 | `escrypt_gensalt_r` + `escrypt_parse_setting` | round trip for `(N_log2, r, p)` = `(10, 8, 1)`, `(14, 8, 1)`, `(9, 8, 2)`, `(1, 1, 1)`, `(1, 8, 512)`, `(63, 1, 1)` (max `N_log2`), `(0, 1, 1)`; `src` = 32 bytes, `buflen = 58` (`= 14 + 43 + 1`) | [x] |
| 8.113 | `escrypt_gensalt_r` | `srclen` variations: `0` (`saltlen = 0`, `need = 15`), `1`, `16`, `32`; `buflen` exactly `need` (tightest accepting size) | [x] |
| 8.114 | `escrypt_gensalt_r` | `r * p` just under the limit: `r = 1, p = 1073741823` (`= 2^30 - 1`) → accepted by `gensalt` (rejected later by the KDF only if actually used) | [x] |
| 8.115 | `escrypt_parse_setting` | parse a real `crypto_pwhash_scryptsalsa208sha256_str` output; returns a pointer to the first salt char (`setting + 14`) and the correct `N_log2`, `r`, `p`; also parse a bare setting with no trailing `$hash` | [x] |
| 8.116 | `escrypt_r` | `setting` from `escrypt_gensalt_r(10, 8, 1, salt32)`, `buflen = 102` (`crypto_pwhash_scryptsalsa208sha256_STRBYTES`) → 101-char `$7$` string; result must equal `crypto_pwhash_scryptsalsa208sha256_str` with the same salt | [x] |
| 8.117 | `escrypt_r` | `setting` **with** a trailing `"$<hash>"` (i.e. an existing password string used as the setting, which is how `_str_verify` works): `strrchr(salt, '$')` bounds the salt, and the recomputed string must equal the input | [x] |
| 8.118 | `escrypt_r` | shorter salt in the setting (e.g. 16-byte salt → `saltlen = 22`): `need = 14 + 22 + 1 + 43 + 1 = 81 <= buflen` → accepted, output is 80 chars + NUL | [x] |
| 8.119 | `escrypt_init_local` / `escrypt_free_local` / `escrypt_alloc_region` / `escrypt_free_region` | init → alloc `size = 1024`, `65536`, `128*8*(1024+1)+256*8+64` → free → free again (idempotent after `init_region`); `region->aligned` is 64-byte aligned when the non-mmap path is used | [x] |
| 8.120 | `escrypt_PBKDF2_SHA256` | `c = 1` (the only value scrypt uses), `dkLen` ∈ {0, 1, 32, 33, 64, 100, 128}; `passwdlen`/`saltlen` ∈ {0, 1, 32}; known PBKDF2-HMAC-SHA256 vectors | [x] |
| 8.121 | `escrypt_PBKDF2_SHA256` | `c = 2` and `c = 4096` (exercises the inner U-chain loop that scrypt itself never uses) with `dkLen = 32` | [x] |
| 8.122 | `escrypt_kdf_nosse` (direct) | called directly with an `escrypt_local_t` reused across several calls with growing `need` (forces the `local->size < need` re-allocation branch) and shrinking `need` (region reused, no re-allocation) | [x] |
| 8.123 | `crypto_ipcrypt_bytes`, `_keybytes` | no input; 16, 16 | [x] |
| 8.124 | `crypto_ipcrypt_nd_keybytes`, `_nd_tweakbytes`, `_nd_inputbytes`, `_nd_outputbytes` | no input; 16, 8, 16, 24 (`OUTPUT == TWEAK + INPUT`) | [x] |
| 8.125 | `crypto_ipcrypt_ndx_keybytes`, `_ndx_tweakbytes`, `_ndx_inputbytes`, `_ndx_outputbytes` | no input; 32, 16, 16, 32 | [x] |
| 8.126 | `crypto_ipcrypt_pfx_keybytes`, `_pfx_bytes` | no input; 32, 16 | [x] |
| 8.127 | `crypto_ipcrypt_keygen`, `_nd_keygen`, `_ndx_keygen`, `_pfx_keygen` | fill 16/16/32/32 bytes; two successive calls differ (randomness), whole buffer written | [x] |
| 8.128 | `crypto_ipcrypt_encrypt` / `crypto_ipcrypt_decrypt` | 16-byte round trip with an all-zero key; input = all-zero 16 bytes; deterministic (same in/key → same out) | [x] |
| 8.129 | `crypto_ipcrypt_encrypt` / `_decrypt` | round trip with a random `crypto_ipcrypt_keygen` key; input = all-0xFF 16 bytes | [x] |
| 8.130 | `crypto_ipcrypt_encrypt` / `_decrypt` | IPv4-mapped input: `::ffff:192.0.2.1` = `00×10 ff ff c0 00 02 01`; note the deterministic variant is *not* format-preserving (output is an arbitrary 16-byte block) | [x] |
| 8.131 | `crypto_ipcrypt_encrypt` / `_decrypt` | IPv4-mapped edge addresses `::ffff:0.0.0.0` and `::ffff:255.255.255.255` | [x] |
| 8.132 | `crypto_ipcrypt_encrypt` / `_decrypt` | pure IPv6 inputs: `::` (all-zero), `::1`, `2001:db8::1`, `ffff:…:ffff` (all-0xFF) | [x] |
| 8.133 | `crypto_ipcrypt_encrypt` | fixed known-answer: it is plain AES-128 ECB on one block, so `crypto_ipcrypt_encrypt` with the FIPS-197 key/plaintext must give the FIPS-197 ciphertext; `crypto_ipcrypt_decrypt` the inverse | [x] |
| 8.134 | `crypto_ipcrypt_nd_encrypt` / `crypto_ipcrypt_nd_decrypt` | 16-byte input, 8-byte tweak (all-zero), 16-byte key (all-zero) → 24-byte output whose first 8 bytes equal the tweak; decrypt recovers the input | [x] |
| 8.135 | `crypto_ipcrypt_nd_encrypt` / `_nd_decrypt` | random 8-byte tweak, random key, IPv4-mapped and IPv6 inputs; two different tweaks over the same input give different ciphertext halves; the same tweak reproduces the same output (deterministic given the tweak) | [x] |
| 8.136 | `crypto_ipcrypt_nd_encrypt` | tweak edge values: all-zero, all-0xFF, and a tweak whose odd bytes are non-zero (`tweak_expand` packs `tweak[2i]` and `tweak[2i+1]` into the low 16 bits of each 32-bit word, so all 8 bytes matter) | [x] |
| 8.137 | `crypto_ipcrypt_nd_decrypt` | feed back a 24-byte buffer built by hand (`tweak ‖ ciphertext`) rather than by `nd_encrypt` — decryption depends only on `in[0..8)` as the tweak | [x] |
| 8.138 | `crypto_ipcrypt_ndx_encrypt` / `crypto_ipcrypt_ndx_decrypt` | 16-byte input, 16-byte tweak (all-zero), 32-byte key with **distinct** halves → 32-byte output whose first 16 bytes equal the tweak; decrypt recovers the input | [x] |
| 8.139 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | random 16-byte tweak, random `crypto_ipcrypt_ndx_keygen` key, IPv4-mapped and IPv6 inputs; different tweaks → different ciphertexts | [x] |
| 8.140 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | **degenerate key**: `k[0..16) == k[16..32)` (e.g. all-zero 32-byte key) → the `d == 0` fixup re-derives the data key as `k[i] ^ 0x5a`; encryption/decryption still round trip, and the result differs from a non-degenerate key | [x] |
| 8.141 | `crypto_ipcrypt_ndx_encrypt` | key halves differing in a single bit (non-degenerate, `d != 0`) → no fixup applied | [x] |
| 8.142 | `crypto_ipcrypt_pfx_encrypt` / `crypto_ipcrypt_pfx_decrypt` | IPv4-mapped input (`::ffff:192.0.2.1`) with a 32-byte key of distinct halves → output keeps the `00×10 ff ff` IPv4-mapped prefix (format-preserving: `prefix_start = 96`, `encrypted[10] = encrypted[11] = 0xff`), only the last 4 bytes are randomised; decrypt recovers the input | [x] |
| 8.143 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | pure IPv6 input (`2001:db8::1`) → `prefix_start = 0`, all 128 bits processed, `pfx_pad_prefix` uses the `padded_prefix[15] = 0x01` seed; round trip | [x] |
| 8.144 | `crypto_ipcrypt_pfx_encrypt` | prefix-preservation property: two IPv4-mapped addresses sharing a /24 (`::ffff:192.0.2.1`, `::ffff:192.0.2.99`) must produce ciphertexts sharing the same leading 24 bits of the 32-bit v4 part; two addresses differing in the first octet must not | [x] |
| 8.145 | `crypto_ipcrypt_pfx_encrypt` | prefix-preservation for IPv6: two addresses sharing a /64 produce ciphertexts sharing the first 64 bits | [x] |
| 8.146 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | **degenerate key** `k[0..16) == k[16..32)` → `k2` re-derived as `k[i] ^ 0x5a`; round trip still holds | [x] |
| 8.147 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | edge inputs: `::` (all-zero), all-0xFF, `::ffff:0.0.0.0`, `::ffff:255.255.255.255` | [x] |
| 8.148 | `_crypto_ipcrypt_pick_best_implementation` | call it; returns 0 and (with no `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H`) keeps `ipcrypt_soft_implementation`; all outputs identical before and after the call | [x] |
| 8.149 | `ipcrypt_soft_implementation` struct | all eight function pointers (`encrypt`, `decrypt`, `nd_encrypt`, `nd_decrypt`, `ndx_encrypt`, `ndx_decrypt`, `pfx_encrypt`, `pfx_decrypt`) are non-NULL and reachable through the `crypto_ipcrypt_*` wrappers | [x] |
| 8.150 | (adjacent, `sodium/codecs.c`) `sodium_ip2bin` / `sodium_bin2ip` | used to build/verify the 16-byte ipcrypt inputs from IP **strings**: `"192.0.2.1"` → IPv4-mapped 16 bytes, `"2001:db8::1"` → IPv6 16 bytes, `"::ffff:192.0.2.1"` → same bytes as `"192.0.2.1"`; `sodium_bin2ip` renders an IPv4-mapped block back in dotted-quad form. **libsodium 1.0.23 has no `crypto_ipcrypt_*_str` entry points** — this row records where the string forms actually live | [x] |

**Row count: 150.** All 150 rows are covered by
`tests/a8_argon2.rs` (8.1 – 8.36), `tests/a8_argon2_core.rs` (8.37 – 8.75, 8.87 – 8.90),
`tests/a8_argon2_encoding.rs` (8.76 – 8.86), `tests/a8_scrypt.rs` (8.91 – 8.122) and
`tests/a8_ipcrypt.rs` (8.123 – 8.150).

Corrections found while writing those tests (the C is authoritative):

* Row 8.35's literal example `"$argon2id$v=19$m=8,t=1,p=2$…"` is **rejected** by
  `argon2_decode_string`'s final `argon2_validate_inputs` (`m_cost < 8 * lanes` →
  `ARGON2_MEMORY_TOO_LITTLE`), so `crypto_pwhash_str_needs_rehash` returns `-1`, not `0`.
  The "`p`/lanes are not compared" quirk is real and is pinned with `m=16,t=1,p=2` instead.
* Rows 8.44/8.45 are only true of the *amount of work*: `argon2_initial_hash` hashes the
  caller's raw `m_cost`, so `m_cost = 8, 9, 10, 11` produce four **different** digests even
  though they all round down to `segment_length = 2`.  The same applies to row 8.17's
  `memlimit = 9216`.

### Notes on axis interactions worth encoding as test-matrix invariants

1. `threads` never influences the digest (`argon2_fill_memory_blocks` ignores it); `lanes` does.
   Rows 8.41/8.42 pin this.
2. `m_cost` is rounded down to `segment_length * lanes * ARGON2_SYNC_POINTS`, so ranges of `m_cost`
   values are observationally equal (rows 8.44/8.45). The high-level API's `memlimit / 1024U`
   truncation adds a second, independent rounding (rows 8.17/8.33).
3. The high-level API fixes `lanes = threads = 1` and `saltlen = crypto_pwhash_SALTBYTES` (16), so
   the lanes/salt axes are only reachable through `argon2_ctx`/`argon2_hash`/`argon2*_hash_*`.
4. `crypto_pwhash_str*` always uses `STR_HASHBYTES = 32` and `p = 1`; the encoded-string axis for
   other `outlen`/`p` values is only reachable via `argon2*_hash_encoded` / `argon2_encode_string`.
5. scrypt's `pickparams` is the only place `opslimit`/`memlimit` are interpreted; both of its
   branches, and the `p > 1` and `N = 2` corners, are covered by rows 8.102–8.106.
6. ipcrypt ND/NDX are deterministic *given the tweak* and carry the tweak in the output
   (`out[0..8)` / `out[0..16)`); PFX is deterministic and prefix-preserving; the deterministic
   variant is raw AES-128 on one block.
