# ERRORS.md — Phase C error / rejection surface table

## How this table was derived

Every error-ish construct in the whole C tree was enumerated mechanically:

```sh
grep -rnE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR|<|>|==|!=|\?|MAX|MIN|#if' \
     --include=*.c --include=*.h c_src | grep -v '/build/'
```

After discarding the licence comment block and the include guard, the **entire
set of surviving lines** is:

```
c_src/src/sieve.c:24:  #include <stdio.h>
c_src/src/sieve.c:35:  if (val % 10 == 9) {
```

Consequences, stated explicitly because they define the shape of this table:

* There is **no** `return` statement of any kind (`sieve` returns `void`).
* There are **zero** error-return macros, error enums, sentinel returns,
  `assert`s, `errno` uses, `exit`/`abort` calls.
* There are **no pointer parameters** anywhere in the public API
  (`void sieve(int start);` is the whole header), so *null-pointer* rejection
  is not reachable — there is no pointer to pass.
* There are **no length/size/count parameters**, so *zero length* and
  *oversized length* rejection is not reachable.
* There are **no enums** in the API, so an *out-of-range enum value* is not
  reachable. The `int` parameter's analogue is exercised below: the full
  `int` range including `INT_MIN`/`INT_MAX` and the values one step past the
  largest input that can terminate without signed overflow.

So rows **R1–R4** record that the four generic C-API rejection classes are
*structurally absent*, and rows **E1–E12** are the real, reachable degenerate /
boundary / undefined-behaviour inputs that the C accepts and must be matched
byte-for-byte. `sieve` has no way to signal failure, therefore "same
error/rejection" is asserted as **same emitted byte stream** (and, for the
inputs whose output is measured in tens of gigabytes, **same output prefix plus
same non-termination-within-budget behaviour**), which is the only observable
this API has.

Notation: `N(s)` = number of lines printed for start `s`.
For `s >= 0` with `s % 10 == 9`, `N = 1`; for other `s >= 0`, `N = 10 - (s % 10)`;
for `s < 0`, `N = 10 - s` (C's `%` truncates, so a negative `s` yields a
negative remainder and can **never** equal 9 — negative starts always count all
the way up to `9`).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| R1 | `sieve` | null pointer argument | **unreachable** — no pointer parameter exists in `sieve.h`; asserted by a compile/API-shape check, not a runtime call | `errors::r1_r4_structurally_absent_rejections` | [x] |
| R2 | `sieve` | zero length / empty input | **unreachable** — no length or count parameter exists | `errors::r1_r4_structurally_absent_rejections` | [x] |
| R3 | `sieve` | oversized length | **unreachable** — no length or count parameter exists | `errors::r1_r4_structurally_absent_rejections` | [x] |
| R4 | `sieve` | out-of-range enum value across FFI | **unreachable** — no enum parameter exists; the `int` parameter has no rejected sub-range, every one of the 2^32 values is accepted (covered by E1–E12) | `errors::r1_r4_structurally_absent_rejections` | [x] |
| E1 | `sieve` | `val = 9` — smallest non-negative value that satisfies `val % 10 == 9` | prints `9\n` and returns; exactly 1 line | `errors::e1_e6_boundary_bytes` | [x] |
| E2 | `sieve` | `val = 0` — zero, residue 0, longest non-negative run | prints `0\n1\n…9\n`; 10 lines | `errors::e1_e6_boundary_bytes` | [x] |
| E3 | `sieve` | `val = -1` — negative, residue `-1` (never 9), crosses the sign boundary | prints `-1\n0\n…9\n`; 11 lines. Does **not** stop early | `errors::e1_e6_boundary_bytes` | [x] |
| E4 | `sieve` | `val = -9` — the trap case: `-9 % 10 == -9` in C, **not** `9` | does **not** break on the first value; prints `-9\n-8\n…9\n`; 19 lines | `errors::e1_e6_boundary_bytes` | [x] |
| E5 | `sieve` | `val = -19`, `-99`, `-109` — further "ends in 9" negatives | same as E4: no early break; prints `val…9`; `10 - val` lines | `errors::e1_e6_boundary_bytes` | [x] |
| E6 | `sieve` | `val = 2147483639` = `INT_MAX - 8`, the largest `int` satisfying `val % 10 == 9` | prints `2147483639\n` and returns; exactly 1 line, no overflow | `errors::e1_e6_boundary_bytes` | [x] |
| E7 | `sieve` | `val = 2147483640` — one step past E6; the first start from which the loop **cannot** reach a value ending in 9 without signed overflow (`INT_MAX` ends in 7, next multiple-of-10-plus-9 is 2147483649 > `INT_MAX`) | C signed-overflow UB. As compiled (gcc, this `CMakeLists.txt`) `val++` wraps `INT_MAX -> INT_MIN`, so it prints 2147483640…2147483647, then -2147483648…9, i.e. 4294967306 lines / ~45 GB. Must not trap or stop early | `errors::e7_e10_overflow_region_prefix` | [x] |
| E8 | `sieve` | `val = 2147483646` — inside the overflow region | same UB/wrapping class as E7; 4294967300 lines | `errors::e7_e10_overflow_region_prefix` | [x] |
| E9 | `sieve` | `val = 2147483647` = `INT_MAX` — the maximum representable input, residue 7 | same UB/wrapping class as E7; prints `2147483647\n` then `-2147483648\n…9\n`; 4294967299 lines | `errors::e7_e10_overflow_region_prefix` | [x] |
| E10 | `sieve` | `val = INT_MIN = -2147483648` — the minimum representable input, residue `-8` | no early break; prints `-2147483648\n…9\n`; 2147483658 lines (~23 GB) | `errors::e7_e10_overflow_region_prefix` | [x] |
| E11 | `sieve` | `val = INT_MIN + 1 = -2147483647` — one step past the minimum | same class as E10; 2147483657 lines | `errors::e7_e10_overflow_region_prefix` | [x] |
| E12 | `sieve` | `val` passed as a value that a caller may sign-extend differently: the full-width bit patterns `0x80000000`, `0xFFFFFFFF`, `0x7FFFFFFF` handed across FFI as `c_int` | interpreted as `INT_MIN`, `-1`, `INT_MAX` respectively — identical to E10 / E3 / E9; no separate validation path | `errors::e12_bit_pattern_reinterpretation` | [x] |

### Why E7–E11 are prefix-compared

These five inputs each emit between 23 GB and 45 GB of text and require between
2.1e9 and 4.3e9 `printf` calls. Running either library to completion exceeds the
600 s command budget by a wide margin. The differential test therefore launches
the C and the Rust `.so` in **separate child processes** writing to a pipe,
compares a bounded **prefix** of the byte streams (which covers the interesting
transitions: the start value, the digit-width changes, and for E7–E9 the exact
`INT_MAX -> INT_MIN` wrap point), asserts neither side terminated or trapped
inside the budget, and then kills both. That is a real differential assertion
about the UB path, not a skipped row.

## Gate

- [x] Every row above has a passing error-path differential test that calls
      **both** the C `.so` and the Rust `.so` and asserts identical observable
      results.
