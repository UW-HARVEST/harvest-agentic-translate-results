# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from every branch / value-dependent behaviour in
`c_src/src/driver.c` and the full set of exported entry points.

## Axes the C code actually distinguishes

**Entry points (both exported, `nm -D`):**

* `run(house_t *the_house, int extra_bedrooms)` — the LOW-LEVEL entry point.
  Takes caller-supplied struct state, so all three fields are free inputs.
* `driver(const char *in)` — the one-shot wrapper. Hard-codes
  `house_t{.floors = 2, .bedrooms = 5, .bathrooms = 2.5}` and calls `run` **twice**
  with the same parsed `extra_bedrooms`.

**Runtime options/modes:** the library exposes no flags, no modes, no
`#ifdef`-selected behaviour, and no global state. `grep -nE '#if|#ifdef|switch|case' c_src/src/driver.c` → no matches. The only `if` is the
`parse_val` accept/reject branch (ERRORS.md) — so the configuration surface is
entirely made of **input shapes**.

**Input shapes the code is sensitive to:**

* `house_t.floors` (`int`): sign, magnitude, `INT_MAX` (`add_floor` does
  `floors++` → signed overflow), `INT_MIN`.
* `house_t.bedrooms` (`int`): interacts with `extra_bedrooms` in
  `bedrooms += extra_bedrooms` → signed overflow in both directions.
* `house_t.bathrooms` (`double`): consumed only by `%.1f` and `+= 1.0`.
  Distinct shapes: normal, negative, `-0.0`, exact-tie rounding
  (`x.x5` → glibc round-half-to-even), huge (`1e308`), subnormal, `NaN`,
  `±Inf` — each formats differently.
* `extra_bedrooms` (`int`): `0`, `±1`, random, `INT_MIN`, `INT_MAX`.
* `in` (`const char *`) accepted shapes: plain digits, leading whitespace
  (`strtol` skips it), explicit `+`/`-` sign, leading zeros, trailing garbage
  (`"42abc"` → 42), base-10 stop at `'x'` (`"0x10"` → 0), `INT_MIN`/`INT_MAX`
  boundaries, single digit vs many digits.

Observable output for every configuration is the exact stdout byte stream
(`print_house` is called 4× per `run`, 8× per `driver`).

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `run` | baseline: `floors=2, bedrooms=5, bathrooms=2.5`, `extra_bedrooms=3` (the shape `driver` itself uses) | [x] |
| C2 | `run` | randomized `floors`/`bedrooms` in a small range × randomized small `extra_bedrooms`, `bathrooms` a random "nice" multiple of 0.5 (200 seeded cases) | [x] |
| C3 | `run` | fully randomized `int` `floors`, `bedrooms`, `extra_bedrooms` over the whole `i32` range × random finite `bathrooms` bit patterns (500 seeded cases) | [x] |
| C4 | `run` | `bathrooms` = exact `%.1f` rounding ties: `0.05, 0.15, 0.25, 0.35, 0.45, 2.25, 2.75, -0.05, -0.25, 1.05, 1.15` (glibc round-half-to-even) × `+= 1.0` applied mid-run | [x] |
| C5 | `run` | `bathrooms` = `-0.0`, `0.0` (sign of zero must survive `%.1f` and `+= 1.0`) | [x] |
| C6 | `run` | `bathrooms` = `NaN`, `-NaN`, `+Inf`, `-Inf` (`%.1f` prints `nan`/`-nan`/`inf`/`-inf`; `+= 1.0` propagates) | [x] |
| C7 | `run` | `bathrooms` = extreme finite: `f64::MAX`, `-f64::MAX`, `1e308`, `f64::MIN_POSITIVE`, `5e-324` (subnormal), `1e-1` (309+ digit `%.1f` expansions) | [x] |
| C8 | `run` | `floors = INT_MAX` → `add_floor` signed-int overflow; also `floors = INT_MAX-1`, `INT_MIN` | [x] |
| C9 | `run` | `bedrooms = INT_MAX`, `extra_bedrooms > 0` → positive signed overflow | [x] |
| C10 | `run` | `bedrooms = INT_MIN`, `extra_bedrooms < 0` → negative signed overflow | [x] |
| C11 | `run` | `extra_bedrooms = INT_MAX` × `bedrooms ∈ {0, 1, -1, INT_MIN, INT_MAX}` | [x] |
| C12 | `run` | `extra_bedrooms = INT_MIN` × `bedrooms ∈ {0, 1, -1, INT_MIN, INT_MAX}` | [x] |
| C13 | `run` | `extra_bedrooms = 0` (no-op add) × `bedrooms ∈ {0, INT_MAX, INT_MIN}` | [x] |
| C14 | `run` | same `house_t` reused for two consecutive `run` calls (state accumulation across calls, mirroring `driver`) | [x] |
| C15 | `driver` | plain positive decimal, single digit and multi-digit (`"0".."9"`, `"7"`, `"12345"`) | [x] |
| C16 | `driver` | explicitly signed: `"+7"`, `"-7"`, `"+0"`, `"-0"` | [x] |
| C17 | `driver` | leading whitespace skipped by `strtol`: `" 42"`, `"\t42"`, `"\n\v\f\r 42"`, `"   -42"` | [x] |
| C18 | `driver` | leading zeros: `"007"`, `"0000000000000000042"`, `"-000042"` | [x] |
| C19 | `driver` | trailing garbage after a valid prefix: `"42abc"`, `"42 43"`, `"42."`, `"7-"`, `"1e5"` (base 10 → 1) | [x] |
| C20 | `driver` | base-10 stops at `'x'`: `"0x10"` → 0, `"0X1f"` → 0, `"0b101"` → 0 | [x] |
| C21 | `driver` | boundary accepted values: `"2147483647"` (`INT_MAX`), `"-2147483648"` (`INT_MIN`), `"2147483646"`, `"-2147483647"` — includes the `bedrooms` overflow inside the two `run`s | [x] |
| C22 | `driver` | randomized decimal renderings of 500 seeded `i32` values (full range), formatted with and without `+`, with and without leading spaces/zeros | [x] |
| C23 | `driver` | pre-existing non-zero `errno` in the calling thread before a **valid** input (C clears `errno` at line 61, so it must still be accepted) | [x] |
| C24 | `driver` | very long but valid input: `"+" + 300 zeros + "2147483647"` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set. Verified mechanically:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # → no match
```

Tests are nevertheless run under `--no-default-features` as well as the default
set, and under BOTH cargo profiles (`dev` and `release` — they differ in UB
checks and panic strategy, which is observable on the C library's null-pointer
paths). See `run_tests.sh`, which enumerates the feature table mechanically and
would pick up any feature added later.

## Binary executable

The project builds **no** executable: `c_src/CMakeLists.txt` has only
`add_library(driver SHARED ...)` (no `add_executable`), and `driver.c` has no
`main`. The "compare C and Rust binary stdout" gate is therefore N/A; all
stdout comparison happens through the `.so` exports instead.

Run everything with:

```
./run_tests.sh
```
