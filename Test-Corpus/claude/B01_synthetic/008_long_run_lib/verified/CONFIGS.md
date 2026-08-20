# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## 1. Mechanical derivation of the axes

The C library (`c_src/src/long.c`, the only translation unit in
`c_src/CMakeLists.txt`) contains **no `if`, no `switch`, no `#ifdef`, no
option/flag/mode parameter and no data-dependent branch** — verified by

```
$ grep -n -E "if *\(|switch|#if" -r src include
include/long.h:24:#ifndef ECHO_H_          # include guard only
```

so the configuration surface is *not* built from runtime flags. What the C
code does branch on / is parameterised by is:

| axis | where it comes from in the C | values it can take |
|------|------------------------------|--------------------|
| A. entry point | the three exported ABI symbols (see `SYMBOLS.md`) | `perform_expensive_operations()` (lowest level), `long_exec(seed)` (one-shot wrapper), `array` (exported 1 MiB data object, read **and** written by callers) |
| B. `array` element value shape | `perform_expensive_operations` does value-dependent arithmetic: `x*3+7` / `x^(x>>3)` / `x-(x<<1)` / `x/2 + x%7` — overflow, arithmetic-shift sign propagation, truncating division and modulo sign all depend on the *value* | zeros (fresh `.bss`), all-`INT_MIN`, all-`INT_MAX`, all `-1`, uniform random `int32`, non-negative random (`rand()`-shaped, `0..2^31-1`), multiples of 7 / `±1`, powers of two `±1`, sign-alternating, near-`INT_MIN`/`INT_MAX` clusters, small magnitudes `-16..16` |
| C. how many elements are written by the caller | the loop `for (i = 0; i < ARRAY_SIZE; i++)` covers all 262144 elements; a caller may leave some at their previous value | all 262144, only `array[0]`, only `array[ARRAY_SIZE-1]`, sparse/strided subset, none (leftover state) |
| D. number of consecutive `perform_expensive_operations()` calls | `long_exec` calls it `ITERATIONS` (2000) times in a row; state lives in the global | 0, 1, 2, 3, 8 (and 2000 in the end-to-end row) |
| E. `seed` value for `long_exec` | `srand(seed)`, `unsigned int` | `0`, `1`, `42`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, random |
| F. observation channel | the two ways the result escapes: the exported `array` object, and `printf("%d\n", xor_result)` on stdout | array bytes (1 MiB, compared byte-for-byte), captured stdout bytes |
| G. Rust build profile | `Cargo.toml`: `[profile.release] opt-level = 3, panic = "abort", codegen-units = 1` vs the default dev profile — different codegen for the wrapping arithmetic | `target/debug/liblong.so`, `target/release/liblong.so` (both compared against the same C `.so`) |
| H. cargo features | `Cargo.toml` has **no `[features]` table**, so the only valid combination is the empty/default one | `--no-default-features` (≡ default) |

Rows below are the pruned cross product of those axes: one row per combination
the C treats differently. Every row is driven through the `.so` exports of both
libraries with **many randomized inputs (fixed seed, `SplitMix64`)** where the
row has a random component, and the full 1 MiB `array` plus (where relevant)
the captured stdout bytes are compared **byte-for-byte**.

## 2. Configuration surface

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| C1 | `array` (data symbol only) | fresh library load, nothing called: whole 1 MiB object must be zero in both (`.bss` placement, size `0x100000`) | `c1_fresh_bss_is_zero` | [x] |
| C2 | `array` write → read | write 262144 random `i32`, read back through the exported symbol, no function called: pure data-object ABI (size, element order, stride) | `c2_array_roundtrip` | [x] |
| C3 | `perform_expensive_operations` | D=1 call, B = all zeros (fresh `.bss`) | `c3_peo_zeros` | [x] |
| C4 | `perform_expensive_operations` | D=1, B = uniform random `i32` over the full range, C = all elements; **64 randomized trials** | `c4_peo_uniform_random` | [x] |
| C5 | `perform_expensive_operations` | D=1, B = non-negative random (`0..=0x7FFF_FFFF`, the shape `rand()` actually produces), C = all; 32 trials | `c5_peo_nonnegative_random` | [x] |
| C6 | `perform_expensive_operations` | D=1, B = all `INT_MIN` / all `INT_MAX` / all `-1` / all `1` / all `7` / all `-7` (uniform extreme arrays) | `c6_peo_uniform_extremes` | [x] |
| C7 | `perform_expensive_operations` | D=1, B = exhaustive-ish boundary set: `INT_MIN..INT_MIN+8`, `INT_MAX-8..INT_MAX`, `-16..16`, `±(2^k)`, `±(2^k ± 1)`, `±7k`, `±7k±1` tiled over the whole array | `c7_peo_boundary_tiling` | [x] |
| C8 | `perform_expensive_operations` | D=1, B = sign-alternating random (`x`, `-x`, …) — exercises the arithmetic-shift/truncating-division sign paths on adjacent elements | `c8_peo_sign_alternating` | [x] |
| C9 | `perform_expensive_operations` | D=1, C = **only `array[0]`** written (rest left zero) — first-element boundary | `c9_peo_first_element_only` | [x] |
| C10 | `perform_expensive_operations` | D=1, C = **only `array[ARRAY_SIZE-1]`** written — last-element boundary, catches off-by-one in the loop bound | `c10_peo_last_element_only` | [x] |
| C11 | `perform_expensive_operations` | D=1, C = sparse/strided subset (every 4093rd element random, rest zero); 8 randomized trials | `c11_peo_strided_subset` | [x] |
| C12 | `perform_expensive_operations` | D = 2, 3 and 8 consecutive calls with no re-write in between, B = random — verifies the global carries state identically and the transform composes identically | `c12_peo_repeated_calls` | [x] |
| C13 | `perform_expensive_operations` | D=0 calls after a write — verifies the function is not invoked implicitly at load time (constructor/`.init_array` parity) | `c13_no_implicit_invocation` | [x] |
| C14 | `perform_expensive_operations` + `array` | B = the exact array `long_exec` would build: `srand(seed)` + 262144 × `rand()`, injected into both `.so`s, then D = 3 calls, F = compare array bytes **and** the XOR reduction; 8 random seeds + `0`,`1`,`42`,`UINT_MAX` | `c14_long_exec_pipeline_composed` | [x] |
| C15 | `long_exec` | E ∈ {`42`, `0`, `7`, `0xFFFFFFFF`}, D = 2000 (the real `ITERATIONS`), F = captured stdout bytes of `printf("%d\n", xor)` + final `array` (full 1 MiB dump for seed 7) — the full end-to-end one-shot, C `.so` vs release Rust `.so` | `e2e_c` / `e2e_rust` / `e2e_compare` with `LONG_E2E_SEED=<seed>` (`#[ignore]`, ~520 s + ~480 s per seed) | [x] |
| C16 | `long_exec` (fill stage) | E = `0`, `1`, `42`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, random — verifies the PRNG stage of `long_exec` is the platform `srand`/`rand` in both (imported symbol check + injected-fill equality) | `c16_seed_axis` | [x] |
| C17 | `array` (F = XOR channel) | XOR reduction over the whole 1 MiB after a random fill and D=1 — the exact reduction `long_exec` prints, evaluated on both `.so`s | `c17_xor_reduction_channel` | [x] |
| C18 | `perform_expensive_operations` | G = both Rust profiles: every row above is executed against whichever `liblong.so` the current `cargo test` profile built, and `c18_debug_and_release_agree` additionally loads **both** `target/debug/liblong.so` and `target/release/liblong.so` in one process and compares them with C | `c18_debug_and_release_agree` | [x] |
| C19 | `perform_expensive_operations` | interleaved use of the two libraries in one process (C `.so` and Rust `.so` loaded simultaneously, alternating calls) — verifies each library uses *its own* `array` and there is no symbol interposition between them | `c19_libraries_are_independent` | [x] |
| C20 | *(whole suite)* | H = the only feature combination (`--no-default-features`, ≡ default, no `[features]` table) × both build profiles; whole suite re-run | `scripts/run_all.sh` | [x] |

## 3. Results

All 20 rows pass. Latest full runs (see `VERIFICATION.md` for the transcripts):

```
tests/phase_b_configs.rs   debug   18 passed   (177 s)
tests/phase_b_configs.rs   release 18 passed   ( 99 s)
```

Row C15 (`long_exec`, the real 2000 iterations) was run for seeds 42, 0, 7 and
0xFFFFFFFF; C and Rust produced byte-identical stdout, identical XOR and, for
seed 7, a byte-identical 1 MiB final `array`.
