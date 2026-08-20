# ERRORS.md — Error-surface table (Phase C gate)

Derived mechanically from `c_src/src/simplestruct.c`. The whole function body is
8 lines, so the rejection surface is enumerable exhaustively rather than
sampled.

## Mechanical grep for every rejection construct

```
$ grep -n 'return\|assert\|RETURN_ERROR\|errno\|exit(\|abort\|NULL\|INT_M' \
        c_src/src/simplestruct.c c_src/include/simplestruct.h
simplestruct.c:35:        return smallest;      <- success return
simplestruct.c:37:    else return -1;          <- THE ONLY rejection
```

Findings:

* error-return macros (`RETURN_ERROR`, `CHECK`, `goto fail`, …): **none**
* `assert` / `static_assert` / `abort` / `exit`: **none**
* `return NULL`: **none** (function returns `int`, not a pointer)
* error enums / status codes: **none** (plain `int` result)
* explicit range checks, min/max constants, length/count checks: **none**
* `errno` use: **none**
* null checks: **exactly one** — `if (head)` at line 27

So the C library has **exactly one** rejection path.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `smallestValue` | `head == NULL` (the `else` arm of `if (head)`, line 37) | returns `-1` exactly | `err_e1_null_head` | [x] |

## Generic FFI boundary cases (required even though not in the table)

These are not distinct C rejection branches; they are the boundary conditions
every C API has. Each is covered by a differential test so the Rust cannot
diverge on them.

| # | condition | C behaviour (ground truth) | test | status |
|---|-----------|----------------------------|------|--------|
| G1 | NULL `head` (same as E1, asserted bit-exact and repeatedly, incl. after successful calls, to prove no cached state) | `-1` | `err_g1_null_head_repeated` | [x] |
| G2 | Single node whose `value` is genuinely `-1` — **aliases the error sentinel** | `-1`, indistinguishable from E1 (preserved quirk) | `err_g2_sentinel_aliasing` | [x] |
| G3 | Node `value == INT_MIN` (one step past the negative end of the value range) | `INT_MIN` (`-2147483648`), no UB / no clamping | `err_g3_int_min` | [x] |
| G4 | Node `value == INT_MAX` (one step past the positive end) | `INT_MAX` (`2147483647`) | `err_g4_int_max` | [x] |
| G5 | `INT_MIN` and `INT_MAX` in the same list — signed `<` must not be an unsigned/wrapping compare | `INT_MIN` | `err_g5_int_min_and_max` | [x] |
| G6 | Zero-length list expressed as NULL vs. length-1 list — the "empty / one" boundary | `-1` vs. the node's value | `err_g6_empty_vs_one` | [x] |
| G7 | Oversized length: 100 000-node list (deep `next` chain; C is iterative, so no stack limit) | the true minimum, no overflow/crash | `err_g7_oversized_length` | [x] |
| G8 | Out-of-range "enum"/tag value across the FFI boundary: the C API declares no `enum`, so the analogous untyped input is an arbitrary uninterpreted `int` bit pattern in `value` (all 2^32 patterns are valid `int`s). Swept with random full-range `u32`-as-`i32` bit patterns. | value returned verbatim / compared with signed `<` | `err_g8_arbitrary_bit_patterns` | [x] |
| G9 | Trailing garbage after the NULL terminator (a node past the end that must never be read) | terminator honoured; the extra node is invisible | `err_g9_node_past_terminator` | [x] |
| G10 | `next` forming a self-loop is **not** tested: the C would loop forever (documented precondition "NULL-terminated"), so it is UB/hang, not a rejection the C handles. | n/a — intentionally out of scope | — | n/a |

**Note on `G10`:** the C code has no cycle detection. Feeding it a cyclic list
makes it spin forever; that is not an error path the C "returns" from, so there
is nothing to differentially compare. It is recorded here only to show the
omission is deliberate, not an oversight.

## Verdict

- [x] Every row in the table above (E1, G1–G9) has a passing differential test
      asserting C and Rust return the **same exact value**, not merely "both
      failed".
