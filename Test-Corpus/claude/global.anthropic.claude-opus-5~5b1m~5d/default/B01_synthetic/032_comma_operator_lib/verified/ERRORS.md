# ERRORS.md — Phase A: error-surface table

Mechanically derived from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). The full body of the library is:

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) {
        printf("%d %d\n", i, j);
    }
}
```

Exhaustive grep for every rejection mechanism:

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|<[[:space:]]*0|INT_(MAX|MIN)|#if' c_src/src/driver.c c_src/include/driver.h
(no matches beyond the include guard #ifndef DRIVER_H_ / #define / #endif)
```

Findings:

* `driver` returns `void` — there is **no** error code, no sentinel, no
  out-parameter, and no `errno` use.
* There are **no** `return` statements, **no** `assert`, **no** `NULL` checks,
  **no** explicit range checks, and **no** min/max constants.
* There are **no** pointer parameters, so no null-pointer rejection exists.
* There are **no** enum parameters, so no out-of-range-enum rejection exists.
* The only conditional in the library is the loop guard `i < x`, which is a
  *valid-path* branch (covered by `CONFIGS.md` rows 1–2), not a rejection: a
  non-positive `x` is accepted and simply produces zero iterations.
* The single library call is `printf`, whose return value the C code **ignores**;
  a `printf` failure is therefore not propagated and is not an observable
  rejection.
* `#ifndef DRIVER_H_` is a header include guard, not a configuration branch.

## Error-surface table

The C library has an **empty rejection surface**. The rows below are therefore
the *generic FFI boundary conditions* that this API can be subjected to at all.
Each is a real input a caller can pass across the FFI boundary, and each has a
differential test asserting C and Rust behave identically (same observable
result: no abort, no diagnostic, and byte-identical stdout).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `driver` | `x == 0` — zero "length"; loop guard `0 < 0` false | returns normally, prints nothing (empty stdout) | `err_row01_zero_length` | [x] |
| 2 | `driver` | `x == -1` — negative count, one step past the valid (>0) range | returns normally, prints nothing (empty stdout) | `err_row02_negative_one` | [x] |
| 3 | `driver` | `x == INT_MIN` (`-2147483648`) — extreme negative / most-negative int | returns normally, prints nothing (empty stdout) | `err_row03_int_min` | [x] |
| 4 | `driver` | `x == INT_MIN + 1` — boundary adjacent to the most-negative int | returns normally, prints nothing (empty stdout) | `err_row04_int_min_plus_one` | [x] |
| 5 | `driver` | arbitrary negative values (randomized, `INT_MIN..0`) — whole rejected half-range | returns normally, prints nothing for every value | `err_row05_random_negatives` | [x] |
| 6 | `driver` | out-of-range "enum-like" `int` bit patterns (`0x7FFF_FFFF`, `0x8000_0000`, `0xFFFF_FFFF`, `0xDEAD_BEEF` reinterpreted as `c_int`) passed where an enum/flag would be | no validation exists: sign decides; negative → empty stdout, positive → loop runs | `err_row06_out_of_range_enum_bits` | [x] |
| 7 | `driver` | repeated back-to-back calls including invalid (`<= 0`) values — no state to corrupt, no cumulative error | each call returns normally; stdout is the concatenation, unaffected by the no-op calls | `err_row07_interleaved_invalid_calls` | [x] |
| 8 | `driver` | `x == 1` — smallest value that is *not* rejected (one step past the rejected range) | returns normally, prints exactly `"0 0\n"` | `err_row08_smallest_accepted` | [x] |

Notes on rows deliberately **not** present:

* **Null pointer**: not applicable — `driver` takes no pointer arguments.
* **Oversized length**: `x == INT_MAX` is accepted by the C code and would run
  ~2^31 iterations (and overflow `j`, which is UB in C). It is not a rejection
  and cannot be executed in bounded time, so it is not an error row; the
  `j`-overflow boundary is documented as untestable in `CONFIGS.md`.
* **Error codes / sentinels**: none exist; `driver` is `void` and ignores
  `printf`'s return value. Equality of the *observable* result (stdout bytes and
  normal return) is the strongest available assertion and is what the tests make.
