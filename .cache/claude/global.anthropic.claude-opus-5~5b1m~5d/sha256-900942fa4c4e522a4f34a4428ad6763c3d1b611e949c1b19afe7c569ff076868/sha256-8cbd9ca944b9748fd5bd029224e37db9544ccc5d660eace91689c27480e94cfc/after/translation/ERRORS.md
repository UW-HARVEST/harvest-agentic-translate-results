# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return NULL`, every implicit
rejection, and every boundary the C arithmetic creates is listed.

Grep of every `return` in the C source:

```
lib.c:9    return 'A' + u;            (encode, static)
lib.c:12   return 'a' + (u - 26);     (encode, static)
lib.c:15   return '0' + (u - 52);     (encode, static)
lib.c:18   return '+';                (encode, static)
lib.c:21   return '/';                (encode, static)
lib.c:34   return NULL;               <-- rejection #1: !src
lib.c:43   return NULL;               <-- rejection #2: !out (calloc failed)
lib.c:82   return out;                (success)
```

There are no `assert`s, no error enums, no range checks other than the loop
bounds, and no min/max constants. The only two rejection sites are the two
`return NULL`s. Rejection #2 is reachable through several *distinct* arithmetic
triggers, so it is broken out into one row per trigger.

`n` below is the allocation request: `n = size * 4 / 3 + 4` computed in `int`
arithmetic, then sign-extended into `calloc`'s `size_t` argument.
C division truncates toward zero, so for negative `size` the quotient rounds up.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `encode_base64` | `src == NULL`, `size` arbitrary (e.g. `0`, `1`, `-1`, `INT_MAX`) — checked *before* anything else, so `size` is never inspected | returns `NULL` |
| 2 | `encode_base64` | `size == -1` → `n = -4/3 + 4 = -1 + 4 = 3` → `calloc(1, 3)` succeeds; loop guard `0 < -1` is false so zero bytes are written | returns **non-NULL** pointer to 3 zero bytes (empty C string) |
| 3 | `encode_base64` | `size == -2` → `n = -8/3 + 4 = -2 + 4 = 2` → `calloc(1, 2)` succeeds; loop body never runs | returns **non-NULL** pointer to 2 zero bytes (empty C string) |
| 4 | `encode_base64` | `size == -3` → `n = -12/3 + 4 = -4 + 4 = 0` → `calloc(1, 0)`; glibc returns a unique non-NULL pointer to a 0-byte region | returns **non-NULL** pointer, 0 readable bytes |
| 5 | `encode_base64` | `size == -4` → `n = -16/3 + 4 = -5 + 4 = -1` → sign-extended to `SIZE_MAX` → `calloc(1, SIZE_MAX)` fails | returns `NULL` (rejection #2) |
| 6 | `encode_base64` | `size == -5` → `n = -20/3 + 4 = -6 + 4 = -2` → `calloc(1, SIZE_MAX-1)` fails | returns `NULL` (rejection #2) |
| 7 | `encode_base64` | any `size <= -4` for which `size * 4` does **not** overflow `int` (i.e. `size > INT_MIN/4`, e.g. `-6`, `-100`, `-1000`, `-1000000`, `-536870911`) → `n < 0` → sign-extended to a near-`SIZE_MAX` request → `calloc` fails | returns `NULL` (rejection #2) |
| 7b | `encode_base64` | `size <= INT_MIN/4` so that `size * 4` **overflows** `int` and `n` comes back `>= 0` (e.g. `INT_MIN + 1` → `size*4` wraps to `4`, `4/3 + 4 = 5` → `calloc(1, 5)` **succeeds**; `INT_MIN/2` wraps to a large positive `n`) → allocation may succeed, and the loop guard `0 < size` is false | returns **non-NULL** zeroed buffer of `n` bytes whenever `calloc` succeeds — i.e. NULL-ness is decided purely by `n = size*4/3+4` and the allocator, never by a range check |
| 8 | `encode_base64` | `size == INT_MIN` → `INT_MIN * 4` wraps to `0`, `0/3 + 4 = 4` → `calloc(1, 4)` **succeeds**, then the loop guard `0 < INT_MIN` is false | returns **non-NULL** pointer to 4 zero bytes (empty C string) |
| 9 | `encode_base64` | `size == 0` with `src` non-NULL: **not** an error — `size` is silently replaced by `strlen(src)`; an empty string therefore yields `n = 4`, `calloc(1,4)`, zero loop iterations | returns **non-NULL** pointer to 4 zero bytes (empty C string) |
| 10 | `encode_base64` | `size == 0` and `strlen(src)` itself is `0` after the `size_t`→`int` truncation (empty string) — same as row 9, documented separately because the truncation is the quirk | returns **non-NULL** empty C string |

## Not testable (undefined behaviour in the C, identical in Rust by construction)

These are *not* rejections; they are recorded so it is clear why no
differential test drives them.

| condition | why untested |
|-----------|--------------|
| `size > 0` larger than the real length of the `src` buffer | out-of-bounds read in **both** implementations; the C is the ground truth and its behaviour here is UB (segfault / garbage), so there is no defined byte-identical result to compare |
| `size` near `INT_MAX` with a genuinely large positive `n` | `size * 4` signed overflow is UB in C; where it wraps to a small `n` (e.g. `INT_MAX` → `n = 3`) the loop then reads `INT_MAX` bytes and crashes. Untestable without a 2 GiB buffer, and the crash is the C's own behaviour |
| non-NUL-terminated `src` with `size == 0` | `strlen` reads past the buffer in both — UB |

## Enum / flag surface

`encode_base64` takes no enum, no flag, no mode, and no option struct. There is
no out-of-range-enum class of input to cover for this API: the only scalar is
`int size`, and *every* `int` bit pattern is a legal input that the C handles.
Rows 1–10 plus the randomized positive-`size` sweep in `CONFIGS.md` cover the
whole `int` domain that does not trip the UB cases above (negatives, zero,
`INT_MIN`, and positives bounded by the buffer).
