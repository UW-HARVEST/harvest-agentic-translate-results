# CONFIGS.md — Configuration / valid-input surface table

Derived **mechanically** from `c_src/src/driver.c`, `c_src/include/driver.h` and
`c_src/CMakeLists.txt`.

## Axes the C code actually branches on

**Build-time axes:** none.
`CMakeLists.txt` passes no `-D`; the source contains no `#ifdef` other than the
`DRIVER_H_` include guard. `Cargo.toml` has no `[features]`. → exactly **one**
build configuration.

**Runtime option/mode/flag axes:** none.
There is no init function, no context struct, no global state, no setter. Both
functions are pure w.r.t. process state except for writing to `stdout`.

**Public entry points** (the *full* set, from `nm -D` — note `printLine` is a
public exported symbol even though `driver.h` only declares `driver`, so it must
be driven **directly** and not only through the `driver` wrapper):

| entry point | signature | in `driver.h`? | exported? |
|---|---|---|---|
| `printLine` | `void printLine(const char *line)` | no | **yes** (lowest level) |
| `driver`    | `void driver(int data)`            | yes | yes (calls `printLine`) |

**Input-shape axes:**

* `printLine`: null / non-null; length 0, 1, many; content class (plain ASCII,
  `%`-format metacharacters, embedded newlines, high/non-ASCII bytes, all 255
  non-NUL byte values); length vs the 4096-byte `stdout` buffer
  (below / exactly at / above → forces a mid-string `fflush` inside glibc).
* `driver`: the `int data` domain, split by the `data < 100` branch and by the
  `strncpy` length semantics:
  `data < 0` (UB/crash), `0`, `1..=98`, `99` (last in-bounds `dest[data]`),
  `100`, `101..`, `INT_MAX`, `INT_MIN`.
* Call-sequence axis: single call vs repeated/mixed calls (glibc `stdout` buffer
  carries state across calls, so interleaving is a distinct shape).

`driver` never encounters a `'\0'` inside the first `data` bytes of `source`
(only `source[99]` is NUL and `data <= 99`), so `strncpy`'s NUL-stop and
zero-padding paths are **unreachable from the public API**, and no row can
exercise them. This is proved (rather than assumed) in `ERRORS.md`; mutation
testing confirms a mutant that deletes the NUL-stop is behaviourally equivalent.
The nearest reachable approximation, `data == 99` — which copies all 99 `'A'`s
and stops exactly one byte before `source`'s NUL — is row C12.

## The table

One row per meaningful combination the C treats differently. Every row is run
against **many randomized inputs with a fixed seed** (`SEED = 0x5EED_1234_ABCD`,
xorshift64\*), not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | randomized inputs | [x] |
|---|----------------|-------------------------------------------|-------------------|-----|
| C1 | `printLine` | non-null, length 0 (`""`) — degenerate | 1 (only one such value) | [x] |
| C2 | `printLine` | non-null, length 1, every possible single non-NUL byte `0x01..0xFF` | all 255 | [x] |
| C3 | `printLine` | non-null, random printable-ASCII strings, len 2..=64 | 512 | [x] |
| C4 | `printLine` | non-null, random **arbitrary non-NUL bytes** (incl. high/non-UTF-8), len 1..=256 | 512 | [x] |
| C5 | `printLine` | non-null, string containing `%s %d %n %%` etc. (format metacharacters) at random positions | 256 | [x] |
| C6 | `printLine` | non-null, string containing embedded `'\n'`, `'\r'`, `'\t'` at random positions | 256 | [x] |
| C7 | `printLine` | non-null, buffer with an **embedded NUL** — bytes after the NUL must be ignored | 256 | [x] |
| C8 | `printLine` | non-null, **long** strings straddling the glibc `stdout` buffer: len ∈ {4095, 4096, 4097, 8191, 8192, 8193} + random len 1000..=20000 | 6 fixed + 64 random | [x] |
| C9 | `printLine` | repeated calls in one capture window (2..=16 calls, random strings) — buffer state carried across calls | 128 | [x] |
| C10 | `driver` | `data == 0` — `strncpy` copies 0 bytes, `dest[0]='\0'` | 1 | [x] |
| C11 | `driver` | `data ∈ 1..=98` — the ordinary in-range path, `data` `'A'`s | all 98, exhaustively | [x] |
| C12 | `driver` | `data == 99` — boundary: last in-bounds `dest[data]`, `strncpy` copies the full 99 `'A'`s and stops one byte before `source`'s NUL | 1 | [x] |
| C13 | `driver` | `data == 100` — first value failing `data < 100`; `dest` untouched | 1 | [x] |
| C14 | `driver` | `data ∈ 101..=INT_MAX` — guard-failing path | 512 random + `101`, `1<<8`, `1<<16`, `1<<30`, `INT_MAX` | [x] |
| C15 | `driver` | `data` swept **exhaustively** over the entire valid (non-crashing) domain `0..=INT_MAX` boundary structure: all of `0..=200`, then random | 201 + 512 random | [x] |
| C16 | `driver` | repeated `driver` calls with random in-range `data` in one capture window (2..=16 calls) — cross-call buffer state | 128 | [x] |
| C17 | `driver` + `printLine` | **mixed** call sequences interleaving both entry points in one capture window (2..=16 calls, random kinds/args) — the composed pipeline | 256 | [x] |
| C18 | `driver` | called after `printLine` left a partially-filled `stdout` buffer (no trailing newline flush) — checks `driver`'s output is not reordered | 128 | [x] |

Rows C10–C16 collectively sweep the **entire non-crashing `int` domain
structure** for `driver`; the crashing sub-domain (`data < 0`) is covered by
`ERRORS.md` rows E8–E10.

## Verification method

For every row, both libraries are loaded through `libloading`
(`c_src/build/libdriver.so` and `target/debug/libdriver.so`), the symbol is
resolved by name (so the `#[no_mangle]`/`extern "C"` export wrappers are what is
actually tested), `stdout` (fd 1) is redirected to a private temp file around
each call, `fflush(NULL)` is issued on the **shared** glibc `stdout` used by both
`.so`s, and the captured byte streams are compared with `assert_eq!` for exact
equality. Rust functions are never called directly.

## Notes discovered while executing the table

* **A stale-`.so` trap.** The crate declares `crate-type = ["cdylib"]` only, so
  `cargo test` does **not** rebuild the library (integration tests never link
  it). A first run of the suite silently tested a `.so` that was 9 minutes older
  than `src/lib.rs`, and an intentionally-broken mutant *passed*. The harness now
  asserts that each `.so` is newer than its newest source file
  (`assert_fresh` in `tests/common/mod.rs`) and `verify.sh` always runs
  `cargo build` before `cargo test`.
* **fd-1 capture must be serialised.** Because both entry points write to the
  process-wide `stdout`, the harness redirects fd 1. libtest's own progress
  output ("`test foo ... ok`"), emitted from other threads, leaked into capture
  windows and corrupted 6 rows on the first run. The suite now forces
  `RUST_TEST_THREADS=1` from an ELF `.init_array` constructor, which also makes
  libtest run tests on the main thread — the safest context for the
  `fork()`-based crash comparisons.
* **`strncpy`'s NUL-stop / zero-pad path is unreachable** from the public API
  (`driver` only ever passes `n = data <= 99`, and `source`'s NUL sits at index
  99), so it has no row of its own. See `ERRORS.md` for the proof.
* **Both build profiles are verified.** `[profile.release] panic = "abort"` makes
  `release` a distinct configuration; `verify.sh` runs every row under both
  `dev` and `release`.
