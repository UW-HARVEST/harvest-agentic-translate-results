# CONFIGS.md — Configuration-surface table (valid inputs)

Axes derived mechanically from the C source:

**Decoder flags** (`load.c`, checked in `parse_json`/`parse_value`/`parse_object`/`lex_scan_number`):
`JSON_REJECT_DUPLICATES 0x1`, `JSON_DISABLE_EOF_CHECK 0x2`, `JSON_DECODE_ANY 0x4`,
`JSON_DECODE_INT_AS_REAL 0x8`, `JSON_ALLOW_NUL 0x10`.

**Encoder flags** (`dump.c`, checked in `do_dump`/`dump_indent`/`dump_string`/`json_dump_callback`):
`JSON_INDENT(n)` = low 5 bits (0..31), `JSON_COMPACT 0x20`, `JSON_ENSURE_ASCII 0x40`,
`JSON_SORT_KEYS 0x80`, `JSON_PRESERVE_ORDER 0x100` (no-op in 2.15),
`JSON_ENCODE_ANY 0x200`, `JSON_ESCAPE_SLASH 0x400`,
`JSON_REAL_PRECISION(n)` = bits 11..15 (0..31), `JSON_EMBED 0x10000`.

**Pack/unpack flags** (`pack_unpack.c`): `JSON_VALIDATE_ONLY 0x1`, `JSON_STRICT 0x2`.

**Entry points** — the full set, including the lowest level ones:
`hashtable_*` (12 fns), `strbuffer_*` (9), `utf8_*` (5), `jsonp_*` (12),
`dtoa`/`dtoa_r`/`freedtoa`/`gethex`, `json_object_*`, `json_array_*`, `json_string*`,
`json_integer*`, `json_real*`, `json_number_value`, `json_true/false/null`,
`json_equal`, `json_copy`, `json_deep_copy`, `do_deep_copy`,
`do_object_update_recursive`, `json_load*` (6), `json_dump*` (6),
`json_pack*`/`json_unpack*`/`json_vsprintf`, `json_*_alloc_funcs*`,
`json_object_seed`, `jansson_version_*`.

**Input shapes** the C special-cases: empty / one / many elements; array growth
(`size=8`, doubling in `json_array_grow`); hashtable rehash at load factor 1
(`INITIAL_HASHTABLE_ORDER=3` ⇒ 8 buckets); 1/2/3/4-byte UTF-8; BMP vs non-BMP
(surrogate pair split at `0x10000`); codepoints `<0x20`, `0x7F`, `>0x7F`;
integer boundaries (`0`, `±1`, `INT64_MIN/MAX`, 25-char `MAX_INTEGER_STR_LENGTH`);
real exponent boundaries in `jsonp_dtostr` (`decpt <= -4`, `decpt > 16`);
`strbuffer` growth from `STRBUFFER_MIN_SIZE=16`; `MAX_BUF_LEN=1024` callback chunking;
parse depth up to `JSON_PARSER_MAX_DEPTH=2048`.

Note: all object iteration order depends on the global `hashtable_seed`; every test
calls `json_object_seed(<fixed>)` on **both** libraries so orders are comparable.

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|-----|
| 1 | `utf8_check_first` | all 256 byte values | [x] |
| 2 | `utf8_check_full` | sizes 1..5 × randomized byte triples/quads incl. boundaries 0x80/0xBF/0xC0 | [x] |
| 3 | `utf8_encode` | codepoints 0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF + randomized | [x] |
| 4 | `utf8_iterate` | randomized buffers, bufsize 0/1/2/3/4, valid & truncated sequences | [x] |
| 5 | `utf8_check_string` | randomized ASCII / 2-byte / 3-byte / 4-byte / mixed / invalid strings, len 0..64 | [x] |
| 6 | `strbuffer_init`+`append_bytes`+`value`+`close` | appends sized 0,1,15,16,17,1000 to force growth past `STRBUFFER_MIN_SIZE` | [x] |
| 7 | `strbuffer_append_byte` / `strbuffer_pop` | randomized byte sequences incl. pop-on-empty | [x] |
| 8 | `strbuffer_clear` / `strbuffer_steal_value` | after N appends, then reuse | [x] |
| 9 | `hashtable_init`/`set`/`get`/`del`/`clear`/`close` | 0,1,7,8,9,64,200 keys → forces `hashtable_do_rehash` (order 3→…) | [x] |
| 10 | `hashtable_set` | duplicate key overwrite path (`pair` found branch) | [x] |
| 11 | `hashtable_iter`/`iter_at`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value`/`iter_set` | full ordered traversal over 0..64 randomized keys, fixed seed | [x] |
| 12 | `hashtable_get`/`del` | keys with embedded NULs and binary keys, `key_len` != `strlen` | [x] |
| 13 | `jsonp_malloc`/`jsonp_free`/`jsonp_realloc`/`jsonp_strndup` | sizes 0,1,16,1024; realloc grow & shrink; `strndup` len 0..64 with NULs | [x] |
| 14 | `json_set_alloc_funcs` + `json_get_alloc_funcs` | set custom malloc/free (do_realloc→NULL) then read back; exercise realloc-emulation path | [x] |
| 15 | `json_set_alloc_funcs2` + `json_get_alloc_funcs2` | set malloc/realloc/free then read back; restore defaults | [x] |
| 16 | `json_object_seed` | seed 0 (autoseed) and fixed nonzero seed; read `hashtable_seed` symbol | [x] |
| 17 | `jsonp_dtostr` | precision 0 and 1..17 × values: 0.0, -0.0, 1.0, 0.1, 1e-5 (`decpt<=-4`), 1e16/1e17 (`decpt>16`), randomized f64 | [x] |
| 18 | `jsonp_strtod` | strbuffer holding randomized decimal/exponent literals | [x] |
| 19 | `dtoa_r` | mode 0 and 2 × ndigits 0..17 × randomized doubles + subnormals | [x] |
| 20 | `dtoa` / `freedtoa` | mode 0..3 × ndigits 0..17 × randomized doubles | [x] |
| 21 | `gethex` | hex float literals `0x1p0`, `0x1.8p3`, randomized | [x] |
| 22 | `json_integer`/`json_integer_value`/`json_integer_set` | 0, ±1, INT64_MIN, INT64_MAX, randomized i64 | [x] |
| 23 | `json_real`/`json_real_value`/`json_real_set` | 0.0, -0.0, subnormal, DBL_MAX, DBL_MIN, randomized finite f64 | [x] |
| 24 | `json_number_value` | on integer / real / string / true / null | [x] |
| 25 | `json_string`/`json_string_value`/`json_string_length` | ASCII / multi-byte UTF-8 / len 0 / len 4096 | [x] |
| 26 | `json_stringn` / `json_stringn_nocheck` | `len` shorter than NUL-terminated content; embedded NULs (nocheck) | [x] |
| 27 | `json_string_set` / `setn` / `set_nocheck` / `setn_nocheck` | replace with shorter/longer/empty values | [x] |
| 28 | `jsonp_stringn_nocheck_own` | ownership-transfer path with `jsonp_malloc`ed buffer | [x] |
| 29 | `json_true`/`json_false`/`json_null` | singleton identity + refcount `(size_t)-1` | [x] |
| 30 | `json_array` + `append_new` | 0, 1, 8 (exactly `size`), 9 (forces grow), 100, 1000 elements | [x] |
| 31 | `json_array_insert_new` | index 0 / middle / == entries (append) at sizes 0,1,8,9 | [x] |
| 32 | `json_array_set_new` | every valid index at sizes 1,8,9 | [x] |
| 33 | `json_array_remove` | index 0 / middle / last at sizes 1,2,8,9 | [x] |
| 34 | `json_array_clear` then reuse | after 0 / 9 / 100 elements | [x] |
| 35 | `json_array_extend` | empty+empty, empty+N, N+empty, N+M (forces grow) | [x] |
| 36 | `json_array_get`/`json_array_size` | full sweep of indices 0..size on mixed-type arrays | [x] |
| 37 | `json_object` + `set_new_nocheck` + `get` | 0,1,7,8,9,64,200 keys → rehash boundary | [x] |
| 38 | `json_object_setn_new` / `setn_new_nocheck` | `key_len` < `strlen(key)`; UTF-8 keys | [x] |
| 39 | `json_object_getn` | key_len variants; keys present/absent | [x] |
| 40 | `json_object_del` / `deln` | delete first/middle/last/absent; then re-add | [x] |
| 41 | `json_object_clear` then reuse | after 0 / 9 / 64 keys | [x] |
| 42 | `json_object_size` | on object / array / string / NULL | [x] |
| 43 | `json_object_iter` + `iter_next` + `iter_key` + `iter_key_len` + `iter_value` | full traversal, 0..64 keys, fixed seed → insertion order must match | [x] |
| 44 | `json_object_iter_at` + `json_object_key_to_iter` + `json_object_iter_set_new` | mid-traversal set; key round-trip through `key_to_iter` | [x] |
| 45 | `json_object_update` | disjoint / overlapping / self-update / empty other | [x] |
| 46 | `json_object_update_existing` | some keys present, some not | [x] |
| 47 | `json_object_update_missing` | some keys present, some not | [x] |
| 48 | `json_object_update_recursive` + `do_object_update_recursive` | nested objects 1..4 deep, mixed object/non-object at same key | [x] |
| 49 | `json_equal` | equal & unequal pairs of every type; nested arrays/objects; different sizes | [x] |
| 50 | `json_copy` | on object / array / string / integer / real / true / false / null | [x] |
| 51 | `json_deep_copy` / `do_deep_copy` | nested object/array trees 1..5 deep, mixed types | [x] |
| 52 | `jsonp_loop_check` | fresh hashtable, same node twice, several nodes | [x] |
| 53 | `json_dumps` | flags = 0 (default: 2-space-less, `": "` separator) on nested object+array | [x] |
| 54 | `json_dumps` | `JSON_COMPACT` | [x] |
| 55 | `json_dumps` | `JSON_INDENT(n)` for n = 1,2,4,8,31 (and 0 = off) | [x] |
| 56 | `json_dumps` | `JSON_INDENT(n) \| JSON_COMPACT` (indent wins in `dump_indent`) | [x] |
| 57 | `json_dumps` | `JSON_ENSURE_ASCII` on BMP + non-BMP (surrogate-pair) strings | [x] |
| 58 | `json_dumps` | `JSON_ESCAPE_SLASH` | [x] |
| 59 | `json_dumps` | `JSON_SORT_KEYS` on 0..64 randomized keys (incl. prefix-equal keys → `compare_keys` len tiebreak) | [x] |
| 60 | `json_dumps` | `JSON_SORT_KEYS \| JSON_COMPACT \| JSON_INDENT(2)` | [x] |
| 61 | `json_dumps` | `JSON_ENCODE_ANY` on scalar roots (string/int/real/true/false/null) | [x] |
| 62 | `json_dumps` | `JSON_PRESERVE_ORDER` (no-op bit) — must not change output | [x] |
| 63 | `json_dumps` | `JSON_REAL_PRECISION(p)` for p = 0..17 on randomized reals | [x] |
| 64 | `json_dumps` | `JSON_EMBED` on array root and object root (drops outer brackets) | [x] |
| 65 | `json_dumps` | `JSON_EMBED \| JSON_INDENT(2) \| JSON_ENCODE_ANY` | [x] |
| 66 | `json_dumps` | randomized flag bit-vectors over the full encoder mask | [x] |
| 67 | `json_dumpb` | buffer exactly the required size / 1 byte too small / 0 size / oversized | [x] |
| 68 | `json_dumpf` | to a `tmpfile()` FILE*, flags 0 and COMPACT+SORT_KEYS | [x] |
| 69 | `json_dumpfd` | to a pipe/temp fd | [x] |
| 70 | `json_dump_file` | to a temp path, then read back and compare bytes | [x] |
| 71 | `json_dump_callback` | user callback collecting chunks; flags 0 / ENCODE_ANY / EMBED | [x] |
| 72 | `json_dumps` string escaping | strings containing `"` `\` `/` `\b\f\n\r\t`, all 0x00–0x1F, 0x7F, U+00FF, U+FFFF, U+10FFFF | [x] |
| 73 | `json_loads` | flags = 0 on randomized valid JSON documents (objects/arrays) | [x] |
| 74 | `json_loads` | `JSON_DECODE_ANY` on scalar roots: string, int, real, true, false, null | [x] |
| 75 | `json_loads` | `JSON_DECODE_INT_AS_REAL` on integer literals incl. > INT64_MAX | [x] |
| 76 | `json_loads` | `JSON_REJECT_DUPLICATES` on documents with and without duplicate keys | [x] |
| 77 | `json_loads` | `JSON_DISABLE_EOF_CHECK` with trailing data | [x] |
| 78 | `json_loads` | `JSON_ALLOW_NUL` with `\u0000` inside strings | [x] |
| 79 | `json_loads` | all decoder-flag combinations (2^5 = 32) on a fixed corpus | [x] |
| 80 | `json_loads` | number shapes: `0`, `-0`, `1e5`, `1E+5`, `1e-5`, `1.5`, `-1.5e-10`, INT64_MIN/MAX literals | [x] |
| 81 | `json_loads` | string escapes: all shortcut escapes, `\uXXXX` BMP, surrogate pairs | [x] |
| 82 | `json_loads` | multi-byte UTF-8 input (2/3/4-byte) exercising `stream_get` buffering | [x] |
| 83 | `json_loads` | whitespace variants (space/tab/CR/LF) and line/column/position tracking in `error` | [x] |
| 84 | `json_loads` | nesting depth 1, 100, 2047, 2048 (at `JSON_PARSER_MAX_DEPTH`) | [x] |
| 85 | `json_loadb` | `buflen` shorter than the NUL terminator; `buflen` = 0; embedded NUL bytes | [x] |
| 86 | `json_loadf` | `tmpfile()` FILE* containing randomized JSON | [x] |
| 87 | `json_loadfd` | fd containing randomized JSON | [x] |
| 88 | `json_load_file` | temp file containing randomized JSON | [x] |
| 89 | `json_load_callback` | callback returning 1-byte, 100-byte and 1024-byte (`MAX_BUF_LEN`) chunks | [x] |
| 90 | round-trip `json_loads` → `json_dumps` | randomized documents × decoder flags × encoder flags | [x] |
| 91 | `json_pack` / `json_pack_ex` / `json_vpack_ex` | every format char: `{} [] s n b i I f o O` | [x] |
| 92 | `json_pack_ex` | `s#` (int length) and `s%` (size_t length) | [x] |
| 93 | `json_pack_ex` | `s+`, `s+#`, `s+%` concatenation chains of 2 and 3 parts | [x] |
| 94 | `json_pack_ex` | optional `s?`, `s*`, `o?`, `o*`, `O?`, `O*` with non-NULL and NULL args | [x] |
| 95 | `json_pack_ex` | nested `{s:{s:[i,i]}}` shapes, 1..4 deep | [x] |
| 96 | `json_pack_ex` | whitespace/`,`/`:` skipping in the format string (`next_token`) | [x] |
| 97 | `json_pack_ex` | `flags` = 0 and `JSON_VALIDATE_ONLY`/`JSON_STRICT` (ignored by pack) | [x] |
| 98 | `json_unpack` / `json_unpack_ex` / `json_vunpack_ex` | every format char: `{} [] s i I b f F o O n` | [x] |
| 99 | `json_unpack_ex` | `s%` length out-param | [x] |
| 100 | `json_unpack_ex` | `JSON_VALIDATE_ONLY` (no stores) vs default (stores) | [x] |
| 101 | `json_unpack_ex` | `JSON_STRICT` on exact / extra keys / extra array elements | [x] |
| 102 | `json_unpack_ex` | `!` and `*` strictness markers in objects and arrays | [x] |
| 103 | `json_unpack_ex` | optional `s?` keys present and absent (`gotopt` path) | [x] |
| 104 | `json_unpack_ex` | nested `{s:{s:[i,i]}}`, 1..4 deep | [x] |
| 105 | `json_sprintf` / `json_vsprintf` | `%d`/`%s`/`%f`/`%%`, empty result, long (>4096) result, multi-byte UTF-8 | [x] |
| 106 | `jsonp_error_init` / `error_set_source` / `error_set` / `error_vset` | sources of length 0, 79, 80, 200; messages of length 0, 159, 300; every `json_error_code` | [x] |
| 107 | `jansson_version_str` / `jansson_version_cmp` | (2,15,0) exact, and randomized (major,minor,micro) | [x] |
| 108 | `json_delete` | on object/array/string/integer/real (never on singletons) | [x] |
