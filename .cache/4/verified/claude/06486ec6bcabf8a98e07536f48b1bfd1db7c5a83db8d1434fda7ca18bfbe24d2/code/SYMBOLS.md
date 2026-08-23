# SYMBOLS.md — exported symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from:
```
nm -D --defined-only c_src/build/libjansson.so
nm -D --defined-only target/release/libjansson.so
```

* C  `.so` exported symbols: **130**
* Rust `.so` exported symbols: **130**
* Missing from Rust: **0**
* Extra in Rust: **0**

## Full symbol table

| # | symbol | nm type (C) | in C .so | in Rust .so |
|---|--------|-------------|----------|-------------|
| 1 | `do_deep_copy` | T | yes | yes |
| 2 | `do_object_update_recursive` | T | yes | yes |
| 3 | `dtoa` | T | yes | yes |
| 4 | `dtoa_divmax` | D | yes | yes |
| 5 | `dtoa_r` | T | yes | yes |
| 6 | `freedtoa` | T | yes | yes |
| 7 | `gethex` | T | yes | yes |
| 8 | `hashtable_clear` | T | yes | yes |
| 9 | `hashtable_close` | T | yes | yes |
| 10 | `hashtable_del` | T | yes | yes |
| 11 | `hashtable_get` | T | yes | yes |
| 12 | `hashtable_init` | T | yes | yes |
| 13 | `hashtable_iter` | T | yes | yes |
| 14 | `hashtable_iter_at` | T | yes | yes |
| 15 | `hashtable_iter_key` | T | yes | yes |
| 16 | `hashtable_iter_key_len` | T | yes | yes |
| 17 | `hashtable_iter_next` | T | yes | yes |
| 18 | `hashtable_iter_set` | T | yes | yes |
| 19 | `hashtable_iter_value` | T | yes | yes |
| 20 | `hashtable_seed` | B | yes | yes |
| 21 | `hashtable_set` | T | yes | yes |
| 22 | `jansson_version_cmp` | T | yes | yes |
| 23 | `jansson_version_str` | T | yes | yes |
| 24 | `json_array` | T | yes | yes |
| 25 | `json_array_append_new` | T | yes | yes |
| 26 | `json_array_clear` | T | yes | yes |
| 27 | `json_array_extend` | T | yes | yes |
| 28 | `json_array_get` | T | yes | yes |
| 29 | `json_array_insert_new` | T | yes | yes |
| 30 | `json_array_remove` | T | yes | yes |
| 31 | `json_array_set_new` | T | yes | yes |
| 32 | `json_array_size` | T | yes | yes |
| 33 | `json_copy` | T | yes | yes |
| 34 | `json_deep_copy` | T | yes | yes |
| 35 | `json_delete` | T | yes | yes |
| 36 | `json_dump_callback` | T | yes | yes |
| 37 | `json_dump_file` | T | yes | yes |
| 38 | `json_dumpb` | T | yes | yes |
| 39 | `json_dumpf` | T | yes | yes |
| 40 | `json_dumpfd` | T | yes | yes |
| 41 | `json_dumps` | T | yes | yes |
| 42 | `json_equal` | T | yes | yes |
| 43 | `json_false` | T | yes | yes |
| 44 | `json_get_alloc_funcs` | T | yes | yes |
| 45 | `json_get_alloc_funcs2` | T | yes | yes |
| 46 | `json_integer` | T | yes | yes |
| 47 | `json_integer_set` | T | yes | yes |
| 48 | `json_integer_value` | T | yes | yes |
| 49 | `json_load_callback` | T | yes | yes |
| 50 | `json_load_file` | T | yes | yes |
| 51 | `json_loadb` | T | yes | yes |
| 52 | `json_loadf` | T | yes | yes |
| 53 | `json_loadfd` | T | yes | yes |
| 54 | `json_loads` | T | yes | yes |
| 55 | `json_null` | T | yes | yes |
| 56 | `json_number_value` | T | yes | yes |
| 57 | `json_object` | T | yes | yes |
| 58 | `json_object_clear` | T | yes | yes |
| 59 | `json_object_del` | T | yes | yes |
| 60 | `json_object_deln` | T | yes | yes |
| 61 | `json_object_get` | T | yes | yes |
| 62 | `json_object_getn` | T | yes | yes |
| 63 | `json_object_iter` | T | yes | yes |
| 64 | `json_object_iter_at` | T | yes | yes |
| 65 | `json_object_iter_key` | T | yes | yes |
| 66 | `json_object_iter_key_len` | T | yes | yes |
| 67 | `json_object_iter_next` | T | yes | yes |
| 68 | `json_object_iter_set_new` | T | yes | yes |
| 69 | `json_object_iter_value` | T | yes | yes |
| 70 | `json_object_key_to_iter` | T | yes | yes |
| 71 | `json_object_seed` | T | yes | yes |
| 72 | `json_object_set_new` | T | yes | yes |
| 73 | `json_object_set_new_nocheck` | T | yes | yes |
| 74 | `json_object_setn_new` | T | yes | yes |
| 75 | `json_object_setn_new_nocheck` | T | yes | yes |
| 76 | `json_object_size` | T | yes | yes |
| 77 | `json_object_update` | T | yes | yes |
| 78 | `json_object_update_existing` | T | yes | yes |
| 79 | `json_object_update_missing` | T | yes | yes |
| 80 | `json_object_update_recursive` | T | yes | yes |
| 81 | `json_pack` | T | yes | yes |
| 82 | `json_pack_ex` | T | yes | yes |
| 83 | `json_real` | T | yes | yes |
| 84 | `json_real_set` | T | yes | yes |
| 85 | `json_real_value` | T | yes | yes |
| 86 | `json_set_alloc_funcs` | T | yes | yes |
| 87 | `json_set_alloc_funcs2` | T | yes | yes |
| 88 | `json_sprintf` | T | yes | yes |
| 89 | `json_string` | T | yes | yes |
| 90 | `json_string_length` | T | yes | yes |
| 91 | `json_string_nocheck` | T | yes | yes |
| 92 | `json_string_set` | T | yes | yes |
| 93 | `json_string_set_nocheck` | T | yes | yes |
| 94 | `json_string_setn` | T | yes | yes |
| 95 | `json_string_setn_nocheck` | T | yes | yes |
| 96 | `json_string_value` | T | yes | yes |
| 97 | `json_stringn` | T | yes | yes |
| 98 | `json_stringn_nocheck` | T | yes | yes |
| 99 | `json_true` | T | yes | yes |
| 100 | `json_unpack` | T | yes | yes |
| 101 | `json_unpack_ex` | T | yes | yes |
| 102 | `json_vpack_ex` | T | yes | yes |
| 103 | `json_vsprintf` | T | yes | yes |
| 104 | `json_vunpack_ex` | T | yes | yes |
| 105 | `jsonp_dtostr` | T | yes | yes |
| 106 | `jsonp_error_init` | T | yes | yes |
| 107 | `jsonp_error_set` | T | yes | yes |
| 108 | `jsonp_error_set_source` | T | yes | yes |
| 109 | `jsonp_error_vset` | T | yes | yes |
| 110 | `jsonp_free` | T | yes | yes |
| 111 | `jsonp_loop_check` | T | yes | yes |
| 112 | `jsonp_malloc` | T | yes | yes |
| 113 | `jsonp_realloc` | T | yes | yes |
| 114 | `jsonp_stringn_nocheck_own` | T | yes | yes |
| 115 | `jsonp_strndup` | T | yes | yes |
| 116 | `jsonp_strtod` | T | yes | yes |
| 117 | `strbuffer_append_byte` | T | yes | yes |
| 118 | `strbuffer_append_bytes` | T | yes | yes |
| 119 | `strbuffer_clear` | T | yes | yes |
| 120 | `strbuffer_close` | T | yes | yes |
| 121 | `strbuffer_init` | T | yes | yes |
| 122 | `strbuffer_pop` | T | yes | yes |
| 123 | `strbuffer_steal_value` | T | yes | yes |
| 124 | `strbuffer_value` | T | yes | yes |
| 125 | `strtod__unused` | T | yes | yes |
| 126 | `utf8_check_first` | T | yes | yes |
| 127 | `utf8_check_full` | T | yes | yes |
| 128 | `utf8_check_string` | T | yes | yes |
| 129 | `utf8_encode` | T | yes | yes |
| 130 | `utf8_iterate` | T | yes | yes |

## Undefined (imported) symbols in the Rust `.so`

All undefined symbols in the Rust `.so` are libc / libgcc-unwind imports —
there are **0 missing/undefined non-libc symbols**:

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
fclose@GLIBC_2.2.5
fgetc@GLIBC_2.2.5
fopen@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat64@GLIBC_2.33
fwrite@GLIBC_2.2.5
getcwd@GLIBC_2.2.5
getenv@GLIBC_2.2.5
getpid@GLIBC_2.2.5
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
posix_memalign@GLIBC_2.2.5
pthread_key_create@GLIBC_2.34
pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34
qsort@GLIBC_2.2.5
read@GLIBC_2.2.5
readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5
realpath@GLIBC_2.3
sched_yield@GLIBC_2.2.5
snprintf@GLIBC_2.2.5
sprintf@GLIBC_2.2.5
stat64@GLIBC_2.33
statx@GLIBC_2.28
stdin@GLIBC_2.2.5
strchr@GLIBC_2.2.5
strcmp@GLIBC_2.2.5
strerror@GLIBC_2.2.5
strlen@GLIBC_2.2.5
strncpy@GLIBC_2.2.5
strtod@GLIBC_2.2.5
strtoll@GLIBC_2.2.5
syscall@GLIBC_2.2.5
vsnprintf@GLIBC_2.2.5
write@GLIBC_2.2.5
writev@GLIBC_2.2.5
```

## Verification

```
$ comm -23 c_syms.txt rs_syms.txt   # in C, not in Rust
(empty)
$ comm -13 c_syms.txt rs_syms.txt   # in Rust, not in C
(empty)
```

**Result: symbol parity is EXACT (130/130).**

---

## Re-verification (final)

Regenerated after all fixes, for **both** Cargo profiles:

| `.so` | exported symbols | missing vs C | extra vs C |
|-------|-----------------:|-------------:|-----------:|
| `c_src/build/libjansson.so` (ground truth) | 130 | — | — |
| `target/debug/libjansson.so` | 130 | 0 | 0 |
| `target/release/libjansson.so` | 130 | 0 | 0 |

The Rust `.so` now also imports `__assert_fail` (as the C does), because
`src/strconv.rs` reproduces the live `assert()` at `c_src/src/strconv.c:53`.
All other undefined symbols remain libc / libgcc-unwind imports, i.e. **0
missing or undefined non-libc symbols**.

`tests/t00_smoke.rs::symbol_parity_nm` re-checks this on every test run, and
`tests/t10_abort_parity.rs::both_libraries_import_assert_fail` checks the
`__assert_fail` import.

## Feature combinations

`Cargo.toml` declares **no `[features]`** and no optional dependencies
(`cargo metadata` reports `features: {}`), so there is exactly **one** feature
combination. All of the following were run and pass:

```
cargo check --no-default-features
cargo check                       # identical to the above
cargo check --all-features        # identical to the above
cargo check --no-default-features --all-targets
cargo test  --no-default-features   # 131 tests, all pass
cargo test                          # 131 tests, all pass
cargo test  --release               # 131 tests, all pass
```
