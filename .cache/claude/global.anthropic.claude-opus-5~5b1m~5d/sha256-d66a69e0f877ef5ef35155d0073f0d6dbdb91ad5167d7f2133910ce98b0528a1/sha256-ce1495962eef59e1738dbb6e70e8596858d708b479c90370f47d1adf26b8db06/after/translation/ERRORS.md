# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum,
no `RETURN_ERROR` macro, no `assert`, and no `errno` usage**; grep confirms:

```
$ grep -nE 'assert|errno|RETURN_ERROR|return NULL|return -1|ERROR' c_src/src/lib.c
   (no matches)
```

Its rejection mechanism is therefore: **`switch` `default:` arms, `if` guards
that early-`return` a sentinel, and the four `-0x7fffffff - 1` (`INT_MIN`)
overflow guards.** Every one of those is enumerated below, one row per distinct
rejection branch, together with the generic FFI boundary cases (out-of-range
enums, extremal integers, NaN/Inf, null pointers).

`expected C result` is the exact value/observable the C produces, and the Rust
`.so` must reproduce it bit-for-bit.

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|----------------------------------------------|-------------------|---|
| E1 | `f2` (lib.c:105-106) | `typeA` is neither `0` (`C2_TYPE_CIRCLE`) nor `1` (`C2_TYPE_AABB`) — outer `switch` `default:`. Covers `2`, `3`, `0xFFFFFFFF`, `INT_MAX`, and every other out-of-range enum int. `A`/`B` are **not** dereferenced. | `return 0` | [x] |
| E2 | `f2` (lib.c:91-92) | `typeA == 0` (CIRCLE) and `typeB` out of range (`>= 2`) — inner `switch` `default:`. `A`/`B` not dereferenced. | `return 0` | [x] |
| E3 | `f2` (lib.c:101-102) | `typeA == 1` (AABB) and `typeB` out of range (`>= 2`) — inner `switch` `default:`. `A`/`B` not dereferenced. | `return 0` | [x] |
| E4 | `f2` | `typeA`/`typeB` out of range **and** `A == NULL`, `B == NULL`. Pointers are unread on the `default:` paths, so this must not fault. | `return 0` | [x] |
| E5 | `f3` (lib.c:111-113) | `v2 == 0` (division by zero guard) — for *any* `v1`, including `INT_MIN`. | `return 0` | [x] |
| E6 | `f3` (lib.c:118, else at 121) | `v1 >= 0` **and** `v2 == INT_MIN` (`-0x7fffffff - 1`): `-v2` would overflow, so C takes `q = 0, r = v1`. Final result depends on the `r >= 0` fix-up. | `v1 == 0` → `0`; `v1 > 0` → `q + (v2>0 ? -1 : 1)` is not taken because `r = v1 >= 0`, so `0` | [x] |
| E7 | `f3` (lib.c:125, else at 127-128) | `v1 < 0`, `v1 != INT_MIN`, **and** `v2 == INT_MIN`: `q = 1, r = v1 - q*v2` (signed overflow → wraps). | `r = v1 - INT_MIN` (wrapping) `>= 0` for all `v1 < 0`, hence `1` | [x] |
| E8 | `f3` (lib.c:122, 129-130) | `v1 == INT_MIN` **and** `v2 >= 1`: guard `v1 != INT_MIN` fails; C uses `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2))%v2)`, with `v1+v2` and its negation possibly overflowing. | floored quotient (wrapping arithmetic); e.g. `f3(INT_MIN, 1) == INT_MIN`, `f3(INT_MIN, 2) == -1073741824` | [x] |
| E9 | `f3` (lib.c:131-132) | `v1 == INT_MIN`, `v2 < 0`, `v2 != INT_MIN`: `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2))%(-v2))`; `v1 - v2` overflows for `v2 < 0`. | wrapping result; **verified against the C**: `f3(INT_MIN, -1) == INT_MIN` (the `+ 1` overflows `INT_MAX` and wraps), `f3(INT_MIN, -2) == 1073741824` | [x] |
| E10 | `f3` (lib.c:133-134) | `v1 == INT_MIN` **and** `v2 == INT_MIN`: both overflow guards fail → `q = 1, r = 0`. | `return 1` | [x] |
| E11 | `f3` (lib.c:135-138) | `r < 0` fix-up path: any `(v1, v2)` whose remainder is negative, e.g. `f3(-1, 3)`, `f3(-5, 3)`, `f3(-7, 2)`. | `q + (v2 > 0 ? -1 : 1)` (wrapping add; can wrap at `INT_MIN`). **Verified quirk:** in the `v1 >= 0, v2 < 0` quadrant the C sets `r = v1 % (-v2)`, which is *never* negative, so the fix-up can never fire and `f3` TRUNCATES instead of flooring there — `f3(7, -2) == -3` (not `-4`), `f3(30575, -412) == -74` (not `-75`). Reproduced verbatim, not fixed. | [x] |
| E12 | `f4` (lib.c:156-162) | Degenerate PRNG state `{0, 0}` — `cn_rnd_next` returns `0`, so `mantissa == 0`. | `1.0 - 1.0 == +0.0` (bit pattern `0x0000000000000000`) | [x] |
| E13 | `f4` (lib.c:145-154) | `state == {UINT64_MAX, UINT64_MAX}` / `{0, UINT64_MAX}` / `{UINT64_MAX, 0}` — `x + y` wraps `uint64_t`. Also verifies `rnd` is mutated in place identically. | wrapping xorshift128+ output; result always in `[0.0, 1.0)`, never NaN; `state` updated | [x] |
| E14 | `f5` (lib.c:164-170) | Bits **above bit 15** set (`a > 0xFFFF`), e.g. `0xFFFF_0000`, `0xDEAD_BEEF`, `UINT32_MAX`. The masks are 16-bit, so the high half is silently discarded — not a reject, but a lossy path a caller can hit. | `f5(a) == f5(a & 0xFFFF)`, always `<= 0xFFFF` | [x] |
| E15 | `f7` (lib.c:451-458) | `blocksize * bitdepth * ...` overflows `uint32_t` (e.g. `blocksize = bitdepth = 0xFFFF_FFFF`), and/or `18 + channels` overflows (`channels = UINT32_MAX`). | wrap-around modulo 2^32 (unsigned, defined) | [x] |
| E16 | `f7` | `channels == 2` (the `channels != 2` term vanishes and the two `channels == 2` terms activate — a distinct branch/quirk), incl. `blocksize = 0`. | value from the `channels == 2` terms only | [x] |
| E17 | `f7` | `bitdepth == 32` (the `bitdepth != 32` correction term becomes `0`), with `channels == 2`. | no `+1` correction added | [x] |
| E18 | `f9` (lib.c:486) | Degenerate triangle → `dot00*dot11 - dot01*dot01 == 0` (collinear/coincident points, e.g. `p1 == p2 == p3`). `1.0f / 0.0f` is **not** guarded. | `invDenom = ±inf`; `u`/`v` become `±inf`, `NaN`, or `±0` per IEEE-754 | [x] |
| E19 | `f9` | Any input coordinate is `NaN` / `±Inf`. Unguarded. | NaN/Inf propagation, bit-identical | [x] |
| E20 | `f10` (lib.c:857-865) | Index-range boundary: `h = 0xFFFF` → `n = 63`, `h & 0x3ff = 0x3ff`, `m__offset[63] = 0x400`, so `m__mantissa[2047]` — the **last** in-bounds element. `h = 0` → `m__mantissa[0]`. Every `uint16_t` is in range by construction; there is no rejection path, so the whole 65536-value domain is exhaustively checked. | table lookup + exponent, `wrapping_add`; includes `NaN`/`±Inf`/subnormal encodings | [x] |
| E21 | `f11` (lib.c:872-877) | `s == 0.0f` (including `-0.0f`, since `-0.0 == 0.0`) → early return, `h`/NaN ignored. | `dest[0..3] = l` (bit-identical, `l` may be NaN/Inf) | [x] |
| E22 | `f11` (lib.c:905-909) | `h` matches **none** of the six range tests → final `else`. Reachable only when `h` is `NaN` or `h >= 360.0f` (note `h < 0` is captured by the `h < 120 && h < 180` arm). | `dest[0..3] = m` | [x] |
| E23 | `f11` (lib.c:889) | The C's third arm is `h < 120.0f && h < 180.0f` (**not** `h >= 120.0f`), so every `h < 0.0f` — including `-Inf` — lands there instead of the `else`. Bug reproduced verbatim, not fixed. | `dest = { m, c+m, x+m }` | [x] |
| E24 | `f12` (lib.c:919-924) | `s == 0.0f` → early return, `h` ignored (even NaN). | `dest[0..3] = v` | [x] |
| E25 | `f12` (lib.c:926, 957-961) | `i = (int)floorf(h/60)` is **negative** (`h < 0`) → `switch` `default:`. | `dest = { v, p, q }` | [x] |
| E26 | `f12` (lib.c:926, 957-961) | `i >= 5` (`h >= 300`) → `switch` `default:`. | `dest = { v, p, q }` | [x] |
| E27 | `f12` (lib.c:926) | `(int)floorf(h)` where the value is **not representable in `int`** — `h = NaN`, `±Inf`, or `|h/60| >= 2^31`. C leaves this undefined; x86-64 `cvttss2si` yields `INT_MIN` (`0x80000000`) → `default:` arm. | `i == INT_MIN` → `dest = { v, p, q }` with `f = h - (float)INT_MIN` | [x] |
| E28 | `f13` (lib.c:984-989) | `delta == 0.0f` (i.e. `r == g == b`) → early return with `h = 0, s = 0`. | `dest = { 0.0, 0.0, max }` | [x] |
| E29 | `f13` (lib.c:984-989) | `max == 0.0f` (division-by-zero guard) — reachable with a *negative* channel, e.g. `{0, 0, -1}` → `max = 0`, `delta = 1`. | `dest = { 0.0, 0.0, 0.0 }` (`v = max = 0`) | [x] |
| E30 | `f13` (lib.c:975-982) | Any channel is `NaN`: the `min`/`max` ternaries are `<`/`>` comparisons that are **false** for NaN, so NaN never displaces the incumbent — a subtle ordering-dependent path. Also `-Inf`/`+Inf`. | bit-identical NaN/Inf propagation through `delta`, `s`, `h` | [x] |
| E31 | `f13` (lib.c:998-999) | `h < 0` fix-up (`h += 360`) — hit when `r == max` and `g < b`. | `h` in `[0, 360)` | [x] |
| E32 | `f13` (lib.c:990) | `s = delta / max` with `max` a tiny subnormal and `delta` large → overflow to `+Inf`; and `delta/max` where `max < 0` is impossible (guarded by `max == 0`)? No — `max` can be **negative** (all channels negative), giving a negative `s`. e.g. `{-1, -2, -3}`. | negative `s`, `h` computed from negative `delta/…` | [x] |
| E33 | `agglom` (lib.c:1033…1099) | 13 separate `isnan()` filters: each sub-result that is NaN is **skipped** rather than added. Triggered by e.g. degenerate `f9` input, NaN colour inputs, `f10` NaN encodings. | NaN contributions omitted; running `double` sum otherwise | [x] |
| E34 | `agglom` | `f3_2 == 0` (so `f3` returns `0`), plus `f3_1 = INT_MIN, f3_2 = INT_MIN`, exercising the `f3` error rows through the aggregate entry point. | matching `double` sum | [x] |
| E35 | `f11`/`f12`/`f13` | `dest == src` (fully aliasing pointers). The C reads all three `src` floats into locals before storing, so aliasing is benign; the Rust wrapper must load-then-store identically. | identical 3-float output | [x] |
| E36 | all pointer-taking entry points (`f2`, `f4`, `f11`, `f12`, `f13`) | `NULL` pointer that **is** dereferenced. The C has no null check — this is UB / `SIGSEGV` in both implementations, so it is documented, not asserted. (`f2` with an out-of-range enum, row E4, is the only null case with defined behaviour, and it is tested.) | `SIGSEGV` in C and in Rust alike — not exercised | n/a |

## Coverage

Rows E1–E35 all have a differential test in
`translation/tests/differential.rs` (see the `err_*` test functions) that
constructs the exact condition, calls **both** `.so`s, and asserts the returned
sentinel / bit pattern is identical. Row E36 is unexercisable by design
(genuine UB in the C ground truth).
