# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The whole public API is one entry point (`c_src/include/sieve.h`):

```c
void sieve(int start);
```

There is **no** init/config/teardown function, **no** options struct, **no**
global or `static` variable, **no** environment variable read, and **no**
`#ifdef` in the implementation, so there are **zero runtime option/mode/flag
axes**. `grep -nE 'if|switch|case|#if|static|extern|global' c_src/src/sieve.c`
finds exactly one branch and nothing else:

```
src/sieve.c:35:        if (val % 10 == 9) {
```

`sieve` is simultaneously the lowest-level and the only entry point — there is no
convenience wrapper to hide behind, so "exercise the low-level entry points
directly" is satisfied by calling `sieve` itself through each `.so`'s exported
symbol via `libloading`.

The axes the C code therefore actually distinguishes are all **input shape**
axes of the single `int` argument, read straight off the two operations the body
performs (`printf("%d\n", val)` and `val % 10 == 9` / `val++`):

* **A1 — last base-10 digit** of `val`: `printf`/`%` make the digit `9` special.
  Values `0..8` in the last digit continue the loop; `9` breaks. 10 distinct
  cases.
* **A2 — sign**: C's `%` truncates toward zero, so for `val < 0` the residue is
  in `{0,-1,..,-9}` and `== 9` is *unreachable*; the sign axis changes the
  termination condition qualitatively. 3 cases (negative / zero / positive).
* **A3 — magnitude / iteration count**: 1 iteration (already ends in 9), 2–10
  iterations (positive), and `|val| + 10` iterations (negative → "many"),
  which also drives the `printf` field width (1 digit … 10 digits + `-` sign)
  and crosses `printf`'s internal 4096-byte stdout buffer boundary many times.
* **A4 — representational boundaries**: `INT_MIN`, `INT_MAX`, `INT_MAX-8`
  (last value that terminates without overflow), and the 8-wide overflow window
  `[INT_MAX-7, INT_MAX]`.
* **A5 — call multiplicity**: one call vs. many calls in sequence (the composed
  pipeline: the library is stateless, so N calls must equal the concatenation of
  N single-call outputs — a per-call test cannot catch a state leak, this can).
* **A6 — stdout destination / buffering**: regular file (fully buffered, 4096 B)
  vs. pipe vs. closed fd. `printf` behaves differently in each and the C code
  ignores `printf`'s return value.

Every row below is a combination of A1–A6 that the C code treats differently.
Each row is exercised with **many randomized inputs drawn from that row's class
using a fixed seed** (SplitMix64, seed `0x5EEDC0DE_5IEVE01` truncated), not one
hand-picked value, and C vs Rust stdout is compared **byte-for-byte**.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | random inputs / row | [x] |
|---|----------------|--------------------------------------------|---------------------|-----|
| 1 | `sieve` | A1=9, A2=positive, A3=1 iteration, single digit: `val == 9` exactly (smallest immediate-break) | fixed + 1 | [x] |
| 2 | `sieve` | A1=9, A2=positive, A3=1 iteration, small multi-digit: `val ∈ {19,29,...,999}` ending in 9 | 200 | [x] |
| 3 | `sieve` | A1=9, A2=positive, A3=1 iteration, large: `val` ending in 9, `val ∈ [10^6, 10^9]` (7–10 digit field width) | 200 | [x] |
| 4 | `sieve` | A1∈{0..8}, A2=positive, A3=2..10 iterations, single digit: `val ∈ [0,8]` — all 9 values exhaustively | 9 (exhaustive) | [x] |
| 5 | `sieve` | A1∈{0..8}, A2=positive, A3=2..10 iterations, small: `val ∈ [10, 9999]` not ending in 9 (crosses a 10-boundary, e.g. 999→…, digit-width change mid-run) | 300 | [x] |
| 6 | `sieve` | A1∈{0..8}, A2=positive, A3=2..10 iterations, large: `val ∈ [10^6, 2·10^9]` not ending in 9 (9–10 digit width, near-max but non-overflowing) | 300 | [x] |
| 7 | `sieve` | A1 anything, A2=positive, A3 spans a **power-of-10 digit-width change** during the run: `val ∈ {8,98,998,9998,...,999999998}` and `val = 10^k - 2` | 10 (exhaustive) | [x] |
| 8 | `sieve` | A2=zero: `val == 0` (10 iterations, `0..9`) | 1 (exhaustive) | [x] |
| 9 | `sieve` | A2=negative, A1=−9 (the "looks like it ends in 9 but C's `%` says −9" case): `val ∈ {−9,−19,…,−9999}` | 200 | [x] |
| 10 | `sieve` | A2=negative, A1∈{0,−1..−8}, A3=small many-iteration: `val ∈ [−999, −1]` (output crosses the `-`→no-`-` sign transition and 0) | 300 | [x] |
| 11 | `sieve` | A2=negative, A3=**very** many iterations: `val ∈ [−200000, −50000]` (≈ 50k–200k lines, > 1 MiB, crosses `printf`'s 4096-byte buffer hundreds of times) | 20 | [x] |
| 12 | `sieve` | A4 boundary: `val == INT_MAX − 8 == 2147483639` (largest value that terminates; 1 iteration, 10-digit field) | 1 (exhaustive) | [x] |
| 13 | `sieve` | A4 boundary: `val ∈ [INT_MAX−7, INT_MAX]` — the 8-value signed-overflow window; unbounded output, compared as a 256 KiB stdout prefix in a forked child | 8 (exhaustive) | [x] |
| 14 | `sieve` | A4 boundary: `val ∈ {INT_MIN, INT_MIN+1, INT_MIN+2, INT_MIN+3}` (11-char `-2147483648` field, unbounded output, 256 KiB prefix) | 4 (exhaustive) | [x] |
| 15 | `sieve` | A3/A4 fully unconstrained: `val` = uniformly random `int32` over the whole 2^32 domain, restricted to the non-overflowing/terminating classes by construction (random `val` then clamped so the run is bounded) | 500 | [x] |
| 16 | `sieve` | A5=many calls: 50 randomized `val`s invoked **back to back on the same loaded `.so` handle**, output concatenated (statelessness / no leak between calls, composed pipeline) | 50 per repetition × 10 | [x] |
| 17 | `sieve` | A5=interleaved C/Rust calls on the **same process stdout** (C call, Rust call, C call, …) — verifies neither library perturbs the shared `FILE *stdout` buffer state the other depends on | 40 | [x] |
| 18 | `sieve` | A6=stdout is a **pipe** (line/full buffering differs from a regular file) rather than a temp file, moderate output | 30 | [x] |
| 19 | `sieve` | A6=stdout is a **closed fd** (every `printf` fails, return value ignored by C) — must still terminate with no output | 5 | [x] |
| 20 | `sieve` | A1×A2 cross-product sweep: for every last digit `d ∈ 0..9` and every sign, a randomized `val` with that exact residue class (`val % 10 == ±d`) — the pruned cross product of the two qualitative axes | 10 digits × 2 signs × 25 | [x] |

| 21 | `sieve` | Exhaustive cross-check: **every** value in `[-3000, 3000]` (all signs, all last digits, all carry patterns for small magnitudes) — 6001 consecutive inputs, ≈9M output lines | 6001 (exhaustive) | [x] |
| 22 | `sieve` | Exhaustive cross-check: **every** value in `[INT_MAX-2008, INT_MAX-8]` — the dense band running right up to the overflow boundary at 10-digit field width | 2001 (exhaustive) | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the only combination is the default
(empty) feature set; `check_features.sh` enumerates the feature list
mechanically from `Cargo.toml`, confirms it is empty, and runs the full suite
under `--no-default-features` and under `--all-features` as well.
