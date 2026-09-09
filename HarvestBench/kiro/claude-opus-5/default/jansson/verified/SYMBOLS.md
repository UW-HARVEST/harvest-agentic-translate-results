# SYMBOLS.md — Symbol parity (C `.so` vs Rust `.so`)

Generated mechanically:

```sh
nm -D --defined-only c_src/build/libjansson.so            | awk '{print $3}' | sort -u > verif/c_syms.txt
nm -D --defined-only translation/target/release/libjansson.so | awk '{print $3}' | sort -u > verif/rust_syms.txt
comm -23 verif/c_syms.txt verif/rust_syms.txt   # missing from Rust
comm -13 verif/c_syms.txt verif/rust_syms.txt   # extra in Rust
```

Result: **130 symbols in C, 130 in Rust, 0 missing, 0 extra.**

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memcpy`,
`memmove`, `memset`, `bcmp`, `strlen`, `strtod`, `strtoll`, `snprintf`,
`sprintf`, `vsnprintf`, `fopen`, `fclose`, `fgetc`, `fwrite`, `open`, `open64`,
`close`, `read`, `write`, `writev`, `lseek64`, `fstat64`, `stat64`, `statx`,
`getenv`, `getpid`, `gettid`, `gettimeofday`, `sched_yield`, `strerror`,
`stdin`, `abort`, `__errno_location`, `mmap64`, `munmap`, `dl_iterate_phdr`,
`realpath`, `readlink`, `getcwd`, `syscall`, `pthread_key_*`, `__cxa_*`,
`_Unwind_*`, `_ITM_*`, `__gmon_start__`, `__tls_get_addr`).
**0 missing/undefined non-libc symbols.**

## Full symbol table

| # | symbol | C `.so` | Rust `.so` | origin (C file) |
|---|--------|---------|-----------|-----------------|
| 1 | `do_deep_copy` | ✅ | ✅ | value.c |
| 2 | `do_object_update_recursive` | ✅ | ✅ | value.c |
| 3 | `dtoa` | ✅ | ✅ | dtoa.c |
| 4 | `dtoa_divmax` | ✅ | ✅ | dtoa.c (global var) |
| 5 | `dtoa_r` | ✅ | ✅ | dtoa.c |
| 6 | `freedtoa` | ✅ | ✅ | dtoa.c |
| 7 | `gethex` | ✅ | ✅ | dtoa.c |
| 8 | `hashtable_clear` | ✅ | ✅ | hashtable.c |
| 9 | `hashtable_close` | ✅ | ✅ | hashtable.c |
| 10 | `hashtable_del` | ✅ | ✅ | hashtable.c |
| 11 | `hashtable_get` | ✅ | ✅ | hashtable.c |
| 12 | `hashtable_init` | ✅ | ✅ | hashtable.c |
| 13 | `hashtable_iter` | ✅ | ✅ | hashtable.c |
| 14 | `hashtable_iter_at` | ✅ | ✅ | hashtable.c |
| 15 | `hashtable_iter_key` | ✅ | ✅ | hashtable.c |
| 16 | `hashtable_iter_key_len` | ✅ | ✅ | hashtable.c |
| 17 | `hashtable_iter_next` | ✅ | ✅ | hashtable.c |
| 18 | `hashtable_iter_set` | ✅ | ✅ | hashtable.c |
| 19 | `hashtable_iter_value` | ✅ | ✅ | hashtable.c |
| 20 | `hashtable_seed` | ✅ | ✅ | hashtable_seed.c (global var) |
| 21 | `hashtable_set` | ✅ | ✅ | hashtable.c |
| 22 | `jansson_version_cmp` | ✅ | ✅ | version.c |
| 23 | `jansson_version_str` | ✅ | ✅ | version.c |
| 24 | `json_array` | ✅ | ✅ | value.c |
| 25 | `json_array_append_new` | ✅ | ✅ | value.c |
| 26 | `json_array_clear` | ✅ | ✅ | value.c |
| 27 | `json_array_extend` | ✅ | ✅ | value.c |
| 28 | `json_array_get` | ✅ | ✅ | value.c |
| 29 | `json_array_insert_new` | ✅ | ✅ | value.c |
| 30 | `json_array_remove` | ✅ | ✅ | value.c |
| 31 | `json_array_set_new` | ✅ | ✅ | value.c |
| 32 | `json_array_size` | ✅ | ✅ | value.c |
| 33 | `json_copy` | ✅ | ✅ | value.c |
| 34 | `json_deep_copy` | ✅ | ✅ | value.c |
| 35 | `json_delete` | ✅ | ✅ | value.c |
| 36 | `json_dump_callback` | ✅ | ✅ | dump.c |
| 37 | `json_dump_file` | ✅ | ✅ | dump.c |
| 38 | `json_dumpb` | ✅ | ✅ | dump.c |
| 39 | `json_dumpf` | ✅ | ✅ | dump.c |
| 40 | `json_dumpfd` | ✅ | ✅ | dump.c |
| 41 | `json_dumps` | ✅ | ✅ | dump.c |
| 42 | `json_equal` | ✅ | ✅ | value.c |
| 43 | `json_false` | ✅ | ✅ | value.c |
| 44 | `json_get_alloc_funcs` | ✅ | ✅ | memory.c |
| 45 | `json_get_alloc_funcs2` | ✅ | ✅ | memory.c |
| 46 | `json_integer` | ✅ | ✅ | value.c |
| 47 | `json_integer_set` | ✅ | ✅ | value.c |
| 48 | `json_integer_value` | ✅ | ✅ | value.c |
| 49 | `json_load_callback` | ✅ | ✅ | load.c |
| 50 | `json_load_file` | ✅ | ✅ | load.c |
| 51 | `json_loadb` | ✅ | ✅ | load.c |
| 52 | `json_loadf` | ✅ | ✅ | load.c |
| 53 | `json_loadfd` | ✅ | ✅ | load.c |
| 54 | `json_loads` | ✅ | ✅ | load.c |
| 55 | `json_null` | ✅ | ✅ | value.c |
| 56 | `json_number_value` | ✅ | ✅ | value.c |
| 57 | `json_object` | ✅ | ✅ | value.c |
| 58 | `json_object_clear` | ✅ | ✅ | value.c |
| 59 | `json_object_del` | ✅ | ✅ | value.c |
| 60 | `json_object_deln` | ✅ | ✅ | value.c |
| 61 | `json_object_get` | ✅ | ✅ | value.c |
| 62 | `json_object_getn` | ✅ | ✅ | value.c |
| 63 | `json_object_iter` | ✅ | ✅ | value.c |
| 64 | `json_object_iter_at` | ✅ | ✅ | value.c |
| 65 | `json_object_iter_key` | ✅ | ✅ | value.c |
| 66 | `json_object_iter_key_len` | ✅ | ✅ | value.c |
| 67 | `json_object_iter_next` | ✅ | ✅ | value.c |
| 68 | `json_object_iter_set_new` | ✅ | ✅ | value.c |
| 69 | `json_object_iter_value` | ✅ | ✅ | value.c |
| 70 | `json_object_key_to_iter` | ✅ | ✅ | value.c |
| 71 | `json_object_seed` | ✅ | ✅ | hashtable_seed.c |
| 72 | `json_object_set_new` | ✅ | ✅ | value.c |
| 73 | `json_object_set_new_nocheck` | ✅ | ✅ | value.c |
| 74 | `json_object_setn_new` | ✅ | ✅ | value.c |
| 75 | `json_object_setn_new_nocheck` | ✅ | ✅ | value.c |
| 76 | `json_object_size` | ✅ | ✅ | value.c |
| 77 | `json_object_update` | ✅ | ✅ | value.c |
| 78 | `json_object_update_existing` | ✅ | ✅ | value.c |
| 79 | `json_object_update_missing` | ✅ | ✅ | value.c |
| 80 | `json_object_update_recursive` | ✅ | ✅ | value.c |
| 81 | `json_pack` | ✅ | ✅ | pack_unpack.c |
| 82 | `json_pack_ex` | ✅ | ✅ | pack_unpack.c |
| 83 | `json_real` | ✅ | ✅ | value.c |
| 84 | `json_real_set` | ✅ | ✅ | value.c |
| 85 | `json_real_value` | ✅ | ✅ | value.c |
| 86 | `json_set_alloc_funcs` | ✅ | ✅ | memory.c |
| 87 | `json_set_alloc_funcs2` | ✅ | ✅ | memory.c |
| 88 | `json_sprintf` | ✅ | ✅ | value.c |
| 89 | `json_string` | ✅ | ✅ | value.c |
| 90 | `json_string_length` | ✅ | ✅ | value.c |
| 91 | `json_string_nocheck` | ✅ | ✅ | value.c |
| 92 | `json_string_set` | ✅ | ✅ | value.c |
| 93 | `json_string_set_nocheck` | ✅ | ✅ | value.c |
| 94 | `json_string_setn` | ✅ | ✅ | value.c |
| 95 | `json_string_setn_nocheck` | ✅ | ✅ | value.c |
| 96 | `json_string_value` | ✅ | ✅ | value.c |
| 97 | `json_stringn` | ✅ | ✅ | value.c |
| 98 | `json_stringn_nocheck` | ✅ | ✅ | value.c |
| 99 | `json_true` | ✅ | ✅ | value.c |
| 100 | `json_unpack` | ✅ | ✅ | pack_unpack.c |
| 101 | `json_unpack_ex` | ✅ | ✅ | pack_unpack.c |
| 102 | `json_vpack_ex` | ✅ | ✅ | pack_unpack.c |
| 103 | `json_vsprintf` | ✅ | ✅ | value.c |
| 104 | `json_vunpack_ex` | ✅ | ✅ | pack_unpack.c |
| 105 | `jsonp_dtostr` | ✅ | ✅ | strconv.c |
| 106 | `jsonp_error_init` | ✅ | ✅ | error.c |
| 107 | `jsonp_error_set` | ✅ | ✅ | error.c |
| 108 | `jsonp_error_set_source` | ✅ | ✅ | error.c |
| 109 | `jsonp_error_vset` | ✅ | ✅ | error.c |
| 110 | `jsonp_free` | ✅ | ✅ | memory.c |
| 111 | `jsonp_loop_check` | ✅ | ✅ | value.c |
| 112 | `jsonp_malloc` | ✅ | ✅ | memory.c |
| 113 | `jsonp_realloc` | ✅ | ✅ | memory.c |
| 114 | `jsonp_stringn_nocheck_own` | ✅ | ✅ | value.c |
| 115 | `jsonp_strndup` | ✅ | ✅ | memory.c |
| 116 | `jsonp_strtod` | ✅ | ✅ | strconv.c |
| 117 | `strbuffer_append_byte` | ✅ | ✅ | strbuffer.c |
| 118 | `strbuffer_append_bytes` | ✅ | ✅ | strbuffer.c |
| 119 | `strbuffer_clear` | ✅ | ✅ | strbuffer.c |
| 120 | `strbuffer_close` | ✅ | ✅ | strbuffer.c |
| 121 | `strbuffer_init` | ✅ | ✅ | strbuffer.c |
| 122 | `strbuffer_pop` | ✅ | ✅ | strbuffer.c |
| 123 | `strbuffer_steal_value` | ✅ | ✅ | strbuffer.c |
| 124 | `strbuffer_value` | ✅ | ✅ | strbuffer.c |
| 125 | `strtod__unused` | ✅ | ✅ | dtoa.c (`strtod` renamed) |
| 126 | `utf8_check_first` | ✅ | ✅ | utf.c |
| 127 | `utf8_check_full` | ✅ | ✅ | utf.c |
| 128 | `utf8_check_string` | ✅ | ✅ | utf.c |
| 129 | `utf8_encode` | ✅ | ✅ | utf.c |
| 130 | `utf8_iterate` | ✅ | ✅ | utf.c |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default (no features). `cargo check
--no-default-features` is equivalent to `cargo check`. Verified below.

## Verified

Re-checked by `translation/run_verification.sh` after every fix, for both the
default build and `--no-default-features`:

```
C symbols   : 130
Rust symbols: 130
missing: none
extra  : none
```

No stubs, no `unimplemented!()`, no fake exports: every symbol is backed by a
real translation, and each one is exercised by at least one differential test in
`translation/tests/` (see `CONFIGS.md` and `ERRORS.md` for the row → test
mapping). The only-in-Rust `strtod` shim is exported under the same
`strtod__unused` name the C uses, and the two globals (`dtoa_divmax`,
`hashtable_seed`) are compared by value through `dlsym`, not just by presence.
