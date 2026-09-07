# ERRORS.md — error / rejection surface table (Phase A, gated in Phase C)

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep evidence

```
$ grep -nE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR' c_src/src/driver.c c_src/include/driver.h
(no matches)
```

The complete set of conditionals in the C source is:

```
src/driver.c:30:    while (x > 0 || y > 0) {
src/driver.c:33:        if (x == 1 && y == 4) {   -> goto label2
src/driver.c:38:        if (x > 0) {
src/driver.c:44:        if (y == 0) {             -> continue
src/driver.c:49:        if (x < 3) {              -> goto label1
```

Consequences for the error surface:

* `driver` returns `void`. There is **no** error code, sentinel, `errno` write,
  `assert`, `abort`, or `exit` anywhere in the library — so there are no
  error-return rows in the classic sense. Equality of "error result" is
  therefore asserted as: *both implementations return normally and emit the
  identical (possibly empty) byte stream on `stdout`*.
* Both parameters are by-value `int`. There are **no pointer parameters**, so
  null-pointer rows are not applicable (there is no pointer to pass).
* There are **no `enum` types, no length/size parameters, and no arrays or
  buffers**, so out-of-range-enum and oversized-length rows are not applicable.
  The generic-boundary obligation is instead discharged over the full `int`
  domain: `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX` on each parameter, plus the
  one-step-past values around every constant the code compares against
  (`0`, `1`, `3`, `4`).
* The only *rejection* the C performs is the loop-entry guard
  `while (x > 0 || y > 0)`: when it is false the function returns immediately
  having produced **no output at all**. Rows 1–6 enumerate the distinct value
  classes that make this guard reject.
* Rows 12–14 are inputs on which the C **does not terminate** (and, in doing
  so, signed-overflows `y--`, i.e. C undefined behaviour). They are recorded
  because they are part of the mechanically-derived surface, and are excluded
  from execution — a differential test cannot call a function that never
  returns. The Rust translation reproduces the same non-terminating shape
  (`y = y.wrapping_sub(1)`), which is what the C compiler actually emits.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `driver` | guard `x > 0 \|\| y > 0` false via `x == 0, y == 0` | returns immediately, **zero bytes** of output | [x] |
| 2 | `driver` | guard false via `x == 0, y < 0` (`y == -1`) | returns immediately, zero bytes | [x] |
| 3 | `driver` | guard false via `x < 0, y == 0` (`x == -1`) | returns immediately, zero bytes | [x] |
| 4 | `driver` | guard false via `x < 0, y < 0` (`x == -1, y == -1`) | returns immediately, zero bytes | [x] |
| 5 | `driver` | guard false at extreme lower boundary `x == INT_MIN, y == INT_MIN` | returns immediately, zero bytes | [x] |
| 6 | `driver` | guard false, mixed extreme `x == INT_MIN, y == 0` / `x == 0, y == INT_MIN` | returns immediately, zero bytes | [x] |
| 7 | `driver` | one step past the rejecting range on `x` only: `x == 1, y == 0` (guard passes on `x`; `y == 0` forces the `continue` branch every iteration) | enters loop, prints `loop\nx\n`, then exits; no `y` line ever | [x] |
| 8 | `driver` | one step past the rejecting range on `y` only: `x == 0, y == 1` (guard passes on `y`; `if (x > 0)` at `label1` is never taken) | enters loop, prints `loop\ny\n`, then exits; no `x` line ever | [x] |
| 9 | `driver` | `x == INT_MIN, y > 0` — most-negative `x` with a live `y`; `x--` is never reached so no underflow, `x < 3` is always true so the `goto label1` back-edge is taken every time | terminates, emits `loop\n` once followed by `y\n` repeated `y` times | [x] |
| 10 | `driver` | `y < 0` with `x <= 0` (e.g. `x == 0, y == INT_MIN`) — negative `y` that is nevertheless rejected by the guard before any decrement | returns immediately, zero bytes (no overflow occurs) | [x] |
| 11 | `driver` | degenerate/extreme-but-terminating magnitudes on the accepted side: `x == 0, y == 4096`; `x == 4096, y == 0`; `x == 4096, y == 4096` | terminates; both implementations must emit identical byte streams | [x] |
| 12 | `driver` | **non-terminating**: `x == 1, y == -1` (guard passes on `x`; `y != 0` so `y--` runs forever, signed-overflowing `y` = C UB) | never returns | n/a — excluded, cannot be differentially executed |
| 13 | `driver` | **non-terminating**: `x > 0, y < 0` generally (e.g. `x == 5, y == -1`; `x == INT_MAX, y == INT_MIN`) | never returns | n/a — excluded, cannot be differentially executed |
| 14 | `driver` | **impractical**: `x == INT_MAX, y == 0` / `y == INT_MAX` — terminates but emits O(2^31) lines | terminates after ~2^31 lines | n/a — excluded for runtime; covered by bounded row 11 |

Rows 1–11 are executed by `translation/tests/differential.rs`
(`phase_c_error_surface_rows_1_through_11`, plus the shared boundary sweep in
`phase_c_generic_int_boundaries`). Rows 12–14 are recorded as excluded above.

## Phase C result (evidence for the checkmarks above)

```
phase C: ERRORS.md rows 1-11 all matched
phase C boundaries: 134 executed, 40 non-terminating (ERRORS rows 12-13), 22 impractical (row 14)
```

The boundary sweep is the full cross-product of
`{INT_MIN, INT_MIN+1, -4097, -2, -1, 0, 1, 2, 3, 4, 5, 6, 4096, INT_MAX}` on
both parameters; the 40 + 22 skips are exactly rows 12–14 above, and the skip
predicate itself is asserted sound by
`phase_c_nontermination_predicate_is_sound`.
