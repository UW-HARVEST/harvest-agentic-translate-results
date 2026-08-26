# ERRORS.md — Error / rejection surface of `c_src/src/driver.c`

## How this table was derived (mechanically, not from docs)

```sh
grep -n -E 'return|NULL|assert|if *\(|else|RETURN|ERROR|exit|abort|/' c_src/src/driver.c
```

Findings — the *complete* set of guard/rejection constructs in the C source:

* `driver.c:32` — `if (line != NULL)` … the **only** explicit null check.
* `driver.c:61` — `if (fabs(data) > 0.000001)` … the **only** explicit range
  check, with an `else` branch at `driver.c:66` that emits a diagnostic string.
* `driver.c:45`, `:54`, `:63` — `int result = (int)(100.0 / data);` … three
  *unguarded* IEEE-754 division + float→int narrowing sites. These are the
  implicit rejection surface: division by zero and the C-undefined
  float→`int` conversion. On the x86-64 target GCC emits `divsd` followed by
  `cvttsd2si` (verified with `objdump -d c_src/build/libdriver.so`), so an
  out-of-range / NaN / infinite quotient yields the "integer indefinite" value
  `0x80000000` = `INT_MIN` = `-2147483648`.
* There are **no** `return -1` / `return NULL` / error enums / `assert()` /
  `exit()` / `abort()` calls: every public function returns `void`. Therefore
  "same error/rejection" means **byte-identical stdout** (including the
  *absence* of output) for the constructed invalid input.

Constants that define the boundaries: `0.000001` (double), `100.0` (double),
`2.0F` (float), and the implicit `int` range `[-2147483648, 2147483647]`.

## Error-surface table

One row per distinct rejection / undefined-input branch. "expected C result" is
the exact bytes the C library writes to `stdout` (`⟨none⟩` = writes nothing).

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `printLine` | `line == NULL` (`driver.c:32` false branch) — the null check rejects the pointer | `⟨none⟩` — returns without touching stdout; **must not crash** |
| E2 | `printLine` | non-null but *dangling-looking* degenerate buffer: pointer to a lone `'\0'` (empty string) | `"\n"` |
| E3 | `printLine` | string whose bytes are `printf` format directives (`"%s %d %n %%"`) — C passes it as `puts` / `%s` *argument*, so it is **not** interpreted | the literal bytes + `"\n"` |
| E4 | `printIntLine` | `intNumber == INT_MIN` (`-2147483648`) — extreme of the valid `int` range, one step below `-2147483647` | `"-2147483648\n"` |
| E5 | `printIntLine` | `intNumber == INT_MAX` (`2147483647`) — one step past is not representable in `int` | `"2147483647\n"` |
| E6 | `bad` | `data == +0.0f` → `100.0 / 0.0` = `+INF` → `(int)+INF` is **C-undefined**; `cvttsd2si` → `INT_MIN` (this is the CWE-369 flaw; `bad` has **no** guard) | `"-2147483648\n"` |
| E7 | `bad` | `data == -0.0f` → `100.0 / -0.0` = `-INF` → `(int)-INF` undefined → `INT_MIN` | `"-2147483648\n"` |
| E8 | `bad` | `data` = quiet NaN (either sign) → quotient NaN → `(int)NaN` undefined → `INT_MIN` | `"-2147483648\n"` |
| E9 | `bad` | `data` = signalling NaN → `divsd` raises *invalid*, quiets the NaN → `INT_MIN` | `"-2147483648\n"` |
| E10 | `bad` | `data` tiny but non-zero so that quotient **overflows** `int`: `0 < abs(data) < 100/2147483648` (e.g. `1e-8f`, `f32::MIN_POSITIVE`, smallest subnormal `1e-45f`) → `(int)` out of range → `INT_MIN` | `"-2147483648\n"` |
| E11 | `bad` | `data` tiny and **negative** so quotient underflows `int`: quotient `< -2147483648` (e.g. `-1e-8f`) → `INT_MIN` | `"-2147483648\n"` |
| E12 | `bad` | one step **past** the largest in-range quotient: walking ULP by ULP across `100/2147483648` (`0x33480000`) the C flips from `"-2147483648\n"` to `"2147483484\n"` at `nextafter(0x33480000, INF)`; the flip must happen at exactly the same float in Rust | `"-2147483648\n"` on the small side, an in-range value on the large side (e.g. `"2147483484\n"`) |
| E13 | `bad` | `data == +INF` → `100.0/INF` = `+0.0` → `(int)0.0` = `0` (well-defined; must **not** be `INT_MIN`) | `"0\n"` |
| E14 | `bad` | `data == -INF` → `100.0/-INF` = `-0.0` → `(int)-0.0` = `0` | `"0\n"` |
| E15 | `good` / `goodB2G` | `fabs(data) <= 0.000001` with `data == +0.0f` — the guard **rejects** the divisor | `"50\n"` (from `goodG2B`) then `"This would result in a divide by zero\n"` |
| E16 | `good` / `goodB2G` | `data == -0.0f` — `fabs` clears the sign, still rejected | `"50\n"` + `"This would result in a divide by zero\n"` |
| E17 | `good` / `goodB2G` | `data` = NaN → `comisd` is **unordered**, `jbe` is taken (C: `NaN > x` is false) → rejected. This is the one case where the guard rejects a value that is *not* small | `"50\n"` + `"This would result in a divide by zero\n"` |
| E18 | `good` / `goodB2G` | `data` = subnormal / `f32::MIN_POSITIVE` / `1e-8f` — `abs(data) <= 1e-6` → rejected (so `good` never divides by these, unlike `bad`) | `"50\n"` + `"This would result in a divide by zero\n"` |
| E19 | `good` / `goodB2G` | boundary: `data == 1e-6f` (= `9.99999997475e-7` as a double, i.e. **just below** the `0.000001` double literal) → `>` is false → rejected | `"50\n"` + `"This would result in a divide by zero\n"` |
| E20 | `good` / `goodB2G` | one step **past** the boundary: `nextafter(1e-6f, INF)` = `0x358637be` > `0.000001` → **accepted**, divides | `"50\n"` + `"99999988\n"` (must not take the reject branch) |
| E21 | `good` / `goodB2G` | negative just past the boundary: `-nextafter(1e-6f, INF)` = `0xb58637be` → `fabs` accepts → divides, quotient negative | `"50\n"` + `"-99999988\n"`, not the diagnostic |
| E22 | `good` / `goodB2G` | `data == ±INF` — passes the `> 0.000001` guard, `100.0/±INF` = `±0.0` → `(int)` = `0` | `"50\n"` + `"0\n"` |
| E23 | `driver` | `badData` = any E6–E12 trigger (0.0, −0.0, NaN, subnormal) — `driver` calls `bad` unguarded **after** `good`, so the whole 6-line transcript must match, in order | `"Calling good()...\n" … "Finished bad()\n"` with `bad`'s line = `"-2147483648\n"` |
| E24 | `driver` | `goodData` = a rejected value **and** `badData` = a rejected value simultaneously (both guards fire in one call) | full transcript with the diagnostic line *and* `"-2147483648\n"` |
| E25 | all entry points | out-of-range "enum"/discriminant values across the FFI boundary: the API has **no enum parameters**, so the analogous input is an arbitrary 32-bit pattern reinterpreted as `float` (`f32::from_bits(x)` for random `x`, incl. all NaN payload classes) and arbitrary `int` bit patterns for `printIntLine` | identical bytes for every bit pattern |

## Status

Every row above is covered by a differential test in
`tests/differential_errors.rs` that constructs the exact condition, calls the
symbol in **both** the C `.so` and the Rust `.so`, and asserts byte-identical
captured `stdout` (including "produced nothing"). See that file's
`ERRORS.md row Exx` comments; all rows pass.

| row | test | status |
|-----|------|--------|
| E1 | `e01_print_line_null` | [x] |
| E2 | `e02_print_line_empty` | [x] |
| E3 | `e03_print_line_format_specifiers` | [x] |
| E4 | `e04_print_int_line_int_min` | [x] |
| E5 | `e05_print_int_line_int_max` | [x] |
| E6 | `e06_bad_positive_zero` | [x] |
| E7 | `e07_bad_negative_zero` | [x] |
| E8 | `e08_bad_quiet_nan` | [x] |
| E9 | `e09_bad_signalling_nan` | [x] |
| E10 | `e10_bad_tiny_overflows_int` | [x] |
| E11 | `e11_bad_tiny_negative_underflows_int` | [x] |
| E12 | `e12_bad_cvttsd2si_boundary` | [x] |
| E13 | `e13_bad_positive_infinity` | [x] |
| E14 | `e14_bad_negative_infinity` | [x] |
| E15 | `e15_good_positive_zero` | [x] |
| E16 | `e16_good_negative_zero` | [x] |
| E17 | `e17_good_nan_unordered` | [x] |
| E18 | `e18_good_subnormals_rejected` | [x] |
| E19 | `e19_good_guard_boundary_below` | [x] |
| E20 | `e20_good_guard_boundary_above` | [x] |
| E21 | `e21_good_guard_boundary_negative` | [x] |
| E22 | `e22_good_infinities` | [x] |
| E23 | `e23_driver_bad_data_rejected` | [x] |
| E24 | `e24_driver_both_rejected` | [x] |
| E25 | `e25_arbitrary_bit_patterns` | [x] |

### Generic C-API boundaries (covered even though not derived from a specific `RETURN_ERROR`)

| what | test | status |
|------|------|--------|
| null pointer — alone, repeated, and interleaved with valid pointers and with `driver()` | `g01_null_pointer_repeated_and_mixed` | [x] |
| zero / one-past every `int` range edge: `INT_MIN`/`INT_MAX` ± 1, every `10^n` and `2^n` digit-count rollover | `g02_int_one_past_every_range` | [x] |
| one ULP either side of **every** float bound in the C source (`0`, `±0.000001`, the two `cvttsd2si` limits, `FLT_MIN`, largest subnormal, `FLT_MAX`, `2.0`, `1.0`, `100.0`) through `bad`, `good` **and** `driver` | `g03_float_one_ulp_past_every_documented_bound` | [x] |
| "out-of-range enum" analogue: the API has no `enum` parameters, so every `f32` **encoding class** is swept exhaustively over the whole exponent field (256 exponents × 4 mantissas × 2 signs = 1536 values, i.e. zeros, all subnormals classes, all normals classes, both infinities, quiet and signalling NaNs) plus arbitrary raw bit patterns | `g04_out_of_range_discriminants`, `e25_arbitrary_bit_patterns` | [x] |
| oversized lengths — `printLine` with buffers of 1 … 65536 bytes, straddling stdio's `BUFSIZ` | `differential_valid::c07_print_line_long_strings` | [x] |

### Note on "no error codes"

Because every C entry point returns `void` and the C never returns a sentinel,
each row's test additionally pins the **exact expected C bytes** (helper
`expect()` in `tests/differential_errors.rs`).  Without that, a row such as E1
("writes nothing") could pass for the wrong reason if the capture were broken;
`harness_selfcheck.rs` guards the same property from the other direction.
