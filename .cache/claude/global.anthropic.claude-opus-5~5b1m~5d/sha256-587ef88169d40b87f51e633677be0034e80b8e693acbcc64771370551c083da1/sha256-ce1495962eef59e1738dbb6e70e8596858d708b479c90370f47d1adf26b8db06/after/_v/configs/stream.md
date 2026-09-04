| stream-1 | crypto_core_salsa20 | c=NULL → built-in sigma constants; 44 in/k cases (all-zero, all-0xff, mixed, 40 random) | [x] |
| stream-2 | crypto_core_salsa20 | c!=NULL → constants loaded from c[0..16]; same 44 in/k cases | [x] |
| stream-3 | crypto_core_salsa20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-4 | crypto_core_salsa2012 | c=NULL, 12 rounds, 44 in/k cases | [x] |
| stream-5 | crypto_core_salsa2012 | c!=NULL, 12 rounds, 44 in/k cases | [x] |
| stream-6 | crypto_core_salsa2012_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-7 | crypto_core_salsa208 | c=NULL, 8 rounds, 44 in/k cases | [x] |
| stream-8 | crypto_core_salsa208 | c!=NULL, 8 rounds, 44 in/k cases | [x] |
| stream-9 | crypto_core_salsa208_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 64 / 16 / 32 / 16 | [x] |
| stream-10 | crypto_core_hsalsa20 | c=NULL → built-in constants; 44 in/k cases; 32-byte output | [x] |
| stream-11 | crypto_core_hsalsa20 | c!=NULL → constants from c[0..16]; 44 in/k cases | [x] |
| stream-12 | crypto_core_hsalsa20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 32 / 16 / 32 / 16 | [x] |
| stream-13 | crypto_core_hchacha20 | c=NULL → built-in constants; 44 in/k cases; 32-byte output | [x] |
| stream-14 | crypto_core_hchacha20 | c!=NULL → constants from c[0..16]; 44 in/k cases | [x] |
| stream-15 | crypto_core_hchacha20_outputbytes, _inputbytes, _keybytes, _constbytes | getters = 32 / 16 / 32 / 16 | [x] |
| stream-16 | crypto_stream_salsa20 | clen=0 → early `return 0`, output buffer untouched | [x] |
| stream-17 | crypto_stream_salsa20 | clen ∈ {64,128,192,256,320,384,512,1024} (whole-block loop writes the core output straight into `c`) | [x] |
| stream-18 | crypto_stream_salsa20 | clen non-multiple of 64 ∈ {1,2,3,31,32,33,63,65,66,100,127,129,130,…,1025} + 20 random ≤1500 (trailing partial block copied out of `block[]`) | [x] |
| stream-19 | crypto_stream_salsa20_xor | out-of-place, 53 lengths (all of stream-17/18 plus 0) | [x] |
| stream-20 | crypto_stream_salsa20_xor | in-place (c == m), 53 lengths, result compared against the out-of-place result | [x] |
| stream-21 | crypto_stream_salsa20_xor_ic | ic=0 (identical to `_xor`), 53 lengths | [x] |
| stream-22 | crypto_stream_salsa20_xor_ic | ic ∈ {1,2,7,0xdeadbeef12345678}, 53 lengths | [x] |
| stream-23 | crypto_stream_salsa20_xor_ic | ic ∈ {2^32-2, 2^32-1, 2^32, 2^32+1} — carry out of the low 4 counter bytes of the 8-byte LE counter | [x] |
| stream-24 | crypto_stream_salsa20_xor_ic | ic ∈ {2^64-2, 2^64-1} — full 64-bit counter overflow (all 8 counter bytes wrap to 0 mid-message) | [x] |
| stream-25 | crypto_stream_salsa20_xor_ic | in-place (c == m) for every (length, ic) pair above | [x] |
| stream-26 | crypto_stream_salsa20_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SODIUM_SIZE_MAX (== SIZE_MAX) | [x] |
| stream-27 | crypto_stream_salsa20_ref_implementation → `.stream` fn-ptr | exported implementation struct; called directly through both libs, 53 lengths | [x] |
| stream-28 | crypto_stream_salsa20_ref_implementation → `.stream_xor_ic` fn-ptr (`stream_ref_xor_ic`) | 53 lengths × 11 ic values, out-of-place + in-place | [x] |
| stream-29 | _crypto_stream_salsa20_pick_best_implementation | returns 0 (ref impl always chosen, no SIMD in this build); dispatchers re-verified afterwards | [x] |
| stream-30 | crypto_stream_salsa2012 | clen=0, multiples of 64, non-multiples of 64, 53 lengths | [x] |
| stream-31 | crypto_stream_salsa2012_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-32 | crypto_stream_salsa2012_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-33 | crypto_stream_salsa208 | clen=0, multiples of 64, non-multiples of 64, 53 lengths | [x] |
| stream-34 | crypto_stream_salsa208_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-35 | crypto_stream_salsa208_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-36 | crypto_stream_xsalsa20 | 24-byte nonce → hsalsa20 subkey + salsa20 keystream on n+16; 53 lengths | [x] |
| stream-37 | crypto_stream_xsalsa20_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-38 | crypto_stream_xsalsa20_xor_ic | 53 lengths × 11 ic values incl. 2^32 and 2^64 rollover, out-of-place + in-place | [x] |
| stream-39 | crypto_stream_xsalsa20_keybytes, _noncebytes, _messagebytes_max | 32 / 24 / SIZE_MAX | [x] |
| stream-40 | crypto_stream | generic dispatcher → xsalsa20; 53 lengths, 24-byte nonce | [x] |
| stream-41 | crypto_stream_xor | generic dispatcher → xsalsa20_xor; out-of-place + in-place, 53 lengths | [x] |
| stream-42 | crypto_stream_keybytes, _noncebytes, _messagebytes_max, _primitive | 32 / 24 / SIZE_MAX / "xsalsa20" | [x] |
| stream-43 | crypto_stream_chacha20 | clen=0 → early `return 0` | [x] |
| stream-44 | crypto_stream_chacha20 | clen < 64 (partial-block path: `memset(tmp)`, `ctarget` redirection) — 1,2,3,31,32,33,63 | [x] |
| stream-45 | crypto_stream_chacha20 | clen == 64 exactly (`bytes <= 64` taken, `bytes < 64` not taken) | [x] |
| stream-46 | crypto_stream_chacha20 | clen > 64 and a multiple of 64 (128,192,256,320,384,512,1024) | [x] |
| stream-47 | crypto_stream_chacha20 | clen > 64 and not a multiple of 64 (65,66,100,127,129,…,1025 + 20 random) — full-block loop followed by the `tmp[]` tail | [x] |
| stream-48 | crypto_stream_chacha20_xor | out-of-place + in-place, 53 lengths, 8-byte nonce | [x] |
| stream-49 | crypto_stream_chacha20_xor_ic | ic ∈ {0,1,2,7,0xdeadbeef12345678} split into low/high 32-bit counter words, 53 lengths | [x] |
| stream-50 | crypto_stream_chacha20_xor_ic | ic ∈ {2^32-2, 2^32-1} with mlen > 64 — `j12` wraps to 0 and increments `j13` (high counter word) | [x] |
| stream-51 | crypto_stream_chacha20_xor_ic | ic ∈ {2^64-2, 2^64-1} — both counter words wrap; also ic = 2^32 / 2^32+1 (high word non-zero from the start) | [x] |
| stream-52 | crypto_stream_chacha20_keybytes, _noncebytes, _messagebytes_max | 32 / 8 / SIZE_MAX | [x] |
| stream-53 | crypto_stream_chacha20_ietf | 12-byte nonce, 32-bit counter word (`chacha_ietf_ivsetup`); 53 lengths incl. 0, <64, ==64, >64 | [x] |
| stream-54 | crypto_stream_chacha20_ietf_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-55 | crypto_stream_chacha20_ietf_ext | internal-but-exported keystream entry point, 53 lengths | [x] |
| stream-56 | crypto_stream_chacha20_ietf_ext_xor_ic | 32-bit ic ∈ {0,1,2,0x7fffffff,0x80000000}, 53 lengths, out-of-place + in-place | [x] |
| stream-57 | crypto_stream_chacha20_ietf_ext_xor_ic | ic ∈ {2^32-2, 2^32-1} with mlen > 64 — counter overflows *into the IV word* (`j13`), the behaviour the `_ext` variant exists for | [x] |
| stream-58 | crypto_stream_chacha20_ietf_xor_ic | ic at the exact largest accepted value 2^32 − ceil(mlen/64) for mlen ∈ {0,1,63,64,65,127,128,129,192,256,1000,1024,1025} | [x] |
| stream-59 | crypto_stream_chacha20_ietf_xor_ic | ic = max−1, 0 and 1 for the same mlen set (inside the accepted range) | [x] |
| stream-60 | crypto_stream_chacha20_ietf_keybytes, _ietf_noncebytes, _ietf_messagebytes_max | 32 / 12 / min(SIZE_MAX, 64·2^32) = 274877906944 | [x] |
| stream-61 | crypto_stream_chacha20_ref_implementation → `.stream`, `.stream_ietf_ext`, `.stream_xor_ic`, `.stream_ietf_ext_xor_ic` | exported implementation struct; every fn-ptr called directly through both libs over the full length × ic matrix | [x] |
| stream-62 | _crypto_stream_chacha20_pick_best_implementation | returns 0 (ref impl always chosen); dispatchers re-verified afterwards | [x] |
| stream-63 | crypto_stream_xchacha20 | 24-byte nonce → hchacha20 subkey + chacha20 on n+16; 53 lengths | [x] |
| stream-64 | crypto_stream_xchacha20_xor | out-of-place + in-place, 53 lengths | [x] |
| stream-65 | crypto_stream_xchacha20_xor_ic | 53 lengths × 11 ic values incl. 2^32 and 2^64 rollover, out-of-place + in-place | [x] |
| stream-66 | crypto_stream_xchacha20_keybytes, _noncebytes, _messagebytes_max | 32 / 24 / SIZE_MAX | [x] |
| stream-67 | crypto_stream_keygen, crypto_stream_salsa20_keygen, crypto_stream_salsa2012_keygen, crypto_stream_salsa208_keygen, crypto_stream_xsalsa20_keygen, crypto_stream_chacha20_keygen, crypto_stream_chacha20_ietf_keygen, crypto_stream_xchacha20_keygen | a deterministic `randombytes_implementation` is installed in BOTH libraries via randombytes_set_implementation, so the 32 written bytes are compared byte-for-byte; canary proves exactly KEYBYTES bytes are written | [x] |
| stream-68 | all `*_xor` / `*_xor_ic` / keystream entry points | output buffer padded with a 32-byte 0x5A canary that is compared too, so any over/under-write is caught | [x] |
