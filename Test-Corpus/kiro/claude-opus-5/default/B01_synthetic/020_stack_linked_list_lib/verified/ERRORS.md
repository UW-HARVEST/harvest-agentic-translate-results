# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Mechanically derived from the complete C source. The whole library body is:

```c
int smallestValue (struct ListNode *head) {   // simplestruct.c:26
    if (head) {                               // :27  <- the only rejection check
        int smallest = head->value;            // :28
        while (head->next) {                   // :29
            head = head->next;                 // :30
            if (head->value < smallest) {      // :31
                smallest = head->value;        // :32
            }
        }
        return smallest;                       // :35
    }
    else return -1;                            // :37 <- the only error return
}
```

Grep inventory of every rejection-capable construct in `c_src/`:

* `return` statements: 2 total (`return smallest;` = success, `return -1;` = error).
* `assert` / `abort` / `exit`: **none**.
* error enums / error macros (`RETURN_ERROR`, `errno`, out-params): **none**.
* explicit range / bounds / min-max constants: **none** (no `INT_MAX`, no length
  cap, no size argument at all).
* null checks: exactly one — `if (head)` on the single parameter.
* `#if` / `#ifdef` conditional compilation in the library body: **none**
  (only the `SIMPLESTRUCT_H_` include guard).

So the C code has exactly **one** distinct way to reject input.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `smallestValue` | `head == NULL` (the `if (head)` check at `simplestruct.c:27` fails) | returns `-1` (`simplestruct.c:37`); no memory is dereferenced | `err_e1_null_head` | [x] |

## Generic FFI-boundary boundaries also covered (not C-table rows)

These are not distinct C rejection branches — the C function has no length,
size, enum, or mode parameter — but they are the generic boundaries required for
any C API, so they are still tested differentially.

| # | boundary | why it is a boundary here | expected identical behavior | test | status |
|---|----------|---------------------------|-----------------------------|------|--------|
| G1 | NULL pointer argument | the only pointer parameter | both return `-1` | `err_e1_null_head` | [x] |
| G2 | "zero length" list | a 0-node list is *representable only* as `head == NULL`; there is no separate count argument | both return `-1` | `err_g2_zero_length_is_null` | [x] |
| G3 | `-1` as a legitimate list value | collides with the NULL sentinel; a real caller cannot distinguish them. C does **not** treat this as an error | both return `-1` for a list containing `-1` as its minimum, i.e. same value as the NULL case | `err_g3_minus_one_sentinel_collision` | [x] |
| G4 | one step past the value range: `INT_MIN`, `INT_MAX` | `int` field with a `<` comparison; `INT_MIN` is the extreme of the signed-comparison path | both return the same `int`; no UB/overflow path exists in C (`<` only, no arithmetic) | `err_g4_extreme_int_values` | [x] |
| G5 | `INT_MIN` / `INT_MAX` at head vs. tail vs. interior | position-dependent update of `smallest` | identical results | `err_g5_extremes_by_position` | [x] |
| G6 | oversized length (very long list, 1,000,000 nodes) | no length cap in C; iterative loop in both — checks neither side stack-overflows or diverges | identical minimum, no crash | `err_g6_oversized_length` | [x] |
| G7 | out-of-range "enum" value across FFI | **N/A by construction** — `smallestValue` takes no enum, mode, flag, or `int` selector parameter; the only argument is `struct ListNode *`. Documented here explicitly so the omission is a derived conclusion, not an oversight. The nearest analogue (an `int` field holding any bit pattern, including ones no "valid variant" would use) is covered by G4/G5 and by the randomized `i32` fuzz in `CONFIGS.md`. | — | (see G4/G5) | [x] |
| G8 | unaligned / garbage non-NULL pointer | C would dereference it (UB) — **deliberately not tested**: the C ground truth crashes, so there is no defined behavior to match. Both implementations dereference any non-NULL `head` identically. | UB in both | not tested (by design) | n/a |
| G9 | cyclic list (`next` forms a loop) | `while (head->next)` never terminates in C — infinite loop, not an error return. **Deliberately not tested** (the C ground truth hangs); the Rust loop has the identical structure. | hang in both | not tested (by design) | n/a |

All rows E1 and G1–G7 have passing differential tests. G7 is a derived
non-applicability; G8/G9 are documented C-undefined/non-terminating cases.
