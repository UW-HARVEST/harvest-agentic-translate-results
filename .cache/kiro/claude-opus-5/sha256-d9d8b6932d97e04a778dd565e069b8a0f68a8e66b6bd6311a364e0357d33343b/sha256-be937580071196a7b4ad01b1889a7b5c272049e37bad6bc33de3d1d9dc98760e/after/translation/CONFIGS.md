# CONFIGS.md — configuration surface table (valid inputs)

## Axes the C actually branches on

Derived from `c_src/CMakeLists.txt`, `c_src/src/mdmacros.h` and the two `.c`
files — not from guesses about what matters.

**Build-time axis 1 — `OP`** (`CMakeLists.txt:26` `set(OP "add" ...)`,
`mdmacros.h:27` `#ifndef OP / #define OP add`). Token-pasting makes `OP` select
*four* different things at once, so each value is a genuinely different program:

| `OP` | `OP_FN(OP)` | `STEP_OP` (`mdmacros.h:48-50`) | `INIT_FOR(OP)` (`:56-58`) | `STR(OP)` |
|------|-------------|--------------------------------|---------------------------|-----------|
| `add` | `op_add` | `acc += i` | `0` | `"add"` |
| `sub` | `op_sub` | `acc -= i` | `0` | `"sub"` |
| `mul` | `op_mul` | `acc *= (i + 1)` | `1` | `"mul"` |

**Build-time axis 2 — `REPEAT`** (`CMakeLists.txt:27`, `mdmacros.h:30-32`).
`RUN_LOOP(op, acc, REPEAT)` = `CHOOSE_REP(REPEAT)` = `REP<REPEAT>`, and only
`REP0`..`REP7` exist (`:63-70`), so the valid range is **0..=7**. This axis is
*not* redundant with axis 1: `REPEAT` also decides whether
`use_generated(REPEAT)` lands on a `switch` case or on `default`, because
`DISPATCH_REP` (`:82-93`) only labels `0..6`. `REPEAT == 7` is therefore a
distinct, asymmetric configuration and gets its own rows.

Cross-product: 3 × 8 = **24** build configurations, plus `--no-default-features`
with no OP/REPEAT feature (the `#ifndef` fallback, ≡ `add,5`) and the `default`
feature set (also `add,5`) ⇒ **26** cargo invocations.

**Runtime axis 3 — entry point.** The full public surface from `mdmacros.h:40-42`
and `:104-110`, lowest level first — the tests call all of them directly through
`nm -D` exports, not just the composed helpers:

1. `op_add`, `op_sub`, `op_mul` — leaf arithmetic (exported regardless of `OP`)
2. `G_OP` — data export, an *indirect* call through the global fn pointer
3. `G_OP_NAME` — data export, a `const char *`
4. `helper_ptr` — leaf op via a local fn pointer + one `printf`
5. `helper_call` — leaf op **plus** the unrolled `RUN_LOOP` accumulator
6. `use_generated` — the `static` macro-generated `accum_<OP>` via `DISPATCH_REP`
7. `driver` executable — the whole pipeline end to end, incl. `atoi` and both
   summary `printf`s

**Runtime axis 4 — input shape.** `int` scalars only; the shapes the code
distinguishes are sign, zero, and the overflow boundaries (`op_mul` and
`STEP_mul` overflow at very different magnitudes than `op_add`), plus, for
`use_generated`, whether `n` falls inside `0..=6`.

## Rows

Every row is executed under **all 26 build configurations** (axes 1-2); the
row itself pins axes 3-4. Randomized rows use `SplitMix64` with a fixed seed
(`0x5EED_1234_ABCD_0001`) so failures reproduce.

| # | entry point(s) | configuration (options set + input shape) | ✓ |
|---|----------------|--------------------------------------------|---|
| 1 | `op_add` | 4096 randomized `(a, b)` over the full `i32` range | [x] |
| 2 | `op_add` | boundary grid: `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}²` (81 pairs), incl. every overflow corner | [x] |
| 3 | `op_sub` | 4096 randomized `(a, b)` over the full `i32` range | [x] |
| 4 | `op_sub` | same 81-pair boundary grid (`0 - INT_MIN`, `INT_MAX - INT_MIN`, …) | [x] |
| 5 | `op_mul` | 4096 randomized `(a, b)` over the full `i32` range | [x] |
| 6 | `op_mul` | same 81-pair boundary grid, plus `{±65536, ±46341, ±2}²` where the product straddles `INT_MAX` | [x] |
| 7 | `op_add`/`op_sub`/`op_mul` | small-magnitude sweep `a, b ∈ -8..=8` (289 pairs each) — catches sign/`INIT` mix-ups the wide random sweep can mask | [x] |
| 8 | `G_OP` (data export) | read the global fn pointer from both `.so`s, then call it with the 4096 randomized pairs **and** assert it agrees with the `op_<OP>` selected by the active `OP` feature (i.e. `G_OP` points at the right leaf for this build) | [x] |
| 9 | `G_OP` (data export) | assert the pointer is non-`NULL` and that C-vs-Rust *behaviour* matches for the boundary grid (addresses necessarily differ between the two `.so`s) | [x] |
| 10 | `G_OP_NAME` (data export) | read the `const char *`, walk to the NUL, compare the byte string against C's and against `STR(OP)` for the active feature | [x] |
| 11 | `helper_ptr` | 4096 randomized `(a, b)` — exercises the local-fn-pointer path, `OP`-dependent | [x] |
| 12 | `helper_ptr` | boundary grid (81 pairs) | [x] |
| 13 | `helper_call` | 4096 randomized `(a, b)` — leaf op **+** `RUN_LOOP(OP, acc, REPEAT)`; the `acc` half is `REPEAT`-dependent and the return is the wrapping sum of both halves | [x] |
| 14 | `helper_call` | boundary grid (81 pairs); under `mul` with `REPEAT >= 2` the `acc` factor is > 1 so `r + acc` overflow is reachable | [x] |
| 15 | `use_generated` | full in-range sweep `n ∈ {0,1,2,3,4,5,6}` — one row per `switch` case, all seven exercised in every build config | [x] |
| 16 | `use_generated` | `n == REPEAT` (what `mdmain.c:42` actually passes) for the active build — including `REPEAT == 7`, which falls to `default` | [x] |
| 17 | `use_generated` | 4096 randomized `n` over the full `i32` range (dominated by the `default` arm, mixed with in-range hits) | [x] |
| 18 | `use_generated` | contiguous sweep `n ∈ -16..=16` — the two `switch`-range edges (`-1`/`0` and `6`/`7`) at fine grain | [x] |
| 19 | all 6 library exports at once | one composed sequence per config in the order `mdmain.c` uses them (`OP_FN`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP`) with the same `(a, b)`, comparing the running `summary` accumulation — catches pipeline-level divergence invisible to per-function tests | [x] |
| 20 | `driver` binary | stdout **and** stderr **and** exit status compared byte-for-byte for 40 argv shapes: `0 0`, `1 2`, `-3 7`, `7 -3`, `2147483647 1`, `-2147483648 -1`, `2147483647 2147483647`, `65536 65536`, `46341 46341`, `"abc" "def"`, `"" ""`, `"12abc" "3x"`, `"  -7" "+9"`, `"--3" "0x10"`, `"99999999999" "1"`, `"-99999999999" "1"`, `"9223372036854775808" "0"`, `"-9223372036854775808" "0"`, `"-9223372036854775809" "0"`, `"0000000000000000005" "-0"`, plus 20 randomized decimal pairs | [x] |
| 21 | `driver` binary | `argc` shapes: 0 extra args, 1 arg, 3 args, 5 args (see `ERRORS.md` rows 1-2, 9) | [x] |

## Notes on interaction effects specifically hunted

- `OP=mul` × `REPEAT=0` — `acc` stays at `INIT_mul == 1`, so `helper_call`
  returns `a*b + 1`, not `a*b`. A translation that hardcoded `INIT = 0` passes
  every `add`/`sub` row and fails only here.
- `OP=mul` × `REPEAT=7` — `RUN_LOOP` multiplies by `7!` = 5040 while
  `use_generated(7)` returns `1`. The two paths disagree *by design*.
- `OP=sub` × `REPEAT>=1` — `acc` goes negative (`-(0+1+…+(REPEAT-1))`), so the
  `helper_call` sum is a mixed-sign wrapping add.
- `REPEAT=7` is the only value where `RUN_LOOP` and `DISPATCH_REP` diverge; rows
  16 and 19 pin it under every `OP`.
