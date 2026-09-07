# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection/error return in `c_src/src/lib.c`.
The whole file is reproduced here for reference (it is the complete error
surface — there are no asserts, no error enums, no other files):

```c
int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src) {
    wchar_t *ptr = dst;
    if (!dst || numElem == 0)          /* -> return 22          */
        return 22;
    if (!src) {                        /* -> dst[0]=0; return 22 */
        dst[0] = 0;
        return 22;
    }
    while (ptr < dst + numElem && *ptr != 0)
        ptr++;
    while (ptr < dst + numElem) {
        if ((*ptr++ = *src++) == 0)
            return 0;
    }
    dst[0] = 0;                        /* -> return 34          */
    return 34;
}
```

Distinct rejection statements in the C source: three (`return 22` at the
argument check, `return 22` after zeroing on the null-`src` path, `return 34`
after zeroing on the no-room path). Each is reachable through several distinct
triggers; one row per distinct trigger below.

| #  | function | trigger (the exact invalid input/condition) | expected C result | ok |
|----|----------|---------------------------------------------|-------------------|----|
| 1  | `wcscat` | `dst == NULL`, `numElem > 0`, `src` valid | returns `22`; nothing written anywhere (short-circuit `!dst` before `dst[0]=0`) | [x] |
| 2  | `wcscat` | `dst == NULL`, `numElem == 0`, `src` valid | returns `22`; no write | [x] |
| 3  | `wcscat` | `dst == NULL`, `src == NULL`, any `numElem` | returns `22`; no write (the `!dst` test wins over the `!src` test) | [x] |
| 4  | `wcscat` | `dst` valid, `numElem == 0`, `src` valid | returns `22`; **`dst` left untouched** — `dst[0]` is *not* zeroed on this path | [x] |
| 5  | `wcscat` | `dst` valid, `numElem == 0`, `src == NULL` | returns `22`; `dst` untouched (`numElem==0` checked before `!src`) | [x] |
| 6  | `wcscat` | `dst` valid, `numElem > 0`, `src == NULL` | returns `22` **and writes `dst[0] = 0`** (destroys the existing contents; the rest of the buffer is left as-is) | [x] |
| 7  | `wcscat` | `dst` valid, `numElem == 1`, `src == NULL` | returns `22`, `dst[0] = 0` (boundary of row 6) | [x] |
| 8  | `wcscat` | no room for the terminator: `wcslen(dst) + wcslen(src) + 1 > numElem` (`dst` NUL-terminated inside the buffer) | returns `34`; buffer is filled with as many `src` elements as fit, **then `dst[0]` is overwritten with `0`** (the partially-copied tail elements stay in the buffer) | [x] |
| 9  | `wcscat` | exactly one element too long: `wcslen(dst) + wcslen(src) + 1 == numElem + 1` | returns `34` (one step past the valid range of row 4/5 in `CONFIGS.md`) | [x] |
| 10 | `wcscat` | `dst` is **not** NUL-terminated within `numElem` elements (seek loop stops at `dst+numElem`, so the copy loop body never runs) | returns `34`; **`src` is never read at all**; only `dst[0] = 0` is written, `dst[1..numElem]` unchanged | [x] |
| 11 | `wcscat` | `dst` unterminated **and** `numElem == 1` (`dst[0] != 0`) | returns `34`, `dst[0] = 0` | [x] |
| 12 | `wcscat` | `numElem == 1`, `dst[0] == 0`, `src` non-empty | returns `34`: one element of `src` is written to `dst[0]`, then the loop ends and `dst[0]` is reset to `0` | [x] |
| 13 | `wcscat` | oversized `numElem` (much larger than the real allocation) but `dst`/`src` short enough that the copy terminates before the real buffer end | returns `0` — the bogus length is never noticed (no length validation beyond `numElem == 0`) | [x] |
| 14 | `wcscat` | "out-of-range enum"/garbage scalar analogue: `wchar_t` elements that are not valid Unicode — negative values, `INT_MIN`, `INT_MAX`, surrogates, `> 0x10FFFF` | no validation whatsoever; treated as ordinary non-zero elements and copied verbatim (only `== 0` is special) | [x] |

Notes on generic boundaries required by Phase C:

* null pointers — rows 1, 2, 3, 5, 6, 7 (both parameters, in both orders).
* zero length — rows 2, 4, 5.
* oversized length — row 13.
* one step past a valid range — row 9 (and row 12, `numElem == 1`).
* out-of-range enum values across the FFI boundary — the API takes no enum; the
  closest real analogue is an arbitrary `int`-valued `wchar_t` with no valid
  Unicode meaning, covered by row 14. The `int` return value is likewise
  compared as a raw `c_int`, not mapped onto a Rust enum.
