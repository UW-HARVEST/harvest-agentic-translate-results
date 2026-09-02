# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return` statement in the C
source was enumerated with `grep -n 'return' src/lib.c`:

```
src/lib.c:8:        return 22;      <- if (!dst || numElem == 0)
src/lib.c:11:        return 22;     <- if (!src)  { dst[0] = 0; ... }
src/lib.c:17:            return 0;  <- success (copy loop wrote src's NUL)
src/lib.c:20:    return 34;         <- fell out of the bounded copy loop
```

There are **no** `assert`s, **no** error enums, **no** `errno` writes, **no**
`#if`/`#ifdef` branches, and **no** min/max constants in the C source (verified
by grep for `assert|RETURN_ERROR|NULL|errno|enum|MAX|MIN|#if` — zero matches).
The only rejection mechanism is the hard-coded `int` return value: `22`
(`EINVAL`) and `34` (`ERANGE`). Note in particular that the C code applies **no
upper bound** to `numElem`, so an absurdly large `numElem` is *not* rejected —
rows 12–13 pin that non-rejection down, because "the C accepts it" is just as
much a behaviour the Rust must match.

The C's two `return 22` sites differ in their side effect on `dst`, and that
difference is load-bearing: line 8 returns **without writing**, line 11 returns
**after zeroing `dst[0]`**. Rows 4–5 vs row 6 separate those.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `wcscat` | `dst == NULL`, `numElem > 0`, `src` valid non-NULL | returns `22`; no memory written anywhere (`src` untouched) |
| 2  | `wcscat` | `dst == NULL`, `numElem == 0`, `src` valid non-NULL | returns `22` (short-circuit `!dst` fires first); no writes |
| 3  | `wcscat` | `dst == NULL`, `src == NULL`, `numElem > 0` | returns `22`; **no NULL dereference** — the `!dst` check precedes the `!src` branch that would do `dst[0] = 0` |
| 4  | `wcscat` | `numElem == 0`, `dst` valid non-NULL, `src` valid non-NULL | returns `22`; `dst` is **left completely unmodified** (`dst[0]` is *not* zeroed on this path) |
| 5  | `wcscat` | `numElem == 0`, `dst` valid non-NULL, `src == NULL` | returns `22`; `dst` left unmodified — the `numElem == 0` test is in the *same* `if` as `!dst` and therefore precedes the `!src` branch |
| 6  | `wcscat` | `src == NULL`, `dst` valid non-NULL, `numElem > 0` | returns `22` **and** writes `dst[0] = 0` (truncates `dst` to the empty string); elements `dst[1..numElem]` untouched |
| 7  | `wcscat` | `dst` contains **no** NUL in its first `numElem` elements (unterminated buffer) | seek loop runs to `ptr == dst + numElem`, so the copy loop body never executes: returns `34` and writes `dst[0] = 0`. `src` is never read at all. |
| 8  | `wcscat` | `src` is longer than the space remaining after the existing `dst` contents (`strlen(dst) + wcslen(src) + 1 > numElem`) | copy loop exhausts the bound: returns `34` and writes `dst[0] = 0`. `dst[1 .. numElem]` retains the partially-copied prefix of `src`. |
| 9  | `wcscat` | Off-by-one boundary: `strlen(dst) + wcslen(src) + 1 == numElem + 1`, i.e. the characters fit but the terminating NUL does not | returns `34`, `dst[0] = 0`; the last buffer element holds the final `src` character (partial copy is **not** rolled back) |
| 10 | `wcscat` | `numElem == 1` with `dst[0] == 0` and a non-empty `src` | returns `34`; `dst[0]` is set to `src[0]` by the copy loop and then re-zeroed by the trailing `dst[0] = 0` |
| 11 | `wcscat` | `numElem == 1` with `dst[0] != 0` (unterminated 1-element buffer) | returns `34`, `dst[0] = 0`; `src` never read (degenerate case of row 7) |
| 12 | `wcscat` | Oversized `numElem` far exceeding the real allocation, but with `dst` NUL-terminated early and a short `src` so no out-of-bounds access occurs | **NOT rejected** — returns `0` and appends normally. The C applies no upper-bound/sanity check on `numElem`. |
| 13 | `wcscat` | `numElem` larger than the true buffer *and* `dst` terminated at the very last real element | **NOT rejected** — returns `0`; the C happily writes past the caller's real allocation because it trusts `numElem`. Documented as non-rejection; the differential test uses a padded allocation so the write stays inside memory we own. |
| 14 | `wcscat` | `src == dst` (aliasing / self-append) with `dst[0] == 0` | **NOT rejected** — no overlap check exists. Deterministic outcome: the seek loop stops at index 0, the copy loop reads `dst[0] == 0`, writes it back and returns `0`. |
| 15 | `wcscat` | `src` points *into* `dst` (partial overlap), `dst` non-empty | **NOT rejected** — no overlap check. Result is whatever the bounded byte-by-byte loop produces; both implementations must agree element-for-element and on the return code. |

## Out-of-range enum values across the FFI boundary

The public API (`c_src/include/lib.h`) is:

```c
int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);
```

There is **no `enum` parameter anywhere in the API**, so there is no
"integer with no valid variant" to smuggle across the boundary for an enum. The
equivalent class of out-of-domain scalar input for *this* API is a `wchar_t`
value that is not a valid Unicode scalar — negative values (`wchar_t` is a
*signed* 32-bit `int` on Linux/glibc), surrogate-range values `0xD800..=0xDFFF`,
values above `0x10FFFF`, and `i32::MIN`/`i32::MAX`. The C treats `wchar_t`
purely as an integer and only ever compares it against `0`, so all such values
must round-trip byte-identically. These are covered by rows 16–19 below and by
the randomized value-class generator in Phase B.

| #  | function | trigger (out-of-domain scalar) | expected C result |
|----|----------|--------------------------------|-------------------|
| 16 | `wcscat` | `src` contains **negative** `wchar_t` values (e.g. `-1`, `i32::MIN`) | **NOT rejected** — copied verbatim; only `== 0` terminates |
| 17 | `wcscat` | `src` contains values above `0x10FFFF` and `i32::MAX` | **NOT rejected** — copied verbatim |
| 18 | `wcscat` | `src` contains UTF-16 surrogate values `0xD800..=0xDFFF` | **NOT rejected** — copied verbatim |
| 19 | `wcscat` | `dst` prefix contains negative / out-of-domain values before its NUL | **NOT rejected** — the seek loop compares against `0` only, so these count as ordinary characters |

## Checklist

| # | test | status |
|---|------|--------|
| 1  | `err_01_dst_null_valid_src`              | [x] passes on both |
| 2  | `err_02_dst_null_numelem_zero`           | [x] passes on both |
| 3  | `err_03_dst_null_src_null`               | [x] passes on both |
| 4  | `err_04_numelem_zero_valid_ptrs`         | [x] passes on both |
| 5  | `err_05_numelem_zero_src_null`           | [x] passes on both |
| 6  | `err_06_src_null_truncates_dst`          | [x] passes on both |
| 7  | `err_07_unterminated_dst`                | [x] passes on both |
| 8  | `err_08_src_too_long`                    | [x] passes on both |
| 9  | `err_09_off_by_one_no_room_for_nul`       | [x] passes on both |
| 10 | `err_10_numelem_one_nonempty_src`        | [x] passes on both |
| 11 | `err_11_numelem_one_unterminated`        | [x] passes on both |
| 12 | `err_12_oversized_numelem_not_rejected`  | [x] passes on both |
| 13 | `err_13_numelem_beyond_buffer_last_elem` | [x] passes on both |
| 14 | `err_14_src_aliases_dst_empty`           | [x] passes on both |
| 15 | `err_15_src_overlaps_dst_interior`       | [x] passes on both |
| 16 | `err_16_negative_wchar_values`           | [x] passes on both |
| 17 | `err_17_above_unicode_max`               | [x] passes on both |
| 18 | `err_18_surrogate_range_values`          | [x] passes on both |
| 19 | `err_19_dst_prefix_out_of_domain`        | [x] passes on both |

## Generic boundaries (required by Phase C beyond the table)

| test | covers |
|------|--------|
| `generic_null_pointer_matrix`             | every (dst, src) nullness combination × `numElem ∈ {0,1,2,3,16,2^20,SIZE_MAX-1,SIZE_MAX}` |
| `generic_zero_length`                     | `numElem == 0` with valid and NULL `src` |
| `generic_numelem_one_step_past_boundaries`| `numElem` at, one below and one above 19 power-of-two / type-max boundaries |
| `generic_minimal_src`                     | minimal valid `src` (`src[0] == 0`) against terminated and unterminated `dst` |
| `generic_numelem_address_space_wraparound`| `numElem` values where `dst + numElem` overflows the address space (see below) |
| `return_code_domain_is_closed`            | 5 000 randomized calls; the return value is always exactly one of {0, 22, 34} and both implementations agree |
| `exhaustive_rejection_space`              | complete enumeration of the rejection space over a 3-letter `dst` alphabet (1 800 tuples) |

## Divergence found and fixed

One real defect was found and fixed in the Rust (the C was never touched).

**`dst + numElem` overflows the address space.** For large `numElem` the C
expression `dst + numElem` (a `wchar_t *`, so a byte offset of `numElem * 4`)
wraps around 64 bits and lands *below* `dst`. Both `ptr < dst + numElem` tests
are then false immediately, so the seek loop and the copy loop are both skipped
and the function falls through to `dst[0] = 0; return 34;`. Measured against the
built C `.so`:

| `numElem` | byte offset | C result |
|-----------|-------------|----------|
| `SIZE_MAX`      | wraps to `-4` | `34`, `dst[0] = 0` |
| `SIZE_MAX - 1`  | wraps to `-8` | `34`, `dst[0] = 0` |
| `SIZE_MAX / 2`  | wraps to `-4` | `34`, `dst[0] = 0` |
| `2^62`          | wraps to `0`  | `34`, `dst[0] = 0` |
| `2^61`          | `2^63`, no wrap | `0`, appends normally |

The Rust originally used `dst.add(numElem)`, which is *undefined behaviour* on
overflow: it emits `getelementptr inbounds`, letting the optimiser assume the
wrap cannot happen, and it trips `ptr::add`'s debug precondition assertion. It
happened to agree with the C under the current codegen, but only by accident.
It is now `dst.wrapping_add(numElem)`, which performs exactly the wrapping byte
arithmetic the C compiler emits and is always safe to compute. The advancing
steps use `wrapping_add(1)` for the same reason. `generic_numelem_address_space_wraparound`
pins the behaviour down across 18 `numElem` values × all `wchar_t` value classes
× NULL/non-NULL `src` and `dst`.

Two further apparent divergences turned out to be **defects in the tests, not in
the translation**: the overlapping-`src` rows initially let the C's bounded copy
loop read past the end of the real allocation (the C then observed unrelated heap
bytes, which is genuinely non-deterministic), and one `err_09` assertion did not
account for `numElem == 1`, where the trailing `dst[0] = 0` overwrites the
character just copied into index 0. Both were fixed on the test side; the C's
behaviour was left exactly as written.
