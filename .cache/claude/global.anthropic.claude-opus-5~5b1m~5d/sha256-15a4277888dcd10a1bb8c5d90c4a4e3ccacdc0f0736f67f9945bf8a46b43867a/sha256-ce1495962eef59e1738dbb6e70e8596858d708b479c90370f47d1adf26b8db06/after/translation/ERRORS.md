# ERRORS.md — Phase A error-surface table

## How this was derived

`c_src` was grepped mechanically for every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|ERROR|error|-1|if *\(|switch|\?|#if|exit|abort|goto' -r src include
#   -> (NO MATCHES)
grep -nE '\b(if|else|for|while|switch|case|do)\b' -r src include
#   -> src/lib.c:8:    for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t)) {
```

**There are zero explicit error paths.** `premultiply` returns `void`, has no
`assert`, no null check, no range check, no error enum, no sentinel, and no
`#ifdef`. Its only control flow is the single `for` loop on line 8.

Therefore an "error" in this API can manifest in exactly two observable ways,
and the table below enumerates every distinct trigger for each:

* **`NO-OP`** — the loop bound evaluates `<= 0`, so the body runs zero times,
  `img->pix` is *never dereferenced*, and the pixel buffer is left byte-identical.
  This is the C code's de-facto input rejection.
* **`SIGSEGV`** — a null/invalid pointer is dereferenced. Undefined behaviour in
  C; both libraries must fault identically.

### The controlling arithmetic (line 6 + line 8)

```c
int stride = w * sizeof(cp_pixel_t);        // (A)
for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
```

* (A) `sizeof` is `size_t`, so `w` is **sign-extended to 64 bits**, multiplied by
  4, then **truncated back to `int`** → `stride = wrap32(w * 4)`.
* The bound `(int)stride * h` is a **32-bit `int` multiply** that wraps →
  `limit = wrap32(stride * h)`.
* `limit` is always a multiple of 4, so the iteration count is
  `iters = if limit > 0 { limit / 4 } else { 0 }`.

Note this bound counts **bytes** (`stride` bytes per row × `h` rows) and is
stepped 4 bytes at a time, so for ordinary positive dimensions it visits exactly
`w * h` pixels.

### Not applicable

* **Out-of-range enum values across FFI** — the API declares *no* enums
  (`lib.h` has only `cp_pixel_t` and `cp_image_t`, both plain structs of
  `uint8_t`/`int`). The closest analogue is an out-of-range *dimension*, which
  rows 4–18 cover exhaustively, including `INT_MIN`/`INT_MAX`.
* **Error codes / sentinels** — the function is `void`; there is no return value
  to compare. Equivalence is asserted on the full post-call byte image of the
  buffer plus surrounding canaries, and on fault signal for the crash rows.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `premultiply` | `img == NULL` | deref of `img->w` faults → `SIGSEGV` | [x] |
| 2 | `premultiply` | `img->pix == NULL`, `w > 0`, `h > 0` (`limit > 0`) | deref of `data[0]` faults → `SIGSEGV` | [x] |
| 3 | `premultiply` | `img->pix` = wild non-null unmapped address, `w,h > 0` | faults → `SIGSEGV` | [x] |
| 4 | `premultiply` | `w == 0`, `h > 0` (zero length) | `stride=0` → `limit=0` → `NO-OP` | [x] |
| 5 | `premultiply` | `h == 0`, `w > 0` (zero length) | `limit=0` → `NO-OP` | [x] |
| 6 | `premultiply` | `w == 0 && h == 0` | `limit=0` → `NO-OP` | [x] |
| 7 | `premultiply` | `w == 0` or `h == 0` **with `pix == NULL`** | `NO-OP`, `pix` never dereferenced → must **not** fault | [x] |
| 8 | `premultiply` | `w < 0`, `h > 0` | `limit < 0` → `NO-OP` | [x] |
| 9 | `premultiply` | `w > 0`, `h < 0` | `limit < 0` → `NO-OP` | [x] |
| 10 | `premultiply` | `w < 0` **and** `h < 0` | `limit > 0` → loop **RUNS** for `|w*h|` pixels (double-negative accepted) | [x] |
| 11 | `premultiply` | `w ≡ 0 (mod 2^30)`, `w != 0`: `w ∈ {2^30, -2^30, INT_MIN}` | `stride` wraps to `0` → `limit=0` → `NO-OP` for *every* `h` | [x] |
| 12 | `premultiply` | `w == ±2^29`, `h` **odd** | `stride` wraps to `INT_MIN`; `limit = INT_MIN < 0` → `NO-OP` | [x] |
| 13 | `premultiply` | `w == ±2^29`, `h` **even** | `stride = INT_MIN`; `limit` wraps to `0` → `NO-OP` | [x] |
| 14 | `premultiply` | `w == 2^29 + 1`, `h == 2` | `stride = INT_MIN+4`; `limit` wraps **positive** to `8` → **2 pixels processed** | [x] |
| 15 | `premultiply` | `w == INT_MAX`, `h == 1` | `stride` wraps to `-4`; `limit = -4` → `NO-OP` | [x] |
| 16 | `premultiply` | `w == INT_MAX`, `h == -1` | `stride = -4`; `limit = 4` → **1 pixel processed** | [x] |
| 17 | `premultiply` | `w == INT_MAX`, `h == INT_MAX` (both oversized) | `stride = -4`, `limit = 4` → **1 pixel processed** | [x] |
| 18 | `premultiply` | `w == 1`, `h == INT_MIN` | `limit` wraps to `0` → `NO-OP` | [x] |
| 19 | `premultiply` | `w == 1`, `h == INT_MAX` (one past max row count) | `limit` wraps to `-4` → `NO-OP` | [x] |
| 20 | `premultiply` | `w == INT_MIN`, `h == INT_MIN` | `stride = 0`, `limit = 0` → `NO-OP` | [x] |
| 21 | `premultiply` | `w == 268435456` (`2^28`), `h == 4` | `stride = 2^30`; `limit` wraps to `0` → `NO-OP` | [x] |
| 22 | `premultiply` | `w == 2^28 + 1`, `h == 4` | `limit` wraps positive to `16` → **4 pixels processed** | [x] |
| 23 | `premultiply` | `img` non-null but **misaligned** `cp_image_t*` (odd address) | x86-64 tolerates unaligned `int` loads → behaves as aligned | [x] |
| 24 | `premultiply` | `img->pix` **misaligned** (offset 1/2/3 from 4-byte boundary) | accessed via `uint8_t*`, so alignment is irrelevant → normal processing | [x] |
| 25 | `premultiply` | write-extent check: `limit > 0` | bytes `>= limit` and the byte **before** `pix` are never written; **the alpha byte `data[i+3]` is never written** (only `+0/+1/+2` are stored) | [x] |

Rows 1–3 are verified in a forked child process, comparing the *fault signal*
raised by the C `.so` against that raised by the Rust `.so`.
Rows 4–25 are verified in-process by differential byte comparison of the
buffer, its guard canaries, and (where relevant) the exact count of mutated
pixels.
