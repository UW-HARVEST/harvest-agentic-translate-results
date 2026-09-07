# ERRORS.md — Phase C error-surface table

## Mechanical extraction

Grep of `c_src/src/lib.c` for every rejection / non-zero-return statement:

```
$ grep -n 'return\|assert\|if (' c_src/src/lib.c
7:    if (!dst || numElem == 0)
8:        return 22;
9:    if (!src) {
11:        return 22;
12:    }
13:    while (ptr < dst + numElem && *ptr != 0)
15:    while (ptr < dst + numElem) {
16:        if ((*ptr++ = *src++) == 0)
17:            return 0;
20:    return 34;
```

There are **no** `assert`s, **no** error enums, **no** `RETURN_ERROR` macros,
**no** `return NULL`, and **no** named min/max constants in the C source. The
complete rejection surface is:

- `return 22` at line 8 — reached by **two independent** trigger conditions
  (`!dst`, `numElem == 0`) which are separate rows because they are distinct
  invalid inputs with distinct side-effect profiles.
- `return 22` at line 11 — `!src`, and it has the side effect `dst[0] = 0`.
- `return 34` at line 20 — the fall-through when the second `while` loop
  exhausts the `numElem` window without copying a NUL. Reached by **two
  distinct** input shapes (no NUL anywhere in the `dst` window, versus a NUL in
  `dst` but insufficient remaining room for `src`), which are separate rows
  because the resulting buffer contents differ.

`22` is `EINVAL` and `34` is `ERANGE` on Linux/glibc. The only non-error return
is `0`.

**Critical side effect that must match:** on every `return 34` path and on the
`!src` `return 22` path, the C writes `dst[0] = 0`. On the `return 34`
insufficient-room path the C has *already* copied part of `src` into `dst`
before clobbering `dst[0]`. So an error-path test must compare the **entire
destination buffer**, not just the return code.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `wcscat` | `dst == NULL`, `numElem > 0`, `src` valid non-NULL | returns `22`; no memory written (cannot be, `dst` is NULL) | [x] |
| E2 | `wcscat` | `dst == NULL`, `numElem == 0`, `src` valid | returns `22`; short-circuit `||` means `dst` is never dereferenced | [x] |
| E3 | `wcscat` | `dst == NULL`, `numElem > 0`, `src == NULL` | returns `22` from line 8 (the `!dst` check precedes the `!src` check), so `dst[0] = 0` is **not** executed | [x] |
| E4 | `wcscat` | `dst` valid, `numElem == 0`, `src` valid non-NULL | returns `22`; `dst` buffer left **completely unmodified** (no `dst[0] = 0`, because this returns from line 8, not line 11) | [x] |
| E5 | `wcscat` | `dst` valid, `numElem == 0`, `src == NULL` | returns `22` from line 8; `dst` **unmodified** — the `!src` branch is never reached | [x] |
| E6 | `wcscat` | `dst` valid, `numElem > 0`, `src == NULL` | sets `dst[0] = 0`, returns `22`; all of `dst[1..]` unmodified | [x] |
| E7 | `wcscat` | `dst` valid, `numElem > 0`, `src` valid, but `dst[0..numElem]` contains **no** NUL (unterminated / already-full destination) | first loop runs to `ptr == dst+numElem`, second loop body never executes, then `dst[0] = 0`; returns `34`. `dst[1..numElem]` keeps its original bytes | [x] |
| E8 | `wcscat` | `dst` valid, `numElem == 1`, `dst[0] != 0` (degenerate case of E7) | returns `34`; `dst[0]` becomes `0` | [x] |
| E9 | `wcscat` | `dst` valid, `numElem > 0`, `src` valid, NUL found in `dst` at index `k`, but `strlen(src) > numElem - k` (src does not fit, not even its terminator) | copies `src[0 .. numElem-k]` into `dst[k .. numElem]`, then sets `dst[0] = 0`; returns `34`. The partial copy IS observable | [x] |
| E10 | `wcscat` | `dst` valid, `numElem == 1`, `dst[0] == 0`, `src` non-empty (degenerate case of E9) | writes `src[0]` to `dst[0]`, window exhausts, then `dst[0] = 0`; returns `34` | [x] |
| E11 | `wcscat` | `dst` valid, `numElem` == exact boundary: NUL at index `k`, `strlen(src) == numElem - k` (one element too long — the terminator has no room) | returns `34` (one step past the largest accepted length, which is `numElem - k - 1`) | [x] |

## Generic FFI boundary cases (required even though not in the C table)

| # | case | expected C result | status |
|---|------|-------------------|--------|
| G1 | both `dst == NULL` and `src == NULL`, `numElem == 0` | `22`, no dereference of either pointer | [x] |
| G2 | `numElem == 1` exhaustively crossed with `dst[0] ∈ {0, nonzero}` and `src` empty / non-empty | `0` only for `dst[0]==0 && src[0]==0`; `34` otherwise | [x] |
| G3 | oversized `numElem` (e.g. `1 << 40`) with an early NUL in a small real `dst` buffer, and `src` short enough that the copy stops long before the nominal window end | `0`; no out-of-bounds access occurs because the NUL terminates the copy first | [x] |
| G4 | `numElem == usize::MAX` — `dst + numElem` wraps the pointer (UB in C; recorded to confirm the compiled C and the Rust `wrapping_add` agree) | observed C behaviour is the ground truth; both loops are skipped because `end < ptr`, so `dst[0] = 0` and `34` | [x] |
| G5 | out-of-range "enum" values across the FFI boundary | **N/A** — the API has no enum parameter. The only non-pointer parameter is `size_t numElem`, whose full domain is covered by rows E4, E8, E10, E11, G3 and G4 | [x] |
| G6 | negative / high-bit-set `wchar_t` values in `src` and `dst` (`wchar_t` is **signed** i32 on this target, so `*ptr != 0` must treat `-1` as non-NUL, and only exact `0` terminates) | identical to any other non-zero value; only `0` terminates | [x] |

## Final status

All 11 error rows (E1–E11) and all 6 generic boundary rows (G1–G6) have a
passing differential test in `tests/phase_c_errors.rs` (17 tests). Each asserts
the **exact** error code (22 / 34 / 0) rather than "both failed somehow", and
also asserts the resulting destination buffer byte-for-byte, because several
error paths have observable side effects (`dst[0] = 0`, and on E9/E11 a partial
copy performed *before* `dst[0]` is clobbered).

Phase C: **PASS** — 17/17 tests, 0 unchecked rows.
