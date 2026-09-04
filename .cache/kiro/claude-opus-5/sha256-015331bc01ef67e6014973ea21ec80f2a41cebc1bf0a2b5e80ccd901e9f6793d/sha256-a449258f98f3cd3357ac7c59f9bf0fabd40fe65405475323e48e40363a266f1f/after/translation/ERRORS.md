# Differential verification of the C → Rust translation

The C program (`c_src/`) is the ground truth. Both binaries were built and then
run as subprocesses over the same arguments, comparing stdout, stderr and exit
status byte for byte.

## How each program is run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver A B C` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver A B C` |

Both build with no errors and no warnings.

`argv[0]` is echoed by the C error path (`fprintf(stderr, "%s requires 4
inputs\n", argv[0])`), and the two executables live at different paths. The
tests therefore force the same `argv[0]` for both processes
(`std::os::unix::process::CommandExt::arg0`), which is what makes a byte-for-byte
stderr comparison meaningful rather than a comparison of two file paths.

## Enumerated input classes

Everything the program branches on, derived from reading `c_src/src/main.c`,
the `VectorNormalizeFast` / `DotProduct` definitions in `c_src/inc/q_shared.h`,
and `Q_rsqrt` in `c_src/src/q_math.c`:

* `argc`: 0, 1, 2, 3, **4 (the only success case)**, 5, 6, 7 — plus `argv[0]`
  values that are empty, non-UTF-8, `printf`-like and 300 bytes long.
* `atof` parse branches: nothing convertible, leading whitespace, sign-only,
  bare `.`/`e`, decimal with/without integer or fractional part, exponents
  (well-formed and truncated), hex floats (`0x`, `0x.`, `0x1p...`, and the
  `0x`-with-no-digits case where `strtod` consumes only the `0`), `inf` /
  `infinity` and every truncated prefix, `nan` with and without a
  parenthesised payload, over- and underflow to `±inf` / `±0`, `double`
  subnormals, and 1000-digit inputs.
* `Q_rsqrt` domains: zero, negative zero, normals, `float` subnormals,
  `DotProduct` overflowing to `inf`, `DotProduct` underflowing to `0`, `±inf`
  inputs, NaN inputs, and invalid operations (`inf * 0`, `inf - inf`).
* `printf("%f")` formatting: negative zero, `inf` / `-inf`, `nan` / `-nan`, and
  exact rounding ties at the sixth decimal.
* Non-UTF-8 argument bytes.

Beyond `translation/tests/differential.rs`, the two binaries were compared over
roughly 100,000 additional generated argument triples (random `float` bit
patterns spelled as exact hex floats, NaN-dense combinations, and random
garbage strings) with no remaining differences. `atof` was also compared
against glibc `strtod` over ~244,000 strings, and `%f` against glibc `printf`
over ~240,000 `float` bit patterns; both are bit-identical.

---

## Mismatch 1 — NaN sign lost through `DotProduct` (found, fixed)

**Symptom.** Whenever two arguments were NaNs of *different* signs, the sign of
the printed NaNs differed:

| argv | C | Rust (before) |
|---|---|---|
| `nan -nan 1` | `nan -nan nan` | `nan -nan -nan` |
| `-nan nan 1` | `-nan nan -nan` | `-nan nan nan` |
| `nan 1 -nan` | `nan nan -nan` | `nan -nan -nan` |
| `1 nan -nan` | `nan nan -nan` | `-nan nan -nan` |

Exit status and stderr matched; only stdout differed. Roughly 0.2% of randomly
generated triples hit this, and every failing case involved at least two NaNs
with opposite signs.

**Cause.** NaN propagation is *not* commutative on x86-64. For the scalar SSE
instructions `addss`/`subss`/`mulss` the destination operand is the left-hand
operand of the C expression, and the rule (verified by compiling a probe
program on this target) is:

* if the left operand is a NaN, the result is that NaN — sign and payload
  preserved, quieted if it was signaling;
* otherwise, if the right operand is a NaN, the result is that NaN, quieted;
* otherwise, if the operation is invalid (`inf - inf`, `0 * inf`) the result is
  the x86 QNaN "floating-point indefinite", `0xFFC0_0000` — note the **set sign
  bit**, so `printf("%f")` renders it as `-nan`.

Rust's `a + b` and `a * b` carry no such operand-order guarantee, and LLVM
does in fact commute them: for
`v[0]*v[0] + v[1]*v[1] + v[2]*v[2]` it emitted the additions with the sources
swapped relative to gcc. With ordinary values that is invisible, but it changes
*which* NaN survives the sum. The surviving NaN becomes `Q_rsqrt`'s argument,
its sign is carried unchanged through the whole of `Q_rsqrt`, and it is then
the sign printed for every vector component that is not itself a NaN.

**Fix.** `translation/src/fops.rs` adds `fadd`, `fsub` and `fmul`, which
implement the propagation rule above explicitly, and the three call sites now
use them with the C expressions' operand order preserved:

* `dot_product` — `fadd(fadd(fmul(x0,y0), fmul(x1,y1)), fmul(x2,y2))`, matching
  the left-associated `DotProduct` macro at `q_shared.h:365`;
* `q_rsqrt` — `fmul(number, 0.5)` and
  `fmul(y, fsub(threehalfs, fmul(fmul(x2, y), y)))`;
* `vector_normalize_fast` — `v[i] = fmul(v[i], ilength)`, since `v[i] *=
  ilength` puts `v[i]` in the destination.

This also pins down the two invalid-operation results that the C program
prints as `-nan` (`inf 0 0` → `-inf -nan -nan`), so they no longer depend on
LLVM choosing not to constant-fold.

Covered by the `mixed_sign_nans_select_the_surviving_nan`,
`invalid_operations_produce_negative_nan` and `nans_mixed_with_infinities`
tests.

---

## Checked and found already correct

These were all candidate mismatches that testing cleared, recorded so the next
reader does not have to re-derive them:

* **`atof` on unparsable input.** `atof` reports no errors, so `driver abc def
  ghi` prints `0.000000 0.000000 0.000000` and exits 0. The Rust
  reimplementation in `src/clib.rs` matches glibc `strtod` bit for bit over
  ~244,000 strings, including hex floats, `0x` with no digits (only the leading
  `0` is consumed), truncated `infinity` prefixes, `nan(payload)`, `double`
  subnormal rounding boundaries and 1000-digit mantissas. Verified in both the
  `dev` and `release` profiles, so the `u128` shifts in `compose` do not trip
  debug overflow checks.
* **`%f` formatting.** Rust's `{:.6}` agrees with glibc's `%f` on all ~240,000
  `float` bit patterns tried, including every exact tie at the sixth decimal
  (odd multiples of `1/128`, where both round half to even) and `-0.000000`.
  glibc prints non-finite values as `nan` / `-nan` / `inf` / `-inf` without a
  precision field, which `format_f` special-cases.
* **`argc == 0`.** Executing the binary with an empty `argv` produces
  `" requires 4 inputs\n"` and exit 1 from both programs.
* **Non-UTF-8 arguments.** `main.rs` reads `args_os()` as raw bytes, so
  arguments that are not valid UTF-8 are parsed and echoed exactly as C's
  `char*` arguments are. (A NUL byte cannot occur inside an `argv` entry, so it
  is not a reachable input class.)
* **`Q_rsqrt`'s bit hack.** `f32::to_bits` / `f32::from_bits` reproduce the
  `memcpy` punning, and `0x5f3759dfu32.wrapping_sub(i >> 1)` reproduces the
  `uint32_t` subtraction. The subtraction can never yield NaN bits, so the
  final multiply's destination operand is always a real number.
* **Unreached C code.** `idppc` is `0` (`q_shared.h:70`), so the `#if !idppc`
  definition of `Q_rsqrt` is the one compiled, and `#if 1` (`q_shared.h:363`)
  selects the macro form of `DotProduct`. Nothing else in `q_math.c` is
  reachable from `main`, which calls only `atof`, `VectorNormalizeFast`,
  `printf`, `fprintf` and `exit`.

## Status

* Both programs build cleanly.
* `cargo test` in `translation/`: 25 tests, all passing, in both the `dev` and
  `release` profiles. No test is `#[ignore]`d, skipped or disabled.
* Nothing in `c_src/` was modified.
