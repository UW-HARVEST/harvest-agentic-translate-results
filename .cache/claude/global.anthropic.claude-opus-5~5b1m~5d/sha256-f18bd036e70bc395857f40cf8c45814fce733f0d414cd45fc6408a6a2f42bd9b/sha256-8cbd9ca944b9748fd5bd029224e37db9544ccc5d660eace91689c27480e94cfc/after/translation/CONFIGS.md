# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branch structure of `c_src/src/driver.c` and the
public surface in `c_src/include/driver.h`.

## Axes the C code actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. entry point | `driver` (top-level dispatcher), `bad`, `good` (mid-level, external linkage), `printIntLine`, `printLine` (lowest-level primitives) | all 5 have external linkage in `driver.c` |
| B. `driver(useGood)` truthiness | `0` → `bad()`; any non-zero → `good()` | `if (useGood)` at `driver.c:71` |
| C. `printLine` pointer state | `NULL` → nothing; non-NULL → `printf("%s\n", line)` | `if (line != NULL)` at `driver.c:31` |
| D. `printLine` string shape | empty, 1 byte, many bytes, > stdio buffer (oversized), embedded `%` specifiers, high/non-ASCII bytes, trailing/leading whitespace, embedded newlines | `%s` semantics — the code special-cases nothing else, so shapes are the value-dependent axis |
| E. `printIntLine` value shape | `0`, positive, negative, `INT_MIN`, `INT_MAX`, values needing 1..11 output chars (sign + digits) | `%d` semantics |
| F. allocation size in `bad` vs `good` | `alloca(10)` (10 bytes, 40 written — CWE-806) vs `alloca(10*sizeof(int))` (40 bytes, 40 written) | `driver.c:43` vs `driver.c:58` |
| G. call sequencing / stdio interleaving | single call vs repeated calls vs mixed sequences of different entry points against the same shared `stdout` buffer | `printf` is the only output mechanism; buffering is shared state |
| H. feature set | only the default/empty set (`Cargo.toml` has no `[features]`) | `translation/Cargo.toml` |

Note: `bad()` and `good()` take no arguments and have no observable inputs, so
their configuration axes are (F) and (G) only — but they must be driven
*directly* via `dlsym`, not only through `driver`, because they are exported.

## Configuration rows (cross-product, pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `printIntLine` | randomized full-range `i32` values (1000 seeded values), one call per capture | `cfg_c1_print_int_line_random_full_range` | [x] |
| C2 | `printIntLine` | boundary/shape values: `0, 1, -1, 9, 10, -9, -10, 99, 100, INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, ±10^k` (all output widths 1..11) | `cfg_c2_print_int_line_boundaries` | [x] |
| C3 | `printIntLine` | many calls in one capture (repeated invocation, shared stdout buffer) — 500 randomized values | `cfg_c3_print_int_line_repeated` | [x] |
| C4 | `printLine` | non-NULL, empty string `""` | `cfg_c4_print_line_empty` | [x] |
| C5 | `printLine` | non-NULL, single byte, randomized over all 1..255 byte values | `cfg_c5_print_line_single_byte` | [x] |
| C6 | `printLine` | non-NULL, randomized ASCII strings, randomized length 0..64 (200 cases) | `cfg_c6_print_line_random_ascii` | [x] |
| C7 | `printLine` | non-NULL, randomized arbitrary non-NUL bytes 0x01..0xFF, length 0..256 (200 cases) — high bytes / invalid UTF-8 | `cfg_c7_print_line_random_bytes` | [x] |
| C8 | `printLine` | non-NULL, strings containing `printf` specifiers (`%s`, `%d`, `%n`, `%%`, `%1000000d`) | `cfg_c8_print_line_format_specifiers` | [x] |
| C9 | `printLine` | non-NULL, oversized strings larger than the stdio buffer: 4096, 4097, 8192, 65536 bytes | `cfg_c9_print_line_oversized` | [x] |
| C10 | `printLine` | non-NULL, embedded newlines / whitespace-only / leading+trailing spaces | `cfg_c10_print_line_newlines_ws` | [x] |
| C11 | `printLine` | NULL pointer (valid, documented configuration — the guard) | `cfg_c11_print_line_null` | [x] |
| C12 | `printLine` | many calls in one capture, mixed NULL and non-NULL randomized strings (300 cases) | `cfg_c12_print_line_mixed_repeated` | [x] |
| C13 | `bad` | direct low-level call, single invocation (under-allocated `alloca(10)`, axis F) | `cfg_c13_bad_direct` | [x] |
| C14 | `bad` | direct low-level call, 100 repeated invocations (stack-slack reuse, axis G) | `cfg_c14_bad_repeated` | [x] |
| C15 | `good` | direct low-level call, single invocation (`alloca(10*sizeof(int))`, axis F) | `cfg_c15_good_direct` | [x] |
| C16 | `good` | direct low-level call, 100 repeated invocations | `cfg_c16_good_repeated` | [x] |
| C17 | `driver` | `useGood == 0` → `bad()` path | `cfg_c17_driver_false` | [x] |
| C18 | `driver` | `useGood == 1` → `good()` path | `cfg_c18_driver_true` | [x] |
| C19 | `driver` | randomized non-zero `i32` values (500 seeded, incl. negative, `INT_MIN`, `INT_MAX`, high-bit-only, low-byte-zero like `0x100`) → all must take `good()` | `cfg_c19_driver_random_nonzero` | [x] |
| C20 | `driver` | randomized sequence of alternating `useGood` values in one capture (axis B × G, 300 calls) | `cfg_c20_driver_random_sequence` | [x] |
| C21 | all five | randomized interleaving of *all* entry points in one capture (200 ops: `driver`/`bad`/`good`/`printIntLine`/`printLine`), exercising the composed pipeline and shared stdio state | `cfg_c21_all_entry_points_interleaved` | [x] |
| C22 | feature matrix | every row above re-run under every Cargo feature combination (only the default/empty set exists — axis H) | `scripts/check_features.sh` | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)` — there is
no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **No binary driver exists**, so the
"compare binary stdout" gate is not applicable. Stdout is nonetheless compared
byte-for-byte in every row above, via `dup2` redirection of the shared C
`stdout` around each FFI call.
