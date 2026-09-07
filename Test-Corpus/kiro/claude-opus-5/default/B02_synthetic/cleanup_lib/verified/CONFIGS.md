# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

There is **no** runtime option, mode, flag, global, or `#ifdef` in the
library: `grep -n '#ifdef\|#if\|extern\|static\|struct'` on `src/lib.c` and
`include/lib.h` yields nothing. The only `#define`s are the two stringize
macros (`STRINGIZE`, `TO_STRING`), which are compile-time constant and expand
`TO_STRING(numbers)` to the literal text `"numbers"` — **not** the array
contents. So the entire configuration surface is *input shape*.

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| A. `switch (numbers[i])` case class | `10` (falls through to `20` ⇒ `+30`), `20` (`+20`), `30` (falls through to `40` ⇒ `+70`), `40` (`+40`), `default` (`+= numbers[i]`) | lines 48–62 |
| B. argument position `i` | `0..3` — the loop visits each of `a,b,c,d`; the case class is chosen independently per position | line 47 |
| C. `default`-value magnitude | negative / zero / positive / `INT_MIN` / `INT_MAX` / values adjacent to case labels | line 60 |
| D. accumulated `result` overflow | `int` `+=` wrapping at the 32-bit boundary | lines 50–60 |
| E. `print_result` label shape | empty / ASCII / format metacharacters / long / NULL | line 80 |
| F. `print_result` result value | `0`, negative, `INT_MIN`, `INT_MAX` | line 80 |
| G. `cleanup_resources` pointer | NULL / non-NULL heap pointer | line 84 |

## Public entry points

All three exported functions are tested **directly** through the `.so`
exports. `cleanup` is the composed pipeline (validate → accumulate →
allocate → format → print → release); `print_result` and `cleanup_resources`
are the lower-level entry points and are driven standalone as well as
implicitly via `cleanup`. Every row asserts the return value **and** the
byte-exact stdout captured by redirecting fd 1 around the call.

## Rows

| # | entry point(s) | configuration (options set + input shape) | randomized? (seed 0x5EED_C0DE) | [x] |
|---|----------------|--------------------------------------------|-------------------------------|-----|
| C1 | `cleanup` | axis A exhaustive × axis B: full cross-product of `{10,20,30,40,default}` over all 4 positions = **625** combinations; `default` slot filled with a fresh pseudo-random non-case `int` each time | 625 rows × 16 random default fillings | [x] |
| C2 | `cleanup` | all four arguments the same case label: `(10,10,10,10)`, `(20,…)`, `(30,…)`, `(40,…)` — pure fall-through accumulation | fixed | [x] |
| C3 | `cleanup` | exactly one case label, other three `default`, for each label × each of the 4 positions (16 combos) — proves the case class is position-independent | 64 random default fillings | [x] |
| C4 | `cleanup` | all four `default`, small magnitudes: uniform random in `-1000..=1000` | 4096 quadruples | [x] |
| C5 | `cleanup` | all four `default`, full `int` range: uniform random over `i32::MIN..=i32::MAX` (exercises axis D wrapping) | 8192 quadruples | [x] |
| C6 | `cleanup` | all four `default`, boundary values adjacent to the case labels: `{9,11,19,21,29,31,39,41}` cross-product | 4096 = 8⁴ exhaustive | [x] |
| C7 | `cleanup` | all four `default`, extreme values: `{i32::MIN, i32::MIN+1, -1, 0, 1, i32::MAX-1, i32::MAX}` cross-product (axis D overflow) | 2401 = 7⁴ exhaustive | [x] |
| C8 | `cleanup` | mixed: case labels interleaved with extreme `default` values (`INT_MAX,10,INT_MIN,30`, …) — overflow *and* fall-through together | 4096 random mixes over `{10,20,30,40} ∪ extremes` | [x] |
| C9 | `print_result` | label ∈ {empty, short ASCII, 4096-byte, format metachars, embedded NUL-free UTF-8} × result ∈ {0, ±random, INT_MIN, INT_MAX} | 2048 random (label-shape, result) pairs | [x] |
| C10 | `cleanup_resources` | non-NULL pointer from libc `malloc` (sizes 1, 50, 4096) — pointer must be released without crash | 3 sizes × 64 iterations | [x] |
| C11 | `cleanup` then `print_result` | end-to-end consumer sequence: run `cleanup` on a random quadruple, feed its return value into `print_result` with a random label, comparing the *combined* stdout of the pair | 2048 sequences | [x] |
| C12 | `cleanup` | repeated invocation (statefulness check): the same quadruple called 32× in a row must give an identical return value and identical stdout every time (no leaked accumulator/heap state) | 256 quadruples × 32 reps | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
build configuration is the default (empty) feature set. Verified with
`cargo metadata` — the feature map for the package is empty. `--no-default-features`
and the default build are therefore the same code path; both are exercised.

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)`
and no `add_executable`. The Rust crate declares `crate-type = ["cdylib"]`
and has no `src/main.rs` / `[[bin]]`. **No driver binary exists**, so the
stdout-of-binary comparison gate is not applicable; stdout is instead
compared per-call at the FFI boundary in every row above.
