# CONFIGS.md — configuration-surface table for VALID inputs (Phase A, gated in Phase B)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axis derivation (from the source, not from assumptions)

**Runtime options / modes / flags:** none.

```
$ grep -nE '#if|#ifdef|#ifndef|#else|#elif' c_src/src/driver.c            # -> no match
$ grep -nE '#if|#ifdef|#ifndef' c_src/include/driver.h
24:#ifndef DRIVER_H_        (include guard only)
$ grep -nE 'switch|static|extern|global' c_src/src/driver.c               # -> no match
```

There are no setters, no context/handle struct, no global state, no
byte-order/format/width selectors, no `switch`, and no compile-time
configuration. `translation/Cargo.toml` has no `[features]` table, so there is
exactly one build configuration.

**Public entry points (full set, including the lowest level):** exactly one —
`void driver(int x, int y)`. It *is* the lowest-level entry point; there is no
convenience wrapper layer to skip past.

**Input shape axes** — the code branches only on the two `int` arguments, and it
compares them against the constants `0`, `1`, `3`, `4`. The axes are therefore
the value classes those comparisons carve out:

| axis | source line(s) | classes |
|---|---|---|
| entry guard | `while (x > 0 \|\| y > 0)` | accept / reject |
| `x` vs `0` (guard, `label1` `if (x > 0)`) | 30, 38 | `x < 0` (incl. `INT_MIN`), `x == 0`, `x > 0` |
| `x` vs `1` (forward-goto predicate) | 33 | `x == 1`, `x != 1` |
| `x` vs `3` (back-edge `goto label1`) | 49 | `x < 3` (back-edge taken), `x >= 3` (fall through to guard re-test) |
| `y` vs `0` (guard, `label2` `if (y == 0) continue`) | 30, 44 | `y < 0`, `y == 0`, `y > 0` |
| `y` vs `4` (forward-goto predicate) | 33 | `y == 4`, `y != 4` |
| combined special case | 33 | `x == 1 && y == 4` → skip the `label1` block once |
| in-loop transition | 38 + 49 | `x` crossing the `3` boundary *during* execution (start `x > 3` with `y` large enough to decrement `x` below `3`) |
| magnitude | — | empty (`0`), one (`1`), few (`2..4`), many (`>4`), large (`~4096`), extreme (`INT_MIN`) |

Rows below are the cross-product of the `x` classes and `y` classes, pruned to
the combinations the code actually distinguishes. The non-terminating region
`x > 0 && y < 0` is excluded here and recorded in `ERRORS.md` rows 12–13.

Every row is exercised with **many randomized inputs drawn from that row's
class** (seeded, deterministic LCG — `SEED = 0x5DEECE66D`) by
`translation/tests/differential.rs::phase_b_configuration_surface`, comparing
the full `stdout` byte stream of the C `.so` against the Rust `.so`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | guard rejects: `x == 0, y == 0` — the empty case, no output | [x] |
| 2 | `driver` | guard rejects: `x < 0` random in `[INT_MIN, -1]`, `y == 0` | [x] |
| 3 | `driver` | guard rejects: `x == 0`, `y < 0` random in `[INT_MIN, -1]` | [x] |
| 4 | `driver` | guard rejects: both negative, random `x, y` in `[INT_MIN, -1]` | [x] |
| 5 | `driver` | `x == 0`, `y == 1` — one, `label1` block never taken | [x] |
| 6 | `driver` | `x == 0`, `y` random in `[2, 4]` — includes `y == 4` with `x != 1` (forward-goto near-miss) | [x] |
| 7 | `driver` | `x == 0`, `y` random in `[5, 4096]` — many/large, `x < 3` back-edge taken every pass | [x] |
| 8 | `driver` | `x < 0` random in `[-4096, -1]`, `y` random in `[1, 4096]` | [x] |
| 9 | `driver` | `x == INT_MIN`, `y` random in `[1, 512]` — extreme `x`, `x--` unreachable so no underflow | [x] |
| 10 | `driver` | `x == 1`, `y == 0` — guard passes on `x` alone; `continue` branch every iteration | [x] |
| 11 | `driver` | `x == 1`, `y == 1` | [x] |
| 12 | `driver` | `x == 1`, `y == 3` — one below the forward-goto constant | [x] |
| 13 | `driver` | `x == 1`, `y == 4` — **the `x == 1 && y == 4` forward `goto label2`**, the single special case | [x] |
| 14 | `driver` | `x == 1`, `y == 5` — one above the forward-goto constant | [x] |
| 15 | `driver` | `x == 1`, `y` random in `[6, 4096]` | [x] |
| 16 | `driver` | `x == 2`, `y == 0` — `x < 3` true, `x != 1` | [x] |
| 17 | `driver` | `x == 2`, `y == 4` — forward-goto predicate fails on `x` only | [x] |
| 18 | `driver` | `x == 2`, `y` random in `[1, 4096]` | [x] |
| 19 | `driver` | `x == 3`, `y == 0` — `x < 3` false on entry (back-edge not taken on first pass) | [x] |
| 20 | `driver` | `x == 3`, `y == 4` | [x] |
| 21 | `driver` | `x == 3`, `y` random in `[1, 4096]` — `x` crosses the `3` boundary mid-run | [x] |
| 22 | `driver` | `x == 4`, `y == 4` — both one past their forward-goto constants | [x] |
| 23 | `driver` | `x` random in `[4, 64]`, `y == 0` — `x >= 3` path with `y` dead | [x] |
| 24 | `driver` | `x` random in `[4, 64]`, `y` random in `[1, 64]` — small-many cross product, `x` crossing `3` | [x] |
| 25 | `driver` | `x` random in `[500, 4096]`, `y` random in `[500, 4096]` — large magnitudes, long back-edge chains | [x] |
| 26 | `driver` | `x` random in `[4, 4096]`, `y` random in `[1, 3]` — `y` exhausts long before `x` | [x] |
| 27 | `driver` | `x` random in `[1, 3]`, `y` random in `[4, 4096]` — `x` exhausts long before `y` | [x] |
| 28 | `driver` | unbiased broad sweep: `x, y` random in `[-8, 40]` (mixes reject/accept, all constants, both goto edges) | [x] |
| 29 | `driver` | `x == 4096`, `y == 4096` — fixed large boundary pair | [x] |
| 30 | `driver` | `x == 0`, `y == 4096` and `x == 4096`, `y == 0` — single-axis large | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
— no `add_executable`. There is no `[[bin]]` target in
`translation/Cargo.toml` and no `src/main.rs`. Verified:

```
$ grep -n add_executable c_src/CMakeLists.txt   # -> no match
$ ls translation/src                            # -> lib.rs
```

Therefore the "compare C and Rust binary stdout" obligation is **not
applicable**; stdout is nevertheless compared byte-for-byte for every row
above, which is the same evidence a driver binary would provide.

## Phase B result (evidence for the checkmarks above)

`./run_all_configs.sh` — all rows executed against both `.so`s, `stdout`
compared byte-for-byte. Identical output in both build profiles:

```
phase B: 4364 differential invocations matched        (all 30 rows, seeded randomized)
phase B grid: 1865 pairs matched                     (exhaustive [-4,40]^2, terminating pairs)
phase B oracle: 359 cases matched C, Rust and oracle
```

The suite also contains a negative control result: three mutants of
`src/lib.rs` (drop the `x == 1 && y == 4` forward goto; change the back-edge
predicate `x < 3` to `x < 2`; clear the skip flag one iteration late) were each
built as a `.so` and rejected by the suite at rows 13, 20 and 13 respectively —
so the passing runs above are not vacuous.
