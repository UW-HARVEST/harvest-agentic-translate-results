# ERRORS.md — Phase C error-surface table

Derived mechanically from the complete text of `c_src/src/driver.c` and
`c_src/include/driver.h`.

## Mechanical grep results

```sh
grep -nE 'return|RETURN_ERROR|assert|NULL|errno|-1|if|switch|<|>|MAX|MIN' c_src/src/driver.c
```

The entire function body is:

```c
void driver(int x, int y) {
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
}
```

Findings, stated exactly:

* return type is `void` → there is **no error return value, no sentinel, no
  errno use, and no error enum** anywhere in the library;
* there are **no** `assert`s, **no** `if`/`switch` branches, **no** explicit
  range checks, **no** null checks (the API takes no pointers), and **no**
  min/max constants in the source;
* there are **no** `#ifdef`s.

Consequently the library's only rejection mechanism is the *implicit* one
inherited from the callee `div(3)`: on x86-64, glibc's `div` performs `idivl`,
which raises the `#DE` (divide-error) fault for the two cases the x86 `idiv`
instruction cannot represent. The process is killed by `SIGFPE` (signal 8) and
nothing is printed. That is the "expected C result" for those rows, and the Rust
must reproduce it — including printing nothing.

Both trapping cases were confirmed empirically with a standalone C probe
(`scratch/probe.c`): exit status `136` == `128 + 8` == killed by `SIGFPE`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `driver` | `y == 0` with `x == 0` | killed by `SIGFPE` (signal 8), no stdout output | `trap_row1_zero_over_zero` | [x] |
| 2 | `driver` | `y == 0` with `x != 0` (swept over positive, negative, `INT_MAX`, `INT_MIN`, and random `x`) | killed by `SIGFPE` (signal 8), no stdout output | `trap_row2_nonzero_over_zero` | [x] |
| 3 | `driver` | `x == INT_MIN && y == -1` (signed-overflow quotient `2^31`, unrepresentable in `int`) | killed by `SIGFPE` (signal 8), no stdout output | `trap_row3_int_min_over_minus_one` | [x] |

Rows 1 and 2 are kept distinct because `0 / 0` and `k / 0` are separately
reachable conditions in the `idiv` fault logic (`0/0` is the degenerate case
where the numerator is also invalid as a limit), and a naive Rust translation
using `checked_div` would collapse them differently (`None` for both) or would
special-case `0` — so they are worth separate assertions.

## Generic FFI boundary cases (required even though absent from the table)

| # | condition | expected C result | test | status |
|---|-----------|-------------------|------|--------|
| G1 | no pointer arguments exist in the ABI, so null-pointer inputs are unrepresentable | n/a — documented as not applicable | `abi_no_pointer_arguments` | [x] |
| G2 | no length/size arguments exist, so zero and oversized lengths are unrepresentable | n/a — documented as not applicable | `abi_no_pointer_arguments` | [x] |
| G3 | no enum arguments exist, so out-of-range enum values are unrepresentable | n/a — documented as not applicable | `abi_no_pointer_arguments` | [x] |
| G4 | "one step past the valid range" for the two `int` parameters — every value is valid for `int`, so the boundaries are the type's own extremes: `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1` in both parameter positions | normal formatted output (except the row-3 combination) | `boundary_extremes_cross_product` | [x] |
| G5 | garbage in the high 32 bits of the 64-bit argument registers (a real hazard: the SysV ABI leaves the upper half of `rdi`/`rsi` undefined for `int` parameters, so the callee must ignore it) | both libraries must truncate to the low 32 bits identically | `abi_high_garbage_bits_ignored` | [x] |

G4 and G5 are exercised in `tests/differential.rs`; rows 1–3 are exercised in
`tests/trap.rs` (each in a fresh child process, since a `SIGFPE` cannot be
survived in-process).
