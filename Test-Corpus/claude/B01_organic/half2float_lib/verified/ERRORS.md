# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

## Mechanical derivation

Every non-table line of `c_src/src/lib.c` (`grep -nvE '^\s*(0x[0-9a-fA-F]+,?\s*)+\}?;?\s*$'`):

```
  1: #include "lib.h"
  3: static uint32_t m__mantissa[2048] = {
346: static uint16_t m__offset[64] = {
355: static uint32_t m__exponent[64] = {
368: float half2float(uint16_t h) {
369:     union {
370:         float flt;
371:         uint32_t num;
372:     } out;
373:     int n = h >> 10;
374:     out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
375:     return out.flt;
376: }
```

Grep results for rejection machinery:

| pattern searched | matches in `c_src/` |
|------------------|---------------------|
| `RETURN_ERROR`               | 0 |
| `return -1` / `return NULL`  | 0 |
| `assert` / `NDEBUG`          | 0 |
| `errno`                      | 0 |
| `if` / `else` / `switch` / `goto` | 0 |
| `for` / `while`              | 0 |
| ternary `?:`                 | 0 |
| `#if` / `#ifdef` / `#define` | 0 |
| explicit range / min / max check | 0 |
| pointer parameter (⇒ null check) | 0 (the only parameter is a by-value `uint16_t`) |
| error enum / status type      | 0 (return type is `float`) |

`grep -cE '\b(if|else|switch|while|for|goto|assert|return)\b' c_src/src/lib.c` → **1**
(the single `return out.flt;`).

## Conclusion: the error surface is EMPTY BY CONSTRUCTION

`half2float` is a total, branch-free function over its entire domain. It has:

* no pointer parameters → **no null-pointer rejection path**,
* no length/count parameters → **no zero-length or oversized-length path**,
* no enum parameter → **no invalid-enum path**,
* no `float`/status error sentinel → **no error code to compare**,
* no explicit range check → **every one of the 65 536 `uint16_t` values is valid input**.

The only way to fault the C is to violate its declared signature (which is a
caller bug, not a library rejection). The table below therefore enumerates the
**implicit** rejection/boundary conditions — the ones that *would* trap if the
index arithmetic could leave its domain — plus the signature-boundary cases a
real FFI caller can actually produce. Each row has a differential test in
`tests/error_paths.rs`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `half2float` | `h = 0x0000` — smallest input; drives `m__offset[0] = 0` so mantissa index is the **lower bound 0** (`m__mantissa[0]`). Any off-by-one under-index would fault here. | no error; returns bits `0x00000000` (`+0.0`) | [x] |
| E2 | `half2float` | `h = 0xFFFF` — largest input; drives `n = 63` (**upper bound of both 64-entry tables**) and mantissa index `1023 + 0x400 = 2047` (**upper bound of the 2048-entry table**). Any off-by-one over-index would fault here. | no error; returns bits `0xFFFFE000` (a negative NaN) | [x] |
| E3 | `half2float` | `n = h >> 10` maximal ⇒ `m__offset[n]` / `m__exponent[n]` read at index 63; C declares both arrays `[64]`, so index 64 would be OOB. Exercised for every `h` in `0xFC00..=0xFFFF`. | no error; index 63 is in range for all `uint16_t` | [x] |
| E4 | `half2float` | mantissa index maximal: `(h & 0x3ff) == 0x3ff` **and** `m__offset[n] == 0x400` ⇒ index `2047`; index 2048 would be OOB. Exercised for every `h` with low 10 bits set and `n ∉ {0,32}`. | no error; index 2047 is the last valid element | [x] |
| E5 | `half2float` | mantissa index taken from the **offset-0** rows (`n == 0` or `n == 32`, i.e. `m__offset[n] == 0x0000`) with `h & 0x3ff == 0x3ff` ⇒ index `1023`, i.e. the *other* branch of the two-region index space. | no error; returns the `m__mantissa[1023]`-based value | [x] |
| E6 | `half2float` | `uint32_t` addition `m__mantissa[i] + m__exponent[n]` — C wraps modulo 2³² (unsigned overflow is defined); Rust `+` would **panic in debug** on overflow. Worst case is `n = 63`: `0xC7800000 + 0x387FE000 = 0xFFFFE000`. Verified over the whole domain that no wrap occurs, and that Rust uses `wrapping_add` so it could not panic even if it did. | no error, no trap; exact `u32` sum | [x] |
| E7 | `half2float` | Value **one step past the declared `uint16_t` range** pushed across the FFI boundary: caller uses a wider prototype (`extern "C" fn(u32) -> f32`) and passes `0x10000`, `0x1FFFF`, `0xFFFFFFFF`. C's `uint16_t` parameter truncates to the low 16 bits. | no error; identical to `half2float(h & 0xFFFF)`, and C and Rust must agree bit-for-bit | [x] |
| E8 | `half2float` | Negative / sign-extended value pushed across the same widened prototype (`-1`, `-32768`, `i32::MIN`) — the "out-of-range enum passed as int" analogue for this API, since C accepts any `int` at a `uint16_t` parameter. | no error; identical to `half2float((v as u32) & 0xFFFF)`; C and Rust agree bit-for-bit | [x] |
| E9 | `half2float` | Return value is produced by **type punning through a `union`** (`out.num` written, `out.flt` read). Inputs whose result is a signalling/quiet NaN (`n = 31` or `63` with `h & 0x3ff != 0`) must be returned with the **payload and sign bits preserved** — a `float`-comparison-based implementation would silently pass while corrupting them. Compared as raw `u32` bits, never with `==` on floats. | no error; exact bit pattern, NaN payload preserved | [x] |
| E10 | `half2float` | Repeated / interleaved calls (the C tables are non-`const` `static` mutable objects in `.data`). No call may mutate observable state. | no error; the function is pure — the same `h` yields the same bits on every call, in any order | [x] |

**Rows: 10. Unchecked rows: 0.**
