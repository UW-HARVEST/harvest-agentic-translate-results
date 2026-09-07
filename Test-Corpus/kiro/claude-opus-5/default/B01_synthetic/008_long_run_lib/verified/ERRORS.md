# ERRORS.md — Phase C error / rejection surface table

## Mechanical derivation

Every rejection-shaped construct was grepped out of the whole C tree
(`c_src/src/long.c`, `c_src/include/long.h`):

```
grep -n "return"                                       -> src/long.c:66:    return;   (bare, void)
grep -nE "assert|NULL|errno|exit\(|abort|RETURN_ERROR|error"  -> (no matches)
grep -ncE "\bif\b|\bswitch\b|\?"                       -> src/long.c:0  include/long.h:0
grep -nE "#define"  -> ARRAY_SIZE (256*1024), ITERATIONS 2000   (no #ifdef feature gates)
```

Result: **the C library contains no error returns, no error enums, no sentinel
values, no `assert`, no null checks, and no range checks.** Both public
functions return `void`; the only `return` is a bare `return;` on the last line
of `long_exec`. There are literally zero conditional branches in the library.

Therefore the "rejection" surface is *the absence of rejection*: every input is
accepted, and the contract the Rust must match is that it likewise **never**
rejects, never panics, never aborts, and produces the same output for inputs
that would look invalid to a defensive implementation. A Rust translation that
added a bounds check, an overflow panic, or a `debug_assert` would diverge —
that is the real failure mode here, and each row below tests for it.

Rows are derived from what the C *actually does* with extreme input: the two
`#define` constants, the domain of the one parameter (`unsigned int seed`), the
full `int` domain of the one mutable global (`array`), the signed-overflow /
shift / division operations in the arithmetic kernel, and the FFI-visible
boundaries of the exported ABI.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|----|----------|----------------------------------------------|-------------------|-----|
| 1  | `long_exec` | `seed = 0` (glibc `srand(0)` is specified to behave as `srand(1)`) | no error; prints the *same* value as `seed = 1`; return type `void`, nothing to report | [x] |
| 2  | `long_exec` | `seed = 1` (glibc `rand()` default state) | no error; prints an `int` + `\n` | [x] |
| 3  | `long_exec` | `seed = UINT_MAX = 4294967295` (max of the parameter domain) | no error; accepted, prints a value | [x] |
| 4  | `long_exec` | `seed = 2147483648 = 2^31` (bit 31 set; a value that is *negative* if reinterpreted as `int`, i.e. an out-of-`int`-range value crossing the FFI boundary into an `unsigned int` parameter) | no error; accepted verbatim, no sign-extension, no clamping | [x] |
| 5  | `long_exec` | `seed = 2147483647 = INT_MAX`, `seed = 2147483649 = INT_MIN as unsigned` (one step either side of the signed boundary) | no error; three distinct results, no aliasing of `2^31-1`/`2^31`/`2^31+1` | [x] |
| 6  | `long_exec` | called twice in a row with the same seed (`srand` re-seed must fully reset PRNG state; `array` still holds the previous run's output) | second call prints byte-identical output to the first | [x] |
| 7  | `long_exec` | called with seed A then seed B (residual `array` state from run A must not leak into run B, because the fill loop overwrites all 262144 slots) | run B prints exactly what a fresh process with seed B prints | [x] |
| 8  | `perform_expensive_operations` | called before any `long_exec`, i.e. on the *untouched* `.bss` array (all 262144 elements `= 0`) | no error; `0` is mapped to `f^100(0)`; no division-by-zero, no special case | [x] |
| 9  | `perform_expensive_operations` | every element `= INT_MIN = -2147483648`; the kernel's `x * 3 + 7`, `x << 1` and `x / 2` all hit the signed-overflow / `INT_MIN` corner (`x*3` overflows, `INT_MIN << 1 == 0`, `INT_MIN / 2` is exact) | no trap; wraps two's-complement; `x - (x<<1)` yields `INT_MIN` again rather than the un-representable `-INT_MIN` | [x] |
| 10 | `perform_expensive_operations` | every element `= INT_MAX = 2147483647` (`x*3+7` overflows on the very first statement) | no trap; wraps | [x] |
| 11 | `perform_expensive_operations` | element `= -1` and other small negatives; `x >> 3` is an *arithmetic* shift on a negative value (implementation-defined in C, sign-propagating on gcc/x86-64) | sign bits propagate; `-1 >> 3 == -1` | [x] |
| 12 | `perform_expensive_operations` | elements chosen so the intermediate before `x / 2 + x % 7` is negative | C truncates division toward zero and `%` takes the sign of the dividend (e.g. `-11/2 == -5`, `-11%7 == -4`), *not* floor division | [x] |
| 13 | `perform_expensive_operations` | elements that are multiples of 7 / of 2, and `x` such that `x % 7 == 0` | no error; `x % 7 == 0` is not special-cased; never a divide-by-zero (both divisors are non-zero literals) | [x] |
| 14 | `perform_expensive_operations` | called on element values that are *fixed points or cycle members* of the kernel (worst case for any iteration-accelerating translation) | plain `f` applied 100×, identical to any other value | [x] |
| 15 | `perform_expensive_operations` | called repeatedly (1, 2, 3, … times) — the C composes it with no guard against repetition, and `long_exec` relies on that composition 2000 times | k-th call output `== f^(100k)` element-wise; no drift, no saturation, no "already done" short-circuit | [x] |
| 16 | `array` (exported object) | caller writes *arbitrary* `int` patterns, including all 2^32 boundary values, directly through the `dlsym`'d pointer before calling `perform_expensive_operations` | no validation whatsoever; every bit pattern is a legal element | [x] |
| 17 | `array` (exported object) | caller reads/writes the last slot, index 262143, and relies on the object being exactly `0x100000` bytes (an off-by-one in the exported object's size is an out-of-range-index bug an external caller *will* hit) | symbol size is `0x100000`; slot 262143 is live and is included in the XOR reduction | [x] |
| 18 | `long_exec` | the XOR reduction accumulates into a **signed** `int` starting at `0` and the result is printed with `"%d"` — so a result with bit 31 set must print as a *negative* decimal, not as a large unsigned one | `printf("%d\n", xor_result)` semantics, sign preserved | [x] |
| 19 | `long_exec` | output stream: value is written with `printf` to `stdout` (C stdio buffer), and nothing else is ever printed — no prefix, no trailing space, exactly one `\n`, no `stderr` output | stdout is exactly `<decimal>\n` | [x] |
| 20 | `perform_expensive_operations` | `long_exec`'s loop counter is `int i` and its array indices are `size_t i`; `ITERATIONS = 2000` and `ARRAY_SIZE = 262144` both fit, so neither loop can wrap or terminate early | exactly 2000 calls, exactly 262144 elements touched per call — no element skipped, none processed twice | [x] |

There is **no** out-of-range *enum* to test: the C declares no enum, and the sole
parameter (`unsigned int seed`) has no invalid value — every one of the 2^32 bit
patterns is accepted, which rows 1–5 sample at the extremes.

All 20 rows are covered by `tests/errors.rs` (rows 1–7, 15, 18–20 also by
`tests/long_exec_diff.rs` for the full 2000-iteration pipeline).  Every row
passed under all four cargo feature combinations.

Independent cross-check of rows 6/7 (residual state) against the C `.so`:
calling its exported `long_exec` three times in one process with seeds
`7, 255, 7` prints `72337063`, `155851384`, `72337063` — i.e. the third run
reproduces the first exactly, and each matches the fresh-process recording.  The
Rust `.so` does the same (`tests/long_exec_diff.rs::rows31_32_…` and the live
`row_live_ordering_and_idempotence`, which drove both libraries through that
sequence in one process and compared every call's stdout).
