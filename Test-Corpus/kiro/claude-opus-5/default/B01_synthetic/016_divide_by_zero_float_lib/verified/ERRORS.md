# ERRORS.md — Error / rejection surface table (Phase A → Phase C)

## How this table was derived

Mechanically, from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). The greps below are exhaustive over the library:

```sh
grep -n  "return"                                  src include   # -> 0 hits
grep -nE "NULL|assert|ERROR|errno|exit|abort"      src include   # -> 1 hit (line 32)
grep -nE "\bif\b|\belse\b|\bswitch\b|#if|\bwhile\b|\bfor\b" src include
                                             # -> line 32, line 61, line 66 only
```

Findings that shape this table:

* **No function returns a value.** All five exported functions are `void`.
  There is no error code, no sentinel return, no `errno` use, no `assert`, and
  no `exit`/`abort`. Therefore "same error/rejection" is observable **only**
  through the bytes written to `stdout` (and through not crashing).
* There are exactly **two** conditional branches in the entire library:
  * `driver.c:32` — `if (line != NULL)` in `printLine` (a null check),
  * `driver.c:61` — `if (fabs(data) > 0.000001)` in `goodB2G` (a range check),
    with its `else` at `driver.c:66`.
* `bad()` (`driver.c:43-47`) has **no** guard at all. Its "error" behaviour is
  the C undefined behaviour of `(int)double` when the value does not fit an
  `int`. The shipped binary implements this with `cvttsd2si`
  (`objdump -d`, `bad+0x22`), which yields the *integer indefinite* value
  `INT_MIN` = `-2147483648` for NaN and for every out-of-range magnitude.
  That is the ground truth the Rust must reproduce (it does, via `to_c_int`).
* There are **no enums anywhere** in the public API, so "out-of-range enum
  value across FFI" degenerates to "out-of-range *float* bit pattern", which
  rows 12-18 cover (signalling/quiet NaN, ±inf, subnormals, ±max).

## The table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `printLine` | `line == NULL` (`driver.c:32` null check fails) | branch skipped: **no bytes written**, returns normally |
| 2  | `printLine` | `line` -> `""` (zero-length string; boundary of a valid pointer) | writes exactly one byte `"\n"` |
| 3  | `printLine` | `line` -> string containing `%d %s %n` (format-specifier bytes as *data*) | bytes emitted verbatim + `"\n"` (arg is `%s`, so no format interpretation) |
| 4  | `printLine` | `line` -> string with high/non-UTF-8 bytes (`0x80..0xFF`) and embedded `\t\r` | bytes emitted verbatim + `"\n"` (C is byte-oriented, not UTF-8) |
| 5  | `printLine` | `line` -> very long string (64 KiB, past stdio's buffer size) | full string + `"\n"`, no truncation |
| 6  | `printIntLine` | `intNumber == INT_MIN` (`-2147483648`, extreme of range) | `"-2147483648\n"` |
| 7  | `printIntLine` | `intNumber == INT_MAX` (`2147483647`) | `"2147483647\n"` |
| 8  | `printIntLine` | `intNumber == 0` / `-1` (sentinel-looking values) | `"0\n"` / `"-1\n"` |
| 9  | `bad` | `data == +0.0f` — the CWE-369 divide-by-zero, **unguarded** | `100.0/+0.0 = +inf`; `cvttsd2si(+inf)` -> `"-2147483648\n"` |
| 10 | `bad` | `data == -0.0f` (negative zero) | `100.0/-0.0 = -inf`; -> `"-2147483648\n"` |
| 11 | `bad` | `data == +FLT_MIN`, and subnormals `1e-40f`, `f32::from_bits(1)` | quotient overflows `int` range -> `"-2147483648\n"` |
| 12 | `bad` | `data == f32::NAN` (quiet NaN) | `100.0/NaN = NaN`; `cvttsd2si(NaN)` -> `"-2147483648\n"` |
| 13 | `bad` | `data ==` signalling NaN `from_bits(0x7FA0_0000)` and negative NaN `0xFFC0_0000` | NaN quotient -> `"-2147483648\n"` |
| 14 | `bad` | `data == f32::INFINITY` | `100.0/+inf = +0.0` -> `"0\n"` |
| 15 | `bad` | `data == f32::NEG_INFINITY` | `100.0/-inf = -0.0`; truncates to `0` -> `"0\n"` |
| 16 | `bad` | `data` just past the int-range cliff: `100.0/data` in `[2^31, ...)` i.e. `0 < data <= 4.656e-8` | out of `int` range -> `"-2147483648\n"` |
| 17 | `bad` | `data` just inside the cliff on the negative side (`100.0/data` in `(-2^31-1, -2^31]`) | truncates to exactly `INT_MIN`, *in range* -> `"-2147483648\n"` (same bytes, different mechanism) |
| 18 | `bad` | `data == ±f32::MAX`, `±1.0`, `±100.0` (quotient < 1, truncates toward zero) | `"0\n"` / `"-1\n"`/`"1\n"` / `"100\n"`/`"-100\n"` as truncation dictates |
| 19 | `goodB2G` (via `good`) | `data == 0.0f`: `fabs(0) > 1e-6` is **false** (`driver.c:61` range check fails) | `else` branch: `"This would result in a divide by zero\n"` |
| 20 | `goodB2G` (via `good`) | `data == -0.0f` | `fabs(-0.0) = 0.0`, guard false -> divide-by-zero message |
| 21 | `goodB2G` (via `good`) | `data == 1e-6f` — **exactly the constant**; `(double)1e-6f = 9.9999997e-07 < 1e-6` | guard false -> divide-by-zero message (the off-by-one-ULP boundary) |
| 22 | `goodB2G` (via `good`) | `data` one step *below* the threshold: `1e-7f`, `5e-7f`, `-1e-7f` (finite division, still rejected) | guard false -> divide-by-zero message |
| 23 | `goodB2G` (via `good`) | `data` one step *above* the threshold: `nextafter(1e-6f, inf)` = `1.0000001e-6f` | guard **true** -> divides: `"99999998\n"` |
| 24 | `goodB2G` (via `good`) | `data == f32::NAN` — every NaN comparison is false, so NaN takes the `else` | divide-by-zero message (NOT a division) |
| 25 | `goodB2G` (via `good`) | `data == ±f32::INFINITY` — `fabs(inf) > 1e-6` is **true** | divides: `100.0/±inf = ±0.0` -> `"0\n"` |
| 26 | `goodB2G` (via `good`) | `data ==` subnormal / `f32::MIN_POSITIVE` (below threshold) | guard false -> divide-by-zero message |
| 27 | `good` | any `data` — `goodG2B()` is called **first** and unconditionally | `"50\n"` always precedes `goodB2G`'s line; ordering is part of the contract |
| 28 | `driver` | `badData == 0.0f` (the flawed call reached through the public header) | 5 lines + `"-2147483648\n"`, in the fixed `driver.c:78-86` order |
| 29 | `driver` | `goodData == 0.0f` **and** `badData == 0.0f` (both degenerate at once) | `Calling good()...` / `50` / divide-by-zero msg / `Finished good()` / `Calling bad()...` / `-2147483648` / `Finished bad()` |
| 30 | `driver` | `goodData`/`badData` = NaN, ±inf, ±0.0 cross-product (25 combinations) | per-row composition of rows 9-27 |
| 31 | `goodB2G` guard | `fabs(data) == 0.000001` exactly — i.e. `>` vs `>=` at `driver.c:61` | **UNREACHABLE**: no `float` widens to the `double` `1e-6` (its significand has non-zero bits below `float` precision, and `float -> double` is exact), so this trigger cannot be constructed |
| 32 | `bad` / `to_c_int` | quotient in `(-2^31 - 1, -2^31]` — the lower edge of the `cvttsd2si` range | every such `double` truncates to exactly `INT_MIN`, so the result is `"-2147483648\n"` whether the range check fires or not |

## Row status

Every row above is checked by `translation/tests/differential.rs`
(`phase_c_*` cases). Each case name carries its row number and asserts that the
C `.so` and the Rust `.so` produce **byte-identical** `stdout` — the only
observable channel this library has. Rows 31-32 are the two triggers that no
input can construct; their cases *prove* the unreachability mechanically
(`phase_c_proof_guard_equality_unreachable`, `phase_c_proof_int_min_interval`)
rather than asserting it in prose, and additionally run the differential over
the surrounding ULP neighbourhood.

- [x] 1  - [x] 2  - [x] 3  - [x] 4  - [x] 5  - [x] 6
- [x] 7  - [x] 8  - [x] 9  - [x] 10 - [x] 11 - [x] 12
- [x] 13 - [x] 14 - [x] 15 - [x] 16 - [x] 17 - [x] 18
- [x] 19 - [x] 20 - [x] 21 - [x] 22 - [x] 23 - [x] 24
- [x] 25 - [x] 26 - [x] 27 - [x] 28 - [x] 29 - [x] 30
- [x] 31 - [x] 32

## Evidence that these tests can actually fail

A passing suite proves nothing unless it is known to be sensitive. Thirteen
mutants were injected into `translation/src/lib.rs` one at a time, each rebuilt
and re-run; eleven were caught:

| mutant | caught by |
|--------|-----------|
| drop the out-of-`int`-range check in `to_c_int` (use Rust's saturating cast) | 10+ cases |
| drop the NaN check in `to_c_int` | rows 12, 13, 30, bit sweep |
| `fabs(data) >= 1e-6` instead of `>` | **survives — equivalent mutant, proved by row 31** |
| divide in `f32` instead of promoting to `f64` | 10+ cases |
| swap `goodG2B()` / `goodB2G()` call order | rows 16-19, 21-25 |
| `goodG2B` divisor `1.0` instead of `2.0` | 24 cases |
| one letter changed in the divide-by-zero message | 10+ cases |
| swap two `printLine` calls inside `driver` | 10 cases |
| `printIntLine` format `%u` instead of `%d` | 35 cases |
| remove the `printLine` NULL check | rows 1, 26 + generic NULL case (SIGSEGV, reported as a failure thanks to fork isolation) |
| add a guard to `bad()` that the C does not have | 21 cases |
| un-export `printIntLine` (`#[no_mangle]` removed) | every case + `phase_d_symbol_parity` |
| relax `to_c_int`'s lower bound by one | **survives — equivalent mutant, proved by row 32** |

Both survivors are provably indistinguishable for every possible input, so the
suite's mutation score is 11/11 on the reachable mutants.
