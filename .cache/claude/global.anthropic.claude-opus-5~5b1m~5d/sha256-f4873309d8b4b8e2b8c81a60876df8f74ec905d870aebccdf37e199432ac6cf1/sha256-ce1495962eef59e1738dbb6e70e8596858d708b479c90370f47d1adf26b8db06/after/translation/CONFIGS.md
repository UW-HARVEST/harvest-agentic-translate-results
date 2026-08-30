# CONFIGS.md — Configuration / valid-input surface table

Mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from what
`c_src/src/driver.c` actually branches on.

## Axes the C code actually distinguishes

This library exposes **no runtime options, modes or flags** — there is no
config struct, no setter, no `enum`, no global state, and no `#ifdef` in
`driver.c`. Grepping the source for `#if`/`#ifdef` returns nothing, and
`Cargo.toml` declares no `[features]`. So the configuration surface is entirely
the **cross-product of entry point × input shape**:

- **Axis 1 — entry point (all 5 exported symbols, low-level first):**
  `printLine` and `printIntLine` (lowest level, the output primitives) →
  `bad` and `good` (mid level, the arithmetic) → `driver` (top-level composed
  pipeline). The mid/low-level symbols are driven **directly**, not only via
  the `driver` wrapper, because a bug in the composed pipeline (wrong call
  order, missing line) is invisible to per-wrapper tests and vice versa.
- **Axis 2 — `const char *` shape** (`printLine`): NULL / empty / 1 byte /
  short ASCII / oversized (8 KiB) / contains `printf` conversion specifiers /
  contains embedded newlines / non-UTF-8 bytes.
- **Axis 3 — `int` shape** (`printIntLine`): `0` / `±1` / `INT_MAX` / `INT_MIN`
  / randomized full-range.
- **Axis 4 — `float` shape** (`bad`, `good`, `driver`): the classes the
  `100.0/data` division and the subsequent `(int)` cast treat differently —
  exact quotient / truncating quotient / negative (truncation toward zero) /
  quotient magnitude `< 1` (rounds to 0) / quotient overflowing `int` /
  `±0.0` / `±inf` / `NaN` / subnormal.
- **Axis 5 — `goodB2G` threshold branch** (`good`, `driver`): `fabs(data) >
  0.000001` true vs false, plus the two values straddling the threshold.

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed seed
`0x5EED_D1FF_1234_5678`, deterministic xorshift64\*) where the row describes a
value *class* rather than a single constant, and asserted byte-for-byte against
the C `.so`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|------------------------------------------|-----|
| C1  | `printLine` | short printable ASCII strings, randomized length 1..64 and randomized bytes | [x] |
| C2  | `printLine` | empty string `""` (zero length, NUL-terminated) | [x] |
| C3  | `printLine` | single-byte strings, every value `0x01..0xFF` (incl. non-UTF-8) | [x] |
| C4  | `printLine` | oversized: 8 KiB string, exceeds any plausible internal buffer | [x] |
| C5  | `printLine` | string containing `%d %s %n %%` conversion specifiers (must print verbatim) | [x] |
| C6  | `printLine` | string containing embedded `\n` and `\t` (output has multiple lines) | [x] |
| C7  | `printIntLine` | boundary integers: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `±2147483647` | [x] |
| C8  | `printIntLine` | randomized full 32-bit range, 512 samples | [x] |
| C9  | `bad` | `data` giving an **exact** quotient (`2.0` → `50`, `4.0` → `25`, `100.0` → `1`) | [x] |
| C10 | `bad` | `data` giving a **truncating** positive quotient (`3.0` → `33`, `7.0` → `14`) | [x] |
| C11 | `bad` | `data` **negative** — truncation toward zero (`-3.0` → `-33`, `-2.0` → `-50`) | [x] |
| C12 | `bad` | `\|quotient\| < 1` so the cast yields `0` (`data` = `1e3`, `1e6`, `1e30`; also negative) | [x] |
| C13 | `bad` | quotient **overflows** `int` (tiny `data`: `1e-8`..`1e-45`, both signs) | [x] |
| C14 | `bad` | `data` = `±0.0`, `±inf`, `NaN`, `FLT_MIN`, `FLT_MAX`, `±1.0`, subnormals | [x] |
| C15 | `bad` | randomized normal floats across ~20 decades, 1024 samples, both signs | [x] |
| C16 | `bad` | randomized **bit patterns** reinterpreted as `f32` (512 samples) — covers NaN payloads, subnormals and infinities without bias | [x] |
| C17 | `good` | threshold branch **TRUE**: `fabs(data) > 0.000001` — prints `50` then the quotient | [x] |
| C18 | `good` | threshold branch **FALSE**: `data` = `0.0`, `-0.0`, `1e-9`, `NaN` — prints `50` then the message | [x] |
| C19 | `good` | threshold **straddle**: `1e-6f`, `1.0000001e-6f`, `9.9e-7f`, `1.1e-6f`, `-1e-6f` | [x] |
| C20 | `good` | randomized floats (mixed magnitudes) so both branches are hit repeatedly, 1024 samples | [x] |
| C21 | `good` | randomized raw bit patterns as `f32`, 512 samples | [x] |
| C22 | `driver` | composed pipeline, `goodData` branch TRUE × `badData` normal — full 6-line transcript, verifies call **order** | [x] |
| C23 | `driver` | `goodData` branch FALSE × `badData` normal | [x] |
| C24 | `driver` | `goodData` branch TRUE × `badData` = `0.0` (the CWE-369 divide-by-zero path) | [x] |
| C25 | `driver` | `goodData` branch FALSE × `badData` = `0.0` / `NaN` / `±inf` | [x] |
| C26 | `driver` | full cross-product of a 12-value "interesting float" set × itself (144 combinations) | [x] |
| C27 | `driver` | randomized `(goodData, badData)` pairs, 512 samples | [x] |
| C28 | interleaving | `printLine` / `printIntLine` / `bad` / `good` / `driver` called **repeatedly in one capture**, verifying no cross-call state, ordering or buffering divergence | [x] |
