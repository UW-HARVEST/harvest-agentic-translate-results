# Differential verification log

The C program (`c_src/src/main.c`) reads one `double` with `scanf("%lf", &f)` and
prints `printf("%llx %a %.4f\n", bits(f), f, f)`. Everything observable therefore
comes from three places: glibc's `scanf` float grammar, glibc's `%a`, and
glibc's `%.4f`. The Rust program re-implements all three.

Reference commands used for every comparison:

- C: `c_src/build/driver` (built with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`)
- Rust: `translation/target/{debug,release}/driver` (`cargo build --release`; the
  test harness uses `$CARGO_BIN_EXE_driver`)

Both were driven as subprocesses with identical stdin, comparing stdout, stderr
and exit status (including termination by signal).

## Mismatch found and fixed

### 1. Exit status when stdout has no reader (SIGPIPE)

- Input: any (`1.5\n`), with stdout connected to a pipe whose read end is already
  closed.
- C: killed by `SIGPIPE`, wait status signal 13, empty stdout/stderr.
- Rust before the fix: exited 0. The Rust runtime sets `SIGPIPE` to `SIG_IGN`
  before `main`, so the failing `write` only returned `EPIPE`, which `driver()`
  discards.
- Cause: a runtime default in Rust that has no counterpart in C, not a
  translation error in the parsing or formatting code.
- Fix: `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE, SIG_DFL)`
  as the first statement of `main`, restoring the C disposition. Covered by
  `sigpipe_when_the_stdout_reader_is_gone`.

No other mismatch was observed. The remainder of this file records the behaviors
that were verified against the C binary, because each of them is a place where a
plausible translation would have diverged.

## Behaviors confirmed against the C binary (no divergence)

`scanf("%lf")` grammar:

- Failed or absent conversion leaves `f` at its initializer, so empty input,
  whitespace-only input, `abc`, `+`, `-`, `.`, `.e5`, `e1` all print
  `0 0x0p+0 0.0000` with exit 0 and empty stderr.
- Leading whitespace is skipped across newlines (`"\t\n  7.25"` reads `7.25`);
  the accepted `isspace` set is the C-locale one (`' '`, `\t`, `\n`, `\v`, `\f`,
  `\r`). `\x1c` is not whitespace, so `"\x1c 1"` converts nothing.
- Trailing input is simply left unread: `1 2`, `1/3`, `1.5\xff` all read the
  first number.
- An `e`/`p` exponent with no digits backtracks: `1e`, `1e+`, `0x1p`, `0x1.8p+q`
  read the significand only.
- `0x` with nothing usable after it is a *matching failure*, so the sign is lost
  too: `-0x` yields `+0.0` (bits `0`), while `-0x.` succeeds through strtod
  backtracking and yields `-0.0` (bits `8000000000000000`). This asymmetry is
  real and is reproduced.
- `inf` and `infinity` are accepted case-insensitively, but a partially typed
  `infi`, `infin`, `infini`, `infinit` is a matching failure (once a fourth `i`
  is consumed the full spelling is required), whereas `infx`, `inf.`, `inf1`
  read `inf`.
- `nan`, `nan()`, `nan(1)`, `nan(0x5)`, `nan(abc)`, unterminated `nan(123` all
  produce the default quiet NaN `7ff8000000000000`; this glibc does **not**
  install the payload. The sign is kept: `-nan` gives `fff8000000000000`.
- Overflow gives `±inf` and underflow gives `±0.0` with the sign preserved
  (`-1e-99999` → `8000000000000000`), and `0e<huge>` stays zero.

`%a` formatting:

- Zero prints `0x0p+0` / `-0x0p+0`.
- Normals print `0x1.<hex>p±<dec>` with trailing mantissa zeros removed and the
  `.` dropped when the mantissa is zero (`0x1p+3`).
- Subnormals are **not** renormalized: glibc keeps the raw mantissa with a
  leading `0` digit and a fixed exponent, e.g. `5e-324` → `0x0.0000000000001p-1022`,
  `1e-320` → `0x0.00000000007e8p-1022`.
- Non-finite values print `inf`, `-inf`, `nan`, `-nan`.

`%.4f` formatting:

- The exact binary value is rounded, ties to even. Exact ties exist only for odd
  multiples of 2^-5: `0.03125` → `0.0312`, `0.09375` → `0.0938`,
  `1048575.03125` → `1048575.0312`. Rust's `{:.4}` agrees.
- Large values print their full expansion (309 digits for `1e308`,
  `0x1.fffffffffffffp+1023`), matching glibc digit for digit.
- Non-finite values print `inf`, `-inf`, `nan`, `-nan`; `-0.0` prints `-0.0000`.

`%llx`: prints the raw 64-bit pattern with no padding, so the sign bit of `-0.0`
and of NaN is visible; this is why signed zero and NaN sign had to be exact.

## Coverage actually executed

- `translation/tests/differential.rs`: 15 tests, ~1500 input cases, comparing
  stdout, stderr and exit status. Includes the whitespace/no-conversion classes,
  every decimal and hexadecimal form, all `inf`/`nan` spellings and their partial
  prefixes, subnormal and overflow/underflow boundaries, `%.4f` tie cases, raw
  non-UTF-8 and embedded-NUL input, inputs up to 20 000 bytes, a
  prefix × body × suffix cross product of the reader grammar (7 × 34 × 3), a
  deterministic sweep over powers of two, boundary bit patterns and 300
  pseudo-random bit patterns fed in both decimal and hex-literal form, and the
  SIGPIPE case above.
- Additional throwaway sweeps during development compared ~24 000 further inputs
  (random 64-bit patterns rendered several ways, random junk strings over a
  float-ish alphabet including NUL and `\r`, 800-digit decimal and hex
  significands, subnormal and near-`DBL_MAX` patterns): no mismatch.
- `cargo test` and `cargo test --release` both pass; nothing is `#[ignore]`d.
- `c_src/src/main.c` and `c_src/CMakeLists.txt` are unmodified (only the
  `c_src/build/` output directory was created, by the documented build command).
