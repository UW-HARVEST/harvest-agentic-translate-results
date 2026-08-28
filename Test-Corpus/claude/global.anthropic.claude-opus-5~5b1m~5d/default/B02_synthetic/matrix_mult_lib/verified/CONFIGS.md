# CONFIGS.md — Phase A configuration surface table

The library has **no runtime options, no flags, no modes, no `#ifdef`s and no
enums**. Grepping `c_src/` for `#if`, `switch`, `getenv`, `static` globals and
setter functions finds none. The only things the C branches on are therefore:

* which of the **7 public entry points** is called
  (`allocate_matrix`, `free_matrix`, `initialize_matrix_from_string`,
  `multiply_matrices`, `matrix_to_string`, `write_to_file`, `driver`);
* the **shape** of the input: `width`, `height` (`int`, any value) and the
  relationship `mat_a->width == mat_b->height`;
* the **format** of the input string, as seen by `strtok_r(.., "\n", ..)` /
  `strtok_r(.., " ", ..)` and `atoi`;
* the **magnitude/sign** of the element values (which changes the number of
  digits `snprintf("%d")` emits and therefore the layout inside the
  `matrix_to_string` buffer);
* for `write_to_file`, the **content length** relative to `BUFSIZ` (which
  decides whether the failure/flush happens inside `fprintf` or inside
  `fclose`) and whether the target file already exists.

Rows below are the cross-product of those axes, pruned to the combinations the
C code actually distinguishes. Every row is driven with **many randomized
inputs** from a fixed-seed PRNG (`SEED = 0x5EED_1234_ABCD_0001`), not a single
hand-picked value.

> **Value-range note (deliberate, not a shortcut).** `matrix_to_string` sizes its
> buffer as `h*(w*10 + w) + h + 1` = 11 bytes per element + 1 byte per row, but
> needs `strlen` per element **+ (w-1) separators + 1 newline** per row. For
> `w >= 2` any row whose elements average more than 10 characters overruns the
> heap allocation — a real bug in the C that is faithfully reproduced in Rust.
> Randomized rows therefore keep `|value| <= 999_999_999` (<= 10 chars) so the
> comparison observes defined behaviour; the `w == 1` rows, where the C sizing is
> *exactly* tight (12 bytes per row, 11 digits + newline), sweep the **full**
> `i32` range including `INT_MIN`/`INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `allocate_matrix` + `free_matrix` | `w,h` over `{0,1,2,3,7,64}²` (incl. `0×0`, `0×h`, `w×0`) — round-trip alloc/free, compare returned struct fields (`width`, `height`, rows-pointer non-null) | [x] |
| C2 | `allocate_matrix` + `free_matrix` | randomized `w,h ∈ [0,128]`, write then read back every cell through the returned `int**` to prove the row/col layout matches | [x] |
| C3 | `initialize_matrix_from_string` | exact fit: `h` rows × `w` cols, single spaces, no trailing newline; randomized `w,h ∈ [1,8]`, values `∈ [-999999999, 999999999]` | [x] |
| C4 | `initialize_matrix_from_string` | exact fit **with** a trailing `"\n"` | [x] |
| C5 | `initialize_matrix_from_string` | **extra columns** present in every row (surplus tokens ignored) | [x] |
| C6 | `initialize_matrix_from_string` | **extra rows** present (surplus rows ignored) | [x] |
| C7 | `initialize_matrix_from_string` | runs of **multiple spaces** between tokens + leading/trailing spaces per row (`strtok_r` collapses them) | [x] |
| C8 | `initialize_matrix_from_string` | **leading / consecutive / trailing newlines** (blank lines are skipped by `strtok_r`) | [x] |
| C9 | `initialize_matrix_from_string` | `w == 1` (no space delimiter appears at all), `h ∈ [1,8]`, **full `i32`** value range incl. `INT_MIN`/`INT_MAX` | [x] |
| C10 | `initialize_matrix_from_string` | `h == 1` (single row, no newline delimiter), `w ∈ [1,16]` | [x] |
| C11 | `initialize_matrix_from_string` | `w == 0`, `h == 0` and `w == 0 && h == 0` — degenerate shapes that still succeed (`malloc(0)`) | [x] |
| C12 | `initialize_matrix_from_string` | token **forms** fed to `atoi`: `"abc"`, `"12abc"`, `"+7"`, `"-0"`, `"007"`, `"0x10"`, `"2147483647"`, `"-2147483648"`, `"99999999999999999999"`, `"-99999999999999999999"`, `"1e3"`, `"."`, `"--3"` | [x] |
| C13 | `matrix_to_string` | `w == 1`, `h ∈ [1,8]`, full `i32` values (no separators; buffer exactly tight) | [x] |
| C14 | `matrix_to_string` | `w >= 2` square/non-square, randomized values `∈ [-999999999,999999999]` (separators + newline per row) | [x] |
| C15 | `matrix_to_string` | degenerate shapes: `0×0` → `""`, `w×0` → `""`, `0×h` → `h` bare newlines | [x] |
| C16 | `matrix_to_string` | all-zero matrix, all-negative matrix, mixed-sign matrix, single-digit vs 10-digit widths (digit-count boundaries 1/2/9/10 chars) | [x] |
| C17 | `multiply_matrices` | `a: h_a×k`, `b: k×w_b` with randomized `h_a, k, w_b ∈ [1,8]`, small values (`|v| <= 32`) so no wrap | [x] |
| C18 | `multiply_matrices` | inner dimension `k == 0` (loop never runs → result is written as all zeros) and `k == 1` | [x] |
| C19 | `multiply_matrices` | `mat_a->height == 0` and/or `mat_b->width == 0` (result allocated but no cell written) | [x] |
| C20 | `multiply_matrices` | **`int` overflow / wraparound** in the accumulator: values near `±2^15…±2^16` with `k >= 4` so products and sums wrap | [x] |
| C21 | `multiply_matrices` | non-square chains `1×N * N×1` (dot product) and `N×1 * 1×N` (outer product) | [x] |
| C22 | `write_to_file` | new file, content `< BUFSIZ`; check return value **and** the resulting file bytes | [x] |
| C23 | `write_to_file` | content `> BUFSIZ` (multiple flushes) and content exactly `BUFSIZ` | [x] |
| C24 | `write_to_file` | **overwrite/truncate**: pre-existing file longer than the new content (`"w"` mode truncation) | [x] |
| C25 | `write_to_file` | `content == ""` (0 bytes; `fprintf` returns 0 which is **not** `< 0`) | [x] |
| C26 | `write_to_file` | content containing `\n`, `\t`, high-bit/UTF-8 bytes and `%` characters (must not be interpreted as a format) | [x] |
| C27 | `write_to_file` | target `/dev/null` (opens, writes, closes successfully → `0`) | [x] |
| C28 | `driver` (full pipeline, low-level entry points composed) | randomized valid `w_a×h_a * w_b×h_b` with `w_a == h_b`, `∈ [1,8]`, small values; compares return code **and** the bytes of `matrix.txt` | [x] |
| C29 | `driver` | degenerate but valid pipelines: `w_a == h_b == 0`, `h_a == 0`, `w_b == 0` | [x] |
| C30 | `driver` | `w_a == h_b == 1` (1×1 result) and `1×N * N×1` / `N×1 * 1×N` shapes | [x] |
| C31 | end-to-end composition | `initialize_matrix_from_string` → `multiply_matrices` → `matrix_to_string` → `write_to_file` driven **manually** through the individual exports (not via `driver`), then the file bytes compared | [x] |
| C32 | all entry points | large-ish stress shapes (`w,h` up to 64) with randomized values, to catch out-of-range indexing / row-stride mistakes | [x] |

## Result

All 32 rows pass, driven from `tests/phase_b_configs.rs` (one `#[test]` per row,
named `c1_…` … `c32_…`). Roughly 3,300 differential comparisons run per suite
invocation; each compares the return value, every reachable byte of the produced
matrix / string / file, **and** the bytes written to `stderr`.

Corrections made to this table while deriving it from the C (recorded so the
table stays honest about what the C actually does):

* `initialize_matrix_from_string("   ", 0, 1)` **succeeds** — `strtok_r` with the
  `"\n"` delimiter returns the whole blank line as row 1, and the `width == 0`
  inner loop never runs. Only asking for a *second* row fails.
* `matrix_to_string` on `width == -1, height == -1` **succeeds** and returns `""`:
  `buffer_size = (-1)*((-1)*10 + -1) + (-1) + 1 = 11`, a valid allocation, and
  both loops are then skipped.
