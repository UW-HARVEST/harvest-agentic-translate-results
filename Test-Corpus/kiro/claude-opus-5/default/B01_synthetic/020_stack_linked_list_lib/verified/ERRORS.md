# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs. The whole library is
`c_src/src/simplestruct.c` (17 lines of code). Exhaustive grep:

```
$ grep -n "return" c_src/src/simplestruct.c c_src/include/simplestruct.h
src/simplestruct.c:35:        return smallest;
src/simplestruct.c:37:    else return -1;

$ grep -nE "assert|NULL|RETURN_ERROR|errno|exit\(|abort\(|enum" \
      c_src/src/simplestruct.c c_src/include/simplestruct.h
(no matches)
```

So the library has:

- exactly **one** `return` that is a rejection (`simplestruct.c:37`),
- **no** `assert`, **no** error enum, **no** `errno` use, **no** explicit range
  check, **no** min/max constant, **no** `NULL` literal (the null check is the
  implicit truthiness test `if (head)` at line 27),
- **no** out-parameters and **no** allocation, therefore no allocation-failure
  path.

The return type is a bare `int` with no reserved error range: every value in
`[INT_MIN, INT_MAX]` is a legitimate success result, including `-1`. That makes
the single sentinel ambiguous by construction; this is preserved, not fixed.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `smallestValue` | `head == NULL` — the `if (head)` test at `simplestruct.c:27` is false, so control reaches `else return -1;` at line 37 | returns `-1` exactly |

That is the complete error surface: one row.

## Generic FFI-boundary boundaries also covered (not distinct C branches)

These are not separate rows because the C code contains no code that
distinguishes them, but Phase C tests them anyway since the task requires
covering the generic boundaries of any C API:

| id | condition | why it is not its own row | expected behavior |
|----|-----------|---------------------------|-------------------|
| G1 | null head pointer | this *is* row E1 | `-1` |
| G2 | list of length 0 | not representable: a 0-length list is the null pointer, i.e. G1/E1 | `-1` |
| G3 | list of length 1 (`head->next == NULL`) | valid input; the `while` guard at line 29 is false on entry, so the loop body never runs | returns `head->value` verbatim, including when that value is `-1` (indistinguishable from the E1 sentinel) |
| G4 | `-1` as a legitimate payload value | success path, shares the E1 sentinel value | returns `-1`; **must** be identical to the E1 result |
| G5 | `INT_MIN` / `INT_MAX` payload values (one step past nothing — the full range of `int` is valid) | no range check exists in the C | returned verbatim; the `<` comparison at line 31 must not overflow or saturate |
| G6 | oversized length (long lists, e.g. 100k nodes) | no length limit or counter exists in the C | returns the true minimum; no overflow, no recursion-depth limit (C loop is iterative, so the Rust must be iterative too) |
| G7 | out-of-range enum value across the FFI boundary | **not applicable**: the API declares no enum and takes no integer mode/flag parameter. The only parameter is `struct ListNode *`. There is no enum to pass an invalid discriminant for. | n/a |
| G8 | misaligned / dangling / non-`ListNode` pointer | undefined behavior in C; the C performs no validation, so neither library can be required to agree | untested by design — UB is not differentially testable |

## Checklist

- [x] E1 — covered by `errors_e1_*` tests in `tests/differential.rs`
- [x] G1 — same test as E1
- [x] G2 — `errors_g2_zero_length_is_null`
- [x] G3 — `errors_g3_single_node_randomized`
- [x] G4 — `errors_g4_minus_one_payload_aliases_sentinel`
- [x] G5 — `errors_g5_int_extremes`
- [x] G6 — `errors_g6_oversized_list`
- [x] G7 — n/a, documented above (`errors_g7_no_enum_surface` asserts the
      signature has no integer/enum parameter to abuse)
- [x] G8 — out of scope (undefined behavior), documented above
