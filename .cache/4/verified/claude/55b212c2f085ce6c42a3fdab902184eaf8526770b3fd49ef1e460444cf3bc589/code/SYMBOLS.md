# SYMBOLS.md — exported-symbol parity (C `.so` vs Rust `.so`)

Generated mechanically by `gen_symbols.sh` (`nm -D --defined-only`).

* C   `.so`: `c_src/build/libjansson.so`  — **130** dynamic symbols
* Rust `.so`: `release/libjansson.so` — **130** dynamic symbols

## Missing in Rust (`comm -23`)

**NONE — 0 missing symbols.**

## Extra jansson-namespace symbols in Rust (should be none)

**NONE.**

## Undefined (imported) non-libc symbols in Rust `.so`

**NONE — every jansson symbol is defined, not imported.**

## Full symbol table

| symbol | C type | Rust type | in Rust? | C source |
|--------|--------|-----------|----------|----------|
| `do_deep_copy` | T | T | yes | value.c |
| `do_object_update_recursive` | T | T | yes | value.c |
| `dtoa` | T | T | yes | dtoa.c |
| `dtoa_divmax` | D | D | yes | dtoa.c |
| `dtoa_r` | T | T | yes | dtoa.c |
| `freedtoa` | T | T | yes | dtoa.c |
| `gethex` | T | T | yes | dtoa.c |
| `hashtable_clear` | T | T | yes | hashtable.c |
| `hashtable_close` | T | T | yes | dump.c |
| `hashtable_del` | T | T | yes | dump.c |
| `hashtable_get` | T | T | yes | hashtable.c |
| `hashtable_init` | T | T | yes | dump.c |
| `hashtable_iter` | T | T | yes | hashtable.c |
| `hashtable_iter_at` | T | T | yes | hashtable.c |
| `hashtable_iter_key` | T | T | yes | hashtable.c |
| `hashtable_iter_key_len` | T | T | yes | hashtable.c |
| `hashtable_iter_next` | T | T | yes | hashtable.c |
| `hashtable_iter_set` | T | T | yes | hashtable.c |
| `hashtable_iter_value` | T | T | yes | hashtable.c |
| `hashtable_seed` | B | B | yes | hashtable.c |
| `hashtable_set` | T | T | yes | hashtable.c |
| `jansson_version_cmp` | T | T | yes | version.c |
| `jansson_version_str` | T | T | yes | version.c |
| `json_array` | T | T | yes | load.c |
| `json_array_append_new` | T | T | yes | load.c |
| `json_array_clear` | T | T | yes | value.c |
| `json_array_extend` | T | T | yes | value.c |
| `json_array_get` | T | T | yes | dump.c |
| `json_array_insert_new` | T | T | yes | value.c |
| `json_array_remove` | T | T | yes | value.c |
| `json_array_set_new` | T | T | yes | value.c |
| `json_array_size` | T | T | yes | dump.c |
| `json_copy` | T | T | yes | value.c |
| `json_deep_copy` | T | T | yes | value.c |
| `json_delete` | T | T | yes | value.c |
| `json_dump_callback` | T | T | yes | dump.c |
| `json_dump_file` | T | T | yes | dump.c |
| `json_dumpb` | T | T | yes | dump.c |
| `json_dumpf` | T | T | yes | dump.c |
| `json_dumpfd` | T | T | yes | dump.c |
| `json_dumps` | T | T | yes | dump.c |
| `json_equal` | T | T | yes | value.c |
| `json_false` | T | T | yes | load.c |
| `json_get_alloc_funcs` | T | T | yes | memory.c |
| `json_get_alloc_funcs2` | T | T | yes | memory.c |
| `json_integer` | T | T | yes | load.c |
| `json_integer_set` | T | T | yes | value.c |
| `json_integer_value` | T | T | yes | dump.c |
| `json_load_callback` | T | T | yes | load.c |
| `json_load_file` | T | T | yes | load.c |
| `json_loadb` | T | T | yes | load.c |
| `json_loadf` | T | T | yes | load.c |
| `json_loadfd` | T | T | yes | load.c |
| `json_loads` | T | T | yes | load.c |
| `json_null` | T | T | yes | load.c |
| `json_number_value` | T | T | yes | pack_unpack.c |
| `json_object` | T | T | yes | load.c |
| `json_object_clear` | T | T | yes | value.c |
| `json_object_del` | T | T | yes | value.c |
| `json_object_deln` | T | T | yes | value.c |
| `json_object_get` | T | T | yes | value.c |
| `json_object_getn` | T | T | yes | dump.c |
| `json_object_iter` | T | T | yes | dump.c |
| `json_object_iter_at` | T | T | yes | value.c |
| `json_object_iter_key` | T | T | yes | dump.c |
| `json_object_iter_key_len` | T | T | yes | dump.c |
| `json_object_iter_next` | T | T | yes | dump.c |
| `json_object_iter_set_new` | T | T | yes | value.c |
| `json_object_iter_value` | T | T | yes | dump.c |
| `json_object_key_to_iter` | T | T | yes | value.c |
| `json_object_seed` | T | T | yes | hashtable_seed.c |
| `json_object_set_new` | T | T | yes | value.c |
| `json_object_set_new_nocheck` | T | T | yes | value.c |
| `json_object_setn_new` | T | T | yes | value.c |
| `json_object_setn_new_nocheck` | T | T | yes | load.c |
| `json_object_size` | T | T | yes | dump.c |
| `json_object_update` | T | T | yes | value.c |
| `json_object_update_existing` | T | T | yes | value.c |
| `json_object_update_missing` | T | T | yes | value.c |
| `json_object_update_recursive` | T | T | yes | value.c |
| `json_pack` | T | T | yes | pack_unpack.c |
| `json_pack_ex` | T | T | yes | pack_unpack.c |
| `json_real` | T | T | yes | load.c |
| `json_real_set` | T | T | yes | pack_unpack.c |
| `json_real_value` | T | T | yes | dump.c |
| `json_set_alloc_funcs` | T | T | yes | memory.c |
| `json_set_alloc_funcs2` | T | T | yes | memory.c |
| `json_sprintf` | T | T | yes | value.c |
| `json_string` | T | T | yes | value.c |
| `json_string_length` | T | T | yes | dump.c |
| `json_string_nocheck` | T | T | yes | value.c |
| `json_string_set` | T | T | yes | value.c |
| `json_string_set_nocheck` | T | T | yes | value.c |
| `json_string_setn` | T | T | yes | value.c |
| `json_string_setn_nocheck` | T | T | yes | value.c |
| `json_string_value` | T | T | yes | dump.c |
| `json_stringn` | T | T | yes | value.c |
| `json_stringn_nocheck` | T | T | yes | pack_unpack.c |
| `json_true` | T | T | yes | load.c |
| `json_unpack` | T | T | yes | pack_unpack.c |
| `json_unpack_ex` | T | T | yes | pack_unpack.c |
| `json_vpack_ex` | T | T | yes | pack_unpack.c |
| `json_vsprintf` | T | T | yes | value.c |
| `json_vunpack_ex` | T | T | yes | pack_unpack.c |
| `jsonp_dtostr` | T | T | yes | dump.c |
| `jsonp_error_init` | T | T | yes | error.c |
| `jsonp_error_set` | T | T | yes | error.c |
| `jsonp_error_set_source` | T | T | yes | error.c |
| `jsonp_error_vset` | T | T | yes | error.c |
| `jsonp_free` | T | T | yes | dtoa.c |
| `jsonp_loop_check` | T | T | yes | dump.c |
| `jsonp_malloc` | T | T | yes | dtoa.c |
| `jsonp_realloc` | T | T | yes | dump.c |
| `jsonp_stringn_nocheck_own` | T | T | yes | load.c |
| `jsonp_strndup` | T | T | yes | memory.c |
| `jsonp_strtod` | T | T | yes | load.c |
| `strbuffer_append_byte` | T | T | yes | load.c |
| `strbuffer_append_bytes` | T | T | yes | dump.c |
| `strbuffer_clear` | T | T | yes | load.c |
| `strbuffer_close` | T | T | yes | dump.c |
| `strbuffer_init` | T | T | yes | dump.c |
| `strbuffer_pop` | T | T | yes | load.c |
| `strbuffer_steal_value` | T | T | yes | dump.c |
| `strbuffer_value` | T | T | yes | load.c |
| `strtod__unused` | T | T | yes | dtoa.c |
| `utf8_check_first` | T | T | yes | load.c |
| `utf8_check_full` | T | T | yes | load.c |
| `utf8_check_string` | T | T | yes | pack_unpack.c |
| `utf8_encode` | T | T | yes | load.c |
| `utf8_iterate` | T | T | yes | dump.c |
