# sodiumutils family — error / rejection paths

Family: SODIUM UTILS + CODECS + RUNTIME
Sources: `c_src/libsodium/sodium/utils.c`, `codecs.c`, `runtime.c`, `version.c`.

Two classes of rejection exist:
- **Graceful**: returns `-1` (or a sentinel) and can be tested in-process.
- **Misuse/abort**: calls `sodium_misuse()` which `abort()`s the process. These
  cannot be exercised in a shared test binary (it would kill the process and
  both loaded libraries), so they are documented but NOT tested. The Rust
  translation mirrors the same `sodium_misuse()` calls, so behavior is identical.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | sodium_hex2bin | odd number of hex nibbles (dangling half-byte) | `-1`, `errno=EINVAL`, `*bin_len=0`, `hex_end` backed up by 1 |
| 2 | sodium_hex2bin | `bin_pos >= bin_maxlen` (output buffer full) | `-1`, `errno=ERANGE`, `*bin_len=0` |
| 3 | sodium_hex2bin | non-hex, non-ignored char with `hex_end==NULL` and unconsumed input | `-1`, `errno=EINVAL` |
| 4 | sodium_hex2bin | non-hex char (not in ignore set) with `hex_end!=NULL` | ret `0`; parsing stops, `*hex_end` points at the offending char |
| 5 | sodium_base642bin | leftover bits: `acc_len > 4` OR non-zero low bits `acc & ((1<<acc_len)-1)` | `-1`, `*bin_len=0` |
| 6 | sodium_base642bin | `bin_pos >= bin_maxlen` while decoding | `-1`, `errno=ERANGE`, `*bin_len=0` |
| 7 | sodium_base642bin | padding expected (padded variant) but missing / non-'=' non-ignored char in padding region | `-1`, `errno=EINVAL` (or `ERANGE` if b64 exhausted) |
| 8 | sodium_base642bin | invalid char (not in ignore) with `b64_end==NULL` and unconsumed input | `-1`, `errno=EINVAL` |
| 9 | sodium_base642bin | invalid char with `b64_end!=NULL` | ret depends on leftover-bit check; parsing stops, `*b64_end` points at the char |
| 10 | sodium_pad | `blocksize == 0` | `-1` |
| 11 | sodium_pad | `xpadded_len >= max_buflen` (padded output would not fit) | `-1` |
| 12 | sodium_pad | `SIZE_MAX - unpadded_buflen <= xpadlen` (overflow) | **misuse -> abort** (not tested) |
| 13 | sodium_unpad | `padded_buflen < blocksize` | `-1` |
| 14 | sodium_unpad | `blocksize == 0` | `-1` |
| 15 | sodium_unpad | invalid padding (no 0x80 barrier byte found in last block) | `-1` (`valid` stays 0 -> returns `(uint8)(0-1)` sign-extended = `-1`); `*unpadded_buflen_p` still written |
| 16 | sodium_bin2hex | `bin_len >= SIZE_MAX/2` OR `hex_maxlen <= bin_len*2` (buffer too small) | **misuse -> abort** (not tested) |
| 17 | sodium_bin2base64 | invalid `variant` (`(variant & ~0x6) != 0x1`) | **misuse -> abort** (not tested) |
| 18 | sodium_bin2base64 | `b64_maxlen <= b64_len` (buffer too small) | **misuse -> abort** (not tested) |
| 19 | sodium_bin2base64 | `nibbles > (SIZE_MAX-5)/4` (length overflow) | **misuse -> abort** (not tested) |
| 20 | sodium_base64_encoded_len | invalid `variant` | **misuse -> abort** (not tested) |
| 21 | sodium_base64_encoded_len | `bin_len/3 > (SIZE_MAX-5)/4` | **misuse -> abort** (not tested); macro path returns `SIZE_MAX` |
| 22 | sodium_base642bin | invalid `variant` | **misuse -> abort** (not tested) |

Notes:
- `sodium_memcmp` returns `0` (equal) / `-1` (differ) only; never errors.
- `sodium_compare` returns `-1/0/1`; never errors.
- `sodium_is_zero` returns `1` (all-zero, incl. empty) / `0`; never errors.
- `sodium_increment/add/sub/memzero/stackzero` are `void` and cannot fail.
- Runtime `sodium_runtime_has_*` and version functions never error.
