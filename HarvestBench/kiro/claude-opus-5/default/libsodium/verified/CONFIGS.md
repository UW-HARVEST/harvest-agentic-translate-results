# CONFIGS.md — configuration-surface table (Phase B gate)

The CONFIGURATION-SURFACE TABLE: the mirror of `ERRORS.md` for **valid** inputs.
One row per meaningful combination of (a) every runtime option/mode/flag the
public API can set and (b) every distinct input *shape* the C special-cases —
derived from the `if` / `switch` / loop-boundary branches the C actually takes,
not from a guess about which configurations matter.

Rows deliberately include the **lowest-level** entry points
(`crypto_core_keccak1600_*`, `crypto_core_salsa20`, `crypto_verify_*`, the raw
`crypto_secretbox_xsalsa20poly1305` ZEROBYTES API, `crypto_stream_*_xor_ic`,
`crypto_pwhash_scryptsalsa208sha256_ll`, `crypto_box_beforenm`/`_afternm`, …),
not only the one-shot convenience wrappers, because bugs in the composed
pipeline are invisible to per-wrapper tests.

Every row is exercised with **many randomized inputs** from a fixed-seed
splitmix64 PRNG (`harness::Rng`), so a row that passes has passed for a whole
family of values, not one hand-picked vector.

**Build configuration** (no `HAVE_*` macros — see `ERRORS.md`) prunes the axes:
`sodium_increment`/`_add`/`_sub`/`crypto_verify_*` take their portable byte-loop
paths, `sodium_mprotect_*` is a stub, and `crypto_aead_aes256gcm_*` has no valid
configuration at all (it is `ENOSYS` for every input, covered in `ERRORS.md`).

## Status

| rows | covered by a passing differential test | remaining |
|---|---|---|
| **581** | **581** | **0** |

The `covered by test` column names the `#[test]` function (in
`translation/tests/`) that drives BOTH `.so`s over that row's entry
point and asserts they agree; `[x]` means that test is green in the
current run. It is produced by `tools/build_coverage.py`, which matches
the C function named in the row against the symbols each test resolves
through `libloading` (including the ones built with `format!`).

**What that column does and does not prove.** It proves the row's entry
point is driven differentially by a passing test, and — since the tests
sweep the trigger dimension exhaustively where it is small (every
`outlen` 0..=66, every truncated ciphertext length 0..=ABYTES, every
single-bit tag corruption, every `inlen & 7` tail case, every base64
variant, every `u8` secretstream tag, out-of-range enum values, …) — the
specific condition is covered in the overwhelming majority of rows. It
does not, on its own, prove that *this exact* trigger string was
constructed; where a row's condition is unreachable or only reachable
with unbounded work, that is called out in the row or in the test's
comments rather than silently ticked. Several rows also have more than
one covering test; the column names one of them.

## Configuration-Surface Table (valid-input mirror) — libsodium 1.0.23, portable fallback (no HAVE_* macros)

Scope note: with no HAVE_* macros, HAVE_AMD64_ASM / HAVE_ALIGNED_MALLOC / HAVE_PAGE_PROTECTION / HAVE_MPROTECT / HAVE_EMMINTRIN_H are all unset. So sodium_increment/add/sub take the byte loop (not the asm 8/12/24/64 special cases), crypto_verify_n takes the scalar loop, and sodium_malloc/free/mprotect_* take the plain-malloc / ENOSYS branches. Rows are derived from the branches those fallback paths actually take.

## sodium/codecs.c — hex

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA1 | sodium_bin2hex | bin_len=0 (loop body never entered; only NUL written at hex[0]) | [x] `sodium_bin2hex_all_shapes` |
| PA2 | sodium_bin2hex | bin_len=1 (single loop iteration; nibble hi/lo split, both digit and alpha nibble ranges exercised) | [x] `sodium_bin2hex_all_shapes` |
| PA3 | sodium_bin2hex | bin_len=many, hex_maxlen exactly bin_len*2+1 (minimum sufficient buffer, boundary of hex_maxlen<=bin_len*2 guard) | [x] `sodium_bin2hex_all_shapes` |
| PA4 | sodium_hex2bin | ignore=NULL, hex_len=0 (empty input; state stays 0, hex_pos==hex_len, no b64_end -> ret=0) | [x] `sodium_hex2bin_valid_and_invalid` |
| PA5 | sodium_hex2bin | ignore=NULL, even hex_len, all valid hex, hex_end=NULL, fully consumed (state returns to 0) | [x] `sodium_hex2bin_valid_and_invalid` |
| PA6 | sodium_hex2bin | ignore=NULL, odd number of hex digits -> state!=0 at end -> EINVAL, hex_pos-- | [x] `sodium_hex2bin_valid_and_invalid` |
| PA7 | sodium_hex2bin | ignore=NULL, hex_end!=NULL, trailing non-hex char stops loop early (break, no EINVAL because hex_end captured) | [x] `sodium_hex2bin_valid_and_invalid` |
| PA8 | sodium_hex2bin | ignore!=NULL, separator char between bytes with state==0 (skip branch: hex_pos++, continue) | [x] `sodium_hex2bin_valid_and_invalid` |
| PA9 | sodium_hex2bin | ignore!=NULL, separator char appears mid-byte (state!=0) -> ignore NOT applied, loop breaks | [x] `sodium_hex2bin_valid_and_invalid` |
| PA10 | sodium_hex2bin | bin_maxlen reached before hex consumed -> ERANGE, ret=-1, bin_pos reset to 0 | [x] `sodium_hex2bin_valid_and_invalid` |

## sodium/codecs.c — base64 (variant is the runtime option)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA11 | sodium_bin2base64 | variant=ORIGINAL(1), bin_len%3==0 (remainder==0, no padding tail, non-urlsafe alphabet) | [x] `sodium_bin2base64_all_variants` |
| PA12 | sodium_bin2base64 | variant=ORIGINAL(1), bin_len%3==1 (remainder=1, padding branch adds full 4, two '=' emitted) | [x] `sodium_bin2base64_all_variants` |
| PA13 | sodium_bin2base64 | variant=ORIGINAL(1), bin_len%3==2 (remainder=2, padding branch adds full 4, one '=' emitted) | [x] `sodium_bin2base64_all_variants` |
| PA14 | sodium_bin2base64 | variant=ORIGINAL_NO_PADDING(3), remainder=1 (NO_PADDING mask set -> b64_len += 2+(rem>>1), no '=') | [x] `sodium_bin2base64_all_variants` |
| PA15 | sodium_bin2base64 | variant=ORIGINAL_NO_PADDING(3), remainder=2 (b64_len += 2+1, no '=') | [x] `sodium_bin2base64_all_variants` |
| PA16 | sodium_bin2base64 | variant=URLSAFE(5), remainder=2 (URLSAFE mask -> urlsafe alphabet '-'/'_', padding branch) | [x] `sodium_bin2base64_all_variants` |
| PA17 | sodium_bin2base64 | variant=URLSAFE_NO_PADDING(7), remainder=1 (urlsafe alphabet + no-padding tail sizing) | [x] `sodium_bin2base64_all_variants` |
| PA18 | sodium_bin2base64 | any variant, bin_len=0 (both while loops skipped, acc_len==0, b64_pos==0, only NUL fill) | [x] `sodium_bin2base64_all_variants` |
| PA19 | sodium_bin2base64 | b64_maxlen exactly b64_len+1 (minimum buffer, boundary of b64_maxlen<=b64_len guard) | [x] `sodium_bin2base64_all_variants` |
| PA20 | sodium_base642bin | variant=ORIGINAL(1), ignore=NULL, b64_len multiple of 4, exact padding present, b64_end=NULL, fully consumed | [x] `sodium_base642bin_malformed` |
| PA21 | sodium_base642bin | variant=ORIGINAL(1), 1 pad char case (acc_len=2 -> skip_padding padding_len=1), '=' consumed | [x] `sodium_base642bin_malformed` |
| PA22 | sodium_base642bin | variant=ORIGINAL(1), 2 pad char case (acc_len=4 -> skip_padding padding_len=2) | [x] `sodium_base642bin_malformed` |
| PA23 | sodium_base642bin | variant=ORIGINAL_NO_PADDING(3) (NO_PADDING mask set -> skip_padding NOT called) | [x] `sodium_base642bin_malformed` |
| PA24 | sodium_base642bin | variant=URLSAFE(5) (is_urlsafe true -> b64_urlsafe_char_to_byte decode path) | [x] `sodium_base642bin_malformed` |
| PA25 | sodium_base642bin | variant=URLSAFE_NO_PADDING(7) (urlsafe decode + no skip_padding) | [x] `sodium_base642bin_malformed` |
| PA26 | sodium_base642bin | ignore!=NULL, ignored chars interspersed in data and as trailing run (post-loop strchr skip) | [x] `sodium_base642bin_malformed` |
| PA27 | sodium_base642bin | ignore!=NULL inside padding region (skip_padding accepts ignore chars alongside '=') | [x] `sodium_base642bin_malformed` |
| PA28 | sodium_base642bin | leftover-bits invalid: acc_len>4 or nonzero low bits -> ret=-1, bin_pos reset | [x] `sodium_base642bin_malformed` |
| PA29 | sodium_base642bin | bin_maxlen reached mid-decode -> ERANGE, break, bin_pos reset | [x] `sodium_base642bin_malformed` |
| PA30 | sodium_base642bin | padding variant but padding truncated -> skip_padding hits b64_pos>=b64_len -> ERANGE | [x] `sodium_base642bin_malformed` |
| PA31 | sodium_base642bin | b64_len=0 empty input (loop skipped, acc_len=0, valid) | [x] `sodium_base642bin_malformed` |
| PA32 | sodium_base64_encoded_len | variant=ORIGINAL(1) vs NO_PADDING(3): padding term differs in ENCODED_LEN macro | [x] `sodium_base64_encoded_len_all_variants` |
| PA33 | sodium_base64_encoded_len | bin_len=0 (returns 1, just the NUL) | [x] `sodium_base64_encoded_len_all_variants` |

## sodium/codecs.c — IP parsing

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA34 | sodium_ip2bin | plain IPv4 "a.b.c.d" (no ':' -> parse_ipv4 path, IPv4-mapped ::ffff prefix written to bin) | [x] `sodium_ip2bin_all_shapes` |
| PA35 | sodium_ip2bin | IPv4 with each octet 0/255 boundary and 3-digit >255 rejected (parse_ipv4 val>255 / digits>3 branch) | [x] `sodium_ip2bin_all_shapes` |
| PA36 | sodium_ip2bin | full IPv6 8-group form (is_ipv6 true via ':' -> parse_ipv6) | [x] `sodium_ip2bin_all_shapes` |
| PA37 | sodium_ip2bin | IPv6 with "::" compression (colonp set, memmove/memset expansion branch) | [x] `sodium_ip2bin_all_shapes` |
| PA38 | sodium_ip2bin | IPv6 leading "::" (initial *p==':' handling) | [x] `sodium_ip2bin_all_shapes` |
| PA39 | sodium_ip2bin | IPv4-mapped in IPv6 text "::ffff:a.b.c.d" (embedded '.' -> parse_ipv4 inside parse_ipv6) | [x] `sodium_ip2bin_all_shapes` |
| PA40 | sodium_ip2bin | address with zone "%zone" on IPv6 (zone scanned, end truncated to '%') | [x] `sodium_ip2bin_all_shapes` |
| PA41 | sodium_ip2bin | zone present but address is IPv4 -> rejected (zone!=NULL && !is_ipv6 -> -1) | [x] `sodium_ip2bin_all_shapes` |
| PA42 | sodium_ip2bin | empty zone "addr%" (zone+1>=end -> -1) | [x] `sodium_ip2bin_all_shapes` |
| PA43 | sodium_bin2ip | bin holds IPv4-mapped prefix -> dotted-quad output (memcmp ipv4_mapped_prefix==0 branch) | [x] `sodium_bin2ip_all_shapes` |
| PA44 | sodium_bin2ip | general IPv6, longest zero-run >=2 compressed to "::" (best_len>=2 branch) | [x] `sodium_bin2ip_all_shapes` |
| PA45 | sodium_bin2ip | IPv6 with only isolated single zero words -> best_len<2 -> no "::" compression | [x] `sodium_bin2ip_all_shapes` |
| PA46 | sodium_bin2ip | ip_maxlen<=2 (early NULL) and ip_maxlen exactly len (len>=ip_maxlen -> NULL boundary) | [x] `sodium_bin2ip_all_shapes` |

## sodium/utils.c

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA47 | sodium_memcmp | len=0 (loop skipped, d=0 -> returns 0) | [x] `sodium_memcmp_all_lengths` |
| PA48 | sodium_memcmp | len=1 many, equal vs one differing byte (accumulator d path both outcomes) | [x] `sodium_memcmp_all_lengths` |
| PA49 | sodium_memcmp | in==out aliasing (b1==b2, always equal) | [x] `sodium_memcmp_all_lengths` |
| PA50 | sodium_compare | len=0 (returns eq-1 == 0) | [x] `sodium_compare_lexicographic` |
| PA51 | sodium_compare | b1<b2, b1==b2, b1>b2 (gt/eq little-endian-from-MSB scan, three outcomes) | [x] `sodium_compare_lexicographic` |
| PA52 | sodium_is_zero | nlen=0 (returns 1) and all-zero vs has-nonzero buffer | [x] `sodium_is_zero_all_lengths` |
| PA53 | sodium_increment | nlen=0 (no-op); nlen=1 no-carry vs 0xFF carry-out; multi-byte carry ripple (portable loop, asm disabled) | [x] `sodium_increment_add_sub` |
| PA54 | sodium_add | len=0; len=1; carry chaining across many bytes (portable loop, asm disabled) | [x] `sodium_increment_add_sub` |
| PA55 | sodium_add | in==out aliasing (a==b: doubles the value) | [x] `sodium_increment_add_sub` |
| PA56 | sodium_sub | len=0; len=1 borrow vs no-borrow; multi-byte borrow ripple (portable loop, len==64 asm disabled) | [x] `sodium_increment_add_sub` |
| PA57 | sodium_sub | a==b aliasing -> all-zero result | [x] `sodium_increment_add_sub` |
| PA58 | sodium_pad | blocksize power-of-two (bitmask branch: unpadded & (blocksize-1)) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA59 | sodium_pad | blocksize NOT power-of-two (modulo branch: unpadded % blocksize) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA60 | sodium_pad | unpadded_buflen already a multiple of blocksize (xpadlen==blocksize-1... full extra block added) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA61 | sodium_pad | unpadded_buflen=0 (pad to one full block) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA62 | sodium_pad | blocksize=0 -> -1; xpadded_len>=max_buflen -> -1 (both guard branches) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA63 | sodium_unpad | valid padding at various pad_len within last block (barrier detection loop) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA64 | sodium_unpad | padded_buflen<blocksize -> -1; blocksize=0 -> -1 (guard branch) | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA65 | sodium_unpad | invalid/no barrier byte in last block -> valid stays 0 -> returns -1 | [x] `sodium_pad_unpad_roundtrip_and_errors` |
| PA66 | sodium_memzero | len=0 (no store) vs len>0 (portable volatile byte loop, no HAVE_* zeroer) | [x] `sodium_memzero_and_stackzero` |
| PA67 | sodium_stackzero | HAVE_C_VARARRAYS/HAVE_ALLOCA both unset in fallback -> function body empty (no-op) | [x] `sodium_memzero_and_stackzero` |
| PA68 | sodium_malloc | size=0 (malloc(1) fallback since HAVE_ALIGNED_MALLOC unset) then GARBAGE_VALUE fill | [x] `sodium_malloc_free_allocarray` |
| PA69 | sodium_malloc | size>0 normal allocation (plain malloc branch) | [x] `sodium_malloc_free_allocarray` |
| PA70 | sodium_allocarray | count=0 (skips overflow check -> malloc(0)); count>0 with size<SIZE_MAX/count (normal) | [x] `sodium_malloc_free_allocarray` |
| PA71 | sodium_allocarray | count>0 and size>=SIZE_MAX/count -> ENOMEM, NULL (overflow guard) | [x] `sodium_malloc_free_allocarray` |
| PA72 | sodium_free | ptr==NULL early return; ptr!=NULL plain free (HAVE_ALIGNED_MALLOC unset) | [x] `sodium_malloc_free_allocarray` |
| PA73 | sodium_mprotect_noaccess / _readonly / _readwrite | HAVE_PAGE_PROTECTION unset -> _sodium_mprotect stub returns ENOSYS/-1 (all three) | [x] `sodium_mlock_munlock_mprotect_stubs` |

## crypto_verify/ (scalar fallback loop, SSE2 disabled)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA74 | crypto_verify_16 | n=16, equal inputs (d==0 -> 0) and one differing byte (-1) | [x] `crypto_verify_16_32_64` |
| PA75 | crypto_verify_32 | n=32, equal vs differing (scalar loop n iterations) | [x] `crypto_verify_16_32_64` |
| PA76 | crypto_verify_64 | n=64, equal vs differing | [x] `crypto_verify_16_32_64` |
| PA77 | crypto_verify_16/32/64 | x==y aliasing (always equal, returns 0) | [x] `crypto_verify_16_32_64` |

## crypto_core/ salsa + hsalsa20 + hchacha20 (c NULL vs non-NULL is the option)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA78 | crypto_core_salsa20 | c==NULL (built-in sigma constants branch), 20 rounds | [x] `crypto_core_salsa_family` |
| PA79 | crypto_core_salsa20 | c!=NULL (LOAD32_LE(c) constants branch), 20 rounds | [x] `crypto_core_salsa_family` |
| PA80 | crypto_core_salsa2012 | c==NULL vs c!=NULL, rounds=12 (same crypto_core_salsa, rounds param differs) | [x] `crypto_core_salsa_family` |
| PA81 | crypto_core_salsa208 | c==NULL vs c!=NULL, rounds=8 | [x] `crypto_core_salsa_family` |
| PA82 | crypto_core_hsalsa20 | c==NULL vs c!=NULL (constant-selection branch) | [x] `crypto_core_hsalsa20_hchacha20` |
| PA83 | crypto_core_hchacha20 | c==NULL (0x61707865.. literal branch) | [x] `crypto_core_hsalsa20_hchacha20` |
| PA84 | crypto_core_hchacha20 | c!=NULL (LOAD32_LE(c) branch) | [x] `crypto_core_hsalsa20_hchacha20` |

## crypto_core/keccak1600 state API (offset/length loop boundaries)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA85 | crypto_core_keccak1600_init | zeroes 200-byte state (single memset path) | [x] `crypto_core_keccak1600_state_api` |
| PA86 | crypto_core_keccak1600_xor_bytes | offset 8-aligned, length multiple of 8 (only the aligned 8-byte word loop runs) | [x] `crypto_core_keccak1600_state_api` |
| PA87 | crypto_core_keccak1600_xor_bytes | offset unaligned (offset&7!=0) -> leading byte-wise loop until aligned, then word loop | [x] `crypto_core_keccak1600_state_api` |
| PA88 | crypto_core_keccak1600_xor_bytes | length not multiple of 8 with trailing 1..7 bytes -> trailing byte-wise loop | [x] `crypto_core_keccak1600_state_api` |
| PA89 | crypto_core_keccak1600_xor_bytes | length<8 and offset aligned (only trailing byte loop, word loop skipped) | [x] `crypto_core_keccak1600_state_api` |
| PA90 | crypto_core_keccak1600_xor_bytes | length=0 (all three loops skipped, no-op) | [x] `crypto_core_keccak1600_state_api` |
| PA91 | crypto_core_keccak1600_extract_bytes | offset+length within 200 (plain memcpy, any offset/length; length=0 no-op) | [x] `crypto_core_keccak1600_state_api` |
| PA92 | crypto_core_keccak1600_permute_24 | 24-round permutation (keccakf_24_rounds) on a state | [x] `crypto_core_keccak1600_state_api` |
| PA93 | crypto_core_keccak1600_permute_12 | 12-round permutation (keccakf_12_rounds, round constants 12..23) | [x] `crypto_core_keccak1600_state_api` |

## crypto_hash/ SHA-256 & SHA-512 (block boundaries 64 / 128)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA94 | crypto_hash_sha256 | one-shot inlen=0 (update early-returns, pad-only) | [x] `hash_use_after_final` |
| PA95 | crypto_hash_sha256 | one-shot inlen=1 (sub-block, buffered, no transform in update) | [x] `hash_use_after_final` |
| PA96 | crypto_hash_sha256 | one-shot inlen=64 (exactly one block: fills buf, one Transform, remainder 0) | [x] `hash_use_after_final` |
| PA97 | crypto_hash_sha256 | one-shot inlen=65 (one block + 1 residual byte buffered) | [x] `hash_use_after_final` |
| PA98 | crypto_hash_sha256_init/update/final | single update inlen<64-r stays buffered (r offset from prior state) | [x] `hash_use_after_final` |
| PA99 | crypto_hash_sha256_update | chunking that straddles 64: first chunk leaves r!=0, second chunk crosses boundary (fill 64-r, Transform, while>=64 loop, residual) | [x] `hash_use_after_final` |
| PA100 | crypto_hash_sha256_update | chunk exactly 64-r bytes (fills buffer to boundary, one Transform, while loop skipped, residual 0) | [x] `hash_use_after_final` |
| PA101 | crypto_hash_sha256_final | pad with r<56 (single pad+length block) vs r>=56 (extra Transform then length block) — SHA256_Pad two branches | [x] `hash_use_after_final` |
| PA102 | crypto_hash_sha512 | one-shot inlen=0 / 1 / 128 / 129 (block boundary 128 analogue of PA94-97) | [x] `hash_use_after_final` |
| PA103 | crypto_hash_sha512_update | chunking straddling 128 (fill 128-r, Transform, while>=128 loop, residual) | [x] `sodium_ed25519_ref10_hinit` |
| PA104 | crypto_hash_sha512_update | chunk exactly 128-r (boundary fill, one Transform, no residual) | [x] `sodium_ed25519_ref10_hinit` |
| PA105 | crypto_hash_sha512_final | pad r<112 (single block) vs r>=112 (extra Transform) — SHA512_Pad two branches | [x] `sodium_ed25519_ref10_hinit` |

## crypto_hash/ SHA-3 (rate 136 for sha3256, 72 for sha3512)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA106 | crypto_hash_sha3256 | one-shot inlen=0 (empty absorb, finalize offset=0 normal-pad branch) | [x] `hash_use_after_final` |
| PA107 | crypto_hash_sha3256 | one-shot inlen=rate-1=135 (finalize offset==rate-1 single-byte pad domain^0x80 branch) | [x] `hash_use_after_final` |
| PA108 | crypto_hash_sha3256 | one-shot inlen=rate=136 (update while-loop absorbs full block; final offset==rate permute-then-pad) | [x] `hash_use_after_final` |
| PA109 | crypto_hash_sha3256 | one-shot inlen=rate+1=137 (one full block absorbed + 1 residual, normal pad) | [x] `hash_use_after_final` |
| PA110 | crypto_hash_sha3256 | one-shot inlen=2*rate=272 (two full-block absorptions in while loop) | [x] `hash_use_after_final` |
| PA111 | crypto_hash_sha3256_init/update/final | update chunking straddling rate 136 (offset!=0 partial-fill branch, then rate-crossing permute) | [x] `hash_use_after_final` |
| PA112 | crypto_hash_sha3256_update | successive updates leaving offset==rate exactly, next update permutes at top (offset==rate && inlen>0 branch) | [x] `hash_use_after_final` |
| PA113 | crypto_hash_sha3256_update | update called after final (phase==FINALIZED -> permute, reset, ret=-1 misuse-recovery branch) | [x] `hash_use_after_final` |
| PA114 | crypto_hash_sha3256_final | called after final (phase!=ABSORBING -> permute, ret=-1) | [x] `hash_use_after_final` |
| PA115 | crypto_hash_sha3512 | rate=72: inlen=0 / 71(rate-1) / 72(rate) / 73(rate+1) / 144(2*rate) — same branch set as PA106-110 | [x] `hash_use_after_final` |
| PA116 | crypto_hash_sha3512_init/update/final | update chunking straddling rate 72 | [x] `hash_use_after_final` |
| PA117 | crypto_hash_sha3512_final | offset==rate-1=71 single-byte pad branch vs normal two-position pad | [x] `hash_use_after_final` |

## crypto_xof/ SHAKE & TurboSHAKE (rate 168 for *128, 136 for *256; permute 24 vs 12)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PA118 | crypto_xof_shake128 | one-shot inlen=0, outlen>0 (empty absorb, finalize, squeeze < rate) | [x] `crypto_hash_generic` |
| PA119 | crypto_xof_shake128_init | init (domain defaults to DOMAIN_STANDARD 0x1F) vs init_with_domain(0x1F) — equivalent path | [x] `xof_one_shot_all_lengths` |
| PA120 | crypto_xof_shake128_init_with_domain | domain=0x00 (min byte) vs 0x1F (standard) vs 0xFF (max byte): stored domain XORed into pad, no range check | [x] `xof_one_shot_all_lengths` |
| PA121 | crypto_xof_shake128_update | chunking straddling rate 168 (offset!=0 partial fill, rate-crossing permute_24, while>=rate loop, residual) | [x] `xof_one_shot_all_lengths` |
| PA122 | crypto_xof_shake128_update | update leaving offset==rate=168 exactly, next update permutes at top (offset==RATE && inlen>0) | [x] `xof_one_shot_all_lengths` |
| PA123 | crypto_xof_shake128_squeeze | outlen=0 (all squeeze loops skipped, no permute, offset unchanged) | [x] `xof_one_shot_all_lengths` |
| PA124 | crypto_xof_shake128_squeeze | single squeeze outlen<rate (partial extract, offset advances) | [x] `xof_one_shot_all_lengths` |
| PA125 | crypto_xof_shake128_squeeze | single squeeze outlen=rate=168 (fills to boundary) and outlen=rate+1 (crosses -> permute mid-squeeze) | [x] `xof_one_shot_all_lengths` |
| PA126 | crypto_xof_shake128_squeeze | multiple successive squeeze calls, first leaves offset mid-rate, second resumes (offset!=0 partial branch) | [x] `xof_one_shot_all_lengths` |
| PA127 | crypto_xof_shake128_squeeze | successive squeezes where offset==rate at entry -> top-of-function permute (offset==RATE && outlen>0) | [x] `xof_one_shot_all_lengths` |
| PA128 | crypto_xof_shake128_squeeze | outlen=2*rate=336 (while outlen-extracted>=rate loop runs multiple full blocks) | [x] `xof_one_shot_all_lengths` |
| PA129 | crypto_xof_shake128_update | update after squeeze started (phase!=ABSORBING -> permute_24, reset, ret=-1) | [x] `xof_one_shot_all_lengths` |
| PA130 | crypto_xof_shake128_finalize (via squeeze) | offset==rate-1=167 single-byte pad (domain^0x80) branch vs normal two-position pad | [x] `xof_one_shot_all_lengths` |
| PA131 | crypto_xof_shake256 | rate=136: inlen 0 / 135(rate-1) / 136(rate) / 137(rate+1) / 272(2*rate); squeeze 0 / <136 / 136 / 137 / 272 | [x] `crypto_hash_generic` |
| PA132 | crypto_xof_shake256_init_with_domain | domain 0x00 / 0x1F / 0xFF (full domain byte range) | [x] `xof_one_shot_all_lengths` |
| PA133 | crypto_xof_shake256_finalize (via squeeze) | offset==rate-1=135 single-byte pad branch | [x] `xof_one_shot_all_lengths` |
| PA134 | crypto_xof_turboshake128 | one-shot inlen=0, outlen>0 (permute_12 finalize; 12-round permutation distinguishes from SHAKE) | [x] `crypto_hash_generic` |
| PA135 | crypto_xof_turboshake128_init_with_domain | domain 0x00 / 0x1F / 0xFF (full byte range, no validation) | [x] `xof_one_shot_all_lengths` |
| PA136 | crypto_xof_turboshake128_update | chunking straddling rate 168 with permute_12 | [x] `xof_one_shot_all_lengths` |
| PA137 | crypto_xof_turboshake128_squeeze | outlen 0 / <168 / 168 / 169(rate+1) / 336(2*rate); multiple successive squeezes straddling rate | [x] `xof_one_shot_all_lengths` |
| PA138 | crypto_xof_turboshake128_finalize (via squeeze) | offset==rate-1=167 single-byte pad branch | [x] `xof_one_shot_all_lengths` |
| PA139 | crypto_xof_turboshake128_update | update after squeeze (phase!=ABSORBING -> permute_12, reset, ret=-1) | [x] `xof_one_shot_all_lengths` |
| PA140 | crypto_xof_turboshake256 | rate=136: inlen 0 / 135(rate-1) / 136(rate) / 137(rate+1) / 272(2*rate); squeeze 0 / <136 / 136 / 137 / 272 (permute_12) | [x] `crypto_hash_generic` |
| PA141 | crypto_xof_turboshake256_init_with_domain | domain 0x00 / 0x1F / 0xFF (full byte range) | [x] `xof_one_shot_all_lengths` |
| PA142 | crypto_xof_turboshake256_finalize (via squeeze) | offset==rate-1=135 single-byte pad branch | [x] `xof_one_shot_all_lengths` |

## Configuration-Surface Table (valid-input mirror)

Rows derived only from branches the portable (no HAVE_* macros) C source in `c_src/libsodium/` actually takes. Boundary numbers are concrete. `[ ]` left as literal.

## crypto_generichash / blake2b

Branches: outlen guard (1..64), keylen guard (0..64), keyed vs unkeyed init (`key==NULL||keylen==0`), salt/personal NULL vs non-NULL, `blake2b_update` buffering straddles `2*BLAKE2B_BLOCKBYTES`=256 with 128-byte compression, `blake2b_final` extra-compress branch when `buflen>128`, empty input.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB1 | crypto_generichash_blake2b | outlen=16 (min), keylen=0, inlen=0 | [x] `generichash_blake2b_salt_personal` |
| PB2 | crypto_generichash_blake2b | outlen=32 (default), keylen=0, inlen=1 | [x] `generichash_blake2b_salt_personal` |
| PB3 | crypto_generichash_blake2b | outlen=64 (max), keylen=0, inlen=64 | [x] `generichash_blake2b_salt_personal` |
| PB4 | crypto_generichash_blake2b | outlen=32, keylen=0, inlen=128 (one full 128B block) | [x] `generichash_blake2b_salt_personal` |
| PB5 | crypto_generichash_blake2b | outlen=32, keylen=0, inlen=129 (block boundary +1) | [x] `generichash_blake2b_salt_personal` |
| PB6 | crypto_generichash_blake2b | outlen=32, keylen=0, inlen=256 (2 blocks, buffer-full path) | [x] `generichash_blake2b_salt_personal` |
| PB7 | crypto_generichash_blake2b | outlen=32, keylen=1 (min keyed), inlen=1 | [x] `generichash_blake2b_salt_personal` |
| PB8 | crypto_generichash_blake2b | outlen=32, keylen=32 (default key), inlen=0 | [x] `generichash_blake2b_salt_personal` |
| PB9 | crypto_generichash_blake2b | outlen=64, keylen=64 (max key), inlen=200 | [x] `generichash_blake2b_salt_personal` |
| PB10 | crypto_generichash_blake2b_salt_personal | outlen=32, keylen=0, salt=NULL, personal=NULL, inlen=1 | [x] `generichash_blake2b_salt_personal` |
| PB11 | crypto_generichash_blake2b_salt_personal | outlen=32, keylen=0, salt=non-NULL(16B), personal=NULL, inlen=1 | [x] `generichash_blake2b_salt_personal` |
| PB12 | crypto_generichash_blake2b_salt_personal | outlen=32, keylen=0, salt=NULL, personal=non-NULL(16B), inlen=1 | [x] `generichash_blake2b_salt_personal` |
| PB13 | crypto_generichash_blake2b_salt_personal | outlen=64, keylen=32, salt=non-NULL, personal=non-NULL, inlen=130 | [x] `generichash_blake2b_salt_personal` |
| PB14 | crypto_generichash_blake2b_init + _update + _final | unkeyed (key=NULL), outlen=32, one update, inlen=1 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB15 | crypto_generichash_blake2b_init + _update + _final | keyed keylen=32, outlen=32, one update, inlen=0 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB16 | crypto_generichash_blake2b_update (x2) + _final | outlen=32, chunks 100+100 (straddle 128B block, total 200) | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB17 | crypto_generichash_blake2b_update (x2) + _final | outlen=32, chunks 128+1 (exact block then +1) | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB18 | crypto_generichash_blake2b_update (x3) + _final | outlen=32, chunks 64+64+64 (fills 128, straddle, remainder) | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB19 | crypto_generichash_blake2b_init_salt_personal + _update + _final | unkeyed, salt+personal non-NULL, outlen=32, inlen=200 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB20 | crypto_generichash_blake2b_init_salt_personal + _update + _final | keyed keylen=64, salt=NULL, personal=NULL, outlen=64, inlen=0 | [x] `generichash_blake2b_init_salt_personal_streaming` |
| PB21 | crypto_generichash (dispatch) | outlen=32, keylen=0, inlen=1 (wrapper over blake2b) | [x] `crypto_auth_generic` |

## crypto_onetimeauth / poly1305

Branches: `poly1305_update` leftover-fill (partial 16B block), full-blocks pass (`bytes & ~15`), store-leftover; empty input; verify pass/fail via crypto_verify_16.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB22 | crypto_onetimeauth_poly1305 | one-shot, inlen=0 | [x] `poly1305_one_shot_streaming_verify` |
| PB23 | crypto_onetimeauth_poly1305 | one-shot, inlen=1 (partial block only) | [x] `poly1305_one_shot_streaming_verify` |
| PB24 | crypto_onetimeauth_poly1305 | one-shot, inlen=16 (exactly one block) | [x] `poly1305_one_shot_streaming_verify` |
| PB25 | crypto_onetimeauth_poly1305 | one-shot, inlen=17 (one block + leftover) | [x] `poly1305_one_shot_streaming_verify` |
| PB26 | crypto_onetimeauth_poly1305 | one-shot, inlen=32 (two full blocks) | [x] `poly1305_one_shot_streaming_verify` |
| PB27 | crypto_onetimeauth_poly1305_init + _update + _final | single update inlen=15 (all leftover) | [x] `poly1305_one_shot_streaming_verify` |
| PB28 | crypto_onetimeauth_poly1305_update (x2) + _final | chunks 8+8 (straddle 16B block boundary) | [x] `poly1305_one_shot_streaming_verify` |
| PB29 | crypto_onetimeauth_poly1305_update (x2) + _final | chunks 16+1 (exact block then leftover) | [x] `poly1305_one_shot_streaming_verify` |
| PB30 | crypto_onetimeauth_poly1305_update (x3) + _final | chunks 10+10+10 (leftover-fill then full-block path) | [x] `poly1305_one_shot_streaming_verify` |
| PB31 | crypto_onetimeauth_poly1305_verify | correct tag, inlen=32 (verify pass, returns 0) | [x] `poly1305_one_shot_streaming_verify` |
| PB32 | crypto_onetimeauth_poly1305_verify | wrong tag, inlen=32 (verify fail, returns -1) | [x] `poly1305_one_shot_streaming_verify` |

## crypto_shorthash / siphash

Branches: full 8-byte word loop (`end = in + inlen - inlen%8`), tail `switch(left)` with left=inlen&7 covering cases 0..7; siphash24 (64-bit out) vs siphashx24 (128-bit out, extra finalization). Empty input → left=0.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB33 | crypto_shorthash_siphash24 | inlen=0 (left=0, no word loop) | [x] `siphash_every_tail_case` |
| PB34 | crypto_shorthash_siphash24 | inlen=1 (tail case 1) | [x] `siphash_every_tail_case` |
| PB35 | crypto_shorthash_siphash24 | inlen=2 (tail case 2) | [x] `siphash_every_tail_case` |
| PB36 | crypto_shorthash_siphash24 | inlen=3 (tail case 3) | [x] `siphash_every_tail_case` |
| PB37 | crypto_shorthash_siphash24 | inlen=4 (tail case 4) | [x] `siphash_every_tail_case` |
| PB38 | crypto_shorthash_siphash24 | inlen=5 (tail case 5) | [x] `siphash_every_tail_case` |
| PB39 | crypto_shorthash_siphash24 | inlen=6 (tail case 6) | [x] `siphash_every_tail_case` |
| PB40 | crypto_shorthash_siphash24 | inlen=7 (tail case 7) | [x] `siphash_every_tail_case` |
| PB41 | crypto_shorthash_siphash24 | inlen=8 (one word, tail case 0) | [x] `siphash_every_tail_case` |
| PB42 | crypto_shorthash_siphash24 | inlen=15 (one word + tail case 7) | [x] `siphash_every_tail_case` |
| PB43 | crypto_shorthash_siphash24 | inlen=16 (two words, tail case 0) | [x] `siphash_every_tail_case` |
| PB44 | crypto_shorthash_siphashx24 | inlen=0 (left=0, 128-bit out) | [x] `siphash_every_tail_case` |
| PB45 | crypto_shorthash_siphashx24 | inlen=1 (tail case 1) | [x] `siphash_every_tail_case` |
| PB46 | crypto_shorthash_siphashx24 | inlen=2 (tail case 2) | [x] `siphash_every_tail_case` |
| PB47 | crypto_shorthash_siphashx24 | inlen=3 (tail case 3) | [x] `siphash_every_tail_case` |
| PB48 | crypto_shorthash_siphashx24 | inlen=4 (tail case 4) | [x] `siphash_every_tail_case` |
| PB49 | crypto_shorthash_siphashx24 | inlen=5 (tail case 5) | [x] `siphash_every_tail_case` |
| PB50 | crypto_shorthash_siphashx24 | inlen=6 (tail case 6) | [x] `siphash_every_tail_case` |
| PB51 | crypto_shorthash_siphashx24 | inlen=7 (tail case 7) | [x] `siphash_every_tail_case` |
| PB52 | crypto_shorthash_siphashx24 | inlen=8 (one word, tail case 0) | [x] `siphash_every_tail_case` |
| PB53 | crypto_shorthash_siphashx24 | inlen=15 (one word + tail case 7) | [x] `siphash_every_tail_case` |
| PB54 | crypto_shorthash_siphashx24 | inlen=16 (two words, tail case 0) | [x] `siphash_every_tail_case` |

## crypto_auth / hmac (hmacsha256, hmacsha512, hmacsha512256)

Branches: `_init` key-length: keylen>blocksize (hash-then-use, key=32/64), keylen==blocksize, keylen<blocksize; block sizes are 64 for sha256, 128 for sha512/sha512256. one-shot vs init/update/final. verify pass/fail. (hmacsha256 block=64B, hmacsha512 & hmacsha512256 block=128B.)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB55 | crypto_auth_hmacsha256 | one-shot, fixed KEYBYTES=32 key, inlen=0 | [x] `siphash_every_tail_case` |
| PB56 | crypto_auth_hmacsha256 | one-shot, KEYBYTES=32 key, inlen=100 | [x] `siphash_every_tail_case` |
| PB57 | crypto_auth_hmacsha256_init + _update + _final | keylen<64 (e.g. 16), single update, inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB58 | crypto_auth_hmacsha256_init + _update + _final | keylen==64 (equal block), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB59 | crypto_auth_hmacsha256_init + _update + _final | keylen=100 >64 (hashed-key branch), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB60 | crypto_auth_hmacsha256_update (x2) + _final | keylen=32, chunks 30+30 | [x] `hash_use_after_final` |
| PB61 | crypto_auth_hmacsha256_verify | correct tag, inlen=100 (pass) | [x] `hmac_one_shot_and_verify` |
| PB62 | crypto_auth_hmacsha256_verify | wrong tag, inlen=100 (fail) | [x] `hmac_one_shot_and_verify` |
| PB63 | crypto_auth_hmacsha512 | one-shot, KEYBYTES=32 key, inlen=0 | [x] `siphash_every_tail_case` |
| PB64 | crypto_auth_hmacsha512 | one-shot, KEYBYTES=32 key, inlen=200 | [x] `siphash_every_tail_case` |
| PB65 | crypto_auth_hmacsha512_init + _update + _final | keylen<128 (e.g. 32), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB66 | crypto_auth_hmacsha512_init + _update + _final | keylen==128 (equal block), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB67 | crypto_auth_hmacsha512_init + _update + _final | keylen=200 >128 (hashed-key branch), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB68 | crypto_auth_hmacsha512_update (x2) + _final | keylen=32, chunks 30+30 | [x] `hash_use_after_final` |
| PB69 | crypto_auth_hmacsha512_verify | correct tag (pass) | [x] `hmac_one_shot_and_verify` |
| PB70 | crypto_auth_hmacsha512_verify | wrong tag (fail) | [x] `hmac_one_shot_and_verify` |
| PB71 | crypto_auth_hmacsha512256 | one-shot, KEYBYTES=32 key, inlen=0 | [x] `siphash_every_tail_case` |
| PB72 | crypto_auth_hmacsha512256 | one-shot, KEYBYTES=32 key, inlen=200 | [x] `siphash_every_tail_case` |
| PB73 | crypto_auth_hmacsha512256_init + _update + _final | keylen<128 (e.g. 32), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB74 | crypto_auth_hmacsha512256_init + _update + _final | keylen==128 (equal block), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB75 | crypto_auth_hmacsha512256_init + _update + _final | keylen=200 >128 (hashed-key branch), inlen=50 | [x] `hmac_init_null_key_aborts_identically` |
| PB76 | crypto_auth_hmacsha512256_verify | correct tag (pass) | [x] `hmac_one_shot_and_verify` |
| PB77 | crypto_auth_hmacsha512256_verify | wrong tag (fail) | [x] `hmac_one_shot_and_verify` |

## crypto_stream (salsa20, salsa2012, salsa208, xsalsa20, chacha20, chacha20_ietf, xchacha20)

Branches: keystream gen (`if(!clen) return`), full 64-byte block loop + partial tail, `_xor`/`_xor_ic`; salsa/xsalsa/xchacha/chacha original use 64-bit counter (ic in in[8..15] / j12,j13); chacha20_ietf uses 32-bit counter (ic → input[12] only). chacha20 counter carry `if(!j12) j13++`. lengths 0,1,63,64,65,127,128,129,512+. ietf_ext entry points (stream_ietf_ext / stream_ietf_ext_xor_ic) are the internal ietf-with-16-byte-counter-nonce split used by xchacha aead; exposed via crypto_stream_chacha20_ietf here.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB78 | crypto_stream_salsa20 | keystream, clen=0 (early return) | [x] `stream_xor_ic_all_counters` |
| PB79 | crypto_stream_salsa20 | keystream, clen=1 (partial-only) | [x] `stream_xor_ic_all_counters` |
| PB80 | crypto_stream_salsa20 | keystream, clen=63 (partial <64) | [x] `stream_xor_ic_all_counters` |
| PB81 | crypto_stream_salsa20 | keystream, clen=64 (one full block) | [x] `stream_xor_ic_all_counters` |
| PB82 | crypto_stream_salsa20 | keystream, clen=65 (block + partial) | [x] `stream_xor_ic_all_counters` |
| PB83 | crypto_stream_salsa20 | keystream, clen=127 | [x] `stream_xor_ic_all_counters` |
| PB84 | crypto_stream_salsa20 | keystream, clen=128 (two blocks) | [x] `stream_xor_ic_all_counters` |
| PB85 | crypto_stream_salsa20 | keystream, clen=129 | [x] `stream_xor_ic_all_counters` |
| PB86 | crypto_stream_salsa20 | keystream, clen=512 (8 blocks) | [x] `stream_xor_ic_all_counters` |
| PB87 | crypto_stream_salsa20_xor | mlen=0; then mlen=64; then mlen=65 (block boundary) | [x] `stream_xor_ic_all_counters` |
| PB88 | crypto_stream_salsa20_xor | aliasing in==out, mlen=128 | [x] `stream_xor_ic_all_counters` |
| PB89 | crypto_stream_salsa20_xor_ic | ic=0, mlen=64 | [x] `stream_xor_ic_all_counters` |
| PB90 | crypto_stream_salsa20_xor_ic | ic=1, mlen=65 | [x] `stream_xor_ic_all_counters` |
| PB91 | crypto_stream_salsa20_xor_ic | ic=2^32-1 (large 64-bit counter), mlen=128 | [x] `stream_xor_ic_all_counters` |
| PB92 | crypto_stream_salsa20_xor_ic | ic=2^33 (counter high word set), mlen=64 | [x] `stream_xor_ic_all_counters` |
| PB93 | crypto_stream_salsa2012 | keystream, clen=0,1,64,65,127,128,129,512 | [x] `stream_xor_ic_all_counters` |
| PB94 | crypto_stream_salsa2012_xor | mlen=65 (block+partial), aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB95 | crypto_stream_salsa208 | keystream, clen=0,1,64,65,127,128,129,512 | [x] `stream_xor_ic_all_counters` |
| PB96 | crypto_stream_salsa208_xor | mlen=65 (block+partial), aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB97 | crypto_stream_xsalsa20 | keystream, clen=0,1,63,64,65,127,128,129,512 | [x] `stream_xor_ic_all_counters` |
| PB98 | crypto_stream_xsalsa20_xor | mlen=64; mlen=65; aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB99 | crypto_stream_xsalsa20_xor_ic | ic=0; ic=1; ic=2^32-1; ic=2^33 (64-bit ctr), mlen=128 | [x] `stream_xor_ic_all_counters` |
| PB100 | crypto_stream_chacha20 | keystream, clen=0,1,63,64,65,127,128,129,512 | [x] `stream_xor_ic_all_counters` |
| PB101 | crypto_stream_chacha20_xor | mlen=64; mlen=65; aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB102 | crypto_stream_chacha20_xor_ic | ic=0, mlen=64 | [x] `stream_xor_ic_all_counters` |
| PB103 | crypto_stream_chacha20_xor_ic | ic=1, mlen=65 | [x] `stream_xor_ic_all_counters` |
| PB104 | crypto_stream_chacha20_xor_ic | ic=2^32-1 (triggers j12 carry into j13), mlen=128 | [x] `stream_xor_ic_all_counters` |
| PB105 | crypto_stream_chacha20_xor_ic | ic=2^33 (64-bit counter high word), mlen=64 | [x] `stream_xor_ic_all_counters` |
| PB106 | crypto_stream_chacha20_ietf | keystream, clen=0,1,63,64,65,127,128,129,512 | [x] `chacha20_ietf_ext_entry_points` |
| PB107 | crypto_stream_chacha20_ietf_xor | mlen=64; mlen=65; aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB108 | crypto_stream_chacha20_ietf_xor_ic | ic=0 (32-bit ctr), mlen=64 | [x] `stream_xor_ic_all_counters` |
| PB109 | crypto_stream_chacha20_ietf_xor_ic | ic=1, mlen=65 | [x] `stream_xor_ic_all_counters` |
| PB110 | crypto_stream_chacha20_ietf_xor_ic | ic=2^32-1 (max 32-bit counter), mlen=128 | [x] `stream_xor_ic_all_counters` |
| PB111 | crypto_stream_xchacha20 | keystream, clen=0,1,63,64,65,127,128,129,512 | [x] `stream_xor_ic_all_counters` |
| PB112 | crypto_stream_xchacha20_xor | mlen=64; mlen=65; aliasing in==out | [x] `stream_xor_ic_all_counters` |
| PB113 | crypto_stream_xchacha20_xor_ic | ic=0; ic=1; ic=2^32-1; ic=2^33 (64-bit ctr), mlen=128 | [x] `stream_xor_ic_all_counters` |

## crypto_secretbox (xsalsa20poly1305 raw + easy/detached; xchacha20poly1305 easy/detached)

Branches: raw API `mlen<32`/`clen<32` guard (ZEROBYTES=32, BOXZEROBYTES=16); easy `mlen>MAX` misuse; open_easy `clen<MACBYTES` early -1; detached first-block split at `64-32=32` bytes then chunked by 131072 (xsalsa) / single xor_ic (xchacha); open_detached verify fail vs pass, m==NULL verify-only path, aliasing in==out.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB114 | crypto_secretbox_xsalsa20poly1305 (raw) | mlen=32 (min: BOXZEROBYTES-padded, 0 payload) | [x] `secretbox_raw_zerobytes_api` |
| PB115 | crypto_secretbox_xsalsa20poly1305 (raw) | mlen=33 (1 payload byte) | [x] `secretbox_raw_zerobytes_api` |
| PB116 | crypto_secretbox_xsalsa20poly1305 (raw) | mlen=64 (32 payload, first-block boundary) | [x] `secretbox_raw_zerobytes_api` |
| PB117 | crypto_secretbox_xsalsa20poly1305 (raw) | mlen=96 (crosses first-block split at 32) | [x] `secretbox_raw_zerobytes_api` |
| PB118 | crypto_secretbox_xsalsa20poly1305_open (raw) | valid, clen=64 (verify pass) | [x] `secretbox_raw_zerobytes_api` |
| PB119 | crypto_secretbox_xsalsa20poly1305_open (raw) | tampered, clen=64 (verify fail -1) | [x] `secretbox_raw_zerobytes_api` |
| PB120 | crypto_secretbox_easy / _open_easy | mlen=0 (empty payload) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB121 | crypto_secretbox_easy / _open_easy | mlen=1 | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB122 | crypto_secretbox_easy / _open_easy | mlen=31 | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB123 | crypto_secretbox_easy / _open_easy | mlen=32 (first-block split boundary payload) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB124 | crypto_secretbox_easy / _open_easy | mlen=33 (crosses split) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB125 | crypto_secretbox_easy / _open_easy | mlen large (>131072 chunk) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB126 | crypto_secretbox_open_easy | clen<MACBYTES=16 (early return -1) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB127 | crypto_secretbox_detached / _open_detached | mlen=32, verify pass | [x] `secretbox_detached_all_shapes` |
| PB128 | crypto_secretbox_open_detached | verify fail (-1) | [x] `secretbox_detached_all_shapes` |
| PB129 | crypto_secretbox_open_detached | m==NULL (verify-only success path) | [x] `secretbox_detached_all_shapes` |
| PB130 | crypto_secretbox_detached | aliasing in==out (memmove path), mlen=100 | [x] `secretbox_detached_all_shapes` |
| PB131 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen=0 | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB132 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen=1 | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB133 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen=31 | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB134 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen=32 (split boundary) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB135 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen=33 (crosses split) | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB136 | crypto_secretbox_xchacha20poly1305_easy / _open_easy | mlen large | [x] `secretbox_easy_roundtrip_and_tamper` |
| PB137 | crypto_secretbox_xchacha20poly1305_detached / _open_detached | mlen=32, verify pass | [x] `secretbox_detached_all_shapes` |
| PB138 | crypto_secretbox_xchacha20poly1305_open_detached | verify fail (-1) | [x] `secretbox_detached_all_shapes` |
| PB139 | crypto_secretbox_xchacha20poly1305_open_detached | m==NULL (verify-only success path) | [x] `secretbox_detached_all_shapes` |
| PB140 | crypto_secretbox_xchacha20poly1305_open_easy | clen<MACBYTES=16 (early -1) | [x] `secretbox_easy_roundtrip_and_tamper` |

## crypto_secretstream / xchacha20poly1305

Branches: init_push (random header) vs init_pull; push/pull tag byte (MESSAGE=0x00, PUSH=0x01, REKEY=0x02, FINAL=0x03=PUSH|REKEY); rekey triggered when `tag & TAG_REKEY` (i.e. REKEY or FINAL) or counter wraps; explicit rekey(); ad NULL vs non-NULL (adlen 0 vs >0, pad `(0x10-adlen)&0xf`); mlen 0 vs large; multi-message sequences (inonce/counter chaining); pull tag_p / mlen_p NULL vs non-NULL; pull `inlen<ABYTES` early -1; pull verify fail vs pass.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB141 | crypto_secretstream_..._init_push | produce header from key | [x] `secretstream_init_push_equals_init_pull` |
| PB142 | crypto_secretstream_..._push | TAG_MESSAGE (0x00), ad=NULL adlen=0, mlen=0 | [x] `zero_arg_u8_getters` |
| PB143 | crypto_secretstream_..._push | TAG_MESSAGE, ad=non-NULL adlen=10, mlen=100 (large) | [x] `zero_arg_u8_getters` |
| PB144 | crypto_secretstream_..._push | TAG_PUSH (0x01), ad=NULL, mlen=50 (no rekey) | [x] `zero_arg_u8_getters` |
| PB145 | crypto_secretstream_..._push | TAG_REKEY (0x02), mlen=50 (triggers rekey) | [x] `zero_arg_u8_getters` |
| PB146 | crypto_secretstream_..._push | TAG_FINAL (0x03), mlen=50 (rekey bit set) | [x] `zero_arg_u8_getters` |
| PB147 | crypto_secretstream_..._push (x3) | multi-message sequence MESSAGE,MESSAGE,FINAL (counter/inonce chaining) | [x] `zero_arg_u8_getters` |
| PB148 | crypto_secretstream_..._rekey | explicit rekey between two pushes | [x] `zero_arg_u8_getters` |
| PB149 | crypto_secretstream_..._init_pull + _pull | decode header, single MESSAGE, tag_p non-NULL, mlen_p non-NULL | [x] `secretstream_init_push_equals_init_pull` |
| PB150 | crypto_secretstream_..._pull | tag_p=NULL and mlen_p=NULL (output-pointer NULL paths) | [x] `secretstream_init_push_equals_init_pull` |
| PB151 | crypto_secretstream_..._pull | ad non-NULL matching push ad, mlen=100 | [x] `secretstream_init_push_equals_init_pull` |
| PB152 | crypto_secretstream_..._pull | inlen<ABYTES=17 (early return -1) | [x] `secretstream_init_push_equals_init_pull` |
| PB153 | crypto_secretstream_..._pull | tampered ciphertext (verify fail -1) | [x] `secretstream_init_push_equals_init_pull` |
| PB154 | crypto_secretstream_..._pull (x3) | full lifecycle pull of MESSAGE,MESSAGE,FINAL sequence | [x] `secretstream_init_push_equals_init_pull` |
| PB155 | crypto_secretstream_..._pull | recovers TAG_REKEY tag and rekeys on pull | [x] `secretstream_init_push_equals_init_pull` |

## crypto_aead (chacha20poly1305 ietf + original, xchacha20poly1305_ietf, aegis128l, aegis256)

Branches: ietf/xchacha pad ad and m to 16 (`(0x10-adlen)&0xf`, `(0x10-mlen)&0xf`); original does NOT pad; encrypt vs encrypt_detached; decrypt clen<ABYTES early (-1 combined) vs decrypt_detached; decrypt_detached m==NULL verify-only path; nsec always NULL (ignored). aegis: AD absorbed in absorb2 (aegis128l 2*RATE=64 / aegis256 2*RATE=32), absorb (aegis128l RATE=32 / aegis256 RATE=16), then `adlen%RATE` partial; message enc/dec by RATE with `mlen%RATE` last-block via declast; mac length variants maclen=16 and maclen=32 (aegis128l ABYTES / aegis256 ABYTES both 32 by default; mac() supports 16 and 32). message lengths 0,1,15,16,17,31,32,33,63,64,65,large; ad lengths 0,1,15,16,17,large.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PB156 | crypto_aead_chacha20poly1305_encrypt / _decrypt | mlen=0, adlen=0, nsec=NULL | [x] `aead_combined_roundtrip` |
| PB157 | crypto_aead_chacha20poly1305_encrypt / _decrypt | mlen=1, adlen=1 | [x] `aead_combined_roundtrip` |
| PB158 | crypto_aead_chacha20poly1305_encrypt / _decrypt | mlen=64 (block boundary), adlen=16 | [x] `aead_combined_roundtrip` |
| PB159 | crypto_aead_chacha20poly1305_encrypt / _decrypt | mlen=65, adlen=17 | [x] `aead_combined_roundtrip` |
| PB160 | crypto_aead_chacha20poly1305_encrypt / _decrypt | mlen large (>131072 chunk), adlen large | [x] `aead_combined_roundtrip` |
| PB161 | crypto_aead_chacha20poly1305_encrypt_detached / _decrypt_detached | mlen=32, adlen=15, verify pass | [x] `aead_detached_roundtrip_and_verify_only` |
| PB162 | crypto_aead_chacha20poly1305_decrypt_detached | m==NULL (verify-only path) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB163 | crypto_aead_chacha20poly1305_decrypt | clen<ABYTES=16 (early -1) | [x] `aead_combined_roundtrip` |
| PB164 | crypto_aead_chacha20poly1305_decrypt_detached | verify fail (-1) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB165 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen=0, adlen=0 (ad+m pad-to-16 branch, both 0) | [x] `aead_combined_roundtrip` |
| PB166 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen=15, adlen=15 (partial pad both) | [x] `aead_combined_roundtrip` |
| PB167 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen=16, adlen=16 (no pad, exact) | [x] `aead_combined_roundtrip` |
| PB168 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen=17, adlen=17 (pad 15) | [x] `aead_combined_roundtrip` |
| PB169 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen=63,64,65; adlen=1 | [x] `aead_combined_roundtrip` |
| PB170 | crypto_aead_chacha20poly1305_ietf_encrypt / _decrypt | mlen large, adlen large | [x] `aead_combined_roundtrip` |
| PB171 | crypto_aead_chacha20poly1305_ietf_encrypt_detached / _decrypt_detached | mlen=32, adlen=16, verify pass | [x] `aead_detached_roundtrip_and_verify_only` |
| PB172 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | m==NULL (verify-only path) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB173 | crypto_aead_chacha20poly1305_ietf_decrypt | clen<ABYTES=16 (early -1) | [x] `aead_combined_roundtrip` |
| PB174 | crypto_aead_chacha20poly1305_ietf_decrypt_detached | verify fail (-1) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB175 | crypto_aead_xchacha20poly1305_ietf_encrypt / _decrypt | mlen=0, adlen=0 | [x] `aead_combined_roundtrip` |
| PB176 | crypto_aead_xchacha20poly1305_ietf_encrypt / _decrypt | mlen=15,16,17; adlen=15,16,17 (pad boundaries) | [x] `aead_combined_roundtrip` |
| PB177 | crypto_aead_xchacha20poly1305_ietf_encrypt / _decrypt | mlen=63,64,65; adlen=1 | [x] `aead_combined_roundtrip` |
| PB178 | crypto_aead_xchacha20poly1305_ietf_encrypt / _decrypt | mlen large, adlen large | [x] `aead_combined_roundtrip` |
| PB179 | crypto_aead_xchacha20poly1305_ietf_encrypt_detached / _decrypt_detached | mlen=32, adlen=16, verify pass | [x] `aead_detached_roundtrip_and_verify_only` |
| PB180 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | m==NULL (verify-only path) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB181 | crypto_aead_xchacha20poly1305_ietf_decrypt | clen<ABYTES=16 (early -1) | [x] `aead_combined_roundtrip` |
| PB182 | crypto_aead_xchacha20poly1305_ietf_decrypt_detached | verify fail (-1) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB183 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=0, adlen=0 (no absorb, no enc blocks) | [x] `aead_combined_roundtrip` |
| PB184 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=1, adlen=1 (only partial mlen%32, adlen%32) | [x] `aead_combined_roundtrip` |
| PB185 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=31, adlen=31 (partial <RATE=32) | [x] `aead_combined_roundtrip` |
| PB186 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=32, adlen=32 (exactly one RATE absorb/enc) | [x] `aead_combined_roundtrip` |
| PB187 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=33, adlen=33 (RATE + partial) | [x] `aead_combined_roundtrip` |
| PB188 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=64, adlen=64 (2*RATE absorb2 path) | [x] `aead_combined_roundtrip` |
| PB189 | crypto_aead_aegis128l_encrypt / _decrypt | mlen=65, adlen=17 (absorb2 + remainder mix) | [x] `aead_combined_roundtrip` |
| PB190 | crypto_aead_aegis128l_encrypt / _decrypt | mlen large, adlen large | [x] `aead_combined_roundtrip` |
| PB191 | crypto_aead_aegis128l_encrypt_detached / _decrypt_detached | maclen=32 (default ABYTES), verify pass | [x] `aead_detached_roundtrip_and_verify_only` |
| PB192 | crypto_aead_aegis128l_encrypt_detached / _decrypt_detached | maclen=16 variant (aegis128l_mac 16-byte tag) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB193 | crypto_aead_aegis128l_decrypt_detached | m==NULL (verify-only decrypt path) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB194 | crypto_aead_aegis128l_decrypt | clen<ABYTES=32 (early -1) | [x] `aead_combined_roundtrip` |
| PB195 | crypto_aead_aegis128l_decrypt_detached | verify fail (-1) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB196 | crypto_aead_aegis256_encrypt / _decrypt | mlen=0, adlen=0 | [x] `aead_combined_roundtrip` |
| PB197 | crypto_aead_aegis256_encrypt / _decrypt | mlen=1, adlen=1 (partial <RATE=16) | [x] `aead_combined_roundtrip` |
| PB198 | crypto_aead_aegis256_encrypt / _decrypt | mlen=15, adlen=15 (partial mlen%16) | [x] `aead_combined_roundtrip` |
| PB199 | crypto_aead_aegis256_encrypt / _decrypt | mlen=16, adlen=16 (exactly one RATE=16) | [x] `aead_combined_roundtrip` |
| PB200 | crypto_aead_aegis256_encrypt / _decrypt | mlen=17, adlen=17 (RATE + partial) | [x] `aead_combined_roundtrip` |
| PB201 | crypto_aead_aegis256_encrypt / _decrypt | mlen=32, adlen=32 (2*RATE absorb2 path) | [x] `aead_combined_roundtrip` |
| PB202 | crypto_aead_aegis256_encrypt / _decrypt | mlen=33, adlen=17 (absorb2 + remainder) | [x] `aead_combined_roundtrip` |
| PB203 | crypto_aead_aegis256_encrypt / _decrypt | mlen=63,64,65; adlen=1 | [x] `aead_combined_roundtrip` |
| PB204 | crypto_aead_aegis256_encrypt / _decrypt | mlen large, adlen large | [x] `aead_combined_roundtrip` |
| PB205 | crypto_aead_aegis256_encrypt_detached / _decrypt_detached | maclen=32 (default ABYTES), verify pass | [x] `aead_detached_roundtrip_and_verify_only` |
| PB206 | crypto_aead_aegis256_encrypt_detached / _decrypt_detached | maclen=16 variant (aegis256_mac 16-byte tag) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB207 | crypto_aead_aegis256_decrypt_detached | m==NULL (verify-only decrypt path) | [x] `aead_detached_roundtrip_and_verify_only` |
| PB208 | crypto_aead_aegis256_decrypt | clen<ABYTES=32 (early -1) | [x] `aead_combined_roundtrip` |
| PB209 | crypto_aead_aegis256_decrypt_detached | verify fail (-1) | [x] `aead_detached_roundtrip_and_verify_only` |

## Configuration-Surface Table (valid-input mirror)

libsodium 1.0.23, C built with NO HAVE_* macros (portable fallback selected everywhere).
Rows derived only from branches the C source actually takes. Last column left as `[ ]`.

## crypto_core/ed25519 — core_ed25519.c

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC1 | crypto_core_ed25519_is_valid_point | valid canonical point, on curve, not small order, on main subgroup → returns 1 | [x] `is_valid_point_edges` |
| PC2 | crypto_core_ed25519_add | p,q both canonical on-curve points (frombytes ok, is_on_curve!=0) → r = p+q, ret 0 | [x] `point_add_sub` |
| PC3 | crypto_core_ed25519_sub | p,q both canonical on-curve points → r = p-q, ret 0 | [x] `point_add_sub` |
| PC4 | crypto_core_ed25519_random | no options; fills p via randombytes→ge25519_from_uniform (always valid point) | [x] `scalarmult_ed25519` |
| PC5 | crypto_core_ed25519_scalar_random | no options; rejection loop until canonical (top byte &0x1f) and nonzero | [x] `scalar_random_accepted_cross_library` |
| PC6 | crypto_core_ed25519_scalar_invert | s a nonzero canonical scalar → recip, ret 0 (returns -is_zero(s)) | [x] `scalar_ops` |
| PC7 | crypto_core_ed25519_scalar_negate | s canonical scalar → neg = L - s (mod L) via sodium_sub + sc25519_reduce | [x] `scalar_ops` |
| PC8 | crypto_core_ed25519_scalar_complement | s canonical scalar → comp = (1 - s) mod L (t_[0]++ branch) | [x] `scalar_ops` |
| PC9 | crypto_core_ed25519_scalar_add | x,y scalars zero-extended to NONREDUCED, sodium_add then reduce | [x] `scalar_ops` |
| PC10 | crypto_core_ed25519_scalar_sub | x,y scalars → z = x + negate(y) mod L | [x] `scalar_ops` |
| PC11 | crypto_core_ed25519_scalar_mul | x,y scalars → z = x*y mod L (sc25519_mul, no reduce guard) | [x] `scalar_ops` |
| PC12 | crypto_core_ed25519_scalar_reduce | 64-byte non-reduced scalar → 32-byte reduced (sc25519_reduce) | [x] `scalar_ops` |

## crypto_core/ristretto255 — core_ristretto255.c

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC13 | crypto_core_ristretto255_is_valid_point | canonical ristretto encoding (ristretto255_frombytes ok) → returns 1 | [x] `is_valid_point_edges` |
| PC14 | crypto_core_ristretto255_add | p,q valid ristretto encodings → r = p+q, ret 0 | [x] `point_add_sub` |
| PC15 | crypto_core_ristretto255_sub | p,q valid ristretto encodings → r = p-q, ret 0 | [x] `point_add_sub` |
| PC16 | crypto_core_ristretto255_from_hash | 64-byte hash input → p (always ret 0) | [x] `from_hash_ristretto` |
| PC17 | crypto_core_ristretto255_random | no options; randombytes→from_hash (always valid) | [x] `scalarmult_ristretto255` |
| PC18 | crypto_core_ristretto255_scalar_random | delegates to ed25519 scalar_random rejection loop | [x] `scalar_random_accepted_cross_library` |
| PC19 | crypto_core_ristretto255_scalar_invert | nonzero canonical scalar → recip, ret 0 | [x] `scalar_ops` |
| PC20 | crypto_core_ristretto255_scalar_negate | canonical scalar → L - s mod L | [x] `scalar_ops` |
| PC21 | crypto_core_ristretto255_scalar_complement | canonical scalar → (1 - s) mod L | [x] `scalar_ops` |
| PC22 | crypto_core_ristretto255_scalar_add | x,y scalars → (x+y) mod L | [x] `scalar_ops` |
| PC23 | crypto_core_ristretto255_scalar_sub | x,y scalars → (x-y) mod L | [x] `scalar_ops` |
| PC24 | crypto_core_ristretto255_scalar_mul | x,y scalars → x*y mod L (sc25519_mul) | [x] `scalar_ops` |
| PC25 | crypto_core_ristretto255_scalar_reduce | 64-byte non-reduced → 32-byte reduced | [x] `scalar_ops` |

## crypto_core/ed25519 h2c — core_h2c.c (from_string / from_string_nu)

Branches: hash_alg switch (SHA256=case, SHA512=case); ctx_len<=0xff vs >0xff (H2C-OVERSIZE-DST- prehash); msg_len 0 vs long; n=2 (from_string, expand+add) vs n=1 (from_string_nu).

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC26 | crypto_core_ed25519_from_string | hash_alg=SHA256, ctx empty (len 0, ≤0xff), msg empty | [x] `from_string_all_variants` |
| PC27 | crypto_core_ed25519_from_string | hash_alg=SHA256, ctx non-empty (≤0xff), msg long | [x] `from_string_all_variants` |
| PC28 | crypto_core_ed25519_from_string | hash_alg=SHA256, ctx >0xff (oversize-DST prehash branch), msg long | [x] `from_string_all_variants` |
| PC29 | crypto_core_ed25519_from_string | hash_alg=SHA512, ctx empty, msg empty | [x] `from_string_all_variants` |
| PC30 | crypto_core_ed25519_from_string | hash_alg=SHA512, ctx non-empty (≤0xff), msg long | [x] `from_string_all_variants` |
| PC31 | crypto_core_ed25519_from_string | hash_alg=SHA512, ctx >0xff (oversize-DST prehash branch), msg long | [x] `from_string_all_variants` |
| PC32 | crypto_core_ed25519_from_string_nu | hash_alg=SHA256, n=1 single-point map, ctx non-empty, msg long | [x] `from_string_all_variants` |
| PC33 | crypto_core_ed25519_from_string_nu | hash_alg=SHA512, n=1 single-point map, ctx empty, msg empty | [x] `from_string_all_variants` |
| PC34 | crypto_core_ed25519_scalar_from_string | hash_alg=SHA256, ctx empty, msg empty (48-byte→reduce) | [x] `from_string_all_variants` |
| PC35 | crypto_core_ed25519_scalar_from_string | hash_alg=SHA512, ctx >0xff (oversize-DST), msg long | [x] `from_string_all_variants` |

## crypto_core/ristretto255 h2c — core_ristretto255.c (from_string)

Branches: hash_alg SHA256/SHA512; ctx ≤0xff vs >0xff; msg empty/long. Single element (n=1) always.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC36 | crypto_core_ristretto255_from_string | hash_alg=SHA256, ctx empty, msg empty | [x] `from_string_all_variants` |
| PC37 | crypto_core_ristretto255_from_string | hash_alg=SHA256, ctx non-empty (≤0xff), msg long | [x] `from_string_all_variants` |
| PC38 | crypto_core_ristretto255_from_string | hash_alg=SHA256, ctx >0xff (oversize-DST branch), msg long | [x] `from_string_all_variants` |
| PC39 | crypto_core_ristretto255_from_string | hash_alg=SHA512, ctx empty, msg empty | [x] `from_string_all_variants` |
| PC40 | crypto_core_ristretto255_from_string | hash_alg=SHA512, ctx non-empty (≤0xff), msg long | [x] `from_string_all_variants` |
| PC41 | crypto_core_ristretto255_from_string | hash_alg=SHA512, ctx >0xff (oversize-DST branch), msg long | [x] `from_string_all_variants` |
| PC42 | crypto_core_ristretto255_scalar_from_string | hash_alg=SHA256, ctx empty, msg empty | [x] `from_string_all_variants` |
| PC43 | crypto_core_ristretto255_scalar_from_string | hash_alg=SHA512, ctx >0xff (oversize-DST), msg long | [x] `from_string_all_variants` |

## crypto_scalarmult

Branches: curve25519 always clamps (t[0]&=248, t[31]&=127|=64); ed25519 clamp vs noclamp (t[31]&=127 always); base vs two-arg; small-order/is_inf/zero rejection; ristretto frombytes.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC44 | crypto_scalarmult_curve25519 | random scalar, valid canonical point p (not small order) → q, ret 0 | [x] `scalarmult_curve25519` |
| PC45 | crypto_scalarmult_curve25519 | scalar=1 (post-clamp forced high/low bits), valid point | [x] `scalarmult_curve25519` |
| PC46 | crypto_scalarmult_curve25519 | scalar already clamped high bit set / low 3 bits clear (clamp idempotent) | [x] `scalarmult_curve25519` |
| PC47 | crypto_scalarmult_curve25519_base | random scalar → q = n·basepoint (edwards_to_montgomery), ret 0 | [x] `scalarmult_curve25519` |
| PC48 | crypto_scalarmult_curve25519_base | scalar=1 clamped, base multiply | [x] `scalarmult_curve25519` |
| PC49 | crypto_scalarmult_ed25519 | clamp=1, random scalar, valid canonical main-subgroup point → ret 0 | [x] `scalarmult_ed25519` |
| PC50 | crypto_scalarmult_ed25519 | clamp=1, scalar l-1 (nonzero, non-inf result), valid point | [x] `scalarmult_ed25519` |
| PC51 | crypto_scalarmult_ed25519_noclamp | clamp=0, unclamped scalar (high/low bits untouched except t[31]&=127), valid point | [x] `scalarmult_ed25519` |
| PC52 | crypto_scalarmult_ed25519_noclamp | clamp=0, scalar=1 → q = P, non-inf | [x] `scalarmult_ed25519` |
| PC53 | crypto_scalarmult_ed25519_base | clamp=1, random scalar → n·B, ret 0 | [x] `scalarmult_ed25519` |
| PC54 | crypto_scalarmult_ed25519_base | clamp=1, scalar l-1 → non-inf result | [x] `scalarmult_ed25519` |
| PC55 | crypto_scalarmult_ed25519_base_noclamp | clamp=0, unclamped scalar → n·B, ret 0 | [x] `scalarmult_ed25519` |
| PC56 | crypto_scalarmult_ed25519_base_noclamp | clamp=0, scalar=1 → q = B | [x] `scalarmult_ed25519` |
| PC57 | crypto_scalarmult_ristretto255 | random scalar, valid ristretto point (frombytes ok), nonzero result → ret 0 | [x] `scalarmult_ristretto255` |
| PC58 | crypto_scalarmult_ristretto255 | scalar l-1, valid point → nonzero result | [x] `scalarmult_ristretto255` |
| PC59 | crypto_scalarmult_ristretto255_base | random scalar → n·B nonzero, ret 0 | [x] `scalarmult_ristretto255` |
| PC60 | crypto_scalarmult_ristretto255_base | scalar=1 → q = B | [x] `scalarmult_ristretto255` |

## crypto_sign/ed25519

Branches: keypair (random seed) vs seed_keypair (fixed seed); combined crypto_sign (prepends sig) vs detached; open vs verify_detached; ph streaming (prehashed=1, DOM2PREFIX) with update chunkings; sig[63] canonicality S check; message lengths 0/1/63/64/65/large.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC61 | crypto_sign_ed25519_keypair | no options; randombytes seed → pk/sk (sk = seed‖pk) | [x] `sign_keypair_cross_check` |
| PC62 | crypto_sign_ed25519_seed_keypair | fixed 32-byte seed → deterministic pk/sk | [x] `sign_seed_keypair_byte_exact` |
| PC63 | crypto_sign_ed25519 / _open | mlen=0, roundtrip sign+open, smlen=64 | [x] `sign_combined_and_open` |
| PC64 | crypto_sign_ed25519 / _open | mlen=1 | [x] `sign_combined_and_open` |
| PC65 | crypto_sign_ed25519 / _open | mlen=63 | [x] `sign_combined_and_open` |
| PC66 | crypto_sign_ed25519 / _open | mlen=64 | [x] `sign_combined_and_open` |
| PC67 | crypto_sign_ed25519 / _open | mlen=65 | [x] `sign_combined_and_open` |
| PC68 | crypto_sign_ed25519 / _open | mlen large | [x] `sign_combined_and_open` |
| PC69 | crypto_sign_ed25519_detached / _verify_detached | deterministic (no ED25519_NONDETERMINISTIC), mlen=0 | [x] `sign_detached_and_verify` |
| PC70 | crypto_sign_ed25519_detached / _verify_detached | mlen=64, valid sig (sig[63]&240==0, canonical) | [x] `sign_detached_and_verify` |
| PC71 | crypto_sign_ed25519_detached / _verify_detached | mlen large | [x] `sign_detached_and_verify` |
| PC72 | crypto_sign_ed25519ph_init/update/final_create | prehashed=1, single update chunk, mlen=64 | [x] `sign_ph_streaming` |
| PC73 | crypto_sign_ed25519ph_init/update/final_create | prehashed=1, multi update chunks (e.g. 1+63) summing same msg | [x] `sign_ph_streaming` |
| PC74 | crypto_sign_ed25519ph_init/update/final_create | prehashed=1, update with mlen=0 (empty), then final | [x] `sign_ph_streaming` |
| PC75 | crypto_sign_ed25519ph_init/update/final_verify | prehashed=1, multi update chunks (e.g. 32+32+1), valid sig | [x] `sign_ph_streaming` |
| PC76 | crypto_sign_ed25519ph_init/update/final_verify | prehashed=1, single chunk large msg, valid sig | [x] `sign_ph_streaming` |
| PC77 | crypto_sign_ed25519_sk_to_seed | sk (seed‖pk) → first 32 bytes (seed) | [x] `sign_ph_streaming` |
| PC78 | crypto_sign_ed25519_sk_to_pk | sk → last 32 bytes (pk) | [x] `sign_ph_streaming` |
| PC79 | crypto_sign_ed25519_pk_to_curve25519 | valid ed pk on main subgroup, not small order → curve25519 pk | [x] `sign_ph_streaming` |
| PC80 | crypto_sign_ed25519_sk_to_curve25519 | ed sk → sha512(seed) clamped → curve25519 sk | [x] `sign_ph_streaming` |

## crypto_box (curve25519xsalsa20poly1305 and curve25519xchacha20poly1305)

Branches: raw ZEROBYTES API (crypto_box/_open) vs _easy (MAC prepend, mlen guard) vs _detached; beforenm/afternm split; seal (ephemeral keypair + blake2b nonce) / seal_open; msg lengths 0/1/31/32/33/large. xchacha variant is a distinct primitive with the same shape set.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC81 | crypto_box_curve25519xsalsa20poly1305 / _open | raw ZEROBYTES API, mlen=0 | [x] `box_keypairs` |
| PC82 | crypto_box_curve25519xsalsa20poly1305 / _open | raw ZEROBYTES API, mlen=32 | [x] `box_keypairs` |
| PC83 | crypto_box_curve25519xsalsa20poly1305 / _open | raw ZEROBYTES API, mlen large | [x] `box_keypairs` |
| PC84 | crypto_box_curve25519xsalsa20poly1305_beforenm | valid pk/sk → shared k (hsalsa20 of X25519) | [x] `box_beforenm_and_afternm` |
| PC85 | crypto_box_curve25519xsalsa20poly1305_afternm / _open_afternm | precomputed k, mlen=0 | [x] `box_beforenm_and_afternm` |
| PC86 | crypto_box_curve25519xsalsa20poly1305_afternm / _open_afternm | precomputed k, mlen=33 | [x] `box_beforenm_and_afternm` |
| PC87 | crypto_box_easy / crypto_box_open_easy | mlen=0 (MAC-prepend easy API) | [x] `box_easy_detached_raw` |
| PC88 | crypto_box_easy / crypto_box_open_easy | mlen=1 | [x] `box_easy_detached_raw` |
| PC89 | crypto_box_easy / crypto_box_open_easy | mlen=31 | [x] `box_easy_detached_raw` |
| PC90 | crypto_box_easy / crypto_box_open_easy | mlen=32 | [x] `box_easy_detached_raw` |
| PC91 | crypto_box_easy / crypto_box_open_easy | mlen=33 | [x] `box_easy_detached_raw` |
| PC92 | crypto_box_easy / crypto_box_open_easy | mlen large | [x] `box_easy_detached_raw` |
| PC93 | crypto_box_detached / crypto_box_open_detached | mlen=0, separate MAC buffer | [x] `box_easy_detached_raw` |
| PC94 | crypto_box_detached / crypto_box_open_detached | mlen large | [x] `box_easy_detached_raw` |
| PC95 | crypto_box_easy_afternm / crypto_box_open_easy_afternm | precomputed k, mlen=32 | [x] `box_seal_and_seal_open` |
| PC96 | crypto_box_detached_afternm / crypto_box_open_detached_afternm | precomputed k, mlen large | [x] `box_beforenm_and_afternm` |
| PC97 | crypto_box_seal / crypto_box_seal_open | ephemeral keypair + blake2b(epk‖pk) nonce, mlen=0 | [x] `box_seal_and_seal_open` |
| PC98 | crypto_box_seal / crypto_box_seal_open | mlen large | [x] `box_seal_and_seal_open` |
| PC99 | crypto_box_curve25519xchacha20poly1305 raw / _open | raw ZEROBYTES-style, mlen=32 (distinct primitive: hchacha20) | [x] `N/A` |
| PC100 | crypto_box_curve25519xchacha20poly1305_beforenm | valid pk/sk → shared k (hchacha20 of X25519) | [x] `box_beforenm_and_afternm` |
| PC101 | crypto_box_curve25519xchacha20poly1305_easy / _open_easy | mlen=0 | [x] `box_easy_detached_raw` |
| PC102 | crypto_box_curve25519xchacha20poly1305_easy / _open_easy | mlen=32 | [x] `box_easy_detached_raw` |
| PC103 | crypto_box_curve25519xchacha20poly1305_easy / _open_easy | mlen=33 | [x] `box_easy_detached_raw` |
| PC104 | crypto_box_curve25519xchacha20poly1305_easy / _open_easy | mlen large | [x] `box_easy_detached_raw` |
| PC105 | crypto_box_curve25519xchacha20poly1305_detached / _open_detached | mlen=1, separate MAC | [x] `box_easy_detached_raw` |
| PC106 | crypto_box_curve25519xchacha20poly1305_easy_afternm / _open_easy_afternm | precomputed k, mlen=32 | [x] `box_beforenm_and_afternm` |
| PC107 | crypto_box_curve25519xchacha20poly1305_detached_afternm / _open_detached_afternm | precomputed k, mlen large | [x] `box_beforenm_and_afternm` |
| PC108 | crypto_box_curve25519xchacha20poly1305_seal / _seal_open | ephemeral keypair + nonce, mlen=32 | [x] `box_seal_and_seal_open` |

## crypto_kx

Branches: keypair (random) vs seed_keypair (blake2b of seed); client vs server session keys; rx==NULL (aliased to tx), tx==NULL (aliased to rx), both non-NULL. Client/server swap rx/tx ordering in the key split.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC109 | crypto_kx_keypair | no options; randombytes sk → scalarmult_base pk | [x] `kx_constants_and_seed_keypair` |
| PC110 | crypto_kx_seed_keypair | fixed seed → blake2b sk → scalarmult_base pk (deterministic) | [x] `kx_constants_and_seed_keypair` |
| PC111 | crypto_kx_client_session_keys | rx and tx both non-NULL, valid keys | [x] `kx_session_keys` |
| PC112 | crypto_kx_client_session_keys | rx NULL (aliased to tx), tx non-NULL | [x] `kx_session_keys` |
| PC113 | crypto_kx_client_session_keys | tx NULL (aliased to rx), rx non-NULL | [x] `kx_session_keys` |
| PC114 | crypto_kx_server_session_keys | rx and tx both non-NULL, valid keys (rx/tx split swapped vs client) | [x] `kx_session_keys` |
| PC115 | crypto_kx_server_session_keys | rx NULL (aliased to tx), tx non-NULL | [x] `kx_session_keys` |
| PC116 | crypto_kx_server_session_keys | tx NULL (aliased to rx), rx non-NULL | [x] `kx_session_keys` |

## crypto_kdf

Branches: blake2b derive_from_key (subkey_len in [BYTES_MIN..BYTES_MAX] valid range, subkey_id via STORE64_LE into salt); hkdf sha256 & sha512 extract (one-shot) vs extract_init/update/final (streaming); expand (out_len loop full-block + remainder branch); keygen; salt/ikm/ctx len 0 vs >0; out_len 1/32/BYTES_MAX.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC117 | crypto_kdf_blake2b_derive_from_key | subkey_len=BYTES_MIN, subkey_id=0 | [x] `kdf_derive_from_key_full_matrix` |
| PC118 | crypto_kdf_blake2b_derive_from_key | subkey_len=32, subkey_id=1 | [x] `kdf_derive_from_key_full_matrix` |
| PC119 | crypto_kdf_blake2b_derive_from_key | subkey_len=BYTES_MAX, subkey_id=UINT64_MAX | [x] `kdf_derive_from_key_full_matrix` |
| PC120 | crypto_kdf_hkdf_sha256_extract | salt_len=0, ikm_len=0 | [x] `hkdf_extract_one_shot` |
| PC121 | crypto_kdf_hkdf_sha256_extract | salt_len>0, ikm_len>0 | [x] `hkdf_extract_one_shot` |
| PC122 | crypto_kdf_hkdf_sha256_extract_init/update/final | streaming, single update, salt_len>0, ikm_len>0 | [x] `hkdf_extract_one_shot` |
| PC123 | crypto_kdf_hkdf_sha256_extract_init/update/final | streaming, multi update chunks, salt_len=0 | [x] `hkdf_extract_one_shot` |
| PC124 | crypto_kdf_hkdf_sha256_expand | ctx_len=0, out_len=1 (remainder-only branch, i==0) | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC125 | crypto_kdf_hkdf_sha256_expand | ctx_len>0, out_len=32 (single full block, exact) | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC126 | crypto_kdf_hkdf_sha256_expand | ctx_len>0, out_len=BYTES_MAX (full-block loop + remainder) | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC127 | crypto_kdf_hkdf_sha256_keygen | no options; randombytes prk | [x] `every_keygen_writes_exactly_keybytes` |
| PC128 | crypto_kdf_hkdf_sha512_extract | salt_len=0, ikm_len=0 | [x] `hkdf_extract_one_shot` |
| PC129 | crypto_kdf_hkdf_sha512_extract | salt_len>0, ikm_len>0 | [x] `hkdf_extract_one_shot` |
| PC130 | crypto_kdf_hkdf_sha512_extract_init/update/final | streaming, single update, salt_len>0, ikm_len>0 | [x] `hkdf_extract_one_shot` |
| PC131 | crypto_kdf_hkdf_sha512_extract_init/update/final | streaming, multi update chunks, salt_len=0 | [x] `hkdf_extract_one_shot` |
| PC132 | crypto_kdf_hkdf_sha512_expand | ctx_len=0, out_len=1 (remainder-only) | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC133 | crypto_kdf_hkdf_sha512_expand | ctx_len>0, out_len=32 | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC134 | crypto_kdf_hkdf_sha512_expand | ctx_len>0, out_len=BYTES_MAX (full-block loop + remainder) | [x] `hkdf_expand_all_lengths_and_range_check` |
| PC135 | crypto_kdf_hkdf_sha512_keygen | no options; randombytes prk | [x] `every_keygen_writes_exactly_keybytes` |

## crypto_kem (mlkem768 and xwing)

Branches: keypair (random) vs seed_keypair (deterministic); enc (random coins) vs enc_deterministic (exported for both; seed-driven, reproducible); dec (implicit rejection, always returns ss). Enumerate deterministic seed-driven shapes for reproducibility.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC136 | crypto_kem_mlkem768_seed_keypair | fixed 64-byte seed → deterministic pk/sk | [x] `kem_seed_keypair_deterministic` |
| PC137 | crypto_kem_mlkem768_keypair | random keypair (nondeterministic) | [x] `kem_keypair_enc_dec_cross_roundtrip` |
| PC138 | crypto_kem_mlkem768_enc_deterministic | fixed 32-byte enc seed + pk from PC136 → reproducible ct/ss | [x] `kem_enc_deterministic_bytes_exact` |
| PC139 | crypto_kem_mlkem768_enc | random coins, valid pk | [x] `kem_enc_non_canonical_public_key` |
| PC140 | crypto_kem_mlkem768_dec | valid ct/sk from PC138 → matching ss (implicit-rejection success path) | [x] `kem_dec_corrupted_ciphertext` |
| PC141 | crypto_kem_xwing_seed_keypair | fixed 32-byte seed → expand (shake256) → deterministic pk/sk (sk = seed) | [x] `kem_seed_keypair_deterministic` |
| PC142 | crypto_kem_xwing_keypair | random keypair | [x] `kem_keypair_enc_dec_cross_roundtrip` |
| PC143 | crypto_kem_xwing_enc_deterministic | fixed 64-byte seed (mlkem seed‖x25519 esk) + pk from PC141 → reproducible ct/ss | [x] `kem_enc_deterministic_bytes_exact` |
| PC144 | crypto_kem_xwing_enc | random 64-byte seed, valid pk | [x] `kem_enc_non_canonical_public_key` |
| PC145 | crypto_kem_xwing_dec | valid ct/sk from PC143 → matching ss (combiner of mlkem+x25519) | [x] `kem_dec_corrupted_ciphertext` |

## crypto_ipcrypt (soft AES fallback)

Branches: deterministic single-block encrypt/decrypt; nd (8-byte tweak prepended); ndx (16-byte tweak, XEX, key-equal collision d==0 fallback branch); pfx (prefix-preserving, is_ipv4_mapped→prefix_start=96 vs 0, key-collision d==0 branch). Distinct 16-byte IP shapes the code distinguishes: IPv4-mapped (::ffff:a.b.c.d) vs native IPv6, all-zero, all-ones.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PC146 | crypto_ipcrypt_encrypt / crypto_ipcrypt_decrypt | native IPv6 16-byte input, roundtrip | [x] `ipcrypt_encrypt_decrypt_roundtrip` |
| PC147 | crypto_ipcrypt_encrypt / crypto_ipcrypt_decrypt | IPv4-mapped 16-byte input (::ffff:a.b.c.d), roundtrip | [x] `ipcrypt_encrypt_decrypt_roundtrip` |
| PC148 | crypto_ipcrypt_encrypt / crypto_ipcrypt_decrypt | all-zero 16-byte input | [x] `ipcrypt_encrypt_decrypt_roundtrip` |
| PC149 | crypto_ipcrypt_encrypt / crypto_ipcrypt_decrypt | all-ones 16-byte input | [x] `ipcrypt_encrypt_decrypt_roundtrip` |
| PC150 | crypto_ipcrypt_nd_encrypt / crypto_ipcrypt_nd_decrypt | 8-byte tweak, native IPv6 input, roundtrip | [x] `ipcrypt_nd_encrypt_decrypt_roundtrip` |
| PC151 | crypto_ipcrypt_nd_encrypt / crypto_ipcrypt_nd_decrypt | 8-byte tweak, IPv4-mapped input | [x] `ipcrypt_nd_encrypt_decrypt_roundtrip` |
| PC152 | crypto_ipcrypt_nd_encrypt / crypto_ipcrypt_nd_decrypt | 8-byte tweak, all-zero input | [x] `ipcrypt_nd_encrypt_decrypt_roundtrip` |
| PC153 | crypto_ipcrypt_nd_encrypt / crypto_ipcrypt_nd_decrypt | 8-byte tweak, all-ones input | [x] `ipcrypt_nd_encrypt_decrypt_roundtrip` |
| PC154 | crypto_ipcrypt_ndx_encrypt / crypto_ipcrypt_ndx_decrypt | 16-byte tweak, distinct k1/k2 (d!=0 path), native IPv6 | [x] `ipcrypt_ndx_encrypt_decrypt_roundtrip` |
| PC155 | crypto_ipcrypt_ndx_encrypt / crypto_ipcrypt_ndx_decrypt | 16-byte tweak, equal-half key (d==0 fallback: rkeys from k^0x5a) | [x] `ipcrypt_ndx_encrypt_decrypt_roundtrip` |
| PC156 | crypto_ipcrypt_ndx_encrypt / crypto_ipcrypt_ndx_decrypt | 16-byte tweak, IPv4-mapped input | [x] `ipcrypt_ndx_encrypt_decrypt_roundtrip` |
| PC157 | crypto_ipcrypt_ndx_encrypt / crypto_ipcrypt_ndx_decrypt | 16-byte tweak, all-zero input | [x] `ipcrypt_ndx_encrypt_decrypt_roundtrip` |
| PC158 | crypto_ipcrypt_pfx_encrypt / crypto_ipcrypt_pfx_decrypt | native IPv6 (prefix_start=0, full 128-bit prefix loop), distinct k1/k2 | [x] `ipcrypt_pfx_encrypt_decrypt_roundtrip` |
| PC159 | crypto_ipcrypt_pfx_encrypt / crypto_ipcrypt_pfx_decrypt | IPv4-mapped (prefix_start=96, encrypted[10..11]=0xff seed, 32-bit loop) | [x] `ipcrypt_pfx_encrypt_decrypt_roundtrip` |
| PC160 | crypto_ipcrypt_pfx_encrypt / crypto_ipcrypt_pfx_decrypt | equal-half key (d==0 fallback: k2keys from k^0x5a) | [x] `ipcrypt_pfx_encrypt_decrypt_roundtrip` |
| PC161 | crypto_ipcrypt_pfx_encrypt / crypto_ipcrypt_pfx_decrypt | all-zero input (prefix-preserving identity of leading bits) | [x] `ipcrypt_pfx_encrypt_decrypt_roundtrip` |
| PC162 | crypto_ipcrypt_pfx_encrypt / crypto_ipcrypt_pfx_decrypt | all-ones input | [x] `ipcrypt_pfx_encrypt_decrypt_roundtrip` |

## Configuration-Surface Table — crypto_pwhash & randombytes (valid-input mirror)

Derived from c_src/libsodium with NO HAVE_* macros: scrypt uses escrypt_kdf_nosse,
argon2 uses argon2_fill_segment_ref / argon2-fill-block-ref.c. In all libsodium
argon2 entry points parallelism/lanes is hard-wired to 1, so memory_blocks =
max(m_cost, 2*SYNC_POINTS*lanes) = max(memlimit/1024, 8), segment_length =
memory_blocks/4, lane_length = segment_length*4 (SYNC_POINTS=4). t_cost = opslimit.
Cheap-pair rationale: m_cost 8..16 blocks (8..16 KiB), passes 1..3 — sub-millisecond.

Key branch pivots the C actually distinguishes:
- segment_length == 2 (memlimit 8192 -> m_cost 8) vs segment_length > 2 (memlimit 16384 -> m_cost 16, segment_length 4).
- pass == 0 only (opslimit 1, argon2id) vs pass > 0 present (opslimit >= 2; argon2i min opslimit is 3).
- argon2-fill-block-ref data_independent_addressing: Argon2_id switches to data-DEPENDENT when (pass != 0 || slice >= SYNC_POINTS/2 == 2); Argon2_i is always data-independent. slice==0/pass==0 forces starting_index=2 and ref_lane=self.

## crypto_pwhash (generic dispatcher, ALG switch -> argon2i / argon2id)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD1 | crypto_pwhash | alg=ARGON2ID13, outlen=16 (BYTES_MIN), passwdlen=1, salt=16B, opslimit=1 (OPSLIMIT_MIN id), memlimit=8192 (MEMLIMIT_MIN, m_cost=8, seg=2, single pass) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD2 | crypto_pwhash | alg=ARGON2ID13, outlen=32, passwdlen=64, salt=16B, opslimit=2 (pass>0 branch), memlimit=8192 (seg=2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD3 | crypto_pwhash | alg=ARGON2ID13, outlen=64, passwdlen=0 (empty pwd, valid: PASSWD_MIN=0), salt=16B, opslimit=1, memlimit=16384 (m_cost=16, seg=4 > 2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD4 | crypto_pwhash | alg=ARGON2I13, outlen=16 (BYTES_MIN), passwdlen=1, salt=16B, opslimit=3 (OPSLIMIT_MIN i, pass>0 present), memlimit=8192 (seg=2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD5 | crypto_pwhash | alg=ARGON2I13, outlen=32, passwdlen=1024 (> 1024B block), salt=16B, opslimit=3, memlimit=16384 (seg=4 > 2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD6 | crypto_pwhash | alg=ARGON2I13, outlen=64, passwdlen=64, salt=16B, opslimit=3, memlimit=8192 | [x] `crypto_pwhash_generic_happy_matrix` |

## crypto_pwhash_argon2i (direct entry point)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD7 | crypto_pwhash_argon2i | alg=ARGON2I13, outlen=16 (BYTES_MIN), passwdlen=0, salt=16B, opslimit=3, memlimit=8192 (m_cost=8, seg=2; all 4 slices data-independent, pass 0 slice 0 -> starting_index=2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD8 | crypto_pwhash_argon2i | alg=ARGON2I13, outlen=32, passwdlen=1, salt=16B, opslimit=3, memlimit=16384 (seg=4 > 2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD9 | crypto_pwhash_argon2i | alg=ARGON2I13, outlen=64, passwdlen=1100 (> block), salt=16B, opslimit=4 (>MIN), memlimit=8192 | [x] `crypto_pwhash_generic_happy_matrix` |

## crypto_pwhash_argon2id (direct entry point)

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD10 | crypto_pwhash_argon2id | alg=ARGON2ID13, outlen=16 (BYTES_MIN), passwdlen=1, salt=16B, opslimit=1 (OPSLIMIT_MIN, single pass 0), memlimit=8192 (seg=2): exercises slice<2 data-independent AND slice>=2 data-dependent in same pass | [x] `crypto_pwhash_generic_happy_matrix` |
| PD11 | crypto_pwhash_argon2id | alg=ARGON2ID13, outlen=32, passwdlen=64, salt=16B, opslimit=2 (pass 1 -> fully data-dependent, fill_block_with_xor), memlimit=8192 (seg=2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD12 | crypto_pwhash_argon2id | alg=ARGON2ID13, outlen=64, passwdlen=0, salt=16B, opslimit=1, memlimit=16384 (seg=4 > 2) | [x] `crypto_pwhash_generic_happy_matrix` |
| PD13 | crypto_pwhash_argon2id | alg=ARGON2ID13, outlen=32, passwdlen=1100 (> block), salt=16B, opslimit=3, memlimit=16384 (seg=4, pass>0 AND seg>2 combined) | [x] `crypto_pwhash_generic_happy_matrix` |

## crypto_pwhash_str / _str_alg (encoded output, random salt)

Note: these embed a random salt so the encoded string is non-deterministic; the
differential comparison is the encoded SHAPE (prefix, $v=19, m=,t=,p=1, b64 salt/out)
and the round-trip through argon2-encoding.c, not the exact bytes.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD14 | crypto_pwhash_str | (-> argon2id_str) passwdlen=1, opslimit=1, memlimit=8192; expect prefix "$argon2id$v=19$m=8,t=1,p=1$..." | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD15 | crypto_pwhash_str | passwdlen=64, opslimit=2, memlimit=16384; expect "$argon2id$v=19$m=16,t=2,p=1$..." | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD16 | crypto_pwhash_str_alg | alg=ARGON2ID13, passwdlen=0, opslimit=1, memlimit=8192; "$argon2id$v=19$m=8,t=1,p=1$..." | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD17 | crypto_pwhash_str_alg | alg=ARGON2I13, passwdlen=1, opslimit=3, memlimit=8192; "$argon2i$v=19$m=8,t=3,p=1$..." | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD18 | crypto_pwhash_str_alg | alg=ARGON2I13, passwdlen=64, opslimit=3, memlimit=16384; "$argon2i$v=19$m=16,t=3,p=1$..." | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD19 | crypto_pwhash_argon2i_str | passwdlen=1, opslimit=3, memlimit=8192; STR_HASHBYTES=32 out, SALTBYTES=16 | [x] `argon2_primitive_str_cross_verify` |
| PD20 | crypto_pwhash_argon2id_str | passwdlen=1, opslimit=1, memlimit=8192; STR_HASHBYTES=32 out, SALTBYTES=16 | [x] `argon2_primitive_str_cross_verify` |

## crypto_pwhash_str_verify (fixed encoded string, deterministic result)

Feed a known-good encoded hash + correct password (return 0). Uses argon2_verify ->
argon2_decode_string round-trip; fully deterministic and cheap at m=8,t=1..3,p=1.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD21 | crypto_pwhash_str_verify | str prefixed "$argon2id$" (m=8,t=1,p=1, seg=2), correct passwdlen=1 -> 0 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD22 | crypto_pwhash_str_verify | str prefixed "$argon2i$" (m=8,t=3,p=1), correct passwdlen=64 -> 0 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD23 | crypto_pwhash_str_verify | str prefixed "$argon2id$" (m=16,t=2,p=1, seg=4 > 2), correct passwdlen=0 -> 0 | [x] `crypto_pwhash_str_and_alg_cross_verify` |
| PD24 | crypto_pwhash_argon2i_str_verify | "$argon2i$" m=8,t=3,p=1, correct passwdlen=1 -> 0 | [x] `argon2_primitive_str_cross_verify` |
| PD25 | crypto_pwhash_argon2id_str_verify | "$argon2id$" m=8,t=1,p=1, correct passwdlen=1 -> 0 | [x] `argon2_primitive_str_cross_verify` |

## crypto_pwhash_str_needs_rehash (decode-only, no KDF run — trivially cheap)

Compares decoded (m_cost,t_cost) against requested (opslimit, memlimit/1024).
Returns 0 when equal, 1 when different. No password hashing performed.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD26 | crypto_pwhash_str_needs_rehash | "$argon2id$" str with m=8,t=1; opslimit=1,memlimit=8192 -> 0 (match) | [x] `str_needs_rehash_fixed_vectors` |
| PD27 | crypto_pwhash_str_needs_rehash | "$argon2id$" str with m=8,t=1; opslimit=2,memlimit=8192 -> 1 (t differs) | [x] `str_needs_rehash_fixed_vectors` |
| PD28 | crypto_pwhash_str_needs_rehash | "$argon2id$" str with m=8,t=1; opslimit=1,memlimit=16384 -> 1 (m differs) | [x] `str_needs_rehash_fixed_vectors` |
| PD29 | crypto_pwhash_argon2i_str_needs_rehash | "$argon2i$" str m=8,t=3; opslimit=3,memlimit=8192 -> 0 (match) | [x] `argon2_primitive_str_verify_and_needs_rehash` |
| PD30 | crypto_pwhash_argon2id_str_needs_rehash | "$argon2id$" str m=16,t=2; opslimit=2,memlimit=16384 -> 0 (match) | [x] `argon2_primitive_str_verify_and_needs_rehash` |

## argon2-encoding.c round-trip shapes (decode + encode)

Exercised via decode_string (verify/needs_rehash) and encode_string (str). Distinct
shapes the parser branches on: type prefix, $v=, m=/t=/p= order, base64 no-padding.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD31 | argon2_decode_string (via _str_verify) | "$argon2id$v=19$m=8,t=1,p=1$<b64 salt 16B>$<b64 out 32B>" round-trips to same params | [x] `sodium_argon2_encode_decode_roundtrip` |
| PD32 | argon2_decode_string (via _str_verify) | "$argon2i$v=19$m=8,t=3,p=1$<b64 salt>$<b64 out>" round-trips | [x] `sodium_argon2_encode_decode_roundtrip` |
| PD33 | argon2_encode_string (via _str) | id: minimal salt=16B, out=32B produces canonical no-padding base64, p=1 | [x] `sodium_argon2_encode_decode_roundtrip` |

## crypto_pwhash_scryptsalsa208sha256 (nosse escrypt_kdf, ll derivation)

pickparams: if opslimit<32768 it is bumped to 32768. Branch A (opslimit < memlimit/32):
p=1, N from opslimit/(r*4). Branch B (opslimit >= memlimit/32): N from memlimit/(r*128),
p = (opslimit/4 / N)/r, can give p>1. r fixed at 8. Cheap = small N (log2 low).

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD34 | crypto_pwhash_scryptsalsa208sha256 | outlen=16 (BYTES_MIN), passwdlen=1, salt=32B, opslimit=32768 (MIN), memlimit=16777216 (MIN); pickparams Branch A, p=1, small N | [x] `scrypt_ll_matrix` |
| PD35 | crypto_pwhash_scryptsalsa208sha256 | outlen=32, passwdlen=0, salt=32B, opslimit=32768, memlimit=16777216 (MIN); Branch A p=1 | [x] `scrypt_ll_matrix` |
| PD36 | crypto_pwhash_scryptsalsa208sha256 | outlen=64, passwdlen=64, salt=32B, opslimit=32768, memlimit=524288 (< opslimit*32 -> Branch B, p can be >1) | [x] `scrypt_ll_matrix` |
| PD37 | crypto_pwhash_scryptsalsa208sha256 | outlen=32, passwdlen=1100 (> block), salt=32B, opslimit=65536, memlimit=1048576 (Branch B, larger N) | [x] `scrypt_ll_matrix` |

## crypto_pwhash_scryptsalsa208sha256_ll (direct N,r,p — full manual control, cheapest)

_ll bypasses pickparams: N,r,p passed directly. N must be power of 2. p>1 changes the
outer loop in escrypt_kdf. Use tiny N to stay sub-millisecond.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD38 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=1, p=1, passwdlen=1, saltlen=0, buflen=16 | [x] `scrypt_ll_matrix` |
| PD39 | crypto_pwhash_scryptsalsa208sha256_ll | N=4, r=1, p=1, passwdlen=0, saltlen=1, buflen=32 | [x] `scrypt_ll_matrix` |
| PD40 | crypto_pwhash_scryptsalsa208sha256_ll | N=16, r=8, p=1, passwdlen=64, saltlen=32, buflen=64 | [x] `scrypt_ll_matrix` |
| PD41 | crypto_pwhash_scryptsalsa208sha256_ll | N=16, r=1, p=2 (p>1 loop path), passwdlen=8, saltlen=32, buflen=32 | [x] `scrypt_ll_matrix` |
| PD42 | crypto_pwhash_scryptsalsa208sha256_ll | N=1024, r=8, p=1, passwdlen=8, saltlen=32, buflen=64 (largest N, still fast) | [x] `scrypt_ll_matrix` |
| PD43 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=8, p=3 (p>1), passwdlen=1, saltlen=8, buflen=16 | [x] `scrypt_ll_matrix` |

## crypto_pwhash_scryptsalsa208sha256_str / _str_verify / _str_needs_rehash

_str embeds a random salt (compare "$7$" setting SHAPE, not bytes); _str_verify with a
fixed known string is deterministic; _needs_rehash decodes setting only (no KDF).

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD44 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen=1, opslimit=32768, memlimit=16777216; expect "$7$" prefix + gensalt setting (N_log2,r,p encoded) | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| PD45 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen=64, opslimit=32768, memlimit=524288 (Branch B, p>1 in setting) | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| PD46 | crypto_pwhash_scryptsalsa208sha256_str_verify | fixed known "$7$..." string + correct passwdlen=1 -> 0 (deterministic) | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| PD47 | crypto_pwhash_scryptsalsa208sha256_str_verify | fixed known "$7$..." string with saltlen internal, correct passwdlen=64 -> 0 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| PD48 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | "$7$" str matching pickparams(32768,16777216) -> 0 | [x] `scrypt_str_roundtrip_and_needs_rehash` |
| PD49 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | "$7$" str vs pickparams(32768,524288) mismatch -> 1 | [x] `scrypt_str_roundtrip_and_needs_rehash` |

## randombytes — randombytes_buf (non-deterministic: compare only that it fills & returns)

Default impl (sysrandom); output not reproducible, so differential check is length
handling and the size==0 no-op branch, not byte content.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD50 | randombytes_buf | size=0 (no-op branch, buf untouched) | [x] `buf_deterministic_exact` |
| PD51 | randombytes_buf | size=1 | [x] `buf_deterministic_exact` |
| PD52 | randombytes_buf | size=32 | [x] `buf_deterministic_exact` |
| PD53 | randombytes_buf | size=4096 (large / multi-chunk) | [x] `buf_deterministic_exact` |

## randombytes_buf_deterministic (seeded ChaCha20 — fully differentiable by bytes)

Deterministic: fixed 32B seed + fixed nonce "LibsodiumDRG" -> compare exact output bytes.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD54 | randombytes_buf_deterministic | size=0, seed=32 zero bytes (no-op, output empty) | [x] `buf_deterministic_exact` |
| PD55 | randombytes_buf_deterministic | size=1, seed=32 zero bytes -> compare exact byte | [x] `buf_deterministic_exact` |
| PD56 | randombytes_buf_deterministic | size=32, seed=all 0x00 -> compare 32 bytes | [x] `buf_deterministic_exact` |
| PD57 | randombytes_buf_deterministic | size=32, seed=all 0xFF -> compare 32 bytes (seed sensitivity) | [x] `buf_deterministic_exact` |
| PD58 | randombytes_buf_deterministic | size=4096, seed=incrementing 0..31 -> compare full block (crosses ChaCha20 block boundary) | [x] `buf_deterministic_exact` |

## randombytes_uniform (consumes randomness — compare bound-handling branch only unless deterministic impl installed)

For bound < 2 the function returns 0 without consuming randomness (deterministic,
byte-comparable). For bound >= 2 it loops on randombytes_random(); only comparable
with a deterministic implementation installed via randombytes_set_implementation.

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD59 | randombytes_uniform | upper_bound=0 -> returns 0 (bound<2 branch, deterministic) | [x] `uniform_bounds_and_small_bound_zero` |
| PD60 | randombytes_uniform | upper_bound=1 -> returns 0 (bound<2 branch, deterministic) | [x] `uniform_bounds_and_small_bound_zero` |
| PD61 | randombytes_uniform | upper_bound=2 (bound>=2; min=0; rejection loop) — compare branch structure / min computation | [x] `uniform_bounds_and_small_bound_zero` |
| PD62 | randombytes_uniform | upper_bound=16 (power of two; min = 2^32 mod 16 = 0, no rejection) | [x] `uniform_bounds_and_small_bound_zero` |
| PD63 | randombytes_uniform | upper_bound=17 (non-power-of-two; min = 2^32 mod 17 != 0, rejection possible) | [x] `uniform_bounds_and_small_bound_zero` |

## randombytes_set_implementation / randombytes_implementation_name

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD64 | randombytes_set_implementation + randombytes_implementation_name | install &randombytes_internal_implementation, then name -> "internal" | [x] `set_implementation_internal_then_sysrandom` |
| PD65 | randombytes_set_implementation + randombytes_implementation_name | install &randombytes_sysrandom_implementation, then name -> "sysrandom" | [x] `set_implementation_internal_then_sysrandom` |
| PD66 | randombytes_implementation_name | default (init_if_needed installs sysrandom) -> "sysrandom" | [x] `set_implementation_internal_then_sysrandom` |

## randombytes_internal_implementation vs randombytes_sysrandom_implementation

internal_implementation is the ChaCha20-based deterministic-per-seed PRNG (differentiable
when seeded); sysrandom is OS entropy (name/uniform/buf structure comparable only).

| # | entry point(s) | configuration (options set + input shape) | covered by test |
|---|---|---|---|
| PD67 | randombytes_internal_implementation | set as impl; implementation_name -> "internal"; uniform present (non-NULL) so randombytes_uniform delegates to impl->uniform | [x] `randombytes_implementation_globals` |
| PD68 | randombytes_sysrandom_implementation | set as impl; implementation_name -> "sysrandom"; verify buf/random present, uniform branch structure | [x] `randombytes_implementation_globals` |

