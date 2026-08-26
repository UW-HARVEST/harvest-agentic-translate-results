# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  : `c_src/build/libtranslated_rust.so` (gcc 11.5.0, x86_64, default CMake flags = `-O0`)
* Rust: `target/release/libgjk_cache_lib.so` (`crate-type = ["cdylib"]`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '$2=="T"{print $3}' | sort > /tmp/c_syms
nm -D --defined-only target/release/libgjk_cache_lib.so | awk '$2=="T"{print $3}' | sort > /tmp/r_syms
comm -23 /tmp/c_syms /tmp/r_syms   # must print nothing
```

`c_src/src/lib.c` is a single translation unit with **no** `static` functions, so
every function in the file has external linkage and is therefore part of the
public ABI, even though `include/lib.h` only declares `gjk_cache`. All 31 are
re-exported from Rust with `#[unsafe(no_mangle)] extern "C"`.

## Symbol parity table (31 / 31)

| C symbol | type | in Rust `.so` |
|----------|------|---------------|
| `c22` | T | present |
| `c23` | T | present |
| `c2Add` | T | present |
| `c2BBVerts` | T | present |
| `c2CCW90` | T | present |
| `c2Clampv` | T | present |
| `c2D` | T | present |
| `c2Det2` | T | present |
| `c2Div` | T | present |
| `c2Dot` | T | present |
| `c2GJK` | T | present |
| `c2GJKSimplexMetric` | T | present |
| `c2L` | T | present |
| `c2Len` | T | present |
| `c2MakeProxy` | T | present |
| `c2Maxv` | T | present |
| `c2Minv` | T | present |
| `c2Mulrv` | T | present |
| `c2MulrvT` | T | present |
| `c2Mulvs` | T | present |
| `c2Mulxv` | T | present |
| `c2Neg` | T | present |
| `c2Norm` | T | present |
| `c2RotIdentity` | T | present |
| `c2Skew` | T | present |
| `c2Sub` | T | present |
| `c2Support` | T | present |
| `c2V` | T | present |
| `c2Witness` | T | present |
| `c2xIdentity` | T | present |
| `gjk_cache` | T | present |

**Verified for both the `dev` and the `release` build. Missing from Rust: 0.** No module of the C source was skipped: `lib.c` is the
only C source file in `CMakeLists.txt` (`add_library(... SHARED src/lib.c)`) and
every one of its functions is translated in `src/lib.rs`. No stubs, no
`unimplemented!()`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
ld.so imports (`malloc`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …). There are
**0 undefined non-libc symbols**, i.e. nothing the Rust library expects the C
library to provide.

## ABI notes checked while establishing parity

| item | C | Rust | ok |
|------|---|------|----|
| `c2v` | `struct { float x, y; }`, 8 B, returned in XMM0 | `#[repr(C)]`, 8 B | yes |
| `c2r`, `c2x`, `c2Circle`, `c2AABB`, `c2Capsule` | float PODs | `#[repr(C)]` | yes |
| `c2GJKCache` | `{ float; int; int[3]; int[3]; float; }` = 36 B (offsets 0/4/8/20/32) | `#[repr(C)]` 36 B, same offsets | yes |
| `c2Proxy` | `{ float; int; c2v[8]; }` = 72 B | `#[repr(C)]` 72 B | yes |
| `c2Simplex` | `{ c2sv a,b,c,d; float div; int count; }` = **152 B**, members at 0/36/72/108, `div` 144, `count` 148 | `verts: [c2sv;4]` + `div` + `count` = 152 B, same offsets | yes |
| `c2sv` | `{ c2v sA,sB,p; float u; int iA,iB; }` = **36 B** (offsets 0/8/16/24/28/32) | `#[repr(C)]` 36 B, same offsets | yes |
| `C2_TYPE` enum | passed as `int` | `pub type C2_TYPE = c_int` | yes |
| `char reverse` | `char` (signed on x86_64) | `c_char` | yes |

The C idiom `c2sv* verts = &s->a; verts[i]` maps onto the Rust `verts: [c2sv; 4]`
array field, which has byte-identical layout to the four named members.
`assert_layout` in `tests/common/mod.rs` pins every size/offset, and the values
were cross-checked against gcc `sizeof`/`offsetof` output rather than assumed:

```
c2v 8   c2r 8   c2x 16   c2Circle 12   c2AABB 16   c2Capsule 20
c2GJKCache 36 (metric 0 count 4 iA 8 iB 20 div 32)
c2Proxy    72 (radius 0 count 4 verts 8)
c2sv       36 (sA 0 sB 8 p 16 u 24 iA 28 iB 32)
c2Simplex 152 (a 0 b 36 c 72 d 108 div 144 count 148)
```

(The first draft of the test harness guessed `c2sv` = 28 B and `c2Simplex` =
120 B; the gcc dump corrected both. `src/lib.rs` was already right.)
