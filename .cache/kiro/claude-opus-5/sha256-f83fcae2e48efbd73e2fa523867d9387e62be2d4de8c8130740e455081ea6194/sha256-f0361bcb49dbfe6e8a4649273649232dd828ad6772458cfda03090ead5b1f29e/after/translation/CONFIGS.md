# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from `c_src/src/lib.c`, `c_src/include/lib.h`, and the
`objdump -d` disassembly of the C `.so`.

## The axes the C actually branches on

**Axis 1 — entry points.** The `.so` exports exactly one: `colourblind`. The
three lowest-level functions (`Protanopia`, `Deuteranopia`, `Tritanopia`) are
`static`, so they are *not* separately reachable across the FFI boundary; the
only way to drive them is `colourblind` with the matching discriminant. Each
discriminant is therefore treated as its own low-level entry point below, so the
matrices are exercised directly rather than only through one convenience call.

**Axis 2 — `cb_impairment` mode (the one runtime option).** `switch` in
`colourblind` distinguishes exactly 3 valid values: `cbProtanopia` (0),
`cbDeuteranopia` (1), `cbTritanopia` (2). Out-of-range values are in
`ERRORS.md`.

**Axis 3 — input value shape.** The code has no size/count/format/byte-order
parameters (it takes 3 scalar `float*`), so the shapes it distinguishes are
purely the `f32` value classes that change the arithmetic outcome. From the
expressions actually present (`mulss`, `addss`, `subss` on `f32`):

| shape | why the C treats it differently |
|-------|-------------------------------|
| ordinary normals in `[0,1]` | the intended sRGB domain, no special rounding |
| large normals near `±FLT_MAX` | products/sums overflow to `±inf` |
| subnormals & `±FLT_MIN` | products underflow to subnormal or `±0` |
| exact `+0.0` / `-0.0` | sign-of-zero rules: `(+0) + (-0) = +0`, `(-0) + (-0) = -0`, `x - x = +0`; the `-`-coefficient rows (`P_BR`, `D_BR`, `T_GR`) flip zero signs |
| `±inf` | `inf * 0 -> QNaN`, `inf - inf -> QNaN`, else `inf` propagates |
| quiet NaN with a non-default payload | SSE forwards an operand's payload, so the exact payload bits are observable in the output |
| signalling NaN | SSE quiets it (sets mantissa MSB) while preserving sign+payload |
| mixed classes across R/G/B | selects *which* operand SSE forwards; the `dst`-before-`src` priority makes the result depend on which of R/G/B is the NaN |
| negative normals | exercises the negative coefficients and `subss` sign handling |

**Axis 4 — pointer shape.** Distinct pointers vs. aliased (`R==G==B`, and each
pair), and unaligned pointers. `movss` is alignment-agnostic, and the C reads
all three inputs into locals *before* the first store — a real behavioural
contract a translation can break.

**Axis 5 — build/feature configuration.** `translation/Cargo.toml` declares
**no `[features]` table**, so `default` is the only feature combination that
exists; `cargo check/test --no-default-features` is the only other spelling and
compiles the identical code. `c_src/CMakeLists.txt` has no options or `#ifdef`
either — one C configuration. Verified by `translation/check_features.sh`.
There is **no binary/driver target** in either build (CMake declares only
`add_library(... SHARED)`, Cargo only `crate-type = ["cdylib"]`), so there is no
stdout to compare.

## The table

Cross-product of axis 2 × axis 3/4, pruned to combinations the C distinguishes.
Every row is driven through the exported `colourblind` symbol of **both** `.so`s
via `libloading`, with **many randomized inputs per row** (fixed-seed SplitMix64
PRNG, 20 000 iterations per row for value rows), and asserts the three output
`f32`s match **bit-for-bit** (`to_bits()`), so NaN payload and zero sign are
compared too.

| #   | entry point(s) | configuration (options set + input shape) | [ ] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `colourblind` / `cbProtanopia` (0) | random normals in `[0,1]` (the sRGB domain) | [x] |
| C2  | `colourblind` / `cbDeuteranopia` (1) | random normals in `[0,1]` | [x] |
| C3  | `colourblind` / `cbTritanopia` (2) | random normals in `[0,1]` | [x] |
| C4  | all 3 modes | random normals in `[-1,1]` (negative coefficients / `subss` sign paths) | [x] |
| C5  | all 3 modes | uniformly random bit patterns restricted to finite values (full binade sweep, any exponent) | [x] |
| C6  | all 3 modes | fully unrestricted uniformly random 32-bit patterns (mixes normals, subnormals, zeros, infs, quiet + signalling NaNs across R/G/B independently) | [x] |
| C7  | all 3 modes | all 8 sign combinations of exact zeros `(±0, ±0, ±0)` | [x] |
| C8  | all 3 modes | all 125 combinations of `{-inf, -0.0, +0.0, +inf, 1.0}^3` (boundary cross-product incl. `inf*0` and `inf-inf`) | [x] |
| C9  | all 3 modes | subnormal / tiny inputs: cross-product of `{±f32::MIN_POSITIVE, ±smallest subnormal, ±1e-40, ±0}` | [x] |
| C10 | all 3 modes | huge inputs: cross-product of `{±f32::MAX, ±f32::MAX/2, ±1e38, ±1.0}` (overflow to `inf`) | [x] |
| C11 | all 3 modes | one channel is a quiet NaN with a randomized non-default payload, other two ordinary normals — each of the 3 channel positions (checks *which* operand SSE forwards) | [x] |
| C12 | all 3 modes | one channel is a **signalling** NaN with randomized payload, others normal — each of the 3 positions (checks SSE quieting) | [x] |
| C13 | all 3 modes | two or three channels are NaN simultaneously with *distinct* payloads (checks the `dst`-before-`src` forwarding priority) | [x] |
| C14 | all 3 modes | pointers fully aliased: `R == G == B` (load-all-then-store contract) | [x] |
| C15 | all 3 modes | pointers partially aliased: `R==G`, `R==B`, `G==B` (three sub-cases) | [x] |
| C16 | all 3 modes | unaligned `float*` (offset 1, 2, 3 bytes into a byte buffer) | [x] |
| C17 | all 3 modes | repeated in-place application (output fed back as input, 64 iterations) — the composed pipeline a real consumer runs, where per-call rounding error accumulates | [x] |
| C18 | all 3 modes | mode switched between calls on the same buffers (0→1→2→0…), interleaving the matrices over accumulating state | [x] |
| C19 | `colourblind`, all 3 modes | integer-valued and exactly-representable inputs (`0,1,2,...,255`, `0.5`, `0.25`) — exact-arithmetic paths where rounding is trivial | [x] |
| C20 | `colourblind`, all 3 modes | coefficient-bit identity check: every `f32` matrix constant in the Rust source equals the corresponding pooled constant in the C `.so`'s `.rodata` (17 distinct words at `0x2000..0x2043`) | [x] |

`Protanopia` / `Deuteranopia` / `Tritanopia` need no separate rows: they have no
other caller in the C, and rows C1–C3 reach each of them directly with the
minimum possible dispatch in between.

---

## Divergences found and fixed

### 1. Harness defect (found first, and it was hiding everything else)

`cargo test` builds the integration-test binaries but **does not rebuild the
`cdylib` artifact**. The first version of `tests/common/mod.rs` `dlopen`ed
`target/release/libcolourblind_lib.so` directly, so every differential test was
comparing the C library against whatever Rust build happened to be on disk. A
deliberate mutation (`P_RG` 0.829… → 0.5, a gross error) still produced 36/36
passes.

Fix: `tests/common/mod.rs` now shells out to `cargo build --lib` for the
detected profile before `dlopen`, and then asserts `mtime(.so) >= mtime(src/lib.rs)`
so a stale load can never pass silently. Re-running the same mutation
afterwards failed 18 of 36 tests (13 valid-path, 5 error-path), confirming the
suite is actually sensitive.

### 2. Real translation bug — misaligned pointer dereference (rows C16 / E10)

The C compiles each `*Red` / `*Green` / `*Blue` access to a plain `movss`, which
has no alignment requirement, so the C library accepts a `float*` that is not
4-byte aligned and computes the correct result. The Rust used plain `*p`
dereferences, which are undefined behaviour on a misaligned pointer: the release
build happened to work, but the **debug** build aborted with

```
panicked at src/lib.rs:147:31:
misaligned pointer dereference: address must be a multiple of 0x4 but is 0x7f98de3f8109
thread caused non-unwinding panic. aborting.
```

That is a hard divergence — the C returns a value, the Rust kills the process.

Fix: `src/lib.rs` now uses `ptr::read_unaligned` / `ptr::write_unaligned` in all
three transforms. For aligned pointers this compiles to the identical `movss`,
so there is no behaviour change on the aligned path; for unaligned pointers it
now matches the C. Both profiles pass afterwards.

## Notes on scope

- **Reference C build.** The ground-truth `.so` is the one produced by the
  documented command, i.e. CMake with no `CMAKE_BUILD_TYPE` → no `-O` flags.
  The disassembly confirms straight-line unoptimised `mulss`/`addss`/`subss`
  with no FMA contraction and no reassociation, which is what the Rust mirrors
  operand-role for operand-role.
- **Constant pooling.** GCC merged several C literals that round to the same
  `f32` (`0x2000` serves both `0.17055699213417f` and `0.17055699092998f`;
  `0x2030` both Tritanopia red coefficients; `0x2038`/`0x203c` are shared
  between the green and blue rows). Rust's decimal→`f32` rounding agrees, which
  row C20 verifies behaviourally through both `.so`s.
- **Soak evidence.** `CB_SOAK=<n>` multiplies every randomized row. Wall time
  scales linearly (`0.03s` → `0.17s` → `1.49s` at 1× / 20× / 200×), which is the
  evidence the loops really run; at 200× the suite performs on the order of
  10^8 paired C/Rust calls with zero bit differences.
