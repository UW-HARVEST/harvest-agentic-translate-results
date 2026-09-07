# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public entry points (full set)

`c_src/include/lib.h` declares exactly one symbol, and it is simultaneously the
lowest-level and the only entry point — there is no convenience wrapper layer to
skip past:

```c
int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);
```

## Axes the C branches on

Enumerated from every `if` / mask / shift / division in the 62-line source:

* **A. `lsbit` mode** (lines 12–29) — the only runtime "option". Four distinct
  states the code distinguishes:
  * `A0`: `lsbit == 0` → whole block skipped.
  * `A4`: `lsbit == 4` → "dither" path: clear bit 0, then re-set it to
    `(uni>>1) & (uni>>2) & 1`.
  * `Aodd`: `lsbit & 1` and `lsbit != 4` → force bit 0 set.
  * `Aeven`: `lsbit` nonzero, even, `!= 4` → force bit 0 clear.
* **B. `uni & 7` position within its 3-bit group** (lines 8, 10) — controls
  whether the `+1` / `-1` neighbours survive:
  * `B0`: `uni & 7 == 0` → `uni2` clamped to `uni` (only `uni1` is a real
    candidate).
  * `B7`: `uni & 7 == 7` → `uni1` clamped to `uni` (only `uni2` is real).
  * `Bmid`: `1..6` → both neighbours are real candidates.
* **C. `uni & 8` sign bit**, evaluated independently for `uni`, `uni1`, `uni2`
  (lines 31, 37, 43): `Cpos` (clear → `+diff`) vs `Cneg` (set → `-diff`).
  Because of axis B the three can disagree, so the sign pattern across the
  candidates is itself a shape.
* **D. `step` magnitude/sign** (line 30 `(2*(uni&7)+1)*step) / 8`):
  `Dzero` (0 → all diffs 0), `Dsmall` (`1..7`, so the `/8` truncates to 0),
  `Dtypical`, `Dneg`, `Dmax` (`INT_MAX`, multiplication overflows),
  `Dmin` (`INT_MIN`).
* **E. `pred` / `tgt` / `tgt2` value shape** (lines 33–56): `Ezero`,
  `Etypical`, `Elarge` (near `INT_MAX`/`INT_MIN` so the subtractions overflow).
* **F. secondary-target weight** (`d3 >> 5`, lines 50/53/56): `Fnone`
  (`tgt2 == tgt`, penalty is symmetric), `Fsame` (`tgt2` small so `>>5` is 0 and
  the penalty vanishes), `Fdominant` (`tgt2` far away so the `>>5` term decides
  the winner).
* **G. selection outcome** (lines 57–61): `Gkeep` (neither `<`), `Gup`
  (`d1 < d0` only → `uni1`), `Gdown` (`d2 < d0` only → `uni2`),
  `Gboth` (both → the second `if` wins, `uni2`).
* **H. `uni` sign** (line 14 `uni &= ~1`, line 17 `uni >> 1`): `Hpos` vs `Hneg`
  (arithmetic shift, two's-complement masking).

There are **no compile-time axes**: `grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c`
returns nothing, and `Cargo.toml` has no `[features]` table. The only build
configuration is the default one.

## Rows (pruned cross-product of the axes the C treats differently)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) in `tests/valid_paths.rs`, not one hand-picked value. The free
axes in each row are randomized; the named axes are pinned.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `encode_quant` | `A0` (`lsbit=0`), `Bmid`, `Cpos`, `Dtypical`, `Etypical` — the baseline path | [x] |
| 2  | `encode_quant` | `A0`, `B0` (`uni&7==0`) → `uni2` clamped; `Dtypical` | [x] |
| 3  | `encode_quant` | `A0`, `B7` (`uni&7==7`) → `uni1` clamped; `Dtypical` | [x] |
| 4  | `encode_quant` | `A0`, `Bmid`, `Cneg` (`uni&8` set) → `diff` negated | [x] |
| 5  | `encode_quant` | `A0`, `B7` + `Cneg` → clamped `uni1` *and* negated diff together | [x] |
| 6  | `encode_quant` | `A0`, `B0` + `Cneg` → clamped `uni2` *and* negated diff together | [x] |
| 7  | `encode_quant` | `A0`, mixed sign pattern: `uni&7==7`/`uni&8` differ across `uni`/`uni1`/`uni2` | [x] |
| 8  | `Aodd` | `lsbit` odd (`1,3,5,7,9,-1,-3`), `Bmid`, `Dtypical` — bit 0 forced set | [x] |
| 9  | `Aodd` | `lsbit` odd, `B0`/`B7` boundary combined with the forced `\|= 1` | [x] |
| 10 | `Aeven` | `lsbit` even nonzero `!= 4` (`2,6,8,10,-2,-4`), `Bmid` — bit 0 forced clear | [x] |
| 11 | `Aeven` | `lsbit` even nonzero `!= 4`, `B0`/`B7` boundary | [x] |
| 12 | `A4` | `lsbit == 4` dither path, `Hpos`, `Bmid` — `(uni>>1)&(uni>>2)&1` | [x] |
| 13 | `A4` | `lsbit == 4`, `Hneg` (`uni` negative) → arithmetic `>>` in the dither | [x] |
| 14 | `A4` | `lsbit == 4` at `B0`/`B7` — clamping interacts with the dither re-set | [x] |
| 15 | `A4` | `lsbit == 4` sweeping `uni` over a full `0..=63` window (all bit patterns of `uni>>1`,`uni>>2`) | [x] |
| 16 | `encode_quant` | `Dzero` (`step == 0`) → all diffs 0, `p0==p1==p2`, ties everywhere | [x] |
| 17 | `encode_quant` | `Dsmall` (`step` in `1..=7`) → `(2*(uni&7)+1)*step/8` truncates to 0 for small `uni&7` | [x] |
| 18 | `encode_quant` | `Dneg` (`step < 0`) → negative numerator, `/8` truncates toward zero | [x] |
| 19 | `encode_quant` | `Dmax` (`step == INT_MAX`) → `(2*(uni&7)+1)*step` overflows, all `lsbit` modes | [x] |
| 20 | `encode_quant` | `Dmin` (`step == INT_MIN`) → overflow *and* `diff = -diff` on `INT_MIN` | [x] |
| 21 | `encode_quant` | `Ezero` (`pred=tgt=tgt2=0`), all `lsbit` modes, randomized `uni`/`step` | [x] |
| 22 | `encode_quant` | `Elarge`: `pred`/`tgt` near `INT_MAX` → `tgt - p0` overflows | [x] |
| 23 | `encode_quant` | `Elarge`: `pred`/`tgt` near `INT_MIN` → `tgt - p0` overflows the other way | [x] |
| 24 | `encode_quant` | `Fsame` (`tgt2 == tgt`) → symmetric secondary penalty | [x] |
| 25 | `encode_quant` | `Fnone` (`tgt2` within 31 of every `p`) → `d3 >> 5 == 0`, penalty vanishes | [x] |
| 26 | `encode_quant` | `Fdominant` (`tgt2` far from `tgt`) → the `>>5` term flips the winner | [x] |
| 27 | `encode_quant` | `Fdominant` with negative `d3` → arithmetic `>>5` rounds toward −inf | [x] |
| 28 | `encode_quant` | `Gkeep`: `step` and targets chosen so neither `d1<d0` nor `d2<d0` | [x] |
| 29 | `encode_quant` | `Gup`: only `d1 < d0` → returns `uni1` | [x] |
| 30 | `encode_quant` | `Gdown`: only `d2 < d0` → returns `uni2` | [x] |
| 31 | `encode_quant` | `Gboth`: both `<` → second `if` overwrites, returns `uni2` (the C quirk) | [x] |
| 32 | `encode_quant` | `Hneg`: `uni` negative across all four `lsbit` modes (two's-complement masking) | [x] |
| 33 | `encode_quant` | exhaustive `uni` in `-64..=64` × `lsbit` in `-4..=9` × small `step`/targets grid | [x] |
| 34 | `encode_quant` | full 6-arg uniform-random fuzz over the entire `i32` range, 200k cases | [x] |
| 35 | `encode_quant` | structured fuzz: each arg independently drawn from `{0,±1,±7,±8,INT_MIN,INT_MAX,rand}` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)` — there is
**no `add_executable`**, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. So there is no driver binary and no
stdout to compare; that completion-gate item is not applicable.
