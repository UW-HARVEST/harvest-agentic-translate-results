# ERRORS.md — error / rejection surface table (Phase C gate)

## How this table was derived

`c_src` is a single 53-line translation unit. Mechanical greps for every
rejection idiom:

```sh
grep -nE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR' c_src/src/driver.c c_src/include/driver.h
grep -nE '<=|>=|<|>|==|!=' c_src/src/driver.c
grep -nE '#(if|ifdef|ifndef|define|else|elif)' c_src/src/driver.c c_src/include/driver.h
```

Findings, verbatim:

* `return` — **0 occurrences**
* `assert` — **0 occurrences**
* `NULL` — **0 occurrences**
* `errno` / `exit(` / `abort` / `RETURN_ERROR` / error enums — **0 occurrences**
* comparison/guard sites — exactly 5, at `src/driver.c:30,33,38,44,49`
* preprocessor conditionals — only the `DRIVER_H_` include guard

So `driver` has **no error-return channel at all**: its signature is
`void driver(int x, int y)`, it takes no pointers, no lengths, no enums, and no
buffers. There is nothing it can validate and nothing it can report. The
"rejection" surface therefore consists of the guard conditions under which the C
declines to do work (the loop guard and the two in-body skip guards), plus the
one input class on which the C never returns. Each is one row below, and each
row's expected result is stated as the C's *only* observable: the exact bytes
written to `stdout` (and whether the call returns at all).

The generic C-API boundaries requested by the protocol are recorded as rows
too, with their `N/A` justification stated explicitly rather than omitted.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `driver` | `src/driver.c:30` loop guard `x > 0 \|\| y > 0` false on entry: `x == 0 && y == 0` | returns immediately, writes **0 bytes** | `err_row_01_guard_zero_zero` | [x] |
| 2 | `driver` | guard false on entry, both strictly negative: `x < 0 && y < 0` (randomized) | returns immediately, 0 bytes | `err_row_02_guard_both_negative` | [x] |
| 3 | `driver` | guard false on entry, `x < 0 && y == 0` (randomized) | returns immediately, 0 bytes | `err_row_03_guard_negx_zeroy` | [x] |
| 4 | `driver` | guard false on entry, `x == 0 && y < 0` (randomized) | returns immediately, 0 bytes | `err_row_04_guard_zerox_negy` | [x] |
| 5 | `driver` | guard false at the extreme boundary: `x == INT_MIN && y == INT_MIN` | returns immediately, 0 bytes | `err_row_05_guard_int_min_both` | [x] |
| 6 | `driver` | guard false, mixed extremes: `(INT_MIN, 0)`, `(0, INT_MIN)`, `(INT_MIN, -1)`, `(-1, INT_MIN)` | returns immediately, 0 bytes | `err_row_06_guard_extreme_mixed` | [x] |
| 7 | `driver` | `src/driver.c:38` skip guard `x > 0` false at `label1` while the loop still runs (`x <= 0 && y > 0`) | `label1` block suppressed: no `"x\n"` is ever emitted; output is `"loop\n"` then `y` `"y\n"` lines | `err_row_07_skip_label1_x_not_positive` | [x] |
| 8 | `driver` | `src/driver.c:44` rejection `if (y == 0) continue;` reached with the loop still running (`x > 0 && y == 0`) | `y`-block suppressed every pass: no `"y\n"` is ever emitted; output is `x` copies of `"loop\nx\n"` | `err_row_08_reject_y_zero_continue` | [x] |
| 9 | `driver` | `src/driver.c:44` `continue` taken on a *later* pass, after `y` has been drained to 0 by the body (`x > 0 && y > 0`) | the `continue` fires on the pass where `y` first reaches 0; byte-exact stream must match | `err_row_09_reject_y_zero_after_drain` | [x] |
| 10 | `driver` | `src/driver.c:49` back-edge guard `x < 3` false (`x >= 3 && y > 0`) — `goto label1` declined, control falls to the `while` re-test | no intra-iteration replay; each outer pass emits one `"loop\n"` | `err_row_10_no_backedge_x_ge_3` | [x] |
| 11 | `driver` | `src/driver.c:33` special-case guard `x == 1 && y == 4` — forward `goto label2` skips `label1` exactly once | first pass emits `"loop\n"` then `"y\n"` with **no** `"x\n"`; subsequent passes do not re-skip | `err_row_11_goto_label2_skip_once` | [x] |
| 12 | `driver` | `x > 0 && y < 0`: `y` is decremented forever at `src/driver.c:47`. The C **never returns** (and signed-overflow UB past `INT_MIN`). This is the one input class with no return at all. | infinite loop; unbounded `"loop\n"/"x\n"/"y\n"` stream. Rust must diverge identically, with a byte-identical output prefix. | `err_row_12_nonterminating_x_pos_y_neg` (forked subprocess, byte-compares a 16 KiB stdout prefix and asserts both children hang) | [x] |
| 13 | `driver` | null-pointer arguments | **N/A** — `void driver(int, int)` has no pointer parameter; there is no pointer to make null. Nearest analogue covered by rows 5–6 (extreme scalar values). | — | [x] |
| 14 | `driver` | zero-length / oversized length arguments | **N/A** — no length or count parameter exists. Nearest analogue: `0` (rows 1–4) and `INT_MIN`/`INT_MAX` (rows 5–6, and `CONFIGS.md` rows 20–21). | — | [x] |
| 15 | `driver` | out-of-range enum value crossing the FFI boundary | **N/A** — the API declares no enum and no struct; both parameters are plain `int`, so *every* `int` bit pattern is an in-range value. The full `int` domain is instead partitioned across `CONFIGS.md` (valid, terminating) and row 12 (non-terminating). | — | [x] |
| 16 | `driver` | value one step past a documented valid range | **N/A as a rejection** — `driver.h` documents no range; the C accepts all `int`s. The one-step-past-boundary inputs the code actually branches on (`x` = -1/0/1/2/3/4, `y` = -1/0/1/3/4/5) are covered as `CONFIGS.md` rows 7–19 and rows 1–11 here. | — | [x] |
| 17 | `driver` | non-zero return / error code | **N/A** — return type is `void`; there is no error code to compare. The differential assertion is therefore on the stdout byte stream, which is the complete observable behaviour of this function. | — | [x] |

## Notes

* Rows 1–11 are exercised with **randomized** inputs drawn from the region each
  row describes (fixed seed), not single hand-picked values.
* Row 12 is the only row where the C does not return. It is verified by forking,
  redirecting the child's `stdout` to a file, letting both implementations run,
  byte-comparing the first 16 KiB each produced, and asserting that **both**
  children are still alive (i.e. both diverge) before they are killed. Asserting
  "both hang" plus "identical output prefix" is the strongest available
  equivalence for a function that never returns.

## Harness and result

`tests/phase_c_errors.rs` — 14 executing tests covering rows 1–17. Same
`libloading`-only discipline as Phase B (see `CONFIGS.md`): both `.so`s are
loaded and called through their exported `driver` symbol.

Rows 7, 8, 10 and 11 do not merely compare bytes; they additionally assert the
*shape* the rejection must produce (no `"x"` line when `label1` is suppressed,
no `"y"` line when the `y == 0` `continue` fires, `x == 3` differing from
`x == 2` at the S5 boundary, and the forward `goto` skipping `label1` exactly
once), so a Rust translation that happened to match a wrong C reading would
still be caught.

Row 12 works by re-executing the test binary as a child process
(`hang_child_worker`, `#[ignore]`d so it never runs on its own), pointing the
child's fd 1 at a file, waiting for 16 KiB of output, asserting the child is
**still alive**, then killing it — for each of `(1,-1)`, `(2,-1)`, `(3,-1)`,
`(4,-7)`, `(9,-3)` and `(1, INT_MIN+1)`. Both implementations must diverge *and*
agree on the 16 KiB prefix. `./mutation_check.sh` confirms this row is load-
bearing: a mutation that makes the Rust return early on `x > 0 && y < 0` is
detected only here.

All 14 tests pass against both the debug and the release Rust `.so`.
