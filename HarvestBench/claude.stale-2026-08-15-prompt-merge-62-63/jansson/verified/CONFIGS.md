# CONFIGS.md — Configuration-Surface Table (Phase B gate)

The VALID-input mirror of `ERRORS.md`. Every axis below was derived by grepping the
`if` / `switch` / `#ifdef` branches the C actually takes on each option, not by guessing
which configurations "matter". Line numbers are from `c_src/`.

## Part 1 — Provenance: the exact branch sites

### 1a. Decoder flags — every `flags &` site in `load.c`

| flag | value | site | branch behavior |
|---|---|---|---|
| `JSON_REJECT_DUPLICATES` | 0x1 | `load.c:691-697` | set: `json_object_getn` probe before insert; hit => `json_error_duplicate_key`. clear: `setn_new_nocheck` at `:713` overwrites, and `hashtable.c:243-245` replaces only the value, so a repeated key keeps its **first** ordinal slot in `ordered_list`. |
| `JSON_DISABLE_EOF_CHECK` | 0x2 | `load.c:867-875` | clear: an extra `lex_scan` must yield `TOKEN_EOF`. set: no trailing scan; `error->position` (`:877-880`) = byte offset just past the parsed value. |
| `JSON_DECODE_ANY` | 0x4 | `load.c:856-861` | clear: first token must be `[` or `{`. |
| `JSON_DECODE_INT_AS_REAL` | 0x8 | `load.c:496` — **the only read of `lex->flags`** | clear: `strtoll` fast path, `ERANGE` => overflow error, `TOKEN_INTEGER`. set: falls through the fraction (`:521-532`) / exponent (`:534-547`) no-ops into `jsonp_strtod` (`:551`) => `TOKEN_REAL`. |
| `JSON_ALLOW_NUL` | 0x10 | `load.c:790-796` | clear: `memchr(value,0,len)` => `json_error_null_character`. Object **keys** with NUL are rejected independently at `:684-689`. |
| depth cap | 2048 | `jansson_config.h:10`, checked `load.c:779-783` | `lex->depth` increments per *value* (including scalars); `> 2048` => `json_error_stack_overflow`. Not decremented on that error path. |

Unknown/high flag bits are never validated — silently ignored.

### 1b. Encoder flags — every `flags &` site in `dump.c`

| flag | value | site | branch behavior |
|---|---|---|---|
| `JSON_INDENT(n)` | `flags & 0x1F` | `FLAGS_TO_INDENT` `:29`; used `:73-87` | n>0: `dump("\n",1)` then `depth*n` spaces drawn from `whitespace[]` (`:69`, 32 spaces) in chunks of 32 (`:80-86`), so `depth*n > 32` iterates the chunk loop. n==0 falls to the `else if` at `:88`. |
| `JSON_COMPACT` | 0x20 | `:88` and `:311-317` | `:88` (only when indent==0) suppresses the space after `,`. `:311` picks `":"` vs `": "` **regardless of indent**. |
| `JSON_ENSURE_ASCII` | 0x40 | `:123-124` | breaks the raw-copy loop for `codepoint > 0x7F`: BMP => `\uXXXX` (`:166-169`), non-BMP => surrogate pair `\uXXXX\uXXXX` (`:172-182`). Without the flag the surrogate-pair branch is **unreachable**. `0x7F` (DEL) is never escaped (strict `>`). |
| `JSON_SORT_KEYS` | 0x80 | `:335-387` | malloc `struct key_len[size]`, fill from the iterator, `qsort` with `compare_keys` (`:203-213`: `memcmp` over the shorter length, tie broken by length so a shorter prefix sorts first), then re-fetch each value with `json_object_getn`. Clear => `:388-413` insertion order. |
| `JSON_PRESERVE_ORDER` | 0x100 | declared `jansson.h:386`, referenced in **zero** `.c` files | **Complete no-op** in 2.15; insertion order is always preserved. |
| `JSON_ENCODE_ANY` | 0x200 | `:485-488` | clear: root must be array or object, else -1 before any output. |
| `JSON_ESCAPE_SLASH` | 0x400 | `:119-120`, emit `:161-163` | `/` becomes `\/`. |
| `JSON_REAL_PRECISION(n)` | `(n & 0x1F) << 11` | `FLAGS_TO_PRECISION` `:30`; used `:251-252` | passed as `precision` to `jsonp_dtostr`; `strconv.c:75` sets dtoa `mode = precision==0 ? 0 : 2`; `strconv.c:86` uses exponent form when `decpt <= -4 || decpt > 16`; `strconv.c:101-112` returns -1 when the result would not fit the 25-byte buffer (`MAX_REAL_STR_LENGTH` `dump.c:27`, `digits[25]` `strconv.c:73`). Measured: precision 0..17 fine; 18..31 return NULL for values needing many digits. |
| `JSON_EMBED` | 0x10000 | captured `:217`, **cleared `:219`** (root-only, not inherited), consumed at `:277,281,301,324,330,416` | suppresses the outermost brackets/braces. Empty container + EMBED => zero bytes. With indent, the leading/trailing newlines remain. |

Unconditional escapes (`:115`, no flag): `\`, `"`, and every codepoint `< 0x20`;
`\b \f \n \r \t` get short forms (`:146-160`), other control chars get `\u00XX`.
`dump_string`'s return value is **ignored for object keys** at `:366` and `:396`.

### 1c. Pack / unpack — `pack_unpack.c`

- `JSON_VALIDATE_ONLY` 0x1: `:686,723,738,753,768,783,792,797` — **unpack only**; suppresses
  `va_arg` consumption of value targets. Object **keys** are still consumed (`:520`).
  Consequence: `{s:s%}` + VALIDATE_ONLY **fails**, because the `%` at `:698` is never consumed.
- `JSON_STRICT` 0x2: `:553` (object), `:658` (array) — if the container had no explicit
  `!`/`*`, promote to `!`.
- `pack` never reads `s->flags`, so **both flags are inert for packing**.
- pack switch `:424-460` supports exactly: `{ [ s n b i I f O o`
- unpack switch `:672-818` supports exactly: `{ [ s i I b f F O o n`
- `unpack_value_starters = "{[siIbfFOon"` (`:42`), gating array elements only (`:633`).
- **`'r'` does not exist in 2.15** (neither switch nor the starters string). `'F'` is
  unpack-only; `'#'` is pack-only.
- Decoration chars ignored by `next_token` (`:76`): space, tab, newline, `,`, `:`.
- `#` (int len) / `%` (size_t len) / `+` (concat): `read_string` `:131,175,177,190`;
  `:153-159` rejects them on optional strings.
- `?`/`*`: `pack_string :337-346`, `pack_object_inter :363-385`, `pack_object :240-256`,
  `pack_array :296-307`.
- `!`/`*`: `unpack_object :508-512`, `unpack_array :627-631`; must be last in the container
  (`:495-500`, `:614-619`).
- `?` optional key: `unpack_object :529-532`, sets `gotopt` (`:566`) forcing a full key scan;
  a missing optional recurses with `root=NULL` (format-only skipping mode, `:534-537`).
- unpack object keys use `strlen(key)` (`:525`), so keys with embedded NUL are unreachable.

### 1d. Size / growth thresholds

- `INITIAL_HASHTABLE_ORDER 3` (`hashtable.c:23-25`), `hashsize(n)=1<<n` => 8 buckets;
  rehash at `hashtable.c:234` when `size >= hashsize(order)` => **the 9th distinct key**
  rehashes to order 4, the 17th to order 5.
- `hashlittle` (`lookup3.h`) has three paths by key alignment (4-byte `:206`, 2-byte `:290`,
  unaligned `~:335`) plus a `length>12` loop and a `switch(length)` tail for 0..12 =>
  key lengths 0, 1-3, 4, 11, 12, 13, >24 hit distinct code.
- Array: initial `size=8` (`value.c:438-439`); `json_array_grow` uses
  `new_size = max(size+amount, size*2)` (`value.c:519`) => the 9th append grows to 16, and a
  large `json_array_extend` takes the `size+amount` side.
- `strbuffer`: `STRBUFFER_MIN_SIZE 16`, `new_size = max(size*2, length+size+1)`
  (`strbuffer.c:17-19,61-79`) => 16 -> 32 -> 64.
- `load_callback` reads in `MAX_BUF_LEN 1024` chunks (`load.c:1055`); a callback returning 0
  or `(size_t)-1` means EOF (`:1072`). `stream_t.buffer[5]` (`:57`) holds one UTF-8 sequence
  and is refilled byte-by-byte, so chunk boundaries may split a multi-byte sequence.
- `MAX_INTEGER_STR_LENGTH 25` (`dump.c:26`) — `LLONG_MIN` is 20 chars, fits.
- `LOOP_KEY_LEN = 2 + sizeof(json_t*)*2 + 1` (`jansson_private.h:93`); `jsonp_loop_check`
  (`value.c:47-58`) keys the parents set on `"%p"`.
- `utf8_encode` **accepts** surrogates D800-DFFF (`utf.c:21-25`) while `utf8_check_full`
  **rejects** them (`utf.c:99-102`) — an asymmetry a port must preserve.
- `json_array_append/insert/set_new` and `json_object_set*` reject `json == value`
  (`value.c:131,484,537,560`), so only *indirect* cycles reach the loop checker.
- `json_load_file` on SUCCESS ends with `error->source == "<stream>"` (overwritten by
  `json_loadf`'s `jsonp_error_init`, `load.c:978`); only the fopen-failure path keeps the path.

## Part 2 — Configuration rows

One row per meaningful combination the C treats differently. Each row is driven through the
`.so` exports of BOTH libraries with MANY randomized inputs (fixed seed) and compared
byte-for-byte. Rows marked **(rejected)** are valid-looking configurations the C actually
refuses; the port must refuse identically.

### Encoder — flag axes

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `json_dumps`/`dumpb`/`dumpf`/`dumpfd`/`dump_file`/`dump_callback` | `flags=0`; root `{"arr":[1,2],"o":{"k":null}}` (baseline `", "` and `": "`, insertion order) | [x] |
| 2 | `json_dumps` | `JSON_COMPACT`; same root (`":"`, no space after `,`) | [x] |
| 3 | `json_dumps` | `JSON_INDENT(1)`; nested object+array | [x] |
| 4 | `json_dumps` | `JSON_INDENT(2)`; nested object+array | [x] |
| 5 | `json_dumps` | `JSON_INDENT(4)`; nested object+array | [x] |
| 6 | `json_dumps` | `JSON_INDENT(31)` (= `JSON_MAX_INDENT`); depth 1 only (single `whitespace[]` chunk) | [x] |
| 7 | `json_dumps` | `JSON_INDENT(31)`; depth >= 2 so `depth*31 > 32` (multi-chunk loop `dump.c:79-87`) | [x] |
| 8 | `json_dumps` | `JSON_INDENT(0)` explicitly (same as `flags=0`; else-branch `:88`) | [x] |
| 9 | `json_dumps` | `JSON_INDENT(4)\|JSON_COMPACT` (indent wins for newlines, COMPACT still forces `":"`) | [x] |
| 10 | `json_dumps` | `JSON_ENSURE_ASCII`; string with 2-byte U+00E9, 3-byte U+20AC, 4-byte U+1D11E | [x] |
| 11 | `json_dumps` | `flags=0`; same non-ASCII string (raw UTF-8 passthrough; surrogate branch unreachable) | [x] |
| 12 | `json_dumps` | `JSON_ENSURE_ASCII`; string containing U+007F DEL (NOT escaped: test is `> 0x7F`) | [x] |
| 13 | `json_dumps` | `JSON_ESCAPE_SLASH`; string `"a/b"` -> `"a\/b"` | [x] |
| 14 | `json_dumps` | `flags=0`; string `"a/b"` -> `"a/b"` | [x] |
| 15 | `json_dumps` | `JSON_SORT_KEYS`; object with keys inserted in non-sorted order | [x] |
| 16 | `json_dumps` | `JSON_SORT_KEYS`; one key a prefix of another (`"a"`,`"ab"`) — `compare_keys` length tie-break | [x] |
| 17 | `json_dumps` | `JSON_SORT_KEYS`; single-key object (`qsort` with size 1) | [x] |
| 18 | `json_dumps` | `JSON_SORT_KEYS`; > 8 keys (post-rehash iteration order vs sorted output) | [x] |
| 19 | `json_dumps` | `JSON_SORT_KEYS\|JSON_INDENT(2)` (the sorted branch's own `dump_indent` calls `:373-384`) | [x] |
| 20 | `json_dumps` | `JSON_PRESERVE_ORDER` alone (documented NO-OP: zero references in any `.c`) | [x] |
| 21 | `json_dumps` | `JSON_ENCODE_ANY`; root = each of string / integer / real / true / false / null | [x] |
| 22 | `json_dumps` | `flags=0`; root = a scalar **(rejected: NULL before any output, `:485-488`)** | [x] |
| 23 | `json_dumps` | `JSON_EMBED`; root = 2-key object (no braces) | [x] |
| 24 | `json_dumps` | `JSON_EMBED`; root = 2-element array (no brackets) | [x] |
| 25 | `json_dumps` | `JSON_EMBED`; root = EMPTY object or EMPTY array (zero bytes of output) | [x] |
| 26 | `json_dumps` | `JSON_EMBED\|JSON_INDENT(2)` (leading and trailing newline retained) | [x] |
| 27 | `json_dumps` | `JSON_EMBED`; nested containers (EMBED cleared at `:219`, children keep brackets) | [x] |
| 28 | `json_dumps` | `JSON_ENSURE_ASCII\|JSON_ESCAPE_SLASH\|JSON_SORT_KEYS\|JSON_COMPACT` (all at once) | [x] |
| 29 | `json_dumps` | every unknown high bit set (flags are never validated; must be ignored) | [x] |

### Encoder — real-number / precision axes

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 30 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `0.0`, `-0.0` (dtoa mode 0) | [x] |
| 31 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `0.1` (shortest round-trip) | [x] |
| 32 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `0.1+0.2` -> 17 significant digits | [x] |
| 33 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `1e15` (decpt=16 -> fixed form) | [x] |
| 34 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `1e16` (decpt=17 -> exponent form; the `decpt > 16` boundary) | [x] |
| 35 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `1e-4` (decpt=-3 -> fixed) | [x] |
| 36 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `1e-5` (decpt=-4 -> exponent; the `decpt <= -4` boundary) | [x] |
| 37 | `json_dumps` | `JSON_REAL_PRECISION(0)`; `DBL_MAX` = 1.7976931348623157e308 | [x] |
| 38 | `json_dumps` | `JSON_REAL_PRECISION(0)`; smallest subnormal 5e-324 | [x] |
| 39 | `json_dumps` | `JSON_REAL_PRECISION(1)`; `1/3` -> `"0.3"`, `DBL_MAX` -> `"2e308"` (mode 2, ndigits 1) | [x] |
| 40 | `json_dumps` | `JSON_REAL_PRECISION(2)`; 5e-324 -> `"4.9e-324"` | [x] |
| 41 | `json_dumps` | `JSON_REAL_PRECISION(17)`; `0.1` -> `"0.10000000000000001"` | [x] |
| 42 | `json_dumps` | `JSON_REAL_PRECISION(17)`; `1e-5` -> `"1.0000000000000001e-5"` | [x] |
| 43 | `json_dumps` | `JSON_REAL_PRECISION(20)`; integral `1e15` (fits) | [x] |
| 44 | `json_dumps` | `JSON_REAL_PRECISION(20)`; `1e-4` or `1e300` **(rejected: 25-byte buffer too small -> NULL)** | [x] |
| 45 | `json_dumps` | `JSON_REAL_PRECISION(25)` and `(31)`; any value needing > 17 digits **(rejected)** | [x] |
| 46 | `json_dumps` | `JSON_REAL_PRECISION(31)`; `0.0` (accepted: short output) | [x] |
| 47 | `json_dumps` | integer `LLONG_MIN` and `LLONG_MAX` (20/19 chars, fits `MAX_INTEGER_STR_LENGTH`) | [x] |
| 48 | `json_dumps` | integers `0`, `-1`, `1` | [x] |
| 49 | `json_dumps` + `dtoa_r` | randomized double bit patterns across `JSON_REAL_PRECISION(0..17)`, plus `dtoa_r` modes 0-5 and ndigits 0-20 (the `tests_c/diff_driver.c` matrix) | [x] |

### Encoder — input shapes

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 50 | `json_dumps` | root = empty object `{}` (early return `:328-331`, no `dump_indent`) | [x] |
| 51 | `json_dumps` | root = empty array `[]` (early return `:279-282`) | [x] |
| 52 | `json_dumps` | root = 1-element array / 1-key object (the n-1 loop runs zero times) | [x] |
| 53 | `json_dumps` | root = 2-element array (only shape exercising both loop body and tail) | [x] |
| 54 | `json_dumps` | object with exactly 8 keys, then exactly 9 (pre- vs post-rehash iteration) | [x] |
| 55 | `json_dumps` | array with 8 then 9 elements (pre/post `json_array_grow`) | [x] |
| 56 | `json_dumps` | root nested 100 levels deep with `JSON_INDENT(2)` (recursion + deep indentation) | [x] |
| 57 | `json_dumps` | root containing all 8 `json_type` values as members | [x] |
| 58 | `json_dumps` | empty string `""` value (`dump_string` with len 0) | [x] |
| 59 | `json_dumps` | string with each mandatory escape: `"` `\` `\b` `\f` `\n` `\r` `\t` | [x] |
| 60 | `json_dumps` | string with control chars lacking a short form (0x00,0x01,0x0B,0x0E,0x1F) -> `\u00XX` | [x] |
| 61 | `json_dumps` | string built by `json_stringn` with an embedded NUL -> `"a\u0000b"` | [x] |
| 62 | `json_dumps` | object key containing an embedded NUL (`json_object_setn`), also with `JSON_SORT_KEYS` | [x] |
| 63 | `json_dumps` | object with the empty key `""` (key_len 0) | [x] |
| 64 | `json_dumps` | the same `json_t` twice as siblings (DAG, not a cycle) — legal, dumped twice | [x] |
| 65 | `json_dumps` | indirect cycle `a=[b]`, `b=[a]` **(rejected: `jsonp_loop_check` -> NULL)** | [x] |

### Encoder — low-level entry points

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 66 | `json_dumpb` | `buffer=NULL, size=0` (measure mode: returns required byte count, writes nothing) | [x] |
| 67 | `json_dumpb` | `size ==` exact required length (full write, returns that length) | [x] |
| 68 | `json_dumpb` | `size = 3` (undersized): returns FULL required length, only chunks that fit are copied | [x] |
| 69 | `json_dumpb` | size much larger than needed (no NUL terminator is ever written) | [x] |
| 70 | `json_dumpb` | dump fails (non-container root without `JSON_ENCODE_ANY`) -> returns 0 | [x] |
| 71 | `json_dump_callback` | callback accepting everything and recording the exact chunk sequence | [x] |
| 72 | `json_dump_callback` | callback returning non-zero at chunk k (error propagates; partial output already emitted) | [x] |
| 73 | `json_dumpf` | `FILE*` to a real file; `dump_to_file` requires `fwrite(...,size,1)==1` | [x] |
| 74 | `json_dumpfd` | valid fd; `dump_to_fd` requires `write()` to return exactly `size` | [x] |
| 75 | `json_dump_file` | new path (`fopen "w"`) with `JSON_INDENT(2)`; return also reflects `fclose` | [x] |
| 76 | `json_dump_file` | unopenable path **(rejected: -1 before any encoding)** | [x] |
| 77 | `json_dumps` | after `json_set_alloc_funcs` (no realloc) -> `jsonp_realloc` emulation path `memory.c:45-61`, hit by the final resize `dump.c:438` | [x] |

### Decoder — flag axes

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 78 | `json_loads`/`loadb`/`loadf`/`loadfd`/`load_file`/`load_callback` | `flags=0`; input `{"a":1}` (baseline) | [x] |
| 79 | `json_loads` | `flags=0`; `{"a":1,"a":2}` (last value wins -> `{"a":2}`) | [x] |
| 80 | `json_loads` | `JSON_REJECT_DUPLICATES`; same input **(rejected: `json_error_duplicate_key`)** | [x] |
| 81 | `json_loads` | `JSON_REJECT_DUPLICATES`; distinct keys only (probe runs, no error) | [x] |
| 82 | `json_loads` | `flags=0`; `{"a":1,"b":2,"a":3}` — key `"a"` keeps its FIRST ordinal position | [x] |
| 83 | `json_loads` | `JSON_DISABLE_EOF_CHECK`; `[1] trailing-garbage` (parses `[1]`, ignores rest) | [x] |
| 84 | `json_loads` | `JSON_DISABLE_EOF_CHECK`; `[1][2]` -> `[1]` with `error->position == 3` (streaming resume point) | [x] |
| 85 | `json_loads` | `flags=0`; `[1] x` **(rejected: `json_error_end_of_input_expected`)** | [x] |
| 86 | `json_loads` | `JSON_DISABLE_EOF_CHECK\|JSON_DECODE_ANY`; `1 2 3` (scalar stream) | [x] |
| 87 | `json_loads` | `JSON_DECODE_ANY`; each of `42`, `-1.5`, `"str"`, `true`, `false`, `null` | [x] |
| 88 | `json_loads` | `flags=0`; `42` **(rejected: `"'[' or '{' expected"`)** | [x] |
| 89 | `json_loads` | `JSON_DECODE_INT_AS_REAL`; `[123]` -> `[123.0]` | [x] |
| 90 | `json_loads` | `JSON_DECODE_INT_AS_REAL`; `[9223372036854775808]` (no longer overflow -> 9.223372036854776e18) | [x] |
| 91 | `json_loads` | `JSON_DECODE_INT_AS_REAL`; already-real literal `[1.5e3]` (same path as without the flag) | [x] |
| 92 | `json_loads` | `flags=0`; `[9223372036854775807]` and `[-9223372036854775808]` (exact LLONG bounds) | [x] |
| 93 | `json_loads` | `flags=0`; `[9223372036854775808]` / `[-9223372036854775809]` **(rejected: too big integer)** | [x] |
| 94 | `json_loads` | `JSON_ALLOW_NUL`; `["a\u0000b"]` (string value of length 3 with an embedded NUL) | [x] |
| 95 | `json_loads` | `flags=0`; `["a\u0000b"]` **(rejected: `json_error_null_character`)** | [x] |
| 96 | `json_loads` | `JSON_ALLOW_NUL`; `{"a\u0000b":1}` **(rejected anyway: key check is flag-independent)** | [x] |
| 97 | `json_loads` | all five decoder flags set simultaneously | [x] |
| 98 | `json_loads` | encoder-only bits passed to the decoder (note `JSON_INDENT(n)` low bits ALIAS the decoder flags) | [x] |

### Decoder — input shapes

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 99 | `json_loads` | `{}` and `[]` (empty-container early returns `load.c:668/745`) | [x] |
| 100 | `json_loads` | `[1]` / `{"a":1}` (single element, loop breaks immediately) | [x] |
| 101 | `json_loads` | array/object with 8 vs 9 members (array grow at 9th; object rehash at 9th distinct key) | [x] |
| 102 | `json_loads` | object with 40 keys (two rehashes; insertion order still preserved on dump) | [x] |
| 103 | `json_loads` | nesting depth 2047 arrays + inner scalar (max legal: 2048 values) | [x] |
| 104 | `json_loads` | nesting depth 2048 arrays + inner scalar **(rejected: stack overflow at 2049)** | [x] |
| 105 | `json_loads` | mixed object/array nesting at depth ~1000 | [x] |
| 106 | `json_loads` | every whitespace form between tokens (space, tab, LF, CR) incl. leading and trailing | [x] |
| 107 | `json_loads` | input with LF so error line/column tracking advances (`stream_get load.c:191-199`) | [x] |
| 108 | `json_loads` | number grammar: `0 -0 1 -1 10 1.0 -1.5 1e2 1E2 1e+2 1e-2 1.5e308 0.0001` | [x] |
| 109 | `json_loads` | `[1e999]` **(rejected: real number overflow via HUGE_VAL/ERANGE)** | [x] |
| 110 | `json_loads` | `[-0.0]` (round-trips to `-0.0`) | [x] |
| 111 | `json_loads` | string escapes `\" \\ \/ \b \f \n \r \t` | [x] |
| 112 | `json_loads` | `\uXXXX` BMP escape with lower- and upper-case hex (both `decode_unicode_escape` branches) | [x] |
| 113 | `json_loads` | valid surrogate pair `𝄞` (high D800-DBFF + low DC00-DFFF) | [x] |
| 114 | `json_loads` | lone high surrogate `\uD834`, or a bad second half **(rejected)** | [x] |
| 115 | `json_loads` | lone low surrogate `\uDC00` **(rejected)** | [x] |
| 116 | `json_loads` | raw 1-, 2-, 3- and 4-byte UTF-8 inside a string (`stream_get` multi-byte buffering `:167-186`) | [x] |
| 117 | `json_loads` | 2-byte UTF-8 as the FIRST byte of a token outside a string (`lex_save_cached` `:623`) | [x] |
| 118 | `json_loads` | `true`/`false`/`null` literals (`l_isalpha` identifier scan + `strcmp` `:599-618`) | [x] |
| 119 | `json_loads` | key/value strings of length 0 (`""`), 1, 12, 13, and > 1024 bytes | [x] |
| 120 | `json_loads` | a string whose escapes make the decoded value shorter than the source (`t`/`p` walk `:358-451`) | [x] |

### Decoder — entry points

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 121 | `json_loads` | input containing an embedded NUL (`string_get` treats it as EOF) | [x] |
| 122 | `json_loadb` | `buflen` spanning a NUL byte (`buffer_get` returns the NUL as data — differs from `json_loads`) | [x] |
| 123 | `json_loadb` | `buflen` shorter than `strlen(buffer)` (truncation honored) | [x] |
| 124 | `json_loadb` | `buflen = 0` (premature end of input) | [x] |
| 125 | `json_loadf` | `FILE*` from a regular file, `flags=0` (source `<stream>`) | [x] |
| 126 | `json_loadf` | `input == stdin` (source `<stdin>`, `load.c:973-974`) | [x] |
| 127 | `json_loadfd` | fd of a regular file (1-byte `read()` loop) | [x] |
| 128 | `json_loadfd` | `fd == STDIN_FILENO` (source `<stdin>`) | [x] |
| 129 | `json_loadfd` | `fd < 0` **(rejected: invalid argument)** | [x] |
| 130 | `json_load_file` | existing file, `flags=0` (on SUCCESS `error->source` is overwritten to `<stream>`) | [x] |
| 131 | `json_load_file` | nonexistent path **(rejected: cannot-open-file, source = the path)** | [x] |
| 132 | `json_load_file` | path longer than `JSON_ERROR_SOURCE_LENGTH` (80) -> `"..."`-prefixed truncation | [x] |
| 133 | `json_load_callback` | callback returning 1 byte per call (per-byte refill; multi-byte UTF-8 split across refills) | [x] |
| 134 | `json_load_callback` | callback returning 2- or 3-byte chunks straddling a 3-/4-byte UTF-8 sequence | [x] |
| 135 | `json_load_callback` | callback returning exactly `MAX_BUF_LEN` (1024) then 0 (buffer-boundary refill) | [x] |
| 136 | `json_load_callback` | callback returning `(size_t)-1` (treated as EOF `:1072`) | [x] |
| 137 | all load entry points | `error == NULL` (every `error_set` call becomes a no-op) | [x] |
| 138 | `json_loads` | valid parse with `error != NULL` (`error->position` set at `:877-880` even on success) | [x] |

### Pack — format characters and modifiers

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 139 | `json_pack`/`pack_ex`/`vpack_ex` | `"{}"` (empty object) | [x] |
| 140 | `json_pack` | `"[]"` (empty array) | [x] |
| 141 | `json_pack` | `"{s:i}"` (single key) | [x] |
| 142 | `json_pack` | `"{s:i,s:i}"` (multiple keys) | [x] |
| 143 | `json_pack` | 9+ keys (hashtable rehash during pack) | [x] |
| 144 | `json_pack` | `"[i]"`, `"[i,i]"`, and 9+ elements (array grow during pack) | [x] |
| 145 | `json_pack` | `"{s:[i,i],s:{s:n}}"` (nested containers) | [x] |
| 146 | `json_pack` | decoration-only chars: `" { s : i , s : i } "` | [x] |
| 147 | `json_pack` | `"s"` (top-level scalar; pack has no ENCODE_ANY restriction) | [x] |
| 148 | `json_pack` | `"s"` with the empty string | [x] |
| 149 | `json_pack` | `"s"` with multi-byte UTF-8 (`utf8_check_string` on the arg `:145`) | [x] |
| 150 | `json_pack` | `"s#"` with `(char*, int len)` shorter than `strlen` (truncation) | [x] |
| 151 | `json_pack` | `"s%"` with `(char*, size_t len)` | [x] |
| 152 | `json_pack` | `"s+"` (two args concatenated) | [x] |
| 153 | `json_pack` | `"s++"` (three args concatenated) | [x] |
| 154 | `json_pack` | `"s+#"` (first arg `strlen`'d, second length-limited) | [x] |
| 155 | `json_pack` | `"s#+#"` (both length-limited) | [x] |
| 156 | `json_pack` | `"s%+%"` (both size_t-limited) | [x] |
| 157 | `json_pack` | `"{s#:i}"`, `"{s%:i}"`, `"{s+:i}"` (modifiers on an object KEY, `optional=0`) | [x] |
| 158 | `json_pack` | `"s?"` with a non-NULL string (normal string) | [x] |
| 159 | `json_pack` | `"s?"` with NULL -> `json_null()` | [x] |
| 160 | `json_pack` | `"[s*]"` with NULL (element omitted entirely -> `[]`) | [x] |
| 161 | `json_pack` | `"{s:s*}"` with NULL value (key omitted entirely -> `{}`) | [x] |
| 162 | `json_pack` | `"{s:s?}"` with NULL value -> `{"k":null}` | [x] |
| 163 | `json_pack` | `"s?#"` **(rejected: "Cannot use '#' on optional strings")** | [x] |
| 164 | `json_pack` | `"n"` (json_null singleton) | [x] |
| 165 | `json_pack` | `"b"` with 0 and with non-zero (false / true singletons) | [x] |
| 166 | `json_pack` | `"i"` with 0, INT_MIN, INT_MAX (int-width vararg) | [x] |
| 167 | `json_pack` | `"I"` with LLONG_MIN, LLONG_MAX (`json_int_t` vararg) | [x] |
| 168 | `json_pack` | `"f"` with 0.0, -0.0, 3.5, DBL_MAX, 5e-324 | [x] |
| 169 | `json_pack` | `"f"` with NaN or Inf **(rejected: "Invalid floating point value")** | [x] |
| 170 | `json_pack` | `"O"` with a non-NULL `json_t` (refcount incremented; `"[O,O]"` shares the pointer) | [x] |
| 171 | `json_pack` | `"o"` with a non-NULL `json_t` (reference stolen, no incref) | [x] |
| 172 | `json_pack` | `"O?"` / `"o?"` with NULL -> null | [x] |
| 173 | `json_pack` | `"O*"` / `"o*"` with NULL (value omitted from the enclosing container) | [x] |
| 174 | `json_pack` | `"O"` / `"o"` with NULL and no `?`/`*` **(rejected: `json_error_null_value`)** | [x] |
| 175 | `json_pack` | `"r"` in any position **(rejected: `'r'` is NOT a format char in 2.15)** | [x] |
| 176 | `json_pack` | `"F"` **(rejected: `'F'` is unpack-only)** | [x] |
| 177 | `json_pack_ex` | `JSON_VALIDATE_ONLY\|JSON_STRICT` (both INERT for packing; pack never reads `s->flags`) | [x] |
| 178 | `json_pack_ex` | `error == NULL` vs a real `json_error_t` | [x] |
| 179 | `json_vpack_ex` | `fmt=NULL` or `""` **(rejected: invalid argument)** | [x] |
| 180 | `json_pack` | `"{s:i}x"` **(rejected: "Garbage after format string")** | [x] |
| 181 | `json_sprintf`/`json_vsprintf` | `"%s-%d"`-style fmt producing ASCII | [x] |
| 182 | `json_sprintf` | fmt producing a zero-length result (special-cased -> `json_string("")` `value.c:846-849`) | [x] |
| 183 | `json_sprintf` | fmt producing multi-byte UTF-8; and one producing invalid UTF-8 **(rejected -> NULL)** | [x] |

### Unpack — format characters, modifiers and flags

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 184 | `json_unpack`/`unpack_ex`/`vunpack_ex` | `"{s:s}"` on a matching object | [x] |
| 185 | `json_unpack` | `"{s:s%}"` (also fills a `size_t*` length target) | [x] |
| 186 | `json_unpack` | `"{s:i}"`, `"{s:I}"`, `"{s:b}"` (on true and on false), `"{s:f}"`, `"{s:n}"` | [x] |
| 187 | `json_unpack` | `"{s:F}"` on a REAL member (`json_number_value`) | [x] |
| 188 | `json_unpack` | `"{s:F}"` on an INTEGER member (the only numeric-widening format) | [x] |
| 189 | `json_unpack` | `"{s:f}"` on an integer member **(rejected: "Expected real, got integer")** | [x] |
| 190 | `json_unpack` | `"{s:o}"` (borrowed pointer, no incref) | [x] |
| 191 | `json_unpack` | `"{s:O}"` (incref'd pointer, caller must decref) | [x] |
| 192 | `json_unpack` | `"{s?i}"` with the key PRESENT | [x] |
| 193 | `json_unpack` | `"{s?i}"` with the key MISSING (target untouched, `gotopt` set) | [x] |
| 194 | `json_unpack` | `"{s?{s:i}}"` with the outer key missing (recursion with `root=NULL`: format-only skipping) | [x] |
| 195 | `json_unpack` | `"{s:i,s:i}"` naming the SAME key twice (`key_set` dedupes; still consistent with `!`) | [x] |
| 196 | `json_unpack` | `"{}"` on an empty object | [x] |
| 197 | `json_unpack` | `"{s:i!}"` (strict: all keys must be consumed) | [x] |
| 198 | `json_unpack` | `"{s:i!}"` leaving keys unconsumed **(rejected: "N object item(s) left unpacked: ...")** | [x] |
| 199 | `json_unpack` | `"{s:i*}"` (explicitly non-strict; overrides `JSON_STRICT`) | [x] |
| 200 | `json_unpack_ex` | `JSON_STRICT` with `"{s:i}"` consuming every key (passes) | [x] |
| 201 | `json_unpack_ex` | `JSON_STRICT` with `"{s:i}"` on a larger object **(rejected: promoted to `!`)** | [x] |
| 202 | `json_unpack_ex` | `JSON_STRICT` + explicit `*` in the container (flag suppressed) | [x] |
| 203 | `json_unpack` | `"{s:i!x}"` **(rejected: "Expected '}' after '!'")** | [x] |
| 204 | `json_unpack` | `"[i,i]"` on a 2-element array | [x] |
| 205 | `json_unpack` | `"[]"` on an empty array | [x] |
| 206 | `json_unpack` | `"[i!]"` on a longer array **(rejected: "N array item(s) left unpacked")** | [x] |
| 207 | `json_unpack` | `"[i*]"` on a longer array (accepted) | [x] |
| 208 | `json_unpack_ex` | `JSON_STRICT` with `"[i,i]"` consuming the whole array | [x] |
| 209 | `json_unpack` | `"[i,i,i]"` on a 2-element array **(rejected: `json_error_index_out_of_range`)** | [x] |
| 210 | `json_unpack` | each member of `unpack_value_starters`: `[s] [i] [I] [b] [f] [F] [o] [O] [n] [{...}] [[...]]` | [x] |
| 211 | `json_unpack` | `"[?i]"` **(rejected: `?` is only handled inside objects)** | [x] |
| 212 | `json_unpack` | `"[%]"` or `"[#]"` **(rejected: not in `unpack_value_starters`)** | [x] |
| 213 | `json_unpack_ex` | `JSON_VALIDATE_ONLY`; `"{s:s,s:i}"` with NO value args (keys are still consumed) | [x] |
| 214 | `json_unpack_ex` | `JSON_VALIDATE_ONLY`; `"{s:s%}"` **(rejected: `%` left unconsumed)** | [x] |
| 215 | `json_unpack_ex` | `JSON_VALIDATE_ONLY\|JSON_STRICT` together | [x] |
| 216 | `json_unpack` | `"{s:s#}"` **(rejected: `#` is pack-only)** | [x] |
| 217 | `json_unpack` | `"{s:r}"` or `"[r]"` **(rejected: `'r'` does not exist)** | [x] |
| 218 | `json_unpack` | root scalar with format `i`/`s`/`b`/`n`/`f`/`F`/`o`/`O` (no container required) | [x] |
| 219 | `json_unpack` | wrong root type for `{` or `[` **(rejected: wrong_type with the `type_names[]` string)** | [x] |
| 220 | `json_unpack` | format with space/tab/LF/`,`/`:` decoration | [x] |
| 221 | `json_unpack` | `"{s:i}x"` **(rejected: "Garbage after format string")** | [x] |
| 222 | `json_vunpack_ex` | `root=NULL` **(rejected)**; `fmt=NULL` or `""` **(rejected)** | [x] |
| 223 | `json_unpack` | object with > 8 keys under `!` (the internal `key_set` hashtable itself rehashes) | [x] |

### Value API — objects, arrays, strings, numbers

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 224 | `json_object`/`json_object_size` | brand-new empty object (size 0, order 3, 8 buckets) | [x] |
| 225 | `json_object_set_new`/`_set` | valid UTF-8 key; key `""`; key lengths 1/4/12/13/>24 (hashlittle tail + alignment paths) | [x] |
| 226 | `json_object_setn_new`/`_setn` | explicit `key_len` shorter than `strlen(key)` | [x] |
| 227 | `json_object_setn_new`/`_setn` | key containing an embedded NUL (accepted: NUL passes `utf8_check_string`) | [x] |
| 228 | `json_object_set_new_nocheck`/`setn_new_nocheck` | invalid-UTF-8 key (accepted; the check is skipped) | [x] |
| 229 | `json_object_setn_new` | invalid UTF-8 key **(rejected: -1, value decref'd)** | [x] |
| 230 | `json_object_set*` | value == the object itself **(rejected: -1, prevents direct cycles)** | [x] |
| 231 | `json_object_set*` | key already present (value replaced, ordinal position preserved) | [x] |
| 232 | `json_object_set*` | inserting the 8th then the 9th distinct key (rehash boundary) | [x] |
| 233 | `json_object_get`/`getn` | present key, absent key, key differing only past `key_len`, NUL-containing key | [x] |
| 234 | `json_object_del`/`deln` | present key (0), absent key (-1), NUL-containing key | [x] |
| 235 | `json_object_clear` | populated object then reuse (buckets reset, order retained) | [x] |
| 236 | `json_object_update` | overlapping + disjoint keys (`{"a":1,"b":2}` + `{"b":9,"c":3}`) | [x] |
| 237 | `json_object_update_existing` | same inputs -> `{"a":1,"b":9}`, no new keys | [x] |
| 238 | `json_object_update_missing` | same inputs -> `{"a":1,"b":2,"c":3}`, existing untouched | [x] |
| 239 | `json_object_update_recursive` | nested objects on both sides (merges the nested object) | [x] |
| 240 | `json_object_update_recursive` | value object vs non-object mismatch (replaces instead of merging) | [x] |
| 241 | `json_object_update_recursive` | self-referential `other` **(rejected: loop check -> -1)** | [x] |
| 242 | `json_object_update*` | non-object argument **(rejected: -1)** | [x] |
| 243 | `json_object_iter`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value` | empty object (NULL), 1 key, 9+ keys | [x] |
| 244 | `json_object_iter_at` | present key / absent key (NULL) | [x] |
| 245 | `json_object_key_to_iter` | pointer returned by `iter_key` (`container_of` round trip) | [x] |
| 246 | `json_object_iter_set_new`/`iter_set` | replace a value through an iterator | [x] |
| 247 | `json_object_iter_*` | `iter == NULL` (each returns NULL/0 defensively) | [x] |
| 248 | `json_array`/`json_array_size` | brand-new array (size 8, entries 0) | [x] |
| 249 | `json_array_append_new` | appends 1..8 (no realloc) then the 9th (grow to 16) | [x] |
| 250 | `json_array_insert_new` | index 0, middle, `index == entries` (no memmove), `index > entries` **(rejected)** | [x] |
| 251 | `json_array_set_new` | valid index; `index >= entries` **(rejected)** | [x] |
| 252 | `json_array_remove` | index 0, middle, last (no memmove), out of range **(rejected)** | [x] |
| 253 | `json_array_clear` | populated array (entries 0, capacity kept) | [x] |
| 254 | `json_array_extend` | `other` larger than 2x capacity (takes the `size+amount` side of grow) | [x] |
| 255 | `json_array_extend` | empty `other`; non-array argument **(rejected)** | [x] |
| 256 | `json_array_get` | valid index, `index == entries`, huge index (NULL) | [x] |
| 257 | `json_string`/`json_stringn` | `""`, ASCII, 2/3/4-byte UTF-8, embedded NUL via `json_stringn` | [x] |
| 258 | `json_string`/`json_stringn` | invalid UTF-8 **(rejected: NULL)** | [x] |
| 259 | `json_string_nocheck`/`json_stringn_nocheck` | invalid UTF-8 (accepted) | [x] |
| 260 | `json_string_set`/`setn` | new value shorter and longer than the old; embedded NUL; invalid UTF-8 **(rejected)** | [x] |
| 261 | `json_string_set_nocheck`/`setn_nocheck` | invalid UTF-8 (accepted) | [x] |
| 262 | `json_string_value`/`json_string_length` | on a string; on a non-string (NULL / 0) | [x] |
| 263 | `json_integer`/`_set`/`_value` | 0, -1, LLONG_MIN, LLONG_MAX; wrong type (0 / -1) | [x] |
| 264 | `json_real`/`_set`/`_value` | 0.0, -0.0, subnormal 5e-324, DBL_MAX | [x] |
| 265 | `json_real`/`_set` | NaN and +/-Inf **(rejected: the only values a json_t cannot hold)** | [x] |
| 266 | `json_number_value` | integer, real, and a non-number (0.0) | [x] |
| 267 | `json_true`/`json_false`/`json_null` | singletons with refcount `(size_t)-1` (incref/decref are no-ops) | [x] |
| 268 | `json_equal` | same pointer; type mismatch (integer 1 vs real 1.0 -> 0); NULL argument -> 0 | [x] |
| 269 | `json_equal` | equal/unequal objects (size then per-key), arrays (order-sensitive), strings (length+memcmp, NUL-safe), integers, reals | [x] |
| 270 | `json_copy` | object/array (shallow: children shared), string, integer, real, and a singleton (SAME pointer) | [x] |
| 271 | `json_deep_copy` | nested object+array (children not shared) | [x] |
| 272 | `json_deep_copy` | indirect cycle **(rejected: NULL via loop check)** | [x] |
| 273 | `json_delete`/`json_decref` | each of the 5 heap types; and a singleton (never freed) | [x] |

### Low-level exported helpers

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 274 | `utf8_encode` | codepoint 0 (1 byte), 0x7F/0x80 (1/2 boundary), 0x7FF/0x800 (2/3), 0xFFFF/0x10000 (3/4), 0x10FFFF (4) | [x] |
| 275 | `utf8_encode` | codepoint in D800-DFFF (**ACCEPTED**, encodes 3 bytes — asymmetric with `utf8_check_full`) | [x] |
| 276 | `utf8_encode` | codepoint `< 0` or `> 0x10FFFF` **(rejected: -1)** | [x] |
| 277 | `utf8_check_first` | 0x00-0x7F ->1; 0x80-0xBF ->0; 0xC0,0xC1 ->0; 0xC2-0xDF ->2; 0xE0-0xEF ->3; 0xF0-0xF4 ->4; 0xF5-0xFF ->0 (all 256 bytes) | [x] |
| 278 | `utf8_check_full` | valid 2/3/4-byte sequences; size 1 or > 4 (0); `codepoint` out-param NULL vs non-NULL | [x] |
| 279 | `utf8_check_full` | overlong (E0 80 A9), surrogate (ED A0 80), value > 0x10FFFF, bad continuation **(each 0)** | [x] |
| 280 | `utf8_iterate` | `bufsize 0` (returns buffer unchanged, codepoint untouched); 1-byte; 3-byte; truncated (NULL) | [x] |
| 281 | `utf8_check_string` | length 0 (valid); ASCII; embedded NUL (valid); multi-byte; sequence truncated by length (invalid) | [x] |
| 282 | `strbuffer_init` | fresh buffer (size 16, length 0, `value[0]=='\0'`) | [x] |
| 283 | `strbuffer_append_byte` | single byte; repeated to cross 16 -> 32 -> 64 | [x] |
| 284 | `strbuffer_append_bytes` | size 0; size exactly filling the buffer (always leaves room for NUL); huge size (overflow guards `:66-69`) | [x] |
| 285 | `strbuffer_pop` | non-empty (returns and removes the last byte); EMPTY (returns `'\0'`, length stays 0) | [x] |
| 286 | `strbuffer_clear` | populated buffer (length 0, capacity retained) | [x] |
| 287 | `strbuffer_value` | fresh, populated, and post-clear buffer | [x] |
| 288 | `strbuffer_steal_value` | populated buffer (value set to NULL; `strbuffer_close` afterwards is safe) | [x] |
| 289 | `strbuffer_close` | after steal (value NULL) and without steal | [x] |
| 290 | `hashtable_init`/`hashtable_close` | fresh table (order 3, 8 buckets) | [x] |
| 291 | `hashtable_set` | new key; existing key (value replaced, order kept); 8th then 9th (rehash to order 4); 17th (order 5) | [x] |
| 292 | `hashtable_set` | `key_len 0`; key with embedded NUL; `key_len` at the `offsetof` overflow guard `:205-208` | [x]&dagger; |
| 293 | `hashtable_get` | hit, miss, hash collision within a bucket (`find_pair` walk); `key_len` mismatch on an equal prefix | [x] |
| 294 | `hashtable_del` | hit (0); miss (-1); sole entry in a bucket; first entry; last entry (three distinct relink branches) | [x] |
| 295 | `hashtable_clear` | populated table then reuse | [x] |
| 296 | `hashtable_iter`/`iter_next`/`iter_key`/`iter_key_len`/`iter_value`/`iter_set`/`iter_at` | empty table (NULL), 1 pair, many pairs, iterate to exhaustion | [x] |
| 297 | `jsonp_strtod` | integer-looking text, fraction, exponent, `-0.0`; HUGE_VAL with ERANGE **(rejected: -1)** | [x] |
| 298 | `jsonp_dtostr` | precision 0 (mode 0) vs 1..17 (mode 2); buffer exactly large enough; buffer too small **(rejected)** | [x] |
| 299 | `jsonp_dtostr` | values on both sides of the `decpt <= -4` and `decpt > 16` exponent thresholds; `-0.0` (sign==1) | [x] |
| 300 | `jsonp_strndup` | len 0; len < strlen; len == strlen; data containing NUL bytes | [x] |
| 301 | `jsonp_malloc` | size 0 (returns NULL by design `:26-27`); non-zero size | [x] |
| 302 | `jsonp_free` | NULL pointer (no-op) and a real pointer | [x] |
| 303 | `jsonp_realloc` | with a realloc hook (direct call); with `realloc==NULL` (malloc+memcpy+free emulation); `newSize 0` in emulation mode | [x] |
| 304 | `jsonp_loop_check` | first visit (inserts `"%p"` key, 0); repeat visit (-1); key buffer of exactly `LOOP_KEY_LEN` | [x] |
| 305 | `json_object_seed` | non-zero seed (deterministic hashing); called twice (second ignored once `hashtable_seed != 0`) | [x] |
| 306 | `hashtable_seed` (exported variable) | read as 0 before seeding and non-zero after | [x]&Dagger; |
| 307 | `json_set_alloc_funcs` | custom malloc+free (`do_realloc` forced to NULL -> emulation path everywhere) | [x] |
| 308 | `json_set_alloc_funcs2` | custom malloc+realloc+free | [x] |
| 309 | `json_get_alloc_funcs`/`_2` | all out-params non-NULL; each individually NULL (skipped) | [x] |
| 310 | `jansson_version_str` | returns `"2.15.0"` | [x] |
| 311 | `jansson_version_cmp` | equal (0); lower/higher major; equal major with lower/higher minor; equal major+minor with differing micro | [x] |
| 312 | `jsonp_error_init`/`set`/`vset`/`set_source` | `error==NULL` (no-op); source NULL; source >= 80 chars (truncated); second set on an already-set error (IGNORED `:47-50`) | [x] |
| 313 | `json_error_code` | reads the code smuggled into `text[JSON_ERROR_TEXT_LENGTH-1]` | [x] |

## Notable findings a port must honour

1. **`JSON_PRESERVE_ORDER` (0x100) is referenced in zero `.c` files** — a pure no-op in 2.15.
2. **Format char `'r'` does not exist** in either the pack or unpack switch, nor in
   `unpack_value_starters`. `'F'` is unpack-only; `'#'` is pack-only.
3. `utf8_encode` accepts surrogates while `utf8_check_full` rejects them.
4. `JSON_INDENT(n)`'s low 5 bits alias the five decoder flags, so passing encoder flags to a
   decode function silently enables decoder options.
5. `json_dumpb` returns the FULL required length even when the buffer is too small, and never
   writes a NUL terminator.

## Coverage caveats (the two rows that are only partially reachable)

**&dagger; Row 292 — `init_pair`'s `key_len` overflow guard is not safely testable.**
`hashtable_set` computes `hash = hash_str(key, key_len)` (`hashtable.c:238`) *before*
`init_pair` reaches its `key_len >= (size_t)-1 - offsetof(pair_t, key)` check
(`:205-208`). Any `key_len` large enough to trip the guard therefore makes
`hashlittle` read ~2^64 bytes and segfault first — in the C exactly as much as in
the Rust. The reachable parts of the row (`key_len 0`, keys containing an embedded
NUL, and every `key_len` up to the real buffer length) ARE covered. The same
reasoning applies to `json_object_setn_new` / `getn` / `deln` with an oversized
`key_len`, which is why those appear as `UB` rows in `ERRORS.md`.

**&Dagger; Row 306 — "`hashtable_seed` reads as 0 before seeding" is unobservable from a test.**
`common::libs()` must call `json_object_seed(FIXED_SEED)` on both handles at dlopen
time, because otherwise the seed comes from `/dev/urandom` and object iteration
order (hence every `json_dumps` of a multi-key object) differs between the two
libraries and between runs. By the time any test body runs, the seed is therefore
already set. What IS verified: the exported `hashtable_seed` variable reads back as
the seeded value in both libraries, and re-seeding afterwards is correctly ignored
(row 305).

## Verification record

All 313 rows were exercised against both `.so` files through their exported symbols
only, via `libloading`, and compared byte-for-byte. Row-labelled tests live in
`tests/phase_b_*.rs`; see `verify_all.sh` for the driver and `coverage_report.sh`
for the mechanical row-label cross-check.

```
cargo test --release : 164 tests, 12 binaries, all ok
cargo test (debug)   : 164 tests, 12 binaries, all ok   <- overflow-checks ON
driver stdout        : 75420 lines, byte-identical
symbol parity        : C=130 Rust=130, 0 missing
```

Randomized property sweeps (fixed seeds) back the deterministic rows: ~4500
generated JSON documents across the load/dump flag matrix, 3000-iteration UTF-8
byte-string sweep, 600 random doubles x 18 precisions, 600 pack/unpack rounds,
320-step randomized mutation sequences, and 340 randomized hashtable sequences.
