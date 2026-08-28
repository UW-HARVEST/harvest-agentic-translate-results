# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` (the whole file, 21 lines). Every
statement in the C that rejects/errors is listed below with the line it lives on.

## Mechanical inventory of the C's rejection machinery

```
$ grep -n 'return\|assert\|== 0\|!dst\|!src\|<' c_src/src/lib.c
7:    if (!dst || numElem == 0)
8:        return 22;
9:    if (!src) {
10:       dst[0] = 0;
11:       return 22;
13:   while (ptr < dst + numElem && *ptr != 0)
15:   while (ptr < dst + numElem) {
16:       if ((*ptr++ = *src++) == 0)
17:           return 0;
19:   dst[0] = 0;
20:   return 34;
```

* error-return statements: **3** (`return 22` at L8, `return 22` at L11, `return 34` at L20 → three `return` sites, two distinct codes)
* `assert`: **0** (none in the file)
* explicit null checks: **2** (`!dst` L7, `!src` L9)
* explicit range checks: **1 scalar** (`numElem == 0`, L7) + **2 pointer bound checks** (`ptr < dst + numElem`, L13 and L15)
* min/max constants / magic numbers: `22` (EINVAL), `34` (ERANGE), `0` (NUL terminator, and the `numElem == 0` bound). No other constants exist.
* enums: **none** in the API — the only "enum-like" input is the `int` return, which is an output. See the "no-enum" note at the bottom for how the equivalent
  out-of-domain-scalar class is covered instead.

## Error-surface table

Legend for "expected C result": `ret` = returned `int`; "dst untouched" means the
caller's buffer is bit-identical to before the call; `dst[0]=0` means **only**
element 0 is overwritten with 0 and everything else is untouched.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `wcscat` | L7 `!dst`: `dst == NULL`, `numElem > 0`, `src` valid non-NULL | `ret 22`; nothing written anywhere | [x] `err01_dst_null` |
| 2 | `wcscat` | L7 `numElem == 0`: `dst` valid non-NULL, `numElem == 0`, `src` valid non-NULL | `ret 22`; **dst untouched** (note: `dst[0]` is *not* zeroed on this path) | [x] `err02_numelem_zero` |
| 3 | `wcscat` | L7 both disjuncts true: `dst == NULL` **and** `numElem == 0` | `ret 22`; nothing written | [x] `err03_dst_null_and_numelem_zero` |
| 4 | `wcscat` | L7 short-circuit precedence: `dst == NULL` **and** `src == NULL` (the `!dst` test runs before the `!src` test, so L10 `dst[0]=0` must NOT be reached / must not fault) | `ret 22`; nothing written, no deref of NULL | [x] `err04_dst_null_and_src_null` |
| 5 | `wcscat` | L7 short-circuit precedence: `numElem == 0` **and** `src == NULL` (`numElem==0` wins, so `dst[0]` is *not* zeroed) | `ret 22`; **dst untouched** | [x] `err05_numelem_zero_and_src_null` |
| 6 | `wcscat` | L9 `!src`: `src == NULL`, `dst` valid, `numElem >= 1` | `ret 22` **and** `dst[0] = 0`; elements `1..` untouched | [x] `err06_src_null_writes_dst0` |
| 7 | `wcscat` | L9 `!src` with the minimum legal window, `numElem == 1` | `ret 22`, `dst[0] = 0` | [x] `err07_src_null_numelem_one` |
| 8 | `wcscat` | L9 `!src` when `dst` was already unterminated/full — the null-`src` check precedes the scan loop, so no scan happens | `ret 22`, `dst[0] = 0`, rest untouched | [x] `err08_src_null_unterminated_dst` |
| 9 | `wcscat` | L13/L20 unterminated `dst`: no `0` element anywhere in `dst[0 .. numElem)`, so the scan loop exhausts the window and the copy loop never runs | `ret 34` **and** `dst[0] = 0`; `dst[1 .. ]` untouched (**src is never read at all**) | [x] `err09_unterminated_dst` |
| 10 | `wcscat` | L13/L20 unterminated `dst` **and** `src` is the empty string (`src[0]==0`) — still an error because the window is full | `ret 34`, `dst[0] = 0`, rest untouched | [x] `err10_unterminated_dst_empty_src` |
| 11 | `wcscat` | L15/L20 truncation: `dst` terminated at index `k`, `strlen(src) == L`, `k + L >= numElem` (no room for the NUL). The copy loop fills `dst[k .. numElem)` with `src[0 .. numElem-k)` and *then* L19 sets `dst[0] = 0` | `ret 34`; `dst[k..numElem)` = first `numElem-k` chars of `src`, then `dst[0]` forced to 0 (clobbering whatever the loop or the original prefix put there) | [x] `err11_truncate_partial_copy` |
| 12 | `wcscat` | L15/L20 exact off-by-one: `k + L == numElem` exactly (the payload fits but the terminator does not) | `ret 34` (not 0), with the same clobbering as row 11 | [x] `err12_truncate_off_by_one` |
| 13 | `wcscat` | L15/L20 single-slot truncation: `numElem == 1`, `dst[0] == 0`, `src[0] != 0`. `dst[0]` is written with `src[0]`, `ptr` hits the end, then L19 rewrites `dst[0] = 0` | `ret 34`, net effect `dst[0] == 0` | [x] `err13_truncate_numelem_one` |
| 14 | `wcscat` | L15/L20 truncation where `k == numElem - 1` (exactly one free slot) and `src[0] != 0` | `ret 34`, `dst[0] = 0` (and `dst[numElem-1] == src[0]` unless `numElem == 1`) | [x] `err14_truncate_one_slot_left` |
| 15 | `wcscat` | L13/L15 `dst + numElem` pointer overflow: `numElem == SIZE_MAX`. `dst + SIZE_MAX` wraps to `dst - 1` element, so `ptr < end` is false immediately and both loops are skipped | `ret 34`, `dst[0] = 0`, rest untouched (verified against the real C `.so`) | [x] `err15_numelem_size_max` |
| 16 | `wcscat` | L13/L15 `dst + numElem` pointer overflow, other wrap witnesses: `numElem == SIZE_MAX/4`, `SIZE_MAX/2`, `1<<62`, `(SIZE_MAX>>2)+1` — each scaled by `sizeof(wchar_t)==4` lands at or before `dst` | `ret 34`, `dst[0] = 0`, rest untouched | [x] `err16_numelem_wrap_witnesses` |
| 17 | `wcscat` | Terminator hidden *outside* the window: `dst` has a `0` at index `>= numElem` but none inside `[0, numElem)` — a "valid C string" that is still rejected because the window is full | `ret 34`, `dst[0] = 0`, element at the out-of-window `0` untouched | [x] `err17_nul_outside_window` |

## Generic FFI-boundary boundaries also covered (beyond the table)

| # | condition | expected | test |
|---|-----------|----------|------|
| G1 | both pointers NULL, `numElem` large | `ret 22`, no faults | [x] `err04_dst_null_and_src_null` |
| G2 | `dst == NULL`, `src == NULL`, `numElem == 0` | `ret 22` | [x] `err03_dst_null_and_numelem_zero`, `err04_dst_null_and_src_null` |
| G3 | `numElem == 1` (minimum non-rejected length) across all `dst`/`src` shapes | see rows 7/13 | [x] `err07_src_null_numelem_one`, `err13_truncate_numelem_one`, `boundary_numelem_one_matrix` |
| G4 | one step past the "fits" boundary: `k + L == numElem - 1` (fits, `ret 0`) vs `== numElem` (`ret 34`) vs `== numElem + 1` (`ret 34`) | 0 / 34 / 34 | [x] `boundary_fit_off_by_one_sweep` |
| G5 | oversized `numElem` that does **not** overflow (`1<<40`) with an early-terminated `dst` and a fitting `src` | `ret 0`, normal append (verified against the C `.so`) | [x] `oversized_numelem_no_overflow` |
| G6 | extreme `wchar_t` payload values passed across FFI: `i32::MIN`, `-1`, `i32::MAX`, `0x110000`, `0xD800` (lone surrogate), `0x41424344` — none are `0`, so none may terminate | treated as ordinary non-NUL characters, copied verbatim | [x] `extreme_wchar_values` |
| G7 | **No enums exist in this API.** The equivalent "out-of-domain scalar crosses the FFI boundary" class is `size_t numElem` values with no sensible meaning (0, `SIZE_MAX`, wrap witnesses) and `wchar_t` values outside Unicode (G6). Both are covered above; there is no `int`-typed enum parameter to feed an invalid variant to. | — | [x] `err02_numelem_zero`, `err15_numelem_size_max`, `err16_numelem_wrap_witnesses`, `extreme_wchar_values` |

## Non-error return (for completeness of the return-code domain)

| # | condition | expected |
|---|-----------|----------|
| N1 | L16/L17: a `0` from `src` is copied while `ptr < dst + numElem` | `ret 0`, `dst` is prefix + `src` + NUL, tail beyond the NUL untouched |

The complete return-code domain of the C function is therefore exactly
`{0, 22, 34}` — asserted in `return_code_domain_is_closed`.
