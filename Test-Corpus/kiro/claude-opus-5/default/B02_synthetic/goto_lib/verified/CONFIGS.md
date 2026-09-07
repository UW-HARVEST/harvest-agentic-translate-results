# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/goto.c` and `c_src/include/goto.h`.

## Axes the C actually branches on

There are **no runtime options, modes, flags, `switch`es, or `#ifdef`s** in the
C source (`grep -c -E 'switch|#ifdef|#if |static .*=' c_src/src/goto.c` → 0
option state). The library is stateless. The axes are therefore:

1. **Entry point** — the full public set, including the two low-level functions
   that are *not* in the header but *are* exported:
   `forward_goto_example` (lowest level), `open_with_cleanup` (lowest level),
   `driver` (the composing wrapper). All three are driven directly.
2. **`int` value shape** for `forward_goto_example`/`driver`:
   negative / zero / small positive / large positive that overflows `x*2` /
   `INT_MIN` / `INT_MAX`.
3. **File shape** for `open_with_cleanup`/`driver` — every distinct shape the
   `fgets`/`printf`/`ferror` loop treats differently:
   missing, empty (0 bytes ⇒ loop body never runs), one short line, many lines,
   no trailing newline, line exactly 98/99/100/101 bytes (the `sizeof(buffer)`
   boundary ⇒ chunk-splitting), very long single line (multi-chunk), embedded
   `\0` bytes (`printf("%s")` stops early — the C's own quirk), `\r\n`, bytes
   ≥ 0x80 / arbitrary binary, file much larger than stdio's buffer, directory,
   unreadable file, NULL name.
4. **Stream capture** — both fd 1 and fd 2 are captured for *every* call, so
   ordering and exact bytes on stdout and stderr are compared, not just the
   return value.
5. **Feature combination** — `Cargo.toml` has no `[features]`, so there is a
   single combination; it is still run explicitly with `--no-default-features`
   and with default features (see `SYMBOLS.md`).

Rows are the pruned cross-product of the combinations the code distinguishes.
Each row is exercised with **many randomized inputs** (`SEED = 0x5EED_C0DE`,
a deterministic xorshift64* PRNG in `tests/common/mod.rs`), not one value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `forward_goto_example` | randomized negative `x` (error path) ×256 | [x] |
| 2 | `forward_goto_example` | randomized non-overflowing positive `x` (`0 < x < 2^30`) ×256 | [x] |
| 3 | `forward_goto_example` | `x == 0` boundary | [x] |
| 4 | `forward_goto_example` | randomized `x` in `[2^30, INT_MAX]` ⇒ `x*2` wraps negative ×256 | [x] |
| 5 | `forward_goto_example` | `INT_MIN`, `INT_MIN+1`, `-1`, `1`, `2^30-1`, `2^30`, `INT_MAX-1`, `INT_MAX` | [x] |
| 6 | `forward_goto_example` | fully randomized `x` over all of `i32` ×2048 (sign mix) | [x] |
| 7 | `open_with_cleanup` | file missing / nonexistent randomized names ×64 | [x] |
| 8 | `open_with_cleanup` | empty file (0 bytes) — `fgets` returns NULL immediately | [x] |
| 9 | `open_with_cleanup` | single line, random printable content, with `\n` ×64 | [x] |
| 10 | `open_with_cleanup` | single line, no trailing newline ×64 | [x] |
| 11 | `open_with_cleanup` | many lines (2..40), random lengths 0..30, with `\n` ×64 | [x] |
| 12 | `open_with_cleanup` | many lines, last line without `\n` ×64 | [x] |
| 13 | `open_with_cleanup` | line length exactly 98 / 99 / 100 / 101 / 199 / 200 bytes — `sizeof(buffer)==100` chunk boundary | [x] |
| 14 | `open_with_cleanup` | one very long line, random length 500..5000, no newline (many `fgets` chunks) | [x] |
| 15 | `open_with_cleanup` | content with embedded `\0` bytes (C's `printf("%s")` truncates at NUL — quirk must match) ×64 | [x] |
| 16 | `open_with_cleanup` | `\r\n` line endings, and lone `\r` ×64 | [x] |
| 17 | `open_with_cleanup` | arbitrary binary bytes 0x00..0xFF, random length 0..4096 ×64 | [x] |
| 18 | `open_with_cleanup` | file larger than stdio's default 4096-byte buffer (8 KiB..64 KiB of random lines) ×16 | [x] |
| 19 | `open_with_cleanup` | file consisting only of newlines (empty lines) ×64 | [x] |
| 20 | `open_with_cleanup` | success path state check: returned `FILE*` non-NULL, `feof != 0`, `ferror == 0` for both libs | [x] |
| 21 | `driver` | `num < 0` (short-circuit) × random filenames incl. valid/missing ×64 | [x] |
| 22 | `driver` | `num >= 0` + missing file ⇒ `-2` ×64 | [x] |
| 23 | `driver` | `num >= 0` + valid file (each of the file shapes in rows 8–19) ⇒ `0` | [x] |
| 24 | `driver` | randomized `num` over all of `i32` × randomized file shape (full cross-product sample) ×512 | [x] |
| 25 | `driver` | overflowing `num` (`>= 2^30`) + valid file ⇒ `Goto output: <negative>` then `0` ×64 | [x] |
| 26 | all three | called repeatedly in sequence in one process (statelessness / no cross-call state) ×256 mixed | [x] |
| 27 | all three | default features **and** `--no-default-features` (single combination, run both ways) | [x] |
