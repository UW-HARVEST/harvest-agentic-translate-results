# ERRORS.md — Phase C error-surface table

## Mechanical grep of the C source for rejection paths

Commands run against `c_src/`:

```
grep -n 'return\|assert\|RETURN_ERROR\|NULL\|errno\|exit\|abort\|if *(\|<\|>\|==\|!=' c_src/src/long.c c_src/include/long.h
```

Findings (whole library is 68 lines / 1 translation unit):

* `return;` — one bare `return` at the end of `long_exec` (not an error return).
* No `assert`, no `RETURN_ERROR`-style macro, no error enum, no `errno` use,
  no `exit`/`abort`.
* No `NULL` checks — neither public entry point takes a pointer.
* No explicit range checks. The only `if`-free control flow is three counted
  `for` loops with compile-time bounds (`ARRAY_SIZE`, `ITERATIONS`, `100`).
* No min/max constants other than `#define ARRAY_SIZE (256 * 1024)` and
  `#define ITERATIONS 2000`, both compile-time loop bounds, not validated input.
* Both public functions return `void`, so there is no error channel at all.

=> **The C library has an EMPTY intrinsic error surface.** Every row below is
therefore a *generic FFI boundary* row that the task mandates be covered
anyway: values at/past the edge of the parameter's domain, and re-entry /
state-boundary conditions. "Expected C result" is what the C `.so` actually
does, established by running it.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `long_exec` | `seed = 0` (boundary: minimum of `unsigned int`) | no error channel; `srand(0)`, runs to completion, prints one `%d\n` line. Rust must print the identical line. | [x] |
| 2 | `long_exec` | `seed = 1` (glibc's implicit default seed) | completes, prints identical line to Rust | [x] |
| 3 | `long_exec` | `seed = UINT_MAX` (0xFFFFFFFF, boundary: maximum) | completes, prints identical line to Rust | [x] |
| 4 | `long_exec` | `seed = 0x80000000` (one past `INT_MAX`, i.e. a negative `int` reinterpreted as `unsigned`) — checks that the parameter is treated as unsigned, not sign-extended | completes, prints identical line to Rust | [x] |
| 5 | `long_exec` | `seed = INT_MAX = 0x7FFFFFFF` (largest value that is also a valid `int`) | completes, prints identical line to Rust | [x] |
| 6 | `long_exec` | a 64-bit garbage value passed in the seed register (`0xDEADBEEF_00000007`) — extra high bits must be ignored because the ABI parameter is 32-bit | identical to `seed = 7`; C ignores upper 32 bits | [x] |
| 7 | `long_exec` | called twice in a row in the same process (re-entry over dirty global `array`; the second call overwrites `array` with fresh `rand()` output, so it must be idempotent w.r.t. the first) | second line == first line for the same seed | [x] |
| 8 | `perform_expensive_operations` | called with `array` fully zeroed (degenerate all-zero input; `0/2 + 0%7 == 0` fixed-point interaction) | no error; deterministic in-place transform | [x] |
| 9 | `perform_expensive_operations` | `array` filled with `INT_MIN` (`-2147483648`) — signed-overflow edge for `x*3`, `x<<1`, and `x/2`; in C `INT_MIN` is where `/` and `%` are most fragile | no error; C's actual (wrapping) result must be matched bit-for-bit | [x] |
| 10 | `perform_expensive_operations` | `array` filled with `INT_MAX` (`2147483647`) — signed-overflow edge for `x*3+7` | matched bit-for-bit | [x] |
| 11 | `perform_expensive_operations` | `array` filled with `-1` — `x >> 3` of a negative value (implementation-defined arithmetic shift) | matched bit-for-bit | [x] |
| 12 | `perform_expensive_operations` | `array` filled with values whose intermediate `x` hits `0` and `±7` (`x % 7 == 0` and sign-of-remainder cases; C `%` truncates toward zero so the remainder can be NEGATIVE) | matched bit-for-bit | [x] |
| 13 | `perform_expensive_operations` | called repeatedly (10x) on the same dirty global array — accumulated state boundary; verifies no drift between C and Rust after long chains | matched bit-for-bit after every call | [x] |
| 14 | `perform_expensive_operations` | out-of-range "enum"/flag equivalent: the function takes **no** parameters, so the corresponding boundary is calling it through a `extern "C" fn(c_int) -> c_int`-shaped pointer with junk arguments in the arg registers — they must be ignored | identical output to the zero-argument call | [x] |
| 15 | `array` | full 32-bit input domain of the per-element transform, swept exhaustively in contiguous blocks plus randomized blocks (there is no "invalid" element value; every one of the 2^32 values is accepted, so every one is a boundary candidate) | every element matches bit-for-bit | [x] |

## Notes on C constructs that could have diverged (and are covered above)

* `x * 3 + 7`, `x - (x << 1)`: signed overflow — UB in ISO C, wraps in the
  compiled artifact. Rows 9, 10, 15.
* `x ^ (x >> 3)`: right shift of a negative `int` — implementation-defined
  (arithmetic on gcc/x86-64). Rows 11, 15.
* `x / 2 + x % 7`: truncating division and sign-following remainder. Rows 9, 12, 15.
* `size_t i` vs `int j` loop counters: no overflow possible at these bounds.
* `printf("%d\n", ...)`: `int` formatting, including the negative case. Rows 1-7.

## Phase C result

All 15 rows have a passing differential test; no row is unchecked.

* Rows 1-7 (`long_exec` seed-domain and re-entry boundaries):
  `tests/long_exec_full.rs` — `compare_recorded_long_exec_runs`,
  `errors06_junk_abi_equals_plain_seed_7`,
  `configs20_errors07_dirty_array_does_not_affect_result`,
  `recorded_results_are_seed_dependent`. All pass; C and Rust agree on the final
  1 MiB `array` and on the exact stdout bytes for every seed, including
  `0`, `1`, `INT_MAX`, `0x80000000`, `UINT_MAX`, and a 64-bit seed register with
  garbage high bits (which must, and does, equal `seed = 7` in both).
* Rows 8-15 (`perform_expensive_operations` value-domain edges, dirty-state
  re-entry, junk argument registers, symbol shape): `tests/differential.rs` —
  `cfg03_err08_all_zeros`, `cfg04_err11_err12_uniform_small_values`,
  `cfg05_err09_err10_uniform_overflow_edges`,
  `cfg14_err13_repeated_invocation_on_dirty_state`,
  `err14_junk_arguments_are_ignored`,
  `err15_no_pointer_parameters_symbol_shapes_match`, plus the sweeps
  `cfg15_...` and `sweep_wide_random` for row 15. All pass.

Since both public functions return `void` and take no pointers, there is no
error code or sentinel to compare: the "same rejection" requirement degenerates
to "same observable effect", which is what is asserted byte-for-byte.
