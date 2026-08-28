# Mismatches found while verifying the C → Rust translation

Both programs are compared by running them: `driver A B C` with identical
`argv` (including `argv[0]`), then diffing stdout, stderr and the exit status.
The C is ground truth; every fix below changed only the Rust side.

The suite lives in `tests/differential.rs`. It builds `c_src` itself with CMake
into `translation/target/c_build`, so nothing is written inside `c_src/`.

Reference for the FP discussion: the C program is compiled with no optimisation
flags (`c_src/CMakeLists.txt` sets none), so each arithmetic expression becomes
one SSE instruction and the source order fixes which register is the
destination.

---

## 1. `atof` applied the sign when no conversion happened

**Symptom** — the sign of a zero component leaked into the output:

| `argv` | C | Rust (before) |
| --- | --- | --- |
| `30002408.0  -.  -0` | `0.999745 0.000000 -0.000000` | `0.999745 -0.000000 -0.000000` |
| `825995.75  -53.72045  --3` | `0.998305 -0.000065 0.000000` | `0.998305 -0.000065 -0.000000` |
| `--3  5e  867.0867724849843` | `0.000000 0.005759 0.998695` | `-0.000000 0.005759 0.998695` |

**Cause** — `cstd::atof` consumed an optional `+`/`-`, and when the rest of the
string turned out to have no digits it still returned `-0.0` for a leading `-`:

```rust
if digits == 0 {
    return if negative { -0.0 } else { 0.0 };   // wrong
}
```

C's `strtod` only applies the sign to a value it actually converted. Inputs
such as `-.`, `--3`, `-`, `-e1` and `-x` perform *no* conversion, so the
function returns `+0.0` and the sign is discarded. `printf("%f")` prints the
sign bit of a zero, which is what made the difference observable.

**Fix** — return `+0.0` unconditionally on the no-conversion path
(`src/cstd.rs`). The sign is still applied for `-0`, `-0.0` and `-0x`, where a
conversion *does* take place; those are covered by
`unconvertible_text_becomes_zero` and `zero_length_vector`.

---

## 2. NaN sign lost because LLVM commuted the dot-product addition

**Symptom** — only when `+nan` and `-nan` were mixed in the same vector:

| `argv` | C | Rust (before) |
| --- | --- | --- |
| `3.93289726677410164e+37  -nan  nan` | `-nan -nan nan` | `nan -nan nan` |
| `-nan  nan  -553.7229153427868` | `-nan nan -nan` | `-nan nan nan` |

**Cause** — on x86, `MULSS`/`ADDSS`/`SUBSS` pick which NaN to return:

```text
if SRC1 (the destination register) is NaN -> QNaN(SRC1)
else if SRC2 is NaN                       -> QNaN(SRC2)
else                                      -> the arithmetic result
```

so with two NaN operands of opposite sign the *destination* wins. gcc at `-O0`
keeps the running total in the destination, evaluating strictly left to right:

```text
mulss %xmm0,%xmm1    # xmm1 = x[0]*y[0]
mulss %xmm2,%xmm0    # xmm0 = x[1]*y[1]
addss %xmm0,%xmm1    # xmm1 = (x[0]*y[0]) + (x[1]*y[1])   <- dest is the left term
```

Rust's `x[0]*y[0] + x[1]*y[1] + x[2]*y[2]` compiled to the *swapped* form,
because LLVM treats `fadd` as commutative and reassociates the operands freely:

```text
addss %xmm0,%xmm1    # xmm1 = (x[1]*y[1]) + (x[0]*y[0])   <- dest is the right term
```

That is exact for every finite value, but it hands back the other NaN. The
wrong NaN then flows out of `Q_rsqrt` as `ilength` and is multiplied into all
three components, so a single swapped `addss` changed two of the three printed
fields.

**Fix** — added `src/sse.rs` with `mul` / `add` / `sub` helpers that model the
instruction explicitly (NaN taken from the destination operand first, and the
propagated NaN returned *as-is* by `sub`, not negated). `q_shared::dot_product`,
`q_shared::vector_normalize_fast` and `q_math::q_rsqrt` now use them in the same
operand order as the disassembly of the C binary. Covered by
`nan_propagation_and_sign`, which checks all 729 triples over
`{nan, -nan, 1, -1, 0, -0, inf, -inf, 1e38}`.

---

## Checked and found already correct

Recorded so the next reader does not have to re-derive them.

- **`printf("%f")` rounding.** glibc and Rust's `{:.6}` both round exact ties to
  even: `0.0078125` → `0.007812`, `0.9921875` → `0.992188`. No adjustment
  needed. `format_f` still has to special-case non-finite values, since Rust
  prints `NaN`/`inf` where C prints `nan`/`-nan`/`inf`/`-inf`.
- **`Q_rsqrt`'s `number * 0.5F` and `threehalfs - (...)`.** LLVM folds the
  negation into the constant, computing `number * -0.5` and then `1.5 + t`. That
  rewrite is NaN-safe here: the operand that changes places is a constant, never
  a NaN, so the propagated NaN is unaffected.
- **The `0x5f3759df - (i >> 1)` bit hack.** `wrapping_sub` on `u32` matches the
  C `uint32_t` arithmetic; no input reachable through this program makes the
  subtraction produce a NaN bit pattern, because a dot product of squares is
  never negative.
- **Invalid-operation results.** `0 * inf` and `inf - inf` produce x86's default
  QNaN, `0xFFC00000`, which prints as `-nan`. Rust's native `f32` operators emit
  the same instructions, so the fall-through path in `sse.rs` reproduces it.
- **`argv` as bytes.** Arguments that are not valid UTF-8 reach `strtod`
  unchanged; `main.rs` reads `args_os()` and works on the raw bytes rather than
  going through `String`. Covered by `non_utf8_and_long_arguments`.
- **`argv[0]` on the error path.** C prints `argv[0]`, so the tests set the same
  `arg0` for both processes; otherwise stderr could never match byte for byte.
- **Exit statuses.** `1` for `argc != 4`, `0` otherwise. Asserted on every case,
  not just the error ones.
