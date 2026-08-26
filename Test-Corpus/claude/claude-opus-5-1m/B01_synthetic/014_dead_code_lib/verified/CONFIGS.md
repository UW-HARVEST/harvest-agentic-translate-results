# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Build-time configuration axes

| source | axes found | combinations |
|---|---|---|
| `Cargo.toml` | **no `[features]` section at all** (`grep -n feature Cargo.toml` → nothing); `crate-type = ["cdylib"]` | exactly **1**: the empty feature set (`--no-default-features` ≡ default) |
| `c_src/CMakeLists.txt` | one `add_library(driver SHARED src/driver.c)`; no `option()`, no `target_compile_definitions`, no generator branches | **1** |
| `c_src/src/driver.c` | no `#ifdef` / `#if` other than the header's include guard | **1** |

So the full feature-combination enumeration is: `{}` (verified as both
`cargo test` and `cargo test --no-default-features`, plus
`cargo check --no-default-features --all-features`, which is also `{}`).

## Runtime configuration axes (derived from the C source)

The library exposes no options, modes, flags, or global state — the only
conditional in the whole translation unit is `if (line != NULL)`. The axes the
C code actually distinguishes are therefore:

* **Axis 1 — entry point** (all 4 exported symbols, lowest-level first):
  `printLine` (leaf), `bad`, `good` (calls `static helperGood`),
  `driver` (calls `good`, `bad`, `printLine`). The two `static` helpers are
  reachable only indirectly: `helperGood` via `good`; `helperBad` is
  **dead code and must stay unreachable**.
* **Axis 2 — `printLine` input shape**: NUL-only (empty), 1 byte, short ASCII,
  long, huge; ASCII vs. control bytes vs. high/invalid-UTF-8 bytes; embedded
  `printf` directives; embedded newlines/tabs; interior/unaligned pointer.
* **Axis 3 — call multiplicity / sequence** (exercises `stdout` buffering and
  the composed pipeline): single call, repeated same call, mixed sequences,
  full `driver()` end-to-end, `driver()` twice.

## Configuration rows

Each row is run against **both** `.so`s through `libloading` and compared
byte-for-byte. Rows marked *randomized* use ≥256 pseudo-random inputs from a
fixed-seed LCG (seed `0x2545F4914F6CDD1D`) so they are reproducible.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printLine` | empty string `""` (zero-length payload) | [x] |
| C2 | `printLine` | single byte, exhaustively for all 255 non-NUL byte values `0x01`–`0xFF` | [x] |
| C3 | `printLine` | short ASCII printable strings, *randomized* (len 1–32) | [x] |
| C4 | `printLine` | random bytes over the full non-NUL alphabet `0x01`–`0xFF`, *randomized* (len 1–256, invalid UTF-8 included) | [x] |
| C5 | `printLine` | strings made only of control bytes `0x01`–`0x1F` + `0x7F`, *randomized* | [x] |
| C6 | `printLine` | strings containing `printf` directives (`%s %d %n %p %%`, `%1$s`), *randomized* mixes | [x] |
| C7 | `printLine` | strings containing embedded `\n`, `\t`, `\r` (multi-line payloads), *randomized* | [x] |
| C8 | `printLine` | boundary lengths: 1, 2, 255, 256, 257, 1023, 1024, 4095, 4096, 65535, 65536, 1 MiB | [x] |
| C9 | `printLine` | interior pointer into a larger buffer (unaligned start offsets 0–7) | [x] |
| C10 | `printLine` | repeated calls in one capture (256 randomized lines, buffered `stdout`) | [x] |
| C11 | `bad` | no-arg, single call — and assert `helperBad()` output never appears | [x] |
| C12 | `bad` | no-arg, repeated calls (×64) | [x] |
| C13 | `good` | no-arg, single call — exercises `static helperGood` indirectly | [x] |
| C14 | `good` | no-arg, repeated calls (×64) | [x] |
| C15 | `driver` | no-arg, single end-to-end call (composed pipeline: `printLine`+`good`+`bad`) | [x] |
| C16 | `driver` | no-arg, repeated calls (×16), state-leak check | [x] |
| C17 | mixed sequence | *randomized* interleavings of `printLine`/`good`/`bad`/`driver` in one capture (64 sequences × up to 12 calls) | [x] |
| C18 | mixed sequence | `printLine(NULL)` interleaved between valid calls (guard must not disturb ordering) | [x] |
| C19 | all 4 | one capture per entry point with `stdout` redirected to a file (fully buffered) vs. many small captures — flush ordering | [x] |
| C20 | all 4 | feature combination `{}` under `--no-default-features` (identical code, re-run of C1–C19) | [x] |

Rows C1–C19 are implemented in `tests/valid_paths.rs`; C20 is the re-run of the
whole suite under the other (equivalent) feature invocation, driven by
`run_all_features.sh`.
