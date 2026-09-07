# CONFIGS.md — Configuration-surface table

## How this table was derived

The C has no runtime options, no flags, no modes, no `#ifdef`, no global state
and no setters:

```
grep -nE '#ifdef|#if |#ifndef|static|extern|global|_Bool|flag|mode|option|set[A-Z_]' \
     c_src/src/driver.c c_src/include/driver.h
```
matches only the header's `#ifndef DRIVER_H_` include guard. The public API is
one entry point, `void driver(int, int)`, which is simultaneously the *lowest*
level entry point and the only one — there are no convenience wrappers to skip
past and no deeper layer to reach.

So the configuration axes are entirely in the **input shape**, i.e. the operand
pairs that `div()` and `printf("%d")` treat differently. Derived from the two
operations the C delegates to:

**Axis 1 — sign combination** (`idiv` truncates toward zero, and C99 fixes
`rem` to take the sign of the *numerator*; all four sign quadrants are distinct
code paths in the sense that they produce different sign patterns in the output,
and are the classic place a translation using floor-division diverges):
`(+,+)`, `(+,-)`, `(-,+)`, `(-,-)`.

**Axis 2 — magnitude relation** (determines whether the quotient is zero):
`|x| < |y|`, `|x| == |y|`, `|x| > |y|`.

**Axis 3 — divisibility** (determines whether the remainder is zero, and
whether the sign-of-remainder rule is even observable — it is invisible when
`rem == 0`, which is why exact and inexact must be separate rows):
`x % y == 0` vs `x % y != 0`.

**Axis 4 — special divisors the hardware/ABI treats specially**:
`y == 1` (identity), `y == -1` (negation — adjacent to the row-2 trap in
`ERRORS.md`), `y == INT_MIN`, `y == INT_MAX`.

**Axis 5 — special numerators**: `x == 0` (both outputs zero),
`x == INT_MIN`, `x == INT_MAX`.

**Axis 6 — `printf("%d")` formatting shape**, the *output* axis: number of
digits (1, 2, 10), presence of a `-` sign in the quotient field, in the
remainder field, in both, in neither. `%d` of `INT_MIN` is the widest possible
field (`-2147483648`, 11 chars) and is the only input where the digits cannot
be produced by negating into a positive `int`.

The table below is the cross-product of axes 1–6, pruned to the combinations
the code actually distinguishes (e.g. `|x| == |y|` forces `rem == 0`, so
`|x|==|y|` × `inexact` is impossible and omitted; `y == 1` forces `rem == 0`
likewise).

Every row is exercised with **many randomized inputs** drawn from that row's
region (fixed-seed SplitMix64 PRNG, so runs are reproducible), not one
hand-picked value, plus the row's boundary values. Both `.so`s are called
through their exported `driver` symbol and the captured stdout bytes are
compared with `assert_eq!` on `Vec<u8>`.

## The table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `(+,+)`, `\|x\| > \|y\|`, inexact — 256 random pairs, `x,y ∈ [1, INT_MAX]`, `y < x` | [x] |
| 2 | `driver` | `(+,+)`, `\|x\| > \|y\|`, exact (`x = y*k`) — 256 random pairs | [x] |
| 3 | `driver` | `(+,+)`, `\|x\| < \|y\|` → quotient 0, remainder `x` — 256 random pairs | [x] |
| 4 | `driver` | `(+,+)`, `\|x\| == \|y\|` → `1, 0` — 256 random pairs | [x] |
| 5 | `driver` | `(+,-)`, `\|x\| > \|y\|`, inexact → negative quotient, **positive** remainder (C99 truncation) — 256 random pairs | [x] |
| 6 | `driver` | `(+,-)`, `\|x\| > \|y\|`, exact → negative quotient, remainder 0 — 256 random pairs | [x] |
| 7 | `driver` | `(+,-)`, `\|x\| < \|y\|` → `0, x` (quotient is `0`, not `-0`/`-1`) — 256 random pairs | [x] |
| 8 | `driver` | `(-,+)`, `\|x\| > \|y\|`, inexact → negative quotient, **negative** remainder — 256 random pairs | [x] |
| 9 | `driver` | `(-,+)`, `\|x\| > \|y\|`, exact — 256 random pairs | [x] |
| 10 | `driver` | `(-,+)`, `\|x\| < \|y\|` → `0, x` with `x < 0` — 256 random pairs | [x] |
| 11 | `driver` | `(-,-)`, `\|x\| > \|y\|`, inexact → **positive** quotient, negative remainder — 256 random pairs | [x] |
| 12 | `driver` | `(-,-)`, `\|x\| > \|y\|`, exact — 256 random pairs | [x] |
| 13 | `driver` | `(-,-)`, `\|x\| < \|y\|` → `0, x` — 256 random pairs | [x] |
| 14 | `driver` | `(-,-)`, `\|x\| == \|y\|` → `1, 0` — 256 random pairs | [x] |
| 15 | `driver` | `x == 0`, `y` random nonzero (both signs) → `0, 0` for every `y` — 256 random pairs | [x] |
| 16 | `driver` | `y == 1`, `x` random over the full `int` range incl. `INT_MIN`/`INT_MAX` → `x, 0` | [x] |
| 17 | `driver` | `y == -1`, `x` random over `[INT_MIN+1, INT_MAX]` (excluding the `ERRORS.md` row-2 trap) → `-x, 0`; includes `x = INT_MIN+1` and `x = INT_MAX` | [x] |
| 18 | `driver` | `y == INT_MIN`, `x` random over the full range → quotient 0 except `x == INT_MIN` (`1, 0`); the one case where `\|y\|` is not representable | [x] |
| 19 | `driver` | `y == INT_MAX`, `x` random over the full range, both signs | [x] |
| 20 | `driver` | `x == INT_MIN`, `y` random over `[INT_MIN, -2] ∪ [1, INT_MAX]` (excluding `y == -1`, `y == 0`) — the widest `%d` field in the numerator | [x] |
| 21 | `driver` | `x == INT_MAX`, `y` random nonzero, both signs | [x] |
| 22 | `driver` | `printf` formatting: quotient and remainder both `INT_MIN`-width / 10-digit, 1-digit, 2-digit; sign in quotient only, remainder only, both, neither — 5 targeted pairs + 256 random full-range pairs | [x] |
| 23 | `driver` | unconstrained fuzz: 20 000 pairs with `x, y` uniform over the entire `u32` bit-pattern space reinterpreted as `i32`, skipping only the two `ERRORS.md` trap conditions — the anti-blind-spot row that does not assume which regions matter | [x] |
| 24 | `driver` | **repeated / stateful invocation**: 512 consecutive `driver` calls into one captured stream without an intervening flush, comparing the whole concatenated stdout. Catches any divergence in `stdout` buffering discipline, ordering, or a missing/extra trailing newline that a single-call test cannot see. | [x] |
| 25 | `driver` | same 512-call sequence with fd 1 a **pipe** (non-seekable, so glibc selects full block buffering) instead of a regular file — glibc picks the buffering mode from the fd type, so this is a real branch in the observable behavior of the shared `printf`. Both libs are driven through the identical pipe setup and the drained bytes compared. | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so there is exactly one
configuration. Enumerated mechanically:

```
sed -n '/\[features\]/,/^\[/p' Cargo.toml    # → no output
```

The suite is nevertheless run under both `--no-default-features` and the
default build by `run_all_configs.sh`, and both must be green.

## Binary executable

None — see `SYMBOLS.md`. `c_src/CMakeLists.txt` builds only a shared library
and `driver.c` has no `main`; `Cargo.toml` has no `[[bin]]`. stdout is compared
through the libraries instead (rows 1–25 all assert on captured stdout bytes).
