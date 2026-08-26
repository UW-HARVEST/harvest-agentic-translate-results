# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## How the axes were derived

`c_src/src/main.c` has no `argc`/`argv`, no `getenv`, no `#ifdef`, and no
options; `c_src/CMakeLists.txt` has no `option()`/`add_definitions()`. So there
are **no build-time and no command-line configuration axes** (verified by grep in
`ERRORS.md`). All of this program's variability is therefore in (a) the *shape of
the input byte stream* and (b) the *kind of the stdin/stdout streams*, because
those are what the C library branches on:

| axis | why it is an axis (what the C actually branches on) | values exercised |
|------|------------------------------------------------------|------------------|
| A. chunk terminator | `fgets(text, 128, stdin)` ends a chunk on `'\n'`, on 127 bytes, or on EOF | newline / 127-byte limit / EOF |
| B. loop trip count | `while (fgets(...))` runs 0, 1, or many times | 0 / 1 / 2 / many |
| C. first NUL position in a chunk | `fputs(text, ...)` stops at the first NUL | none / at 0 / middle / last byte / at index 127 |
| D. trailing newline | decides whether the final chunk ends at `'\n'` or at EOF | present / absent |
| E. byte values | the loop is byte-transparent; only `'\n'` and `'\0'` are special | ASCII / `\r\n` / high bytes / invalid UTF-8 / all 256 values |
| F. total size vs the 4096 stdio buffer | glibc `stdout` is block buffered at `st_blksize` (4096), so this decides *when* bytes appear | < 4096 / == 4096 / > 4096 / many blocks |
| G. stdout stream kind | glibc picks **line** buffering when `isatty(1)`, **block** buffering otherwise | tty / pipe / regular file |
| H. stdin stream kind | changes read granularity and EOF timing | regular file / pipe / `/dev/null` / tty |
| I. entry point | the executable's `main` via the process interface, and the *same* `main` exported from a `.so` via `dlopen` + FFI | `bin` / `.so` symbol |
| J. argv | ignored by `int main()`; must not change anything | none / several |

`I` is the "lowest-level entry point" axis: `main` **is** the lowest-level entry
point this library has (see `SYMBOLS.md` — it is the only exported symbol), and
it is driven both ways: as a process (`tests/differential_cli.rs`) and as a
dlopen()ed FFI symbol called directly (`tests/differential_so.rs`).

Rows below are the pruned cross-product: one row per combination the C treats
differently. Every row is run against **both** the C and the Rust build with
many randomized inputs (fixed seed `0x5EED_1234`, see the `Rng` splitmix64
generator in `tests/common/mod.rs`) unless the row is inherently a single fixed
shape.

## Configuration table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | bin + `.so` | empty input (0 bytes) — loop trips 0 times (axis B=0) | `cfg_01_empty` | [x] |
| 2 | bin + `.so` | single `"\n"` — shortest possible successful chunk (A=newline, C=none) | `cfg_02_single_newline` | [x] |
| 3 | bin + `.so` | one short line with trailing newline (B=1, D=present) | `cfg_03_one_line_nl` | [x] |
| 4 | bin + `.so` | one short line **without** trailing newline (A=EOF, D=absent) | `cfg_04_one_line_no_nl` | [x] |
| 5 | bin + `.so` | many short lines, all newline-terminated (B=many, F<4096) | `cfg_05_many_short_lines` | [x] |
| 6 | bin + `.so` | randomized mix of line lengths 0..300, random trailing newline (A/B/D randomized) | `cfg_06_random_line_lengths` | [x] |
| 7 | bin + `.so` | line of exactly 126 bytes + `\n` (127 total: fits in one chunk exactly) | `cfg_07_len126_plus_nl` | [x] |
| 8 | bin + `.so` | line of exactly 127 bytes + `\n` — chunk 1 hits the 127 limit, chunk 2 is the lone `"\n"` (A=limit then newline) | `cfg_08_len127_plus_nl` | [x] |
| 9 | bin + `.so` | exactly 127 bytes, no newline, then EOF (A=limit, then `fgets`→NULL) | `cfg_09_len127_no_nl` | [x] |
| 10 | bin + `.so` | exactly 128 bytes, no newline (chunk 1 = 127, chunk 2 = 1 byte) | `cfg_10_len128_no_nl` | [x] |
| 11 | bin + `.so` | lengths swept over 120..136 and 250..260, ×{newline, no newline} — the whole 127-boundary neighbourhood | `cfg_11_boundary_sweep` | [x] |
| 12 | bin + `.so` | NUL in the middle of a chunk (C=middle) → `fputs` truncation | `cfg_12_nul_middle` | [x] |
| 13 | bin + `.so` | NUL as the first byte of a chunk (C=0) → chunk emits nothing | `cfg_13_nul_leading` | [x] |
| 14 | bin + `.so` | NUL as the last byte before the newline (C=last) | `cfg_14_nul_before_newline` | [x] |
| 15 | bin + `.so` | NUL exactly at input offset 126 and at 127, i.e. at/next to the chunk boundary (C×A interaction) | `cfg_15_nul_at_boundary` | [x] |
| 16 | bin + `.so` | input consisting only of NUL bytes, no newline (non-empty input, empty output) | `cfg_16_all_nuls` | [x] |
| 17 | bin + `.so` | randomized inputs with NULs sprinkled at random positions and random density (C randomized × A/B randomized) | `cfg_17_random_with_nuls` | [x] |
| 18 | bin + `.so` | `\r\n` line endings — `\r` is not special, must pass through | `cfg_18_crlf` | [x] |
| 19 | bin + `.so` | all 256 byte values in order (E=full range, includes NUL and `\n`) | `cfg_19_all_byte_values` | [x] |
| 20 | bin + `.so` | high bytes / invalid UTF-8 sequences (`\xff\xfe`, lone continuation bytes) — must not be mangled or replaced | `cfg_20_invalid_utf8` | [x] |
| 21 | bin + `.so` | uniformly random binary bytes, no structure (E=random, C/A random) | `cfg_21_random_binary` | [x] |
| 22 | bin + `.so` | input length exactly 4095 / 4096 / 4097 bytes — the stdout block-buffer boundary (F) | `cfg_22_stdio_buffer_boundary` | [x] |
| 23 | bin + `.so` | input spanning many 4096 blocks (~1 MB, F=many) | `cfg_23_multi_block_large` | [x] |
| 24 | bin | stdout is a **pipe** → glibc block buffers: nothing may appear before 4096 bytes accumulate (G=pipe) | `cfg_24_stdout_pipe_block_buffered` | [x] |
| 25 | bin | stdout is a **regular file** (G=file, also block buffered) | `cfg_25_stdout_regular_file` | [x] |
| 26 | bin | stdout is a **tty** → glibc line buffers: each line must appear immediately (G=tty) | `cfg_26_stdout_tty_line_buffered` | [x] |
| 27 | bin | stdin is a **regular file** (H=file) | `cfg_25_stdout_regular_file` / all CLI rows | [x] |
| 28 | bin | stdin is a **pipe** fed incrementally, writer closes late (H=pipe, EOF timing) | `cfg_28_stdin_pipe_incremental` | [x] |
| 29 | bin | stdin is a **tty** (H=tty) with stdout a pipe | `cfg_29_stdin_tty` | [x] |
| 30 | bin | stdin **and** stdout are the same tty — the "interactive echo" the comment describes (G=tty × H=tty) | `cfg_30_interactive_tty_both` | [x] |
| 31 | bin | command-line arguments passed (J=several) — must be ignored | `cfg_31_args_ignored` | [x] |
| 32 | `.so` | `main` called through `dlopen`/`dlsym` (I=FFI) for every input shape above, checking both the returned `int` and the bytes written to fd 1 | `so_differential_all` | [x] |
| 33 | `.so` | `main` called **twice in a row** in the same loaded image (fresh fd 0/1 each time) — state must not leak between calls | `so_differential_repeat` | [x] |
| 34 | bin | stdin **and** stdout are the *same file*, opened separately, with NUL-bearing input so the output is shorter than the input (G=file × H=file × C=middle) — the resulting bytes depend on the exact interleaving of the 4096-byte reads and writes, so this discriminates the buffering emulation and not just the final byte sequence | `cfg_34_same_file_stdin_stdout` | [x] |
| 35 | bin | stdin is a descriptor **shared** with the parent (`dup`, shared file offset) — how much input was consumed is externally visible | `cfg_35_shared_stdin_leftover` | [x] |

## Feature combinations

No `[features]` in `Cargo.toml` and no CMake options ⇒ exactly one combination
(the empty one). `scripts/verify_all.sh` still runs `cargo check`/`cargo test`
with `--no-default-features` and with `--all-features` so the claim is checked
rather than assumed, and it repeats the whole Phase B + Phase C suite for each.

## Divergences found and fixed

Driving these rows against the C build found three real bugs in the original
translation. All three were in the *stream semantics*, which is exactly where a
program this small can hide them:

| # | row that caught it | symptom | fix |
|---|--------------------|---------|-----|
| 1 | 24 (`stdout` is a pipe) | the C build buffers 4096 bytes before writing anything, the Rust build echoed every line immediately, so output became visible at completely different times | `src/echo.rs` reimplements glibc's `stdout` discipline: line buffered when `isatty(1)`, otherwise 4096-byte block buffered, flushed when `main` returns |
| 2 | `ERRORS.md` row 12 (broken pipe) | the C build is **killed by SIGPIPE** (wait status `-13`); the Rust build exited 0, because the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` | `src/main.rs` restores `SIG_DFL` for `SIGPIPE` at startup |
| 3 | `ERRORS.md` row 11 (`stdout` closed) | C ignores the result of `fputs` and keeps draining stdin; the Rust version stopped at the first write error | `echo::run()` discards write errors and runs the loop to end of input |

Rows 26, 34 and 35 exist specifically to pin fixes 1 and 3 in place: row 26
checks the *terminal* half of the buffering rule (which a fully-buffered
implementation would fail), row 34 checks the read/write interleaving byte for
byte, and row 35 checks that stdin still gets drained.
