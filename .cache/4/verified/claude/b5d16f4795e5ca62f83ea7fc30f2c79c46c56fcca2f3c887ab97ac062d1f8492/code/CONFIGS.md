# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/include/driver.h` (public API) and the
`if` / `switch` / `#ifdef` branches actually present in `c_src/src/driver.c`.

## Axes the C code actually branches on

**Build-time axes: none.** `CMakeLists.txt` has no `option()`, no
`target_compile_definitions`, no conditional sources. The only preprocessor
conditional in the library is the `DRIVER_H_` include guard. `Cargo.toml` has no
`[features]`. So the configuration cross-product has **one** build config, and
all remaining axes are runtime.

**Runtime option/mode axis — 1 flag total:**

| flag | set via | state it toggles | C site |
|------|---------|------------------|--------|
| `useGood` | `driver(int useGood)` | selects `good()` (40-byte `alloca`) vs `bad()` (10-byte `alloca`) | `src/driver.c:75` |

**Public entry points — all 5, including the lowest-level ones.** `driver.h`
declares only `driver`, but `printLine`, `printIntLine`, `bad` and `good` all
have external linkage and are exported in `nm -D` (see `SYMBOLS.md`), so they are
part of the real public ABI and are driven **directly**, not only through the
`driver` convenience wrapper:

| entry point | level | inputs |
|-------------|-------|--------|
| `printLine` | lowest (leaf) | `const char *` |
| `printIntLine` | lowest (leaf) | `int` |
| `bad` | mid (calls `printIntLine`) | none |
| `good` | mid (calls `printIntLine`) | none |
| `driver` | top (calls `bad`/`good`) | `int` |

**Input-shape axes the code special-cases:**

- `printLine`: NULL vs non-NULL (`src/driver.c:32`); then length (0, 1, small,
  page-crossing, 1 MiB), byte content (ASCII, high/non-UTF-8 bytes 0x80–0xFF,
  embedded `\n`, `printf` format specifiers, all 255 non-NUL byte values).
- `printIntLine`: `%d` formatting shape — 0, positive, negative, digit-count
  boundaries (9/10 digits), `INT_MIN`/`INT_MAX`, random 32-bit values.
- `bad`/`good`: no inputs; the shape axis is *call multiplicity/order*, since
  each writes through an `alloca` region in a fresh frame — 1 call, many calls,
  and interleaved with the other entry points (this is where a stale-frame or
  shared-buffer translation bug would show up, invisible to single-call tests).

## Configuration-surface table

Cross-product of {entry point} × {input shape}, pruned to combinations the C
distinguishes. Every row is driven through **both** `.so`s via `dlsym` and
compared byte-for-byte on captured stdout. Rows marked *randomized* use ≥256
property-style inputs from a fixed-seed (`0x243F6A8885A308D3`) SplitMix64 PRNG.

| #  | entry point(s) | configuration (options set + input shape) | test | ok |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `printIntLine` | `0` — the value both `bad`/`good` actually print | `cfg_01_print_int_line_zero` | [x] |
| 2  | `printIntLine` | small positives `1..=9` (single digit) | `cfg_02_print_int_line_single_digit` | [x] |
| 3  | `printIntLine` | small negatives `-1..=-9` (sign + single digit) | `cfg_03_print_int_line_small_negative` | [x] |
| 4  | `printIntLine` | digit-count boundaries: `±9`, `±10`, `±99`, `±100`, `±999999999`, `±1000000000` | `cfg_04_print_int_line_digit_boundaries` | [x] |
| 5  | `printIntLine` | `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` (32-bit extremes) | `cfg_05_print_int_line_extremes` | [x] |
| 6  | `printIntLine` | *randomized*: 4096 uniform `i32` values, full 32-bit range | `cfg_06_print_int_line_random` | [x] |
| 7  | `printIntLine` | *randomized*: 1024 values in a long back-to-back run (accumulating stdout, no flush between) | `cfg_07_print_int_line_batch_run` | [x] |
| 8  | `printLine`    | non-NULL, length 0 (`""`) → passes guard, emits only `"\n"` | `cfg_08_print_line_empty` | [x] |
| 9  | `printLine`    | non-NULL, length 1, every value `0x01..=0xFF` (all 255 single non-NUL bytes) | `cfg_09_print_line_all_single_bytes` | [x] |
| 10 | `printLine`    | plain ASCII words, length 2..64 | `cfg_10_print_line_ascii` | [x] |
| 11 | `printLine`    | high / non-UTF-8 bytes (0x80–0xFF mixed with ASCII) — must pass through untouched | `cfg_11_print_line_non_utf8` | [x] |
| 12 | `printLine`    | embedded `\n`, `\r`, `\t` — C adds exactly one trailing `\n` regardless | `cfg_12_print_line_embedded_newlines` | [x] |
| 13 | `printLine`    | `printf` format specifiers as *data* (`%s`, `%d`, `%n`, `%%`, `%1000000d`) | `cfg_13_print_line_format_specifiers` | [x] |
| 14 | `printLine`    | *randomized*: 2048 strings, random length 0..=256, random bytes `0x01..=0xFF` | `cfg_14_print_line_random_bytes` | [x] |
| 15 | `printLine`    | long strings crossing stdio buffer/page sizes: 4095, 4096, 4097, 8192, 65536, 1 MiB | `cfg_15_print_line_long_buffer_boundaries` | [x] |
| 16 | `bad`          | single call, no options — 10-byte `alloca`, writes 10 `int`s, prints `data[0]` | `cfg_16_bad_single` | [x] |
| 17 | `good`         | single call, no options — 40-byte `alloca`, writes 10 `int`s, prints `data[0]` | `cfg_17_good_single` | [x] |
| 18 | `bad`          | 256 repeated calls (fresh frame each time; catches stale/shared backing store) | `cfg_18_bad_repeated` | [x] |
| 19 | `good`         | 256 repeated calls | `cfg_19_good_repeated` | [x] |
| 20 | `driver`       | `useGood = 0` → `bad()` path | `cfg_20_driver_false` | [x] |
| 21 | `driver`       | `useGood = 1` → `good()` path | `cfg_21_driver_true` | [x] |
| 22 | `driver`       | *randomized*: 1024 uniform `i32` flags (mostly nonzero → `good`, exact 0 → `bad`) | `cfg_22_driver_random_flag` | [x] |
| 23 | `driver`       | *randomized*: alternating 0/nonzero sequence, 512 calls, one capture (mode switching under accumulation) | `cfg_23_driver_alternating` | [x] |
| 24 | all 5 mixed    | *randomized*: 1024-step interleaved program over `{printLine, printIntLine, bad, good, driver}` in one capture — the composed pipeline | `cfg_24_interleaved_all_entry_points` | [x] |

### Why rows 7, 18, 19, 23 and 24 exist

Per-call tests flush and compare after every single call, which hides two whole
bug classes: (a) state that persists between calls inside one `.so` (a `static`
backing buffer instead of a fresh frame), and (b) divergence that only appears
once output accumulates in the stdio buffer without an intervening flush. Rows 7,
18, 19, 23 and 24 therefore drive **long sequences of calls inside a single
capture window** and compare the whole accumulated byte stream at the end. Row 24
in particular runs a randomized interleaving of all five entry points — the way
a real consumer composes them — rather than exercising each wrapper in isolation.
