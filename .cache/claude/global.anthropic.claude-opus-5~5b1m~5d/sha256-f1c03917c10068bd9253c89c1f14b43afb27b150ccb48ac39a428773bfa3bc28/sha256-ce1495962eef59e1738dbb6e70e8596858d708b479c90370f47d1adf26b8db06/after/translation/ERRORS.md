# ERRORS.md — Error-surface table (Phase A → gate for Phase C)

Mechanically derived from `c_src/src/goto.c`. Every rejection / early-exit /
sentinel-return in the C source gets one row. There are no `assert`s, no error
enums, no `RETURN_ERROR`-style macros and no numeric range checks in this
library; the complete rejection surface is:

* `goto error` in `forward_goto_example` (guarded by `x < 0`)
* `goto cleanup` in `open_with_cleanup` — reached from **two** distinct
  conditions (`!fp` and `ferror(fp)`)
* `return -1` / `return -2` in `driver`
* the sentinel returns `-1` (int) and `NULL` (`FILE*`)

`open_with_cleanup` has no explicit null check on `filename`; it is forwarded
straight to `fopen`, so glibc's `EFAULT` rejection is the actual behaviour and
is covered as its own row.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | [x] |
|----|----------|-------------------------------------------|-------------------|------|-----|
| 1  | `forward_goto_example` | `x < 0` (`goto error`), randomized negatives | stderr `"Error: negative input\n"` (emitted via `fwrite`), stdout empty, returns `-1` | `err_01_fge_negative` | [x] |
| 2  | `forward_goto_example` | `x == INT_MIN` (boundary, most-negative) | same as #1, returns `-1` | `err_02_fge_int_min` | [x] |
| 3  | `forward_goto_example` | `x == -1` (one step past the valid range `x >= 0`) | same as #1, returns `-1` | `err_03_fge_minus_one` | [x] |
| 4  | `open_with_cleanup` | `fopen` fails, `ENOENT`: path does not exist | stderr `"Error: opening or processing file <p>\n"`, **no** `fclose` (`fp` is NULL), returns `NULL` | `err_04_owc_enoent` | [x] |
| 5  | `open_with_cleanup` | `fopen` fails, `ENOENT`: `filename == ""` (zero-length string) | as #4 with empty `%s` | `err_05_owc_empty_name` | [x] |
| 6  | `open_with_cleanup` | `fopen` fails, `EFAULT`: `filename == NULL` — `%s` formats it as `(null)` | as #4, stderr `"...file (null)\n"`, returns `NULL` | `err_06_owc_null_ptr` | [x] |
| 7  | `open_with_cleanup` | `fopen` fails, `EACCES`: existing file with mode `000`, and mode `200` (write-only, so unreadable) | as #4, returns `NULL` (skipped when euid==0). Contrast with mode `444`, which must SUCCEED — that pair pins the mode string to `"r"` | `err_07_owc_eacces` | [x] |
| 8  | `open_with_cleanup` | `fopen` fails, `ENAMETOOLONG`: 5000-byte basename (oversized length) | as #4, returns `NULL` | `err_08_owc_enametoolong` | [x] |
| 9  | `open_with_cleanup` | `fopen` fails, `ENOTDIR`: regular file used as a path component | as #4, returns `NULL` | `err_09_owc_enotdir` | [x] |
| 10 | `open_with_cleanup` | `fopen` fails, `ELOOP`: self-referential symlink | as #4, returns `NULL` | `err_10_owc_eloop` | [x] |
| 11 | `open_with_cleanup` | `ferror(fp) != 0` after the `fgets` loop — `filename` is a **directory** (`fopen` succeeds, `fgets` fails `EISDIR`) | stderr `"Error: opening or processing file <d>\n"`, `fclose(fp)` **is** called, returns `NULL` | `err_11_owc_ferror_directory` | [x] |
| 12 | `driver` | `res == -1`, i.e. `num < 0` → returns before touching the file | stderr `"Error: negative input\n"`, no stdout, no `fopen` at all, returns `-1` | `err_12_driver_negative_num` | [x] |
| 13 | `driver` | `num < 0` **and** `filename` invalid (proves the file is never opened) | returns `-1`; stderr has only the negative-input line | `err_13_driver_negative_num_bad_file` | [x] |
| 14 | `driver` | `out == NULL` via the `!fp` branch (nonexistent file, `num >= 0`) | stdout `"Processing: n\nGoto output: 2n\n"`, stderr open-failure line, returns `-2` | `err_14_driver_open_fail` | [x] |
| 15 | `driver` | `out == NULL` via the `ferror` branch (`filename` is a directory) | as #14, returns `-2` | `err_15_driver_ferror_fail` | [x] |
| 16 | `driver` | `filename == NULL` with `num >= 0` | stderr `"...file (null)\n"`, returns `-2` | `err_16_driver_null_filename` | [x] |
| 17 | `driver` | `num == INT_MIN` (boundary) | returns `-1` | `err_17_driver_int_min` | [x] |

## Generic FFI boundary cases (required even though absent from the table)

| #  | case | expected | test | [x] |
|----|------|----------|------|-----|
| 18 | null pointer to `open_with_cleanup` / `driver` | see rows 6 / 16 — `NULL` / `-2`, no crash | `err_20_boundary_matrix`, `err_06_owc_null_ptr`, `err_16_driver_null_filename` | [x] |
| 19 | zero length: empty filename (row 5) and 0-byte input file | `NULL` / `-2` for the name; success + no output for the 0-byte file | `err_20_boundary_matrix`, `err_05_owc_empty_name` | [x] |
| 20 | oversized length: 5000-byte path (row 8), 1 MiB file, 64 KiB single line | `NULL` for the path; byte-identical streamed output for the big inputs | `err_20_boundary_matrix`, `err_08_owc_enametoolong` | [x] |
| 21 | one step past the valid `x` range for `forward_goto_example` (`x = -1`) | `-1` (row 3) | `err_03_fge_minus_one`, `err_23_int_full_range_sweep` | [x] |
| 22 | out-of-range **enum** value crossing the FFI boundary | **N/A** — the public API (`goto.h`) declares no `enum` and no `bool`; the only scalar is a plain `int` whose *entire* `INT_MIN..=INT_MAX` domain is valid input. Covered instead by full-range randomized `int` sweeps plus both extremes. | `err_23_int_full_range_sweep`, `cfg_22_driver_cross_product`, `err_02_fge_int_min`, `err_17_driver_int_min`, `cfg_04_overflow` | [x] |
| 23 | `int` values where the *non-error* path produces a negative result: `x >= 2^30` makes `x * 2` wrap (gcc `-O0` emits `add %eax,%eax`, so it wraps two's-complement), e.g. `INT_MAX → -2`; `driver` still treats it as success because it only compares against `-1` | `forward_goto_example` returns the wrapped value; `driver` prints it and continues | `err_23_int_full_range_sweep`, `cfg_04_overflow` | [x] |
| 24 | value that could alias the `-1` sentinel: no `x >= 0` can make `x * 2 == -1` (always even), so `driver`'s `res == -1` test is unambiguous — asserted explicitly | no false `-1` for any non-negative `x` | `err_23_int_full_range_sweep` | [x] |

All 24 rows have a passing differential test (see `cargo test` output), under
every feature combination and for both the `release` and `debug` cdylib.

Each error-path test asserts the SAME SPECIFIC rejection on both sides — the
exact sentinel (`-1`, `-2`, `NULL`) *and* the exact diagnostic bytes on stderr —
and additionally pins the absolute C behaviour (e.g. `err_11` asserts the
directory case really does return `NULL` through the `ferror` branch, `err_06`
asserts the stderr text is literally `...file (null)`). A test therefore cannot
pass by both sides failing in some new, matching-but-wrong way.
