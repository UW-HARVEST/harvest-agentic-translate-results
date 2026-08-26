# Differential verification report — C `libdriver.so` vs. Rust `libdriver.so`

The C implementation in `c_src/` is the ground truth. The Rust translation in
`src/lib.rs` must be byte-identical for every input.

## How to run

```sh
./verify.sh              # every build configuration, end to end
./verify.sh default      # just one configuration
```

`verify.sh` (a) builds the C `.so` with CMake, (b) `cargo check`s and
`cargo build`s each configuration, (c) diffs `nm -D`, and (d) runs the test
suite serially.

Two things that are easy to get wrong and are enforced by the harness:

* **`cargo test` alone does NOT rebuild a `crate-type = ["cdylib"]` artifact**
  (integration tests do not link against it), so a plain `cargo test` after
  editing `src/lib.rs` would happily diff against a *stale* `.so` and report
  success. `tests/common/mod.rs::assert_fresh` compares the `.so`'s mtime with
  every source file and aborts instead of passing vacuously. `verify.sh` always
  runs `cargo build` first.
* The suite captures fd 1 / fd 2 **process-wide** (`dup2`), so it must run
  serially — otherwise libtest's own progress output lands inside a captured
  stream. `require_serial_execution()` refuses to run without
  `-- --test-threads=1`.

## Test architecture

Both libraries are loaded with `libloading` (`dlopen`) and every call goes
through the exported C ABI (`dlsym`), so the Rust `#[no_mangle] extern "C"`
wrappers are exercised exactly as an external consumer would. The Rust
functions are never called directly.

"Output" for this library means three things, all compared:

1. the return value (`int`, or the NULL-ness/state of the returned `FILE*`);
2. the exact bytes written to `stdout` and to `stderr` (captured by `dup2`-ing
   fd 1 / fd 2 onto scratch files; also captured with **both fds pointing at the
   same file** so the interleaving of buffered `stdout` and unbuffered `stderr`
   is compared too);
3. descriptor accounting via `/proc/self/fd`, which is the only way to observe
   the `fclose` calls in the C cleanup path.

| file | purpose |
|------|---------|
| `tests/common/mod.rs` | harness: dual `dlopen`, fd capture, freshness gate, SplitMix64 PRNG, fixtures, differential asserts |
| `tests/phase_b_valid.rs` | Phase B — one test per `CONFIGS.md` row (26) |
| `tests/phase_c_errors.rs` | Phase C — one test per `ERRORS.md` row (19) + 5 generic FFI boundary tests |
| `tests/symbols.rs` | Phase D — `nm -D` parity, `dlsym`-ability, `RTLD_NOW` resolution, shared libc import surface |

## Results

| phase | scope | result |
|-------|-------|--------|
| A | `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` derived from the C source | complete |
| B | 26/26 `CONFIGS.md` rows, thousands of seeded random inputs | **pass** |
| C | 19/19 `ERRORS.md` rows + 5 generic FFI boundary cases | **pass** |
| D | 3/3 C symbols exported by the Rust `.so`; `nm -D` diff empty | **pass** |
| D | all build configurations (default / `--no-default-features` / `--all-features`) and both profiles (`dev`, `release`) | **pass** |

Totals per configuration: 26 + 24 + 5 = **55 tests, 0 failures**.

`Cargo.toml` has no `[features]` and `c_src/CMakeLists.txt` has no options or
`#ifdef` variants, so the three cargo invocations above are the complete set of
build-time configurations; all three are run by `verify.sh`.

## Evidence that the suite is not vacuous

Passing tests only mean something if they can fail. Eight mutations were
injected into `src/lib.rs` one at a time (then reverted); every one was caught:

| mutation of the Rust translation | caught by |
|----------------------------------|-----------|
| `fgets(buf, 100)` → `fgets(buf, 99)` (off-by-one in the read buffer) | `row11_owc_embedded_nuls`, `row12_owc_random_binary`, `row23_driver_binary_file` |
| `"Error: negative input\n"` → `"Error: negative input!\n"` | 5 Phase B rows |
| `if x < 0` → `if x <= 0` (branch boundary) | `row01`, `row19`, `row21`, `row25` |
| `if ferror(fp) == 0` → `!= 0` (inverted cleanup condition) | 20 Phase B rows |
| dropped `printf("Processing: %d\n", x)` | 13 Phase B rows |
| `if res == -1` → `if res < 0` (sentinel widened; only differs on overflowed `num`) | `row22_driver_overflowing_num`, `row26_long_mixed_session` |
| dropped `fclose(fp)` in the cleanup label (invisible in output) | `leak01…`, `leak03…` |
| dropped `fclose(out)` in `driver` (invisible in output) | `leak03…` |

Note which rows did the work: the off-by-one in the `fgets` size is invisible
for ordinary text (splitting a line at a different offset produces the same
concatenated `printf` output) and is only detectable through content with
**embedded NUL bytes**, because `printf("%s", buffer)` truncates there. That is
exactly the kind of blind spot the `CONFIGS.md` cross-product exists to close.

## Notes on faithfulness

Behaviours of the C code that the Rust translation deliberately reproduces and
that the tests pin down:

* `x * 2` is signed-overflow UB in C; GCC at `-O0` wraps, and the Rust uses
  `wrapping_mul`, so `INT_MAX` → `-2` in both (verified — and `-2` is *not* the
  `-1` sentinel `driver` tests for, so `driver` still returns `0`).
* All I/O goes through the same glibc `stdio` entry points in both libraries, so
  the buffering (`stdout` buffered, `stderr` unbuffered) and hence the
  interleaving is identical; a Rust translation using `std::io` would diverge
  here and `row25_merged_streams_interleaving` would fail.
* `printf("%s", buffer)` stops at the first NUL, so lines containing NUL bytes
  are echoed truncated. Preserved.
* `filename` is passed straight through to `fopen`/`fprintf`, so a NULL pointer
  yields glibc's `(null)` rendering and non-UTF-8 paths work unchanged.
* GCC lowers `fprintf(stderr, "literal")` to `fwrite`, which is why the C `.so`
  imports `fwrite` and the Rust one does not; the emitted bytes are identical.
