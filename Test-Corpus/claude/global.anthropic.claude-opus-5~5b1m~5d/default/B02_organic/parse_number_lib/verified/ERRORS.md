# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

Grep inventory of every rejection / error / bound in the C source:

```
$ grep -n 'return\|assert\|NULL\|INT_MAX\|INT_MIN\|default:' c_src/src/lib.c
8:#define can_access_at_index(buffer, index) ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
16:    unsigned char *after_end = NULL;
23:    if ((input_buffer == NULL) || (input_buffer->content == NULL))
25:        return false;
57:            default:
58:                goto loop_end;
64:    if (number_c_string == NULL)
66:        return false; /* allocation failure */
85:    if (number_c_string == after_end)
89:        return false; /* parse_error */
95:    if (number >= INT_MAX)
99:    else if (number <= (double)INT_MIN)
113:    return true;
```

There are **no** `assert`s, **no** error enums, **no** `RETURN_ERROR` macros and
**no** out-parameter error codes. The only failure channel is the
`cJSON_bool` return value: `false == 0`, `true == 1`.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `parse_number` | `input_buffer == NULL` (lib.c:23) | returns `false` (0); `item` completely untouched |
| E2 | `parse_number` | `input_buffer != NULL` but `input_buffer->content == NULL` (lib.c:23) | returns `false` (0); `item` untouched; `*input_buffer` untouched |
| E3 | `parse_number` | `malloc(number_string_length + 1)` returns `NULL` (lib.c:64) — allocation failure | returns `false` (0); `item` untouched; `*input_buffer` untouched. Not reachable deterministically from FFI (would need OOM / malloc interposition); asserted by inspection + a length that cannot be allocated is impossible because `number_string_length <= length`. Covered indirectly: the *only* observable behaviour is "return 0, mutate nothing", which is the same contract as E4. |
| E4 | `parse_number` | `strtod` consumes zero characters, i.e. `after_end == number_c_string` (lib.c:85) | returns `false` (0); `item` untouched; `input_buffer->offset` **not** advanced |
| E4a| `parse_number` | sub-case of E4: `length == 0` ⇒ `can_access_at_index` false at i=0 ⇒ `number_string_length == 0` ⇒ `strtod("")` | returns `false` (0) |
| E4b| `parse_number` | sub-case of E4: `offset == length` (nothing left) | returns `false` (0) |
| E4c| `parse_number` | sub-case of E4: `offset > length` (offset past end; unsigned compare in `can_access_at_index` still false) | returns `false` (0) |
| E4d| `parse_number` | sub-case of E4: `offset + index` wraps (`offset == SIZE_MAX`) ⇒ `can_access_at_index` false | returns `false` (0) |
| E4e| `parse_number` | sub-case of E4: first byte hits the `default:` arm of the switch (lib.c:57), i.e. any byte **not** in `[0-9+\-eE.]` — e.g. `"abc"`, `" 1"`, `"\0"`, `"x"`, `"\x80"` | returns `false` (0) |
| E4f| `parse_number` | sub-case of E4: bytes are in the accepted set but form no `strtod`-parsable prefix — `"+"`, `"-"`, `"."`, `"e"`, `"E"`, `"+."`, `"-."`, `".e1"`, `"e5"`, `"E-2"`, `"++1"`, `"--1"`, `"-e"`, `".-"` | returns `false` (0) |
| B1 | `parse_number` (bound) | `can_access_at_index` scan bound: `(offset + i) < length` (lib.c:8) — scanning stops at `length`, so a valid number that runs to the very end of a non-NUL-terminated buffer is still parsed, and trailing bytes past `length` are never read | scan length = `min(run of accepted bytes, length - offset)` |
| B2 | `parse_number` (bound) | `number >= INT_MAX` (== `2147483647.0`) (lib.c:95) — includes `+inf` from `strtod` overflow (`"1e999"`) | `item->valueint = INT_MAX`, `item->valuedouble = number` (possibly `inf`), returns `true` |
| B3 | `parse_number` (bound) | `number <= (double)INT_MIN` (== `-2147483648.0`) (lib.c:99) — includes `-inf` (`"-1e999"`) and exactly `-2147483648` | `item->valueint = INT_MIN`, returns `true` |
| B4 | `parse_number` (bound) | `INT_MIN < number < INT_MAX` (lib.c:105) — `(int)number` truncates toward zero | `item->valueint = trunc(number)`, returns `true` |
| M1 | `parse_number` (MISSING check, preserved bug) | `item == NULL` on the success path (lib.c:92) — the C never null-checks `item` | **SIGSEGV (signal 11) at `si_addr = 0x8`**, silently, with nothing on stderr. Comparable after all, by running each call in its own child process and comparing the fatal signal. **This row found a real divergence — see below.** |

## Checklist (Phase C)

- [x] E1
- [x] E2
- [x] E3 (by inspection; contract identical to E4, see note)
- [x] E4
- [x] E4a
- [x] E4b
- [x] E4c
- [x] E4d
- [x] E4e
- [x] E4f
- [x] B1
- [x] B2
- [x] B3
- [x] B4
- [x] M1 (real differential test; **found and fixed a divergence**)

## Traceability: row -> test

| row | test |
|-----|------|
| E1  | `phase_c_errors::e1_null_input_buffer_returns_false` |
| E2  | `phase_c_errors::e2_null_content_returns_false` |
| E3  | `phase_c_errors::e3_allocation_failure_contract_matches_e4` (contract check; OOM is not provokable) |
| E4  | `phase_c_errors::e4_strtod_consumed_nothing_returns_false` |
| E4a | `phase_c_errors::e4a_zero_length` |
| E4b | `phase_c_errors::e4b_offset_equals_length` |
| E4c | `phase_c_errors::e4c_offset_past_length` |
| E4d | `phase_c_errors::e4d_offset_size_max_wraps_in_can_access_at_index` |
| E4e | `phase_c_errors::e4e_first_byte_hits_default_arm` (all 256 byte values) |
| E4f | `phase_c_errors::e4f_accepted_but_unparsable_exhaustive` (all 3 615 strings of length 1-3 over the accepted alphabet) and `phase_c_errors::e4f_length_four_and_five_exhaustive_sampled` (all 50 625 of length 4, plus 60 000 sampled of length 5-8) |
| B1  | `phase_c_errors::b1_scan_is_bounded_by_length_only` |
| B2  | `phase_c_errors::b2_saturates_to_int_max` |
| B3  | `phase_c_errors::b3_saturates_to_int_min` |
| B4  | `phase_c_errors::b4_exact_boundary_one_step_either_side` |
| M1  | `phase_c_errors::m1_item_null_produces_the_same_fatal_signal` (re-execs itself once per implementation and compares signal / exit code / stderr) |

Generic FFI-boundary boundaries required in addition to the table:

| coverage | test |
|----------|------|
| every NULL-pointer combination that is defined | `phase_c_errors::generic_null_pointers` |
| zero and oversized `length`, every `offset` in `0..=len+1` | `phase_c_errors::generic_zero_and_oversized_lengths` |
| out-of-range enum ints in `cJSON.type` (`INT_MIN`, `INT_MAX`, negatives, every single-bit and inverted-single-bit pattern) crossed with success and failure inputs | `phase_c_errors::generic_out_of_range_enum_ints_in_item_type` |
| every interesting `double` bit pattern pre-loaded into the out-params (±0, ±inf, quiet/signalling NaN, denormals, `DBL_MAX`) | `phase_c_errors::generic_out_of_range_valueint_and_valuedouble_preimages` |
| one step past every documented binary64 range boundary | `phase_c_errors::generic_one_past_documented_ranges` |
| no state leaks across repeated calls | `phase_c_errors::generic_repeated_calls_do_not_leak_state` |

## Note on the two "unobservable" bounds

Mutation testing (`./mutate.sh`) confirms that changing `number >= INT_MAX` to
`number > INT_MAX`, or `number <= (double)INT_MIN` to `number < (double)INT_MIN`,
is **provably unobservable**: at `number == 2147483647.0` the `else` branch's
`(int)number` yields exactly `INT_MAX` anyway, and at
`number == -2147483648.0` it yields exactly `INT_MIN`. Likewise
`has_decimal_point` cannot be observed, because the replacement loop rewrites
`'.'` to `decimal_point`, which is hard-coded to `'.'` — a no-op. These are
recorded so the absence of a failing test for them is a proof, not a gap.

## The one real divergence this verification found (row M1)

`item == NULL` is UB in the C, but it is still an input a caller can pass, and
*which* fatal signal results is observable. Measured with a `SA_SIGINFO` handler
in a child process:

| artifact | before the fix | after the fix |
|----------|----------------|---------------|
| `c_src/build/libdriver.so` (reference) | SIGSEGV (11), `si_addr = 0x8`, stderr empty | unchanged |
| Rust `target/release/libdriver.so` | SIGSEGV (11), `si_addr = 0x8` | unchanged |
| Rust `target/debug/libdriver.so` | **SIGABRT (6)** + `panicked at src/lib.rs: null pointer dereference occurred` | SIGSEGV (11), `si_addr = 0x8` |
| Rust release with `-C debug-assertions=on` | **SIGABRT (6)** + panic message | SIGSEGV (11), `si_addr = 0x8` |

**Cause.** Written as the place expression `(*item).valuedouble = number`, rustc
emits a null-pointer-dereference UB check whenever `-C debug-assertions` is on —
and Cargo's `dev` profile turns it on by default, so the shipped debug `.so` had
it. The C has no such check.

**Fix** (`src/lib.rs`, `item_store!`): route all five stores to `*item` through
`core::ptr::addr_of_mut!` + `core::ptr::write`. `addr_of_mut!` computes the
field's address without dereferencing, and `ptr::write` is the same plain,
non-volatile, non-atomic store the C performs — so no check is emitted and the
fault is byte-identical in *every* profile, not just release. This is a
code-level fix rather than a `[profile.dev] debug-assertions = false` setting, so
it holds however the crate is built.

`si_addr = 0x8` also confirms the store order matches: `valuedouble` (offset 8)
is written first, exactly as the C's `-O0` cmake build does.
