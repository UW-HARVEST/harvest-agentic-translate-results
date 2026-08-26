# CONFIGS.md — configuration-surface table (Phase B)

## Mechanical derivation

Axes are derived from what the C actually branches on / distinguishes, over the
complete C source (`c_src/include/driver.h`, `c_src/src/driver.c`):

* **Public entry points** — grep of the installed header gives the *full* set:
  ```
  $ grep -n '(' c_src/include/driver.h
  27: void driver(int x);
  ```
  There is exactly **one** public entry point, `driver`. It is simultaneously the
  convenience wrapper and the lowest-level function — there is no deeper layer to
  drive, and `nm -D` confirms nothing else is exported.
* **Runtime options / modes / flags** — none: no setters, no context/handle
  struct, no globals, no environment lookups, no `#ifdef`-gated modes (only the
  `DRIVER_H_` include guard). So there is no option cross-product.
* **Input shapes** — the single parameter is `int x`. The shapes the code
  distinguishes are the arithmetic/formatting classes of `x`:
  `2*x` in range vs. wrapping (positively / negatively), `y += 300` in range vs.
  wrapping, sign of the printed result (`%d` emits a `-`), and the decimal width
  of the printed result (1…10 digits, plus sign). Call-sequence shapes matter for
  the one shared resource the function touches (libc `stdout`): zero / one / many
  calls, repeated values, interleaving with the other library's calls, and the
  stdout buffering mode (file vs. pipe).

Every row is exercised with **many randomized inputs** from a fixed-seed PCG-XSH-RR
generator (seed `0x5EED_1234_ABCD_0001`) unless the row is definitionally an
explicit list; both `.so`s are called through `libloading` and their stdout byte
streams are compared byte-for-byte.

## Configuration surface

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | degenerate/empty shape: zero calls (no output at all) | [x] |
| C2 | `driver` | `x = 0` (single call, degenerate value) | [x] |
| C3 | `driver` | one call only, single small positive `x ∈ [1, 100]`, randomized | [x] |
| C4 | `driver` | one call only, single small negative `x ∈ [-100, -1]`, randomized | [x] |
| C5 | `driver` | many calls, `x ∈ [-149, -1]` → printed result is small positive (1…299): output-width shapes 1–3 digits | [x] |
| C6 | `driver` | `x = -150` exactly → printed result is `0` (sign-boundary of the output) | [x] |
| C7 | `driver` | many calls, `x ∈ [INT_MIN/2, -151]` → printed result negative, no wrap: `%d` emits `-` sign, randomized | [x] |
| C8 | `driver` | many calls, `x ∈ [1, INT_MAX/2]` → `2*x` in range, `y+300` in range: plain positive path, randomized | [x] |
| C9 | `driver` | many calls, `x ∈ (INT_MAX/2, INT_MAX]` → `2*x` wraps positively→negative, randomized | [x] |
| C10 | `driver` | many calls, `x ∈ [INT_MIN, INT_MIN/2)` → `2*x` wraps negatively→positive, randomized | [x] |
| C11 | `driver` | many calls, `x` near the `y += 300` overflow edge (`x ∈ [1073741600, 1073741900]`), where the *addition* (not the multiply) wraps | [x] |
| C12 | `driver` | many calls, `x` uniform over the **entire** `i32` range (all 32-bit patterns reachable), randomized | [x] |
| C13 | `driver` | output decimal-width sweep: one `x` chosen per printed-width class (1,2,3,4,5,6,7,8,9,10 digits, and negative with 1…10 digits) | [x] |
| C14 | `driver` | explicit boundary list: `INT_MIN, INT_MIN+1, -1073741825, -1073741824, -1073741823, -151, -150, -149, -1, 0, 1, 1073741673, 1073741674, 1073741823, 1073741824, INT_MAX-1, INT_MAX` | [x] |
| C15 | `driver` | exhaustive contiguous sweep of 200 000 consecutive `x` values (value-dependent bugs), one buffered stdout window | [x] |
| C16 | `driver` | repeated identical calls (same `x` 1 000×) — no hidden per-call state | [x] |
| C17 | `driver` | interleaved call sequence C→Rust→C→Rust… inside a single stdout window (shared libc `stdout` buffer, interleaving/flush behaviour) | [x] |
| C18 | `driver` | stdout is a **pipe** instead of a regular file (different libc buffering decision), small randomized batch | [x] |
| C19 | `driver` | called from a **non-main thread** (no thread-local/TLS assumptions), randomized batch | [x] |
| C20 | `driver` | large single batch (100 000 randomized calls) crossing many `BUFSIZ` flush boundaries in one window | [x] |
| C21 | `driver` | systematic strided sweep of the **entire** `i32` domain (stride 65 521, 65 552 values — every high-16-bit combination) | [x] |

All rows are implemented in `tests/differential.rs` (module `configs`) and pass
for both `.so`s under every feature combination (see `SYMBOLS.md` — the feature
set is empty, so combinations #1 and #2 are the whole space).
