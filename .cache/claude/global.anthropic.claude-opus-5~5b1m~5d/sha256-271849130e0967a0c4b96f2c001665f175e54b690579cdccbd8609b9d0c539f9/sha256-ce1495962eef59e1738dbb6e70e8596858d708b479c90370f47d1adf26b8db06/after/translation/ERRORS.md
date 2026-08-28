# ERRORS.md — Phase C error-surface table

Every distinct way `c_src/src/lib.c` rejects, clamps, or bails out on input.
Derived mechanically by grepping the C for `return`, `NULL`, `isnan`, `isinf`,
`INT_MAX`, `INT_MIN`, `default:`, and every `if (` guard. There are **no**
`assert`s in the C source, and the only sentinel value used is `-1`
(`malloc` failure). Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `safe_double_to_int` (L49) | `d` is NaN (any payload: `f64::NAN`, `-NAN`, signalling bit patterns) | `0` |
| E2 | `safe_double_to_int` (L53) | `d == +INFINITY` (`isinf` true, `d > 0`) | `INT_MAX` = `2147483647` |
| E3 | `safe_double_to_int` (L53) | `d == -INFINITY` (`isinf` true, `d > 0` false) | `INT_MIN` = `-2147483648` |
| E4 | `safe_double_to_int` (L57) | `d >= (double)INT_MAX`, i.e. `d >= 2147483647.0` (finite overflow, incl. exactly `2147483647.0`) | `INT_MAX` |
| E5 | `safe_double_to_int` (L60) | `d <= (double)INT_MIN`, i.e. `d <= -2147483648.0` (finite underflow, incl. exactly `-2147483648.0`) | `INT_MIN` |
| E6 | `allocate_and_compute` (L105) | `malloc(size * sizeof(DataPoint))` returns `NULL`. `size` is `int` promoted to `size_t`, so **every `size < 0`** becomes a ~2^64 byte request → `NULL` | `-1` |
| E7 | `allocate_and_compute` (L105) | `size > 0` so large that `(size_t)size * 16` exceeds what `malloc` can serve → `NULL` | `-1` |
| E8 | `switch_fallthrough_calculator` (L95) | `operation` matches no `case`: any value `< 0` or `> 4` (incl. `-1`, `5`, `INT_MIN`, `INT_MAX`) — the out-of-range-enum-across-FFI case | `0` (result discarded) |
| E9 | `fallcalc` (L145) | `malloc(5 * sizeof(int))` returns `NULL` (20-byte request; unreachable in practice) | `-1` (returned **unmasked**, before `&= 0777`) |
| E10 | `fallcalc` (L163 → E6) | `param4 % 10 + 1 < 0`, i.e. `param4 % 10 <= -2` (truncated remainder `-2..-9`, e.g. `param4 = -2..-9`, `-12`, `INT_MIN` whose remainder is `-8`) → inner `allocate_and_compute` gets a **negative** size and returns `-1`. Note `param4 % 10 == -1` gives size `0` → `malloc(0)` → succeeds (E13), and `param4 % 10 == 0` gives size `1` → succeeds. | `-1` is *added into* `result`, then `&= 0777`; **not** propagated as an error |
| E11 | `process_array_reverse` (L71) | `count <= 0` (incl. `INT_MIN`) — loop guard `i < count` never true, so `end` is **never dereferenced**; a NULL `end` is accepted | `0` |
| E12 | `foreach_sum` (L130) | `count <= 0` (incl. `INT_MIN`) — `FOREACH` guard `idx < size` never true, so `array` is **never dereferenced**; a NULL `array` is accepted | `0` |
| E13 | `allocate_and_compute` (L109/L115) | `size == 0` → `malloc(0)`, which on glibc returns a **non-NULL** unique pointer, so the `NULL` guard does *not* fire; both loops are skipped | `0` (not `-1`) |
| E14 | `safe_double_to_int` (L64) | `multiplier`/sum arithmetic in `allocate_and_compute` overflows to `±Inf` (e.g. `multiplier = f64::MAX`) → falls into E2/E3 | `INT_MAX` / `INT_MIN` |
| E15 | `allocate_and_compute` (L111) | `multiplier` is NaN or the product is `0 * Inf` → `sum` becomes NaN → E1 | `0` |

## Notes on non-errors (deliberately *not* rejected by the C)

These are inputs a reader might expect to be validated but which the C accepts
silently; the Rust must accept them identically rather than panicking:

- **Signed integer overflow.** `base_value = param1 * 0100 + param2`,
  `result *= OCTAL_BASE`, `result *= 3`, `points[i].value = i * OCTAL_BASE` and
  every accumulation can overflow `int`. This is UB in C; the reference `.so` is
  built with no `-O` flag (CMake sets no `CMAKE_BUILD_TYPE`), so it wraps
  two's-complement. The Rust uses `wrapping_*` everywhere to match, and must
  never panic with `overflow-checks` on.
- **Negative modulo.** `param3 % 5` and `param4 % 10` use C truncated division,
  so negative inputs give **negative** remainders (`-7 % 5 == -2`). A negative
  `param3 % 5` therefore lands in the `default:` arm (E8), and a negative
  `param4 % 10` drives E10. Rust's `wrapping_rem` has the same truncating
  semantics. `INT_MIN % 5` / `INT_MIN % 10` do not trap (divisor is not `-1`).
- **NULL pointers with positive counts.** `process_array_reverse(NULL, 3)` and
  `foreach_sum(NULL, 3)` dereference NULL in both languages (segfault). This is
  UB, is not a defined rejection, and is therefore **not** differentially tested.
- **`process_array_reverse` reads backwards.** `fallcalc` passes
  `data_array + 4` with `count = 5`, so it reads indices 4,3,2,1,0 — in bounds.
  Calling it with a larger `count` walks off the front of the buffer (UB), so
  only in-bounds `count`s are tested.

## Status — ALL ROWS PASSING

| row | test | status |
|-----|------|--------|
| E1 | `errors.rs::e1_nan_returns_zero` (2000 NaN payloads) | [x] |
| E2 | `errors.rs::e2_positive_infinity_returns_int_max` | [x] |
| E3 | `errors.rs::e3_negative_infinity_returns_int_min` | [x] |
| E4 | `errors.rs::e4_at_or_above_int_max_clamps` | [x] |
| E5 | `errors.rs::e5_at_or_below_int_min_clamps` | [x] |
| E6 | `errors.rs::e6_negative_size_returns_minus_one` | [x] |
| E7 | `errors_malloc_failure.rs::e7_and_e9_forced_malloc_failure` | [x] |
| E8 | `errors.rs::e8_out_of_range_operation_returns_zero` | [x] |
| E9 | `errors_malloc_failure.rs::e7_and_e9_forced_malloc_failure` | [x] |
| E10 | `errors.rs::e10_inner_alloc_failure_is_folded_not_propagated` | [x] |
| E11 | `errors.rs::e11_process_array_reverse_nonpositive_count_accepts_null` | [x] |
| E12 | `errors.rs::e12_foreach_sum_nonpositive_count_accepts_null` | [x] |
| E13 | `errors.rs::e13_size_zero_is_not_an_error` | [x] |
| E14 | `errors.rs::e14_sum_overflow_clamps_to_int_extremes` | [x] |
| E15 | `errors.rs::e15_nan_accumulator_returns_zero` | [x] |
| generic | `errors.rs::generic_one_past_every_documented_range` (null ptrs, zero/oversized/negative lengths, one-past-range, out-of-range enum ints) | [x] |

### How E7 and E9 were made reachable

Both are `malloc(...) == NULL` branches that ordinary inputs cannot trigger
(E9's request is only 20 bytes). `tests/errors_malloc_failure.rs` defines
`malloc` in the **test executable**, which on glibc/ELF preempts libc's for the
whole process — including both `dlopen`ed objects. Because the C `.so` calls
libc `malloc` and the Rust `.so` deliberately *imports* libc `malloc` (rather
than using Rust's allocator), one interposer forces the identical failure in
both, which is what makes the comparison fair. E7 is driven for every
`size` in `1..=256` plus larger ones; E9 for nine parameter quadruples, and
additionally in a nested form where `fallcalc`'s own allocation succeeds but the
inner `allocate_and_compute` fails.

### Divergence found and fixed by this row

**E9 exposed a real bug in the Rust, visible only at `-O2` and above.** LLVM
recognised `fallcalc`'s *constant-size* `malloc(5 * sizeof(int))` whose pointer
does not escape and applied "heap-to-stack": the allocation became an `alloca`,
which cannot fail, so `if (data_array == NULL) return -1;` was folded away as
unreachable. With the allocation forced to fail, C returned `-1` while the
release-profile Rust returned `368`. Fixed in `src/lib.rs` by routing both
allocations through `c_malloc`, which hides the size and the result behind
`core::hint::black_box` so the genuine libc call and the NULL comparison
survive every optimisation level. Verified by disassembly (both `malloc` call
sites now present in the release `.so`) and by the test passing under
`release`, `release+opt3`, `release+opt-s`, `release+lto-thin`,
`release+lto-fat` and `release+codegen-units-1`.
