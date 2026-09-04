# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

The C program is the ground truth. Both programs are built and run as
executables and compared on stdout bytes, stderr bytes and exit status.

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver <base> <exponent>`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver <base> <exponent>`
- Tests: `cd translation && cargo test` (25 tests, none ignored or skipped)

## Summary

The Rust translation as delivered produced **no behavioral mismatches**. It was
verified against 8057 argument vectors (hand-enumerated branch inputs, an
exhaustive product of 44 interesting argument fragments, and a seeded random
sweep) with zero divergence in stdout, stderr or exit status.

Two problems *were* found and fixed, both in the verification harness rather
than the translation. They are recorded below because a harness that reports a
false difference — or that misses a real one — is itself a defect.

---

## H1 — False mismatch: `argv[0]` in the usage message

**Symptom.** Every `argc != 3` invocation differed:

```
C: Usage: c_src/build/driver base exponent
R: Usage: translation/target/release/driver base exponent
```

**Cause.** Not a translation defect. The C code prints `argv[0]`:

```c
fprintf(stderr, "Usage: %s base exponent\n", argv[0]);
```

The two binaries live at different paths, so a naive comparison reports a
difference on all 101 wrong-argc vectors even though both programs are correct.

**Fix.** The harness pins `argv[0]` to the literal `driver` for both children
via `std::os::unix::process::CommandExt::arg0`, so the only variable is the
program's own behavior. With this in place all 101 vectors agree. Confirmed
independently outside the test suite with
`bash -c 'exec -a driver "$0" "$@"' <path> <args...>`.

## H2 — Missed mismatch: no coverage of verbatim argument echoing

**Symptom.** Found by mutation testing, not by the initial suite. Injecting
`let base_arg = &argv[1].to_ascii_lowercase();` into the Rust program — which
corrupts the `%s` echo in both base error messages — left all 22 tests passing.

**Cause.** Every argument in the suite that reached an echoing error path was
already lowercase or caseless (`abc`, `2abc`, `1e999`, `0x`), so a
case-normalising bug was invisible. The uppercase inputs present (`INF`, `NAN`,
`0X1P4`) all succeed and never reach an echo.

**Fix.** Added `arguments_are_echoed_verbatim`, covering uppercase invalid input
(`ABC`, `2ABC`, `MiXeDcAsE`, `NaNx`, `0XG`), uppercase range-error spellings
(`1E999`, `1E-999`, `0X1P9999`) whose original casing must survive into the
message, interior whitespace (`1 2 3`, `\t A B \n`), quotes (`it's`), and
format-specifier-looking arguments (`%s%d%n`) that must be echoed as data. The
mutation is now caught.

---

## Behaviors that would have been mismatches in a naive translation

The delivered Rust code already handles each of these; they are listed because
they are the places where a plausible rewrite diverges, and each now has a
regression test.

| C behavior | What a naive Rust port would do | Test |
|---|---|---|
| `strtod` accepts leading whitespace, `+`/`-`, hex floats (`0x10`, `0X1P4`), `inf`/`infinity`/`nan(...)`, bare `5.` and `.5` | `str::parse::<f64>()` rejects hex floats and `nan(1)`, and rejects leading whitespace | `strtod_accepted_forms` |
| `strtod` reports leftover characters via `endptr`; total failure leaves `endptr == nptr`, so `""` is silently accepted as `0.0` | Treats `""` as a parse error and exits 1 | `empty_argument_is_accepted_as_zero` |
| `" "` (whitespace only) consumes nothing, so `*endptr` is the space → *invalid* branch, unlike `""` | Lumps `""` and `" "` together | `empty_argument_is_accepted_as_zero` |
| `strtod` sets `ERANGE` on overflow **and** on gradual underflow, so `1e-308` and `5e-324` are range errors | Parses them fine and continues | `base_range_error`, `exponent_range_error` |
| `ERANGE` is checked **before** `*endptr != '\0'`, so `1e999xyz` reports the range error, not invalid input | Checks trailing garbage first | `base_range_error_wins_over_trailing_garbage` |
| Base is fully validated before the exponent is touched, so `abc def` names only the base | Validates both and reports the exponent, or reports both | `base_is_validated_before_exponent` |
| `pow`'s `EDOM`/`ERANGE` are libm side effects, not properties of the return value | Inspects `result.is_nan()` / `is_infinite()` instead, which misclassifies `pow(inf, 2)`, `pow(nan, 2)` and `pow(1, nan)` as errors | `pow_domain_error`, `pow_range_error`, `formatting_infinity_and_nan` |
| `pow(0, -1)` is a pole error that glibc reports through `ERANGE`, not `EDOM` | Assumes divide-by-zero is a domain error | `pow_range_error` |
| glibc `%.2f` rounds exact binary halfway values to even: `0.125` → `0.12` | Rounds half away from zero → `0.13` | `formatting_rounding_halfway` |
| glibc `%.2f` prints `nan` / `-nan` / `inf` / `-inf` | Rust `{:.2}` prints `NaN` for both NaN signs | `formatting_infinity_and_nan` |
| `%.2f` on `-0.0` prints `-0.00` | Some formatters drop the sign | `formatting_signed_zero` |
| `%.2f` near `DBL_MAX` expands to ~309 exact integer digits | Truncates or switches to exponent notation | `formatting_very_large_values` |
| Error messages echo raw `argv` bytes through `%s`, which need not be UTF-8 | `std::env::args()` panics on invalid UTF-8; `to_string_lossy` substitutes U+FFFD | `non_utf8_arguments` |
| `argc != 3` (not `< 3`), so 4+ arguments is also a usage error | Accepts extra arguments | `wrong_argument_count` |
| Neither program calls `setlocale`, so both stay in the `C` locale and `1,5` is invalid regardless of `LC_ALL` | A locale-aware parser would accept `1,5` under `de_DE` | `locale_does_not_change_behavior` |

## Notes on two things deliberately *not* changed

- **`f64::powf` vs FFI `pow`.** Replacing the FFI call with `base.powf(exponent)`
  did not break any test. That is correct, not a coverage gap: on x86-64 Linux
  `f64::powf` lowers to a call to glibc's `pow` (verified — the binary carries an
  undefined reference to `pow@GLIBC_2.29`) and therefore sets `errno` to 34/33
  identically. The FFI call is kept because it makes the `errno` dependency
  explicit rather than relying on that lowering.
- **`argc == 0`.** The Rust code has a `(null)` fallback for a missing `argv[0]`.
  It is unreachable on this kernel: `execve` with an empty `argv` still yields
  `argc == 1` with `argv[0]` pointing at an empty string. Verified with a helper
  that calls `execve(path, {NULL}, {NULL})` — both programs print
  `Usage:  base exponent`, agreeing.

## Harness confidence

The suite was mutation-tested by injecting 14 deliberate defects into a scratch
copy of `translation/` (never the real tree, never `c_src/`). Every
behavior-changing mutation was caught, including an exit-status-only change that
a stdout-only comparison would have missed:

| Injected defect | Caught |
|---|---|
| base `ERANGE` exits 2 instead of 1 (stdout and stderr unchanged) | yes — 5 tests |
| base `ERANGE` check removed (validation order) | yes — 5 tests |
| `EDOM` / `ERANGE` pow branches swapped | yes — 5 tests |
| `pow` errno never reported | yes — 5 tests |
| `pow` operands swapped | yes — 14 tests |
| wrong `ERANGE` constant (34 → 99) | yes — 10 tests |
| wrong `EDOM` constant (33 → 98) | yes — 3 tests |
| trailing-character check disabled | yes — 8 tests |
| `strtod` replaced with `str::parse` | yes — 3 tests |
| `%.2f` result perturbed by 1e-7 | yes — 6 tests |
| trailing newline dropped from `Result:` | yes — 12 tests |
| `-nan` printed as `nan` | yes — 2 tests |
| base error message sent to stdout instead of stderr | yes — 7 tests |
| usage message wording changed | yes — 1 test |
| `argc != 3` weakened to `argc < 3` | yes — 1 test |
| base argument lowercased before echo | yes, after H2 (no, before) |
