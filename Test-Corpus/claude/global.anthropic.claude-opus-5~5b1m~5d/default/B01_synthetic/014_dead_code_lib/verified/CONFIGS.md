# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C actually branches on

The library is deliberately tiny, so the axis enumeration is short but is
derived, not guessed:

1. **Runtime options / modes / flags:** *none*. There is no global state, no
   setter, no mode enum, no `#ifdef`, no environment lookup. `grep -E
   'static [^v]|#if|getenv' c_src/src/driver.c` finds only the two `static`
   *functions*, no mutable state. So the option axis has exactly one value.
2. **Entry points (the FULL set, lowest level first):**
   - `printLine` — the lowest-level primitive; the only one taking input.
   - `bad`, `good` — mid level; `good` additionally calls `static helperGood`.
   - `driver` — the top-level convenience wrapper (the only symbol in the
     public header); composes the whole pipeline.
   The low-level `printLine`/`bad`/`good` are exercised **directly**, not only
   through `driver`.
3. **Input shape** (only `printLine` has an input; shapes the code /
   `puts` distinguish): NULL vs non-NULL; length 0 / 1 / many / 4095; bytes
   that are printable ASCII, embedded newlines, embedded `%` and `%s`
   (format-string shape — matters because the C passes the string as a `printf`
   *argument*, never as the format), `0x01..0xFF` non-UTF-8 / high-bit bytes,
   and bytes that are themselves `\0`-adjacent.
4. **Call sequence / statefulness:** repeated and interleaved calls, and calls
   made *alternately* into the C `.so` and the Rust `.so` on the same
   `stdout` stream — this is a real axis because both libraries import the same
   `puts@GLIBC` and so share one `FILE*` and its buffer.

## Table

One row per combination the C treats differently. Every row is driven with many
randomized inputs (seeded LCG, fixed seed `0x2026_09_05`) where an input exists.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `printLine` | non-NULL, printable-ASCII, random length 1..64 — 512 random inputs | `cfg_1_print_line_random_ascii` | [x] |
| 2 | `printLine` | non-NULL, length exactly 1, every single byte value `0x01..0xFF` (all 255 valid one-char strings) | `cfg_2_print_line_every_single_byte` | [x] |
| 3 | `printLine` | non-NULL, length 0 (empty string) | `cfg_3_print_line_empty` | [x] |
| 4 | `printLine` | non-NULL, random arbitrary non-zero bytes `0x01..0xFF`, length 1..256 (non-UTF-8 / high-bit shape) — 512 random inputs | `cfg_4_print_line_random_binary` | [x] |
| 5 | `printLine` | non-NULL, string containing embedded `\n`, `\r`, `\t` (multi-line shape) — 256 random inputs | `cfg_5_print_line_embedded_newlines` | [x] |
| 6 | `printLine` | non-NULL, string containing `%`, `%s`, `%n`, `%d` conversion-like sequences (format-string shape; C uses it as an *argument*, so they must be emitted literally) — 256 random inputs | `cfg_6_print_line_format_specifiers` | [x] |
| 7 | `printLine` | non-NULL, long strings: lengths 1, 2, 255, 256, 257, 1023, 1024, 4095 (buffer/boundary shape) | `cfg_7_print_line_lengths_boundary` | [x] |
| 8 | `printLine` | NULL (the guard's false branch, as a *valid* documented call) | `cfg_8_print_line_null` | [x] |
| 9 | `bad` | nullary, single call — no option state | `cfg_9_bad_single` | [x] |
| 10 | `good` | nullary, single call — must also emit the `static helperGood()` line, in order | `cfg_10_good_single` | [x] |
| 11 | `driver` | nullary, single call — full composed pipeline, all 6 lines in exact order | `cfg_11_driver_single` | [x] |
| 12 | `bad`, `good`, `driver` | nullary, repeated 64× each — idempotence / no hidden state accumulation | `cfg_12_nullary_repeated` | [x] |
| 13 | all four | randomized *interleaved* call sequence (200 calls, random pick of `printLine`/`bad`/`good`/`driver` with random payloads) — the composed pipeline, which per-wrapper tests cannot see | `cfg_13_random_interleaved_sequence` | [x] |
| 14 | all four | C and Rust invoked **alternately on the same `stdout` stream** within one sequence, output compared against the concatenation of the separate runs — verifies both share `puts`' buffering discipline | `cfg_14_cross_library_shared_stdout` | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares `add_library(driver SHARED …)` only — there is
`add_executable` nowhere in the project, and `translation/Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]`. **No driver binary is built, so the
"compare binary stdout" gate is not applicable.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and the crate has no
`#[cfg(feature = …)]` anywhere (`grep -rn 'feature' src/` → no matches), so the
only build configuration is the default one. Verified in Phase D by running the
suite under `--no-default-features` as well, which is identical here.
