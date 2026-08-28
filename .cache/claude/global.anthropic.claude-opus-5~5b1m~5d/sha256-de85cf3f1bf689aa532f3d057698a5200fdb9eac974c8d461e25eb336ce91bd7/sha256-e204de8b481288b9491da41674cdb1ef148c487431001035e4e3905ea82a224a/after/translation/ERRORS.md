# Differential verification log (C `driver` vs. Rust `driver`)

Ground truth: `c_src/` (never modified — verified: only `c_src/build/`, a build
artifact directory, was added).

Programs compared by *execution*, never by loading the Rust code as a library:

| program | build command | run command |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver A B C` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver A B C` |

Test suite: `translation/tests/differential.rs` (+ harness `tests/common/mod.rs`),
25 tests, 4 105 compared input vectors = 8 210 subprocess runs (measured by
instrumenting the harness with a counter and reverting it again). Everything is
compared: stdout bytes, stderr bytes, exit code and termination signal. No test
is `#[ignore]`d, disabled or skipped.

---

## 1. Phase A — build result

* `cargo build --release`: **succeeded on the first attempt, zero compile
  errors and zero warnings**, so there was nothing to fix in Phase A.
* C build: clean (`cmake` + `cmake --build`).

## 2. Mismatches found

### 2.1 Harness-level mismatch: `argv[0]` leaks into stderr (found and fixed in the tests)

`main.c` error path is

```c
fprintf(stderr, "%s requires 4 inputs\n", argv[0]);
exit(1);
```

The first naive comparison of the error path reports a stderr difference that is
*not* a translation bug: the C binary prints
`…/c_src/build/driver requires 4 inputs`, the Rust binary prints
`…/translation/target/release/driver requires 4 inputs`. Byte-identical stderr
is only meaningful when both processes are executed with the same `argv[0]`
(what a shell's `exec -a NAME prog` does).

Fix (in the harness, not in the program): every run goes through
`std::os::unix::process::CommandExt::arg0("driver")`, so the two programs see an
identical `argv[0]`. A separate test (`usage_error_prints_argv0_verbatim`) then
varies `argv[0]` on purpose — empty, containing spaces/tabs, containing
`printf`-like `%s %f %d`, and containing the invalid-UTF-8 bytes `\xff\xfe` — to
pin down that both programs echo those bytes verbatim and exit with code 1.

### 2.2 Mismatches in the Rust program: none found

Over every input class enumerated below — 4 105 input vectors in the committed
suite (8 210 subprocess runs), ~17 500 further input vectors in the ad-hoc sweeps
listed at the end of §3, and 30 106 strings compared *bit-for-bit* at the
`atof`/`%f` level — the Rust
binary produced identical stdout, identical stderr and identical exit status to
the C binary, in both the `debug` and the `release` profile.

The translation was already correct in the places where a C-to-Rust port
normally breaks. Each of those places is listed in §3 with the C behaviour it
has to reproduce and the evidence that it does; §4 shows that the test suite
actually detects a regression in each of them (i.e. the tests are not vacuous).

## 3. Input classes enumerated from the C source, and the traps in each

Branches in the C program:

* `main`: `argc != 4` → usage on stderr + `exit(1)`; otherwise
  `atof` ×3 → `VectorNormalizeFast` → `printf("%f %f %f\n")` → `return 0`.
* `atof` (= `strtod`): whitespace, sign, decimal form, hex (`0x`) form,
  `inf`/`infinity`, `nan`/`nan(chars)`, empty subject sequence → `0.0`,
  overflow → `±HUGE_VAL`, underflow → subnormal/zero; then a narrowing
  conversion to `float` that can overflow to `inf` or flush to signed zero.
* `DotProduct` (`q_shared.h:365`, the `#if 1` branch, so the macro — left to
  right in single precision) and `Q_rsqrt` (`q_math.c:494`, compiled because
  `idppc == 0`).
* `printf("%f")`.

| # | input class | C behaviour that must be reproduced | test |
|---|---|---|---|
| 1 | 0, 1, 2, 4, 5, 10, 200 arguments; empty-string arguments | usage line on stderr, exit 1, empty stdout | `usage_error_for_wrong_argument_counts`, `many_arguments` |
| 2 | odd `argv[0]` (empty, spaces, `%s`, non-UTF-8) | `%s` writes raw bytes | `usage_error_prints_argv0_verbatim` |
| 3 | ordinary vectors | `Q_rsqrt`'s *approximate* result (never `1/sqrt`); e.g. `3 4 5` → `0.424068 0.565424 0.706780`, not `0.424264 …` | `plain_vectors` |
| 4 | all-zero and signed-zero vectors | dot product `0` → `Q_rsqrt(0)` returns the raw magic constant `1.98e19`; `-0.0 * 1.98e19` prints `-0.000000` | `zero_and_signed_zero_vectors` |
| 5 | `nan`, `-nan`, `nan(payload)`, `inf`, `-inf` in all 12³ position combinations | NaN *sign* is observable (`nan` vs `-nan`); `-inf` input → `+inf` output; `dot = inf` → `Q_rsqrt` returns `-inf`, so a finite component prints `-inf` and a zero component prints `-nan` (`0 * -inf`) | `special_value_combinations`, `nan_payloads_and_spellings`, `infinity_spellings` |
| 6 | negative NaN dot product | `i = 0x5f3759dfu - (i >> 1)` **wraps around** (unsigned) — a plain `-` panics in a debug Rust build | `q_rsqrt_bit_hack_wraparound` |
| 7 | overflow/underflow of the float conversion and of the dot product | `3.4028236e38` → `inf`; components ≳ `1.8446743e19` make the dot product `inf`; components ≲ `3.74e-23` make it `0`, which yields un-normalized output like `0.001982` | `magnitudes_that_overflow_or_underflow`, `dot_product_overflow_boundary`, `dot_product_underflow_boundary` (bit-level sweeps of ±24/±32 ULP around both boundaries) |
| 8 | `strtod` forms and non-forms: `""`, `" "`, `"\t"`, `"\n"`, `"\x0b"`, `"\x0c"`, `"abc"`, `"+"`, `"-"`, `"."`, `"e5"`, `"1,5"`, `"--1"`, `" - 5"`, `"1e"`, `"1e+"`, `"1.5abc"`, `"5."`, `".5"`, `"1e2147483648"`, `"1e99999999999999999999"`, 51-digit integers, 400-digit fractions | no conversion → `0.0`; a partial prefix converts; an incomplete exponent falls back to the mantissa | `atof_decimal_and_garbage_forms` |
| 9 | hex floats: `0x`, `0x1p`, `0X1P-3`, `0x.8p1`, `0x1.fffffep127`, `0x1.ffffffp127`, `0x1p-149`, `0x1p-1075`, `0x1.00000000000008p0`, `0x…p0` with a 32-digit mantissa, `0xzz`, `0x1p3abc` | glibc parses hex floats (Rust's `str::parse` does not); `"0x"` alone converts the leading `0` | `atof_hexadecimal_forms` |
| 10 | correct-rounding boundaries: `2.2250738585072011e-308`, `4.9e-324`, `1.0000000596046447753906250` (exact float tie), `16777217`, `1.7976931348623159e308` | correctly rounded `strtod`, then round-half-to-even to `float` | `atof_rounding_boundaries` |
| 11 | `printf("%f")` ties | glibc rounds half to **even** on an exact decimal tie; the six triples used make a component land exactly on an odd multiple of 2⁻⁷ (e.g. `0.5390625` → `0.539062`, `0.0234375` → `0.023438`) | `printf_rounding_ties` |
| 12 | non-UTF-8 arguments (`\xff1.5`, `1.5\xff`, `\xc3\x28`, UTF-8 BOM, …) | `argv` is bytes, not text | `non_utf8_arguments` |
| 13 | 100 000-byte arguments (digits, zeros, hex, garbage, `1e` + 10 000 digits) | no truncation, no overflow, same value | `very_long_arguments` |
| 14 | flag-like arguments (`-h`, `--help`, `--`, `-`) | there is no option parsing; they are just unparsable numbers → `0.0` | `flag_like_and_option_arguments` |
| 15 | `LC_ALL`/`LANG`/`LC_NUMERIC` = `de_DE.UTF-8`, `fr_FR.UTF-8`, … | the C program never calls `setlocale`, so the decimal point stays `.` | `locale_does_not_change_formatting` |
| 16 | stdout that fails every write (`/dev/full`) and `/dev/null` | the return value of `printf` is ignored, the program still exits 0 and prints nothing on stderr | `failing_stdout_write_is_ignored_identically` |
| 17 | randomized: 300 exact float bit patterns, 300 random magnitudes (1e-45 … 1e45), 400 random strings over strtod's alphabet, 300 structured numbers (decimal/hex/long-zero forms) | — | `fuzz_*` (deterministic xorshift seeds, so failures are reproducible) |

Additional out-of-suite evidence (run manually during Phase B/C, all identical):

* 3 000 random triples, 3 375 special-value triples, 5 000 fuzz triples run
  against the C binary **and both** the release and debug Rust binaries (the
  debug build has overflow checks enabled, which would expose any wrapping
  arithmetic that is not written as `wrapping_*`).
* 5 500 triples built from exact float bit patterns and power-of-two sweeps.
* 30 106 strings compared bit-exactly at the library level by compiling
  `src/cstd.rs` into a scratch harness next to a `atof`/`printf` harness in C
  and diffing the raw `double` bits, the raw `float` bits and the `%f` text:
  0 differences. (Scratch harnesses only; nothing was added to the crate.)

## 4. Mutation audit — proof the tests would catch a regression

Each mutation was applied to `translation/src/…`, `cargo test` was run, and the
source was restored immediately afterwards (verified afterwards by grepping the
restored lines; the suite passes on the restored tree).

| mutation | result (`cargo test`, 25 tests) |
|---|---|
| `{:.6}` → `{:.7}` in `printf_f` | **caught** — 20 failed |
| `exit(1)` → `exit(2)` on the usage path | **caught** — 4 failed |
| drop the trailing `\n` from the output line | **caught** — 21 failed |
| `"nan"` → `"NaN"` | **caught** — 2 failed (`nan_payloads_and_spellings`, `special_value_combinations`) |
| `"-nan"` → `"nan"` (lose the NaN sign) | **caught** — 9 failed |
| `0x5f3759df.wrapping_sub(i >> 1)` → plain `-` | **caught** — 4 failed (debug-build overflow panic on a negative-NaN dot product) |
| `atof` → `str::parse::<f64>` (Rust-native parsing) | **caught** — 11 failed (hex floats, `inf`/`nan` spellings, trailing garbage, …) |
| `argv` bytes → `to_string_lossy()` | **caught** — 1 failed (`usage_error_prints_argv0_verbatim`) |
| perturb one input by 1e-7 after the `f32` narrowing | **caught** — 7 failed |
| `dot_product` via plain `f32` operators | not caught — see below |
| `q_rsqrt`'s `fpu::{mul,sub}` via plain `f32` operators | not caught — see below |

Two mutations were **not** caught, and that is correct — they are provably
unobservable on this target, which is worth recording so the next reader does
not treat them as gaps:

1. Replacing the `fpu::{mul,sub}` helpers inside `q_rsqrt` with plain `f32`
   operators. In `q_rsqrt` at most one operand of any operation can be NaN (the
   bit hack turns a NaN `number` into an ordinary normal float), and SSE
   propagates the single NaN operand with its sign either way, so the helper and
   the plain operator agree bit-for-bit.
2. Replacing `dot_product`'s `fpu::{add,mul}` with `x[0]*y[0] + x[1]*y[1] +
   x[2]*y[2]`. Rust evaluates that left-to-right in single precision and, at
   runtime, `addss`/`mulss` keep the destination (left) NaN — the same rule the
   helpers implement. It only matters when two *different* NaNs meet (e.g.
   `nan -nan 1`), and `special_value_combinations` covers those 12³ combinations
   and passes both ways.

The `fpu` helpers are therefore belt-and-braces, not dead weight: they make the
NaN-propagation rule explicit instead of leaving it to the optimizer (a
constant-folded `0.0 * -inf` in LLVM yields `+nan`, whereas the hardware — and
therefore the C program — yields `-nan`).

## 5. Known limitation of the harness

`argc == 0` (i.e. `execve` with an empty `argv`, where glibc's `%s` prints
`(null)` for the NULL `argv[0]`) cannot be produced through
`std::process::Command`, which always passes at least `argv[0]`, and is not
reachable from a shell. It is therefore covered by inspection only: `main.rs`
falls back to the literal `(null)` when `std::env::args_os()` is empty, matching
glibc. The adjacent, shell-reachable case (`argv[0] == ""`, via `exec -a ""`) is
covered by `usage_error_prints_argv0_verbatim`.

## 6. Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # C
cd ../../translation && cargo build --release                           # Rust
cargo test              # 25 differential tests, debug binary
cargo test --release    # same suite against the release binary
```

If `c_src/build/driver` is absent, the harness configures and builds the CMake
project into `target/tmp/c_build` instead, so it never writes into `c_src/`.
