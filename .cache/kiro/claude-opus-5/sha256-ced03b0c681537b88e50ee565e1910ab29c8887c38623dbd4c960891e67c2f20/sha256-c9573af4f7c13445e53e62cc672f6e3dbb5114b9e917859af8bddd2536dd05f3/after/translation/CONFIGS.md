# CONFIGS.md — configuration-surface table (Phase A → gate for Phase B)

## Axes derived from the C source

**Runtime options / modes / flags: NONE.** The public API (`include/matrix.h`,
`include/write.h`) exposes no option struct, no mode enum, no flag argument, and
the sources contain no `#ifdef` other than nothing at all — `grep -n '#if\|#ifdef'
c_src/src/*.c c_src/include/*.h` is empty. The only `#define` is
`OUT_FILE "matrix.txt"` in `driver.c`. Cargo declares no `[features]`, so there
is exactly **one** feature combination (default = no features) to verify.

Therefore the configuration surface is entirely **input shape** × **entry point**.

Axes the C actually branches on:

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| entry point | `allocate_matrix`, `free_matrix`, `initialize_matrix_from_string`, `multiply_matrices`, `matrix_to_string`, `write_to_file`, `driver` | all 7 exports, low-level first |
| `height` | `0` (loops never run), `1`, `>1` | `for (i=0;i<height;i++)` in 4 functions |
| `width` | `0` (loops never run), `1` (suppresses the `j < width-1` space branch), `>1` | `matrix_to_string:158`, parse loop |
| aspect | square, row-vector, column-vector, general rectangle | `multiply_matrices` triple loop |
| inner dim (`mat_a->width == mat_b->height`) | `0` (accumulator stays 0), `1`, `>1` | `multiply_matrices:128` |
| element value | `0`, small +, small −, multi-digit, `INT_MAX`, `INT_MIN` (width 1 only), products that overflow `int` | `snprintf("%d")` width; `+=` accumulate |
| token text form | plain digits, leading `+`/`-`, non-numeric (`atoi`→0), digit-then-alpha (`12abc`→12), `0x10` (→0), out-of-range digits (`atoi` clamp) | `atoi(col_token)` |
| row/col separators | single space, runs of spaces (strtok_r collapses), leading space, trailing space, trailing `\n`, no trailing `\n`, blank lines | `strtok_r(…," ")`, `strtok_r(…,"\n")` |
| surplus input | more rows than `height`, more columns than `width` (both silently ignored) | loop bounds |
| `write_to_file` content | `""`, one line, multi-line, `%`-bearing, `> BUFSIZ` (forces a real flush inside `fprintf`) | `fprintf(file,"%s",content)` |
| `write_to_file` target | new file, pre-existing file (mode `"w"` truncates) | `fopen(filename,"w")` |

## Value-range safety note (why element values are bounded in random rows)

`matrix_to_string` allocates `height*(width*10+width)+height+1` bytes but can
emit up to `12*width` bytes per row when values need 11 characters
(`-1000000000 … INT_MIN`). For `width >= 2` that overruns the buffer — a real
heap overflow **in the C**, i.e. UB with no defined output to compare against.
Randomized rows therefore draw values whose decimal form is `<= 10` chars
(`|v| <= 999_999_999`), which provably fits; `INT_MIN`/`INT_MAX` are exercised
explicitly at `width == 1`, where the formula fits exactly.

## Rows — each verified by loading BOTH `.so`s and comparing byte-for-byte

Randomized rows use a fixed-seed xorshift PRNG (seed per row) and many
iterations, not one hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | ✅ |
|----|----------------|-------------------------------------------|----|
| 1  | `allocate_matrix` + `free_matrix` | `width=0, height=0` — both loops skipped, `malloc(0)` | [x] |
| 2  | `allocate_matrix` + `free_matrix` | `width=0, height=n>0`; and `width=n>0, height=0` | [x] |
| 3  | `allocate_matrix` + `free_matrix` | `width=1, height=1` | [x] |
| 4  | `allocate_matrix` + `free_matrix` | randomized `width,height ∈ [0,32]`, 256 iters: compare returned struct fields + row-pointer non-nullness, then free through each `.so` | [x] |
| 5  | `initialize_matrix_from_string` | `1×1`, digits only | [x] |
| 6  | `initialize_matrix_from_string` | column vector `width=1, height=n`, mixed signs | [x] |
| 7  | `initialize_matrix_from_string` | row vector `height=1, width=n`, mixed signs | [x] |
| 8  | `initialize_matrix_from_string` | square `n×n`, randomized values, 200 iters | [x] |
| 9  | `initialize_matrix_from_string` | general rectangle `width != height`, randomized, 200 iters | [x] |
| 10 | `initialize_matrix_from_string` | `width=0, height=n>0` — inner loop skipped, `strtok_r(row," ")` still called | [x] |
| 11 | `initialize_matrix_from_string` | `height=0` — nothing parsed, arbitrary input string | [x] |
| 12 | `initialize_matrix_from_string` | surplus rows: input has `height+k` rows, only first `height` consumed | [x] |
| 13 | `initialize_matrix_from_string` | surplus columns: rows have `width+k` tokens, only first `width` consumed | [x] |
| 14 | `initialize_matrix_from_string` | separator runs: multiple/leading/trailing spaces, tabs are NOT separators | [x] |
| 15 | `initialize_matrix_from_string` | line endings: trailing `\n`, no trailing `\n`, blank lines between rows | [x] |
| 16 | `initialize_matrix_from_string` | non-numeric / partial tokens: `abc`, `12abc`, `0x10`, `+5`, `-0`, `007`, `--3`, `.5` | [x] |
| 17 | `initialize_matrix_from_string` | `atoi` extremes/clamping: `2147483647`, `-2147483648`, `99999999999`, `-99999999999` | [x] |
| 18 | `initialize_matrix_from_string` | randomized *text form* fuzz: random separator runs × random token forms, 300 iters | [x] |
| 19 | `multiply_matrices` | `1×1 · 1×1` | [x] |
| 20 | `multiply_matrices` | inner product: `(1×n) · (n×1)`, randomized, 200 iters | [x] |
| 21 | `multiply_matrices` | outer product: `(n×1) · (1×m)`, randomized, 200 iters | [x] |
| 22 | `multiply_matrices` | square `n×n · n×n`, randomized, 200 iters | [x] |
| 23 | `multiply_matrices` | general `(h_a×w_a) · (w_a×w_b)`, all three dims distinct, randomized, 200 iters | [x] |
| 24 | `multiply_matrices` | inner dim `0`: `mat_a->width = 0 = mat_b->height` → every cell stays `0` | [x] |
| 25 | `multiply_matrices` | `mat_a->height = 0` → result has 0 rows | [x] |
| 26 | `multiply_matrices` | `mat_b->width = 0` → result has 0 columns per row | [x] |
| 27 | `multiply_matrices` | accumulation that overflows `int` (large operands, long inner dim) — wrap behaviour | [x] |
| 28 | `matrix_to_string` | `width=1` (space branch never taken), `height ∈ {1,2,n}` | [x] |
| 29 | `matrix_to_string` | `width>=2` (space branch taken), `height ∈ {1,n}` | [x] |
| 30 | `matrix_to_string` | `height=0` → `""` | [x] |
| 31 | `matrix_to_string` | `width=0, height=n` → `n` bare newlines | [x] |
| 32 | `matrix_to_string` | value forms: `0`, `-1`, 10-digit positives, `INT_MAX`; `INT_MIN` at `width=1` | [x] |
| 33 | `matrix_to_string` | randomized dims `[0,12]×[0,12]` × randomized safe-range values, 300 iters | [x] |
| 34 | `write_to_file` | new file, `content=""` | [x] |
| 35 | `write_to_file` | new file, single-line content | [x] |
| 36 | `write_to_file` | new file, multi-line content with `\n`, `\t`, high bytes | [x] |
| 37 | `write_to_file` | pre-existing longer file → `"w"` truncation semantics | [x] |
| 38 | `write_to_file` | content containing `%s`, `%d`, `%n` (must be emitted literally) | [x] |
| 39 | `write_to_file` | content `> BUFSIZ` (200 KiB) — forces a flush inside `fprintf` | [x] |
| 40 | `write_to_file` | randomized content bytes/lengths, 200 iters, comparing return code **and** resulting file bytes | [x] |
| 41 | `driver` (full pipeline) | `1×1 · 1×1`, compare return code + `matrix.txt` bytes | [x] |
| 42 | `driver` | square `3×3 · 3×3` | [x] |
| 43 | `driver` | general rectangle, all dims distinct | [x] |
| 44 | `driver` | inner dim `0` (`width_a=0, height_b=0`) | [x] |
| 45 | `driver` | `height_a=0` (empty output) and `width_b=0` | [x] |
| 46 | `driver` | surplus rows/columns + messy separators + non-numeric tokens in the input strings | [x] |
| 47 | `driver` | randomized dims/values, 150 iters, comparing return code **and** `matrix.txt` bytes | [x] |
| 48 | `driver` | output file pre-existing with longer content (truncation through the pipeline) | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table and no optional
dependencies, so the complete set of combinations is:

| combo | command |
|-------|---------|
| default (only combo) | `cargo test --release` |
| explicit no-default check | `cargo test --release --no-default-features` |

Both are exercised by `run_all_features.sh`.

## Additional coverage beyond the row table

| file | what it adds | ✅ |
|------|--------------|----|
| `tests/fuzz_differential.rs` | dense fixed-seed sweeps: 480 `matrix_to_string` cases across the decimal-length boundary values at every width 1–12; 2 000 randomized parser inputs; 1 200 randomized multiplications incl. wrapping accumulation; 500 randomized `write_to_file` payloads | [x] |
| `tests/phase_c_crafted.rs` | crafted `matrix_t` states with negative `width`/`height` — the loop-never-runs paths the C reaches without UB, incl. the case `width=-1, height=-1` where `buffer_size` comes out POSITIVE and `matrix_to_string` returns `""` instead of NULL | [x] |
| `tests/phase_d_symbols.rs` | cross-library ABI interop: matrices allocated by one `.so` are multiplied / stringified / freed by the other, plus mixed pipelines (C-parse → Rust-multiply and vice versa) and an explicit `matrix_t` layout pin | [x] |
| `tests/phase_b_matrix.rs::composed_pipeline_randomized` | the full low-level pipeline (`initialize_matrix_from_string` → `multiply_matrices` → `matrix_to_string`) driven end to end, 150 randomized shapes | [x] |

Every comparison also asserts that the two implementations write **identical
bytes to stderr** (fd 2 is redirected to a temp file around each call), so the
`perror` / `fprintf(stderr, …)` diagnostics are part of the differential, not
just return values.
