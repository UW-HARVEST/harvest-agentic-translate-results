# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection/error path in `c_src/src/simplestruct.c`
and `c_src/include/simplestruct.h`.

## Mechanical inventory of the C source

Full grep of every `return`, branch, assert, range check and limit constant:

```
simplestruct.c:27:    if (head) {                        <- the only null check
simplestruct.c:29:        while (head->next) {           <- loop guard, not a rejection
simplestruct.c:31:            if (head->value < smallest) {  <- data compare, not a rejection
simplestruct.c:35:        return smallest;               <- success return
simplestruct.c:37:    else return -1;                    <- THE ONLY ERROR RETURN
```

Findings:

* error-return macros (`RETURN_ERROR`, `CHECK`, goto-fail): **none**
* `assert` / `static_assert`: **none**
* error enums / status codes: **none** (return type is a bare `int`)
* explicit range checks, min/max constants, length/size checks: **none**
* null checks: **exactly one** — `if (head)` at line 27
* allocation (which could fail): **none** — the function never allocates

So the C library has exactly **one** distinct rejection path. Everything else the
function does is unconditional traversal of caller-owned memory.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `smallestValue` | `head == NULL` (the `if (head)` test fails, control reaches `else return -1`) | returns `-1` | `err_e1_null_head` | [x] PASS |

## Generic FFI boundary cases required by Phase C

These are not separate C rejection branches (the C code has only E1), but Phase C
mandates covering the generic boundaries of any C API. Each is exercised
differentially against both `.so`s.

| # | boundary | construction | expected C result | test | status |
|---|----------|--------------|-------------------|------|--------|
| B1 | null pointer | `smallestValue(NULL)` — same as E1, asserted repeatedly/idempotently | `-1` | `err_b1_null_repeated` | [x] PASS |
| B2 | zero length | a list of zero nodes is *only* representable as `NULL` (no count parameter exists) | `-1` | `err_b2_zero_length_is_null` | [x] PASS |
| B3 | minimum non-empty length | 1-node list, `next == NULL`; loop body never runs | node's own `value` | `err_b3_single_node` | [x] PASS |
| B4 | sentinel/return-value collision | a valid list whose true minimum **is** `-1`; C cannot distinguish this from the NULL error | `-1` (ambiguous by design — must be reproduced, not "fixed") | `err_b4_minus_one_ambiguity` | [x] PASS |
| B5 | value one step past range, low | list containing `INT_MIN` (`-2147483648`) in first / middle / last position | `INT_MIN` | `err_b5_int_min` | [x] PASS |
| B6 | value one step past range, high | list where every node is `INT_MAX` (`2147483647`) | `INT_MAX` | `err_b6_int_max` | [x] PASS |
| B7 | signedness trap | values whose bit patterns are large *unsigned* numbers but negative as `int` (e.g. `0x80000000`, `0xFFFFFFFF`); an unsigned comparison would pick a different winner | signed `<` semantics | `err_b7_signed_compare` | [x] PASS |
| B8 | oversized length | very long list (100 000 nodes) — checks no recursion-depth/stack limit divergence | correct minimum, no overflow | `err_b8_oversized_length` | [x] PASS |
| B9 | out-of-range enum across FFI | **N/A — no enum exists.** The public API has no enum, mode or flag parameter; the sole parameter is a pointer and the sole return is a bare `int`. Documented here so the row is explicitly discharged rather than silently skipped. The nearest analogue, an arbitrary non-pointer-valued `int` reinterpreted as the parameter, is undefined behaviour in C (wild pointer dereference) and is therefore *not* a testable input — only `NULL` is a defined invalid pointer value. | n/a | — | [x] N/A (justified) |

## Notes on what is deliberately NOT tested

The C contract requires `head` to be `NULL` or a valid, `NULL`-terminated,
acyclic chain. The following are undefined behaviour in the C original, so there
is no "correct C result" to match and a differential test would compare two
undefined behaviours:

* dangling / unaligned / wild non-null pointers,
* cyclic lists (`while (head->next)` never terminates in C — the Rust loop
  behaves identically, but the test would hang, so it is excluded),
* a `next` pointer that is non-null but not a valid `ListNode`.

## Gate status

- [x] Every row in this table has a passing differential test (or is explicitly
      justified as N/A). Phase C complete.
