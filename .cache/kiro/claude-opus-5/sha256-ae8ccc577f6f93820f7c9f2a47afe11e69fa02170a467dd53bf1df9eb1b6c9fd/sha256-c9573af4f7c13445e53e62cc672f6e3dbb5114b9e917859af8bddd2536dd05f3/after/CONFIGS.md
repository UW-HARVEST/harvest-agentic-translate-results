# CONFIGS.md — configuration surface (VALID inputs)

Mirror of `ERRORS.md` for *accepted* inputs. Axes derived from the branches the C code
actually takes, not from a guess at what matters.

## Axes found in the C source

**Decoder flags** (`load.c`, `jansson.h`) — every one is read by `parse_json` /
`parse_value` / `parse_object` / `lex_scan_number`:
`JSON_REJECT_DUPLICATES 0x1`, `JSON_DISABLE_EOF_CHECK 0x2`, `JSON_DECODE_ANY 0x4`,
`JSON_DECODE_INT_AS_REAL 0x8`, `JSON_ALLOW_NUL 0x10`.

**Encoder flags** (`dump.c`) — `FLAGS_TO_INDENT(f) = f & 0x1F` (0..31),
`JSON_COMPACT 0x20`, `JSON_ENSURE_ASCII 0x40`, `JSON_SORT_KEYS 0x80`,
`JSON_PRESERVE_ORDER 0x100` (accepted, never branched on), `JSON_ENCODE_ANY 0x200`,
`JSON_ESCAPE_SLASH 0x400`, `FLAGS_TO_PRECISION(f) = (f>>11) & 0x1F` (0..31),
`JSON_EMBED 0x10000`.

**Pack/unpack flags** (`pack_unpack.c`): `JSON_VALIDATE_ONLY 0x1`, `JSON_STRICT 0x2`.

**Allocator mode** (`memory.c`): default `malloc/realloc/free`;
`json_set_alloc_funcs` (sets `do_realloc = NULL` -> realloc-emulation path);
`json_set_alloc_funcs2` (custom realloc).

**Entry points** — all public, low-level included:
decode `json_loads/loadb/loadf/loadfd/load_file/load_callback`;
encode `json_dumps/dumpb/dumpf/dumpfd/dump_file/dump_callback`;
value API (all `json_object_*`, `json_array_*`, `json_string*`, `json_integer*`,
`json_real*`, `json_number_value`, `json_equal`, `json_copy`, `json_deep_copy`,
`json_delete`, `json_true/false/null`, `json_object_seed`);
private/low-level exported symbols `hashtable_*` (13), `strbuffer_*` (8),
`utf8_*` (5), `jsonp_malloc/realloc/free/strndup/strtod/dtostr/loop_check/
stringn_nocheck_own/error_*`, `do_deep_copy`, `do_object_update_recursive`,
`dtoa`, `dtoa_r`, `freedtoa`, `gethex`, `strtod__unused`, `dtoa_divmax`,
`jansson_version_str/cmp`.

**Input shapes the code special-cases** — empty / 1 / many; array growth
(`json_array_grow`: `size` starts at 8, doubles) -> sizes 0,1,7,8,9,16,17,100;
hashtable rehash (`INITIAL_HASHTABLE_ORDER 3` -> 8 buckets, rehash at size >= 8) ->
object sizes 0,1,7,8,9,64; nesting depth 1 / 2 / 2048 / 2049
(`JSON_PARSER_MAX_DEPTH`); UTF-8 byte-length 1/2/3/4 and BMP vs non-BMP (surrogate
pair emission in `dump_string`); integer boundaries `0, +-1, INT_MAX, INT_MIN,
INT64_MAX, INT64_MIN`; reals: 0.0, -0.0, subnormal, 1e-5/1e-4 and 1e16/1e17
(`decpt <= -4 || decpt > 16` exponent switch in `jsonp_dtostr`), 17-digit
round-trippers, `DBL_MAX`, `DBL_MIN`; keys: sorted vs unsorted vs equal-prefix
different-length (`compare_keys` tie-break `k1->len - k2->len`).

Each row is checked off only after **both** `.so`s were driven through it with **many
randomized inputs** (fixed seed, `SplitMix64`, see `tests/common/mod.rs`) and produced
byte-identical results.

---

## A. Low-level entry points (no json_t involved)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| A1 | `utf8_check_first` | all 256 byte values | [x] |
| A2 | `utf8_check_full` | every size 0..=5 x 20 000 randomized buffers, plus 14 hand-built valid / overlong / surrogate / out-of-range sequences | [x] |
| A3 | `utf8_encode` | every codepoint class: <0x80, <0x800, <0x10000, <=0x10FFFF; + randomized | [x] |
| A4 | `utf8_iterate` | bufsize 0..4 x randomized buffers; valid 1/2/3/4-byte sequences | [x] |
| A5 | `utf8_check_string` | randomized byte strings, lengths 0..32; + all-valid UTF-8 strings | [x] |
| A6 | `strbuffer_init` + `strbuffer_value` + `strbuffer_close` | fresh buffer, empty | [x] |
| A7 | `strbuffer_append_byte` | 1 byte; 15 bytes (below MIN_SIZE 16); 16; 17 (forces first realloc) | [x] |
| A8 | `strbuffer_append_bytes` | randomized sizes 0..4096, repeated appends crossing growth boundaries | [x] |
| A9 | `strbuffer_pop` | after n appends, pop n and n+1 times (underflow -> `'\0'`) | [x] |
| A10 | `strbuffer_clear` then append | length reset, value `""` | [x] |
| A11 | `strbuffer_steal_value` | steal, then `strbuffer_close` on the emptied buffer | [x] |
| A12 | `jsonp_malloc`/`jsonp_free` | sizes 0, 1, 4096 | [x] |
| A13 | `jsonp_realloc` | grow / shrink / to-0, default allocator | [x] |
| A14 | `jsonp_realloc` | grow / shrink / to-0 under `json_set_alloc_funcs` (realloc == NULL emulation) | [x] |
| A15 | `jsonp_strndup` | len 0, 1, 32; embedded NUL in source | [x] |
| A16 | `hashtable_init`/`close` | fresh, empty | [x] |
| A17 | `hashtable_set`/`get` | 1 key; 7 keys; 8 keys (rehash boundary); 9; 64; randomized keys+lens | [x] |
| A18 | `hashtable_set` | overwrite existing key (value replaced, size unchanged) | [x] |
| A19 | `hashtable_set` | keys with embedded NULs and equal prefixes / differing `key_len` | [x] |
| A20 | `hashtable_del` | delete first / middle / last / only element, then `get` | [x] |
| A21 | `hashtable_clear` then reuse | size 0, iteration empty, then re-insert | [x] |
| A22 | `hashtable_iter`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value` | full traversal of 0,1,8,64 entries — insertion order | [x] |
| A23 | `hashtable_iter_at` + `iter_next` | resume traversal from a middle key | [x] |
| A24 | `hashtable_iter_set` | replace value through an iterator | [x] |
| A25 | `hashtable_seed` (data symbol) + `json_object_seed(n)` | seed 0 (autoseed, non-zero result) and seed n!=0 (exact value stored) | [x] |
| A26 | `jsonp_dtostr` | precision 0 (mode 0) x 17 (mode 2) x randomized doubles, size 25 | [x] |
| A27 | `jsonp_dtostr` | precision 1..31 x doubles spanning the `decpt<=-4 || decpt>16` exponent switch | [x] |
| A28 | `jsonp_dtostr` | buffer sizes 1..40 for fixed values (finds the exact `-1` threshold) | [x] |
| A29 | `jsonp_strtod` | strbuffer holding randomized numeric literals (int-like, frac, exp, +-) | [x] |
| A30 | `dtoa_r` | modes 0..=5 x every ndigits 0..=25 x 616 doubles (fixed + tame + full-range random); compares digits, `decpt`, `sign` and `*rve` | [x] |
| A31 | `dtoa` + `freedtoa` | modes 0..=3 x ndigits {0,1,6,17} x 608 doubles (heap-allocated result) | [x] |
| A32 | `dtoa_divmax` (data symbol) | read the exported value | [x] |
| A33 | `gethex` | hex-float literals `0x1p+0`, `0x1.8p3`, `0x0p0`, randomized hex mantissa/exponent | [x] |
| A34 | `strtod__unused` | randomized decimal strings (the library's own strtod) | [x] |
| A35 | `jansson_version_str` | constant `"2.15.0"` | [x] |
| A36 | `jansson_version_cmp` | (2,15,0) equal; each component +-1; large and negative components | [x] |
| A37 | `jsonp_error_init` + `jsonp_error_set` + `json_error_code` | source lengths 0, 1, 79, 80, 200; msg lengths 0, 10, 159, 300; codes 0..17 | [x] |
| A38 | `jsonp_error_set_source` | overwrite an already-initialised error; length exactly 79/80/81 | [x] |
| A39 | `jsonp_error_vset` twice | second call ignored (first error retained) | [x] |
| A40 | `jsonp_loop_check` | fresh table + same pointer twice; two different pointers | [x] |
| A41 | `jsonp_stringn_nocheck_own` | takes ownership of a `jsonp_malloc`'d buffer, len 0/1/many | [x] |

## B. Value API (composed, per input shape)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| B1 | `json_object` + `json_object_size` | empty | [x] |
| B2 | `json_object_set_new` / `_setn_new` / `_set_new_nocheck` / `_setn_new_nocheck` | 1, 7, 8, 9, 64 randomized keys (rehash boundary) x all four setters | [x] |
| B3 | `json_object_set_new` | overwrite an existing key (size unchanged, order preserved) | [x] |
| B4 | `json_object_getn` | randomized present + absent keys, `key_len` shorter/longer than strlen | [x] |
| B5 | `json_object_del` / `_deln` | remove first/middle/last, then dump to compare order | [x] |
| B6 | `json_object_clear` | on 0, 1, 64 entries; then re-populate | [x] |
| B7 | `json_object_update` | disjoint / overlapping / self (`object == other`) | [x] |
| B8 | `json_object_update_existing` | overlapping subset, empty other | [x] |
| B9 | `json_object_update_missing` | overlapping subset, empty other | [x] |
| B10 | `json_object_update_recursive` | nested objects merged 3 levels deep; scalar-over-object; object-over-scalar | [x] |
| B11 | `do_object_update_recursive` (direct, with own `hashtable_t` parents) | same shapes as B10 | [x] |
| B12 | `json_object_iter*` + `json_object_key_to_iter` | full traversal of 0/1/8/64 entries; `foreach`-style via `key_to_iter` | [x] |
| B13 | `json_object_iter_at` + `iter_next` | resume from middle | [x] |
| B14 | `json_object_iter_set_new` | replace value at iterator | [x] |
| B15 | `json_array` + `json_array_size` | empty | [x] |
| B16 | `json_array_append_new` | 1, 7, 8, 9, 16, 17, 100 elements (crosses `json_array_grow` doubling) | [x] |
| B17 | `json_array_insert_new` | index 0, middle, `entries` (append-equivalent); on sizes 0..17 | [x] |
| B18 | `json_array_set_new` | every valid index on sizes 1, 7, 8, 9, 16, 17, 100 | [x] |
| B19 | `json_array_remove` | index 0, middle, last (no-move path) on every size 1..=17 | [x] |
| B20 | `json_array_clear` | on 0, 1, 100 elements; then re-append | [x] |
| B21 | `json_array_extend` | other empty / 1 / 100; extend with itself | [x] |
| B22 | `json_array_get` | every index 0..entries-1, randomized | [x] |
| B23 | `json_string`/`json_stringn`/`_nocheck` variants | len 0, 1, ASCII, 2/3/4-byte UTF-8, non-BMP, embedded NUL (`nocheck`) | [x] |
| B24 | `json_string_set`/`setn`/`set_nocheck`/`setn_nocheck` | replace with shorter / longer / empty; all four setters | [x] |
| B25 | `json_string_value` + `json_string_length` | after each B23/B24 shape | [x] |
| B26 | `json_integer` + `_value` + `_set` | 0, +-1, INT32_MIN/MAX, INT64_MIN/MAX, randomized i64 | [x] |
| B27 | `json_real` + `_value` + `_set` | 0.0, -0.0, DBL_MIN, DBL_MAX, subnormal, randomized finite f64 | [x] |
| B28 | `json_number_value` | on integer, real, string, object, array, true, false, null | [x] |
| B29 | `json_true`/`json_false`/`json_null` | singleton identity; refcount `(size_t)-1` unchanged by incref/decref | [x] |
| B30 | `json_equal` | identical / differing at each type; nested objects & arrays; equal-content distinct pointers | [x] |
| B31 | `json_copy` | each of the 8 types; shallow semantics (children shared) | [x] |
| B32 | `json_deep_copy` / `do_deep_copy` | nested object/array 4 levels; all leaf types | [x] |
| B33 | `json_delete` via `json_decref` chain | refcount reaching 0 on each type, nested | [x] |

## C. Encoder configuration matrix (`dump.c`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| C1 | `json_dumps` | flags 0 (default: no indent, `": "` separator) on randomized documents | [x] |
| C2 | `json_dumps` | `JSON_INDENT(n)` for every n in 0..31 (incl. 0 = off, 31 = max, 32 -> wraps to 0) | [x] |
| C3 | `json_dumps` | `JSON_COMPACT` alone; `JSON_COMPACT \| JSON_INDENT(n)` (indent wins in `dump_indent`) | [x] |
| C4 | `json_dumps` | `JSON_ENSURE_ASCII` on ASCII / 2-byte / 3-byte / non-BMP (surrogate pair) strings | [x] |
| C5 | `json_dumps` | `JSON_SORT_KEYS` with unsorted keys, equal prefixes of different length, >=8 keys | [x] |
| C6 | `json_dumps` | `JSON_PRESERVE_ORDER` (accepted, no effect) alone and with `JSON_SORT_KEYS` | [x] |
| C7 | `json_dumps` | `JSON_ENCODE_ANY` on each scalar type at top level | [x] |
| C8 | `json_dumps` | `JSON_ESCAPE_SLASH` on strings containing `/` | [x] |
| C9 | `json_dumps` | `JSON_REAL_PRECISION(n)` for every n in 0..31 x randomized reals | [x] |
| C10 | `json_dumps` | `JSON_EMBED` on top-level array and object (brackets suppressed) | [x] |
| C11 | `json_dumps` | full random flag combinations (random masks over all encoder bits) | [x] |
| C12 | `json_dumpb` | `size` = 0, exact, exact-1, exact+1, huge; returns required length | [x] |
| C13 | `json_dumpf` | `FILE*` over a temp file, flags 0 / `INDENT(3)` / `COMPACT|SORT_KEYS` / `ENSURE_ASCII`; file bytes compared | [x] |
| C14 | `json_dumpfd` | writable temp fd, same four flag sets as C13; file bytes compared | [x] |
| C15 | `json_dump_file` | temp path, flags 0 / indent / sort_keys; file contents compared | [x] |
| C16 | `json_dump_callback` | custom callback collecting chunks; verifies chunk *boundaries* match too | [x] |
| C17 | `json_dumps` | escapes: `"`, `\`, `\b`, `\f`, `\n`, `\r`, `\t`, other `<0x20` -> `\u00XX` | [x] |
| C18 | `json_dumps` | integers at `INT64_MIN`/`INT64_MAX` (MAX_INTEGER_STR_LENGTH 25 boundary) | [x] |
| C19 | `json_dumps` | reals across the `decpt<=-4 || decpt>16` exponent switch, both signs | [x] |
| C20 | `json_dumps` | deeply nested arrays/objects x indent 1 (n_spaces > 32 -> whitespace chunking loop) | [x] |
| C21 | `json_dumps` | empty array `[]` and empty object `{}` inside indented parents | [x] |

## D. Decoder configuration matrix (`load.c`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| D1 | `json_loads` | flags 0, randomized valid JSON documents (generated recursively) | [x] |
| D2 | `json_loads` | `JSON_DECODE_ANY` on each scalar at top level | [x] |
| D3 | `json_loads` | `JSON_DECODE_INT_AS_REAL` on integer / big-integer / real inputs | [x] |
| D4 | `json_loads` | `JSON_DISABLE_EOF_CHECK` with trailing garbage / trailing whitespace | [x] |
| D5 | `json_loads` | `JSON_REJECT_DUPLICATES` on documents with and without duplicate keys | [x] |
| D6 | `json_loads` | `JSON_ALLOW_NUL` with `\u0000` in values | [x] |
| D7 | `json_loads` | all 32 combinations of the 5 decoder flags x a fixed corpus | [x] |
| D8 | `json_loadb` | `buflen` shorter than the string (truncation), exact, with trailing NUL, `buflen` 0 | [x] |
| D9 | `json_loadf` | `FILE*` over the same corpus (via `tmpfile()` + rewind) | [x] |
| D10 | `json_loadfd` | fd over the same corpus | [x] |
| D11 | `json_load_file` | temp file over the same corpus | [x] |
| D12 | `json_load_callback` | callback returning 1 byte at a time, `MAX_BUF_LEN`(1024) chunks, and > 1024-byte documents | [x] |
| D13 | `json_loads` | escapes: `\" \\ \/ \b \f \n \r \t`, `\uXXXX` in BMP, surrogate pairs | [x] |
| D14 | `json_loads` | multi-byte UTF-8 input (2/3/4-byte) -> `stream_get` multi-byte path; line/column tracking | [x] |
| D15 | `json_loads` | numbers: `0`, `-0`, big ints, `1.0`, `1e10`, `1E+10`, `1e-10`, `-1.5e-3`, 17-sig-digit reals | [x] |
| D16 | `json_loads` | nesting depth 1, 2, 2048 (max allowed) | [x] |
| D17 | `json_loads` | whitespace forms: spaces, tabs, `\n`, `\r` between every token; error line/col after newlines | [x] |
| D18 | `json_loads` | `error` struct populated on SUCCESS too (`error->position` set by `parse_json`) | [x] |
| D19 | `json_loads` + `json_dumps` | round-trip: load then dump with flags 0 and `JSON_SORT_KEYS`, compare bytes | [x] |
| D20 | `json_loads` | 1000-element array / 1000-key object (growth + rehash under the parser) | [x] |

## E. Pack / unpack configuration matrix (`pack_unpack.c`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| E1 | `json_pack` | every scalar specifier: `n b i I f s` | [x] |
| E2 | `json_pack` | `s#` (int len), `s%` (size_t len), `s+` (concat), `s+#`, `s+%` | [x] |
| E3 | `json_pack` | `s?` and `s*` with non-NULL and NULL args | [x] |
| E4 | `json_pack` | `o` / `O` (with and without `?` / `*`), refcount effects observed via dump | [x] |
| E5 | `json_pack` | nested `{s:[i,i],s:{s:s}}`, whitespace/`,`/`:` skipping in the format string | [x] |
| E6 | `json_pack_ex` / `json_vpack_ex` | flags 0 and `JSON_STRICT` (ignored by pack) with an `error` struct | [x] |
| E7 | `json_sprintf` / `json_vsprintf` | empty result, `%d`/`%s`/`%f` formats, result > 160 bytes, UTF-8 args | [x] |
| E8 | `json_unpack` | every scalar specifier `n b i I f F s o O` against matching roots | [x] |
| E9 | `json_unpack` | `s%` length out-param; `s?` optional-missing and optional-present | [x] |
| E10 | `json_unpack` | `{}`/`[]` nesting, `!` strict marker, `*` non-strict marker | [x] |
| E11 | `json_unpack_ex` | `JSON_VALIDATE_ONLY` (no varargs consumed) on every specifier | [x] |
| E12 | `json_unpack_ex` | `JSON_STRICT` on exact-match and superset roots | [x] |
| E13 | `json_unpack_ex` | `JSON_VALIDATE_ONLY \| JSON_STRICT` combined | [x] |
| E14 | `json_vunpack_ex` | called directly with a `va_list` | [x] |
| E15 | `json_pack` -> `json_dumps` -> `json_loads` -> `json_unpack` | full round-trip on randomized documents | [x] |

## F. Allocator configuration (`memory.c`) — cross-cuts every row above

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| F1 | `json_get_alloc_funcs` / `_2` | read the defaults; NULL out-params | [x] |
| F2 | `json_set_alloc_funcs2` + counting allocator | rerun a representative subset of B/C/D/E; compare malloc/realloc/free *call counts and sizes* | [x] |
| F3 | `json_set_alloc_funcs` (realloc = NULL) | rerun the same subset -> exercises the `jsonp_realloc` emulation path in array growth, strbuffer growth, `json_dumps` | [x] |
| F4 | failing allocator (Nth allocation returns NULL) | N swept over `json_loads`/`json_dumps`/`json_pack`/`json_object_set` — every OOM row in `ERRORS.md` | [x] |
| F5 | restore defaults via `json_set_alloc_funcs2(malloc,realloc,free)` | subsequent behaviour identical to F1 baseline | [x] |

---

## Where each section is tested

| section | test file |
|---------|-----------|
| A (low-level entry points) | `translation/tests/valid_lowlevel.rs` |
| A25 (hash seed; needs a virgin process) | `translation/tests/seed.rs` |
| B (value API) | `translation/tests/valid_value.rs` |
| C (encoder matrix) | `translation/tests/valid_dump.rs` |
| D (decoder matrix) | `translation/tests/valid_load.rs` |
| E (pack / unpack / sprintf, incl. the `va_list` forms via a C shim) | `translation/tests/valid_pack.rs` |
| F (allocators, allocation traces, OOM sweep) | `translation/tests/alloc.rs` |
| symbol parity | `translation/tests/symbols.rs` |

The shared harness is `translation/tests/common/mod.rs`: it `dlopen`s both `.so` files,
exposes all 130 exported symbols as `extern "C"` function pointers, seeds both hash
tables to `TEST_SEED`, serialises tests (`lock()`) because `dtoa`'s `Balloc` freelist and
`memory.c`'s allocator hooks are process-global and deliberately not thread-safe in the C,
and builds a small C shim at test time so `json_vpack_ex` / `json_vunpack_ex` /
`json_vsprintf` can be called with a real `va_list`.

## Feature combinations (Phase D)

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has exactly one
configuration: the default (empty) feature set. `cargo check`, `cargo build --release`,
the `nm -D` symbol diff and the whole test suite are nevertheless run under `DEFAULT`,
`--no-default-features` and `--all-features` by `tests/run_feature_matrix.sh`; the script
also enumerates the powerset of any features that are added later. Latest run:
`tests/out/feature_matrix.txt` — 102 tests passed and an empty symbol diff in all three.
