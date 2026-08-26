# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Reference artifacts:

* C:    `c_src/build/libtranslated_rust.so`
  (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
  — no `CMAKE_BUILD_TYPE`, i.e. the compiler's default `-O0`)
* Rust: `target/debug/libgen_ray_lib.so` (`cargo build`) and
  `target/release/libgen_ray_lib.so` (`cargo build --release`)

Regenerate the comparison with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only target/debug/libgen_ray_lib.so    | awk '{print $3}' | sort > /tmp/r.txt
diff /tmp/c.txt /tmp/r.txt   # must be empty
```

## Result

`nm -D --defined-only` on the C `.so` yields exactly **22** `T` symbols.
All 22 are exported by the Rust `.so` with identical names.
**`diff` is empty — 0 missing symbols, 0 extra symbols.**

| # | symbol | C declaration (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` |
|---|--------|-----------------------------------|-----------|---------------|
| 1 | `c2V` | `c2v c2V(float x, float y)` | T | T |
| 2 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | T | T |
| 3 | `c2Len` | `float c2Len(c2v a)` | T | T |
| 4 | `c2Add` | `c2v c2Add(c2v a, c2v b)` | T | T |
| 5 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | T | T |
| 6 | `c2Mulvs` | `c2v c2Mulvs(c2v a, float b)` | T | T |
| 7 | `c2Div` | `c2v c2Div(c2v a, float b)` | T | T |
| 8 | `c2Norm` | `c2v c2Norm(c2v a)` | T | T |
| 9 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | T | T |
| 10 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | T | T |
| 11 | `c2Skew` | `c2v c2Skew(c2v a)` | T | T |
| 12 | `c2Absv` | `c2v c2Absv(c2v a)` | T | T |
| 13 | `c2RaytoCircle` | `int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out)` | T | T |
| 14 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB A, c2AABB B)` | T | T |
| 15 | `c2RaytoAABB` | `int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out)` | T | T |
| 16 | `c2CCW90` | `c2v c2CCW90(c2v a)` | T | T |
| 17 | `c2MulmvT` | `c2v c2MulmvT(c2m a, c2v b)` | T | T |
| 18 | `c2AABBtoPoint` | `int c2AABBtoPoint(c2AABB A, c2v B)` | T | T |
| 19 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle A, c2v B)` | T | T |
| 20 | `c2RaytoCapsule` | `int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out)` | T | T |
| 21 | `c2CastRay` | `int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out)` | T | T |
| 22 | `gen_ray` | `int gen_ray(c2Raycast*, c2Raycast*, c2Raycast*, float x18)` | T | T |

## Deliberately NOT exported (parity requires their absence)

These two C functions are `static inline`; at `-O0` gcc emits them as **local**
symbols, so they do not appear in `nm -D`.  The Rust translation keeps them as
private `#[inline] fn`s (no `#[no_mangle]`), which is the matching behaviour:

| symbol | C declaration | C `.so` `nm -D` | Rust `.so` `nm -D` |
|--------|---------------|-----------------|--------------------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline float (float p, float n, float d)` | absent (local `t`) | absent |
| `c2RayToPlane_OneDimensional` | `static inline float (float da, float db)` | absent (local `t`) | absent |

Note: at `-O0` these two are *not* inlined and *not* constant-folded — they are
real calls with real `mulss` instructions. The Rust translation reproduces the
arithmetic they perform (see `ERRORS.md` rows E14/E15 and the code comments in
`src/lib.rs`), because folding `p * -1.0f` into `-p` changes NaN results.

## Undefined symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
pthread imports (`memcpy`, `malloc`, `_Unwind_*`, …) pulled in by the Rust
standard library.  There are **no undefined non-libc symbols**, i.e. nothing
from the C library is being referenced instead of translated.

The C `.so` imports `sqrtf@GLIBC` (from `-lm`); the Rust `.so` implements the
equivalent with the `sqrtss` instruction emitted by `f32::sqrt`, which is
bit-identical for every input (verified for ±0, ±inf, qNaN, sNaN, subnormal and
20 000 random bit patterns by `tests/t1_vector_ops.rs::b03_c2len (+ 200 000 more in tests/t10_torture.rs)`).

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so there is exactly one build
configuration (`--no-default-features` == default == all features). `c_src`'s
`CMakeLists.txt` has no options, no `#ifdef`/`#if` in `lib.c` or `lib.h`, and
no `target_compile_definitions`, so the C side likewise has a single
configuration. See `CONFIGS.md` § "Build-time configuration surface".

## Reproducing the whole verification

| script | what it does |
|--------|--------------|
| `./run_diff_tests.sh [args…]` | builds the C `.so` (if absent), builds the Rust cdylib (`cargo test` does **not** rebuild a cdylib), then runs `cargo test`. `FEATURES=…` / `PROFILE_FLAGS=--release` supported. |
| `./check_all_features.sh` | enumerates the power set of `[features]` from `Cargo.toml` (currently exactly one combination, the empty one), and for each × {dev, release}: `cargo check`, `cargo build`, `nm -D` symbol diff, full `cargo test`. |
| `./audit_rows.py [--release]` | checks that the `CONFIGS.md` / `ERRORS.md` row ids are contiguous, that every row is named in the traceability table, that every named test function exists, and that all of them pass. |

A bare `cargo test` also works in a fresh checkout: `cargo test` does *not*
rebuild a `cdylib`, so `tests/common/mod.rs` compares each `.so`'s mtime against
its source and, if the artifact is missing or stale, rebuilds it itself —
the Rust side with a direct `rustc --crate-type=cdylib` call (no dependencies,
so no cargo lock involved) and the C side with
`cc -fPIC -shared -O0 -Ic_src/include c_src/src/lib.c -lm` into
`target/diff-artifacts/` (nothing under `c_src/` is ever written).  This means
the suite can never silently test yesterday's `.so`.

Current status (both `dev` and `release`, the only feature combination):

* `nm -D` diff: **empty** (22/22 symbols, no extras, no missing).
* `cargo test`: **76 test functions, 0 failures**, ≈ 6.5 million bit-exact
  differential comparisons plus 8 crash-signal comparisons.
* gcov of the C reference under the suite: 100 % of branches evaluated, 100 % of
  calls executed, 99.53 % of lines (the one remaining line is provably dead
  code — see `CONFIGS.md` § "Coverage evidence").
