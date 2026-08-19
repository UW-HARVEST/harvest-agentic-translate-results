# CONFIGS.md — Configuration surface (valid inputs) of the C library

## Axes, derived mechanically from the C source

**Build-time configuration axes: none.**
`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` defines no
`target_compile_definitions` / options — it compiles exactly `src/driver.c`. The only
preprocessor conditional in the C is the `DRIVER_H_` include guard. Therefore there is
exactly **one** build configuration: `--no-default-features` == default == full build.

**Runtime option/mode/flag axes: none.** The public API takes no option struct, no
handle, no mode enum, no init call. There is no global/static state in the C
(`grep -n static c_src/src/driver.c` ⇒ no matches), so no state any flag could toggle.

**Full set of public entry points (all 5 exported symbols, incl. the lowest-level
ones — not just the documented `driver` wrapper):**

| entry point | header-declared | level |
|---|---|---|
| `printLine(const char*)` | no (external linkage, exported) | lowest — direct `printf("%s\n", …)`, the only branch in the library |
| `printIntLine(int)` | no (external linkage, exported) | lowest — direct `printf("%d\n", …)` |
| `good()` | no (external linkage, exported) | mid — composes `printIntLine` ×2 |
| `bad()` | no (external linkage, exported) | mid — composes `printIntLine` ×2 |
| `driver()` | **yes** (`include/driver.h`) | top — composes `printLine` ×4, `good`, `bad` |

**Input-shape axes the C actually distinguishes:**

* `printLine`: the `line != NULL` branch (NULL vs non-NULL), then the byte-string
  shape that `%s` consumes: length (0 / 1 / many / past-stdio-buffer), byte domain
  (printable ASCII / control bytes / non-UTF-8 high bytes / all 255 values),
  presence of `printf` format specifiers, presence of embedded NUL, whitespace and
  newline content.
* `printIntLine`: the `int` value shape that `%d` consumes: sign (negative / zero /
  positive), decimal digit count 1..10 (power-of-ten boundaries change the output
  width), and the extremes `INT_MIN` / `INT_MAX`.
* `good` / `bad` / `driver`: no parameters — the distinguishing axis is the *call
  sequence* (single call, repeated calls, interleaving with the low-level entry
  points), which exercises the composed pipeline and the shared `stdout` buffer.

**Observable output.** All five functions return `void`; their entire observable
effect is the byte sequence written to the C runtime's `stdout`. Every row below
captures `stdout` (via `dup2` onto a temp file, then `fflush(NULL)`) around a call
into the C `.so` and around the same call into the Rust `.so`, and asserts the two
byte buffers are **identical byte-for-byte**. Both `.so`s are loaded with
`libloading`; Rust functions are only ever reached through the loaded `.so`'s
exported symbols, never called directly.

**Randomization.** Rows marked *randomized* draw many inputs from a fixed-seed
xorshift64\* PRNG (seed noted per row, reproducible), rather than one hand-picked
value.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `printLine` | NULL pointer (the false side of the library's only branch) | `cfg_01_print_line_null` | [x] |
| 2 | `printLine` | length 0 — empty C string `""` | `cfg_02_print_line_empty` | [x] |
| 3 | `printLine` | length 1, *randomized* over printable ASCII (seed 0x1234\_5678, 256 draws) | `cfg_03_print_line_len1_random` | [x] |
| 4 | `printLine` | length 2..=255, *randomized* printable-ASCII payloads (seed 0xA5A5\_0001, 512 draws) | `cfg_04_print_line_short_random_ascii` | [x] |
| 5 | `printLine` | length 1..=255, *randomized* over the **full non-NUL byte domain** `0x01..=0xFF` (invalid UTF-8 included; seed 0xDEAD\_BEEF, 512 draws) | `cfg_05_print_line_random_bytes` | [x] |
| 6 | `printLine` | payload built from `printf` format specifiers (`%s %d %n %p %% %1000000d`), *randomized* mixes (seed 0x0F0F\_1111, 256 draws) | `cfg_06_print_line_format_specifiers` | [x] |
| 7 | `printLine` | payload containing embedded newlines / `\r` / `\t` / vertical tab / form feed, *randomized* (seed 0xC0FF\_EE01, 256 draws) | `cfg_07_print_line_whitespace` | [x] |
| 8 | `printLine` | payload with an embedded NUL at a *randomized* position (C string truncates there; seed 0x5EED\_0008, 256 draws) | `cfg_08_print_line_embedded_nul` | [x] |
| 9 | `printLine` | lengths straddling the stdio buffer boundary: 1023,1024,1025,4095,4096,4097,8191,8192,8193,65535,65536,65537 (randomized content, seed 0xB00B\_0009) | `cfg_09_print_line_buffer_boundaries` | [x] |
| 10 | `printLine` | oversized: 1 MiB payload | `cfg_10_print_line_oversized` | [x] |
| 11 | `printIntLine` | `0` | `cfg_11_print_int_line_zero` | [x] |
| 12 | `printIntLine` | `±1`, `±2` — smallest magnitudes, both signs | `cfg_12_print_int_line_small` | [x] |
| 13 | `printIntLine` | every decimal-width boundary: `±(10^k)` and `±(10^k − 1)` for k=1..9 (digit-count transitions) | `cfg_13_print_int_line_decimal_boundaries` | [x] |
| 14 | `printIntLine` | `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX−1` | `cfg_14_print_int_line_extremes` | [x] |
| 15 | `printIntLine` | *randomized* full-range `i32` (seed 0x1357\_9BDF, 4096 draws) | `cfg_15_print_int_line_random_full_range` | [x] |
| 16 | `printIntLine` | *randomized* negative-only and positive-only sub-ranges, and all 2^k / −2^k for k=0..31 (seed 0x2468\_ACE0) | `cfg_16_print_int_line_random_signed_ranges` | [x] |
| 17 | `good` | single call — full sequence (`0`, then `1+1=2`) | `cfg_17_good_single` | [x] |
| 18 | `bad` | single call — full sequence (`0`, then the *discarded* `intOne+intTwo`, so `0` again) | `cfg_18_bad_single` | [x] |
| 19 | `driver` | single call — the whole composed pipeline (`printLine`×4 + `good` + `bad`) | `cfg_19_driver_single` | [x] |
| 20 | `good` / `bad` / `driver` | repeated invocation ×64 each in one capture — verifies no hidden accumulated state | `cfg_20_no_arg_fns_repeated` | [x] |
| 21 | all 5 entry points | *randomized interleaving* of `driver`/`good`/`bad`/`printLine`/`printIntLine` into a single `stdout` capture, 64 sequences × up to 32 calls (seed 0xFEED\_FACE) — exercises the composed pipeline and shared stream ordering/buffering, which per-wrapper tests cannot see | `cfg_21_random_interleaved_sequences` | [x] |
| 22 | `printLine` ⊗ `printIntLine` | cross-product of the two low-level entry points back-to-back with *randomized* pairs (seed 0x0BAD\_C0DE, 512 draws), no flush between — checks the two format paths cannot corrupt each other's buffered output | `cfg_22_low_level_cross_product` | [x] |

All 22 rows pass across their randomized inputs (see `tests/differential.rs`).
