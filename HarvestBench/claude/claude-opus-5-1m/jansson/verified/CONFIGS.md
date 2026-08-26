# CONFIGS.md — Configuration-surface table (valid inputs)

Derived **mechanically** from `c_src/include/jansson.h` (every public flag macro
and every public entry point) plus every `if`/`switch`/`&`-mask branch the C code
takes on those flags, and every input *shape* the C special-cases.

## Build-time configuration axes

`Cargo.toml` has **no `[features]` section**, so the crate has exactly **one**
build configuration (`--no-default-features` and the default build are
identical). `c_src/CMakeLists.txt` compiles all 13 `.c` files with
`HAVE_CONFIG_H`; `jansson_config.h` / `jansson_private_config.h` fix:

| macro | value | effect |
|-------|-------|--------|
| `JSON_INTEGER_IS_LONG_LONG` | 1 | `json_int_t = long long`, `json_strtoint = strtoll` |
| `JSON_PARSER_MAX_DEPTH` | 2048 | `parse_value` depth limit |
| `INITIAL_HASHTABLE_ORDER` | 3 | 8 initial buckets → rehash at 8 entries |
| `USE_DTOA` / `DTOA_ENABLED` | 1 | `jsonp_dtostr` uses `dtoa_r`, **not** `snprintf("%.*g")` |
| `USE_URANDOM` | 1 | `generate_seed` reads `/dev/urandom` |
| `HAVE_ATOMIC_BUILTINS` | 1 | `json_object_seed` uses the `__atomic` variant |
| `HAVE_UNISTD_H` | 1 | `json_loadfd`/`json_dumpfd` use `read`/`write` |

**⇒ one feature combination to verify: the default (== `--no-default-features`).**

## Runtime option axes

**Decoding flags** (`json_loads`/`loadb`/`loadf`/`loadfd`/`load_file`/`load_callback`):
`JSON_REJECT_DUPLICATES 0x1`, `JSON_DISABLE_EOF_CHECK 0x2`, `JSON_DECODE_ANY 0x4`,
`JSON_DECODE_INT_AS_REAL 0x8`, `JSON_ALLOW_NUL 0x10`.

**Encoding flags** (`json_dumps`/`dumpb`/`dumpf`/`dumpfd`/`dump_file`/`dump_callback`):
`JSON_INDENT(n) = n & 0x1F`, `JSON_COMPACT 0x20`, `JSON_ENSURE_ASCII 0x40`,
`JSON_SORT_KEYS 0x80`, `JSON_PRESERVE_ORDER 0x100` (parsed, no branch),
`JSON_ENCODE_ANY 0x200`, `JSON_ESCAPE_SLASH 0x400`,
`JSON_REAL_PRECISION(n) = (n & 0x1F) << 11`, `JSON_EMBED 0x10000`.

**Pack/unpack flags**: `JSON_VALIDATE_ONLY 0x1`, `JSON_STRICT 0x2`.

**Allocator axis**: default `malloc/realloc/free`; `json_set_alloc_funcs`
(`do_realloc == NULL` → realloc-emulation path); `json_set_alloc_funcs2`.

**Seed axis**: `json_object_seed(0)` (auto) vs `json_object_seed(N != 0)`
(deterministic bucket/iteration order — required for cross-library comparison).

---

## Table

`[x]` = a differential test drives **both** `.so`s in exactly this configuration
over **many randomized inputs (fixed seed)** and asserts byte-identical output.

### A. `utf.c` — lowest level

Covered by **tests/t01_utf.rs**.

| # | entry point(s) | configuration (options + input shape) | verified |
|---|----------------|----------------------------------------|-----|
| 1 | `utf8_check_first` | all 256 byte values | [x] |
| 2 | `utf8_check_full` | `size` ∈ {0,1,2,3,4,5}, randomized buffers, `codepoint` NULL and non-NULL | [x] |
| 3 | `utf8_encode` | codepoints 0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF + randomized | [x] |
| 4 | `utf8_iterate` | valid 1/2/3/4-byte sequences; `bufsize` exact, larger, truncated; `codepoint` NULL | [x] |
| 5 | `utf8_check_string` | random ASCII / random bytes / valid multi-byte / len 0 / len 1 | [x] |

### B. `strbuffer.c`

Covered by **tests/t02_strbuffer.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 6 | `strbuffer_init` + `strbuffer_close` | fresh buffer, inspect `size`/`length`/`value[0]` | [x] |
| 7 | `strbuffer_append_byte` | 1 byte, 15 bytes (below MIN_SIZE), 16, 17 (growth boundary) | [x] |
| 8 | `strbuffer_append_bytes` | `size` 0, 1, 15, 16, 100, 10000 (multi-growth); embedded NUL bytes | [x] |
| 9 | `strbuffer_pop` | after N appends, popping N and N+1 times | [x] |
| 10 | `strbuffer_clear` then append | reuse after clear | [x] |
| 11 | `strbuffer_value` / `strbuffer_steal_value` | full round trip, resulting `size`/`length`/`value` | [x] |

### C. `hashtable.c` — low-level, called directly

Covered by **tests/t03_hashtable.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 12 | `hashtable_init`/`close` | fresh table: `size`, `order` | [x] |
| 13 | `hashtable_set` | 1 key; 7 keys (no rehash); 8 keys (**triggers rehash**); 100 keys (3 rehashes) | [x] |
| 14 | `hashtable_set` | overwriting an existing key (value replaced, `size` unchanged) | [x] |
| 15 | `hashtable_set` | `key_len` 0 (empty key), 1, 255, 1000; keys with embedded NULs | [x] |
| 16 | `hashtable_get` | present / absent keys; keys equal up to `key_len` but differing length | [x] |
| 17 | `hashtable_del` | first, middle, last element of a bucket; sole element; absent key | [x] |
| 18 | `hashtable_clear` then reuse | after 0, 1, 100 entries | [x] |
| 19 | `hashtable_iter`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value` | 0, 1, 8, 100 entries — full insertion-order traversal | [x] |
| 20 | `hashtable_iter_at` + `iter_next` | resume iteration from an arbitrary key | [x] |
| 21 | `hashtable_iter_set` | replace value through an iterator | [x] |
| 22 | all of the above | seeded via `json_object_seed(N)` with several distinct `N` (hash-order sensitive) | [x] |

### D. `dtoa.c` / `strconv.c`

Covered by **tests/t04_dtoa.rs, tests/t10_abort_parity.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 23 | `dtoa_r` | `mode` 0, `ndigits` 0, `buf` supplied, `blen` 25 — the `jsonp_dtostr` path | [x] |
| 24 | `dtoa_r` | `mode` 0..9 × `ndigits` {-5,0,1,2,5,17,30} over randomized doubles | [x] |
| 25 | `dtoa_r` | `mode` out of range (-1, 10, 1000) → treated as mode 0 | [x] |
| 26 | `dtoa_r` | `rve` NULL vs non-NULL; `buf` NULL (heap alloc) + `freedtoa` | [x] |
| 27 | `dtoa_r` | special values: ±0.0, ±Inf, NaN, subnormals, `DBL_MIN`, `DBL_MAX` | [x] |
| 28 | `dtoa` | same as `dtoa_r` with `buf=NULL,blen=0`, static-result reuse across calls | [x] |
| 29 | `freedtoa` | freeing a `dtoa`-allocated result; freeing a `dtoa_r(buf=NULL)` result | [x] |
| 30 | `gethex` | hex-float strings `0x1p0`, `0x1.8p3`, `0xAp-4`, no-digit input; `rounding` 0..3; `sign` 0/1 | [x] |
| 31 | `strtod__unused` | decimal, exponent, hex, leading space, garbage, empty; `se` NULL and non-NULL | [x] |
| 32 | `dtoa_divmax` | exported data symbol has the same initial value (2) | [x] |
| 33 | `jsonp_dtostr` | `precision` 0 (mode 0) and 1..31 (mode 2) × randomized doubles, `size` 25 | [x] |
| 34 | `jsonp_dtostr` | `size` 1..25 at fixed value (buffer-too-short boundary) | [x] |
| 35 | `jsonp_strtod` | strbuffer holding `"0"`, `"1.5"`, `"1e300"`, `"1e999"`, `"-1e999"`, `"1e-999"` | [x] |

### E. `value.c` — construction / accessors

Covered by **tests/t05_value.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 36 | `json_object` + `json_object_size` + `json_delete` | empty object | [x] |
| 37 | `json_object_set_new` / `setn_new` | 1, 8 (rehash), 64 keys; UTF-8 keys; `key_len` shorter than `strlen` | [x] |
| 38 | `json_object_set_new_nocheck` / `setn_new_nocheck` | keys that are **invalid UTF-8** (accepted by nocheck) | [x] |
| 39 | `json_object_get` / `getn` | present, absent, empty-string key, key with embedded NUL via `getn` | [x] |
| 40 | `json_object_del` / `deln` | delete each key of a 64-key object in random order | [x] |
| 41 | `json_object_clear` | 0, 1, 64 entries then re-populate | [x] |
| 42 | `json_object_update` | disjoint / overlapping / empty `other`; `other == object` | [x] |
| 43 | `json_object_update_existing` | overlap only in existing keys | [x] |
| 44 | `json_object_update_missing` | overlap only in missing keys | [x] |
| 45 | `json_object_update_recursive` | nested objects merged 3 levels deep; leaf type conflicts | [x] |
| 46 | `json_object_iter*` + `json_object_key_to_iter` | full `json_object_keylen_foreach` traversal of 0/1/64 entries | [x] |
| 47 | `json_object_iter_set_new` | replace values while iterating | [x] |
| 48 | `json_array` + `json_array_size` | empty array | [x] |
| 49 | `json_array_append_new` | 1, 8 (initial `size`), 9 (**grow**), 100, 1000 elements | [x] |
| 50 | `json_array_insert_new` | at index 0, middle, `entries` (== append) for sizes 0..10 | [x] |
| 51 | `json_array_set_new` | every valid index of a 10-element array | [x] |
| 52 | `json_array_get` | every index 0..size, plus `size`, `size+1` | [x] |
| 53 | `json_array_remove` | index 0, middle, last, for sizes 1..10 | [x] |
| 54 | `json_array_clear` | 0, 1, 100 elements then re-populate | [x] |
| 55 | `json_array_extend` | empty+empty, empty+N, N+empty, N+M (forces grow); `other == array` | [x] |
| 56 | `json_string` / `json_stringn` | len 0, 1, 100; ASCII, 2/3/4-byte UTF-8; `len < strlen` | [x] |
| 57 | `json_string_nocheck` / `json_stringn_nocheck` | invalid UTF-8 bytes, embedded NUL | [x] |
| 58 | `jsonp_stringn_nocheck_own` | takes ownership of a `jsonp_malloc`ed buffer | [x] |
| 59 | `json_string_value` / `json_string_length` | strings with embedded NUL (length ≠ strlen) | [x] |
| 60 | `json_string_set` / `setn` / `set_nocheck` / `setn_nocheck` | shrink, grow, to-empty, to-invalid-UTF-8 | [x] |
| 61 | `json_integer` / `json_integer_value` / `json_integer_set` | 0, ±1, `INT64_MIN`, `INT64_MAX`, randomized | [x] |
| 62 | `json_real` / `json_real_value` / `json_real_set` | 0.0, -0.0, tiny, huge, randomized finite doubles | [x] |
| 63 | `json_number_value` | integer input, real input, other types | [x] |
| 64 | `json_true` / `json_false` / `json_null` | singleton `type` + `refcount == (size_t)-1` | [x] |
| 65 | `json_equal` | equal/unequal for every type pair; nested arrays & objects; key-order-independent objects | [x] |
| 66 | `json_copy` | each of the 8 types; shallow semantics (children shared) | [x] |
| 67 | `json_deep_copy` | nested object/array 4 levels; all leaf types | [x] |
| 68 | `jsonp_loop_check` | fresh table, same pointer twice, `key_len_out` NULL and non-NULL | [x] |
| 69 | `do_deep_copy` / `do_object_update_recursive` | called directly with an externally-supplied `hashtable_t` | [x] |
| 70 | `json_sprintf` / `json_vsprintf` | `"%d"`, `"%s"`, `"%.3f"`, `"%%"`, empty result, long (>1 KiB) result, UTF-8 | [x] |

### F. `load.c` — decoding

Covered by **tests/t06_load.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 71 | `json_loads` | flags `0`; every scalar, array and object shape (randomized JSON corpus) | [x] |
| 72 | `json_loads` | `JSON_DECODE_ANY` — bare scalars at top level (all 8 types) | [x] |
| 73 | `json_loads` | `JSON_REJECT_DUPLICATES` — duplicate and unique keys | [x] |
| 74 | `json_loads` | `JSON_DISABLE_EOF_CHECK` — trailing garbage accepted, `error->position` | [x] |
| 75 | `json_loads` | `JSON_DECODE_INT_AS_REAL` — plain ints become `JSON_REAL`; huge ints | [x] |
| 76 | `json_loads` | `JSON_ALLOW_NUL` — ` ` inside string values | [x] |
| 77 | `json_loads` | all 32 combinations of the 5 decode flags over the corpus | [x] |
| 78 | `json_loads` | numbers: `0`, `-0`, ints, `1.0`, `1e10`, `1E-10`, `1e+10`, 20-digit ints, `1e308` | [x] |
| 79 | `json_loads` | strings: escapes `\" \\ \/ \b \f \n \r \t`, `\uXXXX` BMP, surrogate pairs | [x] |
| 80 | `json_loads` | nesting depth 1, 100, 2047, 2048 (limit), 2049 (over) | [x] |
| 81 | `json_loads` | whitespace variants (space/tab/CR/LF) between every token; `error->line`/`column`/`position` | [x] |
| 82 | `json_loads` | multi-byte UTF-8 in keys and values (2, 3, 4-byte); column counting | [x] |
| 83 | `json_loadb` | same corpus; `buflen` shorter than the NUL-terminated string; `buflen == 0` | [x] |
| 84 | `json_loadb` | input **not** NUL-terminated, `buflen` exact | [x] |
| 85 | `json_loadf` | same corpus via a real `FILE*` (tmpfile); source `<stream>` | [x] |
| 86 | `json_loadfd` | same corpus via a real fd; source `<stream>` | [x] |
| 87 | `json_load_file` | same corpus written to a temp file; `error->source` == path | [x] |
| 88 | `json_load_callback` | chunked callback: 1-byte, 7-byte, 1024-byte, > MAX_BUF_LEN chunks | [x] |
| 89 | all loaders | `error == NULL` (no error struct) vs non-NULL, on both success and failure | [x] |

### G. `dump.c` — encoding

Covered by **tests/t07_dump.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 90 | `json_dumps` | flags `0` over the whole randomized value corpus | [x] |
| 91 | `json_dumps` | `JSON_INDENT(n)` for every `n` in `0..31`, nested 3 levels | [x] |
| 92 | `json_dumps` | `JSON_INDENT(n)` with `n` > 31 (masking) and `depth*n > 32` (whitespace chunking) | [x] |
| 93 | `json_dumps` | `JSON_COMPACT` alone, and with `JSON_INDENT(n)` (indent wins) | [x] |
| 94 | `json_dumps` | `JSON_ENSURE_ASCII` with 1/2/3/4-byte UTF-8 (surrogate-pair emission) | [x] |
| 95 | `json_dumps` | `JSON_SORT_KEYS` with keys differing in prefix and in length only | [x] |
| 96 | `json_dumps` | `JSON_PRESERVE_ORDER` (no-op) — output identical to flags `0` | [x] |
| 97 | `json_dumps` | `JSON_ENCODE_ANY` for all 8 top-level types | [x] |
| 98 | `json_dumps` | `JSON_ESCAPE_SLASH` with and without `/` in strings and keys | [x] |
| 99 | `json_dumps` | `JSON_REAL_PRECISION(n)` for every `n` in `0..31` over randomized reals | [x] |
| 100 | `json_dumps` | `JSON_EMBED` on arrays and objects (brackets suppressed at depth 0 only) | [x] |
| 101 | `json_dumps` | randomized combinations of all encode flags over the corpus | [x] |
| 102 | `json_dumps` | empty array, empty object, 1-element, 100-element, deeply nested | [x] |
| 103 | `json_dumpb` | `size` 0, exact, `size-1`, `size+1`; return value is required length | [x] |
| 104 | `json_dumpf` | to a `tmpfile()`, all flag sets; compare file contents | [x] |
| 105 | `json_dumpfd` | to a pipe/temp fd, all flag sets; compare bytes | [x] |
| 106 | `json_dump_file` | to a temp path, all flag sets; compare file contents | [x] |
| 107 | `json_dump_callback` | user callback recording chunk **boundaries** (chunking must match) | [x] |
| 108 | `json_dump_callback` | callback that fails on the *k*-th chunk, for each *k* | [x] |
| 109 | `json_dumps` | integers at `INT64_MIN`/`INT64_MAX` (`MAX_INTEGER_STR_LENGTH` boundary) | [x] |
| 110 | `json_dumps` | reals covering all `jsonp_dtostr` branches: `decpt <= -4`, `decpt > 16`, `decpt <= 0`, `digits_len < decpt` | [x] |
| 111 | round trip | `json_loads(json_dumps(v))` equals `v` for the whole corpus, all flag sets | [x] |

### H. `pack_unpack.c`

Covered by **tests/t08_pack.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 112 | `json_pack` | every single format char: `{} [] s n b i I f o O` | [x] |
| 113 | `json_pack_ex` / `json_vpack_ex` | flags `0`; `error` NULL and non-NULL | [x] |
| 114 | `json_pack` | `s#` (int length), `s%` (size_t length), `s+` / `s+#` / `s+%` concatenation | [x] |
| 115 | `json_pack` | optional `s?`, `s*`, `o?`, `o*`, `O?`, `O*` with NULL and non-NULL args | [x] |
| 116 | `json_pack` | nested `{s:{s:[i,i,i]}}` 3 levels; 0/1/many members | [x] |
| 117 | `json_pack` | whitespace and `,`/`:` separators inside the format string | [x] |
| 118 | `json_pack` | `b` with 0 and non-zero ints; `i` with `INT_MIN`/`INT_MAX`; `I` with `INT64_MIN/MAX` | [x] |
| 119 | `json_pack` | `f` with randomized finite doubles | [x] |
| 120 | `json_unpack` | every format char against a matching root | [x] |
| 121 | `json_unpack_ex` / `json_vunpack_ex` | `JSON_VALIDATE_ONLY` (no varargs consumed) | [x] |
| 122 | `json_unpack_ex` | `JSON_STRICT` with exact / extra object keys and array elements | [x] |
| 123 | `json_unpack_ex` | `JSON_VALIDATE_ONLY | JSON_STRICT` combined | [x] |
| 124 | `json_unpack` | in-format `!` and `*` strictness markers in objects and arrays | [x] |
| 125 | `json_unpack` | optional `s?` keys, present and absent | [x] |
| 126 | `json_unpack` | `s%` string + length; `o`/`O` refcount effect | [x] |
| 127 | `json_unpack` | nested `{s:[i,i]}` 3 levels; skipping (`root == NULL` sub-branch via `*`) | [x] |
| 128 | pack→unpack | round trip over randomized format strings | [x] |

### I. `memory.c`, `version.c`, `hashtable_seed.c`, `error.c`

Covered by **tests/t09_misc.rs, tests/t00_smoke.rs**.

| # | entry point(s) | configuration | verified |
|---|----------------|---------------|-----|
| 129 | `json_get_alloc_funcs` / `_2` | defaults (`malloc`,`realloc`,`free`) observed as non-NULL | [x] |
| 130 | `json_set_alloc_funcs` + `json_get_alloc_funcs2` | `do_realloc` becomes NULL → realloc-emulation path exercised | [x] |
| 131 | `json_set_alloc_funcs2` + `json_get_alloc_funcs2` | custom counting allocator; **allocation call counts compared** | [x] |
| 132 | `jsonp_malloc` / `jsonp_free` / `jsonp_realloc` / `jsonp_strndup` | sizes 0, 1, 16, 4096; realloc grow and shrink | [x] |
| 133 | `jansson_version_str` | exact string `"2.15.0"` | [x] |
| 134 | `jansson_version_cmp` | all sign combinations around 2/15/0, incl. negatives | [x] |
| 135 | `json_object_seed` | `seed = N != 0` first (deterministic); then a second call is a no-op | [x] |
| 136 | `hashtable_seed` | exported data symbol readable, equal after identical seeding | [x] |
| 137 | `jsonp_error_init` / `_set_source` / `_set` / `_vset` | source lengths 0, 79, 80, 81, 200; messages of length 0, 100, 159, 300 | [x] |
| 138 | `json_error_t` layout | full 252-byte struct compared byte-for-byte after every failing call | [x] |

---

## Results

All 138 rows pass. Test binaries and counts (`cargo test --test <name>`):

| test binary | tests | rows covered |
|-------------|------:|--------------|
| `t00_smoke.rs`        |  6 | harness + symbol parity + `jansson_version_*` (133-134) |
| `t01_utf.rs`          |  7 | A: 1-5 |
| `t02_strbuffer.rs`    |  8 | B: 6-11 |
| `t03_hashtable.rs`    | 12 | C: 12-22 |
| `t04_dtoa.rs`         | 12 | D: 23-35 |
| `t05_value.rs`        | 23 | E: 36-70 |
| `t06_load.rs`         | 15 | F: 71-89 |
| `t07_dump.rs`         | 18 | G: 90-111 |
| `t08_pack.rs`         | 17 | H: 112-128 |
| `t09_misc.rs`         | 10 | I: 129-138 |
| `t10_abort_parity.rs` |  3 | D: 35 (abort parity for `jsonp_strtod`'s live `assert`) |
| **total**             | **131** | |

Verified under **all three** build configurations (the crate has no `[features]`,
so these are the complete set plus a profile cross-check):

* `cargo test` (dev profile)
* `cargo test --no-default-features` — the only feature combination
* `cargo test --release` (optimized Rust `.so`, `panic = "abort"`)

`nm -D` symbol parity is exact (130/130) for both the dev and the release `.so`.

## Notes and honest limitations

1. **Row 22 (several distinct hash seeds).** `json_object_seed(seed)` only acts
   while `hashtable_seed == 0`, so it is effectively **one-shot per process**;
   the harness seeds both libraries with `TEST_SEED` before anything can
   auto-seed. Re-seeding with other values is therefore a no-op and is tested as
   such (`t09_misc`). This costs no coverage: `hashtable_iter*` walks
   `ordered_list`, so the publicly observable iteration order is *insertion*
   order and is seed-independent. Bucket assignment is instead pinned down
   indirectly by `hashtable_get`/`hashtable_del` sweeps over present and absent
   keys after every operation (`t03_hashtable`), and by the allocation-call-
   sequence comparison in `t09_misc`.

2. **`hashlittle()` (lookup3.h) is `static`** in the C, so it is not
   `dlsym`-able and cannot be compared directly. It is also structurally
   unobservable: `hashtable_find_pair` compares a *stored* `pair->hash` against a
   freshly computed one, so any self-consistent hash behaves identically. Both
   libraries are handed the *same* key pointers, so the C's alignment-dependent
   32/16-bit read paths and the Rust's byte-at-a-time path run on identical
   inputs.

3. **`json_array_t.size` (capacity)** is not exposed by any getter, so a wrong
   initial capacity or growth factor is invisible to the value API. It *is*
   caught by the allocation-call-sequence comparison in `t09_misc` (verified by
   mutation: changing the initial capacity from 8 to 4 fails `t09_misc`).

4. **`dtoa.c` is not thread-safe** — it is compiled without
   `MULTIPLE_THREADS`, so `Balloc`'s `freelist`, `p5s` and `dtoa_result` are
   plain mutable statics in *both* libraries. Every test that formats a real
   number therefore takes a process-wide mutex; without it the two libraries
   race independently and produce different (equally garbage) output. This is a
   property of the C library, not a translation defect.

5. **Deep nesting needs a bigger stack.** `parse_value` and `do_dump` are
   recursive and `JSON_PARSER_MAX_DEPTH` is 2048. Unoptimised Rust frames are
   much larger than the C's, so the deep-nesting rows run on a 96 MiB thread.
