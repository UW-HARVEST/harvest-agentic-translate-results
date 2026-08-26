# CONFIGS.md — configuration surface table (Phase A, gated in Phase B)

Derived **mechanically** from `c_src/src/driver.c`, `c_src/include/driver.h`
and `c_src/CMakeLists.txt`.

## Build-time configuration axes: NONE

* `Cargo.toml` has **no `[features]` section** -> exactly **one** valid feature
  combination: the empty set (`--no-default-features` == default == no
  features). Verified: `grep -n '^\[features\]' Cargo.toml` -> no match, and
  `grep -rn 'feature *=' src/` -> no match.
* `c_src/CMakeLists.txt` declares no `option()`, no
  `add_compile_definitions`, no `target_compile_definitions`, and the C sources
  contain no `#if`/`#ifdef` other than the `DRIVER_H_` include guard.

So Phases B and C are run once, and re-run under `--no-default-features` to
prove the (single) combination is covered — see `run_all_feature_combos.sh`.

## Runtime configuration axes: NO FLAGS, NO STATE

`grep -n 'enum|flag|mode|option|struct|typedef|static|extern|global' c_src/**`
-> no match. There is no init function, no context object, no settable option,
and no global/static state. The library is three pure-ish functions.

The axes the C code actually branches on are therefore purely **input shape**:

| axis | values the C distinguishes | evidence |
|------|----------------------------|----------|
| A. entry point | `fma_array` (lowest level), `call_fma` (mid), `driver` (top / one-shot) | all three are `T` in `nm -D` |
| B. `len` sign/size | `len < 0`, `len == 0` (explicit guard), `len == 1` (`out[len-1] == out[0]`), `len >= 2`, large `len` | `if (len == 0) return 0;`, `for (i = 0; i < len; i++)`, `out[len-1]`, VLA sizing |
| C. element value regime | small values, full `int` range, values whose `mul1*mul2+add` overflows `int` (signed overflow) | `out[i] = mul1[i] * mul2[i] + add[i]` |
| D. pointer aliasing | disjoint buffers, `out` aliasing `mul1` / `mul2` / `add`, all four identical | `int *restrict out` (restrict is *declared*, so aliasing is a distinguishable shape) |
| E. token count (`driver`) | 0, 1, 2, 99, 100 (loop bound), 101, 150 | `for (i = 0; i < 100; i++)` and `data[100]` |
| F. separator bytes (`driver`) | `' '`, `'\t'`, `'\n'`, `'\r'`, `'\v'`, `'\f'`, runs of them, leading/trailing runs | `%d` whitespace-skipping |
| G. sign / digit form (`driver`) | unsigned, `+`, `-`, leading zeros, `-0` | `%d` grammar |
| H. numeric range (`driver`) | in-`int`, `INT_MAX`, `INT_MIN`, out-of-`int`-in-`long`, out-of-`long` | glibc `%d` -> `long` -> truncate to `int` |
| I. `%zn` cursor advance (`driver`) | tokens preceded by whitespace (so `nb` > digits consumed), tokens at offset 0 | `in += nb` |
| J. output channel | `printf("%d\n", result)` on fd 1 — compared **byte for byte** | `driver.c:59` |

## Table (one row per combination the C treats differently)

Every row is driven with **many randomized inputs** (SplitMix64, fixed seed
`0x5EED_C0DE_1234_5678`), not one hand-picked value, and asserted
byte-identical between the C `.so` and the Rust `.so` loaded via `libloading`.

### `fma_array` — the lowest-level entry point (called directly, not only through wrappers)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `fma_array` | `len == 0`, disjoint buffers, pre-filled `out` sentinel | `c1_fma_array_len0` | [x] |
| C2 | `fma_array` | `len == 1`, small random values | `c2_fma_array_len1_small` | [x] |
| C3 | `fma_array` | `len == 1`, full-`int`-range random values (overflow expected) | `c3_fma_array_len1_fullrange` | [x] |
| C4 | `fma_array` | `len == 2..=16`, small random values, no overflow | `c4_fma_array_small_len_small_vals` | [x] |
| C5 | `fma_array` | `len == 2..=16`, full-`int`-range random values (signed overflow on nearly every element) | `c5_fma_array_small_len_fullrange` | [x] |
| C6 | `fma_array` | `len == 2..=16`, operands drawn from the boundary set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` (cross-product sampled) | `c6_fma_array_boundary_values` | [x] |
| C7 | `fma_array` | `len == 1024..=4096` (large), full-range random values | `c7_fma_array_large_len` | [x] |
| C8 | `fma_array` | `len < 0` (random negative incl. `INT_MIN`), buffers pre-filled — must be a no-op in both | `c8_fma_array_negative_len_noop` | [x] |
| C9 | `fma_array` | aliasing: `out == mul1` (restrict violated), random `len`/values | `c9_fma_array_alias_out_mul1` | [x] |
| C10 | `fma_array` | aliasing: `out == mul2`, random `len`/values | `c10_fma_array_alias_out_mul2` | [x] |
| C11 | `fma_array` | aliasing: `out == add`, random `len`/values | `c11_fma_array_alias_out_add` | [x] |
| C12 | `fma_array` | aliasing: all four pointers identical, random `len`/values | `c12_fma_array_alias_all_same` | [x] |
| C13 | `fma_array` | aliasing: `out` overlapping `mul1` at an offset (`out = buf`, `mul1 = buf+1`) — order-of-write sensitive | `c13_fma_array_alias_offset_overlap` | [x] |

### `call_fma` — mid-level entry point

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C14 | `call_fma` | `len == 0` (explicit guard), random `data` | `c14_call_fma_len0` | [x] |
| C15 | `call_fma` | `len == 1` (`out[len-1] == out[0]`), random `data` over full `int` range | `c15_call_fma_len1` | [x] |
| C16 | `call_fma` | `len == 2..=64`, small random `data` | `c16_call_fma_small_len_small_vals` | [x] |
| C17 | `call_fma` | `len == 2..=64`, full-`int`-range random `data` (incl. `INT_MIN`/`INT_MAX`) | `c17_call_fma_small_len_fullrange` | [x] |
| C18 | `call_fma` | `len == 2..=64`, `data` drawn only from the boundary set `{INT_MIN, -1, 0, 1, INT_MAX}` | `c18_call_fma_boundary_values` | [x] |
| C19 | `call_fma` | large `len` (`1024..=65536`) that fits the C VLA stack budget, full-range values | `c19_call_fma_large_len` | [x] |
| C20 | `call_fma` | `len` == 100 exactly (the shape `driver` produces at its loop bound), random `data` | `c20_call_fma_len_100` | [x] |
| C21 | `call_fma` | `data` slice taken at a non-zero offset into a bigger buffer (unaligned-ish/offset pointer), random `len` | `c21_call_fma_offset_data_ptr` | [x] |

### `driver` — top-level entry point (stdout bytes compared)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C22 | `driver` | 1 token, random in-range value, no surrounding whitespace | `c22_driver_one_token` | [x] |
| C23 | `driver` | 2..=20 tokens, single-space separated, random in-range values | `c23_driver_few_tokens_space` | [x] |
| C24 | `driver` | 2..=20 tokens, **randomized separator runs** drawn from `{' ','\t','\n','\r','\v','\f'}` with random run lengths, plus random leading and trailing whitespace | `c24_driver_random_whitespace_mix` | [x] |
| C25 | `driver` | 2..=20 tokens with randomized sign form: bare / explicit `+` / explicit `-` / random leading-zero padding / `-0` | `c25_driver_sign_and_leading_zeros` | [x] |
| C26 | `driver` | tokens sampled from the numeric boundary set `{0, 1, -1, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1}` | `c26_driver_boundary_numbers` | [x] |
| C27 | `driver` | tokens that overflow `int` but fit `long`, and tokens that overflow `long` (random digit counts 10..30, random sign) | `c27_driver_out_of_range_numbers` | [x] |
| C28 | `driver` | exactly 99 / 100 / 101 tokens (the loop bound and one step either side), random values | `c28_driver_token_count_boundary` | [x] |
| C29 | `driver` | 101..=300 tokens (well past the bound), random values | `c29_driver_many_tokens` | [x] |
| C30 | `driver` | valid prefix then a random non-numeric byte at a random token index (early-exit shape) | `c30_driver_random_early_exit` | [x] |
| C31 | `driver` | fully random ASCII byte soup (printable + whitespace, length 0..=256) — property fuzz over the whole parser | `c31_driver_random_ascii_fuzz` | [x] |
| C32 | `driver` | fully random **arbitrary** bytes 1..=255 (non-ASCII included), length 0..=256 | `c32_driver_random_byte_fuzz` | [x] |
| C33 | `driver` | random digit-string soup: alternating digit runs and random separators, so `%zn` cursor advance is stressed at many offsets | `c33_driver_digit_run_fuzz` | [x] |
| C34 | `driver` | long input (4 KiB..=64 KiB) of random tokens — stresses `in += nb` accumulation past the 100-token cutoff | `c34_driver_long_input` | [x] |
| C35 | `driver` | token immediately followed by an alpha suffix at a random position (`"12abc"` shape), random index | `c35_driver_alpha_suffix` | [x] |

### Cross-entry-point / pipeline composition

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C36 | `driver` -> `call_fma` -> `fma_array` (full composed pipeline) | randomized inputs; assert the composed result equals the independently computed `call_fma(parsed_tokens, n)` in *both* libraries, and that C and Rust agree | `c36_pipeline_consistency` | [x] |
| C37 | `call_fma` -> `fma_array` | drive `call_fma` and the equivalent explicit `fma_array(out, ones, data, zeros, len)` sequence, both libraries, random `len`/values | `c37_call_fma_matches_manual_fma_array` | [x] |
| C38 | `fma_array` | repeated calls reusing the same `out` buffer with decreasing `len` (stale-tail shape: verifies the tail is left untouched) | `c38_fma_array_stale_tail` | [x] |
| C39 | glibc entry point equivalence | `sscanf` (Rust's import) vs `__isoc99_sscanf` (C's import) with `"%d%zn"` over randomized inputs | `sscanf_entrypoint_equivalence_d_zn` | [x] |

## Build-configuration axis of the *translated* artifact

`Cargo.toml` declares no features, but the cdylib's own compilation settings do
change generated code for constructs this translation relies on (wrapping
arithmetic, the deliberate stack probe, and rustc's UB instrumentation). The
suite is therefore replayed across each setting by `run_all_feature_combos.sh`,
which drives them through `DRIVER_RUST_OPT` / `DRIVER_RUST_DEBUG_ASSERTIONS`:

| # | configuration | entry points covered | result |
|---|---------------|----------------------|--------|
| B1 | features `''` (the only combination), cdylib `-Copt-level=0 -Cdebug-assertions=off` | all of C1..C39 + E1..E21 + G1..G6 | pass (67/67) |
| B2 | features `''`, cdylib `-Copt-level=2 -Cdebug-assertions=off` | all rows | pass (67/67) |
| B3 | features `''`, cdylib `-Copt-level=3 -Cdebug-assertions=off` | all rows | pass (67/67) |
| B4 | features `''`, cdylib `-Copt-level=0 -Cdebug-assertions=on` (dev profile) | all rows | pass (67/67) |
| B5 | features `''`, cdylib `-Copt-level=3 -Cdebug-assertions=on` | all rows | pass (67/67) |
| B6 | the actual `cargo build --release` artifact (`panic = "abort"`), via `DRIVER_RUST_SO` | all rows | pass (67/67) |

`cargo check --no-default-features --features ''`, `cargo check` (default) and
`cargo build --release` all succeed with no warnings.

Note on B4/B5: `cfg(debug_assertions)` enables rustc's MIR null-pointer check,
which changes how a NULL dereference terminates. The suite asserts the correct
expectation per configuration rather than skipping it — see the E5/E9 note in
`ERRORS.md`.

## Coverage summary

* 39/39 `CONFIGS.md` rows pass, each over many randomized inputs from the fixed
  seed `0x5EED_C0DE_1234_5678` (`tests/phase_b_valid.rs`).
* Lowest-level entry point `fma_array` is driven **directly** (rows C1..C13),
  including the four aliasing shapes its `restrict` qualifier makes
  distinguishable — not only through the `call_fma` / `driver` wrappers.
* The composed pipeline is checked as a pipeline (C36..C38), so a bug that only
  appears when `driver` -> `call_fma` -> `fma_array` are chained is visible.
* `driver` output is compared as the raw byte stream on fd 1, so formatting and
  newline differences are caught, not just the parsed integer.
