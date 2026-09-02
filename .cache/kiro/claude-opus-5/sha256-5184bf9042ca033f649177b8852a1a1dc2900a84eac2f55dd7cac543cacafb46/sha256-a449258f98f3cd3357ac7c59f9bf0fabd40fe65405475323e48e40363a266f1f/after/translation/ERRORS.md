# Mismatches found while verifying the C → Rust translation

The C program (`c_src/`) is the ground truth. Every difference below was fixed
in the Rust crate; nothing in `c_src/` was touched.

Reference commands:

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- Tests: `cd translation && cargo test`

`main.c` prints `argv[0]` on its error path
(`fprintf(stderr, "%s requires 4 inputs\n", argv[0])`), so the two binaries can
only be compared if they are launched with the *same* `argv[0]`. The test
harness does this with `std::os::unix::process::CommandExt::arg0`.

---

## 1. The sign of a NaN was lost narrowing `double` → `float`

**Symptom**

```
$ driver -nan 1 nan
C   : -nan -nan nan
Rust: -nan  nan nan      # second component wrong
```

**Cause**

`main.c` assigns an `atof` result (a `double`) into a `vec3_t` element (a
`float`). The Rust code did this with `atof(...) as f32`.

Rust leaves the resulting NaN bit pattern unspecified for a float-to-float
`as` cast, and LLVM's constant folder canonicalises it to a *positive* quiet
NaN:

| f64 input          | C `(float)` | Rust `as f32` (folded) |
|--------------------|-------------|------------------------|
| `fff8000000000000` | `ffc00000`  | `7fc00000`             |

At runtime the two agree, because both end up in `cvtsd2ss`, which preserves the
sign. The divergence only appeared because LLVM can see through the inlined
`atof` on the `"nan"` branch — its return value is the compile-time constant
`-f64::NAN` — and folds the `fptrunc` itself. That lost sign then reaches
`printf("%f")`, which does print the sign of a NaN.

**Fix** — `src/clib.rs`: `narrow_to_f32`, an explicit narrowing that keeps the
sign bit and the top 22 significand bits, exactly as `cvtsd2ss` does. The
matching `widen_to_f64` covers the `float` → `double` promotion the variadic
`printf` argument undergoes (`cvtss2sd`), which has the same hazard.

## 2. NaN *selection* differed because LLVM reorders commutative float operands

**Symptom** (still failing after fix 1)

```
$ driver -nan nan 1
C   : -nan nan -nan
Rust:  nan nan  nan
```

**Cause**

On x86, `mulss`/`addss`/`subss` do not return a canonical NaN — they return the
**destination** operand if it is a NaN, otherwise the source operand. Which NaN
(and therefore which *sign*) comes out is decided purely by operand order.

`gcc -O0` emits the operand order written in the source. LLVM does not have to:

- `fadd`/`fmul` are commutative, and LLVM canonicalises their operand order, so
  `DotProduct`'s `(v0*v0) + (v1*v1)` could come out with the operands swapped.
- `v[0] *= k; v[1] *= k; v[2] *= k` vectorises into one `mulps` with the
  broadcast `k` as the *destination*, which makes every lane return `k`'s NaN
  rather than each `v[i]`'s. That is exactly the `nan nan nan` seen above.

**Fix** — new `src/fpu.rs` with `fmul`/`fadd`/`fsub` that make the destination
operand explicit and check for NaN operands before doing the arithmetic, so the
optimiser can no longer change which NaN is selected. `q_math.rs` and
`q_shared.rs` now use them and preserve C's exact grouping:

- `DotProduct` → `fadd(fadd(fmul(x0,y0), fmul(x1,y1)), fmul(x2,y2))`
  (`+` associates to the left)
- `x2 * y * y` → `fmul(fmul(x2, y), y)`
- `v[i] *= ilength` → `fmul(v[i], ilength)`, with `v[i]` as the destination

## 3. Invalid operations produce a *negative* NaN on x86

**Symptom** (same investigation as #2)

```
$ driver inf 0 0
C: -inf -nan -nan
```

**Cause**

`inf * 0`, `inf + -inf` and `inf - inf` are invalid operations. x86 returns the
"QNaN floating-point indefinite", whose sign bit is **set** (`0xFFC00000`), so
`printf("%f")` renders it `-nan`. LLVM's constant folder produces a *positive*
NaN for these instead, so any folded occurrence would have printed `nan`.

This is reachable from ordinary input: `driver inf 0 0` makes `ilength` `-inf`,
and `0 * -inf` is then an invalid operation.

**Fix** — `src/fpu.rs` returns `f32::from_bits(0xFFC00000)` explicitly for these
three cases rather than relying on the hardware result surviving optimisation.

## 4. Belt-and-braces: the parsed inputs are kept opaque

In the C program `Inputs` is filled by an opaque libc call on runtime `argv`
data, so no part of the subsequent float arithmetic can be constant folded. In
Rust, `atof` is inlined and LLVM *can* fold that arithmetic for the constant
branches (`"nan"`, `"inf"`, …) — which is how bugs 1–3 became observable in the
first place.

`main.rs` therefore passes the filled array through `std::hint::black_box`
before `vector_normalize_fast`, restoring the C compilation model. Fixes 1–3 are
the real corrections; this makes them robust to future inlining decisions rather
than depending on them.

---

## Not a defect, but worth recording

- **`translation/src/cstd.rs` was dead code.** It was never declared as a module
  (`main.rs` uses `clib.rs`), so it was never compiled. It was a near-duplicate
  of `clib.rs` carrying the same `as f32` bug, so it has been deleted rather than
  left as a second, silently-wrong copy of the parser.
- **`Q_rsqrt` is an approximation, and the error is visible.** `driver 1 2 3`
  prints `0.267214 0.534428 0.801642`, not the exactly normalized
  `0.267261 0.534522 0.801783`. The single Newton iteration is faithfully
  reproduced; the output is meant to be "wrong" in this way.
- **`argc == 0` is not reachable as a distinct case.** Linux (5.18+) inserts an
  empty `argv[0]` when `execve` is handed an empty `argv`, so both programs see
  `argc == 1` and print `" requires 4 inputs"`. Verified with a helper that
  `execv`s with an empty `argv`; C and Rust agreed, as they did for `argv[0]`
  values that were empty, contained spaces, or were not valid UTF-8.
- **No locale sensitivity.** `main.c` never calls `setlocale`, so the program
  stays in the `"C"` locale and `atof` always uses `.` as the decimal point.
  Confirmed identical under `LC_ALL=C`, `en_US.UTF-8`, `de_DE.UTF-8` and
  `fr_FR.UTF-8`; `1,5` parses as `1` in both.
- **`atof` NaN payloads do not matter.** `strtod` accepts `nan(n-char-seq)`, but
  `%f` only exposes a NaN's sign, so the payload is unobservable. Only the sign
  is reproduced.

---

## Coverage

`translation/tests/differential.rs` runs both binaries as subprocesses with
identical `argv` and asserts **stdout, stderr and exit status** all match. 17
tests, none `#[ignore]`d, skipped or `should_panic`:

| Branch / input class in the C source | Test |
|---|---|
| `argc != 4` → `exit(1)`, for argc 1,2,3,5,6,7 | `wrong_arity_takes_the_error_path` |
| `argc == 4` → normalize and print | `happy_path_vectors` |
| `DotProduct == 0`, signed zeros | `zero_and_signed_zero_vectors` |
| `atof` finds nothing convertible, or only a prefix | `atof_returns_zero_or_a_prefix_instead_of_failing` |
| `strtod` hex / `inf` / `infinity` / `nan` forms | `atof_hex_infinity_and_nan_spellings` |
| `double` and `float` overflow / underflow boundaries | `overflow_and_underflow_in_conversion` |
| every sign combination of `inf` / `nan` through `Q_rsqrt` | `infinities_and_nans_through_the_math` |
| `inf * 0` and `inf + -inf` → `-nan` | `invalid_operations_print_negative_nan` |
| `argv` bytes that are not valid UTF-8 | `non_utf8_arguments` |
| multi-kilobyte arguments | `very_long_arguments` |
| all 28×28 pairs of notable `f32` bit patterns | `sweep_special_bit_patterns` |
| 1200 uniformly random `f32` bit patterns | `sweep_random_float_bit_patterns` |
| 1200 random decimal magnitudes, exponents ±350 | `sweep_random_decimal_magnitudes` |
| 900 random hex floats, binary exponents ±1100 | `sweep_random_hex_floats` |
| 1500 adversarial numeric-looking tokens | `sweep_adversarial_numeric_tokens` |
| 1200 random raw byte strings at random arity | `sweep_random_raw_bytes_and_arity` |
| both binaries build and run | `both_binaries_are_runnable` |

Beyond the suite, roughly 87,000 further argv combinations were compared during
investigation (random `f32` bit patterns rendered as exact hex floats, random
decimal and hex magnitudes, adversarial parser input, and random raw bytes) with
no remaining differences.
