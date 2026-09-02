# SYMBOLS.md — Exported symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-1GVLhD.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libgen_ray_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # -> empty
```

The C library is a single translation unit (`c_src/src/lib.c`, 340 lines). No C
source file was skipped by the translation: every non-`static` function in
`lib.c` has a corresponding `#[unsafe(no_mangle)] pub extern "C"` item in
`translation/src/lib.rs`. The two `static inline` helpers are deliberately NOT
exported (the C compiler does not export them either — see the last section).

## Exported symbols (22) — C vs Rust

| # | symbol | C `.so` | Rust `.so` | C signature (from `src/lib.c`) |
|---|--------|---------|-----------|--------------------------------|
| 1 | `c2V`             | yes | yes | `c2v c2V(float, float)` |
| 2 | `c2Dot`           | yes | yes | `float c2Dot(c2v, c2v)` |
| 3 | `c2Len`           | yes | yes | `float c2Len(c2v)` |
| 4 | `c2Add`           | yes | yes | `c2v c2Add(c2v, c2v)` |
| 5 | `c2Sub`           | yes | yes | `c2v c2Sub(c2v, c2v)` |
| 6 | `c2Mulvs`         | yes | yes | `c2v c2Mulvs(c2v, float)` |
| 7 | `c2Div`           | yes | yes | `c2v c2Div(c2v, float)` |
| 8 | `c2Norm`          | yes | yes | `c2v c2Norm(c2v)` |
| 9 | `c2Minv`          | yes | yes | `c2v c2Minv(c2v, c2v)` |
| 10 | `c2Maxv`         | yes | yes | `c2v c2Maxv(c2v, c2v)` |
| 11 | `c2Skew`         | yes | yes | `c2v c2Skew(c2v)` |
| 12 | `c2Absv`         | yes | yes | `c2v c2Absv(c2v)` |
| 13 | `c2CCW90`        | yes | yes | `c2v c2CCW90(c2v)` |
| 14 | `c2MulmvT`       | yes | yes | `c2v c2MulmvT(c2m, c2v)` |
| 15 | `c2RaytoCircle`  | yes | yes | `int c2RaytoCircle(c2Ray, c2Circle, c2Raycast*)` |
| 16 | `c2AABBtoAABB`   | yes | yes | `int c2AABBtoAABB(c2AABB, c2AABB)` |
| 17 | `c2RaytoAABB`    | yes | yes | `int c2RaytoAABB(c2Ray, c2AABB, c2Raycast*)` |
| 18 | `c2AABBtoPoint`  | yes | yes | `int c2AABBtoPoint(c2AABB, c2v)` |
| 19 | `c2CircleToPoint`| yes | yes | `int c2CircleToPoint(c2Circle, c2v)` |
| 20 | `c2RaytoCapsule` | yes | yes | `int c2RaytoCapsule(c2Ray, c2Capsule, c2Raycast*)` |
| 21 | `c2CastRay`      | yes | yes | `int c2CastRay(c2Ray, const void*, C2_TYPE, c2Raycast*)` |
| 22 | `gen_ray`        | yes | yes | `int gen_ray(c2Raycast*, c2Raycast*, c2Raycast*, 16x float)` |

**Symbol diff: EMPTY.** No symbol required a new `#[no_mangle]` wrapper and no
C module was left untranslated. The Rust `.so` also exports **no extra**
dynamic symbols.

### Note on `c2CastRay`

The exported `c2CastRay` is a `#[unsafe(naked)]` shim (`cmp esi, 2` / `ja` /
`jmp <impl>` / `ret`) that reproduces the C's `switch`-with-no-`default`
fall-through, where `%eax` is left untouched. The real dispatch body lives in a
private, name-mangled `c2CastRay_impl` that is **not** in the dynamic symbol
table, so the symbol count stays at exactly 22. See ERRORS.md row 34.

## Deliberately non-exported (matches C)

| C item | storage | reason not exported |
|--------|---------|---------------------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` | internal linkage in C; `nm -D` on the C `.so` does not list it |
| `c2RayToPlane_OneDimensional` | `static inline` | same |

Both are private `#[inline] fn` in Rust, so the Rust `.so` matches.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / `libgcc` unwinder
imports (`memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, …) pulled in by the
Rust standard library. **0 missing/undefined non-libc symbols.**

## ABI notes verified against the C disassembly

`objdump -d` on `c2CastRay` confirms the System V AMD64 classification the Rust
`extern "C"` declarations rely on:

* `c2v` (8 B, all float) — one SSE eightbyte → `xmm0`; also returned in `xmm0`.
* `c2Circle` (12 B, all float) → `xmm0` + `xmm1` (disasm: `movq %r8,%xmm0`, `movss 0x8(%rax),%xmm1`).
* `c2AABB` (16 B, all float) → `xmm0` + `xmm1`.
* `c2Ray` (20 B) and `c2Capsule` (20 B) — over 16 B → MEMORY, passed on the stack.
* `c2m` (16 B, all float) → `xmm0` + `xmm1`.
* `C2_TYPE` enum parameter → passed as `int` in `esi`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. `cargo check
--no-default-features` and the default build are therefore the complete
cross-product; both are exercised by `tests/feature_matrix.sh`.
