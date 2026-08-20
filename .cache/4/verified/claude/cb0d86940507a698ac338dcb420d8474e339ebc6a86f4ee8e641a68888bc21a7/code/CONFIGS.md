# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

Derived **mechanically** from the branches `c_src/src/lib.c` actually takes.

## Axis enumeration (what the C code branches on)

### Axis 0 — runtime options / modes / flags: **NONE**

Verified mechanically:

* `include/lib.h` declares exactly **one** function (line 7) and **one** type.
  There is no `_init`, `_set_*`, `_configure`, context/handle, or options
  struct — nothing to set.
* `grep -E 'enum|union' c_src/src/lib.c c_src/include/lib.h` → **0 matches**,
  so there is no mode/format/flag enum (hence no invalid-enum axis).
* No file-scope mutable state (`grep '^static <type> <name>(=|;)'` → none) and
  no `malloc/calloc/realloc/free` → the library is **pure and stateless**. The
  same input always yields the same output; there is no configuration to vary.
* No `#if`/`#ifdef`/`defined()` in the sources and no `option()` /
  `target_compile_definitions` in `CMakeLists.txt` → **one** build config.
* `Cargo.toml` has no `[features]` → **one** feature combination.

### Axis 1 — public entry points (the FULL set, lowest level included)

`tritanopia` is simultaneously the highest *and* the lowest-level public entry
point: the five pipeline stages (`cbNorm`, `cbRemoveGammaRGB`, `Tritanopia`,
`cbApplyGammaRGB`, `cbDenorm`) are all `static` (internal linkage) and are
therefore **not** reachable across the `.so` boundary — they are absent from
`nm -D` in *both* libraries. There is no convenience-wrapper/low-level split to
be fooled by: the only way to exercise the pipeline is end to end, which is
what every row below does. The composed pipeline
`cbNorm → cbRemoveGammaRGB → Tritanopia → cbApplyGammaRGB → cbDenorm` is
covered in full by row 26 (exhaustive).

### Axis 2 — `cbRemoveGammaRGB` branch, **per channel** (lib.c:13/15/17)

`RGB.c > 0.04045` where `c = u8/255`. Boundary computed exactly:
`10/255 = 0.0392156877` (NOT `>`), `11/255 = 0.0431372561` (`>`).
→ 2 states per channel: **linear** `u8 ∈ 0..=10` / **pow** `u8 ∈ 11..=255`.
Independent per channel → **8** combinations.

### Axis 3 — `cbApplyGammaRGB` branch, **per output channel** (lib.c:36/39/42)

`v > 0.00313080495356037151702786377709`. 2 states per channel
(**linear** `v <= t`, incl. all negatives / **pow** `v > t`).
Reachable counts over all 2^24 inputs: linear branch taken
**1 796 020** (R), **88 320** (G), **88 320** (B).

### Axis 4 — `cbDenorm` narrowing-cast range class, **per output channel**

3 states: `< 0` / `0..=255` / `> 255`. Measured reachable ranges of the
`v*255+0.5` argument over all 2^24 inputs:

| channel | min | max | `<0` count | `>255` count |
|---|---|---|---|---|
| R | **-419.2283** | **269.2830** | 1 666 521 | 191 482 |
| G | 0.5000 | 255.5000 | 0 | 1 280 (all exactly `255.5`) |
| B | 0.5000 | 255.5000 | 0 | 1 280 (all exactly `255.5`) |

### Axis 5 — matrix cross-term magnitude (lib.c:50–52)

`Tritanopia` mixes three very different coefficient scales that a translation
can silently mistype: `~1.0`, `~0.127`/`~0.874`, and the near-zero
`-4.486E-11` (**negative**) and `+3.1113E-10` (**positive**). Only a large `R`
with `G=B=0` isolates the sign/magnitude of the two tiny terms. Their
contribution is ~1e-8 at u8 scale, i.e. below the output quantum, so this axis
is verified by *source-level literal audit* in addition to differential testing
(see the anti-vacuity section).

### Axis 6 — input shape / cardinality

The input is a fixed 3-byte struct — there is no size, count, width, element
type, byte order, or format axis (no arrays, no buffers, no lengths). The only
"shape" axes are the **value** patterns: extremes `{0, 255}`, the Axis-2
boundary `{10, 11}`, near-extremes `{1, 254}`, midpoint `{128}`, grayscale
`R=G=B`, single-channel-only, and full-range random.

### Axis 7 — ABI shape

`sizeof(cb_rgb_255) == 3`, `alignof == 1` (confirmed at runtime). Class
INTEGER → passed in low 3 bytes of `RDI`, returned in low 3 bytes of `RAX`;
the other 5 bytes of each register are unspecified.

## Configuration-surface table

Cross-product of Axes 2–7, pruned to the combinations the C actually
distinguishes. Every row is driven with **many randomized inputs from that
row's sub-domain** (fixed seed `0x5EED_1234`, 20 000 samples/row unless the
sub-domain is smaller, in which case it is enumerated exhaustively).

| # | entry point(s) | configuration (options set + input shape) | ✓ |
|---|----------------|--------------------------------------------|---|
| 1 | `tritanopia` | Axis2 = (linear, linear, linear): `R,G,B ∈ 0..=10` — 1331 inputs, enumerated exhaustively | [x] |
| 2 | `tritanopia` | Axis2 = (linear, linear, **pow**): `R,G ∈ 0..=10`, `B ∈ 11..=255` | [x] |
| 3 | `tritanopia` | Axis2 = (linear, **pow**, linear): `R,B ∈ 0..=10`, `G ∈ 11..=255` | [x] |
| 4 | `tritanopia` | Axis2 = (linear, **pow**, **pow**): `R ∈ 0..=10`, `G,B ∈ 11..=255` | [x] |
| 5 | `tritanopia` | Axis2 = (**pow**, linear, linear): `G,B ∈ 0..=10`, `R ∈ 11..=255` | [x] |
| 6 | `tritanopia` | Axis2 = (**pow**, linear, **pow**): `G ∈ 0..=10`, `R,B ∈ 11..=255` | [x] |
| 7 | `tritanopia` | Axis2 = (**pow**, **pow**, linear): `B ∈ 0..=10`, `R,G ∈ 11..=255` | [x] |
| 8 | `tritanopia` | Axis2 = (**pow**, **pow**, **pow**): `R,G,B ∈ 11..=255` | [x] |
| 9 | `tritanopia` | Axis2 exact boundary: every channel `∈ {10, 11}` — all 8 combos enumerated (flips linear↔pow with a 1-LSB input change) | [x] |
| 10 | `tritanopia` | Axis3 R = **linear** (`R_out <= 0.0031308`, incl. negative): sub-domain `B ≫ G`, i.e. `B ∈ 128..=255`, `G ∈ 0..=8` | [x] |
| 11 | `tritanopia` | Axis3 R = **pow** (`R_out > 0.0031308`): `R ∈ 32..=255`, `G,B` random | [x] |
| 12 | `tritanopia` | Axis3 G = **linear**: `G,B ∈ 0..=4` (G_out driven to ≈0), `R` random full range | [x] |
| 13 | `tritanopia` | Axis3 G = **pow**: `G ∈ 64..=255`, `R,B` random | [x] |
| 14 | `tritanopia` | Axis3 B = **linear**: `G,B ∈ 0..=4`, `R` random full range | [x] |
| 15 | `tritanopia` | Axis3 B = **pow**: `B ∈ 64..=255`, `R,G` random | [x] |
| 16 | `tritanopia` | Axis4 R = **`< 0`** (UB wraparound, → ERRORS E1): `B ∈ 200..=255`, `G,R ∈ 0..=16` | [x] |
| 17 | `tritanopia` | Axis4 R = **`> 255`** (UB wraparound, → ERRORS E2): `R,G ∈ 240..=255`, `B ∈ 0..=16` | [x] |
| 18 | `tritanopia` | Axis4 R = **in range** `0..=255`: `R=G=B` random (identity-ish path) | [x] |
| 19 | `tritanopia` | Axis4 G at exact `255.5` upper boundary: `G=B=255`, `R` all 256 values enumerated | [x] |
| 20 | `tritanopia` | Axis4 B at exact `255.5` upper boundary: `G=B=255`, `R` all 256 values (same 1280-input family as 19) | [x] |
| 21 | `tritanopia` | Axis6 extremes: all **8 vertices** of `{0,255}^3`, enumerated | [x] |
| 22 | `tritanopia` | Axis6 grayscale `R=G=B`: all **256** values enumerated | [x] |
| 23 | `tritanopia` | Axis6 single channel non-zero: `(v,0,0)`, `(0,v,0)`, `(0,0,v)` for all 256 `v` — **768** inputs, enumerated. Row 23 with `(255,0,0)` is the only shape in which Axis5's tiny `-4.486E-11` / `+3.1113E-10` cross-terms are the *dominant* term rather than being swamped by the `~0.874*G` term (their effect is still ~1e-8 at u8 scale, hence below the output quantum — see the M19 mutation note below) | [x] |
| 24 | `tritanopia` (ABI) | Axis7: upper 5 bytes of the argument register set to garbage (`0xDEADBEEF00 \| rgb`) — must not affect the result (→ ERRORS E14) | [x] |
| 25 | `tritanopia` (ABI) | Axis7: struct-by-value layout — `sizeof==3`, `alignof==1`, result read from low 3 bytes of `RAX` only (→ ERRORS E15) | [x] |
| 26 | `tritanopia` | **EXHAUSTIVE: all 2^24 = 16 777 216 inputs.** This is the complete cross-product of Axes 2/3/4/5/6 — every reachable combination of every branch at every value, with no sampling | [x] |
| 27 | `tritanopia` | Axis6 near-extremes / midpoint: `{1, 10, 11, 127, 128, 254, 255}^3` — 343 inputs, enumerated | [x] |
| 28 | `tritanopia` | Uniform random over the full `0..=255` cube (property-style, 200 000 samples, seeded) — cross-check that random sampling agrees with the exhaustive sweep | [x] |

Row 26 subsumes rows 1–23 and 27–28 by construction; the narrower rows are
kept because they localise a divergence to a specific branch when one occurs,
which a single 16.7 M-input pass/fail cannot do.

## Feature/configuration matrix

| build config | rows verified |
|---|---|
| `cargo test` (default features) | 1–28 |
| `cargo test --no-default-features` | 1–28 |
| C reference `-O0` (default CMake config) | 1–28 |
| C reference `-O2` (extra assurance, separate build dir) | 1–28 |
| Rust `--release` (`panic=abort`) and Rust debug `.so` | 1–28 |

Because there is exactly **one** feature combination (no `[features]` in
`Cargo.toml`) and exactly **one** C build configuration (no `#ifdef`, no CMake
options), this matrix is the complete configuration space; the `-O0`/`-O2` and
debug/release axes are added voluntarily to prove the float semantics are not
optimisation-dependent.

---

## Verification results (final)

All 28 rows pass, in every configuration, with the randomized inputs described
above. Reproduce with `./run_all.sh`.

Per-run test totals: **41 tests** = 27 (`phase_b_configs.rs`, rows 1–25 + 27–28
+ a harness-sanity test) + 1 (`phase_b_exhaustive.rs`, row 26) + 13
(`phase_c_errors.rs`, Phase C).

```
=== Phases B+C : features='--no-default-features' profile=debug   C=build       ===  41 passed
=== Phases B+C : features='--no-default-features' profile=debug   C=c_build_O2  ===  41 passed
=== Phases B+C : features='--no-default-features' profile=release C=build       ===  41 passed
=== Phases B+C : features='--no-default-features' profile=release C=c_build_O2  ===  41 passed
=== Phases B+C : features='default'              profile=debug   C=build       ===  41 passed
=== Phases B+C : features='default'              profile=debug   C=c_build_O2  ===  41 passed
=== Phases B+C : features='default'              profile=release C=build       ===  41 passed
=== Phases B+C : features='default'              profile=release C=c_build_O2  ===  41 passed
  ALL CONFIGURATIONS PASSED
```

The `-O0` and `-O2` C builds are genuinely different code (`.text` 3077 vs
2159 bytes), so the float semantics are demonstrably not optimisation-dependent.

### Row 26 (exhaustive) — headline result

```
row26 exhaustive: 16777216 inputs, 0 divergences;
                  E1 negative-R inputs = 1666521, post-matrix R>1.0 inputs = 171997
```

**Every one of the 16 777 216 possible inputs produces byte-identical output in
C and Rust.** For this library that is not a sample — it is the *entire* input
domain, so rows 1–23 and 27–28 are mathematically subsumed. The two
reachability counters are asserted, not merely printed, so the test also fails
if the implementation-defined conversion paths ever stop being exercised.

### Anti-vacuity evidence

A green suite is only meaningful if it can go red. `./mutation_test.sh` injects
15 realistic mistranslations:

* **10 mutations are caught**, e.g. a saturating `as u8` cast (18 tests fail),
  `round` instead of `trunc` (28), `12.92 → 12.29` (27), `/256` instead of
  `/255` (27), doing the gamma math in `f32` instead of `f64` (2), and
  re-grouping `(R + aG) - bB` as `R + (aG - bB)` (caught *only* by row 26,
  which is precisely why the exhaustive row is worth having).
* **5 mutations correctly survive** because they are provably
  semantics-preserving, not because the tests are blind:
  - swapping `0.12739886310880f` ↔ `0.12739886341072f`, and
    `0.87390929928361f` ↔ `0.87390929725848f`: the C writes 14 significant
    decimal digits but suffixes them with `f`, and each pair rounds to the
    **identical f32 bit pattern** (`d974023e` and `85b85f3f` respectively —
    the decimal differences of ~3e-10 and ~2e-9 are ~20× smaller than the f32
    spacing of 1.5e-8 / 6.0e-8 at those magnitudes);
  - perturbing the `pow` exponent by 1e-11 and flipping the sign of the
    `4.486E-11` cross-term: both shift the result by ~1e-8 at u8 scale, far
    below the output quantum, and the exhaustive sweep confirms **0** of
    16 777 216 outputs change;
  - altering the NaN sentinel, whose branch `ERRORS.md` E6 proves unreachable.

  A **source-level literal audit** independently confirms every numeric
  constant in `c_src/src/lib.c` appears in `src/lib.rs` with the identical
  value (including both members of each near-identical pair, so no copy-paste
  collapse occurred), and that the two tiny cross-term signs and the
  left-to-right operand grouping match the C exactly.

### Pitfall found and fixed during Phase B

`cargo test` alone does **not** rebuild/uplift the cdylib, so the suite
originally ran against a **stale** `.so` and three obviously-broken mutations
"survived". Fixed via `crate-type = ["cdylib", "rlib"]`, a mandatory
`cargo build` before `cargo test` in `run_all.sh`, and a hard staleness
assertion in `tests/common/mod.rs`. See `SYMBOLS.md` for details. Any future
run of this suite that silently regresses in the same way will now fail loudly
instead of passing vacuously.
