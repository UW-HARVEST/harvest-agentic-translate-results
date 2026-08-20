# CONFIGS.md — configuration-surface table (Phase A) and its differential tests (Phase B)

## Build-time configuration axes (enumerated mechanically)

| source | axis | values | notes |
|--------|------|--------|-------|
| `Cargo.toml` | `[features]` | **absent** → the only combination is the empty set | `cargo check --no-default-features` ≡ `cargo check` ≡ `cargo check --all-features`; verified by `scripts/check_features.sh` |
| `c_src/CMakeLists.txt` | build options / `option()` / `target_compile_definitions` | **none** — `add_executable(driver src/main.c)` only | no `CMAKE_BUILD_TYPE` is set, so the ground-truth binary is built with **no** `-O` flag (`-O0`) |
| `c_src/src/main.c` | `#ifdef` / `#if` | **none** (only `#define ARRAY_SIZE`, `#define ITERATIONS`) | nothing to configure at C compile time |
| gcc optimisation level | ground truth for UB-dependent arithmetic (signed overflow, `x << 1` on negatives, `x >> 3` on negatives) | `-O0` (CMake default) and `-O2` | both `.so`s are built by `build.rs` and **every** Phase B/C test runs against **both** |
| Cargo profile | `overflow-checks` / `debug-assertions` | `dev` (checks **on**) and `release` (checks **off**, per `[profile.release]`) | the whole suite is run in both profiles by `scripts/run_all.sh` |

So the cross-product to verify is: {no features} × {C `-O0`, C `-O2`} × {Rust dev,
Rust release} — 4 combinations, all automated.

## Runtime configuration axes (derived from the C branches)

The C program has no flags or modes; its behaviour is a function of
(a) `argc`, (b) the bytes of `argv[1]`, and (c) the contents of the global
`array`. The axes the code actually distinguishes:

* **entry point** — `main` (top-level), `perform_expensive_operations` (the
  lowest-level public entry point, called 2000× by `main`), and the exported
  `array` object it mutates. Plus the two behaviours the port reimplements
  instead of importing from libc, driven through the harness hooks:
  `srand`/`rand` (`rng.rs`) and `strtoul`-based validation (`strtoul.rs`,
  `program::parse_seed`).
* **`argc`** — `== 2` (the compute path) vs `!= 2` (usage path → `ERRORS.md`).
* **`argv[1]` shape** — whitespace prefix, sign, leading zeros, digit count,
  value class relative to `UINT_MAX` / `ULONG_MAX` (accept side here, reject
  side in `ERRORS.md`).
* **seed value class** — `0` (glibc's `srandom_r` silently remaps it to `1`),
  `1`, small, `> 2³¹` (makes glibc's `int32_t word = seed` negative during
  Schrage seeding — a real branch), `UINT_MAX`.
* **`array` element shape** — `0` (the `.bss` initial state), `INT_MIN`,
  `INT_MAX`, `±1`, multiples of 7 (`x % 7 == 0`), negatives (arithmetic `>>`,
  truncate-toward-zero `/` and `%`), the transformation's fixed point
  (`-848907408`), values in `[0, 2³¹)` as produced by `rand()`, and uniform
  random `i32`.
* **element position** — index `0`, `1`, `ARRAY_SIZE-2`, `ARRAY_SIZE-1`
  (loop-boundary elements) and the full 262 144-element sweep.
* **repetition count** — 1 / 2 / 3 / 10 successive `perform_expensive_operations`
  calls (state accumulates in `array`), and the full `ITERATIONS = 2000`.

All rows use a fixed-seed PRNG (`SplitMix64`, seed noted per test) so the random
inputs are reproducible, and all rows compare **the entire 1 MB `array`** (plus
the reduced XOR) byte-for-byte, not just a summary.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `array` (data symbol) | symbol size / element count parity: C `array` is 0x100000 bytes = `ARRAY_SIZE × 4`; `ARRAY_SIZE` and `ITERATIONS` compiled into Rust equal the C `#define`s | [x] `symbols.rs::array_symbol_size_matches`, `symbols.rs::compile_time_constants_match` |
| 2 | `perform_expensive_operations` | `array` in its initial all-zero `.bss` state, 1 call | [x] `perform.rs::all_zeros` |
| 3 | `perform_expensive_operations` | all-zero start, 10 successive calls (compounding state) | [x] `perform.rs::all_zeros` |
| 4 | `perform_expensive_operations` | 8 genuinely uniform arrays (one value in all 262 144 slots: `0`, `±1`, `INT_MIN`, `INT_MAX`, `-848907408`, `±715827882`), plus a tiled sweep of 400+ hand-picked edge values (every bit position ±1, every `% 7` residue, the `x*3+7` overflow threshold) | [x] `perform.rs::uniform_value_sweep` |
| 5 | `perform_expensive_operations` | uniform-random `i32` over the **full** signed range (negatives → arithmetic shift + C division rounding), 1 call, 8 randomized rounds | [x] `perform.rs::random_full_range` |
| 6 | `perform_expensive_operations` | uniform-random `i32`, 3 successive calls per round | [x] `perform.rs::random_full_range_repeated` |
| 7 | `perform_expensive_operations` | `rand()`-shaped values (`[0, 2³¹)`) — the only shape the real program ever feeds it | [x] `perform.rs::rand_shaped_values` |
| 8 | `perform_expensive_operations` | boundary elements: `INT_MIN`/`INT_MAX`/`0`/`±1`/`±7`/fixed point planted at indices `0`, `1`, `ARRAY_SIZE-2`, `ARRAY_SIZE-1`, random elsewhere | [x] `perform.rs::boundary_indices` |
| 9 | `perform_expensive_operations` | values that exercise each `x % 7` residue and each `x / 2` rounding direction (odd/even × positive/negative) | [x] `perform.rs::division_and_modulo_shapes` |
| 10 | `perform_expensive_operations` | fully contiguous block `-131072 … 131071`; the two domain ends (`INT_MIN …` and `… INT_MAX`, 131 072 values each); 4 strided sweeps covering the whole `u32` domain at stride 16 384 (~1M distinct values) | [x] `perform.rs::exhaustive_low_and_extreme_ranges` |
| 11 | `srand`/`rand` port (`harness_srand`/`harness_rand`) vs **real glibc** | `seed = 0` (remapped to 1 by glibc), `1`, `2`, `7`, `12345`, `2³¹-1`, `2³¹`, `2³¹+1`, `UINT_MAX`, `UINT_MAX-1` — 400 draws each | [x] `rng.rs::edge_seeds` |
| 12 | `srand`/`rand` port vs real glibc | 4096 random seeds (full `u32` range, incl. `> 2³¹` where glibc's `int32_t word = seed` goes negative) × 400 draws, **plus every seed `0..=2047`** and every power-of-two neighbourhood × 64 draws | [x] `rng.rs::random_seeds` |
| 13 | `srand`/`rand` port vs real glibc | a **full** `ARRAY_SIZE` (262 144) draw sequence — exactly the program's seeding loop — for 7 seeds (`0`, `1`, `2`, `42`, `2³¹-1`, `2³¹`, `UINT_MAX`) | [x] `rng.rs::full_array_fill` |
| 14 | `srand`/`rand` port vs real glibc | re-seeding in the middle of a sequence; two different seeds interleaved | [x] `rng.rs::reseeding` |
| 15 | validation (`harness_parse_seed`) vs real glibc `strtoul` + the C condition | accepted forms: plain digits, `+`/`-` prefixes, leading zeros (1…64), leading whitespace (all six `isspace` bytes), `""`, `UINT_MAX` | [x] `parse.rs::accepted_forms` |
| 16 | validation vs real glibc `strtoul` | `UINT_MAX`/`ULONG_MAX`/`LONG_MAX` boundary values, ±1 around each | [x] `parse.rs::uint_max_boundary` |
| 17 | validation vs real glibc `strtoul` | `ERANGE` region: 20…300-digit numbers, with/without sign, with leading zeros | [x] `parse.rs::erange_boundary` |
| 18 | validation vs real glibc `strtoul` | unsigned negation region: `-0`, `-1`, `-4294967295`, `-18446744073709551615`, `-18446744069414584321` | [x] `parse.rs::negative_wraparound` |
| 19 | validation vs real glibc `strtoul` | 100 000 pseudo-random byte strings (50 000 from a digit/sign/space/garbage alphabet + 50 000 digit-heavy signed numbers), property-style, seed `0x5eed_1234` | [x] `parse.rs::random_strings_property` |
| 19a | validation vs real glibc `strtoul` | **exhaustive**: every 1-byte and every 2-byte argument over bytes `1..=255` (65 280 cases) + every 3-byte combination over the 26-byte alphabet the parser branches on (17 576 cases) | [x] `parse.rs::exhaustive_short_strings` |
| 19b | validation vs real glibc `strtoul` | for every digit length `1..=40`: the largest/smallest number of that length ±1, each with `-`/`+`/leading space/trailing space/leading zeros | [x] `parse.rs::digit_length_sweep` |
| 20 | `main` (composed pipeline, reduced `ITERATIONS`) | `srand(seed)` + 262 144-element `rand()` fill + **1×** `perform` + XOR reduce, for 24 seeds incl. `0`, `1`, `UINT_MAX`, `> 2³¹` | [x] `pipeline.rs::pipeline_one_iteration` |
| 21 | `main` (composed pipeline) | same with **3×** `perform` (state carried across calls) for 6 seeds | [x] `pipeline.rs::pipeline_three_iterations` |
| 22 | `main` (composed pipeline) | seed given in every accepted textual form (`"42"`, `"+42"`, `"0042"`, `" 42"`, `""`, `"-0"`, `"4294967295"`, `"-18446744073709551615"`) — asserts the decoded seed drives the same stream | [x] `pipeline.rs::pipeline_matches_for_seed_arguments` |
| 23 | `main` (composed pipeline) | repeatability: run the reduced pipeline twice in one process (global `array` retains state between calls) | [x] `pipeline.rs::pipeline_is_repeatable` |
| 24 | `main` (full program, `ITERATIONS = 2000`) | end-to-end through `dlopen`'d `main` with fd redirection, seeds `1` and `42`; ~5 min per side, so `#[ignore]`d and run explicitly | [x] `pipeline.rs::full_end_to_end` (`--ignored`) |
| 25 | `main` (full program) | the real artefacts: `c_src/build/driver <seed>` vs `target/release/driver <seed>`, stdout+stderr+exit status, seeds `1`, `42`, `0` | [x] `scripts/e2e_binaries.sh` |
| 26 | `main` | `argc == 2` valid seed → exit `0`, stdout `"<xor>\n"`, empty stderr | [x] `pipeline.rs::full_end_to_end`, `scripts/e2e_binaries.sh` |
| 27 | all of the above | C ground truth built `-O0` **and** `-O2` | [x] every test is parameterised over both `.so`s (`common::c_impls()`) |
| 28 | all of the above | Rust built with `overflow-checks = true` (dev) **and** `false` (release) | [x] `scripts/run_all.sh` runs the suite in both profiles |
| 29 | `perform_expensive_operations` | **exhaustive**: every one of the 2³² possible `int` values, once (16 384 calls × 262 144 slots), shardable | [x] `exhaustive.rs::exhaustive_domain_sweep` (`--ignored`, 16 shards) |
| 30 | `driver` binary (process level) | `argv[0]` forms: normal, absolute path, empty, non-UTF-8 bytes, `%s%d%n` (must not be interpreted), with/without extra args | [x] `binary_cli.rs::cli_argv0_variants` |
| 31 | `driver` binary (process level) | `execve` with an empty `argv` array (kernel normalises it to `argc == 1`, `argv[0] == ""`) | [x] `binary_cli.rs::cli_argc_zero` |
| 32 | `driver` binary (process level) | stdout+stderr are a pipe with no reader: a C program keeps the default `SIGPIPE` disposition and dies from signal 13; Rust's runtime sets `SIG_IGN`, so `src/main.rs` restores `SIG_DFL` | [x] `binary_cli.rs::cli_sigpipe_disposition` |
| 33 | `driver` binary (process level) | exit status + stdout + stderr for 20 rejected argument vectors, `argv[0]` matched via `Command::arg0` | [x] `binary_cli.rs::cli_error_paths` |
| 34 | `perform_expensive_operations` | regression anchors: `f(x)` and `f(f(x))` for `0`, `±1`, `±7`, `INT_MIN`, `INT_MAX` and the fixed point compared against values read out of the C `.so` and hard-coded in the test (proves the harness really observes the function, and pins the behaviour) | [x] `perform.rs::transformation_anchors` |

## Symbol-surface rows (Phase D, `tests/symbols.rs`)

| # | check | test |
|---|-------|------|
| 35 | every `nm -D` symbol of the C `.so` (both `-O0` and `-O2`) is exported by `libdriver.so` with the same name and kind | [x] `symbols.rs::every_c_symbol_is_exported_by_rust` |
| 36 | the Rust `.so` imports nothing but versioned libc/GCC symbols | [x] `symbols.rs::rust_so_has_no_unresolved_non_libc_symbols` |
| 37 | the C surface is exactly `{array, main, perform_expensive_operations}` (fails loudly if the C ever grows a symbol the suite does not exercise) | [x] `symbols.rs::c_exports_no_symbol_the_suite_forgot_to_exercise` |

## Measured results (evidence)

| what | result |
|------|--------|
| Row 29, exhaustive sweep | `EXHAUSTIVE SWEEP PASSED: all 4294967296 int values agree` — 16 384/16 384 chunks, 16 shards, C `-O2` vs Rust `release` |
| Row 24, `.so`-level full program, seed 42 | rust `430392287` (278.6 s), c-O0 `430392287` (688.5 s), c-O2 `430392287` (327.0 s); status 0, empty stderr for all three |
| Row 25, real binaries | seed 42 → `430392287`, seed 1 → `42032659`, seed 0 → `42032659` (identical stdout/stderr/status); plus 6 argv cases identical |
| Rows 1–23, 26–37 | `DIFFERENTIAL SUITE PASSED IN ALL CONFIGURATIONS` (44 tests, Rust `dev` **and** `release`, C `-O0` **and** `-O2`) |
| Feature combinations | `ALL FEATURE COMBINATIONS CHECK CLEAN` (the single empty combination) |

Note that seed `0` and seed `1` print the same value: glibc's `srandom_r`
silently remaps a 0 seed to 1, and `src/rng.rs` reproduces that.

## How to run everything

```sh
# Phase A: enumerate + check every Cargo feature combination (there are none,
# so this is {} == default == --all-features)
scripts/check_features.sh

# Phases B + C in every configuration ({dev, release} x C {-O0, -O2})
scripts/run_all.sh

# Row 29: exhaustive sweep of all 2^32 int values through the arithmetic core
scripts/exhaustive_sweep.sh 16 c-O2

# Rows 24-26: the unreduced program (~5 min per implementation)
E2E_SEEDS=42 E2E_C_IMPLS=c-O0,c-O2 \
  cargo test --release --test pipeline -- --ignored --nocapture
scripts/e2e_binaries.sh 42 1 0
```

Test files: `tests/symbols.rs` (Phase D), `tests/perform.rs`,
`tests/exhaustive.rs`, `tests/rng.rs`, `tests/parse.rs`, `tests/pipeline.rs`
(Phase B), `tests/errors.rs`, `tests/binary_cli.rs` (Phase C),
`tests/common/mod.rs` (dlopen plumbing).
