| G2 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2I13 (1), opslimit=OPSLIMIT_MIN=3, memlimit=MEMLIMIT_MIN=8192, salt=16B fixed, passwdlen=0 (PASSWD_MIN), outlen=BYTES_MIN=16 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=8192, salt=16B, passwdlen=1, outlen=16 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=8192, salt=16B, passwdlen=16, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=8192, salt=16B, passwdlen=64, outlen=64 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=8192, salt=16B, passwdlen=1000, outlen=100 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=8192, salt=16B, passwdlen=4096 (> one argon2 block), outlen=17 (non-multiple-of-8) | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=OPSLIMIT_MIN+1=4 (== OPSLIMIT_INTERACTIVE for argon2i), memlimit=8192, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=OPSLIMIT_MODERATE=6, memlimit=8192, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=MEMLIMIT_MIN+1=8193 (must give the SAME digest as 8192 because of the /1024U truncation) | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=9216 (=9 KiB blocks, non-power-of-two m_cost -> segment_length rounding in argon2_ctx) | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=3, memlimit=16384, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=OPSLIMIT_INTERACTIVE=4, memlimit=MEMLIMIT_INTERACTIVE=33554432 (32 MiB) -- official interactive profile | [ ] |
| G2 | crypto_pwhash | alg=1, opslimit=OPSLIMIT_MODERATE=6, memlimit=MEMLIMIT_MODERATE=134217728 (128 MiB) | [ ] |
| G2 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2ID13 (2), opslimit=OPSLIMIT_MIN=1, memlimit=MEMLIMIT_MIN=8192, salt=16B, passwdlen=0, outlen=16 | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=1, memlimit=8192, passwdlen=16, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=OPSLIMIT_MIN+1=2 (== argon2id OPSLIMIT_INTERACTIVE), memlimit=8192, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=OPSLIMIT_MODERATE=3, memlimit=8192, outlen=32 | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=OPSLIMIT_SENSITIVE=4, memlimit=8192, outlen=64 | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=1, memlimit=MEMLIMIT_MIN+1=8193 (same digest as 8192) | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=1, memlimit=8192, outlen=64 (BLAKE2b native size, blake2b_long single-shot path) | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=1, memlimit=8192, outlen=65 and outlen=100 (blake2b_long multi-block extension path) | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=1, memlimit=8192, outlen=128 and 129 (blake2b_long while-loop path) | [ ] |
| G2 | crypto_pwhash | alg=crypto_pwhash_ALG_DEFAULT (== ALG_ARGON2ID13 == 2), opslimit=OPSLIMIT_INTERACTIVE=2, memlimit=MEMLIMIT_INTERACTIVE=67108864 (64 MiB) -- official interactive profile | [ ] |
| G2 | crypto_pwhash | alg=2, opslimit=OPSLIMIT_MODERATE=3, memlimit=MEMLIMIT_MODERATE=268435456 (256 MiB) | [ ] |
| G2 | crypto_pwhash | salt = all-zero 16 bytes; salt = 0x00..0x0f; salt = all-0xff (SALTBYTES is fixed at 16, always exactly 16 read) | [ ] |
| G2 | crypto_pwhash | passwd containing embedded NUL bytes and 0xff bytes (passwdlen given explicitly, not strlen) | [ ] |
| G2 | crypto_pwhash_argon2i | direct entry point, alg must be 1; opslimit=3, memlimit=8192, salt=16B, outlen=16/32/64 | [ ] |
| G2 | crypto_pwhash_argon2id | direct entry point, alg must be 2; opslimit=1, memlimit=8192, salt=16B, outlen=16/32/64 | [ ] |
| G2 | crypto_pwhash_str | (always argon2id) opslimit=1, memlimit=8192, passwdlen=0; check out is NUL-terminated, strlen(out) < 128, prefix == "$argon2id$" | [ ] |
| G2 | crypto_pwhash_str | opslimit=1, memlimit=8192, passwdlen=16; then feed out to crypto_pwhash_str_verify (round trip) | [ ] |
| G2 | crypto_pwhash_str | opslimit=OPSLIMIT_INTERACTIVE=2, memlimit=MEMLIMIT_INTERACTIVE=67108864, passwdlen=16; round trip | [ ] |
| G2 | crypto_pwhash_str_alg | alg=crypto_pwhash_ALG_ARGON2I13 (1), opslimit=3, memlimit=8192; prefix must be "$argon2i$", "$v=19", "m=8,t=3,p=1" | [ ] |
| G2 | crypto_pwhash_str_alg | alg=crypto_pwhash_ALG_ARGON2ID13 (2), opslimit=1, memlimit=8192; prefix "$argon2id$", "$v=19$m=8,t=1,p=1" | [ ] |
| G2 | crypto_pwhash_str_alg | alg=1, opslimit=OPSLIMIT_INTERACTIVE=4, memlimit=MEMLIMIT_INTERACTIVE=33554432 (m=32768) | [ ] |
| G2 | crypto_pwhash_argon2i_str | opslimit=3, memlimit=8192, passwdlen 0/1/16/1000; 128-byte out buffer, verify tail is zeroed | [ ] |
| G2 | crypto_pwhash_argon2id_str | opslimit=1, memlimit=8192, passwdlen 0/1/16/1000; 128-byte out buffer | [ ] |
| G2 | crypto_pwhash_str + crypto_pwhash_str_verify | round trip, correct password, passwdlen=0 | [ ] |
| G2 | crypto_pwhash_str + crypto_pwhash_str_verify | round trip, correct password, passwdlen=16 | [ ] |
| G2 | crypto_pwhash_str_verify | fixed known-good "$argon2id$v=19$m=8,t=1,p=1$<16B-salt-b64>$<32B-hash-b64>" vector, correct password -> 0 | [ ] |
| G2 | crypto_pwhash_str_verify | fixed known-good "$argon2i$v=19$m=8,t=3,p=1$..." vector, correct password -> 0 (dispatch on the "$argon2i$" prefix) | [ ] |
| G2 | crypto_pwhash_str_verify | valid string, wrong password (single bit flipped) -> -1 with errno=EINVAL | [ ] |
| G2 | crypto_pwhash_str_verify | valid string, password truncated by one byte -> -1 EINVAL | [ ] |
| G2 | crypto_pwhash_argon2i_str_verify / crypto_pwhash_argon2id_str_verify | direct entry points on their own variant's string (correct and wrong password) | [ ] |
| G2 | crypto_pwhash_str_verify | argon2 strings with m_cost = 8 (ARGON2_MIN_MEMORY) and m_cost = 8*p exactly (p=1) | [ ] |
| G2 | crypto_pwhash_str_verify | argon2 string with p=2, m=16 (lanes>1: exercises the multi-lane fill path; m_cost >= 8*lanes) | [ ] |
| G2 | crypto_pwhash_str_verify | argon2 string with p=4, m=32 | [ ] |
| G2 | crypto_pwhash_str_verify | argon2 string with a 8-byte salt (ARGON2_MIN_SALT_LENGTH) and a 16-byte hash (ARGON2_MIN_OUTLEN) | [ ] |
| G2 | crypto_pwhash_str_verify | argon2 string with a 64-byte hash and a 32-byte salt (longer than what _str emits) | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (opslimit=1, memlimit=8192), queried with the identical pair -> 0 | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (1, 8192), queried with opslimit=2 -> 1 | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (1, 8192), queried with memlimit=16384 -> 1 | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (1, 8192), queried with memlimit=8192+1023=9215 -> 0 (memlimit/1024 truncation makes it equal) | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (1, 8192), queried with memlimit=0 -> 1 (no MEMLIMIT_MIN check in this entry point) | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | str produced with (1, 8192), queried with opslimit=0 -> 1 (no OPSLIMIT_MIN check in this entry point) | [ ] |
| G2 | crypto_pwhash_argon2i_str_needs_rehash | argon2i string queried against its own (opslimit=3, memlimit=8192) -> 0, and against (6, 8192) -> 1 | [ ] |
| G2 | crypto_pwhash_argon2id_str_needs_rehash | argon2id string queried against its own params -> 0, changed params -> 1 | [ ] |
| G2 | crypto_pwhash_str_needs_rehash | opslimit = UINT32_MAX = 4294967295 (largest accepted), memlimit = 4398046511104 (largest accepted, /1024 == UINT32_MAX) -> 1 | [ ] |
| G2 | argon2_encode_string + argon2_decode_string (via crypto_pwhash_str / crypto_pwhash_str_verify) | round trip "$argon2i$v=19$m=8,t=3,p=1$<salt>$<out>" -- base64 ORIGINAL_NO_PADDING, no '=' characters | [ ] |
| G2 | argon2_encode_string + argon2_decode_string | round trip "$argon2id$v=19$m=8,t=1,p=1$<salt>$<out>" | [ ] |
| G2 | argon2_decode_string (via str_verify) | salt of 16 bytes -> 22 base64 chars (16 % 3 == 1, so unpadded remainder of 2 chars) | [ ] |
| G2 | argon2_decode_string (via str_verify) | salt of 15 bytes -> 20 chars, and 18 bytes -> 24 chars (all three base64 remainder classes 0/1/2) | [ ] |
| G2 | argon2_decode_string (via str_verify) | hash of 32 bytes -> 43 base64 chars (remainder 2), 33 bytes -> 44 chars (remainder 0), 34 bytes -> 46 chars | [ ] |
| G2 | argon2_decode_string (via str_verify) | m/t/p at their decimal-width boundaries: m=8, m=99, m=100, m=4294967295; t=1, t=4294967295; p=1 | [ ] |
| G2 | argon2_decode_string (via str_verify) | version field "$v=19" (ARGON2_VERSION_NUMBER=0x13) -- the only accepted value; also record that "$v=16" (Argon2 1.0) and "$v=20" are rejected as ARGON2_INCORRECT_TYPE (-26 -> public -1) | [ ] |
| G2 | argon2_decode_string (via str_verify) | string with '=' base64 padding on salt and/or hash -> rejected (ARGON2_DECODING_FAIL); record both the padded-salt and padded-hash cases | [ ] |
| G2 | argon2_decode_string (via str_verify) | pre-1.3 string with no "$v=" segment ("$argon2i$m=8,t=3,p=1$...") -> rejected (mandatory CC("$v=")) | [ ] |
| G2 | argon2_decode_string (via crypto_pwhash_argon2i_str_verify) | an "$argon2id$..." string given to the argon2i decoder (prefix "$argon2i" matches, then "d$v=" fails CC("$v=")) -> ARGON2_DECODING_FAIL | [ ] |
| G2 | argon2_decode_string (via str_verify) | maximal-length valid string: strlen == 127 (one below crypto_pwhash_STRBYTES=128); also strlen == 128 for str_needs_rehash rejection | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=OPSLIMIT_MIN=32768, memlimit=MEMLIMIT_MIN=16777216, salt=32B (SALTBYTES), passwdlen=0, outlen=BYTES_MIN=16 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=32768, memlimit=16777216, salt=32B, passwdlen=16, outlen=32 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=32768, memlimit=16777216, passwdlen=64, outlen=64 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=32768, memlimit=16777216, passwdlen=1000, outlen=100 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=OPSLIMIT_MIN+1=32769, memlimit=16777216, outlen=32 (pickparams: same N/r/p as 32768 -> identical digest) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=0 and opslimit=1 (both silently clamped up to 32768 by pickparams -> identical digest to opslimit=32768); memlimit=16777216 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | memlimit=0 (below MEMLIMIT_MIN but NOT rejected: pickparams takes the else branch, N_log2=1 -> N=2, r=8, p=(opslimit/4)/2/8) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | memlimit=1024 and memlimit=16384 (tiny memlimit, else branch of pickparams, cheap N) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | memlimit = 32*opslimit exactly (boundary of the `opslimit < memlimit/32` branch in pickparams: takes the else branch) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | memlimit = 32*opslimit + 32 (takes the `p=1` / maxN=opslimit/32 branch of pickparams) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | opslimit=OPSLIMIT_INTERACTIVE=524288, memlimit=MEMLIMIT_INTERACTIVE=16777216 -- official interactive profile (N=16384, r=8, p=1) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256 | salt = all-zero 32B, salt = 0x00..0x1f, salt = all-0xff (SALTBYTES fixed at 32) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | opslimit=32768, memlimit=16777216, passwdlen=0; 102-byte out buffer, strlen(out) == 101, prefix "$7$" | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | opslimit=32768, memlimit=16777216, passwdlen=16; then str_verify round trip -> 0 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | opslimit=OPSLIMIT_INTERACTIVE=524288, memlimit=MEMLIMIT_INTERACTIVE=16777216; str_verify round trip | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | memlimit=0 / memlimit=1024 (cheap params still produce a well-formed 101-char "$7$" string) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | fixed known-good 101-char "$7$" vector, correct password -> 0 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | fixed vector, wrong password -> -1 (sodium_memcmp result, errno untouched) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | fixed vector, passwdlen=0 vs correct password (no passwdlen bounds check in this entry point) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | str produced with (32768, 16777216), queried with the same pair -> 0 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | same str, queried with opslimit=524288 (different N_log2) -> 1 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | same str, queried with memlimit=1073741824 (different N_log2/p) -> 1 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | same str, queried with opslimit=32769 (pickparams yields identical N/r/p) -> 0 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=1, p=1, passwdlen=0, saltlen=0, buflen=16 (minimum legal N) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=1, p=1, passwdlen=8, saltlen=8, buflen=32 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | RFC 7914 vector 1: passwd="" saltlen=0, N=16, r=1, p=1, buflen=64 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | RFC 7914 vector 2: passwd="password", salt="NaCl", N=1024, r=8, p=16, buflen=64 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | RFC 7914 vector 3: passwd="pleaseletmein", salt="SodiumChloride", N=16384, r=8, p=1, buflen=64 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=1024, r=8, p=1, buflen=16/32/33/64/100 (buflen crossing the 32-byte PBKDF2 block boundary) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=1024, r=8, p=1, buflen=0 (no minimum-output check at the _ll level; PBKDF2 loop never runs) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=1, p=2 and p=4 and p=8 (multiple parallel blocks; r*p far below 2^30) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=2, r=2 / r=8 / r=16 (r drives the 128*r block size and the SSE/nosse blockmix width) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | N=4, 8, 16, 32, 64, 128, 256 (each power of two exercises a different smix integerify/mod-N path) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | passwdlen 0/1/32/64/65/128/1000 with N=2,r=1,p=1 (HMAC-SHA256 key-length boundaries at 64 bytes) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | saltlen 0/1/31/32/33/64/1000 with N=2,r=1,p=1 (no SALTBYTES constraint at the _ll level) | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | r*p == 2^30 - 1 = 1073741823 boundary (largest accepted; e.g. r=3, p=357913941) -- allocation-bound, see slow note | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | reuse of the same escrypt_local_t across calls is NOT exposed (each _ll call init/frees its own local); confirm two consecutive identical calls give identical output | [ ] |
| G2 | escrypt_parse_setting (via str_verify / str_needs_rehash) | valid "$7$" setting with N_log2 = 1..14 (itoa64 chars '0'..'>'), r=8, p=1 -- round trip through escrypt_gensalt_r | [ ] |
| G2 | escrypt_gensalt_r (via crypto_pwhash_scryptsalsa208sha256_str) | srclen = STRSALTBYTES = 32 -> saltlen = BYTES2CHARS(32) = 43 chars; prefixlen = 14; total setting = 57 = STRSETTINGBYTES | [ ] |
| G2 | escrypt_PBKDF2_SHA256 (indirect, via _ll) | c=1 (the only value used by scrypt), dkLen = 0/1/31/32/33/64/128*r*p | [ ] |
| G2 | sodium_runtime_has_sse2 dispatch | escrypt_kdf_sse vs escrypt_kdf_nosse must produce byte-identical output for the same (N,r,p) -- compare both implementations on the RFC 7914 vectors | [ ] |
| G2 | argon2_pick_best_implementation dispatch | argon2-fill-block-ref vs ssse3/avx2/avx512f/neon must produce byte-identical digests for the same params (crypto_pwhash with alg=1 and alg=2, m=8..64) | [ ] |
| G2 | crypto_pwhash_alg_argon2i13 / alg_argon2id13 / alg_default | constant accessors return 1 / 2 / 2 | [ ] |
| G2 | crypto_pwhash_bytes_min/bytes_max/passwd_min/passwd_max/saltbytes/strbytes/strprefix | 16 / min(SIZE_MAX,4294967295) / 0 / 4294967295 / 16 / 128 / "$argon2id$" | [ ] |
| G2 | crypto_pwhash_opslimit_min/max, memlimit_min/max, *_interactive, *_moderate, *_sensitive | 1 / 4294967295, 8192 / 4398046510080, 2+67108864, 3+268435456, 4+1073741824 | [ ] |
| G2 | crypto_pwhash_primitive | returns "argon2id,argon2i" | [ ] |
| G2 | crypto_pwhash_argon2i_* constant accessors | ALG=1, BYTES_MIN=16, PASSWD_MIN=0, PASSWD_MAX=4294967295, SALTBYTES=16, STRBYTES=128, STRPREFIX="$argon2i$", OPSLIMIT_MIN=3, OPSLIMIT_MAX=4294967295, MEMLIMIT_MIN=8192, INTERACTIVE 4/33554432, MODERATE 6/134217728, SENSITIVE 8/536870912 | [ ] |
| G2 | crypto_pwhash_argon2id_* constant accessors | ALG=2, OPSLIMIT_MIN=1, MEMLIMIT_MIN=8192, INTERACTIVE 2/67108864, MODERATE 3/268435456, SENSITIVE 4/1073741824, STRPREFIX="$argon2id$", STRBYTES=128 | [ ] |
| G2 | crypto_pwhash_scryptsalsa208sha256_* constant accessors | BYTES_MIN=16, BYTES_MAX=0x1fffffffe0=137438953440, PASSWD_MIN=0, PASSWD_MAX=SODIUM_SIZE_MAX, SALTBYTES=32, STRBYTES=102, STRPREFIX="$7$", OPSLIMIT_MIN=32768, OPSLIMIT_MAX=4294967295, MEMLIMIT_MIN=16777216, MEMLIMIT_MAX=68719476736, INTERACTIVE 524288/16777216, SENSITIVE 33554432/1073741824 | [ ] |

<!--
TOO SLOW / TOO MEMORY-HUNGRY TO TEST (do not put these in the default differential suite):

argon2 (memlimit is divided by 1024 to get m_cost in KiB; cost ~ t_cost * m_cost):
 - crypto_pwhash*_MEMLIMIT_SENSITIVE (argon2i 536870912 = 512 MiB; argon2id 1073741824 = 1 GiB) with
   OPSLIMIT_SENSITIVE: several seconds and 0.5-1 GiB RSS per call. SKIP entirely.
 - MEMLIMIT_MODERATE (argon2i 128 MiB / argon2id 256 MiB) with MODERATE opslimit: ~0.2-1 s and up to
   256 MiB per call. Run at most ONE smoke case each; do not sweep.
 - MEMLIMIT_INTERACTIVE (argon2i 32 MiB / argon2id 64 MiB): ~50-150 ms per call. Fine for a handful
   of round-trip cases, NOT for the full outlen x passwdlen cross product.
 - MEMLIMIT_MAX (4398046510080 = 4 TiB) and OPSLIMIT_MAX (4294967295) are UNTESTABLE as accepted
   values; only their rejection side (memlimit = MEMLIMIT_MAX+1, opslimit = OPSLIMIT_MAX+1) is cheap.
 - outlen near BYTES_MAX (4294967295) is untestable (4 GiB output buffer). Cap output sweeps at 128 bytes;
   blake2b_long's multi-block loop is already fully covered by outlen = 65, 100, 128, 129.
 - passwdlen near PASSWD_MAX (4294967295) is untestable; 4096 bytes already crosses the pre-hash
   block boundary.
 CHEAPEST PARAMS THAT STILL EXERCISE EVERY ARGON2 CODE PATH:
   argon2id: opslimit=1, memlimit=8192 (m_cost=8 KiB = ARGON2_MIN_MEMORY, 8 blocks, 1 pass)
   argon2i:  opslimit=3, memlimit=8192 (OPSLIMIT_MIN=3 for argon2i)
   multi-lane / multi-pass paths: p=2 with m=16 and p=4 with m=32 via handcrafted encoded strings fed to
   *_str_verify (crypto_pwhash_* always forces parallelism=1, so lanes>1 is ONLY reachable through
   argon2_decode_string).
   multi-pass: opslimit=2..4 at memlimit=8192 costs microseconds.

scrypt (cost ~ N*r*p; memory ~ 128*r*N):
 - OPSLIMIT_SENSITIVE=33554432 with MEMLIMIT_SENSITIVE=1073741824 (1 GiB): seconds per call. SKIP.
 - RFC 7914 vector 4 (passwd="pleaseletmein", salt="SodiumChloride", N=1048576, r=8, p=1) needs 1 GiB
   and ~1-3 s. SKIP or mark #[ignore].
 - OPSLIMIT_INTERACTIVE=524288 / MEMLIMIT_INTERACTIVE=16777216 yields N=16384, r=8, p=1 -> ~16 MiB and
   ~50 ms. Acceptable for a few cases only.
 - The r*p == 2^30-1 boundary (e.g. r=3, p=357913941) needs >128*r*p bytes = untestable; only its
   rejection side (r*p >= 2^30, which returns EFBIG before allocating) is cheap. Same for N > UINT32_MAX
   and buflen > 137438953440: all are rejected before any allocation, so they are cheap to test.
 CHEAPEST PARAMS THAT STILL EXERCISE EVERY SCRYPT CODE PATH:
   crypto_pwhash_scryptsalsa208sha256*: opslimit=32768 (OPSLIMIT_MIN, also the clamp floor) with
   memlimit=1024 or memlimit=0 -> pickparams gives N=2, r=8, small p: microseconds, a few KiB. This
   still walks pickparams' else-branch, escrypt_gensalt_r, escrypt_parse_setting, escrypt_r, smix and
   the full PBKDF2 wrapper. Use memlimit=33554432 (=32*opslimit+something) for exactly ONE case to
   cover pickparams' `p=1` first branch.
   crypto_pwhash_scryptsalsa208sha256_ll: N=2/4/16, r=1/8, p=1/2 -> effectively free; RFC 7914 vectors
   1-3 are all < 20 MiB and < 100 ms combined.
-->
