# CONFIGS.md — Configuration-surface table (Phase A) / Phase B checklist

## Build-time configurations

| axis | values |
|------|--------|
| Cargo features | **none declared** in `Cargo.toml` ⇒ the single combination `--no-default-features` (≡ default build) |
| C build options | `CMakeLists.txt` has a single `add_library(driver SHARED …)`, no options/definitions; the only `#define` in the C sources is `OUT_FILE "matrix.txt"` (`driver.c`), no `#if`/`#ifdef` at all ⇒ one configuration |
| Cargo profiles | `release` (`panic = "abort"`) **and** `dev`; the suite is run against both `.so`s. `[profile.dev] debug-assertions = false` / `overflow-checks = false` is **required** for behavioural parity — see the "Divergence found and fixed" section of `ERRORS.md` |

## Runtime configuration axes (derived from the C branches)

There are no option/flag setters in this API; the "configuration" is entirely
(a) which entry point is called, (b) the `int width`/`int height` shape, and
(c) the *shape of the input string* that `strtok_r`/`atoi` parse.

* **Entry points (all 7 exported symbols, low-level ones included):**
  `allocate_matrix`, `free_matrix`, `initialize_matrix_from_string`,
  `multiply_matrices`, `matrix_to_string`, `write_to_file`, and the one-shot
  wrapper `driver`.
* **Dimension shapes** the code distinguishes: `0x0`, `w=0,h>0`, `w>0,h=0`,
  `1x1`, `1xN` (`width=1` ⇒ no space separators, `j < width-1` never true),
  `Nx1`, square `NxN`, rectangular `w≠h`, large (`w*h` in the thousands).
* **Value shapes**: all zeros, positives, negatives (extra `-` char), mixed,
  1-digit vs 10-digit magnitudes (`snprintf` into the 12-byte buffer),
  `INT_MAX`, `INT_MIN` (11 chars — only safe in the C buffer when `width == 1`),
  products that overflow `int` (wrap-around accumulation).
* **String shapes** for `initialize_matrix_from_string` (`strtok_r` semantics):
  exact tokens; extra trailing rows; extra trailing columns; runs of consecutive
  spaces; leading/trailing spaces; leading/embedded/trailing newlines (empty
  lines are *skipped* by `strtok_r`); `\t`-containing tokens (`atoi` skips
  leading whitespace); `\r\n` line endings; non-numeric tokens (`atoi` ⇒ 0);
  partially numeric tokens (`"12abc"` ⇒ 12); explicit `+`/`-` signs;
  out-of-`int`-range digit strings (glibc `atoi` = `(int)strtol` clamp+truncate).
* **`write_to_file` shapes**: empty content, single line, many lines, 1 MiB
  content, content containing `%s`/`%d`/`%%` (goes through `fprintf(file,
  "%s", content)` so it must be copied verbatim), overwriting an existing longer
  file (truncation), `/dev/null` target, nested existing directory.

`[x]` = differential test (C `.so` vs Rust `.so`, both loaded with `libloading`)
passes over many seeded-random inputs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `allocate_matrix` + `free_matrix` | `0x0` — `malloc(0)` for the row array, no row allocations; struct fields must read back `w=0,h=0`, pointer non-NULL | [x] `c1_allocate_0x0` |
| C2 | `allocate_matrix` + `free_matrix` | `width=0, height>0` — `height` zero-sized row allocations, all non-NULL | [x] `c2_allocate_zero_width` |
| C3 | `allocate_matrix` + `free_matrix` | `width>0, height=0` — row array `malloc(0)`, loop body never runs | [x] `c3_allocate_zero_height` |
| C4 | `allocate_matrix` + `free_matrix` | `1x1`, `1xN`, `Nx1`, square, rectangular (randomized 1..64) — non-NULL struct, `width`/`height` stored verbatim, all `height` row pointers distinct & writable | [x] `c4_allocate_random_shapes` |
| C5 | `allocate_matrix` + `free_matrix` | large shape (`width*height` ≈ 4096) | [x] `c5_allocate_large_shape` |
| C6 | `allocate_matrix` | boundary `width`/`height` values `{0, 1, INT_MAX}` × `{0, 1}` (success/failure parity only) | [x] `c6_allocate_boundary_dims` |
| C7 | `initialize_matrix_from_string` | exact-fit input, `1x1`, randomized values in `[-999_999_999, 999_999_999]` | [x] `c7_init_1x1_random` |
| C8 | `initialize_matrix_from_string` | exact-fit input, randomized `NxM` (1..12 × 1..12), randomized values; every parsed cell compared | [x] `c8_init_random_shapes` |
| C9 | `initialize_matrix_from_string` | `width=1` (single column per row, `\n`-only splitting) | [x] `c9_init_single_column` |
| C10 | `initialize_matrix_from_string` | `height=1` (single row, space splitting only) | [x] `c10_init_single_row` |
| C11 | `initialize_matrix_from_string` | `height=0` (any `width`, any string incl. `""`) ⇒ empty-but-valid matrix, input never parsed | [x] `c11_init_zero_height` |
| C12 | `initialize_matrix_from_string` | `width=0`, `height>0` ⇒ rows *are* tokenized (need ≥ `height` rows) but no cells stored | [x] `c12_init_zero_width` |
| C13 | `initialize_matrix_from_string` | input has **more** rows than `height` and/or **more** columns than `width` (extra tokens ignored) | [x] `c13_init_extra_rows_and_columns` |
| C14 | `initialize_matrix_from_string` | runs of consecutive spaces / leading spaces / trailing spaces in rows (`strtok_r` collapses delimiters) | [x] `c14_init_whitespace_runs` |
| C15 | `initialize_matrix_from_string` | leading `\n`, embedded blank lines, trailing `\n`(s) (empty lines skipped by `strtok_r`) | [x] `c15_init_blank_lines` |
| C16 | `initialize_matrix_from_string` | `\r\n` line endings (`\r` stays inside the last token ⇒ `atoi` behaviour) and `\t`-prefixed tokens | [x] `c16_init_crlf_and_tabs` |
| C17 | `initialize_matrix_from_string` | non-numeric (`"abc"`, `"-"`, `"+"`, `"."`) and partially numeric (`"12abc"`, `"3.7"`, `"0x1f"`) tokens ⇒ `atoi` semantics | [x] `c17_init_non_numeric_tokens` |
| C18 | `initialize_matrix_from_string` | out-of-range digit strings: `"2147483647"`, `"2147483648"`, `"-2147483648"`, `"-2147483649"`, `"99999999999999999999"`, `"000123"`, `"  +42"` ⇒ glibc `atoi` clamp/truncate | [x] `c18_init_atoi_range_tokens` |
| C19 | `multiply_matrices` | `1x1 × 1x1`, randomized values | [x] `c19_multiply_1x1` |
| C20 | `multiply_matrices` | general `m×k · k×n`, randomized `m,k,n ∈ 1..10`, randomized values in `[-1000,1000]` (no overflow) | [x] `c20_multiply_random_shapes` |
| C21 | `multiply_matrices` | `k = 0` (A `0`-wide, B `0`-high) ⇒ every result cell must be exactly `0` | [x] `c21_multiply_inner_dim_zero` |
| C22 | `multiply_matrices` | `m = 0` (A has no rows) and `n = 0` (B has no columns) ⇒ valid empty results | [x] `c22_multiply_empty_outer_dims` |
| C23 | `multiply_matrices` | values chosen so the accumulation **wraps** `int` (`INT_MAX`-scale products) ⇒ identical wrap-around | [x] `c23_multiply_wrapping_products` |
| C24 | `matrix_to_string` | `h=0` ⇒ `""`; `w=0, h>0` ⇒ `h` bare `\n`s; `1x1`; `1xN`; `Nx1` | [x] `c24_to_string_degenerate_shapes` |
| C25 | `matrix_to_string` | randomized `NxM` with mixed-width values (1–10 chars incl. negatives) ⇒ exact separator/newline layout | [x] `c25_to_string_mixed_widths` |
| C26 | `matrix_to_string` | `1x1` holding `INT_MAX` / `INT_MIN` / `-1000000000` (11-char output, exactly fills the C buffer) | [x] `c26_to_string_11_char_values` |
| C27 | `write_to_file` | normal single-line content into a fresh file in a temp dir ⇒ return 0 + identical file bytes | [x] `c27_write_simple` |
| C28 | `write_to_file` | empty content `""` ⇒ return 0, zero-length file | [x] `c28_write_empty_content` |
| C29 | `write_to_file` | multi-line content, content containing `%s`, `%d`, `%%`, `\t`, high-bit bytes ⇒ copied verbatim | [x] `c29_write_special_bytes` |
| C30 | `write_to_file` | 1 MiB content (multiple `stdio` buffer flushes) | [x] `c30_write_one_mib` |
| C31 | `write_to_file` | overwriting an existing **longer** file (`"w"` truncation) | [x] `c31_write_truncates_existing` |
| C32 | `write_to_file` | target `/dev/null` (character device) ⇒ return 0 | [x] `c32_write_dev_null` |
| C33 | `write_to_file` | randomized filenames/contents in a nested existing directory | [x] `c33_write_nested_random_paths` |
| C34 | full pipeline (`initialize` → `multiply` → `matrix_to_string` → `write_to_file`) driven **by hand** from the low-level exports | randomized `m,k,n ∈ 1..8`, randomized values, compare the produced string *and* the written file bytes | [x] `c34_manual_pipeline` |
| C35 | `driver` | happy path `1x1 · 1x1`; return code + `matrix.txt` bytes | [x] `c35_driver_1x1` |
| C36 | `driver` | happy path randomized `m,k,n ∈ 1..8` with randomized values (incl. negatives) | [x] `c36_driver_random_shapes` |
| C37 | `driver` | degenerate valid shapes: `k=0` (`width_a=0,height_b=0`), `m=0`, `n=0` | [x] `c37_driver_degenerate_shapes` |
| C38 | `driver` | inputs with extra rows/columns, extra whitespace, trailing newlines, `atoi`-quirky tokens | [x] `c38_driver_quirky_inputs` |
| C39 | `driver` | large-ish shape (`8x8 · 8x8`) and values that make the products wrap `int` (result kept 1 column wide, see note below) | [x] `c39_driver_large_and_wrapping` |
| C40 | `initialize_matrix_from_string` + `matrix_to_string` | large shapes: `40x40`, `100x7`, `7x100`, `1x500`, `500x1` | [x] `c8b_init_large_shapes` |
| C41 | `multiply_matrices` | large shapes: `32x32·32x32`, `1x64·64x1`, `64x1·1x64`, `17x5·5x23` | [x] `c20b_multiply_large_shapes` |
| C42 | `initialize_matrix_from_string` | tokens far longer than any `int` (5000 digits, 100 leading zeros, `+111…`) | [x] `c18b_init_huge_tokens` |
| C43 | `write_to_file` | target is a symlink (existing and dangling) | [x] `c31b_write_through_symlink` |
| C44 | full pipeline / `allocate_matrix` / `driver` | same configurations but with the **Rust** library called FIRST (guards against divergences masked by C-first global state such as `errno`) | [x] `c34b_pipeline_rust_first`, `c4b_allocate_rust_first`, `c36b_driver_rust_first` |
| C45 | `driver` | two successive invocations in the same CWD (the second must truncate `matrix.txt`) | [x] `c35b_driver_repeated_calls_truncate` |
| C46 | all entry points | randomised catch-all fuzzing: random dimensions in `[-3, 40]`, random well-formed/mutated/pure-soup input text, random file targets — 4000 + 800 + 600 + 600 iterations | [x] `fuzz_matrix_pipeline`, `fuzz_allocate_free`, `fuzz_write_to_file`, `fuzz_driver` |

### Notes on the two `matrix_to_string` limits the tests respect

1. **C buffer budget.** `matrix_to_string` sizes its buffer as
   `height * (width * 10 + width) + height + 1`, i.e. it only holds an average
   of ≤ 10 characters per cell. Values needing 11 characters
   (`INT_MIN … -1000000000`) overrun that buffer in the **C original** (UB).
   Tests therefore keep the rendered length within the C budget (helper
   `to_string_fits` in `tests/fuzz.rs`, `|value| ≤ 999 999 999` elsewhere) and
   only use 11-character values where the budget still covers them
   (`width == 1`, row C26/C39).
2. **`height == INT_MAX`** is never passed to `allocate_matrix`: the C code would
   attempt 2^31 successive row allocations (machine OOM), which is a resource
   limit, not a behavioural difference. The huge-`size_t` malloc path is covered
   by negative heights (E2) and by `width = INT_MAX` (C6/E4).
