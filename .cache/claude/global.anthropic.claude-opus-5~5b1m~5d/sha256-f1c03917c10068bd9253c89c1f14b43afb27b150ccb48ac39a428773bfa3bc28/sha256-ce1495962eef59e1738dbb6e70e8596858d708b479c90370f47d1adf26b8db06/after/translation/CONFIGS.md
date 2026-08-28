# CONFIGS.md — Configuration-surface table (Phase A → gate for Phase B)

Mechanically derived from `c_src/src/goto.c` + `c_src/include/goto.h`.

## Axes the C code actually branches on

The library exposes **no** runtime options, modes, flags, globals or `#ifdef`s —
`goto.c` contains no `#if`/`#ifdef` and no configuration state. The branch set is
therefore driven purely by argument values and input-data shape:

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| A0 `fopen` mode string | the literal `"r"` — read access only, no write access required | `fopen(filename, "r")` |
| A1 `x` sign | `x < 0` vs `x >= 0` | `if (x < 0)` in `forward_goto_example` |
| A2 `x * 2` wrap | `x < 2^30` (no wrap) vs `x >= 2^30` (wraps negative; `-O0` emits `add %eax,%eax`) | `return x * 2` |
| A3 `fopen` outcome | `fp == NULL` vs `fp != NULL` | `if (!fp)` |
| A4 `fgets` loop trip count | 0 / 1 / many | `while (fgets(buffer, sizeof(buffer), fp))` |
| A5 buffer chunking | line length vs `sizeof(buffer) == 100` → 99 data bytes per chunk (`<99`, `==99`, `>99`) | `fgets(..., 100, ...)` |
| A6 `printf("%s", buffer)` truncation | data with vs without embedded `NUL` bytes | body of the loop |
| A7 `ferror(fp)` | 0 vs non-zero (readable file vs directory) | `if (ferror(fp))` |
| A8 returned `FILE*` | non-NULL (caller owns it: position/EOF/error state observable) vs NULL | `return fp` / `return NULL` |
| A9 `driver` first branch | `res == -1` vs `res != -1` | `if (res == -1)` |
| A10 `driver` second branch | `out == NULL` vs `out != NULL` (`fclose(out)`) | `if (out == NULL)` |

## Public entry points

All three exported symbols are driven **directly** through the `.so` — the
low-level `forward_goto_example` and `open_with_cleanup` are *not* covered only
via the `driver` convenience wrapper.

## Rows — combinations the C treats differently

Every row is compared C-vs-Rust on: return value, full stdout bytes, full stderr
bytes, and (for `FILE*` returns) NULL-ness + `ftell` + `feof` + `ferror` of the
returned stream. Every row uses **many randomized inputs** from a fixed-seed
xorshift PRNG (`SEED = 0x5DEECE66D`), not a single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | axes | reps | test | [x] |
|----|----------------|-------------------------------------------|------|------|------|-----|
| 1  | `forward_goto_example` | `x < 0`, randomized over `INT_MIN..0` | A1 | 400 | `cfg_01_fge_negative` | [x] |
| 2  | `forward_goto_example` | `x == 0` (boundary between A1 branches) | A1 | 1 | `cfg_02_fge_zero` | [x] |
| 3  | `forward_goto_example` | `0 < x < 2^30`, randomized (no wrap) | A1,A2 | 400 | `cfg_03_fge_positive` | [x] |
| 4  | `forward_goto_example` | `x >= 2^30`, randomized + `2^30`, `2^30-1`, `INT_MAX` (`x*2` wraps negative) | A2 | 400+3 | `cfg_04_overflow` | [x] |
| 5  | `open_with_cleanup` | regular file, **0 bytes** → loop runs 0 times, no error | A3,A4,A7,A8 | 1 | `cfg_05_empty_file` | [x] |
| 6  | `open_with_cleanup` | single line, `<99` bytes, **with** trailing `\n` → 1 trip | A4,A5 | 200 | `cfg_06_one_line_nl` | [x] |
| 7  | `open_with_cleanup` | single line, `<99` bytes, **no** trailing newline → 1 trip | A4,A5 | 200 | `cfg_07_one_line_no_nl` | [x] |
| 8  | `open_with_cleanup` | many lines, randomized count `2..40` and randomized lengths `0..250` | A4,A5 | 200 | `cfg_08_multi_line` | [x] |
| 9  | `open_with_cleanup` | line length exactly `97,98,99,100,101` and `199,200,201` (`fgets` 99-data-byte chunk boundary), × with/without trailing `\n` | A5 | 16 | `cfg_09_chunk_boundary` | [x] |
| 10 | `open_with_cleanup` | one enormous line, 64 KiB with no `\n` → ~660 chunked trips | A4,A5 | 3 | `cfg_10_huge_single_line` | [x] |
| 11 | `open_with_cleanup` | data containing embedded `NUL` bytes → `printf("%s")` truncates mid-buffer (output ≠ file bytes) | A6 | 200 | `cfg_11_embedded_nul` | [x] |
| 12 | `open_with_cleanup` | fully random binary bytes `0x00..0xFF` incl. `%`, `\r`, high-bit, invalid UTF-8 | A5,A6 | 200 | `cfg_12_random_binary` | [x] |
| 13 | `open_with_cleanup` | only newlines (randomized 1..64 blank lines) → many 1-byte trips | A4,A5 | 100 | `cfg_13_blank_lines` | [x] |
| 14 | `open_with_cleanup` | 1 MiB file, randomized line lengths (many trips, > `BUFSIZ`) | A4 | 2 | `cfg_14_large_file` | [x] |
| 15 | `open_with_cleanup` | `"/dev/null"` — character device, opens, 0 trips, no error | A3,A4,A7 | 1 | `cfg_15_dev_null` | [x] |
| 16 | `open_with_cleanup` | symlink pointing at a randomized regular file | A3,A4 | 50 | `cfg_16_symlink` | [x] |
| 17 | `open_with_cleanup` | success path: state of the **returned** `FILE*` (`ftell`, `feof`, `ferror`) across randomized file sizes; caller `fclose`s it | A8 | 200 | `cfg_17_returned_stream_state` | [x] |
| 18 | `driver` | `num < 0` (randomized) × {valid file, nonexistent file, `NULL`} — early `return -1`, file never opened | A9 | 300 | `cfg_18_driver_negative` | [x] |
| 19 | `driver` | `num >= 0` (randomized, no wrap) × randomized valid multi-line file → `0`, `fclose(out)` taken | A9,A10 | 300 | `cfg_19_driver_success` | [x] |
| 20 | `driver` | `num >= 2^30` (wrapping `x*2`) × valid file → `0`, prints the wrapped negative value | A2,A9,A10 | 200 | `cfg_20_driver_overflow_success` | [x] |
| 21 | `driver` | `num == 0` × {0-byte file, `/dev/null`, multi-line file} | A9,A10 | 3 | `cfg_21_driver_zero` | [x] |
| 22 | `driver` | full randomized **cross-product**: `num` drawn from the whole `INT_MIN..=INT_MAX` domain × 8 file shapes (missing, empty, 1-line, multi-line, chunk-boundary, NUL-embedded, binary, directory, `""`, `NULL`) | A1–A10 | 600 | `cfg_22_driver_cross_product` | [x] |
| 23 | `forward_goto_example` → `open_with_cleanup` composed by hand (same order `driver` uses), asserting the *composed* stdout stream interleaving matches | A1–A8 | 200 | `cfg_23_manual_pipeline` | [x] |
| 24 | `open_with_cleanup` | repeated calls on the same path in one process (stream/FD leak & buffering state divergence over 200 iterations) | A3,A4,A8 | 200 | `cfg_24_repeat_calls` | [x] |
| 25 | `open_with_cleanup`, `driver` | file permission bits `{0o444, 0o400, 0o440, 0o604, 0o644, 0o666, 0o200, 0o000}` — a **readable-but-not-writable** file must OPEN, which is what pins the mode string to `"r"` (mode `"r+"` would need write access) | A0,A3 | 8×2 | `cfg_25_permission_modes` | [x] |
| 26 | `open_with_cleanup`, `driver` | descriptor accounting over 750 calls mixing the success / `!fp` / `ferror` paths — the only way the cleanup block's `fclose(fp)` becomes observable | A7,A8 | 750 | `cfg_26_no_fd_leak` | [x] |

All 26 rows pass across their randomized inputs under every feature combination
listed in `SYMBOLS.md`, for both the `release` and the `debug` cdylib.

## Observed per call

Each row compares, byte-for-byte: the return value; for `FILE*` returns the
NULL-ness plus `ftell` / `feof` / `ferror` / `fileno` of the returned stream; the
complete stdout bytes; the complete stderr bytes; and the change in the number
of open file descriptors.

Rows 25 and 26 exist because the first version of this suite passed while two
deliberately broken Rust builds (`fopen` mode `"r+"`, and a cleanup path that
skips `fclose`) also passed — see `mutants.sh`. Return value + printed bytes
alone do not observe the mode string or the `fclose`.
