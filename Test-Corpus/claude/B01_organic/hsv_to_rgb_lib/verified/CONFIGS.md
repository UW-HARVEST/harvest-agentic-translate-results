# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## How the axes were derived (mechanical, from the C source)

Public API, from `c_src/include/lib.h` — the *complete* entry-point list, and it
is also the lowest level available (there is no convenience wrapper layered over
anything else):

```c
void hsv_to_rgb(float *dest, const float *src);
```

There is no init/context/options object, no setter, no mode flag, and no
`#ifdef` in `c_src/`, so there are **no runtime option toggles**. The axes are
therefore exactly the branches `c_src/src/lib.c` takes on its *data*:

| axis | source of truth in `lib.c` | values the code distinguishes |
|------|----------------------------|-------------------------------|
| **A** — saturation branch | `if (s == 0)` (line 12) | `s == +0.0` / `s == -0.0` (both true) vs. `s != 0` (incl. `NaN`, which compares false) |
| **B** — `switch` arm | `i = (int)floorf(h/60.0f)`; `switch (i)` cases `0,1,2,3,4,default` (lines 19–55). GCC emits `cmpl $0x4 / ja`, i.e. an **unsigned** bound, so every negative `i` also lands in `default` | `i=0`, `1`, `2`, `3`, `4`, `i>=5`, `i<0`, `i==INT_MIN` (the `cvttss2si` integer-indefinite result for `NaN`/`inf`/out-of-range) |
| **C** — float class of `h`, `s`, `v` | the arithmetic on lines 18–23 (`/`, `-`, `*`) plus `floorf` | normal, `±0.0`, subnormal, `±inf`, quiet `NaN`, signaling `NaN`, exact sector boundaries (`0/60/120/180/240/300/360`), values just below a boundary (`nextafter`), out-of-`int`-range magnitudes |
| **D** — pointer relationship | neither parameter is `restrict`; `src[0..2]` are read into locals *before* any store (lines 8–10 vs 13–15/56–58) | disjoint buffers, `dest == src`, `dest == src+1`, `dest == src-1` |
| **E** — write extent | only `dest[0]`, `dest[1]`, `dest[2]` are ever stored | guard/canary words immediately around the 3-float window must stay untouched |

Every row below is a combination the C actually treats differently (cross
product of A×B×C×D, pruned to distinguishable cases). Each row is exercised with
**many randomized inputs** (deterministic `SplitMix64`, fixed seed `0x5EED_1234_ABCD_9876`)
and compared bit-for-bit — both the 3 output floats *and* the surrounding canary
words of *both* buffers — between the C `.so` and the Rust `.so`, both loaded via
`libloading`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `hsv_to_rgb` | A: `s == +0.0` (early return). `v` random over all bit patterns, `h` random over all bit patterns (must be ignored). Disjoint buffers. | [x] |
| 2 | `hsv_to_rgb` | A: `s == -0.0` (early return also taken). `v`/`h` random full-range bit patterns. Disjoint. | [x] |
| 3 | `hsv_to_rgb` | A: `s == ±0.0`, `v` restricted to the special-value set (`±0.0`, subnormals, `±inf`, qNaN/sNaN payloads, `FLT_MIN`, `FLT_MAX`). Disjoint. | [x] |
| 4 | `hsv_to_rgb` | B: `i == 0` — `h` uniform in `[0, 60)`, `s` uniform in `(0, 1]`, `v` uniform in `[0, 1]`. Disjoint. | [x] |
| 5 | `hsv_to_rgb` | B: `i == 1` — `h` uniform in `[60, 120)`, `s`,`v` in unit range. Disjoint. | [x] |
| 6 | `hsv_to_rgb` | B: `i == 2` — `h` uniform in `[120, 180)`, `s`,`v` in unit range. Disjoint. | [x] |
| 7 | `hsv_to_rgb` | B: `i == 3` — `h` uniform in `[180, 240)`, `s`,`v` in unit range. Disjoint. | [x] |
| 8 | `hsv_to_rgb` | B: `i == 4` — `h` uniform in `[240, 300)`, `s`,`v` in unit range. Disjoint. | [x] |
| 9 | `hsv_to_rgb` | B: `i >= 5` (`default` arm) — `h` uniform in `[300, 360)`, `s`,`v` in unit range. Disjoint. | [x] |
| 10 | `hsv_to_rgb` | B: `i >= 6` (`default` arm, wrap-around hues) — `h` uniform in `[360, 100000)`, `s`,`v` in unit range. Disjoint. | [x] |
| 11 | `hsv_to_rgb` | B: `i < 0` (`default` arm via unsigned `ja`) — `h` uniform in `(-100000, 0)`, `s`,`v` in unit range. Disjoint. | [x] |
| 12 | `hsv_to_rgb` | C: exact sector boundaries `h ∈ {-0.0, 0, 60, 120, 180, 240, 300, 360, 420, -60, -120}` × `nextafter(h, ±inf)` on each side, `s`,`v` random unit range. Catches off-by-one-`i` at the `floorf` seams. | [x] |
| 13 | `hsv_to_rgb` | C+B: `h` such that `h/60` lands exactly on an integer after rounding (`h = 60*k` for `k ∈ [-40, 40]`), `s`,`v` random. | [x] |
| 14 | `hsv_to_rgb` | C: `h/60` out of `int` range → `i == INT_MIN` → `default`. `h ∈ {±1.3e11, ±3.4e38 (FLT_MAX), 60*2^31, -(60*2^31), 2^31·60±ulp}`, `s`,`v` random unit range. | [x] |
| 15 | `hsv_to_rgb` | C: `h == ±inf` (→ `i == INT_MIN`, `default`), `s`,`v` random unit range. | [x] |
| 16 | `hsv_to_rgb` | C: `h == NaN` (quiet and signaling, 8 distinct payloads incl. sign-bit set) → `i == INT_MIN`, `default`. `s`,`v` random unit range. | [x] |
| 17 | `hsv_to_rgb` | C: `s` outside the unit range — `s` uniform in `(1, 16]` and in `[-16, 0)` (so `p = v*(1-s)` goes negative / large), `h` random in `[0,360)` so all 6 arms are hit, `v` random unit range. | [x] |
| 18 | `hsv_to_rgb` | C: `v` outside the unit range — `v` uniform in `[-1e6, 1e6]`, `s` random `(0,1]`, `h` random `[0,360)`. | [x] |
| 19 | `hsv_to_rgb` | C: `s == ±inf` (invalid ops `inf*0`, `inf-inf` → x86 indefinite `0xffc00000`), `h` random `[0,360)`, `v` random. | [x] |
| 20 | `hsv_to_rgb` | C: `v == ±inf`, `s` random `(0,1]`, `h` random `[0,360)`. | [x] |
| 21 | `hsv_to_rgb` | C: `s == NaN` (several payloads, quiet+signaling), `h` random `[0,360)` (all 6 arms), `v` random unit. Exercises NaN payload propagation through `1-s`, `s*f`, `s*(1-f)`. | [x] |
| 22 | `hsv_to_rgb` | C: `v == NaN` (several payloads, quiet+signaling), `s` random `(0,1]`, `h` random `[0,360)`. | [x] |
| 23 | `hsv_to_rgb` | C: subnormal / tiny magnitudes — `h`, `s`, `v` each drawn from the subnormal + `FLT_MIN` neighbourhood (`s` forced nonzero), all arms. | [x] |
| 24 | `hsv_to_rgb` | C: totally unconstrained fuzz — `h`, `s`, `v` are independent **uniform random 32-bit patterns** (covers every float class, all arms, all sign combinations). Disjoint buffers. High iteration count. | [x] |
| 25 | `hsv_to_rgb` | D: `dest == src` (in-place), main path (`s != 0`), `h` random `[0,360)` so all 6 arms. | [x] |
| 26 | `hsv_to_rgb` | D: `dest == src` (in-place), early-return path (`s == ±0.0`). | [x] |
| 27 | `hsv_to_rgb` | D: `dest == src + 1` (overlapping, dest shifted up), random unconstrained bit patterns. | [x] |
| 28 | `hsv_to_rgb` | D: `dest == src - 1` (overlapping, dest shifted down), random unconstrained bit patterns. | [x] |
| 29 | `hsv_to_rgb` | D: `dest == src` with unconstrained random bit patterns (aliasing × every float class). | [x] |
| 30 | `hsv_to_rgb` | E: write-extent / no-out-of-bounds — canary words at `src[-1]`, `src[3]`, `dest[-1]`, `dest[3]` (and the rest of both 16-word buffers) verified unmodified, for both the early-return and main paths, all 4 aliasing modes. Asserted on **every** row above, plus a dedicated row. | [x] |
| 31 | `hsv_to_rgb` | Cross-product sweep: {all 4 aliasing modes} × {`s==+0`, `s==-0`, `s` unit, `s` NaN, `s` inf} × {`i` = 0,1,2,3,4,≥5,<0,INT_MIN}, randomized `v`. Full pruned A×B×D matrix in one driver. | [x] |
| 32 | `hsv_to_rgb` | Idempotence/repeat-call stability: same input applied twice through the same `.so` handle gives identical bits (no hidden global state), for both libraries. | [x] |
| 33 | `hsv_to_rgb` | B+C: large-but-**in**-`int`-range `i` — `h` log-uniform over biased exponents 132..163 (`|h/60|` in `[1, 2^31)`, both signs), so `CVTSI2SS` of `i` must *round*; plus every ULP within 2048 of `|h/60| == 2^31`. | [x] |
| 34 | `hsv_to_rgb` | Near-exhaustive hue sweep: all 65536 high-16-bit halves × 6 low halves × 6 `(s,v)` presets (normal / early-return / NaN / `inf` / negative). | [x] |
| 35 | `hsv_to_rgb` | Near-exhaustive saturation sweep: all 65536 high halves × 3 low halves × 8 hues covering arms 0,1,3, default(≥5), default(<0), `INT_MIN`(inf), `INT_MIN`(NaN). | [x] |
| 36 | `hsv_to_rgb` | Near-exhaustive value sweep: all 65536 high halves × 3 low halves × 6 `(h,s)` presets, incl. the early-return path. | [x] |
| 37 | `hsv_to_rgb` | 1,000,000 unconstrained random triples rotated across all 4 aliasing modes (independent seed `0xDEADBEEF12345678`). | [x] |
| 38 | `hsv_to_rgb` | **Exhaustive**: for each argument slot (`h`, `s`, `v`) × each of 5 fixed-operand presets, ALL 2^32 bit patterns of that slot. 15 configurations × 4,294,967,296 inputs ≈ 6.4×10^10 triples, all byte-identical. Sharded via `run_exhaustive.sh`; `#[ignore]`d in the default suite because it takes ~10 CPU-hours. | [x] |

## Exhaustive sweep record (row 38)

Run with `./run_exhaustive.sh`; 16-way sharded on this host.

| slot swept | preset (other two slots) | inputs | result |
|------------|--------------------------|--------|--------|
| `h` (hue)        | 0 `s=0.5, v=1.0`                  | 4,294,967,296 | byte-identical |
| `h`              | 1 `s=1.0, v=-0.25`                | 4,294,967,296 | byte-identical |
| `h`              | 2 `s=+0.0` (early return), `v=FLT_MIN+1ulp` | 4,294,967,296 | byte-identical |
| `h`              | 3 `s=qNaN(0x7FC01234), v=-sNaN`   | 4,294,967,296 | byte-identical |
| `h`              | 4 `s=+inf, v=-0.0` (invalid op)   | 4,294,967,296 | byte-identical |
| `s` (saturation) | 0 / 1 / 2 / 3 / 4                 | 5 × 4,294,967,296 | byte-identical |
| `v` (value)      | 0 / 1 / 2 / 3 / 4                 | 5 × 4,294,967,296 | byte-identical |

Total: **15 × 4,294,967,296 ≈ 6.44 × 10^10** input triples compared through the
FFI boundary with zero divergences (output triple *and* the canary words around
both buffers).

## Divergences found and fixed by Phase B

The randomized rows above initially failed on 8 of 32 rows. Root cause: **NaN
payload propagation order** in the SSE scalar arithmetic. LLVM regards all NaNs
as interchangeable and freely commutes `fmul`/`fsub`, so the Rust translation
returned the *other* operand's payload than GCC's emitted instruction order did
(e.g. for `src = {inf, 0x7FC00000, 0xFFC00000}` C produced `0x7FC00000` for the
green channel and Rust produced `0xFFC00000`). Fixed in `src/lib.rs` by adding
`subss` / `mulss` / `divss` helpers that reproduce, operand-for-operand, the
instruction sequence GCC emits for `c_src/src/lib.c` — plus explicit handling of
`floorf(NaN)` and of the x86 QNaN-indefinite (`0xFFC0_0000`) produced by an
invalid operation on non-NaN operands. See `ERRORS.md` for the full list.

## Test-suite self-validation (mutation testing)

To prove the suite can actually *detect* divergence (and is not passing
vacuously), 16 mutations were injected into `src/lib.rs`, rebuilt, and the full
suite re-run in both profiles. Result: **every behaviourally-distinct mutation
was caught**; the five that were not are provably semantics-preserving for this
call graph.

| # | mutation | detected? | note |
|---|----------|-----------|------|
| M1 | saturating `as i32` instead of `CVTTSS2SI` semantics | yes | debug + release |
| M3 | quiet `v` on the `s == 0` copy path | yes | 12 tests failed |
| M4 | extra `switch` arm for `i == -1` (signed bound) | yes | 11 tests failed |
| M5b | plain `v * (1.0 - s)` for `p` (the original bug) | yes | debug only — LLVM's chosen operand order differs per profile, which is exactly why the explicit `mulss` helper is needed |
| M6 | `*ptr` deref instead of `ptr::read` | yes | debug UB assertion aborts instead of `SIGSEGV` |
| M8b | `0x7FC00000` instead of `0xFFC00000` for the `MULSS` indefinite | yes | debug + release |
| M9 | `s.to_bits() == 0` (misses `-0.0`) | yes | debug + release |
| M10 | reversed NaN precedence in `mulss` | yes | debug + release |
| M13 | arm 3 channels swapped | yes | debug + release |
| M14 | divisor `60.0` -> `60.0001` | yes | debug + release |
| M15 | `dest[2]` never written | yes | canary/`dest3` checks |
| M2 | swapped `MULSS` operands in `t` | no | equivalent: `t` is only *used* when `i ∈ [0,4]`, which forces `h` finite, hence `1-f` is never NaN, so precedence cannot matter |
| M5 | `subss(1.0,s) * v` (correct order, plain `*`) | no | equivalent under this codegen; M5b is the order-sensitive version |
| M7 | `f32::floor` without explicit NaN quieting | no | equivalent: Rust's `f32::floor` already quiets sNaN and preserves the payload (`floor(0x7f800001) = 0x7fc00001`), identical to glibc `floorf` |
| M11 | reversed NaN precedence in `subss` | no | equivalent: every `subss` call site has a non-NaN operand (`1.0`, or `(float)i`), so at most one operand is ever a NaN |
| M12 | do not quiet the surviving sNaN on the `SRC1` path | no | equivalent: the only `SRC1` sNaN is `h` in `divss(h, 60)`, and every downstream consumer re-quietens it (`floorf`, or the `SRC2` path) before it can reach `dest` |
