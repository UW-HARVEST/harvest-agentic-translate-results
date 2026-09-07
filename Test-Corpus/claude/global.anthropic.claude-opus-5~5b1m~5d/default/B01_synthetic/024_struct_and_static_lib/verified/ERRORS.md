# ERRORS.md — Phase C error-surface table

Derived mechanically from the full text of `c_src/src/driver.c` (66 lines,
23 of which are the licence header) and `c_src/include/driver.h`.

## Mechanical grep for every rejection construct

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if|switch|#if|malloc|free|-1' \
        c_src/src/driver.c c_src/include/driver.h
# (only licence-header prose matches; no code matches)
```

Findings, per construct class:

| construct searched | occurrences in C code |
|--------------------|-----------------------|
| `return` (any, incl. `return -1` / `return NULL`) | 0 |
| error-return macro (`RETURN_ERROR`, etc.) | 0 |
| `assert` / `static_assert` | 0 |
| `if` / `switch` / ternary (any branch at all) | 0 |
| `#if` / `#ifdef` conditional compilation | 0 |
| null-pointer check | 0 |
| explicit range / bounds check | 0 |
| min/max constant, error enum, `errno`, `exit`, `abort` | 0 |
| `malloc` / `free` (allocation failure path) | 0 |
| pointer or enum parameter in the public API | 0 |

Both public functions have signature `void f(int)`: they return `void`, so there
is **no error channel** (no return code, no out-parameter, no sentinel, no
`errno` use), and they take a single `int` by value, so there is **no pointer to
be null** and **no enum to be out of range** at the FFI boundary.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `run(int)` | **there is no rejecting input.** Every one of the 2^32 `int` values is accepted; the value is only used in `the_house.bedrooms += extra_bedrooms`. | no error is reportable (`void`); prints 4 lines and mutates global state | `errors::e1_run_accepts_every_int` (exhaustive over boundary + randomized ints; asserts both `.so`s produce identical output and neither signals failure) | [x] |
| 2 | `driver(int)` | **there is no rejecting input.** Forwards `x` unchanged to `run` twice. | no error is reportable (`void`); prints 8 lines | `errors::e2_driver_accepts_every_int` | [x] |

## Generic FFI boundaries required by Phase C even though absent from the table

| # | boundary | why it is / is not applicable here | test | status |
|---|----------|------------------------------------|------|--------|
| G1 | null pointer argument | **N/A** — no public function takes a pointer. Covered instead by verifying the exported symbols' arity/type: passing a value where a pointer cannot occur. | n/a (documented) | [x] |
| G2 | zero length | **N/A** — no length/size parameter exists. Nearest analogue is `extra_bedrooms == 0`, tested. | `errors::e3_zero_value` | [x] |
| G3 | oversized length | **N/A** — no length parameter. Nearest analogue is `INT_MAX`, which makes `bedrooms += INT_MAX` overflow signed `int`. C is UB here; gcc-compiled C wraps two's-complement, and the Rust uses `wrapping_add`, so both must agree. | `errors::e4_int_max_overflow` | [x] |
| G4 | one step past a valid range | `INT_MIN`, `INT_MAX`, `INT_MAX-1`, `INT_MIN+1`, and values chosen so `bedrooms + extra` lands exactly on `INT_MAX`, `INT_MIN`, and one past each. | `errors::e5_one_past_range` | [x] |
| G5 | out-of-range enum value across FFI | **N/A** — the library declares no enum and no public function takes one. Every `int` bit pattern is a legal argument (G3/G4 cover the extremes). | n/a (documented) | [x] |
| G6 | repeated / re-entrant invocation after an "error" | there is no error state to recover from; state simply accumulates. Verified that C and Rust accumulate identically after extreme arguments. | `errors::e6_state_after_extremes` | [x] |

## Result

- [x] Every row above has a passing error-path differential test (or is
      documented `N/A` with the reason derived from the C source).
