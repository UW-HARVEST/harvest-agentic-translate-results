# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-yJX7LD.so
nm -D --defined-only translation/target/release/libcircle_collide_lib.so
```

`c_src/src/lib.c` is the ONLY C translation unit (see `c_src/CMakeLists.txt`),
so the full C public surface is the 12 non-static functions it defines. No C
module was skipped by the translation; there is no binary/driver target in
either build.

## Symbol table

| # | symbol | C signature | in C `.so` | in Rust `.so` | notes |
|---|--------|-------------|-----------|---------------|-------|
| 1 | `c2V` | `c2v c2V(float,float)` | yes | yes | 8-byte struct return (1 SSE eightbyte) |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v,float)` | yes | yes | operand order `a.x * b` preserved |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v,c2v)` | yes | yes | bare `>` ternary, NOT `f32::max` |
| 4 | `c2Minv` | `c2v c2Minv(c2v,c2v)` | yes | yes | bare `<` ternary, NOT `f32::min` |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v,c2v,c2v)` | yes | yes | `Maxv(lo, Minv(a,hi))` |
| 6 | `c2Sub` | `c2v c2Sub(c2v,c2v)` | yes | yes | |
| 7 | `c2Dot` | `float c2Dot(c2v,c2v)` | yes | yes | `x*x + y*y`, no FMA contraction |
| 8 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle,c2Circle)` | yes | yes | 12-byte structs → 2 SSE eightbytes each |
| 9 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle,c2AABB)` | yes | yes | 12 + 16 bytes, all in xmm |
| 10 | `c2CircletoCapsule` | `int c2CircletoCapsule(c2Circle,c2Capsule)` | yes | yes | `c2Capsule` is 20 B > 16 B → MEMORY class (stack) |
| 11 | `c2Collided` | `int c2Collided(const void*,const void*,C2_TYPE)` | yes | yes | enum arg passed as `int` |
| 12 | `circle_collide` | `int circle_collide(float,float,float)` | yes | yes | the only symbol in `include/lib.h` |

## Diff result

```
comm -23 c_syms rs_syms   ->  (empty)
```

* Symbols exported by C but missing from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**
  (all undefined entries are glibc / `_Unwind_*` / `_ITM_*` / `__gmon_start__`
  runtime imports pulled in by `std`, none of them project code)

## Types (must be layout-identical across the FFI boundary)

```c
typedef enum { C2_TYPE_CIRCLE=0, C2_TYPE_AABB=1, C2_TYPE_CAPSULE=2 } C2_TYPE;
struct c2v       { float x, y; };            /*  8 bytes, align 4 */
struct c2Circle  { c2v p; float r; };        /* 12 bytes, align 4 */
struct c2AABB    { c2v min, max; };          /* 16 bytes, align 4 */
struct c2Capsule { c2v a, b; float r; };     /* 20 bytes, align 4 */
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** — there is exactly
one build configuration (default = no features). `--no-default-features` is
therefore equivalent to the default build; Phase D's feature-combination sweep
collapses to a single combination, and is still executed explicitly.

## Caveat: the C's NaN payloads are optimisation-level dependent

x86 `mulss`/`addss dst, src` return `quiet(dst)` when `dst` is NaN and
`quiet(src)` otherwise, so the *payload and sign* of a NaN result depend on
which operand the C compiler put in the destination register. Verified by
compiling `lib.c` at four levels:

| function | `-O0` (the documented build) | `-O1` / `-O2` | `-O3` |
|----------|------------------------------|---------------|-------|
| `c2Dot` x product | `dst = a.x` | `dst = a.x` | `dst = a.x` |
| `c2Dot` y product | `dst = b.y` | `dst = a.y` | `dst = a.y` |
| `c2Dot` sum | `dst = y term` | `dst = x term` | `dst = x term` |
| `c2Mulvs` x | `dst = a.x` | `dst = a.x` | `mulps`, `dst = a` |
| `c2Mulvs` y | `dst = a.y` | `dst = b` | `mulps`, `dst = a` |

The C therefore **disagrees with itself** about NaN payloads across optimisation
levels; no single translation can match all of them. `c_src/CMakeLists.txt` sets
no `CMAKE_BUILD_TYPE`, so the documented build command produces an unoptimised
library (confirmed: the built `.so`'s `c2Dot` has the `-O0` prologue and stack
spills). The Rust pins its SSE operand order to that build with inline `asm!`,
and the differential tests load exactly that `.so`, so the comparison is against
the real ground truth rather than a guess.

Every other result — all `int` predicates and all non-NaN float results — is
optimisation-independent, so this caveat only concerns NaN payload bits.
