# NaN payload: the one place "byte-identical" is not well-defined

Everything in this crate is compared **bit-for-bit** against the C library
(`f32::to_bits`, no epsilons) with exactly one documented relaxation: when both
the C result and the Rust result are NaN, the NaN *sign bit and payload* are not
compared (`common::canon_f32` maps every NaN to `0x7FC00000`).

## Why

On x86, `mulss`/`addss`/`subss` propagate the **destination** operand when both
operands are NaN. Which source ends up in the destination register is
instruction selection, not language semantics — and GCC changes its mind
depending on the optimisation level, for the *same* C source:

```c
float c2Dot(c2v a, c2v b) { return a.x*b.x + a.y*b.y; }
```

| build | `c2Dot((+NaN,+NaN), (-NaN,-NaN))` |
|-------|-----------------------------------|
| `gcc -O0` | `0xffc00000` (destination = right operand) |
| `gcc -O1`/`-O2`/`-O3` | `0x7fc00000` (destination = left operand) |
| `rustc` (debug and release) | `0x7fc00000` |

Reproduction: `gcc -O0` emits `mulss %xmm1,%xmm0` with the *right* factor in
`%xmm0`; `gcc -O1`+ and LLVM emit it with the *left* factor in `%xmm0`.

The Rust translation therefore already matches an optimised build of the C
source exactly. "Fixing" it to match `-O0` would mean writing every product and
sum with its operands reversed (`b.x*a.x + ...`), which is (a) numerically
identical for all non-NaN inputs anyway, and (b) would then *diverge* from
`gcc -O1`+ builds of the identical C. That trades a defined mismatch for an
equally arbitrary one, so it is deliberately not done.

## What is still compared strictly

Every NaN case whose result is independent of operand order is compared with
full bit equality (`common::same_strict`), and is covered by
`tests/error_paths.rs::err_nonfinite_scalar_helpers` and
`tests/nan_strict.rs`:

* a single NaN operand (the NaN is propagated regardless of which register is
  the destination);
* `inf - inf`, `inf + (-inf)` → `0xffc00000` in both;
* `0 * inf` → `0xffc00000` in both;
* `sqrtf(negative)` → `0xffc00000` in both;
* `±0`, subnormals, overflow to `±inf`, `FLT_MAX`, `FLT_EPSILON` — all exact.
