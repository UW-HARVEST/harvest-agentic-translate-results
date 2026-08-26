# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` (the whole library). The complete
C source is 21 lines:

```c
int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src) {
    wchar_t *ptr = dst;
    if (!dst || numElem == 0)          /* R1 / R2 */
        return 22;
    if (!src) {                        /* R4 */
        dst[0] = 0;
        return 22;
    }
    while (ptr < dst + numElem && *ptr != 0)
        ptr++;
    while (ptr < dst + numElem) {
        if ((*ptr++ = *src++) == 0)
            return 0;                  /* success */
    }
    dst[0] = 0;                        /* R5 / R6 */
    return 34;
}
```

Mechanical inventory of every rejection construct in the file:

* `return 22;` — appears **twice** (line 8 → guarded by `!dst || numElem == 0`,
  which is two distinct triggers; line 11 → guarded by `!src`).
* `return 34;` — appears **once** (line 20), reachable by two distinct
  conditions: truncation while copying, and "no NUL found in `dst[0..numElem]`"
  (second loop never runs).
* `assert` / `abort` / `errno` / error enums / `return NULL` / min-max
  constants: **none present**.
* Magic constants: `22` (= `EINVAL`), `34` (= `ERANGE`), `0` (success).
* The only side effect on the error paths is the store `dst[0] = 0`.

## Table

| #   | function | trigger (the exact invalid input/condition)                                                                                                     | expected C result                                                                                   | test | [x] |
|-----|----------|-------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|------|-----|
| E1  | `wcscat` | `dst == NULL`, `numElem != 0`, `src` valid                                                                                                       | returns `22`; no memory written                                                                      | `e1_null_dst` | [x] |
| E2  | `wcscat` | `dst != NULL`, `numElem == 0`, `src` valid                                                                                                       | returns `22`; `dst` **not** written (short-circuit before `dst[0]=0`)                                | `e2_zero_numelem` | [x] |
| E3  | `wcscat` | `dst == NULL` **and** `numElem == 0` (both first-branch sub-conditions true)                                                                      | returns `22`; nothing written                                                                        | `e3_null_dst_and_zero_numelem` | [x] |
| E4  | `wcscat` | `dst == NULL`, `numElem == 0`, `src == NULL` (all three invalid; first branch must win over the `!src` branch, i.e. **no** NULL deref)            | returns `22`; nothing written                                                                        | `e4_all_null_zero` | [x] |
| E5  | `wcscat` | `dst != NULL`, `numElem != 0`, `src == NULL`                                                                                                      | stores `dst[0] = 0`, returns `22`; rest of `dst` untouched                                           | `e5_null_src_writes_dst0` | [x] |
| E6  | `wcscat` | `src == NULL` with `numElem == 1` (smallest buffer on the `!src` path)                                                                            | stores `dst[0] = 0`, returns `22`                                                                    | `e6_null_src_numelem_1` | [x] |
| E7  | `wcscat` | truncation: `dst` has a NUL at index `k < numElem` but `k + wcslen(src) + 1 > numElem` — the copy loop hits `end` before writing the terminator   | partial copy fills `dst[k..numElem]` with `src[0..numElem-k]`, then `dst[0] = 0`, returns `34`       | `e7_truncation_partial_copy` | [x] |
| E8  | `wcscat` | truncation by exactly one element (`k + wcslen(src) + 1 == numElem + 1`) — the tightest failing fit                                              | as E7 (`dst[0]=0`, returns `34`), full buffer image must match byte-for-byte                         | `e8_truncation_off_by_one` | [x] |
| E9  | `wcscat` | no NUL in `dst[0..numElem]` (buffer entirely non-zero): first scan loop consumes the whole window, second loop body never executes                | `src` is **not** read at all; `dst[0] = 0`, returns `34`; `dst[1..numElem]` untouched                | `e9_unterminated_dst` | [x] |
| E10 | `wcscat` | `numElem == 1` with `dst[0] != 0` (degenerate case of E9)                                                                                        | `dst[0] = 0`, returns `34`                                                                           | `e10_numelem_1_unterminated` | [x] |
| E11 | `wcscat` | `numElem == 1` with `dst[0] == 0` and `src[0] != 0`: one element of room, needed 2 → writes `dst[0]=src[0]`, loop ends, then `dst[0]=0`           | `dst[0] = 0`, returns `34` (the intermediate `src[0]` store is overwritten)                          | `e11_numelem_1_no_room` | [x] |
| E12 | `wcscat` | NUL terminator of `dst` lies **outside** the `numElem` window (window shorter than the real string) — same shape as E9 but buffer is longer       | `dst[0] = 0`, returns `34`; only `dst[0]` modified                                                   | `e12_terminator_outside_window` | [x] |
| E13 | `wcscat` | `numElem` one past the last element that fits (`k == numElem - 1`, `src` non-empty): room for the terminator only                                 | writes `dst[numElem-1] = src[0]`, then `dst[0] = 0`, returns `34`                                    | `e13_room_for_terminator_only` | [x] |
| E14 | `wcscat` | `numElem == SIZE_MAX`: `dst + numElem` wraps (`numElem * 4 mod 2^64 == 2^64 - 4`) so `end == dst - 1` and `end < dst`; both loops skipped         | `dst[0] = 0`, returns `34`                                                                           | `e14_numelem_size_max` | [x] |
| E15 | `wcscat` | `numElem == 2^62`: `numElem * sizeof(wchar_t)` wraps to exactly `0`, so `end == dst`; both loops skipped                                          | `dst[0] = 0`, returns `34`                                                                           | `e15_numelem_wraps_to_zero` | [x] |
| E16 | `wcscat` | `numElem == 2^62 + 1`: wraps to `end == dst + 1`, i.e. behaves exactly like `numElem == 1` (one step past the wrap boundary)                      | identical to the `numElem == 1` cases (E10 / E11 / success when `src` is empty)                      | `e16_numelem_wraps_to_one` | [x] |
| E17 | `wcscat` | `numElem == SIZE_MAX` combined with `src == NULL` (wrap value must still take the `!src` branch, since that check precedes the arithmetic)         | `dst[0] = 0`, returns `22` (**not** `34`)                                                            | `e17_size_max_null_src` | [x] |
| E18 | `wcscat` | `numElem == 0` combined with `dst[0] != 0` (must not clear the buffer)                                                                            | returns `22`, buffer completely unmodified                                                           | `e18_zero_numelem_no_write` | [x] |

## Generic FFI boundary cases (required by Phase C even though not in the table)

| #   | condition                                                                                                | expected                                                        | test | [x] |
|-----|----------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------|------|-----|
| G1  | null pointers in every combination of `(dst, src)` × `numElem ∈ {0, 1, 8}`                               | C and Rust agree on return code and on whether `dst[0]` cleared | `g1_null_pointer_matrix` | [x] |
| G2  | `numElem == 0` (zero length) and oversized lengths `{SIZE_MAX, SIZE_MAX-1, 2^62, 2^62+1, 2^63}`          | identical return codes / buffer images                          | `g2_length_boundaries` | [x] |
| G3  | one step past the valid element range: NUL at `numElem-1`, `numElem`, `numElem+1`                        | identical return codes / buffer images                          | `g3_one_past_range` | [x] |
| G4  | out-of-range "enum"-style integer values crossing FFI. `wcscat` has no enum parameter, so the analogous  | identical return codes / buffer images                          | `g4_out_of_domain_wchar_values` | [x] |
|     | class is `wchar_t` values with no valid character meaning: `-1`, `INT_MIN`, `INT_MAX`, `0x110000`,        |                                                                 |      |     |
|     | `0xD800` (lone surrogate), `0x80000000u as i32`. The C compares only against `0`, so all must round-trip |                                                                 |      |     |
| G5  | `numElem` larger than the real allocation but with a NUL inside the real part (C stops at the NUL, so no | identical return codes / buffer images                          | `g5_numelem_beyond_allocation_but_terminated` | [x] |
|     | out-of-bounds access actually happens)                                                                   |                                                                 |      |     |

`22` is `EINVAL` and `34` is `ERANGE` on glibc, matching the
`_wcscat_s`-style contract this function mimics; the tests assert the exact
integers, not merely "non-zero".

## How to run

```
./run_all_features.sh                        # everything
cargo test --offline --test error_paths      # Phase C only
```

Status: **18/18 `ERRORS.md` rows + 5/5 generic FFI boundary rows pass**
(23 tests) for both feature combinations and for both the debug and the release
Rust `.so`.

Every test asserts the *exact* return code (`22` / `34` / `0`) and the *exact*
resulting memory image of `dst` (and, where applicable, that `src` was not
written and that memory outside the `numElem` window was not touched) — never
merely "both failed somehow".
