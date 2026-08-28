# ERRORS.md — Error-surface table (Phase A, gate for Phase C)

Derived **mechanically** from `c_src/src/lib.c` and `c_src/include/lib.h`, not
from docs or assumptions.

## Mechanical grep of the whole C source

```sh
$ grep -nE 'return|assert|NULL|errno|ERROR|Error|error|exit\(|abort|goto|E[A-Z]+' \
      c_src/src/lib.c c_src/include/lib.h | grep -v '0x'
c_src/src/lib.c:19:    return crc16;

$ grep -nE '\b(if|else|switch|case|while|for|\?)\b' c_src/src/lib.c
6:    while (len >= 8) {
16:    while (len--) {

$ grep -nE '^\s*#' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:1:#include "lib.h"
c_src/include/lib.h:1:#include <stdint.h>
```

**Findings — this is the key fact about this library's error surface:**

* Exactly **one** `return` statement in the entire library: `return crc16;`
  (line 19). It is unconditional and returns a *value*, never a status.
* **Zero** `assert`s, **zero** `NULL` checks, **zero** `errno` writes, **zero**
  error enums / `RETURN_ERROR` macros / sentinel returns, **zero** `goto`s,
  **zero** `exit`/`abort` calls.
* **Zero** explicit range checks. The only comparison in the file is the loop
  condition `len >= 8`; the only other branch is the truthiness test in
  `while (len--)`.
* **Zero** min/max constants and **zero** `#ifdef`s.
* The return type is `tflac_u16`, whose full range `0x0000..=0xFFFF` is a valid
  CRC result — so **no return value can serve as an error sentinel**, by
  construction.

`crc16` is therefore a **total function** over its documented contract
(`d` points to `len` readable bytes): it rejects nothing and cannot fail. There
are no `RETURN_ERROR` branches to enumerate.

Because there are no explicit rejections, the rows below are the complete set of
**degenerate / boundary / implicit-contract conditions** the C code nonetheless
distinguishes or is subjected to — i.e. exactly the generic C-API boundaries
Phase C mandates (null pointers, zero and oversized lengths, one step past a
range, out-of-range enum values). Each row states the *actual* observable C
result, and each has a differential test asserting Rust produces the **same**
value, not merely "both failed somehow".

## Error / boundary table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `crc16` | `len == 0`, `d` = valid non-null pointer. `while (len >= 8)` is false; `while (len--)` tests `0` → false. `d` is never dereferenced. | Returns the seed `crc16` **unchanged**, byte-identical, for every one of the 65536 seeds. No memory read. | `err_len_zero_valid_ptr_returns_seed_unchanged` | [x] |
| 2 | `crc16` | `len == 0`, `d == NULL`. Same path as #1: the null pointer is never dereferenced, so this is *not* a crash in either language. | Returns the seed unchanged; no fault. (Rust's wrapper must early-return **before** `slice::from_raw_parts`, since that call is UB on null even with len 0.) | `err_len_zero_null_ptr_returns_seed_unchanged` | [x] |
| 3 | `crc16` | `len == 0`, `d` = dangling/unmapped non-null pointer (e.g. `0x1`, `usize::MAX`). Never dereferenced. | Returns the seed unchanged; no fault. Proves the zero-length short-circuit precedes any pointer use. | `err_len_zero_wild_ptr_no_deref` | [x] |
| 4 | `crc16` | `len == 1..=7` — strictly less than the slice-by-8 threshold. The `len >= 8` loop body **never** executes; only the byte-at-a-time tail runs. One step below the documented block boundary. | Pure tail-loop CRC. Must match for all 7 lengths × all seeds. | `err_len_below_slice_by_8_threshold` | [x] |
| 5 | `crc16` | `len == 8` exactly — the *first* value that enters the slice-by-8 loop; one step past the `len < 8` range. Loop runs once, `len -= 8` → 0, tail loop runs zero times. | Exactly one slice-by-8 round, no tail. | `err_len_exactly_eight_boundary` | [x] |
| 6 | `crc16` | `len == 7 / 8 / 9` triple straddling the branch boundary, same buffer and seed. | Three different code-path mixes, all must match. | `err_len_seven_eight_nine_straddle` | [x] |
| 7 | `crc16` | Tail-loop `len--` **unsigned wraparound**: when the tail loop's condition finally fails, C's post-decrement still decrements, wrapping `len` from `0` to `UINT32_MAX`. If Rust mis-modelled this as a loop bound it would read ~4 GB out of bounds. | `len` is **dead** after the loop, so the wrap has *no* observable effect: the function returns normally. Rust must not iterate `UINT32_MAX` times. Asserted by requiring the call to return promptly with the right value. | `err_tail_loop_len_postdecrement_wrap_is_dead` | [x] |
| 8 | `crc16` | Seed at its extremes / one step past a byte boundary: `0x0000`, `0x0001`, `0x00FF`, `0x0100`, `0xFF00`, `0xFEFF`, `0xFFFE`, `0xFFFF`. The seed feeds table indices `crc>>8` and `crc & 0xFF`, and `crc << 8` (which discards the high byte on assignment to `tflac_u16`). | Well-defined u16 result; no index can leave `0..=255`. Verifies the truncating `crc16 << 8` matches C's int-promote-then-truncate. | `err_seed_extremes_and_byte_boundaries` | [x] |
| 9 | `crc16` | Every byte value `0x00..=0xFF` placed in **each** of the 8 slice-by-8 lanes. Lane *i* indexes a *different* table, so a swapped/mis-transcribed table row is only visible in one lane. Also the max index `255` — one step past the last valid table slot `254`. | Correct per-lane table lookup; index `255` must hit `tables[k][255]`, never out of bounds. | `err_all_byte_values_in_every_lane` | [x] |
| 10 | `crc16` | "Out-of-range enum value" analogue. The API has **no enum parameter** (grep: zero `enum` / `switch` / `case` in the C source), so there is no invalid-variant class for the compiler to have narrowed. The nearest equivalent is passing arbitrary *unconstrained* bit patterns in the only non-pointer scalars: `len` (full `u32` range, incl. values ≥ 2^31 with the sign bit set) and `crc` (full `u16` range). Rust must accept them without UB/panic, exactly like C. | For representable buffers: identical CRC. `len` with the high bit set is still an unsigned count in both languages (C `tflac_u32`, Rust `u32`) — no sign-extension divergence when widened to `usize`. | `err_no_enum_params_unconstrained_scalars` | [x] |
| 11 | `crc16` | **Oversized length**: `len` far larger than the buffer actually passed. This is a *contract violation* — C reads out of bounds (UB, typically SIGSEGV) and Rust does too. Not differentially testable for a *value*; the well-defined half of the boundary is that `len` up to the real buffer size always works, including `len` = large multiples of 8 (tested at 64 KiB) and lengths straddling every residue. | Both languages read OOB; no defined result to compare. Tested instead: the largest *valid* lengths agree exactly, so no premature truncation of `len` (e.g. `u32`→`u16`) hides in the Rust wrapper. | `err_large_valid_lengths_no_len_truncation` | [x] |
| 12 | `crc16` | `len` values whose `usize` conversion could truncate on a 32-bit target: `0xFFFF_FFFF`, `0x1_0000` etc. Rust's wrapper does `len as usize`. On the 64-bit test target this is a widening (lossless); a `len as u16`/`as u32`-narrowing bug would show up as a wrong CRC at `len > 0xFFFF`. | Identical CRC at `len = 0x1_0000` and above. | `err_len_above_u16_range` | [x] |

| 13 | `crc16` | **`len` with the sign bit set** (`len >= 0x8000_0000`). `tflac_u32` is unsigned, so these are ordinary valid counts in C. This is the *only* input class that can distinguish a correct zero-extending `len as usize` from a sign-extending `len as i32 as usize` (the latter yields `0xFFFF_FFFF_8000_0000` and reads wildly out of bounds). Exercised on a real 2 GiB read-only `MAP_NORESERVE` mapping. | Identical CRC from both `.so`s at `len` = `0x8000_0000`, `+1`, `+7`, `+8`. Verified: e.g. `len=0x80000000, seed=0xFFFF -> 0x800d` from both. | `err_len_with_sign_bit_set_no_sign_extension` | [x] |

**Rows: 13. Unchecked: 0.**

## Non-testable-by-construction, documented explicitly

Row 11's true out-of-bounds case (`len` > real buffer) and a null `d` with
`len > 0` are **undefined behaviour in the C original** — the C code
unconditionally dereferences `d` there. The Rust wrapper's `# Safety` contract
matches the C precondition exactly ("`d` must point to at least `len` readable
bytes"). Both crash; there is no defined return value to compare, so asserting
"same error code" is impossible for these. They are deliberately *not* asserted
as passing values; the adjacent defined boundaries (rows 1–3, 11, 12) are what
pin the behaviour down.
