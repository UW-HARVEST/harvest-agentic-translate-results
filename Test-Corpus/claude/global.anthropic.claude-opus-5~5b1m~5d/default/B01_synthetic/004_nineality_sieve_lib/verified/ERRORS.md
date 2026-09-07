# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection/error construct was grepped out of the *entire* C source
(`c_src/src/sieve.c`, `c_src/include/sieve.h`):

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|RETURN_ERROR|goto|\berr' \
      c_src/src/sieve.c c_src/include/sieve.h | grep -v '^\S*:[0-9]*: *[/*]'
include/sieve.h:24:#ifndef SIEVE_H_          <- include guard, not an error path
src/sieve.c:24:#include <stdio.h>            <- matched "include", not an error path

$ grep -nE 'if *\(|else|switch|case|\?' c_src/src/sieve.c | grep -v ': *[/*]'
src/sieve.c:35:        if (val % 10 == 9) {   <- the ONLY branch in the library
```

Findings, stated exactly as the C code is written:

* `sieve` has return type `void` — there is **no error code, no sentinel, and no
  out-parameter** through which failure could be reported.
* There are **zero** `return` statements (not even a bare `return;`), zero
  `assert`s, zero `NULL` checks, zero `errno` uses, zero range checks, zero
  min/max constants, zero enums, and zero `#ifdef` branches.
* `sieve` takes a single **by-value `int`** — there is no pointer parameter, so a
  "null pointer" argument is not representable; the closest analogue is the
  integer `0`, which is a perfectly valid input (row 3).
* Consequently the library **rejects nothing**. Every one of the 2^32 `int`
  values is accepted and drives the loop. The error surface is therefore not
  "empty because we did not look" — it is empty *by construction*, and the rows
  below record the boundary/degenerate conditions that stand in for it, i.e. the
  inputs where the C code's behaviour is surprising, unbounded, or
  undefined-in-C. Each row still gets a differential test asserting C and Rust
  agree on the *same* observable result (identical stdout bytes / identical
  non-crash), not merely "both did something".

The `int` parameter is 4-byte two's-complement on this target
(`sizeof(int) == 4`, verified by the harness), so the "one step past the valid
range" boundaries are `INT_MIN`/`INT_MAX` wraparound, covered by rows 7–10.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `sieve` | No error-return construct exists anywhere in the library (grep above): return type is `void`, zero `return`/`assert`/`NULL`/range checks | Nothing can be rejected; C never signals failure. Asserted by verifying `sieve` returns normally (no crash, no abort) and produces the same stdout as Rust for every input class in rows 2–14 | `err_row01_no_rejection_path_exists` | [x] |
| 2 | `sieve` | `val` whose last base-10 digit **is** 9 and is positive (`val % 10 == 9` true on entry) — the degenerate single-iteration case, loop body runs exactly once and `val++` never executes | prints `"<val>\n"` and returns; exactly 1 line | `err_row02_immediate_break_positive` | [x] |
| 3 | `sieve` | `val == 0` (the closest representable analogue of a "null" argument, and the zero-length/zero-value boundary) | `0 % 10 == 0 != 9`, so prints `0\n1\n...\n9\n`; 10 lines | `err_row03_zero_argument` | [x] |
| 4 | `sieve` | `val < 0` — C's `%` truncates toward zero, so `val % 10` is in `{0,-1,...,-9}` and can **never** equal 9. The `== 9` test is unreachable while negative; the loop cannot terminate early | counts all the way up through 0 to 9: prints `val..9`, i.e. `-val + 10` lines. **Not** an early exit | `err_row04_negative_never_matches` | [x] |
| 5 | `sieve` | `val == -9` — the trap case: `-9 % 10 == -9`, **not** `9`, so it does *not* break despite "ending in 9" | prints `-9\n-8\n...\n9\n`; 19 lines (not 1) | `err_row05_negative_nine_does_not_break` | [x] |
| 6 | `sieve` | `val == -19, -29, -99, -109` — every other negative "ends in 9" value; `val % 10 == -9 != 9` | never breaks early; runs up to 9 | `err_row05_negative_nine_does_not_break` | [x] |
| 7 | `sieve` | `val == INT_MAX` (`2147483647`, one step past the largest value the loop can legally increment). `2147483647 % 10 == 7 != 9`, so C executes `val++` on `INT_MAX` → **signed integer overflow, undefined behaviour in C** | The compiled C (`-O0`, no `-fwrapv` needed at this opt level) wraps to `INT_MIN` and keeps counting; output is effectively unbounded (~2^32 lines). Asserted by comparing a bounded 256 KiB stdout prefix from a forked child, C vs Rust | `err_row07_int_max_overflow_prefix` | [x] |
| 8 | `sieve` | `val` in `[INT_MAX-7, INT_MAX] = [2147483640, 2147483647]` — the full set of starting values from which no value ending in 9 is reachable before overflow | all 8 overflow exactly as row 7 | `err_row08_overflow_window_all_eight` | [x] |
| 9 | `sieve` | `val == INT_MAX - 8 == 2147483639` — one step *inside* the valid range; `% 10 == 9`, so it breaks instead of overflowing (boundary of row 8) | prints one line `2147483639\n`, no overflow | `err_row09_last_safe_value` | [x] |
| 10 | `sieve` | `val == INT_MIN` (`-2147483648`, the smallest representable argument). `INT_MIN % 10 == -8 != 9` | counts up from `INT_MIN` to 9: ~2.1 billion lines, unbounded for practical purposes. Asserted via bounded 256 KiB prefix, C vs Rust | `err_row10_int_min_prefix` | [x] |
| 11 | `sieve` | `val == INT_MIN + 1 .. INT_MIN + 3` — just inside the low boundary | same unbounded upward count; bounded-prefix comparison | `err_row11_near_int_min_prefix` | [x] |
| 12 | `sieve` | Argument passed as an **out-of-range "enum-like" int**: the API takes a bare `int`, so a C caller may legally pass any `int32` bit pattern including ones no sane caller would use (`0x80000000`, `0x7FFFFFFF`, `-1`, `0xDEADBEEF as i32`, `i32::MIN/2`). C has no variant check and must not reject them | each is treated as an ordinary integer start value; identical stdout from C and Rust (bounded prefix where the output is unbounded) | `err_row12_arbitrary_bit_patterns` | [x] |
| 13 | `sieve` | Called **repeatedly / re-entrantly** with no init or teardown function (there is no state to corrupt: no globals, no allocation) — a caller "misusing" the API by calling it many times in a row | each call is independent; concatenated stdout matches C's | `err_row13_repeated_calls_no_state` | [x] |
| 14 | `sieve` | Called while stdout is a **closed / non-writable fd** — `printf` fails and returns negative; the C code ignores the return value, so the loop must still terminate normally rather than hang or crash | returns normally, produces no output, no crash; Rust identical | `err_row14_stdout_write_failure` | [x] |

## Not applicable (recorded so the absence is deliberate, not an oversight)

| construct | why absent |
|-----------|------------|
| null-pointer argument | `sieve` has no pointer parameter (`void sieve(int)`) |
| zero / oversized length argument | `sieve` has no length or size parameter |
| out-of-range enum variant | the library declares no `enum`; row 12 covers the bare-`int` equivalent |
| error code / sentinel return | return type is `void`; nothing to compare but stdout + termination |
| allocation failure | the library never allocates |
