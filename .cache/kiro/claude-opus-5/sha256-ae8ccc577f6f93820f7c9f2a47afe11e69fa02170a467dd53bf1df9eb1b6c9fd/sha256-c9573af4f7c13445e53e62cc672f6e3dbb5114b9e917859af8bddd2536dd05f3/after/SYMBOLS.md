# SYMBOLS.md — exported-symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from:
```
nm -D --defined-only c_src/build/libjansson.so
nm -D --defined-only translation/target/release/libjansson.so
```

C exports: **130**   Rust exports: **130**   Missing from Rust: **0**

`diff` of the `(type, name)` pairs is EMPTY — every symbol matches by name *and* by
nm type letter (T = text, B = bss). No stubs: every symbol below is backed by a real
translation of the corresponding C function.

All undefined symbols in the Rust `.so` are libc / libgcc-unwind only
(malloc, memcpy, snprintf, strtod, open, read, _Unwind_*, ...) — 0 missing non-libc symbols.

| 1 | `do_deep_copy` | T | T | yes | value.c | value.rs |
| 2 | `do_object_update_recursive` | T | T | yes | value.c | value.rs |
| 3 | `dtoa` | T | T | yes | dtoa.c | dtoa.rs |
| 4 | `dtoa_divmax` | D | D | yes | dtoa.c | dtoa.rs |
| 5 | `dtoa_r` | T | T | yes | strconv.c | dtoa.rs |
| 6 | `freedtoa` | T | T | yes | dtoa.c | dtoa.rs |
| 7 | `gethex` | T | T | yes | dtoa.c | dtoa.rs |
| 8 | `hashtable_clear` | T | T | yes | hashtable.c | hashtable.rs |
| 9 | `hashtable_close` | T | T | yes | hashtable.c | hashtable.rs |
| 10 | `hashtable_del` | T | T | yes | hashtable.c | hashtable.rs |
| 11 | `hashtable_get` | T | T | yes | hashtable.c | hashtable.rs |
| 12 | `hashtable_init` | T | T | yes | hashtable.c | hashtable.rs |
| 13 | `hashtable_iter` | T | T | yes | hashtable.c | hashtable.rs |
| 14 | `hashtable_iter_at` | T | T | yes | hashtable.c | hashtable.rs |
| 15 | `hashtable_iter_key` | T | T | yes | hashtable.c | hashtable.rs |
| 16 | `hashtable_iter_key_len` | T | T | yes | hashtable.c | hashtable.rs |
| 17 | `hashtable_iter_next` | T | T | yes | hashtable.c | hashtable.rs |
| 18 | `hashtable_iter_set` | T | T | yes | hashtable.c | hashtable.rs |
| 19 | `hashtable_iter_value` | T | T | yes | hashtable.c | hashtable.rs |
| 20 | `hashtable_seed` | B | B | yes | hashtable_seed.c | hashtable_seed.rs |
| 21 | `hashtable_set` | T | T | yes | hashtable.c | hashtable.rs |
| 22 | `jansson_version_cmp` | T | T | yes | version.c | version.rs |
| 23 | `jansson_version_str` | T | T | yes | version.c | version.rs |
| 24 | `json_array` | T | T | yes | value.c | value.rs |
| 25 | `json_array_append_new` | T | T | yes | value.c | value.rs |
| 26 | `json_array_clear` | T | T | yes | value.c | value.rs |
| 27 | `json_array_extend` | T | T | yes | value.c | value.rs |
| 28 | `json_array_get` | T | T | yes | value.c | value.rs |
| 29 | `json_array_insert_new` | T | T | yes | value.c | value.rs |
| 30 | `json_array_remove` | T | T | yes | value.c | value.rs |
| 31 | `json_array_set_new` | T | T | yes | value.c | value.rs |
| 32 | `json_array_size` | T | T | yes | value.c | value.rs |
| 33 | `json_copy` | T | T | yes | value.c | value.rs |
| 34 | `json_deep_copy` | T | T | yes | value.c | value.rs |
| 35 | `json_delete` | T | T | yes | value.c | value.rs |
| 36 | `json_dump_callback` | T | T | yes | dump.c | dump.rs |
| 37 | `json_dump_file` | T | T | yes | dump.c | dump.rs |
| 38 | `json_dumpb` | T | T | yes | dump.c | dump.rs |
| 39 | `json_dumpf` | T | T | yes | dump.c | dump.rs |
| 40 | `json_dumpfd` | T | T | yes | dump.c | dump.rs |
| 41 | `json_dumps` | T | T | yes | dump.c | dump.rs |
| 42 | `json_equal` | T | T | yes | value.c | value.rs |
| 43 | `json_false` | T | T | yes | value.c | value.rs |
| 44 | `json_get_alloc_funcs` | T | T | yes | memory.c | memory.rs |
| 45 | `json_get_alloc_funcs2` | T | T | yes | memory.c | memory.rs |
| 46 | `json_integer` | T | T | yes | value.c | value.rs |
| 47 | `json_integer_set` | T | T | yes | value.c | value.rs |
| 48 | `json_integer_value` | T | T | yes | value.c | value.rs |
| 49 | `json_load_callback` | T | T | yes | load.c | load.rs |
| 50 | `json_load_file` | T | T | yes | load.c | load.rs |
| 51 | `json_loadb` | T | T | yes | load.c | load.rs |
| 52 | `json_loadf` | T | T | yes | load.c | load.rs |
| 53 | `json_loadfd` | T | T | yes | load.c | load.rs |
| 54 | `json_loads` | T | T | yes | load.c | load.rs |
| 55 | `json_null` | T | T | yes | value.c | value.rs |
| 56 | `json_number_value` | T | T | yes | value.c | value.rs |
| 57 | `json_object` | T | T | yes | value.c | value.rs |
| 58 | `json_object_clear` | T | T | yes | value.c | value.rs |
| 59 | `json_object_del` | T | T | yes | value.c | value.rs |
| 60 | `json_object_deln` | T | T | yes | value.c | value.rs |
| 61 | `json_object_get` | T | T | yes | value.c | value.rs |
| 62 | `json_object_getn` | T | T | yes | value.c | value.rs |
| 63 | `json_object_iter` | T | T | yes | value.c | value.rs |
| 64 | `json_object_iter_at` | T | T | yes | value.c | value.rs |
| 65 | `json_object_iter_key` | T | T | yes | value.c | value.rs |
| 66 | `json_object_iter_key_len` | T | T | yes | value.c | value.rs |
| 67 | `json_object_iter_next` | T | T | yes | value.c | value.rs |
| 68 | `json_object_iter_set_new` | T | T | yes | value.c | value.rs |
| 69 | `json_object_iter_value` | T | T | yes | value.c | value.rs |
| 70 | `json_object_key_to_iter` | T | T | yes | value.c | value.rs |
| 71 | `json_object_seed` | T | T | yes | hashtable_seed.c | hashtable_seed.rs |
| 72 | `json_object_set_new` | T | T | yes | value.c | value.rs |
| 73 | `json_object_set_new_nocheck` | T | T | yes | value.c | value.rs |
| 74 | `json_object_setn_new` | T | T | yes | value.c | value.rs |
| 75 | `json_object_setn_new_nocheck` | T | T | yes | value.c | value.rs |
| 76 | `json_object_size` | T | T | yes | value.c | value.rs |
| 77 | `json_object_update` | T | T | yes | value.c | value.rs |
| 78 | `json_object_update_existing` | T | T | yes | value.c | value.rs |
| 79 | `json_object_update_missing` | T | T | yes | value.c | value.rs |
| 80 | `json_object_update_recursive` | T | T | yes | value.c | value.rs |
| 81 | `json_pack` | T | T | yes | pack_unpack.c | trampolines.rs |
| 82 | `json_pack_ex` | T | T | yes | pack_unpack.c | trampolines.rs |
| 83 | `json_real` | T | T | yes | value.c | value.rs |
| 84 | `json_real_set` | T | T | yes | value.c | value.rs |
| 85 | `json_real_value` | T | T | yes | value.c | value.rs |
| 86 | `json_set_alloc_funcs` | T | T | yes | memory.c | memory.rs |
| 87 | `json_set_alloc_funcs2` | T | T | yes | memory.c | memory.rs |
| 88 | `json_sprintf` | T | T | yes | value.c | trampolines.rs |
| 89 | `json_string` | T | T | yes | value.c | value.rs |
| 90 | `json_string_length` | T | T | yes | value.c | value.rs |
| 91 | `json_string_nocheck` | T | T | yes | value.c | value.rs |
| 92 | `json_string_set` | T | T | yes | value.c | value.rs |
| 93 | `json_string_set_nocheck` | T | T | yes | value.c | value.rs |
| 94 | `json_string_setn` | T | T | yes | value.c | value.rs |
| 95 | `json_string_setn_nocheck` | T | T | yes | value.c | value.rs |
| 96 | `json_string_value` | T | T | yes | value.c | value.rs |
| 97 | `json_stringn` | T | T | yes | value.c | value.rs |
| 98 | `json_stringn_nocheck` | T | T | yes | value.c | value.rs |
| 99 | `json_true` | T | T | yes | value.c | value.rs |
| 100 | `json_unpack` | T | T | yes | pack_unpack.c | trampolines.rs |
| 101 | `json_unpack_ex` | T | T | yes | pack_unpack.c | trampolines.rs |
| 102 | `json_vpack_ex` | T | T | yes | pack_unpack.c | pack_unpack.rs |
| 103 | `json_vsprintf` | T | T | yes | value.c | value.rs |
| 104 | `json_vunpack_ex` | T | T | yes | pack_unpack.c | pack_unpack.rs |
| 105 | `jsonp_dtostr` | T | T | yes | strconv.c | strconv.rs |
| 106 | `jsonp_error_init` | T | T | yes | error.c | error.rs |
| 107 | `jsonp_error_set` | T | T | yes | error.c | trampolines.rs |
| 108 | `jsonp_error_set_source` | T | T | yes | error.c | error.rs |
| 109 | `jsonp_error_vset` | T | T | yes | error.c | error.rs |
| 110 | `jsonp_free` | T | T | yes | memory.c | memory.rs |
| 111 | `jsonp_loop_check` | T | T | yes | value.c | value.rs |
| 112 | `jsonp_malloc` | T | T | yes | memory.c | memory.rs |
| 113 | `jsonp_realloc` | T | T | yes | memory.c | memory.rs |
| 114 | `jsonp_stringn_nocheck_own` | T | T | yes | value.c | value.rs |
| 115 | `jsonp_strndup` | T | T | yes | memory.c | memory.rs |
| 116 | `jsonp_strtod` | T | T | yes | strconv.c | strconv.rs |
| 117 | `strbuffer_append_byte` | T | T | yes | strbuffer.c | strbuffer.rs |
| 118 | `strbuffer_append_bytes` | T | T | yes | strbuffer.c | strbuffer.rs |
| 119 | `strbuffer_clear` | T | T | yes | strbuffer.c | strbuffer.rs |
| 120 | `strbuffer_close` | T | T | yes | strbuffer.c | strbuffer.rs |
| 121 | `strbuffer_init` | T | T | yes | strbuffer.c | strbuffer.rs |
| 122 | `strbuffer_pop` | T | T | yes | strbuffer.c | strbuffer.rs |
| 123 | `strbuffer_steal_value` | T | T | yes | strbuffer.c | strbuffer.rs |
| 124 | `strbuffer_value` | T | T | yes | strbuffer.c | strbuffer.rs |
| 125 | `strtod__unused` | T | T | yes | dtoa.c | dtoa_strtod.rs |
| 126 | `utf8_check_first` | T | T | yes | utf.c | utf.rs |
| 127 | `utf8_check_full` | T | T | yes | utf.c | utf.rs |
| 128 | `utf8_check_string` | T | T | yes | utf.c | utf.rs |
| 129 | `utf8_encode` | T | T | yes | utf.c | utf.rs |
| 130 | `utf8_iterate` | T | T | yes | utf.c | utf.rs |
