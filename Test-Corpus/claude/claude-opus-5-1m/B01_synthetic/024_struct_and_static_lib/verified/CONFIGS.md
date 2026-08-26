# CONFIGS.md — Phase B configuration-surface table

## Build-time configuration

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` has no
`option()`, no `add_definitions`, no `target_compile_definitions`, and the C source
has no `#ifdef` other than the `DRIVER_H_` include guard. Therefore the complete
set of valid feature combinations is a single one: the empty set.

| # | combination | `cargo check` | `cargo test` |
|---|-------------|---------------|--------------|
| F1 | *(no features)* — `--no-default-features` | [x] | [x] |
| F2 | *(default)* — identical to F1, no `default` feature exists | [x] | [x] |

Automated by `run_all.sh`, which parses `[features]` from `Cargo.toml`, enumerates
the power set, and loops `cargo check` / `cargo test` over it.

## Runtime configuration axes (derived from the C source)

There are **no runtime options or flags** — no setters, no mode enum, no global
config. The axes the C code's behaviour actually varies over are therefore:

* **A. entry point** — `run` (the low-level exported function, `driver.c:53`) and
  `driver` (the wrapper that calls `run` twice, `driver.c:63`). Both are exercised
  directly; `run` is *not* reachable from `driver.h`, so testing only the header's
  `driver` would miss half the ABI.
* **B. argument value** — `int extra_bedrooms` / `int x`, used in
  `house->bedrooms += extra_bedrooms` (`driver.c:42`): zero, ±1, small, large,
  `INT_MAX`, `INT_MIN`, and values crafted to land `bedrooms` on a boundary.
* **C. state shape (the hidden input)** — `static house_t the_house`
  (`driver.c:35`) persists across calls, so *every* call's output depends on the
  whole preceding call sequence. Shapes: fresh instance (`floors=2, bedrooms=5,
  bathrooms=2.5`) vs. accumulated after 1, 2, 3, many calls.
* **D. call-sequence interleaving** — `run`-only, `driver`-only, and mixed
  sequences: `driver` advances the state twice as fast, so interleaving produces
  states unreachable by either alone.
* **E. printed shape at `print_the_house` (`driver.c:50`)** — the `%d`/`%.1f`
  formatting cases the accumulated state can reach: single- vs multi-digit
  `floors` (≥10, ≥100, ≥1000), `bedrooms` positive / zero / negative /
  `INT_MIN`-adjacent, and `bathrooms` half-integers of growing magnitude.

Each row below is a combination of those axes that the C treats differently. Every
row is driven through **both** `.so`s via `libloading` and compared byte-for-byte,
using **many randomized inputs per row** from a fixed-seed PRNG (`SEED = 0x5EED_1234_ABCD_0001`)
unless the row is a specific boundary combination. Fresh state is obtained by
`dlopen`ing a byte-copy of the `.so` under a unique path (distinct inode ⇒ a new,
independent set of globals), which the row C0 test verifies.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C0 | `run` | *loader invariant*: two byte-copies of the same `.so` each start from the fresh state `floors=2, bedrooms=5, bathrooms=2.5`; C and Rust instances are mutually isolated (RTLD_LOCAL, no `run` interposition) | `cfg_c0_fresh_state_isolation` | [x] |
| C1 | `run` | fresh instance, single call, `x = 0` (state unchanged by the argument) | `cfg_c1_run_fresh_zero` | [x] |
| C2 | `run` | fresh instance, single call, `x = 1` / `x = -1` (±1 around the no-op) | `cfg_c2_run_fresh_plus_minus_one` | [x] |
| C3 | `run` | fresh instance, single call, **randomized** small `x ∈ [-1000, 1000]`, 200 fresh instances | `cfg_c3_run_fresh_random_small` | [x] |
| C4 | `run` | fresh instance, single call, **randomized** full-range `x ∈ [INT_MIN, INT_MAX]`, 200 fresh instances (wrap-inducing values) | `cfg_c4_run_fresh_random_full` | [x] |
| C5 | `driver` | fresh instance, single call, `x = 0` — 8 output lines, `run` applied twice to shared state | `cfg_c5_driver_fresh_zero` | [x] |
| C6 | `driver` | fresh instance, single call, **randomized** small `x`, 200 fresh instances | `cfg_c6_driver_fresh_random_small` | [x] |
| C7 | `driver` | fresh instance, single call, **randomized** full-range `x`, 200 fresh instances | `cfg_c7_driver_fresh_random_full` | [x] |
| C8 | `run` | accumulated state: 300 sequential `run` calls with randomized small `x`, compared after **every** call (drives `floors` through 10/100 digit widths, `bathrooms` to 302.5) | `cfg_c8_run_sequence_small` | [x] |
| C9 | `run` | accumulated state: 300 sequential `run` calls with randomized **full-range** `x`, compared after every call (repeated wrap-around of `bedrooms`) | `cfg_c9_run_sequence_full` | [x] |
| C10 | `driver` | accumulated state: 300 sequential `driver` calls with randomized small `x`, compared after every call (`floors` +2 per call → ≥600) | `cfg_c10_driver_sequence_small` | [x] |
| C11 | `driver` | accumulated state: 300 sequential `driver` calls with randomized full-range `x` | `cfg_c11_driver_sequence_full` | [x] |
| C12 | `run` + `driver` mixed | randomized interleaving (300 steps, PRNG picks the entry point *and* the argument from a mixed value pool incl. `INT_MIN`/`INT_MAX`/0/±1) — states unreachable by either entry point alone | `cfg_c12_mixed_random_interleave` | [x] |
| C13 | `run` | `bedrooms` steered onto exact boundaries in sequence: `0`, `-1`, `1`, `INT_MAX`, `INT_MIN`, then one step past each | `cfg_c13_bedrooms_boundary_walk` | [x] |
| C14 | `run` | long accumulation for `%d` digit-width growth: 1200 calls with `x = 0` → `floors` 2→1202 (1→2→3→4 digits), `bathrooms` 2.5→1202.5 | `cfg_c14_digit_width_growth` | [x] |
| C15 | `driver` | `driver` twice in a row from fresh state (`x` random full-range) — checks the wrapper's second invocation sees the first's accumulated state | `cfg_c15_driver_twice` | [x] |
| C16 | `run` then `driver` (and `driver` then `run`) | order sensitivity: both orders from fresh instances with the same randomized `x`, 100 pairs | `cfg_c16_order_sensitivity` | [x] |
| C17 | `run` | `bathrooms` magnitude: 2000 calls so `bathrooms` reaches 2002.5, verifying `%.1f` of large half-integers stays identical | `cfg_c17_bathrooms_magnitude` | [x] |
| C18 | `run` / `driver` | argument extremes as raw bit patterns from fresh instances: `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, `0x00000001`, `0xDEADBEEF`, `0x80000001`, `0x7FFFFFFE` × both entry points (cross-product) | `cfg_c18_raw_bit_pattern_matrix` | [x] |
| C19 | `run` / `driver` | property-style sweep: 60 fresh instance pairs × random sequence length 1..8 × random entry point per step × random argument, comparing after every step (full cross-product of axes A–E under randomization) | `cfg_c19_property_sweep` | [x] |
| C20 | `run` / `driver` | *process-level* configuration: a child process calls the entry point and returns from `main` **without** any explicit `fflush`, so only libc's exit handling flushes the buffered `printf` output (`arg ∈ {0, 7, -3, INT_MAX, INT_MIN, 0xDEADBEEF}` × both entry points) | `cfg_c20_exit_flush_parity` | [x] |

## How the differential comparison is made trustworthy

1. **Output is captured at the file-descriptor level.** `print_the_house` writes
   from inside libc, so the tests `dup2` temporary files over fds 1 *and* 2
   (stderr too, so a Rust panic/abort diagnostic that C never produces is caught),
   `fflush(NULL)`, then restore. Bytes are compared with `assert_eq!`.
2. **Fresh state comes from a fresh `dlopen`.** Each pair copies the `.so` to a
   unique path; the distinct inode makes glibc map an independent instance with
   its own `the_house`. Row C0 asserts this actually happens (a stale-state or
   shared-state loader would otherwise silently invalidate every row).
3. **The runner is sequential (`harness = false`).** With libtest's default
   multi-threaded harness, its own progress lines on fd 1 land inside a capture
   and fake divergences; the custom `main` removes that non-determinism, and
   results are identical with or without `--test-threads`.
4. **The suite is mutation-validated.** With the C untouched, eight independent
   mutations of `src/driver.rs` were each caught (15–31 of the 32 cases failing),
   and the un-mutated baseline is green:
   `%.1f`→`%.2f`; initial `bathrooms` 2.5→2.0; initial `bedrooms` 5→6;
   `wrapping_add`→`saturating_add`; print/`add_floor` reorder; `driver` calling
   `run` once; dropped `floors++`; dropped `bathrooms += 1.0`; swapped `printf`
   argument order.
5. **Stale-artifact guard.** `cargo test` does *not* rebuild a
   `crate-type = ["cdylib"]` artifact, so a plain `cargo test` after an edit can
   load an out-of-date `.so` and pass vacuously (this was observed: the first
   mutation round produced 31/31 "passes" against a stale `.so`).
   `assert_not_stale` now fails loudly if `target/*/libdriver.so` is older than
   `src/*.rs`, or `c_src/build/libdriver.so` older than `c_src/**.{c,h}`, and
   `run_all.sh` always runs `cargo build` before `cargo test`.
