# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-2lMgJL.so

cd translation && cargo build --release
# -> translation/target/release/libspec_ray_lib.so
```

Diff command (this is the gate):

```sh
diff <(nm -D --defined-only c_src/build/*.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/*.so | awk '{print $3}' | sort)
```

Result: **empty diff — 22 symbols exported by C, all 22 exported by Rust.**

## Symbol table

`Rust impl` names the item in `translation/src/lib.rs` carrying
`#[unsafe(no_mangle)] pub extern "C"`.

| # | symbol | C signature | in C `.so` | in Rust `.so` | Rust impl |
|---|--------|-------------|:----------:|:-------------:|-----------|
| 1 | `c2V` | `c2v (float, float)` | T | T | `c2V` |
| 2 | `c2Dot` | `float (c2v, c2v)` | T | T | `c2Dot` |
| 3 | `c2Len` | `float (c2v)` | T | T | `c2Len` |
| 4 | `c2Add` | `c2v (c2v, c2v)` | T | T | `c2Add` |
| 5 | `c2Sub` | `c2v (c2v, c2v)` | T | T | `c2Sub` |
| 6 | `c2Mulvs` | `c2v (c2v, float)` | T | T | `c2Mulvs` |
| 7 | `c2Div` | `c2v (c2v, float)` | T | T | `c2Div` |
| 8 | `c2Norm` | `c2v (c2v)` | T | T | `c2Norm` |
| 9 | `c2Minv` | `c2v (c2v, c2v)` | T | T | `c2Minv` |
| 10 | `c2Maxv` | `c2v (c2v, c2v)` | T | T | `c2Maxv` |
| 11 | `c2Skew` | `c2v (c2v)` | T | T | `c2Skew` |
| 12 | `c2Absv` | `c2v (c2v)` | T | T | `c2Absv` |
| 13 | `c2CCW90` | `c2v (c2v)` | T | T | `c2CCW90` |
| 14 | `c2MulmvT` | `c2v (c2m, c2v)` | T | T | `c2MulmvT` |
| 15 | `c2AABBtoAABB` | `int (c2AABB, c2AABB)` | T | T | `c2AABBtoAABB` |
| 16 | `c2AABBtoPoint` | `int (c2AABB, c2v)` | T | T | `c2AABBtoPoint` |
| 17 | `c2CircleToPoint` | `int (c2Circle, c2v)` | T | T | `c2CircleToPoint` |
| 18 | `c2RaytoCircle` | `int (c2Ray, c2Circle, c2Raycast*)` | T | T | `c2RaytoCircle` |
| 19 | `c2RaytoAABB` | `int (c2Ray, c2AABB, c2Raycast*)` | T | T | `c2RaytoAABB` |
| 20 | `c2RaytoCapsule` | `int (c2Ray, c2Capsule, c2Raycast*)` | T | T | `c2RaytoCapsule` |
| 21 | `c2CastRay` | `int (c2Ray, const void*, C2_TYPE, c2Raycast*)` | T | T | `c2CastRay` |
| 22 | `spec_ray` | `int (c2Raycast*, float×7)` | T | T | `spec_ray` |

## Deliberately NOT exported

Two C functions are `static inline` and so have no external linkage in the C
`.so`. The Rust translation keeps them private (`fn`, no `#[no_mangle]`), which
is what keeps the symbol diff empty in *both* directions:

| C function | why not a symbol |
|------------|------------------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` |
| `c2RayToPlane_OneDimensional` | `static inline` |

One Rust-only helper also stays unexported:

| Rust item | why not a symbol |
|-----------|------------------|
| `c2CastRay_impl` | Private dispatch body. The public `c2CastRay` symbol is an `#[unsafe(naked)]` stub that does the C's `cmp esi, 2` / `ja` range check and then either tail-jumps here or returns with `%eax` untouched, reproducing the C's fall-off-the-end path (see `ERRORS.md` row 34). The name is Rust-mangled and has hidden visibility, so `nm -D` does not list it — confirmed by `nm -D --defined-only … \| grep impl` returning nothing. |

`c2v`, `c2Raycast`, `c2Circle`, `c2AABB`, `c2Capsule`, `c2Ray`, `c2m` and
`C2_TYPE` are types, not symbols; they contribute no `nm` entries. All are
`#[repr(C)]` in Rust with identical field order, so layout matches.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only translation/target/release/*.so` lists only libc /
libgcc-unwind imports (`memcpy`, `malloc`, `_Unwind_*`, `dl_iterate_phdr`, …)
pulled in by the Rust runtime's panic and backtrace machinery. **Zero
non-libc undefined symbols.** The C `.so` imports only `sqrtf` from libm; the
Rust side uses the `f32::sqrt` intrinsic (`sqrtss`), so no libm import appears
— that is an import-set difference, not a missing export, and does not affect
the export gate.

## Missing implementations found in Phase A

None. Every C function in `c_src/src/lib.c` has a real, behaviour-preserving
Rust body. No stubs, no `unimplemented!()`, no `todo!()`:

```sh
grep -c 'unimplemented!\|todo!\|panic!("not' translation/src/lib.rs   # -> 0
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, and
`grep -rn 'feature *=' translation/src/` matches nothing. There is exactly one
build configuration, so "every feature combination" collapses to the default.
Verified by script (`--no-default-features` and default both build and pass);
see the note at the end of `CONFIGS.md`.

## Result

Symbol diff is empty in both directions, for both the release- and debug-built
Rust `.so`, under both `default` and `--no-default-features`. Reproduce with
`scripts/verify_all.sh`, which gates on this diff before running any test.

No missing module or missing implementation was found: `c_src/src/lib.c` is a
single 321-line file and every function in it has a real Rust body. The only
change Phase A/D forced was to `c2CastRay`'s export form, not its presence —
see `ERRORS.md` row 34.
