# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from `c_src/src/lib.c`, `c_src/include/lib.h` and the
reference assembly (`gcc -S -O0`, which is what `c_src/CMakeLists.txt` builds).

## Axes the C code actually branches on

`tfm` is the whole public API, and it is also the lowest-level entry point —
there is no convenience wrapper layered on top of anything, so every row below
drives `tfm` directly through the `.so` export.

### A. Runtime options / flags

The header exposes **no** option, mode, flag, enum, context struct or global.
The only non-pointer parameter is `int count`. `grep -c "#if\|#ifdef\|option(\|target_compile_definitions" c_src/src/lib.c c_src/CMakeLists.txt` → 0.
So the "options" axis degenerates to the value of `count`:

* `count < 0` (loop never entered)
* `count == 0` (loop never entered)
* `count == 1` (single iteration; no pointer advance observed)
* `count == 2` (first iteration's `src += 3` / `dest += 2` observed)
* `count` large (many iterations; strides 3 and 2 fully exercised)

### B. Control-flow branches in the source

| source | branch | condition |
|--------|--------|-----------|
| `lib.c:7` | loop guard | `i < count` |
| `lib.c:8` | arm select | `src[0] < src[1]` — **if** arm (`dx2=src[0]`, `dy2=src[1]`, writes `dest[0]=dx2-lambda`, `dest[1]=dxy`) vs **else** arm (`dy2=src[0]`, `dx2=src[1]`, writes `dest[0]=dxy`, `dest[1]=dx2-lambda`) |
| `lib.c:15`,`25` | clamp | `(0 > sqd) ? 0 : sqd` — clamped vs pass-through |

Note that the two arms are **not** symmetric: they swap which input feeds
`dx2`/`dy2` *and* swap which output slot receives `dxy`, so both must be
exercised independently with the full set of input shapes.

### C. Input shapes the arithmetic special-cases

The five FP operations reachable per element (`mulss`, `addss`, `subss`,
`comiss`, `sqrtf`) distinguish these `f32` classes, so the element generator
draws from all of them:

`+0.0`, `-0.0`, positive/negative subnormal, positive/negative normal (small,
mid, large), values whose square overflows (`|x| > 2^64`), `+inf`, `-inf`,
quiet NaN (both signs, several payloads), signaling NaN (both signs), and
fully random 32-bit patterns.

### D. Memory shapes

* `dest` and `src` disjoint (the normal case)
* `dest == src` (exact aliasing — legal, the C has no `restrict`)
* `dest` inside `src`'s range at a positive offset (partial overlap; `dest`
  stride 2 < `src` stride 3, so `dest` trails `src` and overlap is benign)
* `dest` *ahead* of `src` (destructive overlap — `dest` writes clobber
  not-yet-read `src` elements)
* byte-unaligned `dest` / `src` (`movss` permits it; the Rust uses
  `read_unaligned`/`write_unaligned`, so this must agree)

## Configuration-surface table

One row per combination the C treats differently. Every row is driven with
**many randomized inputs** (fixed seed `0x2b7e151628aed2a6`, ≥256 samples per
row unless stated) through both `.so`s, compared **bit-for-bit** on the raw
`u32` of every output element.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `tfm` | `count=1`, **if** arm forced (`src[0] < src[1]`), operands random finite normals | [x] |
| 2 | `tfm` | `count=1`, **else** arm forced via `src[0] > src[1]`, random finite normals | [x] |
| 3 | `tfm` | `count=1`, **else** arm via `src[0] == src[1]` (equal), random finite normals | [x] |
| 4 | `tfm` | `count=1`, arm chosen by the data (no forcing), operands drawn from the full special-value pool of §C | [x] |
| 5 | `tfm` | `count=1`, clamp **taken** (`sqd < 0`): `dxy` small / `dx2`,`dy2` chosen so `(dy2-dx2)^2 + 4dxy^2` underflows negative — searched randomly, both arms | [x] |
| 6 | `tfm` | `count=1`, clamp **not taken** (`sqd > 0`), both arms, random normals | [x] |
| 7 | `tfm` | `count=1`, `sqd == +0.0` exactly (`dx2 == dy2`, `dxy == 0.0`), both arms | [x] |
| 8 | `tfm` | `count=1`, `sqd == -0.0` (clamp *not* applied, `sqrtf(-0.0) == -0.0`) — **proven unreachable**: `(4.0f*dxy)*dxy` is never negatively signed, so the test asserts 0 hits over 2M+ randomized inputs plus the exhaustive specials sweep while differentially checking that whole space | [x] |
| 9 | `tfm` | `count=1`, `sqd` is NaN (clamp not applied ⇒ `sqrtf(NaN)`), both arms | [x] |
| 10 | `tfm` | `count=1`, `sqd == +inf` via a squaring overflow (`\|dxy\| > 2^64`), both arms | [x] |
| 11 | `tfm` | `count=1`, `sqd == inf - inf` (both `dy2^2` and `2·dx2·dy2` overflow) ⇒ indefinite QNaN, both arms | [x] |
| 12 | `tfm` | `count=1`, `0 * inf` inside `2.0f*dx2*dy2` (`dx2 == ±0`, `dy2 == ±inf`) — reachable, both arms; and inside `4.0f*dxy*dxy` — **proven unreachable** (`\|4*dxy\| >= \|dxy\|`), asserted with 0 hits | [x] |
| 13 | `tfm` | `count=1`, inputs are ±0.0 in every combination (8 patterns) × both arms | [x] |
| 14 | `tfm` | `count=1`, inputs are ±inf in every combination (27 patterns of {−inf,0,+inf}) | [x] |
| 15 | `tfm` | `count=1`, quiet NaN in each of the 3 slots, random payloads and signs (payload propagation / destination-operand rule) | [x] |
| 16 | `tfm` | `count=1`, **signaling** NaN in each of the 3 slots (quiet-bit clear), both signs (quieting rule) | [x] |
| 17 | `tfm` | `count=1`, two/three simultaneous NaNs (which payload wins per SSE destination rule) | [x] |
| 18 | `tfm` | `count=1`, subnormal operands (positive and negative, incl. `0x00000001`) | [x] |
| 19 | `tfm` | `count=1`, fully random 32-bit patterns in all three slots (4096 samples) | [x] |
| 20 | `tfm` | `count=2`, random full-pool elements — verifies the `src+=3` / `dest+=2` stride and that iteration 2 is independent | [x] |
| 21 | `tfm` | `count=3..8`, mixed arms across iterations (guarantees both arms inside one call), full-pool elements | [x] |
| 22 | `tfm` | `count=1024`, full-pool elements — many iterations, both arms, canary bytes checked past the end of `dest` | [x] |
| 23 | `tfm` | `count=1000`, all-identical element triples (every iteration takes the same arm) — both arms, once each | [x] |
| 24 | `tfm` | `count=5`, **byte-unaligned** `dest` and `src` (offsets 1,2,3 bytes), full-pool elements | [x] |
| 25 | `tfm` | `count=8`, `dest == src` (exact aliasing), full-pool elements | [x] |
| 26 | `tfm` | `count=8`, `dest` trailing inside `src`'s buffer (benign overlap), full-pool elements | [x] |
| 27 | `tfm` | `count=8`, `dest` *ahead* of `src` (destructive overlap: writes clobber unread inputs), full-pool elements | [x] |
| 28 | `tfm` | `count=1`, `src[0]`/`src[1]` adjacent-value pairs (`x` vs `nextafter(x)`, ±) — exercises the strict `<` at its boundary | [x] |
| 29 | `tfm` | `count=1`, `src[0] = -0.0`, `src[1] = +0.0` and vice-versa (signed-zero compare boundary) | [x] |
| 30 | `tfm` | `count=1`, values at the extremes of the normal range (`FLT_MIN`, `FLT_MAX`, `±FLT_MAX`) in all three slots | [x] |
| 31 | `tfm` | `count=1`, `dxy` in the overflow band where `4.0f*dxy` and/or `dxy*dxy` overflow, pinning the left-to-right order of `(4*dxy)*dxy` (for `f32` the two groupings agree in *value*, so the order is observable only through NaN sign/payload — which this row's exhaustive `SPECIALS x SPECIALS x dxy` sweep covers) | [x] |
| 32 | `tfm` | `count=1`, `dx2 == dy2` exactly (so `sqd` reduces to `4dxy^2`), random magnitudes, both arms | [x] |
| 33 | `tfm` | `count=64`, elements chosen so `dest` is written with NaN on some iterations and finite on others (mixed NaN/finite stream) | [x] |
| 34 | `tfm` | `count=1`, exhaustive sweep over the 3-slot cross-product of a 24-value special table (13824 cases, deterministic, not randomized) | [x] |

All 34 rows pass under every feature combination from `SYMBOLS.md`
(the empty set, i.e. `--no-default-features` and the equivalent default build),
in both the `dev` and the `release` profile (`release` is the shipping profile:
optimized, `panic = "abort"`). Run `./verify.sh` to reproduce.

Rows whose condition is unreachable (8, and the `4*dxy*dxy` half of 12) use
`assert_unreachable()` rather than a vacuous pass; `tests/reachability.rs`
maintains the full reachable/unreachable map and fails if it goes stale.

Beyond the table, `tests/fuzz_explore.rs` adds ~42 000 unstructured random
triples and 2 000 multi-element calls, and `tests/nan_masking.rs` asserts the
structural facts about the C that make four operand-order variants
unobservable. `./mutation_check.sh` confirms the suite is *sensitive*: it injects
29 plausible mis-translations and requires the 21 behavioural ones to fail the
suite and the 8 provably-equivalent ones to pass.
