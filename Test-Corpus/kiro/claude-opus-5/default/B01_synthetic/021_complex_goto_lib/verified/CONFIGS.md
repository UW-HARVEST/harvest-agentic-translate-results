# CONFIGS.md — configuration-surface table (Phase B gate)

## How this table was derived

### Public entry points (the FULL set, not just wrappers)

`c_src/include/driver.h` declares exactly one function, and `nm -D` on the C
`.so` exports exactly one symbol:

| entry point | signature | kind |
|---|---|---|
| `driver` | `void driver(int x, int y)` | the **only** entry point — it is simultaneously the lowest-level and the highest-level API. There are no convenience/one-shot wrappers layered over a lower-level API here, so "test the low-level entry point too" is satisfied trivially: `driver` *is* the low-level entry point. |

### Runtime options / modes / flags

Mechanically searched for: `#if`/`#ifdef`/`#define` (only the `DRIVER_H_`
include guard), file-scope/`static` state, setter functions, `getenv`, and
struct-of-options parameters. **None exist.** `driver` is a pure function of its
two `int` arguments plus `stdout`. There are therefore **no option axes** — the
entire configuration surface is the input *shape*, i.e. the region of the
`(x, y)` plane the argument pair lands in.

### Input-shape axes the C actually branches on

Every branch site in `c_src/src/driver.c`:

| site | line | predicate |
|---|---|---|
| S1 | 30 | `while (x > 0 \|\| y > 0)` — loop guard |
| S2 | 33 | `if (x == 1 && y == 4)` — forward `goto label2`, skipping `label1` |
| S3 | 38 | `if (x > 0)` — `label1` block (`printf("x\n"); x--;`) |
| S4 | 44 | `if (y == 0) continue;` |
| S5 | 49 | `if (x < 3) goto label1;` — backward edge |

Collapsing those five predicates into equivalence classes gives the two axes:

* **`x` classes** (6): `x < 0`, `x == 0`, `x == 1`, `x == 2`, `x == 3`, `x > 3`
  (`0` splits S1/S3; `1` splits S2; `3` splits S5, with `2` and `3` as the
  one-step-either-side boundary values)
* **`y` classes** (5): `y < 0`, `y == 0`, `0 < y < 4`, `y == 4`, `y > 4`
  (`0` splits S1/S4; `4` splits S2, with `1..3` and `5..` as the neighbours)

The rows below are the **full 6 × 5 cross-product** of those classes — the
combinations the code actually distinguishes — plus extreme-magnitude and
whole-domain sweep rows. The 4 combinations where `x > 0 && y < 0` are the
non-terminating class; they are owned by `ERRORS.md` row 12 and are marked here
as such rather than silently dropped.

**Termination law** (proved from the source, used to pick sweep inputs): `y`
never becomes negative if it starts non-negative (`y--` is guarded by `y != 0`),
`x` never becomes negative if it starts non-negative (`x--` is guarded by
`x > 0`), and every full body pass either decrements `x`, decrements `y`, or
exits. Hence `driver` terminates **iff `y >= 0 || x <= 0`**, and diverges
exactly on `x > 0 && y < 0`.

Every row is exercised with **many randomized inputs** from its region
(`SplitMix64`, fixed seed `0x5D1F_C0DE_1234_5678`), and asserted byte-identical
between the C `.so` and the Rust `.so`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options exist · `x < 0`, `y < 0` — S1 false, body never runs | [x] |
| 2 | `driver` | `x < 0`, `y == 0` — S1 false | [x] |
| 3 | `driver` | `x < 0`, `0 < y < 4` — S1 true via `y`, S3 always false, S5 always true (pure `label1`↔`label2` replay) | [x] |
| 4 | `driver` | `x < 0`, `y == 4` — S2 false (needs `x == 1`), so no skip | [x] |
| 5 | `driver` | `x < 0`, `y > 4` | [x] |
| 6 | `driver` | `x == 0`, `y < 0` — S1 false | [x] |
| 7 | `driver` | `x == 0`, `y == 0` — S1 false, zero-work boundary | [x] |
| 8 | `driver` | `x == 0`, `0 < y < 4` | [x] |
| 9 | `driver` | `x == 0`, `y == 4` — `y == 4` present but `x != 1`, S2 false | [x] |
| 10 | `driver` | `x == 0`, `y > 4` | [x] |
| 11 | `driver` | `x == 1`, `y < 0` — **non-terminating**, owned by `ERRORS.md` row 12 | [x] |
| 12 | `driver` | `x == 1`, `y == 0` — S4 `continue` on the first pass | [x] |
| 13 | `driver` | `x == 1`, `0 < y < 4` — S2 false, S5 true (`1 < 3`) | [x] |
| 14 | `driver` | `x == 1`, `y == 4` — **the S2 special case**: forward `goto label2` skips `label1` once, then the back-edge re-enters `label1` normally | [x] |
| 15 | `driver` | `x == 1`, `y > 4` — S2 false because `y != 4` (one step past the special case) | [x] |
| 16 | `driver` | `x == 2`, `y < 0` — **non-terminating**, `ERRORS.md` row 12 | [x] |
| 17 | `driver` | `x == 2`, `y == 0` | [x] |
| 18 | `driver` | `x == 2`, `0 < y < 4` — S5 true at the boundary (`2 < 3`) | [x] |
| 19 | `driver` | `x == 2`, `y == 4` — `y == 4` with `x` one step past 1 | [x] |
| 20 | `driver` | `x == 2`, `y > 4` | [x] |
| 21 | `driver` | `x == 3`, `y < 0` — **non-terminating**, `ERRORS.md` row 12 | [x] |
| 22 | `driver` | `x == 3`, `y == 0` | [x] |
| 23 | `driver` | `x == 3`, `0 < y < 4` — S5 **false** at the boundary (`3 < 3`), so the back-edge is declined and the `while` guard is re-tested | [x] |
| 24 | `driver` | `x == 3`, `y == 4` | [x] |
| 25 | `driver` | `x == 3`, `y > 4` | [x] |
| 26 | `driver` | `x > 3`, `y < 0` — **non-terminating**, `ERRORS.md` row 12 | [x] |
| 27 | `driver` | `x > 3`, `y == 0` — pure `x`-drain, one `"loop\nx\n"` per outer pass | [x] |
| 28 | `driver` | `x > 3`, `0 < y < 4` — S5 false while `x >= 3`, then flips true as `x` drains below 3 (mode change mid-run) | [x] |
| 29 | `driver` | `x > 3`, `y == 4` | [x] |
| 30 | `driver` | `x > 3`, `y > 4` — both counters large, S5 flips mid-run | [x] |
| 31 | `driver` | extreme low boundary: `x == INT_MIN` with `y` ∈ {`INT_MIN`, `-1`, `0`, `1`, `4`, `5`, `37`} | [x] |
| 32 | `driver` | extreme mixed: `x` ∈ {`-1`, `0`, `1`, `2`, `3`, `4`} × `y` ∈ {`INT_MIN`, `INT_MIN+1`} (guard-false or non-terminating classification per the termination law) | [x] |
| 33 | `driver` | large magnitudes: `x`, `y` randomized in `[10_000, 60_000]` (long output streams, many S5 mode flips). `INT_MAX` itself is excluded only for wall-clock reasons — it would emit ~2^31 lines; it is in the same equivalence class as this row. | [x] |
| 34 | `driver` | **exhaustive** small grid: every `(x, y)` in `[-6, 12] × [-6, 12]` (361 pairs), the interaction cross-product at full density | [x] |
| 35 | `driver` | randomized whole-domain sweep: `(x, y)` uniform over `[-64, 512]²`, 4000 pairs, terminating ones compared byte-for-byte | [x] |
| 36 | `driver` | repeated-call / residual-state check: the same loaded `.so` handle called many times in sequence with different shapes, asserting no cross-call state leaks (C has no `static` state; the Rust must not introduce any) | [x] |

## Harness

`tests/phase_b_configs.rs` (36 tests, one per row) plus `tests/support/mod.rs`.

Both implementations are loaded with `libloading` and called **only** through
their exported `driver` symbol — the Rust side is never called as a Rust
function, so the `#[no_mangle] extern "C"` wrapper is under test too. `dlopen`
keys on the resolved path and `libloading` uses `RTLD_LOCAL`, so the two
identically-named `libdriver.so` files do not collide.

Because `driver` returns `void`, the harness captures fd 1 around each call and
compares the resulting bytes. Two buffering layers must be drained before the
redirect: libc's `stdout` FILE (used by both `.so`s) *and* `std::io::Stdout`'s
own userspace buffer, which libtest fills with progress text. The suite also
**requires a single test thread**, since libtest writes status lines to fd 1
from the test-driving thread; `tests/support/mod.rs` asserts this rather than
leaving it as a flake.

## How to run

```sh
./run_tests.sh        # build C + Rust, check symbols, run the suite vs debug AND release .so
./check_symbols.sh    # Phase A/D symbol parity only
./check_features.sh   # Phase D: every feature combination
./mutation_check.sh   # proves the suite actually detects divergence
```

## Result

All 36 rows pass, against **both** the debug and the release Rust `.so` (release
is a distinct artifact: it is built with `panic = "abort"`).

`./mutation_check.sh` breaks the Rust translation nine different ways — dropping
the `x == 1 && y == 4` forward `goto`, letting that skip persist, moving the
`x < 3` back-edge boundary, making the back-edge re-test the `while` guard,
turning the `y == 0` `continue` into a `break`, changing `||` to `&&` in the loop
guard, loosening the `label1` guard, returning early on the divergent class, and
changing a `printf` string — and the suite detects **all nine**. Green results
here therefore mean something.
