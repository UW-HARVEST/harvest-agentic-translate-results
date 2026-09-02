# Verification notes

## How to run

```
cd translation && ./run_all.sh          # builds C + Rust, checks symbols, runs all combos
```

`run_all.sh` exists because **`cargo test` does not rebuild the `cdylib`**: the
integration tests `dlopen` the `.so` instead of linking it, so cargo sees no
dependency and happily leaves a stale library in `target/release/`. Running
`cargo test` alone can therefore "pass" against a library that does not contain
your latest edit. Always go through the script (or `cargo build --release`
first).

Both `.so` paths can be overridden with `C_SO_PATH` / `RUST_SO_PATH`.

## Test layout

| file | scope |
|------|-------|
| `tests/common/mod.rs` | `dlopen`/`dlsym` of both `.so`s, ABI mirrors of the C structs, bit-exact comparators, seeded PRNG, pathological-value pool, polygon builders |
| `tests/phase_b_vector.rs` | CONFIGS rows 1-20 (scalar / vector / transform layer) |
| `tests/phase_b_predicates.rs` | CONFIGS rows 21-25 (boolean predicates) |
| `tests/phase_b_circle_aabb.rs` | CONFIGS rows 26-36 |
| `tests/phase_b_capsule.rs` | CONFIGS rows 37-43 + all-7-exits branch coverage guard |
| `tests/phase_b_poly.rs` | CONFIGS rows 44-62 + all-4-exits branch coverage guard |
| `tests/phase_b_dispatch.rs` | CONFIGS rows 63-70 (`c2CastRay`, `poly_ray`) |
| `tests/phase_c_errors.rs` | ERRORS rows 1-49 + generic FFI boundaries |
| `tests/nan_storm.rs` | exhaustive / high-volume pathological-input sweeps |
| `tests/phase_d_symbols.rs` | `nm -D` parity, in-suite |
| `tests/targeted_operand_order.rs` | targeted coverage for the two NaN-selection sites that a random sweep reaches only rarely |
| `tests/probe_nan.rs` | diagnostic: identifies which operand each arithmetic site returns on NaN |

Every test loads the Rust library through `libloading` and calls its exported
symbols; no Rust function is ever called directly, so the `#[no_mangle]`
wrappers and the C ABI of every by-value struct argument/return are under test.

## Feature combinations

`Cargo.toml` declares no `[features]` section, so the only combinations that
exist are the default and `--no-default-features` (identical). `run_all.sh`
enumerates them from `Cargo.toml` and runs the full suite plus the symbol diff
for each; adding features later needs no change to the script.

## The one genuinely build-specific behaviour: NaN operand selection

This is the only place where the translation had to be pinned to the reference
build rather than to the C source.

x86 SSE scalar arithmetic (`addss` / `subss` / `mulss` / `divss`) is
`dst = dst OP src`, and when **both** operands are NaN the hardware returns the
*destination* operand quieted. Which of the two program values is in the
destination register is a register-allocation decision, so for a commutative
expression like `a.x * b.x + a.y * b.y` it is not determined by the C source at
all.

Measured with `tests/probe_nan.rs` (NaNs carrying distinct payloads, so the
result names its own source operand):

| build | `c2Dot((N1,N2),(N3,N4))` returns |
|-------|----------------------------------|
| `gcc -O0` (the reference build) | `N4` — i.e. `b.y` |
| `gcc -O1` / `-O2` / `-O3` | `N1` — i.e. `a.x` |
| naive Rust translation (LLVM `-O3`) | `N1` |

So **no single Rust implementation can match every GCC optimisation level** —
the levels disagree with each other. The translation matches the reference
build produced by the documented command:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so that is an unoptimised
(`-O0`) build. Against it the whole suite passes (73/73 tests); against
`gcc -O1..-O3` the NaN-vs-NaN comparisons in `tests/nan_storm.rs` diverge, by
construction.

The pinning lives in `src/lib.rs` in the `addss`/`subss`/`mulss`/`divss`
helpers. Each call site carries a comment quoting the reference
disassembly that fixes its `dst`/`src` assignment. For non-NaN operands the
helpers are plain `+ - * /`, so no ordinary result is affected — only NaN
sign/payload selection is forced.

Note that subtraction and division are unaffected: they are non-commutative, so
the destination is always the minuend/numerator regardless of the compiler.

Only two site groups turned out to be genuinely load-bearing, established by
reverting each pin in isolation and re-testing:

| site | load-bearing? | evidence |
|------|---------------|----------|
| `c2Dot`, `c2Add`, `c2Mulvs`, `c2Div`, `c2MulmvT`, `c2Mulrv`, `c2MulrvT` | **yes** | reverting any of them reintroduces divergences at both Rust opt levels |
| `c2RaytoAABB`: `out->t = tK * A.t` | **yes** | reverting it reintroduces 26 divergences in the unoptimised Rust build (caught by `tests/targeted_operand_order.rs`) |
| `c2RaytoCapsule` wall arithmetic, `c2RaytoCircle`, the plane helpers, `c2RaytoPoly` | no — cosmetic | the naive form also matches at both Rust opt levels; the pins are kept for uniformity and to document the reference `dst`/`src` assignment |

### Result matrix

The Rust library is itself optimisation-level independent — `debug` and
`release` behave identically — which the pinning is what buys:

| Rust profile | C `-O0` (reference) | C `-O1` | C `-O2` | C `-O3` |
|--------------|---------------------|---------|---------|---------|
| `release` | **all 73 pass** | 4 fail | 4 fail | 4 fail |
| `debug` | **all 73 pass** | 4 fail | 4 fail | 4 fail |

The four failures against `-O1`+ are exclusively NaN sign/payload comparisons in
`tests/nan_storm.rs`; every return value, every finite result and every error
sentinel agrees at all four optimisation levels. As shown above this is not
fixable — the GCC levels disagree with each other — so the translation targets
the build the documented command produces.

## Quirks of the C that are reproduced deliberately, not "fixed"

- `c2Absv` uses `x < 0 ? -x : x`. Since `-0.0 < 0` is false, `-0.0` is returned
  **unchanged**, unlike `fabsf`. Pinned by an explicit bit-pattern assertion.
- `c2Minv` / `c2Maxv` use `a < b ? a : b`, which returns `b` whenever the
  comparison is false — so NaN propagation differs from `fminf` / `fmaxf`.
- `c2Div` computes `a * (1.0f / b)`, not `a / b`; the reciprocal's rounding is
  observable.
- `c2AABBtoAABB` and `c2AABBtoPoint` report NaN inputs as *overlapping* /
  *inside*, because all four rejecting comparisons are false.
- `c2CircleToPoint` uses a strict `<`, so a point exactly on the rim is outside.
- `c2RaytoCapsule` writes `*out` (`n = c2Norm(b - a)`, `t = 0`) **before** it
  knows whether it will hit, so `*out` is modified even on the `return 0` path.
  `ERRORS.md` row 28 tests exactly this; a translation that only wrote `*out` on
  success would pass every hit test and fail here.
- Negative radii work like positive ones (only `r * r` is used), and inverted
  AABBs / non-convex polygons / non-unit rotations are never validated.
- `c2CastRay`'s `switch` has no `default`, so any out-of-range `int` returns 0
  without dereferencing the shape pointer. Verified by passing `B = NULL`.

## Deliberately not tested

`out == NULL`. No function in the library null-checks it, so both
implementations dereference it and crash; a differential test would compare two
segfaults. Documented as `ERRORS.md` row 50.

`c2Poly.count > 8` is tested **only** through a shared oversized backing buffer
(`CONFIGS.md` row 56), because the C reads past the declared arrays. Handing
each language its own stack-allocated `c2Poly` would compare different garbage
and produce meaningless failures.

## Harness self-check (mutation testing)

To confirm the suite can actually detect divergence rather than passing
vacuously, mutations were injected into `src/lib.rs` one at a time and the suite
re-run:

| mutation | detected |
|----------|----------|
| `c2Dot` add operand order swapped | yes |
| `c2Absv` → `f32::abs` | yes |
| `c_min` → `f32::min` | yes |
| `c2CircleToPoint` `<` → `<=` | yes |
| `c2Div` → true componentwise division | yes |
| `c2RaytoCapsule` stops pre-writing `*out` | yes |
| `c2RaytoAABB` tie-break `>=` → `>` | yes |
| `c2RaytoAABB` `out->t` operand order swapped | yes (via `tests/targeted_operand_order.rs`) |
| `c2RayToPlane_OneDimensional` `da < 0` → `da <= 0` | yes |
| `c2RaytoPoly` ignores `bx == NULL` and always uses identity | yes |
| `c2CastRay` drops the `C2_TYPE_POLY` arm | yes |
| `index != !0` → `index >= 0` | no — **equivalent mutant**: `index` is only ever `-1` or a loop index in `[0, count)` |
| `c2RaytoCapsule` wall `mulss`/`addss` operand order | no — **equivalent**: those operands are never two *distinct* NaNs in any reachable state (confirmed by reverting the pin entirely: the naive form still matches at both opt levels) |
| plane helper `mulss(p, n)` → `mulss(n, p)` | no — **equivalent**: `n` is a `±1.0` literal at every call site, so it can never be the NaN operand |

Three escapes, all analysed and shown to be semantically equivalent rather than
coverage gaps. One earlier "escape" was a bug in the mutation *script* (it
patched only the `t0` branch of `c2RaytoAABB` while the reachable divergence is
in the `t3` branch); patching the right branch is detected.
