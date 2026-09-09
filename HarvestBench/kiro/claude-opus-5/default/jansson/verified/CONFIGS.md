# CONFIGS.md — Configuration-surface table (valid inputs)

Axes derived mechanically from `jansson.h` + the `if`/`switch` branches in
`c_src/src/*.c`.

## Axis 1 — Decoder flags (`load.c`)

| flag | value | branch it changes |
|---|---|---|
| `JSON_REJECT_DUPLICATES` | 0x1 | `parse_object`: `hashtable_get` pre-check → duplicate_key error |
| `JSON_DISABLE_EOF_CHECK` | 0x2 | `parse_json`: skips `TOKEN_EOF` check |
| `JSON_DECODE_ANY` | 0x4 | `parse_json`: allows top-level scalar |
| `JSON_DECODE_INT_AS_REAL` | 0x8 | `lex_scan_number`: skips the integer fast path, always produces REAL |
| `JSON_ALLOW_NUL` | 0x10 | `parse_value`: allows `\u0000` inside string values (keys still rejected) |

## Axis 2 — Encoder flags (`dump.c`)

| flag | value | branch it changes |
|---|---|---|
| `JSON_INDENT(n)` n=0..31 | n & 0x1F | `dump_indent`: n>0 → `\n` + n*depth spaces (whitespace buffer capped at 32) |
| `JSON_COMPACT` | 0x20 | `dump_indent` space suppression; object separator `":"` vs `": "` |
| `JSON_ENSURE_ASCII` | 0x40 | `dump_string`: escapes every codepoint > 0x7F as `\uXXXX` / surrogate pair |
| `JSON_SORT_KEYS` | 0x80 | `do_dump` object: qsort key array instead of insertion order |
| `JSON_PRESERVE_ORDER` | 0x100 | no branch (deprecated no-op) |
| `JSON_ENCODE_ANY` | 0x200 | `json_dump_callback`: allow top-level scalar |
| `JSON_ESCAPE_SLASH` | 0x400 | `dump_string`: `/` → `\/` |
| `JSON_REAL_PRECISION(n)` n=0..31 | (n&0x1F)<<11 | `do_dump` real: precision passed to `jsonp_dtostr` (0 → dtoa mode 0 shortest) |
| `JSON_EMBED` | 0x10000 | `do_dump`: omit outermost `[]`/`{}`; stripped before recursion |

## Axis 3 — Pack/unpack flags (`pack_unpack.c`)

| flag | value | branch it changes |
|---|---|---|
| `JSON_VALIDATE_ONLY` | 0x1 | `unpack`: type-check only, no `va_arg` writes, no incref for `O` |
| `JSON_STRICT` | 0x2 | `unpack_object`/`unpack_array`: leftover items → error |

## Axis 4 — Entry points (all public, incl. lowest-level)

Low-level: `utf8_encode`, `utf8_check_first`, `utf8_check_full`, `utf8_iterate`,
`utf8_check_string`, `strbuffer_*`, `hashtable_*`, `jsonp_malloc`,
`jsonp_realloc`, `jsonp_free`, `jsonp_strndup`, `jsonp_error_*`,
`jsonp_dtostr`, `jsonp_strtod`, `jsonp_loop_check`, `jsonp_stringn_nocheck_own`,
`dtoa_r`, `freedtoa`, `strtod__unused`.
Mid-level: all `json_object_*`, `json_array_*`, `json_string*`,
`json_integer*`, `json_real*`, `json_equal`, `json_copy`, `json_deep_copy`,
`do_deep_copy`, `do_object_update_recursive`, `json_delete`.
High-level: `json_loads/loadb/loadf/loadfd/load_file/load_callback`,
`json_dumps/dumpb/dumpf/dumpfd/dump_file/dump_callback`,
`json_pack/pack_ex/vpack_ex`, `json_unpack/unpack_ex/vunpack_ex`,
`json_sprintf/vsprintf`, memory hooks, `jansson_version_*`, `json_object_seed`.

## Axis 5 — Input shapes

Empty / one / many; nesting depth 0,1,2,…,2048,2049; ASCII / non-ASCII /
4-byte UTF-8 / surrogate-pair codepoints / control chars / embedded NUL;
integer boundaries (0, ±1, INT64_MIN, INT64_MAX, one past); reals (0.0, -0.0,
denormals, 1e-300, 1e300, values needing 17 digits, integral reals);
key lengths 0..; hashtable sizes crossing the rehash thresholds (8, 16, 32, …);
strbuffer sizes crossing 16→32→64 growth.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `utf8_encode` | every codepoint boundary: -1, 0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, and 4096 random i32 | [x] |
| 2 | `utf8_check_first` | all 256 byte values | [x] |
| 3 | `utf8_check_full` | sizes 0..5 × random byte sequences (2048 random) + hand-built overlong/surrogate/max cases | [x] |
| 4 | `utf8_iterate` | bufsize 0..4 × random buffers (2048 random); returns offset or NULL | [x] |
| 5 | `utf8_check_string` | random byte strings length 0..16 (4096 random) + valid multi-byte strings | [x] |
| 6 | `strbuffer_init`/`append_byte`/`append_bytes`/`value`/`pop`/`clear`/`steal_value`/`close` | randomized op sequences crossing 16→32→64→… growth; pop-on-empty; append size 0 | [x] |
| 7 | `hashtable_init`/`set`/`get`/`del`/`clear`/`iter`/`iter_at`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value`/`iter_set`/`close` | 0/1/many entries crossing rehash at 8,16,32,64; random keys with embedded NUL; duplicate set; del-then-get | [x] |
| 8 | `jsonp_malloc`/`jsonp_realloc`/`jsonp_free`/`jsonp_strndup` | size 0 and >0; realloc grow/shrink; strndup over embedded NUL | [x] |
| 9 | `json_set_alloc_funcs` + `json_get_alloc_funcs` | set custom malloc/free (realloc→NULL), read back, restore | [x] |
| 10 | `json_set_alloc_funcs2` + `json_get_alloc_funcs2` | set custom malloc/realloc/free, read back, restore | [x] |
| 11 | `jsonp_error_init`/`error_set_source`/`error_set`/`error_vset` | source shorter/equal/longer than 80; text longer than 158; double-set | [x] |
| 12 | `jsonp_dtostr` | precision 0..31 × 4096 random doubles + boundary doubles; buffer sizes 1..40 | [x] |
| 13 | `jsonp_strtod` (via strbuffer) | random decimal texts incl. exponent forms, overflow, underflow | [x] |
| 14 | `dtoa_r` | modes 0..5 × ndigits 0..25 × 4096 random doubles (decpt, sign, rve, digits) | [x] |
| 15 | `json_object` + `set_new`/`get`/`size`/`del`/`clear` | 0/1/many keys; key len 0; random keys; overwrite | [x] |
| 16 | `json_object_setn_new` vs `setn_new_nocheck` | keys with embedded NUL and non-ASCII UTF-8; `key_len` shorter than strlen | [x] |
| 17 | `json_object_getn`/`deln` | `key_len` != strlen(key); prefix collisions | [x] |
| 18 | `json_object_iter*` full walk | many-entry object after inserts and deletes; `iter_set_new`; `key_to_iter` round trip | [x] |
| 19 | `json_object_update` | disjoint / overlapping / empty other | [x] |
| 20 | `json_object_update_existing` | disjoint / overlapping | [x] |
| 21 | `json_object_update_missing` | disjoint / overlapping | [x] |
| 22 | `json_object_update_recursive` / `do_object_update_recursive` | nested objects merged at depth 1,2,3; non-object leaf overwrite | [x] |
| 23 | `json_array` + `append_new`/`get`/`size`/`set_new` | 0/1/many (crossing the 8→grow boundary and 100+) | [x] |
| 24 | `json_array_insert_new` | index 0, middle, == entries (append) over many sizes | [x] |
| 25 | `json_array_remove` | first, middle, last over many sizes | [x] |
| 26 | `json_array_clear` / `json_array_extend` | empty+empty, empty+many, many+many | [x] |
| 27 | `json_string`/`stringn`/`string_nocheck`/`stringn_nocheck` + `string_value`/`string_length` | ASCII, UTF-8 multi-byte, embedded NUL (nocheck), len 0, random bytes | [x] |
| 28 | `json_string_set`/`setn`/`set_nocheck`/`setn_nocheck` | replace with shorter/longer/empty; invalid UTF-8 | [x] |
| 29 | `jsonp_stringn_nocheck_own` | takes ownership of `jsonp_malloc`'d buffer, len 0 and >0 | [x] |
| 30 | `json_integer` + `integer_value`/`integer_set`/`number_value` | 0, ±1, INT64_MIN, INT64_MAX, 4096 random i64 | [x] |
| 31 | `json_real` + `real_value`/`real_set`/`number_value` | 0.0, -0.0, denormals, ±1e±300, 4096 random finite f64 | [x] |
| 32 | `json_true`/`json_false`/`json_null` + `json_delete` | singleton identity, refcount SIZE_MAX behaviour | [x] |
| 33 | `json_equal` | all 8×8 type pairs; deep equal/unequal objects and arrays; key-order-insensitive | [x] |
| 34 | `json_copy` | each of 8 types; shallow semantics (child pointer identity) | [x] |
| 35 | `json_deep_copy` / `do_deep_copy` | nested object/array trees depth 1..5, random | [x] |
| 36 | `jsonp_loop_check` | fresh hashtable, repeat same pointer, different pointers | [x] |
| 37 | `json_sprintf` / `json_vsprintf` | plain, `%d`/`%s` substitution, long output > 160 bytes, empty | [x] |
| 38 | `json_loads` | flags = 0; 4096 random + curated valid JSON documents | [x] |
| 39 | `json_loads` | `JSON_DECODE_ANY`; top-level `1`, `"s"`, `true`, `false`, `null`, `1.5` | [x] |
| 40 | `json_loads` | `JSON_DECODE_INT_AS_REAL`; integers, big integers, overflowing integers | [x] |
| 41 | `json_loads` | `JSON_REJECT_DUPLICATES`; objects with and without duplicate keys | [x] |
| 42 | `json_loads` | `JSON_DISABLE_EOF_CHECK`; input with trailing garbage | [x] |
| 43 | `json_loads` | `JSON_ALLOW_NUL`; `"a\u0000b"` in value position | [x] |
| 44 | `json_loads` | all 32 combinations of the 5 decoder flags × curated corpus | [x] |
| 45 | `json_loadb` | `buflen` < strlen (truncated mid-token), == strlen, 0; with each flag | [x] |
| 46 | `json_loadf` | `FILE*` from tmpfile; same corpus; error line/column/position | [x] |
| 47 | `json_loadfd` | raw fd; same corpus; `JSON_DISABLE_EOF_CHECK` | [x] |
| 48 | `json_load_file` | real temp file; same corpus; error `source` field = filename | [x] |
| 49 | `json_load_callback` | chunked callback returning 1,3,7 bytes at a time; same corpus | [x] |
| 50 | deep nesting | arrays/objects nested 1, 10, 100, 1000, 2047, 2048, 2049 deep | [x] |
| 51 | `json_dumps` | flags 0; every value shape produced from the corpus | [x] |
| 52 | `json_dumps` | `JSON_INDENT(n)` for n = 0..31 × nested corpus | [x] |
| 53 | `json_dumps` | `JSON_COMPACT` alone and with `JSON_INDENT(n)` | [x] |
| 54 | `json_dumps` | `JSON_ENSURE_ASCII`; strings with U+00E9, U+4E2D, U+1F600 | [x] |
| 55 | `json_dumps` | `JSON_SORT_KEYS`; objects 0/1/many keys, random key sets | [x] |
| 56 | `json_dumps` | `JSON_ESCAPE_SLASH`; strings containing `/` | [x] |
| 57 | `json_dumps` | `JSON_ENCODE_ANY`; each top-level scalar type | [x] |
| 58 | `json_dumps` | `JSON_REAL_PRECISION(n)` for n = 0..31 × random reals | [x] |
| 59 | `json_dumps` | `JSON_EMBED` on top-level array and object, alone and with INDENT | [x] |
| 60 | `json_dumps` | `JSON_PRESERVE_ORDER` (no-op) sanity | [x] |
| 61 | `json_dumps` | randomized flag cross-product (512 random flag words) × corpus | [x] |
| 62 | `json_dumpb` | buffer size exactly / one less / one more than needed; size 0 | [x] |
| 63 | `json_dumpf` | tmpfile output compared byte-for-byte; with flags | [x] |
| 64 | `json_dumpfd` | raw fd output compared byte-for-byte; with flags | [x] |
| 65 | `json_dump_file` | temp file output compared byte-for-byte; with flags | [x] |
| 66 | `json_dump_callback` | callback recording chunk boundaries; verify identical chunking | [x] |
| 67 | round-trip | `json_loads` then `json_dumps` for the whole corpus, all flag combos | [x] |
| 68 | `json_pack_ex` | `{}`/`[]`/`s`/`n`/`b`/`i`/`I`/`f`/`o`/`O` single-char formats | [x] |
| 69 | `json_pack_ex` | `s#`, `s%`, `s+`, `s+#`, `s+%` length/concat variants | [x] |
| 70 | `json_pack_ex` | `s?`, `o?`, `o*`, `s*` optional variants with NULL and non-NULL | [x] |
| 71 | `json_pack_ex` | nested composite formats `{s:[i,i],s:{s:s}}` | [x] |
| 72 | `json_pack_ex` | whitespace/comma/colon separators in format ignored | [x] |
| 73 | `json_pack` (varargs, no error) | same formats via the non-`_ex` entry point | [x] |
| 74 | `json_unpack_ex` | flags 0; each of `s i I b f F n o O {} []` | [x] |
| 75 | `json_unpack_ex` | `JSON_STRICT` on objects and arrays, exact and leftover | [x] |
| 76 | `json_unpack_ex` | `JSON_VALIDATE_ONLY` (no output writes) on all types | [x] |
| 77 | `json_unpack_ex` | `JSON_STRICT|JSON_VALIDATE_ONLY` combined | [x] |
| 78 | `json_unpack_ex` | `!` and `*` in-format strictness markers | [x] |
| 79 | `json_unpack_ex` | `s%` length output, `s?` optional keys | [x] |
| 80 | `json_unpack` (varargs, no error) | same formats via the non-`_ex` entry point | [x] |
| 81 | `json_object_seed` | seed 0 (auto) and fixed non-zero seed; `hashtable_seed` global | [x] |
| 82 | `jansson_version_str` / `jansson_version_cmp` | version string; cmp across major/minor/micro incl. negatives | [x] |
| 83 | `json_dumps` of loaded reals | reals that round-trip through dtoa shortest repr, 4096 random | [x] |
| 84 | mixed pipeline | build value with low-level API → dump with random flags → reload with random flags → dump again | [x] |

## Status

All 84 rows pass. Every row is driven through `dlsym` on both `.so` files
(`c_src/build/libjansson.so` and `translation/target/release/libjansson.so`),
never by calling Rust functions directly, so the `#[no_mangle]` export wrappers
are covered too.

Row → test mapping:

| rows | test binary / function |
|---|---|
| 1-8, 11-14, 81, 82 | `tests/phase_b_lowlevel.rs` (`row01_*` … `row82_*`) |
| 9, 10 | `tests/phase_b_alloc.rs::row09_row10_alloc_funcs` (own binary: mutates process-global allocator hooks) |
| 15-37 | `tests/phase_b_value.rs` |
| 38-67, 83, 84 | `tests/phase_b_loaddump.rs` |
| 68-80 | `tests/phase_b_pack.rs` |

Randomization: every row that says "random" uses `common::Rng` (splitmix64)
seeded with a fixed per-row constant, so failures reproduce exactly.

## Notes that constrain how the tests must be run

- **`--test-threads=1` is mandatory.** The C library is not thread safe in this
  configuration: `dtoa.c` keeps a global `freelist`/`p5s` (no
  `MULTIPLE_THREADS`), `hashtable_seed` is a process-global, and
  `json_set_alloc_funcs*` replace global function pointers. Running the test
  binaries multi-threaded corrupts the C library and crashes it. Use
  `translation/run_verification.sh`, which passes `--test-threads=1`.
- **Fixed hash seed.** `common::libs()` calls `json_object_seed(0x5EED1234)` in
  both libraries before any object exists, so object iteration order (and hence
  `json_dumps` output without `JSON_SORT_KEYS`) is deterministic and comparable.
- **No binary/driver target.** `c_src/CMakeLists.txt` contains only
  `add_library(jansson SHARED ...)` (`grep -c add_executable` → 0), so there is
  no executable whose stdout could be compared. That completion-gate item is
  not applicable.
- **Feature combinations.** `translation/Cargo.toml` has no `[features]` table,
  so the default build and `--no-default-features` are the only configurations;
  both were built and fully tested.
