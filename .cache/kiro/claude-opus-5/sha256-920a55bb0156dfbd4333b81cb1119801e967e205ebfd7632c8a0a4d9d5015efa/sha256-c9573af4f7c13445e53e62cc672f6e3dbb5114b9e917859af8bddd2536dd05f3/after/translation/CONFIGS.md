# CONFIGS.md — configuration-surface table (Phase A) / valid-path differential tests (Phase B)

## Axes, derived from the C source

**Runtime options / modes / flags: none.**

```sh
grep -n '#ifdef\|#if \|#ifndef\|getenv\|static \|extern \|#define' c_src/src/*.c c_src/include/*.h
# -> c_src/src/driver.c:33:#define OUT_FILE "matrix.txt"
```

The library has no flags, no modes, no env lookups, no conditional compilation
and no global state. `OUT_FILE` is a compile-time constant, so the only
"configuration" a caller can vary is **which entry point** it calls and **what
shape the data has**. The Rust crate likewise declares no `[features]`.

**Entry points (all 7, low-level included — `allocate_matrix` is not in the
header but is exported):**

| kind | symbols |
|------|---------|
| lowest-level | `allocate_matrix`, `free_matrix` |
| mid-level | `initialize_matrix_from_string`, `multiply_matrices`, `matrix_to_string`, `write_to_file` |
| one-shot wrapper | `driver` (composes all six) |

**Input-shape axes the C actually branches on:**

* `height` vs the `for (i = 0; i < height; i++)` loops → `0` / `1` / many
* `width` vs the `for (j = 0; j < width; j++)` loops → `0` / `1` / many
* `matrix_to_string`: `j < mat->width - 1` (separator vs no separator) → the
  last column is special-cased; `width == 0` degenerates to newlines only
* `multiply_matrices`: the inner `k < mat_a->width` loop → shared dimension
  `0` / `1` / many; `k == 0` yields an all-zero result
* value magnitude → `snprintf("%d")` output width 1..11 chars, and `int`
  wraparound in the `+=`/`*` accumulation
* `strtok_r(…, "\n")` / `strtok_r(…, " ")` tokenisation → runs of delimiters are
  collapsed and leading delimiters skipped; extra rows/columns are ignored
* `atoi` semantics → non-numeric ⇒ `0`, partial parse, `+`/`-` signs, leading
  whitespace, out-of-`long` saturation truncated to `int`
* `write_to_file` payload → empty / 1 byte / small / larger than the stdio
  buffer (forces flush inside `fprintf`); existing longer file ⇒ `"w"` truncates

Every row below is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (see `tests/common/mod.rs`), and both libraries are called
through their `.so` exports only.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| C1 | `allocate_matrix` + `free_matrix` | `width`/`height` ∈ {0,1,2,3,7,64} cross-product; assert both return non-NULL, that `width`/`height` fields round-trip, that all `height` row pointers are non-NULL and distinct, and that `free_matrix` on each accepts the pointer | [x] |
| C2 | `allocate_matrix` + `free_matrix` | `height == 0` (`malloc(0)`) with `width` ∈ {0,1,5}: row array is a valid non-NULL zero-size allocation, no rows written | [x] |
| C3 | `allocate_matrix` + `free_matrix` | `width == 0` (`malloc(0)` per row) with `height` ∈ {1,3,16}: every row pointer non-NULL | [x] |
| C4 | `initialize_matrix_from_string` | exact fit, single space separators, trailing `"\n"`, random `height`×`width` in 1..=6, random values in ±10^4 | [x] |
| C5 | `initialize_matrix_from_string` | exact fit, **no** trailing newline | [x] |
| C6 | `initialize_matrix_from_string` | runs of 2..5 spaces between columns (delimiter collapsing) | [x] |
| C7 | `initialize_matrix_from_string` | leading and trailing spaces on each row | [x] |
| C8 | `initialize_matrix_from_string` | blank lines interleaved / leading / trailing (`"\n"` delimiter collapsing) | [x] |
| C9 | `initialize_matrix_from_string` | **extra rows** beyond `height` (ignored), extra columns beyond `width` (ignored), and both at once | [x] |
| C10 | `initialize_matrix_from_string` | `height == 0` (loop skipped, returns a matrix without reading the string) with random garbage input strings | [x] |
| C11 | `initialize_matrix_from_string` | `width == 0`, `height` ∈ 1..=4: rows are tokenised but no column is stored | [x] |
| C12 | `initialize_matrix_from_string` | `atoi` corner tokens: `"0"`, `"-0"`, `"+7"`, `"007"`, `"  9"`, `"\t9"`, `"12abc"`, `"abc"`, `""`-adjacent, `"0x10"`, `"2147483647"`, `"-2147483648"`, `"99999999999999999999"`, `"-99999999999999999999"`, `"1e3"`, `"--5"`, `"3.9"` | [x] |
| C13 | `initialize_matrix_from_string` | 1×1, 1×N (single row), N×1 (single column) degenerate shapes | [x] |
| C14 | `initialize_matrix_from_string` | large shape: 40×30 with random values, single-space canonical form | [x] |
| C15 | `multiply_matrices` | random (m×k)·(k×n), m,k,n ∈ 1..=6, values in ±100; compare the full result matrix element-by-element plus `width`/`height` | [x] |
| C16 | `multiply_matrices` | shared dimension `k == 0` (`mat_a->width == 0 == mat_b->height`): result is `m×n` of zeros | [x] |
| C17 | `multiply_matrices` | `mat_a->height == 0` (empty result, `height == 0`) and `mat_b->width == 0` (rows of width 0) | [x] |
| C18 | `multiply_matrices` | `k == 1` (single accumulation step) and `k == 64` (long accumulation) | [x] |
| C19 | `multiply_matrices` | values near `INT_MAX`/`INT_MIN` so both the `*` and the `+=` wrap around (signed overflow — must wrap identically) | [x] |
| C20 | `multiply_matrices` | chained: `(A·B)·C` — feeding a library's own output back in, exercising the composed pipeline rather than a single call | [x] |
| C21 | `matrix_to_string` | random `height`×`width` in 1..=6, values in ±10^4 (multi-digit, mixed sign, separator/no-separator on the last column) | [x] |
| C22 | `matrix_to_string` | `width == 1` (no separator ever emitted) incl. the exactly-fitting 11-character `INT_MIN` value | [x] |
| C23 | `matrix_to_string` | `width == 0`, `height` ∈ 1..=5 → output is exactly `height` newlines | [x] |
| C24 | `matrix_to_string` | `height == 0` → output is the empty string (`buffer_size == 1`) | [x] |
| C25 | `matrix_to_string` | all values `0`; all values single-digit; all values 10-character (`-999999999`, `1000000000`) — the widest values that still fit the C's buffer formula | [x] |
| C26 | `matrix_to_string` | 40×30 large shape, random 1..10-character values | [x] |
| C27 | `write_to_file` | new file, contents ∈ {empty, 1 byte, short ASCII, text with embedded newlines}; compare return code **and** the resulting file bytes | [x] |
| C28 | `write_to_file` | overwrite an existing **longer** file (`"w"` truncation semantics) | [x] |
| C29 | `write_to_file` | payload larger than the stdio buffer (64 KiB) so the write is flushed inside `fprintf` | [x] |
| C30 | `write_to_file` | filename forms: relative, absolute, name containing `%` and other printf-ish characters (the C passes `content` as a `%s` argument, not as a format) | [x] |
| C31 | `write_to_file` | content containing `%s`/`%n`/`%d` (must be written literally, not interpreted) | [x] |
| C32 | `driver` | full pipeline, random square shapes 1..=5, values ±50; compare return code **and** the bytes of `matrix.txt` | [x] |
| C33 | `driver` | full pipeline, random non-square conformable shapes (m×k)·(k×n), m,k,n ∈ 1..=5 | [x] |
| C34 | `driver` | `width_a == height_b == 0` with non-zero outer dims (all-zero product) | [x] |
| C35 | `driver` | all dims `0` (empty product, empty `matrix.txt`) | [x] |
| C36 | `driver` | inputs with extra rows/columns and irregular whitespace (composition of C6–C9 with the full pipeline) | [x] |
| C37 | `driver` | values chosen so the products wrap `int` | [x] |
| C38 | **hand-built `matrix_t`** fed to `multiply_matrices` / `matrix_to_string` | a `matrix_t` allocated by library X consumed by library Y (and vice versa) — verifies the exported struct layout, not just each library in isolation | [x] |
| C39 | **cross-library pipeline** | `initialize_matrix_from_string` (C) → `multiply_matrices` (Rust) → `matrix_to_string` (C), and the mirror image; all 8 assignments of the 3 stages | [x] |

All 39 rows are checked off; each is asserted byte-for-byte against the C `.so`
across its randomized inputs (fixed seed `0x2545F4914F6CDD1D`).

## How to reproduce

```sh
cd translation && ./verify.sh
```

`verify.sh` rebuilds the C `.so`, enumerates the feature combinations from
`Cargo.toml` (here: default and `--no-default-features`, since no `[features]`
table exists), builds the Rust `cdylib` in both the `dev` and `release`
profiles, diffs `nm -D` against the C `.so`, and runs the whole differential
suite against each artifact via `DRIVER_RUST_SO`.

Test targets:

| file | rows covered | tests |
|------|--------------|-------|
| `tests/phase_b_valid.rs` | C1–C31, C38, C39 | 33 |
| `tests/phase_b_driver.rs` | C32–C37 | 6 |
| `tests/phase_c_errors.rs` | E2–E24 (in-process) | 21 |
| `tests/subprocess_parity.rs` | E12b, E23b, E24a, E25 (forked, one child per library) | 4 |
| `tests/symbol_parity.rs` | Phase D symbol diff | 2 |

66 tests, all passing in all four configurations.
