# CONFIGS.md — Phase B: configuration surface table (valid inputs)

## Mechanical derivation of the axes

The whole library is 2 functions and 12 lines of code:

```c
int foo(const char *in, char c) {
    int res = 0;
    for (const char *s = in; s = strchr(s, c); s++) { res++; }
    return res;
}
void driver(const char *in) {
    printf("A: %d\n", foo(in, 'A'));
    printf("x: %d\n", foo(in, 'x'));
}
```

There are **no runtime options, modes, flags, `#ifdef`s, byte-order choices,
element types, or Cargo features** — grep for `if` / `switch` / `#ifdef` /
`#if` in the C source returns zero non-header-guard hits (see `ERRORS.md`).
So the configuration axes the C actually distinguishes are exactly:

* **Axis E — entry point.** `foo` (the low-level function, *not* declared in
  `driver.h` but exported from the `.so`, so it is driven **directly**) and
  `driver` (the one-shot convenience wrapper, which additionally exercises
  `printf` formatting and stdout). Both are covered; the low-level one is
  covered across the full needle range, not just the two needles `driver` uses.
* **Axis N — the needle `c`** (only meaningful for `foo`; `driver` hard-codes
  `'A'` then `'x'`): `'A'`; `'x'`; other ASCII; a byte absent from the haystack;
  the opposite case (`'a'` vs `'A'`, proving case sensitivity); high-bit /
  negative `signed char` values; the full sweep `1..=255`.
* **Axis S — haystack shape:** empty; length 1; short; long; all-matches;
  no-matches; match at first byte; match at last byte; consecutive-match runs;
  matches separated by exactly one byte; needle interleaved with the *other*
  needle (`'A'` and `'x'` both present, for `driver`); embedded high-bit /
  invalid-UTF-8 bytes; 1 MiB oversized.
* **Axis O — observable:** `foo`'s `int` return value; `driver`'s stdout bytes
  (captured by redirecting fd 1 to a temp file around each call and comparing
  byte-for-byte).

Rows below are the pruned cross-product of these axes. Every row is driven by
**many randomized inputs with a fixed seed** (a small xorshift PRNG in the test
file, seeded per row) unless the row is inherently a single fixed shape, and
every row calls **both** `.so`s through `libloading` and asserts byte-identical
output.

## Configuration surface table

Rows 1–14 live in `tests/differential.rs`; rows 15–22 (whose observable is
stdout) live in `tests/driver_stdout.rs`, a `harness = false` target so that
libtest's own output cannot contaminate the fd-1 capture.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `foo` | empty haystack `""`, needle swept over all of `1..=255` | `cfg_01_foo_empty_haystack_all_needles` | [x] |
| 2 | `foo` | length-1 haystack, every byte value `1..=255` × needle swept `1..=255` (full 255×255 matrix — covers match, no-match, and case-difference at minimum size) | `cfg_02_foo_single_byte_matrix` | [x] |
| 3 | `foo` | random printable-ASCII haystacks, len 1..64, needle `'A'` (the needle `driver` uses first) | `cfg_03_foo_random_ascii_needle_A` | [x] |
| 4 | `foo` | random printable-ASCII haystacks, len 1..64, needle `'x'` (the needle `driver` uses second) | `cfg_04_foo_random_ascii_needle_x` | [x] |
| 5 | `foo` | random haystacks, needle chosen randomly from `1..=127` — needle *guaranteed absent* (haystack drawn from a disjoint alphabet) → count 0 path | `cfg_05_foo_needle_guaranteed_absent` | [x] |
| 6 | `foo` | random haystacks, needle *guaranteed present exactly once* (single match), random position incl. first and last byte | `cfg_06_foo_single_match_random_position` | [x] |
| 7 | `foo` | haystack = needle repeated N times, N ∈ 1..64 → all-matches / consecutive-run path (`s++` lands directly on the next match) | `cfg_07_foo_all_matches_consecutive` | [x] |
| 8 | `foo` | matches separated by exactly one non-matching byte (`"AxAxAx…"`), random length → alternating advance | `cfg_08_foo_alternating_matches` | [x] |
| 9 | `foo` | random haystacks drawn from a **2-symbol** alphabet `{needle, other}` with random density, len 0..256 → many matches, random clustering | `cfg_09_foo_two_symbol_alphabet_random_density` | [x] |
| 10 | `foo` | random haystacks over the **full byte range `1..=255`** (non-UTF-8 bytes included), needle also random over `1..=255`, len 0..256 | `cfg_10_foo_full_byte_range` | [x] |
| 11 | `foo` | needle is a **high-bit byte** passed as a negative `signed char` (`-128..=-1`), haystack contains those exact bytes → sign-extension handling | `cfg_11_foo_negative_char_needles` | [x] |
| 12 | `foo` | needle `'a'` on a haystack containing only `'A'` (and vice versa) → case sensitivity, count must be 0 / N respectively | `cfg_12_foo_case_sensitivity` | [x] |
| 13 | `foo` | long haystack: 4 KiB and 64 KiB random bytes, random needle → large-count path | `cfg_13_foo_long_haystacks` | [x] |
| 14 | `foo` | oversized haystack: 1 MiB, ~50% match density → very large `int` count | `cfg_14_foo_oversized_haystack` | [x] |
| 15 | `driver` | empty haystack `""` → stdout must be `"A: 0\nx: 0\n"` from both | `cfg_15_driver_empty` | [x] |
| 16 | `driver` | haystack with **only** `'A'`s (no `'x'`) → first count > 0, second 0 | `cfg_16_driver_only_A` | [x] |
| 17 | `driver` | haystack with **only** `'x'`s (no `'A'`) → first count 0, second > 0 | `cfg_17_driver_only_x` | [x] |
| 18 | `driver` | haystack with **both** `'A'` and `'x'` interleaved at random densities → both counts > 0, and the *order* of the two `printf` lines is checked | `cfg_18_driver_both_needles_interleaved` | [x] |
| 19 | `driver` | haystack with **neither** `'A'` nor `'x'` (random bytes from a disjoint alphabet) → `"A: 0\nx: 0\n"` | `cfg_19_driver_neither_needle` | [x] |
| 20 | `driver` | random full-byte-range haystacks (incl. invalid UTF-8 and `%`/`%s`/`%n` format bytes), len 0..512, 200 randomized cases → stdout compared byte-for-byte | `cfg_20_driver_random_full_byte_range` | [x] |
| 21 | `driver` | long haystack (64 KiB) with many `'A'`/`'x'` → multi-digit count formatting in `printf` | `cfg_21_driver_long_haystack_multidigit` | [x] |
| 22 | `foo` + `driver` | **composed pipeline / cross-check:** for the same random haystack, assert `driver`'s two stdout numbers equal `foo(in,'A')` and `foo(in,'x')` **taken from the other library** (C stdout vs Rust `foo`, and Rust stdout vs C `foo`), so a bug that is consistent within one library is still caught | `cfg_22_cross_library_pipeline_consistency` | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` table, so `{default}` and
`{--no-default-features}` are the same single configuration. `check_features.sh`
enumerates the feature space out of `Cargo.toml` (so a future `[features]` table
is picked up automatically, powerset included) and re-runs the whole suite plus
the `nm` symbol-parity diff for every combination, under both the `dev` and the
`release` profile — 4 runs in total today, all green.

## Result

All 22 rows pass. Every row was driven with many randomized inputs from a
fixed-seed xorshift PRNG (roughly 150k differential `foo` calls and 3k
differential `driver` calls per run) and every comparison is byte-for-byte
against the C `.so` loaded in the same process. No divergence was found, so no
fix to the Rust source was required.

## Binary executable

Not applicable — `CMakeLists.txt` contains no `add_executable`; the project
builds only `libdriver.so`. The stdout of the library function `driver()` is
nevertheless compared byte-for-byte (rows 15–22).
