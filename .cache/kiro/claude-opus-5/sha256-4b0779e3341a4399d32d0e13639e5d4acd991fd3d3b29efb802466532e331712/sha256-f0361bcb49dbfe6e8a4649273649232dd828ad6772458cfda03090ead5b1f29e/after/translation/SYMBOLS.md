# SYMBOLS.md — Phase A symbol surface

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-LRoEhn.so   | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libreverse_collide_lib.so | awk '$2=="T"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

Result: **C exports 38, Rust exports 38, symbol diff is EMPTY in both directions.**

`c_src/include/lib.h` declares only `reverse_collide`; the other 37 symbols have
external linkage in `src/lib.c` (no `static`), so they are all part of the ABI
surface and all must be (and are) exported by the Rust `cdylib`.

| # | symbol | C signature | Rust export | present |
|---|--------|-------------|-------------|---------|
| 1 | `c2V` | `c2v(float,float)` | `#[unsafe(no_mangle)] extern "C"` | yes |
| 2 | `c2Mulvs` | `c2v(c2v,float)` | ✓ | yes |
| 3 | `c2Maxv` | `c2v(c2v,c2v)` | ✓ | yes |
| 4 | `c2Minv` | `c2v(c2v,c2v)` | ✓ | yes |
| 5 | `c2Clampv` | `c2v(c2v,c2v,c2v)` | ✓ | yes |
| 6 | `c2Sub` | `c2v(c2v,c2v)` | ✓ | yes |
| 7 | `c2Dot` | `float(c2v,c2v)` | ✓ | yes |
| 8 | `c2RotIdentity` | `c2r(void)` | ✓ | yes |
| 9 | `c2xIdentity` | `c2x(void)` | ✓ | yes |
| 10 | `c2BBVerts` | `void(c2v*,c2AABB*)` | ✓ | yes |
| 11 | `c2MakeProxy` | `void(const void*,C2_TYPE,c2Proxy*)` | ✓ | yes |
| 12 | `c2Len` | `float(c2v)` | ✓ | yes |
| 13 | `c2Det2` | `float(c2v,c2v)` | ✓ | yes |
| 14 | `c2GJKSimplexMetric` | `float(c2Simplex*)` | ✓ | yes |
| 15 | `c2Mulrv` | `c2v(c2r,c2v)` | ✓ | yes |
| 16 | `c2Add` | `c2v(c2v,c2v)` | ✓ | yes |
| 17 | `c2Mulxv` | `c2v(c2x,c2v)` | ✓ | yes |
| 18 | `c22` | `void(c2Simplex*)` | ✓ | yes |
| 19 | `c23` | `void(c2Simplex*)` | ✓ | yes |
| 20 | `c2Neg` | `c2v(c2v)` | ✓ | yes |
| 21 | `c2Skew` | `c2v(c2v)` | ✓ | yes |
| 22 | `c2CCW90` | `c2v(c2v)` | ✓ | yes |
| 23 | `c2D` | `c2v(c2Simplex*)` | ✓ | yes |
| 24 | `c2Support` | `int(const c2v*,int,c2v)` | ✓ | yes |
| 25 | `c2Witness` | `void(c2Simplex*,c2v*,c2v*)` | ✓ | yes |
| 26 | `c2Div` | `c2v(c2v,float)` | ✓ | yes |
| 27 | `c2Norm` | `c2v(c2v)` | ✓ | yes |
| 28 | `c2L` | `c2v(c2Simplex*)` | ✓ | yes |
| 29 | `c2MulrvT` | `c2v(c2r,c2v)` | ✓ | yes |
| 30 | `c2GJK` | `float(const void*,C2_TYPE,const c2x*,const void*,C2_TYPE,const c2x*,c2v*,c2v*,int,int*,c2GJKCache*)` | ✓ | yes |
| 31 | `c2AABBtoAABB` | `int(c2AABB,c2AABB)` | ✓ | yes |
| 32 | `c2AABBtoCapsule` | `int(c2AABB,c2Capsule)` | ✓ | yes |
| 33 | `c2CapsuletoCapsule` | `int(c2Capsule,c2Capsule)` | ✓ | yes |
| 34 | `c2CircletoCircle` | `int(c2Circle,c2Circle)` | ✓ | yes |
| 35 | `c2CircletoAABB` | `int(c2Circle,c2AABB)` | ✓ | yes |
| 36 | `c2CircletoCapsule` | `int(c2Circle,c2Capsule)` | ✓ | yes |
| 37 | `c2Collided` | `int(const void*,C2_TYPE,const void*,C2_TYPE)` | ✓ | yes |
| 38 | `reverse_collide` | `int(float,float,float)` | ✓ | yes |

No symbol required translating a missing module; the whole of `src/lib.c` is
present in `translation/src/lib.rs`. No stubs, no `unimplemented!()`.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only build
configuration is the default one. Phase D's "every feature combination" is
therefore satisfied by the default build (verified by
`cargo check --no-default-features` also succeeding).

## Completion gate

Driven by `./verify_all.sh` (run from `translation/`), which builds the C `.so`,
enumerates the cargo feature combinations from `Cargo.toml`, and for each
configuration × build profile re-checks symbol parity and runs the whole test
suite against that specific `.so`.

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (38 of 38) and `ldd -r`
      reports no unresolvable symbols. No `c2*` / `reverse_collide` symbol is
      referenced-but-undefined, i.e. no module was skipped.
- [x] Phase B: every one of the 66 `CONFIGS.md` rows passes across randomized
      inputs (fixed seeds; `tests/phase_b_math.rs`, `phase_b_gjk.rs`,
      `phase_b_collide.rs`).
- [x] No binary/driver target exists in either build (`c_src/CMakeLists.txt`
      declares `add_library(... SHARED ...)` only, no `add_executable`; the crate
      is `crate-type = ["cdylib"]` only), so the stdout-comparison gate is N/A.
      `verify_all.sh` asserts this rather than assuming it.
- [x] Phase C: every one of the 40 `ERRORS.md` rows has a passing error-path
      differential test (`tests/phase_c_errors.rs`), plus a generic FFI boundary
      sweep over null pointers, zero/oversized counts, and out-of-range enum
      values.
- [x] All of the above hold under every configuration: 56 tests × {release,
      debug} × {default, `--no-default-features`} = 4 configurations, all green.

Beyond the tables, `tests/phase_d_fuzz.rs` and `tests/hunt_cap.rs` add ~2.6M
randomized differential calls over arbitrary float bit patterns (NaN, infinities,
subnormals, `FLT_MAX`) with different seeds, and `tests/phase_d_layout.rs` asserts
the `#[repr(C)]` layouts match the C ABI — in particular that `c2Simplex`'s
`[c2sv; 4]` occupies the same bytes as the C's four named `a`/`b`/`c`/`d` members,
which is what makes the translation's pointer walking equivalent to `&s.a`.

All tests load BOTH libraries with `libloading` and call only exported symbols;
the Rust crate is never linked or called directly, so the `#[no_mangle]` wrappers
and the FFI struct passing/returning conventions are themselves under test.

Two input classes are excluded from bit-exact comparison, each with recorded
evidence rather than assumption — see the "Deliberately excluded" section of
`ERRORS.md`: (1) `c2GJK` with an out-of-range `C2_TYPE`, which reads an
uninitialised `c2Proxy` and demonstrably returns different values for identical
arguments; and (2) the sign/payload of a NaN produced from two NaN operands,
which IEEE 754 leaves unspecified and which LLVM will not let Rust source steer.
