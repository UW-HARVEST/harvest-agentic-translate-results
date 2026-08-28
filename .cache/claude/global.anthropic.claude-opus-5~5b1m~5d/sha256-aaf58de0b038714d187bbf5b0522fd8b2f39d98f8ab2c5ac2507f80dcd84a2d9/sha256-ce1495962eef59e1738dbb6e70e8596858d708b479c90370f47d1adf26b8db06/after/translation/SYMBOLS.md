# SYMBOLS.md — exported-symbol parity (C `.so` vs Rust `.so`)

Derived mechanically from `nm -D`:

```sh
nm -D --defined-only c_src/build/libharvest-work-huLMrZ.so   | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libcapsule_lib.so | awk '$2=="T"{print $3}' | sort
```

* C `.so` exported (`T`) symbols: **38**
* Rust `.so` exported (`T`) symbols: **38**
* `comm -3` diff size: **0** (exact parity, both directions)

There is a single C translation unit (`c_src/src/lib.c`, 647 lines) and a single
Rust module (`translation/src/lib.rs`).  No C module was skipped by the
translation, so no missing implementation had to be written; every symbol below
has a real Rust body (no stubs, no `unimplemented!()`).

| # | symbol | C `.so` | Rust `.so` | C def (lib.c) | Rust def (lib.rs) |
|---|--------|:-------:|:----------:|---------------|-------------------|
| 1 | `c22` | ✅ | ✅ | L186 | L360 |
| 2 | `c23` | ✅ | ✅ | L208 | L383 |
| 3 | `c2AABBtoAABB` | ✅ | ✅ | L519 | L769 |
| 4 | `c2AABBtoCapsule` | ✅ | ✅ | L527 | L778 |
| 5 | `c2Add` | ✅ | ✅ | L176 | L347 |
| 6 | `c2BBVerts` | ✅ | ✅ | L106 | L271 |
| 7 | `c2CCW90` | ✅ | ✅ | L275 | L454 |
| 8 | `c2CapsuletoCapsule` | ✅ | ✅ | L533 | L803 |
| 9 | `c2CircletoAABB` | ✅ | ✅ | L547 | L837 |
| 10 | `c2CircletoCapsule` | ✅ | ✅ | L555 | L846 |
| 11 | `c2CircletoCircle` | ✅ | ✅ | L539 | L828 |
| 12 | `c2Clampv` | ✅ | ✅ | L72 | L231 |
| 13 | `c2Collided` | ✅ | ✅ | L576 | L868 |
| 14 | `c2D` | ✅ | ✅ | L282 | L462 |
| 15 | `c2Det2` | ✅ | ✅ | L156 | L317 |
| 16 | `c2Div` | ✅ | ✅ | L338 | L535 |
| 17 | `c2Dot` | ✅ | ✅ | L82 | L243 |
| 18 | `c2GJK` | ✅ | ✅ | L363 | L571 |
| 19 | `c2GJKSimplexMetric` | ✅ | ✅ | L160 | L325 |
| 20 | `c2L` | ✅ | ✅ | L346 | L545 |
| 21 | `c2Len` | ✅ | ✅ | L152 | L310 |
| 22 | `c2MakeProxy` | ✅ | ✅ | L113 | L279 |
| 23 | `c2Maxv` | ✅ | ✅ | L62 | L215 |
| 24 | `c2Minv` | ✅ | ✅ | L67 | L223 |
| 25 | `c2Mulrv` | ✅ | ✅ | L172 | L338 |
| 26 | `c2MulrvT` | ✅ | ✅ | L359 | L558 |
| 27 | `c2Mulvs` | ✅ | ✅ | L56 | L208 |
| 28 | `c2Mulxv` | ✅ | ✅ | L182 | L355 |
| 29 | `c2Neg` | ✅ | ✅ | L264 | L440 |
| 30 | `c2Norm` | ✅ | ✅ | L342 | L540 |
| 31 | `c2RotIdentity` | ✅ | ✅ | L86 | L251 |
| 32 | `c2Skew` | ✅ | ✅ | L268 | L446 |
| 33 | `c2Sub` | ✅ | ✅ | L76 | L236 |
| 34 | `c2Support` | ✅ | ✅ | L298 | L478 |
| 35 | `c2V` | ✅ | ✅ | L49 | L200 |
| 36 | `c2Witness` | ✅ | ✅ | L311 | L494 |
| 37 | `c2xIdentity` | ✅ | ✅ | L93 | L259 |
| 38 | `capsule` | ✅ | ✅ | L619 | L929 |

## Symbols missing from the Rust `.so`

**None.** `comm -23 c.txt r.txt` is empty.

## Extra symbols in the Rust `.so`

**None** among `T` (global text) symbols.  `comm -13 c.txt r.txt` is empty.

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|-----------------------------|
| C `.so` | none (only `sqrtf@GLIBC`, `__cxa_finalize@GLIBC`, weak `_ITM_*`/`__gmon_start__`) |
| Rust `.so` | none — all imports are glibc (`malloc`, `memcpy`, `open64`, …) or the platform unwinder (`_Unwind_*@GCC_*`) pulled in by `std` |

Checklist item **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust** → ✅
