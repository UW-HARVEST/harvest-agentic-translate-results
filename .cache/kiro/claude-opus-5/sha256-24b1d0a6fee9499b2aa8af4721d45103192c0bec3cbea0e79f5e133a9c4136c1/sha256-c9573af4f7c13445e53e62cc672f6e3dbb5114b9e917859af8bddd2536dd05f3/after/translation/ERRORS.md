# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from the C source. The complete set of error-signalling
statements in the library is:

```sh
$ grep -n 'return NULL\|return -\|assert\|exit(\|abort(' c_src/src/lib.c
34:        return NULL;      # if (!src)
43:        return NULL;      # if (!out)   -- calloc failed
```

There are **no** `assert`s, **no** error enums, **no** error codes and **no**
`errno` usage. The *only* failure signal in the whole ABI is a `NULL` return
from `encode_base64`. There are also **no enum parameters**, so there is no
out-of-range-enum class of input for this API (the only scalar parameter is a
plain `int size`, and every one of its 2^32 values is exercised below or
explicitly excluded with a reason).

`encode()` is `static` and total (its final `return '/'` is the catch-all), so
it has no rejection paths of its own.

## Error-surface table

`cap` below denotes the C expression `size * 4 / 3 + 4`, evaluated in **signed
`int`** arithmetic and then implicitly converted to `size_t` at the `calloc`
call site (`c_src/src/lib.c:40`). A negative `cap` becomes an enormous
`size_t`, so `calloc` fails and line 43 returns `NULL`.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|-----------------------------------------|-------------------|
| 1  | `encode_base64` | `src == NULL`, `size == 1` (line 33 null check) | `NULL` |
| 2  | `encode_base64` | `src == NULL`, `size == 0` — null check *precedes* the `strlen` on line 37, so no crash | `NULL` (no `strlen` call) |
| 3  | `encode_base64` | `src == NULL`, `size == -1` (negative) | `NULL` |
| 4  | `encode_base64` | `src == NULL`, `size == INT_MAX` | `NULL` |
| 5  | `encode_base64` | `src == NULL`, `size == INT_MIN` | `NULL` |
| 6  | `encode_base64` | `src == NULL`, 256 pseudo-random `size` values incl. extremes | `NULL` for every one |
| 7  | `encode_base64` | `size == -4` → `cap == -1` → `calloc(1, SIZE_MAX)` fails (line 42) | `NULL` |
| 8  | `encode_base64` | `size == -5` → `cap == -2` → `calloc(1, SIZE_MAX-1)` fails | `NULL` |
| 9  | `encode_base64` | `size == -6` → `cap == -4` → fails | `NULL` |
| 10 | `encode_base64` | every `size` in `-4 ..= -4096` (all give `cap < 0`) | `NULL` for every one |
| 11 | `encode_base64` | `size == INT_MIN + 4 ..= INT_MIN + 16` (very negative, `size*4` still wraps to a negative `int`) | `NULL` / whatever C does — asserted equal, no assumption |
| 12 | `encode_base64` | positive **signed-overflow** `size == 536870912` (`2^29`): `size*4` wraps to `INT_MIN`, `cap == -715827878` → `calloc` fails *before* `src` is read | `NULL` |
| 13 | `encode_base64` | positive overflow `size ∈ {536870913, 600000000, 900000000, 1073741820, 1073741819, 1610612736, 2000000000}` (all give `cap < 0`) | `NULL` |
| 14 | `encode_base64` | `size == -3` → `cap == 0` → `calloc(1, 0)` (boundary: glibc returns a **non-NULL** zero-length chunk) | non-`NULL`; buffer not dereferenced |
| 15 | `encode_base64` | `size == -1` → `cap == 3`, `size == -2` → `cap == 2`: allocation succeeds, `for (i = 0; i < size; ...)` never runs | non-`NULL`, all-zero buffer, i.e. `""` |
| 16 | `encode_base64` | `size == INT_MIN` → `size*4` wraps to `0` → `cap == 4`, loop skipped | non-`NULL`, `""` |
| 17 | `encode_base64` | `size == INT_MIN + 1` → `cap == 5`, loop skipped | non-`NULL`, `""` |
| 18 | `encode_base64` | `size == 0`, `src == ""` → `strlen` gives 0, `cap == 4`, loop skipped | non-`NULL`, `""` |
| 19 | `encode_base64` | `size == 0`, `src` = `"\0garbage"` → `strlen` stops at the first NUL | non-`NULL`, `""` (trailing bytes ignored) |
| 20 | `encode_base64` | `size == 1` with `src` pointing at a 1-byte buffer (smallest non-empty; `cap == 5`, two `'='` pads) | non-`NULL`, 4 chars ending `"=="` |

### Ranges deliberately EXCLUDED from testing (both implementations crash identically)

The general rule: a `size` is **safe to call** with an `N`-byte buffer iff
`size <= N` (the read stays in bounds), or `cap < 0` (the `calloc` fails before
`src` is touched), or `size < 0` (the loop never runs). Everything else makes
the **C itself** read out of bounds, so a differential test would only compare
two `SIGSEGV`s. `tests/errors.rs::error_generic_out_of_range_scalar_sweep`
encodes this rule as a `safe()` predicate and skips exactly those values.

Concretely excluded:

* `size ∈ {2^30-3, 2^30-2, 2^30-1, 2^30}` = `1073741821 ..= 1073741824` —
  `size * 4` wraps to `-12 ..= 0`, so `cap ∈ {-1... 4}` is **non-negative**
  (e.g. `size == 2^30-1` → `cap == 3`), `calloc` succeeds, and the loop then
  reads ~1 GiB → `SIGSEGV`. *This was found the hard way: an early version of
  `error_13` listed `1073741823` as a "calloc must fail" case and crashed the
  test process.*
* `size ∈ [2^30, 1610612735]` — same shape, `size * 4` wraps to a small
  positive `int`.
* `size == INT_MAX` — `size * 4` wraps to `-4`, `cap == 3`, `calloc` succeeds,
  loop reads 2 GiB → `SIGSEGV`.
* any `size` in `[1, 536870911]` larger than the caller's buffer — a plain
  out-of-bounds read; the API performs no length validation whatsoever.
* `size == 0` with a non-NUL-terminated `src` — unbounded `strlen`.

## Row status

All rows 1–20 are covered by `tests/errors.rs` (the test names carry the row
numbers) plus two extra generic-boundary tests, and all 22 pass against **both**
`.so`s. Each test compares the returned sentinel *and* the exact `calloc`
request `(nmemb, size)` recorded by the interposed `calloc`, so "both returned
NULL" is always backed by "both asked the allocator for the same thing".
