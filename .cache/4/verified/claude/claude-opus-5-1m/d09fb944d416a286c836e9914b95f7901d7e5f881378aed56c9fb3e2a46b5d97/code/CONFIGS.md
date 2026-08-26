# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the C source, the public header, and the build files.

## Axis 1 — public entry points (from `c_src/include/driver.h` + `nm -D`)

| entry point | signature | level |
|---|---|---|
| `driver` | `void driver(int x, int y)` | the **only** entry point; it is simultaneously the lowest-level and the highest-level API. There are no convenience wrappers, no init/teardown, no context object, no one-shot vs. streaming variants. |

## Axis 2 — runtime options / modes / flags

Grep of the header and the body for settable state: **none**.
No globals, no `static` state, no setter functions, no option struct, no flags
argument, no `getenv`, no locale calls, no `#ifdef` that changes behaviour
(`driver.h`'s `%:ifndef DRIVER_H_` is a plain include guard), and
`c_src/CMakeLists.txt` defines no compile-time options (no
`target_compile_definitions`, no `option()`).

`Cargo.toml` declares **no `[features]` table**, so the Rust side likewise has
exactly one build configuration. The complete set of feature combinations is
therefore: `{ }` (the empty set) — i.e. `--no-default-features` and the default
build are the same configuration. `check_features.sh` enumerates this
mechanically (power set of the `[features]` table = 1 combination) so the loop
stays correct if features are ever added.

## Axis 3 — input shapes the code's data flow distinguishes

`result = x | ~y` then `printf("%d", result); puts("")`. The value-dependent
behaviour lives in (a) the bit pattern of the two `int`s and (b) the decimal
formatting of the result:

* sign of `x`: negative / zero / positive
* sign of `y`: negative / zero / positive
* collapsing values: `y = 0` (⇒ result `-1` always), `x = -1` (⇒ result `-1`
  always), `y = -1` (⇒ result `= x`, the identity path)
* boundary values: `INT_MIN`, `INT_MAX`, `0`, `-1`, `1`
* single-bit inputs — sweeps every one of the 32 bit positions, incl. the sign bit
* relations between the operands: `x == y`, `x == ~y`, `x & y` disjoint
* magnitude of the result ⇒ printed field width: 1…10 decimal digits, with and
  without a leading `-` (11 characters for `INT_MIN`). Only `x = 0, y = -1`
  yields the result `0`.

## Axis 4 — output-stream shape (the state `printf`/`puts` actually branch on)

The translation deliberately calls the platform libc so both libraries share the
process's `stdout` `FILE`. That makes the stream state a real configuration axis:

* destination kind: regular file (fully buffered) / pipe / `/dev/null`
* buffering mode set by the caller with `setvbuf`: `_IOFBF`, `_IOLBF`, `_IONBF`,
  and a pathological 1-byte `_IOFBF` buffer
* call sequencing: one call per capture / many calls concatenated in one capture
* interleaving with the caller's *own* `stdout` writes before/after the call
  (ordering is only correct if the library writes through the same `FILE`)

## Configuration table

Every row is exercised against **both** `.so` files through `libloading` with
many randomized inputs (SplitMix64, fixed seed `0x5EED_1234_ABCD_F00D`) unless
the row is exhaustive by construction. Tests live in `tests/phase_b_configs.rs`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `driver` | isolated single call, `x = 0, y = 0`; stdout = regular file, default buffering | `cfg01_single_call_zero_zero` | [x] |
| 2 | `driver` | isolated single call, `x = 0, y = -1` — the only input whose result is `0` | `cfg02_single_call_only_zero_result` | [x] |
| 3 | `driver` | random `x > 0`, `y > 0` (both positive), 5000 inputs | `cfg03_random_pos_pos` | [x] |
| 4 | `driver` | random `x > 0`, `y < 0`, 5000 inputs | `cfg04_random_pos_neg` | [x] |
| 5 | `driver` | random `x < 0`, `y > 0`, 5000 inputs | `cfg05_random_neg_pos` | [x] |
| 6 | `driver` | random `x < 0`, `y < 0`, 5000 inputs | `cfg06_random_neg_neg` | [x] |
| 7 | `driver` | uniform random over the **full** 32-bit domain for both args, 20000 inputs | `cfg07_random_full_32bit_domain` | [x] |
| 8 | `driver` | `x = 0` fixed, random full-range `y`, 5000 inputs (result `= ~y`) | `cfg08_x_zero_random_y` | [x] |
| 9 | `driver` | `y = 0` fixed, random full-range `x`, 5000 inputs (result collapses to `-1`) | `cfg09_y_zero_random_x` | [x] |
| 10 | `driver` | `x = -1` fixed, random full-range `y`, 5000 inputs (result collapses to `-1`) | `cfg10_x_minus_one_random_y` | [x] |
| 11 | `driver` | `y = -1` fixed, random full-range `x`, 5000 inputs — identity path, sweeps every printed width and both signs | `cfg11_y_minus_one_random_x_identity` | [x] |
| 12 | `driver` | `x == y`, random full-range, 5000 inputs | `cfg12_x_equals_y` | [x] |
| 13 | `driver` | `x == !y` (`x == ~y`), random full-range, 5000 inputs | `cfg13_x_equals_complement_y` | [x] |
| 14 | `driver` | exhaustive 5×5 grid of the boundary set `{INT_MIN, -1, 0, 1, INT_MAX}` × itself | `cfg14_exhaustive_boundary_grid` | [x] |
| 15 | `driver` | exhaustive small-magnitude grid `x, y ∈ [-4, 4]` (81 combinations, 1–2 digit output) | `cfg15_exhaustive_small_magnitude_grid` | [x] |
| 16 | `driver` | exhaustive single-bit sweep: `x = 1 << i`, `y = 1 << j` for all `i, j ∈ [0, 31]` (1024 combinations, incl. the sign bit) | `cfg16_exhaustive_single_bit_sweep` | [x] |
| 17 | `driver` | hand-picked width sweep: results of 1…10 digits, positive and negative, plus `INT_MIN` (11 chars) and `INT_MAX` | `cfg17_printed_width_sweep` | [x] |
| 18 | `driver` | many sequential calls concatenated into **one** capture (output accumulation, no separator between calls), 20000 random inputs | `cfg18_batched_sequential_output_accumulation` | [x] |
| 19 | `driver` | **one capture per call** (fresh redirect + flush each time), 300 random inputs — proves per-call byte exactness, not just the concatenated stream | `cfg19_isolated_capture_per_call` | [x] |
| 20 | `driver` | caller writes its own bytes to `stdout` via libc `printf` immediately before and after the call — verifies the library writes through the *same* `FILE` (ordering), 200 random inputs | `cfg20_interleaved_with_caller_stdio` | [x] |
| 21 | `driver` | stdout `setvbuf(_IOLBF, 4096)` (line buffered), random inputs | `cfg21_stdout_line_buffered` | [x] |
| 22 | `driver` | stdout `setvbuf(_IONBF)` (unbuffered — `printf` and `puts` each reach `write(2)` separately), random inputs | `cfg22_stdout_unbuffered` | [x] |
| 23 | `driver` | stdout `setvbuf(_IOFBF, 4096)` with a batch large enough to wrap/flush the buffer many times (50000 calls, ≈ 500 KB) | `cfg23_stdout_fully_buffered_buffer_wrap` | [x] |
| 24 | `driver` | stdout is a **pipe** rather than a regular file, random inputs (different default buffering decision inside glibc) | `cfg24_stdout_is_a_pipe` | [x] |
| 25 | `driver` | both `.so`s loaded into the process at the same time, each exporting `driver`; alternate C/Rust calls within one capture — verifies no symbol interposition between the two libraries and that state is not shared | `cfg25_no_interposition_alternating_calls` | [x] |
| 26 | `driver` | deep formatting sweep: one **contiguous** block of 2·2^19+1 consecutive results straddling zero, plus exhaustive ±64 windows around every power of ten, every power of two, and both domain extremes (> 1.07 M inputs). Random sampling of a 2^32 domain leaves the decimal-carry windows (`999`→`1000`, `2^k`) essentially untouched | `cfg26_deep_contiguous_and_boundary_window_sweep` | [x] |

## Status

All 26 rows pass byte-for-byte across their randomized inputs under the single
(only) feature configuration, for **both** the `dev` and the `release` cdylib
(`[profile.release]` sets `panic = "abort"`, so it is built and differentially
tested separately). See `./check_features.sh test`.

### Negative control (`./mutation_check.sh`)

A passing suite only means something if it can fail. `mutation_check.sh` injects
six wrong variants of the translated expression, runs the whole suite
(`--no-fail-fast`, 46 tests) against each, and always restores the original:

| mutant | expression | tests catching it | tests surviving |
|---|---|---|---|
| `drop_complement` | `x \| y` | 41 | 5 |
| `and_instead_of_or` | `x & !y` | 41 | 5 |
| `xor_instead_of_or` | `x ^ !y` | 35 | 11 |
| `complement_x_not_y` | `!x \| y` | 41 | 5 |
| `off_by_one` | `(x \| !y) + 1` | 43 | 3 |
| `swap_operands` | `y \| !x` | 41 | 5 |

Every mutant is detected. Each surviving row is provably insensitive to its
mutant rather than a coverage gap — e.g. row 10 pins `x = -1` (both `x\|~y` and
`x\|y` give `-1`), row 12 pins `x == y` (`x\|~x`, `x^~x` and `~x\|x` are all
`-1`), and rows 11/17/26 pin `y = -1` (`x\|0` and `x^0` are both `x`). The three
constant survivors across all mutants are the symbol/feature-surface tests,
which correctly do not depend on the arithmetic.
