# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-return / sentinel in
`c_src/src/lib.c`. The library has no error enum, no `assert`, no `errno` use —
its *only* failure signal is a `NULL` return from `decode_base64`.

Grep basis:

```
$ grep -n 'return\|assert\|NULL\|if (' c_src/src/lib.c
```

`return` statements in `decode_base64` (lines 46-112):

* `return (NULL);`  after failed `calloc`            (line 54)
* `return (NULL);`  after failed `malloc` + `free`   (line 62)
* `return (dest);`  success                          (line 109)
* `return (NULL);`  fallthrough when guard is false  (line 111)

Guard: `if (src && *src)` (line 47) — two distinct rejection conditions.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `decode_base64` | `src == NULL` (null pointer) — `src &&` short-circuits false | returns `NULL` | `err_01_null_pointer` | [x] |
| 2 | `decode_base64` | `src` points at a valid but **empty** string (`*src == '\0'`) — `*src` is false | returns `NULL` | `err_02_empty_string` | [x] |
| 3 | `decode_base64` | `calloc(1, l + 13)` returns `NULL` (allocation failure) | returns `NULL`, nothing leaked | `err_03_04_allocation_failure_structure` (see note) | [x] |
| 4 | `decode_base64` | `malloc(l)` returns `NULL` (allocation failure) | `free(dest)` then returns `NULL` | `err_03_04_allocation_failure_structure` (see note) | [x] |

### Non-error inputs that *look* like errors (must NOT return NULL)

These are the boundary cases adjacent to the table above. The C code does **not**
reject them, so the Rust must not either — verified in
`tests/differential.rs::err_*`.

| # | trigger | expected C result | test | ✔ |
|---|---------|-------------------|------|---|
| 5 | 1-char string of entirely non-base64 chars, e.g. `"!"` → filtered length `l == 0`, decode loop body never runs | non-`NULL` pointer to an all-zero `calloc`'d buffer (i.e. `""`) | `err_05_all_invalid_chars` | [x] |
| 6 | string of only `'='` padding, e.g. `"===="` | non-`NULL`; `decode('=')` falls through to `return 63`, so byte 1 is `(63<<2)|(63>>4) == 0xFF`; `c3 == '='` and `c4 == '='` suppress bytes 2 and 3, so exactly **1** byte (`0xFF`, *not* `0x00`) is written | `err_06_only_padding` | [x] |
| 7 | filtered length `l % 4 == 1` (a "truncated" base64 group) | non-`NULL`; `c2`/`c3`/`c4` default to `'A'` and **3** bytes are still written | `err_07_dangling_group` | [x] |
| 8 | high-bit / negative `char` bytes (`0x80..0xFF`); `char` is signed on x86-64 so all range checks in `is_base64` are false | non-`NULL`; such bytes are silently dropped by the filter | `err_08_high_bit_bytes` | [x] |
| 9 | embedded `'='` in the *middle* of the input (C does **not** stop at padding) | non-`NULL`; `decode('=')` falls through to `63`, decoding continues past the padding | `err_09_interior_padding` | [x] |
| 10 | out-of-range "enum"-like values across FFI: every single byte `0x01..0xFF` passed as a 1-char string | per-byte identical result (`NULL` never returned; buffer contents identical) | `err_10_every_single_byte` | [x] |
| 11 | oversized length: 1 MiB input string | non-`NULL`; identical buffers | `err_11_oversized_input` | [x] |
| 12 | `src` points to a 0-length slice inside a larger buffer whose first byte is `\0` (zero length) | returns `NULL` (same as row 2) | `err_02_empty_string` | [x] |
| 13 | interior / leading / trailing NUL — the C API is NUL-terminated, so input truncates at the first NUL rather than erroring | non-`NULL`; identical to decoding the truncated prefix | `err_12_interior_nul_truncates` | [x] |

### Note on rows 3 and 4 (allocation-failure paths)

`calloc`/`malloc` failure cannot be induced through the public ABI without
process-wide allocator interposition (`LD_PRELOAD`/`malloc` hooking), which
would perturb the Rust test harness itself and the C library equally and
therefore prove nothing about divergence. These two rows were verified by
**structural equivalence** instead, which is checked mechanically by
`tests/differential.rs::err_03_04_allocation_failure_structure`:

* C: `dest = calloc(...); if (!dest) return NULL;`
  Rust: `let dest = calloc(...) as *mut c_char; if dest.is_null() { return null_mut(); }`
* C: `buf = malloc(l); if (!buf) { free(dest); return NULL; }`
  Rust: `let buf = malloc(l as usize) as *mut c_uchar; if buf.is_null() { free(dest as *mut c_void); return null_mut(); }`

Both use the *same* libc allocator (the Rust translation calls `calloc`/`malloc`
via `extern "C"`, not Rust's global allocator), so their failure thresholds are
identical by construction. The test asserts the source-level shape of both
branches so a future refactor that drops the `free(dest)` or swaps in Rust's
allocator will fail.
