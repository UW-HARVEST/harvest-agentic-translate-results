# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/include/*.h` (the full public API) plus every `if`/loop
bound in `c_src/src/*.c`. There are **no `#ifdef`s, no global options, no
modes/flags** in this library — it is stateless. The configuration surface is
therefore made of *input shape* axes only:

| axis | values the C distinguishes | where |
|---|---|---|
| A. entry point | `allocate_matrix`, `free_matrix`, `initialize_matrix_from_string`, `multiply_matrices`, `matrix_to_string`, `write_to_file`, `driver` | all of `matrix.h`, `write.h`, `driver.c`. `driver` is the one-shot wrapper; the other six are the low-level API and are driven **directly**. |
| B. `height` | `0`, `1`, `>1`; `< 0` (→ error) | loop bounds `i < height`; `malloc(height*8)` |
| C. `width` | `0`, `1`, `>1`; `< 0` (→ error) | loop bounds `j < width`; `malloc(width*4)`; `j < width - 1` separator test |
| D. shape relation for `multiply_matrices` | `a.width == b.height` (ok) vs `!=` (error); square vs non-square; inner dim `0` (result all-zero); `1×n · n×1` (→ 1×1); `n×1 · 1×n` (→ n×n outer product) | `matrix.c:119`, triple loop |
| E. token layout of the input string | exactly `height` lines; **more** lines than `height` (extra ignored); exactly `width` cols; **more** cols than `width` (extra ignored); consecutive delimiters (`strtok_r` collapses runs of `\n` and of `" "`); leading/trailing delimiters; no trailing newline; trailing newline; `\n` only vs `" "` only | `strtok_r` semantics, `matrix.c:89-111` |
| F. cell value magnitude / `atoi` parsing | `0`; positive; negative; `INT_MAX`; `INT_MIN`; leading `+`; leading whitespace; overflowing literals (`"2147483648"`, `"-2147483649"` → `atoi` UB, glibc saturates); trailing garbage (`"12abc"` → `12`); non-numeric (`"abc"` → `0`); `"-"`; `"0x10"` → `0` | `atoi(col_token)` |
| G. product arithmetic | products/accumulations that stay in range vs those that overflow `int` (C UB; both builds must wrap identically) | `matrix.c:129` |
| H. `matrix_to_string` render width | `width == 1` (no separators emitted at all); `width > 1` (`width-1` spaces); `height == 0` (returns `""`); `width == 0` (row is just `"\n"`); digit counts 1..11 (11 chars ⇒ the C sizing formula under-allocates) | `matrix.c:143-163` |
| I. `write_to_file` filename target | fresh file; **existing** file (truncated by mode `"w"`); nested-but-existing dir; path with spaces/UTF-8 | `fopen(filename,"w")` |
| J. `write_to_file` content shape | `""` (empty ⇒ zero-byte file); single line; embedded `\n`; embedded `%` (must be literal — C uses `"%s"`); high bytes ≥ 0x80; long (≫ BUFSIZ) | `fprintf(file,"%s",content)` |
| K. cross-library ABI interop | `matrix_t` built by C consumed by Rust and vice versa (same `repr(C)` layout, same `malloc` heap) | `matrix_t` in `matrix.h` |

## Rows (pruned cross-product — one row per combination the C treats differently)

Every row is exercised with **many randomized inputs** (xorshift64\*, fixed
seed `0x2545F4914F6CDD1D`) unless it is inherently a single fixed shape, and
both `.so`s are called through `libloading` with byte-for-byte comparison of
the return value, the `matrix_t` contents, the rendered string, and the
captured `stderr`.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `allocate_matrix` + `free_matrix` | `width,height` = (1,1) | `cfg_01_alloc_free_1x1` | [x] |
| 2 | `allocate_matrix` + `free_matrix` | `height = 0`, `width` random 0..64 (`malloc(0)` row array, no rows) | `cfg_02_alloc_zero_height` | [x] |
| 3 | `allocate_matrix` + `free_matrix` | `width = 0`, `height` random 1..64 (`malloc(0)` per row) | `cfg_03_alloc_zero_width` | [x] |
| 4 | `allocate_matrix` + `free_matrix` | randomized `width,height` ∈ 1..64 (many) — struct fields + non-NULL parity | `cfg_04_alloc_random` | [x] |
| 5 | `allocate_matrix` | large-but-plausible `width,height` (1..4096 × 1..4096, capped so it fits) | `cfg_05_alloc_large` | [x] |
| 6 | `initialize_matrix_from_string` + `matrix_to_string` | exact-fit input: `height` lines × `width` cols, single spaces, no trailing `\n`; randomized dims 1..12 and values in ±10⁴ | `cfg_06_init_exact_fit` | [x] |
| 7 | `initialize_matrix_from_string` | trailing newline present | `cfg_07_init_trailing_newline` | [x] |
| 8 | `initialize_matrix_from_string` | **more rows** than `height` and **more cols** than `width` (surplus ignored) | `cfg_08_init_surplus_tokens` | [x] |
| 9 | `initialize_matrix_from_string` | runs of consecutive delimiters: `"\n\n"`, multiple spaces, leading/trailing spaces (`strtok_r` collapses) | `cfg_09_init_collapsed_delimiters` | [x] |
| 10 | `initialize_matrix_from_string` | `width = 0` (inner loop never runs; each line consumed as a row token) | `cfg_10_init_zero_width` | [x] |
| 11 | `initialize_matrix_from_string` | `height = 0` (outer loop never runs; returns matrix, input untouched) | `cfg_11_init_zero_height` | [x] |
| 12 | `initialize_matrix_from_string` | `atoi` edge tokens: `"0"`, `"+5"`, `"-0"`, `"007"`, `"2147483647"`, `"-2147483648"`, `"2147483648"`, `"-2147483649"`, `"99999999999999999999"`, `"12abc"`, `"abc"`, `"-"`, `"0x10"`, `" 7"`, `"1e3"` | `cfg_12_init_atoi_edges` | [x] |
| 13 | `initialize_matrix_from_string` | randomized *token text* fuzz: random mixes of digits, signs, letters and spaces | `cfg_13_init_token_fuzz` | [x] |
| 14 | `matrix_to_string` | `width == 1` (no separator branch ever taken), values incl. `INT_MIN`/`INT_MAX` (11-char, still exactly fits the C formula for `width==1`) | `cfg_14_to_string_width1_extremes` | [x] |
| 15 | `matrix_to_string` | `width == 0` (rows render as bare `"\n"`) | `cfg_15_to_string_zero_width` | [x] |
| 16 | `matrix_to_string` | `height == 0` (returns `""`) | `cfg_16_to_string_zero_height` | [x] |
| 17 | `matrix_to_string` | `width > 1`, randomized dims/values kept ≤ 10 chars so the C sizing formula does not under-allocate | `cfg_17_to_string_random` | [x] |
| 18 | `multiply_matrices` | square × square, randomized `n` ∈ 1..12, values ±100 | `cfg_18_mul_square` | [x] |
| 19 | `multiply_matrices` | non-square compatible: `(h_a×w_a) · (w_a×w_b)`, randomized | `cfg_19_mul_nonsquare` | [x] |
| 20 | `multiply_matrices` | inner dimension `0` (`a` is `h×0`, `b` is `0×w`) → every cell `0`, `k`-loop never runs | `cfg_20_mul_zero_inner` | [x] |
| 21 | `multiply_matrices` | `1×n · n×1` → 1×1 dot product | `cfg_21_mul_dot` | [x] |
| 22 | `multiply_matrices` | `n×1 · 1×m` → n×m outer product | `cfg_22_mul_outer` | [x] |
| 23 | `multiply_matrices` | `height_a = 0` or `width_b = 0` (result has an empty dimension) | `cfg_23_mul_empty_result` | [x] |
| 24 | `multiply_matrices` | **overflowing** products/accumulations (values near `INT_MAX`/`INT_MIN`) — signed wrap must match bit-for-bit | `cfg_24_mul_overflow` | [x] |
| 25 | `multiply_matrices` | full pipeline `init → multiply → to_string`, randomized, values bounded so rendering is safe | `cfg_25_pipeline_random` | [x] |
| 26 | `write_to_file` | fresh file, single-line content | `cfg_26_write_fresh` | [x] |
| 27 | `write_to_file` | **existing, longer** file (mode `"w"` truncates) | `cfg_27_write_truncates` | [x] |
| 28 | `write_to_file` | empty content `""` → zero-byte file, returns `0` | `cfg_28_write_empty` | [x] |
| 29 | `write_to_file` | content with embedded `%d`/`%s`/`%%` — must be literal (C uses `"%s"`, not `content` as a format) | `cfg_29_write_percent_literal` | [x] |
| 30 | `write_to_file` | content with high bytes (0x80..0xFF), embedded newlines; randomized length 0..8192 (crosses `BUFSIZ`) | `cfg_30_write_random_bytes` | [x] |
| 31 | `write_to_file` | filename containing spaces / UTF-8 / dots | `cfg_31_write_odd_filenames` | [x] |
| 32 | `driver` | success path, randomized compatible dims/values; compares return code **and the byte content of `matrix.txt`** | `cfg_32_driver_success_random` | [x] |
| 33 | `driver` | success with `width_a = 0` (inner dim 0) and with `height_a = 0` | `cfg_33_driver_degenerate_dims` | [x] |
| 34 | `driver` | success with `1×1 · 1×1`, incl. overflowing values | `cfg_34_driver_1x1_overflow` | [x] |
| 35 | cross-library ABI | matrix built by **C** `initialize_matrix_from_string` → rendered by **Rust** `matrix_to_string`, and vice versa; also C-built × Rust-built into each `multiply_matrices` | `cfg_35_cross_library_interop` | [x] |
| 36 | cross-library ABI | matrix allocated by **C** `allocate_matrix` → freed by **Rust** `free_matrix` and vice versa (same `malloc` arena) | `cfg_36_cross_library_alloc_free` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(driver SHARED …)` — there is
no `add_executable`, and `driver.c` has no `main`. The Rust `Cargo.toml`
likewise declares only `crate-type = ["cdylib"]` with no `[[bin]]`. **No driver
binary exists, so the "compare stdout of the two binaries" gate is not
applicable**; the `driver()` entry point is instead compared through the FFI in
rows 32–34, including the bytes it writes to `matrix.txt`.

## Result

`cargo test --release` → **36 passed, 0 failed** in `tests/phase_b_configs.rs`;
every row above is checked off.

Notes on how the rows are driven:

* All six low-level entry points (`allocate_matrix`, `free_matrix`,
  `initialize_matrix_from_string`, `multiply_matrices`, `matrix_to_string`,
  `write_to_file`) are called **directly** through their exported C symbols, not
  only via the `driver` one-shot wrapper; `driver` itself is additionally
  compared end-to-end in rows 32–34, including the bytes it leaves in
  `matrix.txt`.
* Rows marked "randomized" use a xorshift64\* generator with the fixed seed
  `0x2545F4914F6CDD1D` (per-row salted), 40–300 cases per row, and compare the
  full observable state: the `matrix_t` dimensions, every cell, the rendered
  `char*`, the return code, the written file bytes **and** the captured
  `stderr` bytes.
* `allocate_matrix` rows compare only NULL-ness and the `width`/`height` fields:
  the cell storage it hands back is *uninitialised* `malloc` memory, so its
  contents legitimately differ between the two calls.
* Value ranges in the rendering rows are bounded so that no cell needs more than
  10 characters. That is deliberate: the C sizing formula
  `height * (width*10 + width) + height + 1` reserves only **one** spare byte per
  row for the `width - 1` separators plus the newline, so an 11-character
  rendering (e.g. `-2147483648`) with `width > 1` overruns the buffer and
  corrupts the heap. That bug is faithfully reproduced (same `malloc`, same
  `snprintf`/`strcat`, no intermediate Rust allocations), but it is not a stable
  thing to assert on, so row 14/24 exercise the 11-character values with
  `width == 1`, where the formula happens to fit exactly.
* Rows 35–36 additionally check ABI compatibility of `matrix_t` *across* the two
  libraries: a matrix built by C is rendered/multiplied/freed by Rust and vice
  versa. This works because both `.so`s share the process's single `malloc`
  arena and the Rust `#[repr(C)] matrix_t` is layout-identical.

## Verified configurations of the build itself

| build | how | result |
|---|---|---|
| `target/release/libdriver.so` (`panic = "abort"`) | `cargo build --release` | 66/66 tests pass |
| `target/debug/libdriver.so` | `cargo build`, then `DIFFTEST_RUST_SO=…/target/debug/libdriver.so cargo test --release` | 66/66 tests pass |
| default features | `./check_features.sh` | pass |
| `--no-default-features` | `./check_features.sh` | pass |
