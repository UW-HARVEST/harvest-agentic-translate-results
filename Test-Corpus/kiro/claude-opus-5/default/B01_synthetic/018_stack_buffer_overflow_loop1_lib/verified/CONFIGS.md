# CONFIGS.md — Phase B configuration-surface table

Axes derived from the C source, not guessed:

* **Runtime options / modes.** The library has exactly one: `driver`'s
  `int useGood` parameter, which drives `if (useGood) good(); else bad();`
  (`driver.c:74-83`). Two states: zero → `bad`, non-zero → `good`.
  There are no other flags, no globals, no `#ifdef`, no `switch`.
* **`#ifdef` / compile-time branches.** `grep -c '#if' src/driver.c` → only the
  `DRIVER_H_` include guard in the header. No conditional code.
* **Public entry points (full set, lowest level first).** From `nm -D`, not from
  `driver.h` — the header only declares `driver`, but four more functions are
  exported and callable by any consumer:
  1. `printIntLine(int)` — lowest level, formats one `int`.
  2. `printLine(const char*)` — lowest level, guarded by a null check.
  3. `bad()` — composes `printIntLine` over an under-allocated `alloca`.
  4. `good()` — composes `printIntLine` over a correctly sized `alloca`.
  5. `driver(int)` — the convenience/one-shot wrapper over `bad`/`good`.
  Rows below exercise 1–4 **directly**, not only through 5.
* **Input shapes the code distinguishes.**
  * `printIntLine`: sign and magnitude of the `int` (`%d` formatting) —
    negative / zero / positive, and the `INT_MIN`/`INT_MAX` extremes.
  * `printLine`: null vs non-null (the only branch), then length
    (empty / 1 byte / many / very long) and byte content (ASCII / format
    metacharacters / embedded newlines / high non-UTF-8 bytes).
  * `bad`/`good`: no parameters; the shape axis is the *allocation size*
    (10 bytes vs 40 bytes) and the fixed count 10 of the copy loop. Their only
    observable is `data[0]`, always `source[0] == 0`.
  * Sequencing/state shape: repeated and interleaved calls, since `bad()`'s
    stack overrun could otherwise perturb later calls differently in the two
    implementations.

Cross-product, pruned to combinations the C actually treats differently:

| # | entry point(s) | configuration (options set + input shape) | randomized? | test | [x] |
|---|----------------|-------------------------------------------|-------------|------|-----|
| 1 | `printIntLine` | full 32-bit range, uniform random `int` | 4000 values, seed fixed | `cfg_row01_print_int_line_random_full_range` | [x] |
| 2 | `printIntLine` | small-magnitude values `-1000..=1000` (digit-count boundaries 1/2/3/4 and the `0` / sign edges) | 2001 exhaustive + shuffled | `cfg_row02_print_int_line_small_magnitude` | [x] |
| 3 | `printIntLine` | boundary set: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`, and every ±power-of-two / ±(10^k) digit-rollover | exhaustive boundary list | `cfg_row03_print_int_line_boundaries` | [x] |
| 4 | `printLine` | non-null, printable-ASCII payloads of random length 1..=64 | 1000 strings, seed fixed | `cfg_row04_print_line_random_ascii` | [x] |
| 5 | `printLine` | non-null, random arbitrary non-zero bytes `0x01..=0xFF` (non-UTF-8 included), length 1..=64 | 1000 strings, seed fixed | `cfg_row05_print_line_random_raw_bytes` | [x] |
| 6 | `printLine` | non-null, length boundaries: 0 (empty), 1, 2, 4095, 4096, 4097, 65536 | exhaustive boundary list | `cfg_row06_print_line_length_boundaries` | [x] |
| 7 | `printLine` | non-null, content that stresses stdio: embedded `\n`, `\t`, `\r`, `%`-format metacharacters, backslashes | exhaustive content list | `cfg_row07_print_line_special_content` | [x] |
| 8 | `printLine` | `line == NULL` (the guarded branch) mixed with non-null calls, so the "no output" case is checked *in sequence* | randomized null/non-null interleaving, 500 calls | `cfg_row08_print_line_null_interleaved` | [x] |
| 9 | `bad` | called directly (not via `driver`) — under-allocated `alloca(10)` path, single call | n/a (no inputs) | `cfg_row09_bad_direct_single` | [x] |
| 10 | `bad` | called directly, 200 consecutive times — repeated stack overrun | n/a | `cfg_row10_bad_direct_repeated` | [x] |
| 11 | `good` | called directly, single call — correctly sized `alloca(40)` path | n/a | `cfg_row11_good_direct_single` | [x] |
| 12 | `good` | called directly, 200 consecutive times | n/a | `cfg_row12_good_direct_repeated` | [x] |
| 13 | `driver` | `useGood != 0` (option ON) → `good` branch, random non-zero `int`s | 500 random non-zero values | `cfg_row13_driver_true_branch_random` | [x] |
| 14 | `driver` | `useGood == 0` (option OFF) → `bad` branch | repeated 200× | `cfg_row14_driver_false_branch` | [x] |
| 15 | `driver` | random 50/50 mix of zero and non-zero `useGood`, so the two branches alternate and any cross-branch state leak shows up | 1000 calls, seed fixed | `cfg_row15_driver_mixed_branches` | [x] |
| 16 | all five | fully randomized interleaving of `printIntLine`/`printLine`/`bad`/`good`/`driver` in one process — the composed pipeline, including stdio buffering order across the mixed `printf`/`puts` paths | 3000 randomized ops, seed fixed | `cfg_row16_all_entry_points_interleaved` | [x] |
| 17 | `bad`,`good` adjacency | `bad` and `good` called back-to-back, alternating 400×, to catch identical-code-folding of the two symbols and any `alloca` size confusion | n/a | `cfg_row17_bad_good_alternating` | [x] |
| 18 | `printLine`,`printIntLine` adjacency | interleaved `puts`-path and `printf`-path writes to the same stream, 1000 randomized ops (stdout byte-order check) | 1000 ops, seed fixed | `cfg_row18_puts_printf_interleaved` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)` — there is no
`add_executable`, so the project produces **no driver binary**. The
stdout-comparison requirement is instead satisfied by the `examples/harness`
runner, which is spawned once per row against the C `.so` and once against the
Rust `.so`, with its full stdout compared byte-for-byte.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. Verified mechanically in
`tests/differential.rs::feature_matrix_is_only_default` and by the
`scripts/check_features.sh` loop.
