# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Mechanically derived from `nm -D --defined-only` on both shared objects — the
**complete** dynamic-symbol table, including the 121 `_`-prefixed internal
exports (`_crypto_*_pick_best_implementation`, `_sodium_softaes_*`,
`_sodium_ge25519_*`, `_sodium_argon2_*`, `_sodium_escrypt_*`, ...) and the
macro-generated ones.

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# Rust
cargo build --no-default-features
# compare
nm -D --defined-only c_src/build/libsodium.so     | awk '$3!=""{print $3}' | sort -u > c.txt
nm -D --defined-only target/debug/liblibsodium.so | awk '$3!=""{print $3}' | sort -u > r.txt
comm -23 c.txt r.txt   # exported by C, MISSING from Rust
comm -13 c.txt r.txt   # exported by Rust only
```

Reproduced end to end by `.phaseA/verify_all.sh` (step 3).

## Summary

| metric | value |
|---|---|
| symbols exported by the C `.so` | **890** |
| symbols exported by the Rust `.so` | **890** |
| **exported by C but MISSING from Rust** | **0** |
| exported by Rust but not by C | **0** |
| non-libc **undefined** symbols in the Rust `.so` | **0** |
| exported symbols actually driven by a differential test | **890 / 890** |

## MISSING SYMBOLS

**NONE — the symbol diff is empty in both directions.**

```
$ comm -23 c.txt r.txt | wc -l    ->  0
$ comm -13 c.txt r.txt | wc -l    ->  0
```

## No symbol is stubbed

Symbol parity alone would be satisfiable by stubs, so it is backed by a
**reachability audit** (`.phaseA/symbol_audit.py`): for each exported symbol it
checks that some test in `tests/` actually looks it up and calls it — either by
its literal name or through a `format!("{prefix}_suffix")` construction, which
is how the parameterised tests reach whole primitive families.

```
exported symbols (excl. ELF runtime): 890
  referenced literally in tests      : 798
  reachable via format! composition  : 92
  TOTAL reached                      : 890
  UNREFERENCED                       : 0
```

Every one of the 890 exported symbols is driven, through `dlsym`, by at
least one of the 349 differential tests, and each of those tests compares
the C result against the Rust result byte-for-byte (return value, every
out-parameter, `errno`, output-buffer contents including untouched regions,
and — for abort paths — the termination signal / exit code and the side
effects written before the abort).

## Undefined (imported) symbols in the Rust `.so`

57 undefined symbols, **all** libc / libgcc / ELF-runtime imports:

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
_Unwind_Backtrace@GCC_3.3
_Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0
_Unwind_GetIPInfo@GCC_4.2.0
_Unwind_GetLanguageSpecificData@GCC_3.0
_Unwind_GetRegionStart@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0
_Unwind_Resume@GCC_3.0
_Unwind_SetGR@GCC_3.0
_Unwind_SetIP@GCC_3.0
__cxa_finalize@GLIBC_2.2.5
__cxa_thread_atexit_impl@GLIBC_2.18
__errno_location@GLIBC_2.2.5
__gmon_start__
__tls_get_addr@GLIBC_2.3
abort@GLIBC_2.2.5
bcmp@GLIBC_2.2.5
calloc@GLIBC_2.2.5
close@GLIBC_2.2.5
dl_iterate_phdr@GLIBC_2.2.5
fcntl@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat64@GLIBC_2.33
getcwd@GLIBC_2.2.5
getenv@GLIBC_2.2.5
gettid@GLIBC_2.30
gettimeofday@GLIBC_2.2.5
lseek64@GLIBC_2.2.5
malloc@GLIBC_2.2.5
memchr@GLIBC_2.2.5
memcmp@GLIBC_2.2.5
memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5
memset@GLIBC_2.2.5
mmap64@GLIBC_2.2.5
munmap@GLIBC_2.2.5
open64@GLIBC_2.2.5
open@GLIBC_2.2.5
poll@GLIBC_2.2.5
posix_memalign@GLIBC_2.2.5
pthread_key_create@GLIBC_2.34
pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34
read@GLIBC_2.2.5
readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5
realpath@GLIBC_2.3
stat64@GLIBC_2.33
statx@GLIBC_2.28
strchr@GLIBC_2.2.5
strlen@GLIBC_2.2.5
strrchr@GLIBC_2.2.5
syscall@GLIBC_2.2.5
write@GLIBC_2.2.5
writev@GLIBC_2.2.5
```

The C `.so` imports 27:

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
__assert_fail@GLIBC_2.2.5
__cxa_finalize@GLIBC_2.2.5
__errno_location@GLIBC_2.2.5
__gmon_start__
abort@GLIBC_2.2.5
calloc@GLIBC_2.2.5
close@GLIBC_2.2.5
fcntl@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat@GLIBC_2.33
gettimeofday@GLIBC_2.2.5
malloc@GLIBC_2.2.5
memchr@GLIBC_2.2.5
memcmp@GLIBC_2.2.5
memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5
memset@GLIBC_2.2.5
open@GLIBC_2.2.5
poll@GLIBC_2.2.5
read@GLIBC_2.2.5
strchr@GLIBC_2.2.5
strlen@GLIBC_2.2.5
strncmp@GLIBC_2.2.5
strrchr@GLIBC_2.2.5
syscall@GLIBC_2.2.5
```

## Symbol-type notes

* 9 exports are **data objects** (`D` in both builds) — the implementation
  v-tables. They are compared structurally *and* driven functionally through
  their function pointers by
  `t06_internal_exports::exported_implementation_vtables_are_functionally_equal`:
  * `aegis128l_soft_implementation`
  * `aegis256_soft_implementation`
  * `crypto_onetimeauth_poly1305_donna_implementation`
  * `crypto_scalarmult_curve25519_ref10_implementation`
  * `crypto_stream_chacha20_ref_implementation`
  * `crypto_stream_salsa20_ref_implementation`
  * `ipcrypt_soft_implementation`
  * `randombytes_internal_implementation`
  * `randombytes_sysrandom_implementation`
* 12 exports are **weak** (`W`) in the C `.so` and strong (`T`) in the Rust
  `.so`:
  * `sodium_runtime_has_aesni` -> Rust `T`
  * `sodium_runtime_has_armcrypto` -> Rust `T`
  * `sodium_runtime_has_avx` -> Rust `T`
  * `sodium_runtime_has_avx2` -> Rust `T`
  * `sodium_runtime_has_avx512f` -> Rust `T`
  * `sodium_runtime_has_neon` -> Rust `T`
  * `sodium_runtime_has_pclmul` -> Rust `T`
  * `sodium_runtime_has_rdrand` -> Rust `T`
  * `sodium_runtime_has_sse2` -> Rust `T`
  * `sodium_runtime_has_sse3` -> Rust `T`
  * `sodium_runtime_has_sse41` -> Rust `T`
  * `sodium_runtime_has_ssse3` -> Rust `T`

  Both are dynamically resolvable functions with identical names and
  signatures, so `dlsym` finds them in either build; weak-vs-strong binding is a
  link-time property, not part of the ABI a `dlopen` consumer observes. They are
  compared by `t01_constants::int_accessors_match`.

## Full symbol table

`nm` type letters: `T` text (function), `D` initialised data, `W` weak, `B` bss.

| # | symbol | C type | Rust type | in Rust `.so` | driven by a test |
|---|--------|--------|-----------|---------------|------------------|
| 1 | `_crypto_aead_aegis128l_pick_best_implementation` | T | T | yes | yes |
| 2 | `_crypto_aead_aegis256_pick_best_implementation` | T | T | yes | yes |
| 3 | `_crypto_generichash_blake2b_pick_best_implementation` | T | T | yes | yes |
| 4 | `_crypto_ipcrypt_pick_best_implementation` | T | T | yes | yes |
| 5 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | T | T | yes | yes |
| 6 | `_crypto_pwhash_argon2_pick_best_implementation` | T | T | yes | yes |
| 7 | `_crypto_scalarmult_curve25519_pick_best_implementation` | T | T | yes | yes |
| 8 | `_crypto_sign_ed25519_detached` | T | T | yes | yes |
| 9 | `_crypto_sign_ed25519_ref10_hinit` | T | T | yes | yes |
| 10 | `_crypto_sign_ed25519_verify_detached` | T | T | yes | yes |
| 11 | `_crypto_stream_chacha20_pick_best_implementation` | T | T | yes | yes |
| 12 | `_crypto_stream_salsa20_pick_best_implementation` | T | T | yes | yes |
| 13 | `_sodium_alloc_init` | T | T | yes | yes |
| 14 | `_sodium_argon2_ctx` | T | T | yes | yes |
| 15 | `_sodium_argon2_decode_string` | T | T | yes | yes |
| 16 | `_sodium_argon2_encode_string` | T | T | yes | yes |
| 17 | `_sodium_argon2_fill_memory_blocks` | T | T | yes | yes |
| 18 | `_sodium_argon2_fill_segment_ref` | T | T | yes | yes |
| 19 | `_sodium_argon2_finalize` | T | T | yes | yes |
| 20 | `_sodium_argon2_hash` | T | T | yes | yes |
| 21 | `_sodium_argon2_initialize` | T | T | yes | yes |
| 22 | `_sodium_argon2_validate_inputs` | T | T | yes | yes |
| 23 | `_sodium_argon2_verify` | T | T | yes | yes |
| 24 | `_sodium_argon2i_hash_encoded` | T | T | yes | yes |
| 25 | `_sodium_argon2i_hash_raw` | T | T | yes | yes |
| 26 | `_sodium_argon2i_verify` | T | T | yes | yes |
| 27 | `_sodium_argon2id_hash_encoded` | T | T | yes | yes |
| 28 | `_sodium_argon2id_hash_raw` | T | T | yes | yes |
| 29 | `_sodium_argon2id_verify` | T | T | yes | yes |
| 30 | `_sodium_blake2b` | T | T | yes | yes |
| 31 | `_sodium_blake2b_compress_ref` | T | T | yes | yes |
| 32 | `_sodium_blake2b_final` | T | T | yes | yes |
| 33 | `_sodium_blake2b_init` | T | T | yes | yes |
| 34 | `_sodium_blake2b_init_key` | T | T | yes | yes |
| 35 | `_sodium_blake2b_init_key_salt_personal` | T | T | yes | yes |
| 36 | `_sodium_blake2b_init_param` | T | T | yes | yes |
| 37 | `_sodium_blake2b_init_salt_personal` | T | T | yes | yes |
| 38 | `_sodium_blake2b_long` | T | T | yes | yes |
| 39 | `_sodium_blake2b_pick_best_implementation` | T | T | yes | yes |
| 40 | `_sodium_blake2b_salt_personal` | T | T | yes | yes |
| 41 | `_sodium_blake2b_update` | T | T | yes | yes |
| 42 | `_sodium_core_h2c_string_to_hash` | T | T | yes | yes |
| 43 | `_sodium_escrypt_PBKDF2_SHA256` | T | T | yes | yes |
| 44 | `_sodium_escrypt_alloc_region` | T | T | yes | yes |
| 45 | `_sodium_escrypt_free_local` | T | T | yes | yes |
| 46 | `_sodium_escrypt_free_region` | T | T | yes | yes |
| 47 | `_sodium_escrypt_gensalt_r` | T | T | yes | yes |
| 48 | `_sodium_escrypt_init_local` | T | T | yes | yes |
| 49 | `_sodium_escrypt_kdf_nosse` | T | T | yes | yes |
| 50 | `_sodium_escrypt_parse_setting` | T | T | yes | yes |
| 51 | `_sodium_escrypt_r` | T | T | yes | yes |
| 52 | `_sodium_fe25519_frombytes` | T | T | yes | yes |
| 53 | `_sodium_fe25519_invert` | T | T | yes | yes |
| 54 | `_sodium_fe25519_tobytes` | T | T | yes | yes |
| 55 | `_sodium_ge25519_clear_cofactor` | T | T | yes | yes |
| 56 | `_sodium_ge25519_double_scalarmult_vartime` | T | T | yes | yes |
| 57 | `_sodium_ge25519_from_hash` | T | T | yes | yes |
| 58 | `_sodium_ge25519_from_uniform` | T | T | yes | yes |
| 59 | `_sodium_ge25519_frombytes` | T | T | yes | yes |
| 60 | `_sodium_ge25519_frombytes_negate_vartime` | T | T | yes | yes |
| 61 | `_sodium_ge25519_has_small_order` | T | T | yes | yes |
| 62 | `_sodium_ge25519_is_canonical` | T | T | yes | yes |
| 63 | `_sodium_ge25519_is_on_curve` | T | T | yes | yes |
| 64 | `_sodium_ge25519_is_on_main_subgroup` | T | T | yes | yes |
| 65 | `_sodium_ge25519_p1p1_to_p2` | T | T | yes | yes |
| 66 | `_sodium_ge25519_p1p1_to_p3` | T | T | yes | yes |
| 67 | `_sodium_ge25519_p2_to_p3` | T | T | yes | yes |
| 68 | `_sodium_ge25519_p3_add` | T | T | yes | yes |
| 69 | `_sodium_ge25519_p3_sub` | T | T | yes | yes |
| 70 | `_sodium_ge25519_p3_tobytes` | T | T | yes | yes |
| 71 | `_sodium_ge25519_scalarmult` | T | T | yes | yes |
| 72 | `_sodium_ge25519_scalarmult_base` | T | T | yes | yes |
| 73 | `_sodium_ge25519_tobytes` | T | T | yes | yes |
| 74 | `_sodium_keccak1600_ref_extract_bytes` | T | T | yes | yes |
| 75 | `_sodium_keccak1600_ref_init` | T | T | yes | yes |
| 76 | `_sodium_keccak1600_ref_permute_12` | T | T | yes | yes |
| 77 | `_sodium_keccak1600_ref_permute_24` | T | T | yes | yes |
| 78 | `_sodium_keccak1600_ref_xor_bytes` | T | T | yes | yes |
| 79 | `_sodium_mlkem768_ref_dec` | T | T | yes | yes |
| 80 | `_sodium_mlkem768_ref_enc` | T | T | yes | yes |
| 81 | `_sodium_mlkem768_ref_enc_deterministic` | T | T | yes | yes |
| 82 | `_sodium_mlkem768_ref_keypair` | T | T | yes | yes |
| 83 | `_sodium_mlkem768_ref_seed_keypair` | T | T | yes | yes |
| 84 | `_sodium_ristretto255_from_hash` | T | T | yes | yes |
| 85 | `_sodium_ristretto255_frombytes` | T | T | yes | yes |
| 86 | `_sodium_ristretto255_p3_tobytes` | T | T | yes | yes |
| 87 | `_sodium_runtime_get_cpu_features` | T | T | yes | yes |
| 88 | `_sodium_sc25519_invert` | T | T | yes | yes |
| 89 | `_sodium_sc25519_is_canonical` | T | T | yes | yes |
| 90 | `_sodium_sc25519_mul` | T | T | yes | yes |
| 91 | `_sodium_sc25519_muladd` | T | T | yes | yes |
| 92 | `_sodium_sc25519_reduce` | T | T | yes | yes |
| 93 | `_sodium_shake128_ref` | T | T | yes | yes |
| 94 | `_sodium_shake128_ref_init` | T | T | yes | yes |
| 95 | `_sodium_shake128_ref_init_with_domain` | T | T | yes | yes |
| 96 | `_sodium_shake128_ref_squeeze` | T | T | yes | yes |
| 97 | `_sodium_shake128_ref_update` | T | T | yes | yes |
| 98 | `_sodium_shake256_ref` | T | T | yes | yes |
| 99 | `_sodium_shake256_ref_init` | T | T | yes | yes |
| 100 | `_sodium_shake256_ref_init_with_domain` | T | T | yes | yes |
| 101 | `_sodium_shake256_ref_squeeze` | T | T | yes | yes |
| 102 | `_sodium_shake256_ref_update` | T | T | yes | yes |
| 103 | `_sodium_softaes_block_decrypt` | T | T | yes | yes |
| 104 | `_sodium_softaes_block_decryptlast` | T | T | yes | yes |
| 105 | `_sodium_softaes_block_encrypt` | T | T | yes | yes |
| 106 | `_sodium_softaes_block_encryptlast` | T | T | yes | yes |
| 107 | `_sodium_softaes_expand_key128` | T | T | yes | yes |
| 108 | `_sodium_softaes_expand_key256` | T | T | yes | yes |
| 109 | `_sodium_softaes_inv_mix_columns` | T | T | yes | yes |
| 110 | `_sodium_softaes_invert_key_schedule128` | T | T | yes | yes |
| 111 | `_sodium_softaes_invert_key_schedule256` | T | T | yes | yes |
| 112 | `_sodium_turboshake128_ref` | T | T | yes | yes |
| 113 | `_sodium_turboshake128_ref_init` | T | T | yes | yes |
| 114 | `_sodium_turboshake128_ref_init_with_domain` | T | T | yes | yes |
| 115 | `_sodium_turboshake128_ref_squeeze` | T | T | yes | yes |
| 116 | `_sodium_turboshake128_ref_update` | T | T | yes | yes |
| 117 | `_sodium_turboshake256_ref` | T | T | yes | yes |
| 118 | `_sodium_turboshake256_ref_init` | T | T | yes | yes |
| 119 | `_sodium_turboshake256_ref_init_with_domain` | T | T | yes | yes |
| 120 | `_sodium_turboshake256_ref_squeeze` | T | T | yes | yes |
| 121 | `_sodium_turboshake256_ref_update` | T | T | yes | yes |
| 122 | `aegis128l_soft_implementation` | D | D | yes | yes |
| 123 | `aegis256_soft_implementation` | D | D | yes | yes |
| 124 | `crypto_aead_aegis128l_abytes` | T | T | yes | yes |
| 125 | `crypto_aead_aegis128l_decrypt` | T | T | yes | yes |
| 126 | `crypto_aead_aegis128l_decrypt_detached` | T | T | yes | yes |
| 127 | `crypto_aead_aegis128l_encrypt` | T | T | yes | yes |
| 128 | `crypto_aead_aegis128l_encrypt_detached` | T | T | yes | yes |
| 129 | `crypto_aead_aegis128l_keybytes` | T | T | yes | yes |
| 130 | `crypto_aead_aegis128l_keygen` | T | T | yes | yes |
| 131 | `crypto_aead_aegis128l_messagebytes_max` | T | T | yes | yes |
| 132 | `crypto_aead_aegis128l_npubbytes` | T | T | yes | yes |
| 133 | `crypto_aead_aegis128l_nsecbytes` | T | T | yes | yes |
| 134 | `crypto_aead_aegis256_abytes` | T | T | yes | yes |
| 135 | `crypto_aead_aegis256_decrypt` | T | T | yes | yes |
| 136 | `crypto_aead_aegis256_decrypt_detached` | T | T | yes | yes |
| 137 | `crypto_aead_aegis256_encrypt` | T | T | yes | yes |
| 138 | `crypto_aead_aegis256_encrypt_detached` | T | T | yes | yes |
| 139 | `crypto_aead_aegis256_keybytes` | T | T | yes | yes |
| 140 | `crypto_aead_aegis256_keygen` | T | T | yes | yes |
| 141 | `crypto_aead_aegis256_messagebytes_max` | T | T | yes | yes |
| 142 | `crypto_aead_aegis256_npubbytes` | T | T | yes | yes |
| 143 | `crypto_aead_aegis256_nsecbytes` | T | T | yes | yes |
| 144 | `crypto_aead_aes256gcm_abytes` | T | T | yes | yes |
| 145 | `crypto_aead_aes256gcm_beforenm` | T | T | yes | yes |
| 146 | `crypto_aead_aes256gcm_decrypt` | T | T | yes | yes |
| 147 | `crypto_aead_aes256gcm_decrypt_afternm` | T | T | yes | yes |
| 148 | `crypto_aead_aes256gcm_decrypt_detached` | T | T | yes | yes |
| 149 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | T | T | yes | yes |
| 150 | `crypto_aead_aes256gcm_encrypt` | T | T | yes | yes |
| 151 | `crypto_aead_aes256gcm_encrypt_afternm` | T | T | yes | yes |
| 152 | `crypto_aead_aes256gcm_encrypt_detached` | T | T | yes | yes |
| 153 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | T | T | yes | yes |
| 154 | `crypto_aead_aes256gcm_is_available` | T | T | yes | yes |
| 155 | `crypto_aead_aes256gcm_keybytes` | T | T | yes | yes |
| 156 | `crypto_aead_aes256gcm_keygen` | T | T | yes | yes |
| 157 | `crypto_aead_aes256gcm_messagebytes_max` | T | T | yes | yes |
| 158 | `crypto_aead_aes256gcm_npubbytes` | T | T | yes | yes |
| 159 | `crypto_aead_aes256gcm_nsecbytes` | T | T | yes | yes |
| 160 | `crypto_aead_aes256gcm_statebytes` | T | T | yes | yes |
| 161 | `crypto_aead_chacha20poly1305_abytes` | T | T | yes | yes |
| 162 | `crypto_aead_chacha20poly1305_decrypt` | T | T | yes | yes |
| 163 | `crypto_aead_chacha20poly1305_decrypt_detached` | T | T | yes | yes |
| 164 | `crypto_aead_chacha20poly1305_encrypt` | T | T | yes | yes |
| 165 | `crypto_aead_chacha20poly1305_encrypt_detached` | T | T | yes | yes |
| 166 | `crypto_aead_chacha20poly1305_ietf_abytes` | T | T | yes | yes |
| 167 | `crypto_aead_chacha20poly1305_ietf_decrypt` | T | T | yes | yes |
| 168 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | T | T | yes | yes |
| 169 | `crypto_aead_chacha20poly1305_ietf_encrypt` | T | T | yes | yes |
| 170 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | T | T | yes | yes |
| 171 | `crypto_aead_chacha20poly1305_ietf_keybytes` | T | T | yes | yes |
| 172 | `crypto_aead_chacha20poly1305_ietf_keygen` | T | T | yes | yes |
| 173 | `crypto_aead_chacha20poly1305_ietf_messagebytes_max` | T | T | yes | yes |
| 174 | `crypto_aead_chacha20poly1305_ietf_npubbytes` | T | T | yes | yes |
| 175 | `crypto_aead_chacha20poly1305_ietf_nsecbytes` | T | T | yes | yes |
| 176 | `crypto_aead_chacha20poly1305_keybytes` | T | T | yes | yes |
| 177 | `crypto_aead_chacha20poly1305_keygen` | T | T | yes | yes |
| 178 | `crypto_aead_chacha20poly1305_messagebytes_max` | T | T | yes | yes |
| 179 | `crypto_aead_chacha20poly1305_npubbytes` | T | T | yes | yes |
| 180 | `crypto_aead_chacha20poly1305_nsecbytes` | T | T | yes | yes |
| 181 | `crypto_aead_xchacha20poly1305_ietf_abytes` | T | T | yes | yes |
| 182 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | T | T | yes | yes |
| 183 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | T | T | yes | yes |
| 184 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | T | T | yes | yes |
| 185 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | T | T | yes | yes |
| 186 | `crypto_aead_xchacha20poly1305_ietf_keybytes` | T | T | yes | yes |
| 187 | `crypto_aead_xchacha20poly1305_ietf_keygen` | T | T | yes | yes |
| 188 | `crypto_aead_xchacha20poly1305_ietf_messagebytes_max` | T | T | yes | yes |
| 189 | `crypto_aead_xchacha20poly1305_ietf_npubbytes` | T | T | yes | yes |
| 190 | `crypto_aead_xchacha20poly1305_ietf_nsecbytes` | T | T | yes | yes |
| 191 | `crypto_auth` | T | T | yes | yes |
| 192 | `crypto_auth_bytes` | T | T | yes | yes |
| 193 | `crypto_auth_hmacsha256` | T | T | yes | yes |
| 194 | `crypto_auth_hmacsha256_bytes` | T | T | yes | yes |
| 195 | `crypto_auth_hmacsha256_final` | T | T | yes | yes |
| 196 | `crypto_auth_hmacsha256_init` | T | T | yes | yes |
| 197 | `crypto_auth_hmacsha256_keybytes` | T | T | yes | yes |
| 198 | `crypto_auth_hmacsha256_keygen` | T | T | yes | yes |
| 199 | `crypto_auth_hmacsha256_statebytes` | T | T | yes | yes |
| 200 | `crypto_auth_hmacsha256_update` | T | T | yes | yes |
| 201 | `crypto_auth_hmacsha256_verify` | T | T | yes | yes |
| 202 | `crypto_auth_hmacsha512` | T | T | yes | yes |
| 203 | `crypto_auth_hmacsha512256` | T | T | yes | yes |
| 204 | `crypto_auth_hmacsha512256_bytes` | T | T | yes | yes |
| 205 | `crypto_auth_hmacsha512256_final` | T | T | yes | yes |
| 206 | `crypto_auth_hmacsha512256_init` | T | T | yes | yes |
| 207 | `crypto_auth_hmacsha512256_keybytes` | T | T | yes | yes |
| 208 | `crypto_auth_hmacsha512256_keygen` | T | T | yes | yes |
| 209 | `crypto_auth_hmacsha512256_statebytes` | T | T | yes | yes |
| 210 | `crypto_auth_hmacsha512256_update` | T | T | yes | yes |
| 211 | `crypto_auth_hmacsha512256_verify` | T | T | yes | yes |
| 212 | `crypto_auth_hmacsha512_bytes` | T | T | yes | yes |
| 213 | `crypto_auth_hmacsha512_final` | T | T | yes | yes |
| 214 | `crypto_auth_hmacsha512_init` | T | T | yes | yes |
| 215 | `crypto_auth_hmacsha512_keybytes` | T | T | yes | yes |
| 216 | `crypto_auth_hmacsha512_keygen` | T | T | yes | yes |
| 217 | `crypto_auth_hmacsha512_statebytes` | T | T | yes | yes |
| 218 | `crypto_auth_hmacsha512_update` | T | T | yes | yes |
| 219 | `crypto_auth_hmacsha512_verify` | T | T | yes | yes |
| 220 | `crypto_auth_keybytes` | T | T | yes | yes |
| 221 | `crypto_auth_keygen` | T | T | yes | yes |
| 222 | `crypto_auth_primitive` | T | T | yes | yes |
| 223 | `crypto_auth_verify` | T | T | yes | yes |
| 224 | `crypto_box` | T | T | yes | yes |
| 225 | `crypto_box_afternm` | T | T | yes | yes |
| 226 | `crypto_box_beforenm` | T | T | yes | yes |
| 227 | `crypto_box_beforenmbytes` | T | T | yes | yes |
| 228 | `crypto_box_boxzerobytes` | T | T | yes | yes |
| 229 | `crypto_box_curve25519xchacha20poly1305_beforenm` | T | T | yes | yes |
| 230 | `crypto_box_curve25519xchacha20poly1305_beforenmbytes` | T | T | yes | yes |
| 231 | `crypto_box_curve25519xchacha20poly1305_detached` | T | T | yes | yes |
| 232 | `crypto_box_curve25519xchacha20poly1305_detached_afternm` | T | T | yes | yes |
| 233 | `crypto_box_curve25519xchacha20poly1305_easy` | T | T | yes | yes |
| 234 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | T | T | yes | yes |
| 235 | `crypto_box_curve25519xchacha20poly1305_keypair` | T | T | yes | yes |
| 236 | `crypto_box_curve25519xchacha20poly1305_macbytes` | T | T | yes | yes |
| 237 | `crypto_box_curve25519xchacha20poly1305_messagebytes_max` | T | T | yes | yes |
| 238 | `crypto_box_curve25519xchacha20poly1305_noncebytes` | T | T | yes | yes |
| 239 | `crypto_box_curve25519xchacha20poly1305_open_detached` | T | T | yes | yes |
| 240 | `crypto_box_curve25519xchacha20poly1305_open_detached_afternm` | T | T | yes | yes |
| 241 | `crypto_box_curve25519xchacha20poly1305_open_easy` | T | T | yes | yes |
| 242 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | T | T | yes | yes |
| 243 | `crypto_box_curve25519xchacha20poly1305_publickeybytes` | T | T | yes | yes |
| 244 | `crypto_box_curve25519xchacha20poly1305_seal` | T | T | yes | yes |
| 245 | `crypto_box_curve25519xchacha20poly1305_seal_open` | T | T | yes | yes |
| 246 | `crypto_box_curve25519xchacha20poly1305_sealbytes` | T | T | yes | yes |
| 247 | `crypto_box_curve25519xchacha20poly1305_secretkeybytes` | T | T | yes | yes |
| 248 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` | T | T | yes | yes |
| 249 | `crypto_box_curve25519xchacha20poly1305_seedbytes` | T | T | yes | yes |
| 250 | `crypto_box_curve25519xsalsa20poly1305` | T | T | yes | yes |
| 251 | `crypto_box_curve25519xsalsa20poly1305_afternm` | T | T | yes | yes |
| 252 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | T | T | yes | yes |
| 253 | `crypto_box_curve25519xsalsa20poly1305_beforenmbytes` | T | T | yes | yes |
| 254 | `crypto_box_curve25519xsalsa20poly1305_boxzerobytes` | T | T | yes | yes |
| 255 | `crypto_box_curve25519xsalsa20poly1305_keypair` | T | T | yes | yes |
| 256 | `crypto_box_curve25519xsalsa20poly1305_macbytes` | T | T | yes | yes |
| 257 | `crypto_box_curve25519xsalsa20poly1305_messagebytes_max` | T | T | yes | yes |
| 258 | `crypto_box_curve25519xsalsa20poly1305_noncebytes` | T | T | yes | yes |
| 259 | `crypto_box_curve25519xsalsa20poly1305_open` | T | T | yes | yes |
| 260 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | T | T | yes | yes |
| 261 | `crypto_box_curve25519xsalsa20poly1305_publickeybytes` | T | T | yes | yes |
| 262 | `crypto_box_curve25519xsalsa20poly1305_secretkeybytes` | T | T | yes | yes |
| 263 | `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | T | T | yes | yes |
| 264 | `crypto_box_curve25519xsalsa20poly1305_seedbytes` | T | T | yes | yes |
| 265 | `crypto_box_curve25519xsalsa20poly1305_zerobytes` | T | T | yes | yes |
| 266 | `crypto_box_detached` | T | T | yes | yes |
| 267 | `crypto_box_detached_afternm` | T | T | yes | yes |
| 268 | `crypto_box_easy` | T | T | yes | yes |
| 269 | `crypto_box_easy_afternm` | T | T | yes | yes |
| 270 | `crypto_box_keypair` | T | T | yes | yes |
| 271 | `crypto_box_macbytes` | T | T | yes | yes |
| 272 | `crypto_box_messagebytes_max` | T | T | yes | yes |
| 273 | `crypto_box_noncebytes` | T | T | yes | yes |
| 274 | `crypto_box_open` | T | T | yes | yes |
| 275 | `crypto_box_open_afternm` | T | T | yes | yes |
| 276 | `crypto_box_open_detached` | T | T | yes | yes |
| 277 | `crypto_box_open_detached_afternm` | T | T | yes | yes |
| 278 | `crypto_box_open_easy` | T | T | yes | yes |
| 279 | `crypto_box_open_easy_afternm` | T | T | yes | yes |
| 280 | `crypto_box_primitive` | T | T | yes | yes |
| 281 | `crypto_box_publickeybytes` | T | T | yes | yes |
| 282 | `crypto_box_seal` | T | T | yes | yes |
| 283 | `crypto_box_seal_open` | T | T | yes | yes |
| 284 | `crypto_box_sealbytes` | T | T | yes | yes |
| 285 | `crypto_box_secretkeybytes` | T | T | yes | yes |
| 286 | `crypto_box_seed_keypair` | T | T | yes | yes |
| 287 | `crypto_box_seedbytes` | T | T | yes | yes |
| 288 | `crypto_box_zerobytes` | T | T | yes | yes |
| 289 | `crypto_core_ed25519_add` | T | T | yes | yes |
| 290 | `crypto_core_ed25519_bytes` | T | T | yes | yes |
| 291 | `crypto_core_ed25519_from_string` | T | T | yes | yes |
| 292 | `crypto_core_ed25519_from_string_nu` | T | T | yes | yes |
| 293 | `crypto_core_ed25519_hashbytes` | T | T | yes | yes |
| 294 | `crypto_core_ed25519_is_valid_point` | T | T | yes | yes |
| 295 | `crypto_core_ed25519_nonreducedscalarbytes` | T | T | yes | yes |
| 296 | `crypto_core_ed25519_random` | T | T | yes | yes |
| 297 | `crypto_core_ed25519_scalar_add` | T | T | yes | yes |
| 298 | `crypto_core_ed25519_scalar_complement` | T | T | yes | yes |
| 299 | `crypto_core_ed25519_scalar_from_string` | T | T | yes | yes |
| 300 | `crypto_core_ed25519_scalar_invert` | T | T | yes | yes |
| 301 | `crypto_core_ed25519_scalar_is_canonical` | T | T | yes | yes |
| 302 | `crypto_core_ed25519_scalar_mul` | T | T | yes | yes |
| 303 | `crypto_core_ed25519_scalar_negate` | T | T | yes | yes |
| 304 | `crypto_core_ed25519_scalar_random` | T | T | yes | yes |
| 305 | `crypto_core_ed25519_scalar_reduce` | T | T | yes | yes |
| 306 | `crypto_core_ed25519_scalar_sub` | T | T | yes | yes |
| 307 | `crypto_core_ed25519_scalarbytes` | T | T | yes | yes |
| 308 | `crypto_core_ed25519_sub` | T | T | yes | yes |
| 309 | `crypto_core_ed25519_uniformbytes` | T | T | yes | yes |
| 310 | `crypto_core_hchacha20` | T | T | yes | yes |
| 311 | `crypto_core_hchacha20_constbytes` | T | T | yes | yes |
| 312 | `crypto_core_hchacha20_inputbytes` | T | T | yes | yes |
| 313 | `crypto_core_hchacha20_keybytes` | T | T | yes | yes |
| 314 | `crypto_core_hchacha20_outputbytes` | T | T | yes | yes |
| 315 | `crypto_core_hsalsa20` | T | T | yes | yes |
| 316 | `crypto_core_hsalsa20_constbytes` | T | T | yes | yes |
| 317 | `crypto_core_hsalsa20_inputbytes` | T | T | yes | yes |
| 318 | `crypto_core_hsalsa20_keybytes` | T | T | yes | yes |
| 319 | `crypto_core_hsalsa20_outputbytes` | T | T | yes | yes |
| 320 | `crypto_core_keccak1600_extract_bytes` | T | T | yes | yes |
| 321 | `crypto_core_keccak1600_init` | T | T | yes | yes |
| 322 | `crypto_core_keccak1600_permute_12` | T | T | yes | yes |
| 323 | `crypto_core_keccak1600_permute_24` | T | T | yes | yes |
| 324 | `crypto_core_keccak1600_statebytes` | T | T | yes | yes |
| 325 | `crypto_core_keccak1600_xor_bytes` | T | T | yes | yes |
| 326 | `crypto_core_ristretto255_add` | T | T | yes | yes |
| 327 | `crypto_core_ristretto255_bytes` | T | T | yes | yes |
| 328 | `crypto_core_ristretto255_from_hash` | T | T | yes | yes |
| 329 | `crypto_core_ristretto255_from_string` | T | T | yes | yes |
| 330 | `crypto_core_ristretto255_hashbytes` | T | T | yes | yes |
| 331 | `crypto_core_ristretto255_is_valid_point` | T | T | yes | yes |
| 332 | `crypto_core_ristretto255_nonreducedscalarbytes` | T | T | yes | yes |
| 333 | `crypto_core_ristretto255_random` | T | T | yes | yes |
| 334 | `crypto_core_ristretto255_scalar_add` | T | T | yes | yes |
| 335 | `crypto_core_ristretto255_scalar_complement` | T | T | yes | yes |
| 336 | `crypto_core_ristretto255_scalar_from_string` | T | T | yes | yes |
| 337 | `crypto_core_ristretto255_scalar_invert` | T | T | yes | yes |
| 338 | `crypto_core_ristretto255_scalar_is_canonical` | T | T | yes | yes |
| 339 | `crypto_core_ristretto255_scalar_mul` | T | T | yes | yes |
| 340 | `crypto_core_ristretto255_scalar_negate` | T | T | yes | yes |
| 341 | `crypto_core_ristretto255_scalar_random` | T | T | yes | yes |
| 342 | `crypto_core_ristretto255_scalar_reduce` | T | T | yes | yes |
| 343 | `crypto_core_ristretto255_scalar_sub` | T | T | yes | yes |
| 344 | `crypto_core_ristretto255_scalarbytes` | T | T | yes | yes |
| 345 | `crypto_core_ristretto255_sub` | T | T | yes | yes |
| 346 | `crypto_core_salsa20` | T | T | yes | yes |
| 347 | `crypto_core_salsa2012` | T | T | yes | yes |
| 348 | `crypto_core_salsa2012_constbytes` | T | T | yes | yes |
| 349 | `crypto_core_salsa2012_inputbytes` | T | T | yes | yes |
| 350 | `crypto_core_salsa2012_keybytes` | T | T | yes | yes |
| 351 | `crypto_core_salsa2012_outputbytes` | T | T | yes | yes |
| 352 | `crypto_core_salsa208` | T | T | yes | yes |
| 353 | `crypto_core_salsa208_constbytes` | T | T | yes | yes |
| 354 | `crypto_core_salsa208_inputbytes` | T | T | yes | yes |
| 355 | `crypto_core_salsa208_keybytes` | T | T | yes | yes |
| 356 | `crypto_core_salsa208_outputbytes` | T | T | yes | yes |
| 357 | `crypto_core_salsa20_constbytes` | T | T | yes | yes |
| 358 | `crypto_core_salsa20_inputbytes` | T | T | yes | yes |
| 359 | `crypto_core_salsa20_keybytes` | T | T | yes | yes |
| 360 | `crypto_core_salsa20_outputbytes` | T | T | yes | yes |
| 361 | `crypto_generichash` | T | T | yes | yes |
| 362 | `crypto_generichash_blake2b` | T | T | yes | yes |
| 363 | `crypto_generichash_blake2b_bytes` | T | T | yes | yes |
| 364 | `crypto_generichash_blake2b_bytes_max` | T | T | yes | yes |
| 365 | `crypto_generichash_blake2b_bytes_min` | T | T | yes | yes |
| 366 | `crypto_generichash_blake2b_final` | T | T | yes | yes |
| 367 | `crypto_generichash_blake2b_init` | T | T | yes | yes |
| 368 | `crypto_generichash_blake2b_init_salt_personal` | T | T | yes | yes |
| 369 | `crypto_generichash_blake2b_keybytes` | T | T | yes | yes |
| 370 | `crypto_generichash_blake2b_keybytes_max` | T | T | yes | yes |
| 371 | `crypto_generichash_blake2b_keybytes_min` | T | T | yes | yes |
| 372 | `crypto_generichash_blake2b_keygen` | T | T | yes | yes |
| 373 | `crypto_generichash_blake2b_personalbytes` | T | T | yes | yes |
| 374 | `crypto_generichash_blake2b_salt_personal` | T | T | yes | yes |
| 375 | `crypto_generichash_blake2b_saltbytes` | T | T | yes | yes |
| 376 | `crypto_generichash_blake2b_statebytes` | T | T | yes | yes |
| 377 | `crypto_generichash_blake2b_update` | T | T | yes | yes |
| 378 | `crypto_generichash_bytes` | T | T | yes | yes |
| 379 | `crypto_generichash_bytes_max` | T | T | yes | yes |
| 380 | `crypto_generichash_bytes_min` | T | T | yes | yes |
| 381 | `crypto_generichash_final` | T | T | yes | yes |
| 382 | `crypto_generichash_init` | T | T | yes | yes |
| 383 | `crypto_generichash_keybytes` | T | T | yes | yes |
| 384 | `crypto_generichash_keybytes_max` | T | T | yes | yes |
| 385 | `crypto_generichash_keybytes_min` | T | T | yes | yes |
| 386 | `crypto_generichash_keygen` | T | T | yes | yes |
| 387 | `crypto_generichash_primitive` | T | T | yes | yes |
| 388 | `crypto_generichash_statebytes` | T | T | yes | yes |
| 389 | `crypto_generichash_update` | T | T | yes | yes |
| 390 | `crypto_hash` | T | T | yes | yes |
| 391 | `crypto_hash_bytes` | T | T | yes | yes |
| 392 | `crypto_hash_primitive` | T | T | yes | yes |
| 393 | `crypto_hash_sha256` | T | T | yes | yes |
| 394 | `crypto_hash_sha256_bytes` | T | T | yes | yes |
| 395 | `crypto_hash_sha256_final` | T | T | yes | yes |
| 396 | `crypto_hash_sha256_init` | T | T | yes | yes |
| 397 | `crypto_hash_sha256_statebytes` | T | T | yes | yes |
| 398 | `crypto_hash_sha256_update` | T | T | yes | yes |
| 399 | `crypto_hash_sha3256` | T | T | yes | yes |
| 400 | `crypto_hash_sha3256_bytes` | T | T | yes | yes |
| 401 | `crypto_hash_sha3256_final` | T | T | yes | yes |
| 402 | `crypto_hash_sha3256_init` | T | T | yes | yes |
| 403 | `crypto_hash_sha3256_statebytes` | T | T | yes | yes |
| 404 | `crypto_hash_sha3256_update` | T | T | yes | yes |
| 405 | `crypto_hash_sha3512` | T | T | yes | yes |
| 406 | `crypto_hash_sha3512_bytes` | T | T | yes | yes |
| 407 | `crypto_hash_sha3512_final` | T | T | yes | yes |
| 408 | `crypto_hash_sha3512_init` | T | T | yes | yes |
| 409 | `crypto_hash_sha3512_statebytes` | T | T | yes | yes |
| 410 | `crypto_hash_sha3512_update` | T | T | yes | yes |
| 411 | `crypto_hash_sha512` | T | T | yes | yes |
| 412 | `crypto_hash_sha512_bytes` | T | T | yes | yes |
| 413 | `crypto_hash_sha512_final` | T | T | yes | yes |
| 414 | `crypto_hash_sha512_init` | T | T | yes | yes |
| 415 | `crypto_hash_sha512_statebytes` | T | T | yes | yes |
| 416 | `crypto_hash_sha512_update` | T | T | yes | yes |
| 417 | `crypto_ipcrypt_bytes` | T | T | yes | yes |
| 418 | `crypto_ipcrypt_decrypt` | T | T | yes | yes |
| 419 | `crypto_ipcrypt_encrypt` | T | T | yes | yes |
| 420 | `crypto_ipcrypt_keybytes` | T | T | yes | yes |
| 421 | `crypto_ipcrypt_keygen` | T | T | yes | yes |
| 422 | `crypto_ipcrypt_nd_decrypt` | T | T | yes | yes |
| 423 | `crypto_ipcrypt_nd_encrypt` | T | T | yes | yes |
| 424 | `crypto_ipcrypt_nd_inputbytes` | T | T | yes | yes |
| 425 | `crypto_ipcrypt_nd_keybytes` | T | T | yes | yes |
| 426 | `crypto_ipcrypt_nd_keygen` | T | T | yes | yes |
| 427 | `crypto_ipcrypt_nd_outputbytes` | T | T | yes | yes |
| 428 | `crypto_ipcrypt_nd_tweakbytes` | T | T | yes | yes |
| 429 | `crypto_ipcrypt_ndx_decrypt` | T | T | yes | yes |
| 430 | `crypto_ipcrypt_ndx_encrypt` | T | T | yes | yes |
| 431 | `crypto_ipcrypt_ndx_inputbytes` | T | T | yes | yes |
| 432 | `crypto_ipcrypt_ndx_keybytes` | T | T | yes | yes |
| 433 | `crypto_ipcrypt_ndx_keygen` | T | T | yes | yes |
| 434 | `crypto_ipcrypt_ndx_outputbytes` | T | T | yes | yes |
| 435 | `crypto_ipcrypt_ndx_tweakbytes` | T | T | yes | yes |
| 436 | `crypto_ipcrypt_pfx_bytes` | T | T | yes | yes |
| 437 | `crypto_ipcrypt_pfx_decrypt` | T | T | yes | yes |
| 438 | `crypto_ipcrypt_pfx_encrypt` | T | T | yes | yes |
| 439 | `crypto_ipcrypt_pfx_keybytes` | T | T | yes | yes |
| 440 | `crypto_ipcrypt_pfx_keygen` | T | T | yes | yes |
| 441 | `crypto_kdf_blake2b_bytes_max` | T | T | yes | yes |
| 442 | `crypto_kdf_blake2b_bytes_min` | T | T | yes | yes |
| 443 | `crypto_kdf_blake2b_contextbytes` | T | T | yes | yes |
| 444 | `crypto_kdf_blake2b_derive_from_key` | T | T | yes | yes |
| 445 | `crypto_kdf_blake2b_keybytes` | T | T | yes | yes |
| 446 | `crypto_kdf_bytes_max` | T | T | yes | yes |
| 447 | `crypto_kdf_bytes_min` | T | T | yes | yes |
| 448 | `crypto_kdf_contextbytes` | T | T | yes | yes |
| 449 | `crypto_kdf_derive_from_key` | T | T | yes | yes |
| 450 | `crypto_kdf_hkdf_sha256_bytes_max` | T | T | yes | yes |
| 451 | `crypto_kdf_hkdf_sha256_bytes_min` | T | T | yes | yes |
| 452 | `crypto_kdf_hkdf_sha256_expand` | T | T | yes | yes |
| 453 | `crypto_kdf_hkdf_sha256_extract` | T | T | yes | yes |
| 454 | `crypto_kdf_hkdf_sha256_extract_final` | T | T | yes | yes |
| 455 | `crypto_kdf_hkdf_sha256_extract_init` | T | T | yes | yes |
| 456 | `crypto_kdf_hkdf_sha256_extract_update` | T | T | yes | yes |
| 457 | `crypto_kdf_hkdf_sha256_keybytes` | T | T | yes | yes |
| 458 | `crypto_kdf_hkdf_sha256_keygen` | T | T | yes | yes |
| 459 | `crypto_kdf_hkdf_sha256_statebytes` | T | T | yes | yes |
| 460 | `crypto_kdf_hkdf_sha512_bytes_max` | T | T | yes | yes |
| 461 | `crypto_kdf_hkdf_sha512_bytes_min` | T | T | yes | yes |
| 462 | `crypto_kdf_hkdf_sha512_expand` | T | T | yes | yes |
| 463 | `crypto_kdf_hkdf_sha512_extract` | T | T | yes | yes |
| 464 | `crypto_kdf_hkdf_sha512_extract_final` | T | T | yes | yes |
| 465 | `crypto_kdf_hkdf_sha512_extract_init` | T | T | yes | yes |
| 466 | `crypto_kdf_hkdf_sha512_extract_update` | T | T | yes | yes |
| 467 | `crypto_kdf_hkdf_sha512_keybytes` | T | T | yes | yes |
| 468 | `crypto_kdf_hkdf_sha512_keygen` | T | T | yes | yes |
| 469 | `crypto_kdf_hkdf_sha512_statebytes` | T | T | yes | yes |
| 470 | `crypto_kdf_keybytes` | T | T | yes | yes |
| 471 | `crypto_kdf_keygen` | T | T | yes | yes |
| 472 | `crypto_kdf_primitive` | T | T | yes | yes |
| 473 | `crypto_kem_ciphertextbytes` | T | T | yes | yes |
| 474 | `crypto_kem_dec` | T | T | yes | yes |
| 475 | `crypto_kem_enc` | T | T | yes | yes |
| 476 | `crypto_kem_keypair` | T | T | yes | yes |
| 477 | `crypto_kem_mlkem768_ciphertextbytes` | T | T | yes | yes |
| 478 | `crypto_kem_mlkem768_dec` | T | T | yes | yes |
| 479 | `crypto_kem_mlkem768_enc` | T | T | yes | yes |
| 480 | `crypto_kem_mlkem768_enc_deterministic` | T | T | yes | yes |
| 481 | `crypto_kem_mlkem768_keypair` | T | T | yes | yes |
| 482 | `crypto_kem_mlkem768_publickeybytes` | T | T | yes | yes |
| 483 | `crypto_kem_mlkem768_secretkeybytes` | T | T | yes | yes |
| 484 | `crypto_kem_mlkem768_seed_keypair` | T | T | yes | yes |
| 485 | `crypto_kem_mlkem768_seedbytes` | T | T | yes | yes |
| 486 | `crypto_kem_mlkem768_sharedsecretbytes` | T | T | yes | yes |
| 487 | `crypto_kem_primitive` | T | T | yes | yes |
| 488 | `crypto_kem_publickeybytes` | T | T | yes | yes |
| 489 | `crypto_kem_secretkeybytes` | T | T | yes | yes |
| 490 | `crypto_kem_seed_keypair` | T | T | yes | yes |
| 491 | `crypto_kem_seedbytes` | T | T | yes | yes |
| 492 | `crypto_kem_sharedsecretbytes` | T | T | yes | yes |
| 493 | `crypto_kem_xwing_ciphertextbytes` | T | T | yes | yes |
| 494 | `crypto_kem_xwing_dec` | T | T | yes | yes |
| 495 | `crypto_kem_xwing_enc` | T | T | yes | yes |
| 496 | `crypto_kem_xwing_enc_deterministic` | T | T | yes | yes |
| 497 | `crypto_kem_xwing_keypair` | T | T | yes | yes |
| 498 | `crypto_kem_xwing_publickeybytes` | T | T | yes | yes |
| 499 | `crypto_kem_xwing_secretkeybytes` | T | T | yes | yes |
| 500 | `crypto_kem_xwing_seed_keypair` | T | T | yes | yes |
| 501 | `crypto_kem_xwing_seedbytes` | T | T | yes | yes |
| 502 | `crypto_kem_xwing_sharedsecretbytes` | T | T | yes | yes |
| 503 | `crypto_kx_client_session_keys` | T | T | yes | yes |
| 504 | `crypto_kx_keypair` | T | T | yes | yes |
| 505 | `crypto_kx_primitive` | T | T | yes | yes |
| 506 | `crypto_kx_publickeybytes` | T | T | yes | yes |
| 507 | `crypto_kx_secretkeybytes` | T | T | yes | yes |
| 508 | `crypto_kx_seed_keypair` | T | T | yes | yes |
| 509 | `crypto_kx_seedbytes` | T | T | yes | yes |
| 510 | `crypto_kx_server_session_keys` | T | T | yes | yes |
| 511 | `crypto_kx_sessionkeybytes` | T | T | yes | yes |
| 512 | `crypto_onetimeauth` | T | T | yes | yes |
| 513 | `crypto_onetimeauth_bytes` | T | T | yes | yes |
| 514 | `crypto_onetimeauth_final` | T | T | yes | yes |
| 515 | `crypto_onetimeauth_init` | T | T | yes | yes |
| 516 | `crypto_onetimeauth_keybytes` | T | T | yes | yes |
| 517 | `crypto_onetimeauth_keygen` | T | T | yes | yes |
| 518 | `crypto_onetimeauth_poly1305` | T | T | yes | yes |
| 519 | `crypto_onetimeauth_poly1305_bytes` | T | T | yes | yes |
| 520 | `crypto_onetimeauth_poly1305_donna_implementation` | D | D | yes | yes |
| 521 | `crypto_onetimeauth_poly1305_final` | T | T | yes | yes |
| 522 | `crypto_onetimeauth_poly1305_init` | T | T | yes | yes |
| 523 | `crypto_onetimeauth_poly1305_keybytes` | T | T | yes | yes |
| 524 | `crypto_onetimeauth_poly1305_keygen` | T | T | yes | yes |
| 525 | `crypto_onetimeauth_poly1305_statebytes` | T | T | yes | yes |
| 526 | `crypto_onetimeauth_poly1305_update` | T | T | yes | yes |
| 527 | `crypto_onetimeauth_poly1305_verify` | T | T | yes | yes |
| 528 | `crypto_onetimeauth_primitive` | T | T | yes | yes |
| 529 | `crypto_onetimeauth_statebytes` | T | T | yes | yes |
| 530 | `crypto_onetimeauth_update` | T | T | yes | yes |
| 531 | `crypto_onetimeauth_verify` | T | T | yes | yes |
| 532 | `crypto_pwhash` | T | T | yes | yes |
| 533 | `crypto_pwhash_alg_argon2i13` | T | T | yes | yes |
| 534 | `crypto_pwhash_alg_argon2id13` | T | T | yes | yes |
| 535 | `crypto_pwhash_alg_default` | T | T | yes | yes |
| 536 | `crypto_pwhash_argon2i` | T | T | yes | yes |
| 537 | `crypto_pwhash_argon2i_alg_argon2i13` | T | T | yes | yes |
| 538 | `crypto_pwhash_argon2i_bytes_max` | T | T | yes | yes |
| 539 | `crypto_pwhash_argon2i_bytes_min` | T | T | yes | yes |
| 540 | `crypto_pwhash_argon2i_memlimit_interactive` | T | T | yes | yes |
| 541 | `crypto_pwhash_argon2i_memlimit_max` | T | T | yes | yes |
| 542 | `crypto_pwhash_argon2i_memlimit_min` | T | T | yes | yes |
| 543 | `crypto_pwhash_argon2i_memlimit_moderate` | T | T | yes | yes |
| 544 | `crypto_pwhash_argon2i_memlimit_sensitive` | T | T | yes | yes |
| 545 | `crypto_pwhash_argon2i_opslimit_interactive` | T | T | yes | yes |
| 546 | `crypto_pwhash_argon2i_opslimit_max` | T | T | yes | yes |
| 547 | `crypto_pwhash_argon2i_opslimit_min` | T | T | yes | yes |
| 548 | `crypto_pwhash_argon2i_opslimit_moderate` | T | T | yes | yes |
| 549 | `crypto_pwhash_argon2i_opslimit_sensitive` | T | T | yes | yes |
| 550 | `crypto_pwhash_argon2i_passwd_max` | T | T | yes | yes |
| 551 | `crypto_pwhash_argon2i_passwd_min` | T | T | yes | yes |
| 552 | `crypto_pwhash_argon2i_saltbytes` | T | T | yes | yes |
| 553 | `crypto_pwhash_argon2i_str` | T | T | yes | yes |
| 554 | `crypto_pwhash_argon2i_str_needs_rehash` | T | T | yes | yes |
| 555 | `crypto_pwhash_argon2i_str_verify` | T | T | yes | yes |
| 556 | `crypto_pwhash_argon2i_strbytes` | T | T | yes | yes |
| 557 | `crypto_pwhash_argon2i_strprefix` | T | T | yes | yes |
| 558 | `crypto_pwhash_argon2id` | T | T | yes | yes |
| 559 | `crypto_pwhash_argon2id_alg_argon2id13` | T | T | yes | yes |
| 560 | `crypto_pwhash_argon2id_bytes_max` | T | T | yes | yes |
| 561 | `crypto_pwhash_argon2id_bytes_min` | T | T | yes | yes |
| 562 | `crypto_pwhash_argon2id_memlimit_interactive` | T | T | yes | yes |
| 563 | `crypto_pwhash_argon2id_memlimit_max` | T | T | yes | yes |
| 564 | `crypto_pwhash_argon2id_memlimit_min` | T | T | yes | yes |
| 565 | `crypto_pwhash_argon2id_memlimit_moderate` | T | T | yes | yes |
| 566 | `crypto_pwhash_argon2id_memlimit_sensitive` | T | T | yes | yes |
| 567 | `crypto_pwhash_argon2id_opslimit_interactive` | T | T | yes | yes |
| 568 | `crypto_pwhash_argon2id_opslimit_max` | T | T | yes | yes |
| 569 | `crypto_pwhash_argon2id_opslimit_min` | T | T | yes | yes |
| 570 | `crypto_pwhash_argon2id_opslimit_moderate` | T | T | yes | yes |
| 571 | `crypto_pwhash_argon2id_opslimit_sensitive` | T | T | yes | yes |
| 572 | `crypto_pwhash_argon2id_passwd_max` | T | T | yes | yes |
| 573 | `crypto_pwhash_argon2id_passwd_min` | T | T | yes | yes |
| 574 | `crypto_pwhash_argon2id_saltbytes` | T | T | yes | yes |
| 575 | `crypto_pwhash_argon2id_str` | T | T | yes | yes |
| 576 | `crypto_pwhash_argon2id_str_needs_rehash` | T | T | yes | yes |
| 577 | `crypto_pwhash_argon2id_str_verify` | T | T | yes | yes |
| 578 | `crypto_pwhash_argon2id_strbytes` | T | T | yes | yes |
| 579 | `crypto_pwhash_argon2id_strprefix` | T | T | yes | yes |
| 580 | `crypto_pwhash_bytes_max` | T | T | yes | yes |
| 581 | `crypto_pwhash_bytes_min` | T | T | yes | yes |
| 582 | `crypto_pwhash_memlimit_interactive` | T | T | yes | yes |
| 583 | `crypto_pwhash_memlimit_max` | T | T | yes | yes |
| 584 | `crypto_pwhash_memlimit_min` | T | T | yes | yes |
| 585 | `crypto_pwhash_memlimit_moderate` | T | T | yes | yes |
| 586 | `crypto_pwhash_memlimit_sensitive` | T | T | yes | yes |
| 587 | `crypto_pwhash_opslimit_interactive` | T | T | yes | yes |
| 588 | `crypto_pwhash_opslimit_max` | T | T | yes | yes |
| 589 | `crypto_pwhash_opslimit_min` | T | T | yes | yes |
| 590 | `crypto_pwhash_opslimit_moderate` | T | T | yes | yes |
| 591 | `crypto_pwhash_opslimit_sensitive` | T | T | yes | yes |
| 592 | `crypto_pwhash_passwd_max` | T | T | yes | yes |
| 593 | `crypto_pwhash_passwd_min` | T | T | yes | yes |
| 594 | `crypto_pwhash_primitive` | T | T | yes | yes |
| 595 | `crypto_pwhash_saltbytes` | T | T | yes | yes |
| 596 | `crypto_pwhash_scryptsalsa208sha256` | T | T | yes | yes |
| 597 | `crypto_pwhash_scryptsalsa208sha256_bytes_max` | T | T | yes | yes |
| 598 | `crypto_pwhash_scryptsalsa208sha256_bytes_min` | T | T | yes | yes |
| 599 | `crypto_pwhash_scryptsalsa208sha256_ll` | T | T | yes | yes |
| 600 | `crypto_pwhash_scryptsalsa208sha256_memlimit_interactive` | T | T | yes | yes |
| 601 | `crypto_pwhash_scryptsalsa208sha256_memlimit_max` | T | T | yes | yes |
| 602 | `crypto_pwhash_scryptsalsa208sha256_memlimit_min` | T | T | yes | yes |
| 603 | `crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive` | T | T | yes | yes |
| 604 | `crypto_pwhash_scryptsalsa208sha256_opslimit_interactive` | T | T | yes | yes |
| 605 | `crypto_pwhash_scryptsalsa208sha256_opslimit_max` | T | T | yes | yes |
| 606 | `crypto_pwhash_scryptsalsa208sha256_opslimit_min` | T | T | yes | yes |
| 607 | `crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive` | T | T | yes | yes |
| 608 | `crypto_pwhash_scryptsalsa208sha256_passwd_max` | T | T | yes | yes |
| 609 | `crypto_pwhash_scryptsalsa208sha256_passwd_min` | T | T | yes | yes |
| 610 | `crypto_pwhash_scryptsalsa208sha256_saltbytes` | T | T | yes | yes |
| 611 | `crypto_pwhash_scryptsalsa208sha256_str` | T | T | yes | yes |
| 612 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | T | T | yes | yes |
| 613 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | T | T | yes | yes |
| 614 | `crypto_pwhash_scryptsalsa208sha256_strbytes` | T | T | yes | yes |
| 615 | `crypto_pwhash_scryptsalsa208sha256_strprefix` | T | T | yes | yes |
| 616 | `crypto_pwhash_str` | T | T | yes | yes |
| 617 | `crypto_pwhash_str_alg` | T | T | yes | yes |
| 618 | `crypto_pwhash_str_needs_rehash` | T | T | yes | yes |
| 619 | `crypto_pwhash_str_verify` | T | T | yes | yes |
| 620 | `crypto_pwhash_strbytes` | T | T | yes | yes |
| 621 | `crypto_pwhash_strprefix` | T | T | yes | yes |
| 622 | `crypto_scalarmult` | T | T | yes | yes |
| 623 | `crypto_scalarmult_base` | T | T | yes | yes |
| 624 | `crypto_scalarmult_bytes` | T | T | yes | yes |
| 625 | `crypto_scalarmult_curve25519` | T | T | yes | yes |
| 626 | `crypto_scalarmult_curve25519_base` | T | T | yes | yes |
| 627 | `crypto_scalarmult_curve25519_bytes` | T | T | yes | yes |
| 628 | `crypto_scalarmult_curve25519_ref10_implementation` | D | D | yes | yes |
| 629 | `crypto_scalarmult_curve25519_scalarbytes` | T | T | yes | yes |
| 630 | `crypto_scalarmult_ed25519` | T | T | yes | yes |
| 631 | `crypto_scalarmult_ed25519_base` | T | T | yes | yes |
| 632 | `crypto_scalarmult_ed25519_base_noclamp` | T | T | yes | yes |
| 633 | `crypto_scalarmult_ed25519_bytes` | T | T | yes | yes |
| 634 | `crypto_scalarmult_ed25519_noclamp` | T | T | yes | yes |
| 635 | `crypto_scalarmult_ed25519_scalarbytes` | T | T | yes | yes |
| 636 | `crypto_scalarmult_primitive` | T | T | yes | yes |
| 637 | `crypto_scalarmult_ristretto255` | T | T | yes | yes |
| 638 | `crypto_scalarmult_ristretto255_base` | T | T | yes | yes |
| 639 | `crypto_scalarmult_ristretto255_bytes` | T | T | yes | yes |
| 640 | `crypto_scalarmult_ristretto255_scalarbytes` | T | T | yes | yes |
| 641 | `crypto_scalarmult_scalarbytes` | T | T | yes | yes |
| 642 | `crypto_secretbox` | T | T | yes | yes |
| 643 | `crypto_secretbox_boxzerobytes` | T | T | yes | yes |
| 644 | `crypto_secretbox_detached` | T | T | yes | yes |
| 645 | `crypto_secretbox_easy` | T | T | yes | yes |
| 646 | `crypto_secretbox_keybytes` | T | T | yes | yes |
| 647 | `crypto_secretbox_keygen` | T | T | yes | yes |
| 648 | `crypto_secretbox_macbytes` | T | T | yes | yes |
| 649 | `crypto_secretbox_messagebytes_max` | T | T | yes | yes |
| 650 | `crypto_secretbox_noncebytes` | T | T | yes | yes |
| 651 | `crypto_secretbox_open` | T | T | yes | yes |
| 652 | `crypto_secretbox_open_detached` | T | T | yes | yes |
| 653 | `crypto_secretbox_open_easy` | T | T | yes | yes |
| 654 | `crypto_secretbox_primitive` | T | T | yes | yes |
| 655 | `crypto_secretbox_xchacha20poly1305_detached` | T | T | yes | yes |
| 656 | `crypto_secretbox_xchacha20poly1305_easy` | T | T | yes | yes |
| 657 | `crypto_secretbox_xchacha20poly1305_keybytes` | T | T | yes | yes |
| 658 | `crypto_secretbox_xchacha20poly1305_macbytes` | T | T | yes | yes |
| 659 | `crypto_secretbox_xchacha20poly1305_messagebytes_max` | T | T | yes | yes |
| 660 | `crypto_secretbox_xchacha20poly1305_noncebytes` | T | T | yes | yes |
| 661 | `crypto_secretbox_xchacha20poly1305_open_detached` | T | T | yes | yes |
| 662 | `crypto_secretbox_xchacha20poly1305_open_easy` | T | T | yes | yes |
| 663 | `crypto_secretbox_xsalsa20poly1305` | T | T | yes | yes |
| 664 | `crypto_secretbox_xsalsa20poly1305_boxzerobytes` | T | T | yes | yes |
| 665 | `crypto_secretbox_xsalsa20poly1305_keybytes` | T | T | yes | yes |
| 666 | `crypto_secretbox_xsalsa20poly1305_keygen` | T | T | yes | yes |
| 667 | `crypto_secretbox_xsalsa20poly1305_macbytes` | T | T | yes | yes |
| 668 | `crypto_secretbox_xsalsa20poly1305_messagebytes_max` | T | T | yes | yes |
| 669 | `crypto_secretbox_xsalsa20poly1305_noncebytes` | T | T | yes | yes |
| 670 | `crypto_secretbox_xsalsa20poly1305_open` | T | T | yes | yes |
| 671 | `crypto_secretbox_xsalsa20poly1305_zerobytes` | T | T | yes | yes |
| 672 | `crypto_secretbox_zerobytes` | T | T | yes | yes |
| 673 | `crypto_secretstream_xchacha20poly1305_abytes` | T | T | yes | yes |
| 674 | `crypto_secretstream_xchacha20poly1305_headerbytes` | T | T | yes | yes |
| 675 | `crypto_secretstream_xchacha20poly1305_init_pull` | T | T | yes | yes |
| 676 | `crypto_secretstream_xchacha20poly1305_init_push` | T | T | yes | yes |
| 677 | `crypto_secretstream_xchacha20poly1305_keybytes` | T | T | yes | yes |
| 678 | `crypto_secretstream_xchacha20poly1305_keygen` | T | T | yes | yes |
| 679 | `crypto_secretstream_xchacha20poly1305_messagebytes_max` | T | T | yes | yes |
| 680 | `crypto_secretstream_xchacha20poly1305_pull` | T | T | yes | yes |
| 681 | `crypto_secretstream_xchacha20poly1305_push` | T | T | yes | yes |
| 682 | `crypto_secretstream_xchacha20poly1305_rekey` | T | T | yes | yes |
| 683 | `crypto_secretstream_xchacha20poly1305_statebytes` | T | T | yes | yes |
| 684 | `crypto_secretstream_xchacha20poly1305_tag_final` | T | T | yes | yes |
| 685 | `crypto_secretstream_xchacha20poly1305_tag_message` | T | T | yes | yes |
| 686 | `crypto_secretstream_xchacha20poly1305_tag_push` | T | T | yes | yes |
| 687 | `crypto_secretstream_xchacha20poly1305_tag_rekey` | T | T | yes | yes |
| 688 | `crypto_shorthash` | T | T | yes | yes |
| 689 | `crypto_shorthash_bytes` | T | T | yes | yes |
| 690 | `crypto_shorthash_keybytes` | T | T | yes | yes |
| 691 | `crypto_shorthash_keygen` | T | T | yes | yes |
| 692 | `crypto_shorthash_primitive` | T | T | yes | yes |
| 693 | `crypto_shorthash_siphash24` | T | T | yes | yes |
| 694 | `crypto_shorthash_siphash24_bytes` | T | T | yes | yes |
| 695 | `crypto_shorthash_siphash24_keybytes` | T | T | yes | yes |
| 696 | `crypto_shorthash_siphashx24` | T | T | yes | yes |
| 697 | `crypto_shorthash_siphashx24_bytes` | T | T | yes | yes |
| 698 | `crypto_shorthash_siphashx24_keybytes` | T | T | yes | yes |
| 699 | `crypto_sign` | T | T | yes | yes |
| 700 | `crypto_sign_bytes` | T | T | yes | yes |
| 701 | `crypto_sign_detached` | T | T | yes | yes |
| 702 | `crypto_sign_ed25519` | T | T | yes | yes |
| 703 | `crypto_sign_ed25519_bytes` | T | T | yes | yes |
| 704 | `crypto_sign_ed25519_detached` | T | T | yes | yes |
| 705 | `crypto_sign_ed25519_keypair` | T | T | yes | yes |
| 706 | `crypto_sign_ed25519_messagebytes_max` | T | T | yes | yes |
| 707 | `crypto_sign_ed25519_open` | T | T | yes | yes |
| 708 | `crypto_sign_ed25519_pk_to_curve25519` | T | T | yes | yes |
| 709 | `crypto_sign_ed25519_publickeybytes` | T | T | yes | yes |
| 710 | `crypto_sign_ed25519_secretkeybytes` | T | T | yes | yes |
| 711 | `crypto_sign_ed25519_seed_keypair` | T | T | yes | yes |
| 712 | `crypto_sign_ed25519_seedbytes` | T | T | yes | yes |
| 713 | `crypto_sign_ed25519_sk_to_curve25519` | T | T | yes | yes |
| 714 | `crypto_sign_ed25519_sk_to_pk` | T | T | yes | yes |
| 715 | `crypto_sign_ed25519_sk_to_seed` | T | T | yes | yes |
| 716 | `crypto_sign_ed25519_verify_detached` | T | T | yes | yes |
| 717 | `crypto_sign_ed25519ph_final_create` | T | T | yes | yes |
| 718 | `crypto_sign_ed25519ph_final_verify` | T | T | yes | yes |
| 719 | `crypto_sign_ed25519ph_init` | T | T | yes | yes |
| 720 | `crypto_sign_ed25519ph_statebytes` | T | T | yes | yes |
| 721 | `crypto_sign_ed25519ph_update` | T | T | yes | yes |
| 722 | `crypto_sign_final_create` | T | T | yes | yes |
| 723 | `crypto_sign_final_verify` | T | T | yes | yes |
| 724 | `crypto_sign_init` | T | T | yes | yes |
| 725 | `crypto_sign_keypair` | T | T | yes | yes |
| 726 | `crypto_sign_messagebytes_max` | T | T | yes | yes |
| 727 | `crypto_sign_open` | T | T | yes | yes |
| 728 | `crypto_sign_primitive` | T | T | yes | yes |
| 729 | `crypto_sign_publickeybytes` | T | T | yes | yes |
| 730 | `crypto_sign_secretkeybytes` | T | T | yes | yes |
| 731 | `crypto_sign_seed_keypair` | T | T | yes | yes |
| 732 | `crypto_sign_seedbytes` | T | T | yes | yes |
| 733 | `crypto_sign_statebytes` | T | T | yes | yes |
| 734 | `crypto_sign_update` | T | T | yes | yes |
| 735 | `crypto_sign_verify_detached` | T | T | yes | yes |
| 736 | `crypto_stream` | T | T | yes | yes |
| 737 | `crypto_stream_chacha20` | T | T | yes | yes |
| 738 | `crypto_stream_chacha20_ietf` | T | T | yes | yes |
| 739 | `crypto_stream_chacha20_ietf_ext` | T | T | yes | yes |
| 740 | `crypto_stream_chacha20_ietf_ext_xor_ic` | T | T | yes | yes |
| 741 | `crypto_stream_chacha20_ietf_keybytes` | T | T | yes | yes |
| 742 | `crypto_stream_chacha20_ietf_keygen` | T | T | yes | yes |
| 743 | `crypto_stream_chacha20_ietf_messagebytes_max` | T | T | yes | yes |
| 744 | `crypto_stream_chacha20_ietf_noncebytes` | T | T | yes | yes |
| 745 | `crypto_stream_chacha20_ietf_xor` | T | T | yes | yes |
| 746 | `crypto_stream_chacha20_ietf_xor_ic` | T | T | yes | yes |
| 747 | `crypto_stream_chacha20_keybytes` | T | T | yes | yes |
| 748 | `crypto_stream_chacha20_keygen` | T | T | yes | yes |
| 749 | `crypto_stream_chacha20_messagebytes_max` | T | T | yes | yes |
| 750 | `crypto_stream_chacha20_noncebytes` | T | T | yes | yes |
| 751 | `crypto_stream_chacha20_ref_implementation` | D | D | yes | yes |
| 752 | `crypto_stream_chacha20_xor` | T | T | yes | yes |
| 753 | `crypto_stream_chacha20_xor_ic` | T | T | yes | yes |
| 754 | `crypto_stream_keybytes` | T | T | yes | yes |
| 755 | `crypto_stream_keygen` | T | T | yes | yes |
| 756 | `crypto_stream_messagebytes_max` | T | T | yes | yes |
| 757 | `crypto_stream_noncebytes` | T | T | yes | yes |
| 758 | `crypto_stream_primitive` | T | T | yes | yes |
| 759 | `crypto_stream_salsa20` | T | T | yes | yes |
| 760 | `crypto_stream_salsa2012` | T | T | yes | yes |
| 761 | `crypto_stream_salsa2012_keybytes` | T | T | yes | yes |
| 762 | `crypto_stream_salsa2012_keygen` | T | T | yes | yes |
| 763 | `crypto_stream_salsa2012_messagebytes_max` | T | T | yes | yes |
| 764 | `crypto_stream_salsa2012_noncebytes` | T | T | yes | yes |
| 765 | `crypto_stream_salsa2012_xor` | T | T | yes | yes |
| 766 | `crypto_stream_salsa208` | T | T | yes | yes |
| 767 | `crypto_stream_salsa208_keybytes` | T | T | yes | yes |
| 768 | `crypto_stream_salsa208_keygen` | T | T | yes | yes |
| 769 | `crypto_stream_salsa208_messagebytes_max` | T | T | yes | yes |
| 770 | `crypto_stream_salsa208_noncebytes` | T | T | yes | yes |
| 771 | `crypto_stream_salsa208_xor` | T | T | yes | yes |
| 772 | `crypto_stream_salsa20_keybytes` | T | T | yes | yes |
| 773 | `crypto_stream_salsa20_keygen` | T | T | yes | yes |
| 774 | `crypto_stream_salsa20_messagebytes_max` | T | T | yes | yes |
| 775 | `crypto_stream_salsa20_noncebytes` | T | T | yes | yes |
| 776 | `crypto_stream_salsa20_ref_implementation` | D | D | yes | yes |
| 777 | `crypto_stream_salsa20_xor` | T | T | yes | yes |
| 778 | `crypto_stream_salsa20_xor_ic` | T | T | yes | yes |
| 779 | `crypto_stream_xchacha20` | T | T | yes | yes |
| 780 | `crypto_stream_xchacha20_keybytes` | T | T | yes | yes |
| 781 | `crypto_stream_xchacha20_keygen` | T | T | yes | yes |
| 782 | `crypto_stream_xchacha20_messagebytes_max` | T | T | yes | yes |
| 783 | `crypto_stream_xchacha20_noncebytes` | T | T | yes | yes |
| 784 | `crypto_stream_xchacha20_xor` | T | T | yes | yes |
| 785 | `crypto_stream_xchacha20_xor_ic` | T | T | yes | yes |
| 786 | `crypto_stream_xor` | T | T | yes | yes |
| 787 | `crypto_stream_xsalsa20` | T | T | yes | yes |
| 788 | `crypto_stream_xsalsa20_keybytes` | T | T | yes | yes |
| 789 | `crypto_stream_xsalsa20_keygen` | T | T | yes | yes |
| 790 | `crypto_stream_xsalsa20_messagebytes_max` | T | T | yes | yes |
| 791 | `crypto_stream_xsalsa20_noncebytes` | T | T | yes | yes |
| 792 | `crypto_stream_xsalsa20_xor` | T | T | yes | yes |
| 793 | `crypto_stream_xsalsa20_xor_ic` | T | T | yes | yes |
| 794 | `crypto_verify_16` | T | T | yes | yes |
| 795 | `crypto_verify_16_bytes` | T | T | yes | yes |
| 796 | `crypto_verify_32` | T | T | yes | yes |
| 797 | `crypto_verify_32_bytes` | T | T | yes | yes |
| 798 | `crypto_verify_64` | T | T | yes | yes |
| 799 | `crypto_verify_64_bytes` | T | T | yes | yes |
| 800 | `crypto_xof_shake128` | T | T | yes | yes |
| 801 | `crypto_xof_shake128_blockbytes` | T | T | yes | yes |
| 802 | `crypto_xof_shake128_domain_standard` | T | T | yes | yes |
| 803 | `crypto_xof_shake128_init` | T | T | yes | yes |
| 804 | `crypto_xof_shake128_init_with_domain` | T | T | yes | yes |
| 805 | `crypto_xof_shake128_squeeze` | T | T | yes | yes |
| 806 | `crypto_xof_shake128_statebytes` | T | T | yes | yes |
| 807 | `crypto_xof_shake128_update` | T | T | yes | yes |
| 808 | `crypto_xof_shake256` | T | T | yes | yes |
| 809 | `crypto_xof_shake256_blockbytes` | T | T | yes | yes |
| 810 | `crypto_xof_shake256_domain_standard` | T | T | yes | yes |
| 811 | `crypto_xof_shake256_init` | T | T | yes | yes |
| 812 | `crypto_xof_shake256_init_with_domain` | T | T | yes | yes |
| 813 | `crypto_xof_shake256_squeeze` | T | T | yes | yes |
| 814 | `crypto_xof_shake256_statebytes` | T | T | yes | yes |
| 815 | `crypto_xof_shake256_update` | T | T | yes | yes |
| 816 | `crypto_xof_turboshake128` | T | T | yes | yes |
| 817 | `crypto_xof_turboshake128_blockbytes` | T | T | yes | yes |
| 818 | `crypto_xof_turboshake128_domain_standard` | T | T | yes | yes |
| 819 | `crypto_xof_turboshake128_init` | T | T | yes | yes |
| 820 | `crypto_xof_turboshake128_init_with_domain` | T | T | yes | yes |
| 821 | `crypto_xof_turboshake128_squeeze` | T | T | yes | yes |
| 822 | `crypto_xof_turboshake128_statebytes` | T | T | yes | yes |
| 823 | `crypto_xof_turboshake128_update` | T | T | yes | yes |
| 824 | `crypto_xof_turboshake256` | T | T | yes | yes |
| 825 | `crypto_xof_turboshake256_blockbytes` | T | T | yes | yes |
| 826 | `crypto_xof_turboshake256_domain_standard` | T | T | yes | yes |
| 827 | `crypto_xof_turboshake256_init` | T | T | yes | yes |
| 828 | `crypto_xof_turboshake256_init_with_domain` | T | T | yes | yes |
| 829 | `crypto_xof_turboshake256_squeeze` | T | T | yes | yes |
| 830 | `crypto_xof_turboshake256_statebytes` | T | T | yes | yes |
| 831 | `crypto_xof_turboshake256_update` | T | T | yes | yes |
| 832 | `ipcrypt_soft_implementation` | D | D | yes | yes |
| 833 | `randombytes` | T | T | yes | yes |
| 834 | `randombytes_buf` | T | T | yes | yes |
| 835 | `randombytes_buf_deterministic` | T | T | yes | yes |
| 836 | `randombytes_close` | T | T | yes | yes |
| 837 | `randombytes_implementation_name` | T | T | yes | yes |
| 838 | `randombytes_internal_implementation` | D | D | yes | yes |
| 839 | `randombytes_random` | T | T | yes | yes |
| 840 | `randombytes_seedbytes` | T | T | yes | yes |
| 841 | `randombytes_set_implementation` | T | T | yes | yes |
| 842 | `randombytes_stir` | T | T | yes | yes |
| 843 | `randombytes_sysrandom_implementation` | D | D | yes | yes |
| 844 | `randombytes_uniform` | T | T | yes | yes |
| 845 | `sodium_add` | T | T | yes | yes |
| 846 | `sodium_allocarray` | T | T | yes | yes |
| 847 | `sodium_base642bin` | T | T | yes | yes |
| 848 | `sodium_base64_encoded_len` | T | T | yes | yes |
| 849 | `sodium_bin2base64` | T | T | yes | yes |
| 850 | `sodium_bin2hex` | T | T | yes | yes |
| 851 | `sodium_bin2ip` | T | T | yes | yes |
| 852 | `sodium_compare` | T | T | yes | yes |
| 853 | `sodium_crit_enter` | T | T | yes | yes |
| 854 | `sodium_crit_leave` | T | T | yes | yes |
| 855 | `sodium_free` | T | T | yes | yes |
| 856 | `sodium_hex2bin` | T | T | yes | yes |
| 857 | `sodium_increment` | T | T | yes | yes |
| 858 | `sodium_init` | T | T | yes | yes |
| 859 | `sodium_ip2bin` | T | T | yes | yes |
| 860 | `sodium_is_zero` | T | T | yes | yes |
| 861 | `sodium_library_minimal` | T | T | yes | yes |
| 862 | `sodium_library_version_major` | T | T | yes | yes |
| 863 | `sodium_library_version_minor` | T | T | yes | yes |
| 864 | `sodium_malloc` | T | T | yes | yes |
| 865 | `sodium_memcmp` | T | T | yes | yes |
| 866 | `sodium_memzero` | T | T | yes | yes |
| 867 | `sodium_misuse` | T | T | yes | yes |
| 868 | `sodium_mlock` | T | T | yes | yes |
| 869 | `sodium_mprotect_noaccess` | T | T | yes | yes |
| 870 | `sodium_mprotect_readonly` | T | T | yes | yes |
| 871 | `sodium_mprotect_readwrite` | T | T | yes | yes |
| 872 | `sodium_munlock` | T | T | yes | yes |
| 873 | `sodium_pad` | T | T | yes | yes |
| 874 | `sodium_runtime_has_aesni` | W | T | yes | yes |
| 875 | `sodium_runtime_has_armcrypto` | W | T | yes | yes |
| 876 | `sodium_runtime_has_avx` | W | T | yes | yes |
| 877 | `sodium_runtime_has_avx2` | W | T | yes | yes |
| 878 | `sodium_runtime_has_avx512f` | W | T | yes | yes |
| 879 | `sodium_runtime_has_neon` | W | T | yes | yes |
| 880 | `sodium_runtime_has_pclmul` | W | T | yes | yes |
| 881 | `sodium_runtime_has_rdrand` | W | T | yes | yes |
| 882 | `sodium_runtime_has_sse2` | W | T | yes | yes |
| 883 | `sodium_runtime_has_sse3` | W | T | yes | yes |
| 884 | `sodium_runtime_has_sse41` | W | T | yes | yes |
| 885 | `sodium_runtime_has_ssse3` | W | T | yes | yes |
| 886 | `sodium_set_misuse_handler` | T | T | yes | yes |
| 887 | `sodium_stackzero` | T | T | yes | yes |
| 888 | `sodium_sub` | T | T | yes | yes |
| 889 | `sodium_unpad` | T | T | yes | yes |
| 890 | `sodium_version_string` | T | T | yes | yes |
