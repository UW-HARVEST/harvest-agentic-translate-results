# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-GcYXBw.so   | awk '{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libspec_ray_lib.so | awk '{print $3}' | sort -u > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

The C `.so` exports 22 symbols. All 22 are exported by the Rust `cdylib` under
the identical linker name. `static` C functions
(`c2SignedDistPointToPlane_OneDimensional`, `c2RayToPlane_OneDimensional`) are
*not* exported by the C `.so` and are therefore private helpers in Rust too.

| # | symbol | C signature (from `src/lib.c` / `include/lib.h`) | in C `.so` | in Rust `.so` |
|---|--------|--------------------------------------------------|-----------|---------------|
| 1 | `c2V` | `c2v c2V(float, float)` | yes | yes |
| 2 | `c2Dot` | `float c2Dot(c2v, c2v)` | yes | yes |
| 3 | `c2Len` | `float c2Len(c2v)` | yes | yes |
| 4 | `c2Add` | `c2v c2Add(c2v, c2v)` | yes | yes |
| 5 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | yes | yes |
| 6 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | yes | yes |
| 7 | `c2Div` | `c2v c2Div(c2v, float)` | yes | yes |
| 8 | `c2Norm` | `c2v c2Norm(c2v)` | yes | yes |
| 9 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | yes | yes |
| 10 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | yes | yes |
| 11 | `c2Skew` | `c2v c2Skew(c2v)` | yes | yes |
| 12 | `c2Absv` | `c2v c2Absv(c2v)` | yes | yes |
| 13 | `c2CCW90` | `c2v c2CCW90(c2v)` | yes | yes |
| 14 | `c2MulmvT` | `c2v c2MulmvT(c2m, c2v)` | yes | yes |
| 15 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB, c2AABB)` | yes | yes |
| 16 | `c2AABBtoPoint` | `int c2AABBtoPoint(c2AABB, c2v)` | yes | yes |
| 17 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle, c2v)` | yes | yes |
| 18 | `c2RaytoCircle` | `int c2RaytoCircle(c2Ray, c2Circle, c2Raycast*)` | yes | yes |
| 19 | `c2RaytoAABB` | `int c2RaytoAABB(c2Ray, c2AABB, c2Raycast*)` | yes | yes |
| 20 | `c2RaytoCapsule` | `int c2RaytoCapsule(c2Ray, c2Capsule, c2Raycast*)` | yes | yes |
| 21 | `c2CastRay` | `int c2CastRay(c2Ray, const void*, C2_TYPE, c2Raycast*)` | yes | yes |
| 22 | `spec_ray` | `int spec_ray(c2Raycast*, float, float, float, float, float, float, float)` | yes | yes |

## Result

- Missing from Rust: **0**
- Extra in Rust: **0**
- Undefined (imported) non-libc symbols in the Rust `.so`: **0** — the
  undefined list is entirely glibc + `_Unwind_*` (libgcc) + the standard
  `_ITM_*` / `__gmon_start__` weak stubs. The C `.so` imports `sqrtf@GLIBC`;
  the Rust build lowers `f32::sqrt` to the `sqrtss` instruction, so it has no
  such import. Both compute the same values (see `CONFIGS.md` row on `c2Len`
  / `sqrt` shapes, which is verified differentially including NaN payloads).

No C module was skipped: `c_src` consists of exactly one translation unit
(`src/lib.c`) plus one header (`include/lib.h`), and every non-`static`
definition in it has a Rust counterpart.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
build configuration is the default one. Verified mechanically:

```sh
$ grep -c '^\[features\]' translation/Cargo.toml
0
```

`cargo test`, `cargo test --no-default-features`, and
`cargo test --all-features` therefore all select the same code, and the
symbol table above is identical under each (checked automatically by
`verify_all.sh`, which enumerates the combinations from `Cargo.toml` rather
than hard-coding them, and also runs the whole suite under the `debug`
profile).

## What "the reference C build" means, and why it matters

The C is compiled by the documented command, which sets no
`CMAKE_BUILD_TYPE` and therefore no `-O` flag:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

Two observable behaviours of this library are decided by the compiler rather
than by the C text, so they are matched against *that* build specifically:

1. **NaN payload propagation.** `addss`/`mulss` return the destination
   operand's payload, and which operand is the destination is the register
   allocator's choice. `c2Dot`, `c2Add`, `c2Mulvs`, `c2MulmvT` and the
   arithmetic inside the raycasts are all affected.
2. **`c2CastRay` with an out-of-range `C2_TYPE`.** Control runs off the end of
   a non-`void` function; the compiled meaning is whatever the epilogue happens
   to leave in `%eax`.

Both were verified by disassembly and then measured. `tests/c_build_sensitivity.rs`
(env-gated on `ALT_C_SO`) quantifies it against an independently compiled C:

| C build | `c2Dot` NaN-pair agreement | `c2CastRay` UB-edge agreement |
|---------|----------------------------|-------------------------------|
| reference (cmake, no `-O`) | 400 000 / 400 000 | 2 000 / 2 000 |
| `gcc -O0` (independent)    | 400 000 / 400 000 | 2 000 / 2 000 |
| `gcc -O2`                  | 0 / 400 000       | 0 / 2 000     |

At `-O2` GCC reverses the destination operand of `c2Dot`'s second `mulss` and
of its `addss`, and its `c2CastRay` epilogue returns `%rax`, which it had
loaded with `B`. So the translation is pinned to the reference build, and the
tests are sharp enough to detect a one-instruction difference in either place.
This is deliberate: the reference build is the ground truth.
