# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

### Public entry points (the FULL set, lowest level included)

From `nm -D` / the C source, not from the header alone (the header only declares
one of the three):

| entry point | kind | source | level |
|---|---|---|---|
| `array` | mutable global data symbol, `int[262144]` | `long.c:33` | **lowest** — the state every function reads/writes |
| `perform_expensive_operations` | function, no args, no return | `long.c:36` | **low-level worker**, not in `long.h` but exported |
| `long_exec` | function, `unsigned int seed` | `long.h:27`, `long.c:49` | high-level one-shot wrapper |

`long_exec` is the convenience/one-shot wrapper: it merely seeds `array` from
`rand()`, calls the worker `ITERATIONS` times, and folds the result. The rows
below therefore drive `array` + `perform_expensive_operations` **directly** for
the bulk of the coverage, because one `long_exec` run collapses 5.24e10
`step()` evaluations into a single printed integer and can only ever exercise
one starting distribution (glibc `rand()` output, i.e. non-negative values
`< 2^31`). Driving the worker directly is the only way to reach negative
starting values, `INT_MIN`/`INT_MAX`, and specific residue classes.

### Runtime options / modes / flags

```
$ grep -n '#if\|#ifdef\|switch\|if *(\|getenv\|static .*flag\|bool' c_src/src/long.c
(no matches other than the ECHO_H_ include guard in the header)
```

**There are no runtime options, modes, or flags.** No `setopt`-style API, no
environment variables, no `#ifdef` build variants. Likewise
`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one Rust feature combination (the default, which is also
`--no-default-features`). This is asserted mechanically by
`tests/symbols.rs::features_surface_is_empty`.

The compile-time constants that *would* be the options are fixed:

| constant | value | effect |
|---|---|---|
| `ARRAY_SIZE` | `256 * 1024` = `262144` | element count of `array`; outer loop bound |
| `ITERATIONS` | `2000` | number of `perform_expensive_operations` calls in `long_exec` |
| inner loop bound | `100` | `step()` applications per `perform_expensive_operations` call per element |

### Input shapes the code's arithmetic actually distinguishes

`step()` is `x*3+7`, `x ^= x>>3`, `x -= x<<1`, `x = x/2 + x%7`. Although there
are no `if`s, these operators are **value-dependent** at machine level, so the
"shapes" are the value classes that select a different machine behaviour:

* **sign of `x`** — selects `sar` sign-extension in `x>>3`, the truncation
  *direction* of `x/2`, and the *sign of the remainder* `x%7`
* **overflow occurrence** — in `x*3+7`, in `x<<1`, in `x - (x<<1)`
* **`x % 7` residue class** — 13 distinct outcomes (`-6..=6`)
* **`x` magnitude class** — `0` (a fixed point), `±1`, small, `±2^30`, `±2^31`
  boundary, `INT_MIN`/`INT_MAX`
* **position within `array`** — the Rust worker processes elements in
  `LANES = 8` chunks (`chunks_exact_mut(8)`) whereas the C walks one element at
  a time, so element index mod 8, and the `chunks_exact` remainder path, are
  genuine Rust-only code-path axes that must be shown equivalent
* **count of elements exercised** — empty(all-zero) / one distinct value / many
* **number of composed worker calls** — 0 / 1 / 2 / many (`f^100n`)
* **`seed` value** for the wrapper — 0 / 1 / small / `INT_MAX`-adjacent /
  `0x80000000` / `UINT_MAX`

## Table

Every row is driven through the `.so` exports of **both** libraries via
`libloading` and compared byte-for-byte over the whole 1 MiB `array` (and over
captured `stdout` where applicable). "randomised" rows use a fixed-seed
SplitMix64 generator so runs are reproducible; each such row runs many
independent trials (`262144` values per trial).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `array` (data symbol only) | ABI shape: `st_size == 0x100000` in both, element stride 4, `.bss`/zero-initialised at load, writable through `dlsym` | [x] |
| 2 | `array` + `perform_expensive_operations` | pristine load state: array untouched (all zeros) → 1 call. Ground truth: `0` is *not* a fixed point (`step(0) == -3`); one call yields `-626538949` everywhere | [x] |
| 3 | `array` + `perform_expensive_operations` | all elements the same single value, swept over every "magnitude class": `0, 1, -1, 2, -2, 3, -3, 7, -7, 8, -8, 2^30, -2^30, INT_MAX, INT_MAX-1, INT_MIN, INT_MIN+1` (one trial per value, whole array uniform) | [x] |
| 4 | `array` + `perform_expensive_operations` | one distinct value in element 0 only, rest zero — isolates a single lane of the Rust `chunks_exact(8)` path (`i % 8 == 0`) | [x] |
| 5 | `array` + `perform_expensive_operations` | single non-zero element placed at each offset `i % 8 == 0..7` and at `i = ARRAY_SIZE-1` — covers every lane position of the Rust `LANES = 8` batching against the C's scalar walk | [x] |
| 6 | `array` + `perform_expensive_operations` | **all non-negative** inputs (`rand()`-like, `0 <= x < 2^31`), randomised, many trials — the distribution `long_exec` actually produces | [x] |
| 7 | `array` + `perform_expensive_operations` | **all negative** inputs (`INT_MIN <= x < 0`), randomised, many trials — exercises `sar`, negative truncating division, negative remainder | [x] |
| 8 | `array` + `perform_expensive_operations` | **full-range** inputs (uniform over all 2^32 bit patterns), randomised, many trials — mixed signs *within* a single 8-lane Rust chunk | [x] |
| 9 | `array` + `perform_expensive_operations` | inputs drawn only from the **13 residue classes of `% 7`** (`x ≡ -6..6 mod 7`, with mixed signs) | [x] |
| 10 | `array` + `perform_expensive_operations` | inputs drawn only from **overflow-triggering extremes**: `INT_MAX`, `INT_MIN`, `±2^30`, `±(2^31-1)/3`-adjacent values, randomised placement | [x] |
| 11 | `array` + `perform_expensive_operations` | **boundary-value stripe**: array filled by cycling a hand-picked edge-case table so that every 8-lane chunk contains a different mix of edge cases | [x] |
| 12 | `array` + `perform_expensive_operations` | **composition**: randomised full-range fill, then `n` back-to-back worker calls for `n = 0,1,2,3,5,8,13` — compares after *each* call (catches drift that a single call hides) | [x] |
| 13 | `array` + `perform_expensive_operations` | **long composition**: 40 consecutive calls (= `f^4000`) on a randomised fill, compared after each call. Measured ground truth: the orbit does **not** converge — essentially every element still changes on the next call, so the composed pipeline stays value-sensitive all the way to `ITERATIONS = 2000` and cannot be short-circuited | [x] |
| 14 | `array` + `perform_expensive_operations` | **cross-library state independence**: C's `array` written, Rust's `array` left zero, worker run on both — confirms each `.so` has its own private `array` copy and neither reads the other's | [x] |
| 15 | `array` + `perform_expensive_operations` | **one-past-the-end guard**: canary bytes immediately after `array`'s 1 MiB extent checked untouched after a call, in both libraries | [x] |
| 16 | `long_exec` | seeding sub-behaviour, isolated: `srand(seed); array[i] = rand()` for `seed = 0, 1, 2, 42, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFE, 0xFFFFFFFF` — asserts both `.so`s consume the *same* glibc `rand()` stream in the same order | [x] |
| 17 | `long_exec` | full end-to-end wrapper, `seed = 1`: `srand` → 262144 `rand()` → `ITERATIONS = 2000` worker calls → `xor` fold → `printf("%d\n")`; captured `stdout` compared byte-for-byte (expensive: ~470 s C + ~56 s Rust, `#[ignore]`d, run explicitly) | [x] |
| 18 | `long_exec` | full end-to-end wrapper, second seed (`seed = 0`) to prove the output is seed-dependent and not a constant, plus re-entry after a dirty `array` (row 17 leaves state) | [x] |
| 19 | `long_exec` + `perform_expensive_operations` | **equivalence-shortcut cross-check**: the `xor` fold + `printf` formatting of `long_exec` reproduced by driving the worker directly, confirming the wrapper's own glue (fold order, `%d` formatting of a possibly negative value) matches | [x] |

Rows 1–16, 18(partial), 19 run in the fast default `cargo test`. Row 17 and the
full-run part of row 18 are `#[ignore]`d and driven by
`run_full_long_exec.sh`, which runs the C and Rust halves as two concurrent
background processes so neither command exceeds the time budget.
