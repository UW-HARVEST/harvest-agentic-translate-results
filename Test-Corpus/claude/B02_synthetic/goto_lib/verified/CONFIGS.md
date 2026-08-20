# CONFIGS.md — Configuration-surface table (Phase B)

Mechanically derived from the branches the C source actually takes.

## Public entry points (the FULL set — `nm -D --defined-only`)

| entry point | signature | level |
|-------------|-----------|-------|
| `forward_goto_example` | `int (int)` | lowest level (pure compute + `stdout`/`stderr`) |
| `open_with_cleanup` | `FILE* (const char*)` | lowest level (owns a `FILE*` it hands back to the caller) |
| `driver` | `int (int, const char*)` | convenience wrapper: calls the two above (`goto.h:26`) |

`driver` is the only entry point in the public header, but the other two are
exported and are therefore part of the ABI surface; Phase B drives them
**directly**, not only through `driver`.

## Axes the C code branches on

* **A — `int` value class** (`goto.c:30` `if (x < 0)`, `goto.c:35` `x * 2`,
  `goto.c:67` `if (res == -1)`):
  * A0 `x == 0` (boundary between the two branches)
  * A1 `0 < x < 2^30` (no overflow in `x*2`)
  * A2 `x == 0x3FFFFFFF` = `INT_MAX/2` (largest non-overflowing `x*2`)
  * A3 `0x40000000 <= x <= INT_MAX` (`x*2` overflows → wraps negative;
    `res` becomes negative but is never the `-1` sentinel)
  * A4 `x < 0` (the `goto error` branch — see ERRORS.md)
* **B — file/stream shape** (`goto.c:43` `fopen`, `:49` `fgets` loop with
  `sizeof(buffer) == 100`, `:50` `printf("%s")`, `:53` `ferror`):
  * B0 open fails (see ERRORS.md)
  * B1 opens, 0 bytes (loop body never runs)
  * B2 one line, `\n`-terminated
  * B3 one line, **no** trailing `\n` (last `fgets` returns a partial line)
  * B4 line length exactly on the `fgets` boundary: 98 / 99 / 100 / 101 data
    bytes (`fgets` stores at most 99 bytes + NUL, so 99+ splits into two
    iterations, and the 2nd may return just `"\n"`)
  * B5 many lines (loop runs many times)
  * B6 content contains embedded NUL bytes (`printf("%s")` truncates at the
    first NUL — a quirk that must be preserved)
  * B7 arbitrary binary content (all 256 byte values, incl. NUL and `%`
    characters that must **not** be interpreted as conversions)
  * B8 file of only `\n` bytes (every `fgets` returns a 1-byte string)
  * B9 large file (≥ 64 KiB, crosses the `stdout` buffer many times)
  * B10 `/dev/null` (opens, immediately EOF)
  * B11 path with non-UTF-8 bytes in the *file name* (legal on Linux;
    the name is echoed verbatim on the error path)
  * B12 opens but the read errors → `ferror != 0` (see ERRORS.md row 10)
* **C — stream topology** (the library writes to both `stdout` (buffered) and
  `stderr` (unbuffered); their interleaving is observable):
  * C0 `stdout` and `stderr` captured separately
  * C1 `stdout` and `stderr` redirected to the **same** fd (interleaving is
    part of the observable output)
* **D — call sequencing** (`stdout`'s buffer state persists between calls):
  * D0 one call per captured region
  * D1 several calls of mixed kinds inside one captured region

There are no runtime option setters, no global state, no modes/flags, no
`#ifdef`-selected variants and no `[features]`, so the axes above are the
complete configuration surface.

## Rows (pruned cross-product of the axes)

Implemented in `tests/phase_b_valid.rs`. Every row uses many seeded random
inputs (SplitMix64, fixed seed) unless the row is a single exact boundary.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `forward_goto_example` | A0 `x = 0` | `row01_fge_zero` | [x] |
| 2 | `forward_goto_example` | A1, 400 seeded random `x` in `1..2^30` | `row02_fge_positive_no_overflow` | [x] |
| 3 | `forward_goto_example` | A2 boundaries `0x3FFFFFFE`, `0x3FFFFFFF` | `row03_fge_half_intmax_boundary` | [x] |
| 4 | `forward_goto_example` | A3 overflow: `0x40000000`, `0x40000001`, `INT_MAX-1`, `INT_MAX` + 200 seeded randoms in `2^30..=INT_MAX` | `row04_fge_overflow` | [x] |
| 5 | `forward_goto_example` | A0–A3 exhaustive sweep of 4096 seeded random `i32` (all classes mixed, D1 sequencing) | `row05_fge_random_full_range` | [x] |
| 6 | `open_with_cleanup` | B1 empty regular file | `row06_owc_empty_file` | [x] |
| 7 | `open_with_cleanup` | B2 one `\n`-terminated line, 200 seeded random ASCII bodies (len 1..80) | `row07_owc_single_line_newline` | [x] |
| 8 | `open_with_cleanup` | B3 one line without trailing `\n`, 200 seeded random bodies | `row08_owc_single_line_no_newline` | [x] |
| 9 | `open_with_cleanup` | B4 exact `fgets` boundaries: bodies of 0,1,2,96–102,196–201,297–300 bytes × {with `\n`, without `\n`} × {first line, second line} | `row09_owc_fgets_buffer_boundaries` | [x] |
|10 | `open_with_cleanup` | B5 many lines: 150 seeded random files, 1..40 lines, line len 0..250, random trailing `\n` | `row10_owc_many_random_lines` | [x] |
|11 | `open_with_cleanup` | B6 embedded NUL bytes at line start / middle / end, plus 100 seeded random NUL-peppered files | `row11_owc_embedded_nuls` | [x] |
|12 | `open_with_cleanup` | B7 fully random binary, 100 seeded files of 1..4096 bytes over the full `0x00..=0xFF` alphabet (includes `%s`, `%n`, `\r`) | `row12_owc_random_binary` | [x] |
|13 | `open_with_cleanup` | B8 file of only `\n` (1, 2, 100, 5000 of them) | `row13_owc_only_newlines` | [x] |
|14 | `open_with_cleanup` | B9 large file ≥ 64 KiB (many `stdout` buffer flushes) | `row14_owc_large_file` | [x] |
|15 | `open_with_cleanup` | B10 `/dev/null` | `row15_owc_dev_null` | [x] |
|16 | `open_with_cleanup` | B11 file whose *name* has non-UTF-8 bytes, valid content | `row16_owc_non_utf8_name` | [x] |
|17 | `open_with_cleanup` | success path: the returned `FILE*` must be an open stream in the same state (`feof`, `ferror`, `ftell`) and `fclose` must return the same value — checked for B1,B2,B3,B5,B9 | `row17_owc_returned_stream_state` | [x] |
|18 | `driver` | A4 (`num < 0`) × valid file: file must never be opened → `-1` (200 seeded randoms) | `row18_driver_negative_num_valid_file` | [x] |
|19 | `driver` | A0 `num = 0` × B2 single-line file | `row19_driver_zero_single_line` | [x] |
|20 | `driver` | A1 × B5 multi-line random files, 150 seeded (num, file) pairs | `row20_driver_random_num_and_file` | [x] |
|21 | `driver` | A1 × B1 empty file | `row21_driver_empty_file` | [x] |
|22 | `driver` | A2/A3 overflowing `num` × B2 file → still `0` (`res` negative but ≠ `-1`) | `row22_driver_overflowing_num` | [x] |
|23 | `driver` | A1 × B7 random binary file | `row23_driver_binary_file` | [x] |
|24 | `driver` | A1 × B10 `/dev/null`, A1 × B11 non-UTF-8 name | `row24_driver_devnull_and_weird_name` | [x] |
|25 | all three | C1 `stdout`+`stderr` on the **same** fd — interleaving of buffered `stdout` and unbuffered `stderr` across a mixed batch of success/error calls | `row25_merged_streams_interleaving` | [x] |
|26 | all three | D1 long mixed session: 300 seeded random calls chosen among the three entry points with random valid/invalid inputs, all inside one capture (cumulative buffer state + `FILE*` bookkeeping) | `row26_long_mixed_session` | [x] |

## Resource-effect rows (not visible in stdout/stderr/return value)

The C code's `fclose` calls are observable only as descriptor accounting, so
they get dedicated rows in `ERRORS.md` (17–19) and are checked by comparing the
`/proc/self/fd` delta of both libraries over 64 repetitions
(`leak01…`/`leak02…`/`leak03…` in `tests/phase_c_errors.rs`).

## Status

**26/26 rows pass** (plus the 3 resource rows), under every feature
configuration and in both the `dev` and `release` profiles — see `./verify.sh`.
