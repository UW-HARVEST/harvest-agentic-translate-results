# ERRORS.md — error / rejection surface table

Derived mechanically by grepping `c_src/src/lib.c` for every rejection point:

```
grep -n 'return 0|return -1|return NULL|assert|default:|== 0|!= 0|0x7fffffff|abort|errno|RETURN_ERROR' c_src/src/lib.c
```

**Findings about the shape of this library's error surface** (stated explicitly
because it drives what the Phase C tests can assert):

* There is **no** error enum, no `RETURN_ERROR`-style macro, no `assert`, no
  `errno` use, no `abort`, and **no NULL checks anywhere**. `f2`, `f4`, `f11`,
  `f12`, `f13` dereference their pointer arguments unconditionally.
* All rejection is therefore one of: (a) an early `return 0`/sentinel-value
  return, (b) a `default:` fall-through in a `switch`, (c) an overflow-guard
  branch, or (d) a silent degenerate float result (`inf` / `NaN`) from an
  unguarded division.
* Consequently "same error" in Phase C means **same returned sentinel /
  same written bytes**, compared bit-for-bit — not "both crashed".

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| E1 | `f2` (lib.c:91-92) | `typeA == C2_TYPE_CIRCLE (0)` and `typeB` is any int with no valid variant (e.g. `2`, `3`, `0xFFFFFFFF`, `-1` as `int`) | returns `0`; `A`/`B` never dereferenced for `B` |
| E2 | `f2` (lib.c:101-102) | `typeA == C2_TYPE_AABB (1)` and `typeB` out-of-range enum (`2`, `7`, `0x80000000`, `0xFFFFFFFF`) | returns `0` |
| E3 | `f2` (lib.c:105-106) | `typeA` out-of-range enum (`2`, `3`, `0xFFFFFFFF`), any `typeB` incl. valid ones | returns `0`; neither pointer dereferenced |
| E4 | `f3` (lib.c:111-112) | `v2 == 0` (division by zero), any `v1` incl. `INT_MIN` | returns `0` (guard prevents SIGFPE) |
| E5 | `f3` (lib.c:118) | `v1 >= 0`, `v2 < 0`, `v2 == INT_MIN` → guard `v2 != -0x7fffffff-1` fails | takes `q = 0, r = v1` branch; avoids `v1 / -INT_MIN` overflow |
| E6 | `f3` (lib.c:122) | `v1 == INT_MIN` → guard `v1 != -0x7fffffff-1` fails | falls to the `v1 == INT_MIN` group (lib.c:130-137); avoids `-v1` overflow |
| E7 | `f3` (lib.c:125) | `v1 < 0 && v1 != INT_MIN`, `v2 == INT_MIN` | takes `q = 1, r = v1 - q*v2` branch |
| E8 | `f3` (lib.c:131) | `v1 == INT_MIN` and `v2 == INT_MIN` | takes final `else`: `q = 1, r = 0` → returns `1` |
| E9 | `f3` (lib.c:130) | `v1 == INT_MIN` and `v2 >= 1` | `q = -((-(v1+v2))/v2) - 1` — `v1+v2` **signed overflow is UB in C**; must match what the compiled `-O0` C actually does (wrapping) |
| E10 | `f3` (lib.c:132-134) | `v1 == INT_MIN` and `INT_MIN < v2 < 0` | `q = ((-(v1-v2))/(-v2)) + 1` — `v1-v2` wraps |
| E11 | `f4` (lib.c:156-162) | `rnd == NULL` | **no check** — C dereferences and faults. Not testable differentially; asserted only that both accept any non-NULL state, incl. all-zero state (which is a fixed point of xorshift128+ → always returns `0.0`) |
| E12 | `f4` (lib.c:161) | any state; `*(double*)&result` type-punning with `exponent=1023` | result is always in `[0.0, 1.0)`, never NaN/inf — the `isnan` filter in `agglom` can never fire for `f4` |
| E13 | `f5` (lib.c:164-170) | any `a` with bits set above bit 15 (e.g. `0xFFFF0000`, `0xDEADBEEF`) | high 16 bits are silently **discarded** (every mask is 16-bit); result always `<= 0xFFFF` |
| E14 | `f7` (lib.c:451-458) | `blocksize`/`bitdepth` large enough that `blocksize * bitdepth * channels` overflows `uint32_t` (e.g. `0xFFFFFFFF, 1, 0xFFFFFFFF`) | unsigned wraparound (defined in C); result is the wrapped value, no error |
| E15 | `f7` | `channels == 0` | `channels != 2` is 1 → term `blocksize*bitdepth*0` = 0; returns `18 + 0 + (7/8)` = `18` |
| E16 | `f9` (lib.c:485) | degenerate triangle: `dot00*dot11 - dot01*dot01 == 0` (collinear/duplicate points, e.g. `p1==p2==p3`) | `invDenom = 1.0f/0.0f = ±inf`; `u`,`v` become `inf`/`-inf`/`NaN` (`0*inf`) |
| E17 | `f9` | any input containing NaN | NaN propagates to `u`/`v`; exact payload/sign is x86 `ADDSS`/`MULSS` src1-first selection |
| E18 | `f10` (lib.c:857-865) | `h >= 0xFC00` → `n = h>>10 == 63`, index `(h & 0x3ff) + m__offset[63]` | no range check exists; `n` is provably `<= 63` and the index provably `<= 2047`, so **no OOB is reachable** — verified exhaustively over all 65536 inputs |
| E19 | `f10` | `h` in `0x7C00..0x7FFF` / `0xFC00..0xFFFF` (half inf/NaN encodings) | returns float inf/NaN from the table; `agglom` then filters NaN |
| E20 | `f11` (lib.c:872-877) | `s == 0.0f` (incl. `-0.0f`) | early return, writes `l` to all three of `dest[0..2]`; `h` completely ignored |
| E21 | `f11` (lib.c:894) | **lib.c:894 reads `h < 120.0f && h < 180.0f`, not `h >= 120.0f`.** Verified against the compiled C: this branch therefore catches *every* `h < 120` not already claimed by the two earlier branches — i.e. all **negative** `h` (and `-inf`) land here, writing `{m, c+m, x+m}`. Reproduce, do not fix. | `dest = {m, c+m, x+m}` for `h < 0` |
| E21b | `f11` (lib.c:899-903) | the final `else` is consequently reachable only for `h` in `[120,180)`, `h >= 360` (incl. `+inf`), or `h` NaN | writes `m` to all three |
| E22 | `f11` (lib.c:884) | `h` is NaN with `s != 0` | every `h >= …` and `h < …` compare is false → final `else` → `dest = {m,m,m}` |
| E23 | `f11` (lib.c:883) | `h == -0.0f`, `s != 0` | `h >= 0.0f` is **true** for `-0.0f` → first branch taken (not the `else`) |
| E24 | `f11` | `dest == NULL` or `src == NULL` | no check — faults in both; not differentially testable |
| E25 | `f12` (lib.c:919-924) | `s == 0.0f` | early return `dest = {v,v,v}`; `h` ignored |
| E26 | `f12` (lib.c:957-961) | `i = (int)floorf(h/60)` not in `0..=4` — i.e. `h < 0`, `h >= 300`, or `h` NaN | `default:` branch → `r=v, g=p, b=q` |
| E27 | `f12` (lib.c:930) | `h` NaN or `|h/60| >= 2^31` → `(int)floorf(...)` is an **out-of-range float→int cast, UB in C** | x86 `cvttss2si` yields `0x80000000` (`INT_MIN`) → `default:` branch |
| E28 | `f13` (lib.c:984-989) | `delta == 0` (r==g==b, incl. all-NaN where every compare is false) | early return `dest = {0.0f, 0.0f, max}` |
| E29 | `f13` (lib.c:984) | `max == 0.0f` (e.g. all-zero, or all-negative input where max is `-0.0`/`0`) | early return `dest = {0.0f, 0.0f, max}` — even when `delta != 0` |
| E30 | `f13` (lib.c:998) | `delta != 0`, `max != 0`, and neither `r == max` nor `g == max` | final `else`: `h = 4 + (r-g)/delta` |
| E31 | `f13` (lib.c:1000-1001) | computed `h * 60` is negative | `h += 360.0f` correction applied once (can still leave `h < 0` for large-magnitude inputs — no clamp) |
| E32 | `f13` | input containing NaN such that `delta` is NaN | `delta == 0` false, `max == 0` false → proceeds; `s = NaN`, `h` from whichever `==` first matches, else final branch |
| E33 | `c2Dot` / `c2Sub` / `c2Maxv` / `c2Minv` (lib.c:35-57) | operand is NaN | `>` / `<` are false for NaN, so `c2Maxv(a,b)` returns `b.x` when `a.x` is NaN; `ADDSS`/`MULSS` quiet the **src1** NaN first |
| E34 | `c2CircletoCircle` / `c2CircletoAABB` (lib.c:64, 72) | `d2` or `r2` is NaN | `d2 < r2` is false → returns `0` (reported as "no collision") |
| E35 | `c2AABBtoAABB` (lib.c:75-81) | any coordinate NaN | all four `<` are false → `!(0|0|0|0)` → returns `1` ("collision") for an all-NaN box |
| E36 | `agglom` (lib.c:1023, 1044-1049, 1054, …) | any sub-result is NaN | that term is **skipped** via `!isnan(...)`; `inf`/`-inf` are **not** filtered and do propagate into `ret` |
| E37 | `agglom` | `f3_2 == 0` | `f3` returns `0`, contributes `0` — no error surfaced to the caller |
| E38 | `agglom` | `f2` is always called with `typeA=CIRCLE, typeB=AABB`, so the `default:` rows E1-E3 are **not reachable through `agglom`** — they are only reachable via the directly exported `f2`, which is why Phase C must call `f2` directly |
