# Verification report

C ground truth: `c_src/src/driver.c` (46 lines, 1 translation unit)
Rust under test: `translation/src/lib.rs`

Both are loaded as shared objects via `libloading` and called **only** through
their exported C symbols, so the `#[unsafe(no_mangle)] extern "C"` wrappers are
themselves under test. No Rust function is ever called directly.

## How to run

```sh
# One-shot: builds the C .so, checks every feature combo, runs every phase
# against both the debug and release Rust artifacts, and diffs the symbols.
cd translation && ./verify.sh

# Or by hand:
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo test
```

`translation/.cargo/config.toml` sets `net.offline = true` because this
environment has no crates.io egress; `libloading 0.8.9` resolves from the local
registry cache and `Cargo.lock` pins it. Remove that file if you have network
access and want cargo to refresh the index.

## Result

| phase | artifact | tests | result |
|-------|----------|-------|--------|
| A | `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` | — | complete |
| B | `tests/phase_b_configs.rs` | 34 (one per `CONFIGS.md` row) | **pass** |
| C | `tests/phase_c_errors.rs` | 14 (all `ERRORS.md` rows + generic boundaries) | **pass** |
| D | `tests/phase_d_symbols.rs` | 5 | **pass** |

53 tests, run across 2 feature combinations × 2 Rust build profiles
(= 4 full suite runs) plus the symbol diff. `./verify.sh` reports
`8 passed / 0 failed / ALL CHECKS PASSED`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff between the C `.so` and the Rust `.so` is
      **empty in both directions**. The C exports exactly `driver` and
      `fma_array`; the Rust `.so` exports both with identical names and nothing
      extra beyond Rust/libc toolchain symbols. 0 missing/undefined non-libc
      symbols. The `static` C function `inner` is correctly exported by
      *neither* (asserted by `err_static_inner_not_exported`). No C module was
      left untranslated — `c_src` has a single `.c` file and both of its
      external-linkage functions are implemented.
- [x] **Phase B** — every one of the 34 `CONFIGS.md` rows passes across
      randomized inputs (fixed seed `0x5EEDC0DE12345678`, SplitMix64;
      200 iterations for most rows). Coverage includes the **low-level**
      `fma_array` entry point directly (not just the `driver` wrapper), all
      8 pointer-aliasing patterns × an 18-point length sweep × 3 value
      distributions, full-range `i32` values so overflow is reached naturally,
      lengths 0/1/2/small/medium/4096, and a composed-pipeline cross-check
      (row C31) that ties `driver`'s stdout back to `fma_array`'s buffer.
- [x] **Phase C** — every one of the 16 `ERRORS.md` rows has a passing
      differential test or a documented justification, plus the generic
      boundaries: null pointers, zero and oversized lengths, and values one step
      past the valid range. The C declares **no enum parameter**, so the
      out-of-range-enum class is covered by feeding arbitrary `int` bit patterns
      (including `INT_MIN`) in the `len` position.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]`, so
      the complete set is `{default}` ≡ `{--no-default-features}`; `verify.sh`
      derives this from `Cargo.toml` mechanically and runs both, and would
      expand to the full power set if features were ever added.

## Test sensitivity (mutation testing)

A passing suite proves nothing unless it can fail. Five deliberate defects were
injected into `translation/src/lib.rs` and the suite was re-run:

| mutation | detected? | by |
|----------|-----------|----|
| `wrapping_add` → `wrapping_sub` (wrong arithmetic) | yes | 29 tests |
| `wrapping_mul` → `saturating_mul` (wrong overflow semantics) | yes | 26 tests |
| `printf` format `"%d\n"` → `"%d "` (wrong output framing) | yes | 12 tests |
| `if len > 0` → `if len > 1` (off-by-one allocation) | yes | `cfg_c21_driver_len_one` (SIGSEGV; `verify.sh` exits 1) |
| print loop `i < len` → `i < len - 1` (drops last element) | yes | 12 tests |

`src/lib.rs` was restored byte-for-byte afterwards and the suite returns to
green.

## Behavioural findings

For **every input the C defines**, the two `.so`s agree byte-for-byte — both in
the bytes `fma_array` writes (the whole arena is compared, not just `out`, so
stray writes are caught) and in `driver`'s captured stdout.

Two divergences exist, and both lie strictly inside **undefined behaviour** where
the C `.so` produces no result to match. They are measured, not assumed, and
documented in detail in `ERRORS.md`:

1. **`driver(data, len)` with `len < 0`** — `len * sizeof(int)` at `driver.c:44`
   promotes the negative `int` to `size_t`, so `memcpy` is asked for ~1.8e19
   bytes. Measured across lengths, the C variously **segfaults, takes SIGBUS,
   hangs forever, or returns cleanly** (`len = -1000` exits 0). There is no
   reproducible C result. The Rust guards `len > 0` and returns silently.
   `err_e12_driver_len_negative` asserts the Rust is deterministic and benign,
   and that wherever the C *does* return normally the two agree exactly (both
   silent) — the only case where a comparison is meaningful.
2. **`driver(data, len)` with a large `len`** — `int out[len]` is an
   unprobed VLA, so the C stack-overflows (SIGSEGV) from about `len = 16384`
   upward; the Rust heap-allocates and survives. `err_e14_driver_len_stack_overflow`
   pins this. Phase B stays at or below `len = 4096`, which the C handles.

Both rows are exercised in forked subprocesses with a watchdog alarm so the
exact disposition (exit code vs. terminating signal vs. hang) is compared rather
than guessed. The C stack overflow in row E14 makes the Rust runtime's segfault
handler print `has overflowed its stack / fatal runtime error: stack overflow`
to stderr from the child — that is expected test output, not a failure.

Notably the C's signed-overflow behaviour was **verified against a native C
probe** rather than assumed: `INT_MAX * 2 + 1 == -1` and
`INT_MAX * INT_MAX + INT_MAX == INT_MIN`, i.e. plain two's-complement wrapping,
which the Rust reproduces with `wrapping_mul`/`wrapping_add`. This holds
identically for the optimized (`--release`) Rust artifact.

## Notes on the harness

- `driver` writes with libc `printf`, so output is compared by redirecting file
  descriptor 1 to a temp file (`dup`/`dup2`) around each call. The redirect is
  process-global, so `common::capture_stdout` serializes on a mutex *and* holds
  the Rust `stdout` lock for the duration, which keeps libtest's own progress
  output from contaminating the capture. Verified robust under default parallel
  test threads (5/5 clean runs) as well as `--test-threads=1`.
- The Rust `.so` imports the platform `printf` rather than reimplementing
  integer formatting, so `driver`'s output is byte-identical to the C's and
  interleaves with other C stdio in the same process exactly as before. This is
  asserted by `phase_d_rust_has_no_missing_non_libc_symbols`.
- Tests default to `target/debug/libdriver.so`, which `cargo test` always
  rebuilds, so a stale artifact can never be silently validated. Override with
  `DRIVER_RUST_SO` / `DRIVER_C_SO`.
