# CONFIGS.md — configuration surface table (valid inputs)

Mechanically derived from the branches the C actually takes.

## Axes found in the C

**Runtime options / modes / flags: NONE.** The library is stateless. There is no
init/config/handle type, no global, no `#ifdef` and no `#if` anywhere in
`c_src/src/*.c` or `c_src/include/match.h`; the only compile-time knob is
`#define N_SMOOTH 16`. The entire configuration surface is therefore the
*arguments* of the two entry points plus the *shape and values* of the buffers.

**Public entry points (both covered directly, not just via the wrapper):**

| entry point | signature as compiled | role |
|-------------|----------------------|------|
| `spectral_contrast` | `double spectral_contrast(float *a, float *b, int length)` — `float_t` from `<math.h>`, i.e. `float` | lowest level; also mutates `a` and `b` in place via `normalize` |
| `match` | `int match(double *test, double *reference, int bins, double threshold)` — `float_t` from `match.h`, i.e. `double` | composed pipeline `total` → `preprocess`(`smoothen`/`differentiate`/`smoothen`) → `spectral_contrast` (through the PLT), reinterpreting its `double` VLAs as `float` arrays |

**Shape axis — `bins` / `length`.** The code distinguishes:

| value class | why the C treats it differently |
|-------------|-------------------------------|
| `0` | zero-length VLA. For `match` this is **UB** (`differentiate`'s `v[-1]` store overwrites `preprocess`'s pushed return address and the C segfaults) so it belongs to `ERRORS.md` rows 3–4, not here. For `spectral_contrast` it is defined and returns `+0.0` (`ERRORS.md` row 11). |
| `1` | `differentiate`'s loop runs 0×, only `v[0]=0` fires ⇒ output identically 0 ⇒ `magnitude==0` ⇒ divide-by-zero NaN path |
| `2 … 15` (`< N_SMOOTH`) | *every* element of `smoothen` hits the truncated-kernel tail (`i+j<length` stops the sum early, divisor stays 16) |
| `16` (`== N_SMOOTH`) | only `i==0` gets a full 16-tap window |
| `17 … 31` (`> N_SMOOTH`) | mixed full-window head + truncated tail |
| `32`, `33`, `4096` (`≫ N_SMOOTH`) | full-window body dominates; also long accumulation chains in `total`/`dot_product` |
| even vs odd | `match` hands `bins` **`float` lanes** to `spectral_contrast`, i.e. `ceil(bins/2)` of its `double` slots — odd `bins` leaves the high half-word of the last touched slot untouched |
| very large (`100000`) | long chains; VLA still within the stack rlimit (beyond that: `ERRORS.md` row 8) |

**Value-shape axis.** Distinguished because of `sqrt`, `/magnitude`,
`double`→`float` reinterpretation and the `comisd` gates:

`all +0.0` · `all -0.0` · constant DC · linear ramp · uniform `[0,1)` ·
uniform `[-1,1)` · wide-exponent (`2^±300`) · float-overflow magnitudes
(`|x| > 3.4e38` ⇒ `inf` after the reinterpretation) · subnormal ·
raw-random bit patterns (yield `inf`/`NaN`/subnormal *floats* when the `double`
buffer is read as `float`) · buffers with embedded `NaN`/`±inf` ·
aliased pointers (`a == b`, `test == reference`).

**`threshold` axis (`match` only).** `-inf` · `-1.0` · `-0.0` · `+0.0` ·
`1e-300` · `0.5` · `1.0` · `1.0 - eps` · `1e300` · `+inf` · `NaN`, plus values
computed to land **exactly** on the gate boundary
`total(test) == threshold * total(reference)` (exercises strict `<` and `>=`).

**Gate-outcome axis (`match` only).** energy gate taken (early `return 0`) vs
not taken; and, when not taken, contrast `>=` threshold true vs false.

## Rows

Each row is checked by a `#[test]` in `tests/configs.rs` that drives **both**
`.so`s through their exported symbols with **many seeded-random inputs**
(default 200 iterations per row, `xoshiro256**` seeded from a per-row constant)
and compares the return value bit-for-bit **and** the in-place-mutated buffers
byte-for-byte. `[x]` = passing across all randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|------------------------------------------|-----|
| 1 | `spectral_contrast` | `length=1`, uniform `[0,1)` floats | [x] |
| 2 | `spectral_contrast` | `length=2`, uniform `[-1,1)` floats | [x] |
| 3 | `spectral_contrast` | `length=15` (odd, `<N_SMOOTH`), uniform `[-1,1)` | [x] |
| 4 | `spectral_contrast` | `length=16`, uniform `[-1,1)` | [x] |
| 5 | `spectral_contrast` | `length=17` (odd, `>N_SMOOTH`), uniform `[-1,1)` | [x] |
| 6 | `spectral_contrast` | `length=64`, uniform `[-1,1)` | [x] |
| 7 | `spectral_contrast` | `length=4096`, uniform `[-1,1)` (long accumulation chain) | [x] |
| 8 | `spectral_contrast` | `length∈{1..64}` random, wide exponents `2^±40` (magnitude → `inf`/subnormal) | [x] |
| 9 | `spectral_contrast` | `length∈{1..64}` random, **raw random bit patterns** (`NaN`/`inf`/subnormal floats) | [x] |
| 10 | `spectral_contrast` | `length∈{1..64}`, uniform values with `NaN`/`±inf` sprinkled in (dual-NaN `MULSS`/`ADDSD` operand-role coverage) | [x] |
| 11 | `spectral_contrast` | `length∈{1..64}`, all elements `+0.0` / all `-0.0` (⇒ `magnitude==0`, `0/0` NaN path) | [x] |
| 12 | `spectral_contrast` | `length∈{1..64}`, constant non-zero DC | [x] |
| 13 | `spectral_contrast` | `length∈{1..64}`, subnormal-only values (`magnitude` underflows) | [x] |
| 14 | `spectral_contrast` | aliased buffers: `a == b`, `length∈{1..64}`, uniform | [x] |
| 15 | `match` | `bins=1`, uniform `[0,1)`, `threshold` random in `[-1,2]` | [x] |
| 16 | `match` | `bins=2` (even), uniform `[0,1)`, random `threshold` | [x] |
| 17 | `match` | `bins=15` (odd, `<N_SMOOTH`), uniform `[0,1)`, random `threshold` | [x] |
| 18 | `match` | `bins=16` (`==N_SMOOTH`), uniform `[0,1)`, random `threshold` | [x] |
| 19 | `match` | `bins=17` (odd, `>N_SMOOTH`), uniform `[0,1)`, random `threshold` | [x] |
| 20 | `match` | `bins=32` / `33`, uniform `[0,1)`, random `threshold` | [x] |
| 21 | `match` | `bins=4096`, uniform `[0,1)`, random `threshold` | [x] |
| 22 | `match` | `bins=100000`, uniform `[0,1)`, random `threshold` (large VLA, run on a 64 MiB stack) | [x] |
| 23 | `match` | `bins∈{1..40}`, uniform `[-1,1)` (signed ⇒ `total` can be ≤ 0, flipping the energy gate) | [x] |
| 24 | `match` | `bins∈{1..40}`, **raw random `double` bit patterns** (`NaN`/`inf`/subnormal `double`s *and* garbage `float` lanes) | [x] |
| 25 | `match` | `bins∈{1..40}`, values with `NaN`/`±inf` sprinkled in | [x] |
| 26 | `match` | `bins∈{1..40}`, wide exponents `2^±300` (`total` overflows to `inf`; `float` lanes overflow) | [x] |
| 27 | `match` | `bins∈{1..40}`, constant DC (`differentiate` ⇒ all zeros ⇒ `0/0` NaN contrast) | [x] |
| 28 | `match` | `bins∈{1..40}`, all `+0.0` / all `-0.0` (energy gate `0 < threshold*0`) | [x] |
| 29 | `match` | `bins∈{1..40}`, linear ramp (`differentiate` ⇒ constant) | [x] |
| 30 | `match` | `bins∈{1..40}`, subnormal-only values | [x] |
| 31 | `match` | aliased: `test == reference`, `bins∈{1..40}` (contrast is exactly `1.0` unless NaN) | [x] |
| 32 | `match` | `threshold` swept over the special set `{-inf,-1,-0.0,0.0,1e-300,0.5,1.0,1.0-eps,1e300,+inf,NaN}` × `bins∈{1..40}` random data | [x] |
| 33 | `match` | `threshold` computed to sit **exactly** on the energy-gate boundary (`total(test)/total(reference)`) and one ULP either side | [x] |
| 34 | `match` | `bins∈{1..40}`, `test` random / `reference` all-zero (energy gate: `x < threshold*0`) and vice versa | [x] |
| 35 | `match` + `spectral_contrast` | composed check: `bins∈{1..40}` uniform, reproduce `match`'s pipeline by calling the low-level `spectral_contrast` export on the reinterpreted buffers and confirm both `.so`s agree at the seam | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the default build is the only
configuration. `check_features.sh` verifies that mechanically and runs the suite
under both the default and `--no-default-features`, and against both the
`release` and `debug` Rust `.so`.

## How this was verified (reproduce)

```bash
# 1. C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + symbol parity
cd translation && cargo build --release && ./check_symbols.sh

# 3. Phase B + C differential suite (all rows), every feature combo, both profiles
./check_features.sh

# 4. Heavier randomized pass
DIFF_ITERS=60000 cargo test --release --test configs --test errors

# 5. Prove the suite has teeth
./mutation_check.sh
```

`tests/harness.rs` additionally asserts the two loaded `.so`s are distinct files
and that `match` / `spectral_contrast` resolve to different addresses in each,
so the suite cannot silently compare one implementation against itself.
