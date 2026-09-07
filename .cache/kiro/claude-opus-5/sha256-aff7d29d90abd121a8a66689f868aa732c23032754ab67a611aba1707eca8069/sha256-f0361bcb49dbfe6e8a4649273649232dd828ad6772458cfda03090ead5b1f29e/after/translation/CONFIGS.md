# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**Runtime options / modes / flags:** none. The library has no init function, no
context/handle struct, no global mutable state, no `setvbuf`/mode selection, no
environment-variable lookup, and no `#ifdef` other than the header include
guard. Grep for branches over the whole library yields a single `if`
(`driver.c:31`, the `printLine` null check, an *error* axis → `ERRORS.md`).

**Public entry points (all five exported symbols, lowest level first):**

| level | entry point | signature |
|-------|-------------|-----------|
| 0 (lowest) | `printLine` | `void printLine(const char *)` |
| 0 (lowest) | `printIntLine` | `void printIntLine(int)` |
| 1 (composes level 0) | `good` | `void good(void)` |
| 1 (composes level 0) | `bad` | `void bad(void)` |
| 2 (composes levels 0+1) | `driver` | `void driver(void)` |

Only `driver` is declared in the public header; the other four are exported by
the `.so` and are therefore part of the real ABI surface, so they are all
driven directly rather than only through `driver`.

**Input shapes the code distinguishes:**

* `printLine`: string length (0, 1, small, page-sized, > 1 MiB); byte content
  (ASCII, embedded `printf` directives `%s`/`%d`/`%n`, control bytes `\t \r \x0b`,
  non-UTF-8 `0x80..=0xFF`, all byte values except `0x00`); trailing/leading
  newline already present in the payload.
* `printIntLine`: sign and magnitude (`0`, `±1`, single/multi digit, `INT_MIN`,
  `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`, random 32-bit values) — this is what
  selects `printf`'s `%d` digit/sign path.
* `bad` / `good` / `driver`: no inputs; the axis is *call multiplicity and
  order*, because the only observable is the accumulated `stdout` byte stream
  (`good` assigns `intSum`, `bad` discards `intOne + intTwo` — the two produce
  different digits from otherwise identical code, so ordering matters).

**Cargo feature axis:** `translation/Cargo.toml` has no `[features]` table →
configurations are `--all-features` ≡ default ≡ `--no-default-features`. All
rows are run under each of those invocations in Phase D.

## Rows (pruned cross-product of the axes above)

Each row is exercised with many randomized inputs from a fixed-seed PCG32
(`seed = 0x5EED_1234_ABCD_EF01`) where the row has a value axis, and both
libraries' `stdout` is captured and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` | length 0 and 1; every single byte value `0x01..=0xFF` as a 1-byte payload | `cfg_row1_print_line_tiny_all_bytes` | [x] |
| 2 | `printLine` | random ASCII-printable payloads, length 2..=64, 512 iterations | `cfg_row2_print_line_random_ascii` | [x] |
| 3 | `printLine` | random arbitrary-byte payloads (`0x01..=0xFF`, non-UTF-8 included), length 1..=256, 512 iterations | `cfg_row3_print_line_random_bytes` | [x] |
| 4 | `printLine` | payloads containing `printf` format directives (`%s %d %n %%  %1000000d`) — must be copied verbatim, never interpreted | `cfg_row4_print_line_format_directives` | [x] |
| 5 | `printLine` | payloads containing embedded newlines / `\r` / `\t` / other control bytes, so the emitted stream has interior line breaks | `cfg_row5_print_line_control_bytes` | [x] |
| 6 | `printLine` | large payloads: 4095, 4096, 4097, 8192, 65535, 65536, 1 048 576 bytes (straddling typical stdio buffer sizes) | `cfg_row6_print_line_large` | [x] |
| 7 | `printIntLine` | boundary integers: `INT_MIN`, `INT_MIN+1`, `-100000`, `-10`, `-1`, `0`, `1`, `9`, `10`, `99`, `100`, `INT_MAX-1`, `INT_MAX` | `cfg_row7_print_int_line_boundaries` | [x] |
| 8 | `printIntLine` | 4096 uniformly random `i32` values (full 32-bit range) | `cfg_row8_print_int_line_random` | [x] |
| 9 | `printIntLine` | random values restricted to small magnitudes (`-999..=999`), 1024 iterations — exercises the short-digit path | `cfg_row9_print_int_line_small` | [x] |
| 10 | `good` | single call, no input | `cfg_row10_good_single` | [x] |
| 11 | `bad` | single call, no input (must reproduce the discarded-`intOne + intTwo` bug: two `0` lines) | `cfg_row11_bad_single` | [x] |
| 12 | `driver` | single call, no input — the full end-to-end pipeline (`printLine`+`good`+`printLine`+`printLine`+`bad`+`printLine`) | `cfg_row12_driver_single` | [x] |
| 13 | `good`, `bad` | both called repeatedly in a fixed-seed random interleaving, 256 calls, in one capture (composed pipeline, shared `stdout` stream) | `cfg_row13_good_bad_interleaved` | [x] |
| 14 | `driver` | called 8 times back-to-back in a single capture (idempotence / no cross-call state) | `cfg_row14_driver_repeated` | [x] |
| 15 | all five | fixed-seed random interleaving of `printLine`(random payload), `printIntLine`(random `i32`), `good`, `bad`, `driver` — 512 calls in one capture; the full cross-product of entry points in one stream | `cfg_row15_all_entry_points_interleaved` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(driver SHARED src/driver.c)`
— there is no `add_executable`, and `driver.c` has no `main`. The Rust
`Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` and has no
`src/main.rs` / `[[bin]]`. **No binary is built by either side**, so the
"compare C and Rust binary stdout" item of the completion gate is vacuous.
The equivalent coverage is row 12 (`driver`, the top-level entry point that a
driver `main` would call) plus the `driver_stdout_parity` end-to-end test.
