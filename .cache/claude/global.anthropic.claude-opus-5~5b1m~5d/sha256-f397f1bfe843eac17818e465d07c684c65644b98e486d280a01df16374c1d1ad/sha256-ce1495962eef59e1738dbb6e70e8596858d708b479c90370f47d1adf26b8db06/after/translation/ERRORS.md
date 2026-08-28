# ERRORS.md — Phase A error-surface table

Mechanically grepped from `c_src/src/lib.c`. Every `return NULL`, every guard
condition, every allocation check. There are no `assert`s, no error enums, and
no `RETURN_ERROR` macros in this library; the *only* failure channel of the
public API is a `NULL` return.

```sh
$ grep -n 'return\|if (' c_src/src/lib.c
```

## Rejection / error rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `decode_base64` | `src == NULL` — guard `if (src && *src)` (line 46) fails on the first operand | `return NULL` (line 112) | `e01_null_pointer` | [x] |
| 2 | `decode_base64` | `src != NULL` but `*src == '\0'` (empty string) — guard's second operand fails | `return NULL` (line 112) | `e02_empty_string` | [x] |
| 3 | `decode_base64` | `calloc(sizeof(char), l + 13)` returns `NULL` (line 53–56) — out of memory for the destination buffer | `return NULL` (line 55); no leak, nothing allocated yet | `shim_child` `[E03]` | [x] |
| 4 | `decode_base64` | `malloc(l)` returns `NULL` (line 60–64) — `dest` allocation succeeded, scratch-buffer allocation failed | `free(dest); return NULL` (lines 62–63) | `shim_child` `[E04]` (asserts the `free(dest)` actually happens on both sides) | [x] |
| 5 | `decode_base64` | `strlen(src) + 1` overflows `int` (`int l = strlen(src) + 1`, line 49). For `strlen(src) >= INT_MAX` `l` becomes negative, `l + 13` stays negative, and the sign-extended `size_t` argument to `calloc` is astronomically large ⇒ folds into row 3 | `return NULL` via row 3 | `shim_child` `[E05]` | [x] |
| 5b | `decode_base64` | `strlen(src)` returns `real + 2^32`: the `int` truncation is *benign* and decoding must proceed exactly as normal (pins the truncation semantics without UB) | normal non-`NULL` decode | `shim_child` `[E05b]` | [x] |

Rows 3–5 are unreachable through the public API alone, so they are driven in a
child process with `tests/fixtures/interpose.c` in `LD_PRELOAD`, which
interposes `calloc`/`malloc`/`free`/`strlen` for BOTH libraries at once (both
import all four dynamically). Failures are keyed on the exact requested byte
count, so the test harness's own allocations are untouched. Row 5 uses a
`strlen` that reports `INT_MAX` for one marker pointer, which reproduces the
integer overflow without needing a 2 GiB string.

### Deliberately excluded: a genuinely undefined case

If `strlen` were made to report exactly `2^32 - 1`, then `l` would truncate to
`0`, `malloc(0)` would return a 0-byte buffer, and the filter loop would write
the (short, real) input into it — a heap overflow in the **C** itself. That is
undefined behaviour in the ground truth, so "identical behaviour" is not
well-defined and no such row is asserted.

### Notes on what is deliberately *not* an error

These are the near-miss cases that look like rejections but are not — the C
accepts them and returns a non-`NULL` buffer. They are therefore Phase B
(valid-path) rows, and asserting "error" for them would be wrong:

* **Non-base64 characters** are silently *ignored*, not rejected ("Ignore non
  base64 chars as per the POSIX standard", line 66). `is_base64` returning
  `FALSE` skips the character; it never fails the call.
* **A non-empty string containing no base64 characters at all** (e.g. `"!!!"`,
  `"\x80\xff"`) passes the `if (src && *src)` guard, filters down to `l == 0`,
  runs zero decode iterations, and returns the freshly `calloc`'d — hence
  all-zero, empty-C-string — `dest`. **Non-`NULL`.**
* **Malformed / truncated base64** (length not a multiple of 4, stray `=`,
  padding in the middle, `=` as the very first character) is *not* validated.
  The C decodes whatever it has, defaulting missing quartet members to `'A'`.
  **Non-`NULL`.**
* **Invalid characters that reach `decode`**: only `'/'` and `'='` survive
  `is_base64` without an explicit branch in `decode`, and both fall through to
  the unconditional `return 63`. `decode` has no error path.
* **Out-of-range enum values**: not applicable — the API takes no enum, no
  flags, and no length argument; the sole parameter is a `const char *`.

## Generic FFI boundary cases also covered in Phase C

Even though they are not distinct rows above, `tests/errors.rs` additionally
covers: `NULL`, empty string, a 1-byte string, strings whose bytes are all
`0x80..=0xFF` (negative `char`), embedded-`NUL` truncation, and a value one step
past each `decode`/`is_base64` range boundary (`'@'`/`'['`, `` '`' ``/`'{'`,
`'/'`/`':'`, `'*'`/`','`, `'<'`/`'>'`) — i.e. every character class edge, so
that a sign-extension or off-by-one difference in the range checks cannot hide.
