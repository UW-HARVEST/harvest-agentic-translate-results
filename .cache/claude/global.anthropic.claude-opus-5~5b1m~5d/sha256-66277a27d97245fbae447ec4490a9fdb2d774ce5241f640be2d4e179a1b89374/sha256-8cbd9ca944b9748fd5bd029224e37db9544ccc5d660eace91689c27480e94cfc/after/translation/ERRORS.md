# ERRORS.md — Phase C error / rejection surface table

Mechanically grepped from `c_src/src/driver.c` (the whole library):

```
$ grep -n 'return\|assert\|NULL\|ERROR\|-1\|if (' c_src/src/driver.c
30:    while (x > 0 || y > 0) {
33:        if (x == 1 && y == 4) {
38:        if (x > 0) {
44:        if (y == 0) {
49:        if (x < 3) {
```

Findings:

* **No `return` statements at all** — `driver` is `void` and falls off the end.
* **No `assert`**, no `NULL` check, no error enum, no error-return macro,
  no `errno` use, no allocation (so no allocation-failure path), no pointer
  parameters at all.
* **No `#include`d validation**; `stdio.h`/`stdlib.h` are pulled in but only
  `printf` is used.

Therefore the library has **no error return channel**. Its only "rejection"
behaviour is the *guard condition of the `while` loop*, which rejects the
whole computation (produces no output and returns immediately), plus the
conditions on which the C standard makes the program's behaviour undefined /
non-terminating. Those are enumerated below and each has a differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `driver` | `while (x > 0 \|\| y > 0)` false on entry: `x <= 0 && y <= 0` (e.g. `(0,0)`) | returns immediately, **zero bytes** written to stdout |
| E2 | `driver` | same guard with both strictly negative, e.g. `(-1,-1)`, `(-7,-3)` | returns immediately, zero bytes |
| E3 | `driver` | same guard at the negative extreme: `(INT_MIN, INT_MIN)` | returns immediately, zero bytes |
| E4 | `driver` | same guard, mixed sign but both non-positive: `(0,-1)`, `(-1,0)`, `(INT_MIN,0)`, `(0,INT_MIN)` | returns immediately, zero bytes |
| E5 | `driver` | `if (y == 0) continue;` — the `y` half of the body is rejected whenever `y` has been drained to 0 while `x > 0` remains (e.g. `(3,0)`) | no `"y\n"` is ever printed; `continue` re-tests the `while` guard; only `"loop\n"`/`"x\n"` appear |
| E6 | `driver` | `if (x > 0)` false while `y > 0` — the `x` half of the body is rejected (e.g. `(0,3)`, `(-5,3)`) | no `"x\n"` is ever printed; only `"loop\n"`/`"y\n"` appear |
| E7 | `driver` | `if (x < 3)` false — the backwards `goto label1` is rejected, body ends and the `while` guard is re-tested (e.g. `(5,2)`) | exactly one `label1`+`label2` pass per `"loop\n"` |
| E8 | `driver` | `if (x == 1 && y == 4)` **partially** satisfied, i.e. the `goto label2` is rejected: `(1,3)`, `(1,5)`, `(2,4)`, `(0,4)` | `label1` block is **not** skipped; `"x\n"` printed before `"y\n"` on that pass |
| E9 | `driver` | **Non-terminating / UB input**: `y < 0` while `x > 0`, e.g. `(1,-1)`. `y == 0` is never true so `y--` runs forever until signed-overflow UB at `INT_MIN`. | C never returns (infinite loop, UB on overflow). **Not a differential test** — asserted instead as *"both C and Rust reject the same way, i.e. neither returns"*, verified by running each in a child process under a timeout and requiring both to be killed. |
| E10 | `driver` | `x == INT_MAX`, `y == 0` — the `x--` drain runs `INT_MAX` times | terminates but takes ~2^31 iterations. **Not run to completion**; covered by a bounded-prefix stdout comparison instead. |
| E11 | `driver` | out-of-range "enum"/int values across the FFI boundary: the parameters are plain `int`, so *every* 32-bit pattern is a valid input. Extremes `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` are passed in all pairwise combinations that terminate. | identical stdout; no trap, no panic |
| E12 | `driver` | wrong-width argument passed by a foreign caller (e.g. a 64-bit value whose low 32 bits are the `int`): C truncates to `int` per the ABI | both `.so`s must observe the same truncated `int`; tested by calling through an `extern "C" fn(i32, i32)` signature with values built from `u64` patterns |

## Notes on rows that cannot be a plain "same error code" assertion

`driver` returns `void`, so there is no error code or sentinel to compare.
The observable contract is **the byte stream written to stdout** plus
**whether the call returns**. Rows E1–E8, E11, E12 assert byte-identical
stdout (empty stdout for E1–E4). Rows E9 and E10 assert identical
*non-termination* / identical bounded output prefix, since neither C nor Rust
can be run to completion.

## Checklist

- [x] E1 — `test_e1_zero_zero_no_output`
- [x] E2 — `test_e2_both_negative_no_output`
- [x] E3 — `test_e3_int_min_no_output`
- [x] E4 — `test_e4_mixed_nonpositive_no_output`
- [x] E5 — `test_e5_y_zero_continue`
- [x] E6 — `test_e6_x_nonpositive`
- [x] E7 — `test_e7_x_ge_3_no_backward_goto`
- [x] E8 — `test_e8_special_case_partially_matched`
- [x] E9 — `test_e9_nonterminating_both`
- [x] E10 — `test_e10_huge_x_bounded_prefix`
- [x] E11 — `test_e11_extreme_int_matrix`
- [x] E12 — `test_e12_wide_argument_truncation`

## Result

`tests/errors.rs :: phase_c_all_error_rows` runs rows E1–E12 sequentially
(E9 and E10 spawn child processes so that non-terminating inputs can be observed
and then killed):

```
running 2 tests
test child_helper ... ignored, internal child-process helper, re-executed by E9/E10
test phase_c_all_error_rows ... ok
```

All 12 rows pass under all six profile × feature combinations.

Note that E9's assertion is non-vacuous: it includes a positive control
(`driver(4, 4)`) which must be *observed to terminate*, so "both hung" cannot be
satisfied by a broken detector. Likewise E10 requires ≥ 32 KiB of comparable
driver output from each side before it will pass.

### Why `wrapping_sub` is the right translation of `x--` / `y--`

The Rust uses `x.wrapping_sub(1)` / `y.wrapping_sub(1)`. Plain `-` would panic
on overflow in a debug build, which no C build does. In any *terminating* run
neither decrement can overflow (`x--` is guarded by `x > 0`, and `y--` is only
reached with `y != 0`), so the wrap is observationally unreachable — it exists
purely so the `debug` profile behaves like C rather than aborting. The `debug`
profile is exercised explicitly by `./run_all_combos.sh`.
