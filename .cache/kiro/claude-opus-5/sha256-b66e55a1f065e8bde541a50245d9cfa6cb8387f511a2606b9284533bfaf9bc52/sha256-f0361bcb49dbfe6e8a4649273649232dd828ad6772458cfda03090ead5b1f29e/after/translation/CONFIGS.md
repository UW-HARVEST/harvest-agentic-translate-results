# CONFIGS.md — configuration-surface table

## Mechanical derivation of the axes

There is exactly one public entry point (`c_src/include/lib.h` is one line):

```c
void hsl_to_rgb(float *dest, const float *src);
```

so there is no wrapper/low-level split to worry about: `hsl_to_rgb` **is** the
lowest-level entry point and the only one. The `.so` exports nothing else
(see `SYMBOLS.md`).

There are no runtime option/mode/flag setters, no global state, no init/teardown,
no context struct, and no `#ifdef` in `c_src/src/lib.c`. Confirmed:

```sh
grep -cE '#if|#ifdef|#ifndef|static |extern |struct |enum |typedef' c_src/src/lib.c   # -> 0
```

The configuration surface is therefore entirely the **shape and value class of
the 3-float input**, crossed with the **buffer geometry** of the two pointers.
The axes the C actually branches on:

**Axis H — hue sector** (the `if`/`else if` cascade, 7 outcomes, derived from
lines 18-45). Note arm 3's predicate is `h < 120.0f && h < 180.0f`, so the
reachable partition of the real line is *not* the six nominal sectors:

| region of `h` | arm taken |
|---|---|
| `h < 0` (incl. `-inf`) | arm 3 (`m`, `c+m`, `x+m`) |
| `[0, 60)` | arm 1 |
| `[60, 120)` | arm 2 |
| `[120, 180)` | final `else` (dead range) |
| `[180, 240)` | arm 4 |
| `[240, 300)` | arm 5 |
| `[300, 360)` | arm 6 |
| `>= 360` (incl. `+inf`) | final `else` |
| `NaN` | final `else` |

**Axis S — saturation class** (line 10 `if (s == 0)`): `s == +0.0` / `s == -0.0`
(early-return path) vs `s != 0` (full path). Sub-classes of `s != 0` that the
arithmetic distinguishes: `0 < s <= 1`, `s > 1`, `s < 0`, `±inf`, subnormal, `NaN`.

**Axis L — lightness class** (feeds `c = (1-|2l-1|)*s` and `m = l-0.5c`):
`l == 0.5` (`|2l-1| == 0`, so `c == s`), `l == 0` / `l == 1` (`c == 0`, so
`x == 0` and `m == l`), `0 < l < 1`, `l < 0`, `l > 1`, `±inf`, subnormal, `NaN`.

**Axis F — `fmodf` regime** for `x = c*(1-|fmodf(h/60,2)-1|)`: `h/60` finite
(normal reduction), `h/60` an exact multiple of 2 (`fmodf` returns `±0.0`),
`h/60 == ±inf` (`fmodf` returns `NaN`), `h/60 == NaN`, `h` subnormal
(`h/60` may flush toward `0`).

**Axis N — NaN operand pairing.** The C compiles to two-operand SSE
(`addss`/`subss`/`mulss`/`divss`); when *both* operands of one instruction are
`NaN` the *destination* operand's payload+sign is what propagates. So the
operand order of every arithmetic node is observable, and `l == NaN` with
`s == 1` makes `x` and `m` two `NaN`s with **different sign bits**, which
distinguishes `add(x,m)` from `add(m,x)`. This axis crosses with axis H (each
arm has its own operand order per output lane).

**Axis G — buffer geometry** of the two pointers: disjoint, `dest == src`,
`dest == src+1`, `dest == src-1`, unaligned (4-byte-aligned-but-not-16), plus
guard words to prove exactly `src[0..3]` is read and `dest[0..3]` is written.

**Axis B — build configuration.** `translation/Cargo.toml` has **no
`[features]`** table, so `default` is the only feature combination; the axis has
a single value and is folded into every row.

## Table

One row per combination the C treats differently (cross-product of H x {S,L,F,N}
pruned to the distinctions the source actually makes), plus the geometry rows.
Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, `SplitMix64`) in
`translation/tests/phase_b_configs.rs`, not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `hsl_to_rgb` | S=`+0.0` early return; `l` random over all finite `f32`; `h` random over all `f32` incl. non-finite | [x] |
| 2 | `hsl_to_rgb` | S=`-0.0` early return; `l` random incl. `±inf`/`NaN`/subnormal; `h` random | [x] |
| 3 | `hsl_to_rgb` | H=`[0,60)` x S=`(0,1]` x L=`(0,1)` — the nominal happy path, randomized | [x] |
| 4 | `hsl_to_rgb` | H=`[60,120)` x S=`(0,1]` x L=`(0,1)` | [x] |
| 5 | `hsl_to_rgb` | H=`[120,180)` (dead range -> `else`) x S=`(0,1]` x L=`(0,1)` | [x] |
| 6 | `hsl_to_rgb` | H=`[180,240)` x S=`(0,1]` x L=`(0,1)` | [x] |
| 7 | `hsl_to_rgb` | H=`[240,300)` x S=`(0,1]` x L=`(0,1)` | [x] |
| 8 | `hsl_to_rgb` | H=`[300,360)` x S=`(0,1]` x L=`(0,1)` | [x] |
| 9 | `hsl_to_rgb` | H=`>=360` (`[360, 1e9]`, randomized) -> `else` x S=`(0,1]` x L=`(0,1)` | [x] |
| 10 | `hsl_to_rgb` | H=`<0` (`[-1e9, -eps]`, randomized) -> buggy arm 3 x S=`(0,1]` x L=`(0,1)` | [x] |
| 11 | `hsl_to_rgb` | H = exact sector boundaries `0,60,120,180,240,300,360` and their `nextafter` neighbours on both sides, x randomized `s`,`l` | [x] |
| 12 | `hsl_to_rgb` | H=all 9 regions x L=`0.5` exactly (`c == s`, `m == l - s/2`) | [x] |
| 13 | `hsl_to_rgb` | H=all 9 regions x L=`0.0` and `L=1.0` (`c == 0` -> `x == 0`, `m == l`) | [x] |
| 14 | `hsl_to_rgb` | H=all 9 regions x L outside `[0,1]` (`l < 0`, `l > 1`, randomized) -> `c < 0`, unclamped output | [x] |
| 15 | `hsl_to_rgb` | H=all 9 regions x S=`>1` and S=`<0` (randomized), L=`(0,1)` — no clamping | [x] |
| 16 | `hsl_to_rgb` | F: `h` an exact multiple of `120` (so `h/60` is an even integer and `fmodf` returns `+0.0`, `x == c`): `0,120,240,360,480,-120,-240` | [x] |
| 17 | `hsl_to_rgb` | F: `h` an exact odd multiple of `60` (`fmodf(h/60,2) == 1`, so `x == 0`): `60,180,300,420,-60,-180` | [x] |
| 18 | `hsl_to_rgb` | F: `h == ±inf` -> `h/60 == ±inf` -> `fmodf` = `NaN` -> `x` = `NaN`; `+inf` hits `else`, `-inf` hits arm 3 | [x] |
| 19 | `hsl_to_rgb` | F: `h == NaN` (quiet, both signs, random payloads) -> `else`; and `h` subnormal / `±0.0` | [x] |
| 20 | `hsl_to_rgb` | L=`±inf` x S=`(0,1]` x H=all 9 regions (`c` = `-inf`/`NaN`, `m` = `inf`-`inf` = `NaN`) | [x] |
| 21 | `hsl_to_rgb` | S=`±inf` x L=`(0,1)` x H=all 9 regions | [x] |
| 22 | `hsl_to_rgb` | S and/or L subnormal (incl. smallest positive subnormal) x H=all 9 regions | [x] |
| 23 | `hsl_to_rgb` | N: `l = NaN` (random payload, **both** sign bits) x `s = 1.0` x H=all 9 regions — makes `x` and `m` differing `NaN`s, probing every `addss` operand order | [x] |
| 24 | `hsl_to_rgb` | N: `s = NaN` (random payload, both signs) x `l` finite x H=all 9 regions — probes the `mulss` operand order for `c` | [x] |
| 25 | `hsl_to_rgb` | N: `h = NaN` x `s`,`l` finite; and `h = NaN` x `l = NaN` x `s = NaN` (all-`NaN`) with distinct payloads | [x] |
| 26 | `hsl_to_rgb` | N: signalling `NaN` payloads (`0x7F80_0001`, `0xFF80_0001`) in each of `h`,`s`,`l` — must be quieted identically | [x] |
| 27 | `hsl_to_rgb` | Unrestricted fuzz: all three inputs drawn as **uniform random 32-bit patterns** (any `f32`, any class) — 200 000 cases | [x] |
| 28 | `hsl_to_rgb` | G: `dest == src` (full aliasing), randomized inputs over all regions | [x] |
| 29 | `hsl_to_rgb` | G: `dest == src + 1` and `dest == src - 1` (partial overlap), randomized | [x] |
| 30 | `hsl_to_rgb` | G: guard sentinels around both buffers — proves exactly `src[0..3]` read, `dest[0..3]` written, nothing beyond | [x] |
| 31 | `hsl_to_rgb` | G: `dest`/`src` 4-byte aligned but not 8/16-byte aligned, randomized | [x] |
| 32 | `hsl_to_rgb` | B: every row above re-run under the only feature combination (`default` == `--no-default-features`, no `[features]` in `Cargo.toml`) | [x] |

Rows: 32. No binary/driver target exists (`translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]`; `c_src/CMakeLists.txt` declares only
`add_library(... SHARED ...)`), so the "compare stdout of the two executables"
clause is vacuous — there is nothing but the shared library to compare.
