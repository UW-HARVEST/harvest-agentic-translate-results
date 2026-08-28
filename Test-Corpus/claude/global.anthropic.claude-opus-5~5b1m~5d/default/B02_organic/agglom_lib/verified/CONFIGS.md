# CONFIGS.md — Phase A: configuration surface table

Mechanically enumerated from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

### Runtime options / modes
There is **no** init struct, no flag word, no `#ifdef`, and no global state.
`grep -n '#if' c_src/src/lib.c` → no matches. The only *mode selector* in the
public API is the `C2_TYPE` enum pair passed to `f2`:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `typeA` | `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1) | `switch (typeA)` lib.c:84 |
| `typeB` | `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1) | `switch (typeB)` lib.c:86, 96 |
| `f7` channel mode | `channels == 2` vs `channels != 2` | `(channels != 2)` / `(channels == 2)` lib.c:453-455 |
| `f7` depth mode | `bitdepth == 32` vs `bitdepth != 32` | `(bitdepth != 32)` lib.c:455 |
| `f11` achromatic | `s == 0` vs `s != 0` | lib.c:872 |
| `f11` hue sector | 6 range arms + final `else` (7 states) | lib.c:881-909 |
| `f12` achromatic | `s == 0` vs `s != 0` | lib.c:919 |
| `f12` sector | `i` ∈ {0,1,2,3,4} + `default` (6 states) | `switch (i)` lib.c:931 |
| `f13` degenerate | `delta == 0 \|\| max == 0` vs neither | lib.c:984 |
| `f13` max channel | `r == max`, else `g == max`, else `b` (3 states) | lib.c:991-996 |
| `f13` hue wrap | `h < 0` vs `h >= 0` | lib.c:998 |
| `f3` sign/overflow | 8 nested arms + `r >= 0` fix-up | lib.c:110-139 |

### Input shapes / value classes
* **`float` classes**: positive normal, negative normal, `+0.0`, `-0.0`,
  subnormal, `FLT_MIN`, `FLT_MAX`, `+Inf`, `-Inf`, quiet `NaN` (varied payload
  and sign), signaling-`NaN` bit patterns, and uniformly random 32-bit patterns.
* **`int` classes**: `0`, `±1`, `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`,
  random.
* **`uint32_t` classes**: `0`, `1`, `0xFFFF`, `0x10000`, `UINT32_MAX`, random.
* **`uint64_t` classes**: `0`, `1`, `UINT64_MAX`, random.
* **`uint16_t`**: the **entire** 65536-value domain (`f10`).
* **Geometric shapes** (`c2*`/`f9`): disjoint, touching, overlapping, contained,
  identical, inverted AABB (`min > max`), zero-radius circle, negative-radius
  circle, degenerate/collinear triangle, huge and tiny coordinates.
* **Aliasing**: `dest == src` for `f11`/`f12`/`f13`.

### Feature combinations
`translation/Cargo.toml` declares **no** `[features]` section, so the only
build configuration is the default one (verified by
`cargo read-manifest | jq .features` → `{}`). Phases B–C are additionally run
under `--no-default-features` and in both `dev` and `release` profiles, which
is the complete cross-product of build configurations available.

## Configuration table

Each row is exercised with **many randomized inputs** (fixed seed
`0x243F_6A88_85A3_08D3`, the digits of π — see `tests/common/mod.rs`) plus the
hand-picked boundary values that select that row's branch.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| C1 | `c2V` | random 32-bit float bit patterns (all classes, incl. NaN payloads, ±0, subnormal) — verifies the 8-byte struct return ABI | [x] |
| C2 | `c2Maxv` | both components: `a > b`, `a < b`, `a == b`, `±0` vs `∓0`, NaN in `a`, NaN in `b`, NaN in both (`>` is false → picks `b.x`) | [x] |
| C3 | `c2Minv` | mirror of C2 with `<` (NaN → picks `b`) | [x] |
| C4 | `c2Clampv` | `a` below `lo`, inside, above `hi`; **inverted** range (`lo > hi`); NaN in `a`/`lo`/`hi`; ±0 boundaries | [x] |
| C5 | `c2Sub` | random pairs incl. `Inf - Inf` → NaN, `x - x`, `±0 - ±0`, overflow to `±Inf`, NaN operands (checks `subss` NaN-operand choice) | [x] |
| C6 | `c2Dot` | random pairs; `0 * Inf` → NaN; `Inf*x + -Inf*y` → NaN; catastrophic cancellation; NaN in either component (checks `mulss`/`addss` operand order) | [x] |
| C7 | `c2CircletoCircle` | overlapping / touching (`d2 == r2`, strict `<` → 0) / disjoint / identical circles; `r == 0`; **negative** radii; `r = Inf`; NaN centre or radius | [x] |
| C8 | `c2CircletoAABB` | circle centre inside AABB, outside on each of the 8 sides/corners, exactly on the edge; zero-area AABB; **inverted** AABB (`min > max`); `r == 0`; negative `r`; NaN fields | [x] |
| C9 | `c2AABBtoAABB` | overlapping, edge-touching (`<` is strict), disjoint on each axis/side, one contained in the other, zero-area, inverted, NaN fields (all four `<` false → returns 1) | [x] |
| C10 | `f2` low-level dispatch | `typeA=CIRCLE, typeB=CIRCLE` → `c2CircletoCircle(*A, *B)`; randomized circles | [x] |
| C11 | `f2` low-level dispatch | `typeA=CIRCLE, typeB=AABB` → `c2CircletoAABB(*A, *B)`; randomized circle + AABB | [x] |
| C12 | `f2` low-level dispatch | `typeA=AABB, typeB=CIRCLE` → **argument-swapping** arm `c2CircletoAABB(*B, *A)`; randomized AABB + circle | [x] |
| C13 | `f2` low-level dispatch | `typeA=AABB, typeB=AABB` → `c2AABBtoAABB(*A, *B)`; randomized AABBs | [x] |
| C14 | `f2` | same buffer passed as both `A` and `B` (aliasing), each of the 4 valid type pairs | [x] |
| C15 | `f3` | `v1 >= 0, v2 > 0` — the early `return v1/v2` fast path; random + `INT_MAX/1`, `0/x` | [x] |
| C16 | `f3` | `v1 >= 0, v2 < 0, v2 != INT_MIN` — `q = -(v1/-v2)`, `r = v1 % -v2`, incl. `r != 0` fix-up | [x] |
| C17 | `f3` | `v1 < 0, v1 != INT_MIN, v2 > 0` — `q = -((-v1)/v2)`, `r = -((-v1)%v2)`, fix-up path | [x] |
| C18 | `f3` | `v1 < 0, v1 != INT_MIN, v2 < 0, v2 != INT_MIN` — `q = (-v1)/(-v2)`, `r = -((-v1)%(-v2))` | [x] |
| C19 | `f3` | fully random `(i32, i32)` pairs across all four quadrants, ~40k samples | [x] |
| C20 | `f3` | small exhaustive grid `v1, v2 ∈ [-40, 40]` (all 6561 pairs) — floored-division identity across every sign combination | [x] |
| C21 | `f4` | single call, random `state[2]`; asserts return value **and** the mutated `state` array | [x] |
| C22 | `f4` | **iterated** 256-call chain from one seed (the real consumer pattern), comparing the whole double sequence and the final state | [x] |
| C23 | `f4` | `state` shapes: `{0,0}`, `{1,0}`, `{0,1}`, `{u64::MAX, u64::MAX}`, single-bit states (all 128 positions), values that make `x + y` wrap | [x] |
| C24 | `f5` | `a` in `[0, 0xFFFF]` — **exhaustive** over the low 16 bits (65536 values) | [x] |
| C25 | `f5` | `a > 0xFFFF`: random full-width `u32`, `0xFFFF_FFFF`, `0x1_0000`, single-bit values at bits 16..31 | [x] |
| C26 | `f7` | `channels != 2` (0, 1, 3, 4, 8) × `bitdepth ∈ {8,16,24,32}` × `blocksize ∈ {0,1,16,4096,65535}` | [x] |
| C27 | `f7` | `channels == 2` × `bitdepth != 32` (8,16,24) × same blocksizes — activates both `channels == 2` terms **and** the `+1` depth correction | [x] |
| C28 | `f7` | `channels == 2` × `bitdepth == 32` — the `bitdepth != 32` term is 0 | [x] |
| C29 | `f7` | overflow shapes: `blocksize`/`bitdepth`/`channels` at `UINT32_MAX`, `0x8000_0000`, and random full-width triples (~20k) | [x] |
| C30 | `f9` | non-degenerate triangle, `p` inside / on a vertex / on an edge / outside; randomized normal coordinates | [x] |
| C31 | `f9` | degenerate: `p1 == p2`, `p1 == p3`, `p2 == p3`, all three equal, collinear points → `invDenom = 1/0` | [x] |
| C32 | `f9` | extreme magnitudes: coordinates at `FLT_MAX`, `FLT_MIN`, subnormals, `±Inf`, mixed — dot products overflow/underflow | [x] |
| C33 | `f9` | fully random 32-bit bit patterns in all 8 coordinates (~20k) — stresses the `mulss`/`addss`/`subss`/`divss` NaN-operand ordering of all five dot products | [x] |
| C34 | `f10` | **exhaustive** over all 65536 `uint16_t` values — covers every one of the 64 `m__offset`/`m__exponent` buckets, plus half-float `±0`, subnormals, `±Inf`, and NaN encodings | [x] |
| C35 | `f11` | `s == 0` early return with random `h`, `l` (incl. NaN `h`, NaN `l`, `-0.0` `s`) | [x] |
| C36 | `f11` | sector `0 <= h < 60`, random `s`, `l` in `[0,1]` and outside `[0,1]` | [x] |
| C37 | `f11` | sector `60 <= h < 120` | [x] |
| C38 | `f11` | third arm `h < 120 && h < 180` — reached for `120 <= h < 180` **and** for all `h < 0` (the C's quirk) | [x] |
| C39 | `f11` | sector `180 <= h < 240` | [x] |
| C40 | `f11` | sector `240 <= h < 300` | [x] |
| C41 | `f11` | sector `300 <= h < 360` | [x] |
| C42 | `f11` | final `else` (`h >= 360` or `h` NaN, incl. `+Inf`) | [x] |
| C43 | `f11` | fully random 3-float bit patterns (~20k) — hits `fmodf`, `fabsf` and every arm at random | [x] |
| C44 | `f12` | `s == 0` early return, random `h`, `v` | [x] |
| C45 | `f12` | `i == 0` (`0 <= h < 60`), random `s`, `v` | [x] |
| C46 | `f12` | `i == 1` (`60 <= h < 120`) | [x] |
| C47 | `f12` | `i == 2` (`120 <= h < 180`) | [x] |
| C48 | `f12` | `i == 3` (`180 <= h < 240`) | [x] |
| C49 | `f12` | `i == 4` (`240 <= h < 300`) | [x] |
| C50 | `f12` | `default` arm: `i == 5` (`300 <= h < 360`), `i > 5`, `i < 0`, and the `floorf` boundary `h` exactly `60*k` | [x] |
| C51 | `f12` | fully random 3-float bit patterns (~20k) — exercises `floorf` + `(int)` conversion at random | [x] |
| C52 | `f13` | `r == max` branch, `g >= b` (no hue wrap) and `g < b` (`h += 360` wrap) | [x] |
| C53 | `f13` | `g == max` branch (`h = 2 + (b-r)/delta`) | [x] |
| C54 | `f13` | `b == max` branch (`h = 4 + (r-g)/delta`) | [x] |
| C55 | `f13` | ties: `r == g > b`, `g == b > r`, `r == b > g`, `r == g == b` (delta 0) — branch order matters | [x] |
| C56 | `f13` | negative channels (so `max` can be `0` or negative), subnormal `delta`, `FLT_MAX` spread | [x] |
| C57 | `f13` | fully random 3-float bit patterns (~20k) | [x] |
| C58 | `f11`/`f12`/`f13` | `dest == src` aliasing, and `dest`/`src` at odd alignment inside a larger buffer | [x] |
| C59 | `agglom` (top-level) | all 33 parameters fully random (independent bit patterns) — ~20k iterations, byte-compared as `f64` bits | [x] |
| C60 | `agglom` | all-zero parameters; all-`0xFF` bit patterns; per-parameter one-hot boundary sweeps (each of the 33 params set to a boundary value while the rest are a fixed benign baseline) | [x] |
| C61 | `agglom` | parameters steered so each sub-function takes a *specific* branch simultaneously (`f3_2 = 0`, `channels = 2`, degenerate `f9`, `s = 0` for `f11`/`f12`, `delta = 0` for `f13`) — checks the composed pipeline, not just the wrappers | [x] |
| C62 | `agglom` | inputs chosen to make individual sub-results NaN so the 13 `isnan()` filters fire in every combination | [x] |
| C63 | all 20 entry points | run under `cargo test --release` (optimised Rust `.so`, `panic = "abort"`) **and** `cargo test` (dev profile) | [x] |
| C64 | all 20 entry points | run under `cargo test --no-default-features` (no `[features]` exist, so this is the same code path — recorded for completeness of the feature cross-product) | [x] |

## Results

Every row above passes. Row → test mapping (all tests load BOTH `.so`s via
`libloading` and compare bit-for-bit; the Rust crate is never linked directly):

| rows | file | tests |
|------|------|-------|
| C1 – C14 | `tests/phase_b_geom.rs` | 13 |
| C15 – C29 | `tests/phase_b_int.rs` | 15 |
| C30 – C34 | `tests/phase_b_float.rs` | 5 |
| C35 – C58 | `tests/phase_b_color.rs` | 24 |
| C59 – C62 | `tests/phase_b_agglom.rs` | 4 |
| C63 – C64 | `run_tests.sh` (profile × feature loop) | 4 combinations |

Mechanical cross-check that no row is missing a test:

```
$ comm -23 <(grep -oE '^\| C[0-9]+' CONFIGS.md | tr -d '| ' | sort -V) \
           <(grep -hoE 'fn c([0-9]+)(_c([0-9]+))?_' tests/phase_b_*.rs \
             | grep -oE 'c[0-9]+' | tr -d c | sort -nu | sed 's/^/C/' | sort -V)
C63
C64        <- the two build-configuration rows, covered by run_tests.sh
```

Beyond the per-row tests, `tests/fuzz_deep.rs` blankets all 20 entry points with
uniformly random bit patterns. Soaked at **40,000,000 iterations per function**
(`FUZZ_ITERS=40000000 cargo test --release --test fuzz_deep`) with zero
divergences.

Full run: **108 tests × 4 build configurations = 432 passing test executions**,
`run_tests.sh` exit status 0.
