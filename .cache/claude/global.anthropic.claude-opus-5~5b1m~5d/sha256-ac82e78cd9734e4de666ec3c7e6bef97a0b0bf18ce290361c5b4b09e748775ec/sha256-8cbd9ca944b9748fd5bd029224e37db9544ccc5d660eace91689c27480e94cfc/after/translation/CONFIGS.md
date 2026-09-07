# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/include/lib.h` + `c_src/src/lib.c`.

## Axes the C code actually branches on

**Public entry points** (the header declares exactly one; there is no
convenience wrapper vs. low-level split):

* `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z)` — the only
  exported symbol.
* `static int16_t mp3d_scale_pcm(float)` — internal, reachable only through
  `synth_pair`; exercised via crafted `z` vectors that drive the accumulator to
  each of its branches (that is the low-level path, driven end to end).

**Runtime options / modes:** none (no flags, no global state, no `#ifdef` in
`lib.c`, no init/teardown). The library is a pure stateless function.

**Axis 1 — `nch` (int).** The only integer parameter. It selects the store index
`16 * nch` (line 33). Distinguished values:
`0` (aliases `pcm[0]`, second store wins), `1` (mono stride), `2` (stereo
stride — the real mp3 use), `>2`, negative (index below `pcm`).

**Axis 2 — offset set read from `z`.** Two disjoint strided reads:
* first accumulator: `z[k*64]` for `k` in `0..=14` (15 taps, coefficients
  `29, 213, 459, 2037, 5153, 6574, 37489, 75038` with the paired
  add/subtract structure of lines 15–22);
* second accumulator, after `z += 2`: `z[2 + k*64]` for `k` in
  `{0,2,4,6,8,10,12,14}` (8 taps, coefficients `-5, 146, -45, -9975, 64019,
  9727, 1567, 104`).
  Highest index touched: `2 + 14*64 = 898`, so `z` must span ≥ 899 floats.

**Axis 3 — value shape of `z`.** The `f32` accumulation order and the three
branches of `mp3d_scale_pcm` are value-dependent: small values (round path),
large values (clamp high / clamp low), mixed signs (cancellation, which is
where accumulation *order* becomes observable), exact `±0.0`, subnormals,
`±inf`, `NaN`, and values that make the accumulator land exactly on the
`32766.5` / `-32767.5` boundaries.

**Axis 4 — buffer/aliasing shape of `pcm`.** `pcm[0]` and `pcm[16*nch]` are
either distinct (`nch != 0`) or the same cell (`nch == 0`); with negative `nch`
the second store is at a negative offset.

Each row below is a combination the C treats differently. Every row is run with
**many randomized inputs** (seeded xorshift64\*, fixed seed, ≥ the count in the
row) comparing the full `pcm` buffer byte-for-byte between the C and Rust
`.so`s.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `synth_pair` | `nch=1`, `z` = uniform random in `[-1, 1)` (round path, tiny accumulator), 4096 iters | [x] |
| 2 | `synth_pair` | `nch=2`, `z` = uniform random in `[-1, 1)` (stereo stride), 4096 iters | [x] |
| 3 | `synth_pair` | `nch=1`, `z` = uniform random in `[-0.5, 0.5)` scaled so the accumulator straddles the whole `int16` range (mixed clamp / round), 4096 iters | [x] |
| 4 | `synth_pair` | `nch=2`, `z` = random with magnitude `~1e-2` (accumulator in `(-3, 3)`: exercises `s -= (s<0)`, `s==0`, and both signs densely), 4096 iters | [x] |
| 5 | `synth_pair` | `nch=2`, `z` = large random magnitude `~1e3` → both accumulators saturate high/low most iterations, 2048 iters | [x] |
| 6 | `synth_pair` | `nch=2`, `z` = fully random 32-bit patterns (any `f32`: normals, subnormals, `±0`, `±inf`, `NaN` all appear), 8192 iters | [x] |
| 7 | `synth_pair` | `nch=2`, `z` = all zeros / all `-0.0` / all `+1.0` / all `-1.0` (hand-picked degenerate uniform buffers) | [x] |
| 8 | `synth_pair` | `nch=2`, exactly one tap set to a special value (`+inf`, `-inf`, `NaN`, `f32::MAX`, `f32::MIN_POSITIVE`, subnormal, `-0.0`) and the rest random — swept over **all 23 tap offsets** × 7 specials | [x] |
| 9 | `synth_pair` | `nch=2`, `z` crafted so accumulator 1 hits the exact boundary `32766.5f` / `-32767.5f` and 1 ULP either side (single-tap solve via coefficient `75038`) | [x] |
| 10 | `synth_pair` | `nch=0` — **aliasing row**: `pcm[16*0]` == `pcm[0]`, second store overwrites first; random `z`, 2048 iters | [x] |
| 11 | `synth_pair` | `nch<0` (`-1`, `-2`, `-8`) — negative store offset into a buffer with head-room; random `z`, 1024 iters per `nch` | [x] |
| 12 | `synth_pair` | `nch` large (`3, 4, 7, 16, 64, 512`) — wide stride, buffer sized `16*nch+1`; random `z` | [x] |
| 13 | `synth_pair` | `pcm` pre-filled with a non-zero canary pattern, `nch=2` — verifies **only** indices `0` and `32` are written and every other cell is untouched, identically in C and Rust | [x] |
| 14 | `synth_pair` | `z` buffer of the exact minimum length (899 floats) at the end of an allocation, `nch=1` — confirms neither side reads past index 898 | [x] |
| 15 | `synth_pair` | `z` unaligned-by-element view (pointer offset by 1 float inside a larger buffer), `nch=2`, random — confirms both use plain element-indexed reads, 1024 iters | [x] |
| 16 | `synth_pair` | repeated calls with the same `pcm` and advancing `z` (real decoder loop shape: 32 sub-band iterations over one 899+ float window), `nch=2` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds `add_library(... SHARED ...)` only — there is **no**
driver executable, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. The "compare binary stdout" gate is
therefore **not applicable**.

## Feature combinations

No `[features]` in `Cargo.toml` → the default and `--no-default-features` builds
are byte-identical code paths. Both are run.

---

## Test-adequacy evidence (mutation testing)

Passing tests only prove something if they can fail. `.scratch/mutate.sh`
injects 20 deliberate faults into `src/lib.rs`, rebuilds, and re-runs the suite.
Result: **14 caught, 6 survivors — all 6 proven non-faults.**

| mutant | outcome |
|--------|---------|
| drop `s -= (s < 0)` | CAUGHT (15 tests) |
| coefficient `213 -> 214` | CAUGHT (13) |
| coefficient `75038 -> 75037` | CAUGHT (13) |
| acc2 coefficient `-5 -> 5` | CAUGHT (13) |
| acc2 coefficient `-9975 -> 9975` | CAUGHT (15) |
| `z += 2` becomes `z += 1` | CAUGHT (16) |
| tap stride `14*64 -> 14*63` | CAUGHT (13) |
| reassociate the `459` term | CAUGHT (3 — f32 order matters) |
| rounding `+0.5f -> +0.4f` | CAUGHT (15) |
| clamp-high `32767 -> 32766` | CAUGHT (16) |
| clamp-low `-32768 -> -32767` | CAUGHT (16) |
| `nch` offset made unsigned | CAUGHT (SIGSEGV on negative `nch`) |
| store index `16*nch -> 32*nch` | CAUGHT (3 + SIGABRT) |
| `>=`/`<=` clamp bound flips | *survivor* — **provably equivalent**, see below |
| clamp constant `32766.5 -> 32767.5` | *survivor* — **provably equivalent** |
| `as i32 as i16` -> saturating `as i16` | *survivor* — **provably equivalent** |
| `let mut a: f32` -> `f64` | *survivor* — **does not compile** (rustc rejects it) |
| two textual no-op controls | *survivor* — expected; they change nothing |

`tests/mutant_equivalence.rs` proves the three real survivors by **exhaustive
per-ULP enumeration** of every `f32` in the affected windows:

* `>=` -> `>`: differs only at exactly `a == 32766.5`, where the rounding path
  computes `(int16_t)(32766.5 + .5f) == 32767` — the same value the clamp returns.
* `<=` -> `<`: at exactly `a == -32767.5` the rounding path gives `-32767`, then
  `s -= (s < 0)` yields `-32768` — again identical to the clamp.
* clamp constant raised: for `a` in `[32766.5, 32767.5)`, `a + .5f` lands in
  `[32767.0, 32768.0)` and is exactly representable (ULP = 1/512), so truncation
  gives `32767` regardless of which branch is taken.
* saturating cast: the two guards bound `sample + .5f` to `(-32767.0, 32767.0)`,
  so saturation can never engage; `NaN` maps to `0` under both spellings.

No input can distinguish these from the original, so their survival is correct
rather than a coverage gap.

## Harness correctness note

`cargo test` recompiles the crate but does **not** refresh the `cdylib` artifact
in `target/<profile>/`. An early run of this suite was loading a **stale** `.so`
and every one of the 20 mutants "survived" — the tests were vacuous. Both
`run_tests.sh` (builds before testing) and a staleness assertion in
`tests/common/mod.rs` (refuses to run if the `.so` predates `src/lib.rs`) now
prevent that failure mode. Always invoke the suite via `./run_tests.sh` or
`./verify_all.sh`.
