# ERRORS.md — error / rejection surface table (Phase C)

## Mechanical derivation

Every construct that could reject input was grepped for in the entire C source
(`c_src/src/long.c`, `c_src/include/long.h`):

```sh
grep -n -E 'RETURN_ERROR|return[[:space:]]+[^;]|assert|NULL|errno|exit\(|abort|
            enum|malloc|calloc|free|if[[:space:]]*\(|switch|#ifdef|#if ' \
     c_src/src/long.c c_src/include/long.h
```

Result, after discarding comment/licence lines: the only matches are the three
`for` loop headers and the arithmetic in the kernel body. Concretely:

* `RETURN_ERROR` / error macros: **0**
* `return <value>` statements: **0** (the only `return;` is the bare one at the
  end of `long_exec`; both public functions are `void`)
* `assert` / `abort` / `exit`: **0**
* `NULL` checks, pointer parameters: **0** (no function takes a pointer)
* error `enum`s or status codes: **0**
* `if` / `switch` / `#ifdef` branches: **0**
* heap allocation that could fail: **0** (`array` is a `.bss` global)
* explicit range / min / max checks or constants: **0**

**The C library has no error surface.** Both entry points return `void`, the
only scalar parameter is `unsigned int seed`, and *every* one of its 2^32 values
is a valid, accepted input. There is no invalid input that the C code rejects,
so there is no error code or sentinel to match.

Because "no rejection" is itself a behaviour that the Rust must reproduce
exactly, the table below enumerates one row per *distinct boundary or
would-be-invalid condition that a C API of this shape can be handed*, together
with the behaviour the C actually exhibits. Each row is a differential test:
both `.so`s are driven with that exact condition and must agree byte-for-byte
(stdout bytes, all 0x100000 bytes of `array`, and process survival — no
panic/abort/UB-trap on the Rust side where C completes).

Rows 1–8 are the parameter-domain boundaries; rows 9–16 are the arithmetic
edge values that reach the kernel's overflow / negative-division /
implementation-defined corners (the only places in this library where a naive
Rust translation can diverge or panic); rows 17–21 are the generic C-API
boundaries the brief calls out (out-of-range enum-like values, oversized
lengths, null/zero, state misuse).

## Table

| # | function | trigger (exact invalid input / condition) | expected C result |
|---|----------|-------------------------------------------|-------------------|
| 1 | `long_exec` | `seed = 0` — glibc `srand(0)` internally substitutes seed 1, so this is a genuine special case | no error; completes and prints one `%d\n` line; deterministic value identical to the value produced for the substituted seed |
| 2 | `long_exec` | `seed = 1` | no error; prints the same line as row 1 (glibc `srand(0)` ≡ `srand(1)`) |
| 3 | `long_exec` | `seed = UINT_MAX` (`4294967295`, one past the largest "signed-looking" seed) | no error; completes, prints a value |
| 4 | `long_exec` | `seed = 2147483648` (`2^31`, first value with the sign bit set — would be negative if the parameter were mis-declared `int` in Rust) | no error; completes, prints a value; must not be confused with `-2147483648` |
| 5 | `long_exec` | `seed = 2147483647` (`INT_MAX`, one step below row 4) | no error; distinct deterministic value |
| 6 | `long_exec` | `seed` passed as a *negative* `int` from the caller (`-1`), i.e. an out-of-range value for the declared `unsigned int` | C reinterprets the bit pattern as `4294967295`; identical result to row 3 (no rejection) |
| 7 | `long_exec` | called twice in a row with the same seed | no error; second call reseeds and prints the *identical* line (no hidden accumulated state) |
| 8 | `long_exec` | called twice with different seeds | no error; two different deterministic lines; the second is unaffected by the first |
| 9 | `perform_expensive_operations` | `array[i] = INT_MIN` — `x * 3 + 7`, `x << 1` and the `-x` idiom all overflow (signed overflow is UB in C; the compiled `.so` is the ground truth and wraps) | no error/trap; element rewritten with the wrapped result |
| 10 | `perform_expensive_operations` | `array[i] = INT_MAX` — `x * 3 + 7` overflows immediately | no error/trap; wrapped result |
| 11 | `perform_expensive_operations` | `array[i] = -1` — negative operand of `>> 3` (implementation-defined: arithmetic shift) | no error; result of arithmetic-shift semantics |
| 12 | `perform_expensive_operations` | `array[i] = 0` — `x % 7 == 0`, `x / 2 == 0` degenerate case; also the entire zero-initialised `.bss` state before any other call | no error; deterministic non-zero orbit |
| 13 | `perform_expensive_operations` | `array[i]` negative and odd, e.g. `-3`, `-7`, `-2147483647` — `x / 2` must truncate *toward zero* and `x % 7` must keep the sign of the dividend (C99 semantics, not floor semantics) | no error; truncating-division result (e.g. `-3 / 2 == -1`, `-3 % 7 == -3`) |
| 14 | `perform_expensive_operations` | `array[i] = -2147483648` reached *mid-kernel* so that `x / 2` is applied to `INT_MIN` (the classic `INT_MIN / -1` neighbour; `/2` is safe but is the boundary of the division range) | no error; `INT_MIN / 2 == -1073741824` |
| 15 | `perform_expensive_operations` | every element `= INT_MIN` (whole 1 MiB array at the extreme) — 262144 × 100 overflowing operations, no per-element variation | no error; uniform wrapped result across the array |
| 16 | `perform_expensive_operations` | `array` left at its initial all-zero `.bss` value and the function called before `long_exec` ever runs (state-order misuse) | no error; operates on zeros; array becomes the 100-step orbit of 0 |
| 17 | `perform_expensive_operations` | called with a *non-zero* argument through a mismatched FFI prototype (`void f()` in C accepts any argument list; an out-of-range "enum-like" `int` such as `0x7fffffff` or `-999` is a real value a caller can pass) | argument ignored entirely; identical result to the zero-argument call |
| 18 | `long_exec` | called through a mismatched prototype with *extra* arguments / a 64-bit value whose high half is garbage (`0xDEADBEEF_00000007`) | only the low 32 bits are the `unsigned int` seed; identical result to `seed = 7` |
| 19 | `array` | caller writes *past* the logical element count is impossible to do safely, but the boundary elements `array[0]` and `array[262143]` (last valid index; `262144` is one past the end) must both be processed | both boundary elements transformed; the object is exactly 0x100000 bytes in both `.so`s, so a caller indexing `[0]` and `[262143]` sees identical bytes |
| 20 | `perform_expensive_operations` | called repeatedly (0, 1, 2, 3, … times) with no intervening `long_exec` — "zero length" and "many" composition counts | no error; k-fold composition of the 100-step kernel; `k = 0` leaves the array untouched |
| 21 | `long_exec` | `array` pre-poisoned by the caller with hostile values (`INT_MIN`/`INT_MAX`/random) before the call — the seeded fill must overwrite *all* 262144 elements, so the poison must have **no** effect | no error; printed line identical to the call made from a clean array (proves no element is skipped) |

## Status

| # | test | default features | `debug-stats` |
|---|------|------------------|---------------|
| 1 | `err_row01_02_seed_zero_equals_one` | [x] | [x] |
| 2 | `err_row01_02_seed_zero_equals_one` | [x] | [x] |
| 3 | `err_row03_seed_uint_max` | [x] | [x] |
| 4 | `err_row04_05_sign_bit_seeds` | [x] | [x] |
| 5 | `err_row04_05_sign_bit_seeds` | [x] | [x] |
| 6 | `err_row06_negative_seed_reinterpreted` | [x] | [x] |
| 7 | `err_row07_repeat_same_seed` | [x] | [x] |
| 8 | `err_row08_repeat_different_seed` | [x] | [x] |
| 9 | `err_row09_10_extreme_scalars` | [x] | [x] |
| 10 | `err_row09_10_extreme_scalars` | [x] | [x] |
| 11 | `err_row11_negative_shift_operand` | [x] | [x] |
| 12 | `err_row12_zero_element` | [x] | [x] |
| 13 | `err_row13_truncating_division_signs` | [x] | [x] |
| 14 | `err_row14_int_min_division` | [x] | [x] |
| 15 | `err_row15_whole_array_int_min` | [x] | [x] |
| 16 | `err_row16_bss_initial_state` | [x] | [x] |
| 17 | `err_row17_extra_arg_ignored` | [x] | [x] |
| 18 | `err_row18_high_half_garbage_seed` | [x] | [x] |
| 19 | `err_row19_boundary_indices` | [x] | [x] |
| 20 | `err_row20_composition_counts` | [x] | [x] |
| 21 | `err_row21_poisoned_array_overwritten` | [x] | [x] |

All 21 rows pass under both feature combinations. See `tests/errors.rs`.
