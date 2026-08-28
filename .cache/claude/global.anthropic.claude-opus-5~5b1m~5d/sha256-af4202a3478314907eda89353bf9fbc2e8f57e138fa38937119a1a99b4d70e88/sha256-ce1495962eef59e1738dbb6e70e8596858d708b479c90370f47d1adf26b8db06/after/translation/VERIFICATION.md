# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C ground
truth). Reproduce everything with:

```
cd translation && ./run_all.sh
```

## Completion gate

| # | requirement | status | evidence |
|---|-------------|--------|----------|
| 1 | `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | symbol diff `C \ Rust` empty (`{tfm}` == `{tfm}`); `tests/symbols.rs` (6 tests) |
| 2 | Phase B: every row of `CONFIGS.md` passes across randomized inputs | **PASS** | 44/44 rows, 1:1 with `tests/phase_b.rs` (44 tests) |
| 3 | Phase C: every row of `ERRORS.md` has a passing error-path differential test | **PASS** | 26/26 rows, 1:1 with `tests/phase_c.rs` (26 row tests + 2 helpers) |
| 4 | All of the above hold under **every** feature combination | **PASS** | `Cargo.toml` declares no `[features]`; both `DEFAULT` and `NO_DEFAULT` × both `debug` and `release` `.so` = 24 test steps, all green |

`./run_all.sh` final line: **`ALL CONFIGURATIONS PASS`** (25 steps, 0 failures).

## Row-to-test mapping is mechanically checked

```
$ diff <(grep -oE '^\| B[0-9]+ ' CONFIGS.md | grep -oE '[0-9]+' | awk '{printf "b%02d\n",$0}' | sort) \
       <(grep -oE '^fn b[0-9]+' tests/phase_b.rs | grep -oE 'b[0-9]+' | sort)
$ diff <(grep -oE '^\| E[0-9]+ ' ERRORS.md   | grep -oE '[0-9]+' | awk '{printf "e%02d\n",$0}' | sort) \
       <(grep -oE '^fn e[0-9]+' tests/phase_c.rs | grep -oE 'e[0-9]+' | sort)
# both produce no output -> 1:1, no gaps, no duplicates
```

## Test inventory

| file | tests | role |
|------|-------|------|
| `tests/common/mod.rs` | (harness) | dual `dlopen` loader, bit-exact comparators, guard canaries, SplitMix64 PRNG, IEEE-class generators, `sqd`-regime search, branch observer |
| `tests/symbols.rs` | 6 | Phase A + D symbol parity, `dlsym`-ability, **staleness guard** |
| `tests/phase_b.rs` | 44 | one per `CONFIGS.md` row |
| `tests/phase_c.rs` | 28 | 26 `ERRORS.md` rows + fork child + `sqrt`-argument lemma |
| `tests/recon.rs` | 3 | broad backstop fuzz reporting *all* divergences at once |
| `tests/olevels.rs` | 3 | robustness vs C built at other `-O` levels |
| `probe/abi_probe.c` | — | native C caller linked **directly** against each `.so`, outputs diffed |

Every call goes through `dlopen`/`dlsym` on the `.so` (or, in the probe, through
real dynamic linking). The Rust functions are never called directly, so the
`#[no_mangle] extern "C"` export wrapper is itself under test.

## The one real divergence found — and fixed

**NULL pointer with `count > 0` (`ERRORS.md` E6/E7).**

| | C | Rust `release` | Rust `dev` (before fix) |
|---|---|---|---|
| terminating signal | `SIGSEGV` (11) | `SIGSEGV` (11) | **`SIGABRT` (6)** |

The `dev` profile enabled rustc's UB sanitizer (implied by `debug-assertions`),
which turned the raw-pointer store into a checked operation and panicked with
`null pointer dereference occurred` instead of faulting.

**Fix, Rust side only** — `translation/Cargo.toml`:

```toml
[profile.dev]
debug-assertions = false   # so the dev .so faults exactly like the C
overflow-checks  = true    # nothing else is weakened
```

Why this matters: the *shipped* `release` object was always correct. Only the
`dev` object was wrong. Testing a single profile would have missed it entirely —
this is precisely what gate #4 ("every configuration") exists to catch.

A second, subtler process bug was found alongside it: **`cargo test` does not
rebuild the cdylib**, because the tests `dlopen` it rather than link it. A stale
`.so` was silently "verified" and briefly masked this divergence.
`tests/symbols.rs::shared_objects_are_not_stale` now fails loudly on that, and
`run_all.sh` always builds before testing.

## Two ERRORS.md/CONFIGS.md conditions turned out to be unreachable

Asserting an unreachable condition passes while proving nothing, so each is now
*actively proved* unreachable, with the reachable neighbours tested instead:

* **`sqd == -0.0f`** (E14 / B19). `sqd = dxy_term + acc`; `dxy_term` is a square
  so never `-0.0`, and IEEE round-to-nearest only produces `-0.0` from an
  addition when both addends are `-0.0`. **0 hits** over the exhaustive 24³
  alphabet + the cancellation family + 400 000 random triples. Covered instead by
  pushing `-0.0` through every input lane and by testing `sqd == +0.0` and
  `sqd < 0`.
* **`quiet()` in `fsqrt`'s NaN branch.** Surfaced by mutation testing (below).
  `sqrtf`'s argument can never be a *signalling* NaN, because every value feeding
  `sqd` is the output of an SSE op and those always quiet their NaN result. Proved
  by `phase_c::sqrt_argument_is_never_a_signalling_nan`: **513 824 triples,
  457 542 NaN arguments, 0 signalling.** If that lemma ever breaks the test fails
  and the `quiet()` becomes load-bearing.

Conversely, `sqd < 0` (E13/B20) *is* reachable, but only via rounding —
mathematically `sqd = (dy2-dx2)² + 4dxy² ≥ 0`. It is constructed deliberately from
near-equal operands `1 + p·2⁻²³` vs `1 + q·2⁻²³`, whose residual is
`(rn(p²/2²³) + rn(q²/2²³) − 2·rn(pq/2²³))·2⁻²³`; e.g. `p=2048, q=2049` gives
`0 + 1 − 2 = −1`, i.e. `sqd = −2⁻²³`. **400 hits, split across both C branches**,
so the clamp is genuinely exercised.

## Branch selection is observed, not assumed

`ERRORS.md` E8–E12 claim the C falls into the `else` branch for `==`, `>` and
NaN comparisons. That is now *observable*: the C writes `dxy` **verbatim** (a
plain `movss`, not an FP op) to `dest[1]` in the `if` branch and to `dest[0]` in
the `else` branch, so comparing output bits against `src[2]` reveals which branch
ran. Each test asserts the observed branch is the expected one for both objects,
and fails if the branch was indistinguishable for *every* input — so it cannot
pass vacuously.

## Mutation testing — is the suite able to fail?

Seven deliberate mutations of `src/lib.rs` (restored afterwards; `md5sum`
verified identical):

| # | mutation | caught by | verdict |
|---|----------|-----------|---------|
| M1 | commute the outer `sqd` add: `fadd(dxy_term, acc)` → `fadd(acc, dxy_term)` | recon, phase_b, phase_c, olevels | caught |
| M2 | `if s0 < s1` → `if s0 <= s1` | recon, phase_b, phase_c, olevels | caught |
| M3 | C ternary clamp → `sqd.max(0.0)` (the plausible "cleanup") | recon, phase_b, phase_c, olevels | caught |
| M4 | loop guard `i < count` → `i != count` | phase_b, phase_c | caught |
| M5 | drop `quiet(x)` from `fsqrt`'s NaN branch | *nothing* | **provably equivalent** — see the lemma above |
| M6 | swap `dest[0]`/`dest[1]` in the `else` branch | recon, phase_b, phase_c, olevels | caught |
| M7 | `2.0f*dx2` → `dx2+dx2` (**benign control**) | *nothing* | **correct to survive** — exact for all finite values, identical on overflow, identical signed-zero and NaN propagation |

M7 is a control: a suite that flagged it would be over-fitted to syntax rather
than behaviour. M5 is the only survivor that is not a control, and it is backed
by a proof plus a regression test.

## Optimization-level robustness

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the ground-truth build
passes no `-O` flag. The C library is **not self-consistent** across `-O` levels,
so it matters which build the translation tracks. Over a 126 859-triple NaN-free
corpus and a 126 965-triple NaN-bearing corpus (`tests/olevels.rs`):

| C build | NaN-free: Rust vs it | NaN-free: canonical C vs it | NaN-bearing: Rust vs it | NaN-bearing: canonical C vs it |
|---------|---------------------|------------------------------|--------------------------|--------------------------------|
| `gcc` (no `-O`) | **0** | **0** | **0** | **0** |
| `-O0` | **0** | **0** | **0** | **0** |
| `-O1` | **0** | **0** | 2 906 | 2 906 |
| `-O2` | **0** | **0** | 2 906 | 2 906 |
| `-Os` | **0** | **0** | 2 906 | 2 906 |
| `-O3` | **0** | **0** | 1 849 | 1 849 |
| `-Ofast` | 15 415 | 15 415 | 41 830 | 41 830 |

1. **Every NaN-free input is bit-exact at every conforming `-O` level** — the
   match is not brittle over the realistic input domain.
2. Where the Rust differs from an `-O1`/`-O2`/`-O3` build, **the canonical C
   differs from it in exactly the same number of places**. The residual
   disagreement is GCC's own `-O`-dependent NaN-payload operand ordering
   (C-vs-C instability), not a translation defect. `-Ofast` is `-ffast-math` and
   non-conforming, so it is reported informationally rather than asserted.
3. `-O0` and the default build are bit-identical to the cmake build **and** to
   the Rust across all ~254 000 triples.

## Independent ABI cross-check

`probe/abi_probe.c` is a plain C program **linked directly** against a `.so`
(real dynamic linking and PLT; calling convention chosen by the C compiler, not
by Rust's `extern "C"` shim). Built once against the C `.so` and once against
each Rust `.so`; outputs diffed over the exhaustive alphabet, 4096 random
batched elements, non-positive `count`s, and four in-place/overlap offsets:

```
native C caller: C .so == Rust debug   .so  (22821 lines identical)
native C caller: C .so == Rust release .so  (22821 lines identical)
```

An ordinary external C consumer cannot distinguish the two libraries.

## Changes made to `translation/`

| file | change |
|------|--------|
| `Cargo.toml` | added `libloading = "0.8"` to `[dev-dependencies]`; added `[profile.dev]` with `debug-assertions = false`, `overflow-checks = true` (the E6/E7 fix) |
| `src/lib.rs` | **unchanged** — `md5sum` verified identical before and after mutation testing |
| `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`, `VERIFICATION.md` | new artifacts |
| `tests/`, `probe/`, `run_all.sh` | new tests, ABI probe, configuration-matrix driver |

`c_src/` was **never modified** — only read, and built into `c_src/build/` via the
prescribed cmake commands. The extra `-O`-level objects for `tests/olevels.rs`
are compiled out-of-tree into `translation/target/olevels/`.
