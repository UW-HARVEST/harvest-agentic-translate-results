# CONFIGS.md — configuration surface (valid inputs)

Derived mechanically from the branches the C source actually takes.

## Build-time configurations

`Cargo.toml` has **no `[features]` section**, so the crate has exactly **one**
build configuration:

| # | cargo invocation | notes |
|---|------------------|-------|
| 1 | `cargo build --no-default-features` (== `cargo build`) | no features exist |

`c_src/CMakeLists.txt` likewise builds exactly one configuration: all 13 `.c`
files, `HAVE_CONFIG_H` on, using the checked-in
`include/jansson_private_config.h` / `include/jansson_config.h`.  The compile
time switches those headers pin are therefore **fixed** and must be matched by
the Rust translation (they are *not* selectable at runtime):

| switch | value | effect in C |
|--------|-------|-------------|
| `JSON_INTEGER_IS_LONG_LONG` | 1 | `json_int_t = long long`, `strtoll`, `"%lld"` |
| `JSON_PARSER_MAX_DEPTH` | 2048 | `parse_value` depth guard |
| `INITIAL_HASHTABLE_ORDER` | 3 | 8 buckets, rehash when `size >= 8` |
| `DTOA_ENABLED` / `USE_DTOA` | 1 | `jsonp_dtostr` uses David Gay's `dtoa_r`, **not** `snprintf("%.*g")` |
| `HAVE_STDINT_H`, `HAVE_UNISTD_H` | 1 | `json_dumpfd` / `json_loadfd` actually do I/O |
| `HAVE_ATOMIC_BUILTINS`, `HAVE_SCHED_YIELD` | 1 | `json_object_seed` = atomic test-and-set variant |
| `USE_URANDOM` | 1 | `generate_seed()` reads `/dev/urandom` first |
| `HAVE_SETLOCALE`, `HAVE_LOCALE_H` | 1 | `get_decimal_point()` via `sprintf("%#.0f", 1.0)` |

## Runtime configuration axes the C code branches on

* **encode flags** (`dump.c`): `JSON_INDENT(n)` `n∈0..31` (`FLAGS_TO_INDENT`),
  `JSON_COMPACT`, `JSON_ENSURE_ASCII`, `JSON_SORT_KEYS`, `JSON_PRESERVE_ORDER`
  (accepted, no branch), `JSON_ENCODE_ANY`, `JSON_ESCAPE_SLASH`,
  `JSON_REAL_PRECISION(n)` `n∈0..31` (`FLAGS_TO_PRECISION`), `JSON_EMBED`.
* **decode flags** (`load.c`): `JSON_REJECT_DUPLICATES`, `JSON_DISABLE_EOF_CHECK`,
  `JSON_DECODE_ANY`, `JSON_DECODE_INT_AS_REAL`, `JSON_ALLOW_NUL`.
* **pack/unpack flags** (`pack_unpack.c`): `JSON_VALIDATE_ONLY`, `JSON_STRICT`;
  format alphabet `{}[]siIbfFOon` + modifiers `# % + ? * !` and the
  `unpack_value_starters` set `"{[siIbfFOon"`.
* **allocator mode** (`memory.c`): default (`malloc`/`realloc`/`free`),
  `json_set_alloc_funcs` (⇒ `do_realloc == NULL` ⇒ realloc *emulation* path),
  `json_set_alloc_funcs2` (custom realloc).
* **sinks**: `json_dumps`, `json_dumpb`, `json_dumpf`, `json_dumpfd`,
  `json_dump_file`, `json_dump_callback`.
* **sources**: `json_loads`, `json_loadb`, `json_loadf`, `json_loadfd`,
  `json_load_file`, `json_load_callback` (chunked at `MAX_BUF_LEN = 1024`).
* **input shapes**: element counts 0/1/8/9/16/17/many (array `size=8` doubling,
  hashtable `order=3` rehash), key length 0/1/long, nesting depth
  1/8/64/2047/2048, UTF-8 1/2/3/4-byte sequences, escapes/surrogate pairs,
  control characters, embedded NUL, `json_int_t` extremes, doubles across the
  `decpt <= -4 || decpt > 16` exponent switch and every precision 0..31.

## Configuration table

Legend: `[x]` = verified by a passing differential test across randomised inputs.

### Low-level primitives (called directly, not through wrappers)

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 1 | `jansson_version_str`, `jansson_version_cmp` | all orderings of major/minor/micro incl. equal | [x] |
| 2 | `jsonp_malloc`, `jsonp_free` | sizes 1..4096 + free of the result | [x] |
| 3 | `jsonp_realloc` | default allocator (real `realloc`): grow, shrink, `ptr=NULL`, `newSize=0` | [x] |
| 4 | `jsonp_realloc` | after `json_set_alloc_funcs` (`do_realloc == NULL`) ⇒ emulation: grow/shrink/`newSize=0`/`ptr=NULL` | [x] |
| 5 | `jsonp_strndup` | len 0/1/n, len < strlen, len > strlen (copies NULs) | [x] |
| 6 | `json_get_alloc_funcs`, `json_get_alloc_funcs2` | after default / `set_alloc_funcs` / `set_alloc_funcs2`; NULL out-params | [x] |
| 7 | `json_set_alloc_funcs2` | custom malloc/realloc/free actually used by the whole pipeline | [x] |
| 8 | `utf8_check_first` | all 256 byte values | [x] |
| 9 | `utf8_check_full` | size 2/3/4 × random payloads (valid, overlong, surrogate, >0x10FFFF, bad continuation); `codepoint` NULL and non-NULL | [x] |
| 10 | `utf8_encode` | every boundary codepoint 0/0x7F/0x80/0x7FF/0x800/0xFFFF/0x10000/0x10FFFF + random | [x] |
| 11 | `utf8_iterate` | bufsize 0/1/…/4, truncated multi-byte, random buffers | [x] |
| 12 | `utf8_check_string` | random valid UTF-8, random bytes, truncated tails, len 0 | [x] |
| 13 | `strbuffer_init` + `strbuffer_value` + `strbuffer_close` | fresh buffer (size 16) | [x] |
| 14 | `strbuffer_append_byte` | 1-byte appends across the 16→32→64 growth boundaries | [x] |
| 15 | `strbuffer_append_bytes` | size 0, size == free-1, size == free, size > free (single realloc jump), NUL bytes | [x] |
| 16 | `strbuffer_pop` | on empty, on 1-char, repeatedly to empty | [x] |
| 17 | `strbuffer_clear` then re-append | length/size after clear | [x] |
| 18 | `strbuffer_steal_value` | steal then close (value == NULL) | [x] |
| 19 | `hashtable_init`/`_close` | fresh table: size, order | [x] |
| 20 | `hashtable_set` | 0,1,8,9,16,17,64 distinct keys ⇒ crosses both rehash points | [x] |
| 21 | `hashtable_set` | overwrite existing key (value replaced, size unchanged) | [x] |
| 22 | `hashtable_set` | key_len 0, keys containing embedded NUL, identical prefixes | [x] |
| 23 | `hashtable_get` | present / absent / prefix-of-present / key_len mismatch | [x] |
| 24 | `hashtable_del` | present, absent, then re-insert; delete first/middle/last of a bucket | [x] |
| 25 | `hashtable_clear` then reuse | size/order after clear | [x] |
| 26 | `hashtable_iter`/`_iter_next`/`_iter_key`/`_iter_key_len`/`_iter_value` | full traversal order over 0/1/many entries after mixed insert+delete | [x] |
| 27 | `hashtable_iter_at` | existing key, missing key, key_len 0 | [x] |
| 28 | `hashtable_iter_set` | replace value through an iterator | [x] |
| 29 | `json_object_seed` | explicit non-zero seed installs; second call is a no-op | [x] |
| 30 | `jsonp_loop_check` | first insert (0), duplicate pointer (-1), key/key_len out-params | [x] |
| 31 | `jsonp_strtod` | integers, fractions, exponents, subnormals, huge/tiny, `-0` | [x] |
| 32 | `jsonp_dtostr` | `precision = 0` (mode 0) and 1..31 (mode 2) × doubles across `decpt<=-4`/`decpt>16` | [x] |
| 33 | `jsonp_dtostr` | buffer size 25 (as `dump.c` uses) and larger; every precision | [x] |
| 34 | `dtoa_r` | mode 0/1/2/3/4/5, ndigits 0..25, caller buffer, `rve` out-param | [x] |
| 35 | `dtoa` + `freedtoa` | static-buffer variant, repeated calls (recycles `dtoa_result`) | [x] |
| 36 | `strtod__unused` | decimal, hex (`0x…p…`), inf/nan, partial parses, `se` out-param | [x] |
| 37 | `gethex` | hex float strings, all 4 rounding modes, sign 0/1 | [x] |
| 38 | `dtoa_divmax` | exported data symbol has the same initial value | [x] |
| 39 | `jsonp_error_init` | source NULL and non-NULL; full 248-byte struct compare | [x] |
| 40 | `jsonp_error_set_source` | length < 80, == 79, == 80, > 80 (the `...` truncation) | [x] |
| 41 | `jsonp_error_set` (variadic) | `%s`/`%d`/`%c`/`%.6s` formats, text > 158 bytes (truncation), every `enum json_error_code` value, second call ignored | [x] |

### Value API

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 42 | `json_object`, `json_array` | fresh containers: type + refcount | [x] |
| 43 | `json_true`/`json_false`/`json_null` | singletons: type + `refcount == (size_t)-1` | [x] |
| 44 | `json_integer`, `json_integer_value`, `json_integer_set` | 0, ±1, `INT64_MIN`, `INT64_MAX`, random | [x] |
| 45 | `json_real`, `json_real_value`, `json_real_set` | random finite doubles incl. ±0, subnormal, `f64::MAX` | [x] |
| 46 | `json_number_value` | on integer / real / string / bool / null | [x] |
| 47 | `json_string`, `json_string_value`, `json_string_length` | ASCII, multi-byte UTF-8, empty, 1 byte, long | [x] |
| 48 | `json_stringn` | len < strlen, len > strlen, embedded NUL, len 0 | [x] |
| 49 | `json_string_nocheck`, `json_stringn_nocheck` | invalid UTF-8 accepted; embedded NUL | [x] |
| 50 | `jsonp_stringn_nocheck_own` | takes ownership of a `jsonp_malloc` buffer | [x] |
| 51 | `json_string_set`/`setn`/`set_nocheck`/`setn_nocheck` | replace shorter/longer/empty; invalid UTF-8 via nocheck | [x] |
| 52 | `json_object_set_new` + `json_object_get`/`getn`/`size` | 1,8,9,17,64 keys (rehash), overwrite, key_len variants | [x] |
| 53 | `json_object_setn_new_nocheck` | keys with embedded NUL and invalid UTF-8 | [x] |
| 54 | `json_object_del`/`deln` | delete present/absent, then re-add; delete all | [x] |
| 55 | `json_object_clear` | on empty and on populated object, then reuse | [x] |
| 56 | `json_object_iter`/`iter_at`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value`/`key_to_iter` | full traversal (insertion order) after inserts+deletes+overwrites | [x] |
| 57 | `json_object_iter_set_new` | replace value through iterator | [x] |
| 58 | `json_object_update` | disjoint / overlapping / empty other / self-update | [x] |
| 59 | `json_object_update_existing` | keys present only in `other`, only in `object`, both | [x] |
| 60 | `json_object_update_missing` | same three shapes | [x] |
| 61 | `json_object_update_recursive` | nested objects merged, non-object collisions, deep nesting | [x] |
| 62 | `do_object_update_recursive` (low level) | caller-supplied `parents` hashtable; pre-seeded parents | [x] |
| 63 | `json_array_append_new` | 0→1→8→9→16→17→64 elements (`json_array_grow` doubling) | [x] |
| 64 | `json_array_insert_new` | index 0, middle, `== entries` (append), across growth | [x] |
| 65 | `json_array_set_new` | every valid index, replacing values of different types | [x] |
| 66 | `json_array_remove` | first, middle, last, until empty | [x] |
| 67 | `json_array_clear` | empty and populated, then reuse | [x] |
| 68 | `json_array_extend` | empty+empty, empty+n, n+empty, n+m crossing growth, self-extend | [x] |
| 69 | `json_array_get`/`json_array_size` | every index of random arrays | [x] |
| 70 | `json_equal` | all 8×8 type pairs, equal/unequal scalars, nested containers, key-order-independent objects | [x] |
| 71 | `json_copy` | each of the 8 types; shallow sharing observable via refcounts | [x] |
| 72 | `json_deep_copy` | nested object/array trees, all leaf types | [x] |
| 73 | `do_deep_copy` (low level) | caller-supplied `parents` hashtable | [x] |
| 74 | `json_delete` | on each container type after building; on singletons (no-op) | [x] |
| 75 | `json_sprintf` (variadic) | `%s`/`%d`/`%f`/`%%`, empty result, long result, multi-byte UTF-8 | [x] |

### Encoding (`dump.c`)

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 76 | `json_dumps` | flags 0 on array/object | [x] |
| 77 | `json_dumps` | `JSON_INDENT(n)` for **every** `n ∈ 0..31` on a nested tree (incl. `n>32` spaces wrap in `dump_indent`) | [x] |
| 78 | `json_dumps` | `JSON_COMPACT` alone and combined with indent | [x] |
| 79 | `json_dumps` | `JSON_ENSURE_ASCII` on 1/2/3/4-byte UTF-8 (BMP + surrogate-pair path) | [x] |
| 80 | `json_dumps` | `JSON_SORT_KEYS` with keys of equal prefixes and differing lengths (`compare_keys` len tiebreak) | [x] |
| 81 | `json_dumps` | `JSON_SORT_KEYS \| JSON_COMPACT \| JSON_INDENT(n)` | [x] |
| 82 | `json_dumps` | `JSON_PRESERVE_ORDER` (no-op) alone and with sort | [x] |
| 83 | `json_dumps` | `JSON_ESCAPE_SLASH` with/without slashes | [x] |
| 84 | `json_dumps` | `JSON_ENCODE_ANY` on all 8 scalar types | [x] |
| 85 | `json_dumps` | `JSON_REAL_PRECISION(n)` for every `n ∈ 0..31` × doubles across the exponent switch | [x] |
| 86 | `json_dumps` | `JSON_EMBED` on array and object (omits outer brackets) incl. empty | [x] |
| 87 | `json_dumps` | randomised cross-product of all encode flags on randomised trees | [x] |
| 88 | `json_dumps` | control chars, `"`/`\`, DEL, ` ` (via nocheck strings) | [x] |
| 89 | `json_dumpb` | size 0, size < needed, size == needed, size > needed; return value | [x] |
| 90 | `json_dumpb` | all encode flags; `buffer == NULL` with `size == 0` (size query) | [x] |
| 91 | `json_dumpf` | real `FILE*`, all flag sets, content compared | [x] |
| 92 | `json_dumpfd` | real fd, all flag sets, content compared | [x] |
| 93 | `json_dump_file` | writes then reads back; all flag sets | [x] |
| 94 | `json_dump_callback` | custom callback recording every chunk boundary (chunking is observable) | [x] |
| 95 | `json_dump_callback` | `JSON_EMBED`, nesting depth 64, arrays/objects with 0/1/n children | [x] |

### Decoding (`load.c`)

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 96 | `json_loads` | flags 0 on valid objects/arrays; `error` NULL and non-NULL | [x] |
| 97 | `json_loads` | `JSON_DECODE_ANY` on every scalar at top level | [x] |
| 98 | `json_loads` | `JSON_DECODE_INT_AS_REAL` on integers, incl. > 2^53 and overflowing | [x] |
| 99 | `json_loads` | `JSON_REJECT_DUPLICATES` on inputs with and without duplicates | [x] |
| 100 | `json_loads` | `JSON_DISABLE_EOF_CHECK` with trailing garbage and without | [x] |
| 101 | `json_loads` | `JSON_ALLOW_NUL` with ` ` in values (and in keys) | [x] |
| 102 | `json_loads` | randomised cross-product of all 5 decode flags × generated JSON | [x] |
| 103 | `json_loads` | number grammar: `0`, `-0`, `1e5`, `1E+5`, `1e-5`, `0.5`, big/small exponents, `INT64_MIN/MAX` | [x] |
| 104 | `json_loads` | string escapes `\" \\ \/ \b \f \n \r \t \uXXXX`, surrogate pairs, all-hex-case | [x] |
| 105 | `json_loads` | multi-byte UTF-8 in keys and values (2,3,4-byte) | [x] |
| 106 | `json_loads` | whitespace variants (space/tab/CR/LF) between every token; line/column/position in `error` | [x] |
| 107 | `json_loads` | nesting depth 1, 64, 2047, 2048 (max) | [x] |
| 108 | `json_loadb` | `buflen` shorter than the string, buffer with embedded NUL, `buflen == 0` | [x] |
| 109 | `json_loadf` | `FILE*` source, all decode flags | [x] |
| 110 | `json_loadfd` | fd source, all decode flags | [x] |
| 111 | `json_load_file` | existing file, all decode flags; `error.source` is the path | [x] |
| 112 | `json_load_callback` | chunk sizes 1, 7, 1023, 1024, 1025 ⇒ exercises `MAX_BUF_LEN` refill | [x] |
| 113 | round-trip | `json_loads` → `json_dumps` for a large randomised corpus, all flag pairs | [x] |

### pack / unpack (`pack_unpack.c`)

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 114 | `json_pack` | every scalar format char `s i I b f n` | [x] |
| 115 | `json_pack` | `O` and `o` (refcount differs: incref vs steal) | [x] |
| 116 | `json_pack` | `{}` with 0/1/many keys, `[]` with 0/1/many items, nested | [x] |
| 117 | `json_pack` | `s#` (int length), `s%` (size_t length), `s+` concatenation, `s+#`, `s%+%` | [x] |
| 118 | `json_pack` | optional `s?`, `s*`, `O?`, `O*`, `o?`, `o*` with NULL and non-NULL args | [x] |
| 119 | `json_pack` | separators/whitespace ` \t \n , :` inside the format string | [x] |
| 120 | `json_pack_ex` | `error` populated on success (empty) and flags = 0 / `JSON_STRICT` / `JSON_VALIDATE_ONLY` | [x] |
| 121 | `json_unpack` | every scalar `s i I b f F o O n` against matching values | [x] |
| 122 | `json_unpack` | `s%` (string + length out-params) | [x] |
| 123 | `json_unpack` | `{}` with `s`, `s?` optional keys, subset and full coverage | [x] |
| 124 | `json_unpack` | `[]` with exact, fewer, and `*`-terminated formats | [x] |
| 125 | `json_unpack_ex` | `JSON_STRICT` on exact / extra keys / extra items | [x] |
| 126 | `json_unpack_ex` | `JSON_VALIDATE_ONLY` (no arg consumption) on all format chars | [x] |
| 127 | `json_unpack_ex` | `!` and `*` strictness markers in objects and arrays | [x] |
| 128 | `json_unpack_ex` | skipping mode (`root == NULL` subtree via `s?` on a missing key) | [x] |
| 129 | `json_pack`/`json_unpack` | randomised round-trip: pack a random tree, unpack it back, compare dumps | [x] |

### Locale and hash-seed configuration

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|--------------------------------------------|------|
| 130 | `jsonp_strtod`, `jsonp_dtostr`, `json_loads`, `json_dumps` | `LC_NUMERIC` set to a comma-decimal locale (`de_DE.utf8`) ⇒ `get_decimal_point() != '.'` ⇒ the `to_locale()` branch of `strconv.c` runs and rewrites the strbuffer in place | [x] |
| 131a | `json_object_seed` (first call in the process) | explicit non-zero seed installs verbatim, later calls are no-ops, objects behave identically | [x] |
| 131b | `json_object` / `json_object_seed(0)` (first call in the process) | autoseed from `/dev/urandom`: non-zero, one-shot, and observable behaviour is seed independent | [x] |

Rows 130/131 need their own test binaries (`tests/phase_b_locale.rs`,
`tests/phase_b_seed_explicit.rs`, `tests/phase_b_seed_auto.rs`) because
`hashtable_seed` / `seed_initialized` are process-global one-shot state and
`setlocale()` is process-global.
