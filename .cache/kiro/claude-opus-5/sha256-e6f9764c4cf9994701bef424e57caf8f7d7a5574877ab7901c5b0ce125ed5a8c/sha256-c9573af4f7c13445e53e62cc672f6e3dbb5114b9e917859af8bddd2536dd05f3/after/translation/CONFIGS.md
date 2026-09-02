# CONFIGS.md — configuration-surface table

Derived mechanically from `c_src/src/goto.c` + `c_src/include/goto.h`.

## Axes the C actually branches on

There are **no** runtime options, modes, flags, globals, or `#ifdef`s in this
library — `grep -c '#if\|#ifdef\|static \|extern ' c_src/src/goto.c` finds none
beyond the two `#include`s. The configuration surface is therefore entirely
made of **input shapes**:

1. **Entry point** (3, all exported; only `driver` is in the header, so the two
   lower-level ones must be driven directly):
   `forward_goto_example` (lowest) → `open_with_cleanup` (low) → `driver`
   (composed pipeline over both).
2. **`num` shape** — the sign test `x < 0` and the arithmetic `x * 2`:
   negative / zero / positive-no-overflow / positive-overflowing /
   `INT_MIN` / `INT_MAX`.
3. **File-existence shape** — the `!fp` test: openable / not openable.
4. **File-content shape** — the `fgets` loop and the 100-byte buffer:
   empty / one short line / no trailing newline / many lines /
   length exactly 99, 100, 101 / much longer than the buffer /
   embedded NUL / arbitrary binary.
5. **Readability shape** — the `ferror` test: normal file / directory
   (opens but cannot be read).
6. **Pointer shape** — `filename`: valid / `""` / `NULL` / very long.

## Table

Every row is exercised against **both** `.so`s through `libloading`, comparing
the return value, the exact stdout bytes, and the exact stderr bytes. Rows
marked *randomized* use a fixed-seed PRNG (seed `0x5EED_600F_C0DE_1234`) with
many inputs per row, not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `forward_goto_example` | `x` in `0..=1000` exhaustively, plus 512 *randomized* non-negative `x` in `0 ..= INT_MAX/2` (no overflow) → success path, `printf("Processing: %d\n")`, returns `2x` | `cfg_c1_fwd_nonneg` | [x] |
| C2 | `forward_goto_example` | 512 *randomized* negative `x` in `INT_MIN ..= -1` → error path | `cfg_c2_fwd_negative` | [x] |
| C3 | `forward_goto_example` | signed-overflow boundary of `x * 2`: `INT_MAX/2`, `INT_MAX/2+1`, `INT_MAX`, `1<<30`, `(1<<30)+1`, `0x7FFFFFFE`, plus 256 *randomized* `x` in `INT_MAX/2 ..= INT_MAX` → result wraps negative | `cfg_c3_fwd_overflow` | [x] |
| C4 | `forward_goto_example` | full-`int32` *randomized* sweep, 2048 values, sign not constrained (mixes C1–C3 paths in one stream so stream/buffer state is also compared) | `cfg_c4_fwd_full_sweep` | [x] |
| C5 | `open_with_cleanup` | existing readable file, **empty** (0 bytes) → loop body never runs, `ferror` false, returns non-NULL open handle at EOF | `cfg_c5_empty_file` | [x] |
| C6 | `open_with_cleanup` | single line, short, **with** trailing `\n` | `cfg_c6_single_line_nl` | [x] |
| C7 | `open_with_cleanup` | single line, short, **without** trailing `\n` (final `fgets` returns a chunk not ending in `\n`) | `cfg_c7_single_line_no_nl` | [x] |
| C8 | `open_with_cleanup` | many lines (*randomized* count 2..50, *randomized* per-line length 0..250 incl. empty lines `"\n"`) | `cfg_c8_many_lines` | [x] |
| C9 | `open_with_cleanup` | buffer-boundary line lengths: 98, 99, 100, 101, 198, 199, 200 bytes with and without trailing newline — the `sizeof(buffer)==100` split points | `cfg_c9_buffer_boundaries` | [x] |
| C10 | `open_with_cleanup` | one line far longer than the buffer (*randomized*, 1 KiB–64 KiB) → dozens/hundreds of `fgets` iterations | `cfg_c10_huge_line` | [x] |
| C11 | `open_with_cleanup` | *randomized* raw binary content, 0–8 KiB, all byte values **except** `\0` (so `%s` is lossless) — exercises `\r`, high bytes, invalid UTF-8 | `cfg_c11_binary_no_nul` | [x] |
| C12 | `open_with_cleanup` | *randomized* raw binary content **including** `\0` bytes → `printf("%s")` truncates each chunk at its first NUL; both must truncate identically | `cfg_c12_binary_with_nul` | [x] |
| C13 | `open_with_cleanup` | content whose bytes look like `printf` conversions (`%s %d %n %%`) — must be emitted literally because it is passed as an *argument*, not a format | `cfg_c13_format_like_content` | [x] |
| C14 | `open_with_cleanup` | *returned handle* state on the success path: file was read to EOF but is returned **still open** — compare `ftell`/`feof` of the handle returned by C vs by Rust | `cfg_c14_returned_handle_state` | [x] |
| C15 | `open_with_cleanup` | filename shape: path with directories, name containing `%` and spaces, and a long-but-valid name → only affects the stderr `%s` on failure and the `fopen` call | `cfg_c15_filename_shapes` | [x] |
| C16 | `driver` | composed pipeline, `num >= 0` **and** file OK → full stdout sequence `Processing:` + `Goto output:` + file contents, returns `0`, closes the handle. *Randomized* over `num` × the content shapes of C5–C12 | `cfg_c16_driver_success` | [x] |
| C17 | `driver` | `num >= 0` with overflowing `2*num` → `Goto output:` prints the wrapped negative value, then normal file handling | `cfg_c17_driver_overflow_num` | [x] |
| C18 | `driver` | `num < 0` → short-circuits, `filename` never used (even when it points at a valid file) | `cfg_c18_driver_short_circuit` | [x] |
| C19 | `driver` | `num >= 0`, file unopenable → returns `-2` after the stdout prefix (valid-path composition of an error leaf) | `cfg_c19_driver_file_missing` | [x] |
| C20 | `driver` | `num >= 0`, path is a directory → `ferror` leaf inside the pipeline, returns `-2` | `cfg_c20_driver_directory` | [x] |
| C21 | all three | **stream-interleaving**: stdout and stderr redirected to the *same* fd, mixed success/failure calls in sequence, so the relative order of line-buffered stdout vs unbuffered stderr writes is compared, not just each stream in isolation | `cfg_c21_interleaved_streams` | [x] |
| C22 | all three | **call-sequence/state**: a long *randomized* script of interleaved calls to all three entry points in one process, comparing the whole accumulated output — catches per-call-only tests missing residual stream/buffer state | `cfg_c22_mixed_call_sequence` | [x] |
| C23 | `open_with_cleanup`, `driver` | **special sources**: `/dev/null`, procfs files (`st_size == 0` yet yield data), and a symlink to a regular file — a different kernel read path and different `ftell` behaviour than a regular file | `cfg_c23_special_files` | [x] |
