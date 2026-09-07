# CONFIGS.md — Phase B configuration surface table

Mechanically derived from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axis enumeration (derived from the source, not guessed)

### Axis 1 — runtime options / modes / flags: **none**

The full public API is one line:

```c
uint16_t float2half(float flt);
```

There is no init function, no context/handle struct, no setter, no global
configuration variable, no environment-variable read, no `#ifdef`, and no
`if`/`switch` anywhere in `c_src/src/lib.c`. Grep confirms zero conditionals
(see `ERRORS.md`). So this axis has exactly one value: "the only mode".

### Axis 2 — full set of public entry points: **one**

`float2half` is simultaneously the highest- and lowest-level entry point; there
is no convenience wrapper layered over a lower-level primitive, so "test the
low-level entry points, not just the wrappers" collapses to this one function.
The two lookup tables are `static` and not reachable from outside the `.so`
(confirmed in `SYMBOLS.md`), so they cannot be driven independently — they are
only observable through `float2half`, and the table rows below are chosen
precisely so that every element of both tables is observed.

### Axis 3 — input shape

The single parameter is a by-value `float`. The only "shape" is its 32-bit
pattern, and the code decomposes it into exactly two fields:

- `j = (n >> 23) & 0x1ff` — the top 9 bits (sign + 8 exponent bits); selects
  which of the 512 table slots is used. **512 distinct values.**
- `m = n & 0x007fffff` — the low 23 mantissa bits, used only as `m >> m__shift[j]`.

Because the result is exactly `m__base[j] + (m >> m__shift[j])`, it depends on
the input *only* through the pair `(j, m >> m__shift[j])`. That makes complete
coverage finite and achievable: enumerating every `j` in `0..512` against every
attainable quotient `q = m >> m__shift[j]`, with both the minimum (`all-zero`)
and maximum (`all-one`) discarded low bits for each `q`, is behaviourally
**exhaustive over all 2^32 float bit patterns**. That is what row C1 does.

### Axis 4 — the shift regions the code effectively branches on

`m__shift` takes 12 distinct values (13..24) laid out in **28 contiguous runs**.
Each run is a genuinely different code path in the sense that a different number
of mantissa bits survives (`2^(23-shift)` distinct outcomes per bucket, i.e. 1
outcome for shift 24 up to 1024 outcomes for shift 13). Rows C4–C31 give one row
per run so no region can be silently skipped.

Run layout (computed from the C table):

| run | j range | shift | base at run start | base at run end | meaning |
|-----|---------|-------|-------------------|-----------------|---------|
| 1 | 0..102 | 0x18 (24) | 0x0000 | 0x0000 | +underflow to zero |
| 2 | 103 | 0x17 (23) | 0x0001 | 0x0001 | + smallest subnormals |
| 3 | 104 | 0x16 (22) | 0x0002 | 0x0002 | + subnormal |
| 4 | 105 | 0x15 (21) | 0x0004 | 0x0004 | + subnormal |
| 5 | 106 | 0x14 (20) | 0x0008 | 0x0008 | + subnormal |
| 6 | 107 | 0x13 (19) | 0x0010 | 0x0010 | + subnormal |
| 7 | 108 | 0x12 (18) | 0x0020 | 0x0020 | + subnormal |
| 8 | 109 | 0x11 (17) | 0x0040 | 0x0040 | + subnormal |
| 9 | 110 | 0x10 (16) | 0x0080 | 0x0080 | + subnormal |
| 10 | 111 | 0x0f (15) | 0x0100 | 0x0100 | + subnormal |
| 11 | 112 | 0x0e (14) | 0x0200 | 0x0200 | + largest subnormals |
| 12 | 113..142 | 0x0d (13) | 0x0400 | 0x7800 | + normal range (all 30 half exponents) |
| 13 | 143..254 | 0x18 (24) | 0x7c00 | 0x7c00 | + overflow to +inf |
| 14 | 255 | 0x0d (13) | 0x7c00 | 0x7c00 | +inf / +NaN (anomalous shift 13) |
| 15 | 256..358 | 0x18 (24) | 0x8000 | 0x8000 | -underflow to negative zero |
| 16 | 359 | 0x17 (23) | 0x8001 | 0x8001 | - smallest subnormals |
| 17 | 360 | 0x16 (22) | 0x8002 | 0x8002 | - subnormal |
| 18 | 361 | 0x15 (21) | 0x8004 | 0x8004 | - subnormal |
| 19 | 362 | 0x14 (20) | 0x8008 | 0x8008 | - subnormal |
| 20 | 363 | 0x13 (19) | 0x8010 | 0x8010 | - subnormal |
| 21 | 364 | 0x12 (18) | 0x8020 | 0x8020 | - subnormal |
| 22 | 365 | 0x11 (17) | 0x8040 | 0x8040 | - subnormal |
| 23 | 366 | 0x10 (16) | 0x8080 | 0x8080 | - subnormal |
| 24 | 367 | 0x0f (15) | 0x8100 | 0x8100 | - subnormal |
| 25 | 368 | 0x0e (14) | 0x8200 | 0x8200 | - largest subnormals |
| 26 | 369..398 | 0x0d (13) | 0x8400 | 0xf800 | - normal range (all 30 half exponents) |
| 27 | 399..510 | 0x18 (24) | 0xfc00 | 0xfc00 | -overflow to -inf |
| 28 | 511 | 0x0d (13) | 0xfc00 | 0xfc00 | -inf / -NaN (anomalous shift 13) |

`m__base` holds 84 distinct values across those runs.

### Axis 5 — mantissa sub-shape within a bucket

For a fixed `j` the code distinguishes: mantissa `0`; mantissa entirely below
the shift (quotient 0, non-zero remainder — the bits that get *thrown away*);
quotient 1 (lowest surviving bit); maximum quotient; and the `all-ones`
mantissa. Rows C32–C36 cross this axis against every run.

### Axis 6 — byte order / element type / count: **not applicable**

There is no buffer, array parameter, length, count, stride, element-type or
endianness option in the API — nothing is read from or written to memory across
the boundary. The one representation question is whether the `float` argument
and `uint16_t` return survive the FFI register ABI bit-identically, which row
C37 pins down.

## Configuration table

Every row is exercised against **both** `.so`s through `libloading` and asserted
byte-identical. Rows marked "randomized" use `MANY` inputs per row from a
xorshift PRNG with a fixed seed (`0x2545F4914F6CDD1D`) for reproducibility.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `float2half` | **Behaviourally exhaustive**: all 512 `j` buckets x every attainable quotient `q = m >> shift[j]`, each with all-zero and all-one discarded low bits. Covers all 2^32 inputs by the reduction argument above. | `configs_exhaustive_by_reduction` | [x] |
| C2 | `float2half` | Uniform random full 32-bit patterns, 2,000,000 samples, fixed seed | `configs_random_bit_patterns` | [x] |
| C3 | `float2half` | Random *plausible* floats (random sign x random exponent from a weighted set x random mantissa), 500,000 samples, fixed seed | `configs_random_structured_floats` | [x] |
| C4 | `float2half` | run 1: `j` in 0..102, shift 24 — randomized mantissas | `configs_per_shift_run` | [x] |
| C5 | `float2half` | run 2: `j` = 103, shift 23 — randomized mantissas | `configs_per_shift_run` | [x] |
| C6 | `float2half` | run 3: `j` = 104, shift 22 — randomized mantissas | `configs_per_shift_run` | [x] |
| C7 | `float2half` | run 4: `j` = 105, shift 21 — randomized mantissas | `configs_per_shift_run` | [x] |
| C8 | `float2half` | run 5: `j` = 106, shift 20 — randomized mantissas | `configs_per_shift_run` | [x] |
| C9 | `float2half` | run 6: `j` = 107, shift 19 — randomized mantissas | `configs_per_shift_run` | [x] |
| C10 | `float2half` | run 7: `j` = 108, shift 18 — randomized mantissas | `configs_per_shift_run` | [x] |
| C11 | `float2half` | run 8: `j` = 109, shift 17 — randomized mantissas | `configs_per_shift_run` | [x] |
| C12 | `float2half` | run 9: `j` = 110, shift 16 — randomized mantissas | `configs_per_shift_run` | [x] |
| C13 | `float2half` | run 10: `j` = 111, shift 15 — randomized mantissas | `configs_per_shift_run` | [x] |
| C14 | `float2half` | run 11: `j` = 112, shift 14 — randomized mantissas | `configs_per_shift_run` | [x] |
| C15 | `float2half` | run 12: `j` in 113..142, shift 13 (+normals, 30 buckets) — randomized mantissas | `configs_per_shift_run` | [x] |
| C16 | `float2half` | run 13: `j` in 143..254, shift 24 (+overflow) — randomized mantissas | `configs_per_shift_run` | [x] |
| C17 | `float2half` | run 14: `j` = 255, shift 13 (+inf/+NaN anomaly) — randomized mantissas | `configs_per_shift_run` | [x] |
| C18 | `float2half` | run 15: `j` in 256..358, shift 24 (-underflow) — randomized mantissas | `configs_per_shift_run` | [x] |
| C19 | `float2half` | run 16: `j` = 359, shift 23 — randomized mantissas | `configs_per_shift_run` | [x] |
| C20 | `float2half` | run 17: `j` = 360, shift 22 — randomized mantissas | `configs_per_shift_run` | [x] |
| C21 | `float2half` | run 18: `j` = 361, shift 21 — randomized mantissas | `configs_per_shift_run` | [x] |
| C22 | `float2half` | run 19: `j` = 362, shift 20 — randomized mantissas | `configs_per_shift_run` | [x] |
| C23 | `float2half` | run 20: `j` = 363, shift 19 — randomized mantissas | `configs_per_shift_run` | [x] |
| C24 | `float2half` | run 21: `j` = 364, shift 18 — randomized mantissas | `configs_per_shift_run` | [x] |
| C25 | `float2half` | run 22: `j` = 365, shift 17 — randomized mantissas | `configs_per_shift_run` | [x] |
| C26 | `float2half` | run 23: `j` = 366, shift 16 — randomized mantissas | `configs_per_shift_run` | [x] |
| C27 | `float2half` | run 24: `j` = 367, shift 15 — randomized mantissas | `configs_per_shift_run` | [x] |
| C28 | `float2half` | run 25: `j` = 368, shift 14 — randomized mantissas | `configs_per_shift_run` | [x] |
| C29 | `float2half` | run 26: `j` in 369..398, shift 13 (-normals, 30 buckets) — randomized mantissas | `configs_per_shift_run` | [x] |
| C30 | `float2half` | run 27: `j` in 399..510, shift 24 (-overflow) — randomized mantissas | `configs_per_shift_run` | [x] |
| C31 | `float2half` | run 28: `j` = 511, shift 13 (-inf/-NaN anomaly) — randomized mantissas | `configs_per_shift_run` | [x] |
| C32 | `float2half` | mantissa sub-shape `m = 0` x all 512 buckets | `configs_mantissa_subshapes` | [x] |
| C33 | `float2half` | mantissa sub-shape `m = (1 << shift[j]) - 1` (quotient 0, all discarded bits set) x all 512 buckets | `configs_mantissa_subshapes` | [x] |
| C34 | `float2half` | mantissa sub-shape `m = 1 << shift[j]` (quotient exactly 1) x all 512 buckets | `configs_mantissa_subshapes` | [x] |
| C35 | `float2half` | mantissa sub-shape `m = 0x7fffff` (all bits set, maximum quotient) x all 512 buckets | `configs_mantissa_subshapes` | [x] |
| C36 | `float2half` | mantissa sub-shape `m = 0x400000` (top mantissa bit only, the quiet-NaN bit) x all 512 buckets | `configs_mantissa_subshapes` | [x] |
| C37 | `float2half` | FFI representation: `f32` argument passed by value in an SSE register and `u16` return in `ax` are bit-preserving — verified by feeding bit patterns that are not canonical floats (sNaN payloads, pseudo-denormals) and checking the C and Rust agree, plus that the C result round-trips through a `u32`-typed symbol lookup | `configs_ffi_representation` | [x] |
| C38 | `float2half` | ordinary consumer usage end-to-end: a sweep of human-meaningful values (0, 1, -1, 0.5, 2, 65504, 65520, 6.1e-5, 5.96e-8, pi, e, f32::MIN/MAX/EPSILON/MIN_POSITIVE, powers of two from 2^-30..2^30) | `configs_representative_values` | [x] |
| C39 | `float2half` | repeated / interleaved invocation (no hidden mutable state: same input before and after 10k other calls yields the same answer in both libs) | `configs_statelessness` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the cross-product of
feature combinations is the single default configuration. All rows above are
therefore verified under every feature combination that exists. This is
re-checked mechanically by `check_all_feature_combos.sh`.

## Key semantic finding (recorded so it is not "fixed" later)

The conversion **truncates** the mantissa (`>> m__shift[j]`, round-toward-zero).
It is *not* a round-to-nearest float-to-half conversion, and it does not
saturate to infinity at the usual place:

- every value in the whole binade `[65504, 65536)` maps to `0x7bff`, the largest
  finite half — including `65520.0`, which a round-to-nearest implementation
  would map to `0x7c00` (infinity);
- the jump to half infinity happens exactly at `65536.0`, where the exponent
  bucket advances from `j = 142` to `j = 143`.

An initial test expectation of `65520.0 -> 0x7c00` was written from
round-to-nearest intuition and **the test was corrected to match the C**, which
is ground truth. `mutation_check.sh` includes a "round-to-nearest 'fix'" mutant
to make sure the suite would reject any future attempt to "correct" this.

## Meta-verification: the suite is non-vacuous

`./mutation_check.sh` injects known-wrong changes into `src/lib.rs`, rebuilds the
cdylib and requires the suite to fail. Result: 7/7 observable mutants CAUGHT
(corrupted `m__base` entry, wrong index mask, wrong mantissa mask, off-by-one
result, removed `#[no_mangle]` export, wrong exponent shift, swapped tables).

Two mutants survive and are expected to, because they are **provably equivalent**
— unobservable through the public API rather than untested:

| mutant | why it is equivalent |
|---|---|
| sum computed with `u16::saturating_add` instead of a `u32` intermediate | the maximum attainable `base + (mantissa >> shift)` is exactly `0xffff`, asserted for all 512 buckets by `errors_truncating_cast`, so saturation never triggers |
| `m__shift[103]` changed from 23 to 24 | a mantissa is only 23 bits, so `m >> 23` and `m >> 24` are both identically zero; see `OBSERVABLE_SHIFT_CAP` in `tests/common/mod.rs` |

## Reproducing

```
cd translation
cargo test --release            # 21 differential tests (add --offline if the registry is unreachable)
./mutation_check.sh             # prove the suite is non-vacuous
./check_all_feature_combos.sh   # every feature combination, with a symbol diff per combo
```
