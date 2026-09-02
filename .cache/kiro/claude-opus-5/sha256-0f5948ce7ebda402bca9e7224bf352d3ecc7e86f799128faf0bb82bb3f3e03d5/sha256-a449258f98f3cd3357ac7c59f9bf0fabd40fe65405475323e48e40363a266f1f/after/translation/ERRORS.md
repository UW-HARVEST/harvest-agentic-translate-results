# Differential findings: C reference vs. Rust translation

Reference: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Translation: `translation/` (`target/release/driver`).

The program is a single-value oracle: it reads one `float` with `scanf("%f", &x)`
and prints the four bytes of that `float`'s object representation as lowercase
hex, host byte order, followed by `\n`. stderr is always empty and the exit
status is always 0, so **every** observable difference is a difference in the
value `scanf` stored — or in whether it stored anything at all.

That last point drives the test design. `x` is initialized to `0.f`, and on a
matching failure `scanf` leaves the object untouched, so a failed conversion is
indistinguishable from a successful conversion of `0`. The two cases separate
only when the input carries a leading `-`: a failure prints `00000000` (`+0.0f`),
a success prints `00000080` (`-0.0f`). Every failure-path input in
`tests/differential.rs` is therefore also exercised with a `-` prefix, via
`with_all_prefixes`.

## Mismatches found

### 1. `0x` prefix with neither a hex digit nor a `.` must be a matching failure

* Status: fixed.
* Fixed in: `src/cfloat.rs`, the hex branch of `strtof`.
* Found by: a randomized token sweep over the float-grammar alphabet with a
  forced `-` prefix (now `random_token_soup`); minimized to `-0x`.
* Regression test: `hex_prefix_without_digits`.

Failing inputs, all of the same shape — `0x`/`0X` followed by something that is
neither a hex digit nor `.`:

| input    | C (correct) | Rust (before fix) |
| -------- | ----------- | ----------------- |
| `-0x`    | `00000000`  | `00000080`        |
| `-0X`    | `00000000`  | `00000080`        |
| `-0xg`   | `00000000`  | `00000080`        |
| `-0xp1`  | `00000000`  | `00000080`        |
| `-0x+1`  | `00000000`  | `00000080`        |
| `-0xz9`  | `00000000`  | `00000080`        |
| `-0x\n1` | `00000000`  | `00000080`        |

Cause: the translation treated a digitless hex prefix as "`strtof` converts only
the leading `0`", returning a signed zero. That is what a bare
`strtof("-0x")` does, but `scanf` is not a bare `strtof`. glibc's `vfscanf`
collects the token itself and tracks whether it saw a digit or a decimal point
*after* the `0x` prefix; when it saw neither, the conversion is a matching
failure and nothing is stored, so `x` keeps its initial `+0.0f`. The Rust code
stored `-0.0f` instead, flipping the sign bit.

The neighbouring case must not be broken while fixing this: a collected `.`
satisfies the "saw a decimal point" condition, the token *is* handed to
`strtof`, and `strtof` then rejects the hex form and converts just the leading
`0`. So the sign does survive there:

| input     | C          | Rust (after fix) |
| --------- | ---------- | ---------------- |
| `-0x.`    | `00000080` | `00000080`       |
| `-0x.p1`  | `00000080` | `00000080`       |
| `-0x.g`   | `00000080` | `00000080`       |
| `-0x..`   | `00000080` | `00000080`       |

The fix keys on exactly that: after `0x` with no hex digits anywhere, succeed
with a signed zero if the next byte is `.`, otherwise report a matching failure.

Note the contrast with the decimal form, where a lone `.` has no leading `0` for
`strtof` to fall back on: `-.` consumes nothing, so it fails and prints
`00000000`. The translation already handled that (`parse_dec` returns `None`),
and `matching_failures_leave_the_initial_value` covers it.

## Behaviours confirmed correct (no change needed)

Each was probed specifically because it is a plausible place for a translation
to drift, and each already matched:

* Matching failure vs. `-0`, across every non-numeric leading byte — including
  all 256 byte values in four positions (`every_byte_value_in_every_position`).
* `inf`/`infinity`: `vfscanf` accepts only a 3- or 8-character match, so `infi`,
  `infin`, `infini` and `infinit` are matching failures, not infinity, while
  `infx` and `infinityy` succeed on their 3- and 8-character prefixes.
  Case-insensitive throughout.
* `nan`: the optional `(n-char-sequence)` payload is never collected, so
  `nan(123)` yields a plain quiet NaN rather than a NaN carrying payload 123.
  `-nan` sets the sign bit (`0000c0ff`).
* Incomplete exponents back off without consuming: `1e` → `1.0`, `1e+` → `1.0`,
  `0x1p` → `1.0`.
* Signed zero for every zero-valued token: `-0`, `-0.`, `-.0`, `-0e999`, `-0x0`.
* Round-to-nearest-ties-to-even at exact decimal midpoints between adjacent
  binary32 values, including the `+0`/smallest-subnormal tie, the
  subnormal→normal boundary, and the `2^24` integer-spacing boundary.
* Overflow to `inf` and underflow to `0`, including the half-ulp-above-`FLT_MAX`
  tie (`3.4028236692093846e38`).
* Exponent fields that overflow `int` (40-digit exponents, `1e2147483648`), in
  both decimal and hex forms.
* Tokens up to 200 000 bytes, and leading-zero runs that must not consume
  significand precision.
* `scanf`'s leading-whitespace skip crossing newlines, and the fact that only
  the first conversion runs, so trailing lines are never read.
* Empty input, whitespace-only input, `/dev/null` and a closed descriptor on
  stdin all print `00000000`.
* `argv` is ignored (`main()` takes no parameters).

## Coverage

`c_src/src/main.c` has no data-dependent branch of its own: `print_hex` always
loops `sizeof(float)` == 4 times and `main` discards `scanf`'s return value. All
branching that the output depends on lives inside the `%f` conversion, so the
input classes enumerated above *are* the branch coverage for this program.

`cargo test` runs 23 differential tests over roughly 6 000 inputs, comparing
stdout, stderr and exit status byte for byte. Beyond the committed suite, the
translation was checked against the C binary over ~30 000 additional generated
inputs (structured tokens, random soup over the float alphabet, random
full-byte-range garbage, every corner of the float bit space round-tripped
through `%a`/`%e`/`%f`, and exact rounding ties) with no remaining differences.

The suite was verified to be capable of failing: reverting the `0x`-prefix fix
makes `hex_prefix_without_digits` fail on input `-0x`, and restoring it makes the
suite pass again. No test is `#[ignore]`d, skipped or disabled.
