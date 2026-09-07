# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-eZVnmh.so | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libpoly_ray_lib.so | awk '$2=="T"{print $3}' | sort
```

C `.so` exports **28** `T` symbols. Rust `.so` exports **28** `T` symbols.
`comm -23` (missing from Rust) → **empty**. `comm -13` (extra in Rust) → **empty**.

## Table

| # | symbol | C source (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` | note |
|---|--------|------------------------------|-----------|---------------|------|
| 1 | `c2V` | L49 | T | T | |
| 2 | `c2Dot` | L56 | T | T | |
| 3 | `c2Len` | L60 | T | T | uses `sqrtf` |
| 4 | `c2Add` | L64 | T | T | |
| 5 | `c2Sub` | L70 | T | T | |
| 6 | `c2Mulvs` | L76 | T | T | |
| 7 | `c2Div` | L82 | T | T | reciprocal-then-multiply |
| 8 | `c2Norm` | L86 | T | T | |
| 9 | `c2Minv` | L90 | T | T | ternary min, not `fminf` |
| 10 | `c2Maxv` | L95 | T | T | ternary max, not `fmaxf` |
| 11 | `c2Skew` | L100 | T | T | |
| 12 | `c2Absv` | L107 | T | T | ternary abs, not `fabsf` |
| 13 | `c2RaytoCircle` | L111 | T | T | out-param |
| 14 | `c2AABBtoAABB` | L129 | T | T | |
| 15 | `c2RaytoAABB` | L156 | T | T | out-param |
| 16 | `c2CCW90` | L220 | T | T | |
| 17 | `c2MulmvT` | L227 | T | T | |
| 18 | `c2AABBtoPoint` | L234 | T | T | |
| 19 | `c2CircleToPoint` | L242 | T | T | |
| 20 | `c2RaytoCapsule` | L248 | T | T | out-param |
| 21 | `c2RotIdentity` | L311 | T | T | |
| 22 | `c2xIdentity` | L318 | T | T | |
| 23 | `c2Mulrv` | L325 | T | T | |
| 24 | `c2MulrvT` | L329 | T | T | |
| 25 | `c2MulxvT` | L333 | T | T | |
| 26 | `c2RaytoPoly` | L337 | T | T | nullable `bx` |
| 27 | `c2CastRay` | L367 | T | T | enum dispatch |
| 28 | `poly_ray` | L381 | T | T | public entry (`include/lib.h`) |

## Not exported (correctly absent from both)

| C symbol | reason |
|----------|--------|
| `c2SignedDistPointToPlane_OneDimensional` (L137) | `static inline` — no external linkage in C, private `fn` in Rust |
| `c2RayToPlane_OneDimensional` (L142) | `static inline` — same |

## Undefined-symbol check

C `.so` undefined non-libc: none (`sqrtf`, `__cxa_finalize`, ITM/gmon stubs only).
Rust `.so` undefined: glibc (`memcpy`, `malloc`, `write`, …) + `_Unwind_*`/`__cxa_*`
runtime hooks only. **0 missing/undefined non-libc symbols.**

Note: `c2Len`/`c2RaytoCircle` in Rust call `f32::sqrt`, which rustc lowers to the
`sqrtss` instruction rather than a `sqrtf` PLT call. Same IEEE-754 result
(`sqrtf` is exactly-rounded and correctly-rounded per the standard), so this is
not a behavioural difference.

## Configuration surface

`translation/Cargo.toml` declares **no `[features]`** section and **no `[[bin]]`**
target — a single `cdylib` crate. Therefore "every feature combination" is the
single default configuration; there is also no driver binary whose stdout needs
comparing. Verified with `grep -n 'feature\|\[\[bin\]\]' translation/Cargo.toml`
→ no matches.

---

## Phase D — symbol parity result

Automated by `./verify.sh`, which rebuilds both libraries and diffs `nm -D` for
every feature combination and both optimisation levels:

```
=== combo=default profile=debug ===
PASS  [default/debug] symbol parity (28 symbols, diff empty)
PASS  [default/debug] no non-libc undefined symbols
PASS  [default/debug] tests (97 passed)

=== combo=default profile=release ===
PASS  [default/release] symbol parity (28 symbols, diff empty)
PASS  [default/release] no non-libc undefined symbols
PASS  [default/release] tests (97 passed)
```

No symbol was missing, so no `#[no_mangle]` wrapper had to be added and no C
module turned out to be untranslated. Nothing is stubbed or `unimplemented!()` —
`grep -rn 'unimplemented\|todo!\|panic!' src/` returns no matches.

Both optimisation levels are exercised because they are genuinely different
codegen: the `debug` `.so` is unoptimised Rust and the `release` `.so` is
`opt-level=3`, and LLVM is free to fold and reassociate float expressions
differently between them. Both match the C bit-for-bit.
