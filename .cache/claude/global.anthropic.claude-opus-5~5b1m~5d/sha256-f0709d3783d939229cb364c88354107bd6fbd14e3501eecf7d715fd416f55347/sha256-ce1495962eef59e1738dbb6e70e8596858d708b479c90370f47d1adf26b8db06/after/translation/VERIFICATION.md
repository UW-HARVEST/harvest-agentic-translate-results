# Verification report — C `hello` → Rust translation

The C library is the ground truth. Verification drives **both** shared objects
through `libloading` and compares them across the FFI boundary; no Rust function
is ever called directly, so the `#[unsafe(no_mangle)]` export wrapper is itself
under test.

## How to reproduce

```sh
cd translation
./verify.sh                   # build C + Rust, run every phase, all profiles/features
./verify.sh --with-mutation   # ...and prove the suite is not vacuous
```

`verify.sh` builds the C library with CMake, then for each of
{debug, release} × {default, `--no-default-features`, `--all-features`} runs
`cargo build` followed by `cargo test`, and finishes with the `nm -D` symbol
diff.

## Surface under test

The library is one translation unit exposing one branchless, parameterless
function:

```c
int helloworld() { printf("Hello World!\n"); return 0; }
```

Both compilers lower the `printf` to `puts`, so both `.so`s emit
`Hello World!\n` through the same libc `stdout` stream.

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | 1 exported symbol, `helloworld`; symbol diff empty in both directions |
| `CONFIGS.md` | 17 valid-path configuration rows (B1–B17) |
| `ERRORS.md` | 8 applicable error rows (E1–E8) + applicability table for the inapplicable generic boundary classes |

Because the function takes no arguments, the configuration surface is not the
argument space but the environment the call acts on: `stdout`'s buffering mode
(default / `_IOFBF` / tiny caller buffer / `_IOLBF` / `_IONBF`), the descriptor
underneath it (regular file / pre-positioned file / `O_APPEND` / pipe /
`/dev/null` / closed / read-only / broken pipe / directory), the composition of
many calls, concurrency, and the call signature used at the ABI level.

## Results

```
tests/smoke.rs    2 passed   harness self-check
tests/phase_b.rs 17 passed   Phase B — every CONFIGS.md row
tests/phase_c.rs  9 passed   Phase C — every ERRORS.md row
```

All green for both profiles and all three feature spellings. Symbol parity:
0 missing, 0 extra; the Rust `.so` needs only `libc.so.6` and `libgcc_s.so.1`.

**No divergence from the C was found — the translation required no fixes.**

## Findings worth recording

The valuable output of this exercise was not a translation bug but three ways
the *verification* could have silently reported success:

1. **`cargo test` does not build a `cdylib`-only library target.** Only
   `cargo build` produces `target/<profile>/libhello.so`. An early mutation run
   had **all 12 mutants survive** — including "emit nothing at all" — purely
   because the tests were loading a stale `.so`. Guarded now by
   `assert_so_is_fresh`, which fails with a `STALE ARTIFACT` message, and by
   `verify.sh` always building before testing.

2. **libtest's own output pollutes an fd-1 capture.** Capturing what
   `helloworld` writes means redirecting the process-wide descriptor, and
   libtest writes `test … ok` there too. With 8 test threads the suite failed
   spuriously (4–5 tests per run). `.cargo/config.toml` sets
   `RUST_TEST_THREADS=1`, and `assert_serial_execution` refuses to run with an
   explanatory message rather than degrading into flakiness.

3. **glibc `setvbuf(fp, NULL, _IOFBF, 0)` does not detach a caller-owned
   buffer.** `_IO_setvbuf` returns early for `_IOFBF` with a NULL buffer when the
   stream already has one, so the B7 tiny-buffer row left `stdout` pointing at a
   freed `Vec` and the suite segfaulted. The teardown now goes through `_IONBF`
   first, which really does drop the stream's reference.

A fourth, smaller one: `run_captured` was not panic-safe, so the first real
assertion failure inside a capture had its message written *into the capture
file* and vanished. Restoration is now RAII (`RestoreStdout`), so divergences
are actually reportable.

## Meta-verification (`./mutation_test.sh`)

18 mutants of `src/hello.rs`, each rebuilt and re-tested:

* **15 KILL mutants** — wrong text, missing/extra newline, CRLF, missing `!`,
  double space, no output, doubled output, `return 1`, `return -1`, propagating
  `printf`'s result, reporting I/O failure, panicking on I/O failure, writing via
  Rust's `stdout` instead of the C stream, one extra space byte, and renaming the
  exported symbol — **all detected**.
* **3 EQUIV mutants** — `\x21` for `!`, an extra `printf("")`, and the same bytes
  via a `%s` format — **all correctly pass**, confirming the suite does not
  report false divergences.

The two behaviours that are easy to get wrong in this translation are both
covered by KILL mutants: writing through Rust's `std::io::stdout` instead of the
C `FILE*` (breaks interleaving and buffering — caught by 3 tests), and
propagating `printf`'s failure instead of swallowing it as the C does (caught by
7 tests).

## Layout

```
translation/
  SYMBOLS.md CONFIGS.md ERRORS.md VERIFICATION.md
  verify.sh              full build + test matrix + symbol diff
  mutation_test.sh       proves the suite is not vacuous
  .cargo/config.toml     RUST_TEST_THREADS=1
  tests/common/mod.rs    harness: dual dlopen, fd-level capture, buffering and
                         hostile-stdout control, seeded PRNG, staleness guard
  tests/smoke.rs         harness self-check
  tests/phase_b.rs       B1–B17
  tests/phase_c.rs       E1–E8 + boundary applicability
```
