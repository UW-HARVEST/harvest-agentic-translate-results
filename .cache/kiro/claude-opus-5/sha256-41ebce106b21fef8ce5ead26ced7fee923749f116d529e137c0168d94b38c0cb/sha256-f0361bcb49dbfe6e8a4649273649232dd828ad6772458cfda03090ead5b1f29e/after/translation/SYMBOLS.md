# SYMBOLS.md — Phase A / Phase D symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-C1Lpne.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libpoly_ray_lib.so | awk '$2=="T"{print $3}' | sort > /tmp/rust_syms.txt
diff /tmp/c_syms.txt /tmp/rust_syms.txt
```

C `.so`: `c_src/build/libharvest-work-C1Lpne.so`
Rust `.so`: `translation/target/release/libpoly_ray_lib.so`

## Exported symbol table (28 in C, 28 in Rust, diff empty)

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|--------------|-------|
| 1 | `c2AABBtoAABB` | yes | yes | |
| 2 | `c2AABBtoPoint` | yes | yes | |
| 3 | `c2Absv` | yes | yes | |
| 4 | `c2Add` | yes | yes | |
| 5 | `c2CCW90` | yes | yes | |
| 6 | `c2CastRay` | yes | yes | enum dispatch |
| 7 | `c2CircleToPoint` | yes | yes | |
| 8 | `c2Div` | yes | yes | |
| 9 | `c2Dot` | yes | yes | |
| 10 | `c2Len` | yes | yes | |
| 11 | `c2Maxv` | yes | yes | |
| 12 | `c2Minv` | yes | yes | |
| 13 | `c2MulmvT` | yes | yes | |
| 14 | `c2Mulrv` | yes | yes | |
| 15 | `c2MulrvT` | yes | yes | |
| 16 | `c2Mulvs` | yes | yes | |
| 17 | `c2MulxvT` | yes | yes | |
| 18 | `c2Norm` | yes | yes | |
| 19 | `c2RaytoAABB` | yes | yes | |
| 20 | `c2RaytoCapsule` | yes | yes | |
| 21 | `c2RaytoCircle` | yes | yes | |
| 22 | `c2RaytoPoly` | yes | yes | |
| 23 | `c2RotIdentity` | yes | yes | |
| 24 | `c2Skew` | yes | yes | |
| 25 | `c2Sub` | yes | yes | |
| 26 | `c2V` | yes | yes | |
| 27 | `c2xIdentity` | yes | yes | |
| 28 | `poly_ray` | yes | yes | driver, declared in `include/lib.h` |

## Symbols deliberately NOT exported by either side

These are `static inline` in `src/lib.c`, so the C compiler gives them no
external linkage. The Rust side matches by keeping them private `fn`s.

| C symbol | reason |
|----------|--------|
| `c2SignedDistPointToPlane_OneDimensional` | `static inline` |
| `c2RayToPlane_OneDimensional` | `static inline` |

## Result

`diff /tmp/c_syms.txt /tmp/rust_syms.txt` → **empty**. 0 missing symbols.
0 undefined non-libc symbols in the Rust `.so` (`nm -D -u` shows only the
Rust/libc runtime imports: `memcpy`, `sqrtf`, unwinder, etc.).

No module of `c_src/src/lib.c` was skipped: the file defines exactly 28
externally-linked functions and all 28 are translated in
`translation/src/lib.rs`.

## Phase D — symbol parity under every configuration

`translation/run_all.sh` rebuilds the C reference, then for each cargo profile ×
feature combination rebuilds the Rust `cdylib`, diffs `nm -D`, and runs the full
differential suite. Feature combinations are enumerated from
`cargo read-manifest`, not hard-coded.

```
PASS  symbol parity (28 symbols) [debug / --default]
PASS  tests [debug / --default]
PASS  symbol parity (28 symbols) [debug / --no-default-features]
PASS  tests [debug / --no-default-features]
PASS  symbol parity (28 symbols) [release / --default]
PASS  tests [release / --default]
PASS  symbol parity (28 symbols) [release / --no-default-features]
PASS  tests [release / --no-default-features]
ALL CHECKS PASSED
```

`Cargo.toml` declares no `[features]` section, so the default and
`--no-default-features` sets are the only two configurations that exist and
they are identical. Both are run anyway.

## Completeness cross-check

`c_src/src/lib.c` defines 30 functions: 28 with external linkage and 2 marked
`static inline`. The Rust file carries 28 `#[unsafe(no_mangle)] extern "C"`
exports and 2 private `fn`s — so no C module or function was skipped, and no
symbol is a stub:

```sh
grep -c 'static inline' c_src/src/lib.c                        # 2
grep -c '#\[unsafe(no_mangle)\]' translation/src/lib.rs        # 28
grep -nE 'unimplemented!|todo!|panic!|unreachable!' src/lib.rs # (none)
```
