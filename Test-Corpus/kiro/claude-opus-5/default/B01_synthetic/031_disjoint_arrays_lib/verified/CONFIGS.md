# CONFIGS.md — configuration surface table (Phase A, gates Phase B)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axis discovery (what the C actually branches on)

```
grep -rn '#if\|#ifdef\|#else'      c_src/src c_src/include  # only the DRIVER_H_ include guard
grep -rn 'enum\|struct\|typedef\|static\|extern' c_src/src c_src/include  # no matches
grep -rn 'add_executable'          c_src/CMakeLists.txt                  # no matches
grep -n  '\[features\]'            translation/Cargo.toml                # no matches
```

* **Runtime options / modes / flags: none.** There is no setter, no context
  object, no global state, no environment lookup. Behaviour is a pure function
  of the arguments.
* **Compile-time configuration: none.** One include guard, no `#ifdef` feature
  switches; Cargo declares no features.
* **Binary executable: none.** `CMakeLists.txt` builds only
  `add_library(driver SHARED ...)`, so there is no C/Rust driver binary whose
  stdout could be diffed. `driver()`'s `printf` output is captured instead (by
  redirecting fd 1 in the test harness) and compared byte-for-byte.

So the entire configuration surface is **(entry point) × (input shape)**.

## Public entry points, lowest level first

| level | symbol | signature |
|-------|--------|-----------|
| 1 (lowest) | `fma_array` | `void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len)` |
| 2 | `call_fma` | `int call_fma(const int *data, int len)` — composes `fma_array` with `ones`/`zeros` VLAs |
| 3 (one-shot wrapper) | `driver` | `void driver(const char *in)` — `sscanf` loop → `call_fma` → `printf` |

All three are tested directly through the `.so` exports; `driver` is *not* used
as a proxy for the lower two.

## Input-shape axes the code distinguishes

* `fma_array` / `call_fma` — `len`: `0`, `1`, `2`, small (3..8), `99`, `100`,
  `101`, large (`1000`, `4096`); element values: all-zero, all-one, positive,
  negative, mixed, `INT_MAX`, `INT_MIN`, overflow-producing triples.
* `driver` — the `sscanf("%d%zn")` loop distinguishes: integer **count**
  (0/1/2/99/100/101/many), **separator kind** (space, tab, `\n`, `\r`, `\v`,
  `\f`, runs of mixed whitespace, *no* whitespace when the next token starts
  with a sign), **sign** (none / `+` / `-`), **leading zeros**, **leading and
  trailing whitespace**, **magnitude** (small, `INT_MAX`, `INT_MIN`,
  out-of-`int`-range), and **terminator** (end of string vs. non-numeric byte).

## Configuration table (one row per combination the C treats differently)

Every row is driven with **many randomized inputs** (seeded LCG, fixed seed
`0x5EED_1234_ABCD_0001`, 200+ cases per row unless noted) via both `.so`s and
compared byte-for-byte / value-for-value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `fma_array` | `len = 1`, random `mul1`/`mul2`/`add` in `[-1000,1000]` | [x] |
| 2 | `fma_array` | `len = 2` (smallest multi-element), random small values | [x] |
| 3 | `fma_array` | `len` random in `3..=8`, random full-range `i32` values | [x] |
| 4 | `fma_array` | `len = 99`, random full-range `i32` (overflow expected & wrapped) | [x] |
| 5 | `fma_array` | `len = 100` (the `driver` capacity boundary), random values | [x] |
| 6 | `fma_array` | `len = 101`, random values | [x] |
| 7 | `fma_array` | `len = 1000` and `4096` (large), random values | [x] |
| 8 | `fma_array` | all-zero `mul1` (result is pure `add`), random `add` | [x] |
| 9 | `fma_array` | all-one `mul1` (the `call_fma` shape: result is `mul2 + add`) | [x] |
| 10 | `fma_array` | all-zero `add` (pure product), random `mul1`/`mul2` | [x] |
| 11 | `fma_array` | extremes only: elements drawn from `{INT_MIN, -1, 0, 1, INT_MAX}` — forces `imul`/`add` overflow wrapping | [x] |
| 12 | `fma_array` | `out` buffer pre-filled with a sentinel pattern, `len` shorter than the buffer — verifies no writes past `len` | [x] |
| 13 | `call_fma` | `len = 1`, random `data` | [x] |
| 14 | `call_fma` | `len = 2`, random `data` | [x] |
| 15 | `call_fma` | `len` random in `3..=8`, random full-range `data` | [x] |
| 16 | `call_fma` | `len = 99 / 100 / 101` (capacity boundary), random `data` | [x] |
| 17 | `call_fma` | `len = 1000` (large VLA), random `data` | [x] |
| 18 | `call_fma` | `data` from extremes `{INT_MIN, -1, 0, 1, INT_MAX}`, random `len` in `1..=64` | [x] |
| 19 | `call_fma` | `len` shorter than the actual `data` buffer (tail must be ignored; result is `data[len-1]`) | [x] |
| 20 | `driver` | exactly 1 integer, no surrounding whitespace, random value in `i32` | [x] |
| 21 | `driver` | 2 integers separated by a single space, random values | [x] |
| 22 | `driver` | random count `3..=20`, single-space separated, random `i32` values | [x] |
| 23 | `driver` | random count `3..=20`, separator randomly chosen per gap from `{' ', '\t', '\n', '\r', '\v', '\f'}` and repeated 1..4 times | [x] |
| 24 | `driver` | leading whitespace run before the first integer (`%d` skips it, `%zn` counts it) | [x] |
| 25 | `driver` | trailing whitespace after the last integer (final `sscanf` consumes it then hits EOF) | [x] |
| 26 | `driver` | explicit `+` sign on a random subset of the integers | [x] |
| 27 | `driver` | negative values (explicit `-`) mixed with positives | [x] |
| 28 | `driver` | leading zeros / zero-padded tokens (`"007"`, `"-00042"`, `"0000"`) | [x] |
| 29 | `driver` | **no separator**, next token starts with a sign (`"1-2+3-4"`) — the sign itself terminates the previous conversion | [x] |
| 30 | `driver` | values at the `int` boundaries: tokens drawn from `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` | [x] |
| 31 | `driver` | exactly 99 integers (one below the loop bound) | [x] |
| 32 | `driver` | exactly 100 integers (the loop bound) | [x] |
| 33 | `driver` | 101..300 integers (past the loop bound; only the first 100 are read) | [x] |
| 34 | `driver` | long input: 500..1200 integers, ~10 KB of text | [x] |
| 35 | `driver` | fully random ASCII byte soup (printable range, may or may not parse) — property test that both agree whatever happens | [x] |
| 36 | `driver` | random valid integer prefix followed by random non-numeric garbage tail | [x] |
| 37 | `driver`, `call_fma`, `fma_array` | **composed pipeline**: build a random integer text, run `driver` (captured stdout), and independently run `call_fma`/`fma_array` on the same parsed values through both `.so`s, asserting the three levels agree with each other *and* across languages | [x] |

## Feature combinations (Phase D)

`translation/Cargo.toml` has no `[features]`, so the complete set is:

| combo | command |
|-------|---------|
| default | `cargo test --release` |
| no-default-features (identical to default) | `cargo test --release --no-default-features` |

Both are run by `run_all.sh`.
