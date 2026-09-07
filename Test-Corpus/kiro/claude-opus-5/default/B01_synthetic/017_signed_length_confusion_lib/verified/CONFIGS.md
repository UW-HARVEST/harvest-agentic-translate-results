# CONFIGS.md — configuration surface table (Phase B)

Derived mechanically from `c_src/src/driver.c`, `c_src/include/driver.h`,
`c_src/CMakeLists.txt` and `translation/Cargo.toml` — the same way `ERRORS.md`
is derived. This is the table for **valid** inputs.

## Axis enumeration (mechanical)

### Axis 1 — runtime options / modes / flags

**Empty.** Grep for `option`, `add_definitions`, `target_compile*` in
`CMakeLists.txt` returns nothing; grep for `#if`, `#ifdef`, `#else`, `switch`
in the C sources returns only the `DRIVER_H_` include guard. The library holds
**no global or static state**: both `source` and `dest` are function-local
automatics re-initialised on every `driver` call, so no call can influence a
later one. There is no init/config/teardown function, no context struct, no
setter, and no environment variable read. Consequently there is nothing to
"set up" or "apply" before an operation — an entry point plus its arguments
*is* the entire configuration, and every row below is a complete end-to-end
operation.

### Axis 2 — compile-time feature combinations

**Exactly one: the default.** `translation/Cargo.toml` has no `[features]`
table (grep for `feature` and `cfg(` across `Cargo.toml` and `src/lib.rs`
returns nothing), so the set of feature combinations is the single empty set.
`--no-default-features` and the default build are therefore the same
compilation. Verified in Phase D by enumerating features from `Cargo.toml`
and looping `cargo check` / `cargo test` over the enumerated set.

### Axis 3 — public entry points (the FULL set, lowest level included)

Both exported symbols are covered, including `printLine`, which is **not** in
the public header but has external linkage and is the lower-level primitive
that `driver` composes on top of. It is driven **directly** below (rows 1-8),
not only indirectly through `driver` (rows 9-16), because a bug in the
composed pipeline is invisible to a per-wrapper test and vice-versa.

| entry point | signature | level |
|-------------|-----------|-------|
| `printLine` | `void printLine(const char *line)` | low-level primitive (not in the header, exported by the `.so`) |
| `driver`    | `void driver(int data)` | composed operation: `memset` → `strncpy` → `dest[data]=0` → `printLine` |

### Axis 4 — input shapes the C actually special-cases

`printLine`'s only branch is `line != NULL`; past that the byte string goes
straight to `printf("%s\n", …)`, so the shapes that matter are the ones
`printf`/`%s` distinguishes: empty vs one vs many bytes, and byte *values*
(the format string is a fixed `"%s\n"`, so `%` inside the *argument* is data,
never a directive — a real behaviour worth pinning). `driver`'s only branch is
`data < 100`, and inside it `data` selects both `strncpy`'s length and the
`dest[data]` index, which makes behaviour value-dependent across the whole
`[0, 99]` band. `strlen(source) == 99` is the additional shape boundary where
`strncpy` switches from "truncating copy" to "copy then NUL-pad".

## Configuration table

One row per meaningful combination of the axes above (their cross-product,
pruned to combinations the C treats differently). Each row is run against
**both** `.so`s through their exported symbols and stdout is compared
byte-for-byte. Rows marked *randomized* use many property-style inputs from a
fixed-seed PRNG (seed `0x243F6A8885A308D3`, a SplitMix64 stream), not one
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` | empty string `""` (0 payload bytes, immediate NUL) | `cfg_01_printline_empty` | [x] |
| 2 | `printLine` | single byte, exhaustively every value `0x01..=0xFF` (one-byte strings; `0x00` is row 1) | `cfg_02_printline_single_byte_all` | [x] |
| 3 | `printLine` | many bytes: *randomized* length `2..=512`, *randomized* payload bytes drawn from `0x01..=0xFF` (NUL-free, so the whole payload prints) | `cfg_03_printline_random_many` | [x] |
| 4 | `printLine` | payload containing `printf` format directives as **data** (`%s`, `%d`, `%n`, `%%`, `%p`) — must be emitted literally, since the format string is the fixed `"%s\n"` | `cfg_04_printline_format_directives` | [x] |
| 5 | `printLine` | payload with an **embedded NUL** in the middle — `%s` must stop at the first NUL and drop the tail | `cfg_05_printline_embedded_nul` | [x] |
| 6 | `printLine` | payload of high / non-ASCII bytes (`0x80..=0xFF`) and embedded newlines / `\t` / `\r`, *randomized* | `cfg_06_printline_high_bytes_and_newlines` | [x] |
| 7 | `printLine` | long payload: 4096 and 65536 bytes (crosses libc stdout buffer sizes, so it exercises the flush path) | `cfg_07_printline_long` | [x] |
| 8 | `printLine` | repeated calls in one capture window (accumulating unflushed stdout state across N=64 *randomized* calls) | `cfg_08_printline_repeated_calls` | [x] |
| 9 | `driver` | `data == 0`: guard taken, `strncpy` length 0, `dest[0]='\0'` — copy path with an empty result | `cfg_09_driver_zero` | [x] |
| 10 | `driver` | `data == 1`: minimal non-empty copy | `cfg_10_driver_one` | [x] |
| 11 | `driver` | `data ∈ [2, 98]`: interior of the accepted band, *randomized* over many draws (value-dependent length **and** value-dependent `dest[data]` index) | `cfg_11_driver_interior_randomized` | [x] |
| 12 | `driver` | `data == 98`: last value strictly below `strlen(source)` | `cfg_12_driver_98` | [x] |
| 13 | `driver` | `data == 99` = `strlen(source)`: `strncpy` copies the full source with **no** NUL of its own; termination comes solely from `dest[99]='\0'` | `cfg_13_driver_99` | [x] |
| 14 | `driver` | `data ∈ [0, 99]` **exhaustively** (all 100 accepted values, the complete valid band) | `cfg_14_driver_accepted_band_exhaustive` | [x] |
| 15 | `driver` | `data >= 100`: guard not taken, `dest` printed as the untouched `""`. *Randomized* over `[100, INT_MAX]` plus the fixed endpoints `100`, `101`, `INT_MAX` | `cfg_15_driver_rejected_band_randomized` | [x] |
| 16 | `driver` | repeated `driver` calls with *randomized* `data` in one capture window, interleaved with direct `printLine` calls — checks that the composed pipeline leaves no state behind and that the two entry points compose identically in C and Rust | `cfg_16_driver_printline_interleaved` | [x] |

Rows 9-16 cover `driver` only for `data >= 0`. Negative `data` passes the C's
guard and crashes (the C never checks the lower bound); it is not a valid
configuration and is handled as rows 8-11 of `ERRORS.md`.

## Binary executable

**None.** `c_src/CMakeLists.txt` contains no `add_executable` (only
`add_library(driver SHARED …)`), and `translation/` has no `[[bin]]` target and
no `src/main.rs`. The "compare C and Rust binary stdout" requirement is
therefore vacuous for this project; the stdout comparison is instead done at
the `.so` level, which is where all output originates.

## Verification result

All 16 rows pass. Each row has a named test in
`translation/tests/differential.rs` that drives both `.so`s through
`dlopen`/`dlsym` and compares stdout byte-for-byte plus the termination status.
Randomized rows draw from the fixed SplitMix64 seed above, so a failure is
reproducible; per-row input counts:

| row | inputs exercised |
|-----|------------------|
| 2 | 255 (exhaustive single-byte space) |
| 3 | 400 randomized payloads, length 2..512 |
| 5 | 3 fixed + 150 randomized embedded-NUL payloads |
| 6 | 4 fixed + 300 randomized high-byte/control payloads |
| 7 | 5 lengths incl. 4095/4096/4097 and 65536 |
| 8 | 8 rounds × 64 calls in one process |
| 11 | 500 randomized `data` in `[2, 98]` |
| 14 | 100 (exhaustive `[0, 99]`) |
| 15 | 11 fixed + 500 randomized `data` in `[100, INT_MAX]` |
| 16 | 10 rounds × 40 interleaved `driver`/`printLine` calls |

Rows 1, 4, 5, 7, 10, 14 and 15 additionally assert the **exact C ground-truth
bytes** (not merely C/Rust equality), so a mutually-wrong result cannot pass.

## Harness note

Observing this library means capturing fd 1, which is process-global. An
in-process `dup2` capture was tried first and produced **false divergences**: the
Rust libtest harness writes its own progress lines (`test <name> ... ok`) to fd 1
from another thread, and those bytes landed inside another thread's capture
window. The harness was therefore rewritten so that every run happens in a
re-exec'd child process whose fd 1 is a private file, one spawn per
(row, implementation). This also makes the fatal negative-`data` rows of
`ERRORS.md` testable, and makes the suite pass identically in parallel and
single-threaded mode.

## Configuration coverage

`translation/verify.sh` enumerates feature combinations from `Cargo.toml`
mechanically and loops `cargo check` / `cargo test` over them. It confirms the
`[features]` table is absent (corroborated by `cargo metadata`, which reports
`features: {}`), so the complete set of combinations is the default build and
`--no-default-features` — which are the same compilation. The suite is run for
both, against both the debug and the release Rust `.so`, in parallel and
single-threaded: 8 configurations × 31 tests, all passing.
