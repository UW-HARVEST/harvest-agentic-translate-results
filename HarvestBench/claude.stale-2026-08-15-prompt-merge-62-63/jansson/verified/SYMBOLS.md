# SYMBOLS.md — C/Rust exported symbol parity

Derived mechanically from `nm -D --defined-only` on both shared libraries.

```
C   .so: cbuild/libjansson.so            -> 130 dynamic symbols
Rust .so: target/release/libjansson.so   -> 130 dynamic symbols
MISSING from Rust: 0
EXTRA in Rust    : 0
```

All undefined symbols in the Rust `.so` are libc / libgcc-unwind only (no unresolved
jansson symbols). Verified with `nm -D --undefined-only`.

| # | symbol | C source | in C .so | in Rust .so |
|---|--------|----------|----------|-------------|
| 1 | `do_deep_copy` | value.c | yes | yes |
| 2 | `do_object_update_recursive` | value.c | yes | yes |
| 3 | `dtoa` | dtoa.c | yes | yes |
| 4 | `dtoa_divmax` | dtoa.c | yes | yes |
| 5 | `dtoa_r` | dtoa.c | yes | yes |
| 6 | `freedtoa` | dtoa.c | yes | yes |
| 7 | `gethex` | dtoa.c | yes | yes |
| 8 | `hashtable_clear` | hashtable.c | yes | yes |
| 9 | `hashtable_close` | hashtable.c | yes | yes |
| 10 | `hashtable_del` | hashtable.c | yes | yes |
| 11 | `hashtable_get` | hashtable.c | yes | yes |
| 12 | `hashtable_init` | hashtable.c | yes | yes |
| 13 | `hashtable_iter` | hashtable.c | yes | yes |
| 14 | `hashtable_iter_at` | hashtable.c | yes | yes |
| 15 | `hashtable_iter_key` | hashtable.c | yes | yes |
| 16 | `hashtable_iter_key_len` | hashtable.c | yes | yes |
| 17 | `hashtable_iter_next` | hashtable.c | yes | yes |
| 18 | `hashtable_iter_set` | hashtable.c | yes | yes |
| 19 | `hashtable_iter_value` | hashtable.c | yes | yes |
| 20 | `hashtable_seed` | hashtable_seed.c | yes | yes |
| 21 | `hashtable_set` | hashtable.c | yes | yes |
| 22 | `jansson_version_cmp` | version.c | yes | yes |
| 23 | `jansson_version_str` | version.c | yes | yes |
| 24 | `json_array` | value.c | yes | yes |
| 25 | `json_array_append_new` | value.c | yes | yes |
| 26 | `json_array_clear` | value.c | yes | yes |
| 27 | `json_array_extend` | value.c | yes | yes |
| 28 | `json_array_get` | value.c | yes | yes |
| 29 | `json_array_insert_new` | value.c | yes | yes |
| 30 | `json_array_remove` | value.c | yes | yes |
| 31 | `json_array_set_new` | value.c | yes | yes |
| 32 | `json_array_size` | value.c | yes | yes |
| 33 | `json_copy` | value.c | yes | yes |
| 34 | `json_deep_copy` | value.c | yes | yes |
| 35 | `json_delete` | value.c | yes | yes |
| 36 | `json_dump_callback` | dump.c | yes | yes |
| 37 | `json_dump_file` | dump.c | yes | yes |
| 38 | `json_dumpb` | dump.c | yes | yes |
| 39 | `json_dumpf` | dump.c | yes | yes |
| 40 | `json_dumpfd` | dump.c | yes | yes |
| 41 | `json_dumps` | dump.c | yes | yes |
| 42 | `json_equal` | value.c | yes | yes |
| 43 | `json_false` | value.c | yes | yes |
| 44 | `json_get_alloc_funcs` | memory.c | yes | yes |
| 45 | `json_get_alloc_funcs2` | memory.c | yes | yes |
| 46 | `json_integer` | value.c | yes | yes |
| 47 | `json_integer_set` | value.c | yes | yes |
| 48 | `json_integer_value` | value.c | yes | yes |
| 49 | `json_load_callback` | load.c | yes | yes |
| 50 | `json_load_file` | load.c | yes | yes |
| 51 | `json_loadb` | load.c | yes | yes |
| 52 | `json_loadf` | load.c | yes | yes |
| 53 | `json_loadfd` | load.c | yes | yes |
| 54 | `json_loads` | load.c | yes | yes |
| 55 | `json_null` | value.c | yes | yes |
| 56 | `json_number_value` | value.c | yes | yes |
| 57 | `json_object` | value.c | yes | yes |
| 58 | `json_object_clear` | value.c | yes | yes |
| 59 | `json_object_del` | value.c | yes | yes |
| 60 | `json_object_deln` | value.c | yes | yes |
| 61 | `json_object_get` | value.c | yes | yes |
| 62 | `json_object_getn` | value.c | yes | yes |
| 63 | `json_object_iter` | value.c | yes | yes |
| 64 | `json_object_iter_at` | value.c | yes | yes |
| 65 | `json_object_iter_key` | value.c | yes | yes |
| 66 | `json_object_iter_key_len` | value.c | yes | yes |
| 67 | `json_object_iter_next` | value.c | yes | yes |
| 68 | `json_object_iter_set_new` | value.c | yes | yes |
| 69 | `json_object_iter_value` | value.c | yes | yes |
| 70 | `json_object_key_to_iter` | value.c | yes | yes |
| 71 | `json_object_seed` | hashtable_seed.c | yes | yes |
| 72 | `json_object_set_new` | value.c | yes | yes |
| 73 | `json_object_set_new_nocheck` | value.c | yes | yes |
| 74 | `json_object_setn_new` | value.c | yes | yes |
| 75 | `json_object_setn_new_nocheck` | value.c | yes | yes |
| 76 | `json_object_size` | value.c | yes | yes |
| 77 | `json_object_update` | value.c | yes | yes |
| 78 | `json_object_update_existing` | value.c | yes | yes |
| 79 | `json_object_update_missing` | value.c | yes | yes |
| 80 | `json_object_update_recursive` | value.c | yes | yes |
| 81 | `json_pack` | pack_unpack.c | yes | yes |
| 82 | `json_pack_ex` | pack_unpack.c | yes | yes |
| 83 | `json_real` | value.c | yes | yes |
| 84 | `json_real_set` | value.c | yes | yes |
| 85 | `json_real_value` | value.c | yes | yes |
| 86 | `json_set_alloc_funcs` | memory.c | yes | yes |
| 87 | `json_set_alloc_funcs2` | memory.c | yes | yes |
| 88 | `json_sprintf` | value.c | yes | yes |
| 89 | `json_string` | value.c | yes | yes |
| 90 | `json_string_length` | value.c | yes | yes |
| 91 | `json_string_nocheck` | value.c | yes | yes |
| 92 | `json_string_set` | value.c | yes | yes |
| 93 | `json_string_set_nocheck` | value.c | yes | yes |
| 94 | `json_string_setn` | value.c | yes | yes |
| 95 | `json_string_setn_nocheck` | value.c | yes | yes |
| 96 | `json_string_value` | value.c | yes | yes |
| 97 | `json_stringn` | value.c | yes | yes |
| 98 | `json_stringn_nocheck` | value.c | yes | yes |
| 99 | `json_true` | value.c | yes | yes |
| 100 | `json_unpack` | pack_unpack.c | yes | yes |
| 101 | `json_unpack_ex` | pack_unpack.c | yes | yes |
| 102 | `json_vpack_ex` | pack_unpack.c | yes | yes |
| 103 | `json_vsprintf` | value.c | yes | yes |
| 104 | `json_vunpack_ex` | pack_unpack.c | yes | yes |
| 105 | `jsonp_dtostr` | strconv.c | yes | yes |
| 106 | `jsonp_error_init` | error.c | yes | yes |
| 107 | `jsonp_error_set` | error.c | yes | yes |
| 108 | `jsonp_error_set_source` | error.c | yes | yes |
| 109 | `jsonp_error_vset` | error.c | yes | yes |
| 110 | `jsonp_free` | memory.c | yes | yes |
| 111 | `jsonp_loop_check` | value.c | yes | yes |
| 112 | `jsonp_malloc` | memory.c | yes | yes |
| 113 | `jsonp_realloc` | memory.c | yes | yes |
| 114 | `jsonp_stringn_nocheck_own` | value.c | yes | yes |
| 115 | `jsonp_strndup` | memory.c | yes | yes |
| 116 | `jsonp_strtod` | strconv.c | yes | yes |
| 117 | `strbuffer_append_byte` | strbuffer.c | yes | yes |
| 118 | `strbuffer_append_bytes` | strbuffer.c | yes | yes |
| 119 | `strbuffer_clear` | strbuffer.c | yes | yes |
| 120 | `strbuffer_close` | strbuffer.c | yes | yes |
| 121 | `strbuffer_init` | strbuffer.c | yes | yes |
| 122 | `strbuffer_pop` | strbuffer.c | yes | yes |
| 123 | `strbuffer_steal_value` | strbuffer.c | yes | yes |
| 124 | `strbuffer_value` | strbuffer.c | yes | yes |
| 125 | `strtod__unused` | dtoa.c | yes | yes |
| 126 | `utf8_check_first` | utf.c | yes | yes |
| 127 | `utf8_check_full` | utf.c | yes | yes |
| 128 | `utf8_check_string` | utf.c | yes | yes |
| 129 | `utf8_encode` | utf.c | yes | yes |
| 130 | `utf8_iterate` | utf.c | yes | yes |

## Verification record

Re-checked after the `dtoa_r` rewrite (see below), with `verify_all.sh`:

```
symbol parity: C=130 Rust=130, 0 missing
no unresolved jansson symbols (undefined set is libc / libgcc-unwind only)
driver stdout byte-identical (75420 lines)
```

### Note on `dtoa_r` (the one real completeness failure found)

`nm -D` parity alone was already satisfied before verification began, which is
exactly why symbol parity is necessary but NOT sufficient. `src/dtoa.rs` exported
`dtoa_r` but had translated the **wrong `#ifdef` branch**: its header comment said
it implemented "the classic Bigint code path (equivalent to compiling without
`USE_BF96`)" and asserted that was byte-identical to the real path. It is not.

`c_src/src/dtoa.c:385` defines `USE_BF96` (reached because neither `NO_LONG_LONG`
nor `NO_BF96` is defined), so the C compiles an entirely different exact-integer /
96-bit-bigfloat algorithm driven by the `pten` / `Lhint` / `pfive` / `pfivebits`
tables. Corroborating evidence at the time: `cargo build` reported `PTEN`, `LHINT`,
`BF96` and `PFIVEBITS` as **never used** in the Rust — precisely the tables the
real path depends on.

The divergence was invisible to `json_dumps`, because `jsonp_dtostr`
(`strconv.c:75`) only ever calls `dtoa_r` with `mode` 0 or 2. It appeared only when
the exported `dtoa_r` was called directly in modes 1, 4 and 5, e.g.

| input | mode/ndigits | C | Rust (before) |
|---|---|---|---|
| `0x44b52d02c7e14af6` | 1 / any | `decpt=24`, `"1"` | `decpt=23`, `"9999999999999999"` |
| `0.1` | 4 / 17 | `"10000000000000001"` | `"1"` |
| pi | 4 / 17 | `"31415926535897931"` | `"3141592653589793"` |

`dtoa_r` was retranslated from the preprocessed active C (all `#ifdef`s resolved
with the real build flags), emulating the original `goto` graph with an explicit
state machine. The differential driver went from **232 differing lines to 0**.
