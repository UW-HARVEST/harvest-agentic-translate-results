# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep evidence

```
$ grep -n "return\|assert\|NULL\|errno\|exit(\|abort\|RETURN_ERROR\|enum" c_src/src/driver.c c_src/include/driver.h
(no matches)

$ grep -n "if\|while" c_src/src/driver.c
30:    while (x > 0 || y > 0) {
33:        if (x == 1 && y == 4) {
38:        if (x > 0) {
44:        if (y == 0) {
49:        if (x < 3) {
```

The public API is a single function

```c
void driver(int x, int y);
```

It therefore has:

* **no error return value** (`void`),
* **no error enum / error codes / sentinels**,
* **no pointer parameters** ⇒ no null-pointer rejection possible,
* **no length / size parameters** ⇒ no zero/oversized-length rejection,
* **no enum parameters** ⇒ no out-of-range-enum-across-FFI case (both
  parameters are plain `int`; *all* 2^32 bit patterns are legal input and must
  be handled, which rows 1–13 below cover at the extremes),
* **no `assert`**, **no explicit range check**, **no `NULL` check**.

The complete rejection surface consists of the 5 guard conditions above: the
ways the function *refuses to do work* (returns immediately, or skips /
short-circuits a branch), plus the two conditions under which C itself has
undefined / unbounded behaviour. One row per distinct rejection/guard outcome.

`expected C result` = exact bytes written to `stdout` (`⌀` = zero bytes).

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|-------------------------------------------|-------------------|
| 1  | `driver` | `while (x > 0 \|\| y > 0)` false on entry: `x <= 0 && y <= 0`, e.g. `(0,0)` | returns immediately, `⌀` |
| 2  | `driver` | same guard, both operands negative: `(-1,-1)`, randomized negatives | returns immediately, `⌀` |
| 3  | `driver` | same guard at the most-negative boundary: `(INT_MIN, INT_MIN)` | returns immediately, `⌀`, no trap/overflow |
| 4  | `driver` | same guard, one operand exactly `0` (boundary of `> 0`): `(0,-1)`, `(-1,0)`, `(0,INT_MIN)`, `(INT_MIN,0)` | returns immediately, `⌀` |
| 5  | `driver` | `if (x > 0)` at `label1` false (`x == 0` / `x < 0`) ⇒ `"x\n"` suppressed and `x--` skipped: `(0,3)` | `loop\ny\ny\ny\n` (no `x\n`) |
| 6  | `driver` | `if (x > 0)` false with `x == INT_MIN` (most-negative, must not decrement) : `(INT_MIN,5)` | `loop\ny\ny\ny\ny\ny\n` |
| 7  | `driver` | `if (y == 0) continue` at `label2` ⇒ `"y\n"`/`y--` rejected, jump back to the `while` test: `(3,0)` | `loop\nx\nloop\nx\nloop\nx\n` |
| 8  | `driver` | `if (y == 0) continue` reached with `y` having *become* 0 inside the body (`x < 3` back-edge then `y == 0`): `(2,2)` | `loop\nx\ny\nx\ny\n` |
| 9  | `driver` | `if (x == 1 && y == 4)` true ⇒ `goto label2` *skips* the whole `label1` block (first `x\n` rejected) | `(1,4)` → `loop\ny\nx\ny\ny\ny\n` |
| 10 | `driver` | `if (x == 1 && y == 4)` short-circuit halves: `x == 1 && y != 4` (`(1,3)`, `(1,5)`), `x != 1 && y == 4` (`(0,4)`,`(2,4)`,`(4,4)`) ⇒ branch NOT taken | `(1,3)`→`loop\nx\ny\ny\ny\n`; `(2,4)`→`loop\nx\ny\nx\ny\ny\ny\n` |
| 11 | `driver` | `if (x < 3)` true ⇒ backward `goto label1` which does **not** re-test the `while` condition (no extra `loop\n`): `(1,1)`, `(2,5)` | `(1,1)`→`loop\nx\ny\n`; `(2,5)`→`loop\nx\ny\nx\ny\ny\ny\ny\n` |
| 12 | `driver` | `if (x < 3)` false at the exact boundary `x == 3` after the decrement (i.e. entry `x >= 4`) ⇒ back-edge rejected, `while` re-tested (extra `loop\n`): `(4,4)`, `(5,5)`, `(6,2)` | `(4,4)`→`loop\nx\ny\nloop\nx\ny\nx\ny\nx\ny\n` |
| 13 | `driver` | garbage/extreme `int` bit patterns passed across FFI (no validation exists): `(INT_MIN,0)`, `(-1,INT_MIN)`, `(INT_MIN,INT_MIN)`, `(0,INT_MIN)`, `(-2147483647, -2147483648)` | all satisfy row 1's guard ⇒ `⌀` |
| 14 | `driver` | **C UB / unbounded:** `y < 0 && x > 0` ⇒ `y--` runs from `y` down through `INT_MIN` (signed-overflow UB) for ≈2^31 iterations | *excluded from execution* — see below |
| 15 | `driver` | **unbounded runtime:** `x == INT_MAX` (or any huge `x > 0`) ⇒ ≈2^31 loop iterations | *bounded surrogate tested* — see below |
| 16 | `driver` | **write-error surface:** caller closed fd 1 ⇒ every `puts` fails (`EBADF`); nothing checks the return value, so the loop runs to completion regardless | no output; process exits normally (`fflush` reports failure); tested in all 4 buffering modes |
| 17 | `driver` | **write-error surface:** fd 1 is a pipe whose read end is closed ⇒ `EPIPE` / `SIGPIPE` on first flush | identical termination status from both libraries in all 4 buffering modes |
| 18 | `driver` | **unbounded runtime, `y` side:** `x <= 0 && y` huge (e.g. `y == INT_MAX`) ⇒ `y` drains one per back-edge pass, ≈2^31 iterations | capped-prefix comparison |
| 19 | `driver` | `x == 1 && y == 4` *arriving at `label1`/`label2` through the backward `goto`* — the special-case test sits **above** `label1`, so this state must NOT re-trigger `goto label2`. Reachable: entry `(2,5)` re-enters `label1` with exactly `x==1, y==4` | `(2,5)` → `loop\nx\ny\nx\ny\ny\ny\ny\n` (one `loop`, `x` printed normally) |

## Rows 14 and 15 — justification for surrogate coverage

* **Row 14** (`x > 0 && y < 0`): confirmed against the real C library — the
  loop prints `loop`, `x`, then `y` forever (`y` walks down to `INT_MIN`, where
  `y--` is *undefined behaviour* in C). Reaching termination needs ≈2^31
  iterations / ≈4 GB of stdout, so it cannot be executed inside the test
  budget, and its terminating value is C UB in any case. The Rust translation
  uses `wrapping_sub` for `y--`, i.e. exactly the two's-complement wraparound
  gcc emits here, so the two agree for every step that is actually observable.
  The test suite asserts the *observable prefix* of this case instead: with a
  hard cap on produced bytes, C and Rust must emit the identical prefix
  (`error_paths.rs::row14_negative_y_unbounded_prefix_matches`), which
  exercises the same `y < 0` code path without running to UB.
* **Row 15** (`x == INT_MAX`): the loop decrements `x` by at most 1 per pass, so
  the full run is ≈2^31 iterations. Tested with bounded surrogates
  (`x = 5_000`, `50_000`, `200_000`; `y = 0` and `y > 0`) which take the same
  code path, plus the `x == INT_MAX` *prefix* comparison under a byte cap
  (`error_paths.rs::row15_int_max_x_prefix_matches`).
* **Row 18** is the mirror image on the `y` side (`x <= 0 && y == INT_MAX`) and
  is handled the same way (`row18_huge_y_prefix_matches`). Both rows mean a
  drop-in consumer inherits the C library's effective hang — that is faithful
  behaviour, not a translation defect.

## Status

| row | test | status |
|-----|------|--------|
| 1 | `error_paths.rs::row01_loop_never_entered` | [x] |
| 2 | `error_paths.rs::row02_both_negative_random` | [x] |
| 3 | `error_paths.rs::row03_int_min_pair` | [x] |
| 4 | `error_paths.rs::row04_zero_boundary_of_gt0` | [x] |
| 5 | `error_paths.rs::row05_label1_x_not_positive` | [x] |
| 6 | `error_paths.rs::row06_label1_x_int_min` | [x] |
| 7 | `error_paths.rs::row07_label2_y_zero_continue` | [x] |
| 8 | `error_paths.rs::row08_label2_y_became_zero` | [x] |
| 9 | `error_paths.rs::row09_x1_y4_goto_label2` | [x] |
| 10 | `error_paths.rs::row10_x1_y4_short_circuit_halves` | [x] |
| 11 | `error_paths.rs::row11_backward_goto_taken` | [x] |
| 12 | `error_paths.rs::row12_backward_goto_boundary_x3` | [x] |
| 13 | `error_paths.rs::row13_extreme_int_bit_patterns` | [x] |
| 14 | `error_paths.rs::row14_negative_y_unbounded_prefix_matches` | [x] (bounded prefix) |
| 15 | `error_paths.rs::row15_int_max_x_prefix_matches` + `row15_large_x_surrogates` | [x] (bounded prefix + surrogates) |
| 16 | `error_paths.rs::row16_stdout_closed` | [x] |
| 17 | `error_paths.rs::row17_broken_pipe` | [x] |
| 18 | `error_paths.rs::row18_huge_y_prefix_matches` | [x] (bounded prefix) |
| 19 | `error_paths.rs::row19_special_case_state_reached_via_back_edge` | [x] |

Plus `error_paths.rs::generic_boundary_sweep`, which sweeps the generic C-API
boundaries required regardless of the table: every pair from
`{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, 3, 4, 5, 6}` — i.e. one step past each
implicit valid-range boundary on both parameters. There are no pointer or enum
parameters, so null pointers and out-of-range enum values cannot be constructed;
the equivalent for this API (arbitrary 32-bit patterns with no "valid variant")
is row 13.
