# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the C source and public header. Axes the C code can
actually be varied along:

**Public entry points** (full set, from `c_src/include/hello.h`):

* `int helloworld();` — the *only* public entry point, and it is simultaneously
  the lowest-level one (there are no convenience wrappers layered over
  anything; there is no init/teardown, no context object, no options struct).

**Runtime options / flags the C code branches on**: none inside the library —
there is no `if`, no `switch`, no `#ifdef`, no global mode variable. The
behaviour-relevant configuration is therefore entirely in the *libc `stdout`
state* the caller sets up, which the single `printf` call consumes:

| axis | values the code path actually distinguishes |
|---|---|
| stdout buffering mode | `_IOFBF` (fully buffered, default for files/pipes), `_IOLBF` (line buffered, default for ttys), `_IONBF` (unbuffered) |
| stdout buffer provisioning | libc-allocated buffer vs. caller-supplied buffer (`setvbuf` with own array), tiny buffer (smaller than the 13-byte output → forces multiple `write(2)`s) |
| stdout target kind | regular file, pipe, `/dev/null`, `O_APPEND` file, file with non-zero initial offset |
| flush point | implicit at exit / explicit `fflush(stdout)` / `fflush(NULL)` |
| call count | 0, 1, 2, many (randomized 1..=64), very many (10 000) |
| call ordering | all-C then all-Rust, all-Rust then all-C, randomly interleaved C/Rust into the *same* stream |
| linkage/ABI shape | called as `extern "C" fn() -> c_int`; also via unprototyped-C-style pointer with extra args (see ERRORS.md #6) |
| library load state | freshly `dlopen`ed per call vs. one handle reused for many calls; both libs loaded simultaneously |
| symbol resolution | resolved via `dlsym` on the specific handle (must not accidentally bind to the other library's `helloworld`) |

Rows below are the pruned cross-product of these axes (the combinations libc
actually treats differently). Every row is exercised with randomized
parameters where an axis is numeric (seeded PRNG, fixed seed `0x5EED_1234`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `helloworld` | single call, stdout → regular file, default (`_IOFBF`) buffering, explicit `fflush`; compare returned `int` and captured bytes | [x] |
| 2 | `helloworld` | single call, stdout → regular file, `_IONBF` unbuffered | [x] |
| 3 | `helloworld` | single call, stdout → regular file, `_IOLBF` line buffered, caller-supplied buffer | [x] |
| 4 | `helloworld` | single call, stdout → regular file, `_IOFBF` with a **tiny caller-supplied buffer (4 bytes)** so the 13-byte output spans several `write(2)` calls | [x] |
| 5 | `helloworld` | single call, stdout → **pipe** (read end drained), default buffering | [x] |
| 6 | `helloworld` | single call, stdout → `/dev/null` (write always succeeds, discards) — return value only | [x] |
| 7 | `helloworld` | single call, stdout → file opened `O_APPEND` with pre-existing content (offset semantics) | [x] |
| 8 | `helloworld` | single call, stdout → file positioned at a **non-zero initial offset** (randomized 0..4096 padding) | [x] |
| 9 | `helloworld` | **N randomized calls** (N ∈ 1..=64, seeded) in a row on one handle, `_IOFBF`, file target: byte stream must be exactly N repetitions and every return value `0` | [x] |
| 10 | `helloworld` | **zero calls** (degenerate shape): stream must contain 0 bytes for both libs | [x] |
| 11 | `helloworld` | **randomly interleaved** C and Rust calls into the *same* stdout stream (seeded random schedule, 1..=64 calls) — output must be indistinguishable, i.e. identical to N repetitions | [x] |
| 12 | `helloworld` | all-C-then-all-Rust vs. all-Rust-then-all-C ordering, equal counts, same stream | [x] |
| 13 | `helloworld` | **fresh `dlopen`/`dlclose` per call** (library re-initialised each time) vs. one long-lived handle — no load-time state difference | [x] |
| 14 | `helloworld` | **both** libraries loaded simultaneously; `dlsym` on each handle must yield two *distinct* function addresses (no symbol collision masking the Rust impl) | [x] |
| 15 | `helloworld` | **very many calls** (10 000) with a single flush at the end — buffer-refill path, byte-exact bulk comparison | [x] |
| 16 | `helloworld` | called via unprototyped-style pointer with extra register args (ABI shape axis; mirrors ERRORS.md #6) | [x] |
| 17 | `helloworld` | `fflush(NULL)` (flush-all) instead of `fflush(stdout)` as the flush point | [x] |
| 18 | `helloworld` | multi-threaded: 8 threads × randomized calls each, shared `stdout`; total bytes and all return values compared between C and Rust | [x] |

## Verification result

`./run_tests.sh` (builds both `.so`s, then `RUST_TEST_THREADS=1 cargo test`):

* `tests/phase_b_valid.rs` — 18 tests, one per row above: **18 passed, 0 failed**.
* Randomized rows use the seeded SplitMix64 PRNG (`SEED = 0x5EED_1234`), so runs
  are reproducible: row 8 × 8 iterations, row 9 × 24 iterations (random call
  count × random buffering mode × random flush point), row 11 × 16 random
  C/Rust interleavings, row 16 × 8 random argument sets.

## Feature-combination matrix

`translation/Cargo.toml` declares **no `[features]` section**, so `default` is the
entire matrix; `run_tests.sh` enumerates `[features]` mechanically and would loop
over `--no-default-features --features <combo>` if any existed. The C build has
no `#ifdef`-driven variants either (`c_src/CMakeLists.txt` defines no options).

## Binary / driver executable

Neither side builds an executable: `c_src/CMakeLists.txt` contains no
`add_executable` (grep count 0) and the crate has no `[[bin]]` and no
`src/main.rs`. The "compare C and Rust binary stdout" gate is therefore N/A —
but stdout is still compared byte-for-byte through the `.so`s in every row above.

## Negative control (tests proven non-vacuous)

Three mutations were temporarily injected into `translation/src/hello.rs`, built,
and run; the original source was then restored byte-for-byte:

| mutation | detected by |
|---|---|
| `"Hello World!\n"` → `"Hello World?\n"` (1 byte) | 15 / 18 Phase-B tests failed |
| `return 0` → `return 1` | 16 Phase-B/D + **all 11** Phase-C tests failed |
| `printf` call removed (silent stub, still returns 0) | 15 Phase-B + `d04_no_stubbed_symbol` failed |
