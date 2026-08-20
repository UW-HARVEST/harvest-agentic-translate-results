# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Mechanical derivation of the axes

The C source branches on exactly the following things. Nothing else in
`c_src/src/lib.c` is a decision point.

```
$ grep -nE 'if|\?|switch|#if' c_src/src/lib.c
6:    R = ((float)(R > 0.04045 ? pow((R + 0.055) / 1.055, 2.4) : R / 12.92));
7:    G = ((float)(G > 0.04045 ? pow((G + 0.055) / 1.055, 2.4) : G / 12.92));
8:    B = ((float)(B > 0.04045 ? pow((B + 0.055) / 1.055, 2.4) : B / 12.92));
18:    if (High < Low) {
```

* **No `#ifdef` / `#if` at all** -> the C has exactly **one** compile-time
  configuration. `c_src/CMakeLists.txt` defines no `target_compile_definitions`
  and no options; the only knobs are `SHARED`, the `include`/`src` include dirs
  and `-lm`.
* `Cargo.toml` has **no `[features]` section** -> the Rust crate likewise has
  exactly **one** feature combination: the empty set. `--no-default-features`
  with no features is the *only* valid combination (see "Feature combinations"
  at the bottom).

### Runtime axes

| axis | where it comes from | values |
|------|---------------------|--------|
| **L** — per-channel linearization branch | ternary on lines 6/7/8 | `lin` (channel byte `n <= 10`, since `10/255.f = 0.039215688 <= 0.04045`) or `pow` (`n >= 11`, since `11/255.f = 0.043137256 > 0.04045`). **Chosen independently for each of R, G, B** -> 2^3 = 8 masks per colour, 8x8 = 64 per call. |
| **S** — the `High < Low` swap | `if` on line 18 | `no-swap` (`LumA >= LumB`, incl. the `LumA == LumB` equality edge which does *not* swap) or `swap` (`LumA < LumB`). |
| **D** — denominator magnitude | unguarded `High / Low` on line 21 | `zero` (`Low == +0.0`, i.e. the low colour is `{0,0,0}`), `tiny` (`Low` ~ 1e-5, e.g. `{0,0,1}`), `normal`. |
| **W** — which luminance weight is exercised | `0.2126f*R + 0.7152f*G + 0.0722f*B`, line 9 | `R-only`, `G-only`, `B-only`, `all`. Isolating a weight catches a transposed/swapped coefficient, which an all-channels-equal (greyscale) test cannot see. |
| **V** — channel byte value shape | the `unsigned char` -> `float` -> `double` conversions | `0`, `1`, `2`, `9`, `10` (last `lin`), `11` (first `pow`), `12`, `127`, `128`, `253`, `254`, `255`, uniform-random. |
| **P** — argument-register padding | x86-64 SysV: 3-byte struct in low 24 bits of `rdi`/`rsi` | `zeroed` or `junk`. |

### Public entry points

`c_src/include/lib.h` declares exactly one function, `contrast_ratio`. The two
lower-level functions `cbLuminance` and `cbContrastRatio` are `static`, so they
are **not** reachable across the `.so` boundary — `nm -D` confirms they are not
dynamic symbols, so an external consumer *cannot* call them and there is no
lower-level entry point to test directly. They are covered transitively, and to
avoid the "only tested the convenience wrapper" blind spot they are covered
*densely*: rows C11-C14 sweep the full 0..=255 domain of every individual channel
position (so every input `cbLuminance` can ever see per channel is exercised) and
row C13 exhaustively covers all 65 536 `(LumA, LumB)` greyscale pairings feeding
`cbContrastRatio`.

## Configuration-surface table

One row per combination the C treats differently. All rows use randomized inputs
with a fixed seed (SplitMix64, seeded per row) unless marked *exhaustive*.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `contrast_ratio` | **L**=`lin,lin,lin` for A **and** B (all 6 bytes in `0..=10`), **D**=`normal`/`zero`, both **S** — the pure `x/12.92` path, no `pow` call at all. Randomized. | [x] |
| C2  | `contrast_ratio` | **L**=`pow,pow,pow` for A **and** B (all 6 bytes in `11..=255`), **D**=`normal`, both **S** — the pure `pow` path. Randomized. | [x] |
| C3  | `contrast_ratio` | **L**=all-`lin` for A, all-`pow` for B -> forces **S**=`swap` (dark A, bright B). Randomized. | [x] |
| C4  | `contrast_ratio` | **L**=all-`pow` for A, all-`lin` for B -> forces **S**=`no-swap` (bright A, dark B). Randomized. | [x] |
| C5  | `contrast_ratio` | **L** = full cross product of the 8 branch masks of A x the 8 branch masks of B (**64 sub-configurations**), each with randomized bytes drawn from the matching `0..=10` / `11..=255` region. This is the axis a greyscale-only test completely misses. Randomized per sub-config. | [x] |
| C6  | `contrast_ratio` | **S**=`no-swap` with **D**=`normal`, filtered so `LumA > LumB` strictly. Randomized, rejection-sampled. | [x] |
| C7  | `contrast_ratio` | **S**=`swap` with **D**=`normal`, filtered so `LumA < LumB` strictly. Randomized, rejection-sampled. | [x] |
| C8  | `contrast_ratio` | **S** equality edge: `A == B` (so `LumA == LumB`, `High < Low` false) -> ratio must be the identical bit pattern on both sides. *Exhaustive* over all 256 greys plus randomized colours. | [x] |
| C9  | `contrast_ratio` | **W**=`R-only` / `G-only` / `B-only`: A is `{n,0,0}`, `{0,n,0}`, `{0,0,n}` for **all** `n` in `0..=255`, against several fixed B (white, black, mid-grey, and each single-channel partner). Isolates the three luminance coefficients. *Exhaustive in n.* | [x] |
| C10 | `contrast_ratio` | Corner cross product: A and B each range over the 8 corners `{0,255}^3` -> **64 deterministic pairs**. Covers **D**=`zero`, **S** both ways, **L** both extremes, ratio `== 1`, `+inf` and `NaN` simultaneously. *Exhaustive.* | [x] |
| C11 | `contrast_ratio` | Single-position sweep: for each of the **6** channel positions (`A.R,A.G,A.B,B.R,B.G,B.B`), vary it over **all** `0..=255` while the other 5 bytes are held at each of several fixed backgrounds (all-0, all-11, all-127, all-255, and a random one). Guarantees 100 % per-position domain coverage, incl. every **V** boundary. *Exhaustive.* | [x] |
| C12 | `contrast_ratio` | **V** boundary-focused cross product: every byte drawn from `{0,1,2,9,10,11,12,127,128,253,254,255}` in a large randomized sweep, so the `> 0.04045` boundary (`10` vs `11`) is hit in every position and in every combination with the other positions. Randomized. | [x] |
| C13 | `contrast_ratio` | *Exhaustive* greyscale x greyscale: `A = {n,n,n}`, `B = {m,m,m}` for **all** `(n,m)` in `0..=255 x 0..=255` = **65 536 pairs**. Covers every reachable `(LumA, LumB)` ordering including all `D`=`zero` cases. | [x] |
| C14 | `contrast_ratio` | *Exhaustive* 2-D sweep: `A.R x A.G` over all `256 x 256` with `A.B` and B fixed (repeated for 3 fixed backgrounds), plus `A.R x B.R` over all `256 x 256`. Catches cross-channel interaction bugs. | [x] |
| C15 | `contrast_ratio` | Large uniform randomized sweep over all 6 bytes independently (fixed seed, 300 000 cases) — the unbiased "real consumer" workload. | [x] |
| C16 | `contrast_ratio` | **D**=`zero` crossed with **L**: one colour is exactly `{0,0,0}` while the other ranges over all 8 branch masks with randomized bytes, in both argument positions (so both the `swap` and `no-swap` route into `x / +0.0`). Randomized. | [x] |
| C17 | `contrast_ratio` | **D**=`tiny`: the low colour is one of the 3 darkest non-black single-channel colours (`{1,0,0}`,`{0,1,0}`,`{0,0,1}`) and the high colour ranges over randomized bright colours — denominator ~1e-5..1e-4, unguarded. Randomized. | [x] |
| C18 | `contrast_ratio` | **P**=`junk` crossed with valid configs: same randomized valid inputs as C15 but invoked through an `extern "C" fn(u64, u64) -> f32` signature with all 5 padding bytes of each argument register filled with pseudo-random junk. Verifies the `#[repr(C)]` 3-byte-struct ABI classification matches. Randomized. | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table and no optional dependencies:

```
$ grep -n '\[features\]' Cargo.toml
(no matches)
```

Therefore the complete enumeration of valid feature combinations is a single
element — the empty set:

| # | combination | command |
|---|-------------|---------|
| 1 | *(none)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

For completeness the harness script also runs the implicit-default build
(`cargo test`), which is identical because there is no `default` feature. Both
are executed by `./check_all_features.sh`, and every row above passes under
both.

## Phase B results

All 18 rows pass, bit-for-bit, across their randomized inputs, under **both**
feature invocations (`--no-default-features` and the implicit default) and in
**both** cargo profiles (`dev` and `release`):

```
$ ./check_all_features.sh            # dev
$ ./check_all_features.sh --release  # release
=== features found: 0 (none) ===
=== 1 combination(s) to verify ===
########## --no-default-features (empty feature set) ##########
test result: ok. 18 passed; 0 failed   (tests/phase_b_configs.rs)
test result: ok. 11 passed; 0 failed   (tests/phase_c_errors.rs)
test result: ok.  3 passed; 0 failed   (tests/phase_d_symbols.rs)
########## default features ##########
test result: ok. 18 passed; 0 failed
test result: ok. 11 passed; 0 failed
test result: ok.  3 passed; 0 failed
ALL 1 feature combination(s) + default: PASS
```

Rows map to tests as `C<n>` -> `c<nn>_*` in `tests/phase_b_configs.rs`.

### Extended exhaustive sweeps (`tests/phase_b_exhaustive.rs`)

Run with `cargo test --release -- --ignored`. The full 256^6 = 2.8e14 pair space
is not enumerable, but `contrast_ratio` factors through `cbLuminance`, whose
domain (2^24 colours) *is*. Holding one argument fixed and sweeping the other
over all 16 777 216 colours therefore covers **100 % of the reachable inputs of
the inner luminance routine**, in both argument positions:

| test | sweep | pairs | result |
|------|-------|-------|--------|
| `x1` | all 2^24 colours as A, B = white | 16 777 216 | bit-identical |
| `x2` | all 2^24 colours as A, B = mid-grey (both swap branches) | 16 777 216 | bit-identical |
| `x3` | all 2^24 colours as A, B = `{1,2,3}` (tiny denominator) | 16 777 216 | bit-identical |
| `x4` | all 2^24 colours as B, A = white | 16 777 216 | bit-identical |
| `x5` | all 2^24 colours as B, A = black (the whole inf/NaN surface) | 16 777 216 | bit-identical |
| `x6` | all 2^24 colours as B, A = `{10,11,10}` (branch boundary) | 16 777 216 | bit-identical |
| `x7` | uniform random pairs, both arguments varying | 40 000 000 | bit-identical (39 999 997 finite, 3 `+inf`, 0 `NaN`) |
| `x8` | stratified colour set (12 167 colours) crossed with itself | 148 035 889 | bit-identical |

Total: **~289 million** input pairs verified bit-for-bit through the FFI
boundary, in 49.5 s.

### Robustness to the C compiler's optimization level

The C ground truth was additionally rebuilt at `-O2` and `-O3` and the entire
suite re-run against each (`C_LIB_PATH=... DIFF_N=1000000 cargo test --release`):
all 32 tests pass against all three C builds. `objdump | grep -cE 'vfmadd|fmadd'`
is `0` at every level, confirming GCC never contracts the luminance dot product
into an FMA and that the Rust translation must not either.
