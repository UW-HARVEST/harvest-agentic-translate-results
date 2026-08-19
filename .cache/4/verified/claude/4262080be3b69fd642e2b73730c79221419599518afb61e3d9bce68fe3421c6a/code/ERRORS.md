# ERRORS.md — Phase C error-surface table

## How this table was derived (mechanical, not assumed)

```sh
grep -rnE 'return[^;]*;|assert|NULL|errno|exit\(|abort|\bif\b|\bswitch\b|#ifdef|#if |ERROR|\bwhile\b|\bfor\b' \
     c_src/src/driver.c c_src/include/driver.h   # -> no matches (exit 1)
grep -nE '^\s*#' c_src/src/driver.c c_src/include/driver.h  # -> only the include guard + 3 #includes
```

Result: **`c_src` contains zero rejection paths.** There is no `if`, no `switch`,
no `assert`, no `return` of a value, no error enum, no `NULL` check, no range
check, no min/max constant, and no `errno` use anywhere in the library. Both
exported functions are `void` and take a single unconstrained `int`, so the
library has **no channel through which it could signal an error** and no input it
rejects.

That absence is itself the property under test: for every extreme / "invalid
looking" input, the C accepts it silently and produces output, and the Rust must
do the *same* thing — in particular it must not panic, abort, or trap where C
merely wraps. Signed-integer overflow in `house->bedrooms += extra_bedrooms`
(`driver.c:42`) is the one place where an extreme input changes behaviour, and it
is exercised below against the C `.so` as ground truth.

Test file: `tests/differential.rs`. Rows marked *not reachable* are justified from
the source and have no test to write (nothing to call).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `run` | `extra_bedrooms = INT_MAX` from a fresh instance → signed overflow of `bedrooms` (`5 + 2147483647`) | no error, no abort: prints the wrapped two's-complement value (`-2147483644`) via `%d`; returns normally | `err_e1_run_int_max` | [x] |
| E2 | `run` | `extra_bedrooms = INT_MIN` (`-2147483648`) from a fresh instance → signed overflow the other direction | no error: prints wrapped value (`-2147483643`); returns normally | `err_e2_run_int_min` | [x] |
| E3 | `driver` | `x = INT_MAX` → overflow happens twice (once per inner `run`) and the second `run` starts from the already-wrapped state | no error: both wrapped results printed, 8 lines total | `err_e3_driver_int_max` | [x] |
| E4 | `driver` | `x = INT_MIN` → double overflow, opposite direction | no error: 8 lines, wrapped values | `err_e4_driver_int_min` | [x] |
| E5 | `run` | repeated calls whose *accumulated* `bedrooms` crosses `INT_MAX` (overflow spread across calls, not within one call) | no error: `bedrooms` keeps wrapping mod 2^32, printed signed | `err_e5_accumulated_overflow` | [x] |
| E6 | `run` | `extra_bedrooms = 0` (degenerate/no-op argument) | no error: `bedrooms` unchanged, still 4 printed lines | `err_e6_zero_argument` | [x] |
| E7 | `run` / `driver` | out-of-range "enum" value across the FFI boundary — the API declares no enum, so the full `int` domain is in-domain: `0x80000000`, `0x7fffffff`, `-1`, `0xdeadbeef as i32`, `INT_MIN+1`, `INT_MAX-1` all passed as raw bit patterns | every value accepted and printed with `%d`; no rejection, no trap | `err_e7_raw_bit_patterns` | [x] |
| E8 | `run` | `extra_bedrooms` chosen so `bedrooms` lands exactly on `0`, `-1`, `INT_MIN`, `INT_MAX` (one step past each side of the signed range) | no error: exact boundary values printed, then wrap on the next step | `err_e8_boundary_landings` | [x] |
| E9 | `run` / `driver` | value one step past a "documented valid range" | *not reachable*: `driver.h:27` documents no range; the parameter's only range is `int`'s own, covered by E1/E2/E7/E8 | — | [x] |
| E10 | `run` / `driver` | null pointer argument | *not reachable*: neither exported function takes a pointer (`void driver(int)`, `void run(int)`); the only pointer-taking functions, `add_floor` / `add_bedrooms`, are `static` (`driver.c:37,41`) and absent from `nm -D`, so no caller can reach them across the FFI boundary | — | [x] |
| E11 | `run` / `driver` | zero-length / oversized length argument | *not reachable*: no length, size, count or buffer parameter exists anywhere in the public ABI | — | [x] |
| E12 | `run` / `driver` | error code / sentinel return inspection | *not reachable*: both functions are `void`; there is no return-value channel to compare, so "same error code" is asserted as "same stdout bytes and normal return in both" | — | [x] |
| E13 | `add_floor` (`floors` overflow) | `floors` incremented past `INT_MAX` | *not reachable in test time*: `floors` grows by exactly 1 per `run`, so 2^31 − 2 calls (~10^9 × 4 printf lines) would be required. The identical `wrapping_add` code path is covered by E1/E5 on `bedrooms` | — | [x] |
| E14 | `print_the_house` | `bathrooms` grown large enough that `%.1f` changes shape | no error: `+= 1.0` keeps `bathrooms` a half-integer, so `%.1f` stays exact; verified for large accumulations | `err_e14_bathrooms_growth` | [x] |

All reachable rows (E1–E8, E14) have a differential test that constructs the
exact condition, calls **both** the C `.so` and the Rust `.so` through
`libloading`, and asserts byte-identical stdout plus normal return from both.

## Why "same error" is asserted as "same bytes + normal return"

The library has no error channel (row E12), so the only observable rejection
signals available are: the printed bytes, whether anything is written to stderr,
and whether the call returns normally. Each `step()` in `tests/differential.rs`
asserts all three for both implementations:

* `stdout` bytes equal;
* `stderr` bytes equal **and empty** — a Rust `panic!` (debug) or `abort`
  (release, `panic = "abort"`) on an input the C accepts would print here or kill
  the process, and would therefore fail the row instead of passing silently;
* the expected number of output lines (4 for `run`, 8 for `driver`), so a call
  that bails out early cannot masquerade as success.

## Cross-check against an optimized C build

Signed-integer overflow (E1–E5, E8) is UB in C, so its result could in principle
differ between C compilations. The reference `.so` (cmake default flags, no
optimization) was cross-checked against `gcc -O2 -fPIC -shared` for
`driver(INT_MAX)`, `run(INT_MIN)`, `driver(-1)`, `run(1000000)`: the `-O0` C, the
`-O2` C and the Rust `.so` produce byte-identical output, i.e. the Rust
`wrapping_add` matches what the C actually does in both builds.
