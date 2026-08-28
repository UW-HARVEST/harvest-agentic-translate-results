# ERRORS.md — Phase A: error / rejection surface

Mechanically derived from `c_src/src/lib.c` by enumerating **every** guard,
`return -1`, `default:` fall-through, `NULL` test and implicit pointer
dereference in the file:

```sh
grep -n 'return\|if (\|else\|NULL\|assert\|switch\|case\|default' c_src/src/lib.c
```

The library has **no** error enum, no `errno` use, no `assert`, and no
documented range constants; its entire rejection surface consists of
(a) `return -1` sentinels, (b) *silent* no-op / pass-through guards, and
(c) unchecked pointer dereferences (undefined behaviour, observable as a fatal
signal). Silent guards are included because "does nothing" is exactly the
result the Rust translation has to reproduce, and a wrong guard is invisible in
a return value.

Test file: `tests/phase_c_errors.rs` (`[x]` = differential test written **and
passing** against both `.so`s).

| # | function | trigger (exact invalid input / condition) | expected C result | test | status |
|---|----------|-------------------------------------------|-------------------|------|--------|
| E1 | `arity` | `len` (truncated to `unsigned char`) `== 0` — `lib.c:172 if (len < 2)` | returns `-1`, `params` never dereferenced | `e1_arity_len0` | [x] |
| E2 | `arity` | `len == 1` — same guard | returns `-1` | `e2_arity_len1` | [x] |
| E3 | `arity` | `len == 0` **and** `params == NULL` (guard short-circuits before any load) | returns `-1`, no crash | `e3_arity_len_lt2_null_params` | [x] |
| E4 | `arity` | `len == 256` / `512` / `0x100` — value outside `unsigned char`, truncates to `0` (`mov %edi,%eax; mov %al,-0x4(%rbp)`) | returns `-1` | `e4_arity_len_truncates_to_lt2` | [x] |
| E5 | `arity` | `len == 257` → truncates to `1` | returns `-1` | `e4_arity_len_truncates_to_lt2` | [x] |
| E6 | `arity` | `len == i32::MIN` (`0x8000_0000`) → truncates to `0` | returns `-1` | `e4_arity_len_truncates_to_lt2` | [x] |
| E7 | `arity` | `len < 0`, e.g. `-1` → truncates to `255`, which is **not** `< 2` unsigned → falls into the `else` branch | returns `arity4(params[0..3])` (does **not** reject) | `e5_arity_negative_len_is_unsigned` | [x] |
| E8 | `arity` | `len` in `4..=255` (any value past the last named case) → `else` branch | returns `arity4(params[0..3])` | `e6_arity_len_ge4_dispatch` | [x] |
| E9 | `arity` | `len == i32::MAX` (`0x7fffffff`) → truncates to `255` → `else` branch | `arity4(params[0..3])` | `e6_arity_len_ge4_dispatch` | [x] |
| E10 | `arity` | `len >= 2` with `params == NULL` — `lib.c:175` dereferences unconditionally (UB) | fatal signal (`SIGSEGV`) | `e7_crash_parity` (out-of-process) | [x] |
| E11 | `arity` | `len == 4` with a `params` buffer that only holds 2 valid `int`s — reads `params[2]`/`params[3]` out of bounds | no diagnostic; reads whatever follows (both libraries read the same bytes) | `e8_arity_reads_past_short_buffer` | [x] |
| E12 | `compare_allocations` | `malloc` returns `NULL` for the **first** allocation — `lib.c:91` | `free(NULL); free(ptr2); return -1` | `e9_malloc_failure_returns_minus1` (LD_PRELOAD child) | [x] |
| E13 | `compare_allocations` | `malloc` returns `NULL` for the **second** allocation — same guard, second disjunct | `free(ptr1); free(NULL); return -1` | `e9_malloc_failure_returns_minus1` | [x] |
| E14 | `compare_allocations` | both allocations fail | `return -1` | `e9_malloc_failure_returns_minus1` | [x] |
| E15 | `arity4`/`arity2`/`arity3` | `malloc` failure propagates: `compare_allocations` returns `-1`, which is *added* to `result` (no rejection) | `result += -1` | `e10_malloc_failure_propagates_into_arity4` | [x] |
| E16 | `apply_bitmask` | `operation == 4` (one past the last `case`) — `default:` at `lib.c:66` | returns `value` unchanged | `e11_apply_bitmask_out_of_range_operation` | [x] |
| E17 | `apply_bitmask` | `operation == -1` (negative, no matching `case`) | returns `value` unchanged | `e11_apply_bitmask_out_of_range_operation` | [x] |
| E18 | `apply_bitmask` | `operation == i32::MIN` / `i32::MAX` (extreme out-of-range "enum" values crossing the FFI boundary) | returns `value` unchanged | `e11_apply_bitmask_out_of_range_operation` | [x] |
| E19 | `apply_bitmask` | every `operation` in `-300..=300` plus a randomized sweep of the whole `i32` range (a C `enum`/`switch` accepts any `int`) | `value` for all except `0..=3` | `e12_apply_bitmask_exhaustive_operation_sweep` | [x] |
| E20 | `shift_array` | `positions == 0` — `lib.c:36 positions > 0` fails | no-op: array left byte-identical | `e13_shift_array_rejects_nonpositive_positions` | [x] |
| E21 | `shift_array` | `positions < 0` (e.g. `-1`, `i32::MIN`) — same guard | no-op | `e13_shift_array_rejects_nonpositive_positions` | [x] |
| E22 | `shift_array` | `positions == size` — `lib.c:36 positions < size` fails | no-op | `e14_shift_array_rejects_positions_ge_size` | [x] |
| E23 | `shift_array` | `positions > size` (incl. `i32::MAX`) | no-op | `e14_shift_array_rejects_positions_ge_size` | [x] |
| E24 | `shift_array` | `size == 0` (with any `positions`) | no-op (`positions < 0` is false for `positions > 0`) | `e15_shift_array_zero_or_negative_size` | [x] |
| E25 | `shift_array` | `size < 0` (e.g. `-1`, `i32::MIN`) with `positions > 0` | no-op | `e15_shift_array_zero_or_negative_size` | [x] |
| E26 | `shift_array` | `arr == NULL` but guard fails (`positions <= 0`, or `positions >= size`) | no-op, **no** crash — the null pointer is never used | `e16_shift_array_null_ptr_guarded` | [x] |
| E27 | `shift_array` | `arr == NULL` with `0 < positions < size` → `memmove(NULL+p, NULL, n)` (UB) | fatal signal (`SIGSEGV`) | `e7_crash_parity` (out-of-process) | [x] |
| E28 | `shift_array` | `size` larger than the real buffer (`0 < positions < size`) — out-of-bounds `memmove` and zero-fill, no check exists | writes past the end; both libraries corrupt the identical bytes | `e17_shift_array_size_larger_than_buffer` | [x] |
| E29 | `process_string` | `*str == 0` (empty string) — `lib.c:45` | returns `0` (not `strlen`) | `e18_process_string_empty` | [x] |
| E30 | `process_string` | `str == NULL` — `*str` is dereferenced *before* any check (UB) | fatal signal (`SIGSEGV`) | `e7_crash_parity` (out-of-process) | [x] |
| E31 | `process_string` | non-NUL-terminated buffer — `strlen` runs past the end, no length parameter exists | reads until the first `0` byte; both libraries return the same length | `e19_process_string_unterminated` | [x] |
| E32 | `init_matrix` | `matrix == NULL` — written through unconditionally (UB) | fatal signal (`SIGSEGV`) | `e7_crash_parity` (out-of-process) | [x] |
| E33 | `init_matrix` | buffer shorter than `3*4*sizeof(int)` — no size parameter, always writes 12 `int`s | writes 48 bytes regardless; identical overflow in both | `e20_init_matrix_writes_exactly_12` | [x] |
| E34 | `arity4` | `param3 != 0` with `result * param3` overflowing `int` (signed-overflow UB; gcc `-O0` emits a wrapping `imul`) | wraps, then truncating division by `100` | `e21_arity4_overflow_wraps` | [x] |
| E35 | `arity4` | `param4 != 0` with `result + param4` overflowing `int` (signed-overflow UB) | wraps | `e21_arity4_overflow_wraps` | [x] |
| E36 | `arity4` | `param1 == i32::MIN` in `param1 % 4` (extreme dividend; C truncates toward zero, so a negative `param1` yields a negative "operation" and hits `default:`) | `0` for `i32::MIN`, negative remainders otherwise → `default:` | `e22_arity4_negative_modulo_hits_default` | [x] |
| E37 | `compare_allocations` | `val1 <= 0` — `*uninit_ptr > 0` is false, the `+10` is skipped (`lib.c:111`) | `result` without `+10` | `e23_compare_allocations_nonpositive_val1` | [x] |
| E38 | `arity`, `init_matrix`, `shift_array`, `process_string` | **misaligned** pointer arguments (`+1`/`+2`/`+3` bytes): the C code has no alignment check and uses plain `mov`, which tolerates misalignment on x86-64 | reads/writes succeed; identical results | `e25_misaligned_pointers` | [x] |
| E39 | `compare_allocations` | `ptr1 == ptr2` (an interposed allocator returning the same address twice) — the `else` branch of `lib.c:102-108`, unreachable with a real allocator | `result = 3`, and the `+10` bonus is decided by the value **in memory** (`val2`), not by `val1` | `e24_pointer_order_branches` | [x] |

Notes on the three "no diagnostic" classes above:

* **E10 / E27 / E30 / E32 (null-pointer dereference).** The C code performs the
  dereference before (or without) any check, so the only observable behaviour is
  a fatal signal. `tests/phase_c_errors.rs::e7_crash_parity` therefore re-executes
  the test binary in a child process, calls the C symbol and the Rust symbol in
  separate children, and asserts they die from the **same signal** (`SIGSEGV`)
  — not merely that both "failed somehow".
* **E12–E15 (allocation failure).** `malloc(sizeof(int))` never fails in
  practice, so the branch is reached by interposing a failing `malloc` with
  `LD_PRELOAD` in a child process (the shim is generated and compiled by the
  test itself) and asserting **both** libraries return exactly `-1`.
* **E11 / E28 / E31 / E33 (missing bounds checks).** These are cases where the C
  code has *no* rejection at all. The tests pin the exact out-of-bounds
  footprint (which bytes are read/written) so a Rust translation that clamped,
  or that wrote a different number of elements, would fail.
* **E38 / E39 (implicit inputs).** Alignment and the allocator's address ordering
  are inputs the C code never validates. They are driven explicitly (misaligned
  buffers; `LD_PRELOAD`ed `malloc` returning equal/ascending/descending
  addresses) because they select real C branches that ordinary calls cannot
  reach.

## Rejections that do *not* exist

Recorded so the absence is deliberate rather than overlooked: the library has no
`errno` use, no error enum, no `assert`/`abort`, no logging, no length or
capacity limits, no `NULL` check in `process_string`/`init_matrix`/`arity`, and no
check that `len` matches the real size of `params`. `arity`'s `len` is the only
"validated" parameter, and the only validation is `len < 2`.
