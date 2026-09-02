# CONFIGS.md — Configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the C
source, not from guesses about what matters.

## Axes the C code actually branches on

Enumerated from `c_src/include/driver.h` (the entire public header) and
`c_src/src/driver.c`:

- **Runtime options / modes / flags: none.** The header declares one function
  and no setters, no context struct, no globals, no environment lookups. Grep
  for `if` / `switch` / `#ifdef` in the function body yields 0 hits. There is
  no option cross-product to take.
- **Public entry points: exactly one — `driver`.** It is simultaneously the
  highest- and lowest-level entry point; there is no convenience wrapper hiding
  a lower layer, and no internal helper with external linkage (`nm -D` shows
  only `driver`). So "exercise the lowest-level entry points, not just the
  wrappers" collapses to: call `driver` directly through the `.so`, which is
  what every row does.
- **Input shapes.** Both parameters are `int` by value, so the shape axes are
  purely value-domain ones. The behaviourally distinct axes are the ones
  `idiv` + `%d` formatting distinguish:
  - `sign(x)` ∈ {negative, zero, positive} — C division truncates toward zero
    and `x % y` takes the sign of `x`, so all four sign quadrants differ.
  - `sign(y)` ∈ {negative, positive} (`y == 0` is the fatal row 1/2 of
    `ERRORS.md`, excluded here).
  - `|x|` vs `|y|`: `|x| < |y|` (quotient 0, remainder `x`) vs `|x| >= |y|`.
  - divisibility: `x % y == 0` vs `!= 0`.
  - `|y| == 1` (quotient is `±x`, remainder always 0) — the degenerate divisor.
  - magnitude class: near-zero small values, mid-range, and the `INT_MIN` /
    `INT_MAX` extremes (which also drive the widest `%d` output,
    `"-2147483648"`, i.e. the longest byte string the `printf` can emit).
  - power-of-two vs non-power-of-two `|y|` — same C semantics, but a distinct
    codegen path worth pinning since the Rust uses hand-written `idiv` asm.

The rows below are the pruned cross-product of those axes: one row per
combination the C treats differently. Every row is driven with **many
randomized inputs** from a fixed-seed SplitMix64 PRNG (seed `0x5EED_1234_ABCD`),
not one hand-picked value, and asserts the two `.so`s' stdout bytes and exit
statuses are identical.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options (none exist); `x == 0`, `y == 1` — zero numerator, identity divisor | [x] |
| 2 | `driver` | `x == 0`, `y` random non-zero (both signs) — zero numerator, arbitrary divisor | [x] |
| 3 | `driver` | `x > 0`, `y > 0`, exactly divisible (`x % y == 0`), random | [x] |
| 4 | `driver` | `x > 0`, `y > 0`, **not** divisible, random | [x] |
| 5 | `driver` | `x > 0`, `y < 0`, random (quotient negative, remainder `>= 0`) | [x] |
| 6 | `driver` | `x < 0`, `y > 0`, random (quotient negative, remainder `<= 0`) | [x] |
| 7 | `driver` | `x < 0`, `y < 0`, random (quotient positive, remainder `<= 0`) | [x] |
| 8 | `driver` | `y == 1`, `x` random full-range — quotient `== x`, remainder 0 | [x] |
| 9 | `driver` | `y == -1`, `x` random full-range **excluding `INT_MIN`** — pure negation (`INT_MIN` here is `ERRORS.md` row 3) | [x] |
| 10 | `driver` | `\|x\| < \|y\|`, all four sign quadrants, random — quotient 0, remainder `== x` | [x] |
| 11 | `driver` | `x == INT_MAX`, `y` random non-zero (both signs) | [x] |
| 12 | `driver` | `x == INT_MIN`, `y` random non-zero and `!= -1`; plus `INT_MIN+1`, `INT_MAX-1` neighbours | [x] |
| 13 | `driver` | `y == INT_MAX` and `y == INT_MIN`, `x` random full-range | [x] |
| 14 | `driver` | `x`, `y` both uniform random full-range `i32` (`y != 0`, excluding `INT_MIN/-1`) — bulk property sweep, 4000 pairs | [x] |
| 15 | `driver` | `x`, `y` random small magnitude (`\|v\| <= 64`, `y != 0`) — dense near-zero coverage incl. all sign quadrants and `\|y\|==1` | [x] |
| 16 | `driver` | `\|y\|` an exact power of two (`2^0 … 2^30`, both signs), `x` random full-range — distinct codegen path from the hand-written `idiv` | [x] |
| 17 | `driver` | extremal grid: `x ∈ {INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` × `y ∈ same set minus 0`, exhaustive, minus the `ERRORS.md` row-3 pair | [x] |
| 18 | `driver` | maximum-width `printf` output: quotient `== INT_MIN` (`x == INT_MIN`, `y == 1`) and remainder of maximum width (`x == INT_MIN+1`, `y == INT_MAX`) — longest byte strings the format can emit | [x] |
| 19 | `driver` | repeated / interleaved invocation in one process (1000 mixed calls back to back) — confirms no cross-call state and identical `stdout` buffering & flush ordering through the shared `FILE*` | [x] |

## Notes on how the rows are driven

- Both libraries are loaded **only** through `libloading` and called via the
  exported `driver` symbol — never by calling the Rust function directly — so
  the `#[unsafe(no_mangle)] extern "C"` wrapper is under test too.
- Each row runs in a freshly spawned child process per library, with stdout
  captured as a pipe, so the comparison is on raw bytes including the trailing
  newline and glibc's full buffering behaviour.
- Every row is executed against **both** Rust artifacts,
  `target/debug/libdriver.so` and `target/release/libdriver.so` (the release
  profile sets `panic = "abort"`, a materially different configuration).
