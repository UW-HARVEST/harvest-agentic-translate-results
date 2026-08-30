# CONFIGS.md — Phase B configuration-surface table

## Axes, derived mechanically from the C source

**Public entry points** (`nm -D --defined-only libdriver.so`, cross-checked
against `include/driver.h`):

| entry point | level | declared in header? |
|---|---|---|
| `printLine(const char*)` | **lowest level** — the leaf that does the I/O | no (exported but undeclared) |
| `bad(void)`   | mid level — composes `helperBad` + `printLine` | no (exported but undeclared) |
| `good(void)`  | mid level — composes `helperGood1` + `printLine` | no (exported but undeclared) |
| `driver(int)` | **top-level convenience wrapper** | yes — the only header declaration |

Note that `driver` is the *only* function in `driver.h`, yet three lower-level
symbols are exported. Testing only `driver` would leave `printLine`'s entire
input space untested, so every row below that can be driven through a
lower-level entry point is driven that way *in addition to* through `driver`.

**Runtime options / modes.** The library has exactly one runtime option: the
`int useGood` argument of `driver`, consumed by `if (useGood)` at `driver.c:60`.
It toggles between two states:

| option value | branch taken | callee | inner data source |
|---|---|---|---|
| `useGood == 0` | `else` | `bad()`   | `helperBad()` — automatic array, dangling → NULL in the reference build |
| `useGood != 0` | `then` | `good()`  | `helperGood1()` — `static` array in `.data`, valid |

There are no other flags, no global configuration state, no `#ifdef`-selected
behaviour (the only `#if` is the `driver.h` include guard), and no byte-order or
element-type axes (the sole data type is `char`).

**Input shapes the code distinguishes.** `printLine` branches on exactly one
property of its argument — NULL vs non-NULL — and then hands the pointer to
stdio, which walks it to the first `\0`. The shape axes that therefore matter
are: NULL-ness, string length (0 / 1 / small / stdio-buffer-boundary / huge),
byte content (ASCII / control / high-bit / `printf` metacharacters), and
pointer position (start of buffer vs interior). Call multiplicity matters as a
shape too, because `good`'s buffer has static storage duration and must survive
repeated calls unchanged.

## Table — one row per combination the C actually treats differently

Each row is exercised against **both** `.so`s through `libloading`, comparing
captured `stdout` byte-for-byte. Rows marked *randomized* run many seeded
pseudo-random inputs (LCG, fixed seed `0x2545F4914F6CDD1D`), not one hand-picked
value.

| # | entry point(s) | configuration (options set + input shape) | randomized | test | [x] |
|---|----------------|-------------------------------------------|---|------|-----|
| C1 | `printLine` | non-NULL, length 1, single ASCII byte — all 255 non-NUL byte values swept | exhaustive sweep | `c1_print_line_single_byte_all_values` | [x] |
| C2 | `printLine` | non-NULL, length 0 (empty string) | no | `c2_print_line_empty` | [x] |
| C3 | `printLine` | non-NULL, "many": lengths 2..64, printable-ASCII content | yes (200 cases) | `c3_print_line_short_ascii_random` | [x] |
| C4 | `printLine` | non-NULL, full byte range 0x01..0xFF, lengths 0..512 (invalid UTF-8 included) | yes (2000 cases) | `c4_print_line_arbitrary_bytes_random` | [x] |
| C5 | `printLine` | non-NULL, `printf` metacharacter payloads (`%s`, `%n`, `%%`, `%1$s`, `%99999d`) mixed into random data | yes (300 cases) | `c5_print_line_format_metachars_random` | [x] |
| C6 | `printLine` | non-NULL, control/whitespace payloads: embedded `\n \r \t \x0b \x0c \x1b` | yes (300 cases) | `c6_print_line_control_bytes_random` | [x] |
| C7 | `printLine` | non-NULL, boundary lengths around stdio buffering: 1, 1023, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193, 65535, 65536, 65537, 1 MiB | no (all boundaries) | `c7_print_line_buffer_boundary_lengths` | [x] |
| C8 | `printLine` | non-NULL **interior** pointer `buf.add(k)` into a larger allocation, random `k` | yes (200 cases) | `c8_print_line_interior_pointer_random` | [x] |
| C9 | `printLine` | non-NULL, buffer with trailing garbage *after* the NUL (must stop at the NUL) | yes (200 cases) | `c9_print_line_stops_at_nul` | [x] |
| C10 | `printLine` | NULL argument (the guard's other side — valid input, documented rejection) | no | `c10_print_line_null` | [x] |
| C11 | `good` | option: n/a; shape: single call, static `.data` source | no | `c11_good_single_call` | [x] |
| C12 | `good` | shape: "many" — 256 consecutive calls (static buffer must be stable) | no | `c12_good_repeated` | [x] |
| C13 | `bad` | option: n/a; shape: single call, dangling automatic source | no | `c13_bad_single_call` | [x] |
| C14 | `bad` | shape: "many" — 256 consecutive calls | no | `c14_bad_repeated` | [x] |
| C15 | `driver` | `useGood == 0` → `else` → `bad` | no | `c15_driver_zero` | [x] |
| C16 | `driver` | `useGood == 1` → `then` → `good` | no | `c16_driver_one` | [x] |
| C17 | `driver` | `useGood` = arbitrary non-zero **positive** `int` | yes (500 values) | `c17_driver_random_nonzero_positive` | [x] |
| C18 | `driver` | `useGood` = arbitrary non-zero **negative** `int` (C truthiness: still `good`) | yes (500 values) | `c18_driver_random_negative` | [x] |
| C19 | `driver` | `useGood` = extremal ints: `i32::MIN`, `i32::MIN+1`, `-1`, `0`, `1`, `i32::MAX-1`, `i32::MAX`, and values whose low 32 bits are 0 but which differ in the upper half of the passed register (`0x1_0000_0000` truncated to `0`) | no (all extremes) | `c19_driver_extremal_ints` | [x] |
| C20 | `driver` | shape: "many" — random sequence of 1000 mixed zero/non-zero calls, output concatenated (interleaving of both branches through one entry point) | yes (1000-call sequence) | `c20_driver_random_sequence` | [x] |
| C21 | `printLine` + `good` + `bad` + `driver` | **composed pipeline**: random interleaving of all four entry points in one capture, so buffered-output ordering across entry points is compared, not just per-call output | yes (600-op program) | `c21_mixed_entry_point_program` | [x] |
| C22 | `printLine` | ordering/state: `printLine` called with the caller's own buffer immediately before and after `good()`, checking the `.data` static is not aliased or clobbered | yes (200 cases) | `c22_print_line_around_good` | [x] |

## Coverage argument

* Both sides of both `if`s in the library are covered: `driver.c:30` true (C1–C9,
  C11–C12) and false (C10, C13–C15); `driver.c:60` true (C16–C19) and false (C15).
* All four exported entry points are called directly through `dlsym`, including
  the three that `driver.h` does not declare.
* `helperBad` and `helperGood1` are `static` and unreachable except via `bad`,
  `good`, `driver`; rows C11–C21 reach them.
* Feature combinations: `Cargo.toml` has no `[features]`, so the default build is
  the only configuration (see `SYMBOLS.md` / `FEATURES.md`).

## Cross-references

Rows C11-C21 are additionally re-run against the C source rebuilt at -O0, -O1,
-O2, -O3 and -Os by `tests/optlevels.rs`. Row-by-row sensitivity (which rows
catch which injected defect) is recorded in `FEATURES.md`; the overall gate is in
`VERIFICATION.md`.
