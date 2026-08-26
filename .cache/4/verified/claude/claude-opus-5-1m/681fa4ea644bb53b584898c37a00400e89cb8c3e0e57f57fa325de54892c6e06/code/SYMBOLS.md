# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

Generated mechanically:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort

# Rust
cargo build
nm -D --defined-only target/debug/libspec_ray_lib.so | awk '{print $3}' | sort
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → exactly **one** valid feature
  combination: the default/empty one. Verified with
  `cargo check --no-default-features` (clean) and
  `cargo check --all-features` (clean).
* `c_src/CMakeLists.txt` has no options, no `add_definitions`, no
  `target_compile_definitions`; `grep -n '#if\|#ifdef\|#ifndef\|#define'
  c_src/src/lib.c c_src/include/lib.h` returns **nothing** → the C library has
  no compile-time configuration either. `CMAKE_BUILD_TYPE` is empty, so the
  reference `.so` is built at `-O0` and calls `sqrtf@plt` (glibc wrapper) —
  relevant for NaN/negative-radicand bit patterns.

## Exported (dynamic, defined) symbols

| # | C symbol (`nm -D` type) | in Rust `.so` | notes |
|---|-------------------------|---------------|-------|
| 1 | `c2V` (T) | ✅ `c2V` | `#[no_mangle] extern "C"`, returns `c2v` (8 B → 1 SSE reg) |
| 2 | `c2Dot` (T) | ✅ `c2Dot` | |
| 3 | `c2Len` (T) | ✅ `c2Len` | |
| 4 | `c2Add` (T) | ✅ `c2Add` | |
| 5 | `c2Sub` (T) | ✅ `c2Sub` | |
| 6 | `c2Mulvs` (T) | ✅ `c2Mulvs` | |
| 7 | `c2Div` (T) | ✅ `c2Div` | reciprocal multiply, not divide |
| 8 | `c2Norm` (T) | ✅ `c2Norm` | |
| 9 | `c2Minv` (T) | ✅ `c2Minv` | ternary min (NaN/±0 semantics) |
| 10 | `c2Maxv` (T) | ✅ `c2Maxv` | ternary max |
| 11 | `c2Skew` (T) | ✅ `c2Skew` | |
| 12 | `c2Absv` (T) | ✅ `c2Absv` | ternary abs (keeps `-0.0`) |
| 13 | `c2CCW90` (T) | ✅ `c2CCW90` | |
| 14 | `c2MulmvT` (T) | ✅ `c2MulmvT` | takes `c2m` (16 B → 2 SSE regs) |
| 15 | `c2AABBtoAABB` (T) | ✅ `c2AABBtoAABB` | |
| 16 | `c2AABBtoPoint` (T) | ✅ `c2AABBtoPoint` | |
| 17 | `c2CircleToPoint` (T) | ✅ `c2CircleToPoint` | |
| 18 | `c2RaytoCircle` (T) | ✅ `c2RaytoCircle` | `c2Ray` is 20 B → MEMORY class |
| 19 | `c2RaytoAABB` (T) | ✅ `c2RaytoAABB` | |
| 20 | `c2RaytoCapsule` (T) | ✅ `c2RaytoCapsule` | `c2Capsule` is 20 B → MEMORY class |
| 21 | `c2CastRay` (T) | ✅ `c2CastRay` | `C2_TYPE` crosses FFI as `c_int` |
| 22 | `spec_ray` (T) | ✅ `spec_ray` | the only symbol declared in `include/lib.h` |

Symbols that are `static inline` in the C and therefore **not** exported
(correctly private in Rust too):
`c2SignedDistPointToPlane_OneDimensional`, `c2RayToPlane_OneDimensional`.

## Diff result

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/debug/libspec_ray_lib.so    | awk '{print $3}' | sort)
$ echo $?
0
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libspec_ray_lib.so    | awk '{print $3}' | sort)
$ echo $?
0
```

**22 C symbols, 22 Rust symbols, 0 missing, 0 extra — the exported sets are
identical in both the dev and the release profile.**
Nothing was stubbed: every symbol above is a full translation of the
corresponding C function body (no `unimplemented!()`, no `todo!()`, no empty
bodies — `grep -n 'unimplemented\|todo!\|unreachable!' src/lib.rs` is empty).

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libspec_ray_lib.so` lists only libc /
libgcc-unwind / Rust-std runtime imports (`memcpy`, `malloc`, `free`,
`_Unwind_*`, `__tls_get_addr`, `dl_iterate_phdr`, …).
**0 missing/undefined non-libc symbols.**

## How to reproduce the whole verification

```sh
./verify_all.sh          # C build + every feature combo x {dev, release}
```

which performs, per configuration:

* `cargo check --no-default-features … --all-targets`
* `cargo build …` (produces `target/{debug,release}/libspec_ray_lib.so`)
* the `nm -D` symbol-parity diff shown above,
* the full differential suite: **89 tests**, all of which load BOTH `.so`s with
  `libloading` and call them only through their exported C symbols:

| test binary | rows | tests |
|-------------|------|-------|
| `tests/phase_b_scalar.rs` | `CONFIGS.md` 1–17 | 17 |
| `tests/phase_b_circle_aabb.rs` | `CONFIGS.md` 18–35 | 18 |
| `tests/phase_b_capsule.rs` | `CONFIGS.md` 36–47 | 13 |
| `tests/phase_b_toplevel.rs` | `CONFIGS.md` 48–56 | 9 |
| `tests/phase_c_errors.rs` | `ERRORS.md` 1–35, 38–42, 44 | 30 |
| `tests/phase_c_null.rs` | `ERRORS.md` 36, 37, 43 | 2 |

Last run: `ALL CONFIGURATIONS VERIFIED` — 89/89 tests pass for
`<no features>` × {dev, release}, symbol parity 22/22 in both.

### Guard against testing a stale library

`cargo test` compiles the crate for the *test* target but does **not** re-link
the `cdylib`, so running `cargo test` after editing `src/` would silently test
the previous `.so`.  `tests/common/mod.rs` therefore compares the `.so`'s mtime
against the newest file in `src/` (and the C `.so` against `c_src/`) and fails
the run with a "stale library — rebuild first" message.  `verify_all.sh` always
runs `cargo build` before `cargo test`.

### Mutation check (do the tests have teeth?)

Eight deliberate mutations of `src/lib.rs` were injected one at a time
(`cargo build && cargo test --no-fail-fast`); every one was caught, by exactly
the rows that own the behaviour:

| mutation | failing tests |
|----------|---------------|
| `tern_abs` → `f32::abs` (loses `-0.0` / `-NaN`) | `row12_c2Absv_ternary_not_fabsf` |
| `c2Div` → component-wise `a/b` instead of `a * (1/b)` | 41 tests |
| `c2RaytoAABB`: `t0 >= t1` → `t0 > t1` (tie handling) | 8 tests incl. `row29_raytoaabb_ties_and_corners` |
| `c2CircleToPoint`: `<` → `<=` (rim) | 7 tests incl. `err25_circletopoint_on_rim_is_a_miss` |
| `c2RaytoCapsule`: drop the early `*out` write | 21 tests incl. `row44…`, `err28…` |
| `c2Dot` → naive source operand order (NaN payloads) | 5 tests incl. `row03_c2Dot_zero_inf_nan` |
| `c2CastRay`: invalid `typeB` → `return 1` | `err34_castray_out_of_range_enum_values` |
| `c2RaytoCircle`: `t <= A.t` → `t < A.t` | 8 tests incl. `row22…`, `err03…` |

The suite also passes unchanged with `DIFF_SEED_OFFSET=1,2,3,7,42`, i.e. no row
depends on a lucky seed.
