# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

**Build-time configuration axes**

* `Cargo.toml` has **no `[features]` section** (`grep -n features Cargo.toml`
  → no match) ⇒ exactly one Rust configuration. The only valid "combination" is
  the empty one, checked with `cargo check --no-default-features`.
* `c_src/CMakeLists.txt` declares no `option()`, no
  `target_compile_definitions`, no generator expressions; `driver.c`/`driver.h`
  contain no `#ifdef` other than the header include guard ⇒ exactly one C
  configuration.

**Runtime option / mode / flag axes**

* Public header `c_src/include/driver.h` declares exactly one entry point,
  `void driver(int x)` — it is simultaneously the highest-level and the
  lowest-level public entry point; there is no convenience wrapper hiding a
  lower layer, no context/handle object, no setter, no global, no mode enum.
  ⇒ **the option axis is empty**; nothing can be configured.

**Input-shape axes the code distinguishes**

`driver` computes `y = 2*x; y += 300; printf("%d\n", y)`. The compiled C is
straight-line (`add %eax,%eax` / `addl $0x12c` / `call printf@plt`), so the code
does not *branch* on the value — but the observable output is
value-dependent, and three sub-axes change the emitted bytes:

1. **sign / magnitude of the printed result** — `printf("%d")` emits a `-`
   prefix and a different digit count per magnitude (1 … 10 digits).
2. **32-bit wrapping** of `2*x` and of `+ 300` (the two arithmetic steps can
   each wrap independently).
3. **call multiplicity / stream state** — one call vs. many calls in one
   process, and the `stdout` buffering mode (regular file = fully buffered vs.
   pipe) which decides when bytes appear.

The table below is the pruned cross-product of these axes: every row is a class
the *output bytes* distinguish, and every row is exercised with many randomized
inputs from that class (fixed seed, deterministic SplitMix64 PRNG) plus its
exact boundary values.

## Table

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `driver` | no options (none exist); `x = 0` — the identity/empty shape, result `300` | `cfg_01_zero` | [x] |
| 2  | `driver` | `x` small positive (`1..=1000`), no wrap, positive result, 3–4 digits | `cfg_02_small_positive` | [x] |
| 3  | `driver` | `x` small negative (`-1..=-1000`), no wrap, result crosses into negatives | `cfg_03_small_negative` | [x] |
| 4  | `driver` | `x` in the exact zero-crossing window `-160..=-140`: result changes sign and `x = -150` yields exactly `0` (single digit, no sign) | `cfg_04_sign_crossing_window` | [x] |
| 5  | `driver` | `x` chosen so the result has each possible decimal width 1…10 digits, positive | `cfg_05_all_positive_digit_widths` | [x] |
| 6  | `driver` | `x` chosen so the result has each possible decimal width 1…10 digits, negative (`-` prefix) | `cfg_06_all_negative_digit_widths` | [x] |
| 7  | `driver` | `x` random over the whole `i32` range (no wrap constraint) — uniform 64 samples, mixes wrapping and non-wrapping | `cfg_07_random_full_i32_range` | [x] |
| 8  | `driver` | `x` random in `(INT_MAX/2, INT_MAX]` — `2*x` always wraps to negative, `+300` does not wrap | `cfg_08_random_multiply_wraps_positive_side` | [x] |
| 9  | `driver` | `x` random in `[INT_MIN, INT_MIN/2)` — `2*x` always wraps to positive | `cfg_09_random_multiply_wraps_negative_side` | [x] |
| 10 | `driver` | `x` random in `[INT_MAX/2 - 300, INT_MAX/2]` — `2*x` fits but `+ 300` overflows (second step wraps) | `cfg_10_random_addition_wraps` | [x] |
| 11 | `driver` | all four `i32` range endpoints (`INT_MIN`, `INT_MIN+1`, `INT_MAX-1`, `INT_MAX`) plus `±0x40000000`, `±0x3FFFFFFF` | `cfg_11_range_endpoints` | [x] |
| 12 | `driver` | powers of two and their negations (`±2^k`, `k = 0..=31`) — bit-pattern sweep | `cfg_12_powers_of_two` | [x] |
| 13 | `driver` | **many** calls: 256 randomized values invoked back-to-back inside one capture, comparing the whole multi-line transcript (call multiplicity axis; catches per-call state or missing newline) | `cfg_13_many_calls_one_transcript` | [x] |
| 14 | `driver` | **one** call, `stdout` redirected to a **pipe** instead of a regular file (buffering-mode axis: fully buffered vs. pipe) | `cfg_14_stdout_is_a_pipe` | [x] |
| 15 | `driver` | interleaved C/Rust calls in one process, alternating, sharing the same libc `stdout` FILE — verifies neither library corrupts the shared stream state | `cfg_15_interleaved_c_and_rust_calls` | [x] |
| 16 | `driver` | called through a `extern "C" fn(i64)` signature (widest-argument caller shape) with in-range low 32 bits — ABI/argument-passing axis | `cfg_16_called_via_wide_argument_abi` | [x] |
| 17 | `driver` | 1024 randomized values, exhaustive-ish property sweep, asserting C output == Rust output == `format!("{}\n", 2*x wrapping + 300)` | `cfg_17_property_sweep_1024` | [x] |
| 18 | `driver` | strided sweep over the **entire** `i32` domain — every `2^11`-th value plus a randomized offset inside each stride (**≈ 4.2 million distinct inputs**), compared as chunked transcripts; covers all four wrap regimes and every output width | `cfg_18_strided_full_range_sweep` | [x] |
| 19 | `driver` | **exhaustive, no sampling** over two contiguous windows: `[1073741674 ± 4096]` (the `+300` overflow boundary) and `[-4246 ..= 3946]` (the printed-value sign crossing) | `cfg_19_exhaustive_window_around_wrap_boundary` | [x] |

## Beyond the table: the input-value axis is verified EXHAUSTIVELY

Rows 1–19 are the row-per-configuration requirement, and rows 7–19 sample the
value axis randomly.  Because the only parameter is an `int`, the value axis can
be closed completely rather than sampled: `tests/exhaustive.rs` +
`./exhaustive_sweep.sh` compare C and Rust for **all 4 294 967 296** possible
arguments (see `VERIFICATION.md`).  That is what makes row 7–19 sampling
sufficient — a mutant that is wrong for exactly one input survives a 4.2 M-value
sweep but is caught, and pinpointed, by the exhaustive run.

The option axis needs no such treatment: it is empty (the library has no
options, modes, flags, handles or globals).

Notes on deliberately excluded rows:

* The function's *return value* is not part of the surface (`void` in the
  header); reading `eax` after the call would be reading an unspecified
  register, not defined behaviour, so no row asserts on it.
* Concurrent calls from multiple threads are excluded: `printf` interleaving
  order between threads is not deterministic in C either, so it cannot be
  compared byte-for-byte. The shared-stream concern is covered
  deterministically by row 15.
