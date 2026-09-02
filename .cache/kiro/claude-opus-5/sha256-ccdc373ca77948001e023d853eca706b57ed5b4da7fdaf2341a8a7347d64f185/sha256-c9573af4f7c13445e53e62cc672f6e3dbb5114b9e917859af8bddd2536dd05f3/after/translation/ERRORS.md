# ERRORS.md — Phase A error / rejection surface

Derived mechanically from `c_src/src/long.c` + `c_src/include/long.h`
(the *only* two source files). Greps run over the whole C source:

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|ERROR|error|goto|malloc|free' src/long.c include/long.h
grep -nE 'if *\(|while *\(|switch|#if' src/long.c include/long.h
```

Findings, verbatim, after discarding the licence comment block:

* `return` — 1 hit: `long_exec` line 66, a bare `return;` from a `void`
  function (fall-through, not an error path).
* `assert` — 0 hits.
* `NULL` / `errno` / `exit(` / `abort` / `ERROR` / error enums — 0 hits.
* `goto` — 0 hits.
* `malloc` / `free` — 0 hits (the only buffer is the static `array`).
* `if (` / `switch` / `#if` in code — 0 hits. The only conditionals in the
  translation unit are the three `for`-loop bounds (`i < ARRAY_SIZE`,
  `j < 100`, `i < ITERATIONS`) and `#ifndef ECHO_H_` in the header guard.

**The library therefore has NO explicit error surface**: no error codes, no
sentinel returns, no null checks, no range checks, no asserts. Both public
functions return `void`, and the only scalar parameter (`unsigned int seed`) has
no invalid value — every one of the 2^32 values is passed straight to `srand`.

Because there is no error-code surface to compare, the rejection surface that
*does* exist is the set of **implicit numeric edge conditions** the C code walks
into. Each row below is a distinct condition the C reaches, with the result the
compiled C actually produces (`-O0`, x86-64, gcc 11.5 — the build configured by
`c_src/CMakeLists.txt`, which sets no `CMAKE_BUILD_TYPE` and hence no `-O`
flag). The Rust must reproduce these *exactly*, i.e. not panic, not abort, and
not produce a different value.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `long_exec` | `seed == 0` — glibc `srand(0)` is specified to behave like `srand(1)` | no error; runs normally; identical output to `seed == 1` |
| 2 | `long_exec` | `seed == UINT_MAX` (4294967295) — largest representable seed | no error; runs normally |
| 3 | `long_exec` | `seed == 2147483648` (2^31, sign bit set in the `unsigned int`) — must be passed to `srand` zero-extended, not sign-extended | no error; runs normally |
| 4 | `long_exec` | negative `int` passed by the caller through the `unsigned int` parameter (e.g. `-1` → 4294967295) | no error; same as row 2 (identical bit pattern) |
| 5 | `perform_expensive_operations` | called before any `long_exec`, i.e. `array` still all-zero `.bss` | no error; every element becomes `f^100(0)` |
| 6 | `perform_expensive_operations` | `array[i] == INT_MIN`: `x * 3` overflows signed `int` (UB in ISO C; the `-O0` build wraps two's-complement) | no trap; wrapped result |
| 7 | `perform_expensive_operations` | `array[i] == INT_MAX`: `x * 3 + 7` overflows signed `int` | no trap; wrapped result |
| 8 | `perform_expensive_operations` | `x < 0` at `x ^ (x >> 3)`: right shift of a negative signed value (implementation-defined; gcc = arithmetic shift, sign-propagating) | sign-propagating shift |
| 9 | `perform_expensive_operations` | `x < 0` at `x - (x << 1)`: left shift of a negative signed value (UB in ISO C; `-O0` build emits a plain shift/add) | bit-wise shift, wrapping |
| 10 | `perform_expensive_operations` | `x == INT_MIN` at `x - (x << 1)`: `x << 1` is `0`, `x - 0 == INT_MIN`; the subtraction itself can also overflow for other `x` | no trap; wrapped result |
| 11 | `perform_expensive_operations` | `x < 0` at `x / 2`: C99 requires truncation *toward zero* (not floor) | e.g. `-3 / 2 == -1` |
| 12 | `perform_expensive_operations` | `x == INT_MIN` at `x / 2` | `-1073741824`; no `SIGFPE` (divisor is the constant 2, `INT_MIN / -1` is unreachable) |
| 13 | `perform_expensive_operations` | `x < 0` at `x % 7`: C99 requires the remainder to take the **sign of the dividend** | e.g. `-8 % 7 == -1` |
| 14 | `perform_expensive_operations` | division/modulus by zero | **unreachable** — both divisors are the non-zero literals `2` and `7`; no check needed and none present |
| 15 | `perform_expensive_operations` / `long_exec` | out-of-bounds access on `array` | **unreachable** — all three loops are bounded by `i < ARRAY_SIZE` / `i < ITERATIONS`; no caller-supplied index exists |
| 16 | `long_exec` | `xor_result` accumulation over 262144 elements — `^=` cannot overflow | exact XOR of the final array |
| 17 | `long_exec` | repeated calls / state carry-over: `array` is a global, so a second `long_exec` with the same seed must re-`srand` and overwrite, giving an identical result | idempotent w.r.t. seed |
| 18 | `long_exec` | `printf("%d\n", xor_result)` with a negative `xor_result` | prints a leading `-`; must be `%d` (signed), not `%u`. **Unreachable in practice**: every element of the post-`f^200000` image lies in `[-1073734582, -536871525]`, so all 262144 elements have bit 31 set, and 262144 is even — the XOR's sign bit always cancels. Verified over all 50 reference dumps: the printed value is always in `0 ..= INT_MAX`, where `%d` and `%u` render identically. The `%d`-vs-`%u` distinction is therefore not observable through this API; what *is* checked is that the printed bytes are exactly the C's, for every seed. |
| 19 | both | null-pointer argument | **not applicable** — neither function takes a pointer; `array` is a fixed `.bss` object, not caller-provided |
| 20 | both | out-of-range enum value across the FFI boundary | **not applicable** — the API has no enum, struct or pointer parameter; the only parameter is `unsigned int`, and rows 1–4 already cover its full range including values with no "valid" meaning |

Rows 14, 15, 19 and 20 are recorded as *unreachable / not-applicable* rather
than dropped, so the table documents that those generic C-API hazards were
looked for in the source and are genuinely absent. Rows 1–13 and 16–18 each get
a differential test in `tests/errors.rs` (rows 1–4 additionally in
`tests/long_exec_diff.rs`, which owns the seed extremes).

Beyond the table, the generic C-API boundaries were covered as follows:

* **null pointers** — impossible: no function takes a pointer (row 19).
* **zero / oversized lengths** — impossible: no function takes a length; the one
  buffer is a fixed-size `.bss` object whose size is part of the ABI and is
  asserted equal in both `.so`s (`SYMBOLS.md`, 0x100000 bytes both sides).
* **one step past a documented range** — the only parameter is `unsigned int`,
  whose entire range is valid; `seed_boundary_sweep_does_not_reject` walks
  `0, 1, 2, INT_MAX, INT_MAX+1, 32767/32768, 65535/65536, 2147483646,
  UINT_MAX-1, UINT_MAX`, plus `-1 as u32`, `INT_MIN as u32`, and 64-bit values
  truncated to 32 bits, and requires equal bit patterns to give equal output.
* **out-of-range enum values across FFI** — the API declares no enum, so there
  is no variant space to escape (row 20). The analogous "any int is accepted"
  hazard is the `unsigned int` seed, covered by the sweep above.
* **`k = 0`** — calling `perform_expensive_operations` zero times must leave the
  array bit-unchanged in both libraries (`CONFIGS.md` row 2).
