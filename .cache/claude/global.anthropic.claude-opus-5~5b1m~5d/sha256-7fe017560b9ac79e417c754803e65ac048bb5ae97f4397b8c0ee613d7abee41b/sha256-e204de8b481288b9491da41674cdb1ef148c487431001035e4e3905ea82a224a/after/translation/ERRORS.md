# Differential verification of `c_src/src/main.c` vs. the Rust `driver`

Ground truth: the C program. Both programs are built and then run as
subprocesses with identical `argv`; stdout, stderr and the exit status are
compared byte for byte (`translation/tests/differential.rs`).

## How to reproduce

```sh
# C (ground truth)
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# -> c_src/build/driver          run as: ./driver <base> <exponent>

# Rust
cd translation && cargo build --release
# -> translation/target/release/driver

# Differential suite (builds the C program itself if c_src/build/driver is absent)
cd translation && cargo test
```

`argv[0]` is printed in the usage message, so the tests exec *both* binaries
with a fixed `argv[0]` of `driver` (via `std::os::unix::process::CommandExt::arg0`).
Without that, the usage message would differ purely because the two binaries
live at different paths — an artifact of the harness, not of the translation.

## Result: no output mismatches were found

Every input class listed below produced identical stdout, stderr and exit
status. In addition to the 25 tests in the suite, the translation was driven
through ad-hoc differential sweeps during verification (all clean):

| sweep | cases |
|---|---|
| random argument shapes (integers, decimals, exponents, hex, inf/nan, junk) | ~20 000 |
| random byte soup over `0-9 . e E + - x X p P a A f F n N i I t y ( ) _ \xff` + whitespace, argc 0–3 | ~22 000 |
| exact-decimal sweep around `2^-1074`, `2^-1022`, `DBL_MAX` (Python `decimal`) | ~4 200 |
| `pow` base/exponent grid across the overflow / underflow / pole cliffs | ~2 600 |
| `printf("%.2f")` tie and negative-zero cases | ~35 |

So this file records **no behavioural mismatches**. What it does record is the
verification gaps that were found, and the behaviours that had to be matched
exactly and were double-checked because they are the places a translation
normally breaks.

## Verification gaps found (test-coverage bugs, not translation bugs)

Because no mismatch surfaced, the suite itself was validated by mutation
testing: 26 deliberate mutations were injected into `src/main.rs` /
`src/strtod.rs` one at a time and `cargo test` was re-run. Each surviving
mutant is a branch the tests did not pin down. Four survived the first round:

1. **Exact subnormals were never exercised.**
   Mutant: delete the `is_exact_subnormal` exemption in `decimal_value`, i.e.
   report `ERANGE` for *every* result below `DBL_MIN`. It survived, because
   every "tiny" literal in the suite (`5e-324`, `1e-320`, …) is inexact and
   therefore *does* raise `ERANGE`.
   Cause: reaching the exemption needs the *exact* decimal expansion of a
   multiple of `2^-1074`, which is ~750 digits long.
   Fixed by `exact_subnormals_do_not_raise_erange`, which builds the literal as
   `k * 5^1074` followed by `e-1074` (since `2^-n == 5^n * 10^-n`). glibc
   converts those without `ERANGE`, and so does the Rust program; the half-way
   values `(2k+1) * 5^1075 e-1075` do raise it in both.

2. **The exact glibc underflow threshold was never exercised.**
   Mutant: delete the `value.abs() == f64::MIN_POSITIVE && below_underflow_threshold(...)`
   check. It survived the first round.
   Cause: it only fires for values that round *up* to exactly `DBL_MIN` while
   being below `2^-1022 - 2^-1076`.
   Fixed by `underflow_threshold_is_exact`, which tests `(2^54-1) * 2^-1076`
   exactly (no `ERANGE` in C), that value minus one in the last decimal digit
   (`ERANGE` in C), and one past it.

3. **Hex ties were never exercised.**
   Mutant: change hex rounding from ties-to-even to half-up. It survived,
   because every hex tie in the suite was small enough that one ulp is invisible
   through `%.2f`.
   Fixed by `hex_rounding_ties`, which scales the tie up (`0x20000000000001p10`
   = `2^53+1` scaled by `2^10`) so the ulp shows up in the printed integer part.

4. **The "tiny but rounds up to DBL_MIN" hex shortcut was never exercised.**
   Mutant: delete the early `return (f64::MIN_POSITIVE, false)` in
   `round_to_double`. It survived.
   Fixed by `hex_tiny_values_at_dbl_min` (`0x1.fffffffffffffffp-1023` and
   friends): glibc raises nothing there, even though the value is below the
   smallest normal.

A fifth mutant (shrinking the decimal-exponent saturation cap in
`parse_exp_digits` from `10^12` to `10^3`) initially survived and was **not**
killed by 700-digit mantissas — the saturated exponent is still clamped
identically. It is only observable once the mantissa is long enough to dominate
the clamp (`lo = -(len + 400)`), i.e. beyond ~9 600 significant digits.
`long_mantissa_with_saturating_exponent` now covers 10 000- and 20 000-digit
arguments, which kills it.

Final mutation score: **25 of 26 killed**. The one survivor is an equivalent
mutant, not a gap: deleting `j = dot` in the decimal "lone dot" rollback cannot
change any output, because the very next statement is
`if digits.is_empty() { return no_conversion; }`, which discards `j`. No input
can distinguish it.

## C behaviours that had to be replicated exactly (and were)

- **An empty argument is accepted.** `strtod("")` performs no conversion and
  leaves `endptr == nptr`, which points at the terminating NUL, so
  `*endptr == '\0'` and the check passes with a silent `0.0`. `driver "" 2`
  therefore prints `Result: 0.00` and exits 0 in both programs. `driver " " 2`
  is rejected, because `endptr` then points at a space.
  (`empty_argument_is_accepted_as_zero`, `invalid_base`)
- **Check order.** `ERANGE` is tested before the `*endptr` check, and the base
  is fully validated before the exponent is even parsed, so a bad base wins
  over a bad exponent. (`base_is_checked_before_exponent`)
- **Only `errno` decides.** `pow` is not re-inspected; the C code branches on
  `EDOM`/`ERANGE` alone. Notably glibc's `pow` leaves `errno` clear for
  inexact subnormal results (`1e-300 ^ 1.05` prints `Result: 0.00`, exit 0) but
  sets `ERANGE` once the result flushes to zero (`3 ^ -700`), and sets `ERANGE`
  for the pole error `pow(±0, -1)`. Special operands (`nan`, `inf`) raise
  nothing, so `driver nan 2` prints `Result: nan` and exits 0.
  (`pow_no_errno`, `pow_range_errors`, `infinity_and_nan_forms`)
- **`printf("%.2f")` details.** Ties round to even on the exact binary value
  (`0.125 -> 0.12`, `0.375 -> 0.38`), negative zero keeps its sign
  (`-0.00`), non-finite values print `nan` / `-nan` / `inf` / `-inf` with no
  padding, and huge finite results print their full exact expansion
  (`1e300 ^ 1` is 309 digits). (`printf_formatting`)
- **`strtod` grammar.** Leading `\t \n \v \f \r`, `+`/`-`, `inf`/`infinity`/
  `nan`/`nan(chars)` in any case, hex `0x…p±…` (including `0x` alone
  converting just the leading `0` and leaving `x` unconsumed, which then fails
  the `*endptr` check), and saturating exponent digit runs.
  (`whitespace_and_sign_forms`, `infinity_and_nan_forms`, `hex_forms`,
  `invalid_base`)
- **Bytes, not text.** Arguments are taken as raw bytes, so invalid UTF-8
  (`"\xff\xfe"`, `"1\xff"`) reaches `strtod` and is echoed back into the error
  message unchanged. (`non_utf8_arguments`)
- **`argc` branch.** 0, 1, 3 and 4 arguments all take the usage path and exit 1.
  With `argc == 0` (reachable only via a raw `execv` with an empty `argv`) both
  programs print `Usage:  base exponent`. (`arity_errors`)

## Notes on scope

- Nothing in `c_src/` was modified. The only thing written under it is
  `c_src/build/`, the cmake output directory created by the build command in the
  task instructions. If that directory is absent, the test suite configures its
  own out-of-source cmake build under `translation/target/c_build` instead.
- No test is `#[ignore]`d, skipped or otherwise disabled. `assert_same` also
  refuses to pass on a comparison where the C program produced no output at
  all, so a mutually-silent pair cannot masquerade as a match.
