# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation

### Runtime options / modes / flags

```
$ grep -nE 'if *\(|switch|#ifdef|#if |enum|struct|extern|set[A-Z_]|_flag|option|mode' \
      c_src/include/driver.h c_src/src/driver.c
<no matches other than the #ifndef DRIVER_H_ include guard>
```

There are **none**. The public header declares exactly one function,
`void driver(int x, int y);`. There is no init/config/context object, no
setter, no global, no environment variable read, and no compile-time
`#ifdef` that alters behaviour. So the configuration axes reduce entirely to
**input shape**.

### Feature combinations (Rust side)

```
$ grep -nE '^\[features\]|feature *=' translation/Cargo.toml translation/src/lib.rs
<no matches>
```

`translation/Cargo.toml` declares **no `[features]` table**, and no
`#[cfg(feature = …)]` appears in `src/`. The only Cargo feature axis is therefore
the empty set: `--no-default-features` and the default build are the *same*
compilation. Both are still run in Phase D for completeness.

### Full set of public entry points

`driver` is the only exported symbol (see `SYMBOLS.md`). It **is** the lowest-level
entry point — there is no convenience wrapper layered over a lower API, so
"exercise the low-level entry points directly" is satisfied by calling `driver`
itself via `dlsym`.

### Input shapes the code actually distinguishes

`driver` has no branches of its own, but the `div(3)` it calls, and the `%d`
conversions `printf` performs, do distinguish these axes:

* **sign of `x`** — negative / zero / positive (drives the sign of `quot` *and*
  independently the sign of `rem`; C99 truncates toward zero, so `rem` takes the
  sign of the *dividend*).
* **sign of `y`** — negative / positive.
* **magnitude relation** — `|x| < |y|` (quotient 0, remainder `x`) vs `|x| >= |y|`.
* **divisibility** — `x % y == 0` (remainder 0) vs non-zero remainder.
* **identity/negation divisors** — `y == 1`, `y == -1`.
* **extremes** — `x` or `y` equal to `INT_MIN` / `INT_MAX`, and their neighbours
  `INT_MIN+1` / `INT_MAX-1` (one step inside the boundary).
* **`%d` formatting width** — 1-digit, multi-digit, and the `-2147483648` case
  (11 characters, the only value whose negation is unrepresentable), because the
  printed *bytes* are the observable output.

## The table

One row per meaningful combination the C treats differently. Every row is driven
with **many pseudo-random inputs from a fixed seed** (SplitMix64, seed
`0x243F6A8885A308D3`), not a single hand-picked value, and compared byte-for-byte
between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | no options (none exist); `x > 0`, `y > 0`, **exactly divisible** (`x = k*y`) → `rem == 0` | [x] |
| C2 | `driver` | no options; `x > 0`, `y > 0`, **not divisible** → `quot > 0`, `rem > 0` | [x] |
| C3 | `driver` | no options; `x < 0`, `y > 0` → `quot <= 0`, `rem <= 0` (remainder takes dividend's sign) | [x] |
| C4 | `driver` | no options; `x > 0`, `y < 0` → `quot <= 0`, `rem >= 0` | [x] |
| C5 | `driver` | no options; `x < 0`, `y < 0` → `quot >= 0`, `rem <= 0` | [x] |
| C6 | `driver` | no options; `x == 0`, `y` any non-zero (both signs, incl. extremes) → `quot == 0, rem == 0` | [x] |
| C7 | `driver` | no options; `\|x\| < \|y\|`, all four sign combinations → `quot == 0`, `rem == x` | [x] |
| C8 | `driver` | no options; `y == 1`, `x` full range incl. `INT_MIN`/`INT_MAX` → `quot == x, rem == 0` | [x] |
| C9 | `driver` | no options; `y == -1`, `x` full range **excluding** `INT_MIN` → `quot == -x`, `rem == 0` (`x == INT_MIN` is error row E5) | [x] |
| C10 | `driver` | no options; `x == INT_MIN`, `y` random non-zero and `y != -1`, both signs → tests the 11-byte `-2147483648` formatting path and asymmetric-range division | [x] |
| C11 | `driver` | no options; `x == INT_MAX`, `y` random non-zero, both signs | [x] |
| C12 | `driver` | no options; `y == INT_MIN` with `x` random (incl. `x == INT_MIN`, `INT_MAX`, `0`) → `quot` is 0 or 1 or -1, `rem == x` except `x == INT_MIN` | [x] |
| C13 | `driver` | no options; boundary neighbours — `x, y` drawn from `{INT_MIN, INT_MIN+1, -2, -1, 1, 2, INT_MAX-1, INT_MAX}` cross-product (skipping `y == 0` and the `INT_MIN / -1` pair) → "one step past / inside" the range | [x] |
| C14 | `driver` | no options; **`x`, `y` uniform over the whole `i32` domain**, `y != 0`, `(x,y) != (INT_MIN,-1)` — 20 000 randomized pairs, catches value-dependent bugs no shaped row would pick | [x] |
| C15 | `driver` | no options; **repeated / interleaved invocation** — C and Rust `driver` called alternately many times in one process against the same shared libc `stdout`, verifying there is no hidden per-call state, no cached/leaked buffer, and identical stream-buffering behaviour across a composed sequence rather than one isolated call | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains only `add_library(driver SHARED src/driver.c)` —
there is **no `add_executable`**, so the C project builds **no driver binary** and
there is no stdout-of-binary comparison to make. `translation/Cargo.toml` likewise
declares only `crate-type = ["cdylib"]` with no `[[bin]]`. This gate is
**N/A / vacuously satisfied**; recorded here so it is not mistaken for skipped work.

## Result

All 15 rows pass in `translation/tests/valid_paths.rs`. Every row compares the
**exact stdout bytes** emitted by the C `.so` against those emitted by the Rust
`.so` for the identical argument pair.
