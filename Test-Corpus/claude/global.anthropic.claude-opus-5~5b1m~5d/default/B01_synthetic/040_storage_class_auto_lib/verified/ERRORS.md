# ERRORS.md — Phase C error-surface table

Derived mechanically from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). The entire library body is:

```c
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

## Mechanical grep for every rejection mechanism

Searched the whole of `c_src` for every way C code can reject or fail:

| pattern searched | occurrences in C source |
|---|---|
| `return` (any value or bare) | 0 |
| `RETURN_ERROR` / error macro | 0 |
| `return -1` / `return NULL` / negative sentinel | 0 |
| error `enum` / status code type | 0 |
| `assert` / `static_assert` | 0 |
| `if` / `switch` / `?:` (any conditional) | 0 |
| explicit range check, min/max constant (`INT_MAX`, `LIMIT`, …) | 0 |
| null-pointer check | 0 (the API takes no pointer) |
| `exit` / `abort` / `longjmp` | 0 |
| `errno` use | 0 |
| `#ifdef` / conditional compilation affecting behavior | 0 (only the `DRIVER_H_` include guard) |

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | **none — the C library has no rejection path whatsoever** | — |

`driver` is a `void` function taking a single `int` by value. It has no return
value, no out-parameter, no status code, no sentinel, no pointer argument to
validate, and not a single conditional statement. **Every** `int` bit pattern is
an accepted input: there is no value of `x` for which the C code takes a
different path. Consequently the error-surface table is legitimately empty —
this is a real property of the C, not an omission.

## Boundary / hostile inputs covered anyway (Phase C tests)

Because the table is empty, Phase C instead pins down the generic boundaries
that every C API has, treating the printed line as the "result" that must match.
Each row below is an executed differential test in `tests/differential.rs`; all
inputs below are, per the analysis above, *valid* for the C, so "same
error/rejection" degenerates to "identical printed bytes, and neither side
aborts, traps, or panics". (`driver` returns `void`, so the `printf` return
value is not observable to a caller and cannot differ.)

The "expected C result" column records what the **built C `.so` actually
printed**, measured by linking a probe program against `c_src/build/libdriver.so`
— not what the C standard says, and not an assumption.

| # | trigger | expected C result | test |
|---|---------|-------------------|------|
| E1 | `x = 0` (zero / empty-magnitude input) | prints `300\n` | `phase_c_boundaries` |
| E2 | `x = INT_MAX` (`2147483647`) — `2*x` overflows signed `int` | wraps: prints `298\n`, no trap | `phase_c_boundaries` |
| E3 | `x = INT_MIN` (`-2147483648`) — `2*x` overflows signed `int` | wraps: prints `300\n`, no trap | `phase_c_boundaries` |
| E4 | `x = INT_MAX / 2` (`1073741823`) and `x = INT_MAX/2 + 1` — one step either side of the largest `x` whose `2*x` fits | prints `-2147483350\n` and `-2147483348\n` | `phase_c_boundaries` |
| E5 | `x = INT_MIN / 2` (`-1073741824`) and `x = INT_MIN/2 - 1` — one step past the smallest `x` whose `2*x` fits | prints `-2147483348\n` and `-2147483350\n` | `phase_c_boundaries` |
| E6 | `x` such that `2*x` fits but `2*x + 300` overflows (`x = 1073741524 … 1073741823`) | wraps in the `+= 300`: `x = 1073741524` prints `2147483348\n`, `x = 1073741823` prints `-2147483350\n` | `phase_c_boundaries`, `phase_b_row07_add_only_overflow_exhaustive` |
| E7 | out-of-range "enum"-style value across the FFI boundary: `driver` has no enum parameter, so the analogous hostile input is an arbitrary `int` with no distinguished meaning — every one of the 2^32 bit patterns, sampled exhaustively at the boundaries and randomly elsewhere | bytes match for every sampled value | `phase_c_boundaries`, `phase_b_randomized_full_range` |
| E8 | repeated / interleaved calls (state leakage between C and Rust, stdout buffering divergence) | each call prints exactly one line, same order | `phase_c_interleaved_calls` |

Note on E2/E3/E6: signed overflow is UB in ISO C, but the C library **as
built** is the ground truth, so the tests assert equality against whatever the
compiled C `.so` actually prints rather than against any assumed value. The
Rust translation uses `wrapping_mul`/`wrapping_add`, which matches the built C.

Null-pointer and oversized-length boundaries are inapplicable: the ABI passes
one 32-bit integer by value and there is no buffer, length, or pointer in the
entire public surface.
