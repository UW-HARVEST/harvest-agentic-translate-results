# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Axes the C code actually branches on

Derived from `c_src/src/driver.c` + `c_src/include/driver.h`:

* **Runtime options / modes / flags:** *none*. There is no configuration struct,
  no global variable, no setter, no `#ifdef` other than the header include
  guard `DRIVER_H_`, and no environment lookup. `grep -c '#if\|#ifdef\|#ifndef'
  c_src/src/driver.c` → 0.
* **Cargo features:** `translation/Cargo.toml` declares **no `[features]`
  section**, so the only build configuration is the default one. (Phase D
  feature-combination sweep therefore has exactly one combination; it is still
  run explicitly.)
* **Control-flow branches:** exactly one — `if (line != NULL)` in `printLine`.
* **Input shapes the code distinguishes:**
  * `printLine`: NULL vs non-NULL pointer; then the *content* of the byte
    string — empty, 1 byte, ASCII, embedded printf conversion specifiers,
    embedded whitespace/newlines, non-UTF-8 high bytes, long (page-crossing)
    buffers. Content matters because GCC lowers `printf("%s\n", s)` to
    `puts(s)`, and because a naive Rust translation using `str` would reject
    non-UTF-8.
  * `printIntLine`: the `int` value — 0, +1, -1, small, large, `INT_MIN`,
    `INT_MAX`, and random 32-bit values (sign and digit-count vary the
    `printf("%d")` path).
  * `bad` / `good` / `driver`: no inputs; the shape axis is *call composition*
    (single call, repeated calls, interleaving with the leaf functions).
* **Public entry points — the FULL set (5), not just the `driver` wrapper:**
  the low-level leaves `printLine`, `printIntLine`; the mid-level
  `bad`, `good`; the one-shot wrapper `driver`.

## Configuration rows

Every row is exercised with many randomized inputs (fixed seed, see
`tests/differential.rs` `SEED`), both `.so`s loaded via `libloading`, stdout
captured per call and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `printLine` | non-NULL, empty string (`""`, single NUL byte) | `cfg_c1_print_line_empty` | [x] |
| C2 | `printLine` | non-NULL, 1-byte string, every value `0x01..=0xFF` (all 255 non-NUL bytes) | `cfg_c2_print_line_single_byte_all_values` | [x] |
| C3 | `printLine` | non-NULL, random printable-ASCII strings, lengths 1..64 (randomized, 512 cases) | `cfg_c3_print_line_random_ascii` | [x] |
| C4 | `printLine` | non-NULL, random arbitrary non-NUL bytes `0x01..=0xFF` incl. non-UTF-8 sequences, lengths 1..64 (randomized, 512 cases) | `cfg_c4_print_line_random_arbitrary_bytes` | [x] |
| C5 | `printLine` | non-NULL, strings containing printf conversion specifiers (`%s %d %n %% %p %10000d`) | `cfg_c5_print_line_format_specifiers` | [x] |
| C6 | `printLine` | non-NULL, strings containing embedded `\n`, `\r`, `\t`, `\0`-adjacent and trailing whitespace | `cfg_c6_print_line_whitespace_shapes` | [x] |
| C7 | `printLine` | non-NULL, long buffers at buffering boundaries: 511, 512, 1023, 1024, 4095, 4096, 4097, 8192, 65537 bytes | `cfg_c7_print_line_long_boundaries` | [x] |
| C8 | `printLine` | NULL pointer (the one branch the C has) — valid input per the C, prints nothing | `cfg_c8_print_line_null_branch` | [x] |
| C9 | `printIntLine` | boundary values: `0`, `1`, `-1`, `9`, `10`, `-9`, `-10`, `99`, `100`, `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1` | `cfg_c9_print_int_line_boundaries` | [x] |
| C10 | `printIntLine` | random full-range `i32` values (randomized, 1024 cases) | `cfg_c10_print_int_line_random` | [x] |
| C11 | `printIntLine` | random small-magnitude values `-1000..=1000` (digit-count/sign transitions, randomized, 512 cases) | `cfg_c11_print_int_line_random_small` | [x] |
| C12 | `good` | single call, no arguments (must print `0\n2\n`) | `cfg_c12_good_single_call` | [x] |
| C13 | `bad` | single call, no arguments (CWE-482 defect preserved: must print `0\n0\n`) | `cfg_c13_bad_single_call` | [x] |
| C14 | `driver` | single call — the full composed pipeline (5 labels + `good` + `bad` output, in order) | `cfg_c14_driver_single_call` | [x] |
| C15 | `driver` | repeated calls (8x) in one process — statelessness of the composed pipeline | `cfg_c15_driver_repeated` | [x] |
| C16 | `good` + `bad` | interleaved sequence `good,bad,bad,good,good,bad` — mid-level composition without the wrapper | `cfg_c16_good_bad_interleaved` | [x] |
| C17 | all 5 | randomized interleaving of all five entry points with randomized arguments (256 steps, seeded) — cross-entry-point composed pipeline | `cfg_c17_random_interleaving_all_entry_points` | [x] |
| C18 | `printLine` + `printIntLine` | leaf functions called back-to-back so that the `puts`-lowered and `printf`-lowered paths share the same stdio stream (interleaving/buffering shape) | `cfg_c18_leaf_interleaving_puts_vs_printf` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds `add_library(driver SHARED ...)` only — there is
no `add_executable`, and `translation/Cargo.toml` has `crate-type =
["cdylib"]` with no `[[bin]]`. **No driver binary exists**, so the
"compare binary stdout" gate is not applicable. Row C14 covers the equivalent
end-to-end `driver()` output comparison.
