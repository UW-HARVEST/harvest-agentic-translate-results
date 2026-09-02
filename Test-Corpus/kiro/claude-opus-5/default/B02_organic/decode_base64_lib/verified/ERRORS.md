# ERRORS.md — Error-surface table (Phase C gate)

Derived mechanically from every `return`, rejection branch, allocation check and
range check in `c_src/src/lib.c`. There are no `assert`s, no error enums, no
`RETURN_ERROR`-style macros and no numeric range checks in this library: the
only rejection channel is a `NULL` return from `decode_base64`, plus the
implicit "fall-through" default of the two `static` helpers.

`grep -n 'return\|assert\|NULL\|if ('  c_src/src/lib.c` was used to enumerate
every branch; each distinct rejection / fall-through below is one row.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `decode_base64` | `src == NULL` — `if (src && *src)` fails on the first conjunct (`lib.c:46` → `lib.c:112`) | returns `NULL` | `err_01_null_pointer` | [x] |
| 2 | `decode_base64` | `src` non-NULL but `*src == '\0'` (empty C string `""`) — second conjunct of `if (src && *src)` fails (`lib.c:46` → `lib.c:112`) | returns `NULL` | `err_02_empty_string` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), l + 13)` returns NULL (allocation failure) — `if (!dest) return NULL;` (`lib.c:54-55`) | returns `NULL`; must NOT go on to `malloc`; must NOT `free` | `alloc_contract::part3_calloc_failure` — REAL fault injection | [x] |
| 4 | `decode_base64` | `malloc(l)` returns NULL after `dest` succeeded — `if (!buf) { free(dest); return NULL; }` (`lib.c:61-63`) | returns `NULL`, `dest` freed (exactly one `free`, no leak) | `alloc_contract::part4_malloc_failure_frees_dest` — REAL fault injection | [x] |
| 5 | `decode_base64` | input contains **only** non-base64 characters (e.g. `"!!!"`, `"\n\t "`, high-bit bytes) — `is_base64` rejects every char so the filter loop leaves `l == 0` and the decode loop body never executes (`lib.c:67-72`) | **NOT an error**: returns a valid non-NULL pointer to a zero-filled, NUL-terminated buffer (i.e. the empty string `""`) | `err_05_all_non_base64_returns_empty_not_null` | [x] |
| 6 | `is_base64` | any char outside `[A-Za-z0-9+/=]`, including `0x00`-adjacent control bytes, whitespace, `-`, `_`, `@`, `[`, `` ` ``, `{`, `:`, `*`, `,`, `.`, and all bytes with the high bit set (negative `char` on x86-64 Linux) | returns `FALSE` → char is silently dropped from `buf` (`lib.c:37`) | `err_06_is_base64_rejects_all_non_alphabet_bytes` | [x] |
| 7 | `decode` | fall-through default at `lib.c:25`: a char that is not `[A-Za-z0-9+]`. Reachable from `decode_base64` **only** for `'/'` and `'='`, both of which therefore decode to the value **63** — `'='` shares the sextet value of `'/'`. This is a C quirk and is replicated verbatim, not "fixed". | returns `63` | `err_07_decode_fallthrough_slash_and_equals_both_63` | [x] |
| 8 | `decode_base64` | `'='` appearing at group offset 2 (`c3 == '='`) — suppresses the 2nd output byte (`lib.c:98`); valid input, exercised at *every* group position including mid-string, not just at the tail | non-NULL; 1 or 2 bytes emitted for that group | `err_08_equals_at_c3_suppresses_byte` | [x] |
| 9 | `decode_base64` | `'='` appearing at group offset 3 (`c4 == '='`) — suppresses the 3rd output byte (`lib.c:102`) | non-NULL; 2 bytes emitted for that group | `err_09_equals_at_c4_suppresses_byte` | [x] |
| 10 | `decode_base64` | truncated final group: `l % 4 == 1`, so `k+1 >= l`, `k+2 >= l`, `k+3 >= l` and `c2/c3/c4` keep their `'A'` defaults (`lib.c:79-89`). Reads only `buf[k]`; the C emits 3 bytes for this group because the defaulted `c3`/`c4` are `'A'`, not `'='`. | non-NULL; 3 bytes emitted from 1 input char | `err_10_trailing_group_len_mod4_1` | [x] |
| 11 | `decode_base64` | truncated final group: `l % 4 == 2` — `c3`/`c4` default to `'A'` | non-NULL; 3 bytes emitted | `err_11_trailing_group_len_mod4_2` | [x] |
| 12 | `decode_base64` | truncated final group: `l % 4 == 3` — `c4` defaults to `'A'` | non-NULL; 3 bytes emitted | `err_12_trailing_group_len_mod4_3` | [x] |
| 13 | `decode_base64` | single byte `0x01`..`0xFF` input, every possible value, one at a time — exhaustively separates rows 5/6/7 from the accept path and covers "one step past" each of the four `decode` ranges (`'@'`=0x40, `'['`=0x5B, `` '`' ``=0x60, `'{'`=0x7B, `'/'`=0x2F, `':'`=0x3A, `'*'`=0x2A) | NULL for non-base64 single chars? **No** — non-NULL empty buffer (row 5); non-NULL decoded buffer for base64 chars | `err_13_exhaustive_single_byte` | [x] |
| 14 | `decode_base64` | oversized input (large `strlen`) — `int l = strlen(src) + 1` is an `int`; no bound check exists in the C at all. Tested up to 1 MiB (well below `INT_MAX`, so no UB is invoked); the `2^31`-byte overflow case is not reachable in a test process and both implementations use the identical `int` arithmetic. | non-NULL, decoded output | `err_14_oversized_input_1mib` | [x] |

## Note on rows 3 and 4 (allocation failure)

These branches are only reachable when the allocator fails, so they are driven by
REAL fault injection rather than argued about. `tests/alloc_contract.rs` defines
`calloc`, `malloc` and `free` in the test executable; because the executable is
searched first in the global symbol scope (`-rdynamic`, set in
`.cargo/config.toml`), both dlopened `.so`s bind their PLT slots to those
definitions. Forwarding goes to glibc's `__libc_calloc` / `__libc_malloc` /
`__libc_free` rather than `dlsym(RTLD_NEXT, ...)`, which would itself allocate
and recurse.

With that in place each row is a genuine differential test:

* **row 3** arms `calloc` to return NULL for exactly `strlen(src)+1+13` bytes and
  asserts both implementations return NULL, attempt no `malloc`, and perform no
  `free`;
* **row 4** arms `malloc` to return NULL for exactly `strlen(src)+1` bytes and
  asserts both return NULL having performed exactly **one** `free` — which is
  what actually proves the `free(dest)` cleanup happens and no leak is
  introduced.

The same machinery verifies the sizing contract exactly (see the extra rows
below), which `malloc_usable_size` cannot do: glibc reuses binned chunks and
hands a chunk over whole when the remainder is too small to split, so the usable
size depends on heap state rather than on the requested size. Two false
divergences were produced that way before switching to interposition.

## Extra allocator-contract rows

| # | function | trigger / property | expected C result | test | status |
|---|----------|--------------------|-------------------|------|--------|
| 15 | `decode_base64` | allocation sizing for 307 input lengths (1..300 plus 511, 512, 513, 1000, 4096, 65536, 1 MiB) | exactly one `calloc` of `strlen+1+13`, one `malloc` of `strlen+1`, one `free` | `alloc_contract::part1_exact_alloc_sizes_and_counts` | [x] |
| 16 | `decode_base64` | NULL and empty input | zero `calloc`, zero `malloc`, zero `free` — the C returns before allocating anything | `alloc_contract::part2_null_and_empty_allocate_nothing` | [x] |


## No other rejection channels exist

* no `assert` / `abort` / `exit` in the C source,
* no error enums or out-parameters, so there is **no enum crossing the FFI
  boundary** whose out-of-range integer value could be mishandled (the sole
  parameter is `const char *`),
* the only sentinel is `NULL`, produced by exactly rows 1-4.
