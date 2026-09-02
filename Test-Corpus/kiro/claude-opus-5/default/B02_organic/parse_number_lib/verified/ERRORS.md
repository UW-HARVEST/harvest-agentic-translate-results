# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Mechanically derived by reading every control-flow exit and every comparison in
`c_src/src/lib.c` plus every constant in `c_src/include/lib.h`. There are no
`assert`s, no error enums and no pointer-returning functions in this library —
the only failure channel is the `cJSON_bool` return value (`0` = false).

Grep basis:

```
$ grep -n 'return\|assert\|NULL\|INT_MIN\|INT_MAX\|goto\|<\|>' c_src/src/lib.c
```

Exit points found: `return false` ×3 sites, `return true` ×1 site.
Comparisons found: `can_access_at_index` bound, `has_decimal_point`,
`number_c_string == after_end`, `number >= INT_MAX`, `number <= (double)INT_MIN`.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `parse_number` | `input_buffer == NULL` (first clause of the guard); `item` may be anything, incl. a valid pointer | returns `false` (0); `item` left completely untouched | `e1_null_input_buffer` | [x] |
| E2 | `parse_number` | `input_buffer != NULL` but `input_buffer->content == NULL` (second clause of the guard), any `length`/`offset`/`depth` | returns `false` (0); `item` untouched, `input_buffer` untouched | `e2_null_content` | [x] |
| E3 | `parse_number` | `malloc(number_string_length + 1)` returns `NULL` (allocation failure) | returns `false` (0); `item` untouched, `offset` untouched | `e3_allocation_failure` (documented-unreachable; `number_string_length+1` is ≤ `length+1` and always tiny, and no allocator-injection hook exists in the C `.so`. Rust mirrors it with `try_reserve_exact` returning `Err`, same `false`.) | [x] |
| E4 | `parse_number` | `number_c_string == after_end`, i.e. `strtod` consumed **zero** bytes of the collected string. Reached whenever the accepted-char run is empty **or** begins with a byte `strtod` cannot start a number with. Sub-triggers, each exercised: `offset >= length` (empty run), `length == 0`, first byte is outside `[0-9+\-eE.]` (`default: goto loop_end`), run is exactly `"."`, `"+"`, `"-"`, `"e"`, `"E"`, `"+."`, `"-."`, `"e5"`, `".e1"`, `"++1"`, `"--1"`, `"-e"`, `".+"` … | returns `false` (0); `item` untouched (`valuedouble`/`valueint`/`type` NOT written); `input_buffer->offset` NOT advanced | `e4_strtod_consumed_nothing`, `e4_empty_accepted_run`, `e4_random_unparsable_runs` | [x] |
| E5 | `parse_number` | bound check `can_access_at_index`: `(buffer->offset + index) < buffer->length` is false at `index == 0` → zero-length scan. Includes `offset == length`, `offset > length`, and `length == 0`. Note the C adds `offset + index` in wrapping `size_t` arithmetic — no overflow check. | scan collects 0 bytes → falls into E4 → `false` | `e5_offset_at_or_past_length`, `e5_size_t_wraparound_offset` | [x] |
| E6 | `parse_number` | scan terminator `default: goto loop_end` — the first byte not in `[0-9] ∪ {+,-,e,E,.}` stops collection. The rejected byte and everything after it must NOT be fed to `strtod` (in particular bytes past `length` must never be read). | only the prefix is parsed; `offset` advances by at most the prefix length; result derived from the prefix alone | `e6_terminator_byte_all_256`, `b*` rows | [x] |
| E7 | `parse_number` | numeric overflow at the top of `int` range: `number >= INT_MAX` (`2147483647`), incl. `+inf` produced by `strtod` on e.g. `"1e999"` (which also sets `errno=ERANGE`, ignored by the C) | `item->valueint = INT_MAX`; `item->valuedouble = number` (may be `inf`); returns `true` | `e7_saturate_int_max` | [x] |
| E8 | `parse_number` | numeric overflow at the bottom of `int` range: `number <= (double)INT_MIN` (`-2147483648.0`), incl. `-inf` from `"-1e999"`. Note `<=` (not `<`), so exactly `-2147483648.0` takes this branch. | `item->valueint = INT_MIN`; `item->valuedouble = number`; returns `true` | `e8_saturate_int_min` | [x] |
| E9 | `parse_number` | `(int)number` cast in the `else` branch with a value C's cast cannot represent — unreachable because E7/E8 fence it, and `NaN`/`inf` spellings (`nan`, `inf`) are unreachable through the `[0-9+\-eE.]` scan alphabet. Asserted unreachable rather than assumed. | n/a — branch only ever sees finite values in `(INT_MIN, INT_MAX)` | `e9_int_cast_branch_is_fenced` | [x] |
| E10 | `parse_number` | `item == NULL` — the C performs **no** null check on `item` and dereferences it after a successful `strtod`. This is a real input a caller can pass. | on the success path: dereferencing `NULL` → `SIGSEGV`. On the E1/E2/E4 failure paths `item` is never touched, so `item == NULL` is harmless and `false` is returned. | `e10_null_item_harmless_on_failure_paths`, `e10_null_item_segfaults_on_success_path` (forked child, compares signal) | [x] |
| E11 | `parse_number` | out-of-range "enum" values across the FFI boundary. This library declares no C `enum`; the enum-shaped values are `cJSON_bool` (any `int` is accepted; only `0`/non-`0` semantics) and `cJSON.type` (`int`, arbitrary on input). Passing arbitrary `int` bit patterns in `item->type`, `item->valueint`, arbitrary bits in `item->valuedouble` (incl. signalling NaN / trap representations), and arbitrary `depth` must not change behaviour, and the return value must be exactly `1` or `0` (not merely truthy/falsy). | pre-existing `item->type` is overwritten with `cJSON_Number` (8) on success and untouched on failure; `depth` never read or written; return value is exactly `1` or `0` | `e11_garbage_in_out_params`, `e11_return_value_is_exactly_0_or_1` | [x] |
| E12 | `parse_number` | zero and oversized lengths: `length == 0`; `length == SIZE_MAX` with a short real buffer (C will happily scan past the real allocation — undefined but must match as long as a terminator byte is inside the real allocation); `length` huge with `offset` huge | governed by E4/E5/E6; with an in-allocation terminator both must stop at it | `e12_zero_and_oversized_lengths` | [x] |

## Notes on constants (`c_src/include/lib.h`)

| constant | value | where it gates behaviour |
|----------|-------|--------------------------|
| `INT_MAX` | `2147483647` | E7 |
| `INT_MIN` | `-2147483648` | E8 |
| `cJSON_Number` | `1 << 3` = `8` | value written to `item->type` on success |
| `true` / `false` | `(cJSON_bool)1` / `(cJSON_bool)0` | exact return values (E11) |

## Notes on unreachable / equivalent branches

* **`offset + index` never overflows.** `can_access_at_index` adds in wrapping
  `size_t` arithmetic with no overflow check, but the sum cannot overflow while
  the scan loop is running: reaching iteration `i` requires
  `offset + (i-1) < length <= SIZE_MAX`, hence `offset + i <= SIZE_MAX`; at
  `i == 0` the sum is just `offset`. So the wrapping is never exercised, and
  `wrapping_add` / `saturating_add` / `checked_add` coincide here. `wrapping_add`
  is kept because it is the literal translation of C's defined `size_t`
  semantics. Verified by `e5_size_t_wraparound_offset` and by the fact that the
  `saturating_add` mutant is provably equivalent.
* **E7/E8 use `>=` / `<=`, but `>` / `<` would be equivalent.** At exactly
  `INT_MAX`/`INT_MIN` the `else` branch's `(int)` cast yields the same value the
  saturating branch writes. The `>=`/`<=` spelling is kept because that is what
  the C says.
* **The `'.' → decimal_point` rewrite loop is a no-op.** `decimal_point` is the
  local constant `'.'`, so the loop writes `'.'` over `'.'`. The loop is
  preserved for fidelity, not because it changes anything. (This is the C quirk
  behind the "replace with the decimal point of the current locale" comment: the
  code was never wired to `localeconv()`.) Confirmed across seven `LC_NUMERIC`
  locales by `h5_locale_dependent_strtod`.
* **NaN can never reach `(int)number`.** The scan alphabet is
  `[0-9] ∪ {+,-,e,E,.}`, so `strtod` can never be handed `nan`/`inf` spellings;
  `±inf` is only producible via overflow, and E7/E8 intercept it. Asserted by
  `e9_int_cast_branch_is_fenced`.

## Result

All 12 rows have a passing differential test. No divergence between the C and
Rust `.so`s was found on any error path, and the Rust source required no changes.
