# CONFIGS.md — Configuration / valid-input surface table (Phase B)

## How the axes were derived

The C body is four statements; every axis below comes from one of them:

```c
void sieve(int val) {
    while (1) {
        printf("%d\n", val);        /* axis: %d rendering of val  */
        if (val % 10 == 9) {        /* axis: truncated remainder  */
            break;                  /* axis: exit taken / not     */
        }
        val++;                      /* axis: increment / overflow */
    }
}
```

### Runtime options / modes / flags

**None.** Grep for `#define`, `#ifdef`, `switch`, setters, globals, statics,
env-var reads: zero hits (only the `SIEVE_H_` include guard). The library has
no configuration object, no init function, no mode flag, and no hidden state.
The *entire* configuration space is the single `int` argument. Likewise
`translation/Cargo.toml` declares no `[features]`, so there is exactly one
build configuration.

### Full set of public entry points

`include/sieve.h` exposes exactly one, and it is simultaneously the
highest- and lowest-level entry point (there is no convenience wrapper layer
to hide behind):

| entry point | signature |
|-------------|-----------|
| `sieve` | `void sieve(int val)` |

### Input-shape axes the code actually distinguishes

| axis | distinct cases the C branches on / renders differently |
|------|--------------------------------------------------------|
| A. sign of `val` | negative (`%` yields ≤ 0 ⇒ exit test can never fire) · zero · positive |
| B. `val % 10` (truncated) | `9` (immediate exit) · `0..8` (positive: 1–9 more iterations) · `-9..-1` and `0` (negative: never exits) |
| C. iteration count | 1 (already ends in 9) · 2..10 (positive) · `10 - val` (negative, unbounded in magnitude) |
| D. `%d` field width / digit count | 1 digit · 2 · 3 · 10 digits · with `-` sign · `INT_MIN` (`-2147483648`) |
| E. decimal carry during the run | run stays within one digit count (`3→9`) vs. crosses a power of ten (`-1→0`, `98→99`, `999999998→999999999`) |
| F. proximity to `INT_MAX` | terminates below the wrap (`≤ 2147483639`) vs. overflow-wrap region (`≥ 2147483640`, see ERRORS.md rows 5–6) |
| G. call multiplicity | 0 calls · 1 call · many calls in sequence (state independence) |
| H. stdout destination / buffering | regular file (fully buffered) · pipe (block buffered) · closed fd |

Rows below are the pruned cross-product: one row per combination the C code
treats differently. Every row is exercised with **many randomized inputs
drawn from a fixed-seed PCG-XSH-RR generator** (except rows pinned to one
specific boundary value), and both libraries are called through their `.so`
exports via `libloading`, in a child process whose fd 1 is a file, so the
compared artifact is the raw byte stream.

## The table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `sieve` | A=positive, B=9, C=1 — immediate exit on the *first* iteration; `val = 9` | `cfg_01_single_digit_nine` | [x] |
| 2 | `sieve` | A=zero, B=0, C=10, E=crosses no power of ten; `val = 0` | `cfg_02_zero` | [x] |
| 3 | `sieve` | A=positive, B=1..8, C=2..9, D=1 digit; every `val ∈ 1..8` | `cfg_03_all_single_digit_starts` | [x] |
| 4 | `sieve` | A=positive, D=2 digits, B = each of `0..9`; exhaustive `val ∈ 10..99` (covers every remainder class at a 2-digit width) | `cfg_04_two_digit_exhaustive` | [x] |
| 5 | `sieve` | A=positive, B=each of `0..9` at large width; randomized `val ∈ [10^3, 10^9]`, 400 samples | `cfg_05_random_positive_wide` | [x] |
| 6 | `sieve` | A=positive, E=**crosses a power-of-ten boundary mid-run** (digit count grows during the loop); pinned `val ∈ {8, 98, 998, 9998, 99999998, 999999998, 2147483638}` plus randomized `10^k - 2` | `cfg_06_positive_carry_across_power_of_ten` | [x] |
| 7 | `sieve` | A=positive, F=highest value that still terminates without overflow; `val = 2147483639` (C=1) | `cfg_07_max_terminating_value` | [x] |
| 8 | `sieve` | A=positive, F=just below the wrap region, C=2..10, D=10 digits; exhaustive `val ∈ [2147483630, 2147483639]` | `cfg_08_top_of_range_exhaustive` | [x] |
| 9 | `sieve` | A=negative, B=`-9`, D=1 digit + sign; `val = -9` (never exits early, runs to +9) | `cfg_09_negative_nine` | [x] |
| 10 | `sieve` | A=negative, B=each of `-9..-1`; exhaustive `val ∈ [-9, -1]`, i.e. every negative remainder class at 1 digit | `cfg_10_negative_single_digit_exhaustive` | [x] |
| 11 | `sieve` | A=negative, B=0 (negative multiple of ten); pinned `val ∈ {-10, -20, -100, -1000, -10000}` | `cfg_11_negative_multiples_of_ten` | [x] |
| 12 | `sieve` | A=negative, D=2–4 digits + sign, E=crosses `-1 → 0` sign transition; exhaustive `val ∈ [-300, -1]` | `cfg_12_negative_exhaustive_small` | [x] |
| 13 | `sieve` | A=negative, randomized magnitude; `val ∈ [-5000, -1]`, 200 samples (long runs, sign flip, multi-width) | `cfg_13_random_negative` | [x] |
| 14 | `sieve` | A=negative, large magnitude ⇒ ~10^5–10^6 iterations, exercising sustained `printf` buffering across many buffer flushes; pinned `val ∈ {-99999, -100000, -123457}` | `cfg_14_large_negative_long_run` | [x] |
| 15 | `sieve` | A=negative, D=**7-digit** negative, C≈10^6 iterations (≈8 MiB of output, ≈2000 stdio buffer refills); `val = -1000000` | `cfg_15_million_line_run` | [x] |
| 16 | `sieve` | G=0 calls — library loaded, symbol resolved, never invoked (checks no ctor/dtor side effects on stdout) | `cfg_16_zero_calls` | [x] |
| 17 | `sieve` | G=many calls in one process, mixed signs and widths interleaved; 300 randomized `val ∈ [-200, 200]` in one batch (verifies no cross-call state and identical concatenated stream) | `cfg_17_many_interleaved_calls` | [x] |
| 18 | `sieve` | H=stdout is a **pipe** (block-buffered, different libc flush granularity than a file) with a mixed batch of values | `cfg_18_stdout_is_a_pipe` | [x] |
| 19 | `sieve` | Full contiguous sweep of the low range, every remainder class × sign × width transition together: exhaustive `val ∈ [-64, 64]` in a single batch | `cfg_19_contiguous_sweep` | [x] |
| 20 | `sieve` | Hostile/extreme valid bit patterns reinterpreted as `int`: `0x7FFFFFF7` (=2147483639), `0x0`, `0x1`, `0xFFFFFFFF` (=-1), `0xFFFFFFF7` (=-9), `0x80000000` handled in ERRORS row 7 | `cfg_20_extreme_bit_patterns` | [x] |
| 21 | `sieve` | Randomized whole-`int` domain, restricted to the sub-domain that provably terminates in bounded time (`val ∈ [-3000, 2147483639]`), 600 samples — the broad property-style fuzz row | `cfg_21_broad_random_fuzz` | [x] |
| 22 | `sieve` | G=**concurrent** callers: 2, 4, 8 and 16 threads calling `sieve` simultaneously with randomized mixed-sign values. The C function has no static/global state, so the *multiset* of emitted lines must equal the sequential reference (line order across threads is inherently nondeterministic). | `cfg_22_concurrent_callers` | [x] |

## Deliberately excluded from Phase B (covered in ERRORS.md instead)

`val ≥ 2147483640` (signed-overflow wrap) and `val ≤ -3001` down to `INT_MIN`
produce 10^4–10^9 lines; the extremes (`INT_MAX`, `INT_MIN`) are covered as
bounded **prefix** comparisons in `ERRORS.md` rows 5–7 rather than as
run-to-completion valid-path rows.
