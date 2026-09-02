# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/sieve.c` and `c_src/include/sieve.h`. The
grep used to enumerate every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR|if *\(|switch|#if' \
     c_src/src/sieve.c c_src/include/sieve.h
```

Matches found in the whole library:

* `#include <stdio.h>`               — not a check
* `if (val % 10 == 9) { break; }`    — the loop's *termination* test, not a rejection
* `#ifndef SIEVE_H_`                 — include guard

**The C library contains no error surface at all.** There is:

* no return value (`void sieve(int)`), so no error code or sentinel can be
  produced;
* no `assert`, no `errno` write, no `exit`/`abort`;
* no pointer parameter, therefore no null check to replicate;
* no range check, no min/max constant, no enum parameter;
* exactly one `if`, and it is the termination condition, not a rejection.

Consequently every `int` is a **valid** input: the function always runs and
always returns (see row 3 for the one input class where "returns" depends on
signed-overflow wraparound). The rows below are therefore the *generic C-API
boundaries* the task requires be covered anyway, expressed as
"expected C result" instead of "expected error", because rejection is not
expressible in this ABI. Each row is asserted differentially against both
`.so`s in `translation/tests/error_paths.rs`.

| # | function | trigger (the exact invalid/boundary input or condition) | expected C result | status |
|---|----------|----------------------------------------------------------|-------------------|--------|
| 1 | `sieve` | `val = 0` — zero-length/identity boundary, the additive identity and the smallest non-negative input | No rejection. Prints `0\n1\n…\n9\n` and returns. Exit is normal. | [x] |
| 2 | `sieve` | `val = INT_MIN` (`-2147483648`) — one step past the low end of the parameter's representable range | No rejection. `INT_MIN % 10 == -8` (C truncating remainder ⇒ never `9`), so it counts **up** through 0 to 9 and returns after 2147483658 lines. Verified as a bounded output *prefix* (see row 3 method), not to completion. | [x] |
| 3 | `sieve` | `val = INT_MAX` (`2147483647`) — the high end; `2147483647 % 10 == 7`, so `val++` **signed-overflows**, which is UB in C | No rejection, no trap: the C compiled at `-O0` wraps to `INT_MIN`, so the printed sequence is `2147483647`, `-2147483648`, `-2147483647`, … The Rust uses `wrapping_add` to reproduce exactly this. Asserted on a bounded stdout prefix from a forked child (the full run would emit ~23 GB). | [x] |
| 4 | `sieve` | `val = INT_MAX - 2 = 2147483645`, `INT_MAX - 1`, i.e. values one step *before* the overflow point | No rejection. Same wrapping continuation as row 3; the pre-overflow lines are printed first. | [x] |
| 5 | `sieve` | `val = -9` — a negative value whose *magnitude* ends in 9, the trap for anyone who "fixed" the remainder to Euclidean | No rejection and **no early break**: `-9 % 10 == -9 ≠ 9`, so it prints `-9 … 9` (19 lines). A Euclidean-remainder translation would print only `-9`. | [x] |
| 6 | `sieve` | `val = -1` — the classic error sentinel value, passed as data | No rejection. `-1 % 10 == -1`, prints `-1 … 9` (11 lines). | [x] |
| 7 | `sieve` | `val = 9` — the immediate-termination boundary (loop body runs exactly once) | No rejection. Prints `9\n` only; the print happens *before* the check, so the terminating value is still emitted. | [x] |
| 8 | `sieve` | `val = 19`, `10`, `8` — one step past / before the terminating residue, checking `% 10` is on the value not the iteration count | No rejection. `19` ⇒ one line; `10` ⇒ `10…19`; `8` ⇒ `8`,`9`. | [x] |
| 9 | `sieve` | Out-of-range "enum-like" ints: the ABI takes a bare `int`, so there is no enum with a finite variant set. Values with no semantic meaning (`0x7FFFFFFF`, `0x80000000` reinterpreted, `-2147483647`, `12345678`, random 32-bit words) are all passed across the FFI boundary as-is | No rejection. C treats every bit pattern as a plain `int` and applies the same `% 10` rule; there is no variant check to diverge on. Covered by randomized 32-bit inputs. | [x] |
| 10 | `sieve` | Truncation/width boundary: a value passed in a 64-bit register whose upper 32 bits are set (caller passes `i64`, callee reads `int`) | No rejection. Only the low 32 bits are read, per the SysV AMD64 ABI. Both `.so`s must agree; asserted by calling through an `extern "C" fn(i64)` signature. | [x] |
| 11 | `sieve` | `stdout` is a closed / non-writable fd when `sieve` is called (`printf` fails, returns negative) | No rejection and **no abort**: the C ignores `printf`'s return value, so the loop still terminates on the `% 10 == 9` rule and returns normally. Rust must likewise ignore the return value. | [x] |

## Notes on what is deliberately *not* in the table

* **Null pointers** — the API has no pointer parameter. There is nothing to
  pass null to; a "null pointer" test would have to invent an argument the C
  does not take.
* **Zero and oversized lengths** — the API has no length/count/size parameter
  and performs no allocation, indexing, or copying.
* **Invalid enum variants** — the API has no enum parameter (row 9 records why
  the generic case degenerates to "any 32-bit value", which *is* covered).

These are recorded rather than silently dropped so the absence is a verified
fact about the C surface, not an oversight.
