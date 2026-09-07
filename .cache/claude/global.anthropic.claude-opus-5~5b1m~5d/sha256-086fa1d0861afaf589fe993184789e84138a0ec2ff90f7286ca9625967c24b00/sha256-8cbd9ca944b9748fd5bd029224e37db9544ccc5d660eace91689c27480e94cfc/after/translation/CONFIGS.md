# CONFIGS.md — Configuration-surface table (valid inputs)

Mechanically derived from `c_src/include/driver.h` + `c_src/src/driver.c`.

## Axes the C code actually branches on / varies over

**A. Public entry points** (everything with external linkage — both the
convenience wrapper *and* the low-level one):

| entry point | signature | in public header? | notes |
|---|---|---|---|
| `driver` | `void driver(const char *in)` | yes | one-shot wrapper: parse a string, then `run(x); run(x);` |
| `run`    | `void run(int extra_bedrooms)` | **no** (but non-`static`) | the low-level entry point; must be driven directly, not only through `driver` |

There are **no** runtime option/mode/flag setters, no `#ifdef`s, no
`switch`es, and no Cargo features (`translation/Cargo.toml` declares none).
The single configuration axis the library *does* carry is **persistent
file-scope mutable state**:

**B. Hidden state — `static house_t the_house = {2, 5, 2.5}`.** Every call
mutates it and is never reset, so output is a function of the *whole call
history*, not of one argument. One `run()` performs:
`print → floors++ → print → bathrooms += 1.0 → print → bedrooms += extra → print`
(4 output lines). One successful `driver()` performs two `run()`s (8 lines).
A rejected `driver()` prints 1 line and mutates nothing.

**C. Input shape for `driver`'s `const char *in`** (what `strtol(…, 10)` distinguishes):
bare digits; leading whitespace (`" \t\n\v\f\r"`); explicit `+`; explicit `-`;
leading zeros; trailing garbage; value magnitude class
(`0`, small, `INT_MAX`, `INT_MIN`, `INT_MAX±1`, `LONG_MAX/MIN`, > `LONG_MAX`);
non-converting forms.

**D. Argument value class for `run`'s `int`**: `0`; small positive; small
negative; `INT_MAX`; `INT_MIN`; values chosen so `bedrooms + extra` overflows
`int` (signed-overflow wrap) or lands exactly on `INT_MAX`/`INT_MIN`.

**E. Formatting shape** — `printf("… %d floors, %d bedrooms, and %.1f bathrooms\n")`:
`%d` must render negative and extreme values; `%.1f` must render `bathrooms`
after *k* increments (`2.5, 3.5, 4.5, …`) including values large enough to lose
`.5` exactness and the `%.1f` round-half-to-even path.

## Table — one row per meaningful combination

Every row is exercised with **many randomized inputs** (fixed seed, xorshift64\*)
in addition to its named boundary values, calling both `.so`s through
`libloading` and comparing captured stdout byte-for-byte. Both libraries are
driven in **lockstep** (same ops, same order), so the persistent `the_house`
state is directly part of what is compared — see "How the rows are executed".

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|-------------------------------------------|------|---|
| C1  | `run` | single call, `extra_bedrooms == 0` (identity on `bedrooms`) | `cfg_c1_run_zero` | [x] |
| C2  | `run` | single call, small positive `extra` (1..=1000, randomized) | `cfg_c2_run_small_positive` | [x] |
| C3  | `run` | single call, small negative `extra` (-1000..=-1, randomized) → negative `%d` rendering | `cfg_c3_run_small_negative` | [x] |
| C4  | `run` | single call, `extra == INT_MAX` → `5 + INT_MAX` signed overflow wrap | `cfg_c4_run_int_max` | [x] |
| C5  | `run` | single call, `extra == INT_MIN` → `5 + INT_MIN` underflow | `cfg_c5_run_int_min` | [x] |
| C6  | `run` | single call, `extra` uniformly random over the **full** `i32` range (256 values) | `cfg_c6_run_full_i32_range` | [x] |
| C7  | `run` | single call, `extra` chosen so `bedrooms` lands exactly on `INT_MAX` (`INT_MAX-5`) / `INT_MIN` (`INT_MIN-5` wrapped) / `-1` / `0` | `cfg_c7_run_boundary_landings` | [x] |
| C8  | `run` × N | **state accumulation**: `run` called 1,2,3,5,10,64 times in a row with mixed random `extra` → `floors` and `bathrooms` monotonically advance, `bedrooms` accumulates with wrap | `cfg_c8_run_repeated_accumulation` | [x] |
| C9  | `run` × 300 | long run so `bathrooms` reaches 302.5 → `%.1f` on larger magnitudes | `cfg_c9_run_long_sequence_float_fmt` | [x] |
| C10 | `driver` | valid bare digits, small magnitude (randomized 0..=9999) → 8 output lines | `cfg_c10_driver_small_digits` | [x] |
| C11 | `driver` | valid, explicit `+` sign (`"+7"`, randomized `+N`) | `cfg_c11_driver_plus_sign` | [x] |
| C12 | `driver` | valid, explicit `-` sign (`"-7"`, randomized `-N`) | `cfg_c12_driver_minus_sign` | [x] |
| C13 | `driver` | valid, **leading whitespace** — each of `" "`, `"\t"`, `"\n"`, `"\v"`, `"\f"`, `"\r"` and mixtures, before an optional sign and digits | `cfg_c13_driver_leading_whitespace` | [x] |
| C14 | `driver` | valid, **leading zeros** (`"007"`, `"0000000000000000042"`, `"-00012"`) and plain `"0"`, `"-0"`, `"+0"` | `cfg_c14_driver_leading_zeros` | [x] |
| C15 | `driver` | valid, **trailing garbage** (`"12abc"`, `"3 "`, `"5\n"`, `"1,000"`, `"0x1A"` → parses `0`) | `cfg_c15_driver_trailing_garbage` | [x] |
| C16 | `driver` | valid **boundaries**: `"2147483647"` (`INT_MAX`) and `"-2147483648"` (`INT_MIN`) → `bedrooms` wraps twice (two `run`s) | `cfg_c16_driver_int_boundaries` | [x] |
| C17 | `driver` | valid, `extra` uniformly random over the **full** `i32` range, formatted as decimal text (256 values) | `cfg_c17_driver_full_i32_range` | [x] |
| C18 | `driver` × N | **state accumulation across `driver` calls** (1,2,3,8 calls, random values) → 8 lines each, state carried | `cfg_c18_driver_repeated_accumulation` | [x] |
| C19 | `driver` + `run` **interleaved** | mixed random sequences of both entry points (e.g. `run, driver, run, run, driver`) → verifies the two exports share the *same* `the_house` instance in both libraries | `cfg_c19_interleaved_run_and_driver` | [x] |
| C20 | `driver` (rejecting) + `run` | a **rejected** `driver` call in the middle of a sequence must print 1 line and leave state untouched, so subsequent `run` output is unaffected | `cfg_c20_rejected_driver_preserves_state` | [x] |
| C21 | `driver` | randomized **fuzz over arbitrary byte strings** (printable + whitespace + digit alphabet, lengths 0..24, 4000 cases) — accept/reject decision *and* bytes must match | `cfg_c21_driver_fuzz_arbitrary_strings` | [x] |
| C22 | `driver`/`run` | randomized **long mixed sequences** (200 ops of `run`/valid `driver`/invalid `driver`, 40 sequences) — full composed pipeline | `cfg_c22_long_mixed_random_sequences` | [x] |
| C23 | `run` | `%.1f` **round-half-to-even** probe: state advanced so `bathrooms` hits values where the 1-decimal rounding of a binary double is not exact (very large `bathrooms` via many `run`s is covered in C9; here `bathrooms` is only ever `2.5+k`, so the exact set `{2.5,3.5,…}` is enumerated and compared) | `cfg_c9_run_long_sequence_float_fmt` | [x] |

## How the rows are executed

`tests/harness/mod.rs` loads each `.so` **once per test process** and replays
every operation sequence against **both** libraries while holding one global
lock, so the two instances always observe the same ops in the same order and
their hidden `the_house` state stays in perfect lockstep. Any divergence in the
accumulated state therefore surfaces as a byte difference. (Re-`dlopen`ing a
fresh file copy per call was tried first and is *not* reliable: after `dlclose`
glibc may still have the object mapped and will alias a reused inode back to the
stale one.) `stdout` is captured by `dup2`-ing fd 1 onto a temp file while
holding Rust's `StdoutLock`, which keeps libtest's own progress output from
contaminating the capture under the default multi-threaded runner; every captured
line is additionally validated to be either a house report or the error sentinel.

Absolute pristine-state expectations live in their own test binary,
`tests/phase_b_initial_state.rs` (row C1), which pins the exact C bytes for
`{floors = 2, bedrooms = 5, bathrooms = 2.5}`. All other rows assert
state-independent properties (line counts, `bedrooms` deltas via
`bedrooms_delta`, sentinel identity) on top of the byte-for-byte comparison.

## Result

All 23 rows pass. Volume actually executed, asserted by in-harness counters:
row C21 performs 4,000 comparisons (with >400 accepting and >400 rejecting
cases), row C22 issues 8,000 library calls per library; the whole suite performs
~5,300 C-vs-Rust byte comparisons per run.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` (`cargo metadata` reports
`{}`), so the complete set of feature combinations is `{default}` = `{}`. The
suite is nevertheless run under both `--features` spellings and both profiles by
`./run_all_checks.sh`:

| profile | features | result |
|---|---|---|
| dev | default | 43 tests pass |
| dev | `--no-default-features` | 43 tests pass |
| release | default | 43 tests pass |
| release | `--no-default-features` | 43 tests pass |

`[profile.release] panic = "abort"` makes the release configuration a genuinely
different build of the code under test, which is why both profiles are run.
