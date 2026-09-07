# CONFIGS.md — configuration-surface table (valid inputs)

The mirror of `ERRORS.md`: every **valid** configuration the C code actually
branches on. Derived mechanically from the branch points in `c_src/src/*.c`, not
from a guess about which cases matter.

## Axes the C code actually distinguishes

There are **no runtime option/mode/flag parameters** in this API — the public
headers expose no flags, no enums, no `setopt`-style state, and no `#ifdef`
(`grep -n 'enum\|#if' c_src/{src,include}/*` → 0 hits). Consequently the
configuration space is entirely made of **input shapes**, listed below with the
C branch each one selects.

| axis | values the C distinguishes | branch site |
|------|----------------------------|-------------|
| `height` | `<0` (huge `malloc` → NULL), `0` (`malloc(0)`, loop skipped), `1`, `>1` | matrix.c:43,50 / 91 |
| `width` | `<0`, `0`, `1`, `>1` | matrix.c:51 / 100 |
| shape class | square, wide (`w>h`), tall (`h>w`), degenerate (`0` in a dim), `1x1`, vector (`1xN`, `Nx1`) | multiply / to_string loops |
| row-token supply | exactly `height` rows, MORE rows than `height` (extras ignored), consecutive `\n` (strtok_r collapses delimiters), trailing `\n` | matrix.c:91,108 |
| col-token supply | exactly `width` cols, MORE cols than `width` (extras ignored), multiple/leading/trailing spaces (collapsed by `strtok_r`) | matrix.c:100,104 |
| element text | positive, negative, zero, multi-digit, `+`-prefixed, leading whitespace, non-numeric (`atoi`→0), partially numeric (`12abc`→12), out-of-`int`-range (`atoi` = `(int)strtol` clamp) | `atoi`, matrix.c:103 |
| element value magnitude | 1-digit … 10-digit, and the `j < width-1` separator branch (last column vs not) | matrix.c:151,155 |
| multiply inner dim | `mat_a->width == mat_b->height` = `0`, `1`, `>1`; accumulation with `int` overflow (wrapping) | matrix.c:119,127-130 |
| `matrix_to_string` `buffer_size` | positive (normal), overflowed-negative (→ `malloc` NULL) | matrix.c:142-145 |
| `write_to_file` content | empty string, short (< stdio buffer), long (> stdio buffer, forces flush inside `fprintf`), embedded `\n`, embedded `%` (goes through `"%s"`, must NOT be re-expanded), embedded NUL-free UTF-8 | write.c:43 |
| `write_to_file` target | new file, existing file (truncated by `"w"`) | write.c:38 |
| `driver` | valid multiplication end-to-end; writes relative `matrix.txt` in the cwd | driver.c whole body |
| call hierarchy | `allocate_matrix` alone (lowest level), `initialize_matrix_from_string`, `multiply_matrices` on hand-built matrices, `matrix_to_string` on hand-built matrices, `write_to_file` alone, and the fully composed `driver` | all |

## Rows — one per meaningful combination

Every row is exercised on **both** `.so`s through `libloading` and compared
byte-for-byte (return value, full `matrix_t` contents read back through the
returned pointer, produced C string, `errno`-style return code, written file
bytes, and captured `stderr`). Rows marked *(fuzzed)* use ≥100 randomized cases
from a fixed seed (`SEED = 0x5EED_1234_ABCD_0001`).

### A. `allocate_matrix` — lowest-level entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| A1 | `allocate_matrix` | `1x1` | [x] `cfg_a1_allocate_1x1` |
| A2 | `allocate_matrix` | square `NxN`, N in 2..8 | [x] `cfg_a2_allocate_square` |
| A3 | `allocate_matrix` | `width=0, height>0` (zero-size row mallocs) | [x] `cfg_a3_allocate_zero_width` |
| A4 | `allocate_matrix` | `width>0, height=0` (zero-size row array, loop skipped) | [x] `cfg_a4_allocate_zero_height` |
| A5 | `allocate_matrix` | `width=0, height=0` | [x] `cfg_a5_allocate_zero_zero` |
| A6 | `allocate_matrix` | wide (`w=7,h=2`) and tall (`w=2,h=7`) | [x] `cfg_a6_allocate_wide_and_tall` |
| A7 | `allocate_matrix` | *(fuzzed)* random `width,height` in `0..=32`; compare `width`/`height` fields, non-nullness, and writability of every row slot | [x] `cfg_a7_allocate_fuzz` |

### B. `initialize_matrix_from_string`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| B1 | `initialize_matrix_from_string` | `1x1`, single value | [x] `cfg_b1_init_1x1` |
| B2 | `initialize_matrix_from_string` | square `3x3`, exact rows/cols, single spaces, no trailing `\n` | [x] `cfg_b2_init_square_exact` |
| B3 | `initialize_matrix_from_string` | same but WITH trailing `\n` | [x] `cfg_b3_init_trailing_newline` |
| B4 | `initialize_matrix_from_string` | MORE rows supplied than `height` (extras ignored) | [x] `cfg_b4_init_extra_rows_ignored` |
| B5 | `initialize_matrix_from_string` | MORE cols supplied than `width` (extras ignored) | [x] `cfg_b5_init_extra_cols_ignored` |
| B6 | `initialize_matrix_from_string` | multiple/leading/trailing spaces and consecutive `\n\n` (delimiter collapsing) | [x] `cfg_b6_init_delimiter_collapsing` |
| B7 | `initialize_matrix_from_string` | non-numeric (`abc`), partially numeric (`12abc`), `+5`, `  7`, `-0`, out-of-range `99999999999`, `-99999999999` → `atoi` semantics | [x] `cfg_b7_init_atoi_semantics` |
| B8 | `initialize_matrix_from_string` | `width=0, height=3` (no columns consumed; row tokens still walked) | [x] `cfg_b8_init_zero_width` |
| B9 | `initialize_matrix_from_string` | `height=0` (nothing consumed, valid empty matrix) | [x] `cfg_b9_init_zero_height` |
| B10 | `initialize_matrix_from_string` | wide `4x2` and tall `2x4` | [x] `cfg_b10_init_wide_and_tall` |
| B11 | `initialize_matrix_from_string` | *(fuzzed)* random dims `0..=10`, random values in `-999_999_999..=999_999_999`, random single-space rendering | [x] `cfg_b11_init_fuzz` |
| B12 | `initialize_matrix_from_string` | *(fuzzed)* random dims + randomly-injected extra whitespace runs and extra trailing rows/cols | [x] `cfg_b12_init_fuzz_messy_whitespace` |

### C. `multiply_matrices` (driven on matrices built via `initialize_matrix_from_string`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `init` + `multiply_matrices` | `1x1` × `1x1` | [x] `cfg_c1_multiply_1x1` |
| C2 | `init` + `multiply_matrices` | inner dim `0`: `(w=0,h=2)` × `(w=3,h=0)` → 2x3 of zeros | [x] `cfg_c2_multiply_inner_dim_zero` |
| C3 | `init` + `multiply_matrices` | result height `0`: `(w=2,h=0)` × `(w=2,h=2)` | [x] `cfg_c3_multiply_result_height_zero` |
| C4 | `init` + `multiply_matrices` | result width `0`: `(w=2,h=2)` × `(w=0,h=2)` | [x] `cfg_c4_multiply_result_width_zero` |
| C5 | `init` + `multiply_matrices` | square `3x3` × `3x3` | [x] `cfg_c5_multiply_square` |
| C6 | `init` + `multiply_matrices` | non-square chain `(w=4,h=2)` × `(w=3,h=4)` → `2x3` | [x] `cfg_c6_multiply_non_square_chain` |
| C7 | `init` + `multiply_matrices` | row-vector × col-vector (`1xN` × `Nx1`) and col × row (`Nx1` × `1xN`) | [x] `cfg_c7_multiply_vectors` |
| C8 | `init` + `multiply_matrices` | values chosen to force `int` accumulation overflow (wrapping) | [x] `cfg_c8_multiply_int_overflow_wraps` |
| C9 | `init` + `multiply_matrices` | *(fuzzed)* random `h_a, inner, w_b` in `0..=8`, random values incl. large magnitudes → wrapping | [x] `cfg_c9_multiply_fuzz` |

### D. `matrix_to_string` + `write_to_file` + composed `driver`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| D1 | `init` + `matrix_to_string` | `1x1`; single-column (no separator branch) | [x] `cfg_d1_to_string_single_column` |
| D2 | `init` + `matrix_to_string` | multi-column (exercises `j < width-1` separator branch) | [x] `cfg_d2_to_string_separator_branch` |
| D3 | `init` + `matrix_to_string` | `width=0, height=3` → three bare `\n` | [x] `cfg_d3_to_string_zero_width` |
| D4 | `init` + `matrix_to_string` | `height=0` → empty string | [x] `cfg_d4_to_string_zero_height` |
| D5 | `init` + `matrix_to_string` | mixed-sign, 1..10-digit values (safe-length bound, see ERRORS.md UB note) | [x] `cfg_d5_to_string_digit_widths` |
| D6 | `init` + `matrix_to_string` | *(fuzzed)* random dims `0..=8`, random values `-999_999_999..=999_999_999` | [x] `cfg_d6_to_string_fuzz` |
| D7 | `write_to_file` | new file, short content | [x] `cfg_d7_write_new_file` |
| D8 | `write_to_file` | existing file with longer prior content (truncation by mode `"w"`) | [x] `cfg_d8_write_truncates_existing` |
| D9 | `write_to_file` | empty content `""` | [x] `cfg_d9_write_empty_content` |
| D10 | `write_to_file` | content > stdio buffer (256 KiB, forces flush inside `fprintf`) | [x] `cfg_d10_write_large_content` |
| D11 | `write_to_file` | content containing `%s`/`%n`/`%%` (must pass through `"%s"` unexpanded) | [x] `cfg_d11_write_format_specifiers_not_expanded` |
| D12 | `write_to_file` | content with embedded newlines and UTF-8 bytes | [x] `cfg_d12_write_newlines_and_utf8` |
| D13 | `write_to_file` | *(fuzzed)* random lengths `0..=4096`, random non-NUL bytes | [x] `cfg_d13_write_fuzz` |
| D14 | `driver` (end-to-end) | `2x2` × `2x2`, compares return code + full `matrix.txt` bytes | [x] `cfg_d14_driver_square` |
| D15 | `driver` (end-to-end) | non-square `(2x3)·(3x2)` and `(3x1)·(1x3)` | [x] `cfg_d15_driver_non_square` |
| D16 | `driver` (end-to-end) | zero-dimension shapes (`height=0`, `width=0`) | [x] `cfg_d16_driver_zero_dims` |
| D17 | `driver` (end-to-end) | extra whitespace / extra rows in the input strings | [x] `cfg_d17_driver_messy_input` |
| D18 | `driver` (end-to-end) | *(fuzzed)* random conformable dims `0..=6` and random values; `matrix.txt` compared byte-for-byte | [x] `cfg_d18_driver_fuzz` |

### E. Cross-cutting

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| E1 | `allocate_matrix` + `free_matrix` | allocate/free round trip at every shape class (incl. zero dims), repeated, no leak/crash | [x] `cfg_e1_allocate_free_round_trips` |
| E2 | full pipeline, hand-built `matrix_t` | `multiply_matrices` and `matrix_to_string` fed a `matrix_t` built by the *test* (not by `init`) — proves the struct layout (`*mut*mut int, int, int`) is identical | [x] `cfg_e2_hand_built_matrix_t_layout` |
| E3 | `initialize_matrix_from_string` | pointer returned by C freed with C's `free_matrix`, pointer returned by Rust freed with Rust's — and the returned `char*` from `matrix_to_string` freed with libc `free` (Rust must allocate with libc `malloc`) | [x] `cfg_e3_ownership_and_allocator_interop` |

## Feature combinations

`Cargo.toml` has no `[features]`, so the cross-product of feature combos is the
single default configuration; `check_feature_combos.sh` enumerates and runs it
(`--no-default-features`, default, `--all-features`) so the gate is mechanically
checked rather than assumed.

## Phase B result

All 49 rows pass, in both build profiles and all (three) feature invocations:

```
cargo test --test phase_b_valid -- --test-threads=1
test result: ok. 49 passed; 0 failed
```

Beyond the per-row tests, `tests/phase_d_stress.rs` re-runs the same axes as
larger randomized sweeps (shapes up to 24x24, 400-case composed-pipeline sweep,
500-case token soup, 150-case `driver` sweep, 500-iteration allocator churn) —
8 further tests, all passing.

Randomized rows use `SEED = 0x5EED_1234_ABCD_0001` (splitmix64 in
`tests/common/mod.rs`), so every run is reproducible.

### What each row compares

Not just the return value: for every call the harness compares the `matrix_t`
struct read back through the returned pointer (`width`, `height`, row-pointer
nullness, and every element), the full `char*` payload of `matrix_to_string`,
`write_to_file`'s `errno`-style return, the bytes actually written to disk, and
the process's `stderr` captured by redirecting fd 2 around each call. Pointers
returned by the Rust `.so` are released with libc `free`/`free_matrix` exactly as
a C consumer would, so an allocator mismatch would abort the test process.

### Guarded rows

`cfg_d18`, and the four fuzz sweeps in `phase_d_stress.rs`, ask the **C** library
first whether the values it will stringify stay within its own `11*width`
per-row allocation, and skip the case otherwise. This is required because
`multiply_matrices` can wrap an accumulation into `[-2147483648, -1000000000]`,
which renders as 11 characters and makes the C `strcat` past the end of its own
buffer (see the UB note in `ERRORS.md`). Skips are counted and asserted to be a
minority, so the guard cannot silently empty a sweep:

```
cfg_d18                        : 120 cases,  some skipped
stress_large_shapes_pipeline   :  60 cases, 14 skipped
stress_pipeline_fuzz_all_shapes: 400 cases, 63 skipped
stress_driver_fuzz             : 150 cases, 20 skipped
stress_init_token_soup         : 500 cases, 14 skipped
```

### No binary to compare

Neither project builds an executable (`crate-type = ["cdylib"]`;
`add_library(driver SHARED ...)`; no `main()` in `c_src/src/`), so the
"compare C and Rust stdout" gate does not apply. Rows D14–D18 are the equivalent
end-to-end check: they `chdir` into a private temp directory, call the exported
`driver()` in each `.so`, and compare the return code, the captured `stderr`, and
the bytes of the `matrix.txt` it writes.
