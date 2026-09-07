# Verification record

## How to reproduce

```
./verify_all.sh          # builds the C .so + every Rust config, runs everything
```

or manually:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build && cargo build --release && cargo test
```

Both `.so` files are loaded with `libloading` and every call crosses the FFI
boundary through the exported symbols; no Rust function is ever called directly.
`CTORUST_RUST_SO=<path>` pins which Rust artifact is exercised, so the same
suite runs against the `dev` and the `release` cdylib.

## Results

| suite | tests | result |
|-------|-------|--------|
| `phase_b_valid_paths.rs` (CONFIGS.md rows A1–C10) | 28 | pass |
| `phase_c_error_paths.rs` (ERRORS.md rows 1–13 + generic boundaries) | 15 | pass |
| `phase_d_symbols.rs` (symbol parity / dlsym / load) | 3 | pass |

Run for every configuration: `{default, --no-default-features}` x
`{dev, release}` = 4 combinations, all passing (`verify_all.sh` output).
`Cargo.toml` has no `[features]` table, so those two are the complete set.

`nm -D --defined-only` symbol diff (C minus Rust) is **empty** in all four
configurations.

## Harness sensitivity (mutation check)

Because "all green" is only meaningful if the harness can actually see a
divergence, the suite was validated by injecting deliberate bugs into
`src/lib.rs`, rebuilding, and confirming failures.  `src/lib.rs` was then
restored and verified byte-identical to its original content.

| injected mutation | detected? | failing tests |
|-------------------|-----------|---------------|
| `x * 2` -> `x * 3` | yes | 14 of 28 |
| `if x < 0` -> `if x <= 0` | yes | 16 of 28 |
| `fgets` size 100 -> 99 (buffer boundary) | yes | 18 of 28 |
| `driver` returns `-3` instead of `-2` | yes | 18 of 28 |
| stderr message `negative` -> `Negative` | yes | 18 of 28 |
| `ferror(fp) != 0` -> `== 0` | yes | 28 of 28 |

An early version of the harness produced 7 spurious failures because libtest's
own `test foo ... ok` progress output was landing inside the `dup2` capture
window from other test threads.  Fixed by flushing Rust's `stdout`/`stderr`
alongside `fflush(NULL)` before each redirect, and by pinning
`RUST_TEST_THREADS=1` in `.cargo/config.toml` (fd 1/2 capture is inherently a
per-process operation).

## Notes on behaviours deliberately replicated, not "fixed"

* `forward_goto_example` returns `x * 2`, which **overflows** for
  `x > INT_MAX/2` (UB in C).  The Rust uses `wrapping_mul(2)`, matching the
  two's-complement `imul` the C compiler emits; verified for `INT_MAX` and 500
  randomized overflowing values.
* `forward_goto_example(-1)` and its error path both return `-1`, so `driver`
  cannot distinguish "error" from "the value `-1`".  Replicated verbatim
  (ERRORS.md row 3).
* `printf("%s", buffer)` truncates at an embedded NUL even though `fgets`
  consumed the full line, so a file containing NUL bytes is not echoed
  faithfully.  Replicated (CONFIGS.md row B8).
* `open_with_cleanup` succeeds on a **directory** (glibc `fopen` allows it) and
  only fails later at the `ferror` check, which is the one path where
  `fclose(fp)` runs during cleanup.  Replicated (ERRORS.md row 6).
* The C code declares `char buffer[100]` **uninitialized**; the Rust zeroes it.
  This is unobservable: the buffer is only ever printed after a successful
  `fgets`, which always NUL-terminates.
* `fprintf(stderr, "...%s\n", filename)` with `filename == NULL` prints glibc's
  `(null)`.  Both implementations pass the raw pointer through, so both print
  it (ERRORS.md rows 5 and 12).

## Scope

`c_src` is a single translation unit (`src/goto.c`, 81 lines) with one public
header.  All three external-linkage functions are translated; nothing was
skipped, stubbed, or `unimplemented!()`.  Neither project builds an executable
(`CMakeLists.txt` has no `add_executable`, `Cargo.toml` has no `[[bin]]`), so
stdout is compared directly around the FFI calls instead of by diffing two
binaries' output.
