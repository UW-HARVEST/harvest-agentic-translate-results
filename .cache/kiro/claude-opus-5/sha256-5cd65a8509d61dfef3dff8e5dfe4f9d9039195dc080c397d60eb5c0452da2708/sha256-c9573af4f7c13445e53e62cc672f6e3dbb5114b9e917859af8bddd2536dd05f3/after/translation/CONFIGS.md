# CONFIGS.md — configuration / valid-input surface table

## Axes, derived from the C source

The library has **no** runtime options, flags, modes, `#ifdef`s or setters —
`grep -c '#if\|switch\|enum' c_src/src/*.c c_src/include/*.h` is `0`, and the
only `#define` is `N_SMOOTH 16`. The configuration surface is therefore made of
the *entry point chosen* plus the *shape and value class of the data*, which is
exactly what the C branches on:

| axis | values the C actually distinguishes | where in the C |
|---|---|---|
| **E — entry point** | `spectral_contrast` (lowest level, `float` lanes) · `match` (composed pipeline, `double` lanes) | `include/match.h` |
| **N — count** | `n <= 0` (all loops zero-trip) · `n == 1` (`differentiate` degenerates) · `n < 16` (every `smoothen` row is a truncated kernel) · `n == 16` · `n > 16` (interior rows use the full kernel, last 15 are truncated) · `n` odd vs even (the `float` reinterpretation in `match` reads `n` 4-byte lanes out of `n` 8-byte slots) | `match.c:16` `j < N_SMOOTH && i+j < length`, `match.c:24` `i < length-1`, `spectral_contrast.c` 4-byte stride |
| **T — threshold** | `-inf` · `< 0` · `-0.0` / `+0.0` · `0 < t < 1` · `1.0` · `> 1` · `+inf` · `NaN` — used twice, once multiplied by a total and once compared against a correlation in `[-1, 1]` | `match.c:37`, `match.c:40` |
| **D — data class** | finite random · all-zero (`magnitude == 0`) · constant (`differentiate` → all zero) · ramp · single spike · huge (`|x| ~ 1e300`, `Σx²` overflows) · tiny/subnormal · `±0.0` · `±inf` · `QNaN` · `SNaN` · bit patterns whose **low 32 bits** form `float` `inf`/`NaN`/subnormals (the only part `match` ever forwards to `spectral_contrast`) | `dot_product`, `normalize`, `sqrt` |
| **A — aliasing** | distinct buffers · `a == b` / `test == reference` (no `restrict` anywhere) | `include/match.h` |
| **R — relation** | independent · identical · scaled (`ref = k·test`) · negated (`ref = -test`) | `match.c:37` gate, `dot_product` sign |

`spectral_contrast` **mutates both of its arguments in place**, so every row
below compares, byte-for-byte: the returned `double`'s **bit pattern** and the
**final contents of both buffers**. `match` must leave its inputs untouched, so
its rows also assert the input buffers are bitwise unchanged.

Every row is driven with many randomized inputs (fixed seed, `SEED = 0x5DEECE66D`,
`ITERS` per row) rather than one hand-picked vector.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `spectral_contrast` | `n = 0`, non-null buffers, D=finite random | [x] |
| 2  | `spectral_contrast` | `n = 1`, D=finite random in `[-1,1]`, A=distinct | [x] |
| 3  | `spectral_contrast` | `n = 2`, D=finite random, A=distinct | [x] |
| 4  | `spectral_contrast` | `n = 3` (odd), D=finite random, A=distinct | [x] |
| 5  | `spectral_contrast` | `n = 15` (just under `N_SMOOTH`), D=finite random | [x] |
| 6  | `spectral_contrast` | `n = 16` (== `N_SMOOTH`), D=finite random | [x] |
| 7  | `spectral_contrast` | `n = 17`, D=finite random | [x] |
| 8  | `spectral_contrast` | `n = 33`, D=finite random | [x] |
| 9  | `spectral_contrast` | `n = 129`, D=finite random | [x] |
| 10 | `spectral_contrast` | `n = 1024`, D=finite random | [x] |
| 11 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=**all zeros** → `magnitude = 0` → `0/0` | [x] |
| 12 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=**one nonzero lane**, rest `0.0` | [x] |
| 13 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=constant value (all lanes equal) | [x] |
| 14 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=**huge** (`|x| ∈ [1e30, 3e38]`) → `Σx²` overflows `float`→`double` accumulate | [x] |
| 15 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=**tiny/subnormal** `float`s (`|x| < 1e-38`) → `magnitude` subnormal, `cvtsd2ss` under/overflow | [x] |
| 16 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=mixed `±0.0` | [x] |
| 17 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=finite + `±inf` lanes | [x] |
| 18 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=finite + **QNaN** lanes with random payloads (`addsd`/`mulss` destination-NaN rule) | [x] |
| 19 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=finite + **SNaN** lanes with random payloads (quieting) | [x] |
| 20 | `spectral_contrast` | `n ∈ {1,2,7,16,17,64}`, D=**fully random 32-bit patterns** (any class: NaN/inf/subnormal/normal, both signs) | [x] |
| 21 | `spectral_contrast` | **A = aliased**, `a == b`, `n ∈ {1,2,7,16,17,64}`, D=finite random → double normalisation of one buffer | [x] |
| 22 | `spectral_contrast` | **A = aliased**, `a == b`, D=random bit patterns | [x] |
| 23 | `spectral_contrast` | R=negated (`b[i] = -a[i]`), D=finite random, `n ∈ {1,7,16,17}` → contrast ≈ `-1` | [x] |
| 24 | `spectral_contrast` | R=scaled (`b = k·a`, random `k`), D=finite random | [x] |
| 25 | `spectral_contrast` | R=identical contents, distinct buffers, D=finite random → contrast ≈ `+1` | [x] |
| 26 | `spectral_contrast` | misaligned-in-`double` view: `n` odd, buffer from a `f64` allocation (the exact shape `match` produces) | [x] |
| 27 | `match` | `bins = 0`, T ∈ full threshold set, non-null buffers (row 3/4 of ERRORS.md) | [x] |
| 28 | `match` | `bins = 1`, T ∈ full set, D=finite random positive | [x] |
| 29 | `match` | `bins = 2`, T ∈ full set, D=finite random positive | [x] |
| 30 | `match` | `bins = 3` (odd, `< N_SMOOTH`), T ∈ full set, D=finite random positive | [x] |
| 31 | `match` | `bins = 15`, T ∈ full set, D=finite random positive | [x] |
| 32 | `match` | `bins = 16` (== `N_SMOOTH`), T ∈ full set, D=finite random positive | [x] |
| 33 | `match` | `bins = 17`, T ∈ full set, D=finite random positive | [x] |
| 34 | `match` | `bins = 31`/`32`/`33` (kernel boundary ± odd/even), T ∈ full set, D=finite random positive | [x] |
| 35 | `match` | `bins = 64`, T ∈ full set, D=finite random positive | [x] |
| 36 | `match` | `bins = 257`, T ∈ full set, D=finite random positive | [x] |
| 37 | `match` | `bins = 1000`, T ∈ full set, D=finite random positive | [x] |
| 38 | `match` | D=**signed** random (spectra with negative bins → totals can be negative, flipping the gate direction), T ∈ full set | [x] |
| 39 | `match` | D=**all zeros** in both inputs → gate `0 < t*0`, `magnitude = 0` | [x] |
| 40 | `match` | D=**constant** (`differentiate` → all-zero preprocessed vectors → `magnitude = 0` → `NaN`) | [x] |
| 41 | `match` | D=**ramp** / monotone, T ∈ full set | [x] |
| 42 | `match` | D=**single spike**, spike index swept across the buffer (interior vs last-15 `smoothen` rows) | [x] |
| 43 | `match` | D=**huge** (`|x| ~ 1e300`) → `total` overflows to `±inf`, `inf - inf` in `differentiate` → `NaN` | [x] |
| 44 | `match` | D=**tiny/subnormal `double`s** → preprocessed low words are subnormal/zero `float`s | [x] |
| 45 | `match` | D=finite + `±inf` lanes, T ∈ full set | [x] |
| 46 | `match` | D=finite + **QNaN/SNaN** lanes with random payloads, T ∈ full set | [x] |
| 47 | `match` | D=**fully random 64-bit patterns** (all classes), T ∈ full set | [x] |
| 48 | `match` | D engineered so the **low 32 bits** of every preprocessed `double` are a chosen `float` class (`inf`, `NaN`, subnormal) | [x] |
| 49 | `match` | **A = aliased**, `test == reference`, T ∈ full set, D=finite random | [x] |
| 50 | `match` | R=identical contents, distinct buffers → gate is `x < t·x`; contrast ≈ `1` | [x] |
| 51 | `match` | R=negated (`reference = -test`) → totals opposite sign, gate direction flips | [x] |
| 52 | `match` | R=scaled by random `k ∈ {1e-6, 0.5, 2, 1e6}` → sweeps the gate's decision boundary | [x] |
| 53 | `match` | T swept densely around the **gate boundary** `total(test)/total(reference)` (`±1 ulp`) — makes the `comisd` branch flip | [x] |
| 54 | `match` | T swept densely around the **contrast boundary** (the returned correlation `±1 ulp`) | [x] |
| 55 | `match` + `spectral_contrast` | composed check: run `match`'s exact pipeline by hand (`total` gate, two `preprocess` passes) and feed the resulting `double` buffers to the exported `spectral_contrast` of *both* libraries, verifying the `float`-reinterpretation path end to end | [x] |

Threshold set used wherever "T ∈ full set" appears:
`-inf, -1e300, -1.0, -0.5, -0.0, +0.0, f64::MIN_POSITIVE, 1e-9, 0.25, 0.5,
0.75, 1.0 - eps, 1.0, 1.0 + eps, 2.0, 1e300, +inf, NaN(payload 0x1),
NaN(payload 0x7ff...), -NaN, signalling NaN`.

## Non-vacuity: the rows were checked against deliberately broken builds

A passing suite proves nothing unless it can fail. Each mutant below was built
by editing `src/`, compiled to its own `.so`, and run against the C oracle via
the harness's `RUST_SO_OVERRIDE` hook. All were caught:

| mutant | what it breaks | caught by |
|---|---|---|
| `f32` elements → `f64` elements in `spectral_contrast` ("fixing" the `float_t` bug) | the whole reinterpretation | over-reads its buffers; aborts the `configs` binary outright and fails 14 `errors` rows |
| `mul_ss(bi, ai)` → `mul_ss(ai, bi)` | `mulss` destination operand | rows 18, 19, 20, 55 |
| `add_sd(product, sum)` → `add_sd(sum, product)` | `addsd` destination operand in `dot_product` | rows 18, 19, 20, 22, 26, 55 |
| `mulss` + widen → multiply in `double` | `FLT_EVAL_METHOD == 0` single-precision multiply | rows 3–10, 13–15, 18, … (most of the suite) |
| widen/`divsd`/narrow → divide in `f32` | `normalize`'s mixed-precision divide | rows 3–10, 13, 20, 21, 22, … |
| `add_sd(v[i], sum)` → `add_sd(sum, v[i])` in `total`/`smoothen` | `addsd` destination operand in `match.c` | **not caught, and provably unobservable** — see the module docs of `tests/nan_payload_search.rs` for the proof |
