# ERRORS.md — ERROR-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c` (the only translation unit). Every
`return`, every explicit check, every named limit constant was enumerated:

```
$ grep -n 'return\|== NULL\|NULL)\|INT_MAX\|INT_MIN\|assert\|goto' c_src/src/lib.c
8:  #define can_access_at_index(buffer, index) ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
16:     unsigned char *after_end = NULL;
23:     if ((input_buffer == NULL) || (input_buffer->content == NULL))
25:         return false;
58:                 goto loop_end;
64:     if (number_c_string == NULL)
66:         return false; /* allocation failure */
85:     if (number_c_string == after_end)
89:         return false; /* parse_error */
95:     if (number >= INT_MAX)   ->  item->valueint = INT_MAX;
99:     else if (number <= (double)INT_MIN) -> item->valueint = INT_MIN;
113:    return true;
```

`parse_number` is the only public entry point; it has exactly **3 reachable
`return false` sites** (4 counting the untestable `malloc` failure) and **2
saturating limit branches**. `false == (cJSON_bool)0`, `true == (cJSON_bool)1`.

Nothing is written to `*item` and nothing is added to `input_buffer->offset` on
any of the failure paths — that is part of the expected result and is asserted
by every error-path test.

## Table

| #  | function       | trigger (the exact invalid input/condition)                                                                     | expected C result | test | [x] |
|----|----------------|-----------------------------------------------------------------------------------------------------------------|-------------------|------|-----|
| E1 | `parse_number` | `input_buffer == NULL` (line 23), `item` valid                                                                   | `false` (0); `*item` untouched | `err_e1_null_input_buffer` | [x] |
| E2 | `parse_number` | `input_buffer == NULL` **and** `item == NULL` (both null; the null check precedes any `item` deref)              | `false` (0); no crash | `err_e2_both_null` | [x] |
| E3 | `parse_number` | `input_buffer->content == NULL` (line 23), any `length`/`offset` (incl. `length=0`, `length=10`, `offset=5`)      | `false` (0); `*item` untouched, `offset` unchanged | `err_e3_null_content` | [x] |
| E4 | `parse_number` | `input_buffer->content == NULL` **and** `item == NULL`                                                            | `false` (0); no crash | `err_e4_null_content_null_item` | [x] |
| E5 | `parse_number` | `number_c_string == NULL` — `malloc(number_string_length + 1)` failure (line 64)                                  | `false` (0) — **unreachable in practice**: `number_string_length <= length`, so the allocation is bounded by the caller's buffer. Documented, not testable without an allocator fault injector; Rust's `Vec` would abort instead of returning 0, matching only in that neither returns success. | *n/a (documented)* | [x] |
| E6 | `parse_number` | `number_c_string == after_end` because the temp buffer is **empty**: first byte at `offset` is not in `[0-9+-.eE]` (`goto loop_end` at i=0). Cases: `" 1"`, `"a1"`, `"null"`, `"true"`, `"[1]"`, `"\0" `, `"\t"`, `"x"`, `"N"`, `"n"`, `"i"`, `"I"`, `"#"`, `0x80..0xFF` | `false` (0); `*item` untouched, `offset` unchanged | `err_e6_first_byte_not_numeric` | [x] |
| E7 | `parse_number` | `number_c_string == after_end` because `length == 0` → loop never runs → empty temp buffer → `strtod("")` returns 0 with `endptr == nptr` | `false` (0) | `err_e7_zero_length` | [x] |
| E8 | `parse_number` | `number_c_string == after_end` because `offset >= length` (`offset == length`, `offset == length + 1`, `offset == SIZE_MAX`) → `can_access_at_index` false at `i == 0` → empty temp buffer | `false` (0); `offset` unchanged | `err_e8_offset_at_or_past_end` | [x] |
| E9 | `parse_number` | `number_c_string == after_end` although the temp buffer is **non-empty**: every byte is in the accepted charset but `strtod` cannot consume a single character. Cases: `"+"`, `"-"`, `"."`, `"e"`, `"E"`, `"+."`, `"-."`, `"e5"`, `"E-3"`, `"++1"`, `"--1"`, `"+-1"`, `"-+1"`, `".e1"`, `"..1"`, `"+e"`, `"-E"`, `".+"`, `"e+"`, `"eE"` | `false` (0); `*item` untouched, `offset` unchanged | `err_e9_charset_but_unparsable` | [x] |
| E10 | `parse_number` | Truncated token: `length` cuts the number off so that the *visible* prefix is unparsable (e.g. content `"-12"` with `length = 1` → temp buffer `"-"`). Verifies the C relies on `length`, not on a `'\0'` terminator | `false` (0) | `err_e10_truncated_to_unparsable` | [x] |
| E11 | `parse_number` | Saturating limit `number >= INT_MAX` (line 95): `"2147483647"`, `"2147483648"`, `"1e30"`, `"1e999"` (→ `+HUGE_VAL`), `"2147483646.9999999999"` | `true` (1); `valueint == INT_MAX == 2147483647`; `valuedouble` = full `strtod` value (may be `+inf`) | `err_e11_saturate_int_max` | [x] |
| E12 | `parse_number` | Saturating limit `number <= (double)INT_MIN` (line 99): `"-2147483648"`, `"-2147483649"`, `"-1e30"`, `"-1e999"` (→ `-HUGE_VAL`) | `true` (1); `valueint == INT_MIN == -2147483648`; `valuedouble` = full `strtod` value (may be `-inf`) | `err_e12_saturate_int_min` | [x] |
| E13 | `parse_number` | One step *inside* each limit — `"2147483646.5"`, `"-2147483647.5"` — must take the truncating `(int)number` branch, **not** the saturating one | `true` (1); `valueint == 2147483646` / `-2147483647` | `err_e13_one_step_inside_limits` | [x] |
| E14 | `parse_number` | `strtod` consumes only a **prefix** of the (charset-valid) temp buffer, so `after_end - number_c_string < number_string_length` — `offset` must advance by the *consumed* amount only: `"1e"`, `"1e+"`, `"1."`, `"1.2.3"`, `"1e5e5"`, `"1-2"`, `"1+2"`, `"12--"`, `".5.5"` | `true` (1); `offset += (after_end - number_c_string)`, strictly less than `number_string_length` | `err_e14_partial_consumption` | [x] |
| E15 | `parse_number` | `strtod` underflow to zero / subnormal (`"1e-999"`, `"5e-324"`, `"1e-400"`) — the C ignores `errno == ERANGE` | `true` (1); `valuedouble` = `strtod` result (`0.0` / subnormal), `valueint == 0` | `err_e15_underflow` | [x] |
| E16 | `parse_number` | Out-of-range "enum"-like value smuggled through the FFI boundary: `item->type` pre-set to a value that is not `cJSON_Number` (`-1`, `0`, `0x7FFFFFFF`, `INT_MIN`, `1<<30`) and `item->valueint` pre-set to garbage. On success both must be **overwritten**; on failure both must be **preserved** bit-for-bit | matches C exactly in both directions | `err_e16_out_of_range_type_field` | [x] |
| E17 | `parse_number` | `depth` field (never read or written by `parse_number`) pre-set to `SIZE_MAX` — must be preserved on both the success and the failure paths | `depth` unchanged | `err_e17_depth_preserved` | [x] |
| E18 | `parse_number` | Oversized `length`: `length = SIZE_MAX` with a short real allocation, but with a non-charset byte right after the valid bytes so the scan still terminates in bounds (`"12,"` + `length = SIZE_MAX`). Exercises `offset + index < length` with no `'\0'` safety net | `true` (1); consumes `"12"`, `offset == 2` | `err_e18_oversized_length` | [x] |
| E19 | `parse_number` | `offset + index` **unsigned wraparound** in `can_access_at_index`: `offset = SIZE_MAX`, `length = 8`. The C macro's `size_t` addition would wrap for `index >= 1`, but the loop already fails at `index == 0`, so the body never executes | `false` (0); no OOB access, `offset` unchanged | `err_e19_offset_wraparound` | [x] |
| E20 | `parse_number` | Embedded NUL inside an otherwise valid token (`"1\0 2"`, `length = 4`): `'\0'` hits the `default:` label and stops the scan | `true` (1); consumes `"1"`, `offset == 1` | `err_e20_embedded_nul` | [x] |
| E21 | `parse_number` | `item == NULL` **on the SUCCESS path** — the C never NULL-checks `item`, so `item->valuedouble = number` (line 92) stores through a null pointer | process is killed by **SIGSEGV** (signal 11), no exit code. Compared in a child process for `"123"`, `"0"`, `"-1.5e3"`, `"2147483648"`, `"-1e999"`, `".5"`, `"1e"` | `null_item_on_success_path_behaves_identically` | [x] |
| E21b | `parse_number` | `item == NULL` on the FAILURE paths (control for E21): `"+"`, `"]"`, `""`, `"e"`, `"."` | clean exit, `false` (0) — the store is never reached | `null_item_on_failure_path_is_clean_in_both` | [x] |
| E22 | `parse_number` | **Misaligned** `cJSON *` and/or `parse_buffer *` (skewed by 1..7 bytes). Nothing in `lib.h`/`lib.c` requires natural alignment and the C compiler emits ordinary loads/stores, so the C SUCCEEDS | `true` (1) with the full correct field values, clean exit — for all 3 × 7 combinations of {item, buffer, both} × skew 1..7 | `misaligned_pointers_behave_identically` | [x] |

### Divergences found and fixed by Phase C

| trigger | C | Rust (before) | fix |
|---------|---|---------------|-----|
| E21 `item == NULL` on the success path | SIGSEGV (11) | **SIGABRT (6)** — rustc's `-C debug-assertions` null-pointer UB check turned the fault into a non-unwinding panic | route the three `item` stores through `core::ptr::write_unaligned` (no null/alignment precondition) |
| E22 misaligned `cJSON *` / `parse_buffer *` | succeeds, exit 0 | **SIGABRT (6)** — rustc's debug alignment UB check aborted on an access the C performs happily | route every `item` / `input_buffer` field access through `read_unaligned` / `write_unaligned` |

Both were present only in `dev`-profile (`debug-assertions`) builds; the release
build already matched. They are now identical in **both** profiles.
