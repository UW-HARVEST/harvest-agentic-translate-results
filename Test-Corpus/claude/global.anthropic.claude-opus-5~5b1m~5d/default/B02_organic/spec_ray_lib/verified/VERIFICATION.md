# Verification report — C → Rust differential testing

The Rust `cdylib` in `src/lib.rs` is a translation of `c_src/src/lib.c` (a
cut-down `cute_c2`: 2D ray casts against circles, AABBs and capsules).  This
document records how it was verified and what the results are.

**Everything is verified through the FFI boundary.** Both libraries are loaded
with `libloading` and driven through their exported symbols — the Rust functions
are never called directly, so the `#[no_mangle] extern "C"` wrappers, the SysV
struct-passing/returning ABI and the out-parameter writes are all part of what is
under test.

```
c_src/build/lib<workdir>.so          reference, built by the documented
                                     `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`
                                     (no CMAKE_BUILD_TYPE  =>  -O0)
target/{debug,release}/libspec_ray_lib.so    the Rust cdylib under test
target/cref-O2/lib<workdir>.so       second C build (-O2), used to show which
                                     differences are compiler artefacts
```

## Result

| gate | result |
|---|---|
| `cargo check --all-targets` | clean, 0 errors, 0 warnings |
| `SYMBOLS.md` — symbol parity | **22 / 22**, 0 missing, 0 non-libc undefined |
| `CONFIGS.md` — Phase B rows | **40 / 40 pass** |
| `ERRORS.md` — Phase C rows | **35 / 35 pass** |
| bit-exactness | **62 929 792 comparisons, 0 mismatches** (final pass at `SPEC_RAY_N=200000`) |
| feature combinations | `default` and `--no-default-features` (the crate declares no features) — both pass |
| Rust profiles | dev and release cdylib — both pass |
| C reference builds | `-O0` (the documented build) and `-O2` — both pass |

`./verify.sh` runs that entire matrix; its last line is
`ALL CHECKS PASSED across every feature combination, both Rust profiles and both
C reference builds.`

## Test suites

| file | what it does |
|---|---|
| `tests/common/mod.rs` | harness: `dlopen`s both `.so`s, mirrors the C structs, a seeded splitmix64 PRNG with a "wild" generator (specials, denormals, ±inf, NaN, raw bit patterns), a bit-exact `Checker`, and **branch classifiers built from the C library's own exports** so each test can prove which sub-path it reached |
| `tests/smoke_probe.rs` | all 22 symbols resolve in both `.so`s and agree; `nm -D` symbol-parity assertion |
| `tests/phase_b_valid.rs` | 40 tests, one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 36 tests, one per `ERRORS.md` row (+ a cross-process UB probe) |
| `tests/nan_payload_policy.rs` | measures and justifies the single tolerance (see below) |

## The three things that are *not* bit-identical, and why

Everything the C standard and IEEE-754 actually specify **is** bit-identical.
Three behaviours are outside that, and each is pinned down by a dedicated test
rather than waved away:

1. **NaN payloads when two NaN operands meet in one `mulss`/`addss`.**
   IEEE-754 leaves the payload unspecified; x86 propagates whichever operand the
   compiler chose as the destination register. On the identical 133 520-comparison
   NaN corpus: C `-O0` vs C `-O2` differ on **2210** payloads (0 non-NaN
   differences), C `-O0` vs Rust differ on **1676**. The C source therefore does
   not have "a" payload behaviour — the translation is closer to the reference
   than a second legitimate build of the C itself. The harness compares NaN
   results as "both NaN", counts and prints every payload difference, and asserts
   the Rust count never exceeds the C-vs-C count. `SPEC_RAY_STRICT_NAN=1` turns
   them into failures for inspection. Non-NaN results are compared bit for bit,
   always.

2. **`c2CastRay` with a `typeB` that has no enum variant.** The C `switch` has no
   `default`, so control falls off the end of a non-`void` function; the
   disassembly shows the fall-through jumping straight to `leave; ret` without
   ever writing `eax`. Measured in 5 separate processes, the C "returned" 5
   different values (ASLR-dependent stack/register garbage). There is no
   behaviour to reproduce, so the Rust returns a deterministic `0`;
   `err_24`/`err_24b` assert the entire *defined* part instead: `*out` is
   untouched by both libraries for 15 out-of-range values, neither crashes, and a
   subsequent valid call is unaffected.

3. **The dev-profile null-pointer check.** `c2RaytoCapsule(out = NULL)` and
   `c2CastRay(B = NULL)` fault in both libraries. The **release** cdylib — the
   artifact an external consumer links — dies with SIGSEGV exactly like the C
   (verified in a child process). The **dev** cdylib dies with SIGABRT instead,
   because `-C debug-assertions=on` detects the null dereference itself
   (`"null pointer dereference occurred"`). `err_23`/`err_25` accept exactly
   those two forms and nothing else.

## Notes found while verifying (C behaviour that had to be preserved)

* `c2RaytoCapsule` writes `*out` (`n = c2Norm(b-a)`, `t = 0`) **before** it
  decides anything, so a *miss* still modifies the out-parameter — unlike
  `c2RaytoCircle`/`c2RaytoAABB`, which never touch it on a miss. Both facts are
  asserted (`cfg_40`, `err_20`, `err_21`).
* `c2Minv`/`c2Maxv`/`c2Absv` are raw C ternaries, **not** `fminf`/`fmaxf`/`fabsf`:
  `c2Minv(NaN, x) == x` but `c2Minv(x, NaN) == NaN`, and `c2Absv(-0.0) == -0.0`,
  `c2Absv(-NaN) == -NaN` (`fabsf` would clear the sign). `err_35` asserts the
  asymmetry itself, not just C/Rust agreement.
* `c2AABBtoAABB` *accepts* a box full of NaN (every `<` is false, so
  `!(d0|d1|d2|d3)` is 1) — `err_10`.
* A negative radius behaves like `|r|` everywhere (`r*r`), and `r == 0` means
  "nothing is inside", not even the exact centre (`0 < 0` is false) — `err_19`,
  `err_32`.
* `c2Div` multiplies by the reciprocal (`a * (1.0f/b)`), which is **not** the same
  as dividing; `b = -0.0` gives `-inf`, and normalising the zero vector gives
  `(NaN, NaN)` — `err_27`, `err_28`.

## Reproducing

```sh
cd translation
./verify.sh                                  # the whole matrix (~1 min)
SPEC_RAY_N=200000 ./verify.sh                # heavier randomization
cargo build --offline --release              # then, individually:
cargo test --offline --test phase_b_valid -- --nocapture --test-threads=1
cargo test --offline --test phase_c_errors -- --nocapture --test-threads=1
```

Useful environment variables: `SPEC_RAY_C_SO`, `SPEC_RAY_RUST_SO` (pin either
library), `SPEC_RAY_N` (inputs per row), `SPEC_RAY_STRICT_NAN=1` (fail on NaN
payload differences).  `cargo` needs `--offline` in this sandbox; the tests need
the cdylib to exist, so `cargo build` first — `cargo test` alone does not build a
`cdylib`-only lib target, and the harness fails loudly if the `.so` is older than
`src/lib.rs`.
