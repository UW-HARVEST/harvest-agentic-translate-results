# FIXES.md — divergences found and corrected in the Rust translation

All nine bugs below were found by the differential tests and fixed in
`src/lib.rs`. **Nothing in `c_src/` was modified.**

Every one is the same *class* of bug: the Rust chose the wrong operand as the
SSE **destination**. `mulss dst, src` / `addss` / `subss` return the *quieted
destination* when the destination is NaN, and only otherwise the quieted source
— so `mulss(a, b)` and `mulss(b, a)` return **different NaN payloads** even
though they agree on every non-NaN input. The C is compiled at `-O0`
(`CMakeLists.txt` sets no optimisation flags), so its operand roles are fixed
and were read directly out of `objdump -d` on the C `.so`.

The pre-existing translation already had `addss`/`subss`/`mulss`/`divss` helpers
modelling this; the operand *order* passed to them was simply wrong in nine
places.

| # | function | C `-O0` codegen | was (Rust) | now (Rust) |
|---|----------|-----------------|------------|------------|
| 1 | `c2Dot` | `mulss %xmm0,%xmm1` (dst=`a.x`), `mulss %xmm2,%xmm0` (dst=**`b.y`**), `addss %xmm1,%xmm0` (dst=**y-term**) | `addss(mulss(a.x,b.x), mulss(a.y,b.y))` | `addss(mulss(b.y,a.y), mulss(a.x,b.x))` |
| 2 | `c2CircletoCircle` | `movss A.r,%xmm1; movss B.r,%xmm0; addss %xmm1,%xmm0` → dst=**`B.r`** | `addss(A.r, B.r)` | `addss(B.r, A.r)` |
| 3 | `f9` / `lm_dot2` | same shape as `c2Dot`; all five dot products go through it with the C's own argument order | five hand-inlined, individually-wrong expressions | one shared private `fn lm_dot2` matching the C, called as `lm_dot2(v0,v0)`, `(v0,v1)`, `(v0,v2)`, `(v1,v1)`, `(v1,v2)` |
| 4 | `f9` denominator | `movss dot00,%xmm0; mulss dot11,%xmm1` → dst=**`dot00`** | `mulss(dot11, dot00)` | `mulss(dot00, dot11)` |
| 5 | `f9` `u` | `movss dot01,%xmm1; mulss dot12,%xmm1` → dst=**`dot01`** | `mulss(dot12, dot01)` | `mulss(dot01, dot12)` |
| 6 | `f9` `v` | `mulss` dsts are **`dot00`** and **`dot01`** | `mulss(dot12,dot00)`, `mulss(dot02,dot01)` | `mulss(dot00,dot12)`, `mulss(dot01,dot02)` |
| 7 | `f11` `m` | GCC folds `1.0f *` away entirely — there is no `mulss` by 1.0 at all | `mulss(1.0, subss(l, mulss(c,0.5)))` | `subss(l, mulss(c, 0.5))` |
| 8 | `f11` arms 3–6 | every `x + m` / `c + m` store is `movss x,%xmm0; addss m,%xmm0` → dst is **always `x`/`c`**, never `m` | `addss(m, x)`, `addss(m, c)` in four places | `addss(x, m)`, `addss(c, m)` |
| 9 | `f13` `h` scaling | `movss h,%xmm1; movss 60.0f,%xmm0; mulss %xmm1,%xmm0` → dst=**`60.0f`**; likewise `addss` dst=**`360.0f`** | `mulss(h, 60.0)`, `addss(h, 360.0)` | `mulss(60.0, h)`, `addss(360.0, h)` |

## Reproducer for the first one

```
c2Dot({x: 0x7FC00000, y: 0x7FC00000}, {x: 0xFFC00000, y: 0xFFC00000})
  C    -> 0xFFC00000   (negative qNaN: the y-term's mulss dst is b.y)
  Rust -> 0x7FC00000   (positive qNaN)   [before fix 1]
```

## Things that were already correct and were left alone

Verified against the disassembly, not assumed:

* `c2Maxv` / `c2Minv` / `c2Clampv` — `comiss` + branch, no arithmetic; NaN makes
  the comparison unordered so `jbe` is taken and `b` wins, which
  `if a.x > b.x { a.x } else { b.x }` reproduces exactly.
* `c2Sub` / `lm_sub2` — `subss` dst is `a`, matching the Rust.
* `c2CircletoAABB` — `mulss %xmm1,%xmm0` with both operands `A.r`, so no
  ambiguity.
* `f3` — pure integer; `wrapping_*` everywhere matches the `-O0` `idiv`/`imul`.
  No path can reach `INT_MIN / -1` (which would `SIGFPE`), so `wrapping_div`
  being non-trapping is unobservable.
* `f4` — `subsd` dst is the punned double; state update is verbatim.
* `f5` — pure bit twiddling.
* `f7` — GCC factors `blocksize*bitdepth` out of the first two terms; identical
  modulo 2^32, so the Rust's separate `wrapping_mul`s agree. `shr $3` == `/ 8`
  on `u32`.
* `f10` — `shr $0xa,%ax`, 32-bit `add` (hence `wrapping_add`), table indices max
  out at exactly `m__mantissa[2047]`.
* `f12` — every `mulss`/`subss` dst already matched; the `switch` is
  `cmpl $0x4,i; ja` (an **unsigned** compare), so negative `i` correctly falls
  to `default`, which `match i { 0..=4 => …, _ => … }` reproduces.
* `f13` min/max ternaries — `comiss` + `jbe`, so a NaN never displaces the
  incumbent; the `if min < g { min } else { g }` form is faithful.
* `agglom` — the accumulator `addsd`s have the *value* as destination and `ret`
  as source (the reverse of Rust's `ret += v`), but this is unobservable: the 13
  `isnan()` guards mean the value operand is never NaN, so at most one operand
  can be NaN and both orders select it.
* `f11`/`f12`/`f13` pointer wrappers — the C loads all three `src` floats into
  locals before any store, so `dest == src` aliasing is benign; the Rust
  wrappers do the same load-then-store.

## Two `ERRORS.md` claims that the C disproved

Written from reading the source, then corrected after running the C:

* `f3(INT_MIN, -1)` is **`INT_MIN`**, not `1` — `q = ((-(v1-v2))/(-v2)) + 1`
  computes `INT_MAX + 1`, which overflows and wraps.
* `f3` is **not** floored division when `v1 >= 0 && v2 < 0`: there
  `r = v1 % (-v2)` is never negative, so the `r < 0` fix-up can never fire and
  the function truncates toward zero instead. `f3(7, -2) == -3` (not `-4`);
  `f3(30575, -412) == -74` (not `-75`). The Rust already reproduced this; the
  documentation was what needed fixing.
